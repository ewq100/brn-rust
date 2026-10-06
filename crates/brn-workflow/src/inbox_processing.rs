//! Owned bounded text and DOCX conversion. A preview is pending review work; it is not
//! a saved source, a proposal, or permission to discard its retained original.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    inbox::{InboxKind, InboxOriginal},
};
pub use brn_store::work::inbox_processing::{
    InboxConversionFormat, InboxProcessBatch, InboxProcessOutcome, MAX_PROCESS_BATCH,
    ProcessInboxRequest,
};
pub use brn_store::work::inbox_source::InboxSourceBinding;
use brn_store::work::inbox_source::{DocxInlinePng, DocxSourceConversion};
pub use brn_store::work::inbox_visual::InboxSourceVisual;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceRequest {
    pub candidate: InboxCandidateRequest,
    pub proposal_id: Uuid,
    pub note_id: Uuid,
    pub path: String,
    pub title: String,
}
impl InboxSourceRequest {
    pub fn validate(&self) -> Result<()> {
        crate::vault::VaultPath::parse(&self.path)
            .map_err(|_| WorkflowError::msg("invalid Inbox source destination"))?;
        if self.proposal_id.is_nil()
            || self.note_id.is_nil()
            || self.candidate.batch_id.is_nil()
            || self.candidate.index >= MAX_PROCESS_BATCH
            || self.title.trim().is_empty()
            || self.title.len() > 512
        {
            return Err(WorkflowError::msg(
                "Inbox source needs nonnil IDs, a bounded candidate and title",
            ));
        }
        Ok(())
    }
    pub fn validate_draft(&self, draft: &crate::proposals::DraftRequest) -> Result<()> {
        self.validate()?;
        draft.validate()?;
        let Some(binding) = &draft.inbox_source else {
            return Err(WorkflowError::msg(
                "Inbox source response has no original binding",
            ));
        };
        let path = match draft.changes.as_slice() {
            [crate::proposals::DraftNoteChange::Create { path, .. }]
                if binding.visual.is_none() =>
            {
                path
            }
            [
                crate::proposals::DraftNoteChange::Create { path, .. },
                crate::proposals::DraftNoteChange::CreateAsset { .. },
            ] if binding.visual.is_some() => path,
            _ => {
                return Err(WorkflowError::msg(
                    "Inbox Source response differs from its complete member set",
                ));
            }
        };
        if draft.id != self.proposal_id
            || draft.group_id.is_some()
            || draft.session_id.is_some()
            || draft.title != self.title
            || path != &self.path
            || binding.note_id != self.note_id
            || binding.batch_id != self.candidate.batch_id
            || binding.index != self.candidate.index
        {
            return Err(WorkflowError::msg(
                "Inbox source response differs from the complete preparation request",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxCandidateRequest {
    pub batch_id: Uuid,
    pub index: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxConversionPreview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual: Option<InboxVisualPreview>,
    pub request: InboxCandidateRequest,
    pub original: crate::inbox::InboxItem,
    pub format: InboxConversionFormat,
    pub markdown: String,
    /// Text conversion does not establish semantic ingestion, attachment/visual
    /// completeness, managed identity, or approved provenance.
    pub needs_semantic_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxVisualPreview {
    pub proof: InboxSourceVisual,
    #[serde(with = "brn_store::work::proposals::asset_payload")]
    pub bytes: Vec<u8>,
}

impl InboxVisualPreview {
    pub fn validate(&self, original_sha256: &[u8; 32], converted: &str) -> Result<()> {
        self.proof.validate_converted(original_sha256, converted)?;
        let facts =
            brn_store::work::inbox_source::validate_png_image(&self.bytes, &AtomicBool::new(false))
                .map_err(|_| WorkflowError::msg("Inbox preview is not a complete bounded PNG"))?;
        if self.bytes.len() as u64 != self.proof.byte_len
            || digest(&self.bytes) != self.proof.sha256
            || facts.width != self.proof.width
            || facts.height != self.proof.height
        {
            return Err(WorkflowError::msg(
                "Inbox preview differs from its complete PNG proof",
            ));
        }
        Ok(())
    }
}

fn visual_proof(image: &DocxInlinePng, converted: &str) -> InboxSourceVisual {
    InboxSourceVisual {
        part_name: image.part_name.clone(),
        relationship_id: image.relationship_id.clone(),
        asset_name: image.asset_name.clone(),
        byte_len: image.byte_len,
        sha256: image.sha256,
        width: image.width,
        height: image.height,
        alt_text: image.alt_text.clone(),
        title: image.title.clone(),
        image_start: image.image_start,
        image_end: image.image_end,
        converted_byte_len: converted.len() as u64,
        converted_sha256: digest(converted.as_bytes()),
    }
}

impl InboxConversionPreview {
    /// Check a client response against the retained complete conversion receipt.
    /// This qualifies preview bytes, not source-wrapper size or knowledge approval.
    pub fn validate_receipt(&self, batch: &InboxProcessBatch) -> Result<()> {
        batch.validate()?;
        let Some(entry) = batch.entries.get(self.request.index) else {
            return Err(WorkflowError::msg(
                "Inbox preview index differs from its receipt",
            ));
        };
        let InboxProcessOutcome::Converted {
            format,
            byte_len,
            sha256,
        } = &entry.outcome
        else {
            return Err(WorkflowError::msg(
                "Inbox preview has no completed conversion receipt",
            ));
        };
        if self.request.batch_id != batch.request.id
            || self.original != batch.request.items[self.request.index]
            || self.format != *format
            || self.markdown.len() as u64 != *byte_len
            || digest(self.markdown.as_bytes()) != *sha256
            || !self.needs_semantic_review
        {
            return Err(WorkflowError::msg(
                "Inbox preview differs from its complete conversion receipt",
            ));
        }
        match (&self.visual, self.format) {
            (Some(visual), InboxConversionFormat::DocxInlinePngV1) => {
                visual.validate(&self.original.capture.copy.sha256, &self.markdown)?;
            }
            (None, format) if format != InboxConversionFormat::DocxInlinePngV1 => {}
            _ => {
                return Err(WorkflowError::msg(
                    "Inbox preview visual differs from its conversion profile",
                ));
            }
        }
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn convert(
    kind: InboxKind,
    text: &str,
    cancel: &AtomicBool,
) -> std::result::Result<(InboxConversionFormat, String), InboxProcessOutcome> {
    brn_store::work::inbox_source::convert_original(kind, text, cancel)
}
impl App {
    fn convert_fresh_inbox_original(
        &self,
        item: &crate::inbox::InboxItem,
        cancel: &AtomicBool,
    ) -> std::result::Result<DocxSourceConversion, InboxProcessOutcome> {
        if item.capture.kind == InboxKind::Binary {
            let Some(files) = &self.inbox.files else {
                return Err(InboxProcessOutcome::Failed {
                    code: "original_unavailable".into(),
                });
            };
            return match files.read_binary_bytes(item) {
                Ok(Some(bytes)) => {
                    brn_store::work::inbox_source::convert_docx_source(&bytes, cancel)
                }
                Ok(None) => Err(InboxProcessOutcome::Failed {
                    code: "original_missing".into(),
                }),
                Err(error) => Err(InboxProcessOutcome::Failed {
                    code: if error.kind == ErrorKind::ContextStale {
                        "original_changed"
                    } else {
                        "original_unavailable"
                    }
                    .into(),
                }),
            };
        }
        match self.inbox.original(item) {
            InboxOriginal::Available { text } => {
                convert(item.capture.kind, &text, cancel).map(|(format, body)| {
                    DocxSourceConversion {
                        format,
                        body,
                        visual: None,
                    }
                })
            }
            original => Err(InboxProcessOutcome::Failed {
                code: match original {
                    InboxOriginal::Missing => "original_missing",
                    InboxOriginal::RemovedRetained { .. } => "original_removed_retained",
                    InboxOriginal::Changed { .. } => "original_changed",
                    _ => "original_unavailable",
                }
                .into(),
            }),
        }
    }
    /// Prepare exact source review input. Admission and approval remain separate
    /// existing commands; this never creates a note or discards the original.
    pub fn prepare_inbox_source(
        &mut self,
        request: &InboxSourceRequest,
    ) -> Result<crate::proposals::DraftRequest> {
        request.validate()?;
        let preview = self.inbox_candidate(&request.candidate)?;
        let binding = InboxSourceBinding {
            visual: preview.visual.as_ref().map(|visual| visual.proof.clone()),
            batch_id: request.candidate.batch_id,
            index: request.candidate.index,
            original: preview.original,
            format: preview.format,
            byte_len: preview.markdown.len() as u64,
            sha256: digest(preview.markdown.as_bytes()),
            note_id: request.note_id,
        };
        let text = binding.markdown(&preview.markdown)?;
        let mut changes = vec![crate::proposals::DraftNoteChange::Create {
            path: request.path.clone(),
            text,
        }];
        if let Some(visual) = preview.visual {
            changes.push(crate::proposals::DraftNoteChange::CreateAsset {
                path: visual
                    .proof
                    .asset_path(&request.path, &binding.original.capture.copy.sha256)?,
                bytes: visual.bytes,
            });
        }
        let draft = crate::proposals::DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: Some(Box::new(binding)),
            id: request.proposal_id,
            group_id: None,
            session_id: None,
            title: request.title.clone(),
            changes,
            sources: Vec::new(),
            action_changes: Vec::new(),
        };
        draft.validate()?;
        self.check_inbox_source_identity(request.note_id)?;
        Ok(draft)
    }

    pub(crate) fn check_inbox_source_identity(&self, id: Uuid) -> Result<()> {
        let resolution = self.resolve_note_identity(id)?;
        if resolution.outcome != crate::knowledge::IdentityOutcome::Absent {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox source note identity already exists or cannot be completely inspected",
            ));
        }
        Ok(())
    }

    /// Unfinished application uses retained immutable proof, independent of the
    /// disposable processing rows. Completed historical receipts bypass this.
    pub(crate) fn validate_inbox_source(&self, binding: Option<&InboxSourceBinding>) -> Result<()> {
        let Some(binding) = binding else {
            return Ok(());
        };
        binding.validate()?;
        if self
            .store
            .inbox_item(binding.original.capture.id)?
            .is_some_and(|item| item != binding.original)
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox source capture changed",
            ));
        }
        let converted = if binding.original.capture.kind == InboxKind::Binary {
            self.convert_fresh_inbox_original(&binding.original, &AtomicBool::new(false))
                .map_err(|_| {
                    WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "Inbox source original is missing, changed, unavailable or cannot be reproduced",
                    )
                })?
        } else {
            let InboxOriginal::Available { text } = self.inbox.original(&binding.original) else {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "Inbox source original is missing, changed or unavailable",
                ));
            };
            let (format, body) = convert(
                binding.original.capture.kind,
                &text,
                &AtomicBool::new(false),
            )
            .map_err(|_| WorkflowError::msg("Inbox source conversion cannot be reproduced"))?;
            DocxSourceConversion {
                format,
                body,
                visual: None,
            }
        };
        if converted.format != binding.format
            || converted.body.len() as u64 != binding.byte_len
            || digest(converted.body.as_bytes()) != binding.sha256
            || converted
                .visual
                .as_ref()
                .map(|image| visual_proof(image, &converted.body))
                != binding.visual
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox source conversion proof changed",
            ));
        }
        Ok(())
    }
    /// Explicit admission/replay only. The app lane advances one item at a time.
    pub fn process_inbox(&mut self, request: &ProcessInboxRequest) -> Result<InboxProcessBatch> {
        Ok(self.store.process_inbox(request)?)
    }
    pub fn inbox_processing(&self, id: Uuid) -> Result<InboxProcessBatch> {
        self.store.inbox_processing(id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "Inbox processing does not exist")
        })
    }
    pub fn cancel_inbox_processing(&mut self, id: Uuid) -> Result<InboxProcessBatch> {
        Ok(self.store.cancel_inbox_processing(id)?)
    }
    pub(crate) fn advance_inbox_processing(
        &mut self,
        id: Uuid,
        cancel: &AtomicBool,
    ) -> Result<InboxProcessBatch> {
        if cancel.load(Ordering::Acquire) {
            return self.cancel_inbox_processing(id);
        }
        let batch = self.inbox_processing(id)?;
        let index = batch
            .entries
            .iter()
            .position(|entry| entry.outcome.pending())
            .ok_or_else(|| WorkflowError::msg("Inbox processing has no pending entry"))?;
        let batch = self.store.start_inbox_processing(id, index)?;
        let item = &batch.request.items[index];
        self.store.inbox_item(item.capture.id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "Inbox item does not exist")
        })?;
        let outcome = match self.convert_fresh_inbox_original(item, cancel) {
            Ok(converted) => InboxProcessOutcome::Converted {
                format: converted.format,
                byte_len: converted.body.len() as u64,
                sha256: digest(converted.body.as_bytes()),
            },
            Err(outcome) => outcome,
        };
        let outcome = if cancel.load(Ordering::Acquire) {
            InboxProcessOutcome::Cancelled
        } else {
            outcome
        };
        let batch = self.store.finish_inbox_processing(id, index, outcome)?;
        if cancel.load(Ordering::Acquire) {
            self.cancel_inbox_processing(id)
        } else {
            Ok(batch)
        }
    }
    /// Reproduce only from fresh exact original proof; missing/changed originals
    /// never fall back to an obsolete or guessed preview.
    pub fn inbox_candidate(
        &self,
        request: &InboxCandidateRequest,
    ) -> Result<InboxConversionPreview> {
        let batch = self.inbox_processing(request.batch_id)?;
        let entry = batch
            .entries
            .get(request.index)
            .ok_or_else(|| WorkflowError::msg("invalid Inbox candidate entry"))?;
        let InboxProcessOutcome::Converted { .. } = &entry.outcome else {
            return Err(WorkflowError::typed(
                ErrorKind::OperationConflict,
                "Inbox entry has no completed conversion preview",
            ));
        };
        let item = &batch.request.items[request.index];
        let converted = if item.capture.kind == InboxKind::Binary {
            // Candidate lookup retains its catalog prerequisite; unfinished
            // approval can use the immutable binding without processing rows.
            if self.store.inbox_item(item.capture.id)?.as_ref() != Some(item) {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "Inbox original capture differs from its conversion receipt",
                ));
            }
            self.convert_fresh_inbox_original(item, &AtomicBool::new(false))
                .map_err(|_| WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "Inbox original is unavailable or changed; conversion preview cannot be qualified",
                ))?
        } else {
            let InboxOriginal::Available { text } = self.inbox_item(item.capture.id)?.original
            else {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "Inbox original is unavailable or changed; conversion preview cannot be qualified",
                ));
            };
            let (format, body) = convert(item.capture.kind, &text, &AtomicBool::new(false))
                .map_err(|_| {
                    WorkflowError::msg("retained Inbox conversion cannot be reproduced")
                })?;
            DocxSourceConversion {
                format,
                body,
                visual: None,
            }
        };
        let visual = converted.visual.map(|image| InboxVisualPreview {
            proof: visual_proof(&image, &converted.body),
            bytes: image.bytes,
        });
        let preview = InboxConversionPreview {
            request: request.clone(),
            original: item.clone(),
            format: converted.format,
            markdown: converted.body,
            visual,
            needs_semantic_review: true,
        };
        preview.validate_receipt(&batch).map_err(|_| {
            WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox conversion differs from its retained exact receipt",
            )
        })?;
        Ok(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_receipt_preserves_full_size_and_refuses_changed_proof_or_bytes() {
        use brn_store::work::{
            inbox::{InboxCapture, InboxCopy, InboxItem},
            inbox_processing::InboxProcessEntry,
        };
        let text = "x".repeat(crate::MAX_NOTE_BYTES);
        let item = InboxItem {
            capture: InboxCapture {
                id: Uuid::new_v4(),
                kind: InboxKind::Markdown,
                title: "Exact original".into(),
                original_name: None,
                copy: InboxCopy {
                    directory: "/synthetic/inbox".into(),
                    directory_device: 7,
                    directory_inode: 8,
                    file_device: 7,
                    file_inode: 9,
                    byte_len: text.len() as u64,
                    sha256: digest(text.as_bytes()),
                },
            },
            received_at_ms: 1,
        };
        let batch = InboxProcessBatch {
            request: ProcessInboxRequest {
                id: Uuid::new_v4(),
                items: vec![item.clone()],
            },
            queued_at_ms: 2,
            entries: vec![InboxProcessEntry {
                outcome: InboxProcessOutcome::Converted {
                    format: InboxConversionFormat::VerbatimMarkdownV1,
                    byte_len: text.len() as u64,
                    sha256: digest(text.as_bytes()),
                },
                started_at_ms: Some(3),
                finished_at_ms: Some(4),
            }],
        };
        let preview = InboxConversionPreview {
            visual: None,
            request: InboxCandidateRequest {
                batch_id: batch.request.id,
                index: 0,
            },
            original: item,
            format: InboxConversionFormat::VerbatimMarkdownV1,
            markdown: text,
            needs_semantic_review: true,
        };
        preview.validate_receipt(&batch).unwrap();
        let binding = InboxSourceBinding {
            visual: None,
            batch_id: batch.request.id,
            index: 0,
            original: preview.original.clone(),
            format: preview.format,
            byte_len: preview.markdown.len() as u64,
            sha256: digest(preview.markdown.as_bytes()),
            note_id: Uuid::new_v4(),
        };
        assert!(
            binding.markdown(&preview.markdown).is_err(),
            "source wrapper has its own bound"
        );
        for mutation in 0..7 {
            let mut forged = preview.clone();
            match mutation {
                0 => forged.request.batch_id = Uuid::new_v4(),
                1 => forged.request.index = 1,
                2 => forged.original.capture.title = "Different original".into(),
                3 => forged.format = InboxConversionFormat::LiteralTextV1,
                4 => forged.markdown.replace_range(0..1, "y"),
                5 => {
                    forged.markdown.pop();
                }
                _ => forged.needs_semantic_review = false,
            }
            assert!(forged.validate_receipt(&batch).is_err());
        }
        let mut pending = batch;
        pending.entries[0] = InboxProcessEntry {
            outcome: InboxProcessOutcome::Queued,
            started_at_ms: None,
            finished_at_ms: None,
        };
        assert!(preview.validate_receipt(&pending).is_err());
    }
    #[test]
    fn exact_body_and_safe_fences_preserve_control_bom_crlf_and_delimiters() {
        for text in [
            "",
            "\u{feff}Title õ\r\n\0日本語",
            "```text\n~~~\n``````\n~~~~~~~",
            "ending\r",
            "ending\r\n",
        ] {
            let (format, markdown) =
                convert(InboxKind::Email, text, &AtomicBool::new(false)).unwrap();
            assert_eq!(format, InboxConversionFormat::LiteralTextV1);
            let opening = markdown.find('\n').unwrap() + 1;
            assert_eq!(&markdown[opening..opening + text.len()], text);
            let fence = markdown[..opening - 1].strip_suffix("text").unwrap();
            assert!(!text.contains(fence));
            assert!(markdown.ends_with(&format!("{fence}\n")));
            assert_eq!(
                convert(InboxKind::Markdown, text, &AtomicBool::new(false)).unwrap(),
                (InboxConversionFormat::VerbatimMarkdownV1, text.to_owned())
            );
        }
    }
    #[test]
    fn conversion_bound_and_cancellation_never_truncate() {
        let exact = "x".repeat(crate::MAX_NOTE_BYTES);
        let boundary = "x".repeat(crate::MAX_NOTE_BYTES - 13);
        assert_eq!(
            convert(InboxKind::Text, &boundary, &AtomicBool::new(false))
                .unwrap()
                .1
                .len(),
            crate::MAX_NOTE_BYTES
        );
        assert_eq!(
            convert(InboxKind::Markdown, &exact, &AtomicBool::new(false))
                .unwrap()
                .1,
            exact
        );
        assert!(
            matches!(convert(InboxKind::Text, &exact, &AtomicBool::new(false)), Err(InboxProcessOutcome::Failed { code }) if code == "candidate_too_large")
        );
        assert_eq!(
            convert(InboxKind::Text, "retained", &AtomicBool::new(true)),
            Err(InboxProcessOutcome::Cancelled)
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod docx_tests;
#[cfg(all(test, target_os = "macos"))]
mod source_tests;
