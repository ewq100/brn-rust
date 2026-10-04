use super::*;

impl Desktop {
    pub(super) fn render_centre(
        &mut self,
        resolved: &ResolvedLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette();
        let centre = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .h_full();
        match resolved.mode {
            CentreMode::ChatOnly => centre.child(self.render_chat(cx)),
            CentreMode::Split => centre.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .child(
                        div()
                            .w(px(resolved.doc_w))
                            .flex_shrink_0()
                            .h_full()
                            .child(self.render_document(cx)),
                    )
                    .child(self.render_divider(crate::layout::Divider::Document, window, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h_full()
                            .child(self.render_chat(cx)),
                    ),
            ),
            CentreMode::Tabs => {
                let tab = self.centre_tab;
                let bar = div()
                    .flex()
                    .flex_shrink_0()
                    .gap_1()
                    .p_1()
                    .border_b_1()
                    .border_color(color(p.line))
                    .bg(color(p.panel))
                    .child(
                        Button::new("tab-document")
                            .label("Document")
                            .compact()
                            .selected(tab == CentreTab::Document)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Document;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("tab-chat")
                            .label("Chat")
                            .compact()
                            .selected(tab == CentreTab::Chat)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Chat;
                                cx.notify();
                            })),
                    );
                let pane = match tab {
                    CentreTab::Document => self.render_document(cx).into_any_element(),
                    CentreTab::Chat => self.render_chat(cx).into_any_element(),
                };
                centre
                    .child(bar)
                    .child(div().flex_1().min_h(px(0.)).child(pane))
            }
        }
    }

    pub(super) fn render_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.open_doc {
            Some(DocRef::Evidence) => self.render_evidence_document(cx),
            Some(DocRef::Proposal(_)) => self.render_proposal_review(cx),
            Some(DocRef::Activity) => self.render_activity(cx),
            Some(DocRef::Findings) => self.render_findings(cx),
            Some(DocRef::Draft) => self.render_draft(cx),
            _ => self.render_simple_document(cx),
        }
    }

    fn render_chat(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.render_simple_chat(cx)
    }
}
