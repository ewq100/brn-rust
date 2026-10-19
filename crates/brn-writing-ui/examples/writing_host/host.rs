//! Thin, fixed-document composition. All authority/run/evidence work stays in existing services.
use super::fixture;
use brn_threads_app::*;
use brn_writing_ui::{layout, native::*, theme};
use gpui_kit::base::input::InputEvent;
use gpui_kit::{
    App, Context, FocusHandle, Focusable, IntoElement, Render, Subscription, TestSupportExt,
    Window, div, prelude::*, rgb,
};
use std::collections::BTreeMap;

pub struct WritingHost {
    pub workspace: Workspace,
    runtime: tokio::runtime::Runtime,
    pub panel: WritingPanel,
    pub reference: Record,
    records: Vec<Record>,
    history: Vec<Receipt>,
    candidates: Vec<Prepared>,
    buffers: Vec<EditSession>,
    threads: BTreeMap<String, String>,
    reviews: BTreeMap<String, WritingCandidateReview>,
    requests: BTreeMap<String, WritingSnapshot>,
    pub notice: String,
    next: u64,
    pub last_request: Option<String>,
    pub last_run: Option<String>,
    pub model_requests: String,
    _subscriptions: Vec<Subscription>,
    // No background job outlives an action. Drop Workspace/executor before this own scratch root.
    _root: tempfile::TempDir,
}
impl WritingHost {
    pub fn new(mut f: fixture::Fixture, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = WritingPanel::new(window, cx);
        panel
            .open_note(&mut f.workspace, &f.note.id, None, window, cx)
            .expect("Synthetic Note opens through shared lifecycle");
        panel.margin_mode = layout::MarginMode::Stacked;
        let subscriptions = vec![
            cx.subscribe_in(
                &panel.review_editor,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.panel
                            .review
                            .buffer_changed(editor.read(cx).value().as_ref());
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &panel.note_editor,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) && this.panel.note_input_active(true) {
                        // This shared callback writes begin_edit/update_buffer synchronously,
                        // before any derived read, selection update or notify.
                        let text = editor.read(cx).value().to_string();
                        if let Err(error) = this.panel.note_changed(&mut this.workspace, text) {
                            this.notice = error.to_string();
                        }
                        this.panel.sync_read_text(cx);
                        this.panel.track_selection(cx);
                        cx.notify();
                    }
                },
            ),
            cx.observe_in(&panel.note_editor, window, |this, _, _, cx| {
                this.panel.track_selection(cx);
                cx.notify();
            }),
            cx.observe_in(&panel.note_read_view, window, |this, _, _, cx| {
                if this.panel.note.is_reading() {
                    this.panel.track_selection(cx);
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &panel.comment_input,
                window,
                |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.panel.comment_operation = None;
                        cx.notify();
                    }
                },
            ),
        ];
        Self {
            workspace: f.workspace,
            runtime: tokio::runtime::Builder::new_current_thread().enable_all().build().expect("offline script executor"),
            panel,
            reference: f.reference,
            records: vec![], history: vec![], candidates: vec![], buffers: vec![],
            threads: BTreeMap::new(), reviews: BTreeMap::new(), requests: BTreeMap::new(),
            notice: "Disposable synthetic data; all AI replies are scripted offline. No provider or file picker is connected.".into(),
            next: 0, last_request: None, last_run: None, model_requests: String::new(),
            _subscriptions: subscriptions, _root: f.root,
        }
    }
    fn key(&mut self, purpose: &str) -> String {
        self.next += 1;
        format!("b4-host-{purpose}-{}", self.next)
    }
    fn note(&self) -> AppResult<String> {
        self.panel
            .open_note_id()
            .map(str::to_owned)
            .ok_or_else(|| AppError::Invalid("No working document".into()))
    }
    fn context(&self) -> AppResult<ActiveWritingContext> {
        let context = self.workspace.active_writing_context(&self.note()?, None)?;
        if context.scope.is_some() {
            return Err(AppError::Invalid(
                "This development host displays only its general synthetic document".into(),
            ));
        }
        Ok(context)
    }
    /// No second cache: a fresh token-fenced checked display batch per render.
    fn inspect(&mut self) -> AppResult<()> {
        self.reviews.clear();
        self.requests.clear();
        self.threads.clear();
        let token = self.workspace.store.change_token()?;
        let context = self.context()?;
        self.records = self.workspace.records()?;
        self.history = self.workspace.history()?;
        self.candidates = self.workspace.candidates()?;
        self.buffers = self.workspace.store.recovery_buffers()?;
        let mut shown = self.candidates.clone();
        if let Some(review) = self.panel.review.as_ref() {
            shown.retain(|candidate| candidate.operation != review.candidate.operation);
            shown.push(review.candidate.clone());
        }
        let (observed, checked) =
            self.workspace
                .writing_context_review(&context.note, None, &shown)?;
        if observed != token || self.workspace.store.change_token()? != token {
            return Err(AppError::Unavailable(
                "Synthetic context changed during inspection".into(),
            ));
        }
        self.reviews = checked.candidates;
        self.requests = checked.requests;
        if let Some(thread) = context.conversation {
            self.threads.insert(context.note, thread);
        }
        Ok(())
    }
    fn view(&self) -> WritingView<'_, Self> {
        WritingView {
            host: self,
            panel: &self.panel,
            workspace: &self.workspace,
            displayed_scope: None,
            reference_importing: false,
            records: &self.records,
            history: &self.history,
            candidates: &self.candidates,
            buffers: &self.buffers,
            writing_threads: &self.threads,
            writing_reviews: &self.reviews,
            writing_requests: &self.requests,
            worker: None,
            progress: "",
            centered: true,
            edit_note_blocked: |_| {
                Some("This development host supports scripted whole-note rewrites")
            },
            chat_card_stop: |_| None,
            assets_in_chat: |_, _| std::sync::Arc::new(BTreeMap::new()), // Fixture imports have no assets.
            latest_run: |records, receipts, thread| {
                receipts
                    .iter()
                    .rev()
                    .flat_map(|r| &r.writes)
                    .filter(|w| {
                        w.before.is_none()
                            && matches!(&w.after.data, RecordData::Run(r) if r.thread == thread)
                    })
                    .find_map(|w| records.iter().find(|r| r.id == w.after.id && !r.archived))
            },
            dispatch: Self::dispatch,
        }
    }
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AppResult<()> {
        self.context()?;
        self.panel.refresh_note(&mut self.workspace, window, cx)
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AppResult<()> {
        self.context()?;
        match self.panel.save_note(&mut self.workspace, window, cx)? {
            Some(SaveOutcome::Saved(_)) | None => Ok(()),
            Some(other) => Err(AppError::Invalid(format!(
                "Save refused; edit retained: {other:?}"
            ))),
        }
    }
    fn comment(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AppResult<()> {
        let context = self.context()?;
        let (note, version, range, quote) = self
            .panel
            .comment_anchor
            .clone()
            .ok_or_else(|| AppError::Invalid("Select a saved passage".into()))?;
        if note != context.note
            || self
                .panel
                .note
                .as_ref()
                .is_none_or(|open| open.session().is_some())
        {
            return Err(AppError::Invalid(
                "Save before commenting; draft retained".into(),
            ));
        }
        let thread = context
            .conversation
            .ok_or_else(|| AppError::Invalid("No active writing conversation".into()))?;
        let key = self.key("comment");
        self.workspace.add_writing_comment(
            &key,
            &WritingCommentInput {
                note,
                version,
                thread,
                range: (range.start, range.end),
                quote,
                body: self.panel.comment_input.read(cx).value().to_string(),
            },
        )?;
        self.panel.comment_anchor = None;
        self.panel
            .comment_input
            .update(cx, |editor, cx| editor.set_value("", window, cx));
        self.panel.sync_note_marks(&self.workspace, cx);
        self.notice =
            "Saved passage comment; Rewrite captures it through the shared service.".into();
        Ok(())
    }
    fn rewrite(&mut self, cx: &Context<Self>) -> AppResult<()> {
        let context = self.context()?;
        let open = self
            .panel
            .note
            .as_ref()
            .ok_or_else(|| AppError::Invalid("No document".into()))?;
        if open.session().is_some() {
            return Err(AppError::Invalid("Save before rewriting".into()));
        }
        let version = open.record().version;
        let thread = context
            .conversation
            .ok_or_else(|| AppError::Invalid("No active writing conversation".into()))?;
        let comments = self
            .workspace
            .writing_comments(&context.note)?
            .into_iter()
            .filter(|c| !c.addressed && !self.panel.excluded_comments.contains(&c.record.id))
            .map(|c| c.record.id)
            .collect::<Vec<_>>();
        let feedback = self.panel.general_feedback.read(cx).value().to_string();
        let key = self.key("request");
        let snapshot = self.workspace.snapshot_writing_request_with_feedback(
            &key,
            &context.note,
            version,
            &thread,
            &comments,
            &feedback,
        )?;
        self.run_saved(&snapshot.request_message)
    }
    fn run_saved(&mut self, message: &str) -> AppResult<()> {
        self.context()?;
        let start = self
            .panel
            .rewrite_run_start(&self.workspace, message, None, vec![])
            .map_err(|e| AppError::Invalid(format!("Shared writing start refused: {e:?}")))?;
        let snapshot = self
            .workspace
            .writing_request_in_context(message, &self.note()?, None)?;
        let model = fixture::model(&snapshot, &self.reference)?;
        let observed = model.clone();
        let prepared = self.workspace.prepare_thread_run(start)?;
        // One bounded scripted provider, same runtime/lease/tools/finish path as BRN.
        // Synchronous on purpose: no independent worker or child can outlive this scratch host.
        let result = self
            .runtime
            .block_on(brn_ai::threads::run_prepared_scripted(
                prepared.prepared,
                model,
            ))
            .map_err(|e| AppError::Invalid(e.to_string()))?;
        self.last_request = Some(message.into());
        self.last_run = Some(result.run_id);
        self.model_requests = serde_json::to_string(&observed.requests())?;
        self.notice = format!(
            "Scripted offline run: {:?}. Review any proposal before Apply.",
            result.answer.terminal
        );
        Ok(())
    }
    fn review(
        &mut self,
        candidate: Prepared,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AppResult<()> {
        self.inspect()?;
        if !self.reviews.contains_key(candidate.operation.as_str()) {
            return Err(AppError::Invalid(
                "Proposal is unavailable in this displayed context".into(),
            ));
        }
        let note = self.note()?;
        if !candidate
            .request
            .writes
            .iter()
            .any(|write| write.id == note && matches!(write.data, RecordData::Note(_)))
        {
            return Err(AppError::Invalid(
                "Proposal has no matching document".into(),
            ));
        }
        self.panel
            .open_review(&self.workspace, candidate, Some(note), window, cx)?;
        Ok(())
    }
    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AppResult<()> {
        self.inspect()?;
        let review = self
            .panel
            .review
            .as_ref()
            .ok_or_else(|| AppError::Invalid("Open the exact proposal first".into()))?;
        if !self
            .reviews
            .contains_key(review.candidate.operation.as_str())
        {
            return Err(AppError::Invalid(
                "Proposal is no longer visible in this document/context".into(),
            ));
        }
        let candidate = self.panel.review.exact_candidate()?;
        // Fresh service validation; the display batch is never an Apply grant.
        self.workspace
            .writing_candidate_review(candidate.operation.as_str(), None)?;
        let comments = self
            .panel
            .review
            .comments_to_address(candidate.operation.as_str());
        let result = self.workspace.review_apply_selected_comments(
            candidate.operation.as_str(),
            None,
            &comments,
        )?;
        match &result.outcome {
            ApplyOutcome::Applied(receipt) => {
                let note = self.note()?;
                if let Some(request) = &self.last_request {
                    self.panel
                        .applied_rewrite
                        .insert(note, (request.clone(), receipt.operation.as_str().into()));
                }
                self.panel.review.completed(candidate.operation.as_str());
                self.reload(window, cx)?;
                self.notice =
                    "Applied the exact reviewed proposal; durable Undo is available.".into();
            }
            other => self.notice = format!("Apply refused; proposal retained: {other:?}"),
        }
        if let Some(pending) = result.pending_updates() {
            self.notice = pending;
        }
        Ok(())
    }
    pub fn dispatch(
        &mut self,
        request: WritingRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.handle(request, window, cx) {
            self.notice = error.to_string();
        }
        cx.notify();
    }
    fn handle(
        &mut self,
        request: WritingRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AppResult<()> {
        match request {
            WritingRequest::Save => self.save(window, cx)?,
            WritingRequest::Reload => { self.context()?; let note = self.note()?; self.panel.open_note(&mut self.workspace, &note, None, window, cx)?; },
            WritingRequest::TrackSelection => self.panel.track_selection(cx),
            WritingRequest::CommentSelection | WritingRequest::CommentOrSave => {
                self.context()?;
                if self.panel.note.as_ref().is_some_and(|open| open.session().is_some()) { return Err(AppError::Invalid("Save before selecting a saved passage".into())); }
                self.panel.track_selection(cx);
                let open = self.panel.note.as_ref().unwrap();
                if let Some((range, quote)) = self.panel.selection.clone() { self.panel.comment_anchor = Some((open.record().id.clone(), open.record().version, range, quote)); }
                self.panel.toggle_comments(true);
                self.panel.comment_input.read(cx).focus_handle(cx).focus(window, cx);
            }
            WritingRequest::PostComment => self.comment(window, cx)?,
            WritingRequest::CancelComment => self.panel.comment_anchor = None,
            WritingRequest::Rewrite => self.rewrite(cx)?,
            WritingRequest::Retry(message) => self.run_saved(&message)?,
            WritingRequest::OpenReview(candidate) => self.review(candidate, window, cx)?,
            WritingRequest::ApplyReview => self.apply(window, cx)?,
            WritingRequest::DismissReview => {
                if let Some(review) = self.panel.review.as_ref() {
                    let operation = review.candidate.operation.as_str().to_owned();
                    self.workspace.review_dismiss(&operation)?;
                    self.panel.review.completed(&operation);
                }
            }
            WritingRequest::CardDismiss(candidate) => self.workspace.review_dismiss(candidate.operation.as_str())?,
            WritingRequest::KeepReview => {
                if self.panel.review.is_edited() { self.panel.prepare_review_edit(&mut self.workspace, window, cx)?; }
                self.panel.review.keep()?;
            }
            WritingRequest::StartReviewEdit => {
                if self.panel.review.begin_edit() {
                    self.panel.review_editor.update(cx, |editor, cx| editor.focus(window, cx));
                }
            }
            WritingRequest::CancelReviewEdit => self.panel.cancel_review_edit(window, cx),
            WritingRequest::PrepareReviewEdit => self.panel.prepare_review_edit(&mut self.workspace, window, cx)?,
            WritingRequest::PrepareReviewSelection => self.panel.prepare_review_selection(&mut self.workspace, window, cx)?,
            WritingRequest::ToggleGroup(id) => self.panel.review.toggle_group(&id),
            WritingRequest::ToggleReviewComment(id) => self.panel.review.toggle_comment(&id),
            WritingRequest::Undo(operation) => {
                self.context()?;
                match self.workspace.undo(&operation)? {
                    ApplyOutcome::Applied(_) => { self.panel.applied_rewrite.clear(); self.reload(window, cx)?; self.notice = "Undid the saved rewrite through the shared service.".into(); }
                    other => self.notice = format!("Undo refused: {other:?}"),
                }
            }
            WritingRequest::NoteReading(read) => { self.panel.set_note_reading(&mut self.workspace, read, cx)?; self.panel.sync_read_marks(&self.workspace, cx); }
            WritingRequest::ReviewReading(read) => { self.panel.review.set_reading(read); },
            WritingRequest::ToggleReviewInputs => self.panel.review.toggle_inputs(),
            WritingRequest::ToggleComments => { let show = !self.view().comments_open(); self.panel.toggle_comments(show); }
            WritingRequest::CollapseComments => self.panel.toggle_comments(false),
            WritingRequest::ToggleExcluded(id) => { if !self.panel.excluded_comments.remove(&id) { self.panel.excluded_comments.insert(id); } }
            WritingRequest::ToggleRequest(id) => { if !self.panel.expanded_requests.remove(&id) { self.panel.expanded_requests.insert(id); } }
            WritingRequest::HideApplied(note) => { self.panel.applied_rewrite.remove(&note); }
            WritingRequest::OpenSetup => {
                if self.panel.setup.as_ref().is_some_and(|d| d.value(self.panel.setup_brief.read(cx).value().as_ref()) != d.saved) { return Err(AppError::Invalid("Setup edits retained; Save or explicitly discard them first".into())); }
                self.load_setup(window, cx)?;
            }
            WritingRequest::ReloadSetup => self.load_setup(window, cx)?,
            WritingRequest::CloseSetup => self.panel.setup_open = false,
            WritingRequest::SaveSetup => {
                if self.panel.setup.is_none() { return Err(AppError::Invalid("Open setup first".into())); }
                let brief = self.panel.setup_brief.read(cx).value().to_string();
                let note = self.note()?; let key = self.key("setup");
                self.panel.setup.as_ref().unwrap().save(&mut self.workspace, &key, Some(&note), None, &brief)?;
                self.load_setup(window, cx)?;
            }
            WritingRequest::RecheckSetup => {
                let fresh = setup::SetupDraft::load(&self.workspace, &self.note()?, None)?;
                if self.panel.note.as_ref().is_none_or(|open| open.record().version != fresh.target.note_version) {
                    return Err(AppError::Invalid("Open the current saved document before rechecking".into()));
                }
                if let Some(draft) = &mut self.panel.setup { draft.keep_edits_after_recheck(fresh)?; }
            }
            WritingRequest::Noop => (),
            _ => self.notice = "Unavailable in this development host. Supported: saved passage comments, scripted whole-note Rewrite/Retry, exact Review/Apply/Dismiss/Undo, Read/Write and brief Save. Use BRN for other actions.".into(),
        }
        Ok(())
    }
    fn load_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AppResult<()> {
        self.context()?;
        let draft = setup::SetupDraft::load(&self.workspace, &self.note()?, None)?;
        self.panel.setup_brief.update(cx, |input, cx| {
            input.set_value(draft.saved.brief.as_deref().unwrap_or(""), window, cx)
        });
        self.panel.setup = Some(draft);
        self.panel.setup_open = true;
        Ok(())
    }
}
impl Focusable for WritingHost {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.panel.note_editor.read(cx).focus_handle(cx)
    }
}
impl Render for WritingHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Err(error) = self.inspect() {
            return div()
                .id("host-unavailable")
                .child(format!("Writing context unavailable: {error}"))
                .test_support()
                .into_any_element();
        }
        self.panel.sync_read_text(cx);
        self.panel.sync_read_marks(&self.workspace, cx);
        let p = theme::pal();
        let view = self.view();
        let controls = view.note_controls(cx);
        let body = view.note_tile(window, cx);
        div().id("writing-host").key_context("WritingHost")
            .on_action(cx.listener(|this, _: &Save, window, cx| this.dispatch(WritingRequest::Save, window, cx)))
            .on_action(cx.listener(|this, _: &CommentSelection, window, cx| this.dispatch(WritingRequest::CommentSelection, window, cx)))
            .on_action(cx.listener(|this, _: &Rewrite, window, cx| this.dispatch(WritingRequest::Rewrite, window, cx)))
            .on_action(cx.listener(|this, _: &AskAboutSelection, window, cx| this.dispatch(WritingRequest::AskSelection, window, cx)))
            .size_full().flex().flex_col().bg(rgb(p.tile))
            .child(div().p_2().text_sm().child("Writing UI development host · synthetic disposable documents · scripted offline provider"))
            .child(div().id("host-notice").p_2().child(self.notice.clone()).test_support())
            .child(div().flex().items_center().gap_1().p_1().children(controls))
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }
}

/// The small development host's bindings target the same public panel requests.
pub fn bindings() -> [gpui_kit::KeyBinding; 3] {
    [
        gpui_kit::KeyBinding::new("cmd-s", Save, Some("WritingHost")),
        gpui_kit::KeyBinding::new("cmd-/", CommentSelection, Some("WritingHost")),
        gpui_kit::KeyBinding::new("cmd-shift-r", Rewrite, Some("WritingHost")),
    ]
}
