//! Complete retained finding proof over the shared worker; no knowledge writes.
use super::*;
use brn_workflow::findings::{FindingInspection, FindingRecord, FindingState};
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable, component::Selectable};

pub(super) struct FindingsPane {
    pub(super) proof: Entity<EditorState>,
    pub(super) scroll: ScrollHandle,
    snapshot: Option<FindingRecord>,
    pub(super) quote_index: usize,
}
impl FindingsPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            proof: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            scroll: ScrollHandle::new(),
            snapshot: None,
            quote_index: 0,
        }
    }
}
pub(super) fn proof_widget(editor: &Entity<EditorState>) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label("Complete retained finding and separate fresh evidence")
}
pub(super) fn proof_text(record: &FindingRecord, inspection: Option<&FindingInspection>) -> String {
    let retained = serde_json::to_string_pretty(record).expect("checked finding JSON");
    let fresh = inspection.map_or_else(
        || "Not inspected in this view.".into(),
        |inspection| {
            serde_json::to_string_pretty(&inspection.evidence).expect("finding observations JSON")
        },
    );
    format!(
        "Complete retained finding:\n{retained}\n\nSeparate fresh evidence observations:\n{fresh}"
    )
}
fn display_text(ai: &crate::ai::AiState) -> String {
    let queue = &ai.finding_queue;
    let mut text = queue.selected.as_ref().map_or_else(String::new, |record| {
        proof_text(record, queue.inspection.as_ref())
    });
    if let Some(request) = ai.finding_capture_retry_request() {
        text.push_str("\n\nExact retained capture request:\n");
        text.push_str(
            &serde_json::to_string_pretty(request).expect("finding capture request JSON"),
        );
    }
    if let Some(request) = ai.finding_close_retry_request() {
        text.push_str("\n\nExact retained closure request:\n");
        text.push_str(
            &serde_json::to_string_pretty(request).expect("finding closure request JSON"),
        );
    }
    text
}
pub(super) fn state_name(state: FindingState) -> &'static str {
    match state {
        FindingState::Open => "Open",
        FindingState::Resolved => "Resolved",
        FindingState::Dismissed => "Dismissed",
    }
}
fn copy(id: &'static str, label: &'static str, text: String) -> Button {
    Button::new(id)
        .label(label)
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()))
        })
}

impl Desktop {
    pub(super) fn sync_finding_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ai = self.ai.as_ref().unwrap();
        let queue = &ai.finding_queue;
        if self.findings.snapshot.as_ref() != queue.selected.as_ref() {
            self.findings.snapshot = queue.selected.clone();
            self.findings.quote_index = 0;
        }
        let text = display_text(ai);
        if self.findings.proof.read(cx).value().as_ref() != text {
            self.findings
                .proof
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
    }
    fn findings_blocked(&self) -> bool {
        !self.ai.as_ref().unwrap().ready
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
    }
    fn refresh_finding_page(
        &mut self,
        state: Option<FindingState>,
        before: Option<Uuid>,
        cx: &mut Context<Self>,
    ) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().refresh_findings(state, before);
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        self.findings.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }
    fn select_finding(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().select_finding(id);
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    fn inspect_finding(&mut self, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().inspect_selected_finding();
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn close_finding(&mut self, state: FindingState, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().close_selected_finding(state);
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    fn retry_finding_capture(&mut self, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().retry_finding_capture();
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    fn retry_finding_close(&mut self, cx: &mut Context<Self>) {
        if self.findings_blocked() || self.open_doc != Some(DocRef::Findings) {
            return;
        }
        let command = self.ai.as_mut().unwrap().retry_finding_close();
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn keep_link_finding(&mut self, cx: &mut Context<Self>) {
        if self.inspection_blocked()
            || !matches!(self.open_doc, Some(DocRef::SavedNote | DocRef::Evidence))
        {
            return;
        }
        let command = self
            .ai
            .as_mut()
            .unwrap()
            .capture_link_finding(self.saved_links.item);
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn keep_identity_finding(&mut self, cx: &mut Context<Self>) {
        if self.inspection_blocked()
            || !matches!(self.open_doc, Some(DocRef::SavedNote | DocRef::Evidence))
        {
            return;
        }
        let command = self.ai.as_mut().unwrap().capture_identity_finding();
        if let Some(command) = command {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    fn step_finding_quote(&mut self, next: bool, cx: &mut Context<Self>) {
        let total = self
            .ai
            .as_ref()
            .unwrap()
            .finding_queue
            .selected
            .as_ref()
            .map_or(0, |r| r.draft.evidence.len());
        self.findings.quote_index = if next {
            self.findings
                .quote_index
                .checked_add(1)
                .filter(|index| *index < total)
                .unwrap_or(self.findings.quote_index)
        } else {
            self.findings.quote_index.saturating_sub(1)
        };
        cx.notify();
    }
    pub(super) fn render_findings(&self, cx: &mut Context<Self>) -> AnyElement {
        use super::ui::{self, Tone};
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let queue = &ai.finding_queue;
        let full_text = display_text(ai);
        let blocked = self.findings_blocked();
        let state = queue.state;
        let mut filters = div().flex().flex_wrap().gap(px(2.));
        for (index, (label, filter)) in [
            ("Open", Some(FindingState::Open)),
            ("Resolved", Some(FindingState::Resolved)),
            ("Dismissed", Some(FindingState::Dismissed)),
            ("All", None),
        ]
        .into_iter()
        .enumerate()
        {
            filters = filters.child(
                Button::new(format!("finding-filter-{index}"))
                    .label(label)
                    .compact()
                    .small()
                    .when(state == filter, |button| button.primary())
                    .when(state != filter, |button| button.ghost())
                    .selected(state == filter)
                    .disabled(blocked)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.refresh_finding_page(filter, None, cx)
                    })),
            );
        }
        let count = queue
            .page
            .as_ref()
            .map(|page| format!("{} open findings", page.open_count))
            .unwrap_or_else(|| "Tentative findings with retained evidence".into());
        let before = queue.page.as_ref().and_then(|page| page.next_before);
        let mut content = div().id("findings-scroll").track_scroll(&self.findings.scroll).overflow_y_scroll().vertical_scrollbar(&self.findings.scroll).flex_1().min_h(px(0.)).flex().flex_col().gap(px(tokens::space::MD)).px(px(tokens::space::LG)).py(px(tokens::space::MD))
            .child(ui::view_header("Needs Review", None, Some(count), p).mx(px(-tokens::space::LG)).mt(px(-tokens::space::MD)))
            .child(ui::callout(Tone::Attention, "Tentative findings retain saved evidence. Resolve or Dismiss changes this queue only; correcting knowledge requires a reviewed proposal.", p))
            .child(filters)
            .child(ui::toolbar()
                .child(Button::new("refresh-findings").label("Newest findings").ghost().small().disabled(blocked || ai.findings_loading()).on_click(cx.listener(move |this, _, _, cx| this.refresh_finding_page(state, None, cx))))
                .child(Button::new("older-findings").label("Older page").ghost().small().disabled(blocked || ai.findings_loading() || before.is_none()).on_click(cx.listener(move |this, _, _, cx| { if let Some(before) = before { this.refresh_finding_page(state, Some(before), cx); } }))));
        if ai.findings_loading() {
            content = content.child(ui::hint("Loading findings…", p));
        }
        if let Some(error) = &queue.error {
            content = content.child(ui::callout(Tone::Danger, format!("Findings: {error}"), p));
        }
        if let Some(error) = &queue.capture_error {
            content = content.child(ui::callout(Tone::Danger, format!("Capture: {error}"), p));
        }
        if let Some(request) = ai.finding_capture_retry_request() {
            content = content
                .child(format!(
                    "Retained capture {} · complete request below",
                    request.id
                ))
                .child(
                    Button::new("retry-finding-capture")
                        .label("Retry exact retained capture")
                        .compact()
                        .disabled(blocked || ai.finding_capture_pending())
                        .on_click(cx.listener(|this, _, _, cx| this.retry_finding_capture(cx))),
                );
        }
        if let Some(error) = &queue.close_error {
            content = content.child(format!("Closure: {error}"));
        }
        if let Some(request) = ai.finding_close_retry_request() {
            content = content
                .child(format!(
                    "Retained closure {} · version {} · {}",
                    request.expected.id,
                    request.expected.version,
                    state_name(request.state)
                ))
                .child(
                    Button::new("retry-finding-close")
                        .label("Retry exact retained closure")
                        .compact()
                        .disabled(blocked || ai.finding_close_pending())
                        .on_click(cx.listener(|this, _, _, cx| this.retry_finding_close(cx))),
                );
        }
        if let Some(record) = &queue.capture_result {
            let id = record.draft.request.id;
            content = content
                .child(format!(
                    "Retained finding {id} · {}",
                    state_name(record.state)
                ))
                .child(
                    Button::new("inspect-captured-finding")
                        .label("Inspect captured finding")
                        .compact()
                        .disabled(blocked || ai.finding_loading())
                        .on_click(cx.listener(move |this, _, _, cx| this.select_finding(id, cx))),
                );
        }
        if let Some(record) = &queue.close_result {
            let id = record.draft.request.id;
            content = content
                .child(format!(
                    "Recorded closure {id} · {}",
                    state_name(record.state)
                ))
                .child(
                    Button::new("inspect-recorded-finding-closure")
                        .label("Inspect recorded closure")
                        .compact()
                        .disabled(blocked || ai.finding_loading())
                        .on_click(cx.listener(move |this, _, _, cx| this.select_finding(id, cx))),
                );
        }
        if let Some(page) = &queue.page {
            content = content.child(format!(
                "{} open at this refresh · {} in this page",
                page.open_count,
                page.entries.len()
            ));
            if page.entries.is_empty() {
                content = content.child(ui::empty_state(
                    "No findings in this page.",
                    "Conflicts, stale knowledge and identity questions appear here without interrupting your work.",
                    p,
                ));
            }
            for record in &page.entries {
                let id = record.draft.request.id;
                content = content.child(
                    Button::new(format!("finding-row-{id}"))
                        .label(format!(
                            "{} · {}",
                            state_name(record.state),
                            compact_title(&record.draft.title)
                        ))
                        .selected(
                            queue
                                .selected
                                .as_ref()
                                .is_some_and(|r| r.draft.request.id == id),
                        )
                        .disabled(blocked)
                        .on_click(cx.listener(move |this, _, _, cx| this.select_finding(id, cx))),
                );
            }
        }
        if ai.finding_loading() {
            content = content.child("Loading selected evidence…");
        }
        if let Some(record) = &queue.selected {
            let terminal = record.state != FindingState::Open || record.version != 1;
            content = content.child(format!("{} · version {}", state_name(record.state), record.version))
                .child(div().flex().flex_wrap().gap_1()
                    .child(Button::new("inspect-finding-evidence").label("Inspect current evidence").compact().disabled(blocked || ai.finding_loading()).on_click(cx.listener(|this, _, _, cx| this.inspect_finding(cx))))
                    .child(Button::new("resolve-finding").label("Resolve").compact().disabled(blocked || terminal || ai.finding_close_pending()).on_click(cx.listener(|this, _, _, cx| this.close_finding(FindingState::Resolved, cx))))
                    .child(Button::new("dismiss-finding").label("Dismiss").compact().disabled(blocked || terminal || ai.finding_close_pending()).on_click(cx.listener(|this, _, _, cx| this.close_finding(FindingState::Dismissed, cx)))))
                .child("Complete retained evidence; fresh observations are separate. Unchanged evidence does not establish that the issue still persists.")
                .child(div().h(px(240.)).flex_shrink_0().child(proof_widget(&self.findings.proof)))
                .child(copy("copy-full-finding-evidence", "Copy full displayed evidence", full_text.clone()))
                .child(div().flex().flex_wrap().gap_1()
                    .child(format!("Quote {} of {}", self.findings.quote_index + 1, record.draft.evidence.len()))
                    .child(Button::new("previous-finding-quote").label("Previous quote").compact().disabled(self.findings.quote_index == 0).on_click(cx.listener(|this, _, _, cx| this.step_finding_quote(false, cx))))
                    .child(Button::new("next-finding-quote").label("Next quote").compact().disabled(self.findings.quote_index + 1 >= record.draft.evidence.len()).on_click(cx.listener(|this, _, _, cx| this.step_finding_quote(true, cx)))));
            if let Some(quote) = record
                .draft
                .evidence
                .get(self.findings.quote_index)
                .and_then(|proof| proof.quote.as_ref())
            {
                content = content.child(copy(
                    "copy-exact-finding-quote",
                    "Copy exact original quote",
                    quote.quote.clone(),
                ));
            } else {
                content = content.child("This evidence has no captured quote.");
            }
        } else {
            content = content.child(ui::hint(
                "Select a finding to inspect its complete retained proof.",
                p,
            ));
            if !full_text.is_empty() {
                content = content
                    .child(
                        div()
                            .h(px(240.))
                            .flex_shrink_0()
                            .child(proof_widget(&self.findings.proof)),
                    )
                    .child(copy(
                        "copy-full-finding-evidence",
                        "Copy full displayed evidence",
                        full_text,
                    ));
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(super::theme::color(p.paper))
            .child(content.test_support())
            .into_any_element()
    }
}
