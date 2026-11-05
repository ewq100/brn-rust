//! Navigation panels: Chats and Notes, plus shared page helpers.
//! Every control calls the existing checked services.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, pill};
use super::*;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::{SharedString, WeakEntity};

pub(super) use brn_writing_ui::native::scroll;

/// A compact clickable list row with optional trailing metadata.
pub(super) fn row(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    components::list_row(id, label, meta, selected)
}

/// Pages keep their content in one column of at most 720 px.
pub(super) const PAGE_WIDTH: f32 = 720.;

/// A page's content column, sections stacked without boxes.
pub(super) fn column() -> gpui_kit::Div {
    div().flex().flex_col().w_full().min_w_0()
}

/// A page: its one vertical scroller around a column at most `width` wide.
pub(super) fn page(
    id: impl Into<gpui_kit::ElementId>,
    width: f32,
    column: gpui_kit::Div,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    scroll(id)
        .gap_0()
        .px_4()
        .pt_0()
        .pb_6()
        .child(column.max_w(px(width)))
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
        |view, window, cx| view.choose_import(ImportIntent::UsefulInformation, None, window, cx),
    ))
    .item(item(
        this,
        "Import full note…",
        None,
        busy,
        false,
        |view, window, cx| view.choose_import(ImportIntent::FullNote, None, window, cx),
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
                .accessible_dropdown_menu(move |menu, _, _| new_menu(menu, &this, busy))
                .into_any_element(),
            ],
            TileKind::Chat => self.chat_controls(cx),
            TileKind::Note => self.note_controls(cx),
            _ => vec![],
        }
    }
    /// What a needs-you row or menu item says: the chat's first recorded
    /// reason, or for a prepared review the notes it changes, and whether a
    /// protected note is among them. The service's reason is unchanged.
    pub(super) fn needs_reason(&self, thread: &str) -> Option<(String, bool)> {
        let RecordData::Thread(data) = &self.records.iter().find(|r| r.id == thread)?.data else {
            return None;
        };
        let attention = data.attention.first()?;
        Some(
            match self
                .candidates
                .iter()
                .find(|c| attention.record.as_deref() == Some(c.operation.as_str()))
            {
                Some(candidate) => {
                    let summary = review_summary(candidate, &self.records);
                    (
                        review_label(&summary, &candidate.request.reason),
                        summary.protected,
                    )
                }
                None => (attention.reason.clone(), false),
            },
        )
    }
    /// The Protected marker on a review row: the Lock icon alone in a
    /// narrow list, announced and shown in the tooltip as "Protected".
    fn protected_chip(id: String, compact: bool) -> gpui_kit::AnyElement {
        if compact {
            components::short_status_chip(
                id,
                Some(Tone::Info),
                Some(IconName::Lock),
                "",
                "Protected",
            )
            .into_any_element()
        } else {
            components::status_chip(id, Some(Tone::Info), Some(IconName::Lock), "Protected")
                .into_any_element()
        }
    }
    /// A review row for a prepared proposal: its target notes, and Protected
    /// when one of them is.
    fn candidate_row(
        &self,
        id: String,
        candidate: &Prepared,
        compact: bool,
    ) -> components::ListRow {
        let summary = review_summary(candidate, &self.records);
        let row = components::ListRow::new(
            id.clone(),
            review_label(&summary, &candidate.request.reason),
        );
        if summary.protected {
            row.chip(
                "Protected",
                Self::protected_chip(format!("{id}-protected"), compact),
            )
        } else {
            row
        }
    }
    /// The note a kept edit belongs to.
    fn recover_note_name(&self, buffer: &EditSession) -> String {
        self.records
            .iter()
            .find(|record| record.id == buffer.note)
            .map(title)
            .unwrap_or_else(|| "Unavailable note".into())
    }
    /// A kept edit as a needs-you row: the note's title first, "Recover
    /// edit" below it, and the Unsaved marker, icon-only in a narrow list
    /// (R4) so the title keeps its room.
    fn recover_row(&self, id: String, buffer: &EditSession, compact: bool) -> components::ListRow {
        let chip = format!("{id}-unsaved");
        let marker = if compact {
            components::short_status_chip(
                chip,
                Some(Tone::Warning),
                Some(IconName::TriangleAlert),
                "",
                "Unsaved",
            )
        } else {
            components::status_chip(chip, Some(Tone::Warning), None, "Unsaved")
        };
        components::ListRow::new(id.clone(), self.recover_note_name(buffer))
            .reason("Recover edit")
            .chip("Unsaved", marker)
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
            let row_id = format!("attention-{id}");
            let (reason, protected) = self.needs_reason(&id).unwrap_or_default();
            let mut entry = components::ListRow::new(row_id.clone(), row_data.title.clone())
                .reason(reason)
                .selected(self.chat_thread() == Some(id.as_str()));
            if protected {
                entry = entry.chip(
                    "Protected",
                    Self::protected_chip(format!("{row_id}-protected"), true),
                );
            }
            list = list.child(
                entry
                    .actionable()
                    .build()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                    )
                    .context_menu(move |menu, _, _| {
                        thread_menu(menu, &this, menu_id.clone(), resolved, note.clone())
                    }),
            );
        }
        for (index, buffer) in self.buffers.iter().enumerate() {
            let buffer = buffer.clone();
            let id = buffer.note.clone();
            list = list.child(
                self.recover_row(format!("recover-{index}"), &buffer, true)
                    .build()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, Some(buffer.clone()), window, cx)
                    })),
            );
        }
        for candidate in &self.candidates {
            let open = candidate.clone();
            list = list.child(
                self.candidate_row(
                    format!("candidate-{}", candidate.operation.as_str()),
                    candidate,
                    true,
                )
                .build()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_review(open.clone(), window, cx)
                })),
            );
        }
        list = list.child(section("Recent chats"));
        match bar.recent_empty() {
            None => {}
            Some(lists::RecentEmpty::NoChats) => {
                list = list.child(components::empty_state(
                    "threads-empty",
                    IconName::MessageSquare,
                    "No chats yet",
                    "Start one with ⌘T.",
                    None,
                ));
            }
            // The chats above need you; one quiet line, no second invitation.
            Some(lists::RecentEmpty::OthersNeedYou) => {
                list = list.child(
                    metadata("No other open chats.")
                        .id("threads-empty")
                        .aria_label("No other open chats.")
                        .px_2()
                        .test_support(),
                );
            }
            Some(lists::RecentEmpty::AllArchived) => {
                list = list.child(components::empty_state(
                    "threads-empty",
                    IconName::Archive,
                    "No open chats",
                    "Archived chats are under Archived.",
                    None,
                ));
            }
        }
        for row_data in &bar.open {
            let id = row_data.id.clone();
            let menu_id = id.clone();
            let (resolved, note) = (row_data.resolved, row_data.note.clone());
            let this = this.clone();
            let meta = if row_data.has_action {
                "Task"
            } else if row_data.note.is_some() {
                "Note"
            } else {
                ""
            };
            list = list.child(
                components::ListRow::new(format!("thread-{id}"), row_data.title.clone())
                    .meta(meta)
                    .selected(self.chat_thread() == Some(id.as_str()))
                    .actionable()
                    .build()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
                    )
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
                    "Showing {} of {} · keep typing",
                    lists::LIST_LIMIT,
                    grouped.matched
                )
            } else {
                format!(
                    "{} chat{}",
                    grouped.matched,
                    if grouped.matched == 1 { "" } else { "s" }
                )
            }));
        let mut list = scroll("all-threads-list");
        if grouped.groups.is_empty() {
            list = list.child(components::empty_state(
                "all-threads-empty",
                IconName::Search,
                "No matching chats",
                "Try another word or filter.",
                None,
            ));
        }
        for (name, rows) in grouped.groups {
            list = list.child(section(name));
            for row_data in rows {
                let id = row_data.id.clone();
                let menu_id = id.clone();
                let (resolved, note) = (row_data.resolved, row_data.note.clone());
                let this = this.clone();
                let meta = if row_data.needs_you() {
                    "Needs you"
                } else if resolved {
                    "Archived"
                } else if row_data.has_action {
                    "Task"
                } else if row_data.note.is_some() {
                    "Note"
                } else {
                    ""
                };
                list = list.child(
                    components::ListRow::new(format!("all-thread-{id}"), row_data.title.clone())
                        .meta(meta)
                        .selected(self.chat_thread() == Some(id.as_str()))
                        .actionable()
                        .build()
                        .on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.open_thread(&id, window, cx)
                            }),
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
            let kind = if data.imported { "Imported" } else { "" };
            let meta = match (data.inbound, kind.is_empty()) {
                (0, _) => kind.to_owned(),
                (n, true) => format!("← {n}"),
                (n, false) => format!("{kind} · ← {n}"),
            };
            let row_id = format!("{id_prefix}{id}");
            let mut entry = components::ListRow::new(row_id.clone(), data.title.clone())
                .meta(meta)
                .selected(current.as_deref() == Some(id.as_str()));
            // The Lock alone, as in the Chats list, so a narrow list never
            // cuts "Protected" short; it is announced and the tooltip.
            if data.protected {
                entry = entry.chip(
                    "Protected",
                    Self::protected_chip(format!("{row_id}-protected"), true),
                );
            }
            entry
                .build()
                .on_click(cx.listener(move |this, _, window, cx| {
                    // A Library article opens for reading (spec §6.5).
                    this.writing.read_next();
                    this.open_note(&id, None, window, cx)
                }))
                .role(gpui_kit::Role::Button)
                .tab_index(0)
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
                        .border_color(rgb(pal().hairline)),
                )
                .child(div().flex_1().min_h_0().child(body))
                .into_any_element()
        };
        let count = self.note_rows.len();
        let mut header = div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(Input::new(&self.search).w_full())
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_2()
                    .child(metadata(format!(
                        "{count} note{}",
                        if count == 1 { "" } else { "s" }
                    )))
                    .when(self.results.preparing, |line| {
                        line.child(components::loading("search-preparing", "Preparing search…"))
                    }),
            );
        let mut list = scroll("notes-list");
        if !query.trim().is_empty() {
            let (titles, total) = lists::filter_notes(&self.note_rows, &query, 100);
            list = list.child(section(format!("Titles · {total}")));
            for data in titles {
                list = list.child(note_row("note-", data, cx));
            }
            if total > 100 {
                list = list.child(metadata(format!("Showing 100 of {total} · keep typing")));
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
                list.child(components::empty_state(
                    "notes-empty",
                    IconName::FileText,
                    "No notes yet",
                    "Create one with ⌘N, or add information with +.",
                    None,
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
        // One row that scrolls sideways in a narrow panel; it stays above
        // the list's scroller.
        let mut letters = div()
            .id("notes-letters")
            .flex()
            .flex_shrink_0()
            .gap_1()
            .min_w_0()
            .overflow_x_scroll();
        for &(ch, count) in &view.letters {
            letters = letters.child(
                pill(format!("notes-letter-{}", ch as u32), ch.to_string())
                    .flex_shrink_0()
                    .selected(ch == letter)
                    .tooltip(format!("{count} notes"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.notes_letter = ch;
                        cx.notify();
                    })),
            );
        }
        header = header.child(letters.test_support());
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
}
