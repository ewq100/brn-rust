//! Managed classification and exact proposed History state in ordinary frontmatter.
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

/// Propose the exact Current knowledge bytes with only managed History state
/// changed. Unrelated metadata, wording, BOM and existing line endings stay exact.
/// This pure transition does not approve or install the returned bytes.
pub fn to_history(text: &str) -> Result<String> {
    if text.len() > crate::MAX_NOTE_BYTES || classify(text)? != NoteClassification::default() {
        return Err(invalid(
            "History transition needs bounded Current knowledge",
        ));
    }
    let (range, addition) =
        match crate::note_identity::raw_fields(text, ["brn_kind", "brn_state"], true)? {
            Some(metadata) => match &metadata.fields[1] {
                Some(field) => {
                    // Classification already verified the scalar is exactly current.
                    // Replace its value alone, retaining quotes, whitespace and comments.
                    let leading =
                        field.value.len() - field.value.trim_start_matches([' ', '\t']).len();
                    let quoted = field.value[leading..].starts_with(['\'', '"']);
                    let start =
                        field.line.start + "brn_state:".len() + leading + usize::from(quoted);
                    (start..start + "current".len(), "history".to_owned())
                }
                None => (
                    metadata.insertion..metadata.insertion,
                    format!("brn_state: history{}", metadata.newline),
                ),
            },
            None => {
                let newline = text.find('\n').map_or("\n", |position| {
                    if text[..position].ends_with('\r') {
                        "\r\n"
                    } else {
                        "\n"
                    }
                });
                let insertion = if text.starts_with('\u{feff}') {
                    '\u{feff}'.len_utf8()
                } else {
                    0
                };
                (
                    insertion..insertion,
                    format!("---{newline}brn_state: history{newline}---{newline}"),
                )
            }
        };
    let size = text
        .len()
        .checked_sub(range.len())
        .and_then(|size| size.checked_add(addition.len()))
        .filter(|size| *size <= crate::MAX_NOTE_BYTES)
        .ok_or_else(|| invalid("History transition exceeds the 1 MiB note limit"))?;
    let mut history = String::with_capacity(size);
    history.push_str(&text[..range.start]);
    history.push_str(&addition);
    history.push_str(&text[range.end..]);
    Ok(history)
}
