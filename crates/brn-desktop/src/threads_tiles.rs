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
        .text_sm()
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
                    .font_family(MONO)
                    .text_xs()
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
                view.layout.open(TileKind::Chat);
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
                pill(format!("{}-new", kind.key()), "＋")
                    .tooltip("New chat, note or information")
                    .dropdown_menu(move |menu, _, _| new_menu(menu, &this, busy))
                    .into_any_element(),
            ],
            TileKind::Chat => self.chat_controls(cx),
            TileKind::Note => self.note_controls(cx),
            _ => vec![],
        }
    }
    pub(super) fn threads_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let bar = lists::sidebar(&self.thread_rows, self.chat_thread());
        let mut list = scroll("workspace-nav");
        if !bar.needs.is_empty() || !self.buffers.is_empty() || !self.candidates.is_empty() {
            list = list.child(section("NEEDS YOU"));
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
        list = list.child(section("THREADS"));
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
        list = list.child(
            row(
                "all-threads",
                format!("all chats · {} →", bar.total),
                "",
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.thread_filter = ThreadFilter::All;
                this.show_tile(TileKind::AllThreads, cx)
            })),
        );
        if bar.resolved > 0 {
            list = list.child(
                row(
                    "resolved-count",
                    format!("archived · {}", bar.resolved),
                    "",
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.thread_filter = ThreadFilter::Resolved;
                    this.show_tile(TileKind::AllThreads, cx)
                })),
            );
        }
        let open_actions = self
            .records
            .iter()
            .filter(|record| {
                !record.archived
                    && matches!(&record.data, RecordData::Action(action) if !matches!(action.state, ActionState::Done | ActionState::Cancelled))
            })
            .count();
        let footer = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .p_2()
            .gap_1()
            .border_t_1()
            .border_color(rgb(p.border))
            .child(
                row(
                    "open-notes",
                    "notes",
                    format!("{}", self.note_rows.len()),
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Notes, cx))),
            )
            .child(
                row("actions", "actions", format!("{open_actions}"), false)
                    .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Actions, cx))),
            )
            .child(
                row("history", "what changed", "", false)
                    .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::History, cx))),
            )
            .child(
                row("home", "overview", "⌘0", false)
                    .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Overview, cx))),
            )
            .child(
                row("settings", "settings", "⌘,", false)
                    .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Settings, cx))),
            );
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().child(list.test_support()))
            .child(footer)
            .into_any_element()
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
        for filter in ThreadFilter::ALL {
            filters = filters.child(
                pill(format!("threads-filter-{}", filter.key()), filter.label())
                    .selected(self.thread_filter == filter)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.thread_filter = filter;
                        cx.notify();
                    })),
            );
        }
        let mut list = scroll("all-threads-list")
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
        if grouped.groups.is_empty() {
            list = list.child(empty("all-threads-empty", "No chats match this filter."));
        }
        for (name, rows) in grouped.groups {
            list = list.child(section(name.to_uppercase()));
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
        list.test_support().into_any_element()
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
        let mut list = scroll("notes-list")
            .child(Input::new(&self.search).w_full())
            .child(metadata(if self.results.preparing {
                format!("{} current notes · preparing search…", self.note_rows.len())
            } else {
                format!("{} current notes", self.note_rows.len())
            }));
        if !query.trim().is_empty() {
            let (titles, total) = lists::filter_notes(&self.note_rows, &query, 100);
            list = list.child(section(format!("TITLES · {total}")));
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
            return list.child(results).test_support().into_any_element();
        }
        if self.note_rows.is_empty() {
            return list
                .child(empty(
                    "notes-empty",
                    "Create a note (⌘N) or add information with ＋ to begin.",
                ))
                .test_support()
                .into_any_element();
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
        list = list.child(self.topics_section(cx)).child(section("RECENT"));
        for data in view.recent {
            list = list.child(note_row("notes-recent-", data, cx));
        }
        if view.linked.1 > 0 {
            list = list.child(section(format!(
                "LINKED TO OPEN THREADS · {}",
                view.linked.1
            )));
            for data in view.linked.0 {
                list = list.child(note_row("notes-linked-", data, cx));
            }
        }
        if view.kept.1 > 0 {
            list = list.child(section(format!("PROTECTED & IMPORTED · {}", view.kept.1)));
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
            list = list.child(section(format!("MOST LINKED · {}", view.most_linked.1)));
            for data in view.most_linked.0 {
                list = list.child(note_row("notes-most-linked-", data, cx));
            }
        }
        list = list.child(section("ALL A–Z"));
        let mut letters = div().flex().flex_wrap().gap_1();
        for (ch, count) in view.letters {
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
        list = list.child(letters);
        for data in view.letter.0 {
            list = list.child(note_row("note-", data, cx));
        }
        if view.letter.1 > lists::LETTER_LIMIT {
            list = list.child(metadata(format!(
                "{} more under {letter} · filter above",
                view.letter.1 - lists::LETTER_LIMIT
            )));
        }
        list.test_support().into_any_element()
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
            .child(heading("Needs you"));
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
            let mut recovery = panel().child(heading("Recover your edits")).child(
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
            let mut review = panel().child(heading("Review changes"));
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
            .child(heading("Recent notes"))
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
            .child(heading("What changed"));
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
        recent = recent.child(
            button("home-open-history")
                .label("Open What changed and Undo")
                .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::History, cx))),
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
                                "Accept action"
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
                            .label("Cancel action")
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
            "Actions keep their own progress, including after a chat is archived.",
        ));
        if !self
            .records
            .iter()
            .any(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
        {
            view = view.child(panel().child(empty(
                "actions-empty",
                "No actions yet. Suggested work and tasks you assign will appear here.",
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
                .child(heading(format!("{label} · {}", records.len())));
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
                .child(heading(receipt.reason.clone()))
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
                                    None => this.show_tile(TileKind::Actions, cx),
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
                                    this.notice = error.to_string();
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
            .child(heading("Working knowledge"))
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
                            Ok(outcome) => this.notice = outcome_message(&outcome),
                            Err(error) => this.notice = error.to_string(),
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
        let provider_tile = panel()
            .id("settings-provider")
            .test_support()
            .child(heading("Provider and model"))
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
                "Credentials stay outside notes, exports and History. The Codex auth file is used only with Codex / ChatGPT.",
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
            .child(heading("Portable copies"))
            .child(
                panel()
                    .bg(rgb(pal().alt))
                    .child(heading("Markdown export"))
                    .child("Export a current snapshot of your notes and retained figures.")
                    .child(button("export-now").label("Export Markdown now…").on_click(
                        cx.listener(|this, _, window, cx| this.choose_output(false, window, cx)),
                    ))
                    .child(metadata(self.export_status.clone())),
            )
            .child(
                panel()
                    .bg(rgb(pal().alt))
                    .child(heading("Full backup"))
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
            .child(heading("Theme"))
            .child(metadata("Also in the status bar and the launcher (⌘K)."));
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
        general = general.child(heading("Keys and mouse")).child(
            button("settings-keys")
                .label("Show keys and mouse help")
                .on_click(cx.listener(|this, _, window, cx| this.show_keys(window, cx))),
        );
        let mut nav = div().flex().flex_col().gap_1().w(px(130.)).flex_shrink_0();
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
            SettingsSection::Models => provider_tile.into_any_element(),
            SettingsSection::Delegation => self.working_knowledge(cx),
            SettingsSection::Data => copies.into_any_element(),
        };
        div()
            .id("settings-content")
            .size_full()
            .flex()
            .items_start()
            .gap_3()
            .p_3()
            .overflow_y_scroll()
            .child(nav)
            .child(div().flex_1().min_w_0().child(content))
            .test_support()
            .into_any_element()
    }
}
