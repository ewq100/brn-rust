//! Presentation and correlation only. Authority and network work belong to AppWorker.
#![cfg_attr(not(feature = "native-ui"), allow(dead_code))]
use brn_workflow::{
    AccountStatus, LoginPrompt, ModelOption, NoteEntry, Provider, ReasoningEffort, Selection,
    WorkBudget, WorkConversation, WorkTurn, WorkTurnStatus,
    activity::{ActivityPage, ActivityRequest},
    app_worker::{AppCommand, AppEvent},
    backups::BackupStatus,
    chat_worker::{AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent},
    editor::{
        EditRequest, EditStamp, EditorRecord, EditorView, ReloadRequest, SaveOutcome, SaveReceipt,
        SaveRequest,
    },
    inbox_actions::InboxActionRequest,
    knowledge::NoteProvenance,
    library::{KnowledgeScope, RefreshReport, SearchMode, SearchResults},
    models::ModelDownloadPrompt,
    proposal_apply::{
        ApplyJournal, ApplyOutcome, ApplyReceipt, ApplySummary, ApprovalRequest, RepairDirection,
        RepairReceipt, RepairRequest, UndoRequest,
    },
    proposal_rewrite::{RewriteEvent, RewriteJob, RewriteRequest},
    proposals::{CommentRequest, ProposalRecord, ProposalState, ReviewComment},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use uuid::Uuid;
#[path = "dashboard_state.rs"]
mod dashboard_state;
#[cfg(all(test, target_os = "macos"))]
#[path = "dashboard_state_tests.rs"]
pub(crate) mod dashboard_state_tests;
#[path = "finding_state.rs"]
mod finding_state;
#[cfg(all(test, target_os = "macos"))]
#[path = "finding_state_tests.rs"]
mod finding_state_tests;
#[path = "inbox_analysis_state.rs"]
mod inbox_analysis_state;
#[cfg(all(test, target_os = "macos"))]
#[path = "inbox_analysis_state_tests.rs"]
pub(crate) mod inbox_analysis_state_tests;
#[path = "inbox_copy_state.rs"]
mod inbox_copy_state;
#[path = "inbox_state.rs"]
mod inbox_state;
#[path = "visual_analysis_state.rs"]
mod visual_analysis_state;
#[cfg(all(test, target_os = "macos"))]
#[path = "visual_analysis_state_tests.rs"]
pub(crate) mod visual_analysis_state_tests;
#[cfg(feature = "native-ui")]
pub use inbox_copy_state::{InboxCopyConfirmation, InboxCopyRequest};
#[cfg(all(test, target_os = "macos"))]
#[path = "inbox_state_tests.rs"]
mod inbox_state_tests;
#[path = "link_preparation_state.rs"]
mod link_preparation_state;
#[path = "relationship_state.rs"]
mod relationship_state;

#[path = "session_state.rs"]
pub(crate) mod session_state;
#[cfg(test)]
#[path = "session_state_tests.rs"]
pub(crate) mod session_state_tests;

#[path = "budget_state.rs"]
pub(crate) mod budget_state;
#[cfg(test)]
#[path = "budget_state_tests.rs"]
mod budget_state_tests;

pub struct ActiveTurn {
    pub request: ActiveRequest,
    pub partial: String,
    pub tool: Option<String>,
    pub stopping: bool,
    pub budget_progress: Option<(u16, u16)>,
    pub time_limit_reached: bool,
}
/// Client capture/correlation only. Inbox's domain prompt remains in workflow.
pub enum ActiveRequest {
    Ask(AskRequest),
    Inbox(Box<InboxActionRequest>),
}
impl From<AskRequest> for ActiveRequest {
    fn from(request: AskRequest) -> Self {
        Self::Ask(request)
    }
}
impl ActiveRequest {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Ask(r) => r.id,
            Self::Inbox(r) => r.id,
        }
    }
    pub fn conversation(&self) -> Option<Uuid> {
        match self {
            Self::Ask(r) => r.conversation,
            Self::Inbox(r) => r.conversation,
        }
    }
    pub fn generation(&self) -> u64 {
        match self {
            Self::Ask(r) => r.generation,
            Self::Inbox(r) => r.generation,
        }
    }
    pub fn selection(&self) -> &Selection {
        match self {
            Self::Ask(r) => &r.selection,
            Self::Inbox(r) => &r.selection,
        }
    }
    pub fn effort(&self) -> Option<ReasoningEffort> {
        match self {
            Self::Ask(r) => r.effort,
            Self::Inbox(r) => Some(r.effort),
        }
    }
    pub fn budget(&self) -> Option<WorkBudget> {
        match self {
            Self::Ask(r) => r.budget,
            Self::Inbox(r) => r.budget,
        }
    }
    pub fn question_label(&self) -> String {
        match self {
            Self::Ask(r) => r.question.clone(),
            Self::Inbox(r) => format!(
                "Investigate Inbox evidence: {}",
                r.source.as_ref().map_or_else(
                    || r.intake
                        .as_ref()
                        .map_or("unavailable", |i| i.source_path.as_str()),
                    |s| s.source.path.as_str()
                )
            ),
        }
    }
    pub fn inbox(&self) -> Option<&InboxActionRequest> {
        match self {
            Self::Inbox(r) => Some(r),
            Self::Ask(_) => None,
        }
    }
    fn accepts_inbox_turn(&self, turn: &WorkTurn) -> bool {
        let Some(request) = self.inbox() else {
            return true;
        };
        turn.id == request.id
            && request
                .conversation
                .is_none_or(|id| turn.conversation_id == id)
            && turn.provider == provider_key(request.selection.provider)
            && turn.model == request.selection.model
            && turn.effort.as_deref() == Some(request.effort.as_str())
    }
}
pub struct ActiveRewrite {
    pub request: RewriteRequest,
    pub job: Option<RewriteJob>,
    pub tool: Option<String>,
    pub stopping: bool,
}
pub struct LoginDialog {
    pub operation: Uuid,
    pub prompt: Option<LoginPrompt>,
}
#[derive(Default)]
pub struct AccountRow {
    pub status: Option<AccountStatus>,
    pub error: Option<String>,
    pub models: Vec<ModelOption>,
}
#[derive(Clone)]
pub enum Pending {
    BackupStatus,
    CheckpointBackup,
    InboxCopy(Box<inbox_copy_state::CopyPending>),
    InboxAnalysis(inbox_analysis_state::AnalysisPending),
    Inbox(Box<inbox_state::InboxPending>),
    InboxGuided(Box<inbox_state::GuidedPending>),
    Dashboard(dashboard_state::DashboardQuery),
    ActionComplete(Box<dashboard_state::CompletionCapture>),
    Status,
    Selection,
    Select,
    Effort,
    SelectEffort,
    Proposals,
    Proposal {
        id: Uuid,
        generation: u64,
    },
    ReviewRefresh {
        id: Uuid,
        generation: u64,
    },
    AppliedReview {
        id: Uuid,
        generation: u64,
    },
    ReviewEdit,
    KnowledgePredecessor {
        generation: u64,
    },
    CreateRename {
        generation: u64,
    },
    ReviewMutation {
        id: Uuid,
        generation: u64,
    },
    Approval {
        capture: crate::approval::ApprovalCapture,
        generation: u64,
    },
    ApplyReconcile {
        request: ApprovalRequest,
        generation: u64,
    },
    Applies {
        generation: u64,
    },
    ApplySnapshot {
        operation: Uuid,
        generation: u64,
    },
    UndoPreview {
        request: UndoRequest,
        generation: u64,
    },
    RepairPreview {
        operation: Uuid,
        direction: RepairDirection,
        generation: u64,
    },
    Undo {
        capture: Box<crate::approval::UndoCapture>,
        generation: u64,
    },
    Repair {
        capture: Box<crate::approval::RepairCapture>,
        generation: u64,
    },
    Activity {
        generation: u64,
        before: Option<Uuid>,
    },
    DraftSource {
        form: Uuid,
        path: String,
        binding_generation: u64,
    },
    DraftCreate {
        form: Uuid,
        request: Box<brn_workflow::proposals::DraftRequest>,
    },
    LinkTarget(link_preparation_state::LinkCapture),
    LinkPrepare {
        capture: link_preparation_state::LinkCapture,
        request: Box<brn_workflow::knowledge::LinkRequest>,
    },
    Findings {
        capture: finding_state::PageCapture,
        request: brn_workflow::findings::FindingListRequest,
    },
    Finding {
        capture: finding_state::SelectionCapture,
        record: Option<Box<brn_workflow::findings::FindingRecord>>,
    },
    FindingInspection {
        capture: finding_state::SelectionCapture,
        inspection: u64,
        record: Box<brn_workflow::findings::FindingRecord>,
    },
    FindingCapture(brn_workflow::findings::CaptureFindingRequest),
    FindingClose {
        capture: finding_state::SelectionCapture,
        record: Box<brn_workflow::findings::FindingRecord>,
        request: brn_workflow::findings::CloseFindingRequest,
    },
    Account(AccountCommand),
    Bind,
    Refresh,
    Notes {
        scope: KnowledgeScope,
        generation: u64,
        cursor: Option<String>,
    },
    Evidence {
        path: String,
        scope: KnowledgeScope,
        generation: u64,
    },
    Provenance {
        path: String,
        document_generation: u64,
        inspection_generation: u64,
    },
    Links {
        path: String,
        document_generation: u64,
        inspection_generation: u64,
    },
    Relationships {
        scope: KnowledgeScope,
        offset: usize,
        limit: usize,
        generation: u64,
    },
    Editor {
        generation: u64,
        preserve: bool,
    },
    EditorRecovery,
    EditorSave,
    EditorReconcile,
    EditorReload,
    Editors,
    Search {
        scope: KnowledgeScope,
        generation: u64,
    },
    Session(session_state::SessionPending),
    RunBudget {
        turn: Uuid,
        generation: u64,
    },
    Turns {
        generation: u64,
    },
    Prompt,
    Download,
}
#[derive(Default)]
pub struct AiState {
    pub ready: bool,
    pub startup_failed: bool,
    pub vault_bound: bool,
    pub vault_root: Option<PathBuf>,
    pub model_installed: bool,
    pub selection: Option<Selection>,
    pub selection_error: Option<String>,
    pub effort: Option<ReasoningEffort>,
    pub effort_error: Option<String>,
    pub work_budget: WorkBudget,
    pub run_budgets: HashMap<Uuid, Option<WorkBudget>>,
    pub proposals: Vec<ProposalRecord>,
    pub review: Option<crate::review::ProposalReview>,
    pub review_generation: u64,
    pub review_error: Option<String>,
    pub rewrite: Option<ActiveRewrite>,
    pub last_rewrite: Option<RewriteJob>,
    pub rewrite_storage_failed: bool,
    pub approval_receipts: Vec<ApplyReceipt>,
    pub approval_requests: Vec<ApprovalRequest>,
    pub approval_error: Option<String>,
    pub applies: Vec<ApplySummary>,
    pub applies_error: Option<String>,
    pub applies_generation: u64,
    pub application_snapshot: Option<ApplyJournal>,
    pub snapshot_error: Option<String>,
    pub snapshot_generation: u64,
    pub operation_generation: u64,
    pub operation_error: Option<String>,
    pub undo_preview: Option<crate::approval::UndoCapture>,
    pub repair_preview: Option<crate::approval::RepairCapture>,
    pub last_undo_request: Option<UndoRequest>,
    pub last_repair_request: Option<RepairRequest>,
    pub repair_receipt: Option<RepairReceipt>,
    pub activity: Option<ActivityPage>,
    pub activity_error: Option<String>,
    pub activity_generation: u64,
    pub draft: Option<crate::draft::DraftForm>,
    pub link_preparation: link_preparation_state::LinkPreparation,
    pub finding_queue: finding_state::FindingQueue,
    pub inbox_queue: inbox_state::InboxQueue,
    pub inbox_copy: inbox_copy_state::InboxCopyView,
    pub inbox_analysis: inbox_analysis_state::InboxAnalysisView,
    pub dashboard: dashboard_state::DashboardView,
    pub last_draft_request: Option<brn_workflow::proposals::DraftRequest>,
    pub provider: Option<Provider>,
    pub generation: u64,
    pub conversation: Option<Uuid>,
    pub conversations: Vec<WorkConversation>,
    pub session_history: session_state::SessionHistory,
    pub turns: Vec<WorkTurn>,
    pub active: Option<ActiveTurn>,
    pub unsaved: Option<WorkTurn>,
    pub login: Option<LoginDialog>,
    pub cancelled_login: Option<Uuid>,
    pub accounts: [AccountRow; 2],
    pub pending: HashMap<Uuid, Pending>,
    pub backup_status: Option<BackupStatus>,
    pub backup_request_error: Option<String>,
    pub knowledge_scope: KnowledgeScope,
    pub notes_generation: u64,
    pub notes: Vec<NoteEntry>,
    pub next_cursor: Option<String>,
    pub notes_error: Option<String>,
    pub note_error: Option<String>,
    pub note_generation: u64,
    pub evidence: Option<EvidenceDocument>,
    pub provenance: Option<NoteProvenance>,
    pub provenance_error: Option<String>,
    provenance_generation: u64,
    pub links: Option<brn_workflow::knowledge::NoteLinks>,
    pub links_error: Option<String>,
    links_generation: u64,
    pub relationships: Option<brn_workflow::knowledge::RelationshipPage>,
    pub relationships_error: Option<String>,
    relationships_generation: u64,
    pub editor: Option<SimpleEditor>,
    pub editors: Vec<EditorRecord>,
    pub search: Option<SearchResults>,
    pub search_scope: Option<KnowledgeScope>,
    pub search_generation: u64,
    pub refresh: Option<RefreshReport>,
    pub indexing: Option<(Uuid, usize, usize)>,
    pub model_prompt: Option<ModelDownloadPrompt>,
    pub download: Option<Uuid>,
    pub download_stopping: bool,
    pub download_progress: Option<(u64, u64)>,
    pub model_state: String,
    pub notice: String,
    pub restored: Option<PathBuf>,
}

pub struct EvidenceDocument {
    pub path: String,
    pub scope: KnowledgeScope,
    pub note: Option<brn_workflow::vault::NoteText>,
}

pub fn scope_name(scope: KnowledgeScope) -> &'static str {
    match scope {
        KnowledgeScope::Current => "Current",
        KnowledgeScope::Source => "Source",
        KnowledgeScope::History => "History",
        KnowledgeScope::All => "All",
    }
}

struct EditorMutation {
    id: Uuid,
    edit: EditRequest,
    destination: Option<String>,
    reload_text: Option<String>,
}

/// Live typing is distinct from the worker's acknowledged rolling buffer.
pub struct SimpleEditor {
    pub view: EditorView,
    pub text: String,
    generation: u64,
    acknowledged: EditStamp,
    pending: Option<EditorMutation>,
    pub error: Option<String>,
    recovery_failed: bool,
    last_edit: Option<Instant>,
}
impl SimpleEditor {
    fn new(view: EditorView) -> Self {
        Self {
            text: view.record.text.clone(),
            generation: view.record.stamp.generation,
            acknowledged: view.record.stamp,
            view,
            pending: None,
            error: None,
            recovery_failed: false,
            last_edit: None,
        }
    }
    pub fn edit(&mut self, text: String, now: Instant) -> Result<(), &'static str> {
        if self.replacing() {
            return Err("Waiting for confirmed reload acknowledgement");
        }
        if text == self.text {
            return Ok(());
        }
        if text.len() > 1024 * 1024 {
            return Err("Note exceeds the 1 MiB UTF-8 byte limit");
        }
        self.generation = self
            .generation
            .checked_add(1)
            .filter(|generation| *generation <= i64::MAX as u64)
            .ok_or("Note generation exhausted")?;
        self.text = text;
        self.last_edit = Some(now);
        Ok(())
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn replacing(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.reload_text.is_some())
    }
    pub fn needs_recovery(&self) -> bool {
        self.generation != self.acknowledged.generation || self.text != self.view.record.text
    }
    pub fn can_leave(&self) -> bool {
        !self.pending() && !self.needs_recovery()
    }
    pub fn dirty(&self) -> bool {
        self.view.saved.as_deref() != Some(&self.text)
    }
    pub fn can_save(&self) -> bool {
        !self.pending() && !self.view.conflict && self.view.pending.is_empty()
    }
    pub fn reload_request(&self) -> Option<ReloadRequest> {
        if !self.can_leave() || !self.view.pending.is_empty() {
            return None;
        }
        Some(ReloadRequest {
            path: self.view.record.path.clone(),
            expected: self.acknowledged,
            observed: self.view.observed.clone()?,
            discard: self.dirty(),
        })
    }
    pub fn wants_recovery(&self, now: Instant, leaving: bool) -> bool {
        self.needs_recovery()
            && !self.pending()
            && !self.recovery_failed
            && (leaving
                || self.last_edit.is_some_and(|last| {
                    now.saturating_duration_since(last) >= Duration::from_millis(500)
                }))
    }
    pub fn retry_recovery(&mut self) {
        self.recovery_failed = false;
        self.last_edit = Some(Instant::now() - Duration::from_millis(500));
    }
    fn begin(&mut self, id: Uuid, destination: Option<String>) -> Option<EditRequest> {
        if self.pending() {
            return None;
        }
        let edit = EditRequest {
            path: self.view.record.path.clone(),
            expected: self.acknowledged,
            generation: self.generation,
            text: self.text.clone(),
        };
        self.pending = Some(EditorMutation {
            id,
            edit: edit.clone(),
            destination,
            reload_text: None,
        });
        self.error = None;
        Some(edit)
    }
    fn recovered(&mut self, id: Uuid, record: EditorRecord) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if record.path != pending.edit.path
            || record.stamp.baseline != pending.edit.expected.baseline
            || record.stamp.generation != pending.edit.generation
            || record.text != pending.edit.text
        {
            self.failed(
                id,
                "Recovery acknowledgement did not match submitted text".into(),
                true,
            );
            return;
        }
        self.acknowledged = record.stamp;
        self.view.record = record;
        self.pending = None;
        self.recovery_failed = false;
    }
    fn reloaded(&mut self, id: Uuid, record: EditorRecord) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if record.path != pending.edit.path
            || record.stamp.generation < pending.edit.generation
            || pending.reload_text.as_deref() != Some(record.text.as_str())
        {
            self.failed(
                id,
                "Reload acknowledgement did not match reviewed disk text".into(),
                false,
            );
            return;
        }
        self.text = record.text.clone();
        self.generation = record.stamp.generation;
        self.acknowledged = record.stamp;
        self.view.record = record;
        self.view.saved = Some(self.text.clone());
        self.view.conflict = false;
        self.pending = None;
        self.error = None;
        self.recovery_failed = false;
    }
    fn saved(&mut self, id: Uuid, receipt: &SaveReceipt) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if receipt.operation_id != id
            || receipt.path != pending.edit.path
            || receipt.destination != pending.destination
            || receipt.submitted_generation != pending.edit.generation
            || receipt.stamp.generation != pending.edit.generation
            || pending.destination.is_some()
                && receipt.stamp.baseline != pending.edit.expected.baseline
        {
            self.failed(
                id,
                "Save acknowledgement did not match submitted text".into(),
                true,
            );
            return;
        }
        // The receipt only acknowledges its submitted snapshot, never subsequent typing.
        if pending.destination.is_none() && receipt.outcome == SaveOutcome::Applied {
            self.view.saved = Some(pending.edit.text.clone());
        }
        if receipt.outcome == SaveOutcome::Uncertain {
            if !self.view.pending.contains(&id) {
                self.view.pending.push(id);
            }
            self.error = Some(
                "Save outcome uncertain; reconcile before saving again. Recovery text is retained."
                    .into(),
            );
        }
        self.acknowledged = receipt.stamp;
        self.view.record.stamp = receipt.stamp;
        self.view.record.text = pending.edit.text.clone();
        self.pending = None;
    }
    fn observe(&mut self, view: EditorView) {
        if view.record.path != self.view.record.path {
            return;
        }
        // A queued observation can precede a later acknowledgement.
        if view.record.stamp.baseline != self.acknowledged.baseline
            || view.record.stamp.generation < self.acknowledged.generation
        {
            return;
        }
        // Fresh disk inspection is not an acknowledgement of another submitted
        // snapshot. Only identical live bytes prove the rolling buffer is current.
        if !self.pending()
            && view.record.text == self.text
            && view.record.stamp.generation >= self.generation
        {
            self.acknowledged = view.record.stamp;
            self.generation = view.record.stamp.generation;
        }
        self.view = view;
    }
    fn reconciled(&mut self, receipt: &SaveReceipt) {
        if receipt.path != self.view.record.path
            || !self.view.pending.contains(&receipt.operation_id)
        {
            return;
        }
        // Reconciliation can resolve an older original while a newer rolling
        // buffer is already acknowledged. Adopt its proven baseline, never lower
        // the buffer generation or replace the user's live text.
        if receipt.destination.is_none() && receipt.outcome == SaveOutcome::Applied {
            self.acknowledged.baseline = receipt.stamp.baseline;
            self.view.record.stamp.baseline = receipt.stamp.baseline;
        }
    }
    fn failed(&mut self, id: Uuid, message: String, recovery: bool) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id)
        {
            self.pending = None;
            self.error = Some(message);
            self.recovery_failed |= recovery;
        }
    }
    pub fn status(&self) -> &'static str {
        if self.pending() {
            "Waiting for local acknowledgement"
        } else if !self.view.pending.is_empty() {
            "Save outcome uncertain"
        } else if self.view.conflict {
            "Disk conflict or vault unavailable · buffer retained"
        } else if self.recovery_failed {
            "Recovery failed · text retained in memory"
        } else if self.needs_recovery() {
            "Unacknowledged edits"
        } else if self.dirty() {
            "Recovered in BRN · Markdown unsaved"
        } else {
            "Saved Markdown"
        }
    }
}
pub fn slot(provider: Provider) -> usize {
    match provider {
        Provider::Chatgpt => 0,
        Provider::Copilot => 1,
    }
}
pub fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Chatgpt => "ChatGPT",
        Provider::Copilot => "Copilot",
    }
}
fn provider_key(provider: Provider) -> &'static str {
    match provider {
        Provider::Chatgpt => "chatgpt",
        Provider::Copilot => "copilot",
    }
}
pub fn turn_label(turn: &WorkTurn) -> &'static str {
    match turn.status {
        WorkTurnStatus::Running => "Running (restart recovery pending)",
        WorkTurnStatus::Completed => "Completed",
        WorkTurnStatus::Interrupted => "Stopped",
        WorkTurnStatus::Failed => "Failed",
    }
}
pub fn session_activity_label(conversation: &WorkConversation, now_ms: u64) -> String {
    let Some(activity) = conversation.last_activity_at_ms else {
        return "Activity time unknown".into();
    };
    let Some(elapsed) = now_ms.checked_sub(activity) else {
        return "Activity time is ahead of this clock".into();
    };
    let (count, unit) = if elapsed < 60_000 {
        return "Last active just now".into();
    } else if elapsed < 3_600_000 {
        (elapsed / 60_000, "minute")
    } else if elapsed < 86_400_000 {
        (elapsed / 3_600_000, "hour")
    } else {
        (elapsed / 86_400_000, "day")
    };
    format!(
        "Last active {count} {unit}{} ago",
        if count == 1 { "" } else { "s" }
    )
}
fn unfinalized_turn(request: &ActiveRequest, partial: String) -> WorkTurn {
    WorkTurn {
        id: request.id(),
        conversation_id: request.conversation().unwrap_or(Uuid::nil()),
        question: request.question_label(),
        answer: partial,
        provider: provider_key(request.selection().provider).into(),
        model: request.selection().model.clone(),
        effort: request.effort().map(|value| value.as_str().to_owned()),
        started_at_ms: None,
        finished_at_ms: None,
        status: WorkTurnStatus::Failed,
        error_code: None,
    }
}
impl AiState {
    pub fn application_busy(&self) -> bool {
        self.inbox_copy_pending()
            || self.pending.values().any(|pending| {
                matches!(
                    pending,
                    Pending::ActionComplete(_)
                        | Pending::Approval { .. }
                        | Pending::ApplyReconcile { .. }
                        | Pending::AppliedReview { .. }
                        | Pending::Undo { .. }
                        | Pending::Repair { .. }
                        | Pending::DraftCreate { .. }
                )
            })
    }
    pub fn begin_draft(&mut self, turn: Option<Uuid>) -> bool {
        if !self.ready
            || !self.vault_bound
            || !self.review_can_leave()
            || self.active.is_some()
            || self.rewrite.is_some()
        {
            return false;
        }
        let turn = match turn {
            None => None,
            Some(id) => {
                let Some(turn) = self.turns.iter().find(|turn| turn.id == id) else {
                    self.notice =
                        "Only an acknowledged completed answer can prefill a proposal.".into();
                    return false;
                };
                Some(turn)
            }
        };
        let Some(draft) = crate::draft::DraftForm::new(turn) else {
            self.notice = "The answer is provisional, failed or exceeds the full-note limit. Its complete text remains in chat; no truncated draft was created.".into();
            return false;
        };
        self.draft = Some(draft);
        self.link_preparation = Default::default();
        true
    }
    pub fn discard_draft(&mut self) -> bool {
        if self.draft.as_ref().is_some_and(|draft| draft.pending) {
            return false;
        }
        self.draft = None;
        self.link_preparation = Default::default();
        true
    }
    pub fn begin_action_draft(&mut self, follows_up: Option<Uuid>) -> bool {
        if !self.ready
            || !self.review_can_leave()
            || self.active.is_some()
            || self.rewrite.is_some()
        {
            return false;
        }
        let Some(draft) = crate::draft::DraftForm::new_action(follows_up) else {
            return false;
        };
        self.draft = Some(draft);
        self.link_preparation = Default::default();
        true
    }
    pub fn separate_draft(&mut self) -> bool {
        let Some(draft) = self.draft.as_ref().and_then(|draft| draft.separate()) else {
            return false;
        };
        self.draft = Some(draft);
        self.link_preparation = Default::default();
        true
    }
    pub fn draft_source(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || self.application_busy() {
            return None;
        }
        let draft = self.draft.as_mut()?;
        if draft.pending
            || (draft.action.is_none() && draft.kind == crate::draft::DraftKind::Create)
            || draft.prepared_request().is_some()
        {
            return None;
        }
        if draft.action.is_some() && !draft.can_capture_action_source() {
            draft.source_error = Some("At most 64 complete sources may be bound. Recapture an existing path, or copy the full input and start a new form.".into());
            return None;
        }
        let (form, path, binding_generation) =
            (draft.id, draft.path.clone(), draft.binding_generation);
        draft.source = None;
        draft.source_error = None;
        let command = self.command(
            Pending::DraftSource {
                form,
                path: path.clone(),
                binding_generation,
            },
            AppCommand::ProposalSource(path),
        );
        self.draft.as_mut().expect("matched form").source_operation = Some(command.0);
        Some(command)
    }
    pub fn create_draft(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || (!self.vault_bound
                && self.draft.as_ref().is_none_or(|draft| {
                    draft
                        .action
                        .as_ref()
                        .is_none_or(|action| !action.sources.is_empty())
                }))
            || self.application_busy()
            || self.active.is_some()
            || self.rewrite.is_some()
            || self
                .review
                .as_ref()
                .is_some_and(|review| !review.can_leave())
            || self
                .pending
                .values()
                .any(|pending| matches!(pending, Pending::ReviewMutation { .. }))
        {
            return None;
        }
        if self.link_preparation.operation.is_some() {
            return None;
        }
        let submitted = self.draft.as_mut()?.prepare()?;
        let form = self.draft.as_ref()?.id;
        self.last_draft_request = Some(submitted.request.clone());
        self.pending.insert(
            submitted.operation,
            Pending::DraftCreate {
                form,
                request: Box::new(submitted.request.clone()),
            },
        );
        self.notice = "Creating the exact full review draft; vault knowledge is unchanged.".into();
        Some((
            submitted.operation,
            AppCommand::CreateProposal(submitted.request),
        ))
    }
    fn can_confirm_operation(&self) -> bool {
        self.ready && self.review_can_leave() && self.active.is_none() && self.rewrite.is_none()
    }
    pub fn preview_undo(
        &mut self,
        target_operation: Uuid,
        trash_member: Option<usize>,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.can_confirm_operation() || (trash_member.is_some() && !self.vault_bound) {
            return None;
        }
        let request = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: target_operation,
            trash_member,
        };
        brn_workflow::proposal_apply::validate_undo_request(&request).ok()?;
        self.operation_generation = self.operation_generation.checked_add(1)?;
        self.undo_preview = None;
        self.repair_preview = None;
        self.operation_error = None;
        Some(self.command(
            Pending::UndoPreview {
                request: request.clone(),
                generation: self.operation_generation,
            },
            AppCommand::PreviewProposalUndo(request),
        ))
    }
    pub fn preview_repair(
        &mut self,
        operation: Uuid,
        direction: RepairDirection,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.can_confirm_operation() || !self.vault_bound || operation.is_nil() {
            return None;
        }
        self.operation_generation = self.operation_generation.checked_add(1)?;
        self.undo_preview = None;
        self.repair_preview = None;
        self.operation_error = None;
        Some(self.command(
            Pending::RepairPreview {
                operation,
                direction,
                generation: self.operation_generation,
            },
            AppCommand::PreviewProposalRepair(operation),
        ))
    }
    pub fn confirm_undo(
        &mut self,
        capture: &crate::approval::UndoCapture,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.can_confirm_operation()
            || (!self.vault_bound && !capture.preview().draft.changes.is_empty())
            || self.undo_preview.as_ref() != Some(capture)
        {
            return None;
        }
        let request = capture.request().clone();
        self.last_undo_request = Some(request.clone());
        self.approval_requests = vec![ApprovalRequest {
            operation_id: request.operation_id,
            expected: brn_workflow::proposals::ProposalStamp {
                id: request.operation_id,
                version: 1,
            },
        }];
        self.approval_receipts.clear();
        self.approval_error = None;
        self.operation_error = None;
        self.notice = "Executing the captured Undo; await its recorded outcome.".into();
        Some(self.command(
            Pending::Undo {
                capture: Box::new(capture.clone()),
                generation: self.review_generation,
            },
            capture.command(),
        ))
    }
    pub fn confirm_repair(
        &mut self,
        capture: &crate::approval::RepairCapture,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.can_confirm_operation()
            || !self.vault_bound
            || self.repair_preview.as_ref() != Some(capture)
            || !self.application_snapshot.as_ref().is_some_and(|journal| {
                journal.request.operation_id == capture.preview().operation_id
                    && journal.approved.draft == capture.preview().approved
            })
        {
            return None;
        }
        self.last_repair_request = Some(capture.request().clone());
        self.repair_receipt = None;
        self.operation_error = None;
        self.notice = "Executing the captured repair direction; await its recorded outcome.".into();
        Some(self.command(
            Pending::Repair {
                capture: Box::new(capture.clone()),
                generation: self.review_generation,
            },
            capture.command(),
        ))
    }
    fn approval_vault_ready(&self, record: &ProposalRecord) -> bool {
        self.vault_bound
            || (record.draft.vault.is_none()
                && record.draft.changes.is_empty()
                && record.draft.sources.is_empty()
                && !record.draft.action_changes.is_empty())
    }

    pub fn capture_approval(&self, group: bool) -> Option<crate::approval::ApprovalCapture> {
        if !self.ready || !self.review_can_mutate() || self.rewrite.is_some() {
            return None;
        }
        let current = &self.review.as_ref()?.record;
        if !self.approval_vault_ready(current) {
            return None;
        }
        let group_id = if group {
            Some(current.draft.group_id?)
        } else {
            None
        };
        let records = if let Some(group_id) = group_id {
            let mut records: Vec<_> = self
                .proposals
                .iter()
                .filter(|record| {
                    record.draft.group_id == Some(group_id) && record.state == ProposalState::Draft
                })
                .cloned()
                .collect();
            if let Some(record) = records
                .iter_mut()
                .find(|record| record.draft.id == current.draft.id)
            {
                *record = current.clone();
            } else {
                records.push(current.clone());
            }
            let dependencies: Vec<_> = records
                .iter()
                .filter_map(crate::approval::intake_dependency)
                .map(|binding| binding.source_proposal)
                .collect();
            for stamp in dependencies {
                if let Some(source) = self
                    .proposals
                    .iter()
                    .find(|source| source.stamp() == stamp && source.state == ProposalState::Draft)
                    && !records.iter().any(|r| r.draft.id == source.draft.id)
                {
                    records.push(source.clone());
                }
            }
            records.sort_by_key(|record| record.draft.inbox_source.is_none());
            records
        } else {
            vec![current.clone()]
        };
        if records
            .iter()
            .any(|record| !self.approval_vault_ready(record))
        {
            return None;
        }
        crate::approval::ApprovalCapture::new(records, group_id)
    }
    pub fn confirm_approval(
        &mut self,
        capture: &crate::approval::ApprovalCapture,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.review_can_mutate() || self.rewrite.is_some() {
            return None;
        }
        let current = &self.review.as_ref()?.record;
        if capture
            .records()
            .iter()
            .any(|record| !self.approval_vault_ready(record))
            || !capture.records().iter().any(|record| record == current)
                && capture
                    .group_id()
                    .is_none_or(|group| current.draft.group_id != Some(group))
            || capture.records().iter().any(|record| {
                if record.draft.id == current.draft.id {
                    record != current
                } else {
                    !self.proposals.iter().any(|known| known == record)
                }
            })
        {
            return None;
        }
        self.approval_error = None;
        self.approval_receipts.clear();
        self.approval_requests = capture.requests().to_vec();
        self.notice = "Applying the exact captured review; await the recorded outcome.".into();
        Some(self.command(
            Pending::Approval {
                capture: capture.clone(),
                generation: self.review_generation,
            },
            capture.command(),
        ))
    }
    pub fn refresh_activity(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.activity_generation = self.activity_generation.checked_add(1)?;
        self.activity = None;
        self.activity_error = None;
        Some(self.command(
            Pending::Activity {
                generation: self.activity_generation,
                before: None,
            },
            AppCommand::Activity(ActivityRequest::default()),
        ))
    }
    pub fn more_activity(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.pending.values().any(|pending| matches!(pending, Pending::Activity { generation, .. } if *generation == self.activity_generation)) { return None; }
        let before = self.activity.as_ref()?.next_before?;
        self.activity_error = None;
        Some(self.command(
            Pending::Activity {
                generation: self.activity_generation,
                before: Some(before),
            },
            AppCommand::Activity(ActivityRequest {
                before: Some(before),
                ..ActivityRequest::default()
            }),
        ))
    }
    pub fn refresh_applies(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.applies_generation = self.applies_generation.checked_add(1)?;
        self.applies_error = None;
        Some(self.command(
            Pending::Applies {
                generation: self.applies_generation,
            },
            AppCommand::ProposalRecovery,
        ))
    }
    pub fn inspect_apply(&mut self, operation: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || operation.is_nil() {
            return None;
        }
        self.snapshot_generation = self.snapshot_generation.checked_add(1)?;
        self.application_snapshot = None;
        self.snapshot_error = None;
        Some(self.command(
            Pending::ApplySnapshot {
                operation,
                generation: self.snapshot_generation,
            },
            AppCommand::ProposalApply(operation),
        ))
    }
    pub fn reconcile_apply(&mut self, operation: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.application_busy() || !self.review_can_leave() {
            return None;
        }
        let request = self
            .applies
            .iter()
            .find(|summary| summary.request.operation_id == operation)
            .map(|summary| &summary.request)
            .or_else(|| {
                self.application_snapshot
                    .as_ref()
                    .filter(|journal| journal.request.operation_id == operation)
                    .map(|journal| &journal.request)
            })?
            .clone();
        self.approval_error = None;
        self.approval_requests = vec![request.clone()];
        self.notice = "Reconciling recorded proofs; installation is not repeated.".into();
        Some(self.command(
            Pending::ApplyReconcile {
                request,
                generation: self.review_generation,
            },
            AppCommand::ReconcileProposal(operation),
        ))
    }
    fn refresh_after_application(
        &mut self,
        proposals: &[Uuid],
        generation: u64,
    ) -> Vec<(Uuid, AppCommand)> {
        self.clear_links();
        self.clear_relationships();
        let mut commands = vec![self.command(Pending::Proposals, AppCommand::Proposals(None))];
        if generation == self.review_generation
            && self
                .review
                .as_ref()
                .is_some_and(|review| proposals.contains(&review.record.draft.id))
        {
            let id = self
                .review
                .as_ref()
                .expect("matched review")
                .record
                .draft
                .id;
            commands.push(self.command(
                Pending::AppliedReview { id, generation },
                AppCommand::Proposal(id),
            ));
        }
        if let Some(command) = self.refresh_activity() {
            commands.push(command);
        }
        if let Some(command) = self.refresh_applies() {
            commands.push(command);
        }
        // File effects can invalidate displayed/current evidence even when a
        // terminal error follows the recorded outcome. Keep local editor work.
        self.composer_changed();
        if let Some(command) = self.refresh_notes() {
            commands.push(command);
        }
        commands
    }
    pub fn review_can_leave(&self) -> bool {
        self.review.as_ref().is_none_or(|review| review.can_leave())
            && self.draft.as_ref().is_none_or(|draft| draft.can_leave())
            && !self.application_busy()
            && !self
                .pending
                .values()
                .any(|pending| matches!(pending, Pending::ReviewMutation { .. }))
    }
    pub fn review_editable(&self) -> bool {
        if self.application_busy() {
            return false;
        }
        self.review.as_ref().is_some_and(|review| {
            review.record.state == ProposalState::Draft
                && review.observed.is_none()
                && !review.predecessor_pending()
                && !review.create_rename_pending()
        }) && !self
            .pending
            .values()
            .any(|pending| matches!(pending, Pending::ReviewMutation { .. }))
    }
    pub fn review_can_mutate(&self) -> bool {
        if self.application_busy() {
            return false;
        }
        self.review
            .as_ref()
            .is_some_and(|review| review.can_mutate())
            && !self
                .pending
                .values()
                .any(|pending| matches!(pending, Pending::ReviewMutation { .. }))
    }
    pub fn open_review(&mut self, id: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.review_can_leave() {
            return None;
        }
        self.review_generation = self.review_generation.checked_add(1)?;
        self.review = None;
        self.review_error = None;
        Some(self.command(
            Pending::Proposal {
                id,
                generation: self.review_generation,
            },
            AppCommand::Proposal(id),
        ))
    }
    pub fn refresh_review(&mut self) -> Option<(Uuid, AppCommand)> {
        let id = self.review.as_ref()?.record.draft.id;
        Some(self.command(
            Pending::ReviewRefresh {
                id,
                generation: self.review_generation,
            },
            AppCommand::Proposal(id),
        ))
    }
    pub fn attach_knowledge_predecessor(&mut self, path: String) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || !self.review_can_mutate()
            || self.active.is_some()
            || self.rewrite.is_some()
        {
            return None;
        }
        let (id, request) = self.review.as_mut()?.prepare_predecessor(path)?;
        self.pending.insert(
            id,
            Pending::KnowledgePredecessor {
                generation: self.review_generation,
            },
        );
        self.notice = "Attaching selected Current predecessor; no knowledge changes occur before exact approval.".into();
        Some((id, AppCommand::AttachInboxKnowledgePredecessor(request)))
    }
    pub fn rename_proposal_create(
        &mut self,
        change_index: usize,
        path: String,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || !self.review_can_mutate()
            || self.active.is_some()
            || self.rewrite.is_some()
        {
            return None;
        }
        let (id, request) = self
            .review
            .as_mut()?
            .prepare_create_rename(change_index, path)?;
        self.pending.insert(
            id,
            Pending::CreateRename {
                generation: self.review_generation,
            },
        );
        self.notice = "Revising the selected new-note destination; no vault effect occurs before exact approval.".into();
        Some((id, AppCommand::RenameProposalCreate(request)))
    }
    pub fn recover_review(&mut self) -> Option<(Uuid, AppCommand)> {
        let (id, edit) = self.review.as_mut()?.prepare_edit()?;
        self.pending.insert(id, Pending::ReviewEdit);
        Some((id, AppCommand::EditProposal(edit)))
    }
    pub fn review_comment(
        &mut self,
        comment: ReviewComment,
        update: bool,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.review_can_mutate() {
            return None;
        }
        let expected = self.review.as_ref()?.record.stamp();
        let request = CommentRequest { expected, comment };
        let command = if update {
            AppCommand::UpdateProposalComment(request)
        } else {
            AppCommand::AddProposalComment(request)
        };
        Some(self.command(
            Pending::ReviewMutation {
                id: expected.id,
                generation: self.review_generation,
            },
            command,
        ))
    }
    pub fn remove_review_comment(&mut self, comment: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.review_can_mutate() {
            return None;
        }
        let expected = self.review.as_ref()?.record.stamp();
        Some(self.command(
            Pending::ReviewMutation {
                id: expected.id,
                generation: self.review_generation,
            },
            AppCommand::RemoveProposalComment { expected, comment },
        ))
    }
    pub fn reject_review(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.review_can_mutate() {
            return None;
        }
        let expected = self.review.as_ref()?.record.stamp();
        Some(self.command(
            Pending::ReviewMutation {
                id: expected.id,
                generation: self.review_generation,
            },
            AppCommand::RejectProposal(expected),
        ))
    }
    pub fn can_rewrite(&self) -> bool {
        self.session_allows_new_work()
            && self.review_session_allows_new_work()
            && self.ready
            && self.vault_bound
            && self.review_can_mutate()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.unsaved.is_none()
            && !self.rewrite_storage_failed
            && self.selection.is_some()
            && self.effort.is_some()
            && self.selection_error.is_none()
            && self.effort_error.is_none()
            && !self.pending.values().any(|pending| {
                matches!(
                    pending,
                    Pending::Select | Pending::Effort | Pending::SelectEffort
                )
            })
    }
    pub fn start_rewrite(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_rewrite() {
            return None;
        }
        let request = RewriteRequest {
            id: Uuid::new_v4(),
            expected: self.review.as_ref()?.record.stamp(),
            selection: self.selection.clone()?,
            effort: self.effort?,
            generation: self.review_generation,
        };
        self.rewrite = Some(ActiveRewrite {
            request: request.clone(),
            job: None,
            tool: None,
            stopping: false,
        });
        self.notice = "Rewrite requested; vault knowledge is unchanged until approval.".into();
        Some((request.id, AppCommand::StartProposalRewrite(request)))
    }
    pub fn stop_rewrite(&mut self) -> Option<Uuid> {
        let active = self.rewrite.as_mut()?;
        active.stopping = true;
        Some(active.request.id)
    }
    pub fn display_active(&self) -> Option<&ActiveTurn> {
        self.active
            .as_ref()
            .filter(|active| match active.request.conversation() {
                Some(conversation) => self.conversation == Some(conversation),
                None => {
                    self.conversation.is_none() && active.request.generation() == self.generation
                }
            })
    }
    pub fn display_turns(&self) -> impl Iterator<Item = &WorkTurn> {
        let active = self.display_active().map(|active| active.request.id());
        self.turns
            .iter()
            .filter(move |turn| turn.status != WorkTurnStatus::Running || active != Some(turn.id))
    }
    fn upsert_turn(&mut self, turn: WorkTurn) {
        if let Some(row) = self.turns.iter_mut().find(|row| row.id == turn.id) {
            *row = turn;
        } else {
            self.turns.push(turn);
        }
    }
    pub fn composer_changed(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        self.search = None;
        self.search_scope = None;
    }
    pub fn select_scope(&mut self, scope: KnowledgeScope) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || self.knowledge_scope == scope {
            return None;
        }
        self.knowledge_scope = scope;
        self.clear_relationships();
        self.composer_changed();
        self.refresh_notes()
    }
    pub fn refresh_notes(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound {
            return None;
        }
        self.notes_generation = self.notes_generation.wrapping_add(1);
        self.notes.clear();
        self.next_cursor = None;
        self.notes_error = None;
        Some(self.scoped_notes(None))
    }
    fn scoped_notes(&mut self, cursor: Option<String>) -> (Uuid, AppCommand) {
        self.command(
            Pending::Notes {
                scope: self.knowledge_scope,
                generation: self.notes_generation,
                cursor: cursor.clone(),
            },
            AppCommand::ScopedNotes {
                scope: self.knowledge_scope,
                folder: None,
                cursor,
            },
        )
    }
    pub fn more_notes(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound {
            return None;
        }
        let cursor = self.next_cursor.clone()?;
        if self.pending.values().any(|pending| matches!(pending,
            Pending::Notes { scope, generation, cursor: Some(pending_cursor) }
                if *scope == self.knowledge_scope && *generation == self.notes_generation && pending_cursor == &cursor)) {
            return None;
        }
        self.notes_error = None;
        Some(self.scoped_notes(Some(cursor)))
    }
    pub fn search_notes(&mut self, query: String) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || query.trim().is_empty() {
            return None;
        }
        self.composer_changed();
        Some(self.command(
            Pending::Search {
                scope: self.knowledge_scope,
                generation: self.search_generation,
            },
            AppCommand::ScopedSearch {
                scope: self.knowledge_scope,
                query,
                mode: SearchMode::Hybrid,
                limit: 10,
            },
        ))
    }
    pub fn open_evidence(&mut self, path: String, scope: KnowledgeScope) -> (Uuid, AppCommand) {
        self.clear_provenance();
        self.clear_links();
        self.note_generation = self.note_generation.wrapping_add(1);
        self.editor = None;
        self.note_error = None;
        self.evidence = Some(EvidenceDocument {
            path: path.clone(),
            scope,
            note: None,
        });
        self.command(
            Pending::Evidence {
                path: path.clone(),
                scope,
                generation: self.note_generation,
            },
            AppCommand::ScopedNote { scope, path },
        )
    }
    fn saved_document_path(&self) -> Option<&str> {
        self.evidence
            .as_ref()
            .map(|evidence| evidence.path.as_str())
            .or_else(|| {
                self.editor
                    .as_ref()
                    .map(|editor| editor.view.record.path.as_str())
            })
    }
    pub fn clear_provenance(&mut self) {
        self.provenance_generation = self.provenance_generation.wrapping_add(1);
        self.provenance = None;
        self.provenance_error = None;
    }
    fn provenance_request_matches(
        &self,
        path: &str,
        document_generation: u64,
        inspection_generation: u64,
    ) -> bool {
        self.saved_document_path() == Some(path)
            && document_generation == self.note_generation
            && inspection_generation == self.provenance_generation
    }
    pub fn provenance_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending,
            Pending::Provenance {path,document_generation,inspection_generation}
                if self.provenance_request_matches(path,*document_generation,*inspection_generation)))
    }
    /// Explicit saved-source inspection leaves live typing and document scope
    /// intact. Operational and vault authority remain on the application lane.
    pub fn inspect_provenance(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || self.application_busy() || self.provenance_loading()
        {
            return None;
        }
        let path = self.saved_document_path()?.to_owned();
        self.clear_provenance();
        Some(self.command(
            Pending::Provenance {
                path: path.clone(),
                document_generation: self.note_generation,
                inspection_generation: self.provenance_generation,
            },
            AppCommand::NoteProvenance(path),
        ))
    }
    pub fn backup_pending(&self) -> bool {
        self.pending
            .values()
            .any(|pending| matches!(pending, Pending::BackupStatus | Pending::CheckpointBackup))
    }
    pub fn request_backup(&mut self, checkpoint: bool) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.backup_pending() {
            return None;
        }
        self.backup_request_error = None;
        Some(if checkpoint {
            self.command(Pending::CheckpointBackup, AppCommand::CheckpointBackup)
        } else {
            self.command(Pending::BackupStatus, AppCommand::BackupStatus)
        })
    }
    pub fn command(&mut self, pending: Pending, command: AppCommand) -> (Uuid, AppCommand) {
        let id = Uuid::new_v4();
        self.pending.insert(id, pending);
        (id, command)
    }
    pub fn open_editor(&mut self, path: String) -> (Uuid, AppCommand) {
        self.clear_provenance();
        self.clear_links();
        self.note_generation = self.note_generation.wrapping_add(1);
        self.editor = None;
        self.evidence = None;
        self.note_error = None;
        self.command(
            Pending::Editor {
                generation: self.note_generation,
                preserve: false,
            },
            AppCommand::OpenEditor(path),
        )
    }
    pub fn refresh_editor(&mut self) -> Option<(Uuid, AppCommand)> {
        let path = self.editor.as_ref()?.view.record.path.clone();
        Some(self.command(
            Pending::Editor {
                generation: self.note_generation,
                preserve: true,
            },
            AppCommand::OpenEditor(path),
        ))
    }
    pub fn recover_editor(&mut self) -> Option<(Uuid, AppCommand)> {
        let id = Uuid::new_v4();
        let edit = self.editor.as_mut()?.begin(id, None)?;
        self.pending.insert(id, Pending::EditorRecovery);
        Some((id, AppCommand::RecoverEditor(edit)))
    }
    pub fn save_editor(&mut self, destination: Option<String>) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        let editor = self.editor.as_mut()?;
        if destination.is_none() && !editor.can_save() {
            return None;
        }
        let id = Uuid::new_v4();
        let edit = editor.begin(id, destination.clone())?;
        self.pending.insert(id, Pending::EditorSave);
        Some((
            id,
            AppCommand::SaveEditor(SaveRequest {
                operation_id: id,
                edit,
                destination,
            }),
        ))
    }
    pub fn reload_editor(&mut self, request: ReloadRequest) -> Option<(Uuid, AppCommand)> {
        let editor = self.editor.as_mut()?;
        if !editor.can_leave()
            || editor.view.record.path != request.path
            || editor.acknowledged != request.expected
            || editor.view.observed.as_ref() != Some(&request.observed)
        {
            return None;
        }
        let reviewed = editor.view.saved.clone()?;
        let id = Uuid::new_v4();
        editor.begin(id, None)?;
        editor.pending.as_mut()?.reload_text = Some(reviewed);
        self.pending.insert(id, Pending::EditorReload);
        Some((id, AppCommand::ReloadEditor(request)))
    }
    pub fn can_ask(&self) -> bool {
        self.session_allows_new_work()
            && self.ready
            && !self.application_busy()
            && self.vault_bound
            && self.selection.is_some()
            && self.selection_error.is_none()
            && self.effort.is_some()
            && self.effort_error.is_none()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.unsaved.is_none()
            && !self
                .pending
                .values()
                .any(|p| matches!(p, Pending::Select | Pending::Effort | Pending::SelectEffort))
    }
    pub fn ask(&mut self, question: String) -> Option<AskRequest> {
        if !self.can_ask() || question.trim().is_empty() {
            return None;
        }
        let request = AskRequest {
            id: Uuid::new_v4(),
            conversation: self.conversation,
            question,
            selection: self.selection.clone()?,
            effort: self.effort,
            budget: Some(self.work_budget),
            generation: self.generation,
        };
        self.active = Some(ActiveTurn {
            request: request.clone().into(),
            partial: String::new(),
            tool: None,
            stopping: false,
            budget_progress: None,
            time_limit_reached: false,
        });
        self.notice = "Answer requested; provisional until local finalization.".into();
        Some(request)
    }
    pub fn stop(&mut self) -> Option<Uuid> {
        let active = self.active.as_mut()?;
        active.stopping = true;
        Some(active.request.id())
    }
    pub fn stop_controls(&self) -> Vec<(Uuid, AppCommand)> {
        let mut commands = Vec::new();
        if let Some(active) = &self.rewrite
            && active.stopping
        {
            commands.push((Uuid::new_v4(), AppCommand::CancelTurn(active.request.id)));
        }
        if let Some(active) = &self.active
            && active.stopping
        {
            commands.push((Uuid::new_v4(), AppCommand::CancelTurn(active.request.id())));
        }
        if let Some(id) = self.cancelled_login {
            commands.push((Uuid::new_v4(), AppCommand::CancelAccount(id)));
        }
        if self.download_stopping
            && let Some(id) = self.download
        {
            commands.push((Uuid::new_v4(), AppCommand::CancelModelDownload(id)));
        }
        commands
    }
    pub fn account_busy(&self, provider: Provider) -> bool {
        self.pending.values().any(
            |p| matches!(p, Pending::Account(command) if account_provider(command) == provider),
        )
    }
    pub fn account(&mut self, command: AccountCommand) -> Option<(Uuid, AppCommand)> {
        let provider = account_provider(&command);
        if !self.ready || self.account_busy(provider) {
            return None;
        }
        let id = Uuid::new_v4();
        if matches!(command, AccountCommand::Connect(_)) {
            if self.login.is_some() {
                return None;
            }
            self.login = Some(LoginDialog {
                operation: id,
                prompt: None,
            });
            self.accounts[slot(provider)].error = None;
        }
        if matches!(
            command,
            AccountCommand::Connect(_) | AccountCommand::Disconnect(_) | AccountCommand::Models(_)
        ) {
            self.accounts[slot(provider)].models.clear();
            self.accounts[slot(provider)].error = None;
        }
        self.pending.insert(id, Pending::Account(command.clone()));
        Some((id, AppCommand::Account { id, command }))
    }
    pub fn dismiss_login(&mut self) -> Option<Uuid> {
        let id = self.login.take()?.operation;
        self.cancelled_login = Some(id);
        Some(id)
    }
    pub fn navigate(&mut self, conversation: Option<Uuid>) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.session_change_pending() {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.conversation = conversation;
        self.turns.clear();
        self.run_budgets.clear();
        conversation.map(|id| {
            self.command(
                Pending::Turns {
                    generation: self.generation,
                },
                AppCommand::Turns(id),
            )
        })
    }
    pub fn apply(&mut self, id: Uuid, event: AppEvent) -> Vec<(Uuid, AppCommand)> {
        let mut commands = Vec::new();
        // Backup notifications are independent of the operation whose commit made
        // state dirty. Never let them settle that operation or replace its buffers.
        let backup_request = matches!(
            self.pending.get(&id),
            Some(Pending::BackupStatus | Pending::CheckpointBackup)
        );
        if let AppEvent::BackupStatus(status) = event {
            if id.is_nil() || backup_request {
                self.backup_status = Some(status);
                if backup_request {
                    self.backup_request_error = None;
                    self.pending.remove(&id);
                }
            }
            return commands;
        }
        if backup_request {
            if let AppEvent::Failed(error) = event {
                self.backup_request_error = Some(error.message);
                self.pending.remove(&id);
            }
            return commands;
        }
        if let Some(commands) = self.apply_session_event(id, &event) {
            return commands;
        }
        if let AppEvent::Rewrite(event) = event {
            let Some(active) = self.rewrite.as_ref() else {
                return commands;
            };
            if id != event.id()
                || id != active.request.id
                || event.generation() != active.request.generation
            {
                return commands;
            }
            if let RewriteEvent::Started { job, .. }
            | RewriteEvent::AlreadyRunning { job, .. }
            | RewriteEvent::Finished { job, .. } = &event
                && active.request.check_replay(job).is_err()
            {
                return commands;
            }
            match event {
                RewriteEvent::Started { job, .. } => self.rewrite.as_mut().unwrap().job = Some(job),
                RewriteEvent::ToolStarted { name, .. } => {
                    self.rewrite.as_mut().unwrap().tool = Some(name)
                }
                RewriteEvent::AlreadyRunning { job, .. } | RewriteEvent::Finished { job, .. } => {
                    let request = self.rewrite.take().unwrap().request;
                    self.notice = format!(
                        "Rewrite {:?}. Review the full proposal before approval.",
                        job.status
                    );
                    self.last_rewrite = Some(job);
                    if self.review_generation == request.generation
                        && self
                            .review
                            .as_ref()
                            .is_some_and(|review| review.record.draft.id == request.expected.id)
                        && let Some(command) = self.refresh_review()
                    {
                        commands.push(command);
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                RewriteEvent::Rejected { error, .. } => {
                    self.rewrite = None;
                    self.notice = error.message;
                }
                RewriteEvent::PersistenceFailed { error, .. } => {
                    self.rewrite = None;
                    self.rewrite_storage_failed = true;
                    self.notice = format!(
                        "{} Reopen before another Rewrite; result finalization is not acknowledged.",
                        error.message
                    );
                }
            }
            commands.extend(self.stop_controls());
            return commands;
        }
        if let AppEvent::Chat(event) = event {
            if let Some(active) = &self.active
                && event.id() == active.request.id()
                && id == event.id()
                && event.generation() == active.request.generation()
            {
                if active.request.inbox().is_some() {
                    let valid = match &event {
                        ChatEvent::Finished { turn, .. } => {
                            active.request.accepts_inbox_turn(turn)
                                && turn.status != WorkTurnStatus::Running
                        }
                        ChatEvent::AlreadyRunning { turn, .. } => {
                            active.request.accepts_inbox_turn(turn)
                                && turn.status == WorkTurnStatus::Running
                        }
                        _ => true,
                    };
                    if !valid {
                        return commands;
                    }
                }
                let display = self.display_active().is_some();
                let analysis = active.request.inbox().map(|request| request.id);
                match event {
                    ChatEvent::BudgetProgress {
                        budget,
                        model_turns,
                        tool_rounds,
                        ..
                    } => {
                        let active = self.active.as_mut().unwrap();
                        let monotonic = active.budget_progress.is_none_or(|(models, rounds)| {
                            model_turns >= models && tool_rounds >= rounds
                        });
                        if active.request.budget() == Some(budget)
                            && !active.time_limit_reached
                            && model_turns <= budget.max_tool_rounds + 1
                            && tool_rounds <= budget.max_tool_rounds
                            && monotonic
                        {
                            active.budget_progress = Some((model_turns, tool_rounds));
                        }
                        return self.stop_controls();
                    }
                    ChatEvent::BudgetStopping { .. } => {
                        let active = self.active.as_mut().unwrap();
                        active.stopping = true;
                        active.time_limit_reached = true;
                        self.notice = "Time limit reached; stopping and finalizing".into();
                        return self.stop_controls();
                    }
                    ChatEvent::Text { text, .. } => {
                        let active = self.active.as_mut().unwrap();
                        if !active.time_limit_reached {
                            active.partial.push_str(&text);
                        }
                        return self.stop_controls();
                    }
                    ChatEvent::ToolStarted { name, .. } => {
                        let active = self.active.as_mut().unwrap();
                        if !active.time_limit_reached {
                            active.tool = Some(name);
                        }
                        return self.stop_controls();
                    }
                    ChatEvent::Finished { turn, .. } => {
                        let timed_out = turn.error_code.as_deref() == Some("time_limit_reached");
                        if display {
                            self.conversation = Some(turn.conversation_id);
                            let turn_id = turn.id;
                            self.upsert_turn(turn);
                            commands.push(self.request_run_budget(turn_id));
                        }
                        self.notice = if timed_out {
                            "Time limit reached; turn finalized locally."
                        } else {
                            "Turn finalized locally. Stop does not prove upstream cancellation or no billing."
                        }.into();
                        commands.push(self.refresh_session_summaries());
                        if display {
                            commands.extend(self.request_selected_lifecycle());
                        }
                    }
                    ChatEvent::AlreadyRunning { turn, .. } => {
                        self.notice =
                            "This turn is already recorded Running; it was not resubmitted.".into();
                        if display {
                            let turn_id = turn.id;
                            self.upsert_turn(turn);
                            commands.push(self.request_run_budget(turn_id));
                        }
                    }
                    ChatEvent::Rejected { error, .. } => {
                        self.notice = error.message;
                        if !active.partial.is_empty() {
                            self.unsaved =
                                Some(unfinalized_turn(&active.request, active.partial.clone()));
                            self.notice.push_str(
                                " Partial retained; local finalization not acknowledged.",
                            );
                        }
                    }
                    ChatEvent::PersistenceFailed { partial, error, .. } => {
                        self.unsaved = Some(unfinalized_turn(&active.request, partial));
                        self.notice = format!(
                            "Local finalization failed; partial answer not saved. {}",
                            error.message
                        );
                    }
                }
                self.active = None;
                commands.extend(self.analysis_settled(analysis));
            }
            return commands;
        }
        if let AppEvent::Account(event) = event {
            match event {
                AccountEvent::Login {
                    id: operation,
                    prompt,
                } => {
                    if operation == id
                        && let Some(login) = &mut self.login
                        && login.operation == id
                    {
                        login.prompt = Some(prompt);
                    }
                }
                AccountEvent::Finished {
                    id: operation,
                    provider,
                    reply,
                } => {
                    if id != operation {
                        return commands;
                    }
                    let Some(Pending::Account(command)) = self.pending.remove(&id) else {
                        return commands;
                    };
                    if account_provider(&command) != provider {
                        return commands;
                    }
                    if self.login.as_ref().is_some_and(|l| l.operation == id) {
                        self.login = None;
                    }
                    if self.cancelled_login == Some(id) {
                        self.cancelled_login = None;
                    }
                    let row = &mut self.accounts[slot(provider)];
                    match reply {
                        AccountReply::Status(status) => row.status = Some(status),
                        AccountReply::Disconnected => {
                            row.status = Some(AccountStatus {
                                provider,
                                connected: false,
                                name: None,
                            });
                            row.error = None;
                        }
                        AccountReply::Models(models) => row.models = models,
                        AccountReply::Cancelled => {
                            row.error =
                                Some("Connection cancelled. Retry only by explicit Connect.".into())
                        }
                        AccountReply::Failed(error) => row.error = Some(error.to_string()),
                        AccountReply::Rejected(error) => row.error = Some(error.message),
                    }
                    // Cache persistence can precede a failed/cancelled display-name lookup.
                    if matches!(command, AccountCommand::Connect(_))
                        && let Some(command) = self.account(AccountCommand::Status(provider))
                    {
                        commands.push(command);
                    }
                }
            }
            return commands;
        }
        if let Some(commands) = self.received_dashboard(id, &event) {
            return commands;
        }
        if let Some(commands) = self.received_findings(id, &event) {
            return commands;
        }
        if let Some(commands) = self.received_inbox_copy(id, &event) {
            return commands;
        }
        if let Some(commands) = self.received_guided_inbox(id, &event) {
            return commands;
        }
        if self.apply_inbox_event(id, &event) {
            commands.extend(self.take_inbox_followups());
            return commands;
        }
        if self.received_inbox_analysis(id, &event) {
            commands.extend(self.guided_analysis_ready(id, &event));
            return commands;
        }
        if self.received_link_preparation(id, &event) {
            return commands;
        }
        if let Some(Pending::DraftSource {
            form,
            path,
            binding_generation,
        }) = self.pending.get(&id).cloned()
        {
            let current = self.draft.as_ref().is_some_and(|draft| {
                draft.id == form
                    && draft.path == path
                    && draft.binding_generation == binding_generation
                    && draft.source_operation == Some(id)
            });
            if !current {
                self.pending.remove(&id);
                return commands;
            }
            match event {
                AppEvent::ProposalSource(capture)
                    if capture.source.path == path
                        && capture.source.fingerprint.len == capture.text.len() as u64 =>
                {
                    let draft = self.draft.as_mut().expect("matched form");
                    if draft.action.is_some() {
                        if !draft.retain_action_source(*capture) {
                            return commands;
                        }
                    } else {
                        draft.source = Some(*capture);
                    }
                    draft.source_operation = None;
                    draft.source_error = None;
                }
                AppEvent::Failed(error) => {
                    let draft = self.draft.as_mut().expect("matched form");
                    draft.source_error = Some(error.message);
                    draft.source_operation = None;
                }
                _ => return commands,
            }
            self.pending.remove(&id);
            return commands;
        }
        if let Some(Pending::DraftCreate { form, request }) = self.pending.get(&id).cloned() {
            match event {
                AppEvent::Proposal(record) if crate::draft::creation_matches(&request, &record) => {
                    if let Some(draft) = self.draft.as_mut().filter(|draft| draft.id == form)
                        && !draft.created(id, record.clone())
                    {
                        return commands;
                    }
                    self.notice = format!(
                        "Current proposal {} returned at review version {} · {:?}. No application was requested.",
                        record.draft.id, record.version, record.state
                    );
                }
                AppEvent::Failed(error) => {
                    if let Some(draft) = self.draft.as_mut().filter(|draft| draft.id == form) {
                        draft.failed(id, error.message.clone());
                    }
                    self.notice = format!(
                        "Draft creation returned an error: {} Retained input and the requested proposal UUID remain available; inspect recorded review work before retrying.",
                        error.message
                    );
                }
                _ => return commands,
            }
            self.pending.remove(&id);
            commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
            return commands;
        }
        if let Some(action @ (Pending::UndoPreview { .. } | Pending::RepairPreview { .. })) =
            self.pending.get(&id).cloned()
        {
            let generation = match &action {
                Pending::UndoPreview { generation, .. }
                | Pending::RepairPreview { generation, .. } => *generation,
                _ => unreachable!(),
            };
            if generation != self.operation_generation {
                self.pending.remove(&id);
                return commands;
            }
            match (action, event) {
                (Pending::UndoPreview { request, .. }, AppEvent::ProposalUndoPreview(preview)) => {
                    let Some(capture) = crate::approval::UndoCapture::new(request, preview) else {
                        return commands;
                    };
                    self.undo_preview = Some(capture);
                }
                (
                    Pending::RepairPreview {
                        operation,
                        direction,
                        ..
                    },
                    AppEvent::ProposalRepairPreview(preview),
                ) if preview.operation_id == operation => {
                    let Some(capture) = crate::approval::RepairCapture::new(preview, direction)
                    else {
                        return commands;
                    };
                    self.repair_preview = Some(capture);
                }
                (_, AppEvent::Failed(error)) => {
                    self.operation_error = Some(error.message.clone());
                    self.notice = format!("Operation preview refused: {}", error.message);
                }
                _ => return commands,
            }
            self.pending.remove(&id);
            return commands;
        }
        if let Some(action @ (Pending::Undo { .. } | Pending::Repair { .. })) =
            self.pending.get(&id).cloned()
        {
            let (proposal, generation, original) = match &action {
                Pending::Undo {
                    capture,
                    generation,
                } => (
                    capture.preview().draft.id,
                    *generation,
                    capture.request().operation_id,
                ),
                Pending::Repair {
                    capture,
                    generation,
                } => (
                    capture.preview().approved.id,
                    *generation,
                    capture.request().operation_id,
                ),
                _ => unreachable!(),
            };
            match (&action, event) {
                (Pending::Undo { capture, .. }, AppEvent::ProposalApplied(receipt))
                    if capture.accepts_receipt(&receipt) =>
                {
                    self.notice = format!(
                        "Undo operation {} recorded {:?}.",
                        receipt.operation_id, receipt.outcome
                    );
                    self.approval_receipts = vec![receipt];
                    self.approval_error = None;
                }
                (Pending::Repair { capture, .. }, AppEvent::ProposalRepaired(receipt))
                    if capture.accepts_receipt(&receipt) =>
                {
                    self.notice = format!(
                        "Repair attempt {} recorded {}.",
                        receipt.id,
                        match receipt.outcome {
                            Some(ApplyOutcome::Applied) => "Applied",
                            Some(ApplyOutcome::NotApplied) => "Not applied",
                            Some(ApplyOutcome::Uncertain) => "Uncertain",
                            None => "pending; outcome unconfirmed",
                        }
                    );
                    self.repair_receipt = Some(receipt);
                }
                (_, AppEvent::Failed(error)) => {
                    self.operation_error = Some(error.message.clone());
                    self.notice = format!(
                        "Operation returned an error: {} Inspect its recorded state; an error does not establish no file effects.",
                        error.message
                    );
                }
                _ => return commands,
            }
            self.pending.remove(&id);
            let mut proposals = vec![proposal];
            if matches!(action, Pending::Undo { .. })
                && generation == self.review_generation
                && let Some(review) = &self.review
            {
                proposals.push(review.record.draft.id);
            }
            commands.extend(self.refresh_after_application(&proposals, generation));
            if let Some(command) = self.inspect_apply(original) {
                commands.push(command);
            }
            return commands;
        }
        // Application has its own exact captured request. Unexpected receipt
        // IDs/types do not acknowledge or release admitted critical work.
        if matches!(self.pending.get(&id), Some(Pending::ApplySnapshot { generation, .. }) if *generation == self.snapshot_generation)
            && !matches!(&event, AppEvent::ProposalApply(_) | AppEvent::Failed(_))
        {
            return commands;
        }
        if matches!(self.pending.get(&id), Some(Pending::AppliedReview { generation, .. }) if *generation == self.review_generation)
            && !matches!(&event, AppEvent::Proposal(_) | AppEvent::Failed(_))
        {
            return commands;
        }
        if let Some(action @ (Pending::Approval { .. } | Pending::ApplyReconcile { .. })) =
            self.pending.get(&id).cloned()
        {
            let (proposals, generation, terminal) = match (&action, event) {
                (
                    Pending::Approval {
                        capture,
                        generation,
                    },
                    AppEvent::ProposalApplied(receipt),
                ) if capture.group_id().is_none() && capture.accepts_receipt(0, &receipt) => (
                    capture
                        .records()
                        .iter()
                        .map(|record| record.draft.id)
                        .collect::<Vec<_>>(),
                    *generation,
                    Ok((vec![receipt], None)),
                ),
                (
                    Pending::Approval {
                        capture,
                        generation,
                    },
                    AppEvent::ProposalGroupApplied(result),
                ) if capture.group_id().is_some() && capture.accepts_group(&result) => (
                    capture
                        .records()
                        .iter()
                        .map(|record| record.draft.id)
                        .collect(),
                    *generation,
                    Ok((result.receipts, result.stopped.map(|stop| stop.message))),
                ),
                (
                    Pending::ApplyReconcile {
                        request,
                        generation,
                    },
                    AppEvent::ProposalApplied(receipt),
                ) if crate::approval::receipt_matches(request, &receipt) => (
                    vec![request.expected.id],
                    *generation,
                    Ok((vec![receipt], None)),
                ),
                (
                    Pending::Approval {
                        capture,
                        generation,
                    },
                    AppEvent::Failed(error),
                ) => (
                    capture
                        .records()
                        .iter()
                        .map(|record| record.draft.id)
                        .collect(),
                    *generation,
                    Err(error.message),
                ),
                (
                    Pending::ApplyReconcile {
                        request,
                        generation,
                    },
                    AppEvent::Failed(error),
                ) => (vec![request.expected.id], *generation, Err(error.message)),
                _ => return commands,
            };
            self.pending.remove(&id);
            match terminal {
                Ok((receipts, stopped)) => {
                    let applied = receipts
                        .iter()
                        .filter(|receipt| receipt.outcome == ApplyOutcome::Applied)
                        .count();
                    self.notice = if stopped.is_some() {
                        format!(
                            "Captured group stopped; {applied} recorded Applied. Inspect each outcome and the remaining proposals."
                        )
                    } else if receipts
                        .iter()
                        .all(|receipt| receipt.outcome == ApplyOutcome::Applied)
                    {
                        format!("{applied} proposal(s) recorded Applied.")
                    } else {
                        "Application outcome is not Applied; inspect recorded proofs and retained review work.".into()
                    };
                    self.approval_receipts = receipts;
                    self.approval_error = stopped;
                }
                Err(error) => {
                    self.approval_error = Some(error.clone());
                    self.notice = format!(
                        "Application returned an error: {error} Inspect recorded operations; an error does not establish no file effects."
                    );
                }
            }
            commands.extend(self.refresh_after_application(&proposals, generation));
            return commands;
        }
        let pending = self.pending.get(&id).cloned();
        match event {
            AppEvent::Ready {
                vault_bound,
                model_installed,
            } => {
                self.ready = true;
                self.vault_bound = vault_bound;
                self.model_installed = model_installed;
                self.model_state = if model_installed {
                    "Installed"
                } else {
                    "Keyword-only; no local model installed"
                }
                .into();
                for (pending, command) in [
                    (Pending::Status, AppCommand::Status),
                    (Pending::BackupStatus, AppCommand::BackupStatus),
                    (Pending::Selection, AppCommand::Selection),
                    (Pending::Effort, AppCommand::Effort),
                    (Pending::Proposals, AppCommand::Proposals(None)),
                    (Pending::Prompt, AppCommand::ModelPrompt),
                    (Pending::Editors, AppCommand::Editors),
                ] {
                    commands.push(self.command(pending, command));
                }
                commands.push(self.refresh_session_summaries());
                for provider in [Provider::Chatgpt, Provider::Copilot] {
                    if let Some(c) = self.account(AccountCommand::Status(provider)) {
                        commands.push(c);
                    }
                }
                if vault_bound {
                    commands.push(self.command(Pending::Refresh, AppCommand::Refresh));
                }
                self.notice = if vault_bound {
                    "Workspace ready."
                } else {
                    "Choose a vault to read notes. Accounts and history remain available."
                }
                .into();
            }
            AppEvent::Restored { backup } => self.restored = Some(backup),
            AppEvent::Status(status) => {
                self.vault_root = status.vault_root;
                self.model_installed = status.model_installed;
            }
            AppEvent::Selection(selection) => {
                self.provider = selection.as_ref().map(|s| s.provider);
                self.selection = selection;
                self.selection_error = None;
            }
            AppEvent::SelectionSaved => {
                commands.push(self.command(Pending::Selection, AppCommand::Selection))
            }
            AppEvent::Effort(effort) => {
                if !matches!(pending, Some(Pending::Effort)) {
                    return commands;
                }
                self.effort = effort;
                self.effort_error = None;
            }
            AppEvent::EffortSaved => {
                if !matches!(pending, Some(Pending::SelectEffort)) {
                    return commands;
                }
                commands.push(self.command(Pending::Effort, AppCommand::Effort))
            }
            AppEvent::VaultBound => {
                self.vault_bound = true;
                commands.push(self.command(Pending::Status, AppCommand::Status));
                commands.push(self.command(Pending::Refresh, AppCommand::Refresh));
            }
            AppEvent::Refreshed(report) => {
                self.clear_relationships();
                self.refresh = Some(report);
                if let Some(command) = self.refresh_notes() {
                    commands.push(command);
                }
            }
            AppEvent::Notes(page) => {
                if let Some(Pending::Notes {
                    scope,
                    generation,
                    cursor,
                }) = &pending
                    && *scope == self.knowledge_scope
                    && *generation == self.notes_generation
                    && (cursor.is_none() || *cursor == self.next_cursor)
                {
                    if cursor.is_some() {
                        self.notes.extend(page.notes);
                    } else {
                        self.notes = page.notes;
                    }
                    self.next_cursor = page.next_cursor;
                    self.notes_error = None;
                }
            }
            AppEvent::Proposals(records) => {
                if matches!(pending, Some(Pending::Proposals)) {
                    self.proposals = records;
                }
            }
            AppEvent::Proposal(record) => match pending {
                Some(Pending::Proposal {
                    id: proposal,
                    generation,
                }) if generation == self.review_generation && proposal == record.draft.id => {
                    self.review = Some(crate::review::ProposalReview::new(record));
                    self.review_error = None;
                    commands.extend(self.request_review_lifecycle());
                }
                Some(Pending::CreateRename { generation })
                    if generation == self.review_generation =>
                {
                    if let Some(review) = &mut self.review {
                        let unchanged = review.record == record;
                        if review.acknowledge_create_rename(id, record) {
                            self.notice = if unchanged {
                                "Destination already current; the exact review is unchanged.".into()
                            } else {
                                "New-note destination acknowledged. Inspect the revised full proposal before exact approval.".into()
                            };
                        }
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                Some(Pending::KnowledgePredecessor { generation })
                    if generation == self.review_generation =>
                {
                    if let Some(review) = &mut self.review
                        && review.acknowledge_predecessor(id, record)
                    {
                        self.notice = "Predecessor attached. Review the full successor and protected History member before exact approval.".into();
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                Some(Pending::ReviewEdit) => {
                    if let Some(review) = &mut self.review {
                        review.acknowledge_edit(id, record);
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                Some(
                    Pending::ReviewMutation {
                        id: proposal,
                        generation,
                    }
                    | Pending::ReviewRefresh {
                        id: proposal,
                        generation,
                    },
                ) if generation == self.review_generation && proposal == record.draft.id => {
                    if let Some(review) = &mut self.review {
                        review.observe(record);
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                Some(Pending::AppliedReview {
                    id: proposal,
                    generation,
                }) if generation == self.review_generation => {
                    if record.draft.id != proposal
                        || self
                            .review
                            .as_mut()
                            .is_none_or(|review| !review.observe(record))
                    {
                        return commands;
                    }
                    commands.push(self.command(Pending::Proposals, AppCommand::Proposals(None)));
                }
                _ => {}
            },
            AppEvent::Activity(page) => {
                if let Some(Pending::Activity { generation, before }) = pending
                    && generation == self.activity_generation
                    && (before.is_none()
                        || self
                            .activity
                            .as_ref()
                            .is_some_and(|old| old.next_before == before))
                {
                    if before.is_some() {
                        let old = self.activity.as_mut().expect("matched current page");
                        for entry in page.entries {
                            if !old
                                .entries
                                .iter()
                                .any(|known| known.operation_id == entry.operation_id)
                            {
                                old.entries.push(entry);
                            }
                        }
                        old.next_before = page.next_before;
                    } else {
                        self.activity = Some(page);
                    }
                    self.activity_error = None;
                }
            }
            AppEvent::ProposalRecovery(operations) => {
                if matches!(pending, Some(Pending::Applies { generation }) if generation == self.applies_generation)
                {
                    self.applies = operations;
                    self.applies_error = None;
                }
            }
            AppEvent::ProposalApply(journal) => {
                if let Some(Pending::ApplySnapshot {
                    operation,
                    generation,
                }) = pending
                    && generation == self.snapshot_generation
                {
                    if journal
                        .as_ref()
                        .is_some_and(|journal| journal.request.operation_id != operation)
                    {
                        // A misbound body cannot acknowledge this selected
                        // lookup; keep its exact request pending for its reply.
                        return commands;
                    }
                    self.snapshot_error = journal
                        .is_none()
                        .then(|| "Recorded operation does not exist.".into());
                    self.application_snapshot = journal.map(|journal| *journal);
                }
            }
            AppEvent::Note(note) => {
                if let Some(Pending::Evidence {
                    path,
                    scope,
                    generation,
                }) = &pending
                    && *generation == self.note_generation
                    && let Some(evidence) = &mut self.evidence
                    && evidence.path == *path
                    && evidence.scope == *scope
                {
                    evidence.note = Some(note);
                    self.note_error = None;
                }
            }
            AppEvent::ProposalUndoPreview(_)
            | AppEvent::ProposalRepairPreview(_)
            | AppEvent::ProposalRepaired(_)
            | AppEvent::ProposalSource(_)
            | AppEvent::ProposalAsset(_)
            | AppEvent::Finding(_)
            | AppEvent::Findings(_)
            | AppEvent::Action(_)
            | AppEvent::ActionCompleted(_)
            | AppEvent::Actions(_)
            | AppEvent::ActionDashboard(_)
            | AppEvent::FindingInspection(_)
            | AppEvent::NoteConflicts(_)
            | AppEvent::NoteIdentity(_)
            | AppEvent::IdentityInventory(_)
            | AppEvent::NoteIdentityResolved(_)
            | AppEvent::EvidenceNote(_)
            | AppEvent::NoteIdentityDraft(_)
            | AppEvent::NoteLinkDraft(_)
            | AppEvent::CitationCaptured(_)
            | AppEvent::NoteProvenanceDraft(_)
            | AppEvent::ProposalApplied(_)
            | AppEvent::ProposalGroupApplied(_)
            | AppEvent::ProposalApplies(_) => {}
            AppEvent::NoteLinks(links) => self.received_links(pending.as_ref(), *links),
            AppEvent::Relationships(page) => self.received_relationships(pending.as_ref(), *page),
            AppEvent::NoteProvenance(provenance) => {
                if let Some(Pending::Provenance {
                    path,
                    document_generation,
                    inspection_generation,
                }) = &pending
                    && provenance.path == *path
                    && self.provenance_request_matches(
                        path,
                        *document_generation,
                        *inspection_generation,
                    )
                {
                    self.provenance = Some(*provenance);
                    self.provenance_error = None;
                }
            }
            AppEvent::Editor(view) => {
                if let Some(Pending::Editor {
                    generation,
                    preserve,
                }) = pending
                    && generation == self.note_generation
                {
                    if preserve {
                        if let Some(editor) = &mut self.editor {
                            editor.observe(view);
                        }
                    } else {
                        self.editor = Some(SimpleEditor::new(view));
                    }
                }
            }
            AppEvent::EditorRecovered(record) => {
                if matches!(
                    pending,
                    Some(Pending::EditorRecovery | Pending::EditorReload)
                ) {
                    if matches!(pending, Some(Pending::EditorReload))
                        && self.saved_document_path() == Some(record.path.as_str())
                    {
                        self.clear_provenance();
                        self.clear_links();
                        self.clear_relationships();
                    }
                    if let Some(editor) = &mut self.editor {
                        if matches!(pending, Some(Pending::EditorReload)) {
                            editor.reloaded(id, record);
                        } else {
                            editor.recovered(id, record);
                        }
                    }
                    if matches!(pending, Some(Pending::EditorReload))
                        && let Some(command) = self.refresh_editor()
                    {
                        commands.push(command);
                    }
                    commands.push(self.command(Pending::Editors, AppCommand::Editors));
                }
            }
            AppEvent::EditorSaved(receipt) => {
                if matches!(
                    pending,
                    Some(Pending::EditorSave | Pending::EditorReconcile)
                ) {
                    if receipt.outcome == SaveOutcome::Applied
                        && receipt.destination.is_none()
                        && self.saved_document_path() == Some(receipt.path.as_str())
                    {
                        self.clear_provenance();
                    }
                    if receipt.outcome == SaveOutcome::Applied {
                        // A copy can introduce duplicate UUIDs or satisfy a
                        // formerly absent target even with the original intact.
                        self.clear_links();
                        self.clear_relationships();
                    }
                    if let Some(editor) = &mut self.editor {
                        if matches!(pending, Some(Pending::EditorReconcile)) {
                            editor.reconciled(&receipt);
                        } else {
                            editor.saved(id, &receipt);
                        }
                    }
                    if let Some(command) = self.refresh_editor() {
                        commands.push(command);
                    }
                    commands.push(self.command(Pending::Editors, AppCommand::Editors));
                }
            }
            AppEvent::Editors(editors) => self.editors = editors,
            AppEvent::Search(results) => {
                if let Some(Pending::Search { scope, generation }) = &pending
                    && *generation == self.search_generation
                    && *scope == self.knowledge_scope
                {
                    self.search = Some(results);
                    self.search_scope = Some(*scope);
                }
            }
            AppEvent::Conversations(conversations) => self.conversations = conversations,
            AppEvent::Turns(turns) => {
                if matches!(pending, Some(Pending::Turns { generation }) if generation == self.generation)
                {
                    // A queued snapshot may predate Finished. Keep known durable
                    // endings only for this display; navigation drops this overlay.
                    let terminal: Vec<_> = self
                        .turns
                        .iter()
                        .filter(|turn| turn.status != WorkTurnStatus::Running)
                        .cloned()
                        .collect();
                    self.turns = turns;
                    for turn in terminal {
                        self.upsert_turn(turn);
                    }
                    let ids: Vec<_> = self.turns.iter().map(|turn| turn.id).collect();
                    for turn in ids {
                        commands.push(self.request_run_budget(turn));
                    }
                    commands.extend(self.request_selected_lifecycle());
                }
            }
            AppEvent::RunBudget { id: turn, budget } => {
                if matches!(pending, Some(Pending::RunBudget { turn: expected, generation })
                    if expected == turn && generation == self.generation)
                    && self.turns.iter().any(|value| value.id == turn)
                {
                    self.run_budgets.insert(turn, budget);
                }
            }
            AppEvent::Turn(_) | AppEvent::EditRecovered => {}
            AppEvent::Indexing { embedded, total } => {
                self.indexing = Some((id, embedded, total));
                return commands;
            }
            AppEvent::ModelPrompt(prompt) => self.model_prompt = prompt,
            AppEvent::ModelDownload { received, total } => {
                if self.download == Some(id) {
                    self.download_progress = Some((received, total));
                }
                return commands;
            }
            AppEvent::ModelDownloaded(_) => {
                if self.download == Some(id) {
                    self.model_state = "Downloaded; awaiting activation (not installed)".into();
                }
                return commands;
            }
            AppEvent::ModelInstalled => {
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_installed = true;
                    self.model_state = "Installed; indexing separately".into();
                    self.model_prompt = None;
                }
            }
            AppEvent::ModelDownloadDeclined => {
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_prompt = None;
                    self.model_state = if self.model_installed {
                        "Download declined; the previously installed model is retained."
                    } else {
                        "Download declined; keyword-only. Later Download requires fresh approval."
                    }
                    .into();
                }
            }
            AppEvent::Failed(error) => {
                if let Some(Pending::RunBudget { turn, generation }) = &pending
                    && *generation == self.generation
                    && self.turns.iter().any(|value| value.id == *turn)
                {
                    self.run_budgets.insert(*turn, None);
                }
                let stale_read = match &pending {
                    Some(
                        Pending::KnowledgePredecessor { generation }
                        | Pending::CreateRename { generation },
                    ) => *generation != self.review_generation,
                    Some(Pending::RunBudget { generation, .. }) => *generation != self.generation,
                    Some(Pending::Notes {
                        scope,
                        generation,
                        cursor,
                    }) => {
                        *scope != self.knowledge_scope
                            || *generation != self.notes_generation
                            || (cursor.is_some() && *cursor != self.next_cursor)
                    }
                    Some(Pending::Search { scope, generation }) => {
                        *scope != self.knowledge_scope || *generation != self.search_generation
                    }
                    Some(
                        Pending::Evidence { generation, .. } | Pending::Editor { generation, .. },
                    ) => *generation != self.note_generation,
                    Some(Pending::Provenance {
                        path,
                        document_generation,
                        inspection_generation,
                    }) => !self.provenance_request_matches(
                        path,
                        *document_generation,
                        *inspection_generation,
                    ),
                    Some(Pending::Links {
                        path,
                        document_generation,
                        inspection_generation,
                    }) => !self.links_request_matches(
                        path,
                        *document_generation,
                        *inspection_generation,
                    ),
                    Some(Pending::Relationships {
                        scope, generation, ..
                    }) => !self.relationship_request_matches(*scope, *generation),
                    _ => false,
                };
                if stale_read {
                    self.pending.remove(&id);
                    return commands;
                }
                if matches!(pending, Some(Pending::Notes { .. })) {
                    self.notes_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Provenance { .. })) {
                    self.provenance_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Links { .. })) {
                    self.links_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Relationships { .. })) {
                    self.relationships_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Activity { generation, .. }) if generation == self.activity_generation)
                {
                    self.activity_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Applies { generation }) if generation == self.applies_generation)
                {
                    self.applies_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::ApplySnapshot { generation, .. }) if generation == self.snapshot_generation)
                {
                    self.snapshot_error = Some(error.message.clone());
                }
                let mut retained_partial = false;
                if matches!(pending, Some(Pending::Editor { generation, .. } | Pending::Evidence { generation, .. }) if generation == self.note_generation)
                {
                    self.note_error = Some(error.message.clone());
                }
                if matches!(
                    pending,
                    Some(
                        Pending::EditorRecovery
                            | Pending::EditorSave
                            | Pending::EditorReconcile
                            | Pending::EditorReload
                    )
                ) {
                    if let Some(editor) = &mut self.editor {
                        editor.failed(
                            id,
                            error.message.clone(),
                            matches!(pending, Some(Pending::EditorRecovery)),
                        );
                    }
                    if !matches!(pending, Some(Pending::EditorRecovery))
                        && let Some(command) = self.refresh_editor()
                    {
                        commands.push(command);
                    }
                }
                if self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.request.id() == id)
                {
                    let active = self.active.as_ref().unwrap();
                    let analysis = active.request.inbox().map(|request| request.id);
                    if !active.partial.is_empty() {
                        self.unsaved =
                            Some(unfinalized_turn(&active.request, active.partial.clone()));
                        retained_partial = true;
                    }
                    self.active = None;
                    commands.extend(self.analysis_settled(analysis));
                }
                if let Some(Pending::Account(command)) = &pending {
                    let provider = account_provider(command);
                    self.accounts[slot(provider)].error = Some(error.message.clone());
                    if self
                        .login
                        .as_ref()
                        .is_some_and(|login| login.operation == id)
                    {
                        self.login = None;
                    }
                    if self.cancelled_login == Some(id) {
                        self.cancelled_login = None;
                    }
                    if matches!(command, AccountCommand::Connect(_)) {
                        self.pending.remove(&id);
                        if let Some(command) = self.account(AccountCommand::Status(provider)) {
                            commands.push(command);
                        }
                    }
                }
                if matches!(pending, Some(Pending::Selection | Pending::Select)) {
                    self.selection_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::Effort | Pending::SelectEffort)) {
                    self.effort_error = Some(error.message.clone());
                }
                if matches!(pending, Some(Pending::CreateRename { generation }) if generation == self.review_generation)
                    && let Some(review) = &mut self.review
                {
                    review.fail_create_rename(id, error.message.clone());
                }
                if matches!(pending, Some(Pending::KnowledgePredecessor { generation }) if generation == self.review_generation)
                    && let Some(review) = &mut self.review
                {
                    review.fail_predecessor(id, error.message.clone());
                }
                if matches!(pending, Some(Pending::ReviewEdit))
                    && let Some(review) = &mut self.review
                {
                    review.fail_edit(id, error.message.clone());
                }
                if matches!(pending, Some(Pending::Proposal { generation, .. }) if generation == self.review_generation)
                {
                    self.review_error = Some(error.message.clone());
                }
                if let Some(
                    Pending::ReviewMutation {
                        id: proposal,
                        generation,
                    }
                    | Pending::ReviewRefresh {
                        id: proposal,
                        generation,
                    }
                    | Pending::AppliedReview {
                        id: proposal,
                        generation,
                    },
                ) = pending
                    && generation == self.review_generation
                    && let Some(review) = &mut self.review
                    && review.record.draft.id == proposal
                {
                    review.error = Some(error.message.clone());
                }
                if self
                    .rewrite
                    .as_ref()
                    .is_some_and(|active| active.request.id == id)
                {
                    self.rewrite = None;
                }
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_state = format!("Installation failed: {}", error.message);
                    self.model_prompt = None;
                }
                if self.indexing.is_some_and(|(job, _, _)| job == id) {
                    self.indexing = None;
                }
                if !self.ready {
                    self.startup_failed = true;
                }
                self.notice = error.message;
                if retained_partial {
                    self.notice
                        .push_str(" Partial retained; local finalization not acknowledged.");
                }
            }
            AppEvent::TurnCancelRequested { .. }
            | AppEvent::AccountCancelRequested { .. }
            | AppEvent::ModelCancelRequested { .. } => return commands,
            AppEvent::ConversationSummaries { .. }
            | AppEvent::ConversationLifecycle(_)
            | AppEvent::ConversationLifecycleChanged(_)
            | AppEvent::ProposalRewrite(_)
            | AppEvent::InboxActionAnalysis(_)
            | AppEvent::InboxVisualEvidence(_)
            | AppEvent::InboxVisualDraft(_)
            | AppEvent::InboxExtraction(_)
            | AppEvent::InboxRetainedExtractions { .. }
            | AppEvent::InboxIntakeBinding(_) => return commands,
            AppEvent::Rewrite(_) | AppEvent::BackupStatus(_) => unreachable!(),
            AppEvent::Chat(_)
            | AppEvent::Account(_)
            | AppEvent::InboxCaptured(_)
            | AppEvent::InboxItems(_)
            | AppEvent::InboxItem(_)
            | AppEvent::InboxReview(_)
            | AppEvent::InboxRemovalPreview(_)
            | AppEvent::InboxOriginalRemoved(_)
            | AppEvent::InboxOriginalRestored(_)
            | AppEvent::InboxOriginalRemoval { .. }
            | AppEvent::InboxOriginalRestore { .. }
            | AppEvent::InboxOriginalOperations { .. }
            | AppEvent::ArchivedInboxAnalysis { .. }
            | AppEvent::InboxProcessing(_)
            | AppEvent::InboxSourceDraft(_)
            | AppEvent::InboxCandidate(_) => unreachable!(),
        }
        self.pending.remove(&id);
        commands
    }
}
fn account_provider(command: &AccountCommand) -> Provider {
    match command {
        AccountCommand::Connect(p)
        | AccountCommand::Disconnect(p)
        | AccountCommand::Status(p)
        | AccountCommand::Models(p) => *p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::{AiError, AiErrorKind, WorkTurnStatus};

    #[cfg(target_os = "macos")]
    mod review_state {
        include!("review_state_tests.rs");
    }
    mod scope_state {
        include!("scope_state_tests.rs");
    }
    mod provenance_state {
        include!("provenance_state_tests.rs");
    }
    mod relationship_state {
        include!("relationship_state_tests.rs");
    }
    mod backups {
        include!("backup_state_tests.rs");
    }
    mod session_timestamps {
        include!("session_timestamp_tests.rs");
    }
    #[cfg(target_os = "macos")]
    mod approval_state {
        include!("approval_state_tests.rs");
    }

    fn selection() -> Selection {
        Selection {
            provider: Provider::Copilot,
            model: "explicit-model".into(),
        }
    }
    fn ready() -> AiState {
        let mut state = AiState::default();
        state.apply(
            Uuid::new_v4(),
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false,
            },
        );
        state.pending.clear();
        state.selection = Some(selection());
        state.effort = Some(ReasoningEffort::High);
        state
    }
    #[test]
    fn effort_choice_requires_acknowledgement_and_cannot_change_active_or_history() {
        let mut state = ready();
        state.effort = None;
        assert!(!state.can_ask());
        assert!(state.ask("missing effort".into()).is_none());
        let (id, _) = state.command(
            Pending::SelectEffort,
            AppCommand::SelectEffort(ReasoningEffort::High),
        );
        assert!(
            state
                .apply(Uuid::new_v4(), AppEvent::EffortSaved)
                .is_empty()
        );
        let queries = state.apply(id, AppEvent::EffortSaved);
        assert!(!state.can_ask());
        let query = queries[0].0;
        state.apply(Uuid::new_v4(), AppEvent::Effort(Some(ReasoningEffort::Low)));
        assert!(state.effort.is_none());
        state.apply(query, AppEvent::Effort(Some(ReasoningEffort::High)));
        let request = state.ask("captured choice".into()).unwrap();
        let (change, _) = state.command(
            Pending::SelectEffort,
            AppCommand::SelectEffort(ReasoningEffort::Low),
        );
        let query = state.apply(change, AppEvent::EffortSaved)[0].0;
        state.apply(query, AppEvent::Effort(Some(ReasoningEffort::Low)));
        assert_eq!(
            state.active.as_ref().unwrap().request.effort(),
            Some(ReasoningEffort::High)
        );
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: ending(&request, WorkTurnStatus::Completed),
            }),
        );
        assert_eq!(state.turns[0].effort.as_deref(), Some("high"));
        super::session_state_tests::acknowledge_active(&mut state);
        assert_eq!(
            state.ask("next choice".into()).unwrap().effort,
            Some(ReasoningEffort::Low)
        );
    }

    #[test]
    fn invalid_effort_blocks_ask_but_explicit_choice_recovers_without_changing_selection() {
        let mut state = ready();
        let selected = state.selection.clone();
        let (query, _) = state.command(Pending::Effort, AppCommand::Effort);
        state.apply(
            query,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("invalid effort")),
        );
        assert!(!state.can_ask());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Chatgpt))
                .is_some()
        );
        let (change, _) = state.command(
            Pending::SelectEffort,
            AppCommand::SelectEffort(ReasoningEffort::Medium),
        );
        let query = state.apply(change, AppEvent::EffortSaved)[0].0;
        state.apply(query, AppEvent::Effort(Some(ReasoningEffort::Medium)));
        assert!(state.can_ask());
        assert_eq!(state.selection, selected);
    }

    pub(super) fn editor_view(path: &str, text: &str) -> EditorView {
        let mut view = EditorView {
            record: EditorRecord {
                path: path.into(),
                stamp: EditStamp {
                    baseline: Uuid::new_v4(),
                    generation: 0,
                },
                baseline: serde_json::from_value(serde_json::json!({
                    "device": 1, "inode": 2, "len": text.len(), "sha256": vec![0; 32]
                }))
                .unwrap(),
                baseline_text: text.into(),
                text: text.into(),
            },
            saved: Some(text.into()),
            conflict: false,
            pending: Vec::new(),
            observed: None,
        };
        view.observed = Some(view.record.baseline.clone());
        view
    }
    fn editing(text: &str) -> AiState {
        let mut state = ready();
        let (id, _) = state.open_editor("plan.md".into());
        state.apply(id, AppEvent::Editor(editor_view("plan.md", text)));
        state
    }
    #[test]
    fn recovery_snapshot_preserves_later_typing_and_close_waits_for_latest_ack() {
        let exact = "\u{feff}---\r\ntitle: e\u{301}\n---\r\n正文 🧭\r";
        let mut state = editing(exact);
        let now = Instant::now();
        let editor = state.editor.as_mut().unwrap();
        editor.edit(format!("{exact}first"), now).unwrap();
        assert!(!editor.wants_recovery(now, false));
        assert!(editor.wants_recovery(now, true));
        let (id, AppCommand::RecoverEditor(request)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        state
            .editor
            .as_mut()
            .unwrap()
            .edit(format!("{exact}latest"), now)
            .unwrap();
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.text = request.text.clone();
        record.stamp.generation = request.generation;
        state.apply(Uuid::new_v4(), AppEvent::EditorRecovered(record.clone()));
        assert!(state.editor.as_ref().unwrap().pending());
        state.apply(id, AppEvent::EditorRecovered(record));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, format!("{exact}latest"));
        assert!(editor.dirty());
        assert!(!editor.can_leave());
        let (id, AppCommand::RecoverEditor(latest)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(latest.generation, 2);
        assert_eq!(latest.expected.generation, 1);
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.text = latest.text;
        record.stamp.generation = latest.generation;
        state.apply(id, AppEvent::EditorRecovered(record));
        assert!(state.editor.as_ref().unwrap().can_leave());
    }
    #[test]
    fn save_acknowledges_only_submitted_generation_then_recovers_newer_typing() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        let (id, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
            panic!("save");
        };
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("newer typing".into(), now)
            .unwrap();
        let new_baseline = Uuid::new_v4();
        state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: id,
                path: request.edit.path,
                destination: None,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    baseline: new_baseline,
                    generation: 1,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "newer typing");
        assert_eq!(editor.view.saved.as_deref(), Some("submitted"));
        assert!(!editor.can_leave());
        let (_, AppCommand::RecoverEditor(next)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(next.expected.baseline, new_baseline);
        assert_eq!(next.expected.generation, 1);
        assert_eq!(next.generation, 2);
    }
    #[test]
    fn stale_same_generation_observation_cannot_undo_applied_save_baseline() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        let (save, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
            panic!("save");
        };
        let mut stale = state.editor.as_ref().unwrap().view.clone();
        stale.record.stamp.generation = request.edit.generation;
        stale.record.text = request.edit.text.clone();
        let observation = state.refresh_editor().unwrap().0;
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("newer".into(), now)
            .unwrap();
        let baseline = Uuid::new_v4();
        state.apply(
            save,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: save,
                path: request.edit.path,
                destination: None,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    baseline,
                    generation: request.edit.generation,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        state.apply(observation, AppEvent::Editor(stale));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.baseline, baseline);
        assert_eq!(editor.text, "newer");
        assert_eq!(editor.view.saved.as_deref(), Some("submitted"));
        let (_, AppCommand::RecoverEditor(next)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(next.expected.baseline, baseline);
        assert_eq!(next.generation, 2);
    }
    #[test]
    fn observation_during_recovery_does_not_acknowledge_a_different_snapshot() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        state.recover_editor().unwrap();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("latest".into(), now)
            .unwrap();
        let observation = state.refresh_editor().unwrap().0;
        let mut view = state.editor.as_ref().unwrap().view.clone();
        view.record.stamp.generation = 2;
        view.record.text = "different snapshot".into();
        state.apply(observation, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.generation, 0);
        assert_eq!(editor.generation, 2);
        assert_eq!(editor.text, "latest");
        assert!(!editor.can_leave());
    }
    #[test]
    fn reconciled_original_adopts_proven_baseline_without_lowering_recovered_generation() {
        let mut state = editing("base");
        let operation = Uuid::new_v4();
        let editor = state.editor.as_mut().unwrap();
        editor.text = "newer recovered buffer".into();
        editor.generation = 2;
        editor.acknowledged.generation = 2;
        editor.view.record.text = editor.text.clone();
        editor.view.record.stamp.generation = 2;
        editor.view.pending.push(operation);
        let (id, _) = state.command(
            Pending::EditorReconcile,
            AppCommand::ReconcileEditor(operation),
        );
        let baseline = Uuid::new_v4();
        let followups = state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: operation,
                path: "plan.md".into(),
                destination: None,
                submitted_generation: 1,
                stamp: EditStamp {
                    baseline,
                    generation: 1,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let observation = followups
            .iter()
            .find_map(|(id, command)| matches!(command, AppCommand::OpenEditor(_)).then_some(*id))
            .unwrap();
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.generation, 2);
        assert_eq!(editor.acknowledged.baseline, baseline);
        let mut view = editor.view.clone();
        view.pending.clear();
        view.saved = Some("older saved snapshot".into());
        state.apply(observation, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "newer recovered buffer");
        assert_eq!(editor.generation, 2);
        assert!(editor.can_leave());
        assert!(editor.dirty());
    }
    #[test]
    fn recovery_failure_blocks_leaving_and_does_not_loop_without_explicit_retry() {
        let mut state = editing("base");
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("latest".into(), Instant::now())
            .unwrap();
        let (id, _) = state.recover_editor().unwrap();
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("disk full")),
        );
        let editor = state.editor.as_mut().unwrap();
        assert_eq!(editor.text, "latest");
        assert!(!editor.can_leave());
        assert!(!editor.wants_recovery(Instant::now() + Duration::from_secs(1), true));
        editor.retry_recovery();
        assert!(editor.wants_recovery(Instant::now(), true));
    }
    #[test]
    fn copy_keeps_original_dirty_and_cannot_resolve_uncertain_original() {
        let mut state = editing("base");
        let original = Uuid::new_v4();
        let editor = state.editor.as_mut().unwrap();
        editor.view.pending.push(original);
        editor.view.conflict = true;
        editor.edit("rescue".into(), Instant::now()).unwrap();
        assert!(state.save_editor(None).is_none());
        let (id, AppCommand::SaveEditor(request)) =
            state.save_editor(Some("rescue.md".into())).unwrap()
        else {
            panic!("copy");
        };
        state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: id,
                path: request.edit.path,
                destination: request.destination,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    generation: request.edit.generation,
                    ..request.edit.expected
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.dirty());
        assert_eq!(editor.view.saved.as_deref(), Some("base"));
        assert_eq!(editor.view.pending, vec![original]);
        assert_eq!(editor.status(), "Save outcome uncertain");
    }
    #[test]
    fn oversized_utf8_edits_retain_prior_bytes_and_generation() {
        let mut state = editing("");
        let editor = state.editor.as_mut().unwrap();
        let exact = "é".repeat(512 * 1024);
        editor.edit(exact.clone(), Instant::now()).unwrap();
        assert!(editor.edit(format!("{exact}x"), Instant::now()).is_err());
        assert_eq!(editor.text, exact);
        assert_eq!(editor.generation, 1);
    }
    #[test]
    fn confirmed_reload_binds_reviewed_disk_and_replaces_only_after_matching_ack() {
        let mut state = editing("baseline");
        let editor = state.editor.as_mut().unwrap();
        editor.view.saved = Some("external disk".into());
        editor.view.conflict = true;
        let request = editor.reload_request().unwrap();
        let (id, _) = state.reload_editor(request).unwrap();
        assert!(
            state
                .editor
                .as_mut()
                .unwrap()
                .edit("later typing".into(), Instant::now())
                .is_err()
        );
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.stamp.baseline = Uuid::new_v4();
        record.stamp.generation = 1;
        record.text = "external disk".into();
        record.baseline_text = record.text.clone();
        state.apply(Uuid::new_v4(), AppEvent::EditorRecovered(record.clone()));
        assert_eq!(state.editor.as_ref().unwrap().text, "baseline");
        state.apply(id, AppEvent::EditorRecovered(record));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "external disk");
        assert!(!editor.dirty());
        assert!(editor.can_leave());
        assert!(!editor.view.conflict);
    }
    fn ending(request: &AskRequest, status: WorkTurnStatus) -> WorkTurn {
        WorkTurn {
            id: request.id,
            conversation_id: Uuid::new_v4(),
            question: request.question.clone(),
            answer: "partial λ".into(),
            provider: "copilot".into(),
            model: request.selection.model.clone(),
            effort: request.effort.map(|value| value.as_str().to_owned()),
            started_at_ms: None,
            finished_at_ms: None,
            status,
            error_code: None,
        }
    }
    #[test]
    fn followup_same_history_click_keeps_owned_stream_and_replaces_running() {
        let mut state = ready();
        let conversation = Uuid::new_v4();
        state.navigate(Some(conversation));
        super::session_state_tests::acknowledge_active(&mut state);
        let request = state.ask("followup".into()).unwrap();
        let mut running = ending(&request, WorkTurnStatus::Running);
        running.conversation_id = conversation;
        state.turns.push(running.clone());
        let (history, _) = state.navigate(Some(conversation)).unwrap();
        state.apply(history, AppEvent::Turns(vec![running.clone()]));
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "full partial".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "full partial");
        assert!(state.display_active().is_some());
        assert_eq!(state.display_turns().count(), 0);
        running.status = WorkTurnStatus::Completed;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: running.clone(),
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.turns.len(), 1);
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
    }
    #[test]
    fn followup_away_back_and_late_running_snapshot_never_regress_terminal() {
        for snapshot_first in [true, false] {
            let mut state = ready();
            let conversation = Uuid::new_v4();
            state.navigate(Some(conversation));
            super::session_state_tests::acknowledge_active(&mut state);
            let request = state.ask("followup".into()).unwrap();
            state.navigate(Some(Uuid::new_v4()));
            assert!(state.display_active().is_none());
            for (id, generation) in [
                (Uuid::new_v4(), request.generation),
                (request.id, request.generation.wrapping_add(1)),
            ] {
                state.apply(
                    id,
                    AppEvent::Chat(ChatEvent::Text {
                        id: request.id,
                        generation,
                        text: "uncorrelated".into(),
                    }),
                );
            }
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Text {
                    id: request.id,
                    generation: request.generation,
                    text: "away ".into(),
                }),
            );
            let (history, _) = state.navigate(Some(conversation)).unwrap();
            assert!(state.display_active().is_some());
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Text {
                    id: request.id,
                    generation: request.generation,
                    text: "back".into(),
                }),
            );
            assert_eq!(state.active.as_ref().unwrap().partial, "away back");
            let mut running = ending(&request, WorkTurnStatus::Running);
            running.conversation_id = conversation;
            if snapshot_first {
                state.apply(history, AppEvent::Turns(vec![running.clone()]));
                assert_eq!(state.display_turns().count(), 0);
            }
            let mut terminal = running.clone();
            terminal.status = WorkTurnStatus::Completed;
            terminal.answer = "away back".into();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            if !snapshot_first {
                state.apply(history, AppEvent::Turns(vec![running]));
            }
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            assert!(state.active.is_none());
            assert_eq!(state.turns.len(), 1);
            assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
            assert_eq!(state.turns[0].answer, terminal.answer);
            assert_eq!(state.display_turns().count(), 1);
        }
    }
    #[test]
    fn late_running_snapshot_preserves_known_terminal_without_duplicate() {
        let mut state = ready();
        let conversation = Uuid::new_v4();
        state.navigate(Some(conversation));
        super::session_state_tests::acknowledge_active(&mut state);
        let request = state.ask("followup".into()).unwrap();
        state.navigate(Some(Uuid::new_v4()));
        let (history, _) = state.navigate(Some(conversation)).unwrap();
        let mut running = ending(&request, WorkTurnStatus::Running);
        running.conversation_id = conversation;
        let mut terminal = running.clone();
        terminal.status = WorkTurnStatus::Completed;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: terminal,
            }),
        );
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
        state.apply(history, AppEvent::Turns(vec![running]));
        assert_eq!(state.turns.len(), 1);
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
    }
    #[test]
    fn followup_finished_in_other_or_new_blank_display_clears_global_active_only() {
        for destination in [Some(Uuid::new_v4()), None] {
            let mut state = ready();
            let conversation = Uuid::new_v4();
            state.navigate(Some(conversation));
            super::session_state_tests::acknowledge_active(&mut state);
            let request = state.ask("followup".into()).unwrap();
            state.navigate(destination);
            let mut terminal = ending(&request, WorkTurnStatus::Completed);
            terminal.conversation_id = conversation;
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            assert!(state.active.is_none());
            assert!(state.turns.is_empty());
            assert_eq!(state.conversation, destination);
            let (history, _) = state.navigate(Some(conversation)).unwrap();
            state.apply(history, AppEvent::Turns(vec![terminal]));
            assert_eq!(state.display_turns().count(), 1);
        }
    }
    #[test]
    fn new_first_chat_blank_navigation_retains_unsaved_without_adopting_conversation() {
        let mut state = ready();
        let request = state.ask("first".into()).unwrap();
        state.navigate(None);
        assert!(state.display_active().is_none());
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "retained".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::PersistenceFailed {
                id: request.id,
                generation: request.generation,
                partial: "retained".into(),
                error: brn_workflow::WorkflowError::msg("unsaved"),
            }),
        );
        assert!(state.active.is_none());
        assert!(state.conversation.is_none());
        assert!(state.turns.is_empty());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "retained");
        assert!(!state.can_ask());
    }
    #[test]
    fn composer_change_keeps_stream_and_followup_conversation() {
        let mut state = ready();
        let request = state.ask("first question".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        state.composer_changed();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: " λ complete".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "partial λ complete");
        assert_eq!(
            state.generation, request.generation,
            "composer must not hide the active answer"
        );
        let mut turn = ending(&request, WorkTurnStatus::Completed);
        turn.answer = "partial λ complete".into();
        let conversation = turn.conversation_id;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn,
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.turns[0].answer, "partial λ complete");
        super::session_state_tests::acknowledge_active(&mut state);
        let followup = state.ask("followup".into()).unwrap();
        assert_eq!(followup.conversation, Some(conversation));
        assert_eq!(followup.selection, request.selection);
    }
    #[test]
    fn explicit_frozen_selection_and_single_active_ask() {
        let mut state = ready();
        let request = state.ask("question".into()).unwrap();
        state.selection = Some(Selection {
            provider: Provider::Chatgpt,
            model: "other".into(),
        });
        assert_eq!(
            state.active.as_ref().unwrap().request.selection(),
            &selection()
        );
        assert!(state.ask("second".into()).is_none());
        assert_eq!(request.selection, selection());
    }
    #[test]
    fn stop_before_admission_retries_on_first_progress_and_never_claims_saved() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        assert_eq!(state.stop(), Some(request.id));
        state.apply(
            Uuid::new_v4(),
            AppEvent::TurnCancelRequested {
                turn: request.id,
                accepted: false,
            },
        );
        let commands = state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        assert!(commands.iter().any(
            |(_, command)| matches!(command, AppCommand::CancelTurn(id) if *id == request.id)
        ));
        assert!(state.active.is_some());
    }
    #[test]
    fn stopped_and_failed_keep_partial_and_stored_identity() {
        for (status, label) in [
            (WorkTurnStatus::Interrupted, "Stopped"),
            (WorkTurnStatus::Failed, "Failed"),
        ] {
            let mut state = ready();
            let request = state.ask("q".into()).unwrap();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: ending(&request, status),
                }),
            );
            assert!(state.active.is_none());
            assert_eq!(state.turns[0].answer, "partial λ");
            assert_eq!(state.turns[0].model, "explicit-model");
            assert_eq!(turn_label(&state.turns[0]), label);
        }
    }
    #[test]
    fn stale_display_does_not_strand_global_active_and_wrong_uuid_is_ignored() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        let selected = Uuid::new_v4();
        let (history, _) = state.navigate(Some(selected)).unwrap();
        state.apply(
            Uuid::new_v4(),
            AppEvent::Chat(ChatEvent::Text {
                id: Uuid::new_v4(),
                generation: request.generation,
                text: "wrong".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "stale".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "stale");
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: ending(&request, WorkTurnStatus::Completed),
            }),
        );
        assert!(state.active.is_none());
        assert!(state.turns.is_empty());
        assert_eq!(state.conversation, Some(selected));
        state.apply(history, AppEvent::Turns(vec![]));
        super::session_state_tests::acknowledge_active(&mut state);
        assert_eq!(
            state.ask("selected followup".into()).unwrap().conversation,
            Some(selected)
        );
    }
    #[test]
    fn persistence_failure_retains_unsaved_partial() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::PersistenceFailed {
                id: request.id,
                generation: request.generation,
                partial: "unsaved".into(),
                error: brn_workflow::WorkflowError::msg("safe error"),
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "unsaved");
        assert!(state.notice.contains("not saved"));
        assert!(!state.can_ask());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Copilot))
                .is_some()
        );
    }
    #[test]
    fn account_refresh_clears_obsolete_picker_options_without_changing_selection() {
        for command in [
            AccountCommand::Connect(Provider::Chatgpt),
            AccountCommand::Disconnect(Provider::Chatgpt),
            AccountCommand::Models(Provider::Chatgpt),
        ] {
            let mut state = ready();
            let saved = state.selection.clone();
            state.accounts[0].models = vec![ModelOption {
                id: "synthetic-old-account".into(),
                live_qualified: false,
            }];
            state.accounts[1].models = vec![ModelOption {
                id: "synthetic-other-account".into(),
                live_qualified: false,
            }];
            let (id, _) = state.account(command).unwrap();
            assert!(
                state.accounts[0].models.is_empty(),
                "a new account/catalog request hides obsolete options"
            );
            state.apply(
                id,
                AppEvent::Account(AccountEvent::Finished {
                    id,
                    provider: Provider::Chatgpt,
                    reply: AccountReply::Failed(AiError::new(AiErrorKind::Network)),
                }),
            );
            assert!(
                state.accounts[0].models.is_empty(),
                "failed discovery never restores a substitute list"
            );
            assert_eq!(state.accounts[1].models[0].id, "synthetic-other-account");
            assert_eq!(
                state.selection, saved,
                "account/catalog refresh never selects another model"
            );
        }
    }

    #[test]
    fn login_cancel_exact_operation_and_failed_connect_queries_actual_status() {
        let mut state = ready();
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        state.apply(
            id,
            AppEvent::Account(AccountEvent::Login {
                id,
                prompt: brn_workflow::LoginPrompt {
                    verification_uri: "https://example.invalid".into(),
                    user_code: "synthetic".into(),
                },
            }),
        );
        assert!(state.login.is_some());
        assert_eq!(state.dismiss_login(), Some(id));
        assert!(state.login.is_none());
        let commands = state.apply(
            id,
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider: Provider::Copilot,
                reply: AccountReply::Failed(AiError::new(AiErrorKind::CodeExpired)),
            }),
        );
        assert!(state.login.is_none());
        assert!(commands.iter().any(|(_, c)| matches!(
            c,
            AppCommand::Account {
                command: AccountCommand::Status(Provider::Copilot),
                ..
            }
        )));
        assert!(
            state.accounts[1]
                .error
                .as_ref()
                .unwrap()
                .contains("expired")
        );
    }
    #[test]
    fn navigation_correlates_history_and_keeps_diagnostic_access_without_selection() {
        let mut state = ready();
        state.selection = None;
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let (old, _) = state.navigate(Some(first)).unwrap();
        let (current, _) = state.navigate(Some(second)).unwrap();
        let request = AskRequest {
            id: Uuid::new_v4(),
            conversation: None,
            question: "q".into(),
            selection: selection(),
            effort: None,
            budget: None,
            generation: 0,
        };
        state.apply(
            old,
            AppEvent::Turns(vec![ending(&request, WorkTurnStatus::Completed)]),
        );
        assert!(state.turns.is_empty());
        state.apply(
            current,
            AppEvent::Turns(vec![ending(&request, WorkTurnStatus::Completed)]),
        );
        assert_eq!(state.turns.len(), 1);
        assert!(!state.can_ask());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Chatgpt))
                .is_some()
        );
    }
    #[test]
    fn immediate_submission_failure_clears_active_and_login_and_refreshes_cache_status() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.active.is_none());
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        let commands = state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.login.is_none());
        assert!(!state.pending.contains_key(&id));
        assert!(commands.iter().any(|(_, command)| matches!(
            command,
            AppCommand::Account {
                command: AccountCommand::Status(Provider::Copilot),
                ..
            }
        )));
    }
    #[test]
    fn startup_never_admits_login_discovery_download_or_default_selection() {
        let mut state = AiState::default();
        let commands = state.apply(
            Uuid::new_v4(),
            AppEvent::Ready {
                vault_bound: false,
                model_installed: false,
            },
        );
        assert!(state.selection.is_none());
        assert!(
            commands
                .iter()
                .any(|(_, command)| matches!(command, AppCommand::Editors))
        );
        assert!(commands.iter().all(|(_, command)| matches!(
            command,
            AppCommand::Status
                | AppCommand::BackupStatus
                | AppCommand::Selection
                | AppCommand::Effort
                | AppCommand::Proposals(None)
                | AppCommand::ConversationSummaries(
                    brn_workflow::conversations::ConversationFilter::Active
                )
                | AppCommand::ModelPrompt
                | AppCommand::Editors
                | AppCommand::Account {
                    command: AccountCommand::Status(_),
                    ..
                }
        )));
        assert!(!state.can_ask());
    }
    #[test]
    fn recovery_buffer_remains_accessible_when_the_vault_is_unavailable() {
        let mut state = ready();
        state.vault_bound = false;
        let mut view = editor_view("missing.md", "exact recovery\r\n");
        view.saved = None;
        view.observed = None;
        view.conflict = true;
        state.apply(Uuid::new_v4(), AppEvent::Editors(vec![view.record.clone()]));
        assert_eq!(state.editors[0].path, "missing.md");
        let (id, _) = state.open_editor("missing.md".into());
        state.apply(id, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "exact recovery\r\n");
        assert!(!editor.can_save());
        assert!(editor.can_leave());
    }
    #[test]
    fn correlated_lane_failure_after_delta_keeps_unfinalized_partial_in_memory() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.active.is_none());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "partial");
        assert!(state.notice.contains("not acknowledged"));
    }
    #[test]
    fn failed_connect_can_be_connected_with_unavailable_name_without_automatic_retry() {
        let mut state = ready();
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        let commands = state.apply(
            id,
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider: Provider::Copilot,
                reply: AccountReply::Cancelled,
            }),
        );
        let (status, _) = commands.into_iter().next().unwrap();
        state.apply(
            status,
            AppEvent::Account(AccountEvent::Finished {
                id: status,
                provider: Provider::Copilot,
                reply: AccountReply::Status(AccountStatus {
                    provider: Provider::Copilot,
                    connected: true,
                    name: None,
                }),
            }),
        );
        assert!(state.accounts[1].status.as_ref().unwrap().connected);
        assert!(state.accounts[1].status.as_ref().unwrap().name.is_none());
        assert!(state.login.is_none());
        assert!(!state.account_busy(Provider::Copilot));
    }
    #[test]
    fn every_chat_terminal_unblocks_except_wrong_generation() {
        for choice in 0..2 {
            let mut state = ready();
            let request = state.ask("q".into()).unwrap();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Rejected {
                    id: request.id,
                    generation: request.generation + 1,
                    error: brn_workflow::WorkflowError::msg("wrong generation"),
                }),
            );
            assert!(state.active.is_some());
            let event = if choice == 0 {
                ChatEvent::Rejected {
                    id: request.id,
                    generation: request.generation,
                    error: brn_workflow::WorkflowError::msg("refused"),
                }
            } else {
                ChatEvent::AlreadyRunning {
                    id: request.id,
                    generation: request.generation,
                    turn: ending(&request, WorkTurnStatus::Running),
                }
            };
            state.apply(request.id, AppEvent::Chat(event));
            assert!(state.active.is_none());
        }
    }
    #[test]
    fn downloaded_is_not_installed_and_failure_ends_correlated_progress() {
        let mut state = ready();
        let (id, _) = state.command(
            Pending::Download,
            AppCommand::DownloadModel {
                consent: true,
                target: PathBuf::from("/synthetic/model"),
            },
        );
        state.download = Some(id);
        state.apply(
            id,
            AppEvent::ModelDownload {
                received: 7,
                total: 10,
            },
        );
        state.apply(
            id,
            AppEvent::ModelDownloaded(brn_workflow::models::ModelInstallReport {
                directory: PathBuf::from("/synthetic/model"),
                downloaded_bytes: 10,
            }),
        );
        assert!(!state.model_installed);
        assert_eq!(state.download, Some(id));
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("activation failed")),
        );
        assert!(state.download.is_none());
        assert!(state.download_progress.is_none());
        assert!(!state.model_installed);
        assert!(state.model_state.contains("failed"));
        let (decline, _) = state.command(
            Pending::Download,
            AppCommand::DownloadModel {
                consent: false,
                target: PathBuf::from("/synthetic/model"),
            },
        );
        state.download = Some(decline);
        state.apply(decline, AppEvent::ModelDownloadDeclined);
        assert!(state.download.is_none());
        assert!(state.model_prompt.is_none());
    }
    #[test]
    fn stale_selection_error_keeps_ready_history_and_accounts_available() {
        let mut state = ready();
        let (id, _) = state.command(Pending::Selection, AppCommand::Selection);
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError {
                kind: brn_workflow::ErrorKind::ModelRefused,
                message: "saved selection unavailable".into(),
            }),
        );
        assert!(state.ready);
        assert!(!state.can_ask());
        assert!(state.navigate(Some(Uuid::new_v4())).is_some());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Copilot))
                .is_some()
        );
    }
    #[test]
    fn stale_model_decline_does_not_cancel_newer_explicit_installation() {
        let mut state = ready();
        let active = Uuid::new_v4();
        state.download = Some(active);
        state.apply(Uuid::new_v4(), AppEvent::ModelDownloadDeclined);
        assert_eq!(state.download, Some(active));
    }
    #[test]
    fn declining_later_download_does_not_claim_installed_model_is_keyword_only() {
        let mut state = ready();
        state.model_installed = true;
        let id = Uuid::new_v4();
        state.download = Some(id);
        state.apply(id, AppEvent::ModelDownloadDeclined);
        assert!(state.model_installed);
        assert!(!state.model_state.contains("keyword-only"));
    }
    #[test]
    fn cancel_download_intent_survives_control_before_admission_and_ends_only_on_terminal() {
        let mut state = ready();
        let active = Uuid::new_v4();
        state.download = Some(active);
        state.download_stopping = true;
        state.apply(
            Uuid::new_v4(),
            AppEvent::ModelCancelRequested {
                operation: active,
                accepted: false,
            },
        );
        assert!(
            state
                .stop_controls()
                .iter()
                .any(|(_, c)| matches!(c, AppCommand::CancelModelDownload(id) if *id == active))
        );
        state.apply(
            active,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("cancelled")),
        );
        assert!(!state.download_stopping);
        assert!(state.stop_controls().is_empty());
    }
    #[test]
    fn note_failure_is_terminal_and_stale_read_does_not_replace_new_document() {
        let mut state = ready();
        let (old, _) = state.open_editor("old.md".into());
        let (current, _) = state.open_editor("current.md".into());
        state.apply(old, AppEvent::Editor(editor_view("old.md", "old")));
        assert!(state.editor.is_none());
        state.apply(
            current,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("note unavailable")),
        );
        assert!(!state.pending.contains_key(&current));
        assert_eq!(state.note_error.as_deref(), Some("note unavailable"));
    }
    #[test]
    fn search_refresh_unreadable_and_index_failure_are_projected_without_invented_model() {
        let mut state = ready();
        let (id, _) = state.command(
            Pending::Search {
                scope: state.knowledge_scope,
                generation: state.search_generation,
            },
            AppCommand::Search {
                query: "q".into(),
                mode: brn_workflow::library::SearchMode::Hybrid,
                limit: 10,
            },
        );
        state.apply(
            id,
            AppEvent::Search(SearchResults {
                hits: vec![],
                keyword_only: true,
            }),
        );
        assert!(state.search.as_ref().unwrap().keyword_only);
        let (refresh, _) = state.command(Pending::Refresh, AppCommand::Refresh);
        state.apply(
            refresh,
            AppEvent::Refreshed(RefreshReport {
                unreadable: vec![brn_workflow::library::Unreadable {
                    path: "bad.md".into(),
                    reason: "invalid UTF-8",
                }],
                ..Default::default()
            }),
        );
        assert_eq!(state.refresh.as_ref().unwrap().unreadable[0].path, "bad.md");
        state.apply(
            refresh,
            AppEvent::Indexing {
                embedded: 16,
                total: 35,
            },
        );
        assert_eq!(state.indexing, Some((refresh, 16, 35)));
        state.apply(
            refresh,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("index failed")),
        );
        assert!(state.indexing.is_none());
        assert!(!state.model_installed);
    }
}
