use crate::drafts::DraftEditor;
use crate::notes::{CloseRoute, NoteEditor, NoteObservations, NoteScheduler};
use brn_workflow::notes::{NoteComparison, NoteRecovery, NoteView};
use brn_workflow::worker::{
    Action, Approval, ChatTurn, CommentStatusChange, Draft, DraftRevision, DraftWriteWithComments,
    Evidence, Outcome, Profile, SourceDocument, Worker,
};
use brn_workflow::{AnchorState, CommentStatus, Config, SearchResult, SessionSummary};
use gpui_kit::{
    AppContext, Bounds, Context, Entity, KeyBinding, Menu, MenuItem, PathPromptOptions,
    ScrollAnchor, ScrollHandle, Subscription, Task, Window, WindowBounds, WindowOptions,
    base::Disableable,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState},
        scroll::ScrollableElement,
    },
    div, point,
    prelude::*,
    px, size,
};
gpui_kit::actions!(brn, [Quit, SaveNote]);
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Workspace,
    Activity,
    Drafts,
    Notes,
    Settings,
}

enum NoteControl {
    Open {
        op: Uuid,
        vault: PathBuf,
        relative: PathBuf,
    },
    Select(Uuid),
    Save,
    Compare,
    Reload,
    Relink(PathBuf),
    Copy(PathBuf),
    Accept {
        save_op: Uuid,
        file_state: Uuid,
    },
    Approve {
        file_state: Uuid,
    },
}
struct PendingNoteOpen {
    job: u64,
    op: Option<Uuid>,
    id: Option<Uuid>,
}

#[derive(Debug, Clone)]
enum Phase {
    Opening,
    Idle,
    Running { label: String, since: Instant },
    Cancelling { label: String, since: Instant },
    Failed(String),
}
impl Phase {
    fn can_submit(&self) -> bool {
        matches!(self, Self::Idle)
    }
    fn terminal(&mut self, initial: bool, error: Option<&str>) {
        *self = if initial {
            match error {
                Some(error) => Self::Failed(error.to_string()),
                None => Self::Idle,
            }
        } else {
            Self::Idle
        };
    }
    fn cancel(&mut self) {
        if let Self::Running { label, since } = self {
            *self = Self::Cancelling {
                label: label.clone(),
                since: *since,
            };
        }
    }
    fn shows_live_answer(&self) -> bool {
        matches!(self, Self::Running { label, .. } | Self::Cancelling { label, .. } if label == "Answer")
    }
}

fn compact_title(title: &str) -> String {
    let single_line = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = single_line.chars();
    let prefix: String = chars.by_ref().take(36).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn passage_button_label(index: usize, hit: &Evidence, sources: &[SourceDocument]) -> String {
    let title = sources
        .iter()
        .find(|source| source.source_id.to_string() == hit.source_id)
        .map(|source| compact_title(&source.title))
        .unwrap_or_else(|| "Source".into());
    format!(
        "[{}] {} · bytes {}..{}",
        index + 1,
        title,
        hit.start_byte,
        hit.end_byte
    )
}

fn saved_evidence_button_label(index: usize, hit: &Evidence) -> String {
    let revision: String = hit.version_id.chars().take(8).collect();
    format!(
        "[{}] Saved evidence · rev {} · {}..{}",
        index + 1,
        revision,
        hit.start_byte,
        hit.end_byte
    )
}
fn spaced_identifier(value: &str) -> String {
    value
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 32 == 0 {
                vec!['\u{200b}', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
struct Desktop {
    worker: Worker,
    query: Entity<EditorState>,
    draft_title: Entity<InputState>,
    draft_editor: Entity<EditorState>,
    comment_input: Entity<EditorState>,
    draft_state: Option<DraftEditor>,
    note_editor: Entity<EditorState>,
    note_path: Entity<InputState>,
    note_state: Option<NoteEditor>,
    note_schedule: NoteScheduler,
    note_views: Vec<NoteView>,
    note_recoveries: Vec<NoteRecovery>,
    note_observations: NoteObservations,
    note_control: Option<NoteControl>,
    pending_note_job: Option<(u64, Uuid)>,
    pending_note_open: Option<PendingNoteOpen>,
    note_comparison: Option<NoteComparison>,
    note_copy: Option<NoteView>,
    original_note_operation: Option<Uuid>,
    note_sources_changed: bool,
    last_note_failure: Option<brn_workflow::notes::NoteFailure>,
    vault_path: Option<PathBuf>,
    drafts: Vec<Draft>,
    revisions: Vec<DraftRevision>,
    review: Option<DraftRevision>,
    comment_original: Option<DraftRevision>,
    compare_before: Option<Uuid>,
    compare_after: Option<Uuid>,
    diff: Option<String>,
    pending_draft_job: Option<(u64, Uuid)>,
    pending_status_job: Option<(u64, Uuid, Uuid)>,
    draft_scroll: ScrollHandle,
    draft_editor_anchor: ScrollAnchor,
    import_path: Option<PathBuf>,
    choosing_file: bool,
    phase: Phase,
    page: Page,
    path: PathBuf,
    config: Config,
    message: String,
    generation: u64,
    profile: Profile,
    selected_session: Option<Uuid>,
    sources: Vec<SourceDocument>,
    source_states: Vec<brn_workflow::SourceStateSummary>,
    sessions: Vec<SessionSummary>,
    history: Vec<ChatTurn>,
    selected_turn: Option<usize>,
    search: Option<SearchResult>,
    selected_evidence: Option<Evidence>,
    selected_saved_evidence: Option<Evidence>,
    active: Option<u64>,
    active_generation: Option<u64>,
    streamed_text: String,
    progress: String,
    last_elapsed: u64,
    last_update: u64,
    _subscriptions: Vec<Subscription>,
    _poll_task: Task<()>,
}
impl Desktop {
    fn new(path: PathBuf, config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let draft_title = cx.new(|cx| InputState::new(window, cx).placeholder("New draft title"));
        let draft_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value("")
        });
        let comment_input = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let note_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value("")
        });
        let note_path =
            cx.new(|cx| InputState::new(window, cx).placeholder("Vault-relative .md destination"));
        let note_subscription = cx.subscribe_in(
            &note_editor,
            window,
            |this, editor, event: &InputEvent, window, cx| {
                match event {
                    InputEvent::Change => {
                        if let Some(state) = &mut this.note_state {
                            let text = editor.read(cx).value().to_string();
                            if state.text() != text {
                                match state.edit_input(text) {
                                    Ok(()) => this.note_schedule.edited(Instant::now()),
                                    Err(error) => {
                                        this.message = error.message;
                                        let retained = state.input_text();
                                        editor.update(cx, |editor, cx| {
                                            editor.set_value(retained, window, cx)
                                        });
                                    }
                                }
                            }
                        }
                    }
                    InputEvent::Focus => {
                        if let Some(state) = &this.note_state {
                            this.note_observations.insert(state.id());
                        }
                    }
                    _ => {}
                }
                cx.notify();
            },
        );
        let query_subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.generation = this.generation.wrapping_add(1);
                this.search = None;
                this.selected_evidence = None;
                this.streamed_text.clear();
                cx.notify();
            }
        });
        let draft_subscription =
            cx.subscribe(&draft_editor, |this, editor, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(state) = &mut this.draft_state {
                        let text = editor.read(cx).value().to_string();
                        if state.text() != text {
                            state.edit(text);
                        }
                    }
                    cx.notify();
                }
            });
        let comment_subscription =
            cx.subscribe(&comment_input, |this, editor, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(state) = &mut this.draft_state {
                        state.set_composer(editor.read(cx).value().to_string());
                    }
                    cx.notify();
                }
            });
        let draft_scroll = ScrollHandle::new();
        let draft_editor_anchor = ScrollAnchor::for_handle(draft_scroll.clone());
        let quit_subscription = cx.on_app_quit(|this, _| {
            this.worker.shutdown();
            async {}
        });
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.note_observations
                    .extend(this.note_views.iter().map(|view| view.id));
                cx.notify();
            }
        });
        let poll_task = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(60))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| this.poll(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            worker: Worker::start(path.clone(), config.clone()),
            query,
            draft_title,
            draft_editor,
            comment_input,
            draft_state: None,
            note_editor,
            note_path,
            note_state: None,
            note_schedule: NoteScheduler::default(),
            note_views: vec![],
            note_recoveries: vec![],
            note_observations: NoteObservations::default(),
            note_control: None,
            pending_note_job: None,
            pending_note_open: None,
            note_comparison: None,
            note_copy: None,
            original_note_operation: None,
            note_sources_changed: false,
            last_note_failure: None,
            vault_path: None,
            drafts: Vec::new(),
            revisions: Vec::new(),
            review: None,
            comment_original: None,
            compare_before: None,
            compare_after: None,
            diff: None,
            pending_draft_job: None,
            pending_status_job: None,
            draft_scroll,
            draft_editor_anchor,
            import_path: None,
            choosing_file: false,
            phase: Phase::Opening,
            page: Page::Workspace,
            path,
            config,
            message: "Opening workspace…".into(),
            generation: 0,
            profile: Profile::Keyword,
            selected_session: None,
            sources: Vec::new(),
            source_states: Vec::new(),
            sessions: Vec::new(),
            history: Vec::new(),
            selected_turn: None,
            search: None,
            selected_evidence: None,
            selected_saved_evidence: None,
            active: None,
            active_generation: None,
            streamed_text: String::new(),
            progress: String::new(),
            last_elapsed: 0,
            last_update: 0,
            _subscriptions: vec![
                query_subscription,
                draft_subscription,
                comment_subscription,
                quit_subscription,
                note_subscription,
                activation_subscription,
            ],
            _poll_task: poll_task,
        }
    }
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Some(notice) = self.worker.take_note_notice() {
            self.note_observations.notice(&notice, &self.note_views);
            changed = true;
        }
        if let Some(snapshot) = self.worker.snapshot(self.last_update) {
            self.last_update = snapshot.update;
            self.active = snapshot.active;
            self.progress = snapshot.progress;
            if self.active_generation == Some(self.generation) {
                self.streamed_text = snapshot.streamed_text;
            }
            changed = true;
        }
        while let Some(terminal) = self.worker.take_terminal() {
            changed = true;
            let initial = terminal.id == 0;
            self.phase
                .terminal(initial, terminal.outcome.as_ref().err().map(String::as_str));
            match terminal.outcome {
                Err(error) => {
                    if self
                        .pending_note_job
                        .is_some_and(|(id, _)| id == terminal.id)
                    {
                        let (_, op) = self.pending_note_job.take().unwrap();
                        if let Some(state) = &mut self.note_state {
                            if let Some(detail) = terminal.note_failure.clone()
                                && state.pending()
                                && let Err(error) = state.fail(op, detail)
                            {
                                self.message = error.message;
                            }
                            self.note_observations.insert(state.id());
                        }
                        self.note_schedule.recovery_failed();
                    }
                    if self
                        .pending_note_open
                        .as_ref()
                        .is_some_and(|pending| pending.job == terminal.id)
                    {
                        self.pending_note_open = None;
                    }
                    if let Some(detail) = terminal.note_failure {
                        self.message = format!(
                            "{:?}: {} (phase {:?}, effect {:?}, recovery confirmed: {})",
                            detail.code,
                            detail.message,
                            detail.phase,
                            detail.filesystem_outcome,
                            detail.recovery_available
                        );
                        self.last_note_failure = Some(detail);
                    } else {
                        self.message = error.clone();
                    }
                    if self
                        .pending_draft_job
                        .is_some_and(|(id, _)| id == terminal.id)
                        && let Some((_, op)) = self.pending_draft_job.take()
                        && let Some(state) = &mut self.draft_state
                    {
                        state.fail(op);
                    }
                    if self
                        .pending_status_job
                        .is_some_and(|(id, _, _)| id == terminal.id)
                    {
                        self.pending_status_job = None;
                    }
                    self.streamed_text.clear();
                }
                Ok(Outcome::Ready {
                    sources,
                    source_states,
                    sessions,
                    history,
                    selected,
                    recovered_operations,
                }) => {
                    self.sources = sources;
                    self.source_states = source_states;
                    self.sessions = sessions;
                    self.history = history;
                    self.search = None;
                    self.selected_evidence = None;
                    self.generation = self.generation.wrapping_add(1);
                    self.selected_session = selected;
                    self.selected_turn = self.history.len().checked_sub(1);
                    self.selected_saved_evidence = None;
                    self.message = if recovered_operations > 0 {
                        format!(
                            "Workspace ready. {recovered_operations} interrupted operation(s) recovered."
                        )
                    } else {
                        "Workspace ready.".into()
                    };
                    self.submit(Action::ListDrafts, "Draft list", cx);
                }
                Ok(Outcome::Imported {
                    result,
                    sources,
                    source_states,
                }) => {
                    self.sources = sources;
                    self.source_states = source_states;
                    self.search = None;
                    self.selected_evidence = None;
                    self.generation = self.generation.wrapping_add(1);
                    self.message = if result.changed {
                        "Source imported and approved. Build the index to search it.".into()
                    } else {
                        "Source was already current; approval updated.".into()
                    };
                }
                Ok(Outcome::ApprovalChanged {
                    sources,
                    source_states,
                }) => {
                    self.sources = sources;
                    self.source_states = source_states;
                    self.search = None;
                    self.selected_evidence = None;
                    self.generation = self.generation.wrapping_add(1);
                    self.message =
                        "Source approval changed. Rebuild the index before searching.".into();
                }
                Ok(Outcome::Indexed { generation }) => {
                    self.search = None;
                    self.selected_evidence = None;
                    self.generation = self.generation.wrapping_add(1);
                    self.message = format!("Index ready: {generation}")
                }
                Ok(Outcome::Searched(result)) => {
                    if terminal.generation == Some(self.generation) {
                        self.message = format!("{} supporting passages.", result.evidence.len());
                        self.search = Some(result);
                        self.selected_evidence = None;
                    } else {
                        self.message =
                            "Search finished after the question changed; result hidden.".into();
                    }
                }
                Ok(Outcome::Answered {
                    turn,
                    sessions,
                    history,
                }) => {
                    self.sessions = sessions;
                    if terminal.generation == Some(self.generation) {
                        self.selected_session = Some(turn.session_id);
                        self.history = history;
                        self.selected_turn = self.history.len().checked_sub(1);
                        self.selected_saved_evidence = None;
                        self.streamed_text.clear();
                        self.message = format!("Answer saved ({:?}).", turn.status);
                    } else {
                        self.message = "Answer saved to its session. Refresh or select that session to review it.".into();
                    }
                }
                Ok(Outcome::History { session, turns }) => {
                    if self.selected_session == Some(session) {
                        self.history = turns;
                        self.selected_turn = self.history.len().checked_sub(1);
                        self.selected_saved_evidence = None;
                    }
                }
                Ok(Outcome::DraftsListed { drafts }) => {
                    self.drafts = drafts;
                    self.message = "Draft list ready.".into();
                    self.submit(Action::ListNoteRecoveries, "Note recovery list", cx);
                }
                Ok(Outcome::DraftCreated { draft }) | Ok(Outcome::DraftOpened { draft, .. }) => {
                    let id = draft.id;
                    let may_open = if let Some(state) = &mut self.draft_state {
                        state.replace(draft.clone())
                    } else {
                        self.draft_state = Some(DraftEditor::new(draft.clone()));
                        true
                    };
                    if let Some(old) = self.drafts.iter_mut().find(|d| d.id == id) {
                        *old = draft.clone();
                    } else {
                        self.drafts.push(draft.clone());
                    }
                    if may_open {
                        self.draft_editor.update(cx, |editor, cx| {
                            editor.set_value(draft.text.clone(), window, cx)
                        });
                        self.comment_input
                            .update(cx, |editor, cx| editor.set_value("", window, cx));
                        self.revisions.clear();
                        self.review = None;
                        self.comment_original = None;
                        self.diff = None;
                        self.compare_before = None;
                        self.compare_after = None;
                        self.message = "Draft opened. Working copy saved.".into();
                        self.submit(Action::ListDraftRevisions { id }, "Revision list", cx);
                    } else {
                        self.message = "Draft opened in storage, but newer edits remain in the current editor. Save or discard them before switching.".into();
                    }
                }
                Ok(Outcome::DraftCommentsOpened {
                    id,
                    saved,
                    snapshots,
                }) => {
                    if saved.draft.id != id {
                        continue;
                    }
                    let may_open = if let Some(state) = &mut self.draft_state {
                        state.replace_comments(saved.clone(), snapshots)
                    } else {
                        self.draft_state =
                            Some(DraftEditor::from_comments(saved.clone(), snapshots));
                        true
                    };
                    if may_open {
                        self.draft_editor.update(cx, |editor, cx| {
                            editor.set_value(saved.draft.text.clone(), window, cx)
                        });
                        self.comment_input
                            .update(cx, |editor, cx| editor.set_value("", window, cx));
                        self.revisions.clear();
                        self.review = None;
                        self.comment_original = None;
                        self.diff = None;
                        self.compare_before = None;
                        self.compare_after = None;
                        if let Some(old) = self.drafts.iter_mut().find(|d| d.id == id) {
                            *old = saved.draft.clone();
                        } else {
                            self.drafts.push(saved.draft.clone());
                        }
                        self.message = "Draft and comments opened.".into();
                        self.submit(Action::ListDraftRevisions { id }, "Revision list", cx);
                    } else {
                        self.message = "Current draft has unsaved text or comment. Save or discard before switching.".into();
                    }
                }
                Ok(Outcome::DraftCommentsRefreshed {
                    id,
                    saved,
                    snapshots,
                }) => {
                    if self
                        .draft_state
                        .as_mut()
                        .is_some_and(|s| s.id() == id && s.refresh_comments(saved, snapshots))
                    {
                        self.message = "Comment list refreshed.".into();
                    }
                }
                Ok(Outcome::DraftCommentCreated { result, snapshots }) => {
                    let op = result.op;
                    let id = result.saved.draft.id;
                    let matched = self.pending_draft_job == Some((terminal.id, op));
                    if matched {
                        self.pending_draft_job = None;
                    }
                    if matched
                        && self.draft_state.as_mut().is_some_and(|s| {
                            s.id() == id
                                && s.acknowledge_comments(op, result.saved.clone(), snapshots)
                        })
                    {
                        if let Some(old) = self.drafts.iter_mut().find(|d| d.id == id) {
                            *old = result.saved.draft;
                        }
                        if self
                            .draft_state
                            .as_ref()
                            .is_some_and(|s| s.composer().is_empty())
                        {
                            self.comment_input
                                .update(cx, |editor, cx| editor.set_value("", window, cx));
                        }
                        self.message = if self.draft_state.as_ref().is_some_and(DraftEditor::dirty)
                        {
                            "Comment saved; newer draft edits remain unsaved.".into()
                        } else {
                            "Comment saved with a checkpoint.".into()
                        };
                        self.submit(Action::ListDraftRevisions { id }, "Revision list", cx);
                    }
                }
                Ok(Outcome::DraftWrittenWithComments {
                    op,
                    draft_id,
                    submitted_generation,
                    submitted_text,
                    saved,
                    snapshots,
                }) => {
                    let matched = self.pending_draft_job == Some((terminal.id, op));
                    if matched {
                        self.pending_draft_job = None;
                    }
                    if matched
                        && saved.draft.stamp.generation == submitted_generation
                        && saved.draft.text == submitted_text
                        && self.draft_state.as_mut().is_some_and(|s| {
                            s.id() == draft_id
                                && s.acknowledge_comments(op, saved.clone(), snapshots)
                        })
                    {
                        if let Some(old) = self.drafts.iter_mut().find(|d| d.id == draft_id) {
                            *old = saved.draft;
                        }
                        self.message = if self.draft_state.as_ref().is_some_and(DraftEditor::dirty)
                        {
                            "Saved submitted snapshot; newer edits remain unsaved.".into()
                        } else {
                            "Working copy saved.".into()
                        };
                        self.submit(
                            Action::ListDraftRevisions { id: draft_id },
                            "Revision list",
                            cx,
                        );
                    }
                }
                Ok(Outcome::CommentStatusChanged {
                    result,
                    saved,
                    snapshots,
                }) => {
                    let matched = self.pending_status_job
                        == Some((terminal.id, result.op, result.comment.id));
                    if matched {
                        self.pending_status_job = None;
                    }
                    if matched
                        && let Some(state) = &mut self.draft_state
                        && state.id() == result.comment.draft_id
                    {
                        let refreshed = state.refresh_comments(saved, snapshots);
                        let applied = state.apply_status(&result);
                        if refreshed || applied {
                            self.message = format!("Comment {:?}.", result.comment.status);
                        }
                    }
                }
                Ok(Outcome::DraftSaved {
                    op,
                    id,
                    draft,
                    submitted_generation,
                    submitted_text,
                })
                | Ok(Outcome::DraftCheckpointed {
                    op,
                    id,
                    draft,
                    submitted_generation,
                    submitted_text,
                }) => {
                    let matched = self.pending_draft_job == Some((terminal.id, op));
                    if matched {
                        self.pending_draft_job = None;
                    }
                    if matched
                        && self.draft_state.as_mut().is_some_and(|state| {
                            state.id() == id
                                && state.generation() >= submitted_generation
                                && state.acknowledge(op, draft.clone())
                        })
                    {
                        if let Some(old) = self.drafts.iter_mut().find(|d| d.id == id) {
                            *old = draft;
                        }
                        self.message = if self.draft_state.as_ref().is_some_and(DraftEditor::dirty)
                        {
                            "Saved submitted snapshot; newer edits remain unsaved.".into()
                        } else {
                            "Working copy saved.".into()
                        };
                        let _ = submitted_text;
                        self.submit(Action::ListDraftRevisions { id }, "Revision list", cx);
                    }
                }
                Ok(Outcome::DraftRevisions { id, revisions }) => {
                    if self
                        .draft_state
                        .as_ref()
                        .is_some_and(|state| state.id() == id)
                    {
                        self.revisions = revisions;
                        if self.compare_before.is_none() {
                            self.compare_before = self.revisions.first().map(|r| r.id);
                        }
                        if self.compare_after.is_none() {
                            self.compare_after = self.revisions.last().map(|r| r.id);
                        }
                        self.message = if self.draft_state.as_ref().is_some_and(DraftEditor::dirty)
                        {
                            "Revision history ready; working copy has unsaved edits.".into()
                        } else {
                            "Revision history ready; working copy saved.".into()
                        };
                    }
                }
                Ok(Outcome::DraftRevisionOpened { draft, revision }) => {
                    if self
                        .draft_state
                        .as_ref()
                        .is_some_and(|state| state.id() == draft)
                    {
                        self.review = Some(revision);
                        self.message = "Read-only revision ready below the history.".into();
                    }
                }
                Ok(Outcome::DraftCompared {
                    draft,
                    before,
                    after,
                    diff,
                }) => {
                    if self
                        .draft_state
                        .as_ref()
                        .is_some_and(|state| state.id() == draft)
                        && self.compare_before == Some(before)
                        && self.compare_after == Some(after)
                    {
                        self.diff = Some(diff);
                        self.message = "Revision comparison ready below the selectors.".into();
                    }
                }
                Ok(Outcome::CandidateSaved { draft, revision }) => {
                    if self
                        .draft_state
                        .as_ref()
                        .is_some_and(|state| state.id() == draft)
                    {
                        self.message = format!(
                            "Candidate saved from answer {} under parent {}.",
                            revision.origin_turn.unwrap(),
                            revision.parent_id.unwrap()
                        );
                        self.submit(
                            Action::ListDraftRevisions { id: draft },
                            "Revision list",
                            cx,
                        );
                    }
                }
                Ok(
                    note @ (Outcome::NoteOpened { .. }
                    | Outcome::NotesObserved { .. }
                    | Outcome::NoteRecovery { .. }
                    | Outcome::NoteRecoveries { .. }
                    | Outcome::NoteBufferSaved { .. }
                    | Outcome::NoteSaved { .. }
                    | Outcome::NoteReconciled { .. }
                    | Outcome::NoteCompared { .. }
                    | Outcome::NoteReloaded { .. }
                    | Outcome::NoteRelinked { .. }
                    | Outcome::NoteCopySaved { .. }
                    | Outcome::NoteDiskAccepted { .. }
                    | Outcome::NoteApproved { .. }),
                ) => {
                    self.note_outcome(terminal.id, note, window, cx);
                }
            }
        }
        self.poll_notes(window, cx);
        let elapsed = match &self.phase {
            Phase::Running { since, .. } | Phase::Cancelling { since, .. } => {
                since.elapsed().as_secs()
            }
            _ => 0,
        };
        if elapsed != self.last_elapsed {
            self.last_elapsed = elapsed;
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }
    fn submit(&mut self, action: Action, label: &str, cx: &mut Context<Self>) -> Option<u64> {
        if !self.phase.can_submit() {
            return None;
        }
        let generation = action.generation();
        match self.worker.submit(action) {
            Ok(id) => {
                self.active = Some(id);
                self.phase = Phase::Running {
                    label: label.into(),
                    since: Instant::now(),
                };
                self.active_generation = generation;
                self.progress.clear();
                self.streamed_text.clear();
                self.message = format!("{label} started.");
                cx.notify();
                Some(id)
            }
            Err(error) => {
                if error == "workspace worker unavailable" {
                    self.phase = Phase::Failed(error.clone());
                }
                self.message = error;
                cx.notify();
                None
            }
        }
    }
    fn draft_guard(&mut self, cx: &mut Context<Self>) -> bool {
        if self
            .draft_state
            .as_ref()
            .is_some_and(|state| !state.can_replace())
        {
            self.message = "Save or discard draft edits and unsaved comment text, then wait for any pending save before switching.".into();
            cx.notify();
            false
        } else {
            true
        }
    }
    fn close_guard(&mut self, route: CloseRoute, cx: &mut Context<Self>) -> bool {
        if self
            .draft_state
            .as_ref()
            .is_some_and(|state| !state.can_close())
        {
            self.message = "Draft edits, comment text, or a save are still pending. Save or discard them before closing.".into();
            cx.notify();
            false
        } else if self
            .note_state
            .as_ref()
            .is_some_and(|state| !state.can_close())
            || self.pending_note_job.is_some()
            || self.pending_note_open.is_some()
            || self.worker.critical_note_pending()
        {
            self.note_schedule.request_close(route);
            self.note_control = None;
            self.message = "Close deferred: waiting for the latest note buffer recovery acknowledgement. Markdown is not saved by closing.".into();
            cx.notify();
            false
        } else {
            true
        }
    }
    fn choose_draft(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.phase.can_submit() || !self.draft_guard(cx) {
            return;
        }
        self.submit(Action::OpenDraftComments { id }, "Open draft", cx);
    }
    fn create_draft(&mut self, cx: &mut Context<Self>) {
        if !self.phase.can_submit() || !self.draft_guard(cx) {
            return;
        }
        let title = self.draft_title.read(cx).value().to_string();
        if title.trim().is_empty() {
            self.message = "Enter a draft title first.".into();
            cx.notify();
            return;
        }
        self.submit(
            Action::CreateDraft {
                op: Uuid::new_v4(),
                title,
                text: String::new(),
            },
            "Create draft",
            cx,
        );
    }
    fn save_draft(&mut self, checkpoint: bool, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
        }
        let Some(state) = &mut self.draft_state else {
            return;
        };
        if state.text().len() > brn_workflow::worker::MAX_DRAFT_BYTES {
            self.message = "Draft exceeds 1 MiB; shorten it before saving.".into();
            cx.notify();
            return;
        }
        let Some(submitted) = state.begin_save(Uuid::new_v4()) else {
            return;
        };
        let action = Action::WriteDraftWithComments {
            request: DraftWriteWithComments {
                op: submitted.op,
                draft_id: submitted.id,
                expected: submitted.expected,
                generation: submitted.generation,
                text: submitted.text.clone(),
                edits: submitted.edits.clone(),
                checkpoint,
            },
        };
        if let Some(job) = self.submit(
            action,
            if checkpoint {
                "Save checkpoint"
            } else {
                "Save working copy"
            },
            cx,
        ) {
            self.pending_draft_job = Some((job, submitted.op));
            self.message="Saving submitted snapshot; keep typing if needed. Close waits for acknowledgement.".into();
        } else if let Some(state) = &mut self.draft_state {
            state.fail(submitted.op);
        }
        cx.notify();
    }
    fn discard_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            self.message = "Wait for the current action before discarding.".into();
            cx.notify();
            return;
        }
        if let Some(state) = &mut self.draft_state
            && state.discard()
        {
            let text = state.text().to_owned();
            self.draft_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
            self.message = "Unsaved edits discarded; acknowledged working copy restored.".into();
            cx.notify();
        }
    }
    fn capture_comment(&mut self, cx: &mut Context<Self>) {
        let editor = self.draft_editor.read(cx);
        let text = editor.value().to_string();
        let range = editor.selected_range();
        let Some(state) = &mut self.draft_state else {
            return;
        };
        if state.text() != text {
            self.message = "Editor changed; select the passage again.".into();
        } else {
            self.message = match state.capture(range) {
                Ok(()) => "Passage captured. Write a comment, then save its checkpoint.".into(),
                Err(error) => error,
            };
        }
        cx.notify();
    }
    fn add_comment(&mut self, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
        }
        let Some(state) = &mut self.draft_state else {
            return;
        };
        let Some((submitted, request)) = state.begin_comment(Uuid::new_v4()) else {
            self.message = "Capture a current passage and enter a comment (up to 64 KiB).".into();
            cx.notify();
            return;
        };
        if let Some(job) = self.submit(
            Action::CreateDraftComment { request },
            "Save checkpoint + add comment",
            cx,
        ) {
            self.pending_draft_job = Some((job, submitted.op));
            self.message = "Saving captured snapshot; you may continue typing.".into();
        } else if let Some(state) = &mut self.draft_state {
            state.fail(submitted.op);
        }
        cx.notify();
    }
    fn discard_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = &mut self.draft_state {
            if state.pending() {
                return;
            }
            state.discard_composer();
            self.comment_input
                .update(cx, |editor, cx| editor.set_value("", window, cx));
            self.message = "Unsaved comment discarded.".into();
            cx.notify();
        }
    }
    fn change_comment_status(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
        }
        let Some(state) = &self.draft_state else {
            return;
        };
        let Some(view) = state.comments().iter().find(|v| v.comment.id == id) else {
            return;
        };
        let status = if view.comment.status == CommentStatus::Open {
            CommentStatus::Resolved
        } else {
            CommentStatus::Open
        };
        let request = CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: state.id(),
            comment_id: id,
            expected_status_version: view.comment.status_version,
            status,
        };
        if let Some(job) = self.submit(
            Action::SetCommentStatus {
                request: request.clone(),
            },
            "Update comment status",
            cx,
        ) {
            self.pending_status_job = Some((job, request.op, id));
        }
    }
    fn show_comment_passage(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &self.draft_state else {
            return;
        };
        if self.draft_editor.read(cx).value() != state.text() {
            self.message = "Editor changed; passage selection needs a refreshed preview.".into();
            cx.notify();
            return;
        }
        let Some(index) = state.comments().iter().position(|v| v.comment.id == id) else {
            return;
        };
        let Ok(states) = state.preview_states() else {
            self.message = "Current comment locations could not be validated.".into();
            cx.notify();
            return;
        };
        let AnchorState::Anchored { start, end } = states[index] else {
            self.message = "This comment has no safe current passage.".into();
            cx.notify();
            return;
        };
        if state.text().get(start..end)
            != Some(state.comments()[index].comment.original_quote.as_str())
        {
            self.message = "Current passage no longer matches the original quote.".into();
            cx.notify();
            return;
        }
        self.draft_editor.update(cx, |editor, cx| {
            editor.set_selected_range(start..end, cx);
            editor.focus(window, cx);
        });
        self.draft_editor_anchor.scroll_to(window, cx);
        self.message = "Current passage selected.".into();
        cx.notify();
    }
    fn show_comment_original(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(state) = &self.draft_state else {
            return;
        };
        if let Some(revision) = state.original_revision(id) {
            self.comment_original = Some(revision);
            self.message = "Read-only original revision shown with this comment.".into();
        } else {
            self.message = "Original revision unavailable in this projection.".into();
        }
        cx.notify();
    }
    fn save_candidate(&mut self, turn: Uuid, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
        }
        let Some(state) = &self.draft_state else {
            self.message = "Open a target draft before saving a candidate.".into();
            cx.notify();
            return;
        };
        let parent = self
            .review
            .as_ref()
            .map(|revision| revision.id)
            .unwrap_or(state.stamp().base_revision);
        self.submit(
            Action::SaveCandidate {
                op: Uuid::new_v4(),
                draft: state.id(),
                parent,
                turn,
            },
            "Save candidate",
            cx,
        );
    }
    fn import(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.import_path.clone() else {
            self.message = "Choose a UTF-8 .md or .txt file first.".into();
            cx.notify();
            return;
        };
        if !path.is_absolute() {
            self.message = "Import path must be absolute.".into();
            cx.notify();
            return;
        }
        self.submit(
            Action::Import {
                path,
                approval: Approval::Approved,
            },
            "Import",
            cx,
        );
    }
    fn choose_file(&mut self, cx: &mut Context<Self>) {
        if self.choosing_file || !self.phase.can_submit() {
            return;
        }
        self.choosing_file = true;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose source".into()),
        });
        cx.spawn(async move |this, cx| {
            let selection = receiver.await;
            let _ = this.update(cx, |this, cx| {
                this.choosing_file = false;
                match selection {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            this.import_path = Some(path);
                            this.message =
                                "File selected. Import and approve it when ready.".into();
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => this.message = format!("File chooser failed: {error}"),
                    Err(error) => this.message = format!("File chooser closed: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn query(&self, cx: &Context<Self>) -> String {
        self.query.read(cx).value().trim().to_string()
    }
    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
        if query.is_empty() {
            self.message = "Enter a question first.".into();
            cx.notify();
            return;
        }
        self.submit(
            Action::Search {
                query,
                profile: self.profile,
                generation: self.generation,
            },
            "Search",
            cx,
        );
    }
    fn ask(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
        if query.is_empty() {
            self.message = "Enter a question first.".into();
            cx.notify();
            return;
        }
        self.submit(
            Action::Ask {
                session: self.selected_session,
                query,
                profile: self.profile,
                generation: self.generation,
            },
            "Answer",
            cx,
        );
    }
    fn choose_profile(&mut self, profile: Profile, cx: &mut Context<Self>) {
        if self.profile != profile {
            self.profile = profile;
            self.generation = self.generation.wrapping_add(1);
            self.search = None;
            self.selected_evidence = None;
            self.streamed_text.clear();
        }
        cx.notify();
    }
    fn choose_session(&mut self, session: Option<Uuid>, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
        }
        self.selected_session = session;
        self.generation = self.generation.wrapping_add(1);
        self.search = None;
        self.selected_evidence = None;
        self.streamed_text.clear();
        self.history.clear();
        self.selected_turn = None;
        self.selected_saved_evidence = None;
        if let Some(session) = session {
            self.submit(Action::History { session }, "History", cx);
        } else {
            self.message =
                "New session selected. The next answer creates a persisted conversation.".into();
            cx.notify();
        }
    }
    fn remember_note(&mut self, view: NoteView) {
        if let Some(old) = self.note_views.iter_mut().find(|old| old.id == view.id) {
            *old = view;
        } else {
            self.note_views.push(view);
        }
    }
    fn install_note(&mut self, view: NoteView, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .note_state
            .as_ref()
            .is_some_and(|state| !state.can_close())
        {
            self.message = "Opened in storage; later edits in the current note remain protected. Flush them before switching.".into();
            return;
        }
        self.note_state = Some(NoteEditor::new(view));
        let text = self.note_state.as_ref().unwrap().input_text();
        self.note_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.note_schedule = NoteScheduler::default();
        self.note_comparison = None;
        self.note_copy = None;
        self.original_note_operation = None;
        self.page = Page::Notes;
        self.message =
            "Note opened. Save writes Markdown; recovery only protects work in BRN.".into();
    }
    fn note_outcome(
        &mut self,
        job: u64,
        outcome: Outcome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut acknowledged = Ok(());
        match outcome {
            Outcome::NoteOpened { op, view } => {
                self.note_sources_changed = true;
                self.remember_note(view.clone());
                if self
                    .pending_note_open
                    .as_ref()
                    .is_some_and(|pending| pending.job == job && pending.op == Some(op))
                {
                    self.pending_note_open = None;
                    self.install_note(view, window, cx);
                }
            }
            Outcome::NotesObserved { views } => {
                for view in views {
                    self.note_sources_changed |= self
                        .note_views
                        .iter()
                        .find(|old| old.id == view.id)
                        .is_none_or(|old| {
                            old.current_file_state != view.current_file_state
                                || old.availability != view.availability
                                || old.search_approval != view.search_approval
                        });
                    self.remember_note(view.clone());
                    if self
                        .pending_note_open
                        .as_ref()
                        .is_some_and(|pending| pending.job == job && pending.id == Some(view.id))
                    {
                        self.pending_note_open = None;
                        self.install_note(view, window, cx);
                    } else if let Some(state) = &mut self.note_state
                        && state.id() == view.id
                        && !state.pending()
                    {
                        acknowledged = state.observe(view);
                    }
                }
            }
            Outcome::NoteRecoveries { recoveries } => {
                self.note_observations
                    .extend(recoveries.iter().map(|recovery| recovery.note_id));
                self.note_recoveries = recoveries;
            }
            Outcome::NoteRecovery { id, recovery } => {
                if let Some(recovery) = recovery.filter(|recovery| recovery.note_id == id) {
                    self.message = format!(
                        "Recovery baseline:\n{}\n\nDurable working text:\n{}\n\nPending operations: {:?}",
                        recovery.baseline, recovery.working, recovery.pending_operations
                    );
                    self.note_recoveries.retain(|old| old.note_id != id);
                    self.note_recoveries.push(recovery);
                } else {
                    self.message = "No recovery exists for that note.".into();
                }
            }
            Outcome::NoteBufferSaved {
                submission,
                receipt,
            } => {
                if self.pending_note_job == Some((job, submission.operation_id))
                    && let Some(state) = &mut self.note_state
                    && state.id() == submission.note_id
                {
                    self.pending_note_job = None;
                    acknowledged = state.acknowledge_recovery(submission.operation_id, receipt);
                    self.message =
                        "Submitted buffer is recoverable in BRN; Markdown was not written.".into();
                }
            }
            Outcome::NoteSaved {
                submission,
                receipt,
            } => {
                if self.pending_note_job == Some((job, submission.operation_id))
                    && let Some(state) = &mut self.note_state
                    && state.id() == submission.note_id
                {
                    self.pending_note_job = None;
                    acknowledged = state.acknowledge(submission.operation_id, receipt);
                    self.note_observations.insert(state.id());
                    self.message = "Submitted snapshot saved to Markdown; any later typing remains in the editor.".into();
                }
            }
            Outcome::NoteCopySaved {
                submission,
                receipt,
                destination,
            } => {
                if self.pending_note_job == Some((job, submission.operation_id))
                    && let Some(state) = &mut self.note_state
                    && state.id() == submission.note_id
                {
                    self.pending_note_job = None;
                    acknowledged =
                        state.acknowledge_copy(submission.operation_id, receipt, &destination);
                    if acknowledged.is_ok() {
                        self.note_copy = Some(destination.clone());
                        self.remember_note(destination);
                        self.message = "Separate copy verified. The original editor and any uncertain original save are unchanged.".into();
                    }
                }
            }
            Outcome::NoteReloaded {
                op,
                id,
                expected,
                view,
            } => {
                if self.pending_note_job == Some((job, op))
                    && let Some(state) = &mut self.note_state
                    && state.id() == id
                    && state.stamp() == expected
                {
                    self.pending_note_job = None;
                    acknowledged = state.acknowledge_discard(op, view.clone());
                    if acknowledged.is_ok() {
                        let text = state.input_text();
                        self.note_editor
                            .update(cx, |editor, cx| editor.set_value(text, window, cx));
                        self.remember_note(view);
                        self.note_comparison = None;
                        self.message =
                            "Confirmed reload replaced local text with a fresh disk baseline."
                                .into();
                    }
                }
            }
            Outcome::NoteRelinked {
                op,
                id,
                expected,
                view,
            } => {
                if self.pending_note_job == Some((job, op))
                    && let Some(state) = &mut self.note_state
                    && state.id() == id
                    && state.stamp() == expected
                {
                    self.pending_note_job = None;
                    acknowledged = state.acknowledge_rebase(op, view.clone());
                    self.remember_note(view);
                    self.note_comparison = None;
                    self.message = "Confirmed relink established the selected identity; local edits were retained.".into();
                }
            }
            Outcome::NoteDiskAccepted { op, save_op, view } => {
                if self.pending_note_job == Some((job, op))
                    && self.original_note_operation == Some(save_op)
                    && let Some(state) = &mut self.note_state
                    && state.id() == view.id
                {
                    self.pending_note_job = None;
                    acknowledged = state.acknowledge_rebase(op, view.clone());
                    self.remember_note(view);
                    self.note_comparison = None;
                    self.message = "Reviewed disk state accepted. Original outcome and recovery remain retained; no Markdown write or approval occurred.".into();
                }
            }
            Outcome::NoteCompared { comparison } => {
                if self
                    .note_state
                    .as_ref()
                    .is_some_and(|state| state.id() == comparison.note_id)
                {
                    self.note_comparison = Some(comparison);
                    self.message = "Read-only comparison: starting bytes, local work, and newly observed disk/deletion. Unexpected displacement remains protected by the save operation.".into();
                }
            }
            Outcome::NoteReconciled { op, receipt } => {
                if self.pending_note_job != Some((job, op)) {
                    return;
                }
                self.message = format!(
                    "Operation {op}: {:?}; recovery confirmed: {}. This receipt does not prove the file is still current.",
                    receipt.filesystem_outcome, receipt.recovery_available
                );
                self.note_observations.insert(receipt.source_note_id);
                if self.pending_note_job == Some((job, op)) {
                    self.pending_note_job = None;
                }
            }
            Outcome::NoteApproved { receipt } => {
                if self.pending_note_job == Some((job, receipt.operation_id))
                    && self
                        .note_state
                        .as_ref()
                        .is_some_and(|state| state.id() == receipt.note_id)
                {
                    self.pending_note_job = None;
                    self.note_observations.insert(receipt.note_id);
                    self.message = "Saved snapshot explicitly approved for search, not publication. Rebuild the index.".into();
                    self.note_sources_changed = true;
                }
            }
            _ => unreachable!("only note outcomes are routed here"),
        }
        if let Err(error) = acknowledged {
            self.message = format!("Note state refused receipt: {}", error.message);
            self.note_schedule.recovery_failed();
        }
    }
    fn queue_note_control(&mut self, control: NoteControl, cx: &mut Context<Self>) {
        if matches!(control, NoteControl::Open { .. } | NoteControl::Select(_))
            && !self.draft_guard(cx)
        {
            return;
        }
        if self.note_schedule.closing() {
            self.message = "A note flush/close decision is already pending. Finish it or cancel it before another action.".into();
            cx.notify();
            return;
        }
        self.note_control = Some(control);
        self.note_schedule.request_close(CloseRoute::Switch);
        self.message =
            "Waiting for the worker and the latest buffer recovery before the note action.".into();
        cx.notify();
    }
    fn run_note_control(&mut self, control: NoteControl, cx: &mut Context<Self>) {
        if let NoteControl::Open {
            op,
            vault,
            relative,
        } = control
        {
            if let Some(job) = self.submit(
                Action::OpenNote {
                    op,
                    vault,
                    relative,
                },
                "Open Markdown note",
                cx,
            ) {
                self.pending_note_open = Some(PendingNoteOpen {
                    job,
                    op: Some(op),
                    id: None,
                });
            }
            return;
        }
        if let NoteControl::Select(id) = control {
            if let Some(job) = self.submit(
                Action::ObserveNotes { ids: vec![id] },
                "Open registered note",
                cx,
            ) {
                self.pending_note_open = Some(PendingNoteOpen {
                    job,
                    op: None,
                    id: Some(id),
                });
            }
            return;
        }
        let Some(state) = &mut self.note_state else {
            self.message = "Open a Markdown note first.".into();
            return;
        };
        let id = state.id();
        let op = Uuid::new_v4();
        let request = match &control {
            NoteControl::Save => state.begin_save(op),
            NoteControl::Copy(_) => state.begin_copy(op),
            NoteControl::Reload => state.begin_discard(op),
            NoteControl::Relink(_) | NoteControl::Accept { .. } => state.begin_rebase(op),
            _ => Ok(brn_workflow::notes::NoteSubmission {
                operation_id: op,
                note_id: id,
                expected: state.stamp(),
                generation: state.stamp().generation,
                text: state.text().into(),
            }),
        };
        let request = match request {
            Ok(request) => request,
            Err(error) => {
                self.message = error.message;
                return;
            }
        };
        let action = match control {
            NoteControl::Save => {
                self.original_note_operation = Some(op);
                Action::SaveNote { request }
            }
            NoteControl::Copy(relative) => Action::SaveNoteCopy { request, relative },
            NoteControl::Compare => Action::CompareNote { id },
            NoteControl::Reload => Action::ReloadNote {
                op,
                id,
                expected: request.expected,
                discard: true,
            },
            NoteControl::Relink(relative) => Action::RelinkNote {
                op,
                id,
                expected: request.expected,
                relative,
                confirm_identity: true,
            },
            NoteControl::Accept {
                save_op,
                file_state,
            } => Action::AcceptNoteDiskState {
                op,
                save_op,
                file_state,
            },
            NoteControl::Approve { file_state } => {
                Action::ApproveNoteSnapshot { op, id, file_state }
            }
            NoteControl::Open { .. } | NoteControl::Select(_) => unreachable!(),
        };
        let tracked = !matches!(action, Action::CompareNote { .. });
        self.note_sources_changed |= tracked;
        if let Some(job) = self.submit(action, "Note action", cx) {
            if tracked {
                self.pending_note_job = Some((job, op));
            }
        } else {
            self.refuse_note_admission(op);
        }
        self.note_observations.insert(id);
    }
    fn refuse_note_admission(&mut self, op: Uuid) {
        if let Some(state) = &mut self.note_state
            && state.pending()
        {
            let failure = brn_workflow::notes::NoteFailure {
                code: brn_workflow::notes::NoteErrorCode::WorkspaceBusy,
                message: self.message.clone(),
                operation_id: Some(op),
                note_id: Some(state.id()),
                phase: None,
                filesystem_outcome: brn_workflow::notes::FileOutcome::NotApplied,
                recovery_available: false,
            };
            if let Err(error) = state.fail(op, failure) {
                self.message = error.message;
            }
            self.note_schedule.recovery_failed();
        }
    }
    fn poll_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let idle = self.phase.can_submit()
            && self.pending_note_job.is_none()
            && self.pending_note_open.is_none();
        if !idle {
            return;
        }
        let registered: Vec<_> = self
            .note_recoveries
            .iter()
            .map(|recovery| recovery.note_id)
            .chain(self.note_views.iter().map(|view| view.id))
            .collect();
        // After a refused save, observe the durable input stamp before retrying recovery.
        if let Some(ids) = self.note_observations.take(idle, &registered) {
            if self
                .submit(
                    Action::ObserveNotes { ids: ids.clone() },
                    "Observe registered notes",
                    cx,
                )
                .is_none()
            {
                self.note_observations.extend(ids);
            }
            return;
        }
        if let Some(state) = &mut self.note_state
            && self
                .note_schedule
                .wants_recovery(Instant::now(), state, idle)
        {
            match state.begin_recovery(Uuid::new_v4()) {
                Ok(request) => {
                    let op = request.operation_id;
                    if let Some(job) = self.submit(
                        Action::SaveNoteBuffer { request },
                        "Recover note buffer (not Markdown)",
                        cx,
                    ) {
                        self.pending_note_job = Some((job, op));
                    } else {
                        self.refuse_note_admission(op);
                    }
                }
                Err(error) => {
                    self.message = error.message;
                    self.note_schedule.recovery_failed();
                }
            }
            return;
        }
        let ready = self
            .note_state
            .as_ref()
            .is_none_or(|state| self.note_schedule.can_finish(state, idle));
        if self.note_schedule.closing()
            && ready
            && let Some(route) = self.note_schedule.take_close()
        {
            match route {
                CloseRoute::Switch => {
                    if let Some(control) = self.note_control.take() {
                        self.run_note_control(control, cx);
                    }
                }
                CloseRoute::Quit | CloseRoute::Window => {
                    if !self.close_guard(route, cx) {
                        return;
                    }
                    if route == CloseRoute::Quit {
                        cx.quit();
                    } else {
                        window.remove_window();
                    }
                }
            }
            return;
        }
        if self.note_sources_changed
            && self
                .submit(
                    Action::Refresh {
                        session: self.selected_session,
                    },
                    "Refresh saved source states",
                    cx,
                )
                .is_some()
        {
            self.note_sources_changed = false;
        }
    }
    fn choose_note_path(&mut self, vault: bool, cx: &mut Context<Self>) {
        if self.choosing_file {
            return;
        }
        if !vault && self.vault_path.is_none() {
            self.message = "Choose the registered vault root first.".into();
            cx.notify();
            return;
        }
        self.choosing_file = true;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: !vault,
            directories: vault,
            multiple: false,
            prompt: Some(
                if vault {
                    "Choose local vault"
                } else {
                    "Open Markdown note"
                }
                .into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            let selection = receiver.await;
            let _ = this.update(cx, |this, cx| {
                this.choosing_file = false;
                match selection {
                    Ok(Ok(Some(paths))) => if let Some(path) = paths.into_iter().next() {
                        if vault {
                            this.vault_path = Some(path);
                            this.message = "Vault selected. A different registered root requires a separate data directory.".into();
                        } else if let Some(root) = &this.vault_path {
                            match path.strip_prefix(root) {
                                Ok(relative) => this.queue_note_control(NoteControl::Open {
                                    op: Uuid::new_v4(), vault: root.clone(), relative: relative.to_owned(),
                                }, cx),
                                Err(_) => this.message = "Choose a Markdown file within the selected vault.".into(),
                            }
                        }
                    },
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => this.message = format!("Note chooser failed: {error}"),
                    Err(error) => this.message = format!("Note chooser closed: {error}"),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}
impl Render for Desktop {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .id("workspace-body")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .gap_3()
            .p_3()
            .overflow_y_scroll()
            .track_scroll(&self.draft_scroll)
            .vertical_scrollbar(&self.draft_scroll)
            .child(self.message.clone());
        if matches!(self.phase, Phase::Failed(_)) {
            body = body.child("Workspace unavailable. Review the error above, correct the workspace, then relaunch.");
        }
        if !self.progress.is_empty()
            && matches!(self.phase, Phase::Running { .. } | Phase::Cancelling { .. })
        {
            body = body.child(format!("Progress: {}", self.progress));
        }
        match self.page {
            Page::Notes => {
                body = body
                    .child("Local Markdown notes")
                    .child("Explicit Save/Cmd-S writes Markdown. Recovery in BRN is not publication or search approval.")
                    .child(
                        div().flex().flex_wrap().gap_2()
                            .child(Button::new("choose-vault").label("Choose vault...")
                                .disabled(self.choosing_file || self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.choose_note_path(true, cx))))
                            .child(Button::new("open-note").label("Open Markdown note...")
                                .disabled(self.choosing_file || self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.choose_note_path(false, cx))))
                            .child(Button::new("note-recoveries").label("Refresh recovery list")
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.submit(Action::ListNoteRecoveries, "Note recoveries", cx);
                                }))),
                    )
                    .child(format!("Selected vault: {}", self.vault_path.as_ref().map_or(
                        "not selected (registered notes can still be reopened below)".into(),
                        |path| path.display().to_string()
                    )));
                if let Some(failure) = &self.last_note_failure {
                    body = body.child(format!("Recorded note failure: operation {:?}, note {:?}, {:?}, phase {:?}, filesystem {:?}, recovery confirmed: {}. {}",
                        failure.operation_id, failure.note_id, failure.code, failure.phase,
                        failure.filesystem_outcome, failure.recovery_available, failure.message))
                        .child(Button::new("dismiss-note-error").label("Dismiss error display (does not resolve operation)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.last_note_failure = None;
                                cx.notify();
                            })));
                }
                for view in &self.note_views {
                    let id = view.id;
                    body = body.child(
                        Button::new(format!("registered-note-{id}"))
                            .label(format!(
                                "{} - {:?} - search {:?}",
                                view.relative_path.display(),
                                view.availability,
                                view.search_approval
                            ))
                            .disabled(self.note_schedule.closing())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.queue_note_control(NoteControl::Select(id), cx);
                            })),
                    );
                }
                if let Some(state) = &self.note_state {
                    let id = state.id();
                    body = body
                        .child(format!(
                            "{} - {}",
                            state.view().relative_path.display(),
                            state.display_state()
                        ))
                        .child(format!(
                            "Availability: {:?}; last observed search approval: {:?}",
                            state.view().availability,
                            state.view().search_approval
                        ));
                    if let Some(message) = &state.view().availability_message {
                        body = body.child(message.clone());
                    }
                    if let Some(failure) = state.failure() {
                        body = body.child(format!(
                            "Retained operation {:?}: {:?}, phase {:?}, effect {:?}; {}",
                            failure.operation_id,
                            failure.code,
                            failure.phase,
                            failure.filesystem_outcome,
                            failure.message
                        ));
                    }
                    if let Some(failure) = state.copy_failure() {
                        body = body.child(format!(
                            "Separate copy outcome {:?}: {:?}; {}",
                            failure.operation_id, failure.filesystem_outcome, failure.message
                        ));
                    }
                    if state.needs_recovery() || state.pending() || self.note_schedule.closing() {
                        body = body.child("Recovery/operation pending. The 500 ms timer schedules work; only an acknowledged commit protects the latest typing. Busy provider work may delay it.");
                    }
                    body = body
                        .child(Editor::new(&self.note_editor).h(px(300.)).flex_shrink_0()
                            .disabled(self.pending_note_open.is_some() || state.discard_pending())
                            .aria_label("Markdown note editor"))
                        .child(div().flex().flex_wrap().gap_2()
                            .child(Button::new("save-note").label("Save to Markdown (Cmd-S)")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.queue_note_control(NoteControl::Save, cx))))
                            .child(Button::new("retry-note-recovery").label("Flush / retry recovery")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.retry();
                                    cx.notify();
                                })))
                            .child(Button::new("cancel-note-close").label("Cancel pending close/action")
                                .disabled(!self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.take_close();
                                    this.note_control = None;
                                    this.message = "Close/action cancelled; note work remains accessible.".into();
                                    cx.notify();
                                })))
                            .child(Button::new("compare-note").label("Compare baseline / local / disk")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.queue_note_control(NoteControl::Compare, cx))))
                            .child(Button::new("reload-note").label("Confirm reload: discard local edits")
                                .disabled(!self.phase.can_submit() || state.pending() || self.pending_note_job.is_some())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.take_close();
                                    this.note_control = None;
                                    this.run_note_control(NoteControl::Reload, cx);
                                })))
                            .child(Button::new("discard-unrecovered-note").label("Confirm discard ONLY unrecovered typing")
                                .disabled(state.pending() || self.pending_note_job.is_some())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(state) = &mut this.note_state {
                                        let result = state.begin_discard(Uuid::new_v4())
                                            .and_then(|request| state.discard_unrecovered(request.operation_id));
                                        match result {
                                            Ok(()) => {
                                                let text = state.input_text();
                                                this.note_editor.update(cx, |editor, cx| editor.set_value(text, window, cx));
                                                this.message = "Unrecovered typing explicitly discarded; acknowledged recovery and any uncertain operation remain retained.".into();
                                            }
                                            Err(error) => this.message = error.message,
                                        }
                                    }
                                    cx.notify();
                                })))
                            .child(Button::new("inspect-note-recovery").label("Inspect retained recovery")
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.submit(Action::NoteRecovery { id }, "Inspect note recovery", cx);
                                }))))
                        .child(Input::new(&self.note_path))
                        .child(div().flex().flex_wrap().gap_2()
                            .child(Button::new("copy-note").label("Save separate copy to chosen path")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let path = PathBuf::from(this.note_path.read(cx).value().to_string());
                                    this.queue_note_control(NoteControl::Copy(path), cx);
                                })))
                            .child(Button::new("relink-note").label("Confirm selected path identity; retain edits")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let path = PathBuf::from(this.note_path.read(cx).value().to_string());
                                    this.queue_note_control(NoteControl::Relink(path), cx);
                                }))));
                    if let Some(token) = state.view().current_file_state {
                        body = body.child(
                            Button::new("approve-note")
                                .label("Explicitly approve saved snapshot for search (not Save)")
                                .disabled(
                                    self.note_schedule.closing()
                                        || state.view().availability
                                            != brn_workflow::notes::NoteAvailability::Available,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.queue_note_control(
                                        NoteControl::Approve { file_state: token },
                                        cx,
                                    );
                                })),
                        );
                    }
                    if let Some(recovery) = self
                        .note_recoveries
                        .iter()
                        .find(|recovery| recovery.note_id == id)
                    {
                        for op in &recovery.pending_operations {
                            let op = *op;
                            body = body.child(
                                Button::new(format!("reconcile-note-{op}"))
                                    .label(format!(
                                        "Inspect/reconcile operation {op} (no filesystem replay)"
                                    ))
                                    .disabled(!self.phase.can_submit() || state.pending())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.original_note_operation = Some(op);
                                        if let Some(job) = this.submit(
                                            Action::ReconcileNoteSave { op },
                                            "Reconcile note save",
                                            cx,
                                        ) {
                                            this.pending_note_job = Some((job, op));
                                        }
                                    })),
                            );
                        }
                    }
                }
                if let Some(comparison) = &self.note_comparison {
                    let local = self
                        .note_state
                        .as_ref()
                        .map_or(comparison.working.as_str(), NoteEditor::text);
                    body = body.child(format!("Starting snapshot:\n{}\n\nLocal editor:\n{}\n\nObserved disk:\n{}\n\nUnexpected displacement, if any, remains protected in the operation recovery; comparison does not delete it.",
                        comparison.baseline, local, comparison.observed.as_deref().unwrap_or("[deleted or unavailable]")));
                    if let (Some(save_op), Some(file_state)) =
                        (self.original_note_operation, comparison.observed_file_state)
                    {
                        body = body.child(Button::new("accept-note-disk")
                            .label("Accept reviewed current disk state; KEEP recovery and original outcome")
                            .disabled(self.note_schedule.closing())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.queue_note_control(NoteControl::Accept { save_op, file_state }, cx);
                            })));
                    }
                }
                if let Some(copy) = &self.note_copy {
                    body = body.child(format!("Verified separate copy: {} ({}) - {:?}. Original editor retained; copy has no inherited search approval.",
                        copy.relative_path.display(), copy.id, copy.availability));
                }
            }
            Page::Workspace => {
                body = body.child("Grounded workspace")
                    .child("Import a UTF-8 Markdown or text file and explicitly approve it for search.")
                    .child(div().flex().flex_wrap().gap_2()
                        .child(Button::new("choose-file").label(if self.choosing_file { "Choosing…" } else { "Choose file…" }).disabled(self.choosing_file || !self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| this.choose_file(cx))))
                        .child(Button::new("import-approved").label("Import and approve").disabled(!self.phase.can_submit() || self.import_path.is_none()).on_click(cx.listener(|this, _, _, cx| this.import(cx))))
                        .child(Button::new("build-index").label("Build index").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| { this.submit(Action::Build, "Index build", cx); })))
                        .child(Button::new("refresh").label("Refresh").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| { this.submit(Action::Refresh { session: this.selected_session }, "Refresh", cx); }))))
                    .child(div().min_w(px(0.)).child(format!("Selected file: {}", self.import_path.as_ref().map_or("none".into(), |p| spaced_identifier(&p.display().to_string())))));
                body = body.child(format!(
                    "Sources (last validated observation): {}",
                    self.source_states.len()
                ));
                for state in self
                    .source_states
                    .iter()
                    .filter(|s| s.current_state != brn_workflow::SourceCurrentState::Current)
                {
                    body = body.child(div().min_w(px(0.)).child(format!(
                        "{} · {:?} · {}",
                        state.title,
                        state.current_state,
                        state.message.as_deref().unwrap_or("not current")
                    )));
                }
                for source in &self.sources {
                    let source_id = source.source_id;
                    let version = source.version_id;
                    body = body.child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w(px(0.))
                            .gap_2()
                            .child(format!(
                                "{} · Current at last observation · {:?} · {} bytes",
                                source.title,
                                source.approval,
                                source.bytes.len()
                            ))
                            .child(format!(
                                "Revision: {}",
                                spaced_identifier(&version.to_string())
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        Button::new(format!("approve-{source_id}"))
                                            .label("Approve")
                                            .disabled(!self.phase.can_submit())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.submit(
                                                    Action::SetApproval {
                                                        source: source_id,
                                                        version,
                                                        approval: Approval::Approved,
                                                    },
                                                    "Approve",
                                                    cx,
                                                );
                                            })),
                                    )
                                    .child(
                                        Button::new(format!("withdraw-{source_id}"))
                                            .label("Withdraw")
                                            .disabled(!self.phase.can_submit())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.submit(
                                                    Action::SetApproval {
                                                        source: source_id,
                                                        version,
                                                        approval: Approval::Withdrawn,
                                                    },
                                                    "Withdraw",
                                                    cx,
                                                );
                                            })),
                                    ),
                            ),
                    );
                }
                body = body
                    .child("Question")
                    .child(
                        Editor::new(&self.query)
                            .h(px(110.))
                            .flex_shrink_0()
                            .aria_label("Question for approved sources"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(Button::new("profile-keyword").label("Keyword").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.choose_profile(Profile::Keyword, cx)
                                }),
                            ))
                            .child(Button::new("profile-semantic").label("Semantic").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.choose_profile(Profile::Semantic, cx)
                                }),
                            ))
                            .child(Button::new("profile-hybrid").label("Hybrid").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.choose_profile(Profile::Hybrid, cx)
                                }),
                            )),
                    )
                    .child(format!("Selected profile: {:?}", self.profile))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new("search")
                                    .label("Search")
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(|this, _, _, cx| this.search(cx))),
                            )
                            .child(
                                Button::new("ask")
                                    .label("Ask from sources")
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(|this, _, _, cx| this.ask(cx))),
                            ),
                    );
                if let Some(search) = &self.search {
                    body = body.child(format!("Search passages for: {}", search.query));
                    for (i, hit) in search.evidence.iter().enumerate() {
                        let evidence = hit.clone();
                        body = body.child(
                            Button::new(format!("passage-{i}"))
                                .label(passage_button_label(i, hit, &self.sources))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected_evidence = Some(evidence.clone());
                                    cx.notify();
                                })),
                        );
                    }
                }
                if let Some(hit) = &self.selected_evidence {
                    body = body.child(format!(
                        "Passage from last validated search · source {} · revision {} · bytes {}..{}\n{}",
                        hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote
                    ));
                }
                if self.phase.shows_live_answer() && !self.streamed_text.is_empty() {
                    let heading = if matches!(self.phase, Phase::Cancelling { .. }) {
                        "Provisional partial answer while cancellation finishes (not saved)"
                    } else {
                        "Provisional answer in progress (not saved)"
                    };
                    body = body.child(format!("{heading}:\n{}", self.streamed_text));
                }
                if let Some(turn) = self.selected_turn.and_then(|i| self.history.get(i))
                    && let Some(answer) = &turn.answer
                {
                    body = body
                        .child(format!("Saved answer ({:?}):\n{}", turn.status, answer))
                        .child(
                            Button::new("view-saved-answer")
                                .label("View saved answer and evidence")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.page = Page::Activity;
                                    cx.notify();
                                })),
                        );
                }
            }
            Page::Activity => {
                body = body
                    .child("Saved conversations")
                    .child(
                        Button::new("new-session")
                            .label("New session")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(|this, _, _, cx| this.choose_session(None, cx))),
                    )
                    .child(format!(
                        "Selected session: {}",
                        self.selected_session
                            .map_or("new".into(), |id| id.to_string())
                    ));
                for session in &self.sessions {
                    let id = session.id;
                    body =
                        body.child(
                            Button::new(format!("session-{id}"))
                                .label(format!(
                                    "{}… · {} turns{}",
                                    &id.to_string()[..8],
                                    session.turns,
                                    if session.has_thread {
                                        " · provider linked"
                                    } else {
                                        " · recovery needed"
                                    }
                                ))
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_session(Some(id), cx)
                                })),
                        );
                }
                for (i, turn) in self.history.iter().enumerate() {
                    body = body.child(
                        Button::new(format!("turn-{i}"))
                            .label(format!(
                                "Historical: {} · {} · {:?} · {:?}",
                                compact_title(&turn.question),
                                turn.profile,
                                turn.status,
                                turn.evidence_currentness
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_turn = Some(i);
                                this.selected_saved_evidence = None;
                                cx.notify();
                            })),
                    );
                }
                if let Some(turn) = self.selected_turn.and_then(|i| self.history.get(i)) {
                    body = body.child(format!(
                        "Historical question: {}\nAnswer: {}\nStatus: {:?}\nEvidence at completion: {:?}\nProvider turn: {}",
                        turn.question,
                        turn.answer.as_deref().unwrap_or("No answer saved"),
                        turn.status,
                        turn.evidence_currentness,
                        spaced_identifier(turn.provider_turn_id.as_deref().unwrap_or("none"))
                    ));
                    if let Ok(evidence) = serde_json::from_str::<Vec<Evidence>>(&turn.evidence_json)
                    {
                        for (i, hit) in evidence.iter().enumerate() {
                            let saved = hit.clone();
                            body = body.child(
                                Button::new(format!("saved-evidence-{i}"))
                                    .label(saved_evidence_button_label(i, hit))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.selected_saved_evidence = Some(saved.clone());
                                        cx.notify();
                                    })),
                            );
                        }
                    }
                    if turn.status == brn_workflow::worker::OperationStatus::Completed
                        && turn.answer.is_some()
                        && let Some(state) = &self.draft_state
                    {
                        let turn_id = turn.operation_id;
                        let parent = self
                            .review
                            .as_ref()
                            .map(|r| r.id)
                            .unwrap_or(state.stamp().base_revision);
                        body = body
                            .child(format!(
                                "Candidate target: {} · parent revision {} · answer {}",
                                state.title(),
                                parent,
                                turn_id
                            ))
                            .child(
                                Button::new("save-candidate")
                                    .label("Save as candidate")
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.save_candidate(turn_id, cx)
                                    })),
                            );
                    }
                }
                if let Some(hit) = &self.selected_saved_evidence {
                    body = body.child(format!("Saved evidence snapshot (may be historical) · source {} · revision {} · bytes {}..{}\n{}", hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote));
                }
            }
            Page::Drafts => {
                let mut panel = div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .min_w(px(0.))
                    .gap_3();
                panel = panel
                    .child("Drafts · exact Markdown working copies")
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Input::new(&self.draft_title)
                                    .w(px(260.))
                                    .aria_label("New draft title"),
                            )
                            .child(
                                Button::new("create-draft")
                                    .label("Create blank draft")
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(|this, _, _, cx| this.create_draft(cx))),
                            )
                            .child(
                                Button::new("refresh-drafts")
                                    .label("Refresh list")
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.submit(Action::ListDrafts, "Draft list", cx);
                                    })),
                            ),
                    )
                    .child(format!("{} drafts", self.drafts.len()));
                for draft in &self.drafts {
                    let id = draft.id;
                    panel = panel.child(
                        Button::new(format!("draft-{id}"))
                            .label(format!(
                                "{} · {}…",
                                compact_title(&draft.title),
                                &id.to_string()[..8]
                            ))
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(move |this, _, _, cx| this.choose_draft(id, cx))),
                    );
                }
                if let Some(state) = &self.draft_state {
                    let status = if state.pending() {
                        "Saving snapshot; edits remain editable"
                    } else if state.dirty() {
                        "Unsaved changes"
                    } else {
                        "Saved"
                    };
                    let draft_id = state.id();
                    panel =
                        panel
                            .child(format!(
                                "Working copy: {} · {} · {} bytes · generation {}",
                                state.title(),
                                status,
                                state.text().len(),
                                state.generation()
                            ))
                            .child(
                                div()
                                    .id("draft-editor-anchor")
                                    .anchor_scroll(Some(self.draft_editor_anchor.clone()))
                                    .child(
                                        Editor::new(&self.draft_editor)
                                            .h(px(250.))
                                            .flex_shrink_0()
                                            .aria_label("Markdown working copy"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        Button::new("save-working-copy")
                                            .label("Save working copy")
                                            .disabled(
                                                !self.phase.can_submit()
                                                    || !state.dirty()
                                                    || state.text().len()
                                                        > brn_workflow::worker::MAX_DRAFT_BYTES,
                                            )
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.save_draft(false, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("save-checkpoint")
                                            .label("Save checkpoint")
                                            .disabled(
                                                !self.phase.can_submit()
                                                    || state.text().len()
                                                        > brn_workflow::worker::MAX_DRAFT_BYTES,
                                            )
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.save_draft(true, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("discard-draft-edits")
                                            .label("Discard edits")
                                            .disabled(!self.phase.can_submit() || !state.dirty())
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.discard_draft(window, cx)
                                            })),
                                    ),
                            )
                            .child(format!("Base checkpoint: {}", state.stamp().base_revision))
                            .child(format!("Revision history: {}", self.revisions.len()));
                    let mut comment_panel = div()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("Comments")
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    Button::new("capture-comment-selection")
                                        .label("Capture selection")
                                        .disabled(state.pending())
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.capture_comment(cx)),
                                        ),
                                )
                                .child(
                                    Button::new("refresh-comments")
                                        .label("Refresh comments")
                                        .disabled(!self.phase.can_submit())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.submit(
                                                Action::RefreshDraftComments { id: draft_id },
                                                "Refresh comments",
                                                cx,
                                            );
                                        })),
                                ),
                        );
                    if let Some(quote) = state.capture_quote() {
                        comment_panel = comment_panel.child("Captured exact quote:").child(
                            div()
                                .id("captured-quote-scroll")
                                .h(px(90.))
                                .min_w(px(0.))
                                .flex_shrink_0()
                                .border_1()
                                .overflow_y_scroll()
                                .child(
                                    div()
                                        .w_full()
                                        .font_family("Menlo")
                                        .flex()
                                        .flex_col()
                                        .children(
                                            quote
                                                .split('\n')
                                                .map(|line| div().child(line.to_owned())),
                                        ),
                                ),
                        );
                    } else {
                        comment_panel =
                            comment_panel.child("Select a passage in the editor, then capture it.");
                    }
                    comment_panel = comment_panel
                        .child(
                            Editor::new(&self.comment_input)
                                .h(px(95.))
                                .flex_shrink_0()
                                .aria_label("Comment body"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    Button::new("add-draft-comment")
                                        .label("Save checkpoint + add comment")
                                        .disabled(
                                            !self.phase.can_submit()
                                                || !state.can_add_comment(state.composer()),
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.add_comment(cx)),
                                        ),
                                )
                                .child(
                                    Button::new("discard-draft-comment")
                                        .label("Discard comment")
                                        .disabled(state.composer().is_empty() || state.pending())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.discard_comment(window, cx)
                                        })),
                                ),
                        );
                    let preview = state.preview_states();
                    if let Err(error) = &preview {
                        comment_panel =
                            comment_panel.child(format!("Preview unavailable: {error}"));
                    }
                    if state.comments().is_empty() {
                        comment_panel = comment_panel.child("No comments yet.");
                    }
                    for (index, view) in state.comments().iter().enumerate() {
                        let id = view.comment.id;
                        let anchor = preview.as_ref().ok().and_then(|states| states.get(index));
                        let location = match anchor {
                            Some(AnchorState::Anchored { start, end }) => {
                                format!("Anchored at bytes {start}..{end}")
                            }
                            Some(AnchorState::Deleted) => "Deleted passage".into(),
                            Some(AnchorState::Ambiguous { reason }) => {
                                format!("Location ambiguous: {reason:?}")
                            }
                            None => "Location unavailable".into(),
                        };
                        let toggle = if view.comment.status == CommentStatus::Open {
                            "Resolve"
                        } else {
                            "Reopen"
                        };
                        let mut card = div()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .border_1()
                            .child(format!(
                                "Comment {}… · {:?} · {location}",
                                &id.to_string()[..8],
                                view.comment.status
                            ))
                            .child("Original quote:")
                            .child(
                                div()
                                    .id(format!("quote-scroll-{id}"))
                                    .h(px(128.))
                                    .min_w(px(0.))
                                    .flex_shrink_0()
                                    .border_1()
                                    .overflow_y_scroll()
                                    .child(
                                        div()
                                            .w_full()
                                            .font_family("Menlo")
                                            .flex()
                                            .flex_col()
                                            .children(
                                                view.comment
                                                    .original_quote
                                                    .split('\n')
                                                    .map(|line| div().child(line.to_owned())),
                                            ),
                                    ),
                            )
                            .child("Comment body:")
                            .child(
                                div()
                                    .id(format!("comment-body-scroll-{id}"))
                                    .h(px(105.))
                                    .min_w(px(0.))
                                    .flex_shrink_0()
                                    .overflow_y_scroll()
                                    .child(
                                        div().w_full().flex().flex_col().children(
                                            view.comment
                                                .body
                                                .split('\n')
                                                .map(|line| div().child(line.to_owned())),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        Button::new(format!("show-passage-{id}"))
                                            .label("Show passage")
                                            .disabled(!matches!(
                                                anchor,
                                                Some(AnchorState::Anchored { .. })
                                            ))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.show_comment_passage(id, window, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new(format!("show-original-{id}"))
                                            .label("Show original revision")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.show_comment_original(id, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new(format!("toggle-comment-{id}"))
                                            .label(toggle)
                                            .disabled(!self.phase.can_submit())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.change_comment_status(id, cx)
                                            })),
                                    ),
                            );
                        if let Some(original) = &self.comment_original
                            && original.id == view.comment.original_revision_id
                        {
                            card = card
                                .child(format!("Original checkpoint {} · read-only", original.id))
                                .child(
                                    div()
                                        .id(format!("comment-original-scroll-{id}"))
                                        .h(px(170.))
                                        .min_w(px(0.))
                                        .flex_shrink_0()
                                        .border_1()
                                        .overflow_y_scroll()
                                        .child(
                                            div()
                                                .w_full()
                                                .font_family("Menlo")
                                                .flex()
                                                .flex_col()
                                                .children(
                                                    original
                                                        .text
                                                        .split('\n')
                                                        .map(|line| div().child(line.to_owned())),
                                                ),
                                        ),
                                );
                        }
                        comment_panel = comment_panel.child(card);
                    }
                    panel = panel.child(comment_panel);
                    for revision in &self.revisions {
                        let id = revision.id;
                        let label = format!(
                            "{:?} · {}…{}",
                            revision.kind,
                            &id.to_string()[..8],
                            revision.origin_turn.map_or(String::new(), |turn| format!(
                                " · answer {}…",
                                &turn.to_string()[..8]
                            ))
                        );
                        panel = panel.child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    Button::new(format!("review-{id}"))
                                        .label(format!("View {label}"))
                                        .disabled(!self.phase.can_submit())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.submit(
                                                Action::OpenDraftRevision {
                                                    draft: draft_id,
                                                    revision: id,
                                                },
                                                "Review revision",
                                                cx,
                                            );
                                        })),
                                )
                                .child(
                                    Button::new(format!("before-{id}"))
                                        .label("Use as before")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.compare_before = Some(id);
                                            this.diff = None;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new(format!("after-{id}"))
                                        .label("Use as after")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.compare_after = Some(id);
                                            this.diff = None;
                                            cx.notify();
                                        })),
                                ),
                        );
                    }
                    if let Some(revision) = &self.review {
                        panel = panel
                            .child(format!(
                                "Read-only {:?} {} · parent {} · origin answer {}",
                                revision.kind,
                                revision.id,
                                revision
                                    .parent_id
                                    .map_or("none".into(), |id| id.to_string()),
                                revision
                                    .origin_turn
                                    .map_or("none".into(), |id| id.to_string())
                            ))
                            .child(
                                div()
                                    .id("revision-content")
                                    .h(px(170.))
                                    .flex_shrink_0()
                                    .min_w(px(0.))
                                    .overflow_y_scroll()
                                    .border_1()
                                    .child(
                                        div()
                                            .w_full()
                                            .font_family("Menlo")
                                            .child(revision.text.clone()),
                                    ),
                            );
                    }
                    panel = panel
                        .child(format!(
                            "Compare: before {} · after {}",
                            self.compare_before
                                .map_or("none".into(), |id| id.to_string()),
                            self.compare_after
                                .map_or("none".into(), |id| id.to_string())
                        ))
                        .child(
                            Button::new("compare-revisions")
                                .label("Compare revisions")
                                .disabled(
                                    !self.phase.can_submit()
                                        || self.compare_before.is_none()
                                        || self.compare_after.is_none(),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let (Some(before), Some(after)) =
                                        (this.compare_before, this.compare_after)
                                    {
                                        this.submit(
                                            Action::CompareDraftRevisions {
                                                draft: draft_id,
                                                before,
                                                after,
                                            },
                                            "Compare revisions",
                                            cx,
                                        );
                                    }
                                })),
                        );
                    if let Some(diff) = &self.diff {
                        panel = panel.child(
                            div()
                                .id("revision-diff")
                                .h(px(220.))
                                .flex_shrink_0()
                                .min_w(px(0.))
                                .overflow_y_scroll()
                                .border_1()
                                .child(div().w_full().font_family("Menlo").child(diff.clone())),
                        );
                    }
                }
                body = body.child(panel);
            }
            Page::Settings => {
                body = body.child("Local settings")
                    .child(format!("Data directory: {}", spaced_identifier(&self.path.display().to_string())))
                    .child(format!("Codex executable: {}", self.config.codex.as_ref().map_or("not selected".into(), |p| spaced_identifier(&p.display().to_string()))))
                    .child(format!("Retrieval model directory: {}", self.config.model_dir.as_ref().map_or("not selected".into(), |p| spaced_identifier(&p.display().to_string()))))
                    .child("Model access uses the existing managed ChatGPT sign-in in Codex. Select an absolute executable with --codex before asking.");
            }
        }
        let status = match &self.phase {
            Phase::Opening => "Opening workspace…".into(),
            Phase::Idle => "Ready".into(),
            Phase::Running { label, since } => {
                format!("{label} · {}s elapsed", since.elapsed().as_secs())
            }
            Phase::Cancelling { label, since } => format!(
                "Cancelling {label} · {}s elapsed · awaiting safe stop",
                since.elapsed().as_secs()
            ),
            Phase::Failed(error) => {
                format!("Workspace failed to open · {}", compact_title(error))
            }
        };
        div()
            .id("brn-desktop")
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child("BRN · local grounded research")
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("workspace-page")
                            .label("Workspace")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Workspace;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("activity-page")
                            .label("Activity")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Activity;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("drafts-page")
                            .label("Drafts")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Drafts;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("notes-page")
                            .label("Notes")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Notes;
                                if let Some(state) = &this.note_state {
                                    this.note_observations.insert(state.id());
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("settings-page")
                            .label("Settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Settings;
                                cx.notify();
                            })),
                    ),
            )
            .child(status)
            .child(if matches!(self.phase, Phase::Failed(_)) {
                "Review details below".into()
            } else {
                compact_title(&self.message)
            })
            .child(
                Button::new("cancel")
                    .label("Cancel current action")
                    .disabled(!matches!(self.phase, Phase::Running { .. }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.worker.cancel() {
                            this.phase.cancel();
                            this.message = "Cancellation requested; awaiting safe stop.".into();
                            cx.notify();
                        }
                    })),
            )
            .child(body)
    }
}
pub fn run(path: PathBuf, config: Config) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None), KeyBinding::new("cmd-s", SaveNote, None)]);
            cx.set_menus([Menu::new("BRN").items(vec![
                MenuItem::action("Save to Markdown", SaveNote),
                MenuItem::action("Quit BRN", Quit),
            ])]);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.spawn(async move |cx| {
                let bounds = Bounds::new(point(px(80.), px(80.)), size(px(1100.), px(800.)));
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(800.), px(600.))),
                        ..Default::default()
                    },
                    |window, cx| {
                        let desktop = cx.new(|cx| Desktop::new(path, config, window, cx));
                        let quit_target = desktop.downgrade();
                        cx.on_action::<Quit>(move |_, cx| {
                            let allow = quit_target
                                .update(cx, |this, cx| this.close_guard(CloseRoute::Quit, cx))
                                .unwrap_or(true);
                            if allow {
                                cx.quit();
                            }
                        });
                        let save_target = desktop.downgrade();
                        cx.on_action::<SaveNote>(move |_, cx| {
                            let _ = save_target.update(cx, |this, cx| {
                                if this.page == Page::Notes {
                                    this.queue_note_control(NoteControl::Save, cx);
                                } else {
                                    this.message = "Cmd-S writes the open Markdown note on the Notes page; standalone drafts remain separate.".into();
                                    cx.notify();
                                }
                            });
                        });
                        let weak = desktop.downgrade();
                        window.on_window_should_close(cx, move |_, cx| {
                            weak.update(cx, |this, cx| this.close_guard(CloseRoute::Window, cx))
                                .unwrap_or(true)
                        });
                        cx.new(|cx| Root::new(desktop, window, cx))
                    },
                )
                .expect("failed to open BRN desktop window");
            })
            .detach();
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_blocks_actions_until_open_and_through_cancellation() {
        let mut phase = Phase::Opening;
        assert!(!phase.can_submit());
        phase.terminal(true, Some("database is newer"));
        assert!(matches!(phase, Phase::Failed(ref error) if error == "database is newer"));
        assert!(!phase.can_submit());
        phase = Phase::Opening;
        phase.terminal(true, None);
        assert!(phase.can_submit());
        phase = Phase::Running {
            label: "Answer".into(),
            since: Instant::now(),
        };
        assert!(phase.shows_live_answer());
        phase.cancel();
        assert!(matches!(phase, Phase::Cancelling { .. }));
        assert!(phase.shows_live_answer());
        assert!(!phase.can_submit());
        phase.terminal(false, Some("cancelled"));
        assert!(phase.can_submit());
        assert!(!phase.shows_live_answer());
    }
    #[test]
    fn evidence_rows_keep_quotes_out_of_fixed_height_buttons() {
        let source_id = Uuid::new_v4();
        let version_id = Uuid::new_v4();
        let hit = Evidence {
            source_id: source_id.to_string(),
            version_id: version_id.to_string(),
            source_hash: "hash".into(),
            start_byte: 12,
            end_byte: 47,
            quote: "first line\nsecond line".into(),
            passage_id: "passage".into(),
            generation: "generation".into(),
            score: 1.0,
            score_kind: "keyword".into(),
        };
        let source = SourceDocument {
            source_id,
            version_id,
            title: "Field\nnotes on the northern map".into(),
            origin: "/tmp/source.md".into(),
            bytes: Vec::new(),
            sha256: [0; 32],
            approval: Approval::Approved,
        };
        for label in [
            passage_button_label(0, &hit, &[source]),
            saved_evidence_button_label(0, &hit),
        ] {
            assert!(!label.contains('\n'));
            assert!(!label.contains("first line"));
            assert!(label.contains("12..47"));
        }
    }
    #[test]
    fn compact_title_keeps_long_multiline_turns_on_one_short_button_line() {
        let label = compact_title(
            "  A very long question about the northern map\nand every route to the cobalt lantern",
        );
        assert!(!label.contains('\n'));
        assert!(label.chars().count() <= 37);
        assert!(label.ends_with('…'));
    }
}
