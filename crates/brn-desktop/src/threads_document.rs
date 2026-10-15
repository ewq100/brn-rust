//! Document presentation is derived from the one checked Markdown buffer.
use super::shell::pill;
use super::*;
use gpui_kit::base::RangeHighlight;
use gpui_kit::component::menu::DropdownMenu;

pub(super) fn readable_document(
    view: &Entity<TextViewState>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
) -> TextView {
    let p = pal();
    markdown(view, assets)
        .font_family("Georgia")
        .text_base()
        .style(
            gpui_kit::base::TextViewStyle::default()
                .with_foreground(rgb(p.ink).into())
                .with_muted_foreground(rgb(p.muted).into())
                .with_link(rgb(p.accent).into())
                .with_border(rgb(p.border).into())
                .with_selection(rgb(p.selection).into())
                .with_code_background(rgb(p.alt).into())
                .with_dark(p.dark)
                .with_paragraph_gap(gpui_kit::rems(0.95)),
        )
}

impl ThreadsUi {
    pub(super) fn set_review_reading(
        &mut self,
        reading: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.review_editing
            || !self
                .review
                .as_ref()
                .is_some_and(|review| review.note.is_some())
        {
            return;
        }
        self.review_reading = reading;
        self.focus_root(window, cx);
        if reading {
            self.candidate_view
                .read(cx)
                .focus_handle()
                .clone()
                .focus(window, cx);
        } else {
            self.diff_editor
                .update(cx, |editor, cx| editor.focus(window, cx));
        }
        cx.notify();
    }
    pub(super) fn sync_read_text(&mut self, cx: &mut Context<Self>) {
        let source = self.note_editor.read(cx).value().to_string();
        if self.read_source != source {
            self.read_source = source.clone();
            self.read_marks_key.clear();
            self.selection = None;
            self.note_read_view.update(cx, |view, cx| {
                view.clear_selection(cx);
                view.set_text(&source, cx);
            });
        }
    }

    pub(super) fn set_note_reading(
        &mut self,
        reading: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.review.is_some() || self.note.is_none() {
            return;
        }
        self.preserve_visible_note_input(cx);
        self.sync_read_text(cx);
        self.note_reading = reading;
        self.selection = None;
        self.focus_visible_editor(window, cx);
        cx.notify();
    }

    pub(super) fn sync_read_marks(&mut self, cx: &mut Context<Self>) {
        if !self.note_reading {
            return;
        }
        let Some(open) = &self.note else {
            return;
        };
        let RecordData::Note(note) = &open.record.data else {
            return;
        };
        let rendered = self.note_read_view.read(cx).rendered_text();
        if rendered.source() != self.read_source {
            return;
        }
        let mut ranges = vec![];
        if open.session.is_none() && self.read_source == note.markdown {
            for entry in self
                .workspace
                .writing_comments(&open.record.id)
                .unwrap_or_default()
            {
                if !entry.addressed
                    && let RecordData::Comment(comment) = entry.record.data
                    && let Some(source) =
                        comment_location(&comment, open.record.version, &note.markdown)
                    && let Some(range) = rendered.range_for_source(source)
                {
                    ranges.push(range);
                }
            }
            if let Some((id, version, source, _)) = &self.comment_anchor
                && id == &open.record.id
                && *version == open.record.version
                && let Some(range) = rendered.range_for_source(source.clone())
            {
                ranges.push(range);
            }
        }
        let key = format!(
            "{}:{}:{}:{ranges:?}",
            open.record.id,
            open.record.version,
            pal().comment
        );
        if key != self.read_marks_key {
            self.read_marks_key = key;
            let highlights = ranges
                .into_iter()
                .map(|range| RangeHighlight::new(range, rgb(pal().comment)))
                .collect::<Vec<_>>();
            self.note_read_view.update(cx, |view, cx| {
                let _ = view.set_range_highlights(highlights, cx);
            });
        }
    }

    pub(super) fn document_toolbar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let blocked = self.recovery_blocked() || self.review.is_some();
        let mut bar = div()
            .id("document-toolbar")
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(pal().border))
            .child(
                pill("note-write", "Write")
                    .selected(!self.note_reading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.set_note_reading(false, window, cx)),
                    ),
            )
            .child(
                pill("note-read", "Read")
                    .selected(self.note_reading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.set_note_reading(true, window, cx)),
                    ),
            );
        if !self.note_reading {
            for (id, label, kind, hint) in [
                (
                    "format-heading",
                    "Heading",
                    formatting::Format::Heading,
                    "Toggle a heading on the selected lines",
                ),
                (
                    "format-bold",
                    "Bold",
                    formatting::Format::Bold,
                    "Bold the selected words",
                ),
                (
                    "format-italic",
                    "Italic",
                    formatting::Format::Italic,
                    "Italicize the selected words",
                ),
                (
                    "format-list",
                    "List",
                    formatting::Format::List,
                    "Toggle a bullet list on selected lines",
                ),
                (
                    "format-quote",
                    "Quote",
                    formatting::Format::Quote,
                    "Toggle a quotation on selected lines",
                ),
            ] {
                bar = bar.child(pill(id, label).disabled(blocked).tooltip(hint).on_click(
                    cx.listener(move |this, _, window, cx| this.format_note(kind, window, cx)),
                ));
            }
        } else {
            let outline = sections(self.note_editor.read(cx).value().as_ref());
            let this = cx.entity().downgrade();
            bar = bar
                .child(
                    pill("note-outline", "Outline")
                        .dropdown_caret(true)
                        .dropdown_menu(move |mut menu, _, _| {
                            for (label, range) in &outline {
                                let range = range.clone();
                                menu = menu.item(super::shell::item(
                                    &this,
                                    label.clone(),
                                    None,
                                    false,
                                    false,
                                    move |view, window, cx| view.reveal(range.clone(), window, cx),
                                ));
                            }
                            menu
                        }),
                )
                .child(metadata("Select text to comment"));
        }
        bar.test_support().into_any_element()
    }

    pub(super) fn format_note(
        &mut self,
        kind: formatting::Format,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Toolkit programmatic replacement intentionally bypasses readonly.
        if !self.note_editor_visible() || self.note_reading || self.review.is_some() {
            return;
        }
        let editor = self.note_editor.read(cx);
        let text = editor.value().to_string();
        let range = editor.selected_range();
        let Some(edit) = formatting::format(&text, range, kind) else {
            self.warn("Select plain text or a supported paragraph to format. Code and complex Markdown stay unchanged.");
            cx.notify();
            return;
        };
        self.note_editor.update(cx, |editor, cx| {
            editor.set_selected_range(edit.range, cx);
            editor.replace(edit.replacement, window, cx);
            editor.set_selected_range(edit.selection, cx);
            editor.focus(window, cx);
        });
        self.note_changed(self.note_editor.read(cx).value().to_string());
        self.sync_read_text(cx);
        self.track_selection(cx);
        cx.notify();
    }

    pub(super) fn save_and_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.review.is_some() || self.recovery_blocked() {
            return;
        }
        let Some((range, quote)) = self.selection.clone() else {
            return;
        };
        let Some(open) = &self.note else {
            return;
        };
        let id = open.record.id.clone();
        let source = self.note_editor.read(cx).value().to_string();
        if source.get(range.clone()) != Some(quote.as_str()) {
            return;
        }
        self.save(window, cx);
        let committed = self.note.as_ref().is_some_and(|open| {
            open.record.id == id
                && open.session.is_none()
                && matches!(&open.record.data, RecordData::Note(note) if note.markdown == source)
        });
        if committed && self.note_editor.read(cx).value().as_ref() == source {
            self.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(range.clone(), cx)
            });
            self.selection = Some((range, quote));
            self.comment_on_selection(window, cx);
        }
        cx.notify();
    }
}
