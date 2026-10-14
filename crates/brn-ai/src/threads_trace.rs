//! An opt-in record of one run's tool use (evaluation design §1). It holds
//! structure only: tool names, rounds, IDs, versions, offsets, counts, byte
//! sizes, durations and token usage. Note and source text, titles,
//! snippets, answers, prepared record data, relation names, credentials and
//! paths are never recorded. `Full` adds query text and is meant for
//! synthetic evaluation data only.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    hash::BuildHasher,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Calls kept per trace; later calls only increase `dropped_calls`.
pub const MAX_CALLS: usize = 256;
const MAX_IDS: usize = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    #[default]
    Off,
    Summary,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TraceUsage {
    pub input: u64,
    pub output: u64,
    pub cached_input: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TraceCall {
    pub seq: usize,
    /// The tool round the call belongs to (1-based).
    pub round: u16,
    pub tool: String,
    pub args: Value,
    /// `success`, `error`, or the `apply_changes` outcome.
    pub outcome: String,
    /// UTF-8 length of the result returned to the model.
    pub bytes: usize,
    pub ms: u64,
    pub result: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunTrace {
    pub format: u8,
    pub level: TraceLevel,
    pub guide: String,
    /// Whether the host offered `search_notes` and `note_connections`.
    pub search: bool,
    pub rounds: u16,
    pub max_rounds: u16,
    pub model_turns: usize,
    /// `budget`, `invalid_batch` or `cancelled`; `None` when the run ended normally.
    pub stopped: Option<String>,
    /// `None` when the provider reported no token counts.
    pub usage: Option<TraceUsage>,
    pub elapsed_ms: u64,
    pub calls: Vec<TraceCall>,
    pub dropped_calls: usize,
    /// What the host retrieved for a source run before the first turn:
    /// counts, IDs, versions, sizes and timing, with query text at `Full`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_context: Option<Value>,
}

struct Pending {
    seq: usize,
    started: Instant,
}

/// Collects a trace from the run's hook events.
pub(crate) struct TraceCollector {
    level: TraceLevel,
    started: Instant,
    key: std::collections::hash_map::RandomState,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    calls: Vec<TraceCall>,
    pending: BTreeMap<String, Pending>,
    dropped: usize,
    seq: usize,
    usage: TraceUsage,
    usage_seen: bool,
    model_turns: usize,
    rounds: u16,
    stopped: Option<String>,
    host_context: Option<Value>,
}

fn ids(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .take(MAX_IDS)
                .collect()
        })
        .unwrap_or_default()
}

fn field_ids(rows: Option<&Value>, path: &[&str]) -> Vec<String> {
    rows.and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let mut value = row;
                    for key in path {
                        value = value.get(key)?;
                    }
                    value.as_str().map(str::to_owned)
                })
                .take(MAX_IDS)
                .collect()
        })
        .unwrap_or_default()
}

impl TraceCollector {
    pub(crate) fn new(level: TraceLevel) -> Option<Self> {
        (level != TraceLevel::Off).then(|| Self {
            level,
            started: Instant::now(),
            key: std::collections::hash_map::RandomState::new(),
            state: Mutex::new(State::default()),
        })
    }

    /// A keyed hash shows repeated queries within this run without
    /// revealing them; it cannot be compared across runs.
    fn query(&self, query: &str) -> Value {
        let mut out = json!({
            "query_hash": format!("{:016x}", self.key.hash_one(query))[..12].to_owned(),
            "words": query.split_whitespace().count(),
        });
        if self.level == TraceLevel::Full {
            out["query"] = json!(query);
        }
        out
    }

    fn args(&self, tool: &str, args: &Value) -> Value {
        let n = |key: &str| args.get(key).cloned().unwrap_or(Value::Null);
        match tool {
            "search_notes" => {
                let mut out = self.query(args.get("query").and_then(Value::as_str).unwrap_or(""));
                out["limit"] = n("limit");
                out["mode"] = n("mode");
                out["cursor"] = json!(!args.get("cursor").is_none_or(Value::is_null));
                out
            }
            "read_records" => {
                let ids = ids(args.get("ids"));
                let query = args.get("query").and_then(Value::as_str);
                let mut out = json!({
                    "id_count": ids.len(),
                    "ids": ids,
                    "catalog": ids.is_empty() && query.is_none(),
                });
                if let Some(query) = query {
                    out["query"] = self.query(query);
                }
                out
            }
            "read_note" | "read_history" => json!({
                "id": n("id"), "version": n("version"), "start": n("start"),
                "section": n("section"), "max_bytes": n("max_bytes"),
            }),
            "note_connections" => {
                json!({"id": n("id"), "hops": n("hops"), "sources": n("sources")})
            }
            "read_source" => json!({"source": "supplied"}),
            "read_skill" => json!({"name": n("id")}),
            "prepare_changes" | "refresh_candidate" => {
                let request = args.get("request");
                let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
                for write in request
                    .and_then(|r| r.get("writes"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(kind) = write
                        .get("data")
                        .and_then(Value::as_object)
                        .and_then(|data| data.keys().next())
                    {
                        *kinds.entry(kind.clone()).or_default() += 1;
                    }
                }
                let inputs = request
                    .and_then(|r| r.get("inputs"))
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                json!({"writes": kinds, "inputs": inputs})
            }
            "apply_changes" => json!({"operation": n("operation")}),
            _ => json!({}),
        }
    }

    fn result(tool: &str, value: Option<&Value>) -> Value {
        let Some(value) = value else {
            return Value::Null;
        };
        match tool {
            "search_notes" => json!({
                "total": value.get("total"), "shown": value.get("shown"),
                "ids": field_ids(value.get("results"), &["id"]),
                "semantic": value.get("semantic"),
            }),
            "read_records" => {
                let rows = value.get("records").and_then(Value::as_array);
                json!({
                    "rows": rows.map_or(0, Vec::len),
                    "ids": field_ids(value.get("records"), &["id"]),
                    "truncated": rows.is_some_and(|rows| rows.iter().any(|r| r.get("truncated").is_some())),
                })
            }
            "read_note" | "read_history" => {
                let note = value.get("note");
                json!({
                    "outcome": value.get("outcome"),
                    "returned_bytes": note
                        .and_then(|n| n.get("markdown"))
                        .and_then(Value::as_str)
                        .map_or(0, str::len),
                    "next_start": note.and_then(|n| n.get("next_start")).is_some_and(|v| !v.is_null()),
                })
            }
            "note_connections" => {
                let hops = value.get("hops").and_then(Value::as_array);
                let neighbours: Vec<String> = hops
                    .into_iter()
                    .flatten()
                    .flat_map(|hop| field_ids(hop.get("neighbours"), &["id"]))
                    .take(MAX_IDS)
                    .collect();
                json!({"neighbours": neighbours.len(), "ids": neighbours})
            }
            "prepare_changes" | "refresh_candidate" => json!({"operation": value.get("operation")}),
            _ => Value::Null,
        }
    }

    pub(crate) fn dispatch(&self, key: String, tool: &str, args: &str, round: u16) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.seq += 1;
        if state.calls.len() >= MAX_CALLS {
            state.dropped += 1;
            return;
        }
        let parsed = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
        let seq = state.seq;
        state.calls.push(TraceCall {
            seq,
            round,
            tool: tool.to_owned(),
            args: self.args(tool, &parsed),
            outcome: "pending".into(),
            bytes: 0,
            ms: 0,
            result: Value::Null,
        });
        state.pending.insert(
            key,
            Pending {
                seq,
                started: Instant::now(),
            },
        );
    }

    pub(crate) fn outcome(
        &self,
        key: &str,
        tool: &str,
        outcome: &str,
        result: Option<&rig::tool::ToolResult>,
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some(pending) = state.pending.remove(key) else {
            return;
        };
        let output = result.map(|r| r.output());
        let json = output.and_then(|o| o.as_json());
        let bytes = match (json, output.and_then(|o| o.as_text())) {
            (Some(json), _) => json.to_string().len(),
            (None, Some(text)) => text.len(),
            _ => 0,
        };
        if let Some(call) = state.calls.iter_mut().find(|c| c.seq == pending.seq) {
            call.outcome = outcome.to_owned();
            call.bytes = bytes;
            call.ms = duration_ms(pending.started.elapsed());
            call.result = Self::result(tool, json);
        }
    }

    pub(crate) fn model_turn(&self, usage: &rig::completion::Usage, rounds: u16) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.model_turns += 1;
        state.rounds = rounds;
        let (input, output) = (
            usage.input_tokens.unwrap_or(0),
            usage.output_tokens.unwrap_or(0),
        );
        let cached = usage.cached_input_tokens.unwrap_or(0);
        if input + output > 0 {
            state.usage_seen = true;
        }
        state.usage.input += input;
        state.usage.output += output;
        state.usage.cached_input += cached;
    }

    pub(crate) fn host_context(&self, mut summary: Value) {
        if self.level != TraceLevel::Full
            && let Some(object) = summary.as_object_mut()
        {
            object.remove("query_text");
            object.remove("uncovered_detail");
        }
        if let Ok(mut state) = self.state.lock() {
            state.host_context = Some(summary);
        }
    }

    /// Records why the run stopped early (first reason wins).
    pub(crate) fn stop(&self, reason: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.stopped.get_or_insert_with(|| reason.to_owned());
        }
    }

    pub(crate) fn finish(&self, guide: &str, search: bool, max_rounds: u16) -> RunTrace {
        let state = self.state.lock().map(|s| {
            (
                s.calls.clone(),
                s.dropped,
                s.model_turns,
                s.usage_seen.then(|| s.usage.clone()),
                s.rounds,
                s.stopped.clone(),
                s.host_context.clone(),
            )
        });
        let (calls, dropped, model_turns, usage, rounds, stopped, host_context) =
            state.unwrap_or_default();
        RunTrace {
            format: 1,
            level: self.level,
            guide: guide.to_owned(),
            search,
            rounds,
            max_rounds,
            model_turns,
            stopped,
            usage,
            elapsed_ms: duration_ms(self.started.elapsed()),
            calls,
            dropped_calls: dropped,
            host_context,
        }
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
