//! Fixed DOCX text-profile mechanics. No files, catalog, provider or authority.
use std::sync::atomic::{AtomicBool, Ordering};

mod document;
mod opc;
mod package;
mod xml;

#[cfg(test)]
mod tests;

const MAX_ENTRIES: usize = 256;
const MAX_EXPANDED: usize = 32 * 1024 * 1024;
const MAX_PART: usize = 8 * 1024 * 1024;
const MAX_XML: usize = 8 * 1024 * 1024;
const MAX_XML_ITEMS: usize = 50_000;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Invalid,
    Unsupported,
    Limit,
    Cancelled,
}
type Result<T> = std::result::Result<T, Failure>;

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Failure::Cancelled)
    } else {
        Ok(())
    }
}

pub(super) fn convert(
    bytes: &[u8],
    cancel: &AtomicBool,
) -> std::result::Result<(super::InboxConversionFormat, String), super::InboxProcessOutcome> {
    use super::{InboxConversionFormat, InboxProcessOutcome};
    if cancel.load(Ordering::Acquire) {
        return Err(InboxProcessOutcome::Cancelled);
    }
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(InboxProcessOutcome::Failed {
            code: "binary_unsupported".into(),
        });
    }
    let result = package::load(bytes, cancel).and_then(|parts| opc::render(&parts, cancel));
    result
        .map(|text| (InboxConversionFormat::DocxTextV1, text))
        .map_err(|failure| match failure {
            Failure::Cancelled => InboxProcessOutcome::Cancelled,
            Failure::Invalid => InboxProcessOutcome::Failed {
                code: "docx_invalid".into(),
            },
            Failure::Unsupported => InboxProcessOutcome::Failed {
                code: "docx_unsupported".into(),
            },
            Failure::Limit => InboxProcessOutcome::Failed {
                code: "docx_limit".into(),
            },
        })
}
