//! Workspace shell: top bar, tile row, status bar, launcher and menus.
use super::*;
use components::tokens;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::scroll::{Scrollbar, ScrollbarMode};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{
    Action, ClickEvent, DragMoveEvent, Focusable, Keystroke, MouseButton, SharedString, WeakEntity,
};

/// Below this window width the title and navigation bars use icon controls.
const COMPACT_WIDTH: f32 = 1000.;
/// Below this window height the chat queue collapses to one row.
const SHORT_HEIGHT: f32 = 700.;
/// Below this panel width the header leaves Maximize to its ⋯ menu: a
/// 170 pt list then has room for its title beside ＋, ⋯ and Close.
const NARROW_HEADER: f32 = 220.;
fn compact(window: &Window) -> bool {
    window.viewport_size().width < px(COMPACT_WIDTH)
}

/// Header drag payload for swapping tiles.
#[derive(Clone, Copy)]
pub(super) struct TileDrag(pub(super) TileKind);
/// Gap drag payload for resizing the tiles on either side.
#[derive(Clone, Copy)]
pub(super) struct SplitDrag(pub(super) usize);

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

pub(super) fn pill(id: impl Into<gpui_kit::ElementId>, label: impl Into<SharedString>) -> Button {
    button(id)
        .label(label)
        .compact()
        .ghost()
        .h(tokens::COMPACT_CONTROL)
}

/// Collapses whitespace so a title renders on one line; the element
/// truncates it with an ellipsis instead of cutting the text.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The kind icon shown in panel headers and launcher rows.
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
        lists::Command::Layout(_) => IconName::LayoutDashboard,
        lists::Command::Theme(_) => IconName::Palette,
        lists::Command::OpenTile(kind) => kind_icon(kind),
        lists::Command::Keys => IconName::Keyboard,
    }
}

// Panel items shared by the header ⋯ and the right-click menu, so both run
// the same layout handlers.
fn maximize_item(this: &WeakEntity<ThreadsUi>, kind: TileKind, layout: &Layout) -> PopupMenuItem {
    item(
        this,
        if layout.maximized() == Some(kind) {
            "Restore"
        } else {
            "Maximize"
        },
        Some(Box::new(MaximizeTile)),
        false,
        false,
        move |view, window, cx| view.relayout(window, cx, |layout| layout.toggle_maximize(kind)),
    )
}
fn move_item(
    this: &WeakEntity<ThreadsUi>,
    kind: TileKind,
    delta: isize,
    layout: &Layout,
) -> PopupMenuItem {
    item(
        this,
        if delta < 0 { "Move left" } else { "Move right" },
        None,
        !layout.can_move(kind, delta),
        false,
        move |view, _, cx| {
            view.layout.move_by(kind, delta);
            cx.notify();
        },
    )
}
fn close_item(this: &WeakEntity<ThreadsUi>, kind: TileKind) -> PopupMenuItem {
    item(
        this,
        "Close",
        None,
        false,
        false,
        move |view, window, cx| view.relayout(window, cx, |layout| layout.close(kind)),
    )
}
fn close_splits_item(
    this: &WeakEntity<ThreadsUi>,
    kind: TileKind,
    layout: &Layout,
) -> PopupMenuItem {
    item(
        this,
        "Close other splits",
        None,
        !layout.has_splits(),
        false,
        move |view, window, cx| view.relayout(window, cx, |layout| layout.close_other_splits(kind)),
    )
}
fn beside_items(
    mut menu: PopupMenu,
    this: &WeakEntity<ThreadsUi>,
    kind: TileKind,
    layout: &Layout,
) -> PopupMenu {
    for other in TileKind::ALL
        .into_iter()
        .filter(|other| *other != TileKind::Settings)
    {
        menu = menu.item(item(
            this,
            other.label(),
            None,
            false,
            layout.contains(other),
            move |view, window, cx| {
                view.layout.set_focus(kind);
                view.show_beside(other, window, cx);
            },
        ));
    }
    menu
}
/// The header ⋯'s last section: the right-click menu's panel actions, so
/// splitting is discoverable without right-clicking.
fn panel_section(
    menu: PopupMenu,
    this: &WeakEntity<ThreadsUi>,
    kind: TileKind,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let Some(view) = this.upgrade() else {
        return menu;
    };
    let layout = view.read(cx).layout.clone();
    let (beside_this, beside_layout) = (this.clone(), layout.clone());
    menu.separator()
        .label("Panel")
        .item(maximize_item(this, kind, &layout))
        .item(move_item(this, kind, -1, &layout))
        .item(move_item(this, kind, 1, &layout))
        .submenu("Open beside", window, cx, move |menu, _, _| {
            beside_items(menu, &beside_this, kind, &beside_layout)
        })
        .item(close_splits_item(this, kind, &layout))
        .item(close_item(this, kind))
}

pub(super) const KEYS: [(&str, &str); 20] = [
    ("Launcher", "⌘K · click the search field"),
    ("New chat / note", "⌘T / ⌘N · + in Chats and Notes"),
    ("Send / new line", "↵ or ⌘↵ / ⇧↵ · send button"),
    ("Stop", "⌘. · status bar · Stop button"),
    ("Comment on selection", "⌘/ · selection strip · right-click"),
    ("Rewrite with AI", "⌘⇧R · writing bar · ⋯ menu"),
    ("Save note", "⌘S · status bar · ⋯ menu"),
    ("Layouts", "⌥1 Write · ⌥2 Chat · ⌥3 Solo / restore"),
    ("Focus panel", "⌘[ ⌘] · click a panel"),
    (
        "Maximize",
        "⌘⇧↵ · double-click its header · Maximize button",
    ),
    ("Extra split", "Open beside · navigation bar or panel menu"),
    ("Resize panels", "drag the gap between panels"),
    ("Swap panels", "drag a panel header onto another panel"),
    ("Panel menu", "⋯ in the header · right-click the header"),
    ("Toggle chats", "⌘\\ · Layout menu"),
    ("Theme", "Settings → General · ⌘K"),
    ("Settings", "⌘, · navigation gear"),
    ("Home", "⌘0 · brn logo · launcher"),
    ("Menus", "Tab to ⋯ · ↵ opens · ↑↓ ↵ · Esc"),
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
            dialog.title("Keys and mouse").on_ok(|_, _, _| true).child(
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
    /// The panel's visible title (the object, in sentence case) and its
    /// announced name, which keeps the kind: ("Pilot proposal", "Note: Pilot
    /// proposal"). During review the Note keeps the note's title.
    pub(super) fn tile_title(&self, kind: TileKind) -> (String, String) {
        let named = |title: String| (title.clone(), format!("{}: {title}", kind.label()));
        match kind {
            // All chats is a view of the Chats panel (Recent / All / Archived).
            TileKind::AllThreads => (
                TileKind::Threads.label().into(),
                TileKind::Threads.label().into(),
            ),
            TileKind::Note => match (&self.review, &self.note) {
                (_, Some(open)) => named(one_line(&title(&open.record))),
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
    fn tile_header(
        &self,
        kind: TileKind,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let key = kind.key();
        let focused = self.layout.focus() == Some(kind);
        let maximized = self.layout.maximized() == Some(kind);
        let this = cx.entity().downgrade();
        let (label, announced) = self.tile_title(kind);
        let ghost = SharedString::from(label.clone());
        let full = SharedString::from(label.clone());
        let header = div()
            .id(format!("tile-header-{key}"))
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(tokens::PANEL_HEADER)
            .px_2()
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .bg(rgb(p.tile))
            // The focused panel: accent title plus a 2 px accent underline.
            .when(focused, |header| {
                header.border_b_2().border_color(rgb(p.accent))
            })
            .when(!focused, |header| {
                header.border_b_1().border_color(rgb(p.hairline))
            })
            .cursor_grab()
            .child(
                Icon::new(kind_icon(kind))
                    .with_size(Size::Small)
                    .flex_shrink_0()
                    .text_color(rgb(if focused { p.accent } else { p.muted })),
            )
            .child(
                div()
                    .id(format!("tile-title-{key}"))
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(rgb(if focused { p.accent } else { p.ink }))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .aria_label(announced)
                    .tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx))
                    .child(label)
                    .test_support(),
            )
            .when(
                self.layout.area(kind) == Some(layout::Area::Beside),
                |header| {
                    header.child(
                        components::status_chip(format!("tile-split-{key}"), None, None, "Split")
                            .tooltip(|window, cx| {
                                Tooltip::new(
                                    "Opened with Open beside. Navigation doesn't replace it.",
                                )
                                .build(window, cx)
                            }),
                    )
                },
            )
            .when(kind != TileKind::Note, |header| {
                header.children(self.tile_controls(kind, cx))
            });
        // Exactly one ⋯ per panel: the panel's own items, then a Panel section.
        let object = match kind {
            TileKind::Note => self.note_menu_section(cx),
            TileKind::Projects => self.projects_menu_section(cx),
            _ => None,
        };
        let menu_this = this.clone();
        let more = if kind == TileKind::Note {
            components::overflow_menu("note-menu", "Note actions")
        } else {
            components::overflow_menu(
                format!("tile-menu-{key}"),
                format!("{} panel actions", kind.label()),
            )
        }
        .dropdown_menu(move |menu, window, cx| {
            let menu = match &object {
                Some(section) => section(menu),
                None => menu,
            };
            panel_section(menu, &menu_this, kind, window, cx)
        });
        header
            .child(more)
            .when(!narrow || maximized, |header| {
                header.child(
                    components::icon_button(
                        format!("tile-max-{key}"),
                        if maximized {
                            IconName::Minimize
                        } else {
                            IconName::Maximize
                        },
                        // The announced name says which panel.
                        format!(
                            "{} {}",
                            if maximized { "Restore" } else { "Maximize" },
                            kind.label()
                        ),
                    )
                    .tooltip(if maximized {
                        "Restore (⌘⇧↵ · double-click header)"
                    } else {
                        "Maximize (⌘⇧↵ · double-click header)"
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.relayout(window, cx, |layout| layout.toggle_maximize(kind))
                    })),
                )
            })
            .child(
                components::icon_button(
                    format!("tile-close-{key}"),
                    IconName::Close,
                    format!("Close {}", kind.label()),
                )
                .tooltip("Close panel · your work stays saved or recoverable")
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.relayout(window, cx, |layout| layout.close(kind))
                })),
            )
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    this.relayout(window, cx, |layout| layout.toggle_maximize(kind));
                } else {
                    this.layout.set_focus(kind);
                    cx.notify();
                }
            }))
            .on_drag(TileDrag(kind), move |_, _, _, cx| {
                let ghost = ghost.clone();
                cx.new(|_| DragGhost(ghost))
            })
            .context_menu(move |menu, _, cx| {
                let Some(view) = this.upgrade() else {
                    return menu;
                };
                let layout = view.read(cx).layout.clone();
                let menu = menu
                    .item(maximize_item(&this, kind, &layout))
                    .item(move_item(&this, kind, -1, &layout))
                    .item(move_item(&this, kind, 1, &layout))
                    .item(close_item(&this, kind))
                    .item(close_splits_item(&this, kind, &layout))
                    .separator()
                    .label("Open beside");
                beside_items(menu, &this, kind, &layout)
            })
            .into_any_element()
    }
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
    fn tile_row(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let visible = self.layout.visible();
        let reveal = (
            visible.iter().map(|tile| tile.kind).collect::<Vec<_>>(),
            self.layout.focus(),
        );
        if self.tile_reveal.as_ref() != Some(&reveal) {
            if let Some(index) = visible
                .iter()
                .position(|tile| Some(tile.kind) == self.layout.focus())
            {
                // The row alternates panels and split handles.
                self.tile_scroll.scroll_to_item(index * 2);
            }
            self.tile_reveal = Some(reveal);
        }
        let mut row = div()
            .id("tile-row")
            .test_support()
            .flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .p(px(layout::PANEL_GAP))
            .overflow_x_scroll()
            .track_scroll(&self.tile_scroll)
            .bg(rgb(p.paper))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<SplitDrag>, _, cx| {
                    let boundary = event.drag(cx).0;
                    let x = event.event.position.x - event.bounds.left();
                    let fraction = x / event.bounds.size.width;
                    this.layout.resize(boundary, fraction);
                    cx.notify();
                }),
            );
        // Comments sit beside the text only when the panels' minimums leave
        // room for the margin; the Note's bars fold by its resolved width.
        self.margin_mode =
            layout::margin_mode(f32::from(window.viewport_size().width), &self.layout, false);
        let note_width = self.note_panel_width(window);
        // The toolbar folds by what this note's row needs, measured from
        // its labels, status chip and comment count.
        self.note_fit = note_width.map_or(layout::NoteFit::Full, |width| {
            layout::NoteFit::for_width(width, self.note_toolbar_needs(window))
        });
        self.review_fit = note_width.map_or(layout::ReviewFit::Full, layout::ReviewFit::for_width);
        self.chat_narrow = self
            .panel_width(TileKind::Chat, window)
            .is_some_and(|width| width < CHAT_NARROW);
        if visible.is_empty() {
            return row
                .items_center()
                .justify_center()
                .child(components::empty_state(
                    "tiles-empty",
                    IconName::LayoutDashboard,
                    "No panels open",
                    "Press ⌘K, or choose Write, Chat or Solo.",
                    None,
                ))
                .into_any_element();
        }
        for (index, tile) in visible.iter().enumerate() {
            if index > 0 {
                row = row.child(
                    div()
                        .id(format!("split-{}", index - 1))
                        .test_support()
                        .flex_shrink_0()
                        .w(px(layout::PANEL_GAP))
                        .h_full()
                        .cursor_col_resize()
                        .hover(|style| style.bg(rgb(p.border)))
                        .on_drag(SplitDrag(index - 1), |_, _, _, cx| {
                            cx.new(|_| DragGhost(SharedString::default()))
                        }),
                );
            }
            let kind = tile.kind;
            let mut frame = div()
                .id(format!("tile-{}", kind.key()))
                .test_support()
                .flex()
                .flex_col()
                .h_full()
                .min_h_0()
                .min_w(px(kind.min_width()
                    + if kind == TileKind::Note {
                        self.note_margin_width()
                    } else {
                        0.
                    }))
                .flex_basis(px(0.))
                .border_1()
                .border_color(rgb(p.hairline))
                .rounded(tokens::RADIUS_PANEL)
                .overflow_hidden()
                .bg(rgb(p.tile))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        if this.layout.focus() != Some(kind) {
                            this.layout.set_focus(kind);
                            cx.notify();
                        }
                    }),
                )
                .drag_over::<TileDrag>(move |style, _, _, _| style.border_color(rgb(pal().accent)))
                .on_drop(cx.listener(move |this, drag: &TileDrag, _, cx| {
                    this.layout.swap(drag.0, kind);
                    cx.notify();
                }));
            frame.style().flex_grow = Some(tile.weight);
            frame.style().flex_shrink = Some(1.);
            let narrow = self
                .panel_width(kind, window)
                .is_some_and(|width| width < NARROW_HEADER);
            let header = self.tile_header(kind, narrow, cx);
            // The Note's one toolbar row; it folds controls by `note_fit`
            // rather than wrapping.
            let controls = if kind == TileKind::Note {
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
            row = row.child(
                frame.child(header).children(controls).child(
                    div()
                        .id(format!("tile-body-{}", kind.key()))
                        .track_focus(self.panel_body(kind))
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .child(body)
                        .test_support(),
                ),
            );
        }
        // Kit draws the horizontal bar only while the panels overflow the row.
        div()
            .relative()
            .flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(row)
            .child(
                Scrollbar::horizontal(&self.tile_scroll)
                    .id("tile-row-scrollbar")
                    .mode(ScrollbarMode::Always),
            )
            .into_any_element()
    }
}

impl ThreadsUi {
    /// Width the comments margin adds to the Note's minimum: the mode's
    /// margin while it shows beside the text, nothing when stacked.
    fn note_margin_width(&self) -> f32 {
        if self.comments_beside() {
            self.margin_mode.width()
        } else {
            0.
        }
    }
    /// The Note panel's width as the tile row's flex layout resolves it, from
    /// the same weights, minimums and gaps the renderer uses.
    pub(super) fn note_panel_width(&self, window: &Window) -> Option<f32> {
        self.panel_width(TileKind::Note, window)
    }
    /// A visible panel's resolved width.
    pub(super) fn panel_width(&self, kind: TileKind, window: &Window) -> Option<f32> {
        let tiles: Vec<(TileKind, f32)> = self
            .layout
            .visible()
            .iter()
            .map(|tile| (tile.kind, tile.weight))
            .collect();
        let index = tiles.iter().position(|(tile, _)| *tile == kind)?;
        let available = f32::from(window.viewport_size().width);
        layout::panel_widths(available, &tiles, self.note_margin_width())
            .get(index)
            .copied()
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
    pub(super) fn show_beside(
        &mut self,
        kind: TileKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.relayout(window, cx, |layout| layout.open_beside(kind));
        self.refresh();
        if kind == TileKind::Connections {
            self.refresh_connections(cx);
        }
        cx.notify();
    }
    /// Navigation reflects the focused destination, not all panels left open.
    pub(super) fn navigation_focus(&self) -> Option<TileKind> {
        match self.layout.focus()? {
            TileKind::Threads | TileKind::AllThreads | TileKind::Chat | TileKind::Linked => {
                Some(TileKind::Threads)
            }
            TileKind::Notes | TileKind::Note | TileKind::Connections | TileKind::Compare => {
                Some(TileKind::Notes)
            }
            kind => Some(kind),
        }
    }
    fn navigation_bar(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let layout = self.layout.clone();
        let compact = compact(window);
        let mut bar = div()
            .id("workspace-navigation")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(tokens::NAV_BAR)
            .px_2()
            .overflow_x_scroll()
            .bg(rgb(p.alt))
            .border_b_1()
            .border_color(rgb(p.hairline));
        for (id, label, kind) in [
            ("browse-chats", "Chats", TileKind::Threads),
            ("open-notes", "Notes", TileKind::Notes),
            ("open-projects", "Projects", TileKind::Projects),
            ("actions", "Tasks", TileKind::Actions),
        ] {
            bar = bar.child(
                pill(id, label)
                    .selected(self.navigation_focus() == Some(kind))
                    .on_click(
                        cx.listener(move |view, _, window, cx| view.show_tile(kind, window, cx)),
                    ),
            );
        }
        bar.child(div().flex_1())
            .child(
                pill("history", "Changes")
                    .selected(self.navigation_focus() == Some(TileKind::History))
                    .tooltip("Workspace changes and Undo")
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.show_tile(TileKind::History, window, cx)
                    })),
            )
            .child(
                components::icon_button_with_action(
                    "settings",
                    IconName::Settings,
                    "Settings",
                    &OpenSettings,
                )
                .selected(self.layout.settings_open())
                .on_click(
                    cx.listener(|view, _, window, cx| {
                        view.show_tile(TileKind::Settings, window, cx)
                    }),
                ),
            )
            .child(
                if compact {
                    components::icon_button("open-beside", IconName::PanelRightOpen, "Open beside")
                } else {
                    pill("open-beside", "Open beside").icon(IconName::PanelRightOpen)
                }
                .tooltip("Add an extra split · normal navigation reuses your working areas")
                .dropdown_menu(move |mut menu, _, _| {
                    menu = menu.label("Open beside · existing panels are focused");
                    for kind in TileKind::ALL
                        .into_iter()
                        .filter(|kind| *kind != TileKind::Settings)
                    {
                        menu = menu.item(item(
                            &this,
                            kind.label(),
                            None,
                            false,
                            layout.contains(kind),
                            move |view, window, cx| view.show_beside(kind, window, cx),
                        ));
                    }
                    menu.separator().item(item(
                        &this,
                        "Close extra splits",
                        None,
                        !layout.has_splits(),
                        false,
                        |view, window, cx| {
                            view.relayout(window, cx, |layout| {
                                layout.close_other_splits(TileKind::Settings)
                            })
                        },
                    ))
                }),
            )
            .test_support()
            .into_any_element()
    }
    fn settings_surface(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        // Framed and headed like a panel: hairline outline, panel radius and
        // a 36 px header with the kind icon and a semibold title.
        div()
            .id("settings-surface")
            .flex()
            .flex_1()
            .min_h_0()
            .justify_center()
            .p(px(layout::PANEL_GAP))
            .bg(rgb(p.paper))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(920.))
                    .h_full()
                    .min_h_0()
                    .bg(rgb(p.tile))
                    .border_1()
                    .border_color(rgb(p.hairline))
                    .rounded(tokens::RADIUS_PANEL)
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .gap_1()
                            .h(tokens::PANEL_HEADER)
                            .px_2()
                            .font_family(UI_FONT)
                            .text_size(UI_TEXT)
                            .border_b_1()
                            .border_color(rgb(p.hairline))
                            .child(
                                Icon::new(kind_icon(TileKind::Settings))
                                    .with_size(Size::Small)
                                    .flex_shrink_0()
                                    .text_color(rgb(p.muted)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(rgb(p.ink))
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child("Settings"),
                            )
                            .child(pill("close-settings", "Done · Esc").on_click(cx.listener(
                                |view, _, window, cx| {
                                    view.relayout(window, cx, |layout| {
                                        layout.close(TileKind::Settings)
                                    });
                                },
                            ))),
                    )
                    .child(
                        div()
                            .id("tile-body-settings")
                            .track_focus(self.panel_body(TileKind::Settings))
                            .flex_1()
                            .min_h_0()
                            .child(self.settings_tile(cx))
                            .test_support(),
                    ),
            )
            .test_support()
            .into_any_element()
    }
    fn top_bar(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let compact = compact(window);
        let mut bar = div()
            .id("top-bar")
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .flex_1()
            .h_full()
            .pr_3()
            .child(
                pill("home", "brn")
                    .font_family(MONO)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .text_color(rgb(p.accent))
                    .accessibility_label("Home")
                    .tooltip("Home (⌘0)")
                    .selected(self.navigation_focus() == Some(TileKind::Overview))
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.show_tile(TileKind::Overview, window, cx)
                    })),
            )
            .child(
                if compact {
                    components::icon_button(
                        "launcher",
                        IconName::Search,
                        "Search notes, chats and commands",
                    )
                    .tooltip_with_action("Search", &OpenLauncher, None)
                } else {
                    button("launcher")
                        .label("Search…")
                        .child(div().flex_1())
                        .children(Keystroke::parse("cmd-k").ok().map(Kbd::new))
                        .accessibility_label("Search notes, chats and commands")
                        .w(px(220.))
                        .min_w(px(150.))
                        .flex_shrink(1.)
                        .tooltip("Search notes, chats and commands (⌘K)")
                }
                .on_click(cx.listener(|this, _, window, cx| this.open_launcher(window, cx))),
            );
        // Each item names the chat and why it needs you, on one line; the
        // full text is its accessible name and tooltip.
        let needs: Vec<(String, String)> = self
            .thread_rows
            .iter()
            .filter(|row| row.needs_you())
            .map(|row| {
                let text = match self.needs_reason(&row.id) {
                    Some((reason, protected)) => format!(
                        "{} · {reason}{}",
                        one_line(&row.title),
                        if protected { " · Protected" } else { "" }
                    ),
                    None => one_line(&row.title),
                };
                (row.id.clone(), text)
            })
            .collect();
        if !needs.is_empty() {
            let count = needs.len();
            let announced = format!("{count} need{} you", if count == 1 { "s" } else { "" });
            bar = bar.child(
                button("needs-badge")
                    .label(if compact {
                        format!("● {count}")
                    } else {
                        format!("● {announced}")
                    })
                    .accessibility_label(announced)
                    .compact()
                    .primary()
                    .dropdown_menu(move |mut menu, _, _| {
                        menu = menu.max_w(px(420.)).label("Needs you");
                        for (id, text) in &needs {
                            let (open, text) = (id.clone(), SharedString::from(text.clone()));
                            let this = this.clone();
                            let id = format!("needs-item-{id}");
                            menu = menu.item(
                                PopupMenuItem::element(move |_, _| {
                                    let tooltip = text.clone();
                                    // Kit's item row grows to its content, so
                                    // the line sets its own limit.
                                    div()
                                        .id(SharedString::from(id.clone()))
                                        .max_w(px(380.))
                                        .min_w_0()
                                        .truncate()
                                        .aria_label(text.clone())
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(tooltip.clone()).build(window, cx)
                                        })
                                        .child(text.clone())
                                        .test_support()
                                })
                                .on_click(move |_, window, cx| {
                                    let _ = this
                                        .update(cx, |view, cx| view.open_thread(&open, window, cx));
                                }),
                            );
                        }
                        menu
                    }),
            );
        }
        // Write, Chat and Solo are one mode switch; each segment keeps its ID,
        // handler and tooltip.
        let presets =
            Preset::ALL
                .into_iter()
                .zip(["⌥1", "⌥2", "⌥3"])
                .map(|(preset, shortcut)| {
                    button(format!("layout-{}", preset.key()))
                        .label(preset.label())
                        .selected(self.layout.preset() == Some(preset))
                        .tooltip(if preset == Preset::Solo {
                            "Focus current panel / restore arrangement (⌥3)".into()
                        } else {
                            format!("Reset to {} layout ({shortcut})", preset.label())
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.apply_layout(preset, window, cx)
                        }))
                })
                .collect();
        bar = bar
            .child(div().flex_1())
            .child(components::segmented("layout-presets", presets));
        gpui_kit::component::TitleBar::new()
            .h(TOP_BAR_HEIGHT)
            .bg(rgb(p.alt))
            .border_color(rgb(p.hairline))
            .child(bar)
            .into_any_element()
    }
    fn status_bar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let mut bar = div()
            .id("status-bar")
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(tokens::STATUS_BAR)
            .px_2()
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.hairline));
        if let Some(worker) = &self.worker {
            let status = self.worker_status(worker);
            let thread = worker.thread.clone();
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
            bar = bar.child(
                div()
                    .id("worker-status")
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_shrink(1.)
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_2()
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
                    .test_support(),
            );
            if worker.kind.can_cancel() {
                bar = bar.child(
                    pill("stop-work", "Stop")
                        .h(tokens::ICON_CONTROL)
                        .icon(IconName::Square)
                        .tooltip("Stop the current work and pause the queue · ⌘.")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                );
            }
        } else {
            // Nothing runs, but saved queue work is not idle: say what waits,
            // in the queue strip's words (R2).
            let queue = self.queue_state();
            let waiting = queue.queue.len() + usize::from(queue.pending.is_some());
            let status = match (waiting, queue.paused) {
                (0, _) => "Idle".to_owned(),
                (n, true) => format!("Queue paused · {n} waiting"),
                (n, false) => format!("Queue · {n} waiting"),
            };
            bar = bar.child(
                div()
                    .id("idle-status")
                    .px_2()
                    .text_color(rgb(if waiting > 0 && queue.paused {
                        p.warning
                    } else {
                        p.muted
                    }))
                    .aria_label(status.clone())
                    .child(status)
                    .test_support(),
            );
        }
        // The open note's save state stays visible even when its panel is
        // hidden: unsaved saves on click, saved opens Changes.
        if let Some(open) = &self.note {
            let name = one_line(&title(&open.record));
            bar = bar.child(if open.session.is_some() {
                pill("status-save", "● Unsaved · ⌘S")
                    .h(tokens::ICON_CONTROL)
                    .text_color(rgb(p.warning))
                    .tooltip(format!("Save {name} (⌘S)"))
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
            } else {
                pill("status-save", "Saved")
                    .h(tokens::ICON_CONTROL)
                    .text_color(rgb(p.muted))
                    .tooltip(format!(
                        "{name} v{} saved · Open Changes",
                        open.record.version
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::History, window, cx)
                    }))
            });
        }
        let layout_label = match self.layout.maximized() {
            Some(kind) => format!("Maximized · {}", kind.label()),
            None => "Layout".into(),
        };
        let layout_this = this.clone();
        let layout = self.layout.clone();
        let work_status_visible = self
            .last_work_notice
            .as_ref()
            .is_some_and(|(_, last)| last == &self.notice);
        let can_show_work_status =
            self.last_work_notice.is_some() && self.notice.is_empty() && self.worker.is_none();
        // Theme lives in Settings → General and ⌘K only (owner decision), so
        // the status bar and its Layout menu carry no theme control.
        bar.child(div().flex_1())
            .child(
                pill("status-layout", layout_label)
                    .h(tokens::ICON_CONTROL)
                    .icon(IconName::LayoutDashboard)
                    .tooltip("Layouts, panels and the last work status")
                    .dropdown_menu(move |mut menu, _, _| {
                        menu = menu.label("Layouts");
                        for (preset, action) in Preset::ALL.into_iter().zip([
                            Box::new(LayoutWrite) as Box<dyn Action>,
                            Box::new(LayoutChat),
                            Box::new(LayoutSolo),
                        ]) {
                            menu = menu.item(item(
                                &layout_this,
                                preset.label(),
                                Some(action),
                                false,
                                layout.preset() == Some(preset),
                                move |view, window, cx| view.apply_layout(preset, window, cx),
                            ));
                        }
                        menu = menu.separator().label("Panels");
                        for kind in TileKind::ALL {
                            menu = menu.item(item(
                                &layout_this,
                                kind.label(),
                                (kind == TileKind::Threads)
                                    .then(|| Box::new(ToggleThreads) as Box<dyn Action>),
                                false,
                                layout.contains(kind),
                                move |view, window, cx| {
                                    view.relayout(window, cx, |layout| layout.toggle(kind));
                                    view.refresh();
                                },
                            ));
                        }
                        menu.separator().item(item(
                            &layout_this,
                            "Last work status",
                            None,
                            !can_show_work_status,
                            work_status_visible,
                            |view, _, cx| view.show_last_work_notice(cx),
                        ))
                    }),
            )
            .child(
                components::icon_button("status-keys", IconName::Keyboard, "Keys")
                    .tooltip("Keys and mouse")
                    .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
            )
            .into_any_element()
    }
    fn notice_line(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        if self.notice.is_empty() {
            return None;
        }
        let restorable = self
            .last_work_notice
            .as_ref()
            .is_some_and(|(_, last)| last == &self.notice);
        let dismiss = components::icon_button("dismiss-notice", IconName::Close, "Dismiss")
            .tooltip(if restorable {
                "Hide status · restore from Layout → Last work status"
            } else {
                "Dismiss"
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.notice.clear();
                cx.notify();
            }));
        Some(
            components::banner(
                "workspace-notice",
                self.notice_tone(),
                self.notice.clone(),
                vec![],
                Some(dismiss),
            )
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .into_any_element(),
        )
    }
    fn launcher_overlay(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let items = self.launcher_items(cx);
        let viewport = window.viewport_size();
        // At most 580 wide with 32 pt either side; the results fit their
        // content up to 60 % of the window (less the search field).
        let width = px(580.).min(viewport.width - px(64.));
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
                    .h(tokens::COMPACT_CONTROL)
                    .px_2()
                    .rounded(tokens::RADIUS_CONTROL)
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
        div()
            .id("launcher-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            .pt(px(60.))
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
                    .p_2()
                    .bg(rgb(p.tile))
                    .border_1()
                    .border_color(rgb(p.border))
                    .rounded(tokens::RADIUS_PANEL)
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
    /// Focus lands in the destination's visible content: the Note's editor,
    /// the Chat's composer, the Notes search and the All chats filter.
    /// Other panels focus their body, then (after the frame that draws it)
    /// its first control; with none, focus stays on the body.
    pub(super) fn focus_tile_contents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Explicit navigation also reveals a panel manually scrolled away since last focus.
        self.tile_reveal = None;
        self.focus_first = self.focus_landing(window, cx);
        cx.notify();
    }
    /// Focus the focused panel's landing from the focus table. Returns the
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
    /// focus first moves to the focused panel's landing, which stays
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
    /// The panel body's focus handle, tracked by the element around it.
    pub(super) fn panel_body(&self, kind: TileKind) -> &gpui_kit::FocusHandle {
        &self.panel_focus[&kind]
    }
    /// After the frame that drew the panel, move from its body (or the
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
        let _retained = (&self.subscriptions, &self.poll);
        let p = pal();
        self.schedule_focus_first(window);
        // Short windows collapse the chat queue to one row.
        self.short_window = window.viewport_size().height < px(SHORT_HEIGHT);
        let tiles = if self.layout.settings_open() {
            self.settings_surface(cx)
        } else {
            self.tile_row(window, cx)
        };
        let mut root = div()
            .id("workspace-root")
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .track_focus(&self.root_focus)
            .key_context("BrnWorkspace")
            // Escape closes Settings last: bubbling, it reaches the root only
            // after the launcher, an open menu or popover, a dialog and the
            // stacked comments section had their turn. It never stops work.
            .on_key_down(
                cx.listener(|view, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape"
                        && view.layout.settings_open()
                        && !view.launcher_open
                    {
                        view.relayout(window, cx, |layout| layout.close(TileKind::Settings));
                        cx.stop_propagation();
                    }
                }),
            )
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(p.paper))
            .text_color(rgb(p.ink))
            .on_action(cx.listener(|this, _: &Save, window, cx| {
                if this.review.as_ref().is_some_and(|review| review.edited) {
                    this.prepare_review_edit(window, cx);
                } else if this.note.is_some() {
                    this.save(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Ask, window, cx| {
                if this.comment_input.focus_handle(cx).is_focused(window) {
                    this.post_comment(window, cx);
                } else {
                    this.ask(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Home, window, cx| {
                this.show_tile(TileKind::Overview, window, cx)
            }))
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
            .on_action(cx.listener(|this, _: &Rewrite, _, cx| this.rewrite_note(cx)))
            .on_action(cx.listener(|this, _: &ToggleThreads, window, cx| {
                this.relayout(window, cx, |layout| layout.toggle(TileKind::Threads))
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousTile, window, cx| {
                this.layout.cycle_focus(-1);
                this.focus_tile_contents(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusNextTile, window, cx| {
                this.layout.cycle_focus(1);
                this.focus_tile_contents(window, cx);
            }))
            .on_action(cx.listener(|this, _: &MaximizeTile, window, cx| {
                if let Some(kind) = this.layout.focus() {
                    this.relayout(window, cx, |layout| layout.toggle_maximize(kind));
                }
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.show_tile(TileKind::Settings, window, cx)
            }))
            .on_action(cx.listener(|this, _: &LayoutWrite, window, cx| {
                this.apply_layout(Preset::Write, window, cx)
            }))
            .on_action(cx.listener(|this, _: &LayoutChat, window, cx| {
                this.apply_layout(Preset::Chat, window, cx)
            }))
            .on_action(cx.listener(|this, _: &LayoutSolo, window, cx| {
                this.apply_layout(Preset::Solo, window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowKeys, window, cx| this.show_keys(window, cx)))
            .child(self.top_bar(window, cx))
            .child(self.navigation_bar(window, cx))
            .child(tiles)
            .children(self.notice_line(cx))
            .child(self.status_bar(cx));
        if self.launcher_open {
            root = root.child(self.launcher_overlay(window, cx));
        }
        root
    }
}
