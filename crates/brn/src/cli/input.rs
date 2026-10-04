//! Exact bounded text preparation and cancellation guards before workspace access.
use super::error::CliError;
use brn_workflow::MAX_NOTE_BYTES;
use std::{fs, io::Read, path::Path};

/// Read once, bounded by bytes, without newline or Unicode normalization.
pub(super) fn read_text_file(path: &Path, kind: &str) -> Result<String, CliError> {
    fn io(error: std::io::Error) -> CliError {
        CliError::Workflow(error.to_string())
    }
    let meta = fs::metadata(path).map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_NOTE_BYTES as u64 {
        return Err(CliError::Workflow(format!(
            "{kind} text requires a regular UTF-8 text file up to 1 MiB"
        )));
    }
    let mut file = fs::File::open(path).map_err(io)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_NOTE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_NOTE_BYTES {
        return Err(CliError::Workflow(format!("{kind} text exceeds 1 MiB")));
    }
    String::from_utf8(bytes)
        .map_err(|_| CliError::Workflow(format!("{kind} text file is not valid UTF-8")))
}
