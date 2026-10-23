//! Navigation and workspace panels: Chats, Notes, Home, Tasks, Changes and
//! Settings. Every control calls the existing checked services.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, pill};
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::component::radio::Radio;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tooltip::Tooltip;
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
/// Settings forms are narrower.
const FORM_WIDTH: f32 = 640.;

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

/// A page section: its label, then rows.
fn group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).flex().flex_col().gap(px(2.)).min_w_0()
}

fn section_with_count(label: &str, count: usize) -> gpui_kit::Div {
    components::section_header(label.to_owned(), Some(count), None)
}

/// A row's tooltip: its text and the full recorded time.
fn timed(text: &str, stamp: &str) -> String {
    if stamp.is_empty() {
        text.to_owned()
    } else {
        format!("{text} · {stamp}")
    }
}

/// A labelled form field: the label above its control.
fn field(label: &str, control: impl IntoElement) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(metadata(label.to_owned()))
        .child(control)
}

use components::unavailable;

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
    /// A kept edit's full text: "Recover edit · {note}".
    fn recover_label(&self, buffer: &EditSession) -> String {
        format!("Recover edit · {}", self.recover_note_name(buffer))
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
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
                )
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
    /// First run: no current notes and no open chats. Derived, not saved.
    pub(super) fn first_run(&self) -> bool {
        self.note_rows.is_empty() && self.thread_rows.iter().all(|row| row.resolved)
    }
    pub(super) fn overview_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = column();
        if self.first_run() {
            view = view.child(
                components::card("home-intro", None)
                    .mt(components::tokens::SPACE_4)
                    .aria_label("Your working knowledge")
                    .child(
                        div()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child("Your working knowledge"),
                    )
                    .child(
                        "Give BRN information and a goal. Your notes, work and decisions stay connected.",
                    )
                    .child(metadata(
                        "⌘T starts a chat · ⌘N creates a note · ⌘K finds anything",
                    ))
                    .test_support(),
            );
        }
        if self.delegation_host() == Some(TileKind::Overview) {
            view = view.child(
                div()
                    .mt(components::tokens::SPACE_4)
                    .child(self.working_knowledge(cx)),
            );
        }
        let needs: Vec<_> = self
            .thread_rows
            .iter()
            .filter(|row| row.needs_you())
            .collect();
        let mut queue = group("home-needs-you").child(section("Needs you"));
        if needs.is_empty() {
            queue = queue.child(
                metadata("Nothing needs you right now.")
                    .id("home-attention-empty")
                    .px_2()
                    .test_support(),
            );
        }
        for data in needs {
            let id = data.id.clone();
            let row_id = format!("home-attention-{id}");
            let (reason, protected) = self.needs_reason(&id).unwrap_or_default();
            let mut entry =
                components::ListRow::new(row_id.clone(), data.title.clone()).reason(reason);
            if protected {
                entry = entry.chip(
                    "Protected",
                    Self::protected_chip(format!("{row_id}-protected"), false),
                );
            }
            queue = queue.child(entry.build().on_click(
                cx.listener(move |this, _, window, cx| this.open_thread(&id, window, cx)),
            ));
        }
        view = view.child(queue.test_support());
        if !self.buffers.is_empty() {
            let mut recovery = group("home-recovery").child(section("Recover your edits"));
            for (index, buffer) in self.buffers.iter().enumerate() {
                let buffer = buffer.clone();
                let id = buffer.note.clone();
                let tooltip = format!(
                    "{} · Kept apart from the saved note until you save or discard it",
                    self.recover_label(&buffer)
                );
                recovery = recovery.child(
                    self.recover_row(format!("home-recover-{index}"), &buffer, false)
                        .tooltip(tooltip)
                        .build()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, Some(buffer.clone()), window, cx)
                        })),
                );
            }
            view = view.child(recovery);
        }
        if !self.candidates.is_empty() {
            let mut review = group("home-review").child(section("Review changes"));
            for candidate in &self.candidates {
                let open = candidate.clone();
                review = review.child(
                    self.candidate_row(
                        format!("home-candidate-{}", candidate.operation.as_str()),
                        candidate,
                        false,
                    )
                    .build()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_review(open.clone(), window, cx)
                    })),
                );
            }
            view = view.child(review);
        }
        let mut notes: Vec<&NoteRow> = self.note_rows.iter().collect();
        notes.sort_by_key(|row| std::cmp::Reverse(row.order));
        let mut note_list =
            group("home-notes").child(section_with_count("Recent notes", notes.len()));
        // Home's sections stay quiet when empty: one line each, since the
        // first-run intro already says how to begin.
        if notes.is_empty() {
            note_list = note_list.child(
                metadata("No notes yet. Create one with ⌘N.")
                    .id("home-notes-empty")
                    .px_2()
                    .test_support(),
            );
        }
        for data in notes.into_iter().take(6) {
            let id = data.id.clone();
            let stamp = self.activity.stamp(&id).to_owned();
            note_list = note_list.child(
                components::ListRow::new(format!("home-note-{id}"), data.title.clone())
                    .meta(lists::human_time(&stamp))
                    .tooltip(timed(&data.title, &stamp))
                    .build()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, None, window, cx)
                    })),
            );
        }
        let changes: Vec<&Receipt> = self
            .history
            .iter()
            .rev()
            .filter(|receipt| !lists::is_background(receipt))
            .take(6)
            .collect();
        let mut recent = group("home-recent-changes").child(components::section_header(
            "Recent changes",
            None,
            Some(
                pill("home-open-history", "All changes")
                    .tooltip("Open Changes, where each change has Undo")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::History, window, cx)
                    }))
                    .into_any_element(),
            ),
        ));
        if changes.is_empty() {
            recent = recent.child(
                metadata("No changes yet.")
                    .id("home-changes-empty")
                    .px_2()
                    .test_support(),
            );
        }
        for receipt in changes {
            let op = receipt.operation.as_str().to_owned();
            let reason = lists::receipt_reason(&receipt.reason);
            let mut entry = components::ListRow::new(format!("home-change-{op}"), reason.clone())
                .meta(lists::human_time(&receipt.committed_at))
                .tooltip(timed(&reason, &receipt.committed_at));
            if !receipt.needs_refresh.is_empty() {
                entry = entry.chip(
                    "Needs refresh",
                    components::status_chip(
                        format!("home-change-{op}-refresh"),
                        Some(Tone::Warning),
                        None,
                        "Needs refresh",
                    ),
                );
            }
            recent =
                recent.child(entry.build().on_click(cx.listener(|this, _, window, cx| {
                    this.show_tile(TileKind::History, window, cx)
                })));
        }
        page(
            "home-content",
            PAGE_WIDTH,
            view.child(note_list.test_support())
                .child(recent.test_support())
                .child(self.knowledge_panel(cx)),
        )
        .test_support()
        .into_any_element()
    }
    /// A task row: its description and facts, the one main action for its
    /// state (outline) and a row ⋯ with the other state changes. `show_state`
    /// names the state where no section does.
    pub(super) fn action_row(
        &self,
        record: &Record,
        show_state: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let RecordData::Action(action) = &record.data else {
            return div().into_any_element();
        };
        let p = pal();
        let id = record.id.clone();
        // The task as it would be after a step; Confirm done records how.
        let next = move |step: lists::TaskStep| {
            let mut next = action.clone();
            next.state = step.state();
            if step == lists::TaskStep::ConfirmDone {
                next.evidence = Some("Owner confirmed completion in BRN.".into());
            }
            RecordData::Action(next)
        };
        let facts: Vec<String> = [
            show_state.then(|| format!("{:?}", action.state)),
            action
                .due
                .as_ref()
                .map(|due| format!("Due {}", lists::human_time(due))),
            action.evidence.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let (main, steps) = lists::task_steps(&action.state);
        let mut controls = div().flex().flex_shrink_0().items_center().gap_1();
        if let Some(step) = main {
            let (record, data) = (record.clone(), next(step));
            let control = match step {
                lists::TaskStep::Accept => format!("open-action-{id}"),
                _ => format!("done-action-{id}"),
            };
            // The row's one main action is quiet: rows never hold a filled
            // primary.
            controls = controls.child(
                button(control)
                    .label(step.label())
                    .outline()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change(step.reason(), record.clone(), data.clone(), cx)
                    })),
            );
        }
        if !steps.is_empty() {
            let this = cx.entity().downgrade();
            let items: Vec<_> = steps
                .into_iter()
                .map(|step| (step, record.clone(), next(step)))
                .collect();
            controls = controls.child(
                components::overflow_menu(format!("task-menu-{id}"), "Task actions")
                    .accessible_dropdown_menu(move |mut menu, _, _| {
                        for (step, record, data) in items.clone() {
                            if step == lists::TaskStep::Cancel {
                                menu = menu.separator();
                            }
                            menu = menu.item(item(
                                &this,
                                step.label(),
                                None,
                                false,
                                false,
                                move |view, _, cx| {
                                    view.change(step.reason(), record.clone(), data.clone(), cx)
                                },
                            ));
                        }
                        menu
                    }),
            );
        }
        div()
            .id(format!("action-tile-{id}"))
            .flex()
            .flex_shrink_0()
            .items_start()
            .gap_2()
            .px_2()
            .py_2()
            .min_w_0()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id(format!("action-description-{id}"))
                            .aria_label(action.description.clone())
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(p.ink))
                            .child(action.description.clone())
                            .test_support(),
                    )
                    .when(!facts.is_empty(), |text| {
                        text.child(metadata(facts.join(" · ")))
                    }),
            )
            .child(controls)
            .test_support()
            .into_any_element()
    }
    /// A section's session-only disclosure: "Show N" or "Hide".
    fn disclosure(
        &self,
        key: &'static str,
        count: usize,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let open = self.open_sections.contains(key);
        pill(
            format!("{key}-toggle"),
            if open {
                "Hide".to_owned()
            } else {
                format!("Show {count}")
            },
        )
        .accessibility_label(if open {
            "Hide".to_owned()
        } else {
            format!("Show {count}")
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            if !this.open_sections.remove(key) {
                this.open_sections.insert(key);
            }
            cx.notify();
        }))
        .into_any_element()
    }
    pub(super) fn actions_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = column();
        if !self
            .records
            .iter()
            .any(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
        {
            view = view.child(components::empty_state(
                "actions-empty",
                IconName::ListTodo,
                "No tasks yet",
                "Tasks keep their own progress, including after a chat is archived.",
                None,
            ));
        }
        for (status, key, label, collapsible) in [
            (ActionState::Suggested, "suggested", "Suggested", None),
            (ActionState::Open, "open", "Open", None),
            (ActionState::Waiting, "waiting", "Waiting", None),
            (ActionState::Done, "done", "Done", Some("tasks-done")),
            (
                ActionState::Cancelled,
                "cancelled",
                "Cancelled",
                Some("tasks-cancelled"),
            ),
        ] {
            let records: Vec<_> = self
                .records
                .iter()
                .filter(|r| {
                    !r.archived && matches!(&r.data, RecordData::Action(a) if a.state == status)
                })
                .collect();
            if records.is_empty() {
                continue;
            }
            // Done and Cancelled start collapsed, with their counts.
            let shown = collapsible.is_none_or(|key| self.open_sections.contains(key));
            let mut group =
                group(format!("actions-group-{key}")).child(components::section_header(
                    label,
                    Some(records.len()),
                    collapsible.map(|toggle| self.disclosure(toggle, records.len(), cx)),
                ));
            if shown {
                for record in records {
                    group = group.child(self.action_row(record, false, cx));
                }
            }
            view = view.child(group.test_support());
        }
        page("actions-content", PAGE_WIDTH, view)
            .test_support()
            .into_any_element()
    }
    /// A record that may need refresh, in owner terms on one line: its
    /// title, or for a chat Message its kind, never its stored payload (R3;
    /// a rewrite request's text is its saved JSON snapshot).
    fn refresh_name(&self, record: &Record) -> String {
        let name = match &record.data {
            // The request's note title was resolved when history refreshed.
            RecordData::Message(message) if message.role == WRITING_REQUEST_ROLE => {
                match self.refresh_names.get(&record.id) {
                    Some(note) => format!("Rewrite request · {note}"),
                    None => "Rewrite request".into(),
                }
            }
            RecordData::Message(_) => "Chat message".into(),
            _ => title(record),
        };
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    /// One recorded change: its reason, what it touched and when, an Undo
    /// ghost, and for derived work a "May need refresh" sub-row.
    fn receipt_row(&self, receipt: &Receipt, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let id = receipt.operation.as_str().to_owned();
        let reason = lists::receipt_reason(&receipt.reason);
        let affected = lists::affected_titles(receipt);
        let stamp = receipt.committed_at.clone();
        let undo_label = format!("Undo: {reason}");
        let undo_id = id.clone();
        let mut entry = div()
            .id(format!("receipt-{id}"))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_2()
            .min_w_0()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .id(format!("receipt-reason-{id}"))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .text_color(rgb(p.ink))
                                    .aria_label(reason.clone())
                                    .child(reason.clone())
                                    .test_support(),
                            )
                            .child(
                                div()
                                    .id(format!("receipt-facts-{id}"))
                                    .flex()
                                    .min_w_0()
                                    .gap_1()
                                    .text_size(SMALL_TEXT)
                                    .text_color(rgb(p.muted))
                                    .when(!affected.is_empty(), |facts| {
                                        facts.child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .child(format!("{affected} ·")),
                                        )
                                    })
                                    .child(div().flex_shrink_0().child(lists::human_time(&stamp)))
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(stamp.clone()).build(window, cx)
                                    })
                                    .test_support(),
                            ),
                    )
                    .child(
                        button(format!("undo-{id}"))
                            .label("Undo")
                            .icon(IconName::Undo2)
                            .ghost()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .flex_shrink_0()
                            .accessibility_label(undo_label.clone())
                            .tooltip(undo_label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.undo_receipt(&undo_id, window, cx)
                            })),
                    ),
            );
        if !receipt.needs_refresh.is_empty() {
            let mut refresh = div()
                .id(format!("receipt-refresh-{id}"))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded(components::tokens::RADIUS_CONTROL)
                .bg(rgb(p.warning_bg))
                .text_color(rgb(p.ink))
                .aria_label("May need refresh")
                .child(components::tone_mark(
                    format!("receipt-refresh-{id}-tone"),
                    Tone::Warning,
                ))
                .child(
                    div()
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .mr_1()
                        .child("May need refresh"),
                );
            if let Ok(records) = self.workspace.refresh_records(&id) {
                for record in records {
                    let note =
                        matches!(record.data, RecordData::Note(_)).then(|| record.id.clone());
                    let name = self.refresh_name(&record);
                    // One line that ellipsizes inside the panel (R3); the
                    // full name is the tooltip and the accessible name.
                    refresh = refresh.child(
                        pill(format!("refresh-record-{id}-{}", record.id), name.clone())
                            .max_w(gpui_kit::relative(1.))
                            .min_w_0()
                            .accessibility_label(format!("May need refresh: {name}"))
                            .tooltip(if note.is_some() {
                                format!("{name} · Open this note to review its inputs")
                            } else {
                                format!("{name} · Open Tasks to review it")
                            })
                            .on_click(cx.listener(move |this, _, window, cx| match &note {
                                Some(id) => this.open_note(id, None, window, cx),
                                None => this.show_tile(TileKind::Actions, window, cx),
                            })),
                    );
                }
            }
            let receipt_id = id.clone();
            refresh = refresh.child(
                button(format!("refresh-followup-{id}"))
                    .label("Open refresh follow-up")
                    .outline()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .tooltip(
                        "Open a chat that reviews the current inputs before anything is updated",
                    )
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
            entry = entry.child(refresh.test_support());
        }
        entry.test_support().into_any_element()
    }
    pub(super) fn history_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let (background, changes): (Vec<&Receipt>, Vec<&Receipt>) = self
            .history
            .iter()
            .rev()
            .partition(|r| lists::is_background(r));
        let mut view = column();
        if changes.is_empty() {
            view = view.child(components::empty_state(
                "history-empty",
                IconName::RotateCcwClock,
                "No changes yet",
                "Recorded changes and the same checked Undo used throughout your workspace.",
                None,
            ));
        } else {
            let mut list = group("history-changes").pt_2();
            for receipt in changes {
                list = list.child(self.receipt_row(receipt, cx));
            }
            view = view.child(list);
        }
        // Chat-control bookkeeping stays visible and undoable, grouped.
        if !background.is_empty() {
            let mut list = group("history-background").child(components::section_header(
                "Background",
                Some(background.len()),
                Some(self.disclosure("history-background", background.len(), cx)),
            ));
            if self.open_sections.contains("history-background") {
                for receipt in background {
                    list = list.child(self.receipt_row(receipt, cx));
                }
            }
            view = view.child(list.test_support());
        }
        page("history-content", PAGE_WIDTH, view)
            .test_support()
            .into_any_element()
    }
    /// The delegation choice. In Settings it is two radios and, once chosen,
    /// the maintenance switch; on Home or in a new chat it is the pending
    /// first-use card with two quiet buttons. Both apply immediately.
    pub(super) fn working_knowledge(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let status = delegation_status(&self.settings);
        let chosen = self.settings.delegation_chosen;
        let review_first = self.settings.review_first;
        let options = [
            (
                DelegationChoice::MaintainOrdinaryNotes,
                "delegation-maintain",
                "Maintain ordinary notes",
                "BRN updates ordinary notes and source references.",
                chosen && !review_first,
            ),
            (
                DelegationChoice::ReviewFirst,
                "delegation-review-first",
                "Review changes first",
                "BRN proposes; you approve.",
                chosen && review_first,
            ),
        ];
        let status_line = metadata(status)
            .id("delegation-status")
            .aria_label(status)
            .test_support();
        let protected_line =
            metadata("Protected notes change only with your instruction or review.");
        if self.delegation_host() != Some(TileKind::Settings) {
            // GPUI may wrap before "?", so the last word is its own unit: a
            // narrow chat wraps "notes?" whole onto the next line.
            let (lead, last) = DELEGATION_QUESTION
                .rsplit_once(' ')
                .unwrap_or(("", DELEGATION_QUESTION));
            let mut card = components::card("delegation-pending", None)
                .child(
                    div()
                        .id("delegation-question")
                        .flex()
                        .flex_wrap()
                        .gap_x(px(4.))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .aria_label(DELEGATION_QUESTION)
                        .child(div().min_w_0().child(lead))
                        .child(
                            div()
                                .id("delegation-question-end")
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .child(last)
                                .test_support(),
                        )
                        .test_support(),
                )
                .child(status_line);
            for (choice, id, label, explanation, _) in options {
                card = card.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x_2()
                        .gap_y_1()
                        .child(
                            button(id)
                                .label(label)
                                .outline()
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .accessibility_label(format!("{label}: {explanation}"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_delegation(choice, cx)
                                })),
                        )
                        // Beside the button when there is room, otherwise on
                        // its own line, wrapping inside the card.
                        .child(
                            metadata(explanation)
                                .id(format!("{id}-why"))
                                .flex_grow(1.)
                                .flex_basis(px(160.))
                                .min_w_0()
                                .test_support(),
                        ),
                );
            }
            return card.child(protected_line).test_support().into_any_element();
        }
        let mut radios = div()
            .id("delegation-choice")
            .flex()
            .flex_col()
            .gap_3()
            .aria_label(DELEGATION_QUESTION);
        for (choice, id, label, explanation, checked) in options {
            radios =
                radios.child(
                    Radio::new(id)
                        .label(label)
                        .checked(checked)
                        .accessibility_label(format!("{label}: {explanation}"))
                        .child(metadata(explanation))
                        .on_click(cx.listener(move |this, _: &bool, _, cx| {
                            this.choose_delegation(choice, cx)
                        })),
                );
        }
        let mut form = column()
            .id(if chosen {
                "delegation-settings"
            } else {
                "delegation-pending"
            })
            .gap_3()
            .child(section(DELEGATION_QUESTION))
            .child(radios.test_support())
            .child(protected_line)
            .child(status_line);
        if chosen {
            let running = self.settings.maintenance;
            form = form.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pt_2()
                    .child(subheading("Maintenance"))
                    .child(
                        Switch::new("maintenance-setting")
                            .checked(running)
                            .label(if running { "Running" } else { "Paused" })
                            .accessibility_label(if running {
                                "Maintenance: Running"
                            } else {
                                "Maintenance: Paused"
                            })
                            .tooltip(if running {
                                "Pause maintenance"
                            } else {
                                "Resume maintenance"
                            })
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                match this.workspace.pause(this.settings.maintenance) {
                                    Ok(outcome) => this.report_outcome(&outcome),
                                    Err(error) => this.fail(error.to_string()),
                                }
                                this.refresh();
                                cx.notify();
                            })),
                    ),
            );
        }
        form.test_support().into_any_element()
    }
    /// A Settings segmented choice: each segment keeps its own ID and
    /// handler.
    fn choice<T: Copy + 'static>(
        id: &str,
        options: &[(T, &str, &str)],
        selected: impl Fn(T) -> bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Self, T) + Copy + 'static,
    ) -> gpui_kit::component::button::ButtonGroup {
        components::segmented(
            SharedString::from(id.to_owned()),
            options
                .iter()
                .map(|&(value, key, label)| {
                    button(SharedString::from(key.to_owned()))
                        .label(label.to_owned())
                        .selected(selected(value))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            set(this, value);
                            cx.notify();
                        }))
                })
                .collect(),
        )
    }
    fn models_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let provider = &self.provider_form.provider;
        let efforts = [
            ("low", "effort-low", "Low"),
            ("medium", "effort-medium", "Medium"),
            ("high", "effort-high", "High"),
        ];
        let model = group("settings-provider")
            .gap_3()
            .child(section("Provider and model"))
            .child(field(
                "Provider",
                Self::choice(
                    "settings-provider-choice",
                    &[
                        ("chatgpt", "provider-chatgpt", "Codex / ChatGPT"),
                        ("copilot", "provider-copilot", "GitHub Copilot"),
                    ],
                    // The default "Codex" provider is the Codex / ChatGPT segment.
                    |value| {
                        provider.eq_ignore_ascii_case(value)
                            || value == "chatgpt" && provider.eq_ignore_ascii_case("codex")
                    },
                    cx,
                    |this, value| this.provider_form.provider = value.into(),
                ),
            ))
            .child(field(
                "Model",
                Input::new(&self.model).w_full().aria_label("Model"),
            ))
            .child(field(
                "Reasoning effort",
                Self::choice(
                    "settings-effort-choice",
                    &efforts,
                    |value| self.provider_form.effort == value,
                    cx,
                    |this, value| this.provider_form.effort = value.into(),
                ),
            ))
            .child(field(
                "Codex auth file",
                Input::new(&self.auth_path)
                    .w_full()
                    .aria_label("Codex auth file"),
            ))
            .child(field(
                "Copilot credentials folder",
                Input::new(&self.credentials_path)
                    .w_full()
                    .aria_label("Copilot credentials folder"),
            ))
            .child(metadata(
                "Credentials stay outside notes, exports and Changes. The Codex auth file is used only with Codex / ChatGPT.",
            ))
            .child(div().child({
                let connect = button("connect-copilot")
                    .label("Connect GitHub Copilot…")
                    .outline()
                    .disabled(self.worker.is_some())
                    .on_click(
                        cx.listener(|this, _, window, cx| this.connect_copilot(window, cx)),
                    );
                if self.worker.is_some() {
                    let (announced, tooltip) =
                        unavailable("Connect GitHub Copilot", Some("BRN is working"));
                    connect.accessibility_label(announced).tooltip(tooltip)
                } else {
                    connect.tooltip(
                        "Sign in with GitHub in your browser; uses the folder above or ~/.config/brn/copilot",
                    )
                }
            }));
        let helper_on = |value: &str| match &self.provider_form.helper_provider {
            None => value == "off",
            Some(provider) => provider.eq_ignore_ascii_case(value),
        };
        let mut helper = group("settings-helper")
            .gap_3()
            .child(section("Helper sub-agents"))
            .child(
                "Read-only helpers can research your notes for the chat model. They can't change notes or stand in for an unavailable model.",
            )
            .child(
                metadata(brn_threads_app::HELPER_TESTED_NOTE)
                    .id("settings-helper-tested")
                    .aria_label(brn_threads_app::HELPER_TESTED_NOTE)
                    .test_support(),
            )
            .child(field(
                "Helper provider",
                Self::choice(
                    "settings-helper-choice",
                    &[
                        ("off", "helper-provider-off", "Off"),
                        ("Codex", "helper-provider-Codex", "Codex / ChatGPT"),
                        ("Copilot", "helper-provider-Copilot", "GitHub Copilot"),
                    ],
                    helper_on,
                    cx,
                    |this, value| {
                        if value == "off" {
                            this.provider_form.helper_provider = None;
                            this.provider_form.helper_effort = "high".into();
                        } else {
                            this.provider_form.helper_provider = Some(value.into());
                        }
                    },
                ),
            ));
        if self.provider_form.helper_provider.is_some() {
            helper = helper
                .child(field(
                    "Helper model",
                    Input::new(&self.helper_model)
                        .w_full()
                        .aria_label("Helper model"),
                ))
                .child(field(
                    "Helper reasoning effort",
                    Self::choice(
                        "settings-helper-effort-choice",
                        &[
                            ("low", "helper-effort-low", "Low"),
                            ("medium", "helper-effort-medium", "Medium"),
                            ("high", "helper-effort-high", "High"),
                        ],
                        |value| self.provider_form.helper_effort == value,
                        cx,
                        |this, value| this.provider_form.helper_effort = value.into(),
                    ),
                ))
                .child(metadata(
                    "Uses the credential location above for its provider.",
                ));
        }
        // One Save covers exactly what `configure` saves, helper included.
        let why = self.worker.is_some().then_some("BRN is working");
        let (announced, tooltip) = unavailable("Save settings", why);
        let save = div()
            .flex()
            .flex_col()
            .gap_1()
            .pt_4()
            .mt_4()
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .child(
                div().child(
                    button("save-settings")
                        .label("Save settings")
                        .primary()
                        .disabled(why.is_some())
                        .accessibility_label(announced)
                        .tooltip(tooltip)
                        .on_click(cx.listener(|this, _, _, cx| this.configure(cx))),
                ),
            )
            .child(metadata(
                "Saves provider, model, credentials and helper settings.",
            ))
            .children(why.map(|_| {
                metadata("Available when the current work finishes.")
                    .id("save-settings-reason")
                    .test_support()
            }));
        column()
            .gap_6()
            .child(model)
            .child(helper)
            .child(save)
            .into_any_element()
    }
    fn data_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let folder = self
            .settings
            .backup_directory
            .clone()
            .unwrap_or_else(|| "Backup folder not chosen".into());
        let backup = group("settings-backup")
            .gap_2()
            .child(section("Full backup"))
            .child("Includes notes, discussions, history, figures and recovery state.")
            .child(
                div()
                    .id("backup-directory")
                    .aria_label(folder.clone())
                    .font_family(MONO)
                    .text_size(SMALL_TEXT)
                    .child(folder)
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
                        button("backup-now")
                            .label("Create full backup")
                            .primary()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_output(true, window, cx)
                            })),
                    )
                    .child(
                        button("backup-change-folder")
                            .label("Change folder…")
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_backup_folder(false, window, cx)
                            })),
                    ),
            )
            .child(metadata(self.backup_status.clone()));
        let export =
            group("settings-export")
                .gap_2()
                .child(section("Markdown export"))
                .child("Export a current snapshot of your notes and retained figures.")
                .child(
                    div().child(
                        button("export-now")
                            .label("Export Markdown…")
                            .outline()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_output(false, window, cx)
                            })),
                    ),
                )
                .child(metadata(self.export_status.clone()));
        column()
            .id("settings-copies")
            .gap_6()
            .child(backup)
            .child(export)
            .test_support()
            .into_any_element()
    }
    fn general_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let mut themes = group("settings-themes");
        for theme in ThemeId::ALL {
            let colors = theme.palette();
            let selected = self.theme == theme;
            let swatches = div().flex().flex_shrink_0().gap_1().children(
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
                        .rounded(px(3.))
                        .border_1()
                        .border_color(rgb(p.hairline))
                        .bg(rgb(color))
                }),
            );
            themes =
                themes.child(
                    button(format!("settings-theme-{}", theme.key()))
                        .ghost()
                        .w_full()
                        .h(components::tokens::FORM_CONTROL)
                        .selected(selected)
                        .accessibility_label(theme.label())
                        .tooltip(theme.description())
                        .child(div().w(px(110.)).flex_shrink_0().child(theme.label()))
                        .child(swatches)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(if selected { p.ink } else { p.muted }))
                                .child(theme.description()),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.choose_theme(theme, window, cx)
                        })),
                );
        }
        column()
            .id("settings-general")
            .gap_2()
            .child(components::section_header(
                "Theme",
                None,
                Some(metadata("Also in ⌘K.").into_any_element()),
            ))
            .child(themes)
            .child(section("Keys and mouse"))
            .child(
                div().child(
                    button("settings-keys")
                        .label("Show keys and mouse help")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
                ),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn settings_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let sections = SettingsSection::ALL
            .into_iter()
            .map(|item| {
                button(format!("settings-section-{}", item.key()))
                    .label(item.label())
                    .selected(self.settings_section == item)
                    .on_click(
                        cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                            this.settings_section = item;
                            // Kit buttons don't take focus on a click, so the
                            // ring would stay on the segment focus landed on.
                            // A pointer click hands focus to the Settings body;
                            // a key press keeps it on the pressed segment.
                            if !event.is_keyboard() {
                                this.panel_body(TileKind::Settings)
                                    .clone()
                                    .focus(window, cx);
                            }
                            cx.notify();
                        }),
                    )
            })
            .collect();
        let content = match self.settings_section {
            SettingsSection::General => self.general_settings(cx),
            SettingsSection::Models => self.models_settings(cx),
            SettingsSection::Delegation => self.working_knowledge(cx),
            SettingsSection::Documents => self.documents_settings(cx),
            SettingsSection::Data => self.data_settings(cx),
            SettingsSection::Skills => self.skills_settings(cx),
            SettingsSection::Mcp => self.mcp_settings(cx),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(
                components::toolbar("settings-sections-bar")
                    .px_4()
                    .child(components::segmented("settings-sections", sections)),
            )
            .child(
                div()
                    .id("settings-content")
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .pb_6()
                    .font_family(UI_FONT)
                    .text_size(UI_TEXT)
                    .overflow_y_scroll()
                    .child(div().w_full().max_w(px(FORM_WIDTH)).child(content))
                    .test_support(),
            )
            .into_any_element()
    }
}

/// The delegation question, asked once in Settings and on the first-use card.
const DELEGATION_QUESTION: &str = "How should BRN update your notes?";
