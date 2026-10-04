//! Read-only classification from the documented ordinary frontmatter scalars.
use crate::{Result, invalid, note_identity::selected_fields};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoteClassification {
    pub source: bool,
    pub history: bool,
}

/// Absent fields describe current knowledge. This reads saved bytes without
/// stamping metadata, rewriting source wording or interpreting unrelated fields.
/// Archive-path policy belongs to the caller, rather than the note's text.
pub fn classify(text: &str) -> Result<NoteClassification> {
    let Some(metadata) = selected_fields(text, ["brn_kind", "brn_state"], false)? else {
        return Ok(NoteClassification::default());
    };
    let source = match metadata.values[0] {
        None | Some("knowledge") => false,
        Some("source") => true,
        Some(_) => return Err(invalid("managed brn_kind must be knowledge or source")),
    };
    let history = match metadata.values[1] {
        None | Some("current") => false,
        Some("history") => true,
        Some(_) => return Err(invalid("managed brn_state must be current or history")),
    };
    Ok(NoteClassification { source, history })
}
