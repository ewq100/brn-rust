//! Owned bounded text conversion. A preview is pending review work; it is not
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
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxCandidateRequest {
    pub batch_id: Uuid,
    pub index: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxConversionPreview {
    pub request: InboxCandidateRequest,
    pub original: crate::inbox::InboxItem,
    pub format: InboxConversionFormat,
    pub markdown: String,
    /// Text conversion does not establish semantic ingestion, attachment/visual
    /// completeness, managed identity, or approved provenance.
    pub needs_semantic_review: bool,
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn convert(
    kind: InboxKind,
    text: &str,
    cancel: &AtomicBool,
) -> std::result::Result<(InboxConversionFormat, String), InboxProcessOutcome> {
    if cancel.load(Ordering::Acquire) {
        return Err(InboxProcessOutcome::Cancelled);
    }
    if kind == InboxKind::Markdown {
        return Ok((InboxConversionFormat::VerbatimMarkdownV1, text.to_owned()));
    }
    // A fence longer than every run cannot be closed by imported delimiters.
    // Prefer the shorter safe delimiter. The exact body is never normalized.
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
    if length.is_none_or(|len| len > crate::MAX_NOTE_BYTES) {
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
impl App {
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
        let outcome = match self.inbox_item(item.capture.id)?.original {
            InboxOriginal::Available { text } => match convert(item.capture.kind, &text, cancel) {
                Ok((format, markdown)) => InboxProcessOutcome::Converted {
                    format,
                    byte_len: markdown.len() as u64,
                    sha256: digest(markdown.as_bytes()),
                },
                Err(outcome) => outcome,
            },
            InboxOriginal::Missing => InboxProcessOutcome::Failed {
                code: "original_missing".into(),
            },
            InboxOriginal::Changed { .. } => InboxProcessOutcome::Failed {
                code: "original_changed".into(),
            },
            InboxOriginal::Unavailable { .. } => InboxProcessOutcome::Failed {
                code: "original_unavailable".into(),
            },
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
        let InboxProcessOutcome::Converted {
            format,
            byte_len,
            sha256,
        } = &entry.outcome
        else {
            return Err(WorkflowError::typed(
                ErrorKind::OperationConflict,
                "Inbox entry has no completed conversion preview",
            ));
        };
        let item = &batch.request.items[request.index];
        let InboxOriginal::Available { text } = self.inbox_item(item.capture.id)?.original else {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox original is unavailable or changed; conversion preview cannot be qualified",
            ));
        };
        let (actual_format, markdown) = convert(item.capture.kind, &text, &AtomicBool::new(false))
            .map_err(|_| WorkflowError::msg("retained Inbox conversion cannot be reproduced"))?;
        if actual_format != *format
            || markdown.len() as u64 != *byte_len
            || digest(markdown.as_bytes()) != *sha256
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "Inbox conversion differs from its retained exact receipt",
            ));
        }
        Ok(InboxConversionPreview {
            request: request.clone(),
            original: item.clone(),
            format: *format,
            markdown,
            needs_semantic_review: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
