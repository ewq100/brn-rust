use crate::layout::{self, LayoutState, Loaded, Rail, ResolvedLayout};
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, Menu, MenuItem,
    PathPromptOptions, ScrollHandle, Subscription, Task, WeakEntity, Window, WindowBounds,
    WindowOptions,
    component::{
        Root, TitleBar,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState, TextareaState},
        scroll::ScrollableElement,
    },
    div, point,
    prelude::*,
    px, size,
};
gpui_kit::actions!(
    brn,
    [
        Quit,
        ToggleHistory,
        ToggleVault,
        ToggleFocus,
        FocusComposer,
        NewChat,
        OpenSettings,
        CancelRunning,
        SaveNote
    ]
);
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

mod action_draft;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod action_draft_tests;
mod action_review;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod action_review_tests;
mod approval;
mod dashboard;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod dashboard_tests;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod dialog_tests;
mod draft;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod draft_link_tests;
mod findings;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod findings_tests;
mod inbox;
mod inbox_analysis;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod inbox_analysis_tests;
mod inbox_copy;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod inbox_copy_tests;
mod inbox_guided;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod inbox_navigation_tests;
mod inbox_reader;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod inbox_tests;
mod intake_preview;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod login_tests;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod provenance_tests;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod relationship_tests;
mod relationships;
mod review;
#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
mod scope_tests;
mod shell;
mod simple;
mod theme;
mod visual;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocRef {
    SavedNote,
    Evidence,
    Proposal(Uuid),
    Activity,
    Dashboard,
    Findings,
    Inbox,
    Draft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CentreTab {
    Document,
    Chat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseRoute {
    Window,
    Quit,
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

fn review_title_state(window: &mut Window, cx: &mut Context<TextareaState>) -> TextareaState {
    TextareaState::new(window, cx)
        .placeholder("Proposal title")
        .auto_grow(1, 4)
}
struct Desktop {
    app_worker: Option<brn_workflow::app_worker::AppWorker>,
    ai: Option<crate::ai::AiState>,
    simple_note_path: Option<String>,
    simple_transition: Option<simple::EditorTransition>,
    simple_editor_generation: Option<u64>,
    evidence_editor_generation: Option<u64>,
    login_dialog: Option<Uuid>,
    closing: Option<(CloseRoute, std::sync::mpsc::Receiver<simple::Closed>)>,
    closed: bool,
    close_failed: bool,
    layout_task: Option<Task<()>>,
    query: Entity<EditorState>,
    note_editor: Entity<EditorState>,
    evidence_editor: Entity<EditorState>,
    provenance_open: bool,
    provenance_snapshot: Option<brn_workflow::knowledge::NoteProvenance>,
    provenance_quotes: Vec<Entity<EditorState>>,
    provenance_scroll: ScrollHandle,
    document_scroll: ScrollHandle,
    saved_links: relationships::SavedLinksPane,
    relationships: relationships::RelationshipsPane,
    findings: findings::FindingsPane,
    inbox: inbox::InboxPane,
    dashboard: dashboard::DashboardPane,
    action_editors: action_review::ActionEditors,
    initial_action: action_draft::ActionInputs,
    review_editor: Entity<EditorState>,
    review_title: Entity<TextareaState>,
    review_predecessor: Entity<TextareaState>,
    review_predecessor_proposal: Option<Uuid>,
    review_comment: Entity<EditorState>,
    review_comment_draft: Option<(Uuid, Option<Uuid>)>,
    review_comment_pending: Option<(Uuid, Uuid, String)>,
    review_member: usize,
    review_scroll: ScrollHandle,
    draft_title: Entity<TextareaState>,
    draft_path: Entity<TextareaState>,
    draft_editor: Entity<EditorState>,
    draft_link_target: Entity<TextareaState>,
    draft_link_label: Entity<TextareaState>,
    draft_link_proofs: Entity<EditorState>,
    draft_widget_id: Option<Uuid>,
    draft_scroll: ScrollHandle,
    note_path: Entity<InputState>,
    note_scroll: ScrollHandle,
    choosing_file: bool,
    path: PathBuf,
    layout: LayoutState,
    system_dark: bool,
    layout_note: Option<String>,
    resolved: ResolvedLayout,
    open_doc: Option<DocRef>,
    centre_tab: CentreTab,
    settings_requested: bool,
    history_scroll: ScrollHandle,
    vault_scroll: ScrollHandle,
    chat_scroll: ScrollHandle,
    dragging: Option<shell::divider::DividerDrag>,
    divider_focus: [FocusHandle; 3],
    focus_composer: bool,
    message: String,
    _subscriptions: Vec<Subscription>,
    _poll_task: Task<()>,
}
impl Desktop {
    fn new(
        path: PathBuf,
        config: brn_workflow::app::AppConfig,
        preferences: (LayoutState, Loaded),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let note_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value("")
        });
        let note_path =
            cx.new(|cx| InputState::new(window, cx).placeholder("Vault-relative .md destination"));
        let evidence_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let review_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value("")
        });
        let review_title = cx.new(|cx| review_title_state(window, cx));
        let review_predecessor = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Current knowledge path, e.g. policy.md")
                .auto_grow(1, 2)
        });
        let review_comment = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let draft_title = cx.new(|cx| review_title_state(window, cx));
        let draft_path = cx.new(|cx| draft::path_state(window, cx));
        let draft_editor = cx.new(|cx| draft::body_state(window, cx));
        let draft_link_target = cx.new(|cx| draft::link_target_state(window, cx));
        let draft_link_label = cx.new(|cx| draft::link_label_state(window, cx));
        let draft_link_proofs = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let draft_title_subscription = cx.subscribe_in(
            &draft_title,
            window,
            |this, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(draft) = &mut this.ai.as_mut().unwrap().draft {
                        draft.edit(
                            input.read(cx).value().to_string(),
                            draft.path.clone(),
                            draft.text.clone(),
                            draft.kind,
                        );
                    }
                    cx.notify();
                }
            },
        );
        let draft_link_target_subscription = cx.subscribe_in(
            &draft_link_target,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.capture_link_widgets(cx);
                    cx.notify();
                }
            },
        );
        let draft_link_label_subscription = cx.subscribe_in(
            &draft_link_label,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.capture_link_widgets(cx);
                    cx.notify();
                }
            },
        );
        let draft_path_subscription = cx.subscribe_in(
            &draft_path,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(draft) = &mut this.ai.as_mut().unwrap().draft {
                        if draft.prepared_request().is_some() {
                            let path = draft.path.clone();
                            if input.read(cx).value().as_ref() != path {
                                input.update(cx, |input, cx| input.set_value(path, window, cx));
                            }
                            return;
                        }
                        if draft.action.is_some() {
                            draft.edit_action_source_path(input.read(cx).value().to_string());
                            cx.notify();
                            return;
                        }
                        draft.edit(
                            draft.title.clone(),
                            input.read(cx).value().to_string(),
                            draft.text.clone(),
                            draft.kind,
                        );
                    }
                    cx.notify();
                }
            },
        );
        let draft_body_subscription = cx.subscribe_in(
            &draft_editor,
            window,
            |this, editor, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(draft) = &mut this.ai.as_mut().unwrap().draft {
                        draft.edit(
                            draft.title.clone(),
                            draft.path.clone(),
                            editor.read(cx).value().to_string(),
                            draft.kind,
                        );
                    }
                    cx.notify();
                }
            },
        );
        let review_subscription = cx.subscribe_in(
            &review_editor,
            window,
            |this, editor, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = editor.read(cx).value().to_string();
                    let editable = this.ai.as_ref().unwrap().review_editable()
                        && this.simple_transition.is_none()
                        && this.closing.is_none()
                        && !this.closed
                        && !this.close_failed;
                    if let Some(review) = &mut this.ai.as_mut().unwrap().review {
                        let result = if editable {
                            review.edit_text(this.review_member, text, Instant::now())
                        } else {
                            Err("Waiting for review acknowledgement or navigation")
                        };
                        if let Err(error) = result {
                            this.ai.as_mut().unwrap().notice = error.into();
                            this.sync_review_widgets(window, cx);
                        }
                    }
                    cx.notify();
                }
            },
        );
        let review_title_subscription = cx.subscribe_in(
            &review_title,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let title = input.read(cx).value().to_string();
                    let editable = this.ai.as_ref().unwrap().review_editable()
                        && this.simple_transition.is_none()
                        && this.closing.is_none()
                        && !this.closed
                        && !this.close_failed;
                    if let Some(review) = &mut this.ai.as_mut().unwrap().review {
                        let result = if editable {
                            review.edit_title(title, Instant::now())
                        } else {
                            Err("Waiting for review acknowledgement or navigation")
                        };
                        if let Err(error) = result {
                            this.ai.as_mut().unwrap().notice = error.into();
                            this.sync_review_widgets(window, cx);
                        }
                    }
                    cx.notify();
                }
            },
        );
        let note_subscription = cx.subscribe_in(
            &note_editor,
            window,
            |this, editor, event: &InputEvent, window, cx| {
                match event {
                    InputEvent::Change => {
                        if let Some(ai) = &mut this.ai {
                            if let Some(state) = &mut ai.editor {
                                let text = editor.read(cx).value().to_string();
                                let result = if state.replacing()
                                    || this.simple_transition.is_some()
                                    || this.closing.is_some()
                                    || this.closed
                                {
                                    Err("Waiting for note recovery before leaving")
                                } else {
                                    state.edit(text, Instant::now())
                                };
                                if let Err(error) = result {
                                    this.message = error.into();
                                    let retained = state.text.clone();
                                    editor.update(cx, |editor, cx| {
                                        editor.set_value(retained, window, cx)
                                    });
                                }
                            }
                            cx.notify();
                            return;
                        }
                    }
                    InputEvent::Focus => {
                        if let Some(ai) = &mut this.ai {
                            if let Some(command) = ai.refresh_editor() {
                                this.simple_send(command, cx);
                            }
                            return;
                        }
                    }
                    _ => {}
                }
                cx.notify();
            },
        );
        let query_subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(ai) = &mut this.ai {
                    ai.composer_changed();
                }
                cx.notify();
            }
        });
        let quit_subscription = cx.on_app_quit(|this, cx| {
            if let Some(ai) = &mut this.ai {
                ai.dismiss_login();
            }
            let app_worker = this.app_worker.take();
            let preferences = this.layout_task.take();
            cx.background_executor().spawn(simple::final_quit(
                move || {
                    if let Some(mut worker) = app_worker {
                        let _ = worker.shutdown();
                    }
                },
                async move {
                    if let Some(preferences) = preferences {
                        preferences.await;
                    }
                },
            ))
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
        let (layout, loaded) = preferences;
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
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.end_divider_drag(cx);
            }
        });
        let mut ai = crate::ai::AiState::default();
        let app_worker = match brn_workflow::app_worker::AppWorker::start(path.clone(), config) {
            Ok(worker) => Some(worker),
            Err(error) => {
                ai.startup_failed = true;
                ai.notice = error.message;
                None
            }
        };
        Self {
            app_worker,
            ai: Some(ai),
            simple_note_path: None,
            simple_transition: None,
            simple_editor_generation: None,
            evidence_editor_generation: None,
            login_dialog: None,
            closing: None,
            closed: false,
            close_failed: false,
            layout_task: None,
            query,
            note_editor,
            evidence_editor,
            provenance_open: false,
            provenance_snapshot: None,
            provenance_quotes: Vec::new(),
            provenance_scroll: ScrollHandle::new(),
            document_scroll: ScrollHandle::new(),
            saved_links: relationships::SavedLinksPane::new(window, cx),
            relationships: relationships::RelationshipsPane::new(window, cx),
            findings: findings::FindingsPane::new(window, cx),
            inbox: inbox::InboxPane::new(window, cx),
            dashboard: dashboard::DashboardPane::new(window, cx),
            action_editors: action_review::ActionEditors::default(),
            initial_action: action_draft::ActionInputs::default(),
            review_editor,
            review_title,
            review_predecessor,
            review_predecessor_proposal: None,
            review_comment,
            review_comment_draft: None,
            review_comment_pending: None,
            review_member: 0,
            review_scroll: ScrollHandle::new(),
            draft_title,
            draft_path,
            draft_editor,
            draft_link_target,
            draft_link_label,
            draft_link_proofs,
            draft_widget_id: None,
            draft_scroll: ScrollHandle::new(),
            note_path,
            note_scroll: ScrollHandle::new(),
            choosing_file: false,
            path,
            layout,
            system_dark,
            layout_note,
            resolved,
            open_doc: None,
            centre_tab: CentreTab::Chat,
            settings_requested: false,
            history_scroll: ScrollHandle::new(),
            vault_scroll: ScrollHandle::new(),
            chat_scroll: ScrollHandle::new(),
            dragging: None,
            divider_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            focus_composer: false,
            message: "Opening workspace…".into(),
            _subscriptions: vec![
                query_subscription,
                quit_subscription,
                note_subscription,
                review_subscription,
                review_title_subscription,
                draft_title_subscription,
                draft_path_subscription,
                draft_body_subscription,
                draft_link_target_subscription,
                draft_link_label_subscription,
                appearance_subscription,
                activation_subscription,
            ],
            _poll_task: poll_task,
        }
    }
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.poll_closing(window, cx) {
            self.poll_simple(window, cx);
        }
    }
    fn close_guard(&mut self, route: CloseRoute, cx: &mut Context<Self>) -> bool {
        if self.closed {
            return true;
        }
        if self.closing.is_none() && !self.close_failed {
            self.simple_leave(simple::EditorTransition::Close(route), cx);
        }
        false
    }
    fn query(&self, cx: &Context<Self>) -> String {
        self.query.read(cx).value().trim().to_string()
    }
    fn close_document(&mut self, cx: &mut Context<Self>) {
        self.simple_leave(simple::EditorTransition::Hide, cx);
    }
    fn cancel_running(&mut self, cx: &mut Context<Self>) {
        self.simple_stop(cx);
    }
    fn phase_status(&self) -> String {
        let ai = self.ai.as_ref().unwrap();
        if self.closing.is_some() {
            "Closing · waiting for local finalization".into()
        } else if let Some(active) = &ai.active {
            active.status_label().into()
        } else if let Some(active) = &ai.rewrite {
            if active.stopping {
                "Stopping Rewrite · finalization pending"
            } else {
                "Rewriting proposal · knowledge unchanged"
            }
            .into()
        } else if ai.startup_failed {
            "Workspace unavailable".into()
        } else if ai.ready {
            "Ready".into()
        } else {
            "Opening workspace…".into()
        }
    }
}
impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_shell(window, cx)
    }
}

fn desktop_root(desktop: Entity<Desktop>, window: &mut Window, cx: &mut Context<Root>) -> Root {
    let view = cx.new(|cx| DesktopWindow::new(desktop, cx));
    Root::new(view, window, cx)
}

/// Dialog builders inspect Desktop, so render them outside its mutable render
/// borrow. The toolkit Root owns modal state but clients render its dialog layer.
struct DesktopWindow {
    desktop: Entity<Desktop>,
    _desktop_updates: Subscription,
}

impl DesktopWindow {
    fn new(desktop: Entity<Desktop>, cx: &mut Context<Self>) -> Self {
        let updates = cx.observe(&desktop, |_, _, cx| cx.notify());
        Self {
            desktop,
            _desktop_updates: updates,
        }
    }
}

impl Render for DesktopWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child(self.desktop.clone())
            .children(Root::render_dialog_layer(window, cx))
    }
}
/// Routes an app-level action to the desktop entity, like the existing Quit handler.
fn route<A: gpui_kit::Action>(
    cx: &mut App,
    target: WeakEntity<Desktop>,
    apply: fn(&mut Desktop, &mut Context<Desktop>),
) {
    cx.on_action::<A>(move |_, cx| {
        let _ = target.update(cx, apply);
    });
}

pub fn run(
    path: PathBuf,
    config: brn_workflow::app::AppConfig,
    preferences: (LayoutState, Loaded),
) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-0", ToggleHistory, None),
                KeyBinding::new("alt-cmd-0", ToggleVault, None),
                KeyBinding::new("shift-cmd-enter", ToggleFocus, None),
                KeyBinding::new("cmd-l", FocusComposer, None),
                KeyBinding::new("cmd-n", NewChat, None),
                KeyBinding::new("cmd-,", OpenSettings, None),
                KeyBinding::new("cmd-.", CancelRunning, None),
                KeyBinding::new("cmd-.", CancelRunning, Some("Input")),
                KeyBinding::new("cmd-s", SaveNote, Some("MarkdownNote")),
            ]);
            let mut app_menu = vec![
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::action("Save to Markdown", SaveNote),
            ];
            app_menu.extend([MenuItem::separator(), MenuItem::action("Quit BRN", Quit)]);
            cx.set_menus([
                Menu::new("BRN").items(app_menu),
                Menu::new("View").items(vec![
                    MenuItem::action("Toggle History", ToggleHistory),
                    MenuItem::action("Toggle Vault", ToggleVault),
                    MenuItem::action("Toggle Focus", ToggleFocus),
                ]),
                Menu::new("Navigate").items(vec![
                    MenuItem::action("New Chat", NewChat),
                    MenuItem::action("Focus Composer", FocusComposer),
                    MenuItem::action("Cancel Running Action", CancelRunning),
                ]),
            ]);
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
                        let desktop =
                            cx.new(|cx| Desktop::new(path, config, preferences, window, cx));
                        let quit_target = desktop.downgrade();
                        cx.on_action::<Quit>(move |_, cx| {
                            let allow = quit_target
                                .update(cx, |this, cx| this.close_guard(CloseRoute::Quit, cx))
                                .unwrap_or(true);
                            if allow {
                                cx.quit();
                            }
                        });
                        route::<ToggleHistory>(cx, desktop.downgrade(), |this, cx| {
                            this.toggle_rail(Rail::History, cx)
                        });
                        route::<ToggleVault>(cx, desktop.downgrade(), |this, cx| {
                            this.toggle_rail(Rail::Vault, cx)
                        });
                        route::<ToggleFocus>(cx, desktop.downgrade(), |this, cx| {
                            this.toggle_focus(cx)
                        });
                        route::<FocusComposer>(cx, desktop.downgrade(), |this, cx| {
                            this.centre_tab = CentreTab::Chat;
                            this.focus_composer = true;
                            cx.notify();
                        });
                        route::<NewChat>(cx, desktop.downgrade(), |this, cx| {
                            this.simple_history(None, cx)
                        });
                        route::<OpenSettings>(cx, desktop.downgrade(), |this, cx| {
                            this.settings_requested = true;
                            cx.notify();
                        });
                        route::<CancelRunning>(cx, desktop.downgrade(), |this, cx| {
                            this.cancel_running(cx)
                        });
                        route::<SaveNote>(cx, desktop.downgrade(), |this, cx| {
                            if this.ai.is_some() && this.open_doc == Some(DocRef::SavedNote) {
                                this.simple_save(None, cx);
                            }
                        });
                        let weak = desktop.downgrade();
                        window.on_window_should_close(cx, move |_, cx| {
                            weak.update(cx, |this, cx| this.close_guard(CloseRoute::Window, cx))
                                .unwrap_or(true)
                        });
                        cx.new(|cx| desktop_root(desktop, window, cx))
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
    fn compact_title_keeps_long_multiline_turns_on_one_short_button_line() {
        let label = compact_title(
            "  A very long question about the northern map\nand every route to the cobalt lantern",
        );
        assert!(!label.contains('\n'));
        assert!(label.chars().count() <= 37);
        assert!(label.ends_with('…'));
    }
}
