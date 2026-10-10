//! Final-use Threads persistence. Authority is supplied by the trusted host,
//! never decoded from a model request. All writers use SQLite IMMEDIATE transactions.
mod edit;
mod store;
mod types;
pub use store::{DATA_MARKER, DATABASE_FILENAME, Store};
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("record encoding: {0}")]
    Json(#[from] serde_json::Error),
    #[error("incompatible or unsafe data directory: {0}")]
    UnsafeDirectory(String),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("operation request is immutable")]
    ImmutableRequest,
    #[error("record or operation not found: {0}")]
    Missing(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn integer(value: u64) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| Error::Invalid("version or generation exceeds SQLite integer range".into()))
}
pub(crate) fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}
