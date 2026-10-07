use super::theme::color;
use super::*;
use crate::ai::{Pending, provider_name, scope_name, session_activity_label, slot, turn_label};
use brn_workflow::{
    Provider, ReasoningEffort, Selection,
    app_worker::{AppCommand, AppEvent},
    chat_worker::AccountCommand,
    knowledge::{CitationOutcome, NoteProvenance},
    library::KnowledgeScope,
};
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, WindowExt, link::Link},
};

pub(super) struct Closed {
    result: brn_workflow::Result<()>,
    events: Vec<(Uuid, AppEvent)>,
}
pub(super) enum EditorTransition {
    Note(String),
    Evidence { path: String, scope: KnowledgeScope },
    Review(Uuid),
    Activity,
    Dashboard,
    Findings,
    Finding { analysis: Uuid, id: Uuid },
    Inbox,
    AnalyzeInboxSource(String),
    InboxSourceDraft,
    Draft(Option<Uuid>),
    ActionDraft(Option<Uuid>),
    Hide,
    Close(CloseRoute),
}

pub(super) fn final_quit(
    app_shutdown: impl FnOnce(),
    preferences: impl std::future::Future<Output = ()>,
) -> impl std::future::Future<Output = ()> {
    // Drain admitted work before GPUI starts its quit-future deadline. This
    // defensive system hook submits no latest-typing flush; guarded close
    // routes flush and join asynchronously before this hook is reached.
    app_shutdown();
    async move {
        preferences.await;
    }
}

fn project_notice(state: &crate::ai::AiState, message: &mut String) -> bool {
    if *message == state.notice {
        return false;
    }
    *message = state.notice.clone();
    true
}

fn ask_command(state: &mut crate::ai::AiState, question: String) -> Option<(Uuid, AppCommand)> {
    let request = state.ask(question)?;
    Some((request.id, AppCommand::Ask(request)))
}
fn search_command(state: &mut crate::ai::AiState, query: String) -> Option<(Uuid, AppCommand)> {
    state.search_notes(query)
}

fn login_prompt_is_current(
    desktop: &Desktop,
    id: Uuid,
    prompt: &brn_workflow::LoginPrompt,
) -> bool {
    desktop.login_dialog == Some(id)
        && desktop
            .ai
            .as_ref()
            .and_then(|ai| ai.login.as_ref())
            .filter(|login| login.operation == id)
            .and_then(|login| login.prompt.as_ref())
            .is_some_and(|current| {
                current.verification_uri == prompt.verification_uri
                    && current.user_code == prompt.user_code
            })
}

pub(super) fn evidence_widget(editor: &Entity<EditorState>) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label("Full exact saved evidence")
}
pub(super) fn provenance_quote_widget(editor: &Entity<EditorState>) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label("Stored exact source quote")
}
pub(super) fn provenance_copy_button(index: usize, quote: &str) -> Button {
    let quote = quote.to_owned();
    Button::new(format!("copy-source-quote-{index}"))
        .label("Copy exact quote")
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(quote.clone()));
        })
}
fn citation_status(outcome: CitationOutcome) -> &'static str {
    match outcome {
        CitationOutcome::Matched => "Matched · exact saved source",
        CitationOutcome::Changed => "Changed · source differs from the cited version",
        CitationOutcome::Absent => "Absent · source identity was not found",
        CitationOutcome::Ambiguous => "Ambiguous · multiple files have this source identity",
        CitationOutcome::Incomplete => "Incomplete · source lookup could not be completed",
    }
}
fn download_command(
    state: &mut crate::ai::AiState,
    consent: bool,
    target: PathBuf,
) -> (Uuid, AppCommand) {
    state.model_prompt = None;
    let command = state.command(
        Pending::Download,
        AppCommand::DownloadModel { consent, target },
    );
    state.download = Some(command.0);
    state.download_stopping = false;
    state.model_state = if consent {
        "Downloading (not installed)"
    } else {
        "Recording decline; no network"
    }
    .into();
    command
}

impl Desktop {
    pub(super) fn simple_send(&mut self, command: (Uuid, AppCommand), cx: &mut Context<Self>) {
        let (id, command) = command;
        let result = self.app_worker.as_ref().map_or_else(
            || {
                Err(brn_workflow::WorkflowError {
                    kind: brn_workflow::ErrorKind::Cancelled,
                    message: "Application lane is closing".into(),
                })
            },
            |worker| worker.submit(id, command),
        );
        if let Err(error) = result
            && let Some(ai) = &mut self.ai
        {
            if self
                .review_comment_pending
                .as_ref()
                .is_some_and(|(operation, _, _)| *operation == id)
            {
                self.review_comment_pending = None;
            }
            let followups = ai.apply(id, AppEvent::Failed(error));
            for command in followups {
                self.simple_send(command, cx);
            }
        }
        cx.notify();
    }
    pub(super) fn simple_command(
        &mut self,
        pending: Pending,
        command: AppCommand,
        cx: &mut Context<Self>,
    ) {
        if self.closing.is_some() || self.closed {
            return;
        }
        let command = self.ai.as_mut().unwrap().command(pending, command);
        self.simple_send(command, cx);
    }
    pub(super) fn poll_simple(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut events = Vec::new();
        if let Some(worker) = &self.app_worker {
            while let Some(event) = worker.try_event() {
                events.push(event);
            }
        }
        let changed = !events.is_empty();
        for (id, event) in events {
            self.settle_comment_draft(id, &event, window, cx);
            if matches!(
                self.ai.as_ref().unwrap().pending.get(&id),
                Some(Pending::EditorReload)
            ) && matches!(&event, AppEvent::EditorRecovered(_) | AppEvent::Failed(_))
            {
                self.simple_editor_generation = None;
            }
            let commands = self.ai.as_mut().unwrap().apply(id, event);
            for command in commands {
                self.simple_send(command, cx);
            }
        }
        if changed {
            self.sync_review_widgets(window, cx);
        }
        self.sync_draft_widgets(window, cx);
        self.sync_provenance_widgets(window, cx);
        self.sync_relationship_widgets(window, cx);
        self.sync_finding_widgets(window, cx);
        self.sync_inbox_widgets(window, cx);
        self.sync_dashboard_widgets(window, cx);
        if self
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .is_some_and(|review| {
                review.wants_recovery(Instant::now(), self.simple_transition.is_some())
            })
            && let Some(command) = self.ai.as_mut().unwrap().recover_review()
        {
            self.simple_send(command, cx);
        }
        // Cancellation can precede lane admission, including before the first delta.
        for command in self.ai.as_ref().unwrap().stop_controls() {
            self.simple_send(command, cx);
        }
        if self.ai.as_ref().unwrap().editor.is_some()
            && self.simple_editor_generation != Some(self.ai.as_ref().unwrap().note_generation)
        {
            let text = self
                .ai
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .text
                .clone();
            self.note_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
            self.simple_editor_generation = Some(self.ai.as_ref().unwrap().note_generation);
        }
        if let Some(note) = self
            .ai
            .as_ref()
            .unwrap()
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.note.as_ref())
            && self.evidence_editor_generation != Some(self.ai.as_ref().unwrap().note_generation)
        {
            let text = note.text.clone();
            self.evidence_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
            self.evidence_editor_generation = Some(self.ai.as_ref().unwrap().note_generation);
        }
        if self
            .ai
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .is_some_and(|editor| {
                editor.wants_recovery(Instant::now(), self.simple_transition.is_some())
            })
            && let Some(command) = self.ai.as_mut().unwrap().recover_editor()
        {
            self.simple_send(command, cx);
        }
        self.simple_progress_transition(cx);
        let ai = self.ai.as_ref().unwrap();
        let notice_changed = project_notice(ai, &mut self.message);
        if let Some(id) = self.login_dialog
            && ai.login.as_ref().is_none_or(|login| login.operation != id)
        {
            self.login_dialog = None;
            window.close_dialog(cx);
        }
        if self.ai.as_ref().unwrap().model_prompt.is_some() && !window.has_active_dialog(cx) {
            self.open_model_consent(window, cx);
        }
        if changed || notice_changed {
            cx.notify();
        }
    }
    pub(super) fn simple_ask(&mut self, cx: &mut Context<Self>) {
        let question = self.query(cx);
        if let Some(command) = ask_command(self.ai.as_mut().unwrap(), question) {
            self.simple_send(command, cx);
        }
    }
    pub(super) fn simple_stop(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.ai.as_mut().unwrap().stop_rewrite() {
            self.simple_send((Uuid::new_v4(), AppCommand::CancelTurn(id)), cx);
        }
        if let Some(id) = self.ai.as_mut().unwrap().stop() {
            self.simple_send((Uuid::new_v4(), AppCommand::CancelTurn(id)), cx);
        }
    }
    pub(super) fn simple_search(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
        if let Some(command) = search_command(self.ai.as_mut().unwrap(), query) {
            self.simple_send(command, cx);
        }
    }
    pub(super) fn simple_history(&mut self, conversation: Option<Uuid>, cx: &mut Context<Self>) {
        self.centre_tab = CentreTab::Chat;
        if let Some(command) = self.ai.as_mut().unwrap().navigate(conversation) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn simple_note(&mut self, path: String, cx: &mut Context<Self>) {
        if self.open_doc == Some(DocRef::SavedNote)
            && self.simple_note_path.as_deref() == Some(&path)
            && self.ai.as_ref().unwrap().editor.is_some()
        {
            self.centre_tab = CentreTab::Document;
            cx.notify();
            return;
        }
        self.simple_leave(EditorTransition::Note(path), cx);
    }
    pub(super) fn simple_scoped_note(
        &mut self,
        path: String,
        scope: KnowledgeScope,
        cx: &mut Context<Self>,
    ) {
        if scope == KnowledgeScope::Current {
            self.simple_note(path, cx);
        } else {
            self.simple_leave(EditorTransition::Evidence { path, scope }, cx);
        }
    }
    fn simple_open_note(&mut self, path: String, cx: &mut Context<Self>) {
        self.clear_saved_link_panel();
        self.reset_provenance_panel();
        let ai = self.ai.as_mut().unwrap();
        ai.review = None;
        ai.review_generation = ai.review_generation.wrapping_add(1);
        let command = self.ai.as_mut().unwrap().open_editor(path.clone());
        self.simple_note_path = Some(path.clone());
        self.note_scroll.set_offset(point(px(0.), px(0.)));
        self.open_doc = Some(DocRef::SavedNote);
        self.centre_tab = CentreTab::Document;
        self.simple_send(command, cx);
    }
    pub(super) fn simple_leave(&mut self, transition: EditorTransition, cx: &mut Context<Self>) {
        if self.closing.is_some() || self.closed {
            return;
        }
        self.capture_draft_widgets(cx);
        if let Some(draft) = &self.ai.as_ref().unwrap().draft
            && !draft.can_leave()
            && !(draft.pending && matches!(transition, EditorTransition::Close(_)))
        {
            self.ai.as_mut().unwrap().notice = "Initial proposal input is retained. Create its exact review draft, or copy and explicitly discard it before leaving.".into();
            cx.notify();
            return;
        }
        self.simple_transition = Some(transition);
        self.simple_progress_transition(cx);
        cx.notify();
    }
    pub(super) fn simple_progress_transition(&mut self, cx: &mut Context<Self>) {
        if self.simple_transition.is_none() {
            return;
        }
        if self.review_comment_draft.is_some() && !self.review_comment.read(cx).value().is_empty() {
            self.ai.as_mut().unwrap().notice = "A comment draft is not acknowledged. Copy or explicitly discard it before leaving.".into();
            return;
        }
        if !self.ai.as_ref().unwrap().review_can_leave() {
            self.ai.as_mut().unwrap().notice = if self
                .ai
                .as_ref()
                .unwrap()
                .draft
                .as_ref()
                .is_some_and(|draft| !draft.can_leave())
            {
                "Waiting for initial proposal acknowledgement or retained input resolution before leaving. Copy or explicitly discard later input after the request settles."
            } else {
                "Waiting for latest full review acknowledgement before leaving. Copy retained text or resolve the review error."
            }.into();
            return;
        }
        if self
            .ai
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .is_some_and(|editor| !editor.can_leave())
        {
            self.ai.as_mut().unwrap().notice = "Waiting for latest buffer recovery before leaving. Closing does not save Markdown.".into();
            return;
        }
        let transition = self.simple_transition.take().unwrap();
        if let EditorTransition::Finding { analysis, id } = &transition
            && self
                .ai
                .as_ref()
                .unwrap()
                .inbox_analysis_finding(*analysis, *id)
                .is_none()
        {
            self.ai.as_mut().unwrap().notice = "This conflict no longer belongs to the selected analysis. Refresh the retained analysis before opening Needs Review.".into();
            return;
        }
        let finding = match &transition {
            EditorTransition::Finding { id, .. } => Some(*id),
            _ => None,
        };
        if !matches!(
            &transition,
            EditorTransition::Findings
                | EditorTransition::Finding { .. }
                | EditorTransition::Draft(_)
                | EditorTransition::ActionDraft(_)
        ) {
            self.ai.as_mut().unwrap().close_findings();
        }
        if !matches!(&transition, EditorTransition::Dashboard) {
            self.ai.as_mut().unwrap().close_dashboard();
        }
        if !matches!(
            &transition,
            EditorTransition::Inbox
                | EditorTransition::AnalyzeInboxSource(_)
                | EditorTransition::InboxSourceDraft
                | EditorTransition::Draft(_)
                | EditorTransition::ActionDraft(_)
        ) {
            self.ai.as_mut().unwrap().close_inbox();
        }
        let action_draft = matches!(&transition, EditorTransition::ActionDraft(_));
        let analysis_path = match &transition {
            EditorTransition::AnalyzeInboxSource(path) => Some(path.clone()),
            _ => None,
        };
        match transition {
            EditorTransition::Note(path) => self.simple_open_note(path, cx),
            EditorTransition::Evidence { path, scope } => {
                self.clear_saved_link_panel();
                self.reset_provenance_panel();
                let ai = self.ai.as_mut().unwrap();
                ai.review = None;
                ai.review_generation = ai.review_generation.wrapping_add(1);
                let command = ai.open_evidence(path.clone(), scope);
                self.simple_note_path = Some(path);
                self.open_doc = Some(DocRef::Evidence);
                self.centre_tab = CentreTab::Document;
                self.simple_send(command, cx);
            }
            EditorTransition::Review(id) => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                self.simple_note_path = None;
                self.review_member = 0;
                self.open_doc = Some(DocRef::Proposal(id));
                self.centre_tab = CentreTab::Document;
                if let Some(command) = ai.open_review(id) {
                    self.simple_send(command, cx);
                }
            }
            EditorTransition::Dashboard => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.open_doc = Some(DocRef::Dashboard);
                self.centre_tab = CentreTab::Document;
                if let Some(command) = ai.open_dashboard() {
                    self.simple_send(command, cx);
                }
            }
            EditorTransition::Activity => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.open_doc = Some(DocRef::Activity);
                self.centre_tab = CentreTab::Document;
                let commands = [ai.refresh_activity(), ai.refresh_applies()];
                for command in commands.into_iter().flatten() {
                    self.simple_send(command, cx);
                }
            }
            EditorTransition::Findings | EditorTransition::Finding { .. } => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.open_doc = Some(DocRef::Findings);
                self.centre_tab = CentreTab::Document;
                let page = ai.open_findings();
                let selected = finding.and_then(|id| ai.select_finding(id));
                for command in [page, selected].into_iter().flatten() {
                    self.simple_send(command, cx);
                }
            }
            EditorTransition::Inbox | EditorTransition::AnalyzeInboxSource(_) => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.open_doc = Some(DocRef::Inbox);
                self.centre_tab = CentreTab::Document;
                if let Some(command) = ai.open_inbox() {
                    self.simple_send(command, cx);
                }
                if let Some(path) = analysis_path {
                    self.inbox.analysis_path_target = Some(path.clone());
                    if let Some(command) = self
                        .ai
                        .as_mut()
                        .unwrap()
                        .inspect_inbox_analysis_source(path)
                    {
                        self.simple_send(command, cx);
                    }
                }
            }
            EditorTransition::InboxSourceDraft => {
                if !self.ai.as_mut().unwrap().open_inbox_source_draft() {
                    cx.notify();
                    return;
                }
                self.ai.as_mut().unwrap().close_inbox();
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.draft_widget_id = None;
                self.open_doc = Some(DocRef::Draft);
                self.centre_tab = CentreTab::Document;
            }
            EditorTransition::Draft(turn) | EditorTransition::ActionDraft(turn) => {
                let started = if action_draft {
                    self.ai.as_mut().unwrap().begin_action_draft(turn)
                } else {
                    self.ai.as_mut().unwrap().begin_draft(turn)
                };
                if !started {
                    cx.notify();
                    return;
                }
                self.ai.as_mut().unwrap().close_findings();
                self.ai.as_mut().unwrap().close_inbox();
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.review_generation = ai.review_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                self.simple_note_path = None;
                self.draft_widget_id = None;
                self.open_doc = Some(DocRef::Draft);
                self.centre_tab = CentreTab::Document;
            }
            EditorTransition::Hide => {
                self.clear_saved_link_panel();
                self.clear_saved_sources();
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.editor = None;
                ai.evidence = None;
                ai.review = None;
                ai.review_generation = ai.review_generation.wrapping_add(1);
                self.simple_note_path = None;
                self.open_doc = None;
                self.centre_tab = CentreTab::Chat;
            }
            EditorTransition::Close(route) => self.begin_close(route, cx),
        }
    }
    fn reset_provenance_panel(&mut self) {
        self.provenance_open = false;
        self.provenance_snapshot = None;
        self.provenance_quotes.clear();
        self.provenance_scroll.set_offset(point(px(0.), px(0.)));
    }
    pub(super) fn clear_saved_sources(&mut self) {
        self.ai.as_mut().unwrap().clear_provenance();
        self.reset_provenance_panel();
    }
    pub(super) fn inspect_saved_sources(&mut self, cx: &mut Context<Self>) {
        if self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || !matches!(self.open_doc, Some(DocRef::SavedNote | DocRef::Evidence))
            || !self.ai.as_ref().unwrap().ready
            || !self.ai.as_ref().unwrap().vault_bound
            || self.ai.as_ref().unwrap().application_busy()
        {
            return;
        }
        self.provenance_open = true;
        if let Some(command) = self.ai.as_mut().unwrap().inspect_provenance() {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn sync_provenance_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ai = self.ai.as_ref().unwrap();
        if ai.provenance.is_none() && ai.provenance_loading() {
            return;
        }
        if self.provenance_snapshot.as_ref() == ai.provenance.as_ref() {
            return;
        }
        let next: Option<NoteProvenance> = ai.provenance.clone();
        self.provenance_quotes = next.as_ref().map_or_else(Vec::new, |provenance| {
            provenance
                .citations
                .iter()
                .map(|resolved| {
                    cx.new(|cx| {
                        EditorState::new(window, cx).default_value(resolved.citation.quote.clone())
                    })
                })
                .collect()
        });
        self.provenance_snapshot = next;
        self.provenance_scroll.set_offset(point(px(0.), px(0.)));
    }
    fn render_saved_sources(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.provenance_open {
            return None;
        }
        let ai = self.ai.as_ref().unwrap();
        let p = self.palette();
        let mut content = div()
            .id("saved-sources-scroll")
            .overflow_y_scroll()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .track_scroll(&self.provenance_scroll)
            .vertical_scrollbar(&self.provenance_scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_3()
            .p_2();
        if ai.provenance_loading() {
            content = content.child("Inspecting saved sources…");
        } else if let Some(error) = &ai.provenance_error {
            content = content.child(format!("Saved sources unavailable: {error}"));
        } else if let Some(provenance) = &ai.provenance {
            if provenance.inbox_source.is_some() {
                let path = ai
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.path.clone())
                    .or_else(|| {
                        ai.editor
                            .as_ref()
                            .map(|editor| editor.view.record.path.clone())
                    });
                if let Some(path) = path {
                    let generation = ai.note_generation;
                    content = content.child(
                        Button::new("analyze-saved-inbox-source")
                            .label("Inspect this Source for analysis")
                            .disabled(
                                self.simple_transition.is_some()
                                    || self.closing.is_some()
                                    || self.closed
                                    || self.close_failed
                                    || ai.application_busy(),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let ai = this.ai.as_ref().unwrap();
                                let current_path = ai
                                    .evidence
                                    .as_ref()
                                    .map(|evidence| &evidence.path)
                                    .or_else(|| {
                                        ai.editor.as_ref().map(|editor| &editor.view.record.path)
                                    });
                                if this.simple_transition.is_none()
                                    && this.closing.is_none()
                                    && !this.closed
                                    && !this.close_failed
                                    && ai.note_generation == generation
                                    && current_path == Some(&path)
                                    && ai
                                        .provenance
                                        .as_ref()
                                        .is_some_and(|proof| proof.inbox_source.is_some())
                                {
                                    this.simple_leave(
                                        EditorTransition::AnalyzeInboxSource(path.clone()),
                                        cx,
                                    );
                                }
                            })),
                    );
                }
            }
            if provenance.citations.is_empty() {
                content = content.child("This saved note has no source citations.");
            }
            for (index, resolved) in provenance.citations.iter().enumerate() {
                let mut source = div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap_1()
                    .child(citation_status(resolved.outcome))
                    .child(format!("Source UUID: {}", resolved.citation.note_id))
                    .child(format!(
                        "Cited byte range: {}–{}",
                        resolved.citation.start_byte, resolved.citation.end_byte
                    ));
                for matched in &resolved.matches {
                    source = source.child(format!("Observed path: {}", matched.path));
                }
                for issue in &resolved.issues {
                    source = source.child(format!(
                        "Inspection issue: {} · {}",
                        issue.path, issue.reason
                    ));
                }
                if let Some(quote) = self.provenance_quotes.get(index) {
                    source = source.child(
                        div()
                            .h(px(96.))
                            .flex_shrink_0()
                            .child(provenance_quote_widget(quote)),
                    );
                }
                content = content
                    .child(source.child(provenance_copy_button(index, &resolved.citation.quote)));
            }
        } else {
            content = content.child("Select Refresh to inspect this note's saved sources.");
        }
        Some(
            div()
                .h(px(260.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(color(p.line))
                .bg(color(p.panel))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .p_2()
                        .child(div().flex_1().child("Saved sources"))
                        .child(
                            Button::new("refresh-saved-sources")
                                .label("Refresh")
                                .compact()
                                .disabled(
                                    self.simple_transition.is_some()
                                        || self.closing.is_some()
                                        || self.closed
                                        || ai.application_busy()
                                        || ai.provenance_loading(),
                                )
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.inspect_saved_sources(cx)),
                                ),
                        )
                        .child(
                            Button::new("close-saved-sources")
                                .label("Close sources")
                                .compact()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.clear_saved_sources();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .px_2()
                        .child("Saved Markdown only; unsaved edits are not included."),
                )
                .child(content.test_support())
                .into_any_element(),
        )
    }
    pub(super) fn simple_save(&mut self, destination: Option<String>, cx: &mut Context<Self>) {
        if self.simple_transition.is_some() || self.closing.is_some() || self.closed {
            return;
        }
        if let Some(command) = self.ai.as_mut().unwrap().save_editor(destination) {
            self.simple_send(command, cx);
        }
    }
    fn simple_compare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.ai.as_ref().unwrap().editor.as_ref() else {
            return;
        };
        let baseline = editor.view.record.baseline_text.clone();
        let local = editor.text.clone();
        let disk = editor
            .view
            .saved
            .clone()
            .unwrap_or("Disk unavailable".into());
        window.open_dialog(cx, move |dialog, _, _| {
            let mut body = div().flex().flex_col().gap_2();
            for (label, text) in [
                ("Editing baseline", &baseline),
                ("Local buffer", &local),
                ("Observed disk", &disk),
            ] {
                body = body.child(label).child(
                    div()
                        .id(label)
                        .max_h(px(150.))
                        .overflow_y_scroll()
                        .child(text.clone()),
                );
            }
            dialog
                .title("Compare note versions")
                .w(px(640.))
                .child(body)
        });
    }
    fn simple_confirm_reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.ai.as_ref().unwrap().editor.as_ref() else {
            return;
        };
        let Some(request) = editor.reload_request() else {
            return;
        };
        let reviewed = editor.view.saved.clone().unwrap_or_default();
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let target = desktop.clone();
            let request = request.clone();
            dialog.title("Reload reviewed disk text?").w(px(640.))
                .child(div().flex().flex_col().gap_2()
                    .child("This replaces the current local buffer with the exact disk version shown below. Markdown is not changed.")
                    .child(div().id("reviewed-disk").max_h(px(250.)).overflow_y_scroll().child(reviewed.clone()))
                    .child(Button::new("confirm-simple-reload").label("Confirm reload / discard local buffer")
                        .on_click(move |_, window, cx| {
                            let _ = target.update(cx, |this, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().reload_editor(request.clone()) {
                                    this.simple_send(command, cx);
                                } else {
                                    this.ai.as_mut().unwrap().notice = "Editor changed while confirmation was open; review the disk version again.".into();
                                    cx.notify();
                                }
                            });
                            window.close_dialog(cx);
                        })))
        });
    }
    pub(super) fn simple_choose_vault(&mut self, cx: &mut Context<Self>) {
        if self.choosing_file || !self.ai.as_ref().unwrap().ready {
            return;
        }
        self.choosing_file = true;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose Markdown vault".into()),
        });
        cx.spawn(async move |this, cx| {
            let selection = receiver.await;
            let _ = this.update(cx, |this, cx| {
                this.choosing_file = false;
                match selection {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            this.simple_command(Pending::Bind, AppCommand::BindVault(path), cx);
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.ai.as_mut().unwrap().notice = "Vault chooser failed or closed.".into()
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn simple_account(
        &mut self,
        command: AccountCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let connect = matches!(command, AccountCommand::Connect(_));
        if let Some(command) = self.ai.as_mut().unwrap().account(command) {
            let id = command.0;
            self.simple_send(command, cx);
            if connect && self.ai.as_ref().unwrap().login.is_some() {
                window.close_dialog(cx);
                self.open_login(id, window, cx);
            }
        }
    }
    pub(super) fn open_login(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        self.login_dialog = Some(id);
        let desktop = cx.entity();
        // Dialog-local state preserves selection across redraws without saving codes.
        let code_input = cx.new(|cx| InputState::new(window, cx));
        window.open_dialog(cx, move |dialog, window, cx| {
            let target = desktop.downgrade();
            let cancel_target = target.clone();
            let prompt = desktop
                .read(cx)
                .ai
                .as_ref()
                .and_then(|ai| ai.login.as_ref())
                .filter(|login| login.operation == id)
                .and_then(|login| login.prompt.clone());
            let code = prompt
                .as_ref()
                .map_or("", |prompt| prompt.user_code.as_str());
            if code_input.read(cx).value().as_ref() != code {
                code_input.update(cx, |input, cx| input.set_value(code.to_owned(), window, cx));
            }
            let mut body = div()
                .flex()
                .flex_col()
                .gap_3()
                .child("Explicit connection · waiting for authorization");
            if let Some(prompt) = prompt {
                let link_target = target.clone();
                let link_prompt = prompt.clone();
                let copy_target = target.clone();
                body = body
                    .child(
                        div().id("login-verification-url").test_support().child(
                            Link::new("login-verification-link")
                                .child(prompt.verification_uri.clone())
                                .on_click(move |_, _, cx| {
                                    if link_target.upgrade().is_some_and(|desktop| {
                                        login_prompt_is_current(desktop.read(cx), id, &link_prompt)
                                    }) {
                                        cx.open_url(&link_prompt.verification_uri);
                                    }
                                }),
                        ),
                    )
                    .child(
                        Input::new(&code_input)
                            .id("login-code")
                            .readonly(true)
                            .aria_label("Device authorization code"),
                    )
                    .child(Button::new("copy-login-code").label("Copy code").on_click(
                        move |_, _, cx| {
                            if copy_target.upgrade().is_some_and(|desktop| {
                                login_prompt_is_current(desktop.read(cx), id, &prompt)
                            }) {
                                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                    prompt.user_code.clone(),
                                ));
                            }
                        },
                    ));
            }
            body = body.child(
                Button::new("cancel-login")
                    .label("Cancel connection")
                    .on_click(move |_, window, cx| {
                        let _ = cancel_target.update(cx, |this, cx| this.cancel_login(id, cx));
                        window.close_dialog(cx);
                    }),
            );
            dialog
                .title("Connect account")
                .w(px(460.))
                .child(body)
                .on_close(move |_, _, cx| {
                    let _ = target.update(cx, |this, cx| this.cancel_login(id, cx));
                })
        });
    }
    fn cancel_login(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self
            .ai
            .as_ref()
            .and_then(|ai| ai.login.as_ref())
            .is_some_and(|l| l.operation == id)
            && let Some(id) = self.ai.as_mut().unwrap().dismiss_login()
        {
            self.simple_send((Uuid::new_v4(), AppCommand::CancelAccount(id)), cx);
        }
        if self.login_dialog == Some(id) {
            self.login_dialog = None;
        }
    }
    pub(super) fn open_model_consent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
        }
        let desktop = cx.entity();
        let prompt = self
            .ai
            .as_ref()
            .unwrap()
            .model_prompt
            .clone()
            .unwrap_or_else(|| brn_workflow::models::ModelDownloadPrompt {
                source: brn_workflow::models::MODEL_SOURCE.into(),
                bytes: brn_workflow::models::MODEL_BYTES,
                cost: brn_workflow::models::MODEL_COST.into(),
                destination: self.path.join(brn_workflow::models::MODEL_RELATIVE_DIR),
            });
        // Keep consent transient. Persist only through the owning command lane.
        self.ai.as_mut().unwrap().model_prompt = Some(prompt.clone());
        window.open_dialog(cx, move |dialog, _, _| {
            let approve = desktop.downgrade();
            let decline = approve.clone();
            let close = approve.clone();
            let target = prompt.destination.clone();
            dialog.title("Optional local search model").w(px(460.))
                .child(div().flex().flex_col().gap_2().child(prompt.source.clone())
                    .child(format!("{} bytes · {}", prompt.bytes, prompt.cost))
                    .child(spaced_identifier(&prompt.destination.display().to_string()))
                    .child("Decline makes no network request. Download is fresh approval, not a startup model load.")
                    .child(Button::new("approve-model").label("Download").on_click(move |_, window, cx| {
                        let _ = approve.update(cx, |this, cx| this.model_decision(true, target.clone(), cx));
                        window.close_dialog(cx);
                    }))
                    .child(Button::new("decline-model").label("Decline").on_click(move |_, window, cx| {
                        let _ = decline.update(cx, |this, cx| {
                            this.model_decision(false, this.path.join(brn_workflow::models::MODEL_RELATIVE_DIR), cx)
                        });
                        window.close_dialog(cx);
                    })))
                .on_close(move |_, _, cx| {
                    let _ = close.update(cx, |this, cx| {
                        if this.ai.as_ref().unwrap().model_prompt.is_some() {
                            this.model_decision(false, this.path.join(brn_workflow::models::MODEL_RELATIVE_DIR), cx);
                        }
                    });
                })
        });
    }
    fn model_decision(&mut self, consent: bool, target: PathBuf, cx: &mut Context<Self>) {
        let command = download_command(self.ai.as_mut().unwrap(), consent, target);
        self.simple_send(command, cx);
    }
    pub(super) fn begin_close(&mut self, route: CloseRoute, cx: &mut Context<Self>) {
        if let Some(ai) = &mut self.ai {
            ai.dismiss_login();
            ai.stop();
            ai.model_prompt = None;
        }
        let mut app = self.app_worker.take();
        let preferences = self.layout_task.take();
        let (tx, rx) = std::sync::mpsc::channel();
        self.closing = Some((route, rx));
        self.message =
            "Closing: cancelling and joining local work; this does not save Markdown.".into();
        cx.background_executor()
            .spawn(async move {
                let mut events = Vec::new();
                let result = if let Some(worker) = &mut app {
                    let result = worker.shutdown();
                    while let Some(event) = worker.try_event() {
                        events.push(event);
                    }
                    result
                } else {
                    Ok(())
                };
                if let Some(preferences) = preferences {
                    preferences.await;
                }
                let _ = tx.send(Closed { result, events });
            })
            .detach();
        cx.notify();
    }
    pub(super) fn poll_closing(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((route, receiver)) = &self.closing else {
            return false;
        };
        let route = *route;
        match receiver.try_recv() {
            Ok(closed) => {
                if let Some(ai) = &mut self.ai {
                    for (id, event) in closed.events {
                        ai.apply(id, event);
                    }
                }
                self.closing = None;
                match closed.result {
                    Ok(()) => {
                        self.closed = true;
                        if route == CloseRoute::Quit {
                            cx.quit();
                        } else {
                            window.remove_window();
                        }
                    }
                    Err(error) => {
                        self.close_failed = true;
                        self.message = format!(
                            "Close blocked: local finalization failed. Partial text is not saved. {}",
                            error.message
                        );
                        cx.notify();
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.closing = None;
                self.close_failed = true;
                self.message = "Close blocked: local drain could not be confirmed.".into();
                cx.notify();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        true
    }
    pub(super) fn render_simple_history(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        use gpui_kit::assets::IconName;
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let blocked = self.closing.is_some() || self.closed || self.close_failed;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| u64::try_from(time.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0);
        let dashboard_signal = ai.dashboard.page.as_ref().and_then(|page| {
            let counts = &page.counts;
            if counts.overdue > 0 {
                Some((format!("{} overdue", counts.overdue), Tone::Danger))
            } else if counts.follow_up > 0 {
                Some((format!("{} follow-up", counts.follow_up), Tone::Attention))
            } else {
                let unfinished = counts.open + counts.waiting + counts.blocked;
                (unfinished > 0).then(|| (unfinished.to_string(), Tone::Neutral))
            }
        });
        let inbox_signal = ai
            .inbox_queue
            .page
            .as_ref()
            .filter(|page| page.total_count > 0)
            .map(|page| (page.total_count.to_string(), Tone::Attention));
        let findings_signal = ai
            .finding_queue
            .page
            .as_ref()
            .filter(|page| page.open_count > 0)
            .map(|page| (page.open_count.to_string(), Tone::Attention));
        let awaiting: Vec<_> = ai
            .proposals
            .iter()
            .filter(|record| {
                !matches!(
                    record.state,
                    brn_workflow::proposals::ProposalState::Applied
                        | brn_workflow::proposals::ProposalState::Rejected
                )
            })
            .collect();
        let decided: Vec<_> = ai
            .proposals
            .iter()
            .filter(|record| {
                matches!(
                    record.state,
                    brn_workflow::proposals::ProposalState::Applied
                        | brn_workflow::proposals::ProposalState::Rejected
                )
            })
            .collect();
        let review_signal =
            (!awaiting.is_empty()).then(|| (awaiting.len().to_string(), Tone::Attention));

        let mut list = div()
            .id("history-rail-list")
            .track_scroll(&self.history_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .px_2()
            .pt_2()
            .pb_3()
            .child(
                ui::nav_row(
                    "new-session",
                    IconName::Plus,
                    "New chat",
                    Some(("⌘N".into(), Tone::Neutral)),
                    p,
                )
                .outline()
                .disabled(!ai.ready)
                .on_click(cx.listener(|this, _, _, cx| this.simple_history(None, cx))),
            )
            .child(ui::section_label("Workspace", p))
            .child(
                ui::nav_row(
                    "open-dashboard",
                    IconName::LayoutDashboard,
                    "Dashboard",
                    dashboard_signal,
                    p,
                )
                .selected(self.open_doc == Some(DocRef::Dashboard))
                .disabled(!ai.ready)
                .on_click(cx.listener(|this, _, window, cx| {
                    if !window.has_active_dialog(cx) {
                        this.simple_leave(EditorTransition::Dashboard, cx);
                    }
                })),
            )
            .child(
                ui::nav_row("open-inbox", IconName::Inbox, "Inbox", inbox_signal, p)
                    .selected(self.open_doc == Some(DocRef::Inbox))
                    .disabled(!ai.ready || blocked)
                    .on_click(cx.listener(|this, _, window, cx| {
                        if !window.has_active_dialog(cx) {
                            this.simple_leave(EditorTransition::Inbox, cx);
                        }
                    })),
            )
            .child(
                ui::nav_row(
                    "open-findings",
                    IconName::Bell,
                    "Needs Review",
                    findings_signal,
                    p,
                )
                .selected(self.open_doc == Some(DocRef::Findings))
                .disabled(!ai.ready)
                .on_click(
                    cx.listener(|this, _, _, cx| this.simple_leave(EditorTransition::Findings, cx)),
                ),
            )
            .child(
                ui::nav_row(
                    "open-activity",
                    IconName::GalleryVerticalEnd,
                    "Activity",
                    None,
                    p,
                )
                .selected(self.open_doc == Some(DocRef::Activity))
                .disabled(!ai.ready)
                .on_click(
                    cx.listener(|this, _, _, cx| this.simple_leave(EditorTransition::Activity, cx)),
                ),
            )
            .child(ui::section_label("Chats", p));
        if ai.conversations.is_empty() {
            list = list.child(ui::hint("No saved chats yet.", p).px_2());
        }
        for conversation in &ai.conversations {
            let id = conversation.id;
            list = list.child(
                ui::list_row(
                    format!("conversation-{id}"),
                    compact_title(&conversation.title),
                    Some(format!(
                        "{} · {} turn{}",
                        session_activity_label(conversation, now_ms),
                        conversation.turns,
                        if conversation.turns == 1 { "" } else { "s" }
                    )),
                    None,
                    p,
                )
                .selected(ai.conversation == Some(id))
                .on_click(cx.listener(move |this, _, _, cx| this.simple_history(Some(id), cx))),
            );
        }
        list = list.child(
            div()
                .flex()
                .items_center()
                .child(ui::section_label("Review", p).flex_1())
                .when_some(review_signal, |row, (count, tone)| {
                    row.child(
                        div()
                            .pt(px(tokens::space::MD))
                            .pb(px(tokens::space::XS))
                            .child(ui::badge(format!("{count} waiting"), tone, p)),
                    )
                })
                .child(
                    div().pt(px(tokens::space::SM)).child(
                        Button::new("refresh-proposal-list")
                            .icon(IconName::RotateCw)
                            .ghost()
                            .xsmall()
                            .tooltip("Refresh proposals")
                            .disabled(!ai.ready)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.simple_command(
                                    Pending::Proposals,
                                    AppCommand::Proposals(None),
                                    cx,
                                )
                            })),
                    ),
                ),
        );
        if awaiting.is_empty() {
            list = list.child(ui::hint("Nothing waiting for approval.", p).px_2());
        }
        let proposal_row = |record: &brn_workflow::proposals::ProposalRecord| {
            let id = record.draft.id;
            let (label, tone) = proposal_state_badge(record.state);
            ui::list_row(
                format!("proposal-{id}"),
                compact_title(&record.draft.title),
                None,
                Some(ui::badge(label, tone, p).into_any_element()),
                p,
            )
            .selected(self.open_doc == Some(DocRef::Proposal(id)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.simple_leave(EditorTransition::Review(id), cx)
            }))
        };
        for record in &awaiting {
            list = list.child(proposal_row(record));
        }
        list = list.child(
            ui::toolbar()
                .px_1()
                .pt_1()
                .child(
                    Button::new("new-proposal-form")
                        .icon(IconName::Plus)
                        .label("Proposal")
                        .ghost()
                        .small()
                        .tooltip("Draft a new note proposal for review")
                        .selected(
                            self.open_doc == Some(DocRef::Draft)
                                && ai.draft.as_ref().is_some_and(|form| form.action.is_none()),
                        )
                        .disabled(!ai.ready || !ai.vault_bound || ai.application_busy() || blocked)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_leave(EditorTransition::Draft(None), cx)
                        })),
                )
                .child(
                    Button::new("new-action-form")
                        .icon(IconName::Plus)
                        .label("Action")
                        .ghost()
                        .small()
                        .tooltip("Draft a new Action for review")
                        .selected(
                            self.open_doc == Some(DocRef::Draft)
                                && ai.draft.as_ref().is_some_and(|form| form.action.is_some()),
                        )
                        .disabled(!ai.ready || ai.application_busy() || blocked)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !window.has_active_dialog(cx) {
                                this.simple_leave(EditorTransition::ActionDraft(None), cx);
                            }
                        })),
                ),
        );
        if !decided.is_empty() {
            let open = self.show_decided;
            list = list.child(
                div().pt_2().child(
                    Button::new("toggle-decided")
                        .icon(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .label(format!("Decided ({})", decided.len()))
                        .ghost()
                        .xsmall()
                        .tooltip("Approved and rejected proposals. Activity has the full history.")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_decided = !this.show_decided;
                            cx.notify();
                        })),
                ),
            );
            if open {
                for record in &decided {
                    list = list.child(proposal_row(record));
                }
            }
        }
        div()
            .w(px(self.layout.history_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(color(p.line))
                    .p_2()
                    .child(
                        ui::nav_row(
                            "settings-footer",
                            IconName::Settings,
                            "Settings",
                            Some(("⌘,".into(), Tone::Neutral)),
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                        ),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn render_simple_vault(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        use gpui_kit::assets::IconName;
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let scope = ai.knowledge_scope;
        let mut header = div()
            .flex()
            .items_center()
            .child(ui::section_label("Vault", p).flex_1())
            .child(
                div().pt(px(tokens::space::SM)).child(
                    Button::new("refresh-vault")
                        .icon(IconName::RotateCw)
                        .ghost()
                        .xsmall()
                        .tooltip("Rescan saved notes")
                        .disabled(!ai.ready || !ai.vault_bound)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_command(Pending::Refresh, AppCommand::Refresh, cx)
                        })),
                ),
            );
        if !ai.vault_bound {
            header = header.child(
                div().pt(px(tokens::space::SM)).child(
                    Button::new("choose-vault")
                        .label("Choose vault…")
                        .primary()
                        .xsmall()
                        .disabled(!ai.ready || ai.vault_bound || self.choosing_file)
                        .on_click(cx.listener(|this, _, _, cx| this.simple_choose_vault(cx))),
                ),
            );
        }
        let identity = match &ai.vault_root {
            Some(root) => div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .px_2()
                .child(
                    div()
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .child(
                            root.file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| root.display().to_string()),
                        ),
                )
                .child(ui::meta(spaced_identifier(&home_relative(root)), p)),
            None => div().px_2().child(ui::hint(
                "No vault selected. Choose the Markdown folder BRN should read; BRN never moves or rewrites it without an approved proposal.",
                p,
            )),
        };
        let mut scopes = div().flex().gap(px(2.)).px_2();
        for option in [
            KnowledgeScope::Current,
            KnowledgeScope::Source,
            KnowledgeScope::History,
            KnowledgeScope::All,
        ] {
            scopes = scopes.child(
                Button::new(format!("knowledge-scope-{}", scope_name(option)))
                    .label(scope_name(option))
                    .compact()
                    .small()
                    .when(scope == option, |button| button.primary())
                    .when(scope != option, |button| button.ghost())
                    .selected(scope == option)
                    .disabled(!ai.ready || !ai.vault_bound || self.closing.is_some() || self.closed)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(command) = this.ai.as_mut().unwrap().select_scope(option) {
                            this.simple_send(command, cx);
                            if this.relationships.open {
                                this.refresh_relationship_page(0, cx);
                            }
                        }
                    })),
            );
        }
        let mut list = div()
            .id("vault-rail-list")
            .track_scroll(&self.vault_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .px_2()
            .pb_3()
            .child(header)
            .child(identity)
            .child(ui::section_label("Browse and search", p))
            .child(scopes)
            .child(ui::hint(scope_description(scope), p).px_2().pt_1())
            .child(
                ui::toolbar().px_1().pt_1().child(
                    Button::new("inspect-relationships")
                        .icon(IconName::Network)
                        .label("Relationships")
                        .ghost()
                        .small()
                        .selected(self.relationships.open)
                        .disabled(
                            !ai.ready
                                || !ai.vault_bound
                                || ai.application_busy()
                                || self.closing.is_some()
                                || self.closed,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.relationships.open {
                                this.close_relationship_page(cx);
                            } else {
                                this.refresh_relationship_page(0, cx);
                            }
                        })),
                ),
            );
        if let Some(relationships) = self.render_relationship_page(cx) {
            list = list.child(relationships);
        }
        if let Some(error) = &ai.notes_error {
            list = list.child(ui::callout(Tone::Danger, error.clone(), p));
        }
        if !ai.editors.is_empty() {
            list = list.child(ui::section_label("Recovered edits", p));
            for record in &ai.editors {
                let path = record.path.clone();
                let unsaved = record.text != record.baseline_text;
                list = list.child(
                    ui::list_row(
                        format!("recovered-{}", record.path),
                        compact_title(&record.path),
                        None,
                        unsaved
                            .then(|| ui::badge("unsaved", Tone::Attention, p).into_any_element()),
                        p,
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.simple_note(path.clone(), cx)),
                    ),
                );
            }
        }
        list = list.child(
            div()
                .flex()
                .items_center()
                .child(ui::section_label(format!("{} notes", scope_name(scope)), p).flex_1())
                .when(!ai.notes.is_empty(), |row| {
                    row.child(
                        div()
                            .pt(px(tokens::space::MD))
                            .pb(px(tokens::space::XS))
                            .px_2()
                            .child(ui::meta(
                                format!(
                                    "{}{}",
                                    ai.notes.len(),
                                    if ai.next_cursor.is_some() { "+" } else { "" }
                                ),
                                p,
                            )),
                    )
                }),
        );
        if ai.vault_bound && ai.notes.is_empty() && ai.notes_error.is_none() {
            list = list.child(ui::hint("No saved notes in this scope.", p).px_2());
        }
        for note in &ai.notes {
            let path = note.path.clone();
            list = list.child(
                ui::list_row(
                    format!("note-{}", note.path),
                    compact_title(&note.title),
                    Some(note.path.clone()),
                    None,
                    p,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.simple_scoped_note(path.clone(), scope, cx)
                })),
            );
        }
        if ai.next_cursor.is_some() {
            list = list.child(ui::quiet("more-notes", "More notes").on_click(cx.listener(
                move |this, _, _, cx| {
                    if let Some(command) = this.ai.as_mut().unwrap().more_notes() {
                        this.simple_send(command, cx);
                    }
                },
            )));
        }
        let mut index = div().flex().flex_col().gap_1().px_2().pt_2();
        index = index.child(ui::hint(ai.model_state.clone(), p));
        if let Some((_, embedded, total)) = ai.indexing {
            index = index.child(ui::meta(
                format!("Indexed embeddings: {embedded}/{total}"),
                p,
            ));
        }
        if let Some(report) = &ai.refresh {
            index = index.child(ui::meta(
                format!(
                    "Refresh: {} added · {} updated · {} removed · {} unchanged",
                    report.added, report.updated, report.removed, report.unchanged
                ),
                p,
            ));
            for unreadable in &report.unreadable {
                index = index.child(ui::callout(
                    Tone::Attention,
                    format!("Unreadable: {} · {}", unreadable.path, unreadable.reason),
                    p,
                ));
            }
        }
        list = list.child(ui::section_label("Index", p)).child(index);
        div()
            .w(px(self.layout.vault_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
            .into_any_element()
    }
    pub(super) fn render_simple_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let leaving = self.simple_transition.is_some() || self.closing.is_some() || self.closed;
        let inspection_open = self.provenance_open || self.saved_links.open;
        let mut body = div()
            .id("saved-note-body")
            .overflow_y_scroll()
            .track_scroll(&self.document_scroll)
            .vertical_scrollbar(&self.document_scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .px(px(tokens::space::LG))
            .py(px(tokens::space::MD));
        if let Some(editor) = &ai.editor {
            body = body.child(ui::meta(editor.status(), p));
            if let Some(error) = &editor.error {
                body = body.child(ui::callout(Tone::Danger, error.clone(), p));
            }
            body = body
                .child(div().key_context("MarkdownNote").flex_1().min_h(px(if inspection_open { 160. } else { 0. })).child(
                    Editor::new(&self.note_editor).h_full()
                        .disabled(leaving || editor.replacing()).aria_label("Markdown note editor")))
                .child(ui::toolbar()
                    .child(Button::new("simple-save").label("Save to Markdown").primary().small().tooltip("Write this exact text to the Markdown file (⌘S)")
                        .disabled(leaving || !editor.can_save())
                        .on_click(cx.listener(|this, _, _, cx| this.simple_save(None, cx))))
                    .child(Button::new("simple-flush-recovery").label("Flush / retry recovery").ghost().small()
                        .disabled(editor.pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor) = this.ai.as_mut().unwrap().editor.as_mut() { editor.retry_recovery(); }
                            if let Some(command) = this.ai.as_mut().unwrap().recover_editor() { this.simple_send(command, cx); }
                        })))
                    .child(Button::new("simple-observe-disk").label("Observe disk").ghost().small()
                        .disabled(leaving || editor.pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(command) = this.ai.as_mut().unwrap().refresh_editor() { this.simple_send(command, cx); }
                        })))
                    .child(Button::new("simple-compare").label("Compare baseline / local / disk").ghost().small()
                        .on_click(cx.listener(|this, _, window, cx| this.simple_compare(window, cx))))
                    .child(Button::new("simple-reload").label("Reload reviewed disk…").ghost().small()
                        .disabled(leaving || editor.reload_request().is_none())
                        .on_click(cx.listener(|this, _, window, cx| this.simple_confirm_reload(window, cx))))
                    .child(Button::new("cancel-simple-leave").label("Cancel pending close / navigation").ghost().small()
                        .disabled(self.simple_transition.is_none())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_transition = None;
                            this.ai.as_mut().unwrap().notice = "Close / navigation cancelled; editor retained.".into();
                            cx.notify();
                        }))))
                .child(ui::section_label("Save a copy", p).px_0())
                .child(div().flex().gap_2()
                    .child(Input::new(&self.note_path).aria_label("Unused vault-relative .md copy destination"))
                    .child(Button::new("simple-save-copy").label("Save Copy").outline().small()
                        .disabled(leaving || editor.pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            let destination = this.note_path.read(cx).value().to_string();
                            if destination.is_empty() {
                                this.ai.as_mut().unwrap().notice = "Enter an unused vault-relative .md destination for Save Copy.".into();
                                cx.notify();
                            } else { this.simple_save(Some(destination), cx); }
                        }))));
            for operation in &editor.view.pending {
                let operation = *operation;
                body = body.child(
                    Button::new(format!("reconcile-{operation}"))
                        .label("Reconcile uncertain Save")
                        .outline()
                        .small()
                        .disabled(editor.pending() || leaving)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.simple_command(
                                Pending::EditorReconcile,
                                AppCommand::ReconcileEditor(operation),
                                cx,
                            );
                        })),
                );
            }
        } else {
            body = body.child(match &ai.note_error {
                Some(error) => ui::callout(Tone::Danger, error.clone(), p).into_any_element(),
                None => ui::hint("Opening Markdown / recovery buffer…", p).into_any_element(),
            });
        }
        if let Some(sources) = self.render_saved_sources(cx) {
            body = body.child(sources);
        }
        if let Some(links) = self.render_saved_links(cx) {
            body = body.child(links);
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(
                ui::view_header(
                    self.simple_note_path
                        .as_ref()
                        .and_then(|path| ai.notes.iter().find(|note| &note.path == path))
                        .map(|note| note.title.clone())
                        .unwrap_or_else(|| note_title(self.simple_note_path.as_deref())),
                    Some(("Current · editable", Tone::Success)),
                    self.simple_note_path.clone(),
                    p,
                )
                .child(
                    Button::new("saved-note-sources")
                        .ghost()
                        .small()
                        .label("Sources")
                        .compact()
                        .selected(self.provenance_open)
                        .disabled(
                            leaving
                                || ai.editor.is_none()
                                || ai.application_busy()
                                || ai.provenance_loading(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.inspect_saved_sources(cx))),
                )
                .child(
                    Button::new("saved-note-links")
                        .ghost()
                        .small()
                        .label("Links")
                        .compact()
                        .selected(self.saved_links.open)
                        .disabled(
                            leaving
                                || ai.editor.is_none()
                                || ai.application_busy()
                                || ai.links_loading(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.saved_links.open {
                                this.close_saved_links(cx);
                            } else {
                                this.inspect_saved_links(cx);
                            }
                        })),
                )
                .child(
                    Button::new("close-document")
                        .ghost()
                        .small()
                        .label("Close")
                        .compact()
                        .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                ),
            )
            .child(body.test_support())
            .into_any_element()
    }
    pub(super) fn render_evidence_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let inspection_open = self.provenance_open || self.saved_links.open;
        let mut body = div()
            .id("evidence-note-body")
            .overflow_y_scroll()
            .track_scroll(&self.document_scroll)
            .vertical_scrollbar(&self.document_scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .px(px(tokens::space::LG))
            .py(px(tokens::space::MD));
        let (kind, tone) = match ai.evidence.as_ref().map(|evidence| evidence.scope) {
            Some(KnowledgeScope::Source) => ("Source · read only", Tone::Info),
            Some(KnowledgeScope::History) => ("History · read only", Tone::Neutral),
            Some(KnowledgeScope::All) => ("All scopes · read only", Tone::Neutral),
            _ => ("Read only", Tone::Neutral),
        };
        let heading = ai
            .evidence
            .as_ref()
            .and_then(|evidence| ai.notes.iter().find(|note| note.path == evidence.path))
            .map(|note| note.title.clone())
            .unwrap_or_else(|| {
                note_title(ai.evidence.as_ref().map(|evidence| evidence.path.as_str()))
            });
        let title = ai
            .evidence
            .as_ref()
            .map(|evidence| {
                format!(
                    "{} · {} scope · Read only",
                    evidence.path,
                    scope_name(evidence.scope)
                )
            })
            .unwrap_or_default();
        if let Some(note) = ai
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.note.as_ref())
        {
            let text = note.text.clone();
            body = body
                .child(
                    div()
                        .flex_1()
                        .min_h(px(if inspection_open { 160. } else { 0. }))
                        .child(evidence_widget(&self.evidence_editor)),
                )
                .child(
                    Button::new("copy-exact-evidence")
                        .label("Copy exact saved text")
                        .outline()
                        .small()
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                text.clone(),
                            ));
                        })),
                );
        } else {
            body = body.child(match &ai.note_error {
                Some(error) => ui::callout(Tone::Danger, error.clone(), p).into_any_element(),
                None => ui::hint("Opening exact saved evidence…", p).into_any_element(),
            });
        }
        if let Some(sources) = self.render_saved_sources(cx) {
            body = body.child(sources);
        }
        if let Some(links) = self.render_saved_links(cx) {
            body = body.child(links);
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(
                ui::view_header(heading, Some((kind, tone)), Some(title), p)
                    .child(
                        Button::new("evidence-sources")
                            .ghost()
                            .small()
                            .label("Sources")
                            .compact()
                            .selected(self.provenance_open)
                            .disabled(
                                self.simple_transition.is_some()
                                    || self.closing.is_some()
                                    || self.closed
                                    || ai
                                        .evidence
                                        .as_ref()
                                        .is_none_or(|evidence| evidence.note.is_none())
                                    || ai.application_busy()
                                    || ai.provenance_loading(),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.inspect_saved_sources(cx))),
                    )
                    .child(
                        Button::new("evidence-links")
                            .ghost()
                            .small()
                            .label("Links")
                            .compact()
                            .selected(self.saved_links.open)
                            .disabled(
                                self.simple_transition.is_some()
                                    || self.closing.is_some()
                                    || self.closed
                                    || ai
                                        .evidence
                                        .as_ref()
                                        .is_none_or(|evidence| evidence.note.is_none())
                                    || ai.application_busy()
                                    || ai.links_loading(),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.saved_links.open {
                                    this.close_saved_links(cx);
                                } else {
                                    this.inspect_saved_links(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("close-evidence")
                            .ghost()
                            .small()
                            .label("Close")
                            .compact()
                            .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                    ),
            )
            .child(body.test_support())
            .into_any_element()
    }
    pub(super) fn render_simple_chat(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        use gpui_kit::assets::IconName;
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let blocked = self.closing.is_some() || self.closed || self.close_failed;
        let mut body = div()
            .id("chat-transcript")
            .track_scroll(&self.chat_scroll)
            .vertical_scrollbar(&self.chat_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap(px(tokens::space::XL))
            .px(px(tokens::space::XL))
            .py(px(tokens::space::LG));
        if ai.turns.is_empty() && ai.active.is_none() && ai.unsaved.is_none() && ai.search.is_none()
        {
            body = body.child(self.render_chat_welcome(cx));
        }
        let ai = self.ai.as_ref().unwrap();
        for turn in ai.display_turns() {
            let tone = match turn.status {
                brn_workflow::WorkTurnStatus::Completed => Tone::Success,
                brn_workflow::WorkTurnStatus::Running => Tone::Ai,
                brn_workflow::WorkTurnStatus::Interrupted => Tone::Attention,
                brn_workflow::WorkTurnStatus::Failed => Tone::Danger,
            };
            let question = ai
                .inbox_analysis_path_for_turn(turn.id)
                .map(|path| format!("Analyze saved Inbox Source: {path}"))
                .unwrap_or_else(|| turn.question.clone());
            let mut block = chat_exchange(
                question,
                turn.answer.clone(),
                format!(
                    "{} · {} · effort {}",
                    turn.provider,
                    turn.model,
                    turn.effort
                        .as_deref()
                        .unwrap_or("unavailable in older history"),
                ),
                ui::badge(turn_label(turn), tone, p),
                p,
            );
            if let Some(code) = &turn.error_code {
                block = block.child(ui::callout(
                    Tone::Danger,
                    format!("Safe failure category: {code}"),
                    p,
                ));
            }
            if turn.status == brn_workflow::WorkTurnStatus::Completed {
                let id = turn.id;
                block = block.child(
                    ui::toolbar().child(
                        Button::new(format!("review-completed-answer-{id}"))
                            .icon(IconName::Plus)
                            .label("Review as new note…")
                            .ghost()
                            .small()
                            .tooltip("Prepare a note proposal from this answer. Nothing is saved until you approve it.")
                            .disabled(
                                !ai.ready || !ai.vault_bound || ai.application_busy() || blocked,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.simple_leave(EditorTransition::Draft(Some(id)), cx)
                            })),
                    ),
                );
            }
            body = body.child(block);
        }
        if let Some(turn) = &ai.unsaved {
            let partial = turn.answer.clone();
            body = body.child(
                chat_exchange(
                    turn.question.clone(),
                    turn.answer.clone(),
                    format!(
                        "{} · {} · effort {}",
                        turn.provider,
                        turn.model,
                        turn.effort.as_deref().unwrap_or("unavailable")
                    ),
                    ui::badge("Failed · in-memory partial · finalization not acknowledged", Tone::Danger, p),
                    p,
                )
                .child(ui::callout(
                    Tone::Danger,
                    "Copy this partial before closing/restarting. Further Ask is blocked until this unfinalized workspace is reopened.",
                    p,
                ))
                .child(
                    ui::toolbar().child(
                        ui::secondary("copy-unfinalized-partial", "Copy in-memory partial")
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(partial.clone()));
                            })),
                    ),
                ),
            );
        }
        if let Some(active) = ai.display_active() {
            let mut block = chat_exchange(
                active.request.question_label(),
                active.partial.clone(),
                format!(
                    "{} · {} · effort {}",
                    provider_name(active.request.selection().provider),
                    active.request.selection().model,
                    active
                        .request
                        .effort()
                        .map(ReasoningEffort::as_str)
                        .unwrap_or("unavailable"),
                ),
                ui::badge(
                    if active.stopping {
                        "Stopping (not finalized)"
                    } else {
                        "Streaming (provisional)"
                    },
                    Tone::Ai,
                    p,
                ),
                p,
            );
            if let Some(tool) = &active.tool {
                block = block.child(ui::meta(format!("Tool started: {tool}"), p));
            }
            body = body.child(block);
        }
        if let Some(results) = &ai.search {
            let scope = ai.search_scope.unwrap_or(KnowledgeScope::Current);
            let mut section = div().flex().flex_col().gap_2().child(
                ui::toolbar()
                    .child(
                        div()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(format!("Search results · {} scope", scope_name(scope))),
                    )
                    .child(ui::badge(
                        if results.keyword_only {
                            "Search: keyword-only (no installed model)"
                        } else {
                            "Search: hybrid"
                        },
                        Tone::Neutral,
                        p,
                    )),
            );
            if results.hits.is_empty() {
                section = section.child(ui::hint("No saved notes matched.", p));
            }
            for (i, hit) in results.hits.iter().enumerate() {
                let path = hit.path.clone();
                section = section.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            ui::list_row(
                                format!("simple-hit-{i}"),
                                hit.path.clone(),
                                Some(format!("bytes {}..{}", hit.start_byte, hit.end_byte)),
                                None,
                                p,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.simple_scoped_note(path.clone(), scope, cx)
                                },
                            )),
                        )
                        .child(ui::callout(Tone::Info, hit.quote.clone(), p)),
                );
            }
            body = body.child(section);
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(body)
            .child(self.render_composer(cx))
            .into_any_element()
    }

    fn render_chat_welcome(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        use gpui_kit::assets::IconName;
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let check = |done: bool, label: String, todo: &'static str, ready: &'static str| {
            let detail = if done { ready } else { todo };
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(ui::badge(
                    if done { "ready" } else { "to do" },
                    if done { Tone::Success } else { Tone::Attention },
                    p,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.))
                        .child(div().min_w(px(0.)).child(label))
                        .child(ui::hint(detail, p)),
                )
        };
        let model = match (&ai.selection, ai.effort) {
            (Some(selection), Some(effort)) => format!(
                "Model: {} / {} · effort {}",
                provider_name(selection.provider),
                selection.model,
                effort.as_str()
            ),
            (Some(selection), None) => format!(
                "Model: {} / {} · choose an effort",
                provider_name(selection.provider),
                selection.model
            ),
            _ => "Model: not selected".into(),
        };
        let mut examples = ui::toolbar();
        for (index, example) in [
            "Are there any open actions?",
            "What am I waiting for from other people?",
            "What changed in my notes this week?",
        ]
        .into_iter()
        .enumerate()
        {
            examples = examples.child(
                Button::new(("chat-example", index))
                    .label(example)
                    .outline()
                    .small()
                    .tooltip("Put this question in the composer")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.query
                            .update(cx, |query, cx| query.set_value(example, window, cx));
                        this.focus_composer = true;
                        cx.notify();
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(tokens::space::LG))
            .max_w(px(tokens::size::READING_MAX_WIDTH))
            .pt(px(tokens::space::XL))
            .child(
                div()
                    .text_size(px(tokens::text::DISPLAY))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("What are you working on?"),
            )
            .child(
                div()
                    .text_size(px(tokens::text::READING))
                    .line_height(px(22.))
                    .text_color(color(p.muted))
                    .child("Ask about your notes, sources and Actions. BRN answers from approved current knowledge by default, labels what it cannot establish, and never changes knowledge or Actions without your approval."),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(check(
                        ai.vault_bound,
                        ai.vault_root
                            .as_ref()
                            .map(|root| {
                                format!(
                                    "Vault: {}",
                                    root.file_name()
                                        .map(|name| name.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| home_relative(root))
                                )
                            })
                            .unwrap_or_else(|| "Vault: not selected".into()),
                        "Choose a vault in the Vault rail.",
                        "Notes stay ordinary Markdown files you own.",
                    ))
                    .child(check(
                        ai.selection.is_some() && ai.effort.is_some(),
                        model,
                        "Choose the provider, model and reasoning effort in Settings.",
                        "BRN keeps this choice until you change it; it never switches silently.",
                    ))
                    .when(ai.selection.is_none() || ai.effort.is_none(), |list| {
                        list.child(
                            ui::toolbar().child(
                                Button::new("welcome-settings")
                                    .icon(IconName::Settings)
                                    .label("Open Settings")
                                    .outline()
                                    .small()
                                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
                            ),
                        )
                    }),
            )
            .child(ui::section_label("Try asking", p).px_0())
            .child(examples)
            .into_any_element()
    }

    fn render_composer(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        use gpui_kit::assets::IconName;
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let model = match &ai.selection {
            Some(selection) => ui::meta(
                format!(
                    "{} / {} · effort {}",
                    provider_name(selection.provider),
                    selection.model,
                    ai.effort.map(ReasoningEffort::as_str).unwrap_or("not set")
                ),
                p,
            )
            .into_any_element(),
            None => ui::badge("No explicit provider/model selected", Tone::Attention, p)
                .into_any_element(),
        };
        let mut actions = ui::toolbar().child(model).child(div().flex_1()).child(
            Button::new("search")
                .icon(IconName::Search)
                .label(format!("Search {}", scope_name(ai.knowledge_scope)))
                .outline()
                .small()
                .tooltip("Search saved notes in the selected vault scope. No AI is used.")
                .disabled(!ai.ready || !ai.vault_bound)
                .on_click(cx.listener(|this, _, _, cx| this.simple_search(cx))),
        );
        if ai.active.is_some() {
            actions = actions.child(
                Button::new("stop")
                    .label("Stop")
                    .danger()
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| this.simple_stop(cx))),
            );
        }
        actions = actions.child(
            ui::primary("ask", "Ask")
                .icon(IconName::ArrowUp)
                .tooltip("Ask BRN. Answers use approved Current knowledge by default.")
                .disabled(!ai.can_ask() || self.closing.is_some() || self.close_failed)
                .on_click(cx.listener(|this, _, _, cx| this.simple_ask(cx))),
        );
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .px(px(tokens::space::MD))
            .py(px(tokens::space::SM))
            .border_t_1()
            .border_color(color(p.line))
            .bg(color(p.panel))
            .child(
                Editor::new(&self.query)
                    .h(px(76.))
                    .aria_label("Question about saved notes"),
            )
            .child(actions)
            .into_any_element()
    }
}

/// One question and its answer, with the answer's recorded provider/model/effort.
fn chat_exchange(
    question: String,
    answer: String,
    provenance: String,
    status: gpui_kit::Div,
    p: crate::tokens::Palette,
) -> gpui_kit::Div {
    use super::ui;
    div()
        .flex()
        .flex_col()
        .gap(px(tokens::space::SM))
        .max_w(px(tokens::size::READING_MAX_WIDTH))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px(px(tokens::space::MD))
                .py(px(tokens::space::SM))
                .bg(color(p.active))
                .child(
                    div()
                        .text_size(px(tokens::text::CAPTION))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(color(p.muted))
                        .child("YOU"),
                )
                .child(
                    div()
                        .text_size(px(tokens::text::READING))
                        .line_height(px(21.))
                        .child(question),
                ),
        )
        .child(
            ui::toolbar()
                .child(
                    div()
                        .text_size(px(tokens::text::CAPTION))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(color(p.purple))
                        .child("BRN"),
                )
                .child(ui::meta(provenance, p))
                .child(status),
        )
        .child(
            div()
                .text_size(px(tokens::text::READING))
                .line_height(px(22.))
                .child(answer),
        )
}

pub(super) fn account_settings(desktop: &Entity<Desktop>, cx: &App) -> AnyElement {
    let this = desktop.read(cx);
    let ai = this.ai.as_ref().unwrap();
    let mut body =
        div().flex().flex_col().gap_2().child(
            "Connected means locally persisted credentials, not proof of live availability.",
        );
    for provider in [Provider::Chatgpt, Provider::Copilot] {
        let row = &ai.accounts[slot(provider)];
        let busy = ai.account_busy(provider)
            || !ai.ready
            || this.closing.is_some()
            || this.closed
            || this.close_failed;
        let connected = row.status.as_ref().is_some_and(|status| status.connected);
        let label = row
            .status
            .as_ref()
            .map_or("Status unknown".into(), |status| {
                if status.connected {
                    format!(
                        "Connected · {}",
                        status.name.as_deref().unwrap_or("account name unavailable")
                    )
                } else {
                    "Disconnected / reconnect needed".into()
                }
            });
        let target = desktop.downgrade();
        let disconnect = target.clone();
        let status = target.clone();
        let models = target.clone();
        let choose = target.clone();
        body = body.child(format!("{} · {label}", provider_name(provider)));
        if let Some(error) = &row.error {
            body = body.child(format!(
                "{error} · explicit Connect to retry; no automatic retry."
            ));
        }
        body = body.child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(
                    Button::new(format!("connect-{}", slot(provider)))
                        .label(if connected { "Reconnect" } else { "Connect" })
                        .disabled(busy || ai.login.is_some())
                        .on_click(move |_, window, cx| {
                            let _ = target.update(cx, |this, cx| {
                                this.simple_account(AccountCommand::Connect(provider), window, cx)
                            });
                        }),
                )
                .child(
                    Button::new(format!("disconnect-{}", slot(provider)))
                        .label("Disconnect")
                        .disabled(busy || !connected)
                        .on_click(move |_, window, cx| {
                            let _ = disconnect.update(cx, |this, cx| {
                                this.simple_account(
                                    AccountCommand::Disconnect(provider),
                                    window,
                                    cx,
                                )
                            });
                        }),
                )
                .child(
                    Button::new(format!("status-{}", slot(provider)))
                        .label("Status")
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            let _ = status.update(cx, |this, cx| {
                                this.simple_account(AccountCommand::Status(provider), window, cx)
                            });
                        }),
                )
                .child(
                    Button::new(format!("models-{}", slot(provider)))
                        .label("Discover models")
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            let _ = models.update(cx, |this, cx| {
                                this.simple_account(AccountCommand::Models(provider), window, cx)
                            });
                        }),
                )
                .child(
                    Button::new(format!("provider-{}", slot(provider)))
                        .label("Select provider")
                        .selected(ai.provider == Some(provider))
                        .disabled(!ai.ready)
                        .on_click(move |_, _, cx| {
                            let _ = choose.update(cx, |this, cx| {
                                this.ai.as_mut().unwrap().provider = Some(provider);
                                cx.notify();
                            });
                        }),
                ),
        );
        if ai.provider == Some(provider) {
            for (index, model) in row.models.iter().enumerate() {
                let target = desktop.downgrade();
                let selection = Selection {
                    provider,
                    model: model.id.clone(),
                };
                body = body.child(
                    Button::new(format!("model-{}-{index}", slot(provider)))
                        .label(format!(
                            "{} · {}",
                            model.id,
                            if model.live_qualified {
                                "qualified"
                            } else {
                                "not live-qualified"
                            }
                        ))
                        .selected(ai.selection.as_ref() == Some(&selection))
                        .disabled(!ai.ready)
                        .on_click(move |_, _, cx| {
                            let _ = target.update(cx, |this, cx| {
                                this.simple_command(
                                    Pending::Select,
                                    AppCommand::Select(selection.clone()),
                                    cx,
                                )
                            });
                        }),
                );
            }
        }
    }
    if let Some(error) = &ai.selection_error {
        body = body.child(format!(
            "Selection unavailable: {error}. Account/history diagnostics remain available."
        ));
    }
    body = body.child("Reasoning effort for new Ask and Rewrite requests");
    let effort_busy = !ai.ready
        || this.closed
        || this.closing.is_some()
        || this.close_failed
        || ai
            .pending
            .values()
            .any(|pending| matches!(pending, Pending::Effort | Pending::SelectEffort));
    for effort in [
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
    ] {
        let target = desktop.downgrade();
        body = body.child(
            Button::new(format!("reasoning-effort-{}", effort.as_str()))
                .label(effort.as_str())
                .selected(ai.effort == Some(effort))
                .disabled(effort_busy)
                .on_click(move |_, _, cx| {
                    let _ = target.update(cx, |this, cx| {
                        this.simple_command(
                            Pending::SelectEffort,
                            AppCommand::SelectEffort(effort),
                            cx,
                        )
                    });
                }),
        );
    }
    if let Some(error) = &ai.effort_error {
        body = body.child(format!("Reasoning effort unavailable: {error}"));
    } else if ai.effort.is_none() {
        body = body.child("Choose low, medium or high before asking AI.");
    }
    let target = desktop.downgrade();
    let cancel = target.clone();
    body.child("ChatGPT chat is conditionally qualified: quota reset alone does not prove availability. No automatic model/provider fallback.")
        .child(ai.model_state.clone())
        .children(ai.download_progress.map(|(received, total)| div().child(format!("Download: {received}/{total} bytes"))))
        .child(Button::new("download-model").label("Download local search model…").disabled(!cfg!(feature = "native-retrieval") || !ai.ready || ai.download.is_some() || this.closed || this.closing.is_some() || this.close_failed)
            .on_click(move |_, window, cx| { let _ = target.update(cx, |this, cx| this.open_model_consent(window, cx)); }))
        .child(Button::new("cancel-download").label("Cancel Download").disabled(ai.download.is_none())
            .on_click(move |_, _, cx| { let _ = cancel.update(cx, |this, cx| {
                if let Some(id) = this.ai.as_ref().unwrap().download {
                    this.ai.as_mut().unwrap().download_stopping = true;
                    this.ai.as_mut().unwrap().model_state = "Cancelling download; awaiting local join".into();
                    this.simple_send((Uuid::new_v4(), AppCommand::CancelModelDownload(id)), cx);
                }
            }); })).into_any_element()
}

pub(super) fn proposal_state_badge(
    state: brn_workflow::proposals::ProposalState,
) -> (&'static str, super::ui::Tone) {
    use super::ui::Tone;
    use brn_workflow::proposals::ProposalState;
    match state {
        ProposalState::Draft => ("Draft", Tone::Attention),
        ProposalState::Applying => ("Applying", Tone::Info),
        ProposalState::Uncertain => ("Uncertain", Tone::Danger),
        ProposalState::Applied => ("Applied", Tone::Success),
        ProposalState::Rejected => ("Rejected", Tone::Neutral),
    }
}

fn scope_description(scope: KnowledgeScope) -> &'static str {
    match scope {
        KnowledgeScope::Current => "Approved current knowledge. Ask uses this by default.",
        KnowledgeScope::Source => "Imported originals and evidence. Read only.",
        KnowledgeScope::History => "Superseded and archived knowledge. Read only.",
        KnowledgeScope::All => "Every saved note, labelled by kind. Read only.",
    }
}

/// Display a path with the home directory abbreviated to `~`.
fn home_relative(path: &std::path::Path) -> String {
    let display = path.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && display.starts_with(&home) => {
            format!("~{}", &display[home.len()..])
        }
        _ => display,
    }
}

/// Human title for a vault-relative path: the file stem, or a placeholder.
fn note_title(path: Option<&str>) -> String {
    path.and_then(|path| std::path::Path::new(path).file_stem())
        .map(|stem| stem.to_string_lossy().replace(['-', '_'], " "))
        .unwrap_or_else(|| "Note".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AiState;
    fn ready() -> AiState {
        let mut state = AiState::default();
        state.apply(
            Uuid::new_v4(),
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false,
            },
        );
        state.selection = Some(Selection {
            provider: Provider::Copilot,
            model: "explicit".into(),
        });
        state.effort = Some(ReasoningEffort::High);
        state.pending.clear();
        state
    }
    #[cfg(target_os = "macos")]
    struct EditorFixture(PathBuf);
    #[cfg(target_os = "macos")]
    impl EditorFixture {
        fn new() -> Self {
            let base = std::env::temp_dir()
                .canonicalize()
                .unwrap()
                .join(format!("brn-current-desktop-{}", Uuid::new_v4()));
            std::fs::create_dir(&base).unwrap();
            std::fs::create_dir(base.join("data")).unwrap();
            std::fs::create_dir(base.join("vault")).unwrap();
            Self(base)
        }
        fn worker(&self) -> brn_workflow::app_worker::AppWorker {
            let worker = brn_workflow::app_worker::AppWorker::start(
                self.0.join("data"),
                brn_workflow::app::AppConfig {
                    vault_root: Some(self.0.join("vault")),
                    credentials_dir: Some(self.0.join("credentials")),
                    model_dir: None,
                },
            )
            .unwrap();
            loop {
                match worker
                    .recv_event_timeout(Duration::from_secs(10))
                    .unwrap()
                    .1
                {
                    AppEvent::Ready { .. } => return worker,
                    AppEvent::Failed(error) => panic!("startup: {}", error.message),
                    _ => {}
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    impl Drop for EditorFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[cfg(target_os = "macos")]
    fn reply(
        worker: &brn_workflow::app_worker::AppWorker,
        command: (Uuid, AppCommand),
    ) -> (Uuid, AppEvent) {
        let (id, command) = command;
        worker.submit(id, command).unwrap();
        loop {
            let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
            if event.0 == id {
                if let AppEvent::Failed(error) = &event.1 {
                    panic!("editor command failed: {:?}: {}", error.kind, error.message);
                }
                return event;
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn gpui_rope_preserves_exact_markdown_through_app_worker_save_and_restart() {
        use brn_workflow::editor::SaveOutcome;
        use gpui_kit::component::input::{Rope, RopeExt};
        use std::os::unix::fs::MetadataExt;
        let fixture = EditorFixture::new();
        let path = fixture.0.join("vault/plan.md");
        let original = "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文\nlast\r";
        std::fs::write(&path, original).unwrap();
        let before = std::fs::metadata(&path).unwrap();
        let mut worker = fixture.worker();
        let mut ai = ready();
        let open = ai.open_editor("plan.md".into());
        let (id, event) = reply(&worker, open);
        ai.apply(id, event);
        let mut rope = Rope::from(ai.editor.as_ref().unwrap().text.as_str());
        ai.editor
            .as_mut()
            .unwrap()
            .edit(
                gpui_kit::SharedString::new(rope.to_string()).to_string(),
                Instant::now(),
            )
            .unwrap();
        let save = ai.save_editor(None).unwrap();
        let (id, event) = reply(&worker, save);
        assert!(
            matches!(&event, AppEvent::EditorSaved(receipt) if receipt.outcome == SaveOutcome::NotApplied)
        );
        ai.apply(id, event);
        assert_eq!(std::fs::read(&path).unwrap(), original.as_bytes());
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), before.ino());
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );

        let start = original.find("正文").unwrap();
        rope.replace(start..start + "正文".len(), "文書 🦀");
        let saved = rope.to_string();
        ai.editor
            .as_mut()
            .unwrap()
            .edit(saved.clone(), Instant::now())
            .unwrap();
        let save = ai.save_editor(None).unwrap();
        let (id, event) = reply(&worker, save);
        let latest = format!("{saved}later typing λ");
        ai.editor
            .as_mut()
            .unwrap()
            .edit(latest.clone(), Instant::now())
            .unwrap();
        assert!(
            matches!(&event, AppEvent::EditorSaved(receipt) if receipt.outcome == SaveOutcome::Applied)
        );
        ai.apply(id, event);
        assert_eq!(std::fs::read(&path).unwrap(), saved.as_bytes());
        assert_eq!(ai.editor.as_ref().unwrap().text, latest);
        assert!(!ai.editor.as_ref().unwrap().can_leave());
        let recovery = ai.recover_editor().unwrap();
        let (id, event) = reply(&worker, recovery);
        ai.apply(id, event);
        assert!(ai.editor.as_ref().unwrap().can_leave());
        worker.shutdown().unwrap();

        let mut worker = fixture.worker();
        let (_, event) = reply(
            &worker,
            (Uuid::new_v4(), AppCommand::OpenEditor("plan.md".into())),
        );
        let AppEvent::Editor(view) = event else {
            panic!("reopened editor");
        };
        assert_eq!(view.record.text, latest);
        assert_eq!(view.saved.as_deref(), Some(saved.as_str()));
        worker.shutdown().unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn final_quit_drains_admitted_save_before_returning_timed_future() {
        let fixture = EditorFixture::new();
        let path = fixture.0.join("vault/a.md");
        std::fs::write(&path, "original\r\n").unwrap();
        let mut worker = fixture.worker();
        let mut ai = ready();
        let open = ai.open_editor("a.md".into());
        let (id, event) = reply(&worker, open);
        ai.apply(id, event);
        ai.editor
            .as_mut()
            .unwrap()
            .edit("admitted λ\r\n".into(), Instant::now())
            .unwrap();
        let (id, save) = ai.save_editor(None).unwrap();
        worker.submit(id, save).unwrap();
        let (joined, receipt) = std::sync::mpsc::channel();
        let future = final_quit(
            move || {
                let result = worker.shutdown();
                let mut saved = false;
                while let Some((actual, event)) = worker.try_event() {
                    if actual == id && matches!(event, AppEvent::EditorSaved(_)) {
                        saved = true;
                    }
                }
                joined.send((result, saved)).unwrap();
            },
            std::future::pending(),
        );
        let (result, saved) = receipt
            .try_recv()
            .expect("admitted Save must drain before returning the timed quit future");
        let mut future = std::pin::pin!(future);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(future.as_mut(), &mut cx).is_pending());
        result.unwrap();
        assert!(saved);
        assert_eq!(std::fs::read(&path).unwrap(), "admitted λ\r\n".as_bytes());
    }

    #[test]
    fn composer_invalidates_only_search_and_history_filters_only_chat() {
        use brn_workflow::library::SearchResults;
        let mut state = ready();
        let (old, _) = search_command(&mut state, "old".into()).unwrap();
        state.composer_changed();
        state.composer_changed();
        let (current, _) = search_command(&mut state, "current".into()).unwrap();
        state.apply(
            old,
            AppEvent::Search(SearchResults {
                hits: vec![],
                keyword_only: false,
            }),
        );
        assert!(state.search.is_none());
        let selected = Uuid::new_v4();
        state.navigate(Some(selected));
        state.apply(
            current,
            AppEvent::Search(SearchResults {
                hits: vec![],
                keyword_only: true,
            }),
        );
        assert!(state.search.as_ref().unwrap().keyword_only);
        state.composer_changed();
        assert!(state.search.is_none());
        assert_eq!(state.conversation, Some(selected));
    }
    #[test]
    fn native_ask_routes_exact_uuid_frozen_model_and_disables_concurrent_ask() {
        let mut state = ready();
        let (id, command) = ask_command(&mut state, "λ native question".into()).unwrap();
        assert!(
            matches!(command, AppCommand::Ask(request) if request.id == id &&
                request.selection.model == "explicit" && request.question == "λ native question"
                && request.effort == Some(ReasoningEffort::High))
        );
        assert!(ask_command(&mut state, "concurrent".into()).is_none());
        assert!(search_command(&mut state, "local".into()).is_some());
    }
    #[test]
    fn native_actions_disabled_until_ready_and_vault_but_history_is_independent() {
        let mut state = AiState::default();
        assert!(ask_command(&mut state, "q".into()).is_none());
        assert!(search_command(&mut state, "q".into()).is_none());
        state.ready = true;
        assert!(search_command(&mut state, "q".into()).is_none());
        assert!(state.navigate(Some(Uuid::new_v4())).is_some());
        state.vault_bound = true;
        assert!(search_command(&mut state, "".into()).is_none());
        assert!(search_command(&mut state, "q".into()).is_some());
    }
    #[test]
    fn native_download_decline_routes_only_fresh_owner_command_not_startup_loading() {
        let mut state = ready();
        let target = PathBuf::from("/synthetic/install-target");
        let (id, command) = download_command(&mut state, false, target.clone());
        assert_eq!(state.download, Some(id));
        assert!(
            matches!(command, AppCommand::DownloadModel { consent: false, target: path } if path == target)
        );
        assert!(state.model_prompt.is_none());
        assert!(!state.model_installed);
    }
    #[test]
    fn native_notice_change_without_queued_worker_event_requests_redraw() {
        let mut state = ready();
        let mut message = "Opening".to_string();
        state.notice = "Submission failed".into();
        assert!(project_notice(&state, &mut message));
        assert_eq!(message, "Submission failed");
        assert!(!project_notice(&state, &mut message));
    }
}
