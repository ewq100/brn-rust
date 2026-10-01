//! Single owned workflow worker for the native UI. No store or provider call runs on the GUI thread.
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
    pub fn generation(&self) -> Option<u64> {
        match self {
            Self::Search { generation, .. } | Self::Ask { generation, .. } => Some(*generation),
            Self::CreateDraftComment { request } => Some(request.generation),
            Self::WriteDraftWithComments { request } => Some(request.generation),
            _ => None,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Outcome {
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
    provider_active: bool,
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
}
impl Worker {
    pub fn start(data_dir: PathBuf, config: Config) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let busy = Arc::new(AtomicBool::new(true));
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let closing = Arc::new(AtomicBool::new(false));
        let phase = Arc::new(Mutex::new(PhaseState::default()));
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
                        },
                    );
                    return;
                }
            };
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
                },
            );
            while let Ok(job) = rx.recv() {
                if closing_worker.load(Ordering::Acquire) {
                    break;
                }
                let generation = job.action.generation();
                let outcome = execute(
                    &mut workspace,
                    job.action,
                    &cancel_worker,
                    &shared_worker,
                    &phase_worker,
                );
                complete(
                    &shared_worker,
                    &busy_worker,
                    Terminal {
                        id: job.id,
                        generation,
                        outcome,
                    },
                );
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
        }
    }
    pub fn submit(&self, action: Action) -> Result<u64, String> {
        if self.closing.load(Ordering::Acquire) {
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
        if sender.try_send(Job { id, action }).is_err() {
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
    pub fn shutdown(&mut self) {
        let provider_active = {
            let mut phase = self.phase.lock().unwrap();
            phase.closing = true;
            phase.provider_active
        };
        self.closing.store(true, Ordering::Release);
        self.cancel_flag.store(true, Ordering::Release);
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            if provider_active {
                // The provider owns a child process and bounded cancellation/reap path.
                // Wait for that owner before this desktop process exits.
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
    phase: &Mutex<PhaseState>,
) -> Result<Outcome, String> {
    run_action(workspace, action, cancel, shared, phase).map_err(String::from)
}

fn run_action(
    workspace: &mut Workspace,
    action: Action,
    cancel: &AtomicBool,
    shared: &Mutex<Shared>,
    phase: &Mutex<PhaseState>,
) -> crate::Result<Outcome> {
    match action {
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
                || {
                    let mut state = phase.lock().unwrap();
                    if state.closing {
                        return Err("workspace is closing".into());
                    }
                    state.provider_active = true;
                    Ok(())
                },
                |delta| {
                    let mut state = shared.lock().unwrap();
                    state.streamed_text.push_str(delta);
                    state.touch();
                },
            );
            phase.lock().unwrap().provider_active = false;
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
}
