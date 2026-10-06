//! Fixed DOCX text-profile mechanics. No files, catalog, provider or authority.
use std::sync::atomic::{AtomicBool, Ordering};

mod document;
mod image;
mod opc;
mod package;
mod xml;

#[cfg(test)]
mod image_tests;
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

fn outcome(failure: Failure, prefix: &str) -> super::InboxProcessOutcome {
    use super::InboxProcessOutcome;
    let suffix = match failure {
        Failure::Cancelled => return InboxProcessOutcome::Cancelled,
        Failure::Invalid => "invalid",
        Failure::Unsupported => "unsupported",
        Failure::Limit => "limit",
    };
    InboxProcessOutcome::Failed {
        code: format!("{prefix}_{suffix}"),
    }
}

pub(super) fn validate_png(
    bytes: &[u8],
    cancel: &AtomicBool,
) -> std::result::Result<super::PngImageFacts, super::InboxProcessOutcome> {
    image::validate_png(bytes, cancel).map_err(|failure| outcome(failure, "png"))
}

pub(super) fn convert_source(
    bytes: &[u8],
    cancel: &AtomicBool,
) -> std::result::Result<super::DocxSourceConversion, super::InboxProcessOutcome> {
    check_cancel(cancel).map_err(|failure| outcome(failure, "docx"))?;
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(super::InboxProcessOutcome::Failed {
            code: "binary_unsupported".into(),
        });
    }
    package::load(bytes, cancel)
        .and_then(|parts| opc::render_source(&parts, bytes, cancel))
        .map_err(|failure| outcome(failure, "docx"))
}

pub(super) fn image_markdown(
    asset: &str,
    alt: Option<&str>,
    title: Option<&str>,
) -> crate::Result<String> {
    document::image_markdown(asset, alt, title, &AtomicBool::new(false))
        .map_err(|_| crate::invalid("Invalid or over-budget DOCX inline PNG markup"))
}
