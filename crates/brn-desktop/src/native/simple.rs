use super::theme::color;
use super::*;
use crate::ai::{Pending, provider_name, slot, turn_label};
use brn_workflow::{
    Provider, ReasoningEffort, Selection,
    app_worker::{AppCommand, AppEvent},
    chat_worker::AccountCommand,
    library::SearchMode,
};
use gpui_kit::{
    AnyElement,
    base::Disableable,
    component::{Selectable, WindowExt},
};

pub(super) struct Closed {
    result: brn_workflow::Result<()>,
    events: Vec<(Uuid, AppEvent)>,
}
pub(super) enum EditorTransition {
    Note(String),
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
    if !state.ready || !state.vault_bound || query.trim().is_empty() {
        return None;
    }
    Some(state.command(
        Pending::Search {
            generation: state.search_generation,
        },
        AppCommand::Search {
            query,
            mode: SearchMode::Hybrid,
            limit: 10,
        },
    ))
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
        if self.simple_note_path.as_deref() == Some(&path)
            && self.ai.as_ref().unwrap().editor.is_some()
        {
            self.centre_tab = CentreTab::Document;
            cx.notify();
            return;
        }
        self.simple_leave(EditorTransition::Note(path), cx);
    }
    fn simple_open_note(&mut self, path: String, cx: &mut Context<Self>) {
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
        self.simple_transition = Some(transition);
        self.simple_progress_transition(cx);
        cx.notify();
    }
    fn simple_progress_transition(&mut self, cx: &mut Context<Self>) {
        if self.simple_transition.is_none() {
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
        match self.simple_transition.take().unwrap() {
            EditorTransition::Note(path) => self.simple_open_note(path, cx),
            EditorTransition::Hide => {
                let ai = self.ai.as_mut().unwrap();
                ai.note_generation = ai.note_generation.wrapping_add(1);
                ai.editor = None;
                self.simple_note_path = None;
                self.open_doc = None;
                self.centre_tab = CentreTab::Chat;
            }
            EditorTransition::Close(route) => self.begin_close(route, cx),
        }
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
    fn open_login(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        self.login_dialog = Some(id);
        let desktop = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            let target = desktop.downgrade();
            let cancel_target = target.clone();
            let ai = desktop.read(cx).ai.as_ref().unwrap();
            let mut body = div()
                .flex()
                .flex_col()
                .gap_3()
                .child("Explicit connection · waiting for authorization");
            if let Some(login) = &ai.login
                && login.operation == id
                && let Some(prompt) = &login.prompt
            {
                body = body
                    .child(prompt.verification_uri.clone())
                    .child(prompt.user_code.clone());
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
                cost: "Approximately 87 MiB network and storage; optional local retrieval model"
                    .into(),
                destination: self.path.join("models/minilm"),
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
                            this.model_decision(false, this.path.join("models/minilm"), cx)
                        });
                        window.close_dialog(cx);
                    })))
                .on_close(move |_, _, cx| {
                    let _ = close.update(cx, |this, cx| {
                        if this.ai.as_ref().unwrap().model_prompt.is_some() {
                            this.model_decision(false, this.path.join("models/minilm"), cx);
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
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let mut list = div()
            .id("history-rail-list")
            .track_scroll(&self.history_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .p_2()
            .child(
                Button::new("new-session")
                    .label("+ New chat")
                    .disabled(!ai.ready)
                    .on_click(cx.listener(|this, _, _, cx| this.simple_history(None, cx))),
            );
        for conversation in &ai.conversations {
            let id = conversation.id;
            list = list.child(
                Button::new(format!("conversation-{id}"))
                    .label(format!(
                        "{} · {} turns",
                        compact_title(&conversation.title),
                        conversation.turns
                    ))
                    .selected(ai.conversation == Some(id))
                    .on_click(cx.listener(move |this, _, _, cx| this.simple_history(Some(id), cx))),
            );
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
                    .child(Button::new("settings-footer").label("⚙ Settings").on_click(
                        cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                    )),
            )
            .into_any_element()
    }
    pub(super) fn render_simple_vault(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let mut list = div()
            .id("vault-rail-list")
            .track_scroll(&self.vault_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_2()
            .p_2()
            .child("Markdown notes")
            .child(
                ai.vault_root
                    .as_ref()
                    .map_or("No vault selected".into(), |p| {
                        spaced_identifier(&p.display().to_string())
                    }),
            )
            .child(
                Button::new("choose-vault")
                    .label("Choose vault…")
                    .disabled(!ai.ready || ai.vault_bound || self.choosing_file)
                    .on_click(cx.listener(|this, _, _, cx| this.simple_choose_vault(cx))),
            )
            .child(
                Button::new("refresh-vault")
                    .label("Refresh saved notes")
                    .disabled(!ai.ready || !ai.vault_bound)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.simple_command(Pending::Refresh, AppCommand::Refresh, cx)
                    })),
            )
            .child(ai.model_state.clone());
        if !ai.editors.is_empty() {
            list = list.child("Registered / recovered buffers");
            for record in &ai.editors {
                let path = record.path.clone();
                let unsaved = record.text != record.baseline_text;
                list = list.child(
                    Button::new(format!("recovered-{}", record.path))
                        .label(format!(
                            "{}{}",
                            compact_title(&record.path),
                            if unsaved { " · unsaved" } else { "" }
                        ))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.simple_note(path.clone(), cx)),
                        ),
                );
            }
        }
        if let Some((_, embedded, total)) = ai.indexing {
            list = list.child(format!("Indexed embeddings: {embedded}/{total}"));
        }
        if let Some(report) = &ai.refresh {
            list = list.child(format!(
                "Refresh: {} added · {} updated · {} removed · {} unchanged",
                report.added, report.updated, report.removed, report.unchanged
            ));
            for unreadable in &report.unreadable {
                list = list.child(format!(
                    "Unreadable: {} · {}",
                    unreadable.path, unreadable.reason
                ));
            }
        }
        for note in &ai.notes {
            let path = note.path.clone();
            list = list.child(
                Button::new(format!("note-{}", note.path))
                    .label(compact_title(&note.title))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.simple_note(path.clone(), cx)),
                    ),
            );
        }
        if let Some(cursor) = &ai.next_cursor {
            let cursor = cursor.clone();
            list = list.child(
                Button::new("more-notes")
                    .label("More notes")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.simple_command(
                            Pending::Notes { append: true },
                            AppCommand::Notes {
                                folder: None,
                                cursor: Some(cursor.clone()),
                            },
                            cx,
                        )
                    })),
            );
        }
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
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let leaving = self.simple_transition.is_some() || self.closing.is_some() || self.closed;
        let mut body = div().flex_1().min_h(px(0.)).flex().flex_col().gap_2().p_3();
        if let Some(editor) = &ai.editor {
            body = body.child(editor.status());
            if let Some(error) = &editor.error {
                body = body.child(error.clone());
            }
            body = body
                .child(div().key_context("MarkdownNote").flex_1().min_h(px(0.)).child(
                    Editor::new(&self.note_editor).h_full()
                        .disabled(leaving || editor.replacing()).aria_label("Markdown note editor")))
                .child(div().flex().flex_wrap().gap_2()
                    .child(Button::new("simple-save").label("Save to Markdown (Cmd-S)")
                        .disabled(leaving || !editor.can_save())
                        .on_click(cx.listener(|this, _, _, cx| this.simple_save(None, cx))))
                    .child(Button::new("simple-flush-recovery").label("Flush / retry recovery")
                        .disabled(editor.pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor) = this.ai.as_mut().unwrap().editor.as_mut() { editor.retry_recovery(); }
                            if let Some(command) = this.ai.as_mut().unwrap().recover_editor() { this.simple_send(command, cx); }
                        })))
                    .child(Button::new("simple-observe-disk").label("Observe disk")
                        .disabled(leaving || editor.pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(command) = this.ai.as_mut().unwrap().refresh_editor() { this.simple_send(command, cx); }
                        })))
                    .child(Button::new("simple-compare").label("Compare baseline / local / disk")
                        .on_click(cx.listener(|this, _, window, cx| this.simple_compare(window, cx))))
                    .child(Button::new("simple-reload").label("Reload reviewed disk…")
                        .disabled(leaving || editor.reload_request().is_none())
                        .on_click(cx.listener(|this, _, window, cx| this.simple_confirm_reload(window, cx))))
                    .child(Button::new("cancel-simple-leave").label("Cancel pending close / navigation")
                        .disabled(self.simple_transition.is_none())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_transition = None;
                            this.ai.as_mut().unwrap().notice = "Close / navigation cancelled; editor retained.".into();
                            cx.notify();
                        }))))
                .child(div().flex().gap_2()
                    .child(Input::new(&self.note_path).aria_label("Unused vault-relative .md copy destination"))
                    .child(Button::new("simple-save-copy").label("Save Copy")
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
            body = body.child(
                ai.note_error
                    .clone()
                    .unwrap_or("Opening Markdown / recovery buffer…".into()),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(self.simple_note_path.clone().unwrap_or_default()),
                    )
                    .child(
                        Button::new("close-document")
                            .label("Close")
                            .compact()
                            .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                    ),
            )
            .child(body)
            .into_any_element()
    }
    pub(super) fn render_simple_chat(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let mut body = div()
            .id("chat-transcript")
            .track_scroll(&self.chat_scroll)
            .vertical_scrollbar(&self.chat_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_3()
            .p_3();
        if ai.turns.is_empty() {
            body = body.child("Select a provider, model and reasoning effort in Settings, then ask about saved notes. AI has read-only tools.");
        }
        for turn in ai.display_turns() {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "{} / {} · effort: {} · {}",
                        turn.provider,
                        turn.model,
                        turn.effort
                            .as_deref()
                            .unwrap_or("unavailable in older history"),
                        turn_label(turn)
                    ))
                    .child(turn.question.clone())
                    .child(turn.answer.clone()),
            );
            if let Some(code) = &turn.error_code {
                body = body.child(format!("Safe failure category: {code}"));
            }
        }
        if let Some(turn) = &ai.unsaved {
            let partial = turn.answer.clone();
            body = body
                .child(format!(
                    "{} / {} · effort: {} · Failed · in-memory partial · finalization not acknowledged",
                    turn.provider, turn.model, turn.effort.as_deref().unwrap_or("unavailable")
                ))
                .child(turn.question.clone())
                .child(turn.answer.clone())
                .child("Copy this partial before closing/restarting. Further Ask is blocked until this unfinalized workspace is reopened.")
                .child(Button::new("copy-unfinalized-partial").label("Copy in-memory partial")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(partial.clone()));
                    })));
        }
        if let Some(active) = ai.display_active() {
            body = body
                .child(format!(
                    "{} / {} · effort: {} · {}",
                    provider_name(active.request.selection.provider),
                    active.request.selection.model,
                    active
                        .request
                        .effort
                        .map(ReasoningEffort::as_str)
                        .unwrap_or("unavailable"),
                    if active.stopping {
                        "Stopping (not finalized)"
                    } else {
                        "Streaming (provisional)"
                    }
                ))
                .child(active.request.question.clone())
                .child(active.partial.clone());
            if let Some(tool) = &active.tool {
                body = body.child(format!("Tool started: {tool}"));
            }
        }
        if let Some(results) = &ai.search {
            body = body.child(if results.keyword_only {
                "Search: keyword-only (no installed model)"
            } else {
                "Search: hybrid"
            });
            for (i, hit) in results.hits.iter().enumerate() {
                let path = hit.path.clone();
                body = body
                    .child(
                        Button::new(format!("simple-hit-{i}"))
                            .label(format!(
                                "{} · bytes {}..{}",
                                hit.path, hit.start_byte, hit.end_byte
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.simple_note(path.clone(), cx)
                            })),
                    )
                    .child(hit.quote.clone());
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.paper))
            .child(body)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .border_t_1()
                    .border_color(color(p.line))
                    .bg(color(p.panel))
                    .child(
                        Editor::new(&self.query)
                            .h(px(100.))
                            .aria_label("Question about saved notes"),
                    )
                    .child(
                        ai.selection
                            .as_ref()
                            .map_or("No explicit provider/model selected".into(), |s| {
                                format!("{} / {}", provider_name(s.provider), s.model)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("search")
                                    .label("Search")
                                    .disabled(!ai.ready || !ai.vault_bound)
                                    .on_click(cx.listener(|this, _, _, cx| this.simple_search(cx))),
                            )
                            .child(
                                Button::new("ask")
                                    .label("Ask")
                                    .disabled(
                                        !ai.can_ask()
                                            || self.closing.is_some()
                                            || self.close_failed,
                                    )
                                    .on_click(cx.listener(|this, _, _, cx| this.simple_ask(cx))),
                            )
                            .child(
                                Button::new("stop")
                                    .label("Stop")
                                    .disabled(ai.active.is_none())
                                    .on_click(cx.listener(|this, _, _, cx| this.simple_stop(cx))),
                            ),
                    ),
            )
            .into_any_element()
    }
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
