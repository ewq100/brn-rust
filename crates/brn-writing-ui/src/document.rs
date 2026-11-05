//! Concrete editor and formatted Read presentation over one Markdown buffer.
use crate::native::*;
use crate::{
    components, formatting, layout,
    theme::{metrics, pal},
};
use brn_threads_app::*;
use gpui_kit::base::{Disableable, RangeHighlight, Selectable, TextView, TextViewState};
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::{Context, Entity, Image, Window, prelude::*, px, rgb};
use std::{collections::BTreeMap, sync::Arc};
/// The formatting actions: icon buttons in a wide Note, Format ▾ below.
const FORMATS: [(&str, &str, IconName, formatting::Format, &str); 5] = [
    (
        "format-heading",
        "Heading",
        IconName::Heading,
        formatting::Format::Heading,
        "Heading · toggle a heading on the selected lines",
    ),
    (
        "format-bold",
        "Bold",
        IconName::Bold,
        formatting::Format::Bold,
        "Bold · bold the selected words",
    ),
    (
        "format-italic",
        "Italic",
        IconName::Italic,
        formatting::Format::Italic,
        "Italic · italicize the selected words",
    ),
    (
        "format-list",
        "List",
        IconName::List,
        formatting::Format::List,
        "List · toggle a bullet list on the selected lines",
    ),
    (
        "format-quote",
        "Quote",
        IconName::TextQuote,
        formatting::Format::Quote,
        "Quote · toggle a quotation on the selected lines",
    ),
];

/// A rendered document (note, proposal result) in the shared article styles
/// (spec §10.1). `collector` records the headings of its parse for the
/// outline when this view supplies one.
pub fn readable_document(
    view: &Entity<TextViewState>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
    collector: Option<&crate::outline::HeadingCollector>,
) -> TextView {
    let base = TextView::new(view)
        .font_family(UI_FONT)
        .text_size(px(metrics::TEXT_DOCUMENT))
        .line_height(gpui_kit::relative(metrics::DOCUMENT_LINE_HEIGHT))
        .w_full()
        .h_full()
        .scrollable(true);
    let view = match collector {
        Some(collector) => crate::markdown::observed_markdown(base, assets, collector),
        None => crate::markdown::safe_markdown(base, assets),
    };
    view.style(document_style())
        .code_block_actions(|block, _, _| copy_code_button(block.code()))
}

/// The article styles: H1–H6 28/22/18/16/16/16 medium at 1.3, 12 between
/// paragraphs, 24 before and 8 after headings (the paragraph gap above a
/// heading collapses into its 24), quiet quotes, readable code and tables
/// whose wide content scrolls inside the table.
pub fn document_style() -> gpui_kit::base::TextViewStyle {
    use gpui_kit::{Overflow, StyleRefinement};
    let p = pal();
    let mut table = StyleRefinement::default();
    table.overflow.x = Some(Overflow::Scroll);
    gpui_kit::base::TextViewStyle::default()
        .with_foreground(rgb(p.ink).into())
        .with_muted_foreground(rgb(p.muted).into())
        .with_link(rgb(p.link).into())
        .with_border(rgb(p.hairline).into())
        .with_selection(rgb(p.selection).into())
        .with_code_background(rgb(p.alt).into())
        .with_dark(p.dark)
        .with_paragraph_gap(gpui_kit::rems(
            metrics::DOCUMENT_PARAGRAPH_GAP / metrics::TEXT_DOCUMENT,
        ))
        .with_heading(|level| {
            let size = metrics::DOCUMENT_HEADINGS[usize::from(level.clamp(1, 6)) - 1];
            StyleRefinement::default()
                .text_size(px(size))
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .line_height(gpui_kit::relative(metrics::DOCUMENT_HEADING_LINE_HEIGHT))
                .pt(px(
                    metrics::DOCUMENT_HEADING_BEFORE - metrics::DOCUMENT_PARAGRAPH_GAP
                ))
                .pb(px(metrics::DOCUMENT_HEADING_AFTER))
        })
        .with_blockquote_style(StyleRefinement::default().border_color(rgb(p.hairline)))
        .with_code_block(
            StyleRefinement::default()
                .font_family(MONO)
                .text_size(px(metrics::TEXT_BODY))
                .rounded(px(metrics::RADIUS_CONTROL)),
        )
        .with_table(table)
        .with_table_head(
            StyleRefinement::default()
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(rgb(p.ink)),
        )
        .with_table_cell(StyleRefinement::default().px(px(10.)).py(px(6.)))
}

/// A code block's Copy: the block's exact code, keyboard reachable.
fn copy_code_button(code: gpui_kit::SharedString) -> impl IntoElement {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    code.hash(&mut hasher);
    button(format!("copy-code-{:x}", hasher.finish()))
        .label("Copy")
        .compact()
        .ghost()
        .h(components::tokens::COMPACT_CONTROL)
        .accessibility_label("Copy code")
        .tooltip("Copy this code")
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(code.to_string()));
        })
}

/// Why formatting is unavailable while a kept edit blocks the text.
const RECOVER_FIRST_REASON: &str = "Recover the kept edit first";
impl<T: 'static> WritingView<'_, T> {
    /// The Note toolbar's left side: the Write | Read switch, the formatting
    /// controls (Write only) and the Outline both modes share. `fit` folds
    /// them as the Note panel narrows.
    pub fn document_controls(
        &self,
        fit: layout::NoteFit,
        cx: &mut Context<T>,
    ) -> Vec<gpui_kit::AnyElement> {
        let blocked = self.recovery_blocked() || self.review.is_some();
        let this = cx.entity().downgrade();
        let dispatch = self.dispatch;
        let mut controls = vec![
            components::segmented(
                "note-mode",
                vec![
                    button("note-read")
                        .label("Read")
                        .selected(self.note.is_reading())
                        .tooltip("Read the formatted document; select text to comment")
                        .on_click(self.on(cx, WritingRequest::NoteReading(true))),
                    button("note-write")
                        .label("Edit")
                        .selected(!self.note.is_reading())
                        .tooltip("Edit the Markdown text")
                        .on_click(self.on(cx, WritingRequest::NoteReading(false))),
                ],
            )
            .into_any_element(),
        ];
        if !self.note.is_reading() {
            if fit.format_menu() {
                let this = this.clone();
                let trigger = if fit.icons() {
                    components::icon_button("format-menu", IconName::Type, "Format")
                } else {
                    pill("format-menu", "Format")
                        .dropdown_caret(true)
                        .tooltip("Format the selected text")
                };
                controls.push(
                    trigger
                        .dropdown_menu(move |mut menu, _, _| {
                            // A kept edit makes the text read-only; say so once.
                            if blocked {
                                menu = menu.label(RECOVER_FIRST_REASON);
                            }
                            for (_, label, _, kind, _) in FORMATS {
                                menu = menu.item(item(
                                    &this,
                                    label,
                                    None,
                                    blocked,
                                    false,
                                    WritingRequest::Format(kind),
                                    dispatch,
                                ));
                            }
                            menu
                        })
                        .into_any_element(),
                );
            } else {
                for (id, label, icon, kind, hint) in FORMATS {
                    let (announced, tooltip) = if blocked {
                        components::unavailable(label, Some("recover the kept edit first"))
                    } else {
                        (label.to_owned(), hint.to_owned())
                    };
                    controls.push(
                        components::icon_button(id, icon, label)
                            .disabled(blocked)
                            .accessibility_label(announced)
                            .tooltip(tooltip)
                            .on_click(self.on(cx, WritingRequest::Format(kind)))
                            .into_any_element(),
                    );
                }
            }
        }
        // One Outline toggle for Read and Edit; the region itself is
        // `canvas::outline_region`.
        controls.push(self.outline_toggle(fit, cx).into_any_element());
        controls
    }
}

use crate::diff;
use crate::layout::comment_location;
use gpui_kit::HighlightStyle;
use gpui_kit::base::input::{RangeDecoration, RangeDecorationStyle, TextDecoration};
impl WritingPanel {
    pub fn sync_note_marks<T: 'static>(&mut self, workspace: &Workspace, cx: &mut Context<T>) {
        let p = pal();
        let mut ranges = Vec::new();
        if let Some(open) = self.note.as_ref()
            && open.session().is_none()
            && let RecordData::Note(note) = &open.record().data
        {
            for entry in workspace
                .writing_comments(&open.record().id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| !entry.addressed)
            {
                if let RecordData::Comment(comment) = &entry.record.data
                    && let Some(range) =
                        comment_location(comment, open.record().version, &note.markdown)
                {
                    ranges.push(
                        RangeDecoration::new(range)
                            .with_style(RangeDecorationStyle::Fill)
                            .with_color(rgb(p.comment).into()),
                    );
                }
            }
            if let Some((anchor, version, range, _)) = &self.comment_anchor
                && anchor == &open.record().id
                && *version == open.record().version
            {
                ranges.push(
                    RangeDecoration::new(range.clone())
                        .with_style(RangeDecorationStyle::Fill)
                        .with_color(rgb(p.comment_strong).into()),
                );
            }
        }
        match &self.comment_marks {
            Some(marks) => marks.set(ranges, cx),
            None => {
                self.comment_marks = Some(self.note_editor.update(cx, |editor, cx| {
                    editor.create_range_decorations_collection(ranges, cx)
                }))
            }
        }
    }
    pub fn sync_diff<T: 'static>(
        &mut self,
        workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) {
        let p = pal();
        let (text, decorations) = match self.review.as_ref() {
            Some(review) if review.note.is_some() => {
                let current = review
                    .note
                    .as_ref()
                    .and_then(|id| workspace.store.note(id).ok().flatten())
                    .and_then(|record| match record.data {
                        RecordData::Note(note) => Some(note.markdown),
                        _ => None,
                    })
                    .unwrap_or_default();
                let tracked = diff::tracked(&current, &review.original);
                let decorations = tracked
                    .marks
                    .iter()
                    .map(|(range, mark)| {
                        let style = match mark {
                            diff::Mark::Deleted => HighlightStyle {
                                background_color: Some(rgb(p.delete).into()),
                                strikethrough: Some(gpui_kit::StrikethroughStyle {
                                    thickness: px(1.),
                                    color: Some(rgb(p.ink).into()),
                                }),
                                ..Default::default()
                            },
                            diff::Mark::Inserted => HighlightStyle {
                                background_color: Some(rgb(p.add).into()),
                                underline: Some(gpui_kit::UnderlineStyle {
                                    thickness: px(1.),
                                    color: Some(rgb(p.ink).into()),
                                    wavy: false,
                                }),
                                ..Default::default()
                            },
                        };
                        TextDecoration::new(range.clone(), style)
                    })
                    .collect::<Vec<_>>();
                (tracked.text, decorations)
            }
            _ => (String::new(), vec![]),
        };
        self.diff_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        match &self.diff_marks {
            Some(marks) => marks.set(decorations, cx),
            None => {
                self.diff_marks = Some(self.diff_editor.update(cx, |editor, cx| {
                    editor.create_decorations_collection(decorations, cx)
                }))
            }
        }
    }
    pub fn track_selection<T: 'static>(&mut self, cx: &mut Context<T>) {
        let editor = self.note_editor.read(cx);
        let range = if self.note.is_reading() {
            let view = self.note_read_view.read(cx);
            if view.rendered_text().source() != editor.value().as_ref() {
                None
            } else {
                view.selected_source_range()
            }
        } else {
            Some(editor.selected_range())
        };
        let selection = range
            .filter(|range| range.start < range.end)
            .and_then(|range| {
                editor
                    .value()
                    .get(range.clone())
                    .map(|quote| (range, quote.to_owned()))
            });
        if selection != self.selection {
            self.selection = selection;
            cx.notify();
        }
    }
    pub fn sync_read_text<T: 'static>(&mut self, cx: &mut Context<T>) {
        let source = self.note_editor.read(cx).value().to_string();
        if self.read_source != source {
            // Text that extends the old would be parsed as an appended
            // fragment; the outline needs one whole parse of the document,
            // so replace it in full.
            let extends = !self.read_source.is_empty() && source.starts_with(&self.read_source);
            self.read_source = source.clone();
            self.read_marks_key.clear();
            self.selection = None;
            self.note_read_view.update(cx, |view, cx| {
                view.clear_selection(cx);
                if extends {
                    view.set_text("", cx);
                }
                view.set_text(&source, cx);
            });
        }
    }
    pub fn sync_read_marks<T: 'static>(&mut self, workspace: &Workspace, cx: &mut Context<T>) {
        if !self.note.is_reading() {
            return;
        }
        let Some(open) = self.note.as_ref() else {
            return;
        };
        let RecordData::Note(note) = &open.record().data else {
            return;
        };
        let rendered = self.note_read_view.read(cx).rendered_text();
        if rendered.source() != self.read_source {
            return;
        }
        let mut ranges = vec![];
        if open.session().is_none() && self.read_source == note.markdown {
            for entry in workspace
                .writing_comments(&open.record().id)
                .unwrap_or_default()
            {
                if !entry.addressed
                    && let RecordData::Comment(comment) = entry.record.data
                    && let Some(source) =
                        comment_location(&comment, open.record().version, &note.markdown)
                    && let Some(range) = rendered.range_for_source(source)
                {
                    ranges.push(range);
                }
            }
            if let Some((id, version, source, _)) = &self.comment_anchor
                && id == &open.record().id
                && *version == open.record().version
                && let Some(range) = rendered.range_for_source(source.clone())
            {
                ranges.push(range);
            }
        }
        let key = format!(
            "{}:{}:{}:{ranges:?}",
            open.record().id,
            open.record().version,
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
}

impl WritingPanel {
    /// Apply a supported format using the real editor's replacement/selection path.
    /// The host performs its current visibility guard and checks the resulting buffer immediately.
    pub fn format_selection<T: 'static>(
        &mut self,
        kind: formatting::Format,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> bool {
        let editor = self.note_editor.read(cx);
        let text = editor.value().to_string();
        let range = editor.selected_range();
        let Some(edit) = formatting::format(&text, range, kind) else {
            return false;
        };
        self.note_editor.update(cx, |editor, cx| {
            editor.set_selected_range(edit.range, cx);
            editor.replace(edit.replacement, window, cx);
            editor.set_selected_range(edit.selection, cx);
            editor.focus(window, cx);
        });
        true
    }
}
