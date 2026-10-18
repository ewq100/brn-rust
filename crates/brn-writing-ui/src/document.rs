//! Concrete editor and formatted Read presentation over one Markdown buffer.
use crate::native::*;
use crate::{components, formatting, layout, theme::pal};
use brn_threads_app::*;
use gpui_kit::base::{Disableable, RangeHighlight, Selectable, TextView, TextViewState};
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

/// An Outline row reads like its heading, without Markdown emphasis.
fn outline_label(title: &str) -> String {
    short(
        title
            .strip_prefix("**")
            .and_then(|value| value.strip_suffix("**"))
            .unwrap_or(title),
    )
}

pub fn readable_document(
    view: &Entity<TextViewState>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
) -> TextView {
    let p = pal();
    crate::markdown::markdown(view, assets)
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
                    button("note-write")
                        .label("Write")
                        .selected(!self.note_reading)
                        .tooltip("Edit the Markdown text")
                        .on_click(self.on(cx, WritingRequest::NoteReading(false))),
                    button("note-read")
                        .label("Read")
                        .selected(self.note_reading)
                        .tooltip("Read the formatted note; select text to comment")
                        .on_click(self.on(cx, WritingRequest::NoteReading(true))),
                ],
            )
            .into_any_element(),
        ];
        if !self.note_reading {
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
        // One Outline for Write and Read: the same headings and handler.
        let outline: Vec<(String, std::ops::Range<usize>)> =
            sections(self.note_editor.read(cx).value().as_ref())
                .into_iter()
                .map(|(title, range)| (outline_label(&title), range))
                .collect();
        let trigger = if fit.icons() {
            components::icon_button("note-outline", IconName::TableOfContents, "Outline")
        } else {
            pill("note-outline", "Outline")
                .dropdown_caret(true)
                .tooltip("Jump to a heading")
        };
        controls.push(
            trigger
                .dropdown_menu(move |menu, _, _| {
                    let mut menu = menu.scrollable(outline.len() > 12);
                    if outline.is_empty() {
                        return menu.item(item(
                            &this,
                            "No headings in this note",
                            None,
                            true,
                            false,
                            WritingRequest::Noop,
                            dispatch,
                        ));
                    }
                    for (label, range) in &outline {
                        let range = range.clone();
                        menu = menu.item(item(
                            &this,
                            label.clone(),
                            None,
                            false,
                            false,
                            WritingRequest::Reveal(range.clone()),
                            dispatch,
                        ));
                    }
                    menu
                })
                .into_any_element(),
        );
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
        if let Some(open) = &self.note
            && open.session.is_none()
            && let RecordData::Note(note) = &open.record.data
        {
            for entry in workspace
                .writing_comments(&open.record.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| !entry.addressed)
            {
                if let RecordData::Comment(comment) = &entry.record.data
                    && let Some(range) =
                        comment_location(comment, open.record.version, &note.markdown)
                {
                    ranges.push(
                        RangeDecoration::new(range)
                            .with_style(RangeDecorationStyle::Fill)
                            .with_color(rgb(p.comment).into()),
                    );
                }
            }
            if let Some((anchor, version, range, _)) = &self.comment_anchor
                && anchor == &open.record.id
                && *version == open.record.version
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
        let (text, decorations) = match &self.review {
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
        let range = if self.note_reading {
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
            self.read_source = source.clone();
            self.read_marks_key.clear();
            self.selection = None;
            self.note_read_view.update(cx, |view, cx| {
                view.clear_selection(cx);
                view.set_text(&source, cx);
            });
        }
    }
    pub fn sync_read_marks<T: 'static>(&mut self, workspace: &Workspace, cx: &mut Context<T>) {
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
            for entry in workspace
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
    pub fn note_changed(
        &mut self,
        workspace: &mut Workspace,
        text: String,
    ) -> AppResult<Option<(Tone, &'static str)>> {
        let Some(open) = &mut self.note else {
            return Ok(None);
        };
        let RecordData::Note(note) = &open.record.data else {
            return Ok(None);
        };
        if open.session.is_none() && text == note.markdown {
            return Ok(None);
        }
        if open.session.is_none() {
            open.session = Some(
                workspace
                    .store
                    .begin_edit(&open.record.id, open.record.version)?,
            );
        }
        let session = open.session.as_ref().unwrap();
        if session.markdown == text {
            return Ok(None);
        }
        Ok(Some(
            match workspace
                .store
                .update_buffer(&session.id, session.generation + 1, &text)?
            {
                BufferOutcome::Updated(updated) => {
                    open.session = Some(updated);
                    (
                        Tone::Info,
                        "Your edit is recoverable. Save to update the note.",
                    )
                }
                BufferOutcome::Ignored(_) => (
                    Tone::Warning,
                    "This recovery buffer changed elsewhere. Your visible text is retained; compare before saving.",
                ),
            },
        ))
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
