use super::*;

impl Desktop {
    pub(super) fn render_vault_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let open_draft = self
            .draft_state
            .as_ref()
            .filter(|_| self.open_doc == Some(DocRef::Draft))
            .map(|state| state.id());
        let mut list = div()
            .id("vault-rail-list")
            .track_scroll(&self.vault_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .p_2()
            .child(section_label("Sources", p))
            .child(
                div()
                    .text_color(color(p.muted))
                    .child(format!("{} sources", self.sources.len())),
            );
        for source in &self.sources {
            let id = source.source_id;
            list = list.child(
                Button::new(format!("source-{id}"))
                    .label(format!(
                        "{} · {}",
                        compact_title(&source.title),
                        approval_tag(source.approval)
                    ))
                    .selected(self.open_doc == Some(DocRef::Source(id)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.open_doc != Some(DocRef::Source(id)) {
                            this.source_scroll.set_offset(point(px(0.), px(0.)));
                        }
                        this.show_document(DocRef::Source(id));
                        cx.notify();
                    })),
            );
        }
        list = list
            .child(section_label("Working drafts", p))
            .child(Input::new(&self.draft_title).aria_label("New draft title"))
            .child(
                Button::new("create-draft")
                    .label("Create blank draft")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.create_draft(cx))),
            )
            .child(
                Button::new("refresh-drafts")
                    .label("Refresh list")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(Action::ListDrafts, "Draft list", cx);
                    })),
            )
            .child(
                div()
                    .text_color(color(p.muted))
                    .child(format!("{} drafts", self.drafts.len())),
            );
        for draft in &self.drafts {
            let id = draft.id;
            list = list.child(
                Button::new(format!("vault-draft-{id}"))
                    .label(format!(
                        "{} · {}…",
                        compact_title(&draft.title),
                        &id.to_string()[..8]
                    ))
                    .selected(open_draft == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.open_draft_from_list(id, cx))),
            );
        }
        list = list
            .child(section_label("Import", p))
            .child(div().text_color(color(p.muted)).child(
                "Import a UTF-8 Markdown or text file and explicitly approve it for search.",
            ))
            .child(
                Button::new("choose-file")
                    .label(if self.choosing_file {
                        "Choosing…"
                    } else {
                        "Choose file…"
                    })
                    .disabled(self.choosing_file || !self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.choose_file(cx))),
            )
            .child(
                Button::new("import-approved")
                    .label("Import and approve")
                    .disabled(!self.phase.can_submit() || self.import_path.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.import(cx))),
            )
            .child(
                Button::new("build-index")
                    .label("Build index")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(Action::Build, "Index build", cx);
                    })),
            )
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(
                            Action::Refresh {
                                session: this.selected_session,
                            },
                            "Refresh",
                            cx,
                        );
                    })),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .text_color(color(p.muted))
                    .child(format!(
                        "Selected file: {}",
                        self.import_path.as_ref().map_or("none".into(), |path| {
                            spaced_identifier(&path.display().to_string())
                        })
                    )),
            );
        div()
            .w(px(self.layout.vault_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
    }
}
