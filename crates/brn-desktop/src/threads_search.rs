//! Owner search in the notes tile and the ⌘K launcher. Requests run on the
//! workspace search worker and are awaited off the UI thread; a newer query
//! cancels and replaces the previous one, so typing never waits for the
//! index. Note hits and saved chat Messages page independently: one answer
//! fills both lanes after typing, and each lane's Previous/Next asks for its
//! own cursor afterwards.
use super::shell::pill;
use super::*;
use brn_threads_app::search::CancelOnDrop;
use gpui_kit::StyledText;

/// Results per page, as in the accepted ordinary-wiki search.
pub(super) const PAGE: usize = 12;
/// Chat Messages the launcher lists under "In chats".
pub(super) const LAUNCH_MESSAGES: usize = 5;

/// One paged lane of the owner search: note hits or chat Messages.
#[derive(Default)]
struct Lane {
    /// The request this lane is waiting for.
    awaiting: Option<u64>,
    /// Cursors of earlier pages, for Previous.
    back: Vec<Option<String>>,
    current: Option<String>,
    task: Option<Task<()>>,
    cancel: Option<CancelOnDrop>,
}
impl Lane {
    fn pending(&self) -> bool {
        self.awaiting.is_some()
    }
    /// Drops this lane's request and cursor history.
    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Which lanes a request fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lanes {
    /// A new query: both lanes' first page.
    Both,
    Notes,
    Chats,
}

#[derive(Default)]
pub(super) struct SearchView {
    query: String,
    requests: u64,
    /// The index is being brought up to date at startup.
    pub(super) preparing: bool,
    /// The note hits' page (`hits`, `total`, `cursor`).
    pub(super) page: Option<SearchPage>,
    /// The chat Messages' page (`messages`, `message_total`,
    /// `message_cursor`): the same answer as `page` until a lane pages.
    pub(super) chats: Option<SearchPage>,
    pub(super) error: Option<String>,
    notes: Lane,
    messages: Lane,
    /// A new query's request, which fills both lanes; until it answers no
    /// earlier page shows. Paging a lane never sets it.
    fresh: Option<u64>,
}
impl SearchView {
    /// The saved chat Messages on the shown page, best first.
    pub(super) fn message_hits(&self) -> &[ChatMessageHit] {
        self.chats
            .as_ref()
            .map_or(&[], |page| page.messages.as_slice())
    }
}

/// The launcher's own first-page search, for its "In chats" group.
#[derive(Default)]
pub(super) struct LauncherSearch {
    requests: u64,
    pub(super) pending: bool,
    pub(super) messages: Vec<ChatMessageHit>,
    task: Option<Task<()>>,
    cancel: Option<CancelOnDrop>,
}

/// How a Message's role reads in the owner's terms, as in the transcript.
pub(super) fn role_label(role: &str) -> &'static str {
    match role {
        "owner" => "You",
        _ => "BRN",
    }
}

/// The one Archived label for a hit from an archived chat (owner decision
/// 3), a note or a Message alike: a neutral chip, `{row}-archived`, that
/// joins the row's announced name.
fn archived_chip(entry: components::ListRow, row: &str, archived: bool) -> components::ListRow {
    if !archived {
        return entry;
    }
    entry.chip(
        "Archived",
        components::status_chip(format!("{row}-archived"), None, None, "Archived"),
    )
}

/// A snippet with the service's highlight ranges in ink and semibold.
pub(super) fn highlighted(snippet: &str, ranges: &[std::ops::Range<usize>]) -> StyledText {
    let ink = pal().ink;
    StyledText::new(snippet.to_owned()).with_highlights(ranges.iter().map(|range| {
        (
            range.clone(),
            HighlightStyle {
                font_weight: Some(gpui_kit::FontWeight::SEMIBOLD),
                color: Some(rgb(ink).into()),
                ..Default::default()
            },
        )
    }))
}

impl ThreadsUi {
    /// Brings the index up to date in the background so the first search
    /// answers quickly, even at 10,000 notes.
    pub(super) fn prepare_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ticket = match self.workspace.search_handle() {
            Ok(handle) => handle.prepare(),
            Err(error) => {
                self.results.error = Some(error.to_string());
                return;
            }
        };
        self.results.preparing = true;
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { ticket.wait() })
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.results.preparing = false;
                if let Err(error) = answer
                    && this.results.page.is_none()
                {
                    this.results.error = Some(error.to_string());
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn search_changed(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.results.query = query;
        self.results.notes.reset();
        self.results.messages.reset();
        self.run_search(None, Lanes::Both, window, cx);
    }

    fn run_search(
        &mut self,
        cursor: Option<String>,
        lanes: Lanes,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let results = &mut self.results;
        results.requests += 1;
        let request = results.requests;
        results.error = None;
        let notes = lanes != Lanes::Chats;
        let chats = lanes != Lanes::Notes;
        if lanes == Lanes::Both {
            results.fresh = None;
        }
        if notes {
            let lane = &mut results.notes;
            (lane.task, lane.cancel, lane.awaiting) = (None, None, None);
            lane.current = cursor.clone();
            self.hits.clear();
        }
        if chats {
            let lane = &mut results.messages;
            (lane.task, lane.cancel, lane.awaiting) = (None, None, None);
            lane.current = cursor.clone();
        }
        if results.query.trim().is_empty() {
            results.page = None;
            results.chats = None;
            return;
        }
        let handle = match self.workspace.search_handle() {
            Ok(handle) => handle,
            Err(error) => {
                self.results.error = Some(error.to_string());
                return;
            }
        };
        let results = &mut self.results;
        let ticket = handle.request_page(&results.query, None, PAGE, cursor.as_deref());
        let cancel = ticket.cancel_on_drop();
        let task = cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { ticket.wait() })
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.search_answered(request, answer.map(Into::into), cx)
            });
        });
        // The lane that pages owns the request; a new query's request sits
        // in the notes lane, and both lanes wait for it.
        let owner = if chats && !notes {
            &mut results.messages
        } else {
            &mut results.notes
        };
        owner.task = Some(task);
        owner.cancel = Some(cancel);
        if notes {
            results.notes.awaiting = Some(request);
        }
        if chats {
            results.messages.awaiting = Some(request);
        }
        if lanes == Lanes::Both {
            results.fresh = Some(request);
        }
    }

    fn search_answered(
        &mut self,
        request: u64,
        answer: AppResult<SearchPage>,
        cx: &mut Context<Self>,
    ) {
        let results = &mut self.results;
        let notes = results.notes.awaiting == Some(request);
        let chats = results.messages.awaiting == Some(request);
        if !notes && !chats {
            return;
        }
        if results.fresh == Some(request) {
            results.fresh = None;
        }
        if notes {
            results.notes.awaiting = None;
            results.notes.cancel = None;
        }
        if chats {
            results.messages.awaiting = None;
            results.messages.cancel = None;
        }
        match answer {
            Ok(page) => {
                if notes {
                    self.hits = page.hits.clone();
                }
                match (notes, chats) {
                    (true, true) => {
                        self.results.chats = Some(page.clone());
                        self.results.page = Some(page);
                    }
                    (true, false) => self.results.page = Some(page),
                    _ => self.results.chats = Some(page),
                }
            }
            Err(error) => {
                if notes {
                    self.results.page = None;
                }
                if chats {
                    self.results.chats = None;
                }
                self.results.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    fn next_search_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(next) = self.results.page.as_ref().and_then(|p| p.cursor.clone()) else {
            return;
        };
        let current = self.results.notes.current.take();
        self.results.notes.back.push(current);
        self.run_search(Some(next), Lanes::Notes, window, cx);
    }

    fn previous_search_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.results.notes.back.pop() {
            self.run_search(previous, Lanes::Notes, window, cx);
        }
    }

    fn next_chats_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(next) = self
            .results
            .chats
            .as_ref()
            .and_then(|p| p.message_cursor.clone())
        else {
            return;
        };
        let current = self.results.messages.current.take();
        self.results.messages.back.push(current);
        self.run_search(Some(next), Lanes::Chats, window, cx);
    }

    fn previous_chats_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.results.messages.back.pop() {
            self.run_search(previous, Lanes::Chats, window, cx);
        }
    }

    /// Opens a Message hit's chat, scrolled to that Message.
    pub(super) fn open_chat_message(
        &mut self,
        chat: &str,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        *self.reveal_message.borrow_mut() = Some((chat.to_owned(), message.to_owned()));
        self.open_thread(chat, window, cx);
    }

    /// The launcher's query changed: list matching saved chat Messages
    /// under "In chats", as the Notes search does.
    pub(super) fn launcher_search_changed(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let search = &mut self.launcher_search;
        search.requests += 1;
        let request = search.requests;
        (search.task, search.cancel) = (None, None);
        search.messages.clear();
        search.pending = false;
        if query.trim().is_empty() {
            return;
        }
        // Without the search worker the launcher keeps matching labels only.
        let Ok(handle) = self.workspace.search_handle() else {
            return;
        };
        let ticket = handle.request_page(&query, None, LAUNCH_MESSAGES, None);
        let search = &mut self.launcher_search;
        search.cancel = Some(ticket.cancel_on_drop());
        search.pending = true;
        search.task = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { ticket.wait() })
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                let search = &mut this.launcher_search;
                if search.requests != request {
                    return;
                }
                search.pending = false;
                search.cancel = None;
                // A failed search leaves label matches; the Notes search
                // reports search errors.
                search.messages = answer
                    .map(|page| SearchPage::from(page).messages)
                    .unwrap_or_default();
                cx.notify();
            });
        }));
    }

    /// One lane's Previous and Next, disabled with their reason. Each lane
    /// waits only for its own request (a new query's request is both
    /// lanes'), never for the other lane's page.
    fn search_paging(&self, lane: Lanes, cx: &mut Context<Self>) -> gpui_kit::Div {
        let results = &self.results;
        let (id, has_previous, has_next, pending) = match lane {
            Lanes::Chats => (
                "search-chats",
                !results.messages.back.is_empty(),
                results
                    .chats
                    .as_ref()
                    .is_some_and(|page| page.message_cursor.is_some()),
                results.messages.pending(),
            ),
            _ => (
                "search",
                !results.notes.back.is_empty(),
                results
                    .page
                    .as_ref()
                    .is_some_and(|page| page.cursor.is_some()),
                results.notes.pending(),
            ),
        };
        let why = |has: bool, edge: &'static str| {
            if pending {
                Some("search is preparing")
            } else if !has {
                Some(edge)
            } else {
                None
            }
        };
        div()
            .flex()
            .gap_1()
            .pt_1()
            .child(
                pill(format!("{id}-previous"), "← Previous")
                    .disabled(!has_previous || pending)
                    .when_some(
                        why(has_previous, "this is the first page"),
                        |button, why| {
                            let (announced, tooltip) =
                                components::unavailable("Previous", Some(why));
                            button.accessibility_label(announced).tooltip(tooltip)
                        },
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        match lane {
                            Lanes::Chats => this.previous_chats_page(window, cx),
                            _ => this.previous_search_page(window, cx),
                        }
                        cx.notify();
                    })),
            )
            .child(
                pill(format!("{id}-next"), "Next →")
                    .disabled(!has_next || pending)
                    .when_some(why(has_next, "this is the last page"), |button, why| {
                        let (announced, tooltip) = components::unavailable("Next", Some(why));
                        button.accessibility_label(announced).tooltip(tooltip)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        match lane {
                            Lanes::Chats => this.next_chats_page(window, cx),
                            _ => this.next_search_page(window, cx),
                        }
                        cx.notify();
                    })),
            )
    }

    /// The "In note text" block, then "In chats": exact counts, matching
    /// context and paging for each.
    pub(super) fn search_results(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let results = &self.results;
        let typing = results.fresh.is_some();
        let notes_pending = results.notes.pending();
        let mut block = div()
            .id("search-results")
            .flex()
            .flex_col()
            .gap_1()
            .child(section("In note text"));
        if notes_pending {
            block = block.child(components::loading(
                "search-status",
                if results.preparing {
                    "Preparing search…"
                } else {
                    "Searching…"
                },
            ));
        }
        let status = match (&results.page, notes_pending) {
            (_, true) => String::new(),
            (Some(page), false) if page.total > 0 => {
                let first = page.offset + 1;
                let last = page.offset + page.hits.len();
                let mut status = format!(
                    "{} {} · {first}–{last}",
                    page.total,
                    if page.total == 1 { "match" } else { "matches" }
                );
                if page.reachable < page.total {
                    status.push_str(&format!(
                        " · best {} reachable; add words to narrow",
                        page.reachable
                    ));
                }
                match page.semantic.as_str() {
                    "" | "none" => {}
                    "complete" => status.push_str(" · with meaning"),
                    other => status.push_str(&format!(" · meaning {other}")),
                }
                status
            }
            _ => String::new(),
        };
        if !status.is_empty() {
            block = block.child(
                metadata(status)
                    .id("search-status")
                    .font_family(UI_FONT)
                    .test_support(),
            );
        }
        if let Some(error) = &results.error {
            block = block.child(components::banner(
                "search-error",
                Tone::Error,
                format!(
                    "Search failed: {error}. Note titles above still match; edit the search to try again."
                ),
                vec![],
                None,
            ));
        }
        for (index, hit) in self.hits.iter().enumerate() {
            let id = hit.note.clone();
            let range = hit.start..hit.end;
            let mut facts = Vec::new();
            if !hit.heading_path.is_empty() {
                facts.push(format!("§ {}", hit.heading_path));
            }
            if hit.matches > 1 {
                facts.push(format!("{} passages", hit.matches));
            }
            if hit.imported {
                facts.push("imported".into());
            }
            if hit.protected {
                facts.push("protected".into());
            }
            let links = if hit.inbound_links > 0 {
                format!("← {}", hit.inbound_links)
            } else {
                String::new()
            };
            let row = format!("hit-{index}");
            let mut entry = div().flex().flex_col().pb_1().child(
                archived_chip(
                    components::ListRow::new(row.clone(), hit.title.clone()).meta(links),
                    &row,
                    hit.archived,
                )
                .build()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_note(&id, None, window, cx);
                    this.reveal(range.clone(), window, cx);
                })),
            );
            if !facts.is_empty() {
                entry = entry.child(
                    div()
                        .px_2()
                        .font_family(UI_FONT)
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child(facts.join(" · ")),
                );
            }
            entry = entry.child(
                div()
                    .px_2()
                    .text_size(UI_TEXT)
                    .text_color(rgb(p.muted))
                    .child(highlighted(&hit.snippet, &hit.highlights)),
            );
            block = block.child(entry);
        }
        let answered = results.page.is_some() && !notes_pending;
        if answered && self.hits.is_empty() {
            block = block.child(components::empty_state(
                "search-empty",
                IconName::Search,
                "No matching note text",
                if results.message_hits().is_empty() {
                    "Try another word, or clear the search to browse."
                } else {
                    "Matching chat messages are listed below."
                },
                None,
            ));
        }
        if results
            .page
            .as_ref()
            .is_some_and(|p| p.total > p.hits.len())
        {
            block = block.child(self.search_paging(Lanes::Notes, cx));
        }
        block = block.child(self.chat_results(typing, cx));
        block.test_support().into_any_element()
    }

    /// "In chats": saved chat Messages that match, each opening its chat.
    /// Shown once a page holds any, or while its own page loads.
    fn chat_results(&self, typing: bool, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let results = &self.results;
        // A new query's answer replaces the old page; nothing stale shows.
        if typing {
            return div().into_any_element();
        }
        let paging = results.messages.pending();
        let Some(page) = results
            .chats
            .as_ref()
            .filter(|page| page.message_total > 0 || paging)
        else {
            return div().into_any_element();
        };
        let mut block = div()
            .id("search-chats")
            .flex()
            .flex_col()
            .gap_1()
            .child(section("In chats"));
        if paging {
            block = block.child(components::loading(
                "search-chats-status",
                "Searching chats…",
            ));
        } else {
            let first = page.offset + 1;
            let last = page.offset + page.messages.len();
            let total = page.message_total;
            let status = if page.messages.is_empty() {
                format!(
                    "{total} {}",
                    if total == 1 { "message" } else { "messages" }
                )
            } else {
                format!(
                    "{total} {} · {first}–{last}",
                    if total == 1 { "message" } else { "messages" }
                )
            };
            block = block.child(
                metadata(status)
                    .id("search-chats-status")
                    .font_family(UI_FONT)
                    .test_support(),
            );
            for (index, hit) in page.messages.iter().enumerate() {
                let row = format!("chat-hit-{index}");
                let entry = archived_chip(
                    components::ListRow::new(row.clone(), hit.chat_title.clone())
                        .meta(role_label(&hit.role)),
                    &row,
                    hit.archived,
                );
                let (chat, message) = (hit.chat.clone(), hit.message.clone());
                block =
                    block.child(
                        div()
                            .flex()
                            .flex_col()
                            .pb_1()
                            .child(entry.build().on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_chat_message(&chat, &message, window, cx)
                                },
                            )))
                            .child(
                                div()
                                    .id(gpui_kit::SharedString::from(format!("{row}-snippet")))
                                    .px_2()
                                    .text_size(UI_TEXT)
                                    .text_color(rgb(p.muted))
                                    .aria_label(hit.snippet.clone())
                                    .child(highlighted(&hit.snippet, &hit.highlights))
                                    .test_support(),
                            ),
                    );
            }
        }
        if page.message_total > page.messages.len() || !results.messages.back.is_empty() {
            block = block.child(self.search_paging(Lanes::Chats, cx));
        }
        block.test_support().into_any_element()
    }
}
