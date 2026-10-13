//! Ranked note search and Connections for agent runs (D4). The host supplies
//! a [`RunNoteSearch`] handle; `brn-ai` never opens the search index. The
//! context is always the run's chat, never a model argument, and the tools
//! are registered only when a handle is present: there is no substitute.
use super::*;
use serde::Serialize;

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
    fn search(
        &self,
        chat: &str,
        query: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> std::result::Result<NoteSearchPage, NoteSearchError>;
    fn connections(
        &self,
        chat: &str,
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
    /// Dense coverage: `none` until an embedding model is installed.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    pub inbound_links: usize,
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
fn host(scope: &Scope) -> AiResult<(Arc<dyn RunNoteSearch>, String)> {
    if scope.writing_snapshot.is_some() {
        return Err(rejected());
    }
    let search = scope.search.clone().ok_or_else(rejected)?;
    let store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    Ok((search, scope.chat.clone()))
}

pub(super) async fn search_notes(scope: Arc<Scope>, args: SearchNotesArgs) -> AiResult<Value> {
    let (search, chat) = host(&scope)?;
    let page = tokio::task::spawn_blocking(move || {
        search.search(&chat, &args.query, args.limit, args.cursor.as_deref())
    })
    .await
    .map_err(storage)?
    .map_err(failed)?;
    serde_json::to_value(page).map_err(storage)
}

pub(super) async fn note_connections(scope: Arc<Scope>, args: ConnectionsArgs) -> AiResult<Value> {
    let (search, chat) = host(&scope)?;
    let neighbourhood = tokio::task::spawn_blocking(move || {
        search.connections(&chat, &args.id, args.hops, args.sources)
    })
    .await
    .map_err(storage)?
    .map_err(failed)?;
    serde_json::to_value(neighbourhood).map_err(storage)
}
