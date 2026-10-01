use crate::drafts::DraftEditor;
use crate::layout::{self, LayoutState, Loaded, ResolvedLayout};
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
        Root, TitleBar,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState},
        scroll::ScrollableElement,
    },
    div, point,
    prelude::*,
    px, size,
};
gpui_kit::actions!(brn, [Quit]);
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

mod shell;
mod theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocRef {
    Draft,
    Source(Uuid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CentreTab {
    Document,
    Chat,
}

/// Correlates a draft-open completion with the navigation that requested it,
/// so a late completion cannot override a newer Close or source selection.
#[derive(Debug, Default)]
struct DocNavigation {
    generation: u64,
    pending_draft: Option<u64>,
}

impl DocNavigation {
    fn moved(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
    fn request_draft(&mut self) {
        self.pending_draft = Some(self.generation);
    }
    fn take_draft_completion(&mut self) -> bool {
        self.pending_draft.take() == Some(self.generation)
    }
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
    path: PathBuf,
    config: Config,
    layout: LayoutState,
    system_dark: bool,
    layout_note: Option<String>,
    resolved: ResolvedLayout,
    open_doc: Option<DocRef>,
    centre_tab: CentreTab,
    nav: DocNavigation,
    settings_requested: bool,
    history_scroll: ScrollHandle,
    vault_scroll: ScrollHandle,
    chat_scroll: ScrollHandle,
    source_scroll: ScrollHandle,
    message: String,
    generation: u64,
    profile: Profile,
    selected_session: Option<Uuid>,
    sources: Vec<SourceDocument>,
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
        let (layout, loaded) = layout::load(&path);
        let resolved = layout.resolve(1100.0, false);
        let layout_note = match loaded {
            Loaded::Reset(note) => Some(note),
            Loaded::Missing | Loaded::Restored => None,
        };
        let system_dark = theme::system_dark(window);
        let theme_error = theme::apply(layout.appearance, window, cx).err();
        let layout_note = layout_note.or(theme_error);
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.system_dark = theme::system_dark(window);
            if let Err(error) = theme::apply(this.layout.appearance, window, cx) {
                this.message = error;
            }
            cx.notify();
        });
        Self {
            worker: Worker::start(path.clone(), config.clone()),
            query,
            draft_title,
            draft_editor,
            comment_input,
            draft_state: None,
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
            path,
            config,
            layout,
            system_dark,
            layout_note,
            resolved,
            open_doc: None,
            centre_tab: CentreTab::Chat,
            nav: DocNavigation::default(),
            settings_requested: false,
            history_scroll: ScrollHandle::new(),
            vault_scroll: ScrollHandle::new(),
            chat_scroll: ScrollHandle::new(),
            source_scroll: ScrollHandle::new(),
            message: "Opening workspace…".into(),
            generation: 0,
            profile: Profile::Keyword,
            selected_session: None,
            sources: Vec::new(),
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
                appearance_subscription,
            ],
            _poll_task: poll_task,
        }
    }
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
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
                    self.message = error;
                }
                Ok(Outcome::Ready {
                    sources,
                    sessions,
                    history,
                    selected,
                    recovered_operations,
                }) => {
                    self.sources = sources;
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
                Ok(Outcome::Imported { result, sources }) => {
                    self.sources = sources;
                    self.search = None;
                    self.selected_evidence = None;
                    self.generation = self.generation.wrapping_add(1);
                    self.message = if result.changed {
                        "Source imported and approved. Build the index to search it.".into()
                    } else {
                        "Source was already current; approval updated.".into()
                    };
                }
                Ok(Outcome::ApprovalChanged { sources }) => {
                    self.sources = sources;
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
                        if self.nav.take_draft_completion() {
                            self.show_document(DocRef::Draft);
                        }
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
                        if self.nav.take_draft_completion() {
                            self.show_document(DocRef::Draft);
                        }
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
            }
        }
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
    fn close_guard(&mut self, cx: &mut Context<Self>) -> bool {
        if self
            .draft_state
            .as_ref()
            .is_some_and(|state| !state.can_close())
        {
            self.message = "Draft edits, comment text, or a save are still pending. Save or discard them before closing.".into();
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
        self.nav.request_draft();
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
        self.nav.request_draft();
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
        self.centre_tab = CentreTab::Chat;
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
    fn show_document(&mut self, doc: DocRef) {
        self.nav.moved();
        self.open_doc = Some(doc);
        self.centre_tab = CentreTab::Document;
    }
    /// Hides the document pane. A draft's in-memory editor state is kept.
    fn close_document(&mut self) {
        self.nav.moved();
        self.open_doc = None;
        self.centre_tab = CentreTab::Chat;
    }
    fn open_draft_from_list(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self
            .draft_state
            .as_ref()
            .is_some_and(|state| state.id() == id)
        {
            self.show_document(DocRef::Draft);
            cx.notify();
            return;
        }
        self.choose_draft(id, cx);
    }
    fn cancel_running(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, Phase::Running { .. }) && self.worker.cancel() {
            self.phase.cancel();
            self.message = "Cancellation requested; awaiting safe stop.".into();
            cx.notify();
        }
    }
    fn phase_status(&self) -> String {
        match &self.phase {
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
        }
    }
}
impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_shell(window, cx)
    }
}
pub fn run(path: PathBuf, config: Config) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.set_menus([Menu::new("BRN").items(vec![MenuItem::action("Quit BRN", Quit)])]);
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
                        window_min_size: Some(size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN))),
                        ..TitleBar::window_options()
                    },
                    |window, cx| {
                        let desktop = cx.new(|cx| Desktop::new(path, config, window, cx));
                        let quit_target = desktop.downgrade();
                        cx.on_action::<Quit>(move |_, cx| {
                            let allow = quit_target
                                .update(cx, |this, cx| this.close_guard(cx))
                                .unwrap_or(true);
                            if allow {
                                cx.quit();
                            }
                        });
                        let weak = desktop.downgrade();
                        window.on_window_should_close(cx, move |_, cx| {
                            weak.update(cx, |this, cx| this.close_guard(cx))
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
    fn approval_tags_pair_text_with_state() {
        assert_eq!(shell::approval_tag(Approval::Approved), "✓ approved");
        assert_eq!(shell::approval_tag(Approval::Draft), "○ not approved");
        assert_eq!(shell::approval_tag(Approval::Withdrawn), "– withdrawn");
    }

    #[test]
    fn draft_completion_navigates_only_without_newer_navigation() {
        let mut nav = DocNavigation::default();
        assert!(!nav.take_draft_completion());
        nav.request_draft();
        assert!(nav.take_draft_completion());
        assert!(!nav.take_draft_completion());
        nav.request_draft();
        nav.moved();
        assert!(!nav.take_draft_completion());
    }

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
