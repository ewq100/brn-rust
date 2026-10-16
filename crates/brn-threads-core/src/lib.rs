//! Final-use Threads persistence. Authority is supplied by the trusted host,
//! never decoded from a model request. All writers use SQLite IMMEDIATE transactions.
mod backup;
mod inspect;
pub use inspect::{CandidateRow, OperationInfo, OperationState, RECORD_KINDS};
mod converge;
pub use converge::{Converged, Unchanged};
mod chat;
pub use chat::{ChatCommand, ChatCommandOutcome};
mod context;
mod projects;
pub use projects::{
    Project, ProjectActionChoice, ProjectCommand, ProjectDeletePreview, ProjectTrash,
    TrashedRecord, project_context_fences, project_document_ids, project_for_chat,
};
mod edit;
pub mod locator;
pub use locator::{canonical_locator, find_source, matching_sources, same_locator};
mod read;
mod reader;
mod restore;
mod revision_read;
pub mod run_attention;
mod stage;
mod steering;
mod store;
mod types;
mod upgrade;
mod writing;
pub use read::{MAX_NOTE_READ_BYTES, NoteHeading, NoteReadOutcome, NoteSlice, note_headings};
pub use revision_read::{RevisionPage, RevisionSummary};
pub use store::{DATA_MARKER, DATABASE_FILENAME, DERIVED_DIRECTORY, Store};
pub use types::*;
pub use writing::{
    MAX_WRITING_REQUEST_BYTES, WRITING_REQUEST_ROLE, WritingSnapshot, exact_writing_range,
};

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
    /// The note already has an open edit session, which stays untouched so
    /// the owner can compare, save or discard it.
    #[error("this note already has an open unsaved edit")]
    EditSessionOpen {
        note: String,
        session: String,
        base_version: u64,
    },
    /// The data folder cannot be opened or created. The message names the
    /// folder and the reason in the owner's terms; the OS detail stays in
    /// `cause`, reachable through `source()` for logs.
    #[error("BRN cannot use the data folder {folder}: {problem}")]
    DataFolder {
        folder: String,
        problem: DataFolderProblem,
        #[source]
        cause: std::io::Error,
    },
}
pub type Result<T> = std::result::Result<T, Error>;

/// Why a data folder is unusable. Its text never carries OS error wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFolderProblem {
    ReadOnly,
    PermissionDenied,
    NotAFolder,
    SymlinkRefused,
    MissingParent,
}
impl std::fmt::Display for DataFolderProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ReadOnly => "it is on a read-only disk. Choose a folder you can write to.",
            Self::PermissionDenied => {
                "BRN does not have permission to use it. Choose a folder you own."
            }
            Self::NotAFolder => "that path, or part of it, is a file rather than a folder.",
            Self::SymlinkRefused => {
                "part of its path is a symbolic link. Choose the folder's real location."
            }
            Self::MissingParent => "a folder above it is missing or unavailable.",
        })
    }
}

pub(crate) fn integer(value: u64) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| Error::Invalid("version or generation exceeds SQLite integer range".into()))
}
pub(crate) fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

pub use context::{managed_asset_ids, project_scope, visible as record_in_chat};
pub use reader::{
    MessageScope, SEARCHABLE_MESSAGE_ROLES, SearchEligibility, Snapshot, StoreReader,
    chat_is_archived, message_scope, message_scopes_for, message_searchable, search_eligibility,
};
