//! Exact bounded text preparation and cancellation guards before workspace access.
use super::error::CliError;
use brn_workflow::MAX_NOTE_BYTES;
use std::{fs, io::Read, path::Path};

/// Small strict typed request, decoded without waiting for a FIFO writer.
pub(super) fn read_small_json_file<T: serde::de::DeserializeOwned>(
    path: &Path,
    kind: &str,
) -> Result<T, CliError> {
    use std::os::unix::fs::OpenOptionsExt;
    const MAX: usize = 64 * 1024;
    let io = |error: std::io::Error| CliError::Workflow(error.to_string());
    let mut file = fs::File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let metadata = file.metadata().map_err(io)?;
    if !metadata.is_file() || metadata.len() > MAX as u64 {
        return Err(super::usage(format!(
            "{kind} needs a regular JSON file up to 64 KiB"
        )));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX {
        return Err(super::usage(format!("{kind} JSON exceeds 64 KiB")));
    }
    serde_json::from_slice(&bytes).map_err(|_| super::usage(format!("invalid {kind} JSON schema")))
}

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

/// An explicit owner-selected binary input, bounded before workflow startup.
/// The workflow independently proves its owned installed copy, never this path.
pub(super) fn read_binary_file(path: &Path) -> Result<Vec<u8>, CliError> {
    fn io(error: std::io::Error) -> CliError {
        CliError::Workflow(error.to_string())
    }
    let limit = brn_workflow::inbox::MAX_INBOX_BINARY_BYTES;
    let meta = fs::metadata(path).map_err(io)?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err(CliError::Workflow(
            "Inbox binary original requires a regular file up to 16 MiB".into(),
        ));
    }
    let mut file = fs::File::open(path).map_err(io)?;
    let before = file.metadata().map_err(io)?;
    if !before.is_file() || before.len() > limit as u64 {
        return Err(CliError::Workflow(
            "Inbox binary original requires a regular file up to 16 MiB".into(),
        ));
    }
    let modified = before.modified().map_err(io)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    let after = file.metadata().map_err(io)?;
    if bytes.len() > limit
        || !after.is_file()
        || before.len() != after.len()
        || after.len() != bytes.len() as u64
        || modified != after.modified().map_err(io)?
    {
        return Err(CliError::Workflow(
            "Inbox binary original changed or exceeded 16 MiB during input".into(),
        ));
    }
    Ok(bytes)
}
