//! Derived citation review stays inside Needs Review and owns no editable document.
use super::*;
use crate::ai::citation_review_state::{NeedsReviewMode, RowCapture};
use brn_workflow::knowledge::CitationReviewDetail;
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable, component::Selectable};

pub(super) struct CitationReviewPane {
    pub(super) consumer: Entity<EditorState>,
    pub(super) proof: Entity<EditorState>,
    pub(super) quotes: Vec<Entity<EditorState>>,
    pub(super) scroll: ScrollHandle,
    snapshot: Option<CitationReviewDetail>,
}
impl CitationReviewPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            consumer: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            proof: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            quotes: Vec::new(),
            scroll: ScrollHandle::new(),
            snapshot: None,
        }
    }
}
pub(super) fn consumer_widget(editor: &Entity<EditorState>) -> Editor {
    simple::evidence_widget(editor).aria_label("Complete exact saved citation consumer")
}
pub(super) fn citation_proof_widget(editor: &Entity<EditorState>) -> Editor {
    findings::proof_widget(editor).aria_label("Complete saved source citation details")
}
fn copy(id: &'static str, label: &'static str, text: String) -> Button {
    Button::new(id)
        .label(label)
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()));
        })
}
impl Desktop {
    pub(super) fn sync_citation_review_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let detail = self.ai.as_ref().unwrap().citation_review.detail.clone();
        if self.citation_review.snapshot == detail {
            return;
        }
        let text = detail
            .as_ref()
            .map_or_else(String::new, |detail| detail.text.clone());
        let proof = detail.as_ref().map_or_else(String::new, |detail| {
            serde_json::to_string_pretty(&detail.provenance).expect("citation provenance JSON")
        });
        self.citation_review
            .consumer
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.citation_review
            .proof
            .update(cx, |editor, cx| editor.set_value(proof, window, cx));
        self.citation_review.quotes = detail.as_ref().map_or_else(Vec::new, |detail| {
            detail
                .provenance
                .citations
                .iter()
                .map(|resolved| {
                    cx.new(|cx| {
                        EditorState::new(window, cx).default_value(resolved.citation.quote.clone())
                    })
                })
                .collect()
        });
        self.citation_review.snapshot = detail;
    }
    pub(super) fn switch_needs_review_mode(
        &mut self,
        mode: NeedsReviewMode,
        cx: &mut Context<Self>,
    ) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let ai = self.ai.as_mut().unwrap();
        if ai.needs_review_mode == mode {
            return;
        }
        let command = match mode {
            NeedsReviewMode::Findings => ai.open_findings(),
            NeedsReviewMode::CitationEvidence => ai.open_citation_review(),
        };
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn render_needs_review_modes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ai = self.ai.as_ref().unwrap();
        let mut modes = div().flex().flex_wrap().gap_1();
        for (id, label, mode) in [
            (
                "needs-review-findings",
                "Findings",
                NeedsReviewMode::Findings,
            ),
            (
                "needs-review-citations",
                "Citation evidence",
                NeedsReviewMode::CitationEvidence,
            ),
        ] {
            modes = modes.child(
                Button::new(id)
                    .label(label)
                    .compact()
                    .selected(ai.needs_review_mode == mode)
                    .disabled(
                        self.findings_blocked()
                            || (mode == NeedsReviewMode::CitationEvidence && !ai.vault_bound),
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.switch_needs_review_mode(mode, cx)),
                    ),
            );
        }
        modes
    }
    fn refresh_citation_review(&mut self, more: bool, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let ai = self.ai.as_mut().unwrap();
        let command = if more {
            ai.load_more_citation_review()
        } else {
            ai.refresh_citation_review()
        };
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        self.citation_review
            .scroll
            .set_offset(point(px(0.), px(0.)));
        cx.notify();
    }
    fn select_citation_review(&mut self, row: RowCapture, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        if let Some(command) = self.ai.as_mut().unwrap().select_citation_review(row) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn render_citation_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let review = &ai.citation_review;
        let blocked = self.findings_blocked();
        let more = review
            .page
            .as_ref()
            .is_some_and(|page| page.next_cursor.is_some());
        let mut content = div().id("citation-review-scroll").track_scroll(&self.citation_review.scroll).overflow_y_scroll().vertical_scrollbar(&self.citation_review.scroll).flex_1().min_h(px(0.)).flex().flex_col().gap_2().p_2()
            .child("Needs Review")
            .child(self.render_needs_review_modes(cx))
            .child("Current notes whose saved citation evidence needs inspection. These observations do not establish that a claim is false or stale.")
            .child("Incomplete means unavailable or uncertain lookup.")
            .child(div().flex().flex_wrap().gap_1()
                .child(Button::new("refresh-citation-review").label("Refresh").compact().disabled(blocked).on_click(cx.listener(|this, _, _, cx| this.refresh_citation_review(false, cx))))
                .child(Button::new("more-citation-review").label("Load more").compact().disabled(blocked || ai.citation_review_loading() || !more).on_click(cx.listener(|this, _, _, cx| this.refresh_citation_review(true, cx)))));
        if ai.citation_review_loading() {
            content = content.child("Inspecting saved citation evidence…");
        }
        if let Some(error) = &review.error {
            content = content.child(format!("Citation evidence: {error}"));
        }
        if let Some(page) = &review.page {
            content = content.child(format!(
                "{} Current notes inspected · {} notes with citation observations in this page",
                page.inspected_count,
                page.entries.len()
            ));
            content = content.child(format!(
                "Coverage: {} · {} diagnostics · {} displayed{}",
                if page.coverage.incomplete {
                    "incomplete"
                } else {
                    "complete"
                },
                page.coverage.diagnostic_count,
                page.coverage.diagnostics.len(),
                if page.coverage.diagnostics_truncated {
                    " (truncated)"
                } else {
                    ""
                }
            ));
            for issue in &page.coverage.diagnostics {
                content = content.child(format!(
                    "Lookup diagnostic: {} · {}",
                    issue.path, issue.reason
                ));
            }
            if page.entries.is_empty() {
                content = content.child(if more { "No citation observations in this bounded page. Load more to continue inspecting Current notes." } else { "No citation observations in this page." });
            }
            for (index, entry) in page.entries.iter().enumerate() {
                if let Some(row) = ai.citation_review_row_capture(entry) {
                    let outcomes = entry
                        .citations
                        .iter()
                        .map(|issue| {
                            format!(
                                "#{} {}",
                                issue.index + 1,
                                simple::citation_status(issue.outcome)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · ");
                    content = content.child(
                        Button::new(format!("citation-review-row-{index}"))
                            .label(format!(
                                "{} · {} · {outcomes}",
                                compact_title(&entry.title),
                                entry.path
                            ))
                            .selected(review.selected.as_ref() == Some(entry))
                            .disabled(blocked)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_citation_review(row.clone(), cx)
                            })),
                    );
                }
            }
        }
        if ai.citation_review_detail_loading() {
            content = content.child("Loading exact saved consumer and source evidence…");
        }
        if let Some(detail) = &review.detail {
            content = content
                .child(format!(
                    "Complete saved consumer · {} · {}",
                    detail.title, detail.path
                ))
                .child(
                    div()
                        .h(px(240.))
                        .flex_shrink_0()
                        .child(consumer_widget(&self.citation_review.consumer)),
                )
                .child(copy(
                    "copy-citation-consumer",
                    "Copy exact saved consumer",
                    detail.text.clone(),
                ))
                .child("Saved sources · complete citation details")
                .child(
                    div()
                        .h(px(240.))
                        .flex_shrink_0()
                        .child(citation_proof_widget(&self.citation_review.proof)),
                )
                .child(copy(
                    "copy-citation-proof",
                    "Copy full citation details",
                    serde_json::to_string_pretty(&detail.provenance)
                        .expect("citation provenance JSON"),
                ));
            for (index, resolved) in detail.provenance.citations.iter().enumerate() {
                let mut source = div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap_1()
                    .child(simple::citation_status(resolved.outcome))
                    .child(format!(
                        "Source UUID: {} · cited byte range {}–{}",
                        resolved.citation.note_id,
                        resolved.citation.start_byte,
                        resolved.citation.end_byte
                    ));
                for matched in &resolved.matches {
                    source = source.child(format!("Observed path: {}", matched.path));
                }
                for issue in &resolved.issues {
                    source = source.child(format!(
                        "Inspection issue: {} · {}",
                        issue.path, issue.reason
                    ));
                }
                if let Some(quote) = self.citation_review.quotes.get(index) {
                    source = source.child(
                        div()
                            .h(px(96.))
                            .flex_shrink_0()
                            .child(simple::provenance_quote_widget(quote)),
                    );
                }
                content = content.child(source.child(simple::provenance_copy_button(
                    index,
                    &resolved.citation.quote,
                )));
            }
        } else {
            content = content
                .child("Select a note to inspect its exact saved consumer and complete citations.");
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(content.test_support())
            .into_any_element()
    }
}
