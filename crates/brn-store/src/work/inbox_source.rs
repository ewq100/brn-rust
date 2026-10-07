//! Immutable conversion proof carried by the existing proposal/recovery records.
use super::{
    WorkStore,
    inbox::{InboxItem, InboxKind},
    inbox_processing::{InboxConversionFormat, InboxProcessOutcome},
    proposal_apply::{self, ApplyJournal, ApplyOutcome},
    proposals::{self, NoteChange, SourceVersion},
};
use crate::{MAX_NOTE_BYTES, Result, hash, invalid};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

mod legacy_markup;

/// Complete maintained PNG decode facts, for historical saved-image validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngImageFacts {
    pub width: u32,
    pub height: u32,
}

/// Exact historical literal markup, without any package/document interpretation.
pub fn docx_inline_png_markdown(
    asset_name: &str,
    alt_text: Option<&str>,
    title: Option<&str>,
) -> Result<String> {
    legacy_markup::image_markdown(asset_name, alt_text, title)
}
pub fn validate_png_image(
    bytes: &[u8],
    cancel: &AtomicBool,
) -> std::result::Result<PngImageFacts, InboxProcessOutcome> {
    if cancel.load(Ordering::Acquire) {
        return Err(InboxProcessOutcome::Cancelled);
    }
    let (width, height) =
        brn_intake::validate_png_image(bytes).map_err(|_| InboxProcessOutcome::Failed {
            code: "png_invalid".into(),
        })?;
    if cancel.load(Ordering::Acquire) {
        return Err(InboxProcessOutcome::Cancelled);
    }
    Ok(PngImageFacts { width, height })
}

/// The deterministic original conversion used by proposal admission and later
/// preservation checks. Imported Markdown remains verbatim body evidence.
pub fn convert_original(
    kind: InboxKind,
    text: &str,
    cancel: &AtomicBool,
) -> std::result::Result<(InboxConversionFormat, String), InboxProcessOutcome> {
    if cancel.load(Ordering::Acquire) {
        return Err(InboxProcessOutcome::Cancelled);
    }
    if kind == InboxKind::Binary {
        return Err(InboxProcessOutcome::Failed {
            code: "binary_unsupported".into(),
        });
    }
    if kind == InboxKind::Markdown {
        return Ok((InboxConversionFormat::VerbatimMarkdownV1, text.to_owned()));
    }
    let mut longest = [0usize; 2];
    let mut runs = [0usize; 2];
    for (offset, byte) in text.bytes().enumerate() {
        if offset % 4096 == 0 && cancel.load(Ordering::Acquire) {
            return Err(InboxProcessOutcome::Cancelled);
        }
        for (i, delimiter) in b"`~".iter().copied().enumerate() {
            runs[i] = if byte == delimiter { runs[i] + 1 } else { 0 };
            longest[i] = longest[i].max(runs[i]);
        }
    }
    let i = usize::from(longest[1] < longest[0]);
    let width = (longest[i] + 1).max(3);
    let delimiter = if i == 0 { '`' } else { '~' };
    let length = text
        .len()
        .checked_add(2 * width + 6 + usize::from(!text.ends_with('\n')));
    if length.is_none_or(|len| len > MAX_NOTE_BYTES) {
        return Err(InboxProcessOutcome::Failed {
            code: "candidate_too_large".into(),
        });
    }
    let fence: String = std::iter::repeat_n(delimiter, width).collect();
    let mut markdown = format!("{fence}text\n{text}");
    if !text.ends_with('\n') {
        markdown.push('\n');
    }
    markdown.push_str(&fence);
    markdown.push('\n');
    Ok((InboxConversionFormat::LiteralTextV1, markdown))
}

/// Exact original preservation by one historical approval and a fresh saved
/// Source. The caller observes the bound vault and unique physical identity;
/// this validator performs no filesystem or catalog access.
pub struct InboxSourcePreservation<'a> {
    pub original: &'a InboxItem,
    pub original_text: &'a str,
    pub approval: &'a ApplyJournal,
    pub saved: &'a SourceVersion,
    pub saved_text: &'a str,
}

impl InboxSourcePreservation<'_> {
    pub fn validate(&self) -> Result<()> {
        self.original.validate()?;
        if self.original.capture.kind == InboxKind::Binary {
            return Err(invalid("Binary Inbox Source preservation is not supported"));
        }
        self.approval.validate()?;
        if self.approval.undo.is_some()
            || self.approval.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied)
        {
            return Err(invalid(
                "Inbox preservation requires a non-Undo Applied Source approval",
            ));
        }
        let binding = self
            .approval
            .approved
            .draft
            .inbox_source
            .as_ref()
            .ok_or_else(|| invalid("Inbox preservation requires a bound Source approval"))?;
        let [
            NoteChange::Create {
                text: approved_text,
                ..
            },
        ] = self.approval.approved.draft.changes.as_slice()
        else {
            return Err(invalid(
                "Inbox preservation requires one approved Source Create",
            ));
        };
        if &binding.original != self.original
            || self.original_text.len() as u64 != self.original.capture.copy.byte_len
            || hash(self.original_text.as_bytes()) != self.original.capture.copy.sha256
        {
            return Err(invalid(
                "Inbox preservation differs from its exact original",
            ));
        }
        let (format, converted) = convert_original(
            self.original.capture.kind,
            self.original_text,
            &AtomicBool::new(false),
        )
        .map_err(|_| invalid("Inbox preservation cannot reconstruct its original conversion"))?;
        if binding.format != format
            || binding.byte_len != converted.len() as u64
            || binding.sha256 != hash(converted.as_bytes())
        {
            return Err(invalid(
                "Inbox preservation differs from its reconstructed conversion",
            ));
        }
        binding.validate_markdown(approved_text)?;
        let destination = self
            .approval
            .observations
            .as_ref()
            .and_then(|proofs| proofs.first())
            .and_then(|proof| proof.destination.as_ref())
            .ok_or_else(|| invalid("Inbox preservation lacks its terminal Source destination"))?;
        if destination.len != approved_text.len() as u64
            || destination.sha256 != hash(approved_text.as_bytes())
        {
            return Err(invalid(
                "Inbox preservation terminal proof differs from approved Source bytes",
            ));
        }
        proposals::validate_path(&self.saved.path)?;
        if self.saved_text.len() > MAX_NOTE_BYTES
            || self.saved.fingerprint.len != self.saved_text.len() as u64
            || self.saved.fingerprint.sha256 != hash(self.saved_text.as_bytes())
        {
            return Err(invalid(
                "Inbox preservation differs from its fresh saved Source proof",
            ));
        }
        if crate::note_identity::read(self.saved_text)? != Some(binding.note_id)
            || !crate::note_metadata::classify(self.saved_text)?.source
            || read_provenance(self.saved_text)? != Some(binding.provenance())
            || self.saved_text[crate::note_identity::body_start(self.saved_text)?..] != converted
        {
            return Err(invalid(
                "Inbox preservation requires exact Source identity, provenance and original body",
            ));
        }
        Ok(())
    }
}

impl WorkStore {
    /// Historical non-Undo Applied Source approval IDs, ordered by SQL
    /// operation_id. Every listed journal is fully checked in one read-only
    /// snapshot, including unrelated rows; this grants no filesystem freshness.
    pub fn inbox_source_approval_ids(&self, item_id: Uuid) -> Result<Vec<Uuid>> {
        proposals::nonnil(item_id)?;
        let tx = self.conn.unchecked_transaction()?;
        let mut ids = Vec::new();
        {
            let mut stmt =
                tx.prepare("SELECT operation_id FROM proposal_applies ORDER BY operation_id")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id = crate::parse_id(row.get(0)?)?;
                let journal = proposal_apply::read_journal(&tx, id)?
                    .ok_or_else(|| invalid("listed Source approval journal is missing"))?;
                if journal.undo.is_none()
                    && journal.receipt.as_ref().map(|r| r.outcome) == Some(ApplyOutcome::Applied)
                    && journal
                        .approved
                        .draft
                        .inbox_source
                        .as_ref()
                        .is_some_and(|binding| binding.original.capture.id == item_id)
                {
                    ids.push(id);
                }
            }
        }
        tx.commit()?;
        Ok(ids)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction: Option<ExtractionBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual: Option<super::inbox_visual::InboxSourceVisual>,
    pub batch_id: Uuid,
    pub index: usize,
    pub original: InboxItem,
    pub format: InboxConversionFormat,
    pub byte_len: u64,
    pub sha256: [u8; 32],
    pub note_id: Uuid,
}

/// Compact immutable extraction receipt. Payloads remain in the checked snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionBinding {
    pub snapshot_id: Uuid,
    pub snapshot_sha256: [u8; 32],
    pub assets: Vec<ExtractionAsset>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionAsset {
    pub name: String,
    pub byte_len: u64,
    pub sha256: [u8; 32],
}
impl ExtractionBinding {
    pub fn validate(&self) -> Result<()> {
        if self.snapshot_id.is_nil() || self.assets.len() > 32 {
            return Err(invalid("invalid extraction snapshot binding"));
        }
        let mut names = std::collections::HashSet::new();
        for asset in &self.assets {
            super::proposals::validate_asset_path(&asset.name)?;
            if asset.name.contains('/')
                || !names.insert(&asset.name)
                || asset.byte_len > super::proposals::MAX_ASSET_BYTES as u64
            {
                return Err(invalid("invalid extraction asset binding"));
            }
        }
        Ok(())
    }
    pub fn asset_path(&self, source_path: &str, name: &str) -> Result<String> {
        super::proposals::validate_path(source_path)?;
        if !self.assets.iter().any(|asset| asset.name == name) {
            return Err(invalid("unbound extraction asset"));
        }
        let folder = source_path.rsplit_once('/').map(|(folder, _)| folder);
        let path = folder.map_or_else(|| name.to_owned(), |folder| format!("{folder}/{name}"));
        super::proposals::validate_asset_path(&path)?;
        Ok(path)
    }
}

/// Durable semantic provenance contains no machine-specific file paths/inodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction: Option<ExtractionBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual: Option<super::inbox_visual::InboxSourceVisual>,
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
    if value.kind == InboxKind::Binary
        && (!matches!(
            value.format,
            InboxConversionFormat::DocxTextV1
                | InboxConversionFormat::DocxInlinePngV1
                | InboxConversionFormat::MaintainedExtractionV1
        ) || !(22..=super::inbox::MAX_INBOX_BINARY_BYTES as u64)
            .contains(&value.original_byte_len))
    {
        return Err(invalid("Binary Inbox Source provenance is not supported"));
    }
    if value.item_id.is_nil()
        || value.received_at_ms > i64::MAX as u64
        || std::iter::once(&value.title)
            .chain(value.original_name.iter())
            .any(|s| s.trim().is_empty() || s.len() > 512 || s.chars().any(char::is_control))
        || !match value.kind {
            InboxKind::Binary => matches!(
                value.format,
                InboxConversionFormat::DocxTextV1
                    | InboxConversionFormat::DocxInlinePngV1
                    | InboxConversionFormat::MaintainedExtractionV1
            ),
            InboxKind::Markdown => {
                value.format == InboxConversionFormat::VerbatimMarkdownV1
                    && value.original_byte_len <= MAX_NOTE_BYTES as u64
            }
            _ => {
                matches!(
                    value.format,
                    InboxConversionFormat::LiteralTextV1
                        | InboxConversionFormat::MaintainedExtractionV1
                ) && value.original_byte_len <= MAX_NOTE_BYTES as u64
            }
        }
        || value.original_byte_len == 0 && value.original_sha256 != hash(&[])
    {
        return Err(invalid("Inbox provenance has invalid original metadata"));
    }
    if let Some(extraction) = &value.extraction {
        extraction.validate()?;
        if value.format != InboxConversionFormat::MaintainedExtractionV1 || value.visual.is_some() {
            return Err(invalid("extraction provenance profile differs"));
        }
    } else if value.format == InboxConversionFormat::MaintainedExtractionV1 {
        return Err(invalid("extraction provenance has no snapshot"));
    }
    match (&value.visual, value.format) {
        (Some(visual), InboxConversionFormat::DocxInlinePngV1) => {
            visual.validate_source_body(
                &value.original_sha256,
                &text[crate::note_identity::body_start(text)?..],
            )?;
        }
        (None, format) if format != InboxConversionFormat::DocxInlinePngV1 => {}
        _ => {
            return Err(invalid(
                "Inbox visual provenance differs from its conversion profile",
            ));
        }
    }
    Ok(Some(value))
}

impl InboxSourceBinding {
    pub fn validate(&self) -> Result<()> {
        self.original.validate()?;
        if let Some(extraction) = &self.extraction {
            extraction.validate()?;
            if self.format != InboxConversionFormat::MaintainedExtractionV1
                || self.visual.is_some()
                || self.batch_id.is_nil()
                || self.note_id.is_nil()
                || self.index >= super::inbox_processing::MAX_PROCESS_BATCH
                || self.byte_len > MAX_NOTE_BYTES as u64
            {
                return Err(invalid("invalid maintained extraction Source binding"));
            }
            return Ok(());
        }
        if self.format == InboxConversionFormat::MaintainedExtractionV1 {
            return Err(invalid("maintained extraction requires a snapshot"));
        }
        if self.original.capture.kind == InboxKind::Binary
            && (!matches!(
                self.format,
                InboxConversionFormat::DocxTextV1
                    | InboxConversionFormat::DocxInlinePngV1
                    | InboxConversionFormat::MaintainedExtractionV1
            ) || !(22..=super::inbox::MAX_INBOX_BINARY_BYTES as u64)
                .contains(&self.original.capture.copy.byte_len)
                || self.byte_len == 0 && self.sha256 != hash(&[]))
        {
            return Err(invalid(
                "Binary Inbox Source conversion needs an exact DOCX proof",
            ));
        }
        match (&self.visual, self.format) {
            (Some(visual), InboxConversionFormat::DocxInlinePngV1) => {
                visual.validate(&self.original.capture.copy.sha256)?;
                if visual.converted_byte_len != self.byte_len
                    || visual.converted_sha256 != self.sha256
                {
                    return Err(invalid(
                        "Inbox visual differs from its complete conversion proof",
                    ));
                }
            }
            (None, format) if format != InboxConversionFormat::DocxInlinePngV1 => {}
            _ => {
                return Err(invalid(
                    "Inbox visual binding differs from its conversion profile",
                ));
            }
        }
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
            InboxKind::Binary
                if matches!(
                    self.format,
                    InboxConversionFormat::DocxTextV1
                        | InboxConversionFormat::DocxInlinePngV1
                        | InboxConversionFormat::MaintainedExtractionV1
                ) => {}
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
            extraction: self.extraction.clone(),
            visual: self.visual.clone(),
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
        let section = if let Some(visual) = &self.visual {
            visual.validate_converted(&self.original.capture.copy.sha256, converted)?;
            super::inbox_visual::pending_section()
        } else {
            String::new()
        };
        if converted.len() as u64 != self.byte_len
            || hash(converted.as_bytes()) != self.sha256
            || header
                .len()
                .checked_add(converted.len())
                .and_then(|len| len.checked_add(section.len()))
                .is_none_or(|len| len > MAX_NOTE_BYTES)
        {
            return Err(invalid(
                "Inbox source exceeds the note limit or differs from exact conversion",
            ));
        }
        Ok(header + converted + &section)
    }
    pub fn validate_markdown(&self, text: &str) -> Result<()> {
        let header = self.header()?;
        let body = text.strip_prefix(&header).ok_or_else(|| {
            invalid("Inbox source must preserve its identity, scope and original provenance")
        })?;
        let converted = if self.visual.is_some() {
            body.strip_suffix(&super::inbox_visual::pending_section())
                .ok_or_else(|| {
                    invalid("Inbox visual Create needs its pending interpretation section")
                })?
        } else {
            body
        };
        if self.markdown(converted)? != text {
            return Err(invalid(
                "Inbox Source Create differs from its exact reconstructed bytes",
            ));
        }
        Ok(())
    }
    pub fn validate_members(&self, changes: &[NoteChange]) -> Result<()> {
        if let Some(extraction) = &self.extraction {
            self.validate()?;
            let Some(NoteChange::Create { path, text, .. }) = changes.first() else {
                return Err(invalid("extraction Source requires a Create"));
            };
            self.validate_markdown(text)?;
            if changes.len() != 1 + extraction.assets.len() {
                return Err(invalid("extraction Source requires every bound asset"));
            }
            for (asset, change) in extraction.assets.iter().zip(&changes[1..]) {
                let NoteChange::CreateAsset {
                    path: asset_path,
                    bytes,
                    ..
                } = change
                else {
                    return Err(invalid("extraction asset must be created exactly"));
                };
                if *asset_path != extraction.asset_path(path, &asset.name)?
                    || bytes.len() as u64 != asset.byte_len
                    || hash(bytes) != asset.sha256
                {
                    return Err(invalid("extraction asset differs from immutable evidence"));
                }
            }
            return Ok(());
        }
        let (path, text, asset) = match changes {
            [NoteChange::Create { path, text, .. }] if self.visual.is_none() => (path, text, None),
            [
                NoteChange::Create { path, text, .. },
                NoteChange::CreateAsset {
                    path: asset_path,
                    bytes,
                    ..
                },
            ] if self.visual.is_some() => (path, text, Some((asset_path, bytes))),
            _ => {
                return Err(invalid(
                    "Inbox Source requires its exact Create and optional bound PNG Create",
                ));
            }
        };
        self.validate_markdown(text)?;
        if let Some((asset_path, bytes)) = asset {
            self.validate_asset(path, asset_path, bytes)?;
        }
        Ok(())
    }
    pub fn validate_asset(&self, source_path: &str, asset_path: &str, bytes: &[u8]) -> Result<()> {
        self.validate()?;
        let visual = self
            .visual
            .as_ref()
            .ok_or_else(|| invalid("Inbox Source has no visual binding"))?;
        if asset_path != visual.asset_path(source_path, &self.original.capture.copy.sha256)?
            || bytes.len() as u64 != visual.byte_len
            || hash(bytes) != visual.sha256
        {
            return Err(invalid(
                "Inbox Source asset differs from its complete PNG binding",
            ));
        }
        let facts = validate_png_image(bytes, &AtomicBool::new(false))
            .map_err(|_| invalid("Inbox Source asset is not a complete bounded PNG"))?;
        if facts.width != visual.width || facts.height != visual.height {
            return Err(invalid(
                "Inbox Source PNG dimensions differ from its decoded bytes",
            ));
        }
        Ok(())
    }
}
