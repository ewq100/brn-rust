//! Owner search in the notes tile. Requests run on the workspace search
//! worker and are awaited off the UI thread; a newer query cancels and
//! replaces the previous one, so typing never waits for the index.
use super::shell::pill;
use super::*;
use brn_threads_app::search::CancelOnDrop;
use gpui_kit::StyledText;

/// Results per page, as in the accepted ordinary-wiki search.
pub(super) const PAGE: usize = 12;

#[derive(Default)]
pub(super) struct SearchView {
    query: String,
    generation: u64,
    /// A request is outstanding for the current query.
    pub(super) pending: bool,
    /// The index is being brought up to date at startup.
    pub(super) preparing: bool,
    pub(super) page: Option<SearchPage>,
    pub(super) error: Option<String>,
    /// Cursors of earlier pages, for Previous.
    back: Vec<Option<String>>,
    current: Option<String>,
    task: Option<Task<()>>,
    cancel: Option<CancelOnDrop>,
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
        self.results.back.clear();
        self.run_search(None, window, cx);
    }

    fn run_search(&mut self, cursor: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let results = &mut self.results;
        results.generation += 1;
        results.error = None;
        results.task = None;
        results.cancel = None;
        results.current = cursor.clone();
        self.hits.clear();
        if results.query.trim().is_empty() {
            results.pending = false;
            results.page = None;
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
        results.cancel = Some(ticket.cancel_on_drop());
        results.pending = true;
        let generation = results.generation;
        results.task = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { ticket.wait() })
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.search_answered(generation, answer.map(Into::into), cx)
            });
        }));
    }

    fn search_answered(
        &mut self,
        generation: u64,
        answer: AppResult<SearchPage>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.results.generation {
            return;
        }
        self.results.pending = false;
        self.results.cancel = None;
        match answer {
            Ok(page) => {
                self.hits = page.hits.clone();
                self.results.page = Some(page);
            }
            Err(error) => {
                self.results.page = None;
                self.results.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    fn next_search_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(next) = self.results.page.as_ref().and_then(|p| p.cursor.clone()) else {
            return;
        };
        let current = self.results.current.take();
        self.results.back.push(current);
        self.run_search(Some(next), window, cx);
    }

    fn previous_search_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.results.back.pop() {
            self.run_search(previous, window, cx);
        }
    }

    /// The "In note text" block: exact counts, matching context and paging.
    pub(super) fn search_results(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let results = &self.results;
        let mut block = div()
            .id("search-results")
            .flex()
            .flex_col()
            .gap_1()
            .child(section("IN NOTE TEXT"));
        let status = match (&results.page, results.pending) {
            (_, true) if results.preparing => "Preparing search…".to_owned(),
            (_, true) => "Searching…".to_owned(),
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
            block = block.child(empty("search-error", format!("Search failed: {error}")));
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
            let marks = hit.highlights.iter().map(|range| {
                (
                    range.clone(),
                    HighlightStyle {
                        font_weight: Some(gpui_kit::FontWeight::SEMIBOLD),
                        color: Some(rgb(p.ink).into()),
                        ..Default::default()
                    },
                )
            });
            let mut entry = div().flex().flex_col().pb_1().child(
                tiles::row(format!("hit-{index}"), short(&hit.title), links, false).on_click(
                    cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, None, window, cx);
                        this.reveal(range.clone(), window, cx);
                    }),
                ),
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
                    .child(StyledText::new(hit.snippet.clone()).with_highlights(marks)),
            );
            block = block.child(entry);
        }
        let answered = results.page.is_some() && !results.pending;
        if answered && self.hits.is_empty() {
            block = block.child(empty(
                "search-empty",
                "No current note text matches. Try another word or clear the filter to browse.",
            ));
        }
        if let Some(page) = results.page.as_ref().filter(|p| p.total > p.hits.len()) {
            let has_previous = !results.back.is_empty();
            let has_next = page.cursor.is_some();
            block = block.child(
                div()
                    .flex()
                    .gap_1()
                    .pt_1()
                    .child(
                        pill("search-previous", "← Previous")
                            .disabled(!has_previous || results.pending)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.previous_search_page(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        pill("search-next", "Next →")
                            .disabled(!has_next || results.pending)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.next_search_page(window, cx);
                                cx.notify();
                            })),
                    ),
            );
        }
        block.test_support().into_any_element()
    }
}
