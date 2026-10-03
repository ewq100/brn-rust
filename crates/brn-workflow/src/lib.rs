//! Shared authoritative workflow used by the desktop and headless driver.
pub mod ai_tools;
pub mod app;
pub mod app_worker;
pub mod chat_worker;
mod comments;
mod drafts;
pub mod editor;
pub mod error;
pub mod library;
pub mod models;
#[cfg(all(test, feature = "native-retrieval"))]
mod models_tests;
#[cfg(test)]
mod simple_worker_tests;
pub use brn_ai::{
    AccountStatus, AiError, AiErrorKind, Auth, HistoryPair, LoginPrompt, ModelOption, NoteEntry,
    NotePage, Passage, Provider, ReadTools, Selection, ToolNote, ToolSearch,
};
pub mod notes;
pub mod vault;
pub mod worker;
use brn_retrieval::{Document, Evidence, Index, Profile};
use brn_store::{Approval, OperationStatus, Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

pub use brn_retrieval::Profile as SearchProfile;
pub use brn_store::Approval as SearchApproval;
pub use brn_store::anchors::{MAX_EDIT_STEPS, MAX_TRACE_REPLACEMENT_BYTES};
pub use brn_store::work::{WorkConversation, WorkTurn, WorkTurnStatus};
pub use brn_store::workspace_mode::WorkspaceMode;

pub fn workspace_mode(data: &Path) -> Result<WorkspaceMode> {
    Ok(brn_store::workspace_mode::classify(data)?)
}
pub use brn_store::{
    AmbiguityReason, AnchorProjection, AnchorState, ChatTurn, CommentAnchorSnapshot,
    CommentCapture, CommentCreated, CommentStatus, CommentStatusChange, CommentStatusChanged,
    Draft, DraftComment, DraftCommentView, DraftComments, DraftRevision, DraftStamp,
    DraftWriteWithComments, EditTrace, ImportResult, MAX_COMMENT_BODY_BYTES, MAX_DRAFT_BYTES,
    OriginalAnchor, RecoveryReference, SourceDocument, TextEdit, apply_edit, derive_edit,
    map_anchor, replay_trace,
};
pub type Result<T> = std::result::Result<T, error::WorkflowError>;
pub use error::{ErrorKind, WorkflowError};
pub use notes::eligibility::{SourceCurrentState, SourceStateSummary};
use notes::eligibility::{note_workflow_error, stale};

/// Whether the provider outcome is confirmed or unobservable. Transport loss
/// is never proof of cancellation, so it stays `Unknown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderOutcome {
    Unknown,
    Confirmed(OperationStatus),
}

/// An ask failure carrying the typed category, the exact human message and
/// every identifier and outcome fact known when the failure happened.
#[derive(Debug, Clone)]
pub struct AskFailure {
    pub kind: ErrorKind,
    pub message: String,
    /// Operation id of the attempt (caller-supplied or freshly generated).
    pub operation_id: Uuid,
    /// Session established during this run: created here, or the caller's
    /// validated argument. `None` before any session exists.
    pub session_id: Option<Uuid>,
    /// Durable local record state known to exist (`None`: no turn record known).
    pub recorded_status: Option<OperationStatus>,
    pub provider_outcome: ProviderOutcome,
    /// Historical receipt, never a successful current answer.
    pub receipt: Option<Box<ChatTurn>>,
}

/// Honesty rule for a durably recorded turn status: completed and failed are
/// provider-confirmed terminal states; an interrupted record is ambiguous
/// (written either from a server-confirmed interruption or after uncertain
/// transport loss), and pending/running records say nothing about the provider.
pub fn provider_outcome_of_recorded(status: OperationStatus) -> ProviderOutcome {
    match status {
        OperationStatus::Completed => ProviderOutcome::Confirmed(OperationStatus::Completed),
        OperationStatus::Failed => ProviderOutcome::Confirmed(OperationStatus::Failed),
        OperationStatus::Interrupted | OperationStatus::Pending | OperationStatus::Running => {
            ProviderOutcome::Unknown
        }
    }
}

impl From<AskFailure> for WorkflowError {
    fn from(f: AskFailure) -> Self {
        Self {
            kind: f.kind,
            message: f.message,
        }
    }
}

/// Pre-connect style failure: no record, provider outcome unknown.
fn ask_failure(
    kind: ErrorKind,
    message: impl Into<String>,
    op: Uuid,
    session: Option<Uuid>,
) -> AskFailure {
    AskFailure {
        kind,
        message: message.into(),
        operation_id: op,
        session_id: session,
        recorded_status: None,
        provider_outcome: ProviderOutcome::Unknown,
        receipt: None,
    }
}

pub const MAX_IMPORT_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub model_dir: Option<PathBuf>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ActiveIndex {
    format: u32,
    directory: String,
    fingerprint: String,
    #[serde(default)]
    eligibility_fingerprint: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub query: String,
    pub profile: Profile,
    pub evidence: Vec<Evidence>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub id: Uuid,
    pub has_thread: bool,
    pub turns: usize,
}
/// Read-only snapshot of derived index state for status surfaces.
#[derive(Debug, Clone)]
pub struct WorkspaceStatus {
    pub active_present: bool,
    pub active_fingerprint: Option<String>,
    pub index_error: Option<String>,
}
/// Whether this build compiles the native retrieval backend.
pub fn native_retrieval_compiled() -> bool {
    cfg!(feature = "native-retrieval")
}
pub struct Workspace {
    store: Store,
    notes: notes::NoteState,
    path: PathBuf,
    config: Config,
    index: Option<Index>,
    active: Option<ActiveIndex>,
    index_error: Option<String>,
    pub recovered_operations: usize,
}
fn error(e: impl std::fmt::Display) -> WorkflowError {
    WorkflowError::msg(e.to_string())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn docs_fingerprint(docs: &[Document]) -> Result<String> {
    serde_json::to_vec(docs).map(|x| digest(&x)).map_err(error)
}
fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(WorkflowError::cancelled())
    } else {
        Ok(())
    }
}
impl Workspace {
    pub fn open(path: &Path, config: Config) -> Result<Self> {
        if !path.is_absolute() {
            return Err("data directory must be absolute".into());
        }
        let path = fs::canonicalize(path).map_err(error)?;
        let (store, recovery) = Store::open(&path).map_err(WorkflowError::from)?;
        // Indexes are derived. Delay opening until search so corrupt derived data cannot prevent history access.
        let active_path = path.join("active-index.json");
        let (active, index_error) = if active_path.exists() {
            match fs::read(active_path)
                .map_err(error)
                .and_then(|bytes| serde_json::from_slice::<ActiveIndex>(&bytes).map_err(error))
            {
                Ok(active) => (Some(active), None),
                Err(_) => (
                    None,
                    Some("invalid active index pointer; rebuild index".to_string()),
                ),
            }
        } else {
            (None, None)
        };
        Ok(Self {
            store,
            notes: notes::NoteState::default(),
            path,
            config,
            index: None,
            active,
            index_error,
            recovered_operations: recovery.interrupted_operations,
        })
    }
    pub fn workspace_status(&self) -> WorkspaceStatus {
        WorkspaceStatus {
            active_present: self.active.is_some(),
            active_fingerprint: self.active.as_ref().map(|a| a.fingerprint.clone()),
            index_error: self.index_error.clone(),
        }
    }
    pub fn import_file(
        &mut self,
        cancel: &AtomicBool,
        op: Uuid,
        path: &Path,
        approval: Approval,
    ) -> Result<ImportResult> {
        cancelled(cancel)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "md" | "txt") {
            return Err("select a .md or .txt file".into());
        }
        let selected = self.managed_note_for_path(path)?;
        let requested = path.to_path_buf();
        let path = if selected.is_some() {
            path.to_path_buf()
        } else {
            fs::canonicalize(path).map_err(error)?
        };
        if let Some(id) = selected.or(self.managed_note_for_path(&path)?) {
            let (receipt, changed) = self
                .note_snapshot_operation(
                    op,
                    &brn_store::notes::NoteSearchRequest::Import {
                        note_id: id,
                        path: requested,
                        approval,
                    },
                )
                .map_err(note_workflow_error)?;
            return Ok(ImportResult {
                source_id: receipt.source_id,
                version_id: receipt.version_id,
                changed,
            });
        }
        let mut file = File::open(&path).map_err(error)?;
        let meta = file.metadata().map_err(error)?;
        if !meta.is_file() || meta.len() > MAX_IMPORT_BYTES as u64 {
            return Err("import requires a regular UTF-8 file up to 1 MiB".into());
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take((MAX_IMPORT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(error)?;
        if bytes.len() > MAX_IMPORT_BYTES
            || std::str::from_utf8(&bytes).is_err()
            || bytes.is_empty()
        {
            return Err("import requires nonempty UTF-8 up to 1 MiB".into());
        }
        let origin = path.to_str().ok_or("source path must be UTF-8")?;
        let title = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid source filename")?;
        cancelled(cancel)?;
        self.store
            .import_text(op, origin, title, &bytes, approval)
            .map_err(WorkflowError::from)
    }
    pub fn set_approval(
        &mut self,
        cancel: &AtomicBool,
        op: Uuid,
        source: Uuid,
        version: Uuid,
        approval: Approval,
    ) -> Result<()> {
        cancelled(cancel)?;
        self.store
            .reconcile_note_sources()
            .map_err(note_workflow_error)?;
        let associations = self
            .store
            .note_source_associations()
            .map_err(note_workflow_error)?;
        if let Some((id, _, shadowed)) = associations.iter().find(|(_, s, _)| *s == source) {
            if *shadowed {
                return Err(stale(
                    "shadowed imports cannot be independently approved; explicitly approve the managed note",
                ));
            }
            let request = brn_store::notes::NoteSearchRequest::SetApproval {
                note_id: *id,
                source_id: source,
                version_id: version,
                approval,
            };
            self.note_snapshot_operation(op, &request)
                .map_err(note_workflow_error)?;
            return Ok(());
        }
        self.store
            .set_approval(op, source, version, approval)
            .map_err(WorkflowError::from)
    }
    fn documents(&mut self) -> Result<Vec<Document>> {
        let mut docs: Vec<_> = self
            .source_projection()?
            .0
            .into_iter()
            .filter(|d| d.approval == Approval::Approved)
            .map(|d| {
                Ok(Document {
                    source_id: d.source_id.to_string(),
                    version_id: d.version_id.to_string(),
                    title: d.title,
                    text: String::from_utf8(d.bytes).map_err(error)?,
                    source_hash: format!(
                        "{:x}",
                        Sha256::digest(
                            self.store
                                .version(d.version_id)
                                .map_err(error)?
                                .ok_or("missing revision")?
                                .bytes
                        )
                    ),
                })
            })
            .collect::<Result<_>>()?;
        docs.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        Ok(docs)
    }
    pub fn build_index(
        &mut self,
        cancel: &AtomicBool,
        progress: impl FnMut(&str),
    ) -> Result<String> {
        cancelled(cancel)?;
        let docs = self.documents()?;
        if docs.is_empty() {
            return Err("import and approve at least one source for search".into());
        }
        let fingerprint = docs_fingerprint(&docs)?;
        let eligibility_fingerprint = self.eligibility_fingerprint(&docs)?;
        let root = self.path.join("indexes");
        fs::create_dir_all(&root).map_err(error)?;
        let directory = Uuid::new_v4().to_string();
        let candidate = Index::build(
            &root.join(&directory),
            &docs,
            self.config.model_dir.as_deref(),
            cancel,
            progress,
        )
        .map_err(WorkflowError::from)?;
        cancelled(cancel)?;
        let current = self.documents()?;
        if fingerprint != docs_fingerprint(&current)?
            || eligibility_fingerprint != self.eligibility_fingerprint(&current)?
        {
            return Err(WorkflowError::index_stale(
                "sources changed during indexing; rebuild",
            ));
        }
        let active = ActiveIndex {
            format: 1,
            directory,
            fingerprint,
            eligibility_fingerprint: Some(eligibility_fingerprint),
        };
        let tmp = self
            .path
            .join(format!(".active-index-{}.tmp", Uuid::new_v4()));
        let result: Result<()> = (|| {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .map_err(error)?;
            f.write_all(&serde_json::to_vec(&active).map_err(error)?)
                .map_err(error)?;
            f.sync_all().map_err(error)?;
            cancelled(cancel)?;
            let current = self.documents()?;
            if active.fingerprint != docs_fingerprint(&current)?
                || active.eligibility_fingerprint.as_deref()
                    != Some(self.eligibility_fingerprint(&current)?.as_str())
            {
                return Err(WorkflowError::index_stale(
                    "sources changed before index publication; rebuild",
                ));
            }
            fs::rename(&tmp, self.path.join("active-index.json")).map_err(error)?;
            File::open(&self.path)
                .and_then(|f| f.sync_all())
                .map_err(error)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result?;
        let generation = candidate.generation().to_string();
        self.index = Some(candidate);
        self.active = Some(active);
        self.index_error = None;
        Ok(generation)
    }
    pub fn search(&mut self, query: &str, profile: Profile) -> Result<SearchResult> {
        let docs = self.documents()?;
        let eligibility_fingerprint = self.eligibility_fingerprint(&docs)?;
        if let Some(e) = &self.index_error {
            return Err(WorkflowError::index_invalid(e.clone()));
        }
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| WorkflowError::index_missing("no active index; build it first"))?;
        if active.format != 1 || Uuid::parse_str(&active.directory).is_err() {
            return Err(WorkflowError::index_invalid(
                "invalid active index pointer; rebuild",
            ));
        }
        if active.fingerprint != docs_fingerprint(&docs)?
            || active
                .eligibility_fingerprint
                .as_ref()
                .is_some_and(|f| f != &eligibility_fingerprint)
        {
            let reasons = self
                .source_states()?
                .into_iter()
                .filter(|s| s.current_state != SourceCurrentState::Current)
                .map(|s| {
                    format!(
                        "{} {:?}: {}",
                        s.source_id,
                        s.current_state,
                        s.message.unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(WorkflowError::index_stale(format!(
                "index is stale after source changes; rebuild it; {reasons}"
            )));
        }
        if self.index.is_none() {
            self.index = Some(
                Index::open(&self.path.join("indexes").join(&active.directory))
                    .map_err(WorkflowError::from)?,
            );
        }
        if self.index.as_ref().unwrap().fingerprint() != active.fingerprint {
            return Err("index snapshot does not match active corpus; rebuild".into());
        }
        let evidence = self
            .index
            .as_mut()
            .unwrap()
            .search(query, profile, 5)
            .map_err(WorkflowError::from)?;
        for e in &evidence {
            self.validate_evidence(e)?;
        }
        let current = self.documents()?;
        if docs_fingerprint(&current)? != docs_fingerprint(&docs)?
            || self.eligibility_fingerprint(&current)? != eligibility_fingerprint
        {
            return Err(WorkflowError::index_stale(
                "sources changed before search return; rebuild",
            ));
        }
        Ok(SearchResult {
            query: query.into(),
            profile,
            evidence,
        })
    }
    pub fn validate_evidence(&mut self, evidence: &Evidence) -> Result<()> {
        self.validate_current_note_evidence(evidence)?;
        let source = Uuid::parse_str(&evidence.source_id).map_err(error)?;
        let version = Uuid::parse_str(&evidence.version_id).map_err(error)?;
        let doc = self
            .store
            .documents()
            .map_err(error)?
            .into_iter()
            .find(|d| {
                d.source_id == source && d.version_id == version && d.approval == Approval::Approved
            })
            .ok_or_else(|| stale("evidence is no longer current and approved"))?;
        let text = std::str::from_utf8(&doc.bytes).map_err(error)?;
        if digest(&doc.bytes) != evidence.source_hash
            || text.get(evidence.start_byte..evidence.end_byte) != Some(evidence.quote.as_str())
            || evidence.start_byte >= evidence.end_byte
        {
            return Err("evidence hash or quote mismatch".into());
        }
        Ok(())
    }
    pub fn sessions(&self) -> Result<Vec<SessionSummary>> {
        self.store
            .sessions()
            .map_err(error)?
            .into_iter()
            .map(|s| {
                Ok(SessionSummary {
                    id: s.id,
                    has_thread: s.thread_id.is_some(),
                    turns: self.store.turns(s.id).map_err(error)?.len(),
                })
            })
            .collect()
    }
    pub fn history(&self, session: Uuid) -> Result<Vec<ChatTurn>> {
        self.store.turns(session).map_err(error)
    }
    pub fn ask(
        &mut self,
        op: Uuid,
        session: Option<Uuid>,
        query: &str,
        profile: Profile,
        cancel: &AtomicBool,
        on_delta: impl FnMut(&str),
    ) -> Result<ChatTurn> {
        self.ask_detailed(op, session, query, profile, cancel, on_delta)
            .map_err(WorkflowError::from)
    }
    #[allow(clippy::too_many_arguments)] // Explicit cancellation and provider-entry guards serve different lifecycle boundaries.
    pub fn ask_guarded(
        &mut self,
        op: Uuid,
        session: Option<Uuid>,
        query: &str,
        profile: Profile,
        cancel: &AtomicBool,
        before_provider: impl FnMut() -> Result<()>,
        on_delta: impl FnMut(&str),
    ) -> Result<ChatTurn> {
        self.ask_full(
            op,
            session,
            query,
            profile,
            cancel,
            before_provider,
            on_delta,
        )
        .map_err(WorkflowError::from)
    }
    /// `ask` with structured failure context instead of a bare message.
    pub fn ask_detailed(
        &mut self,
        op: Uuid,
        session: Option<Uuid>,
        query: &str,
        profile: Profile,
        cancel: &AtomicBool,
        on_delta: impl FnMut(&str),
    ) -> std::result::Result<ChatTurn, AskFailure> {
        self.ask_full(op, session, query, profile, cancel, || Ok(()), on_delta)
    }
    /// Compatibility entry point; refuses before storage, guard or network work.
    #[allow(clippy::too_many_arguments)]
    pub fn ask_full(
        &mut self,
        op: Uuid,
        session: Option<Uuid>,
        _query: &str,
        _profile: Profile,
        _cancel: &AtomicBool,
        _before_provider: impl FnMut() -> Result<()>,
        _on_delta: impl FnMut(&str),
    ) -> std::result::Result<ChatTurn, AskFailure> {
        Err(ask_failure(
            ErrorKind::LegacyAiRetired,
            "legacy AI retired; use a separate simple workspace for new chat",
            op,
            session,
        ))
    }
}
pub fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Keyword => "keyword",
        Profile::Semantic => "semantic",
        Profile::Hybrid => "hybrid",
    }
}

#[cfg(test)]
mod ask_failure_tests {
    use super::*;
    use brn_store::Approval;

    #[test]
    fn provider_outcome_of_recorded_is_honest() {
        // Provider-confirmed terminal states.
        assert_eq!(
            provider_outcome_of_recorded(OperationStatus::Completed),
            ProviderOutcome::Confirmed(OperationStatus::Completed)
        );
        assert_eq!(
            provider_outcome_of_recorded(OperationStatus::Failed),
            ProviderOutcome::Confirmed(OperationStatus::Failed)
        );
        // An interrupted record is ambiguous: written either from a
        // server-confirmed interruption or after uncertain transport loss.
        assert_eq!(
            provider_outcome_of_recorded(OperationStatus::Interrupted),
            ProviderOutcome::Unknown
        );
        // Non-terminal records say nothing about the provider.
        assert_eq!(
            provider_outcome_of_recorded(OperationStatus::Pending),
            ProviderOutcome::Unknown
        );
        assert_eq!(
            provider_outcome_of_recorded(OperationStatus::Running),
            ProviderOutcome::Unknown
        );
    }

    #[test]
    fn retirement_precedes_operation_lookup_callbacks_and_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("launch.md");
        std::fs::write(
            &file,
            "The synthetic Aurora mission launches on Tuesday.\r\n",
        )
        .unwrap();
        let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
        let op = Uuid::new_v4();
        w.import_file(&AtomicBool::new(false), op, &file, Approval::Approved)
            .unwrap();
        let before = w.sessions().unwrap();
        let failure = w
            .ask_full(
                op,
                None,
                "When does Aurora launch?",
                Profile::Keyword,
                &AtomicBool::new(true),
                || panic!("retired entry must not invoke submission guard"),
                |_| panic!("retired entry must not emit deltas"),
            )
            .unwrap_err();
        assert_eq!(failure.kind, ErrorKind::LegacyAiRetired);
        assert_eq!(failure.operation_id, op);
        assert_eq!(failure.session_id, None);
        assert_eq!(failure.recorded_status, None);
        assert_eq!(failure.provider_outcome, ProviderOutcome::Unknown);
        assert!(failure.receipt.is_none());
        assert_eq!(w.sessions().unwrap().len(), before.len());
        assert_eq!(
            w.store.operation(op).unwrap().unwrap().status,
            OperationStatus::Completed
        );
        let fresh = Uuid::new_v4();
        assert_eq!(
            w.ask(
                fresh,
                None,
                "",
                Profile::Hybrid,
                &AtomicBool::new(false),
                |_| panic!()
            )
            .unwrap_err()
            .kind,
            ErrorKind::LegacyAiRetired
        );
        assert!(w.store.operation(fresh).unwrap().is_none());
    }
}
