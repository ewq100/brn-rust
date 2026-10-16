//! The `--json` machine contract: statuses, stable error codes, the envelope
//! and its classification. Presentation only; every rule stays in the shared service.
use crate::spec::{self, Class, ReceiptAt};
use crate::Invocation;
use brn_ai::conversion::ConversionFailure;
use brn_threads_app::{AppError, RetrievalError, Store, Workspace};
use brn_threads_intake::ConversionError;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::io::Write;

/// The machine contract version in every envelope and the description.
pub(crate) const CONTRACT: u64 = 2;

/// Machine-mode status, one-to-one with the exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Ok,
    Internal,
    Usage,
    Stale,
    Deferred,
    NeedsReview,
    Rejected,
    Failed,
    Interrupted,
    Terminated,
}
impl Status {
    pub(crate) const ALL: [Status; 10] = [
        Status::Ok,
        Status::Internal,
        Status::Usage,
        Status::Stale,
        Status::Deferred,
        Status::NeedsReview,
        Status::Rejected,
        Status::Failed,
        Status::Interrupted,
        Status::Terminated,
    ];
    pub(crate) fn exit(self) -> u8 {
        match self {
            Status::Ok => 0,
            Status::Internal => 1,
            Status::Usage => 2,
            Status::Stale => 3,
            Status::Deferred => 4,
            Status::NeedsReview => 5,
            Status::Rejected => 6,
            Status::Failed => 7,
            Status::Interrupted => 130,
            Status::Terminated => 143,
        }
    }
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Internal => "internal",
            Status::Usage => "usage",
            Status::Stale => "stale",
            Status::Deferred => "deferred",
            Status::NeedsReview => "needs_review",
            Status::Rejected => "rejected",
            Status::Failed => "failed",
            Status::Interrupted => "interrupted",
            Status::Terminated => "terminated",
        }
    }
    pub(crate) fn meaning(self) -> &'static str {
        match self {
            Status::Ok => "Did what was asked, including an identical replay. Reads returned. ask Completed.",
            Status::Internal => "Unexpected storage, I/O or encoding failure.",
            Status::Usage => "argv or input does not fit the documented grammar. No requested change was committed. A fresh data directory may have been initialised when the error is detected after opening.",
            Status::Stale => "Not applied: what the request was based on changed. For ask and useful intake (outcome Interrupted without SIGINT), another writer stopped the run; result shows what the run recorded before it was stopped.",
            Status::Deferred => "Not applied: a guard is active, either an open edit session on a target note or a Working run. conflict[].kind tells which (Note or Run).",
            Status::NeedsReview => "Not applied: owner review is required.",
            Status::Rejected => "Refused by a safeguard or by current state: not found, invalid request (including an ask or useful intake refused before contacting a provider, and a text over a shared-service field limit), key conflict, unsafe or occupied path, open edit session.",
            Status::Failed => "A dependency failed: provider, conversion, helper, semantic index, or an ask run ended Failed.",
            Status::Interrupted => "SIGINT was acknowledged. The requested change was not committed, unless result shows a recorded partial run (ask). A fresh data directory may have been initialised.",
            Status::Terminated => "SIGTERM was acknowledged (machine mode). The requested change was not committed, unless result shows a recorded partial run (ask) or details.hard is true: then the process stopped without reaching a safe point and may have committed. A fresh data directory may have been initialised.",
        }
    }
    pub(crate) fn action(self) -> &'static str {
        match self {
            Status::Ok => "Continue.",
            Status::Internal => "Report it. Retry only if retry is replay, converge or read, or after checking state. For refuse, read state first: the retry performs the change only if the first attempt did not.",
            Status::Usage => "Fix the invocation; do not retry unchanged.",
            Status::Stale => "Re-read (conflict has current versions; for a stopped run, read --id <result.run>) and decide. Under a --key, that key stays bound to the request that went stale: make any new attempt with new inputs and a new key.",
            Status::Deferred => "Wait, within your own time budget, until the guard clears (recovery no longer lists the session, or read --id RUN no longer shows the run Working); then report it to the owner if it remains. Never discard a session you did not open. Once the guard clears, retry the same argv (same --key) if retry is replay or converge; if retry is duplicate, a retry is a new operation and the first attempt stays behind as an unapplied candidate.",
            Status::NeedsReview => "Surface it to the owner; do not retry.",
            Status::Rejected => "Fix the cause, or wait for the state to change; do not retry unchanged against unchanged state.",
            Status::Failed => "May succeed later or with other input; check retry first.",
            Status::Interrupted => "Re-run if still wanted. Nothing was committed, so the re-run attempts the change afresh and applies it at most once. After ask or useful intake, a re-run is another model turn on top of the recorded partial run.",
            Status::Terminated => "Re-run if still wanted, as for interrupted. When details.hard is true, first read state as the retry class describes.",
        }
    }
}

/// Stable error codes; `message` is human text and not part of the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Code {
    MissingDataDir,
    InvalidDataDir,
    MissingCommand,
    UnknownCommand,
    MalformedOption,
    MissingValue,
    RepeatedOption,
    UnknownOption,
    ConflictingOptions,
    MissingOption,
    InvalidValue,
    InvalidInput,
    NotFound,
    InvalidRequest,
    KeyConflict,
    UnsafeDirectory,
    DestinationExists,
    EditSessionOpen,
    CredentialsNotConfigured,
    Unavailable,
    ProviderFailed,
    ConversionFailed,
    Timeout,
    Busy,
    Interrupted,
    Terminated,
    StorageError,
    IoError,
    Internal,
}
impl Code {
    pub(crate) const ALL: [Code; 29] = [
        Code::MissingDataDir,
        Code::InvalidDataDir,
        Code::MissingCommand,
        Code::UnknownCommand,
        Code::MalformedOption,
        Code::MissingValue,
        Code::RepeatedOption,
        Code::UnknownOption,
        Code::ConflictingOptions,
        Code::MissingOption,
        Code::InvalidValue,
        Code::InvalidInput,
        Code::NotFound,
        Code::InvalidRequest,
        Code::KeyConflict,
        Code::UnsafeDirectory,
        Code::DestinationExists,
        Code::EditSessionOpen,
        Code::CredentialsNotConfigured,
        Code::Unavailable,
        Code::ProviderFailed,
        Code::ConversionFailed,
        Code::Timeout,
        Code::Busy,
        Code::Interrupted,
        Code::Terminated,
        Code::StorageError,
        Code::IoError,
        Code::Internal,
    ];
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Code::MissingDataDir => "missing_data_dir",
            Code::InvalidDataDir => "invalid_data_dir",
            Code::MissingCommand => "missing_command",
            Code::UnknownCommand => "unknown_command",
            Code::MalformedOption => "malformed_option",
            Code::MissingValue => "missing_value",
            Code::RepeatedOption => "repeated_option",
            Code::UnknownOption => "unknown_option",
            Code::ConflictingOptions => "conflicting_options",
            Code::MissingOption => "missing_option",
            Code::InvalidValue => "invalid_value",
            Code::InvalidInput => "invalid_input",
            Code::NotFound => "not_found",
            Code::InvalidRequest => "invalid_request",
            Code::KeyConflict => "key_conflict",
            Code::UnsafeDirectory => "unsafe_directory",
            Code::DestinationExists => "destination_exists",
            Code::EditSessionOpen => "edit_session_open",
            Code::CredentialsNotConfigured => "credentials_not_configured",
            Code::Unavailable => "unavailable",
            Code::ProviderFailed => "provider_failed",
            Code::ConversionFailed => "conversion_failed",
            Code::Timeout => "timeout",
            Code::Busy => "busy",
            Code::Interrupted => "interrupted",
            Code::Terminated => "terminated",
            Code::StorageError => "storage_error",
            Code::IoError => "io_error",
            Code::Internal => "internal",
        }
    }
    pub(crate) fn status(self) -> Status {
        match self {
            Code::MissingDataDir
            | Code::InvalidDataDir
            | Code::MissingCommand
            | Code::UnknownCommand
            | Code::MalformedOption
            | Code::MissingValue
            | Code::RepeatedOption
            | Code::UnknownOption
            | Code::ConflictingOptions
            | Code::MissingOption
            | Code::InvalidValue
            | Code::InvalidInput => Status::Usage,
            Code::NotFound
            | Code::InvalidRequest
            | Code::KeyConflict
            | Code::UnsafeDirectory
            | Code::DestinationExists
            | Code::EditSessionOpen
            | Code::CredentialsNotConfigured => Status::Rejected,
            Code::Unavailable
            | Code::ProviderFailed
            | Code::ConversionFailed
            | Code::Timeout
            | Code::Busy => Status::Failed,
            Code::Interrupted => Status::Interrupted,
            Code::Terminated => Status::Terminated,
            Code::StorageError | Code::IoError | Code::Internal => Status::Internal,
        }
    }
    pub(crate) fn meaning(self) -> &'static str {
        match self {
            Code::MissingDataDir => "The first argument (after --json) is not --data-dir, or the data directory has no value.",
            Code::InvalidDataDir => "The data directory is not absolute.",
            Code::MissingCommand => "No command after the data directory.",
            Code::UnknownCommand => "The command is not in the table.",
            Code::MalformedOption => "A token where --name was expected, or an argument that is not valid UTF-8. details.position is 1-based among the arguments after the program name, counting a leading --json.",
            Code::MissingValue => "--name with no value.",
            Code::RepeatedOption => "An option given twice.",
            Code::UnknownOption => "An option the command does not allow; details.allowed lists the allowed ones.",
            Code::ConflictingOptions => "Options that cannot be combined: --NAME with --NAME-file, --key on a variant that refuses it, --project with --thread, --helper off with helper options, --edit-note with --record-decision.",
            Code::MissingOption => "A required option is absent.",
            Code::InvalidValue => "A bad integer, enum, yes/no value, key format, relative path, or malformed --actions JSON. details.expected is human text.",
            Code::InvalidInput => "A file or stdin text problem (details.reason), an unreadable or empty intake source, or an unreadable or malformed --preview-file.",
            Code::NotFound => "A record or operation does not exist; read returning null in machine mode.",
            Code::InvalidRequest => "The shared service refused the request as invalid for the current state, including a backup or export destination whose parent folder does not exist. For ask and useful intake (details.kind tool_rejected or model_refused): ToolRejected and ModelRefused are almost always pre-provider refusals from begin/Selection::validate; finish can also return ToolRejected after provider contact when the run record is inconsistent; the class stays invalid_request/refuse.",
            Code::KeyConflict => "The same key was used for a different request; use a new key for a new change.",
            Code::UnsafeDirectory => "Old or foreign format, missing database, symlink, non-empty restore target, or backup inside the workspace.",
            Code::DestinationExists => "The backup or export destination already exists; choose a new path.",
            Code::EditSessionOpen => "This note has an open unsaved edit session (details.note, details.base_version). Write to it only if your own successful begin-edit returned it; otherwise report it to the owner.",
            Code::CredentialsNotConfigured => "Machine mode: ask or a provider-backed intake on a workspace with no credential source (a workspace the CLI created and nobody configured). Nothing was committed and no provider was contacted. Run configure --provider NAME --model ID --effort LEVEL with --auth-file ABSOLUTE_FILE or --credentials-dir ABSOLUTE_DIR on the owner's instruction (details.provider names the selected provider).",
            Code::Unavailable => "A service is not available in this build or platform, for example semantic search, conversion for this format, or model installation.",
            Code::ProviderFailed => "ask or useful intake: any other AiErrorKind (none is known to reach Err from run at 771116f0; kept as a fallback); also a provider failure during document conversion. details.kind is the AiErrorKind in snake_case.",
            Code::ConversionFailed => "Document conversion failed; details.reason names the failure.",
            Code::Timeout => "Document conversion exceeded its time limit, or the --timeout deadline passed before the command committed (details.deadline_seconds; details.hard is true when the process stopped without reaching a safe point and may have committed).",
            Code::Busy => "SQLite stayed locked past the 5 s busy timeout. This transaction did not commit, although earlier steps of the command may have.",
            Code::Interrupted => "SIGINT was observed before the change was committed.",
            Code::Terminated => "SIGTERM was observed before the change was committed (machine mode; details.hard is true when the process stopped without reaching a safe point and may have committed).",
            Code::StorageError => "Unexpected storage or record-encoding failure.",
            Code::IoError => "Unexpected I/O failure on the CLI's own paths.",
            Code::Internal => "Unexpected failure, or a committed review whose attention cleanup needs a retry (details.applied or details.dismissed: retry the same command).",
        }
    }
}

/// Echo rule for argv tokens in error details.
pub(crate) fn echo(token: &str) -> Option<&str> {
    let mut bytes = token.bytes();
    let first = bytes.next()?;
    (first.is_ascii_lowercase()
        && token.len() <= 40
        && bytes.all(|b| b.is_ascii_lowercase() || b == b'-'))
    .then_some(token)
}

#[derive(Debug)]
pub(crate) struct CliError {
    pub(crate) code: Code,
    pub(crate) message: String,
    pub(crate) details: Box<Map<String, Value>>,
    /// Default-mode text when it differs from `message`.
    pub(crate) default_text: Option<String>,
}
impl CliError {
    pub(crate) fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Box::default(),
            default_text: None,
        }
    }
    pub(crate) fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
    /// Adds `details.option` when the token follows the echo rule.
    pub(crate) fn with_option(self, option: &str) -> Self {
        match echo(option) {
            Some(option) => self.with("option", option),
            None => self,
        }
    }
    pub(crate) fn missing_option(option: &str) -> Self {
        Self::new(Code::MissingOption, format!("Missing --{option}")).with_option(option)
    }
    /// `message` says what is expected; `details.expected` repeats it.
    pub(crate) fn invalid_value(option: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(Code::InvalidValue, message.clone())
            .with_option(option)
            .with("expected", message)
    }
    pub(crate) fn conflicting(options: &[&str], message: impl Into<String>) -> Self {
        Self::new(Code::ConflictingOptions, message).with("options", options)
    }
    pub(crate) fn invalid_input(option: &str, reason: &str, message: impl Into<String>) -> Self {
        Self::new(Code::InvalidInput, message)
            .with_option(option)
            .with("reason", reason)
    }
    pub(crate) fn interrupted() -> Self {
        Self::new(Code::Interrupted, "Interrupted before operation")
    }
    /// `key_conflict` names the caller's `--key`, which matches the key pattern.
    pub(crate) fn bind_key(mut self, key: Option<&str>) -> Self {
        if let (Code::KeyConflict, Some(key)) = (self.code, key) {
            self.details.insert("key".into(), key.into());
        }
        self
    }
    pub(crate) fn default_message(&self) -> &str {
        self.default_text.as_deref().unwrap_or(&self.message)
    }
}

impl From<brn_threads_app::Error> for CliError {
    fn from(error: brn_threads_app::Error) -> Self {
        use brn_threads_app::Error;
        let message = error.to_string();
        // Exhaustive on purpose: a new core variant must be mapped here.
        let code = match &error {
            Error::Sqlite(e) if busy(e.sqlite_extended_error_code()) => Code::Busy,
            Error::Sqlite(_) | Error::Json(_) => Code::StorageError,
            Error::Io(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Self::new(Code::DestinationExists, message).with("option", "destination");
            }
            Error::Io(_) => Code::IoError,
            Error::UnsafeDirectory(_) => Code::UnsafeDirectory,
            Error::Invalid(_) => Code::InvalidRequest,
            Error::ImmutableRequest => Code::KeyConflict,
            Error::Missing(_) => Code::NotFound,
        };
        Self::new(code, message)
    }
}
impl From<AppError> for CliError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Core(e) => e.into(),
            AppError::Search(e) => e.into(),
            AppError::Io(e) => e.into(),
            AppError::Json(e) => Self::new(Code::StorageError, e.to_string()),
            AppError::Invalid(message) => Self::new(Code::InvalidRequest, message),
            AppError::Unavailable(message) => Self::new(Code::Unavailable, message),
        }
    }
}
impl From<RetrievalError> for CliError {
    fn from(error: RetrievalError) -> Self {
        let message = error.to_string();
        let code = match &error {
            RetrievalError::Invalid(_) => Code::InvalidRequest,
            RetrievalError::Unavailable(_) | RetrievalError::Stale => Code::Unavailable,
            RetrievalError::Cancelled => Code::Interrupted,
            RetrievalError::Sql(e) if busy(e.sqlite_extended_error_code()) => Code::Busy,
            RetrievalError::Corrupt(_) | RetrievalError::Sql(_) | RetrievalError::Native(_) => {
                Code::StorageError
            }
            RetrievalError::Io(_) => Code::IoError,
            // ModelInstall exists only when brn-retrieval's native feature is on,
            // which feature unification can enable without this crate's feature.
            #[allow(unreachable_patterns)]
            _ => return Self::new(Code::Unavailable, message).with("reason", "model_install"),
        };
        Self::new(code, message)
    }
}
/// SQLITE_BUSY or SQLITE_LOCKED (any extended form): the lock outlasted the busy timeout.
fn busy(extended_code: Option<std::ffi::c_int>) -> bool {
    extended_code.is_some_and(|code| matches!(code & 0xff, 5 | 6))
}
/// snake_case of a unit variant's CamelCase `Debug` name.
pub(crate) fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
fn conversion_error(error: &ConversionError, message: String) -> CliError {
    match error {
        ConversionError::Cancelled => CliError::new(Code::Interrupted, message),
        ConversionError::Timeout => CliError::new(Code::Timeout, message),
        ConversionError::Unavailable => CliError::new(Code::Unavailable, message),
        ConversionError::Budget
        | ConversionError::Invalid
        | ConversionError::PdfTextInvalid
        | ConversionError::PdfRenderInvalid
        | ConversionError::PdfPreviewInvalid
        | ConversionError::Protocol
        | ConversionError::Io => CliError::new(Code::ConversionFailed, message)
            .with("reason", snake(&format!("{error:?}"))),
    }
}
impl From<ConversionFailure> for CliError {
    fn from(failure: ConversionFailure) -> Self {
        let message = failure.to_string();
        let reason = |reason: &str| {
            Self::new(Code::ConversionFailed, message.clone()).with("reason", reason)
        };
        match &failure {
            ConversionFailure::Intake(e) => conversion_error(e, message),
            ConversionFailure::Provider(e) => Self::new(Code::ProviderFailed, message.clone())
                .with("kind", snake(&format!("{:?}", e.kind))),
            ConversionFailure::Interrupted => Self::new(Code::Interrupted, message),
            ConversionFailure::Timeout => Self::new(Code::Timeout, message),
            ConversionFailure::InvalidCandidate => reason("invalid_candidate"),
            ConversionFailure::InvalidPlanFormat(_) => reason("invalid_plan_format"),
            ConversionFailure::Budget => reason("budget"),
            ConversionFailure::Worker => reason("worker"),
        }
    }
}
/// An intake `--source` read: caller input, so usage codes.
pub(crate) fn source_read_error(error: ConversionError) -> CliError {
    let message = error.to_string();
    let reason = match error {
        ConversionError::Io => "unreadable",
        ConversionError::Budget => "empty_or_too_large",
        ConversionError::Invalid => "invalid",
        _ => return conversion_error(&error, message),
    };
    CliError::invalid_input("source", reason, message)
}
/// `brn_ai::threads::run` refused or failed before producing a terminal.
pub(crate) fn run_error(error: brn_ai::AiError) -> CliError {
    use brn_ai::AiErrorKind;
    let message = format!("Provider run failed: {:?}", error.kind);
    let kind = snake(&format!("{:?}", error.kind));
    match error.kind {
        AiErrorKind::ToolRejected | AiErrorKind::ModelRefused => {
            CliError::new(Code::InvalidRequest, message).with("kind", kind)
        }
        AiErrorKind::Storage => CliError::new(Code::StorageError, message),
        _ => CliError::new(Code::ProviderFailed, message).with("kind", kind),
    }
}
/// A failed `begin-edit`: an open session on the note is reported by note and
/// base version, never by session ID (owner decision B3 part 1); the
/// default-mode text stays the original error.
pub(crate) const EDIT_SESSION_OPEN: &str = "This note has an open unsaved edit session (details.note, details.base_version). Write to it only if your own successful begin-edit returned it; otherwise report it to the owner.";
pub(crate) fn begin_edit_error(
    store: &Store,
    note: &str,
    error: brn_threads_app::Error,
) -> CliError {
    if matches!(error, brn_threads_app::Error::Sqlite(_)) {
        if let Ok(sessions) = store.recovery_buffers() {
            if let Some(session) = sessions.iter().find(|s| s.note == note) {
                let mut open = CliError::new(Code::EditSessionOpen, EDIT_SESSION_OPEN)
                    .with("note", session.note.clone())
                    .with("base_version", session.base_version);
                open.default_text = Some(error.to_string());
                return open;
            }
        }
    }
    error.into()
}
/// Machine mode: a provider run needs a configured credential source. A
/// workspace the CLI created has none until `configure`; default mode keeps
/// today's path.
pub(crate) fn require_credentials(
    inv: &Invocation,
    settings: &brn_threads_app::WorkspaceSettings,
) -> Result<(), CliError> {
    if inv.machine && settings.auth_file.is_none() && settings.credentials_dir.is_none() {
        return Err(CliError::new(
            Code::CredentialsNotConfigured,
            "No provider credentials are configured for this workspace; run configure --provider … with --auth-file ABS or --credentials-dir ABS",
        )
        .with("provider", settings.provider.clone()));
    }
    Ok(())
}
/// `backup` and `export` destinations are caller input.
pub(crate) fn destination_error(destination: &std::path::Path, error: AppError) -> CliError {
    let not_found = match &error {
        AppError::Io(e) | AppError::Core(brn_threads_app::Error::Io(e)) => {
            e.kind() == std::io::ErrorKind::NotFound
        }
        _ => false,
    };
    let absolute = std::env::current_dir()
        .map(|dir| dir.join(destination))
        .unwrap_or_else(|_| destination.to_path_buf());
    if not_found
        && absolute
            .parent()
            .is_some_and(|parent| matches!(parent.try_exists(), Ok(false)))
    {
        return CliError::new(Code::InvalidRequest, error.to_string())
            .with("option", "destination")
            .with("reason", "parent_missing");
    }
    match error {
        AppError::Invalid(message)
            if destination.is_absolute() && matches!(destination.try_exists(), Ok(true)) =>
        {
            CliError::new(Code::DestinationExists, message).with("option", "destination")
        }
        AppError::Io(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            CliError::new(Code::DestinationExists, e.to_string()).with("option", "destination")
        }
        other => other.into(),
    }
}
impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(Code::Internal, error.to_string())
    }
}
impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::new(Code::IoError, error.to_string())
    }
}
impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self::new(Code::InvalidRequest, message)
    }
}
impl From<&str> for CliError {
    fn from(message: &str) -> Self {
        Self::new(Code::InvalidRequest, message)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Changed {
    pub(crate) id: String,
    pub(crate) kind: Option<String>,
    pub(crate) version: u64,
    pub(crate) archived: bool,
    pub(crate) created: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Conflict {
    pub(crate) id: String,
    pub(crate) kind: Option<String>,
    pub(crate) version: Option<u64>,
    pub(crate) archived: Option<bool>,
    pub(crate) open_edit: bool,
}
/// What command arms and `fill_conflicts` know beyond the legacy result.
#[derive(Debug, Default)]
pub(crate) struct Annotations {
    pub(crate) operation: Option<String>,
    pub(crate) changed: Vec<Changed>,
    pub(crate) conflict: Option<Vec<Conflict>>,
    /// The stop reason seen when the result was classified: `Some(None)` for none.
    pub(crate) stop: Option<Option<&'static str>>,
    /// The envelope `page` of a paged result.
    pub(crate) page: Option<Value>,
}

/// The `RecordData` variant name of an encoded record's `data`.
fn kind_of(record: &Value) -> Option<String> {
    record["data"]
        .as_object()
        .and_then(|data| data.keys().next().cloned())
}
/// One entry per encoded receipt write (`{before, after}`).
pub(crate) fn changed(writes: &Value) -> Vec<Changed> {
    writes
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|write| {
            let after = &write["after"];
            Some(Changed {
                id: after["id"].as_str()?.to_owned(),
                kind: kind_of(after),
                version: after["version"].as_u64()?,
                archived: after["archived"].as_bool().unwrap_or(false),
                created: write["before"].is_null(),
            })
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub(crate) struct Classified {
    pub(crate) status: Status,
    pub(crate) outcome: Option<String>,
    pub(crate) operation: Option<String>,
    pub(crate) changed: Vec<Changed>,
}

fn apply_status(outcome: &str) -> Status {
    match outcome {
        "Applied" => Status::Ok,
        "Stale" | "Superseded" => Status::Stale,
        "Deferred" => Status::Deferred,
        "NeedsReview" => Status::NeedsReview,
        _ => Status::Internal,
    }
}
fn save_status(outcome: &str) -> Status {
    match outcome {
        "Saved" => Status::Ok,
        "Stale" | "GenerationMismatch" | "Closed" => Status::Stale,
        "Deferred" => Status::Deferred,
        _ => Status::Internal,
    }
}
fn label(value: &Value) -> Option<String> {
    value["outcome"].as_str().map(str::to_owned)
}

/// Status, outcome, operation and changed records of a successful execution.
pub(crate) fn classify(
    inv: &Invocation,
    value: &Value,
    notes: &Annotations,
    stop: Option<&str>,
) -> Classified {
    let ok = |outcome: Option<String>| Classified {
        status: Status::Ok,
        outcome,
        operation: None,
        changed: vec![],
    };
    match spec::receipt_for(inv) {
        ReceiptAt::None => match inv.command.as_str() {
            "update-buffer" => {
                let outcome = label(value);
                let session = &value["session"];
                let identical = session["closed"] == false
                    && inv
                        .args
                        .get("generation")
                        .and_then(|g| g.parse::<u64>().ok())
                        == session["generation"].as_u64()
                    && inv.args.get("text").map(String::as_str) == session["markdown"].as_str();
                let status = match outcome.as_deref() {
                    Some("Updated") => Status::Ok,
                    Some("Ignored") if identical => Status::Ok,
                    _ => Status::Stale,
                };
                Classified {
                    status,
                    ..ok(outcome)
                }
            }
            "discard" => {
                let outcome = label(value);
                let status = match outcome.as_deref() {
                    Some("Discarded") => Status::Ok,
                    _ => Status::Stale,
                };
                Classified {
                    status,
                    ..ok(outcome)
                }
            }
            "dismiss" => ok(label(value)),
            // `read --limit` returns a NoteReadOutcome; a later page of a changed note is Stale.
            "read" if inv.args.contains_key("limit") => {
                let outcome = label(value);
                let status = match outcome.as_deref() {
                    Some("Stale") => Status::Stale,
                    _ => Status::Ok,
                };
                Classified {
                    status,
                    ..ok(outcome)
                }
            }
            // A keyed seeded knowledge-check reports its message operation.
            _ => Classified {
                operation: notes.operation.clone(),
                ..ok(None)
            },
        },
        // A converging command that found its state already held recorded nothing.
        ReceiptAt::Root if value["outcome"] == "Unchanged" => ok(Some("Unchanged".into())),
        ReceiptAt::Root => Classified {
            status: Status::Ok,
            outcome: Some("Applied".into()),
            operation: value["operation"].as_str().map(str::to_owned),
            changed: changed(&value["writes"]),
        },
        ReceiptAt::Arm => {
            let outcome = label(value).unwrap_or_else(|| "Applied".into());
            Classified {
                status: apply_status(&outcome),
                outcome: Some(outcome),
                operation: notes.operation.clone(),
                changed: notes.changed.clone(),
            }
        }
        ReceiptAt::Field => {
            let outcome = label(value).unwrap_or_default();
            let (status, done) = if inv.command == "save" {
                (save_status(&outcome), outcome == "Saved")
            } else {
                (apply_status(&outcome), outcome == "Applied")
            };
            let receipt = &value["receipt"];
            Classified {
                status,
                outcome: Some(outcome),
                operation: done
                    .then(|| receipt["operation"].as_str().map(str::to_owned))
                    .flatten(),
                changed: if done {
                    changed(&receipt["writes"])
                } else {
                    vec![]
                },
            }
        }
        ReceiptAt::NoteAndPointer => {
            let pointer = &value["pointer"];
            let outcome = label(pointer).unwrap_or_default();
            let mut written = changed(&json!([{"before": null, "after": value["note"]}]));
            if outcome == "Applied" {
                written.extend(changed(&pointer["receipt"]["writes"]));
            }
            Classified {
                status: apply_status(&outcome),
                outcome: Some(outcome),
                operation: None,
                changed: written,
            }
        }
        ReceiptAt::Terminal => {
            let terminal = value["terminal"].as_str().unwrap_or_default();
            let (status, outcome) = match terminal {
                "Completed" => (Status::Ok, "Completed"),
                "Interrupted" if stop == Some("sigint") => (Status::Interrupted, "Interrupted"),
                "Interrupted" if stop == Some("sigterm") => (Status::Terminated, "Interrupted"),
                "Interrupted" if stop == Some("deadline") => (Status::Failed, "Interrupted"),
                "Interrupted" => (Status::Stale, "Interrupted"),
                _ if terminal.starts_with("Failed") => (Status::Failed, "Failed"),
                _ => (Status::Internal, "Failed"),
            };
            Classified {
                status,
                ..ok(Some(outcome.into()))
            }
        }
    }
}

/// IDs the outcome names (`Stale.records`, `Deferred.guarded`,
/// `Superseded.run`, a session's note, a stopped run).
fn conflict_ids(inv: &Invocation, value: &Value) -> Vec<String> {
    let source = if inv.command == "topics-create" {
        &value["pointer"]
    } else {
        value
    };
    let mut ids: Vec<String> = ["records", "guarded"]
        .iter()
        .flat_map(|key| source[*key].as_array().into_iter().flatten())
        .filter_map(|id| id.as_str().map(str::to_owned))
        .collect();
    ids.extend(source["run"].as_str().map(str::to_owned));
    ids.extend(source["session"]["note"].as_str().map(str::to_owned));
    if inv.command == "read" && source["outcome"] == "Stale" {
        ids.extend(source["id"].as_str().map(str::to_owned));
    }
    ids
}

/// Machine mode only: records the SIGINT snapshot and, for stale and
/// deferred outcomes, the current state of the records the outcome names.
/// Read-only; a failed lookup leaves `conflict` empty.
pub(crate) fn fill_conflicts(
    workspace: &Workspace,
    inv: &Invocation,
    value: &Value,
    notes: &mut Annotations,
) {
    if !inv.machine {
        return;
    }
    let stop = crate::stop::reason();
    notes.stop = Some(stop);
    let status = classify(inv, value, notes, stop).status;
    if !matches!(status, Status::Stale | Status::Deferred) {
        return;
    }
    let lookup = || -> Option<Vec<Conflict>> {
        let sessions = workspace.store.recovery_buffers().ok()?;
        conflict_ids(inv, value)
            .into_iter()
            .map(|id| {
                let record = workspace.store.record(&id).ok()?;
                let open_edit = sessions.iter().any(|s| s.note == id);
                Some(match record {
                    Some(record) => Conflict {
                        kind: kind_of(&serde_json::to_value(&record).ok()?),
                        version: Some(record.version),
                        archived: Some(record.archived),
                        open_edit,
                        id,
                    },
                    None => Conflict {
                        id,
                        kind: None,
                        version: None,
                        archived: None,
                        open_edit: false,
                    },
                })
            })
            .collect()
    };
    notes.conflict = Some(lookup().unwrap_or_default());
}

pub(crate) struct Envelope<'a> {
    pub(crate) command: Option<&'a str>,
    pub(crate) status: Status,
    pub(crate) outcome: Option<String>,
    pub(crate) operation: Option<String>,
    pub(crate) key: Option<&'a str>,
    pub(crate) retry: Class,
    pub(crate) changed: Vec<Changed>,
    pub(crate) conflict: Option<Vec<Conflict>>,
    pub(crate) result: Value,
    pub(crate) error: Option<CliError>,
    /// `{limit, shown, cursor, total}` on a page, else null.
    pub(crate) page: Option<Value>,
    /// Why the command stopped early: `sigint`, `sigterm` or `deadline`.
    pub(crate) stop: Option<&'static str>,
}
impl Envelope<'_> {
    pub(crate) fn to_value(&self) -> Value {
        // Usage and rejected refusals repeat unchanged while their cause stays.
        let retry = match self.status {
            Status::Usage | Status::Rejected => Class::Refuse,
            _ => self.retry,
        };
        let conflict = match self.status {
            Status::Stale | Status::Deferred => Some(self.conflict.clone().unwrap_or_default()),
            _ => None,
        };
        json!({
            "contract": CONTRACT,
            "command": self.command,
            "ok": self.status == Status::Ok,
            "status": self.status.as_str(),
            "exit": self.status.exit(),
            "outcome": self.outcome,
            "operation": self.operation,
            "key": self.key.filter(|key| spec::key_is_valid(key)),
            "retry": retry.as_str(),
            "changed": self.changed,
            "conflict": conflict,
            "result": self.result,
            "error": self.error.as_ref().map(|e| json!({
                "code": e.code.as_str(), "message": e.message, "details": e.details,
            })),
            "page": self.page,
            "stop": self.stop,
        })
    }
}

/// Writes one line to stdout; a closed stdout never panics.
pub(crate) fn emit_line(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}
/// Writes exactly `text` to stderr; a closed stderr never panics.
pub(crate) fn emit_err(text: &str) {
    let mut err = std::io::stderr().lock();
    let _ = err.write_all(text.as_bytes());
    let _ = err.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_ai::AiErrorKind;
    use std::collections::BTreeMap;

    fn inv(command: &str, args: &[(&str, &str)]) -> Invocation {
        Invocation {
            data: "/nonexistent/brn-contract".into(),
            command: command.into(),
            args: args
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            machine: true,
            key: args
                .iter()
                .find(|(k, _)| *k == "key")
                .map(|(_, v)| v.to_string()),
            pending_inputs: vec![],
            timeout: None,
        }
    }
    fn write(id: &str, version: u64, before: bool) -> Value {
        let after = json!({"id":id,"version":version,"archived":false,"data":{"Note":{}}});
        json!({"before": if before { after.clone() } else { Value::Null }, "after": after})
    }
    fn receipt(operation: &str) -> Value {
        json!({"operation":operation,"writes":[write("n:0", 2, true)]})
    }
    fn status(command: &str, args: &[(&str, &str)], value: Value, sigint: bool) -> Status {
        let stop = sigint.then_some("sigint");
        classify(&inv(command, args), &value, &Annotations::default(), stop).status
    }

    #[test]
    fn every_domain_outcome_maps_to_its_status() {
        for (label, expected) in [
            ("Applied", Status::Ok),
            ("Stale", Status::Stale),
            ("Superseded", Status::Stale),
            ("Deferred", Status::Deferred),
            ("NeedsReview", Status::NeedsReview),
        ] {
            for command in [
                "apply",
                "keep-in-brn",
                "delegation",
                "configure",
                "pause",
                "resume",
                "topics-choose",
                "undo",
            ] {
                let value = json!({"outcome":label,"receipt":receipt("op")});
                assert_eq!(
                    status(command, &[], value, false),
                    expected,
                    "{command} {label}"
                );
            }
            let intake = json!({"outcome":label,"receipt":receipt("op")});
            assert_eq!(
                status("intake", &[("intent", "full")], intake, false),
                expected
            );
            let topics = json!({"note":{"id":"t:0","version":1,"archived":false,"data":{"Note":{}}},
                "pointer":{"outcome":label}});
            assert_eq!(status("topics-create", &[], topics, false), expected);
        }
        for (label, expected) in [
            ("Saved", Status::Ok),
            ("Stale", Status::Stale),
            ("GenerationMismatch", Status::Stale),
            ("Closed", Status::Stale),
            ("Deferred", Status::Deferred),
        ] {
            let value = json!({"outcome":label,"receipt":receipt("op"),"session":{"note":"n:0"}});
            assert_eq!(status("save", &[], value, false), expected, "{label}");
        }
        let session = |closed: bool, generation: u64, markdown: &str| json!({"note":"n:0","closed":closed,"generation":generation,"markdown":markdown});
        let args = [("generation", "2"), ("text", "same")];
        let buffer = |outcome: &str, s: Value| json!({"outcome":outcome,"session":s});
        assert_eq!(
            status(
                "update-buffer",
                &args,
                buffer("Updated", session(false, 2, "same")),
                false
            ),
            Status::Ok
        );
        assert_eq!(
            status(
                "update-buffer",
                &args,
                buffer("Ignored", session(false, 2, "same")),
                false
            ),
            Status::Ok
        );
        assert_eq!(
            status(
                "update-buffer",
                &args,
                buffer("Ignored", session(true, 2, "same")),
                false
            ),
            Status::Stale
        );
        assert_eq!(
            status(
                "update-buffer",
                &args,
                buffer("Ignored", session(false, 3, "same")),
                false
            ),
            Status::Stale
        );
        assert_eq!(
            status(
                "update-buffer",
                &args,
                buffer("Ignored", session(false, 2, "other")),
                false
            ),
            Status::Stale
        );
        assert_eq!(
            status(
                "discard",
                &[],
                buffer("Discarded", session(true, 2, "")),
                false
            ),
            Status::Ok
        );
        assert_eq!(
            status(
                "discard",
                &[],
                buffer("Ignored", session(true, 2, "")),
                false
            ),
            Status::Stale
        );
        assert_eq!(
            status(
                "dismiss",
                &[],
                json!({"outcome":"Dismissed","operation":"op"}),
                false
            ),
            Status::Ok
        );
        let root = receipt("op");
        let created = classify(&inv("queue", &[]), &root, &Annotations::default(), None);
        assert_eq!(created.status, Status::Ok);
        assert_eq!(created.outcome.as_deref(), Some("Applied"));
        assert_eq!(created.operation.as_deref(), Some("op"));
        assert_eq!(created.changed.len(), 1);
        for read in ["notes", "read", "topics", "init", "begin-edit", "export"] {
            let classified = classify(&inv(read, &[]), &Value::Null, &Annotations::default(), None);
            assert_eq!(
                classified,
                Classified {
                    status: Status::Ok,
                    outcome: None,
                    operation: None,
                    changed: vec![]
                }
            );
        }
    }

    #[test]
    fn run_terminals_tell_sigint_from_another_writer() {
        let run = |terminal: &str| json!({"run":"r1","terminal":terminal});
        for command in [("ask", vec![]), ("intake", vec![("intent", "useful")])] {
            let (name, args) = command;
            assert_eq!(status(name, &args, run("Completed"), false), Status::Ok);
            assert_eq!(status(name, &args, run("Completed"), true), Status::Ok);
            assert_eq!(
                status(name, &args, run("Interrupted"), true),
                Status::Interrupted
            );
            assert_eq!(
                status(name, &args, run("Interrupted"), false),
                Status::Stale
            );
            assert_eq!(
                status(name, &args, run("Failed:RateLimited"), false),
                Status::Failed
            );
            let classified = classify(
                &inv(name, &args),
                &run("Interrupted"),
                &Annotations::default(),
                None,
            );
            assert_eq!(classified.outcome.as_deref(), Some("Interrupted"));
            assert_eq!((classified.operation, classified.changed), (None, vec![]));
        }
        assert_eq!(conflict_ids(&inv("ask", &[]), &run("Interrupted")), ["r1"]);
        let interrupted = Envelope {
            command: Some("ask"),
            status: Status::Interrupted,
            outcome: Some("Interrupted".into()),
            operation: None,
            key: None,
            retry: Class::Duplicate,
            changed: vec![],
            conflict: Some(vec![]),
            result: run("Interrupted"),
            error: None,
            page: None,
            stop: None,
        }
        .to_value();
        assert_eq!(interrupted["exit"], 130);
        assert_eq!(interrupted["conflict"], Value::Null);
    }

    #[test]
    fn receipts_give_operation_and_changed_only_when_applied() {
        let full = inv("intake", &[("intent", "full")]);
        let applied = classify(
            &full,
            &json!({"outcome":"Applied","receipt":receipt("op")}),
            &Annotations::default(),
            None,
        );
        assert_eq!(applied.operation.as_deref(), Some("op"));
        assert!(!applied.changed[0].created);
        let stale = classify(
            &full,
            &json!({"outcome":"Stale","records":["n:0"]}),
            &Annotations::default(),
            None,
        );
        assert_eq!((stale.operation, stale.changed), (None, vec![]));
        let useful = inv("intake", &[("intent", "useful")]);
        assert_eq!(spec::receipt_for(&useful), ReceiptAt::Terminal);
        let note = json!({"id":"t:0","version":1,"archived":false,"data":{"Note":{}}});
        let deferred = json!({"note":note,"pointer":{"outcome":"Deferred","guarded":["run-1"]}});
        let topics = classify(
            &inv("topics-create", &[]),
            &deferred,
            &Annotations::default(),
            None,
        );
        assert_eq!(topics.status, Status::Deferred);
        assert_eq!(topics.operation, None);
        assert_eq!(topics.changed.len(), 1);
        assert!(topics.changed[0].created);
        assert_eq!(topics.changed[0].kind.as_deref(), Some("Note"));
        assert_eq!(
            conflict_ids(&inv("topics-create", &[]), &deferred),
            ["run-1"]
        );
        let pointer = json!({"note":note,"pointer":{"outcome":"Applied","receipt":{"operation":"p","writes":[write("s:0", 3, true)]}}});
        assert_eq!(
            classify(
                &inv("topics-create", &[]),
                &pointer,
                &Annotations::default(),
                None
            )
            .changed
            .len(),
            2
        );
        let notes = Annotations {
            operation: Some("comp".into()),
            changed: vec![],
            ..Default::default()
        };
        let undo = classify(
            &inv("undo", &[]),
            &json!({"outcome":"Deferred","guarded":["n:0"]}),
            &notes,
            None,
        );
        assert_eq!(
            (undo.status, undo.operation.as_deref()),
            (Status::Deferred, Some("comp"))
        );
        let save_stale = json!({"outcome":"Stale","session":{"note":"n:0"}});
        assert_eq!(conflict_ids(&inv("save", &[]), &save_stale), ["n:0"]);
        assert_eq!(
            conflict_ids(
                &inv("apply", &[]),
                &json!({"outcome":"Superseded","run":"r"})
            ),
            ["r"]
        );
    }

    fn mapped(error: impl Into<CliError>) -> (Code, Value) {
        let error = error.into();
        (error.code, Value::Object(*error.details))
    }

    #[test]
    fn every_error_variant_maps_to_one_code_and_details_shape() {
        use brn_threads_app::Error;
        let io = |kind| std::io::Error::new(kind, "synthetic");
        let json_error = serde_json::from_str::<Value>("{").unwrap_err();
        assert_eq!(
            mapped(Error::Json(json_error)),
            (Code::StorageError, json!({}))
        );
        assert_eq!(
            mapped(Error::Io(io(std::io::ErrorKind::Other))),
            (Code::IoError, json!({}))
        );
        assert_eq!(
            mapped(Error::Io(io(std::io::ErrorKind::AlreadyExists))),
            (Code::DestinationExists, json!({"option":"destination"}))
        );
        assert_eq!(
            mapped(Error::UnsafeDirectory("x".into())),
            (Code::UnsafeDirectory, json!({}))
        );
        assert_eq!(
            mapped(Error::Invalid("x".into())),
            (Code::InvalidRequest, json!({}))
        );
        assert_eq!(
            mapped(Error::Missing("x".into())),
            (Code::NotFound, json!({}))
        );
        assert_eq!(
            mapped(Error::ImmutableRequest),
            (Code::KeyConflict, json!({}))
        );
        let keyed = CliError::from(Error::ImmutableRequest).bind_key(Some("k1"));
        assert_eq!(Value::Object(*keyed.details), json!({"key":"k1"}));
        assert_eq!(
            CliError::from(Error::ImmutableRequest)
                .bind_key(None)
                .details
                .len(),
            0
        );
        // A real SQLite error: a second session on one note.
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(dir.path().join("data")).unwrap();
        let note = workspace.new_note("N", "x").unwrap();
        workspace.store.begin_edit(&note.id, 1).unwrap();
        let sqlite = workspace.store.begin_edit(&note.id, 1).unwrap_err();
        assert!(matches!(sqlite, Error::Sqlite(_)));
        let text = sqlite.to_string();
        let open = begin_edit_error(&workspace.store, &note.id, sqlite);
        assert_eq!(open.code, Code::EditSessionOpen);
        assert_eq!(open.default_message(), text);
        assert!(!open.message.contains("Save or discard"));
        assert_eq!(
            Value::Object(*open.details),
            json!({"note":note.id,"base_version":1})
        );
        let other = workspace.new_note("M", "y").unwrap();
        workspace.store.begin_edit(&other.id, 1).unwrap();
        let sqlite = workspace.store.begin_edit(&other.id, 1).unwrap_err();
        assert_eq!(
            begin_edit_error(&workspace.store, "unrelated", sqlite).code,
            Code::StorageError
        );

        assert_eq!(
            mapped(AppError::Invalid("x".into())),
            (Code::InvalidRequest, json!({}))
        );
        assert_eq!(
            mapped(AppError::Unavailable("x".into())),
            (Code::Unavailable, json!({}))
        );
        assert_eq!(
            mapped(AppError::Io(io(std::io::ErrorKind::Other))),
            (Code::IoError, json!({}))
        );
        let json_error = serde_json::from_str::<Value>("{").unwrap_err();
        assert_eq!(
            mapped(AppError::Json(json_error)),
            (Code::StorageError, json!({}))
        );
        assert_eq!(
            mapped(AppError::Core(Error::Missing("x".into()))),
            (Code::NotFound, json!({}))
        );
        assert_eq!(
            mapped(AppError::Search(RetrievalError::Stale)),
            (Code::Unavailable, json!({}))
        );

        assert_eq!(
            mapped(RetrievalError::Invalid("x")),
            (Code::InvalidRequest, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Unavailable("x")),
            (Code::Unavailable, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Stale),
            (Code::Unavailable, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Cancelled),
            (Code::Interrupted, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Corrupt("x")),
            (Code::StorageError, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Native("x".into())),
            (Code::StorageError, json!({}))
        );
        assert_eq!(
            mapped(RetrievalError::Io(io(std::io::ErrorKind::Other))),
            (Code::IoError, json!({}))
        );

        for (error, code, reason) in [
            (ConversionError::Cancelled, Code::Interrupted, None),
            (ConversionError::Timeout, Code::Timeout, None),
            (ConversionError::Unavailable, Code::Unavailable, None),
            (
                ConversionError::Budget,
                Code::ConversionFailed,
                Some("budget"),
            ),
            (
                ConversionError::Invalid,
                Code::ConversionFailed,
                Some("invalid"),
            ),
            (
                ConversionError::PdfTextInvalid,
                Code::ConversionFailed,
                Some("pdf_text_invalid"),
            ),
            (
                ConversionError::PdfRenderInvalid,
                Code::ConversionFailed,
                Some("pdf_render_invalid"),
            ),
            (
                ConversionError::PdfPreviewInvalid,
                Code::ConversionFailed,
                Some("pdf_preview_invalid"),
            ),
            (
                ConversionError::Protocol,
                Code::ConversionFailed,
                Some("protocol"),
            ),
            (ConversionError::Io, Code::ConversionFailed, Some("io")),
        ] {
            let details = reason.map_or(json!({}), |reason| json!({"reason":reason}));
            assert_eq!(mapped(ConversionFailure::Intake(error)), (code, details));
        }
        let provider = brn_ai::AiError {
            kind: AiErrorKind::RateLimited,
            retry_after_seconds: None,
            detail: None,
        };
        assert_eq!(
            mapped(ConversionFailure::Provider(provider)),
            (Code::ProviderFailed, json!({"kind":"rate_limited"}))
        );
        assert_eq!(
            mapped(ConversionFailure::Interrupted),
            (Code::Interrupted, json!({}))
        );
        assert_eq!(
            mapped(ConversionFailure::Timeout),
            (Code::Timeout, json!({}))
        );
        for (failure, reason) in [
            (ConversionFailure::InvalidCandidate, "invalid_candidate"),
            (
                ConversionFailure::InvalidPlanFormat("shape".into()),
                "invalid_plan_format",
            ),
            (ConversionFailure::Budget, "budget"),
            (ConversionFailure::Worker, "worker"),
        ] {
            assert_eq!(
                mapped(failure),
                (Code::ConversionFailed, json!({"reason":reason}))
            );
        }
        for (error, reason) in [
            (ConversionError::Io, "unreadable"),
            (ConversionError::Budget, "empty_or_too_large"),
            (ConversionError::Invalid, "invalid"),
        ] {
            let read = source_read_error(error);
            assert_eq!(read.code, Code::InvalidInput);
            assert_eq!(
                Value::Object(*read.details),
                json!({"option":"source","reason":reason})
            );
        }
        assert_eq!(
            source_read_error(ConversionError::Cancelled).code,
            Code::Interrupted
        );
        assert_eq!(
            mapped(serde_json::from_str::<Value>("{").unwrap_err()),
            (Code::Internal, json!({}))
        );
        assert_eq!(
            mapped(io(std::io::ErrorKind::Other)),
            (Code::IoError, json!({}))
        );
        assert_eq!(mapped("x"), (Code::InvalidRequest, json!({})));
        assert_eq!(mapped(String::from("x")), (Code::InvalidRequest, json!({})));
    }

    #[test]
    fn destinations_are_classified_as_caller_input() {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let missing = dir.path().join("missing-parent/dest");
        let not_found = || AppError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert_eq!(
            mapped(destination_error(&missing, not_found())),
            (
                Code::InvalidRequest,
                json!({"option":"destination","reason":"parent_missing"})
            )
        );
        let core = AppError::Core(brn_threads_app::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "gone",
        )));
        assert_eq!(
            mapped(destination_error(&missing, core)).0,
            Code::InvalidRequest
        );
        assert_eq!(
            mapped(destination_error(&dir.path().join("x"), not_found())).0,
            Code::IoError
        );
        let exists =
            AppError::Invalid("Export destination already exists; choose a new snapshot".into());
        assert_eq!(
            mapped(destination_error(dir.path(), exists)),
            (Code::DestinationExists, json!({"option":"destination"}))
        );
        let relative = AppError::Invalid("Choose an absolute fresh export destination".into());
        assert_eq!(
            mapped(destination_error(std::path::Path::new("rel"), relative)).0,
            Code::InvalidRequest
        );
    }

    #[test]
    fn committed_review_cleanup_is_internal_and_keeps_the_replay_class() {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(dir.path().join("data")).unwrap();
        let receipt = workspace.new_note_with_key("n1", "N", "x").unwrap();
        let op = receipt.operation.as_str();
        let apply = inv("apply", &[("operation", op)]);
        let error = crate::review_error(
            &workspace,
            &apply,
            "applied",
            AppError::Invalid("cleanup".into()),
        );
        assert_eq!(mapped(error), (Code::Internal, json!({"applied":true})));
        assert_eq!(spec::retry_for(&apply), Class::Replay);
        let unknown = inv("apply", &[("operation", "missing")]);
        let error = crate::review_error(
            &workspace,
            &unknown,
            "applied",
            AppError::Invalid("no".into()),
        );
        assert_eq!(error.code, Code::InvalidRequest);
        let envelope = Envelope {
            command: Some("apply"),
            status: error.code.status(),
            outcome: None,
            operation: None,
            key: None,
            retry: spec::retry_for(&unknown),
            changed: vec![],
            conflict: None,
            result: Value::Null,
            error: Some(error),
            page: None,
            stop: None,
        };
        assert_eq!(envelope.to_value()["retry"], "refuse");
        let dismiss = inv("dismiss", &[("operation", op)]);
        let error = crate::review_error(
            &workspace,
            &dismiss,
            "dismissed",
            AppError::Invalid("cleanup".into()),
        );
        assert_eq!(mapped(error), (Code::Internal, json!({"dismissed":true})));
    }

    #[test]
    fn every_ai_error_kind_has_one_run_error_mapping() {
        let kinds = [
            AiErrorKind::QuoteNotFound,
            AiErrorKind::QuoteAmbiguous,
            AiErrorKind::QuoteOccurrenceInvalid,
            AiErrorKind::ReconnectNeeded,
            AiErrorKind::CodeExpired,
            AiErrorKind::RateLimited,
            AiErrorKind::Network,
            AiErrorKind::ModelRefused,
            AiErrorKind::InvalidToolUse,
            AiErrorKind::ToolLimitReached,
            AiErrorKind::TimeLimitReached,
            AiErrorKind::UnsafeCredentials,
            AiErrorKind::ToolRejected,
            AiErrorKind::IndexStale,
            AiErrorKind::Storage,
            AiErrorKind::Other,
        ];
        for kind in kinds {
            let error = run_error(brn_ai::AiError {
                kind,
                retry_after_seconds: None,
                detail: None,
            });
            assert_eq!(error.message, format!("Provider run failed: {kind:?}"));
            let snake_kind = serde_json::to_value(kind).unwrap();
            assert_eq!(json!(snake(&format!("{kind:?}"))), snake_kind);
            let (code, details) = match kind {
                AiErrorKind::ToolRejected | AiErrorKind::ModelRefused => {
                    (Code::InvalidRequest, json!({"kind":snake_kind}))
                }
                AiErrorKind::Storage => (Code::StorageError, json!({})),
                _ => (Code::ProviderFailed, json!({"kind":snake_kind})),
            };
            assert_eq!(
                (error.code, Value::Object(*error.details)),
                (code, details),
                "{kind:?}"
            );
        }
        assert_eq!(Code::InvalidRequest.status().exit(), 6);
        assert_eq!(Code::StorageError.status().exit(), 1);
        assert_eq!(Code::ProviderFailed.status().exit(), 7);
    }

    #[test]
    fn statuses_and_codes_are_one_to_one_and_complete() {
        let mut exits: Vec<u8> = Status::ALL.iter().map(|s| s.exit()).collect();
        exits.dedup();
        assert_eq!(exits, [0, 1, 2, 3, 4, 5, 6, 7, 130, 143]);
        let names: std::collections::BTreeSet<_> = Status::ALL.iter().map(|s| s.as_str()).collect();
        assert_eq!(names.len(), 10);
        let codes: std::collections::BTreeSet<_> = Code::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(codes.len(), Code::ALL.len());
        for code in Code::ALL {
            assert!(Status::ALL.contains(&code.status()));
            assert!(!code.meaning().contains("opening the provider client"));
        }
    }

    #[test]
    fn envelopes_refuse_on_usage_and_rejected_and_never_echo_bad_keys() {
        for status in Status::ALL {
            for class in Class::ALL {
                let value = Envelope {
                    command: Some("note-create"),
                    status,
                    outcome: None,
                    operation: None,
                    key: Some("bad key"),
                    retry: class,
                    changed: vec![],
                    conflict: None,
                    result: Value::Null,
                    error: None,
                    page: None,
                    stop: None,
                }
                .to_value();
                let expected = match status {
                    Status::Usage | Status::Rejected => "refuse",
                    _ => class.as_str(),
                };
                assert_eq!(value["retry"], expected);
                assert_eq!(value["key"], Value::Null);
                assert_eq!(value["ok"], status == Status::Ok);
                assert_eq!(
                    value["conflict"].is_array(),
                    matches!(status, Status::Stale | Status::Deferred)
                );
                assert_eq!(value.as_object().unwrap().len(), 15);
            }
        }
        for (token, echoed) in [
            ("text", true),
            ("text-file", true),
            ("Text", false),
            ("secret value", false),
            ("t3xt", false),
            ("-x", false),
            (&"a".repeat(41), false),
            (&"a".repeat(40), true),
            ("", false),
        ] {
            assert_eq!(echo(token).is_some(), echoed, "{token}");
        }
        assert_eq!(CliError::missing_option("SECRET").details.len(), 0);
    }
}
