//! Navigation and workspace tiles: threads, notes, overview, Actions, What
//! changed and Settings. Every control calls the existing checked services.
use super::shell::{item, pill};
use super::*;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu};
use gpui_kit::{SharedString, WeakEntity};

pub(super) fn scroll(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
}

/// A compact clickable list row with optional trailing metadata.
pub(super) fn row(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let p = pal();
    let label = label.into();
    let meta = meta.into();
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .text_size(UI_TEXT)
        .cursor_pointer()
        .bg(rgb(if selected { p.selection } else { p.tile }))
        .hover(|style| style.bg(rgb(pal().hover)))
        .aria_label(label.clone())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(label),
        )
        .when(!meta.is_empty(), |row| {
            row.child(
                div()
                    .flex_shrink_0()
                    .font_family(UI_FONT)
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .child(meta),
            )
        })
        .test_support()
}

pub(super) fn thread_menu(
    menu: PopupMenu,
    this: &WeakEntity<ThreadsUi>,
    id: String,
    resolved: bool,
    note: Option<String>,
) -> PopupMenu {
    let open = id.clone();
    let lifecycle = id.clone();
    let mut menu = menu
        .item(item(
            this,
            "Open",
            None,
            false,
            false,
            move |view, window, cx| view.open_thread(&open, window, cx),
        ))
        .item(item(
            this,
            if resolved { "Unarchive" } else { "Archive" },
            None,
            false,
            false,
            move |view, _, cx| {
                if resolved {
                    view.reopen_thread(&lifecycle, cx)
                } else {
                    view.resolve_thread(&lifecycle, cx)
                }
            },
        ));
    if let Some(note) = note {
        menu = menu.item(item(
            this,
            "Open note",
            None,
            false,
            false,
            move |view, window, cx| view.open_note(&note, None, window, cx),
        ));
    }
    menu = menu.separator();
    let chat = id.clone();
    menu = menu.item(item(
        this,
        "Move to project…",
        None,
        false,
        false,
        move |view, window, cx| view.move_chat_dialog(&chat, window, cx),
    ));
    menu
}

fn note_menu(menu: PopupMenu, this: &WeakEntity<ThreadsUi>, id: String) -> PopupMenu {
    let open = id.clone();
    menu.item(item(
        this,
        "Open",
        None,
        false,
        false,
        move |view, window, cx| view.open_note(&open, None, window, cx),
    ))
    .item(item(
        this,
        "New chat about this note",
        None,
        false,
        false,
        move |view, window, cx| {
            view.open_note(&id, None, window, cx);
            if view.open_note_id() == Some(id.as_str()) {
                view.new_writing_thread(window, cx);
            }
        },
    ))
}

pub(super) fn new_menu(menu: PopupMenu, this: &WeakEntity<ThreadsUi>, busy: bool) -> PopupMenu {
    menu.item(item(
        this,
        "New chat",
        Some(Box::new(NewThread)),
        false,
        false,
        |view, window, cx| view.new_thread(window, cx),
    ))
    .item(item(
        this,
        "New note",
        Some(Box::new(NewNote)),
        false,
        false,
        |view, window, cx| view.new_note(window, cx),
    ))
    .separator()
    .item(item(
        this,
        "Add useful information…",
        None,
        busy,
        false,
        |view, window, cx| view.choose_import(ImportIntent::UsefulInformation, window, cx),
    ))
    .item(item(
        this,
        "Import full note…",
        None,
        busy,
        false,
        |view, window, cx| view.choose_import(ImportIntent::FullNote, window, cx),
    ))
}

impl ThreadsUi {
    pub(super) fn tile_controls(
        &self,
        kind: TileKind,
        cx: &mut Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        match kind {
            TileKind::Threads | TileKind::Notes => vec![
                components::icon_button(
                    format!("{}-new", kind.key()),
                    IconName::Plus,
                    "New chat, note or information",
                )
                .dropdown_menu(move |menu, _, _| new_menu(menu, &this, busy))
                .into_any_element(),
            ],
            TileKind::Chat => self.chat_controls(cx),
            TileKind::Note => self.note_controls(cx),
            _ => vec![],
        }
    }
    pub(super) fn threads_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let bar = lists::sidebar(&self.thread_rows, self.chat_thread());
        let mut list = scroll("workspace-nav");
        if !bar.needs.is_empty() || !self.buffers.is_empty() || !self.candidates.is_empty() {
            list = list.child(section("Needs you"));
        }
        for row_data in &bar.needs {
            let id = row_data.id.clone();
            let menu_id = id.clone();
            let (resolved, note) = (row_data.resolved, row_data.note.clone());
            let this = this.clone();
            list = list.child(
                row(
                    format!("attention-{id}"),
                    format!("● {}", row_data.title),
                    row_data
                        .reasons
                        .first()
                        .map(|reason| short(reason))
                        .unwrap_or_default(),
                    self.chat_thread() == Some(id.as_str()),
                )
                .on_click(cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)))
                .context_menu(move |menu, _, _| {
                    thread_menu(menu, &this, menu_id.clone(), resolved, note.clone())
                }),
            );
        }
        for (index, buffer) in self.buffers.iter().enumerate() {
            let buffer = buffer.clone();
            let id = buffer.note.clone();
            let name = self
                .records
                .iter()
                .find(|record| record.id == id)
                .map(title)
                .unwrap_or_else(|| "Unavailable note".into());
            list = list.child(
                row(
                    format!("recover-{index}"),
                    format!("Recover edit · {name}"),
                    "unsaved",
                    false,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_note(&id, Some(buffer.clone()), window, cx)
                })),
            );
        }
        for candidate in &self.candidates {
            let candidate = candidate.clone();
            list = list.child(
                row(
                    format!("candidate-{}", candidate.operation.as_str()),
                    format!("Review · {}", short(&candidate.request.reason)),
                    "proposal",
                    false,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_review(candidate.clone(), window, cx)
                })),
            );
        }
        list = list.child(section("Recent chats"));
        if bar.open.is_empty() {
            list = list.child(empty(
                "threads-empty",
                "Your chats appear here. ⌘T starts one; chats stay in history until you archive them.",
            ));
        }
        for row_data in &bar.open {
            let id = row_data.id.clone();
            let menu_id = id.clone();
            let (resolved, note) = (row_data.resolved, row_data.note.clone());
            let this = this.clone();
            let meta = if row_data.has_action {
                "action"
            } else if row_data.note.is_some() {
                "note"
            } else {
                ""
            };
            list = list.child(
                row(
                    format!("thread-{id}"),
                    row_data.title.clone(),
                    meta,
                    self.chat_thread() == Some(id.as_str()),
                )
                .on_click(cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)))
                .context_menu(move |menu, _, _| {
                    thread_menu(menu, &this, menu_id.clone(), resolved, note.clone())
                }),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.chat_navigation(true, cx))
            .child(div().flex_1().min_h_0().child(list.test_support()))
            .into_any_element()
    }
    fn chat_navigation(&self, recent: bool, cx: &mut Context<Self>) -> gpui_kit::Div {
        let counts = lists::sidebar(&self.thread_rows, self.chat_thread());
        let mut tabs = div().flex().flex_wrap().flex_shrink_0().gap_1().p_2();
        for (id, label, filter) in [
            ("chats-recent", "Recent".to_owned(), None),
            (
                "all-threads",
                format!("All · {}", counts.total),
                Some(ThreadFilter::All),
            ),
            (
                "resolved-count",
                format!("Archived · {}", counts.resolved),
                Some(ThreadFilter::Resolved),
            ),
        ] {
            tabs = tabs.child(
                pill(id, label)
                    .selected(if let Some(filter) = filter {
                        !recent && self.thread_filter == filter
                    } else {
                        recent
                    })
                    .on_click(cx.listener(move |view, _, window, cx| {
                        if let Some(filter) = filter {
                            view.thread_filter = filter;
                            view.show_tile(TileKind::AllThreads, window, cx);
                        } else {
                            view.show_tile(TileKind::Threads, window, cx);
                        }
                    })),
            );
        }
        tabs
    }
    pub(super) fn all_threads_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let query = self.threads_query.read(cx).value().to_string();
        let grouped = lists::all_threads(
            &self.thread_rows,
            self.thread_filter,
            &query,
            lists::today(),
        );
        let mut filters = div().flex().flex_wrap().gap_1();
        for filter in ThreadFilter::ALL
            .into_iter()
            .filter(|filter| !matches!(filter, ThreadFilter::All | ThreadFilter::Resolved))
        {
            filters = filters.child(
                pill(format!("threads-filter-{}", filter.key()), filter.label())
                    .selected(self.thread_filter == filter)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.thread_filter = filter;
                        cx.notify();
                    })),
            );
        }
        let controls = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            .pb_2()
            .child(Input::new(&self.threads_query).w_full())
            .child(filters)
            .child(metadata(if grouped.matched > lists::LIST_LIMIT {
                format!(
                    "{} chats · showing the {} most recent; keep typing to narrow",
                    grouped.matched,
                    lists::LIST_LIMIT
                )
            } else {
                format!("{} chats", grouped.matched)
            }));
        let mut list = scroll("all-threads-list");
        if grouped.groups.is_empty() {
            list = list.child(empty("all-threads-empty", "No chats match this filter."));
        }
        for (name, rows) in grouped.groups {
            list = list.child(section(name));
            for row_data in rows {
                let id = row_data.id.clone();
                let menu_id = id.clone();
                let (resolved, note) = (row_data.resolved, row_data.note.clone());
                let this = this.clone();
                let meta = if row_data.needs_you() {
                    "needs you".to_owned()
                } else if resolved {
                    "archived".to_owned()
                } else if row_data.has_action {
                    "action".to_owned()
                } else if row_data.note.is_some() {
                    "note".to_owned()
                } else {
                    String::new()
                };
                list = list.child(
                    row(
                        format!("all-thread-{id}"),
                        row_data.title.clone(),
                        meta,
                        self.chat_thread() == Some(id.as_str()),
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                    )
                    .context_menu(move |menu, _, _| {
                        thread_menu(menu, &this, menu_id.clone(), resolved, note.clone())
                    }),
                );
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.chat_navigation(false, cx))
            .child(controls)
            .child(div().flex_1().min_h_0().child(list.test_support()))
            .into_any_element()
    }
    pub(super) fn notes_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let query = self.search.read(cx).value().to_string();
        let current = self.open_note_id().map(str::to_owned);
        let note_row = |id_prefix: &str, data: &NoteRow, cx: &mut Context<Self>| {
            let id = data.id.clone();
            let menu_id = id.clone();
            let this = this.clone();
            let kind = match (data.protected, data.imported) {
                (true, true) => "protected · imported",
                (true, false) => "protected",
                (false, true) => "imported",
                _ => "",
            };
            let meta = match (data.inbound, kind.is_empty()) {
                (0, _) => kind.to_owned(),
                (n, true) => format!("← {n}"),
                (n, false) => format!("{kind} · ← {n}"),
            };
            row(
                format!("{id_prefix}{id}"),
                data.title.clone(),
                meta,
                current.as_deref() == Some(id.as_str()),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)))
            .context_menu(move |menu, _, _| note_menu(menu, &this, menu_id.clone()))
        };
        let frame = |header: gpui_kit::Div, body: gpui_kit::AnyElement| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .min_h_0()
                .child(
                    header
                        .flex_shrink_0()
                        .p_2()
                        .border_b_1()
                        .border_color(rgb(pal().border)),
                )
                .child(div().flex_1().min_h_0().child(body))
                .into_any_element()
        };
        let mut header = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(Input::new(&self.search).w_full())
            .child(metadata(if self.results.preparing {
                format!("{} current notes · preparing search…", self.note_rows.len())
            } else {
                format!("{} current notes", self.note_rows.len())
            }));
        let mut list = scroll("notes-list");
        if !query.trim().is_empty() {
            let (titles, total) = lists::filter_notes(&self.note_rows, &query, 100);
            list = list.child(section(format!("Titles · {total}")));
            for data in titles {
                list = list.child(note_row("note-", data, cx));
            }
            if total > 100 {
                list = list.child(metadata(format!(
                    "{} more titles · keep typing",
                    total - 100
                )));
            }
            let results = self.search_results(cx);
            return frame(
                header,
                list.child(results).test_support().into_any_element(),
            );
        }
        if self.note_rows.is_empty() {
            return frame(
                header,
                list.child(empty(
                    "notes-empty",
                    "Create a note (⌘N) or add information with ＋ to begin.",
                ))
                .test_support()
                .into_any_element(),
            );
        }
        let linked = lists::notes_linked_to_open(&self.records, &self.thread_rows);
        let letters_present: Vec<char> = {
            let mut letters: Vec<char> = self
                .note_rows
                .iter()
                .map(|row| lists::letter(&row.title))
                .collect();
            letters.dedup();
            letters
        };
        let letter = if letters_present.contains(&self.notes_letter) {
            self.notes_letter
        } else {
            lists::letter(&self.note_rows[0].title)
        };
        let view = lists::notes_view(&self.note_rows, &linked, letter);
        let mut letters = div().flex().flex_wrap().gap_1();
        for &(ch, count) in &view.letters {
            letters = letters.child(
                pill(format!("notes-letter-{}", ch as u32), ch.to_string())
                    .selected(ch == letter)
                    .tooltip(format!("{count} notes"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.notes_letter = ch;
                        cx.notify();
                    })),
            );
        }
        header = header.child(letters);
        list = list.child(self.topics_section(cx)).child(section("Recent"));
        for data in view.recent {
            list = list.child(note_row("notes-recent-", data, cx));
        }
        if view.linked.1 > 0 {
            list = list.child(section(format!("Linked to open chats · {}", view.linked.1)));
            for data in view.linked.0 {
                list = list.child(note_row("notes-linked-", data, cx));
            }
        }
        if view.kept.1 > 0 {
            list = list.child(section(format!("Protected & imported · {}", view.kept.1)));
            for data in view.kept.0 {
                list = list.child(note_row("notes-kept-", data, cx));
            }
            if view.kept.1 > lists::SECTION_LIMIT {
                list = list.child(metadata(format!(
                    "{} more · filter above or browse A–Z",
                    view.kept.1 - lists::SECTION_LIMIT
                )));
            }
        }
        if view.most_linked.1 > 0 {
            list = list.child(section(format!("Most linked · {}", view.most_linked.1)));
            for data in view.most_linked.0 {
                list = list.child(note_row("notes-most-linked-", data, cx));
            }
        }
        list = list.child(section("All A–Z"));
        for data in view.letter.0 {
            list = list.child(note_row("note-", data, cx));
        }
        if view.letter.1 > lists::LETTER_LIMIT {
            list = list.child(metadata(format!(
                "{} more under {letter} · filter above",
                view.letter.1 - lists::LETTER_LIMIT
            )));
        }
        frame(header, list.test_support().into_any_element())
    }
    pub(super) fn overview_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = scroll("home-content").gap_3().p_3().child(
            panel()
                .child(heading("Your working knowledge"))
                .child("Give BRN information and a goal. Your notes, work and decisions stay connected.")
                .child(metadata(
                    "Ask in the chat tile, open notes from the notes tile, or press ⌘K.",
                )),
        );
        if self.delegation_host() == Some(TileKind::Overview) {
            view = view.child(self.working_knowledge(cx));
        }
        let mut queue = panel()
            .id("home-needs-you")
            .test_support()
            .child(subheading("Needs you"));
        let needs: Vec<_> = self
            .thread_rows
            .iter()
            .filter(|row| row.needs_you())
            .collect();
        if needs.is_empty() {
            queue = queue.child(empty(
                "home-attention-empty",
                "Nothing needs your decision right now.",
            ));
        }
        for data in needs {
            let id = data.id.clone();
            let mut entry = div().flex().flex_col().gap_1().child(
                row(
                    format!("home-attention-{id}"),
                    data.title.clone(),
                    "",
                    false,
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                ),
            );
            for reason in &data.reasons {
                entry = entry.child(div().px_2().child(reason.clone()));
            }
            queue = queue.child(entry);
        }
        view = view.child(queue);
        if !self.buffers.is_empty() {
            let mut recovery = panel().child(subheading("Recover your edits")).child(
                "These edits remain separate from current notes until you Save or discard them.",
            );
            for (index, buffer) in self.buffers.iter().enumerate() {
                let buffer = buffer.clone();
                let id = buffer.note.clone();
                let label = self
                    .records
                    .iter()
                    .find(|r| r.id == id)
                    .map(title)
                    .unwrap_or_else(|| "Unavailable note".into());
                recovery = recovery.child(
                    button(format!("home-recover-{index}"))
                        .label(format!("Recover {}", short(&label)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, Some(buffer.clone()), window, cx)
                        })),
                );
            }
            view = view.child(recovery);
        }
        if !self.candidates.is_empty() {
            let mut review = panel().child(subheading("Review changes"));
            for candidate in &self.candidates {
                let candidate = candidate.clone();
                review = review.child(
                    button(format!("home-candidate-{}", candidate.operation.as_str()))
                        .label(short(&candidate.request.reason))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_review(candidate.clone(), window, cx)
                        })),
                );
            }
            view = view.child(review);
        }
        let mut notes: Vec<&NoteRow> = self.note_rows.iter().collect();
        notes.sort_by_key(|row| std::cmp::Reverse(row.order));
        let mut note_tile = panel()
            .id("home-notes")
            .test_support()
            .child(subheading("Recent notes"))
            .child(metadata(format!("{} current notes", notes.len())));
        if notes.is_empty() {
            note_tile = note_tile.child(empty(
                "home-notes-empty",
                "Create a note or import information to begin.",
            ));
        }
        for data in notes.into_iter().take(6) {
            let id = data.id.clone();
            note_tile = note_tile.child(
                button(format!("home-note-{id}"))
                    .label(short(&data.title))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, None, window, cx)
                    })),
            );
        }
        let mut recent = panel()
            .id("home-recent-changes")
            .test_support()
            .child(subheading("What changed"));
        for receipt in self.history.iter().rev().take(6) {
            recent = recent.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(receipt.reason.clone())
                    .child(metadata(receipt.committed_at.clone())),
            );
        }
        if self.history.is_empty() {
            recent = recent.child(empty(
                "home-changes-empty",
                "Your meaningful changes will appear here.",
            ));
        }
        recent =
            recent.child(
                button("home-open-history")
                    .label("Open What changed and Undo")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::History, window, cx)
                    })),
            );
        view.child(note_tile)
            .child(recent)
            .child(self.knowledge_panel(cx))
            .test_support()
            .into_any_element()
    }
    pub(super) fn action_row(
        &self,
        record: &Record,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let RecordData::Action(action) = &record.data else {
            return div().into_any_element();
        };
        let mut row = panel()
            .id(format!("action-tile-{}", record.id))
            .test_support()
            .child(
                div()
                    .id(format!("action-description-{}", record.id))
                    .aria_label(action.description.clone())
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(action.description.clone())
                    .test_support(),
            )
            .child(format!(
                "{:?}{}",
                action.state,
                action
                    .due
                    .as_ref()
                    .map(|due| format!(" · due {due}"))
                    .unwrap_or_default()
            ));
        if let Some(evidence) = &action.evidence {
            row = row.child(evidence.clone());
        }
        if action.state != ActionState::Done && action.state != ActionState::Cancelled {
            let record_open = record.clone();
            let mut opened = action.clone();
            opened.state = ActionState::Open;
            let record_wait = record.clone();
            let mut waiting = action.clone();
            waiting.state = ActionState::Waiting;
            let record_done = record.clone();
            let mut done = action.clone();
            done.state = ActionState::Done;
            done.evidence = Some("Owner confirmed completion in BRN.".into());
            let record_cancel = record.clone();
            let mut cancelled = action.clone();
            cancelled.state = ActionState::Cancelled;
            row = row.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        button(format!("open-action-{}", record.id))
                            .label(if action.state == ActionState::Suggested {
                                "Accept task"
                            } else {
                                "Open"
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner opened an Action",
                                    record_open.clone(),
                                    RecordData::Action(opened.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        button(format!("wait-action-{}", record.id))
                            .label("Waiting")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner marked an Action Waiting",
                                    record_wait.clone(),
                                    RecordData::Action(waiting.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        button(format!("done-action-{}", record.id))
                            .label("Confirm done")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner confirmed an Action is complete",
                                    record_done.clone(),
                                    RecordData::Action(done.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        button(format!("cancel-action-{}", record.id))
                            .label("Cancel task")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner cancelled an Action",
                                    record_cancel.clone(),
                                    RecordData::Action(cancelled.clone()),
                                    cx,
                                )
                            })),
                    ),
            );
        }
        row.into_any_element()
    }
    pub(super) fn actions_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = scroll("actions-content").gap_2().child(metadata(
            "Tasks keep their own progress, including after a chat is archived.",
        ));
        if !self
            .records
            .iter()
            .any(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
        {
            view = view.child(panel().child(empty(
                "actions-empty",
                "No tasks yet. Suggested work and tasks you assign will appear here.",
            )));
        }
        for (status, key, label) in [
            (ActionState::Suggested, "suggested", "Suggested"),
            (ActionState::Open, "open", "Open"),
            (ActionState::Waiting, "waiting", "Waiting"),
            (ActionState::Done, "done", "Done"),
            (ActionState::Cancelled, "cancelled", "Cancelled"),
        ] {
            let records: Vec<_> = self
                .records
                .iter()
                .filter(|r| {
                    !r.archived && matches!(&r.data, RecordData::Action(a) if a.state == status)
                })
                .collect();
            let mut group = panel()
                .id(format!("actions-group-{key}"))
                .test_support()
                .bg(rgb(pal().alt))
                .child(subheading(format!("{label} · {}", records.len())));
            if records.is_empty() {
                group = group.child(metadata(format!("No {} actions.", label.to_lowercase())));
            }
            for record in records {
                group = group.child(self.action_row(record, cx));
            }
            view = view.child(group);
        }
        view.test_support().into_any_element()
    }
    pub(super) fn history_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = scroll("history-content").gap_2().child(metadata(
            "Recorded changes and the same checked Undo used throughout your workspace.",
        ));
        if self.history.is_empty() {
            view = view.child(panel().child(empty(
                "history-empty",
                "No recorded changes yet. Saved changes and their Undo controls will appear here.",
            )));
        }
        for receipt in self.history.iter().rev() {
            let id = receipt.operation.as_str().to_owned();
            let mut entry = panel()
                .id(format!("receipt-{id}"))
                .test_support()
                .child(subheading(receipt.reason.clone()))
                .child(metadata(receipt.committed_at.clone()));
            for write in &receipt.writes {
                entry = entry.child(format!(
                    "{} · version {}",
                    title(&write.after),
                    write.after.version
                ));
            }
            if !receipt.needs_refresh.is_empty() {
                entry = entry.child("Directly derived work may need refreshing after this change. Review its current inputs before updating it.");
                if let Ok(records) = self.workspace.refresh_records(&id) {
                    for record in records {
                        let note =
                            matches!(record.data, RecordData::Note(_)).then(|| record.id.clone());
                        entry = entry.child(
                            button(format!("refresh-record-{id}-{}", record.id))
                                .label(format!("May need refresh: {}", short(&title(&record))))
                                .on_click(cx.listener(move |this, _, window, cx| match &note {
                                    Some(id) => this.open_note(id, None, window, cx),
                                    None => this.show_tile(TileKind::Actions, window, cx),
                                })),
                        );
                    }
                }
                let receipt_id = id.clone();
                entry = entry.child(
                    button(format!("refresh-followup-{id}"))
                        .label("Open refresh follow-up")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            match this.workspace.refresh_followup(&receipt_id) {
                                Ok(thread) => {
                                    this.refresh();
                                    this.open_thread(&thread.id, window, cx);
                                }
                                Err(error) => {
                                    this.fail(error.to_string());
                                    cx.notify();
                                }
                            }
                        })),
                );
            }
            entry = entry.child(
                button(format!("undo-{id}"))
                    .label("Undo this change")
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.undo_receipt(&id, window, cx)),
                    ),
            );
            view = view.child(entry);
        }
        view.test_support().into_any_element()
    }
    pub(super) fn working_knowledge(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let status = delegation_status(&self.settings);
        let mut knowledge = panel()
            .id(if self.settings.delegation_chosen { "delegation-settings" } else { "delegation-pending" })
            .child(subheading("Working knowledge"))
            .child(div().id("delegation-status").aria_label(status).child(status).test_support())
            .child("Maintain ordinary notes lets BRN update ordinary notes and useful source references. Review changes first keeps proposed changes for your approval. Protected notes still need an applicable instruction or your review.")
            .child(button("delegation-maintain")
                .label("Maintain ordinary notes")
                .selected(self.settings.delegation_chosen && !self.settings.review_first)
                .on_click(cx.listener(|this, _, _, cx| this.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx))))
            .child(button("delegation-review-first")
                .label("Review changes first")
                .selected(self.settings.delegation_chosen && self.settings.review_first)
                .on_click(cx.listener(|this, _, _, cx| this.choose_delegation(DelegationChoice::ReviewFirst, cx))));
        if self.settings.delegation_chosen {
            knowledge = knowledge.child(
                button("maintenance-setting")
                    .label(if self.settings.maintenance {
                        "Pause maintenance"
                    } else {
                        "Resume maintenance"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        match this.workspace.pause(this.settings.maintenance) {
                            Ok(outcome) => this.report_outcome(&outcome),
                            Err(error) => this.fail(error.to_string()),
                        }
                        this.refresh();
                        cx.notify();
                    })),
            );
        }
        knowledge.test_support().into_any_element()
    }
    pub(super) fn settings_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut provider = div().flex().flex_wrap().gap_2();
        for (value, label) in [
            ("chatgpt", "Codex / ChatGPT"),
            ("copilot", "GitHub Copilot"),
        ] {
            provider = provider.child(
                button(format!("provider-{value}"))
                    .label(label)
                    .selected(
                        self.settings.provider.eq_ignore_ascii_case(value)
                            || value == "Codex"
                                && self.settings.provider.eq_ignore_ascii_case("chatgpt"),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.provider = value.into();
                        cx.notify();
                    })),
            );
        }
        let mut efforts = div().flex().flex_wrap().gap_2();
        for effort in ["low", "medium", "high"] {
            efforts = efforts.child(
                button(format!("effort-{effort}"))
                    .label(effort)
                    .selected(self.settings.effort == effort)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.effort = effort.into();
                        cx.notify();
                    })),
            );
        }
        let mut helper_choice = div().flex().flex_wrap().gap_2();
        for (value, label) in [
            ("off", "Off"),
            ("Codex", "Codex / ChatGPT"),
            ("Copilot", "GitHub Copilot"),
        ] {
            let selected = match &self.settings.helper {
                None => value == "off",
                Some(helper) => helper.provider.eq_ignore_ascii_case(value),
            };
            helper_choice = helper_choice.child(
                button(format!("helper-provider-{value}"))
                    .label(label)
                    .selected(selected)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.helper = (value != "off").then(|| {
                            let mut helper = this.settings.helper.clone().unwrap_or(
                                brn_threads_app::HelperSettings {
                                    provider: String::new(),
                                    model: String::new(),
                                    effort: "high".into(),
                                    auth_file: None,
                                    credentials_dir: None,
                                },
                            );
                            helper.provider = value.into();
                            helper
                        });
                        cx.notify();
                    })),
            );
        }
        let mut helper_tile = panel()
            .id("settings-helper")
            .test_support()
            .child(subheading("Helper sub-agents"))
            .child(
                "When on, the chat model can ask read-only helpers to research your notes. \
                 Helpers cannot change notes and never replace an unavailable model.",
            )
            .child(metadata(brn_threads_app::HELPER_TESTED_NOTE))
            .child(metadata("Helper provider"))
            .child(helper_choice);
        if let Some(helper) = &self.settings.helper {
            let mut helper_efforts = div().flex().flex_wrap().gap_2();
            for effort in ["low", "medium", "high"] {
                helper_efforts = helper_efforts.child(
                    button(format!("helper-effort-{effort}"))
                        .label(effort)
                        .selected(helper.effort == effort)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(helper) = &mut this.settings.helper {
                                helper.effort = effort.into();
                            }
                            cx.notify();
                        })),
                );
            }
            helper_tile = helper_tile
                .child(metadata("Helper model"))
                .child(
                    Input::new(&self.helper_model)
                        .w_full()
                        .aria_label("Helper model"),
                )
                .child(metadata("Helper reasoning effort"))
                .child(helper_efforts)
                .child(metadata(
                    "The helper uses the credential location above for its provider. Save settings to apply.",
                ));
        }
        let provider_tile = panel()
            .id("settings-provider")
            .test_support()
            .child(subheading("Provider and model"))
            .child(metadata("Provider"))
            .child(provider)
            .child(metadata("Model"))
            .child(Input::new(&self.model).w_full().aria_label("Model"))
            .child(metadata("Reasoning effort"))
            .child(efforts)
            .child(metadata("Codex auth file"))
            .child(
                Input::new(&self.auth_path)
                    .w_full()
                    .aria_label("Codex auth file"),
            )
            .child(metadata("Copilot credentials folder"))
            .child(
                Input::new(&self.credentials_path)
                    .w_full()
                    .aria_label("Copilot credentials folder"),
            )
            .child(metadata(
                "Credentials stay outside notes, exports and Changes. The Codex auth file is used only with Codex / ChatGPT.",
            ))
            .child(
                button("save-settings")
                    .label("Save settings")
                    .disabled(self.worker.is_some())
                    .on_click(cx.listener(|this, _, _, cx| this.configure(cx))),
            )
            .child(
                button("connect-copilot")
                    .label("Connect GitHub Copilot…")
                    .disabled(self.worker.is_some())
                    .tooltip("Sign in with GitHub in your browser; uses the folder above or ~/.config/brn/copilot")
                    .on_click(cx.listener(|this, _, window, cx| this.connect_copilot(window, cx))),
            );
        let copies = panel()
            .id("settings-copies")
            .test_support()
            .child(subheading("Portable copies"))
            .child(
                panel()
                    .bg(rgb(pal().alt))
                    .child(subheading("Markdown export"))
                    .child("Export a current snapshot of your notes and retained figures.")
                    .child(button("export-now").label("Export Markdown now…").on_click(
                        cx.listener(|this, _, window, cx| this.choose_output(false, window, cx)),
                    ))
                    .child(metadata(self.export_status.clone())),
            )
            .child(
                panel()
                    .bg(rgb(pal().alt))
                    .child(subheading("Full backup"))
                    .child("Includes notes, discussions, history, figures and recovery state.")
                    .child(
                        div()
                            .id("backup-directory")
                            .aria_label(
                                self.settings
                                    .backup_directory
                                    .clone()
                                    .unwrap_or_else(|| "Backup folder not chosen".into()),
                            )
                            .child(
                                self.settings
                                    .backup_directory
                                    .clone()
                                    .unwrap_or_else(|| "Backup folder not chosen".into()),
                            )
                            .test_support(),
                    )
                    .child(metadata(
                        "Choose a folder covered by your separate backup system.",
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                button("backup-change-folder")
                                    .label("Change folder…")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.choose_backup_folder(false, window, cx)
                                    })),
                            )
                            .child(button("backup-now").label("Create full backup").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.choose_output(true, window, cx)
                                }),
                            )),
                    )
                    .child(metadata(self.backup_status.clone())),
            );
        let p = pal();
        let mut general = panel()
            .id("settings-general")
            .test_support()
            .child(subheading("Theme"))
            .child(metadata("Also in ⌘K."));
        for theme in ThemeId::ALL {
            let colors = theme.palette();
            let swatches = div().flex().gap_1().children(
                [
                    colors.paper,
                    colors.tile,
                    colors.alt,
                    colors.accent,
                    colors.ink,
                ]
                .into_iter()
                .map(|color| {
                    div()
                        .w(px(14.))
                        .h(px(14.))
                        .border_1()
                        .border_color(rgb(p.border))
                        .bg(rgb(color))
                }),
            );
            general = general.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        button(format!("settings-theme-{}", theme.key()))
                            .label(theme.label())
                            .selected(self.theme == theme)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.choose_theme(theme, window, cx)
                            })),
                    )
                    .child(swatches)
                    .child(metadata(theme.description())),
            );
        }
        general = general.child(subheading("Keys and mouse")).child(
            button("settings-keys")
                .label("Show keys and mouse help")
                .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
        );
        let mut nav = div().flex().flex_wrap().gap_1().p_2().flex_shrink_0();
        for item in SettingsSection::ALL {
            nav = nav.child(
                button(format!("settings-section-{}", item.key()))
                    .label(item.label())
                    .selected(self.settings_section == item)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings_section = item;
                        cx.notify();
                    })),
            );
        }
        let content = match self.settings_section {
            SettingsSection::General => general.into_any_element(),
            SettingsSection::Models => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(provider_tile)
                .child(helper_tile)
                .into_any_element(),
            SettingsSection::Delegation => self.working_knowledge(cx),
            SettingsSection::Data => copies.into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(nav.border_b_1().border_color(rgb(p.border)))
            .child(
                div()
                    .id("settings-content")
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .overflow_y_scroll()
                    .child(content)
                    .test_support(),
            )
            .into_any_element()
    }
}
