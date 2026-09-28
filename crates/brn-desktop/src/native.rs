use brn_workflow::worker::{
    Action, Approval, ChatTurn, Evidence, Outcome, Profile, SourceDocument, Worker,
};
use brn_workflow::{Config, SearchResult, SessionSummary};
use gpui_kit::{
    AppContext, Bounds, Context, Entity, PathPromptOptions, Subscription, Task, Window,
    WindowBounds, WindowOptions,
    base::Disableable,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, InputEvent},
    },
    div, point,
    prelude::*,
    px, size,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Workspace,
    Activity,
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
        let query_subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.generation = this.generation.wrapping_add(1);
                this.search = None;
                this.selected_evidence = None;
                this.streamed_text.clear();
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
                if this.update_in(cx, |this, _, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        });
        Self {
            worker: Worker::start(path.clone(), config.clone()),
            query,
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
            _subscriptions: vec![query_subscription, quit_subscription],
            _poll_task: poll_task,
        }
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
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
    fn submit(&mut self, action: Action, label: &str, cx: &mut Context<Self>) {
        if !self.phase.can_submit() {
            return;
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
            }
            Err(error) => {
                if error == "workspace worker unavailable" {
                    self.phase = Phase::Failed(error.clone());
                }
                self.message = error;
            }
        }
        cx.notify();
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
                        .child(Button::new("build-index").label("Build index").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| this.submit(Action::Build, "Index build", cx))))
                        .child(Button::new("refresh").label("Refresh").disabled(!self.phase.can_submit()).on_click(cx.listener(|this, _, _, cx| this.submit(Action::Refresh { session: this.selected_session }, "Refresh", cx)))))
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
                                                )
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
                                                )
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
                }
                if let Some(hit) = &self.selected_saved_evidence {
                    body = body.child(format!("Saved evidence snapshot (may be historical) · source {} · revision {} · bytes {}..{}\n{}", hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote));
                }
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
