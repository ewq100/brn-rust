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
    /// Host-owned visibility; absent means independently retained BRN knowledge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<String>,
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
            chat: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ImportReport>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStatus {
    Complete,
    Partial,
    Unchecked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportIssueKind {
    Limitation,
    Omission,
    Unverified,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportIssue {
    pub kind: ImportIssueKind,
    pub locator: String,
    pub detail: String,
}
impl ImportIssue {
    pub fn display(&self) -> String {
        let category = match self.kind {
            ImportIssueKind::Limitation => "Converter limitation",
            ImportIssueKind::Omission => "Detected omission",
            ImportIssueKind::Unverified => "Unverified coverage",
        };
        format!("{category} · {}: {}", self.locator, self.detail)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion: Option<DocumentConversionProvenance>,
    pub status: ImportStatus,
    pub issues: Vec<ImportIssue>,
}
impl ImportReport {
    pub fn summary(&self) -> &'static str {
        if self
            .issues
            .iter()
            .any(|issue| issue.kind == ImportIssueKind::Omission)
        {
            "Detected omissions · Partial"
        } else if self
            .issues
            .iter()
            .any(|issue| issue.kind == ImportIssueKind::Unverified)
        {
            "Coverage unverified"
        } else {
            match self.status {
                ImportStatus::Complete => "Source coverage checked",
                ImportStatus::Partial => "Partial · inspection required",
                ImportStatus::Unchecked => "Coverage not independently checked",
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadState {
    Open,
    Resolved,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    /// Sticky host-owned provenance: saved discussion may contain private project context.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub private_context: bool,
    /// Changes only with host context membership, never ordinary attention.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub context_epoch: u64,
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
    /// Explicit owner decision to continue an obligation independently.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub kept_outside_project: bool,
    /// Host-owned private retention; absent preserves existing general Actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<String>,
    pub description: String,
    pub state: ActionState,
    pub internal: bool,
    pub evidence: Option<String>,
    pub due: Option<String>,
    /// Reported responsible person or team, not an access-control identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Person, team or response needed before this obligation can progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_on: Option<String>,
    /// Calendar date to revisit the obligation; distinct from its due date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<String>,
}
impl Action {
    pub(crate) fn validate_follow_up(&self) -> crate::Result<()> {
        for (field, value) in [("owner", &self.owner), ("waiting on", &self.waiting_on)] {
            if value.as_ref().is_some_and(|text| {
                text.trim().is_empty() || text.len() > 512 || text.chars().any(char::is_control)
            }) {
                return Err(crate::Error::Invalid(format!(
                    "Action {field} must be a nonempty label of at most 512 bytes without control characters, or null"
                )));
            }
        }
        if let Some(date) = &self.follow_up {
            let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d");
            if date.len() != 10
                || parsed.is_err()
                || parsed.is_ok_and(|parsed| parsed.to_string() != *date)
            {
                return Err(crate::Error::Invalid(
                    "Action follow_up must be an exact calendar date YYYY-MM-DD, or null".into(),
                ));
            }
        }
        Ok(())
    }

    /// Facts defining the obligation, independent of progress or completion evidence.
    pub(crate) fn same_commitment(&self, other: &Self) -> bool {
        self.description == other.description
            && self.due == other.due
            && self.owner == other.owner
            && self.waiting_on == other.waiting_on
            && self.follow_up == other.follow_up
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Observations from one chat must not overwrite another context's evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<String>,
    pub locator: String,
    pub label: String,
    pub observed_at: String,
    pub external_version: Option<String>,
    pub outcome: String,
    pub gaps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ImportReport>,
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
    /// Exact trusted posted instruction, never inferred from matching prose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_claim: Option<String>,
    pub thread: String,
    pub provider: String,
    pub model: String,
    pub guide_identity: String,
    pub budget: u64,
    pub effort: String,
    pub loaded_skills: Vec<String>,
    /// Host-verified identities of instruction files actually delivered. No bodies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loaded_skill_details: Vec<SkillLoad>,
    pub fence: u64,
    pub state: RunState,
    pub progress: String,
    pub operations: Vec<OperationId>,
    pub sources: Vec<String>,
}
/// Canonical document engine choice; independent of model refinement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentConverter {
    #[default]
    Xberg,
    XbergBrn,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConversionSettings {
    pub converter: DocumentConverter,
    pub ai: bool,
}
impl DocumentConversionSettings {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConversionProvenance {
    pub settings: DocumentConversionSettings,
    pub engine_identity: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    #[serde(
        default,
        skip_serializing_if = "DocumentConversionSettings::is_default"
    )]
    pub document_conversion: DocumentConversionSettings,
    /// Owner configuration only; no invocation-local data grant is persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpPolicy>,
    /// Canonical owner policy; installation alone never enables a package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillBinding>,
    pub provider: String,
    pub model: String,
    pub effort: String,
    pub auth_file: Option<String>,
    pub credentials_dir: Option<String>,
    pub maintenance: bool,
    pub review_first: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub delegation_chosen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_directory: Option<String>,
    /// Appearance preference only; it never affects authority or invocations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// The root Topics note (D6): an ordinary note linking every topic page.
    /// Owner configuration only; titles can change, so the ID is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topics_note: Option<String>,
    /// The owner-chosen read-only helper model; `None` (the default) offers no
    /// `research` tool. The parent model never chooses or changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helper: Option<HelperSettings>,
}

/// One explicitly approved anonymous local connection, disabled by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPolicy {
    pub endpoint: String,
    /// Empty only for disabled configuration awaiting explicit discovery.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub server_digest: String,
    pub enabled: bool,
    pub tools: Vec<McpApprovedTool>,
    pub access: Vec<McpAccess>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpApprovedTool {
    pub name: String,
    pub metadata_digest: String,
    pub schema_digest: String,
    pub contract: McpInputContract,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpInputContract {
    Empty,
    Query,
}
/// General excludes private/Project context. Chat means that exact chat only;
/// Project means its current member chats, never another Project's policy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpAccess {
    General,
    Chat(String),
    Project(String),
}

/// Exact content approved by the owner in one scope. Workspace bindings apply
/// to general chats only; a project needs its own explicit installed binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillBinding {
    pub id: String,
    pub digest: String,
    pub project: Option<String>,
    pub enabled: bool,
    pub selected: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillPolicyChange {
    Set(SkillBinding),
    Select {
        id: Option<String>,
        digest: Option<String>,
        project: Option<String>,
    },
    Clear {
        id: String,
        project: Option<String>,
    },
    /// Disable this exact installed identity in every scope before removal.
    Revoke {
        id: String,
        digest: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillLoad {
    pub id: String,
    pub origin: String,
    pub project: Option<String>,
    pub package_digest: String,
    pub file: String,
    pub file_digest: String,
}
impl WorkspaceSettings {
    pub fn validate_skills(&self) -> crate::Result<()> {
        let mut identities = BTreeSet::new();
        let mut selections = BTreeSet::new();
        if self.skills.len() > 128 {
            return Err(crate::Error::Invalid(
                "at most 128 scoped skill bindings".into(),
            ));
        }
        for binding in &self.skills {
            if !skill_id_valid(&binding.id)
                || !skill_digest_valid(&binding.digest)
                || binding.project.as_ref().is_some_and(|p| {
                    p.trim().is_empty() || p.len() > 256 || p.chars().any(char::is_control)
                })
                || !identities.insert((&binding.project, &binding.id))
                || (binding.selected && (!binding.enabled || !selections.insert(&binding.project)))
            {
                return Err(crate::Error::Invalid(
                    "invalid, duplicate or conflicting scoped skill binding".into(),
                ));
            }
        }
        Ok(())
    }
}
pub fn skill_id_valid(id: &str) -> bool {
    let name = id.strip_prefix("installed:").unwrap_or(id);
    (1..=64).contains(&name.len())
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
pub fn skill_digest_valid(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// An explicit helper selection with its own explicit credential route. An
/// unavailable helper is never replaced by another model or account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperSettings {
    pub provider: String,
    pub model: String,
    pub effort: String,
    pub auth_file: Option<String>,
    pub credentials_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationChoice {
    MaintainOrdinaryNotes,
    ReviewFirst,
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
    ChatControl(ChatControl),
    Project(crate::Project),
    WritingSetup(crate::WritingSetup),
}

/// Host recovery/control state; model tools cannot edit or read this record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatControl {
    pub queue: Vec<QueuedMessage>,
    pub pending: Option<QueuedMessage>,
    pub paused: bool,
    pub quiet: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub thread: String,
    pub text: String,
    pub message: Option<String>,
    pub run: Option<String>,
    pub dispatch: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDraft {
    pub destination: String,
    pub generation: u64,
    pub text: String,
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
    /// Current reads must still be active at this version when the group commits.
    /// Historical citations retain provenance without asserting current truth.
    #[serde(default, skip_serializing_if = "InputExpectation::is_current")]
    pub expectation: InputExpectation,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputExpectation {
    /// Host precondition: current Thread epoch equals this revision's epoch.
    ChatContext,
    /// Omission is safe for runtime tools and preserves old request bytes/hashes.
    /// Unapplied old candidates therefore gain current-read validation.
    #[default]
    Current,
    Historical,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
impl InputExpectation {
    fn is_current(&self) -> bool {
        *self == Self::Current
    }
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
    pub(crate) chat_binding: Option<(String, bool)>,
    pub(crate) project_binding: Option<(String, Option<(String, u64)>)>,
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
            chat_binding: None,
            project_binding: None,
        }
    }
    pub fn maintenance(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Maintenance,
            run_binding: None,
            settings_binding: None,
            chat_binding: None,
            project_binding: None,
        }
    }
    /// Host coordinator authority to manage one concrete Run record only.
    pub fn runtime(actor: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            scope: Scope::Runtime { run: run_id.into() },
            run_binding: None,
            settings_binding: None,
            chat_binding: None,
            project_binding: None,
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
            chat_binding: None,
            project_binding: None,
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
    /// Trusted context boundary, independent of model arguments and delegation.
    pub fn for_chat(mut self, chat: &str, private_context: bool) -> Self {
        self.chat_binding = Some((chat.into(), private_context));
        self
    }
    /// Exact host project context; None binds a standalone conversation.
    pub fn for_project_context(mut self, chat: &str, project: Option<(String, u64)>) -> Self {
        self.project_binding = Some((chat.into(), project));
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
    /// Who may continue or close the session: the owner host (desktop),
    /// or the caller holding its token.
    #[serde(default)]
    pub holder: EditHolder,
}
/// Who holds an edit session. A caller-held session has a token binding;
/// every other session is held by the owner host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditHolder {
    /// Only the owner, in the BRN desktop app, continues or closes it.
    #[default]
    #[serde(rename = "desktop")]
    OwnerHost,
    /// The caller that received the session's token.
    #[serde(rename = "caller")]
    Caller,
}
/// The secret of a caller-held edit session: `brnet1_` and 64 lowercase hex.
/// Only its SHA-256 is stored. It has no `Serialize` or `Display`, and its
/// `Debug` is redacted, so it cannot reach an error, listing or log by accident.
#[derive(Clone)]
pub struct EditToken(String);
impl EditToken {
    pub const PREFIX: &'static str = "brnet1_";
    /// Accepts exactly the published pattern `^brnet1_[0-9a-f]{64}$`.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix(Self::PREFIX)?;
        (hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then(|| Self(text.to_owned()))
    }
    /// The token text, for handing it to the caller once.
    pub fn expose(&self) -> &str {
        &self.0
    }
    /// 244 random bits from two version-4 UUIDs (OS CSPRNG).
    pub(crate) fn generate() -> Self {
        let mut text = String::from(Self::PREFIX);
        for byte in uuid::Uuid::new_v4()
            .as_bytes()
            .iter()
            .chain(uuid::Uuid::new_v4().as_bytes())
        {
            text.push_str(&format!("{byte:02x}"));
        }
        Self(text)
    }
    /// The stored form: lowercase hex of SHA-256 of the token text.
    pub(crate) fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(self.0.as_bytes()))
    }
}
impl std::fmt::Debug for EditToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditToken(..)")
    }
}
/// A caller-held session and its token, returned once.
#[derive(Debug, Clone)]
pub struct IssuedEdit {
    pub session: EditSession,
    pub token: EditToken,
}
/// Why a presented token cannot write to a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTokenProblem {
    /// The session is caller-held, by another token.
    Mismatch,
    /// The session is held by the owner host; no token writes to it.
    OwnerHost,
}
impl std::fmt::Display for EditTokenProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mismatch => "the edit token does not match this edit session",
            Self::OwnerHost => {
                "this edit session is held by the owner in the BRN desktop app; only the owner can continue or close it"
            }
        })
    }
}
/// What a workspace is for. Purpose is identity, set when the workspace is
/// created; only the owner host changes it later.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspacePurpose {
    #[default]
    Owner,
    Test,
}
impl WorkspacePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Test => "test",
        }
    }
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
