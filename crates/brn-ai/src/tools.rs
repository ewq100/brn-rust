use crate::{AiError, AiErrorKind, AiResult};
use rig::tool::{Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub const READ_NOTE_BYTES: usize = 50_000;

pub fn capped_text(text: &str) -> (&str, bool) {
    let mut end = text.len().min(READ_NOTE_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], end < text.len())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Passage {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteEntry {
    pub path: String,
    pub title: String,
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

#[derive(Default)]
pub(crate) struct ToolRounds {
    used: usize,
}

impl ToolRounds {
    pub(crate) fn admit(&mut self, contains_tools: bool) -> AiResult<()> {
        if !contains_tools {
            return Ok(());
        }
        if self.used == 8 {
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
pub(crate) struct ScopedPage {
    scope: ReadScope,
    #[serde(flatten)]
    result: NotePage,
}

pub(crate) struct SearchNotes(pub Arc<dyn ReadTools>);
pub(crate) struct ReadNote(pub Arc<dyn ReadTools>);
pub(crate) struct ListNotes(pub Arc<dyn ReadTools>);

impl Tool for SearchNotes {
    const NAME: &'static str = "search_notes";
    type Args = SearchArgs;
    type Output = ScopedSearch;
    type Error = AiError;

    fn description(&self) -> String {
        "Search current knowledge by default. Select source for original evidence, history for historical notes, or all when explicitly relevant. Results label scope; keyword_only indicates keyword rather than semantic results.".into()
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
        "Read fresh saved text. Omitted scope means current knowledge; source/history/all permit explicit original or historical evidence. Results label scope and preserve bytes, capped at 50000 UTF-8 bytes with a truncation flag."
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

impl Tool for ListNotes {
    const NAME: &'static str = "list_notes";
    type Args = ListArgs;
    type Output = ScopedPage;
    type Error = AiError;

    fn description(&self) -> String {
        "List at most 200 saved notes per page, optionally within a folder or after a cursor. Omitted scope means current knowledge; select source/history/all for explicit evidence. Results label scope."
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
    fn oversized_page_is_rejected_not_silently_changed() {
        let page = NotePage {
            notes: vec![
                NoteEntry {
                    path: "a.md".into(),
                    title: "a".into()
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
