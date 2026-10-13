//! Workspace shell: top bar, tile row, status bar, launcher and menus.
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::{
    Action, ClickEvent, DragMoveEvent, Focusable, MouseButton, SharedString, WeakEntity,
};

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
            .font_family(MONO)
            .text_xs()
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
    button(id).label(label).compact().text_xs().h(px(24.))
}

const KEYS: [(&str, &str); 17] = [
    ("Launcher", "⌘K · click the search field"),
    ("New thread / note", "⌘T / ⌘N · ＋ in threads and notes"),
    ("Send / new line", "↵ or ⌘↵ / ⇧↵ · send button"),
    ("Stop", "⌘. · status bar or run line"),
    ("Comment on selection", "⌘/ · selection strip · right-click"),
    ("Rewrite with AI", "⌘⇧R · note foot bar · ⋯ menu"),
    ("Save note", "⌘S · status bar · ⋯ menu"),
    ("Layouts", "⌥1 ⌥2 ⌥3 · top bar · status bar"),
    ("Focus tile", "⌘[ ⌘] · click a tile"),
    ("Maximize tile", "⌘⇧↵ · double-click its header · ⤢"),
    ("Resize tiles", "drag the gap between tiles"),
    ("Swap tiles", "drag a tile header onto another tile"),
    ("Tile menu", "right-click a tile header"),
    ("Toggle threads", "⌘\\ · layout menu"),
    ("Theme", "status bar · launcher · Settings"),
    ("Settings", "⌘, · threads tile"),
    ("Overview", "⌘0 · launcher"),
];

impl ThreadsUi {
    pub(super) fn show_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, _| {
            let p = pal();
            dialog.title("Keys and mouse").child(
                div()
                    .id("keys-help")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(KEYS.iter().map(|(what, how)| {
                        div()
                            .flex()
                            .gap_3()
                            .child(div().w(px(170.)).child(*what))
                            .child(
                                div()
                                    .font_family(MONO)
                                    .text_xs()
                                    .text_color(rgb(p.muted))
                                    .child(*how),
                            )
                    }))
                    .test_support(),
            )
        });
    }
    fn tile_title(&self, kind: TileKind) -> String {
        match kind {
            TileKind::Note => match (&self.review, &self.note) {
                (Some(review), None) => {
                    format!("review · {}", short(&review.candidate.request.reason))
                }
                (_, Some(open)) => format!("note · {}", short(&title(&open.record))),
                _ => "note".into(),
            },
            TileKind::Chat => match &self.chat {
                ChatTarget::Thread(id) => format!(
                    "chat · {}",
                    self.thread_rows
                        .iter()
                        .find(|row| &row.id == id)
                        .map_or_else(|| id.clone(), |row| short(&row.title))
                ),
                ChatTarget::NewForNote(note) => format!(
                    "chat · new thread about {}",
                    self.note_rows
                        .iter()
                        .find(|row| &row.id == note)
                        .map_or_else(|| note.clone(), |row| short(&row.title))
                ),
                ChatTarget::New => "chat · new thread".into(),
            },
            kind => kind.label().into(),
        }
    }
    fn tile_header(&self, kind: TileKind, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let key = kind.key();
        let focused = self.layout.focus() == Some(kind);
        let maximized = self.layout.maximized() == Some(kind);
        let this = cx.entity().downgrade();
        let label = self.tile_title(kind);
        let ghost = SharedString::from(label.clone());
        let mut header = div()
            .id(format!("tile-header-{key}"))
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .h(px(30.))
            .px_2()
            .font_family(MONO)
            .text_xs()
            .bg(rgb(if focused { p.active } else { p.alt }))
            .border_b_1()
            .border_color(rgb(p.border))
            .cursor_grab()
            .child(
                div()
                    .id(format!("tile-title-{key}"))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(rgb(if focused { p.accent } else { p.ink }))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .aria_label(label.clone())
                    .child(label)
                    .test_support(),
            );
        header = header.children(self.tile_controls(kind, cx));
        header
            .child(
                pill(format!("tile-max-{key}"), if maximized { "⤡" } else { "⤢" })
                    .tooltip(if maximized {
                        "Restore (⌘⇧↵ · double-click header)"
                    } else {
                        "Maximize (⌘⇧↵ · double-click header)"
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.relayout(window, cx, |layout| layout.toggle_maximize(kind))
                    })),
            )
            .child(
                pill(format!("tile-close-{key}"), "✕")
                    .tooltip("Close tile")
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
                let maximized = layout.maximized() == Some(kind);
                let mut menu = menu
                    .item(item(
                        &this,
                        if maximized { "Restore" } else { "Maximize" },
                        Some(Box::new(MaximizeTile)),
                        false,
                        false,
                        move |view, window, cx| {
                            view.relayout(window, cx, |layout| layout.toggle_maximize(kind))
                        },
                    ))
                    .item(item(
                        &this,
                        "Move left",
                        None,
                        !layout.can_move(kind, -1),
                        false,
                        move |view, _, cx| {
                            view.layout.move_by(kind, -1);
                            cx.notify();
                        },
                    ))
                    .item(item(
                        &this,
                        "Move right",
                        None,
                        !layout.can_move(kind, 1),
                        false,
                        move |view, _, cx| {
                            view.layout.move_by(kind, 1);
                            cx.notify();
                        },
                    ))
                    .item(item(
                        &this,
                        "Close",
                        None,
                        false,
                        false,
                        move |view, window, cx| {
                            view.relayout(window, cx, |layout| layout.close(kind))
                        },
                    ))
                    .separator()
                    .label("Open tile");
                for other in TileKind::ALL
                    .into_iter()
                    .filter(|other| !layout.contains(*other))
                {
                    menu = menu.item(item(
                        &this,
                        other.label(),
                        None,
                        false,
                        false,
                        move |view, _, cx| {
                            view.layout.set_focus(kind);
                            view.show_tile(other, cx);
                        },
                    ));
                }
                menu
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
        }
    }
    fn tile_row(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let visible = self.layout.visible();
        let mut row = div()
            .id("tile-row")
            .test_support()
            .flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .p(px(6.))
            .overflow_x_scroll()
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
        if visible.is_empty() {
            return row
                .items_center()
                .justify_center()
                .child(empty(
                    "tiles-empty",
                    "No tiles open · ⌘K or choose Write, Chat or Solo",
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
                        .w(px(6.))
                        .h_full()
                        .cursor_col_resize()
                        .hover(|style| style.bg(rgb(p.border)))
                        .on_drag(SplitDrag(index - 1), |_, _, _, cx| {
                            cx.new(|_| DragGhost(SharedString::default()))
                        }),
                );
            }
            let kind = tile.kind;
            let focused = self.layout.focus() == Some(kind);
            let mut frame = div()
                .id(format!("tile-{}", kind.key()))
                .test_support()
                .flex()
                .flex_col()
                .h_full()
                .min_h_0()
                .min_w(px(kind.min_width()
                    + if kind == TileKind::Note
                        && self.margin
                        && self.review.is_none()
                        && self.note.is_some()
                    {
                        250.
                    } else {
                        0.
                    }))
                .flex_basis(px(0.))
                .border_1()
                .border_color(rgb(if focused { p.accent } else { p.border }))
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
            let header = self.tile_header(kind, cx);
            let body = self.tile_body(kind, window, cx);
            row = row.child(
                frame
                    .child(header)
                    .child(div().flex_1().min_h_0().w_full().child(body)),
            );
        }
        row.into_any_element()
    }
}

pub(super) fn provider_key(provider: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "codex" | "chatgpt" => "chatgpt",
        "copilot" => "copilot",
        _ => "",
    }
}
pub(super) const PROVIDERS: [(&str, &str); 2] =
    [("chatgpt", "ChatGPT"), ("copilot", "GitHub Copilot")];

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
        let mut menu = menu;
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
                |view, _, cx| {
                    view.settings_section = SettingsSection::Models;
                    view.show_tile(TileKind::Settings, cx);
                },
            ))
    }
    pub(super) fn effort_menu(&self, menu: PopupMenu, this: &WeakEntity<Self>) -> PopupMenu {
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
    fn top_bar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
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
                div()
                    .font_family(MONO)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .text_color(rgb(p.accent))
                    .child("brn"),
            )
            .child(
                button("launcher")
                    .label("⌕ Search notes, threads, commands…  ⌘K")
                    .w(px(420.))
                    .tooltip("Launcher (⌘K)")
                    .on_click(cx.listener(|this, _, window, cx| this.open_launcher(window, cx))),
            )
            .child(div().flex_1());
        for (preset, shortcut) in Preset::ALL.into_iter().zip(["⌥1", "⌥2", "⌥3"]) {
            bar = bar.child(
                button(format!("layout-{}", preset.key()))
                    .label(preset.label())
                    .compact()
                    .selected(
                        self.layout.preset() == Some(preset) && self.layout.maximized().is_none(),
                    )
                    .tooltip(format!("{} layout ({shortcut})", preset.label()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.apply_layout(preset, window, cx)
                    })),
            );
        }
        let needs: Vec<(String, String)> = self
            .thread_rows
            .iter()
            .filter(|row| row.needs_you())
            .map(|row| (row.id.clone(), row.title.clone()))
            .collect();
        if !needs.is_empty() {
            let count = needs.len();
            bar = bar.child(
                button("needs-badge")
                    .label(format!(
                        "● {count} need{} you",
                        if count == 1 { "s" } else { "" }
                    ))
                    .compact()
                    .primary()
                    .dropdown_menu(move |mut menu, _, _| {
                        menu = menu.label("Needs you");
                        for (id, title) in &needs {
                            let id = id.clone();
                            menu = menu.item(item(
                                &this,
                                short(title),
                                None,
                                false,
                                false,
                                move |view, window, cx| view.open_thread(&id, window, cx),
                            ));
                        }
                        menu
                    }),
            );
        }
        gpui_kit::component::TitleBar::new()
            .h(TOP_BAR_HEIGHT)
            .bg(rgb(p.alt))
            .border_color(rgb(p.border))
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
            .h(px(32.))
            .px_2()
            .font_family(MONO)
            .text_xs()
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.border));
        if let Some(worker) = &self.worker {
            let status = worker.kind.status(&self.progress);
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
                    .flex_shrink(1.)
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_2()
                    .text_color(rgb(p.accent))
                    .cursor_pointer()
                    .aria_label(progress.clone())
                    .child(format!("◌ {progress}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(thread) = &thread {
                            this.open_thread(thread, window, cx);
                        }
                    }))
                    .test_support(),
            );
            if worker.kind.can_cancel() {
                bar = bar.child(
                    pill("stop-work", "■ Stop ⌘.")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                );
            }
        } else {
            let model = format!(
                "○ idle · {} · {} · {}",
                self.settings.provider, self.settings.model, self.settings.effort
            );
            let menu_this = this.clone();
            bar = bar.child(
                pill("status-model", model).dropdown_menu(move |menu, _, cx| {
                    match menu_this.upgrade() {
                        Some(view) => view.read(cx).model_menu(menu, &menu_this),
                        None => menu,
                    }
                }),
            );
        }
        if let Some(open) = &self.note {
            let name = short(&title(&open.record));
            bar = bar.child(if open.session.is_some() {
                pill("status-save", format!("● {name} unsaved · ⌘S"))
                    .tooltip("Save (⌘S)")
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
            } else {
                pill(
                    "status-save",
                    format!("{name} v{} saved", open.record.version),
                )
                .tooltip("Saved · open What changed")
                .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::History, cx)))
            });
        }
        let layout_label = match (self.layout.maximized(), self.layout.preset()) {
            (Some(kind), _) => format!("⊞ max · {}", kind.label()),
            (None, Some(preset)) => format!("⊞ {}", preset.label()),
            (None, None) => "⊞ custom".into(),
        };
        let layout_this = this.clone();
        let theme_this = this.clone();
        let current_theme = self.theme;
        let layout = self.layout.clone();
        bar.child(div().flex_1())
            .child(
                pill("status-layout", layout_label).dropdown_menu(move |mut menu, _, _| {
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
                    menu = menu.separator().label("Tiles");
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
                    menu
                }),
            )
            .child(
                pill("status-theme", format!("◐ {}", current_theme.label())).dropdown_menu(
                    move |mut menu, _, _| {
                        for theme in ThemeId::ALL {
                            menu = menu.item(item(
                                &theme_this,
                                format!("{} · {}", theme.label(), theme.description()),
                                None,
                                false,
                                theme == current_theme,
                                move |view, window, cx| view.choose_theme(theme, window, cx),
                            ));
                        }
                        menu
                    },
                ),
            )
            .child(
                pill("status-keys", "? keys")
                    .tooltip("Keys and mouse")
                    .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
            )
            .into_any_element()
    }
    fn notice_line(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        if self.notice.is_empty() {
            return None;
        }
        let p = pal();
        Some(
            div()
                .id("workspace-notice")
                .aria_label(self.notice.clone())
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap_2()
                .px_3()
                .py_1()
                .text_sm()
                .bg(rgb(p.tile))
                .border_t_1()
                .border_color(rgb(p.border))
                .child(div().flex_1().min_w_0().child(self.notice.clone()))
                .child(
                    pill("dismiss-notice", "✕")
                        .tooltip("Dismiss")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.notice.clear();
                            cx.notify();
                        })),
                )
                .test_support()
                .into_any_element(),
        )
    }
    fn launcher_overlay(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let items = self.launcher_items(cx);
        let mut list = div()
            .id("launcher-results")
            .flex()
            .flex_col()
            .max_h(px(420.))
            .overflow_y_scroll();
        if items.is_empty() {
            list = list.child(empty(
                "launcher-empty",
                "No matching notes, threads or commands.",
            ));
        }
        for (index, entry) in items.iter().enumerate() {
            let shortcut = match entry {
                lists::LaunchItem::Command(command) => command.shortcut(),
                _ => "",
            };
            let selected = index == self.launcher_index;
            let label = entry.label();
            list = list.child(
                div()
                    .id(format!("launcher-item-{index}"))
                    .flex()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .bg(rgb(if selected { p.selection } else { p.tile }))
                    .hover(|style| style.bg(rgb(pal().hover)))
                    .aria_label(label.clone())
                    .child(div().flex_1().min_w_0().child(label))
                    .child(
                        div()
                            .font_family(MONO)
                            .text_xs()
                            .text_color(rgb(p.muted))
                            .child(shortcut),
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
                    .w(px(580.))
                    .h_auto()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .bg(rgb(p.tile))
                    .border_1()
                    .border_color(rgb(p.accent))
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
    pub(super) fn focus_tile_contents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.layout.focus() {
            Some(TileKind::Note) => self.focus_visible_editor(window, cx),
            Some(TileKind::Chat) => self
                .composer
                .update(cx, |editor, cx| editor.focus(window, cx)),
            _ => self.focus_root(window, cx),
        }
        cx.notify();
    }
}

impl Render for ThreadsUi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _retained = (&self.subscriptions, &self.poll);
        let p = pal();
        let tiles = self.tile_row(window, cx);
        let mut root = div()
            .id("workspace-root")
            .track_focus(&self.root_focus)
            .key_context("BrnWorkspace")
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
            .on_action(cx.listener(|this, _: &Home, _, cx| this.show_tile(TileKind::Overview, cx)))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &NewThread, window, cx| this.new_thread(window, cx)))
            .on_action(
                cx.listener(|this, _: &FocusAsk, window, cx| this.focus_composer(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                if this.launcher_open {
                    this.close_launcher(window, cx);
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
            .on_action(
                cx.listener(|this, _: &OpenSettings, _, cx| this.show_tile(TileKind::Settings, cx)),
            )
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
            .child(self.top_bar(cx))
            .child(tiles)
            .children(self.notice_line(cx))
            .child(self.status_bar(cx));
        if self.launcher_open {
            root = root.child(self.launcher_overlay(cx));
        }
        root
    }
}
