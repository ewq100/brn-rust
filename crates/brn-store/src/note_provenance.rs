//! Durable exact vault citations in one ordinary Markdown frontmatter field.
//! These pure helpers do not resolve sources, approve changes or write files.
use crate::{
    MAX_NOTE_BYTES, Result, invalid,
    note_identity::{has_field, raw_fields},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_CITATIONS: usize = 32;
pub const MAX_QUOTE_BYTES: usize = 16 * 1024;
const KEY: &str = "brn_provenance";
const BOM: &str = "\u{feff}";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultCitation {
    pub note_id: Uuid,
    pub sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}

impl VaultCitation {
    /// Validate the bounded citation shape. The workflow separately checks this
    /// range and exact quote against the uniquely resolved saved source bytes.
    pub fn validate(&self) -> Result<()> {
        if self.note_id.is_nil()
            || self.quote.is_empty()
            || self.quote.len() > MAX_QUOTE_BYTES
            || self.start_byte >= self.end_byte
            || self.end_byte > MAX_NOTE_BYTES
            || self.end_byte - self.start_byte != self.quote.len()
        {
            return Err(invalid(
                "vault citation needs a nonnil identity and bounded exact byte range/quote",
            ));
        }
        Ok(())
    }
}

pub fn validate(citations: &[VaultCitation]) -> Result<()> {
    if citations.len() > MAX_CITATIONS {
        return Err(invalid("note provenance exceeds the 32 citation limit"));
    }
    for (index, citation) in citations.iter().enumerate() {
        citation.validate()?;
        if citations[..index].contains(citation) {
            return Err(invalid(
                "note provenance contains an exact duplicate citation",
            ));
        }
    }
    Ok(())
}

/// Read only the optional managed root field. Its value is a strict single-line
/// JSON array; absent fields do not reinterpret unrelated legacy YAML or body text.
pub fn read(text: &str) -> Result<Vec<VaultCitation>> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(invalid("note provenance exceeds the 1 MiB note limit"));
    }
    if !has_field(text, KEY) {
        return Ok(Vec::new());
    }
    let metadata = raw_fields(text, [KEY], false)?
        .ok_or_else(|| invalid("managed note provenance needs complete frontmatter"))?;
    let Some(field) = &metadata.fields[0] else {
        return Err(invalid(
            "managed note provenance needs an ordinary root field",
        ));
    };
    let citations: Vec<VaultCitation> = serde_json::from_str(field.value).map_err(|_| {
        invalid("managed brn_provenance must contain a strict single-line citation JSON array")
    })?;
    validate(&citations)?;
    Ok(citations)
}

/// Return complete proposed Markdown bytes, preserving all unrelated bytes.
/// Existing invalid provenance is refused, never repaired or silently removed.
pub fn write(text: &str, citations: &[VaultCitation]) -> Result<String> {
    validate(citations)?;
    let existing = read(text)?;
    if existing == citations {
        return Ok(text.to_owned());
    }
    let json = serde_json::to_string(citations)
        .map_err(|_| invalid("note provenance could not be encoded"))?;
    let (range, addition) = if let Some(metadata) = raw_fields(text, [KEY], true)? {
        if let Some(field) = &metadata.fields[0] {
            (
                field.line.clone(),
                format!("{KEY}: {json}{}", field.newline),
            )
        } else {
            (
                metadata.insertion..metadata.insertion,
                format!("{KEY}: {json}{}", metadata.newline),
            )
        }
    } else {
        let newline = text.find('\n').map_or("\n", |position| {
            if text[..position].ends_with('\r') {
                "\r\n"
            } else {
                "\n"
            }
        });
        let insertion = if text.starts_with(BOM) { BOM.len() } else { 0 };
        (
            insertion..insertion,
            format!("---{newline}{KEY}: {json}{newline}---{newline}"),
        )
    };
    let size = text
        .len()
        .checked_sub(range.len())
        .and_then(|size| size.checked_add(addition.len()))
        .filter(|size| *size <= MAX_NOTE_BYTES)
        .ok_or_else(|| invalid("note provenance replacement exceeds the 1 MiB note limit"))?;
    let mut proposed = String::with_capacity(size);
    proposed.push_str(&text[..range.start]);
    proposed.push_str(&addition);
    proposed.push_str(&text[range.end..]);
    Ok(proposed)
}
