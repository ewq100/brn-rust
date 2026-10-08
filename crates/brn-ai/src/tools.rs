use crate::{AiError, AiErrorKind, AiResult};
use rig::tool::{Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub const READ_NOTE_BYTES: usize = 50_000;
pub const READ_ACTION_BYTES: usize = 1024 * 1024;

pub fn capped_text(text: &str) -> (&str, bool) {
    let mut end = text.len().min(READ_NOTE_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], end < text.len())
}

/// Retained conflict knowledge for the exact saved-note proof in this result.
/// Unknown is never zero; a checked lookup can report a count without a winner.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConflictKnowledge {
    #[default]
    Unknown,
    Known {
        open_count: usize,
    },
}

/// Workflow-observed classification of the complete saved bytes, before text caps.
/// Source/History may overlap. Neither classification nor conflict count proves truth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteFacts {
    /// Canonical managed UUID, when present; unmanaged notes remain supported.
    pub note_id: Option<String>,
    pub sha256: [u8; 32],
    pub source: bool,
    pub history: bool,
    pub conflicts: ConflictKnowledge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Passage {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    pub facts: NoteFacts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSearch {
    pub hits: Vec<Passage>,
    pub keyword_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolNote {
    pub path: String,
    pub text: String,
    pub truncated: bool,
    pub facts: NoteFacts,
}

/// Exact half-open UTF-8 byte interval bound to a complete saved-note hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteRangeRequest {
    pub path: String,
    #[serde(default)]
    pub scope: ReadScope,
    pub expected_sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
}

impl NoteRangeRequest {
    pub fn validate(&self) -> AiResult<()> {
        if !(1..=512).contains(&self.path.len())
            || self
                .end_byte
                .checked_sub(self.start_byte)
                .is_none_or(|len| len > READ_NOTE_BYTES)
        {
            return Err(rejected());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolNoteRange {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub total_bytes: usize,
    pub text: String,
    pub facts: NoteFacts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteEntry {
    pub path: String,
    pub title: String,
    pub facts: NoteFacts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotePage {
    pub notes: Vec<NoteEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadScope {
    #[default]
    Current,
    Source,
    History,
    All,
}

pub trait ReadTools: Send + Sync {
    fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch>;
    fn read_note(&self, path: &str) -> AiResult<ToolNote>;
    fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage>;

    /// Read an exact interval from the supplied full saved-note content proof.
    /// Legacy implementations refuse rather than returning a capped prefix.
    fn read_note_range(&self, _request: &NoteRangeRequest) -> AiResult<ToolNoteRange> {
        Err(rejected())
    }

    /// Read a complete approved Action through the application boundary.
    fn read_action(&self, _id: &str) -> AiResult<Value> {
        Err(rejected())
    }
    /// Page complete approved Actions; the application owns state/cursor semantics.
    fn list_actions(
        &self,
        _state: Option<&str>,
        _limit: usize,
        _cursor: Option<&str>,
    ) -> AiResult<Value> {
        Err(rejected())
    }

    /// Read complete conflict evidence; workflow owns path/scope/cursor semantics.
    fn read_conflicts(
        &self,
        _path: &str,
        _scope: ReadScope,
        _limit: usize,
        _cursor: Option<&str>,
    ) -> AiResult<Value> {
        Err(rejected())
    }

    fn search_notes_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: ReadScope,
    ) -> AiResult<ToolSearch> {
        if scope != ReadScope::Current {
            return Err(rejected());
        }
        self.search_notes(query, limit)
    }
    fn read_note_scoped(&self, path: &str, scope: ReadScope) -> AiResult<ToolNote> {
        if scope != ReadScope::Current {
            return Err(rejected());
        }
        self.read_note(path)
    }
    fn list_notes_scoped(
        &self,
        folder: Option<&str>,
        cursor: Option<&str>,
        scope: ReadScope,
    ) -> AiResult<NotePage> {
        if scope != ReadScope::Current {
            return Err(rejected());
        }
        self.list_notes(folder, cursor)
    }
}

pub(crate) struct ToolRounds {
    used: usize,
    limit: usize,
}

impl Default for ToolRounds {
    fn default() -> Self {
        Self { used: 0, limit: 8 }
    }
}

impl ToolRounds {
    pub(crate) fn new(limit: u16) -> AiResult<Self> {
        if !(1..=32).contains(&limit) {
            return Err(rejected());
        }
        Ok(Self {
            used: 0,
            limit: usize::from(limit),
        })
    }
    pub(crate) fn used(&self) -> u16 {
        self.used as u16
    }

    pub(crate) fn admit(&mut self, contains_tools: bool) -> AiResult<()> {
        if !contains_tools {
            return Ok(());
        }
        if self.used == self.limit {
            return Err(AiError::new(AiErrorKind::ToolLimitReached));
        }
        self.used += 1;
        Ok(())
    }
}

fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}

fn validate_search(query: &str, limit: usize) -> AiResult<()> {
    if !(1..=512).contains(&query.len()) || !(1..=10).contains(&limit) {
        return Err(rejected());
    }
    Ok(())
}

fn validate_page(page: NotePage) -> AiResult<NotePage> {
    if page.notes.len() > 200 {
        return Err(rejected());
    }
    Ok(page)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchArgs {
    query: String,
    limit: usize,
    #[serde(default)]
    scope: ReadScope,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadArgs {
    path: String,
    #[serde(default)]
    scope: ReadScope,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListArgs {
    folder: Option<String>,
    cursor: Option<String>,
    #[serde(default)]
    scope: ReadScope,
}

#[derive(Serialize)]
pub(crate) struct ScopedSearch {
    scope: ReadScope,
    #[serde(flatten)]
    result: ToolSearch,
}
#[derive(Serialize)]
pub(crate) struct ScopedNote {
    scope: ReadScope,
    #[serde(flatten)]
    result: ToolNote,
}
#[derive(Serialize)]
pub(crate) struct ScopedNoteRange {
    scope: ReadScope,
    #[serde(flatten)]
    result: ToolNoteRange,
}
#[derive(Serialize)]
pub(crate) struct ScopedPage {
    scope: ReadScope,
    #[serde(flatten)]
    result: NotePage,
}

pub(crate) struct SearchNotes(pub Arc<dyn ReadTools>);
pub(crate) struct ReadNote(pub Arc<dyn ReadTools>);
pub(crate) struct ReadNoteRange(pub Arc<dyn ReadTools>);
pub(crate) struct ListNotes(pub Arc<dyn ReadTools>);
pub(crate) struct ReadAction(pub Arc<dyn ReadTools>);
pub(crate) struct ListActions(pub Arc<dyn ReadTools>);
pub(crate) struct ReadConflicts(pub Arc<dyn ReadTools>);

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadConflictsArgs {
    path: String,
    #[serde(default)]
    scope: ReadScope,
    #[serde(default = "conflict_limit")]
    limit: usize,
    cursor: Option<String>,
}
fn conflict_limit() -> usize {
    10
}
impl Tool for ReadConflicts {
    const NAME: &'static str = "read_conflicts";
    type Args = ReadConflictsArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Look up unresolved conflicts for a relevant saved note before claiming current facts. Path selects a saved note; omitted scope means Current. Source, History and All explicitly select evidence. Limit defaults to 10 (1–100); pass the opaque next_cursor unchanged for another page. Results retain exact note_id/source proof and facts with known open_count across pages, including stale findings. Known zero proves no retained open findings, never consistency or a winner. Whole replies over 1 MiB are refused, never clipped. Incomplete pages or failed lookup never mean no conflict. Disclose unresolved or stale evidence and do not choose a winner.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "path":{"type":"string","minLength":1,"maxLength":512},
            "scope":{"type":"string","enum":["current","source","history","all"]},
            "limit":{"type":"integer","minimum":1,"maximum":100},
            "cursor":{"type":["string","null"],"maxLength":8192}
        },"required":["path"]})
    }
    async fn call(&self, _: &mut ToolContext, args: ReadConflictsArgs) -> AiResult<Value> {
        if !(1..=512).contains(&args.path.len())
            || !(1..=100).contains(&args.limit)
            || args
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.len() > 8192)
        {
            return Err(rejected());
        }
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            complete_action_reply(tools.read_conflicts(
                &args.path,
                args.scope,
                args.limit,
                args.cursor.as_deref(),
            )?)
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadActionArgs {
    id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListActionsArgs {
    state: Option<String>,
    #[serde(default = "action_limit")]
    limit: usize,
    cursor: Option<String>,
}
fn action_limit() -> usize {
    20
}

fn complete_action_reply(reply: Value) -> AiResult<Value> {
    let bytes = serde_json::to_vec(&reply).map_err(|_| rejected())?;
    if bytes.len() > READ_ACTION_BYTES {
        return Err(rejected());
    }
    Ok(reply)
}

impl Tool for ReadAction {
    const NAME: &'static str = "read_action";
    type Args = ReadActionArgs;
    type Output = Value;
    type Error = AiError;

    fn description(&self) -> String {
        "Read one complete approved Action by UUID, including current fields, state, revision, immutable approved origin and checked_ref binding the complete record. Use this checked_ref as a propose_actions Replace target; do not construct a before record. This is read-only; it cannot approve or complete work. Replies over 1 MiB are refused, never truncated.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,
            "properties":{"id":{"type":"string","minLength":1,"maxLength":64}},
            "required":["id"]})
    }
    async fn call(&self, _: &mut ToolContext, args: ReadActionArgs) -> AiResult<Value> {
        if !(1..=64).contains(&args.id.len()) {
            return Err(rejected());
        }
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || complete_action_reply(tools.read_action(&args.id)?))
            .await
            .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for ListActions {
    const NAME: &'static str = "list_actions";
    type Args = ListActionsArgs;
    type Output = Value;
    type Error = AiError;

    fn description(&self) -> String {
        "Page complete approved Actions in stable creation order. Omitted state includes all states; each record labels its state. Limit defaults to 20 (1–20). Pass next_cursor unchanged for another page. Replies over 1 MiB are refused; retry with a smaller limit. Read-only.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "state":{"type":["string","null"],"enum":["open","waiting","blocked","completed",null]},
            "limit":{"type":"integer","minimum":1,"maximum":20},
            "cursor":{"type":["string","null"],"maxLength":256}},"required":[]})
    }
    async fn call(&self, _: &mut ToolContext, args: ListActionsArgs) -> AiResult<Value> {
        if !(1..=20).contains(&args.limit)
            || args.state.as_ref().is_some_and(|s| s.len() > 16)
            || args.cursor.as_ref().is_some_and(|s| s.len() > 256)
        {
            return Err(rejected());
        }
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            complete_action_reply(tools.list_actions(
                args.state.as_deref(),
                args.limit,
                args.cursor.as_deref(),
            )?)
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for SearchNotes {
    const NAME: &'static str = "search_notes";
    type Args = SearchArgs;
    type Output = ScopedSearch;
    type Error = AiError;

    fn description(&self) -> String {
        "Search current knowledge by default. Select source for original evidence, history for historical notes, or all when explicitly relevant. Results label requested scope separately from each hit’s facts for the complete saved bytes: optional managed note_id, sha256, independent source/history and unknown conflict status. Unknown is not zero; classification is evidence, not truth. keyword_only indicates keyword rather than semantic results.".into()
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "query": {"type": "string", "minLength": 1, "maxLength": 512,
                    "description": "Query of 1-512 UTF-8 bytes"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 10},
                "scope": {"type": "string", "enum": ["current", "source", "history", "all"],
                    "description": "Omit for current knowledge; choose an explicit evidence scope when relevant."}
            },
            "required": ["query", "limit"]
        })
    }
    async fn call(&self, _: &mut ToolContext, args: SearchArgs) -> AiResult<ScopedSearch> {
        validate_search(&args.query, args.limit)?;
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let result = tools.search_notes_scoped(&args.query, args.limit, args.scope)?;
            if result.hits.len() > args.limit {
                return Err(rejected());
            }
            Ok(ScopedSearch {
                scope: args.scope,
                result,
            })
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for ReadNote {
    const NAME: &'static str = "read_note";
    type Args = ReadArgs;
    type Output = ScopedNote;
    type Error = AiError;

    fn description(&self) -> String {
        "Read fresh saved text. Omitted scope means current knowledge; source/history/all permit explicit original or historical evidence. Results label requested scope and facts for complete saved bytes: optional managed note_id, full sha256, independent source/history and unknown conflict status. Unknown is not zero; classification is evidence, not truth. Text preserves bytes, capped at 50000 UTF-8 bytes with a truncation flag; facts.sha256 still covers the full note."
            .into()
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "scope": {"type": "string", "enum": ["current", "source", "history", "all"]}
            },
            "required": ["path"]
        })
    }
    async fn call(&self, _: &mut ToolContext, args: ReadArgs) -> AiResult<ScopedNote> {
        if args.path.is_empty() {
            return Err(rejected());
        }
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let mut note = tools.read_note_scoped(&args.path, args.scope)?;
            let (prefix, truncated) = capped_text(&note.text);
            if truncated {
                note.text = prefix.to_owned();
                note.truncated = true;
            }
            Ok(ScopedNote {
                scope: args.scope,
                result: note,
            })
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for ReadNoteRange {
    const NAME: &'static str = "read_note_range";
    type Args = NoteRangeRequest;
    type Output = ScopedNoteRange;
    type Error = AiError;

    fn description(&self) -> String {
        "Read an exact saved-note interval using expected_sha256 copied from fresh note/search/list facts. Offsets are zero-based half-open UTF-8 bytes of the complete saved file, including BOM, frontmatter and CRLF. Both offsets must be character boundaries; at most 50000 bytes per interval, with no clipping or normalization. Empty intervals (including 0..0) return total_bytes for discovering the length. A changed full-note hash refuses the read even when the requested interval is unchanged. Omitted scope means Current; Source/History/All select explicit evidence. Facts describe the complete checked bytes; unknown conflicts never mean zero. This checked content snapshot grants no approval or mutation authority.".into()
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "path":{"type":"string","minLength":1,"maxLength":512},
            "scope":{"type":"string","enum":["current","source","history","all"]},
            "expected_sha256":{"type":"array","minItems":32,"maxItems":32,
                "items":{"type":"integer","minimum":0,"maximum":255}},
            "start_byte":{"type":"integer","minimum":0},
            "end_byte":{"type":"integer","minimum":0}
        },"required":["path","expected_sha256","start_byte","end_byte"]})
    }

    async fn call(
        &self,
        _: &mut ToolContext,
        request: NoteRangeRequest,
    ) -> AiResult<ScopedNoteRange> {
        request.validate()?;
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let result = tools.read_note_range(&request)?;
            if result.path != request.path
                || result.start_byte != request.start_byte
                || result.end_byte != request.end_byte
                || result.end_byte > result.total_bytes
                || result.text.len() != request.end_byte - request.start_byte
                || result.facts.sha256 != request.expected_sha256
            {
                return Err(rejected());
            }
            Ok(ScopedNoteRange {
                scope: request.scope,
                result,
            })
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for ListNotes {
    const NAME: &'static str = "list_notes";
    type Args = ListArgs;
    type Output = ScopedPage;
    type Error = AiError;

    fn description(&self) -> String {
        "List at most 200 saved notes per page, optionally within a folder or after a cursor. Omitted scope means current knowledge; select source/history/all for explicit evidence. Results label requested scope separately from each note’s facts for complete saved bytes: optional managed note_id, sha256, independent source/history and unknown conflict status. Unknown is not zero; classification is evidence, not truth."
            .into()
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "folder": {"type": ["string", "null"]},
                "cursor": {"type": ["string", "null"]},
                "scope": {"type": "string", "enum": ["current", "source", "history", "all"]}
            },
            "required": []
        })
    }
    async fn call(&self, _: &mut ToolContext, args: ListArgs) -> AiResult<ScopedPage> {
        let tools = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let result = validate_page(tools.list_notes_scoped(
                args.folder.as_deref(),
                args.cursor.as_deref(),
                args.scope,
            )?)?;
            Ok(ScopedPage {
                scope: args.scope,
                result,
            })
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AiErrorKind;

    #[test]
    fn eighth_tool_round_is_allowed_and_ninth_is_refused() {
        let mut budget = ToolRounds::default();
        for _ in 0..8 {
            assert!(budget.admit(true).is_ok());
        }
        assert_eq!(
            budget.admit(true).unwrap_err().kind,
            AiErrorKind::ToolLimitReached
        );
        assert!(budget.admit(false).is_ok());
    }

    #[test]
    fn note_cap_preserves_utf8_and_exact_prefix() {
        let text = format!("{}\u{00e9}", "x".repeat(49_999));
        let (prefix, truncated) = capped_text(&text);
        assert!(truncated);
        assert_eq!(prefix.len(), 49_999);
        assert_eq!(prefix, &text[..49_999]);
        assert_eq!(capped_text("\u{feff} a\r\n"), ("\u{feff} a\r\n", false));
        assert_eq!(capped_text(""), ("", false));
        let exact = "x".repeat(READ_NOTE_BYTES);
        assert_eq!(capped_text(&exact), (exact.as_str(), false));
        let four_byte = format!("{}🦀", "x".repeat(49_997));
        assert_eq!(capped_text(&four_byte), (&four_byte[..49_997], true));
    }

    #[test]
    fn query_and_limit_validation_uses_bytes() {
        assert!(validate_search("", 1).is_err());
        assert!(validate_search(&"é".repeat(257), 1).is_err());
        assert!(validate_search(&"é".repeat(256), 10).is_ok());
        assert!(validate_search("x", 0).is_err());
        assert!(validate_search("x", 11).is_err());
    }

    #[test]
    fn note_results_require_explicit_conflict_knowledge_without_unknown_to_zero() {
        let note = json!({"path":"a.md","text":"exact","truncated":false});
        assert!(serde_json::from_value::<ToolNote>(note.clone()).is_err());
        let mut facts = json!({"note_id":null,"sha256":([17; 32]),"source":true,"history":true,
            "conflicts":{"status":"unknown"}});
        let mut note = note;
        note["facts"] = facts.clone();
        let unknown: ToolNote = serde_json::from_value(note.clone()).unwrap();
        assert_eq!(unknown.facts.conflicts, ConflictKnowledge::Unknown);
        assert!(unknown.facts.source && unknown.facts.history);
        facts["conflicts"] = json!({"status":"known","open_count":0});
        note["facts"] = facts;
        let known: ToolNote = serde_json::from_value(note.clone()).unwrap();
        assert_eq!(
            known.facts.conflicts,
            ConflictKnowledge::Known { open_count: 0 }
        );
        assert_ne!(known.facts, unknown.facts);
        note["facts"].as_object_mut().unwrap().remove("conflicts");
        assert!(serde_json::from_value::<ToolNote>(note).is_err());
        assert!(serde_json::from_value::<ConflictKnowledge>(json!({"status":"known"})).is_err());
        // Serde's tagged unit variant ignores payload fields. They cannot
        // promote uninspected evidence to a checked zero count.
        assert_eq!(
            serde_json::from_value::<ConflictKnowledge>(json!({"status":"unknown","open_count":0}))
                .unwrap(),
            ConflictKnowledge::Unknown
        );
    }

    #[test]
    fn oversized_page_is_rejected_not_silently_changed() {
        let page = NotePage {
            notes: vec![
                NoteEntry {
                    path: "a.md".into(),
                    title: "a".into(),
                    facts: NoteFacts {
                        note_id: None,
                        sha256: [17; 32],
                        source: false,
                        history: false,
                        conflicts: ConflictKnowledge::Unknown,
                    },
                };
                201
            ],
            next_cursor: None,
        };
        assert_eq!(
            validate_page(page).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
    }
}
