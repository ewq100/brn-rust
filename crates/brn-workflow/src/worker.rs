//! Single owned workflow worker for the native UI. No store or provider call runs on the GUI thread.
use crate::{Config, SearchResult, SessionSummary, Workspace};
pub use brn_retrieval::{Evidence, Profile};
pub use brn_store::{Approval, ChatTurn, ImportResult, SourceDocument};
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
}
impl Action {
    pub fn generation(&self) -> Option<u64> {
        match self {
            Self::Search { generation, .. } | Self::Ask { generation, .. } => Some(*generation),
            _ => None,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Outcome {
    Ready {
        sources: Vec<SourceDocument>,
        sessions: Vec<SessionSummary>,
        history: Vec<ChatTurn>,
        selected: Option<Uuid>,
        recovered_operations: usize,
    },
    Imported {
        result: ImportResult,
        sources: Vec<SourceDocument>,
    },
    ApprovalChanged {
        sources: Vec<SourceDocument>,
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
            let ready = refresh(&workspace, None).map(|(sources, sessions, history, selected)| {
                Outcome::Ready {
                    sources,
                    sessions,
                    history,
                    selected,
                    recovered_operations: workspace.recovered_operations,
                }
            });
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
    Vec<SessionSummary>,
    Vec<ChatTurn>,
    Option<Uuid>,
);
fn refresh(workspace: &Workspace, preferred: Option<Uuid>) -> Result<Refreshed, String> {
    let sources = workspace.sources()?;
    let sessions = workspace.sessions()?;
    let selected = preferred
        .filter(|id| sessions.iter().any(|s| s.id == *id))
        .or_else(|| sessions.last().map(|s| s.id));
    let history = selected
        .map(|id| workspace.history(id))
        .transpose()?
        .unwrap_or_default();
    Ok((sources, sessions, history, selected))
}
fn execute(
    workspace: &mut Workspace,
    action: Action,
    cancel: &AtomicBool,
    shared: &Mutex<Shared>,
    phase: &Mutex<PhaseState>,
) -> Result<Outcome, String> {
    match action {
        Action::Refresh { session } => {
            let (sources, sessions, history, selected) = refresh(workspace, session)?;
            Ok(Outcome::Ready {
                sources,
                sessions,
                history,
                selected,
                recovered_operations: workspace.recovered_operations,
            })
        }
        Action::Import { path, approval } => {
            let result = workspace.import_file(Uuid::new_v4(), &path, approval)?;
            Ok(Outcome::Imported {
                result,
                sources: workspace.sources()?,
            })
        }
        Action::SetApproval {
            source,
            version,
            approval,
        } => {
            workspace.set_approval(Uuid::new_v4(), source, version, approval)?;
            Ok(Outcome::ApprovalChanged {
                sources: workspace.sources()?,
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
