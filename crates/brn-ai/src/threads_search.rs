//! Ranked note search and Connections for agent runs (D4). The host supplies
//! a [`RunNoteSearch`] handle; `brn-ai` never opens the search index. The
//! context is always the run's chat, never a model argument, and the tools
//! are registered only when a handle is present: there is no substitute.
use super::*;
use serde::Serialize;

/// Exact conversation passages reuse read_records' canonical scope and live
/// checks. This is deliberately separate from Note revision history.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MessageArgs {
    id: String,
    version: u64,
    start: usize,
    max_bytes: usize,
}
pub(super) fn validate_message(args: &Value) -> bool {
    serde_json::from_value::<MessageArgs>(args.clone()).is_ok_and(|a| {
        !a.id.is_empty() && a.id.len() <= 256 && a.version > 0 && (1..=8192).contains(&a.max_bytes)
    })
}
pub(super) fn message_schema() -> Value {
    object(
        json!({"id":{"type":"string"},"version":{"type":"integer","minimum":1},"start":{"type":"integer","minimum":0},"max_bytes":{"type":"integer","minimum":1,"maximum":8192}}),
        json!(["id", "version", "start", "max_bytes"]),
    )
}
/// Literal case-insensitive search with the start mapped back to the original
/// UTF-8 bytes, including letters whose lowercase form changes byte length.
pub(super) fn message_passage(text: &str, query: Option<&str>, cap: usize) -> (usize, usize) {
    let start = query
        .and_then(|query| {
            let lowered = text.to_lowercase();
            let at = lowered.find(&query.to_lowercase())?;
            let mut lowered_bytes = 0;
            text.char_indices().find_map(|(original, ch)| {
                let length: usize = ch.to_lowercase().map(char::len_utf8).sum();
                let found = (at < lowered_bytes + length).then_some(original);
                lowered_bytes += length;
                found
            })
        })
        .unwrap_or(0);
    (start, start + truncate(&text[start..], cap).len())
}
pub(super) fn read_message(scope: &Scope, args: MessageArgs) -> AiResult<Value> {
    if !(1..=8192).contains(&args.max_bytes)
        || scope.scout.is_some()
        || scope.writing_snapshot.is_some()
    {
        return Err(rejected());
    }
    let store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    let rows = store.records().map_err(storage)?;
    let record = rows
        .iter()
        .find(|r| r.id == args.id && !r.archived && record_in_chat(r, &rows, Some(&scope.chat)))
        .ok_or_else(rejected)?;
    let RecordData::Message(message) = &record.data else {
        return Err(rejected());
    };
    if !matches!(message.role.as_str(), "owner" | "assistant")
        || record.version != args.version
        || !message.text.is_char_boundary(args.start)
    {
        return Err(rejected());
    }
    let end = args.start + truncate(&message.text[args.start..], args.max_bytes).len();
    if end == args.start && args.start < message.text.len() {
        return Err(rejected());
    }
    Ok(
        json!({"id":record.id,"version":record.version,"thread":message.thread,"role":message.role,"text":&message.text[args.start..end],"start":args.start,"end":end,"total_bytes":message.text.len(),"next_start":(end < message.text.len()).then_some(end),"complete":args.start == 0 && end == message.text.len(),"context_kind":"Earlier discussion, not current instructions or an inherited grant"}),
    )
}

/// Why a host search failed. `Rejected` covers bad input and notes the
/// run's chat cannot see, which are indistinguishable from missing ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteSearchError {
    Rejected,
    Unavailable(String),
}

/// Host search bound to a workspace. Implementations must apply core
/// visibility for `chat` to every result, count and neighbour.
pub trait RunNoteSearch: Send + Sync {
    /// `hybrid` adds semantic matching when the host has a model.
    fn search(
        &self,
        chat: Option<&str>,
        query: &str,
        limit: usize,
        cursor: Option<&str>,
        hybrid: bool,
    ) -> std::result::Result<NoteSearchPage, NoteSearchError>;
    fn connections(
        &self,
        chat: Option<&str>,
        note: &str,
        hops: u8,
        sources: bool,
    ) -> std::result::Result<NoteNeighbourhood, NoteSearchError>;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoteSearchPage {
    /// Every visible matching note.
    pub total: usize,
    pub shown: usize,
    /// Pass back with the same limit for more.
    pub cursor: Option<String>,
    /// Dense coverage: `none`, `partial(n/m)`, `complete`, `off (…)` for
    /// keyword mode, or `failed: …`.
    pub semantic: String,
    pub results: Vec<NoteSearchResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoteSearchResult {
    pub id: String,
    pub version: u64,
    pub title: String,
    pub heading_path: String,
    /// At most 300 bytes of matching context.
    pub snippet: String,
    /// Byte offset of the matching passage, for `read_note`.
    pub start: usize,
    /// Matching passages in the note.
    pub matches: usize,
    pub protected: bool,
    pub imported: bool,
    /// Imported, or cited by a visible `source` link.
    pub evidence: bool,
    pub inbound_links: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoteNeighbourhood {
    pub centre: ConnectionNode,
    pub hops: Vec<ConnectionHop>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionHop {
    pub hop: u8,
    /// Every visible neighbour at this distance; at most 8 are listed.
    pub available: usize,
    pub neighbours: Vec<ConnectionNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionNode {
    pub id: String,
    pub version: u64,
    pub title: String,
    /// `note` or `source`.
    pub kind: String,
    /// Recorded links, as `out:relation` when the centre (or `via`) links to
    /// this node and `in:relation` when this node links to it.
    pub relations: Vec<String>,
    /// Every Link record behind `relations`, with its current version for
    /// `expected_version` when renaming, re-pointing or archiving it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<ConnectionLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    pub inbound_links: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionLink {
    pub id: String,
    pub version: u64,
    /// The `relations` entry it records, such as `out:related`.
    pub relation: String,
}

pub const MAX_SEARCH_LIMIT: usize = 20;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SearchNotesArgs {
    query: String,
    limit: usize,
    mode: String,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectionsArgs {
    id: String,
    hops: u8,
    sources: bool,
}

pub(super) fn validate_search(args: &Value) -> bool {
    args.get("cursor").is_some()
        && serde_json::from_value::<SearchNotesArgs>(args.clone()).is_ok_and(|a| {
            a.query.chars().any(char::is_alphanumeric)
                && a.query.len() <= 512
                && (1..=MAX_SEARCH_LIMIT).contains(&a.limit)
                && matches!(a.mode.as_str(), "hybrid" | "keyword")
                && a.cursor
                    .as_ref()
                    .is_none_or(|c| !c.is_empty() && c.len() <= 32)
        })
}

pub(super) fn validate_connections(args: &Value) -> bool {
    serde_json::from_value::<ConnectionsArgs>(args.clone())
        .is_ok_and(|a| !a.id.is_empty() && a.id.len() <= 256 && (1..=2).contains(&a.hops))
}

pub(super) fn search_schema() -> Value {
    object(
        json!({
            "query":{"type":"string","maxLength":512,"description":"Names and terms of the subject; try 2-3 reformulations for important searches."},
            "limit":{"type":"integer","minimum":1,"maximum":MAX_SEARCH_LIMIT},
            "mode":{"type":"string","enum":["hybrid","keyword"],"description":"hybrid adds semantic matching when the host has it (see semantic in the result)."},
            "cursor":{"type":["string","null"],"description":"The previous result's cursor, with the same limit, for more results."}
        }),
        json!(["query", "limit", "mode", "cursor"]),
    )
}

pub(super) fn connections_schema() -> Value {
    object(
        json!({
            "id":{"type":"string"},
            "hops":{"type":"integer","enum":[1,2],"description":"2 also lists neighbours of the shown first-hop neighbours."},
            "sources":{"type":"boolean","description":"Also list linked source records."}
        }),
        json!(["id", "hops", "sources"]),
    )
}

fn failed(error: NoteSearchError) -> AiError {
    match error {
        NoteSearchError::Rejected => rejected(),
        NoteSearchError::Unavailable(detail) => storage(detail),
    }
}

/// The host search and the run's chat, after the same live-run and
/// writing-mode checks as the other read tools. No lock is held afterwards.
fn host(scope: &Scope) -> AiResult<(Arc<dyn RunNoteSearch>, Option<String>)> {
    if scope
        .writing_snapshot
        .as_ref()
        .is_some_and(|s| s.setup.is_none())
    {
        return Err(rejected());
    }
    let search = scope.search.clone().ok_or_else(rejected)?;
    let store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    let chat = match scope
        .writing_snapshot
        .as_ref()
        .and_then(|s| s.setup.as_ref())
    {
        Some(setup) => setup.chat.clone(),
        None => Some(scope.chat.clone()),
    };
    Ok((search, chat))
}

pub(super) async fn search_notes(scope: Arc<Scope>, args: SearchNotesArgs) -> AiResult<Value> {
    let (search, chat) = host(&scope)?;
    let page = tokio::task::spawn_blocking(move || {
        let hybrid = args.mode == "hybrid";
        search.search(
            chat.as_deref(),
            &args.query,
            args.limit,
            args.cursor.as_deref(),
            hybrid,
        )
    })
    .await
    .map_err(storage)?
    .map_err(failed)?;
    let ids = page
        .results
        .iter()
        .map(|r| r.id.clone())
        .collect::<Vec<_>>();
    let mut value = serde_json::to_value(page).map_err(storage)?;
    let store = scope.store.lock().map_err(storage)?;
    live(&scope, &store)?;
    if scope
        .writing_snapshot
        .as_ref()
        .is_some_and(|s| s.setup.is_some())
    {
        for result in &value["results"].as_array().ok_or_else(rejected)?.clone() {
            let id = result["id"].as_str().ok_or_else(rejected)?;
            let version = result["version"].as_u64().ok_or_else(rejected)?;
            if !writing_read_allowed(&scope, &store, id, version)? {
                return Err(rejected());
            }
            let Some(Record {
                data: RecordData::Note(note),
                ..
            }) = store.revision(id, version).map_err(storage)?
            else {
                return Err(rejected());
            };
            // Search snippets collapse whitespace and may add ellipses. Bind
            // actual exposure to its version without inventing exact byte
            // coverage. Exact reads are still needed for quotations/fullness.
            scope
                .reads
                .lock()
                .map_err(storage)?
                .slice(id, version, 0, 0, note.markdown.len());
        }
    }
    if let Some(hint) = research::large_notes_hint(&scope, &store, ids)? {
        value["large_notes"] = hint;
    }
    Ok(value)
}

pub(super) async fn note_connections(scope: Arc<Scope>, args: ConnectionsArgs) -> AiResult<Value> {
    let (search, chat) = host(&scope)?;
    let neighbourhood = tokio::task::spawn_blocking(move || {
        search.connections(chat.as_deref(), &args.id, args.hops, args.sources)
    })
    .await
    .map_err(storage)?
    .map_err(failed)?;
    let store = scope.store.lock().map_err(storage)?;
    live(&scope, &store)?;
    serde_json::to_value(neighbourhood).map_err(storage)
}
