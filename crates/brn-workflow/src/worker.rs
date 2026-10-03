//! Single owned workflow worker for the native UI. No store or provider call runs on the GUI thread.
use crate::notes::{
    NoteBufferReceipt, NoteComparison, NoteFailure, NoteFileNotice, NoteNoticeSink, NoteReceipt,
    NoteRecovery, NoteResult, NoteSearchReceipt, NoteStamp, NoteSubmission, NoteView,
};
use crate::{Config, SearchResult, SessionSummary, SourceStateSummary, Workspace};
pub use brn_retrieval::{Evidence, Profile};
pub use brn_store::{
    Approval, ChatTurn, CommentAnchorSnapshot, CommentCapture, CommentCreated, CommentStatusChange,
    CommentStatusChanged, Draft, DraftComments, DraftRevision, DraftStamp, DraftWriteWithComments,
    ImportResult, MAX_DRAFT_BYTES, OperationStatus, RevisionKind, SourceDocument,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Action {
    OpenNote {
        op: Uuid,
        vault: PathBuf,
        relative: PathBuf,
    },
    ObserveNotes {
        ids: Vec<Uuid>,
    },
    NoteRecovery {
        id: Uuid,
    },
    ListNoteRecoveries,
    SaveNoteBuffer {
        request: NoteSubmission,
    },
    SaveNote {
        request: NoteSubmission,
    },
    ReconcileNoteSave {
        op: Uuid,
    },
    CompareNote {
        id: Uuid,
    },
    ReloadNote {
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        discard: bool,
    },
    RelinkNote {
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        relative: PathBuf,
        confirm_identity: bool,
    },
    SaveNoteCopy {
        request: NoteSubmission,
        relative: PathBuf,
    },
    AcceptNoteDiskState {
        op: Uuid,
        save_op: Uuid,
        file_state: Uuid,
    },
    ApproveNoteSnapshot {
        op: Uuid,
        id: Uuid,
        file_state: Uuid,
    },
    Refresh {
        session: Option<Uuid>,
    },
    Import {
        path: PathBuf,
        approval: Approval,
    },
    SetApproval {
        source: Uuid,
        version: Uuid,
        approval: Approval,
    },
    Build,
    Search {
        query: String,
        profile: Profile,
        generation: u64,
    },
    Ask {
        session: Option<Uuid>,
        query: String,
        profile: Profile,
        generation: u64,
    },
    History {
        session: Uuid,
    },
    ListDrafts,
    OpenDraft {
        id: Uuid,
    },
    OpenDraftComments {
        id: Uuid,
    },
    RefreshDraftComments {
        id: Uuid,
    },
    CreateDraftComment {
        request: CommentCapture,
    },
    WriteDraftWithComments {
        request: DraftWriteWithComments,
    },
    SetCommentStatus {
        request: CommentStatusChange,
    },
    CreateDraft {
        op: Uuid,
        title: String,
        text: String,
    },
    SaveDraft {
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: String,
    },
    CheckpointDraft {
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: String,
    },
    ListDraftRevisions {
        id: Uuid,
    },
    OpenDraftRevision {
        draft: Uuid,
        revision: Uuid,
    },
    CompareDraftRevisions {
        draft: Uuid,
        before: Uuid,
        after: Uuid,
    },
    SaveCandidate {
        op: Uuid,
        draft: Uuid,
        parent: Uuid,
        turn: Uuid,
    },
}
impl Action {
    fn critical_note_mutation(&self) -> bool {
        matches!(
            self,
            Self::OpenNote { .. }
                | Self::SaveNoteBuffer { .. }
                | Self::SaveNote { .. }
                | Self::SaveNoteCopy { .. }
                | Self::ReloadNote { .. }
                | Self::RelinkNote { .. }
                | Self::AcceptNoteDiskState { .. }
                | Self::ApproveNoteSnapshot { .. }
                | Self::ReconcileNoteSave { .. }
                // These existing actions can route through managed-note snapshot mutations.
                | Self::Import { .. }
                | Self::SetApproval { .. }
        )
    }
    pub fn generation(&self) -> Option<u64> {
        match self {
            Self::Search { generation, .. } | Self::Ask { generation, .. } => Some(*generation),
            Self::CreateDraftComment { request } => Some(request.generation),
            Self::WriteDraftWithComments { request } => Some(request.generation),
            Self::SaveNoteBuffer { request }
            | Self::SaveNote { request }
            | Self::SaveNoteCopy { request, .. } => Some(request.generation),
            _ => None,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Outcome {
    NoteOpened {
        op: Uuid,
        view: NoteView,
    },
    NotesObserved {
        views: Vec<NoteView>,
    },
    NoteRecovery {
        id: Uuid,
        recovery: Option<NoteRecovery>,
    },
    NoteRecoveries {
        recoveries: Vec<NoteRecovery>,
    },
    NoteBufferSaved {
        submission: NoteSubmission,
        receipt: NoteBufferReceipt,
    },
    NoteSaved {
        submission: NoteSubmission,
        receipt: NoteReceipt,
    },
    NoteReconciled {
        op: Uuid,
        receipt: NoteReceipt,
    },
    NoteCompared {
        comparison: NoteComparison,
    },
    NoteReloaded {
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        view: NoteView,
    },
    NoteRelinked {
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        view: NoteView,
    },
    NoteCopySaved {
        submission: NoteSubmission,
        receipt: NoteReceipt,
        destination: NoteView,
    },
    NoteDiskAccepted {
        op: Uuid,
        save_op: Uuid,
        view: NoteView,
    },
    NoteApproved {
        receipt: NoteSearchReceipt,
    },
    Ready {
        sources: Vec<SourceDocument>,
        source_states: Vec<SourceStateSummary>,
        sessions: Vec<SessionSummary>,
        history: Vec<ChatTurn>,
        selected: Option<Uuid>,
        recovered_operations: usize,
    },
    Imported {
        result: ImportResult,
        sources: Vec<SourceDocument>,
        source_states: Vec<SourceStateSummary>,
    },
    ApprovalChanged {
        sources: Vec<SourceDocument>,
        source_states: Vec<SourceStateSummary>,
    },
    Indexed {
        generation: String,
    },
    Searched(SearchResult),
    Answered {
        turn: ChatTurn,
        sessions: Vec<SessionSummary>,
        history: Vec<ChatTurn>,
    },
    History {
        session: Uuid,
        turns: Vec<ChatTurn>,
    },
    DraftsListed {
        drafts: Vec<Draft>,
    },
    DraftOpened {
        id: Uuid,
        draft: Draft,
    },
    DraftCommentsOpened {
        id: Uuid,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    },
    DraftCommentsRefreshed {
        id: Uuid,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    },
    DraftCommentCreated {
        result: CommentCreated,
        snapshots: Vec<CommentAnchorSnapshot>,
    },
    DraftWrittenWithComments {
        op: Uuid,
        draft_id: Uuid,
        submitted_generation: u64,
        submitted_text: String,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    },
    CommentStatusChanged {
        result: CommentStatusChanged,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    },
    DraftCreated {
        draft: Draft,
    },
    DraftSaved {
        op: Uuid,
        id: Uuid,
        draft: Draft,
        submitted_generation: u64,
        submitted_text: String,
    },
    DraftCheckpointed {
        op: Uuid,
        id: Uuid,
        draft: Draft,
        submitted_generation: u64,
        submitted_text: String,
    },
    DraftRevisions {
        id: Uuid,
        revisions: Vec<DraftRevision>,
    },
    DraftRevisionOpened {
        draft: Uuid,
        revision: DraftRevision,
    },
    DraftCompared {
        draft: Uuid,
        before: Uuid,
        after: Uuid,
        diff: String,
    },
    CandidateSaved {
        draft: Uuid,
        revision: DraftRevision,
    },
}
#[derive(Debug, Clone)]
pub struct Terminal {
    pub id: u64,
    pub generation: Option<u64>,
    pub outcome: Result<Outcome, String>,
    pub note_failure: Option<NoteFailure>,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub active: Option<u64>,
    pub progress: String,
    pub streamed_text: String,
    pub update: u64,
}
#[derive(Default)]
struct Shared {
    active: Option<u64>,
    progress: String,
    streamed_text: String,
    terminals: VecDeque<Terminal>,
    update: u64,
}
impl Shared {
    fn touch(&mut self) {
        self.update = self.update.wrapping_add(1);
    }
}
struct Job {
    id: u64,
    action: Action,
}
#[derive(Default)]
struct PhaseState {
    closing: bool,
    critical_note_admitted: bool,
}

pub struct Worker {
    sender: Option<SyncSender<Job>>,
    shared: Arc<Mutex<Shared>>,
    busy: Arc<AtomicBool>,
    cancel_flag: Arc<AtomicBool>,
    closing: Arc<AtomicBool>,
    phase: Arc<Mutex<PhaseState>>,
    next_id: AtomicU64,
    thread: Option<JoinHandle<()>>,
    notices: NoteNoticeSink,
    pending_notices: Mutex<VecDeque<NoteFileNotice>>,
    #[cfg(test)]
    test_pause: Arc<Mutex<Option<tests::Pause>>>,
}
impl Worker {
    pub fn start(data_dir: PathBuf, config: Config) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let busy = Arc::new(AtomicBool::new(true));
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closing = Arc::new(AtomicBool::new(false));
        let phase = Arc::new(Mutex::new(PhaseState::default()));
        let notices = NoteNoticeSink::default();
        let notices_worker = notices.clone();
        #[cfg(test)]
        let test_pause: Arc<Mutex<Option<tests::Pause>>> = Arc::default();
        #[cfg(test)]
        let pause_worker = test_pause.clone();
        let (tx, rx) = mpsc::sync_channel::<Job>(1);
        let shared_worker = shared.clone();
        let busy_worker = busy.clone();
        let cancel_worker = cancel_flag.clone();
        let closing_worker = closing.clone();
        let phase_worker = phase.clone();
        let thread = std::thread::spawn(move || {
            let mut workspace = match Workspace::open(&data_dir, config) {
                Ok(workspace) => workspace,
                Err(error) => {
                    complete(
                        &shared_worker,
                        &busy_worker,
                        Terminal {
                            id: 0,
                            generation: None,
                            outcome: Err(format!("Workspace could not open: {error}")),
                            note_failure: None,
                        },
                    );
                    return;
                }
            };
            workspace.notes.notices = notices_worker;
            let ready = refresh(&mut workspace, None).map(
                |(sources, source_states, sessions, history, selected)| Outcome::Ready {
                    sources,
                    source_states,
                    sessions,
                    history,
                    selected,
                    recovered_operations: workspace.recovered_operations,
                },
            );
            complete(
                &shared_worker,
                &busy_worker,
                Terminal {
                    id: 0,
                    generation: None,
                    outcome: ready,
                    note_failure: None,
                },
            );
            while let Ok(job) = rx.recv() {
                #[cfg(test)]
                let pause = pause_worker.lock().unwrap().take();
                #[cfg(test)]
                if let Some(pause) = &pause
                    && matches!(pause.when, tests::PauseWhen::Queued)
                {
                    pause.entered.send(()).unwrap();
                    pause.release.recv().unwrap();
                }
                let critical = job.action.critical_note_mutation();
                if closing_worker.load(Ordering::Acquire) && !critical {
                    break;
                }
                #[cfg(test)]
                if let Some(pause) = &pause
                    && matches!(pause.when, tests::PauseWhen::Before)
                {
                    pause.entered.send(()).unwrap();
                    pause.release.recv().unwrap();
                }
                let generation = job.action.generation();
                let mut note_failure = None;
                let outcome = execute(
                    &mut workspace,
                    job.action,
                    &cancel_worker,
                    &shared_worker,
                    &mut note_failure,
                );
                #[cfg(test)]
                if let Some(pause) = &pause
                    && matches!(pause.when, tests::PauseWhen::After)
                {
                    pause.entered.send(()).unwrap();
                    pause.release.recv().unwrap();
                }
                let mut phase = phase_worker.lock().unwrap();
                complete(
                    &shared_worker,
                    &busy_worker,
                    Terminal {
                        id: job.id,
                        generation,
                        outcome,
                        note_failure,
                    },
                );
                phase.critical_note_admitted = false;
            }
        });
        Self {
            sender: Some(tx),
            shared,
            busy,
            cancel_flag,
            closing,
            phase,
            next_id: AtomicU64::new(1),
            thread: Some(thread),
            notices,
            pending_notices: Mutex::default(),
            #[cfg(test)]
            test_pause,
        }
    }
    pub fn submit(&self, action: Action) -> Result<u64, String> {
        // Serialize admission with shutdown before the job enters the one-item channel.
        let mut phase = self.phase.lock().unwrap();
        if phase.closing {
            return Err("workspace is closing".into());
        }
        if self.shared.lock().unwrap().terminals.len() >= 16 {
            return Err("read completed workspace events before starting more work".into());
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("another workspace action is running".into());
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.cancel_flag.store(false, Ordering::Release);
        {
            let mut state = self.shared.lock().unwrap();
            state.active = Some(id);
            state.progress.clear();
            state.streamed_text.clear();
            state.touch();
        }
        let Some(sender) = &self.sender else {
            self.busy.store(false, Ordering::Release);
            return Err("workspace worker unavailable".into());
        };
        phase.critical_note_admitted = action.critical_note_mutation();
        if sender.try_send(Job { id, action }).is_err() {
            phase.critical_note_admitted = false;
            self.busy.store(false, Ordering::Release);
            let mut state = self.shared.lock().unwrap();
            state.active = None;
            state.touch();
            return Err("workspace worker unavailable".into());
        }
        Ok(id)
    }
    pub fn cancel(&self) -> bool {
        if self.busy.load(Ordering::Acquire) {
            self.cancel_flag.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }
    pub fn snapshot(&self, since: u64) -> Option<Snapshot> {
        let state = self.shared.try_lock().ok()?;
        if state.update == since {
            return None;
        }
        Some(Snapshot {
            active: state.active,
            progress: state.progress.clone(),
            streamed_text: state.streamed_text.clone(),
            update: state.update,
        })
    }
    pub fn take_terminal(&self) -> Option<Terminal> {
        self.shared.try_lock().ok()?.terminals.pop_front()
    }
    pub fn critical_note_pending(&self) -> bool {
        self.phase.lock().unwrap().critical_note_admitted
    }
    pub fn take_note_notice(&self) -> Option<NoteFileNotice> {
        let mut pending = self.pending_notices.try_lock().ok()?;
        if pending.is_empty() {
            // Release this mutex before any adapter/Workspace drop: presenter Drop
            // joins callbacks that may themselves be waiting for the notice mutex.
            let drained = {
                let mut notices = self.notices.try_lock().ok()?;
                notices.drain()
            };
            pending.extend(drained);
        }
        pending.pop_front()
    }
    pub fn shutdown(&mut self) {
        let must_join = {
            let mut phase = self.phase.lock().unwrap();
            phase.closing = true;
            phase.critical_note_admitted
        };
        self.closing.store(true, Ordering::Release);
        self.cancel_flag.store(true, Ordering::Release);
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            if must_join {
                // Join admitted note mutations before this desktop process exits.
                let _ = thread.join();
            } else {
                // Native model loading is not yet interruptible. Keep Workspace owned
                // by the reaper until the local job returns, while close remains responsive.
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
                while !thread.is_finished() && std::time::Instant::now() < deadline {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                if thread.is_finished() {
                    let _ = thread.join();
                } else {
                    std::thread::spawn(move || {
                        let _ = thread.join();
                    });
                }
            }
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn complete(shared: &Mutex<Shared>, busy: &AtomicBool, terminal: Terminal) {
    let mut state = shared.lock().unwrap();
    state.active = None;
    state.terminals.push_back(terminal);
    state.touch();
    busy.store(false, Ordering::Release);
}
type Refreshed = (
    Vec<SourceDocument>,
    Vec<SourceStateSummary>,
    Vec<SessionSummary>,
    Vec<ChatTurn>,
    Option<Uuid>,
);
fn refresh(workspace: &mut Workspace, preferred: Option<Uuid>) -> Result<Refreshed, String> {
    let (sources, source_states) = workspace.source_projection()?;
    let sessions = workspace.sessions()?;
    let selected = preferred
        .filter(|id| sessions.iter().any(|s| s.id == *id))
        .or_else(|| sessions.last().map(|s| s.id));
    let history = selected
        .map(|id| workspace.history(id))
        .transpose()?
        .unwrap_or_default();
    Ok((sources, source_states, sessions, history, selected))
}
fn execute(
    workspace: &mut Workspace,
    action: Action,
    cancel: &AtomicBool,
    shared: &Mutex<Shared>,
    note_failure: &mut Option<NoteFailure>,
) -> Result<Outcome, String> {
    run_action(workspace, action, cancel, shared, note_failure).map_err(String::from)
}

fn note_result<T>(result: NoteResult<T>, detail: &mut Option<NoteFailure>) -> crate::Result<T> {
    result.map_err(|failure| {
        let error = failure.message.clone().into();
        *detail = Some(failure);
        error
    })
}

fn copy_observation_failure(mut failure: NoteFailure, receipt: &NoteReceipt) -> NoteFailure {
    failure.operation_id = Some(receipt.operation_id);
    failure.note_id = Some(receipt.note_id);
    if receipt.filesystem_outcome == crate::notes::FileOutcome::Applied {
        failure.phase = Some(crate::notes::SavePhase::Complete);
    }
    failure.filesystem_outcome = receipt.filesystem_outcome;
    failure.recovery_available = receipt.recovery_available;
    failure
}

fn run_action(
    workspace: &mut Workspace,
    action: Action,
    cancel: &AtomicBool,
    shared: &Mutex<Shared>,
    note_failure: &mut Option<NoteFailure>,
) -> crate::Result<Outcome> {
    match action {
        Action::OpenNote {
            op,
            vault,
            relative,
        } => note_result(workspace.open_note(op, &vault, &relative), note_failure)
            .map(|view| Outcome::NoteOpened { op, view }),
        Action::ObserveNotes { ids } => note_result(
            ids.into_iter().map(|id| workspace.note(id)).collect(),
            note_failure,
        )
        .map(|views| Outcome::NotesObserved { views }),
        Action::NoteRecovery { id } => note_result(workspace.note_recovery(id), note_failure)
            .map(|recovery| Outcome::NoteRecovery { id, recovery }),
        Action::ListNoteRecoveries => note_result(workspace.note_recoveries(), note_failure)
            .map(|recoveries| Outcome::NoteRecoveries { recoveries }),
        Action::SaveNoteBuffer { request } => {
            note_result(workspace.save_note_buffer(request.clone()), note_failure).map(|receipt| {
                Outcome::NoteBufferSaved {
                    submission: request,
                    receipt,
                }
            })
        }
        Action::SaveNote { request } => {
            note_result(workspace.save_note(request.clone()), note_failure).map(|receipt| {
                Outcome::NoteSaved {
                    submission: request,
                    receipt,
                }
            })
        }
        Action::ReconcileNoteSave { op } => {
            note_result(workspace.reconcile_note_save(op), note_failure)
                .map(|receipt| Outcome::NoteReconciled { op, receipt })
        }
        Action::CompareNote { id } => note_result(workspace.compare_note(id), note_failure)
            .map(|comparison| Outcome::NoteCompared { comparison }),
        Action::ReloadNote {
            op,
            id,
            expected,
            discard,
        } => note_result(
            workspace.reload_note(op, id, expected, discard),
            note_failure,
        )
        .map(|view| Outcome::NoteReloaded {
            op,
            id,
            expected,
            view,
        }),
        Action::RelinkNote {
            op,
            id,
            expected,
            relative,
            confirm_identity,
        } => note_result(
            workspace.relink_note(op, id, expected, &relative, confirm_identity),
            note_failure,
        )
        .map(|view| Outcome::NoteRelinked {
            op,
            id,
            expected,
            view,
        }),
        Action::SaveNoteCopy { request, relative } => {
            let receipt = note_result(
                workspace.save_note_copy(request.clone(), &relative),
                note_failure,
            )?;
            let destination = note_result(
                workspace
                    .note(receipt.note_id)
                    .map_err(|failure| copy_observation_failure(failure, &receipt)),
                note_failure,
            )?;
            Ok(Outcome::NoteCopySaved {
                submission: request,
                receipt,
                destination,
            })
        }
        Action::AcceptNoteDiskState {
            op,
            save_op,
            file_state,
        } => note_result(
            workspace.accept_note_disk_state(op, save_op, file_state),
            note_failure,
        )
        .map(|view| Outcome::NoteDiskAccepted { op, save_op, view }),
        Action::ApproveNoteSnapshot { op, id, file_state } => note_result(
            workspace.approve_note_snapshot(op, id, file_state),
            note_failure,
        )
        .map(|receipt| Outcome::NoteApproved { receipt }),
        Action::Refresh { session } => {
            let (sources, source_states, sessions, history, selected) =
                refresh(workspace, session)?;
            Ok(Outcome::Ready {
                sources,
                source_states,
                sessions,
                history,
                selected,
                recovered_operations: workspace.recovered_operations,
            })
        }
        Action::Import { path, approval } => {
            let result = workspace.import_file(cancel, Uuid::new_v4(), &path, approval)?;
            let (sources, source_states) = workspace.source_projection()?;
            Ok(Outcome::Imported {
                result,
                sources,
                source_states,
            })
        }
        Action::SetApproval {
            source,
            version,
            approval,
        } => {
            workspace.set_approval(cancel, Uuid::new_v4(), source, version, approval)?;
            let (sources, source_states) = workspace.source_projection()?;
            Ok(Outcome::ApprovalChanged {
                sources,
                source_states,
            })
        }
        Action::Build => {
            let generation = workspace.build_index(cancel, |message| {
                let mut state = shared.lock().unwrap();
                state.progress = message.into();
                state.touch();
            })?;
            Ok(Outcome::Indexed { generation })
        }
        Action::Search { query, profile, .. } => {
            workspace.search(&query, profile).map(Outcome::Searched)
        }
        Action::Ask {
            session,
            query,
            profile,
            ..
        } => {
            let turn_result = workspace.ask_guarded(
                Uuid::new_v4(),
                session,
                &query,
                profile,
                cancel,
                || Ok(()),
                |delta| {
                    let mut state = shared.lock().unwrap();
                    state.streamed_text.push_str(delta);
                    state.touch();
                },
            );
            let turn = turn_result?;
            let sessions = workspace.sessions()?;
            let history = workspace.history(turn.session_id)?;
            Ok(Outcome::Answered {
                turn,
                sessions,
                history,
            })
        }
        Action::History { session } => workspace
            .history(session)
            .map(|turns| Outcome::History { session, turns }),
        Action::ListDrafts => workspace
            .drafts()
            .map(|drafts| Outcome::DraftsListed { drafts }),
        Action::OpenDraft { id } => workspace
            .draft(id)?
            .ok_or("draft does not exist".into())
            .map(|draft| Outcome::DraftOpened { id, draft }),
        Action::OpenDraftComments { id } => Ok(Outcome::DraftCommentsOpened {
            id,
            saved: workspace.draft_comments(id)?,
            snapshots: workspace.comment_anchor_snapshots(id)?,
        }),
        Action::RefreshDraftComments { id } => Ok(Outcome::DraftCommentsRefreshed {
            id,
            saved: workspace.draft_comments(id)?,
            snapshots: workspace.comment_anchor_snapshots(id)?,
        }),
        Action::CreateDraftComment { request } => {
            let id = request.draft_id;
            let result = workspace.create_draft_comment(request)?;
            Ok(Outcome::DraftCommentCreated {
                result,
                snapshots: workspace.comment_anchor_snapshots(id)?,
            })
        }
        Action::WriteDraftWithComments { request } => {
            let op = request.op;
            let draft_id = request.draft_id;
            let submitted_generation = request.generation;
            let submitted_text = request.text.clone();
            let saved = workspace.write_draft_with_comments(request)?;
            Ok(Outcome::DraftWrittenWithComments {
                op,
                draft_id,
                submitted_generation,
                submitted_text,
                saved,
                snapshots: workspace.comment_anchor_snapshots(draft_id)?,
            })
        }
        Action::SetCommentStatus { request } => {
            let id = request.draft_id;
            let result = workspace.set_comment_status(request)?;
            Ok(Outcome::CommentStatusChanged {
                result,
                saved: workspace.draft_comments(id)?,
                snapshots: workspace.comment_anchor_snapshots(id)?,
            })
        }
        Action::CreateDraft { op, title, text } => workspace
            .create_draft(op, &title, &text)
            .map(|draft| Outcome::DraftCreated { draft }),
        Action::SaveDraft {
            op,
            id,
            expected,
            generation,
            text,
        } => workspace
            .save_draft(op, id, expected, generation, &text)
            .map(|draft| Outcome::DraftSaved {
                op,
                id,
                draft,
                submitted_generation: generation,
                submitted_text: text,
            }),
        Action::CheckpointDraft {
            op,
            id,
            expected,
            generation,
            text,
        } => workspace
            .checkpoint_draft(op, id, expected, generation, &text)
            .map(|draft| Outcome::DraftCheckpointed {
                op,
                id,
                draft,
                submitted_generation: generation,
                submitted_text: text,
            }),
        Action::ListDraftRevisions { id } => workspace
            .draft_revisions(id)
            .map(|revisions| Outcome::DraftRevisions { id, revisions }),
        Action::OpenDraftRevision { draft, revision } => {
            let found = workspace
                .draft_revision(revision)?
                .ok_or("revision does not exist")?;
            if found.draft_id != draft {
                return Err("revision does not belong to selected draft".into());
            }
            Ok(Outcome::DraftRevisionOpened {
                draft,
                revision: found,
            })
        }
        Action::CompareDraftRevisions {
            draft,
            before,
            after,
        } => workspace
            .compare_draft_revisions(draft, before, after)
            .map(|diff| Outcome::DraftCompared {
                draft,
                before,
                after,
                diff,
            }),
        Action::SaveCandidate {
            op,
            draft,
            parent,
            turn,
        } => workspace
            .candidate_from_turn(op, draft, parent, turn)
            .map(|revision| Outcome::CandidateSaved { draft, revision }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, Instant},
    };
    fn terminal(worker: &Worker) -> Terminal {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(event) = worker.take_terminal() {
                return event;
            }
            assert!(Instant::now() < deadline, "worker terminal timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn owned_worker_imports_builds_and_searches_without_provider() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.md");
        fs::write(
            &source,
            "# Atlas\nThe cobalt lantern is kept beside the northern map.\n",
        )
        .unwrap();
        let worker = Worker::start(dir.path().to_path_buf(), Config::default());
        assert!(matches!(
            terminal(&worker).outcome,
            Ok(Outcome::Ready { .. })
        ));
        worker
            .submit(Action::Import {
                path: source,
                approval: Approval::Approved,
            })
            .unwrap();
        assert!(matches!(
            terminal(&worker).outcome,
            Ok(Outcome::Imported { .. })
        ));
        worker.submit(Action::Build).unwrap();
        assert!(matches!(
            terminal(&worker).outcome,
            Ok(Outcome::Indexed { .. })
        ));
        worker
            .submit(Action::Search {
                query: "cobalt lantern".into(),
                profile: Profile::Keyword,
                generation: 7,
            })
            .unwrap();
        let result = terminal(&worker);
        assert_eq!(result.generation, Some(7));
        match result.outcome {
            Ok(Outcome::Searched(found)) => assert!(
                found
                    .evidence
                    .iter()
                    .any(|hit| hit.quote.contains("cobalt lantern"))
            ),
            other => panic!("unexpected search: {other:?}"),
        }
    }

    #[test]
    fn post_copy_observation_failure_retains_verified_save_context() {
        use crate::notes::{FileOutcome, NoteErrorCode, NoteStamp, SavePhase};
        let receipt = NoteReceipt {
            operation_id: Uuid::new_v4(),
            source_note_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            submitted_generation: 3,
            stamp: NoteStamp {
                file_state: Uuid::new_v4(),
                generation: 0,
            },
            filesystem_outcome: FileOutcome::Applied,
            recovery_available: true,
        };
        let failure = NoteFailure {
            code: NoteErrorCode::Io,
            message: "destination observation failed".into(),
            operation_id: None,
            note_id: None,
            phase: None,
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: false,
        };
        let contextual = copy_observation_failure(failure.clone(), &receipt);
        assert_eq!(contextual.code, NoteErrorCode::Io);
        assert_eq!(contextual.message, "destination observation failed");
        assert_eq!(contextual.operation_id, Some(receipt.operation_id));
        assert_eq!(contextual.note_id, Some(receipt.note_id));
        assert_eq!(contextual.phase, Some(SavePhase::Complete));
        assert_eq!(contextual.filesystem_outcome, FileOutcome::Applied);
        assert!(contextual.recovery_available);
        let unapplied = NoteReceipt {
            filesystem_outcome: FileOutcome::NotApplied,
            ..receipt
        };
        let contextual = copy_observation_failure(failure, &unapplied);
        assert_eq!(contextual.filesystem_outcome, FileOutcome::NotApplied);
        assert_eq!(contextual.phase, None);
    }

    #[cfg(target_os = "macos")]
    fn note_fixture() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        Worker,
        crate::notes::NoteView,
    ) {
        let data = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        fs::write(vault.path().join("plan.md"), "\u{feff}# café\r\n").unwrap();
        let worker = Worker::start(data.path().to_owned(), Config::default());
        terminal(&worker).outcome.unwrap();
        worker
            .submit(Action::OpenNote {
                op: Uuid::new_v4(),
                vault: vault.path().to_owned(),
                relative: "plan.md".into(),
            })
            .unwrap();
        let view = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteOpened { view, .. } => view,
            other => panic!("unexpected open: {other:?}"),
        };
        (data, vault, worker, view)
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn note_actions_preserve_generations_and_typed_failure_outcome() {
        use crate::notes::{FileOutcome, NoteErrorCode, NoteSubmission};
        let (_data, vault, worker, view) = note_fixture();
        let request = NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: view.id,
            expected: view.stamp,
            generation: 1,
            text: "\u{feff}# edited café\r\n".into(),
        };
        worker
            .submit(Action::SaveNoteBuffer {
                request: request.clone(),
            })
            .unwrap();
        let recovered = terminal(&worker);
        assert_eq!(recovered.generation, Some(1));
        let stamp = match recovered.outcome.unwrap() {
            Outcome::NoteBufferSaved {
                submission,
                receipt,
            } => {
                assert_eq!(submission, request);
                receipt.stamp
            }
            other => panic!("unexpected buffer: {other:?}"),
        };
        assert_eq!(
            fs::read(vault.path().join("plan.md")).unwrap(),
            "\u{feff}# café\r\n".as_bytes()
        );
        fs::write(vault.path().join("plan.md"), "external").unwrap();
        let save = NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: stamp,
            ..request
        };
        worker
            .submit(Action::SaveNote {
                request: save.clone(),
            })
            .unwrap();
        let failed = terminal(&worker);
        assert!(failed.outcome.is_err());
        let detail = failed.note_failure.unwrap();
        assert_eq!(detail.operation_id, Some(save.operation_id));
        assert_eq!(detail.note_id, Some(view.id));
        assert_eq!(detail.code, NoteErrorCode::Conflict);
        assert_eq!(detail.filesystem_outcome, FileOutcome::NotApplied);
        assert!(detail.recovery_available);
        assert_eq!(
            fs::read_to_string(vault.path().join("plan.md")).unwrap(),
            "external"
        );
    }

    #[derive(Clone, Copy)]
    pub(super) enum PauseWhen {
        Queued,
        Before,
        After,
    }
    pub(super) struct Pause {
        pub when: PauseWhen,
        pub entered: mpsc::Sender<()>,
        pub release: mpsc::Receiver<()>,
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn shutdown_joins_queued_and_inflight_critical_note_jobs() {
        use crate::notes::{FileOutcome, NoteSubmission};
        for (when, recover_only) in [
            (PauseWhen::Queued, false),
            (PauseWhen::Before, false),
            (PauseWhen::After, false),
            (PauseWhen::Queued, true),
            (PauseWhen::Before, true),
            (PauseWhen::After, true),
        ] {
            let (data, vault, mut worker, view) = note_fixture();
            let (entered_tx, entered_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            *worker.test_pause.lock().unwrap() = Some(Pause {
                when,
                entered: entered_tx,
                release: release_rx,
            });
            let request = NoteSubmission {
                operation_id: Uuid::new_v4(),
                note_id: view.id,
                expected: view.stamp,
                generation: 1,
                text: "saved by joined owner\r\n".into(),
            };
            worker
                .submit(if recover_only {
                    Action::SaveNoteBuffer {
                        request: request.clone(),
                    }
                } else {
                    Action::SaveNote {
                        request: request.clone(),
                    }
                })
                .unwrap();
            entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(worker.critical_note_pending());
            worker.cancel();
            let closing = worker.closing.clone();
            let (done_tx, done_rx) = mpsc::channel();
            let join = std::thread::spawn(move || {
                worker.shutdown();
                done_tx.send(worker).unwrap();
            });
            let deadline = Instant::now() + Duration::from_secs(5);
            while !closing.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline, "shutdown never entered closing");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                done_rx.recv_timeout(Duration::from_millis(400)).is_err(),
                "critical note owner must not use the 250 ms detached reaper"
            );
            release_tx.send(()).unwrap();
            let worker = done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            join.join().unwrap();
            assert!(!worker.critical_note_pending());
            match terminal(&worker).outcome.unwrap() {
                Outcome::NoteSaved {
                    submission,
                    receipt,
                } => {
                    assert_eq!(submission, request);
                    assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
                    assert!(receipt.recovery_available);
                }
                Outcome::NoteBufferSaved {
                    submission,
                    receipt,
                } => {
                    assert!(recover_only);
                    assert_eq!(submission, request);
                    assert_eq!(receipt.stamp.generation, 1);
                }
                other => panic!("unexpected save terminal: {other:?}"),
            }
            assert_eq!(
                fs::read_to_string(vault.path().join("plan.md")).unwrap(),
                if recover_only {
                    "\u{feff}# café\r\n"
                } else {
                    &request.text
                }
            );
            let workspace = Workspace::open(data.path(), Config::default()).unwrap();
            assert_eq!(
                workspace.note_recovery(view.id).unwrap().unwrap().working,
                request.text
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn notices_drain_independently_while_a_job_is_active() {
        use crate::notes::{NoteFileNotice, NoteNoticeKind, NoteSubmission};
        let (_data, _vault, worker, view) = note_fixture();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        *worker.test_pause.lock().unwrap() = Some(Pause {
            when: PauseWhen::Before,
            entered: entered_tx,
            release: release_rx,
        });
        worker
            .submit(Action::SaveNoteBuffer {
                request: NoteSubmission {
                    operation_id: Uuid::new_v4(),
                    note_id: view.id,
                    expected: view.stamp,
                    generation: 1,
                    text: "recover me".into(),
                },
            })
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let notice = NoteFileNotice {
            vault_id: view.vault_id,
            relative_path: Some("plan.md".into()),
            kind: NoteNoticeKind::Changed,
        };
        {
            let mut queue = worker.notices.lock().unwrap();
            queue.drain();
            queue.push(notice.clone());
            queue.push(notice.clone());
        }
        assert_eq!(worker.take_note_notice(), Some(notice));
        assert_eq!(worker.take_note_notice(), None);
        assert!(worker.take_terminal().is_none());
        assert!(
            worker
                .submit(Action::ObserveNotes { ids: vec![view.id] })
                .is_err()
        );
        release_tx.send(()).unwrap();
        terminal(&worker).outcome.unwrap();
        worker
            .submit(Action::ObserveNotes { ids: vec![view.id] })
            .unwrap();
        assert!(matches!(
            terminal(&worker).outcome,
            Ok(Outcome::NotesObserved { .. })
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn copy_compare_accept_relink_reload_and_approval_use_owned_workspace() {
        use crate::notes::{FileOutcome, NoteAvailability, NoteSubmission};
        let (data, vault, mut worker, view) = note_fixture();
        worker
            .submit(Action::ApproveNoteSnapshot {
                op: Uuid::new_v4(),
                id: view.id,
                file_state: view.current_file_state.unwrap(),
            })
            .unwrap();
        assert!(matches!(
            terminal(&worker).outcome,
            Ok(Outcome::NoteApproved { .. })
        ));
        let request = NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: view.id,
            expected: view.stamp,
            generation: 1,
            text: "protected local\r\n".into(),
        };
        worker
            .submit(Action::SaveNoteCopy {
                request: request.clone(),
                relative: "copy.md".into(),
            })
            .unwrap();
        match terminal(&worker).outcome.unwrap() {
            Outcome::NoteCopySaved {
                submission,
                receipt,
                destination,
            } => {
                assert_eq!(submission, request);
                assert_eq!(receipt.source_note_id, view.id);
                assert_ne!(destination.id, view.id);
                assert_eq!(destination.id, receipt.note_id);
                assert_eq!(destination.stamp, receipt.stamp);
                assert_eq!(destination.search_approval, Approval::Draft);
                assert_eq!(destination.saved.as_deref(), Some("protected local\r\n"));
            }
            other => panic!("unexpected copy: {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(vault.path().join("copy.md")).unwrap(),
            "protected local\r\n"
        );
        assert_eq!(
            fs::read_to_string(vault.path().join("plan.md")).unwrap(),
            "\u{feff}# café\r\n"
        );
        worker
            .submit(Action::ObserveNotes { ids: vec![view.id] })
            .unwrap();
        let original = match terminal(&worker).outcome.unwrap() {
            Outcome::NotesObserved { views } => views.into_iter().next().unwrap(),
            other => panic!("unexpected observation: {other:?}"),
        };
        assert_eq!(original.buffer, "protected local\r\n");
        let save = NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: original.stamp,
            ..request
        };
        worker.shutdown();
        {
            use brn_store::notes::{DestinationPrecondition, NoteWriteKind};
            let mut workspace = Workspace::open(data.path(), Config::default()).unwrap();
            let record = workspace.store.note_record(view.id).unwrap();
            let baseline = workspace.note_recovery(view.id).unwrap().unwrap().baseline;
            workspace
                .store
                .begin_note_save(
                    &save,
                    &record.relative_path,
                    NoteWriteKind::Replace,
                    &DestinationPrecondition::Existing {
                        fingerprint: record.baseline,
                        baseline_text: baseline,
                    },
                )
                .unwrap();
        }
        fs::write(vault.path().join("plan.md"), "external").unwrap();
        worker = Worker::start(data.path().to_owned(), Config::default());
        terminal(&worker).outcome.unwrap();
        worker
            .submit(Action::ReconcileNoteSave {
                op: save.operation_id,
            })
            .unwrap();
        let failure = terminal(&worker).note_failure.unwrap();
        // Restart has no live exchange-progress proof, unlike the preflight refusal test.
        assert_eq!(failure.filesystem_outcome, FileOutcome::Unknown);
        assert_eq!(failure.phase, Some(crate::notes::SavePhase::Intent));
        worker.submit(Action::CompareNote { id: view.id }).unwrap();
        let token = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteCompared { comparison } => {
                assert_eq!(comparison.baseline, "\u{feff}# café\r\n");
                assert_eq!(comparison.working, "protected local\r\n");
                assert_eq!(comparison.observed.as_deref(), Some("external"));
                comparison.observed_file_state.unwrap()
            }
            other => panic!("unexpected comparison: {other:?}"),
        };
        worker
            .submit(Action::AcceptNoteDiskState {
                op: Uuid::new_v4(),
                save_op: save.operation_id,
                file_state: token,
            })
            .unwrap();
        let accepted = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteDiskAccepted { view, .. } => view,
            other => panic!("unexpected accept: {other:?}"),
        };
        assert_eq!(accepted.buffer, "protected local\r\n");
        worker
            .submit(Action::ReconcileNoteSave {
                op: save.operation_id,
            })
            .unwrap();
        let replay = terminal(&worker);
        assert!(replay.outcome.is_err());
        assert_eq!(replay.note_failure, Some(failure));
        fs::rename(vault.path().join("plan.md"), vault.path().join("moved.md")).unwrap();
        worker
            .submit(Action::RelinkNote {
                op: Uuid::new_v4(),
                id: view.id,
                expected: accepted.stamp,
                relative: "moved.md".into(),
                confirm_identity: true,
            })
            .unwrap();
        let relinked = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteRelinked { view, .. } => view,
            other => panic!("unexpected relink: {other:?}"),
        };
        assert_eq!(relinked.buffer, "protected local\r\n");
        assert_eq!(relinked.availability, NoteAvailability::Available);
        worker
            .submit(Action::ReloadNote {
                op: Uuid::new_v4(),
                id: view.id,
                expected: relinked.stamp,
                discard: true,
            })
            .unwrap();
        let reloaded = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteReloaded { view, .. } => view,
            other => panic!("unexpected reload: {other:?}"),
        };
        assert_eq!(reloaded.buffer, "external");
        worker.submit(Action::ListNoteRecoveries).unwrap();
        assert!(
            matches!(terminal(&worker).outcome, Ok(Outcome::NoteRecoveries { recoveries }) if recoveries.len() == 2)
        );
        worker.submit(Action::NoteRecovery { id: view.id }).unwrap();
        assert!(
            matches!(terminal(&worker).outcome, Ok(Outcome::NoteRecovery { recovery: Some(recovery), .. }) if recovery.working == "external")
        );
    }
}
