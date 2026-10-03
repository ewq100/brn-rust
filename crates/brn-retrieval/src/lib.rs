//! Rebuildable current-note keyword and native retrieval indexes.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid retrieval input: {0}")]
    Invalid(&'static str),
    #[error("retrieval generation is incomplete or corrupt: {0}")]
    Corrupt(&'static str),
    #[error("retrieval profile unavailable: {0}")]
    Unavailable(&'static str),
    #[error("embedding model identity or dimension does not match the index")]
    ModelMismatch,
    #[error("shared embedding model lock is poisoned")]
    EmbedderPoisoned,
    #[cfg(feature = "native")]
    #[error("{0}")]
    ModelInstall(#[from] native::download::ModelInstallError),
    #[error("retrieval build cancelled")]
    Cancelled,
    #[error("retrieval I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("retrieval format: {0}")]
    Format(#[from] serde_json::Error),
    #[error("native retrieval: {0}")]
    Native(String),
    #[error("index database: {0}")]
    Sql(#[from] rusqlite::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "native")]
fn check_cancel(cancel: &std::sync::atomic::AtomicBool) -> Result<()> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

mod chunk;
#[cfg(feature = "native")]
pub mod native;
pub mod note_index;
