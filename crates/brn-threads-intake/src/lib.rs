//! Bounded transient conversion. Returned knowledge has no original-file payload.
//! Semantic selection for useful intake and protected-note creation belong to the host.
mod documents;
pub use brn_intake::document::expected_engine_identity;
pub use documents::{DocumentHelpers, document_conversion_label};
mod process;
mod transient;

use brn_intake::IntakeLimits;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportIntent {
    UsefulInformation,
    FullNote,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentKind {
    Docx,
    Pdf,
    Markdown,
    Text,
    Eml,
    Html,
    Xlsx,
    Pptx,
}

/// Host import profiles stay within the qualified hard limits, including cold PDF extraction.
pub fn import_limits(kind: DocumentKind) -> IntakeLimits {
    IntakeLimits {
        wall_time_ms: if kind == DocumentKind::Pdf {
            20_000
        } else {
            5_000
        },
        ..IntakeLimits::default()
    }
}

/// Read from one opened source with a hard cap, even if it grows after metadata inspection.
pub fn read_input_file(
    path: &std::path::Path,
    max_bytes: usize,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, ConversionError> {
    use std::io::Read;
    if cancel.load(Ordering::Acquire) {
        return Err(ConversionError::Cancelled);
    }
    if !path.is_absolute() || max_bytes == 0 || max_bytes > brn_intake::MAX_INPUT_BYTES {
        return Err(ConversionError::Invalid);
    }
    let file = fs::File::open(path).map_err(|_| ConversionError::Io)?;
    let meta = file.metadata().map_err(|_| ConversionError::Io)?;
    if !meta.is_file() || meta.len() > max_bytes as u64 {
        return Err(ConversionError::Budget);
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ConversionError::Io)?;
    if cancel.load(Ordering::Acquire) {
        return Err(ConversionError::Cancelled);
    }
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(ConversionError::Budget);
    }
    Ok(bytes)
}
#[derive(Clone, Debug)]
pub struct Converter {
    /// Explicit private application scratch directory, outside the knowledge store.
    pub temp_root: PathBuf,
    pub limits: IntakeLimits,
}
pub struct ConversionRequest<'a> {
    pub kind: DocumentKind,
    pub intent: ImportIntent,
    pub bytes: &'a [u8],
    pub source_reference: &'a str,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetainedAsset {
    pub name: String,
    pub media_type: String,
    pub sha256: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCoverage {
    pub locator: String,
    pub text: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoverageStatus {
    Complete,
    Partial,
    Unchecked,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapKind {
    Limitation,
    Omission,
    Unverified,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageGap {
    pub kind: GapKind,
    pub locator: String,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coverage {
    /// Coverage is host evidence, never an inference from a successful model response.
    pub status: CoverageStatus,
    pub checked_items: Vec<String>,
    pub gaps: Vec<CoverageGap>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversionResult {
    pub intent: ImportIntent,
    pub markdown: String,
    pub assets: Vec<RetainedAsset>,
    pub source_sha256: [u8; 32],
    pub source_reference: String,
    pub converter: String,
    pub sources: Vec<SourceCoverage>,
    pub coverage: Coverage,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionError {
    Cancelled,
    Timeout,
    Budget,
    Unavailable,
    Invalid,
    Protocol,
    Io,
}
impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ConversionError {}
