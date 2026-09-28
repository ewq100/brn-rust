use crate::drafts::DraftEditor;
use brn_workflow::worker::{
    Action, Approval, ChatTurn, Draft, DraftRevision, Evidence, Outcome, Profile, SourceDocument,
    Worker,
};
use brn_workflow::{Config, SearchResult, SessionSummary};
use gpui_kit::{
    AppContext, Bounds, Context, Entity, KeyBinding, Menu, MenuItem, PathPromptOptions,
    Subscription, Task, Window, WindowBounds, WindowOptions,
    base::Disableable,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState},
    },
    div, point,
    prelude::*,
    px, size,
};
gpui_kit::actions!(brn, [Quit]);
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Workspace,
    Activity,
    Drafts,
    Settings,
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
    draft_state: Option<DraftEditor>,
    drafts: Vec<Draft>,
    revisions: Vec<DraftRevision>,
    review: Option<DraftRevision>,
    compare_before: Option<Uuid>,
    compare_after: Option<Uuid>,
    diff: Option<String>,
    pending_draft_job: Option<(u64, Uuid)>,
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
                        state.edit(editor.read(cx).value().to_string());
                    }
                    cx.notify();
                }
            });
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
        Self {
            worker: Worker::start(path.clone(), config.clone()),
            query,
            draft_title,
            draft_editor,
            draft_state: None,
            drafts: Vec::new(),
            revisions: Vec::new(),
            review: None,
            compare_before: None,
            compare_after: None,
            diff: None,
            pending_draft_job: None,
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
            _subscriptions: vec![query_subscription, draft_subscription, quit_subscription],
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
                        self.draft_editor.update(cx, |editor, cx| {
                            editor.set_value(draft.text.clone(), window, cx)
                        });
                        self.revisions.clear();
                        self.review = None;
                        self.diff = None;
                        self.compare_before = None;
                        self.compare_after = None;
                        self.message = "Draft opened. Working copy saved.".into();
                        self.submit(Action::ListDraftRevisions { id }, "Revision list", cx);
                    } else {
                        self.message = "Draft opened in storage, but newer edits remain in the current editor. Save or discard them before switching.".into();
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
            self.message = "Save the draft and wait for acknowledgement, or choose Discard edits, before switching or closing.".into();
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
            self.message = "Draft has unsaved edits or a save awaiting acknowledgement. Save or explicitly discard before closing.".into();
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
        self.submit(Action::OpenDraft { id }, "Open draft", cx);
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
        let action = if checkpoint {
            Action::CheckpointDraft {
                op: submitted.op,
                id: submitted.id,
                expected: submitted.expected,
                generation: submitted.generation,
                text: submitted.text.clone(),
            }
        } else {
            Action::SaveDraft {
                op: submitted.op,
                id: submitted.id,
                expected: submitted.expected,
                generation: submitted.generation,
                text: submitted.text.clone(),
            }
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
    fn quit_guarded(&mut self, cx: &mut Context<Self>) {
        if self.close_guard(cx) {
            cx.quit();
        }
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
            Page::Workspace => {
                body = body.child("Grounded workspace")
                    .child("Import a UTF-8 Markdown or text file and explicitly approve it for search.")
                    .child(div().flex().flex_wrap().gap_2()
                        .child(Button::new("choose-file").label(if self.choosing_file { "Choosing…" } else { "Choose file…" }).disabled(self.choosing_file || !self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| this.choose_file(cx))))
                        .child(Button::new("import-approved").label("Import and approve").disabled(!self.phase.can_submit() || self.import_path.is_none()).on_click(cx.listener(|this, _, _, cx| this.import(cx))))
                        .child(Button::new("build-index").label("Build index").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| { this.submit(Action::Build, "Index build", cx); })))
                        .child(Button::new("refresh").label("Refresh").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| { this.submit(Action::Refresh { session: this.selected_session }, "Refresh", cx); }))))
                    .child(div().min_w(px(0.)).child(format!("Selected file: {}", self.import_path.as_ref().map_or("none".into(), |p| spaced_identifier(&p.display().to_string())))));
                body = body.child(format!("Sources: {}", self.sources.len()));
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
                                "{} · {:?} · {} bytes",
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
                        "Selected passage · source {} · revision {} · bytes {}..{}\n{}",
                        hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote
                    ));
                }
                if self.phase.shows_live_answer() && !self.streamed_text.is_empty() {
                    let heading = if matches!(self.phase, Phase::Cancelling { .. }) {
                        "Partial answer while cancellation finishes (not saved)"
                    } else {
                        "Answer in progress (not saved)"
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
                                "{} · {} · {:?}",
                                compact_title(&turn.question),
                                turn.profile,
                                turn.status
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
                        "Question: {}\nAnswer: {}\nStatus: {:?}\nProvider turn: {}",
                        turn.question,
                        turn.answer.as_deref().unwrap_or("No answer saved"),
                        turn.status,
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
                                Editor::new(&self.draft_editor)
                                    .h(px(250.))
                                    .flex_shrink_0()
                                    .aria_label("Markdown working copy"),
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
            .on_action(cx.listener(|this, _: &Quit, _, cx| this.quit_guarded(cx)))
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
                        window_min_size: Some(size(px(800.), px(600.))),
                        ..Default::default()
                    },
                    |window, cx| {
                        let desktop = cx.new(|cx| Desktop::new(path, config, window, cx));
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
