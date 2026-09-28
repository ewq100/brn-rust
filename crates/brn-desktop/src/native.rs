use brn_workflow::worker::{
    Action, Approval, ChatTurn, Evidence, Outcome, Profile, SourceDocument, Worker,
};
use brn_workflow::{Config, SearchResult, SessionSummary};
use gpui_kit::{
    AppContext, Context, Entity, Subscription, Task, Window, WindowOptions,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, InputEvent},
    },
    div,
    prelude::*,
    px,
};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Workspace,
    Activity,
    Settings,
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
struct Desktop {
    worker: Worker,
    query: Entity<EditorState>,
    import_path: Entity<EditorState>,
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
    last_update: u64,
    _subscriptions: Vec<Subscription>,
    _poll_task: Task<()>,
}
impl Desktop {
    fn new(path: PathBuf, config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let import_path = cx.new(|cx| EditorState::new(window, cx).default_value(""));
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
            import_path,
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
            match terminal.outcome {
                Err(error) => self.message = error,
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
        if changed {
            cx.notify();
        }
    }
    fn submit(&mut self, action: Action, label: &str, cx: &mut Context<Self>) {
        let generation = action.generation();
        match self.worker.submit(action) {
            Ok(id) => {
                self.active = Some(id);
                self.active_generation = generation;
                self.progress.clear();
                self.streamed_text.clear();
                self.message = format!("{label} started.");
            }
            Err(error) => self.message = error,
        }
        cx.notify();
    }
    fn import(&mut self, cx: &mut Context<Self>) {
        let path = self.import_path.read(cx).value().trim().to_string();
        if path.is_empty() {
            self.message = "Enter an absolute .md or .txt file path.".into();
            cx.notify();
            return;
        }
        let path = PathBuf::from(path);
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
            .gap_3()
            .p_3()
            .overflow_y_scroll();
        match self.page {
            Page::Workspace => {
                body = body.child("Grounded workspace")
                    .child("Import a UTF-8 Markdown or text file and explicitly approve it for search.")
                    .child(Editor::new(&self.import_path).h(px(56.)).aria_label("Absolute source file path"))
                    .child(Button::new("import-approved").label("Import and approve for search").on_click(cx.listener(|this, _, _, cx| this.import(cx))))
                    .child(div().flex().gap_2()
                        .child(Button::new("build-index").label("Build index").on_click(cx.listener(|this, _, _, cx| this.submit(Action::Build, "Index build", cx))))
                        .child(Button::new("refresh").label("Refresh").on_click(cx.listener(|this, _, _, cx| this.submit(Action::Refresh { session: this.selected_session }, "Refresh", cx)))));
                body = body.child(format!("Sources: {}", self.sources.len()));
                for source in &self.sources {
                    let source_id = source.source_id;
                    let version = source.version_id;
                    body = body.child(
                        div()
                            .flex()
                            .gap_2()
                            .child(format!(
                                "{} · {:?} · revision {} · {} bytes",
                                source.title,
                                source.approval,
                                version,
                                source.bytes.len()
                            ))
                            .child(
                                Button::new(format!("approve-{source_id}"))
                                    .label("Approve")
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
                    );
                }
                body = body
                    .child("Question")
                    .child(
                        Editor::new(&self.query)
                            .h(px(110.))
                            .aria_label("Question for approved sources"),
                    )
                    .child(
                        div()
                            .flex()
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
                            .gap_2()
                            .child(
                                Button::new("search")
                                    .label("Search")
                                    .on_click(cx.listener(|this, _, _, cx| this.search(cx))),
                            )
                            .child(
                                Button::new("ask")
                                    .label("Ask from sources")
                                    .on_click(cx.listener(|this, _, _, cx| this.ask(cx))),
                            )
                            .child(Button::new("cancel").label("Cancel").on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.message = if this.worker.cancel() {
                                        "Cancellation requested; awaiting safe stop."
                                    } else {
                                        "No running action."
                                    }
                                    .into();
                                    cx.notify();
                                },
                            ))),
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
                if !self.streamed_text.is_empty() {
                    body = body.child(format!("Answer in progress:\n{}", self.streamed_text));
                }
            }
            Page::Activity => {
                body = body
                    .child("Saved conversations")
                    .child(
                        Button::new("new-session")
                            .label("New session")
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
                                    "{} · {} turns{}",
                                    id,
                                    session.turns,
                                    if session.has_thread {
                                        " · provider linked"
                                    } else {
                                        " · recovery needed"
                                    }
                                ))
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
                                turn.question, turn.profile, turn.status
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
                        turn.provider_turn_id.as_deref().unwrap_or("none")
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
                    .child(format!("Data directory: {}", self.path.display()))
                    .child(format!("Codex executable: {}", self.config.codex.as_ref().map_or("not selected".into(), |p| p.display().to_string())))
                    .child(format!("Retrieval model directory: {}", self.config.model_dir.as_ref().map_or("not selected".into(), |p| p.display().to_string())))
                    .child("Model access uses the existing managed ChatGPT sign-in in Codex. Select an absolute executable with --codex before asking.");
            }
        }
        let status = if self.active.is_some() {
            format!("Working · {}", self.progress)
        } else {
            "Idle".into()
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
            .child(self.message.clone())
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
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let desktop = cx.new(|cx| Desktop::new(path, config, window, cx));
                    cx.new(|cx| Root::new(desktop, window, cx))
                })
                .expect("failed to open BRN desktop window");
            })
            .detach();
        });
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
