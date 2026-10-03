//! Operational storage for BRN. WorkStore owns `brn.sqlite`, its data-directory
//! lock and checked migrations; the vault owns saved Markdown bytes.
use sha2::{Digest, Sha256};
use std::{os::unix::fs::MetadataExt, path::Path};
use uuid::Uuid;

pub mod files;
pub mod work;
pub mod workspace_mode;
pub use work::{MAX_NOTE_BYTES, OpenReport, UnsavedEdit, WorkStore};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    Invalid(String),
    /// The data directory lock is held by another live process.
    WorkspaceBusy(String),
    /// The folder contains markers for the other database authority.
    WorkspaceModeConflict(String),
    /// A durable operation ID was reused with different kind or payload.
    OperationConflict(String),
    /// A requested durable record does not exist.
    NotFound(String),
    /// A submitted editor baseline or generation no longer matches durable work.
    StateChanged(String),
    /// An original Markdown save must be reconciled before another can begin.
    SaveUncertain(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Sql(e) => write!(f, "SQLite: {e}"),
            Self::Invalid(e)
            | Self::WorkspaceBusy(e)
            | Self::WorkspaceModeConflict(e)
            | Self::OperationConflict(e)
            | Self::NotFound(e)
            | Self::StateChanged(e)
            | Self::SaveUncertain(e) => f.write_str(e),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn parse_id(s: String) -> Result<Uuid> {
    Uuid::parse_str(&s).map_err(|_| invalid("invalid stored UUID"))
}

/// Acquires the owner lock: `WouldBlock` is genuine contention (including
/// the transient fork+exec window) and is retried until `deadline`; any other
/// lock failure is an I/O error and returns immediately. Classification is
/// the enum variant, never the error text.
fn acquire_owner_lock(
    mut attempt: impl FnMut() -> std::result::Result<(), std::fs::TryLockError>,
    deadline: std::time::Instant,
) -> Result<()> {
    loop {
        match attempt() {
            Ok(()) => return Ok(()),
            Err(e @ std::fs::TryLockError::WouldBlock) => {
                if std::time::Instant::now() >= deadline {
                    return Err(Error::WorkspaceBusy(format!(
                        "data directory is already owned: {e}"
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(std::fs::TryLockError::Error(e)) => return Err(Error::Io(e)),
        }
    }
}

fn check_regular_single_link(path: &Path) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if !meta.file_type().is_file() || meta.nlink() != 1 {
        return Err(invalid(
            "database or lock path is not an unaliased regular file",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_lock_succeeds_on_first_attempt() {
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let result = acquire_owner_lock(
            || {
                calls += 1;
                Ok(())
            },
            Instant::now() + Duration::from_secs(1),
        );
        assert!(result.is_ok());
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_retries_would_block_then_succeeds() {
        use std::fs::TryLockError;
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let result = acquire_owner_lock(
            || {
                calls += 1;
                if calls < 4 {
                    Err(TryLockError::WouldBlock)
                } else {
                    Ok(())
                }
            },
            Instant::now() + Duration::from_secs(10),
        );
        assert!(result.is_ok());
        assert_eq!(calls, 4);
    }

    #[test]
    fn owner_lock_would_block_past_deadline_is_workspace_busy() {
        use std::fs::TryLockError;
        use std::time::Instant;
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::WouldBlock)
            },
            Instant::now(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::WorkspaceBusy(ref msg) if msg.contains("already owned")));
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_io_error_returns_immediately_without_retry() {
        use std::fs::TryLockError;
        use std::io::ErrorKind;
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::Error(std::io::Error::new(
                    ErrorKind::PermissionDenied,
                    "lock file not writable",
                )))
            },
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(ref e) if e.kind() == ErrorKind::PermissionDenied));
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_classifies_by_variant_not_error_text() {
        use std::fs::TryLockError;
        use std::io::ErrorKind;
        use std::time::{Duration, Instant};
        // An Error variant whose text claims contention must still be Io.
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::Error(std::io::Error::other(
                    "lock acquisition failed because the operation would block",
                )))
            },
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(ref e) if e.kind() == ErrorKind::Other));
        assert_eq!(calls, 1);
        // WouldBlock carries no text and is always contention (covered by the
        // retry and deadline tests above).
    }
}
