use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationId(pub(crate) String);
impl OperationId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Creation identities are fixed before preparation, so a lost response is replayable.
    pub fn creation_id(&self, index: usize) -> String {
        format!("{}:{index}", self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub title: String,
    pub markdown: String,
    pub protected: bool,
    pub confirmed: bool,
    pub superseded_by: Option<String>,
    pub import: Option<ImportCoverage>,
}
impl Note {
    pub fn working(title: impl Into<String>, markdown: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            markdown: markdown.into(),
            protected: false,
            confirmed: false,
            superseded_by: None,
            import: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportCoverage {
    pub source: String,
    pub full_note: bool,
    pub gaps: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadState {
    Open,
    Resolved,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub state: ThreadState,
    pub attention: Vec<Attention>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionKind {
    Question,
    Review,
    Conflict,
    Blocker,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attention {
    pub kind: AttentionKind,
    pub reason: String,
    pub record: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionState {
    Suggested,
    Open,
    Waiting,
    Done,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub description: String,
    pub state: ActionState,
    pub internal: bool,
    pub evidence: Option<String>,
    pub due: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub locator: String,
    pub label: String,
    pub observed_at: String,
    pub external_version: Option<String>,
    pub outcome: String,
    pub gaps: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub note: String,
    pub source: Option<String>,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub relation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub note: String,
    pub base_version: u64,
    pub quote: String,
    pub range: Option<(usize, usize)>,
    pub mapped_version: Option<u64>,
    pub mapped_range: Option<(usize, usize)>,
    pub body: String,
    pub unresolved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub thread: String,
    pub role: String,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    Working,
    Interrupted,
    Completed,
    Cancelled,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub thread: String,
    pub provider: String,
    pub model: String,
    pub guide_identity: String,
    pub budget: u64,
    pub effort: String,
    pub loaded_skills: Vec<String>,
    pub fence: u64,
    pub state: RunState,
    pub progress: String,
    pub operations: Vec<OperationId>,
    pub sources: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub provider: String,
    pub model: String,
    pub effort: String,
    pub auth_file: Option<String>,
    pub credentials_dir: Option<String>,
    pub maintenance: bool,
    pub review_first: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordData {
    Note(Note),
    Thread(Thread),
    Action(Action),
    Source(Source),
    Asset(Asset),
    Link(Link),
    Comment(Comment),
    Message(Message),
    Run(Run),
    Settings(WorkspaceSettings),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub version: u64,
    pub archived: bool,
    pub data: RecordData,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Put {
    pub id: String,
    pub expected_version: Option<u64>,
    pub archived: bool,
    pub data: RecordData,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionInput {
    pub record: String,
    pub version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub reason: String,
    pub writes: Vec<Put>,
    pub inputs: Vec<RevisionInput>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prepared {
    pub operation: OperationId,
    pub request_hash: String,
    pub request: ChangeRequest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Written {
    pub before: Option<Record>,
    pub after: Record,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub operation: OperationId,
    pub request_hash: String,
    pub reason: String,
    pub writes: Vec<Written>,
    pub authority: String,
    pub committed_at: String,
    pub undo_of: Option<OperationId>,
    pub needs_refresh: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied(Receipt),
    Deferred { guarded: Vec<String> },
    Stale { records: Vec<String> },
    NeedsReview { denied: Vec<String> },
    Superseded { run: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    EditContent,
    Protect,
    Unprotect,
    Archive,
    Supersede,
    Confirm,
    Commitment,
    CompleteHuman,
    Undo,
    ManageRun,
    Configure,
}
/// This type deliberately has no Deserialize implementation. Only trusted host
/// code constructs it from a real owner action or bound instruction.
#[derive(Debug, Clone)]
pub struct HostAuthority {
    pub(crate) actor: String,
    pub(crate) scope: Scope,
    pub(crate) run_binding: Option<(String, u64)>,
    pub(crate) settings_binding: Option<(String, u64)>,
}
#[derive(Debug, Clone)]
pub(crate) enum Scope {
    Owner,
    Maintenance,
    Runtime {
        run: String,
    },
    Grant {
        operation: OperationId,
        targets: BTreeSet<String>,
        capabilities: BTreeSet<Capability>,
        instruction: String,
    },
}
impl HostAuthority {
    pub fn owner(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Owner,
            run_binding: None,
            settings_binding: None,
        }
    }
    pub fn maintenance(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Maintenance,
            run_binding: None,
            settings_binding: None,
        }
    }
    /// Host coordinator authority to manage one concrete Run record only.
    pub fn runtime(actor: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Runtime { run: run_id.into() },
            run_binding: None,
            settings_binding: None,
        }
    }
    pub fn instruction(
        actor: impl Into<String>,
        instruction: impl Into<String>,
        operation: OperationId,
        targets: impl IntoIterator<Item = String>,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Grant {
                operation,
                targets: targets.into_iter().collect(),
                capabilities: capabilities.into_iter().collect(),
                instruction: instruction.into(),
            },
            run_binding: None,
            settings_binding: None,
        }
    }
    /// Host-coordinated steering/cancellation fence, excluded from model arguments.
    pub fn for_run(mut self, run: impl Into<String>, fence: u64) -> Self {
        self.run_binding = Some((run.into(), fence));
        self
    }
    /// Exact persisted configuration captured by the host, including account paths.
    /// Receipt replay remains valid after later configuration changes.
    pub fn for_settings(mut self, id: impl Into<String>, version: u64) -> Self {
        self.settings_binding = Some((id.into(), version));
        self
    }
    pub(crate) fn allows(&self, operation: &OperationId, id: &str, capability: Capability) -> bool {
        match &self.scope {
            Scope::Owner => true,
            Scope::Maintenance => false,
            Scope::Runtime { run } => capability == Capability::ManageRun && run == id,
            Scope::Grant {
                operation: bound,
                targets,
                capabilities,
                ..
            } => {
                capability != Capability::ManageRun
                    && capability != Capability::Configure
                    && bound == operation
                    && targets.contains(id)
                    && capabilities.contains(&capability)
            }
        }
    }
    pub(crate) fn audit(&self) -> String {
        let grant = match &self.scope {
            Scope::Owner => format!("owner:{}", self.actor),
            Scope::Maintenance => format!("maintenance:{}", self.actor),
            Scope::Runtime { run } => format!("runtime:{}:{run}", self.actor),
            Scope::Grant {
                instruction,
                targets,
                capabilities,
                ..
            } => format!(
                "instruction:{}:{}:{targets:?}:{capabilities:?}",
                self.actor, instruction
            ),
        };
        match &self.run_binding {
            Some((run, fence)) => format!("{grant};run:{run};fence:{fence}"),
            None => grant,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditSession {
    pub id: String,
    pub note: String,
    pub base_version: u64,
    pub generation: u64,
    pub markdown: String,
    pub closed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferOutcome {
    Updated(EditSession),
    Ignored(EditSession),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved(Receipt),
    Stale(EditSession),
    GenerationMismatch(EditSession),
    Closed(EditSession),
    Deferred { guarded: Vec<String> },
}

impl Asset {
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&self.bytes))
    }
}
