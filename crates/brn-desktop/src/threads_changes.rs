//! Recent changes: global operation receipts, newest first, each with the
//! same checked receipt Undo.
use super::shell::pill;
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::tooltip::Tooltip;

use super::tiles::page;
use super::tiles::{PAGE_WIDTH, column};

fn group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).flex().flex_col().gap(px(2.)).min_w_0()
}

impl ThreadsUi {
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
}
