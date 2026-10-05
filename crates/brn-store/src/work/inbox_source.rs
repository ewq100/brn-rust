//! Immutable conversion proof carried by the existing proposal/recovery records.
use super::{inbox::InboxItem, inbox_processing::InboxConversionFormat};
use crate::{MAX_NOTE_BYTES, Result, hash, invalid};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceBinding {
    pub batch_id: Uuid,
    pub index: usize,
    pub original: InboxItem,
    pub format: InboxConversionFormat,
    pub byte_len: u64,
    pub sha256: [u8; 32],
    pub note_id: Uuid,
}

/// Durable semantic provenance contains no machine-specific file paths/inodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceProvenance {
    pub item_id: Uuid,
    pub kind: super::inbox::InboxKind,
    pub title: String,
    pub original_name: Option<String>,
    pub received_at_ms: u64,
    pub original_byte_len: u64,
    pub original_sha256: [u8; 32],
    pub format: InboxConversionFormat,
}

/// Read semantic provenance without accessing the catalog or original files.
pub fn read_provenance(text: &str) -> Result<Option<InboxSourceProvenance>> {
    const KEY: &str = "brn_inbox_source";
    if text.len() > MAX_NOTE_BYTES {
        return Err(invalid("Inbox source provenance exceeds the note limit"));
    }
    if !crate::note_identity::has_field(text, KEY) {
        return Ok(None);
    }
    let fields = crate::note_identity::raw_fields(text, [KEY], false)?
        .ok_or_else(|| invalid("Inbox provenance needs complete frontmatter"))?;
    let field = fields.fields[0]
        .as_ref()
        .ok_or_else(|| invalid("Inbox provenance needs an ordinary root field"))?;
    let value: InboxSourceProvenance = serde_json::from_str(field.value)
        .map_err(|_| invalid("Inbox provenance needs strict single-line JSON"))?;
    if value.item_id.is_nil()
        || value.received_at_ms > i64::MAX as u64
        || value.original_byte_len > MAX_NOTE_BYTES as u64
        || std::iter::once(&value.title)
            .chain(value.original_name.iter())
            .any(|s| s.trim().is_empty() || s.len() > 512 || s.chars().any(char::is_control))
        || (value.kind == super::inbox::InboxKind::Markdown)
            != (value.format == InboxConversionFormat::VerbatimMarkdownV1)
        || value.original_byte_len == 0 && value.original_sha256 != hash(&[])
    {
        return Err(invalid("Inbox provenance has invalid original metadata"));
    }
    Ok(Some(value))
}

impl InboxSourceBinding {
    pub fn validate(&self) -> Result<()> {
        self.original.validate()?;
        if self.batch_id.is_nil()
            || self.index >= super::inbox_processing::MAX_PROCESS_BATCH
            || self.note_id.is_nil()
            || self.byte_len > MAX_NOTE_BYTES as u64
        {
            return Err(invalid(
                "Inbox source needs a nonnil note UUID and bounded conversion",
            ));
        }
        match self.original.capture.kind {
            super::inbox::InboxKind::Markdown
                if self.format == InboxConversionFormat::VerbatimMarkdownV1
                    && self.byte_len == self.original.capture.copy.byte_len
                    && self.sha256 == self.original.capture.copy.sha256 => {}
            super::inbox::InboxKind::Markdown => {
                return Err(invalid(
                    "Markdown source must retain its exact original conversion proof",
                ));
            }
            _ if self.format == InboxConversionFormat::LiteralTextV1
                && self.byte_len >= self.original.capture.copy.byte_len.saturating_add(12) => {}
            _ => return Err(invalid("Text source requires faithful literal conversion")),
        }
        Ok(())
    }
    pub fn provenance(&self) -> InboxSourceProvenance {
        let capture = &self.original.capture;
        InboxSourceProvenance {
            item_id: capture.id,
            kind: capture.kind,
            title: capture.title.clone(),
            original_name: capture.original_name.clone(),
            received_at_ms: self.original.received_at_ms,
            original_byte_len: capture.copy.byte_len,
            original_sha256: capture.copy.sha256,
            format: self.format,
        }
    }
    fn header(&self) -> Result<String> {
        self.validate()?;
        let provenance = serde_json::to_string(&self.provenance())
            .map_err(|_| invalid("could not encode Inbox source provenance"))?;
        Ok(format!(
            "---\nbrn_id: {}\nbrn_kind: source\nbrn_state: current\nbrn_inbox_source: {provenance}\n---\n",
            self.note_id
        ))
    }
    /// Imported frontmatter is body evidence, never adopted as managed authority.
    pub fn markdown(&self, converted: &str) -> Result<String> {
        let header = self.header()?;
        if converted.len() as u64 != self.byte_len
            || hash(converted.as_bytes()) != self.sha256
            || header
                .len()
                .checked_add(converted.len())
                .is_none_or(|len| len > MAX_NOTE_BYTES)
        {
            return Err(invalid(
                "Inbox source exceeds the note limit or differs from exact conversion",
            ));
        }
        Ok(header + converted)
    }
    pub fn validate_markdown(&self, text: &str) -> Result<()> {
        let header = self.header()?;
        let body = text.strip_prefix(&header).ok_or_else(|| {
            invalid("Inbox source must preserve its identity, scope and original provenance")
        })?;
        self.markdown(body)?;
        Ok(())
    }
}
