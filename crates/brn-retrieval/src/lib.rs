//! The rebuildable workspace search index (`search_index`), its Markdown
//! chunker, snippets, and optional native embeddings with a pinned installer.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid retrieval input: {0}")]
    Invalid(&'static str),
    #[error("retrieval generation is incomplete or corrupt: {0}")]
    Corrupt(&'static str),
    #[error("retrieval profile unavailable: {0}")]
    Unavailable(&'static str),
    #[cfg(feature = "native")]
    #[error("{0}")]
    ModelInstall(#[from] native::download::ModelInstallError),
    #[error("retrieval build cancelled")]
    Cancelled,
    #[error("search index moved since this change was computed")]
    Stale,
    #[error("retrieval I/O: {0}")]
    Io(#[from] std::io::Error),
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

pub mod embedding;
pub mod markdown_chunk;
#[cfg(feature = "native")]
pub mod native;
pub mod search_index;
pub mod snippet;
