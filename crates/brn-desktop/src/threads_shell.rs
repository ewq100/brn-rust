//! Application shell (spec §4, §8): the quiet sidebar, the workspace header
//! with the object's title and local actions, one primary view with an
//! optional supporting pane, the centred search launcher, and focus.
use super::components::AccessibleDropdownMenu;
use super::*;
use components::tokens;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{
    Action, DragMoveEvent, Focusable, Keystroke, MouseButton, MouseMoveEvent, SharedString,
    WeakEntity,
};

/// Below this window height the chat queue collapses to one row.
const SHORT_HEIGHT: f32 = 700.;
/// Room the macOS traffic lights take at the left of the sidebar top.
const TRAFFIC_LIGHTS: f32 = 80.;
/// Recent projects listed under Projects in the sidebar.
const SIDEBAR_PROJECTS: usize = 5;

/// Divider drag payload: which pane edge is being resized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PaneDrag {
    Sidebar,
    Support,
}

pub(super) struct DragGhost(SharedString);
impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let p = pal();
        if self.0.is_empty() {
            return div();
        }
        div()
            .px_2()
            .py_1()
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .bg(rgb(p.alt))
            .border_1()
            .border_color(rgb(p.accent))
            .text_color(rgb(p.ink))
            .child(self.0.clone())
    }
}

/// A menu row that calls the view directly; an action only supplies the
/// displayed shortcut.
pub(super) fn item(
    this: &WeakEntity<ThreadsUi>,
    label: impl Into<SharedString>,
    action: Option<Box<dyn Action>>,
    disabled: bool,
    checked: bool,
    f: impl Fn(&mut ThreadsUi, &mut Window, &mut Context<ThreadsUi>) + 'static,
) -> PopupMenuItem {
    let this = this.clone();
    let mut item = PopupMenuItem::new(label)
        .disabled(disabled)
        .checked(checked)
        .on_click(move |_, window, cx| {
            let _ = this.update(cx, |view, cx| f(view, window, cx));
        });
    if let Some(action) = action {
        item = item.action(action);
    }
    item
}

pub(super) use brn_writing_ui::native::pill;

/// Collapses whitespace so a title renders on one line; the element
/// truncates it with an ellipsis instead of cutting the text.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The kind icon shown in sidebar rows and launcher rows.
pub(super) fn kind_icon(kind: TileKind) -> IconName {
    match kind {
        TileKind::Threads | TileKind::AllThreads => IconName::MessagesSquare,
        TileKind::Chat => IconName::MessageSquare,
        TileKind::Note => IconName::FileText,
        TileKind::Notes => IconName::BookOpen,
        TileKind::Linked => IconName::Link,
        TileKind::Overview => IconName::House,
        TileKind::Actions => IconName::ListTodo,
        TileKind::History => IconName::RotateCcwClock,
        TileKind::Settings => IconName::Settings,
        TileKind::Compare => IconName::Columns2,
        TileKind::Connections => IconName::Network,
        TileKind::Projects => IconName::Folder,
    }
}

/// Launcher rows show what a command does with an icon, not a prefix.
fn command_icon(command: lists::Command) -> IconName {
    match command {
        lists::Command::NewThread | lists::Command::NewNote => IconName::Plus,
        lists::Command::Rewrite => IconName::Replace,
        lists::Command::Save => IconName::Check,
        lists::Command::Theme(_) => IconName::Palette,
        lists::Command::OpenTile(kind) => kind_icon(kind),
        lists::Command::Keys => IconName::Keyboard,
    }
}

pub(super) const KEYS: [(&str, &str); 18] = [
    ("Search", "⌘K · Search in the sidebar"),
    ("New chat", "⌘T or ⌘0 · New chat in the sidebar"),
    ("New note", "⌘N · + in the Library header"),
    ("Send / new line", "↵ or ⌘↵ / ⇧↵ · send button"),
    ("Stop", "⌘. · Stop in the header"),
    ("Focus composer", "⌘L"),
    ("Comment on selection", "⌘/ · selection strip · right-click"),
    ("Rewrite with AI", "⌘⇧R · writing bar · ⋯ menu"),
    ("Save note", "⌘S · Save in the document header"),
    ("Move focus", "⌘[ ⌘] · sidebar, workspace and assistant"),
    ("Sidebar", "⌘\\ · collapse to the rail or expand"),
    (
        "Assistant",
        "Assistant in the document header · drag its edge or choose a width in its menu",
    ),
    (
        "Settings",
        "⌘, · workspace menu at the bottom of the sidebar",
    ),
    ("Recent changes", "workspace menu · ⌘K"),
    ("Theme", "Settings → General · ⌘K"),
    (
        "Menus",
        "Tab to ⋯ · ↵ opens · ↑↓ ↵ · Esc · right-click a chat",
    ),
    (
        "Escape",
        "Closes the top sheet, menu, dialog or Settings · never stops work",
    ),
    (
        "Dialogs",
        "↵ submits a one-field dialog · ⌘↵ submits a form · Space presses the focused button · Esc cancels",
    ),
];

impl ThreadsUi {
    pub(super) fn show_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Information: ↵ and Esc both close it, Kit's default.
        self.open_dialog(window, cx, |dialog, window, _| {
            let p = pal();
            dialog
                .title("Keyboard shortcuts")
                .on_ok(|_, _, _| true)
                .child(
                    components::dialog_body("keys-help-body", window)
                        .child(
                            div()
                                .id("keys-help")
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(KEYS.iter().enumerate().map(|(index, (what, how))| {
                                    div()
                                        .id(format!("keys-row-{index}"))
                                        .flex()
                                        .gap_3()
                                        .aria_label(format!("{what}: {how}"))
                                        .child(div().w(px(170.)).flex_shrink_0().child(*what))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .font_family(UI_FONT)
                                                .text_size(SMALL_TEXT)
                                                .text_color(rgb(p.muted))
                                                .child(*how),
                                        )
                                        .test_support()
                                }))
                                .test_support(),
                        )
                        .test_support(),
                )
        });
    }
    /// The header's visible title (the object, in sentence case) and its
    /// announced name, which keeps the kind: ("Pilot proposal", "Note: Pilot
    /// proposal"). During review the Note keeps the note's title.
    pub(super) fn tile_title(&self, kind: TileKind) -> (String, String) {
        let named = |title: String| (title.clone(), format!("{}: {title}", kind.label()));
        match kind {
            // Tasks is the Inbox's second local view.
            TileKind::Actions => (
                TileKind::Overview.label().into(),
                format!("{}: {}", TileKind::Overview.label(), kind.label()),
            ),
            TileKind::Note => match (self.writing.review.as_ref(), self.writing.note.as_ref()) {
                (_, Some(open)) => named(one_line(&title(open.record()))),
                (Some(review), None) => named(format!(
                    "Review · {}",
                    one_line(&review.candidate.request.reason)
                )),
                _ => (kind.label().into(), kind.label().into()),
            },
            TileKind::Chat => named(match &self.chat {
                ChatTarget::Thread(id) => self
                    .thread_rows
                    .iter()
                    .find(|row| &row.id == id)
                    .map_or_else(|| id.clone(), |row| one_line(&row.title)),
                ChatTarget::NewForNote(note) => format!(
                    "New chat about {}",
                    self.note_rows
                        .iter()
                        .find(|row| &row.id == note)
                        .map_or_else(|| note.clone(), |row| one_line(&row.title))
                ),
                ChatTarget::New => "New chat".into(),
            }),
            kind => (kind.label().into(), kind.label().into()),
        }
    }
    /// A `narrow` header leaves Maximize to the ⋯ menu's Panel section, a
    /// double-click and ⌘⇧↵, so the title keeps room to read.
    fn tile_body(
        &mut self,
        kind: TileKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        match kind {
            TileKind::Threads => self.threads_tile(cx),
            TileKind::AllThreads => self.all_threads_tile(cx),
            TileKind::Notes => self.notes_tile(cx),
            TileKind::Overview => self.overview_tile(cx),
            TileKind::Actions => self.actions_tile(cx),
            TileKind::History => self.history_tile(cx),
            TileKind::Settings => self.settings_tile(cx),
            TileKind::Chat => self.chat_tile(cx),
            TileKind::Note => self.note_tile(window, cx),
            TileKind::Linked => self.linked_tile(cx),
            TileKind::Compare => self.compare_tile(cx),
            TileKind::Connections => self.connections_tile(cx),
            TileKind::Projects => self.projects_tile(cx),
        }
    }
}

/// Below this Chat panel width the model control also names the effort.
const CHAT_NARROW: f32 = 420.;
/// Why the model and effort menus' rows are disabled while work runs.
const BUSY_MENU_REASON: &str = "Available when the current work finishes";

pub(super) fn provider_key(provider: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "codex" | "chatgpt" => "chatgpt",
        "copilot" => "copilot",
        _ => "",
    }
}
pub(super) const PROVIDERS: [(&str, &str); 2] =
    [("chatgpt", "ChatGPT"), ("copilot", "GitHub Copilot")];
/// "ChatGPT · gpt-6.1-sol", or the model alone when no provider is chosen.
pub(super) fn model_display(provider: &str, model: &str) -> String {
    let key = provider_key(provider);
    match PROVIDERS.iter().find(|(provider, _)| *provider == key) {
        Some((_, name)) => format!("{name} · {model}"),
        None => model.to_owned(),
    }
}

impl ThreadsUi {
    /// Explicit models already configured or used by earlier runs; no fallback.
    pub(super) fn model_choices(&self) -> Vec<String> {
        let mut models = vec![self.settings.model.clone()];
        for record in self.records.iter().rev() {
            if let RecordData::Run(run) = &record.data
                && !run.model.trim().is_empty()
                && !models.contains(&run.model)
                && models.len() < 6
            {
                models.push(run.model.clone());
            }
        }
        models
    }
    pub(super) fn model_menu(&self, menu: PopupMenu, this: &WeakEntity<Self>) -> PopupMenu {
        let busy = self.worker.is_some();
        let current = provider_key(&self.settings.provider);
        // Disabled rows say why once, above them.
        let mut menu = if busy {
            menu.label(BUSY_MENU_REASON)
        } else {
            menu
        };
        for (provider, label) in PROVIDERS {
            menu = menu.label(label);
            let models = if provider == "copilot" && !self.copilot_models.is_empty() {
                self.copilot_models.clone()
            } else {
                self.model_choices()
            };
            for model in models {
                let selected = provider == current && model == self.settings.model;
                let chosen = model.clone();
                menu = menu.item(item(
                    this,
                    model,
                    None,
                    busy,
                    selected,
                    move |view, window, cx| view.set_model(provider, &chosen, window, cx),
                ));
            }
        }
        menu.separator()
            .item(item(
                this,
                "Load GitHub Copilot models",
                None,
                busy,
                false,
                |view, _, cx| view.load_copilot_models(cx),
            ))
            .item(item(
                this,
                "Provider and model settings…",
                None,
                false,
                false,
                |view, window, cx| {
                    view.settings_section = SettingsSection::Models;
                    view.show_tile(TileKind::Settings, window, cx);
                },
            ))
    }
    pub(super) fn effort_menu(&self, menu: PopupMenu, this: &WeakEntity<Self>) -> PopupMenu {
        let menu = if self.worker.is_some() {
            menu.label(BUSY_MENU_REASON)
        } else {
            menu
        };
        self.effort_section(menu, this)
    }
    /// The effort choices; the model menu adds them below 420 pt after its
    /// own reason line.
    pub(super) fn effort_section(&self, menu: PopupMenu, this: &WeakEntity<Self>) -> PopupMenu {
        let busy = self.worker.is_some();
        let mut menu = menu.label("Reasoning effort");
        for effort in ["low", "medium", "high"] {
            menu = menu.item(item(
                this,
                effort,
                None,
                busy,
                self.settings.effort == effort,
                move |view, window, cx| view.set_effort(effort, window, cx),
            ));
        }
        menu
    }
    /// The destination the sidebar marks as current. A chat is marked by its
    /// own row, Recent changes and Settings by nothing.
    pub(super) fn navigation_focus(&self) -> Option<TileKind> {
        match self.layout.primary() {
            TileKind::Notes | TileKind::Note | TileKind::Connections | TileKind::Compare => {
                Some(TileKind::Notes)
            }
            TileKind::Overview | TileKind::Actions => Some(TileKind::Overview),
            TileKind::Projects => Some(TileKind::Projects),
            TileKind::AllThreads => Some(TileKind::AllThreads),
            TileKind::Chat | TileKind::Linked if self.chat_thread().is_none() => {
                Some(TileKind::Chat)
            }
            _ => None,
        }
    }
    /// Needs-you chats: the Inbox's actionable attention count.
    pub(super) fn attention_count(&self) -> usize {
        self.thread_rows
            .iter()
            .filter(|row| row.needs_you())
            .count()
    }
    /// A press here may start moving the window (native titlebar
    /// behaviour); a double click zooms like the macOS titlebar. Controls
    /// sit outside these regions, so pressing one never moves the window.
    fn window_drag(&self, region: gpui_kit::Div, cx: &mut Context<Self>) -> gpui_kit::Div {
        region
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui_kit::MouseDownEvent, window, _| {
                    if event.click_count >= 2 {
                        this.titlebar_drag = false;
                        if cfg!(target_os = "macos") {
                            window.titlebar_double_click();
                        } else {
                            window.zoom_window();
                        }
                    } else {
                        this.titlebar_drag = true;
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, _| {
                if this.titlebar_drag {
                    this.titlebar_drag = false;
                    if event.pressed_button == Some(MouseButton::Left) {
                        window.start_window_move();
                    }
                }
            }))
    }
    /// A 36 pt sidebar row: line icon, 13 pt name truncated with an
    /// ellipsis, the full name as accessible name and tooltip, and a quiet
    /// selected state. A Kit button, so Tab, ↵ and Space reach it.
    fn nav_row(
        id: impl Into<gpui_kit::ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
        selected: bool,
    ) -> gpui_kit::component::button::Button {
        let p = pal();
        let label: SharedString = label.into();
        button(id)
            .ghost()
            .h(px(theme::metrics::SIDEBAR_ROW))
            .px_2()
            .rounded(px(theme::metrics::RADIUS_CONTROL))
            .accessibility_label(label.clone())
            .tooltip(label.clone())
            .selected(selected)
            .when(selected, |row| row.bg(rgb(p.active)))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .text_size(px(theme::metrics::TEXT_SIDEBAR))
                    .text_color(rgb(p.ink))
                    .child(
                        Icon::new(icon)
                            .with_size(Size::Small)
                            .flex_shrink_0()
                            .text_color(rgb(if selected { p.ink } else { p.muted })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .when(selected, |label| {
                                label.font_weight(gpui_kit::FontWeight::MEDIUM)
                            })
                            .child(label),
                    ),
            )
    }
    /// A quiet section label inside the sidebar.
    fn nav_label(id: &'static str, label: &'static str) -> gpui_kit::AnyElement {
        div()
            .id(id)
            .flex_shrink_0()
            .px_2()
            .pt(tokens::SPACE_4)
            .pb(tokens::SPACE_1)
            .text_size(px(theme::metrics::TEXT_LABEL))
            .text_color(rgb(pal().muted))
            .aria_label(label)
            .child(label)
            .test_support()
            .into_any_element()
    }
    /// The workspace area at the bottom of the sidebar: Settings and the
    /// workspace-level commands.
    fn workspace_menu(&self, rail: bool, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let can_restore = self.notice.can_restore(self.worker.is_some());
        let restored = self.notice.is_work_status();
        let trigger = if rail {
            components::icon_button("workspace-menu", IconName::Settings, "Workspace menu")
                .tooltip("Settings, keyboard shortcuts, recent changes and quit")
        } else {
            button("workspace-menu")
                .ghost()
                .w_full()
                .h(px(44.))
                .px_2()
                .rounded(px(theme::metrics::RADIUS_CONTROL))
                .accessibility_label("Workspace menu")
                .tooltip("Settings, keyboard shortcuts, recent changes and quit")
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_shrink_0()
                                .items_center()
                                .justify_center()
                                .size(px(28.))
                                .rounded_full()
                                .bg(rgb(p.surface_raised))
                                .border_1()
                                .border_color(rgb(p.hairline))
                                .text_size(px(theme::metrics::TEXT_LABEL))
                                .text_color(rgb(p.ink))
                                .child("B"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(theme::metrics::TEXT_SIDEBAR))
                                        .text_color(rgb(p.ink))
                                        .child("Workspace"),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(theme::metrics::TEXT_METADATA))
                                        .text_color(rgb(p.muted))
                                        .child("Settings and shortcuts"),
                                ),
                        ),
                )
        };
        trigger.accessible_dropdown_menu(move |menu, _, _| {
            menu.item(item(
                &this,
                "Settings",
                Some(Box::new(OpenSettings)),
                false,
                false,
                |view, window, cx| view.show_tile(TileKind::Settings, window, cx),
            ))
            .item(item(
                &this,
                "Keyboard shortcuts",
                Some(Box::new(ShowKeys)),
                false,
                false,
                |view, window, cx| view.show_keys(window, cx),
            ))
            .item(item(
                &this,
                "Recent changes",
                None,
                false,
                false,
                |view, window, cx| view.show_tile(TileKind::History, window, cx),
            ))
            .item(item(
                &this,
                "Last work status",
                None,
                !can_restore,
                restored,
                |view, _, cx| view.show_last_work_notice(cx),
            ))
            .separator()
            .item(item(
                &this,
                "Quit BRN",
                Some(Box::new(Quit)),
                false,
                false,
                |_, window, cx| window.dispatch_action(Box::new(Quit), cx),
            ))
        })
    }
    /// One chat row with its ⋯ (the same rename-free lifecycle menu as its
    /// right-click menu: Open, Archive, Open note, Move to project).
    fn chat_row(
        &self,
        row: &ThreadRow,
        needs: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let id = row.id.clone();
        let row_id = if needs {
            format!("attention-{id}")
        } else {
            format!("thread-{id}")
        };
        let selected =
            self.chat_thread() == Some(id.as_str()) && self.layout.contains(TileKind::Chat);
        let title = one_line(&row.title);
        let (resolved, note) = (row.resolved, row.note.clone());
        let (menu_this, menu_id, menu_note) = (this.clone(), id.clone(), note.clone());
        let open = id.clone();
        // One line: the title, then a quiet attention mark whose announced
        // text (the reason, and Protected) joins the accessible name.
        let mut entry = components::ListRow::new(row_id.clone(), title.clone()).selected(selected);
        if needs {
            let announced = match self.needs_reason(&id) {
                Some((reason, protected)) => {
                    format!("{reason}{}", if protected { " · Protected" } else { "" })
                }
                None => "Needs you".to_owned(),
            };
            entry = entry.chip(
                announced,
                div()
                    .flex_shrink_0()
                    .size(px(6.))
                    .rounded_full()
                    .bg(rgb(p.accent)),
            );
        }
        let entry = entry
            .actionable()
            .build()
            .flex_1()
            .h(px(theme::metrics::SIDEBAR_ROW))
            .text_size(px(theme::metrics::TEXT_SIDEBAR))
            // The quiet selected row, as for the destinations above.
            .when(selected, |row| row.bg(rgb(p.active)))
            .on_click(cx.listener(move |view, _, window, cx| view.open_thread(&open, window, cx)));
        div()
            .id(SharedString::from(format!("{row_id}-row")))
            .flex()
            .flex_shrink_0()
            .items_center()
            .w_full()
            .min_w_0()
            .child(entry)
            .child(
                components::overflow_menu(
                    format!("{row_id}-menu"),
                    format!("Chat actions: {title}"),
                )
                .accessible_dropdown_menu(move |menu, _, _| {
                    tiles::thread_menu(menu, &this, id.clone(), resolved, note.clone())
                }),
            )
            .context_menu(move |menu, _, _| {
                tiles::thread_menu(
                    menu,
                    &menu_this,
                    menu_id.clone(),
                    resolved,
                    menu_note.clone(),
                )
            })
            .into_any_element()
    }
    /// Active projects, most recently changed first.
    fn recent_projects(&self) -> Vec<(String, String)> {
        let mut projects: Vec<&Record> = self
            .records
            .iter()
            .filter(|record| !record.archived && matches!(record.data, RecordData::Project(_)))
            .collect();
        projects.sort_by_key(|record| std::cmp::Reverse(self.activity.order(&record.id)));
        projects
            .into_iter()
            .take(SIDEBAR_PROJECTS)
            .map(|record| (record.id.clone(), one_line(&title(record))))
            .collect()
    }
    fn open_project(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.projects_trash = false;
        self.selected_project = Some(id.to_owned());
        self.show_tile(TileKind::Projects, window, cx);
    }
    /// The expanded sidebar (§4.1): New chat, Search, Library, Inbox,
    /// Projects with recent projects, Recent chats with a route to all and
    /// archived chats, and the workspace area.
    fn sidebar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let current = self.navigation_focus();
        let attention = self.attention_count();
        let inbox_name = match attention {
            0 => "Inbox".to_owned(),
            n => format!("Inbox, {n} need{} you", if n == 1 { "s" } else { "" }),
        };
        let top = self
            .window_drag(
                div()
                    .flex()
                    .flex_1()
                    .h_full()
                    .items_center()
                    .pl(px(TRAFFIC_LIGHTS))
                    .child(
                        div()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_size(px(theme::metrics::TEXT_SECTION))
                            .text_color(rgb(p.ink))
                            .child("brn"),
                    ),
                cx,
            )
            .into_any_element();
        let header = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(TOP_BAR_HEIGHT)
            .pr_2()
            .child(top)
            .child(
                components::icon_button_with_action(
                    "toggle-sidebar",
                    IconName::PanelLeftClose,
                    "Collapse sidebar",
                    &ToggleSidebar,
                )
                .on_click(cx.listener(|this, _, window, cx| this.toggle_sidebar(window, cx))),
            );
        let destinations = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .px_2()
            .gap(px(2.))
            .child(
                Self::nav_row(
                    "nav-new-chat",
                    IconName::SquarePen,
                    "New chat",
                    current == Some(TileKind::Chat),
                )
                .tooltip_with_action("New chat", &NewThread, None)
                .on_click(cx.listener(|this, _, window, cx| this.new_thread(window, cx))),
            )
            .child(
                Self::nav_row("launcher", IconName::Search, "Search", false)
                    .accessibility_label("Search notes, chats and commands")
                    .tooltip_with_action("Search notes, chats and commands", &OpenLauncher, None)
                    .on_click(cx.listener(|this, _, window, cx| this.open_launcher(window, cx))),
            )
            .child(
                Self::nav_row(
                    "nav-library",
                    IconName::BookOpen,
                    "Library",
                    current == Some(TileKind::Notes),
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.show_tile(TileKind::Notes, window, cx)),
                ),
            )
            .child(
                Self::nav_row(
                    "nav-inbox",
                    IconName::Inbox,
                    "Inbox",
                    current == Some(TileKind::Overview),
                )
                .accessibility_label(inbox_name.clone())
                .tooltip(inbox_name)
                .when(attention > 0, |row| {
                    row.child(
                        div()
                            .id("nav-inbox-count")
                            .flex_shrink_0()
                            .px_1()
                            .text_size(px(theme::metrics::TEXT_LABEL))
                            .text_color(rgb(p.accent))
                            .child(attention.to_string())
                            .test_support(),
                    )
                })
                .on_click(
                    cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::Overview, window, cx)
                    }),
                ),
            );
        let mut list = scroll("sidebar-list").gap(px(2.)).px_2().pt_0().pb_2();
        list = list.child(
            Self::nav_row(
                "nav-projects",
                IconName::Folder,
                "Projects",
                current == Some(TileKind::Projects) && self.selected_project.is_none(),
            )
            .w_full()
            .mt(tokens::SPACE_3)
            .on_click(cx.listener(|this, _, window, cx| {
                this.selected_project = None;
                this.show_tile(TileKind::Projects, window, cx)
            })),
        );
        for (id, name) in self.recent_projects() {
            let selected = current == Some(TileKind::Projects)
                && self.selected_project.as_deref() == Some(id.as_str());
            let open = id.clone();
            list =
                list.child(
                    Self::nav_row(format!("project-{id}"), IconName::Folder, name, selected)
                        .w_full()
                        .pl(px(28.))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_project(&open, window, cx)
                        })),
                );
        }
        list = list.child(Self::nav_label("recent-chats-label", "Recent chats"));
        let bar = lists::sidebar(&self.thread_rows, self.chat_thread());
        for row in &bar.needs {
            list = list.child(self.chat_row(row, true, cx));
        }
        for row in &bar.open {
            list = list.child(self.chat_row(row, false, cx));
        }
        if let Some(empty) = bar.recent_empty() {
            let text = match empty {
                lists::RecentEmpty::NoChats => "No chats yet · Start one with ⌘T.",
                lists::RecentEmpty::OthersNeedYou => "No other open chats.",
                lists::RecentEmpty::AllArchived => {
                    "No open chats · Archived chats are in All chats."
                }
            };
            list = list.child(
                metadata(text)
                    .id("threads-empty")
                    .aria_label(text)
                    .px_2()
                    .py_1()
                    .test_support(),
            );
        }
        let all = format!("All chats · {}", bar.total);
        list = list.child(
            Self::nav_row(
                "nav-all-chats",
                IconName::MessagesSquare,
                all,
                current == Some(TileKind::AllThreads),
            )
            .w_full()
            .on_click(cx.listener(|this, _, window, cx| {
                this.thread_filter = ThreadFilter::All;
                this.show_tile(TileKind::AllThreads, window, cx)
            })),
        );
        div()
            .id("sidebar")
            .key_context("BrnSidebar")
            .track_focus(self.panel_body(TileKind::Threads))
            .relative()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(self.arrangement.sidebar))
            .h_full()
            .bg(rgb(p.sidebar))
            .border_r_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.layout.focus_pane(layout::Pane::Sidebar)),
            )
            .child(header)
            .child(destinations)
            .child(div().flex_1().min_h_0().child(list.test_support()))
            .child(
                div()
                    .flex_shrink_0()
                    .p_2()
                    .border_t_1()
                    .border_color(rgb(p.hairline))
                    .child(self.workspace_menu(false, cx)),
            )
            .child(Self::resize_handle(
                "sidebar-resize",
                PaneDrag::Sidebar,
                true,
            ))
            .test_support()
            .into_any_element()
    }
    /// The 56 pt rail: the same destinations as labelled icon controls.
    fn rail(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let current = self.navigation_focus();
        let attention = self.attention_count();
        let inbox_name = match attention {
            0 => "Inbox".to_owned(),
            n => format!("Inbox, {n} need{} you", if n == 1 { "s" } else { "" }),
        };
        let icon = |id: &'static str, icon: IconName, label: SharedString, selected: bool| {
            components::icon_button(id, icon, label.clone())
                .tooltip(label)
                .selected(selected)
                .when(selected, |button| button.bg(rgb(pal().active)))
        };
        let inbox =
            div()
                .relative()
                .child(
                    icon(
                        "nav-inbox",
                        IconName::Inbox,
                        inbox_name.into(),
                        current == Some(TileKind::Overview),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::Overview, window, cx)
                    })),
                )
                .when(attention > 0, |inbox| {
                    inbox.child(
                        div()
                            .id("nav-inbox-count")
                            .absolute()
                            .top(px(2.))
                            .right(px(2.))
                            .size(px(7.))
                            .rounded_full()
                            .bg(rgb(p.accent))
                            .test_support(),
                    )
                });
        div()
            .id("sidebar")
            .key_context("BrnSidebar")
            .track_focus(self.panel_body(TileKind::Threads))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .w(px(theme::metrics::SIDEBAR_RAIL))
            .h_full()
            .pb_2()
            .bg(rgb(p.sidebar))
            .border_r_1()
            .border_color(rgb(p.hairline))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.layout.focus_pane(layout::Pane::Sidebar)),
            )
            // The traffic lights sit in this strip.
            .child(self.window_drag(div().w_full().h(px(40.)).flex_shrink_0(), cx))
            .child(
                components::icon_button_with_action(
                    "toggle-sidebar",
                    IconName::PanelLeftOpen,
                    "Expand sidebar",
                    &ToggleSidebar,
                )
                .on_click(cx.listener(|this, _, window, cx| this.toggle_sidebar(window, cx))),
            )
            .child(div().h(tokens::SPACE_2))
            .child(
                icon(
                    "nav-new-chat",
                    IconName::SquarePen,
                    "New chat".into(),
                    current == Some(TileKind::Chat),
                )
                .on_click(cx.listener(|this, _, window, cx| this.new_thread(window, cx))),
            )
            .child(
                icon(
                    "launcher",
                    IconName::Search,
                    "Search notes, chats and commands".into(),
                    false,
                )
                .on_click(cx.listener(|this, _, window, cx| this.open_launcher(window, cx))),
            )
            .child(
                icon(
                    "nav-library",
                    IconName::BookOpen,
                    "Library".into(),
                    current == Some(TileKind::Notes),
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.show_tile(TileKind::Notes, window, cx)),
                ),
            )
            .child(inbox)
            .child(
                icon(
                    "nav-projects",
                    IconName::Folder,
                    "Projects".into(),
                    current == Some(TileKind::Projects),
                )
                .on_click(
                    cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::Projects, window, cx)
                    }),
                ),
            )
            .child(
                icon(
                    "nav-all-chats",
                    IconName::MessagesSquare,
                    "All chats".into(),
                    current == Some(TileKind::AllThreads),
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.thread_filter = ThreadFilter::All;
                    this.show_tile(TileKind::AllThreads, window, cx)
                })),
            )
            .child(div().flex_1())
            .child(self.workspace_menu(true, cx))
            .test_support()
            .into_any_element()
    }
    /// A 7 pt grab strip on a pane edge; the 1 pt divider stays visible.
    fn resize_handle(id: &'static str, drag: PaneDrag, right_edge: bool) -> gpui_kit::AnyElement {
        div()
            .id(id)
            .absolute()
            .top_0()
            .h_full()
            .w(px(7.))
            .when(right_edge, |handle| handle.right(px(-4.)))
            .when(!right_edge, |handle| handle.left(px(-4.)))
            .cursor_col_resize()
            .on_drag(drag, |_, _, _, cx| {
                cx.new(|_| DragGhost(SharedString::default()))
            })
            .test_support()
            .into_any_element()
    }
    fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let width = Self::usable_width(window);
        self.relayout(window, cx, |layout| layout.toggle_sidebar(width));
    }
    /// The document header's Assistant control: show the assistant (or
    /// close whatever supporting view is showing). Compact windows only
    /// toggle the temporary sheet.
    fn toggle_assistant(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let width = Self::usable_width(window);
        if self.arrangement.support_visible() {
            self.relayout(window, cx, |layout| layout.toggle_support(width));
        } else {
            self.relayout(window, cx, |layout| {
                layout.open(TileKind::Chat);
                if !layout.arrange(width).support_visible() {
                    layout.toggle_support(width);
                }
            });
        }
    }
    /// Running work, or saved queue work waiting, stays visible in every
    /// view's header: the outcome the status bar carried.
    fn work_status(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let mut out = vec![];
        if let Some(worker) = &self.worker {
            let status = self.worker_status(worker);
            let thread = worker.run_identity().map(|run| run.thread.clone());
            let progress = thread.as_ref().map_or_else(
                || status.clone(),
                |thread| {
                    let name = self
                        .records
                        .iter()
                        .find(|record| &record.id == thread)
                        .map(title)
                        .unwrap_or_else(|| thread.clone());
                    format!("Work in {} · {}", short(&name), status)
                },
            );
            out.push(
                div()
                    .id("worker-status")
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_shrink(1.)
                    .min_w_0()
                    .max_w(px(280.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_2()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.ink))
                    .cursor_pointer()
                    .aria_label(progress.clone())
                    .tooltip({
                        let progress = SharedString::from(progress.clone());
                        move |window, cx| Tooltip::new(progress.clone()).build(window, cx)
                    })
                    .child(
                        Icon::new(IconName::LoaderCircle)
                            .with_size(Size::XSmall)
                            .flex_shrink_0()
                            .text_color(rgb(p.info)),
                    )
                    .child(div().min_w_0().truncate().child(progress))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(thread) = &thread {
                            this.open_thread(thread, window, cx);
                        }
                    }))
                    .test_support()
                    .into_any_element(),
            );
            if worker.kind().can_cancel() {
                out.push(
                    pill("stop-work", "Stop")
                        .icon(IconName::Square)
                        .tooltip("Stop the current work and pause the queue · ⌘.")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
                        .into_any_element(),
                );
            }
        } else {
            let queue = self.queue_state();
            let waiting = queue.queue.len() + usize::from(queue.pending.is_some());
            if waiting > 0 {
                let status = if queue.paused {
                    format!("Queue paused · {waiting} waiting")
                } else {
                    format!("Queue · {waiting} waiting")
                };
                out.push(
                    div()
                        .id("idle-status")
                        .px_2()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(if queue.paused { p.warning } else { p.muted }))
                        .aria_label(status.clone())
                        .child(status)
                        .test_support()
                        .into_any_element(),
                );
            }
        }
        out
    }
    /// The open note's save state: unsaved saves on click, saved opens
    /// Recent changes.
    fn save_status(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let p = pal();
        let open = self.writing.note.as_ref()?;
        let name = one_line(&title(open.record()));
        Some(
            if open.session().is_some() {
                pill("status-save", "● Unsaved · ⌘S")
                    .text_color(rgb(p.warning))
                    .tooltip(format!("Save {name} (⌘S)"))
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
            } else {
                pill("status-save", "Saved")
                    .text_color(rgb(p.muted))
                    .tooltip(format!(
                        "{name} v{} saved · Open Recent changes",
                        open.record().version
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::History, window, cx)
                    }))
            }
            .into_any_element(),
        )
    }
    /// The local actions the current view supplies to the header slot.
    fn header_actions(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mut out = self.work_status(cx);
        let primary = self.layout.primary();
        // Unsaved document work stays visible (and savable) from any view.
        if primary != TileKind::Note
            && self
                .writing
                .note
                .as_ref()
                .is_some_and(|open| open.session().is_some())
        {
            out.extend(self.save_status(cx));
        }
        match primary {
            TileKind::Note => {
                out.extend(self.save_status(cx));
                if let Some(section) = self.note_menu_section(cx) {
                    out.push(
                        components::overflow_menu("note-menu", "Note actions")
                            .accessible_dropdown_menu(move |menu, _, _| section(menu)),
                    );
                }
                let shown = self.arrangement.support_visible();
                let label = match self.layout.support() {
                    Some(kind) if shown && kind != TileKind::Chat => {
                        format!("Hide {}", kind.label().to_lowercase())
                    }
                    _ if shown => "Hide assistant".to_owned(),
                    _ => "Assistant".to_owned(),
                };
                out.push(
                    pill("toggle-assistant", label)
                        .icon(IconName::PanelRightOpen)
                        .selected(shown)
                        .tooltip("Ask about this document beside it")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_assistant(window, cx)),
                        )
                        .into_any_element(),
                );
            }
            TileKind::Chat => out.extend(self.chat_controls(cx)),
            TileKind::Notes => out.extend(self.tile_controls(TileKind::Notes, cx)),
            TileKind::Projects => {
                if let Some(section) = self.projects_menu_section(cx) {
                    out.push(
                        components::overflow_menu("projects-menu", "Project actions")
                            .accessible_dropdown_menu(move |menu, _, _| section(menu)),
                    );
                }
            }
            TileKind::Overview | TileKind::Actions => {
                let views = [
                    ("inbox-for-you", "For you", TileKind::Overview),
                    ("inbox-tasks", "Tasks", TileKind::Actions),
                ]
                .into_iter()
                .map(|(id, label, kind)| {
                    button(id).label(label).selected(primary == kind).on_click(
                        cx.listener(move |this, _, window, cx| this.show_tile(kind, window, cx)),
                    )
                })
                .collect();
                out.push(components::segmented("inbox-views", views).into_any_element());
            }
            TileKind::Settings => out.push(
                pill("close-settings", "Done · Esc")
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.relayout(window, cx, |layout| layout.close(TileKind::Settings));
                    }))
                    .into_any_element(),
            ),
            _ => {}
        }
        out
    }
    /// The 56 pt workspace header: the current object's title and the
    /// view's local actions. The empty parts move the window.
    fn workspace_header(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let primary = self.layout.primary();
        let (label, announced) = self.tile_title(primary);
        let full = SharedString::from(label.clone());
        // Beside the rail the traffic lights reach into the header.
        let inset = if self.arrangement.sidebar_expanded {
            tokens::SPACE_6
        } else {
            px(TRAFFIC_LIGHTS - theme::metrics::SIDEBAR_RAIL) + tokens::SPACE_4
        };
        let title = self.window_drag(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .pl(inset)
                .child(
                    div()
                        .id(format!("tile-title-{}", primary.key()))
                        .min_w_0()
                        .truncate()
                        .text_size(px(theme::metrics::TEXT_DOCUMENT))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(rgb(p.ink))
                        .aria_label(announced)
                        .tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx))
                        .child(label)
                        .test_support(),
                ),
            cx,
        );
        div()
            .id("workspace-header")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(TOP_BAR_HEIGHT)
            .pr_4()
            .bg(rgb(p.paper))
            .border_b_1()
            .border_color(rgb(p.hairline))
            .child(title)
            .child(
                div()
                    .id("header-actions")
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_1()
                    .children(self.header_actions(cx))
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }
    /// One view's frame: `tile-{kind}` around `tile-body-{kind}`, which
    /// tracks the view's focus. The Note keeps its toolbar row above.
    fn pane(
        &mut self,
        kind: TileKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let toolbar = if kind == TileKind::Note {
            let controls = self.tile_controls(kind, cx);
            (!controls.is_empty()).then(|| {
                components::toolbar("note-toolbar")
                    .overflow_hidden()
                    .children(controls)
                    .test_support()
            })
        } else {
            None
        };
        let body = self.tile_body(kind, window, cx);
        div()
            .id(format!("tile-{}", kind.key()))
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.layout.focus() != Some(kind) {
                        this.layout.set_focus(kind);
                        cx.notify();
                    }
                }),
            )
            .children(toolbar)
            .child(
                div()
                    .id(format!("tile-body-{}", kind.key()))
                    .track_focus(self.panel_body(kind))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(body)
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }
    /// The supporting pane: a small header (name, width choices, close)
    /// over the view. As a compact sheet its close returns to the document.
    fn support_pane(
        &mut self,
        kind: TileKind,
        sheet: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let (object, announced) = self.tile_title(kind);
        let name = if kind == TileKind::Chat {
            "Assistant"
        } else {
            kind.label()
        };
        let current = self.layout.support_width();
        let close_label = if sheet {
            "Back to document".to_owned()
        } else {
            format!("Close {}", name.to_lowercase())
        };
        let header = div()
            .id("assistant-header")
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(px(44.))
            .pl_3()
            .pr_2()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .child(
                div()
                    .id(format!("tile-title-{}", kind.key()))
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(UI_TEXT)
                    .text_color(rgb(p.ink))
                    .aria_label(announced.clone())
                    .tooltip({
                        let announced = SharedString::from(announced);
                        move |window, cx| Tooltip::new(announced.clone()).build(window, cx)
                    })
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child(name),
                            )
                            .when(object != name, |line| {
                                line.child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_color(rgb(p.muted))
                                        .child(format!("· {object}")),
                                )
                            }),
                    )
                    .test_support(),
            )
            .when(!sheet, |header| {
                header.child(
                    components::icon_button(
                        "assistant-width",
                        IconName::ChevronsLeftRight,
                        format!("{name} width"),
                    )
                    .tooltip("Width · or drag the divider")
                    .accessible_dropdown_menu(move |mut menu, _, _| {
                        for (label, width) in [
                            ("Narrow", theme::metrics::ASSISTANT_MIN),
                            ("Standard", theme::metrics::ASSISTANT),
                            ("Wide", theme::metrics::ASSISTANT_MAX),
                        ] {
                            menu = menu.item(item(
                                &this,
                                label,
                                None,
                                false,
                                (current - width).abs() < 0.5,
                                move |view, _, cx| {
                                    view.layout.set_support_width(width);
                                    cx.notify();
                                },
                            ));
                        }
                        menu
                    }),
                )
            })
            .child(
                components::icon_button("close-assistant", IconName::Close, close_label.clone())
                    .tooltip(close_label)
                    .on_click(cx.listener(|this, _, window, cx| {
                        let width = Self::usable_width(window);
                        this.relayout(window, cx, |layout| layout.toggle_support(width));
                    })),
            );
        let controls = (kind == TileKind::Chat).then(|| {
            components::toolbar("assistant-toolbar")
                .flex_wrap()
                .h_auto()
                .children(self.chat_controls(cx))
                .test_support()
        });
        div()
            .id("assistant-pane")
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(header)
            .children(controls)
            .child(div().flex_1().min_h_0().child(self.pane(kind, window, cx)))
            .test_support()
            .into_any_element()
    }
    /// The workspace: header, the one inline notice, then the primary view
    /// with its docked supporting pane or compact sheet.
    fn workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let header = self.workspace_header(cx);
        let notice = self.notice_line(cx);
        let primary = self.layout.primary();
        let main = self.pane(primary, window, cx);
        let mut body = div()
            .id("workspace-body")
            .relative()
            .flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(main),
            );
        let support = self.layout.support();
        match (self.arrangement.support, support) {
            (layout::Support::Docked(width), Some(kind)) => {
                let pane = self.support_pane(kind, false, window, cx);
                body = body.child(
                    div()
                        .id(format!("tile-support-{}", kind.key()))
                        .relative()
                        .flex_shrink_0()
                        .w(px(width))
                        .h_full()
                        .bg(rgb(p.sidebar))
                        .border_l_1()
                        .border_color(rgb(p.hairline))
                        .child(pane)
                        .child(Self::resize_handle(
                            "assistant-resize",
                            PaneDrag::Support,
                            false,
                        ))
                        .test_support(),
                );
            }
            (layout::Support::Sheet { open: true, width }, Some(kind)) => {
                let pane = self.support_pane(kind, true, window, cx);
                body = body.child(
                    div()
                        .id(format!("tile-support-{}", kind.key()))
                        .absolute()
                        .top_0()
                        .right_0()
                        .w(px(width))
                        .h_full()
                        .bg(rgb(p.surface_raised))
                        .border_l_1()
                        .border_color(rgb(p.border))
                        .shadow_lg()
                        .occlude()
                        .child(pane)
                        .test_support(),
                );
            }
            _ => {}
        }
        div()
            .id("workspace")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(rgb(p.paper))
            .child(header)
            .children(notice)
            .child(body)
            .test_support()
            .into_any_element()
    }
    /// Fold the document's toolbar, review bar and comments margin, and the
    /// chat's model control, by the widths the pane policy resolved.
    fn fit_panes(&mut self, window: &Window) {
        let note = self.panel_width(TileKind::Note);
        self.writing.margin_mode = note.map_or(layout::MarginMode::Stacked, layout::margin_mode);
        self.writing.note_fit = note.map_or(layout::NoteFit::Full, |width| {
            layout::NoteFit::for_width(width, self.note_toolbar_needs(window))
        });
        self.writing.review_fit =
            note.map_or(layout::ReviewFit::Full, layout::ReviewFit::for_width);
        self.chat_narrow = self
            .panel_width(TileKind::Chat)
            .is_some_and(|width| width < CHAT_NARROW);
    }
    /// A drawn view's resolved width, from the last arrangement.
    pub(super) fn panel_width(&self, kind: TileKind) -> Option<f32> {
        if self.layout.primary() == kind {
            Some(self.arrangement.primary)
        } else if self.layout.support() == Some(kind) {
            self.arrangement.support_width()
        } else {
            None
        }
    }
    /// The Note's width at the window's current usable width.
    #[cfg(all(test, feature = "native-test-support"))]
    pub(super) fn note_panel_width(&self, window: &Window) -> Option<f32> {
        (self.layout.primary() == TileKind::Note)
            .then(|| self.layout.arrange(Self::usable_width(window)).primary)
    }
}

impl ThreadsUi {
    fn notice_line(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        if self.notice.text().is_empty() {
            return None;
        }
        let restorable = self.notice.is_work_status();
        let dismiss = components::icon_button("dismiss-notice", IconName::Close, "Dismiss")
            .tooltip(if restorable {
                "Hide status · restore from the workspace menu → Last work status"
            } else {
                "Dismiss"
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.notice.dismiss();
                cx.notify();
            }));
        // One quiet inline notice at the top of the workspace, same precedence.
        Some(
            components::banner(
                "workspace-notice",
                self.notice_tone(),
                self.notice.text().to_owned(),
                vec![],
                Some(dismiss),
            )
            .border_b_1()
            .border_color(rgb(pal().hairline))
            .into_any_element(),
        )
    }
    fn launcher_overlay(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let items = self.launcher_items(cx);
        let viewport = window.viewport_size();
        // A centred search surface: at most 640 wide with 32 pt either side;
        // the results fit their content up to 60 % of the window.
        let width = px(640.).min(viewport.width - px(64.));
        let results = (viewport.height * 0.6 - px(56.)).max(px(120.));
        let mut list = div()
            .id("launcher-results")
            .test_support()
            .flex()
            .flex_col()
            .max_h(results)
            .overflow_y_scroll()
            .track_scroll(&self.launcher_scroll);
        if items.is_empty() && self.launcher_search.pending {
            // Saved chat Messages are still being searched.
            list = list.child(div().px_2().py_1().child(components::loading(
                "launcher-searching",
                "Searching chats…",
            )));
        } else if items.is_empty() {
            list = list.child(components::empty_state(
                "launcher-empty",
                IconName::Search,
                "No matches",
                "No note, chat or command matches. Try fewer words.",
                None,
            ));
        }
        let mut group = "";
        for (index, entry) in items.iter().enumerate() {
            if entry.group() != group {
                group = entry.group();
                list = list.child(
                    components::section_header(group, None, None)
                        .id(format!(
                            "launcher-group-{}",
                            group.to_lowercase().replace(' ', "-")
                        ))
                        .px_2()
                        .pt(if index == 0 {
                            tokens::SPACE_1
                        } else {
                            tokens::SPACE_2
                        })
                        .aria_label(group)
                        .test_support(),
                );
            }
            let selected = index == self.launcher_index;
            let (icon, keystroke, archived) = match entry {
                lists::LaunchItem::Thread { resolved, .. } => {
                    (IconName::MessageSquare, None, *resolved)
                }
                lists::LaunchItem::Note { .. } => (IconName::FileText, None, false),
                lists::LaunchItem::Message { archived, .. } => {
                    (IconName::MessageSquare, None, *archived)
                }
                lists::LaunchItem::Command(command) => {
                    (command_icon(*command), command.keystroke(), false)
                }
            };
            let label = match entry {
                lists::LaunchItem::Message {
                    snippet,
                    highlights,
                    ..
                } => search_view::highlighted(snippet, highlights).into_any_element(),
                _ => entry.label().into_any_element(),
            };
            let meta = entry.meta();
            list = list.child(
                div()
                    .id(format!("launcher-item-{index}"))
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .h(px(theme::metrics::SIDEBAR_ROW))
                    .px_2()
                    .rounded(px(theme::metrics::RADIUS_CONTROL))
                    .cursor_pointer()
                    .text_color(rgb(p.ink))
                    .when(selected, |row| row.bg(rgb(p.selection)))
                    .when(!selected, |row| {
                        row.hover(|style| style.bg(rgb(pal().hover)))
                    })
                    .aria_label(entry.accessible_label())
                    .aria_selected(selected)
                    .child(
                        Icon::new(icon)
                            .with_size(Size::Small)
                            .flex_shrink_0()
                            .text_color(rgb(if selected { p.ink } else { p.muted })),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(label))
                    .children(meta.map(|meta| {
                        div()
                            .flex_shrink_0()
                            .max_w(gpui_kit::relative(0.4))
                            .min_w_0()
                            .truncate()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(components::row_meta_color(selected)))
                            .child(meta)
                    }))
                    .when(archived, |row| {
                        row.child(components::status_chip(
                            format!("launcher-item-{index}-archived"),
                            None,
                            None,
                            "Archived",
                        ))
                    })
                    .children(
                        keystroke
                            .and_then(|key| Keystroke::parse(key).ok())
                            .map(Kbd::new),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_launcher(Some(index), window, cx)
                    }))
                    .test_support(),
            );
        }
        if !items.is_empty() && self.launcher_search.pending {
            // Saved chat Messages join below the label matches when the
            // search answers; the rows above never move.
            list = list.child(div().px_2().py_1().child(components::loading(
                "launcher-searching",
                "Searching chats…",
            )));
        }
        div()
            .id("launcher-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            .items_start()
            .pt(px(96.).min(viewport.height * 0.12))
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_launcher(window, cx)),
            )
            .child(
                div()
                    .id("launcher")
                    .key_context("BrnLauncher")
                    .w(width)
                    .h_auto()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(rgb(p.surface_raised))
                    .border_1()
                    .border_color(rgb(p.border))
                    .rounded(px(theme::metrics::RADIUS_CARD))
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    // The text input owns escape/up/down; the launcher claims them first.
                    .capture_key_down(cx.listener(
                        |this, event: &gpui_kit::KeyDownEvent, window, cx| {
                            match event.keystroke.key.as_str() {
                                "escape" => this.close_launcher(window, cx),
                                "down" => this.move_launcher(1, cx),
                                "up" => this.move_launcher(-1, cx),
                                _ => return,
                            }
                            cx.stop_propagation();
                        },
                    ))
                    .child(Input::new(&self.launcher_query).w_full())
                    .child(list)
                    .test_support(),
            )
            .into_any_element()
    }
    /// Focus lands in the focused region's visible content: the Note's
    /// editor, the Chat's composer, the Library search and the All chats
    /// filter. The sidebar and other views focus their body, then (after
    /// the frame that draws it) its first control; with none, focus stays on
    /// the body.
    pub(super) fn focus_tile_contents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_first = self.focus_landing(window, cx);
        cx.notify();
    }
    /// Focus the focused region's landing from the focus table. Returns the
    /// container that took focus, for the move to its first control.
    fn focus_landing(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::FocusHandle> {
        match self.layout.focus() {
            Some(TileKind::Note) => self.focus_visible_editor(window, cx),
            // An archived chat's composer can't send: its one next step is
            // the archived banner's Unarchive, so focus moves there (U11).
            Some(TileKind::Chat)
                if self
                    .chat_thread()
                    .is_some_and(|thread| self.thread_resolved(thread)) =>
            {
                self.archived_banner.focus(window, cx);
                return Some(self.archived_banner.clone());
            }
            Some(TileKind::Chat) => self
                .composer
                .update(cx, |editor, cx| editor.focus(window, cx)),
            Some(TileKind::Notes) => self.search.update(cx, |input, cx| input.focus(window, cx)),
            Some(TileKind::AllThreads) => self
                .threads_query
                .update(cx, |input, cx| input.focus(window, cx)),
            Some(kind) => {
                self.panel_focus[&kind].focus(window, cx);
                return Some(self.panel_focus[&kind].clone());
            }
            None => self.focus_root(window, cx),
        }
        None
    }
    /// Open a dialog that hands focus back to something on screen when it
    /// closes. Kit returns focus to whatever held it at open. A menu item's
    /// handler runs while its menu holds focus, and the menu closes with
    /// the click, so Escape would leave the keyboard nowhere. In that case
    /// focus first moves to the focused region's landing, which stays
    /// rendered under the dialog; a button or field keeps its own focus.
    pub(super) fn open_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        build: impl Fn(
            gpui_kit::component::dialog::Dialog,
            &mut Window,
            &mut App,
        ) -> gpui_kit::component::dialog::Dialog
        + 'static,
    ) {
        let in_menu = window
            .context_stack()
            .iter()
            .any(|context| context.contains("PopupMenu"));
        if in_menu || window.focused(cx).is_none() {
            self.focus_first = None;
            self.focus_landing(window, cx);
        }
        window.open_dialog(cx, build);
    }
    /// A view's (or the sidebar's) focus handle, tracked by the element
    /// around it.
    pub(super) fn panel_body(&self, kind: TileKind) -> &gpui_kit::FocusHandle {
        &self.panel_focus[&kind]
    }
    /// After the frame that drew the view, move from its body (or the
    /// archived chat's banner) to its first control, unless focus moved
    /// elsewhere meanwhile.
    fn schedule_focus_first(&mut self, window: &mut Window) {
        let Some(body) = self.focus_first.take() else {
            return;
        };
        window.on_next_frame(move |window, cx| {
            if body.is_focused(window) {
                window.focus_next(cx);
                if !body.contains_focused(window, cx) {
                    body.focus(window, cx);
                }
            }
        });
    }
}

impl Render for ThreadsUi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_writing_review_cache();
        let _retained = (&self.subscriptions, &self.poll);
        let p = pal();
        self.schedule_focus_first(window);
        // Short windows collapse the chat queue to one row.
        self.short_window = window.viewport_size().height < px(SHORT_HEIGHT);
        // The pane policy runs on every frame from the usable width; it
        // reads the stored preferences and never writes them back.
        self.arrangement = self.layout.arrange(Self::usable_width(window));
        self.layout.settle(&self.arrangement);
        // A supporting pane that is not drawn cannot hold the keyboard region.
        // When the window narrows under it, focus returns to the document.
        if self.layout.pane() == layout::Pane::Assistant && !self.arrangement.support_visible() {
            self.layout.focus_pane(layout::Pane::Primary);
            self.focus_first = self.focus_landing(window, cx);
        }
        self.fit_panes(window);
        let sidebar = if self.arrangement.sidebar_expanded {
            self.sidebar(cx)
        } else {
            self.rail(cx)
        };
        let workspace = self.workspace(window, cx);
        let mut root = div()
            .id("workspace-root")
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .track_focus(&self.root_focus)
            .key_context("BrnWorkspace")
            // Escape closes the compact assistant sheet, then Settings:
            // bubbling, it reaches the root only after the launcher, an open
            // menu or popover, a dialog and the stacked comments section had
            // their turn. It never stops work or discards a draft.
            .on_key_down(
                cx.listener(|view, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key != "escape" || view.launcher_open {
                        return;
                    }
                    let width = Self::usable_width(window);
                    if view.layout.close_sheet(width) {
                        view.focus_tile_contents(window, cx);
                        cx.stop_propagation();
                    } else if view.layout.settings_open() {
                        view.relayout(window, cx, |layout| layout.close(TileKind::Settings));
                        cx.stop_propagation();
                    }
                }),
            )
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<PaneDrag>, window, cx| {
                    let x = f32::from(event.event.position.x);
                    match event.drag(cx) {
                        PaneDrag::Sidebar => this.layout.set_sidebar_width(x),
                        PaneDrag::Support => this
                            .layout
                            .set_support_width(Self::usable_width(window) - x),
                    }
                    cx.notify();
                }),
            )
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(p.paper))
            .text_color(rgb(p.ink))
            .on_action(cx.listener(|this, _: &Save, window, cx| {
                if this.writing.review.is_edited() {
                    this.prepare_review_edit(window, cx);
                } else if this.writing.note.is_some() {
                    this.save(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Ask, window, cx| {
                if this
                    .writing
                    .comment_input
                    .focus_handle(cx)
                    .is_focused(window)
                {
                    this.post_comment(window, cx);
                } else {
                    this.ask(window, cx);
                }
            }))
            // ⌘0 opens New chat (Home is retired as a destination).
            .on_action(cx.listener(|this, _: &Home, window, cx| this.new_thread(window, cx)))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &NewThread, window, cx| this.new_thread(window, cx)))
            .on_action(
                cx.listener(|this, _: &FocusAsk, window, cx| this.focus_composer(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                if this.launcher_open {
                    this.close_launcher(window, cx);
                } else if this.layout.settings_open() {
                    this.relayout(window, cx, |layout| layout.close(TileKind::Settings));
                } else {
                    this.cancel(cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &OpenLauncher, window, cx| this.open_launcher(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &CloseLauncher, window, cx| this.close_launcher(window, cx)),
            )
            .on_action(cx.listener(|this, _: &LauncherNext, _, cx| this.move_launcher(1, cx)))
            .on_action(cx.listener(|this, _: &LauncherPrevious, _, cx| this.move_launcher(-1, cx)))
            .on_action(cx.listener(|this, _: &CommentSelection, window, cx| {
                this.track_selection(cx);
                this.comment_on_selection(window, cx)
            }))
            .on_action(cx.listener(|this, _: &AskAboutSelection, window, cx| {
                this.track_selection(cx);
                this.ask_about_selection(window, cx)
            }))
            .on_action(cx.listener(|this, _: &Rewrite, window, cx| this.rewrite_note(window, cx)))
            .on_action(
                cx.listener(|this, _: &ToggleSidebar, window, cx| this.toggle_sidebar(window, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusPreviousPane, window, cx| {
                let width = Self::usable_width(window);
                this.layout.cycle_focus(-1, width);
                this.focus_tile_contents(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusNextPane, window, cx| {
                let width = Self::usable_width(window);
                this.layout.cycle_focus(1, width);
                this.focus_tile_contents(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.show_tile(TileKind::Settings, window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowKeys, window, cx| this.show_keys(window, cx)))
            .child(sidebar)
            .child(workspace);
        if self.launcher_open {
            root = root.child(self.launcher_overlay(window, cx));
        }
        root
    }
}
