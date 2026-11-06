//! The document canvas's own state: the docked or temporary Outline, the
//! save state the header shows, and the Read default for opened articles.
use crate::native::*;
use crate::outline::{Generation, OutlineState, OutlineTarget, Stamp, current_section};
use crate::{
    components, layout,
    theme::{metrics, pal},
};
use gpui_kit::base::{Selectable, TextViewState};
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::{
    App, Context, Entity, SharedString, TestSupportExt, Window, div, prelude::*, px, rgb,
};
use std::{cell::Cell, cell::RefCell, rc::Rc};

/// A document's save state from the actual service results (WRITE-02).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    /// The open edit holds text the note does not.
    Unsaved,
    /// A checked Save of this edit is in progress.
    Saving,
    /// The shown text is the committed note.
    Saved,
    /// The last Save of this edit failed; the edit is kept.
    Failed(String),
    /// The note changed or another edit holds it; the edit is kept.
    Conflict(String),
}
impl SaveStatus {
    /// The header label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unsaved => "● Unsaved · ⌘S",
            Self::Saving => "Saving…",
            Self::Saved => "Saved",
            Self::Failed(_) => "Save failed · Retry",
            Self::Conflict(_) => "Conflict · edit kept",
        }
    }
}

/// The last Save attempt of one edit session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveAttempt {
    pub note: String,
    pub session: String,
    pub state: AttemptState,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptState {
    Saving,
    Failed(String),
    Conflict(String),
}

impl WritingPanel {
    /// The open document's save state; `None` with no document open.
    /// Saved needs the visible text to be the committed note's bytes; text
    /// that is neither committed nor kept in an edit session never reads as
    /// Saved.
    pub fn save_status(&self, cx: &App) -> Option<SaveStatus> {
        let open = self.note.as_ref()?;
        let failure = self
            .preserve_failure
            .as_ref()
            .filter(|(note, _)| *note == open.record().id)
            .map(|(_, why)| SaveStatus::Failed(why.clone()));
        let Some(session) = open.session() else {
            let committed = match &open.record().data {
                brn_threads_app::RecordData::Note(note) => Some(note.markdown.as_str()),
                _ => None,
            };
            let visible = self.note_editor.read(cx).value();
            if committed == Some(visible.as_ref()) {
                return Some(SaveStatus::Saved);
            }
            return Some(match (failure, self.note.open_edit_elsewhere()) {
                (Some(failed), _) => failed,
                (None, Some(_)) => SaveStatus::Conflict("Another edit of this note is open".into()),
                (None, None) => SaveStatus::Unsaved,
            });
        };
        let attempt = self
            .save_attempt
            .as_ref()
            .filter(|attempt| attempt.note == open.record().id && attempt.session == session.id);
        Some(match attempt.map(|attempt| &attempt.state) {
            Some(AttemptState::Saving) => SaveStatus::Saving,
            Some(AttemptState::Failed(why)) => SaveStatus::Failed(why.clone()),
            Some(AttemptState::Conflict(why)) => SaveStatus::Conflict(why.clone()),
            None => failure.unwrap_or(SaveStatus::Unsaved),
        })
    }

    /// The next different note opens in Read: a Library article or a found
    /// note. A recovered edit still opens in Edit.
    pub fn read_next(&mut self) {
        self.prefer_reading = true;
    }

    /// The outline's content generation: the open edit buffer, or the
    /// committed version.
    fn outline_generation(&self) -> Option<(String, Generation)> {
        let open = self.note.as_ref()?;
        let generation = match open.session() {
            Some(session) => Generation::Buffer {
                session: session.id.clone(),
                generation: session.generation,
            },
            None => Generation::Committed {
                version: open.record().version,
            },
        };
        Some((open.record().id.clone(), generation))
    }

    /// The outline of the displayed text, from the reading view's committed
    /// parse of exactly that text; Updating while a parse is pending.
    pub fn outline_state(&self, cx: &App) -> OutlineState {
        let Some((document, generation)) = self.outline_generation() else {
            return OutlineState::Updating;
        };
        let rendered = self.note_read_view.read(cx).rendered_text();
        if rendered.source() != self.read_source {
            return OutlineState::Updating;
        }
        match self.read_outline.entries(&rendered) {
            Some(entries) => OutlineState::Ready {
                stamp: Stamp::new(document, generation, &self.read_source),
                entries,
            },
            None => OutlineState::Updating,
        }
    }

    /// The owner's stored Outline choice for the open document.
    pub fn outline_preferred(&self) -> bool {
        self.open_note_id()
            .and_then(|id| self.outline_shown.get(id))
            .copied()
            .unwrap_or(false)
    }

    /// Whether the outline docks beside a document region of `width` that
    /// also holds a comments margin of `margin`: the document keeps 560.
    pub fn outline_docks_at(width: f32, margin: f32) -> bool {
        width - metrics::OUTLINE - metrics::DIVIDER - margin >= metrics::DOCUMENT_REGION_MIN
    }

    /// Toolbar Outline. Docked, it flips the stored choice; otherwise it
    /// opens or closes the temporary contents view and stores nothing.
    /// Returns whether the contents view opened.
    pub fn toggle_outline(&mut self, docks: bool, window: &mut Window, cx: &mut App) -> bool {
        let Some(id) = self.open_note_id().map(str::to_owned) else {
            return false;
        };
        if docks {
            self.outline_view = false;
            let shown = !self.outline_preferred();
            self.outline_shown.insert(id, shown);
            false
        } else {
            self.outline_view = !self.outline_view;
            if self.outline_view {
                // The contents view replaces open stacked comments; their
                // draft and anchor stay for when they reopen.
                self.comments_expanded = false;
                // Without entries, focus Close so Escape still returns to the
                // document (READ-03).
                if self.outline_state(cx).entries().is_empty() {
                    self.outline_close_focus.focus(window, cx);
                } else {
                    self.focus_outline_entry(self.outline_current.get().unwrap_or(0), window, cx);
                }
            } else {
                self.outline_toggle_focus.focus(window, cx);
            }
            self.outline_view
        }
    }

    /// Close the outline. The temporary view returns focus to the toggle;
    /// a docked outline records the choice.
    pub fn close_outline(&mut self, window: &mut Window, cx: &mut App) {
        if self.outline_view {
            self.outline_view = false;
        } else if let Some(id) = self.open_note_id().map(str::to_owned) {
            self.outline_shown.insert(id, false);
        }
        self.outline_toggle_focus.focus(window, cx);
    }

    /// Go to `target` when it still names a heading of the displayed
    /// outline; otherwise refuse with the reason and navigate nowhere.
    pub fn outline_go(
        &mut self,
        target: &OutlineTarget,
        window: &mut Window,
        cx: &mut App,
    ) -> std::result::Result<(), &'static str> {
        let state = self.outline_state(cx);
        let Some(entry) = state.resolve(target).cloned() else {
            return Err(
                "That heading is no longer in the displayed text. The outline has refreshed; choose a heading again.",
            );
        };
        if self.note.is_reading() {
            if entry.nested {
                self.note_read_view.update(cx, |view, cx| {
                    let _ = view.reveal_range(entry.rendered.clone(), cx);
                });
            } else {
                self.note_read_view.update(cx, |view, cx| {
                    view.list_state().scroll_to(gpui_kit::ListOffset {
                        item_ix: entry.block,
                        offset_in_item: px(0.),
                    });
                    cx.notify();
                });
            }
            self.outline_current.set(Some(target.index));
            if self.outline_view {
                // The temporary view returns to the document at the section.
                self.outline_view = false;
                self.note_read_view
                    .read(cx)
                    .focus_handle()
                    .clone()
                    .focus(window, cx);
            }
        } else {
            // Edit: the editor's own selection places the caret at the
            // heading and scrolls it into view.
            let at = entry.source.start..entry.source.start;
            self.outline_view = false;
            self.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(at, cx);
                editor.focus(window, cx);
            });
        }
        Ok(())
    }

    /// Up/Down between entries, without wrapping.
    pub fn outline_step(&mut self, step: i32, window: &mut Window, cx: &mut App) {
        let focused = self
            .outline_entry_focus
            .borrow()
            .iter()
            .position(|handle| handle.is_focused(window));
        let count = self.outline_state(cx).entries().len();
        if count == 0 {
            return;
        }
        let next = match focused {
            Some(index) => (index as i64 + i64::from(step)).clamp(0, count as i64 - 1) as usize,
            None => 0,
        };
        self.focus_outline_entry(next, window, cx);
    }

    fn focus_outline_entry(&self, index: usize, window: &mut Window, cx: &mut App) {
        let mut handles = self.outline_entry_focus.borrow_mut();
        while handles.len() <= index {
            handles.push(cx.focus_handle());
        }
        handles[index].focus(window, cx);
    }
}

/// Keep the current section in step with the reading list's scrolling, and
/// re-render the host only when it changes. Focus never moves.
pub fn follow_reading_position<T: 'static>(
    view: &Entity<TextViewState>,
    current: Rc<Cell<Option<usize>>>,
    blocks: Rc<RefCell<Vec<usize>>>,
    cx: &mut Context<T>,
) {
    let host = cx.entity().downgrade();
    view.read(cx)
        .list_state()
        .set_scroll_handler(move |event, _, cx| {
            let next = current_section(&blocks.borrow(), event.visible_range.start);
            if next != current.get() {
                current.set(next);
                let _ = host.update(cx, |_, cx| cx.notify());
            }
        });
}

impl<T: 'static> WritingView<'_, T> {
    /// The outline docks when the document keeps its 560 beside it.
    pub fn outline_docks(&self) -> bool {
        let margin = if self.comments_beside() {
            self.margin_mode.width()
        } else {
            0.
        };
        WritingPanel::outline_docks_at(self.note_width, margin)
    }

    /// Whether an outline region (docked or temporary) shows now.
    pub fn outline_showing(&self) -> bool {
        self.review.is_none()
            && self.note.is_some()
            && if self.outline_docks() {
                self.outline_preferred()
            } else {
                self.outline_view
            }
    }

    /// The toolbar's labelled Outline toggle.
    pub fn outline_toggle(
        &self,
        fit: layout::NoteFit,
        cx: &mut Context<T>,
    ) -> gpui_kit::component::button::Button {
        let shown = self.outline_showing();
        let name = if shown { "Hide outline" } else { "Outline" };
        let toggle = if fit.icons() {
            components::icon_button("note-outline", IconName::TableOfContents, name)
        } else {
            pill("note-outline", "Outline")
                .icon(IconName::TableOfContents)
                .accessibility_label(name)
        };
        toggle
            .track_focus(&self.outline_toggle_focus)
            .selected(shown)
            .tooltip(if shown {
                "Hide the document's headings"
            } else {
                "Show the document's headings"
            })
            .on_click(self.on(cx, WritingRequest::ToggleOutline))
    }

    /// The Outline region: a docked column beside the document, or the
    /// temporary contents view in its place when the document cannot keep
    /// 560 beside it. Entries come from the reading view's own parse.
    pub fn outline_region(&self, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        let p = pal();
        let docked = self.outline_docks();
        let state = self.panel.outline_state(cx);
        let entries = state.entries();
        *self.outline_blocks.borrow_mut() = entries.iter().map(|entry| entry.block).collect();
        let current = if self.note.is_reading() {
            let top = self
                .note_read_view
                .read(cx)
                .list_state()
                .logical_scroll_top()
                .item_ix;
            let current = current_section(&self.outline_blocks.borrow(), top);
            self.outline_current.set(current);
            current
        } else {
            None
        };
        {
            let mut handles = self.outline_entry_focus.borrow_mut();
            while handles.len() < entries.len() {
                handles.push(cx.focus_handle());
            }
        }
        let shallowest = entries.iter().map(|entry| entry.level).min().unwrap_or(1);
        let mut list = div()
            .id("note-outline-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .gap(px(2.))
            .px_2()
            .pb_2();
        match &state {
            OutlineState::Updating => {
                list = list.child(
                    div()
                        .id("note-outline-updating")
                        .px_2()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child("Updating outline…")
                        .test_support(),
                );
            }
            OutlineState::Ready { .. } if entries.is_empty() => {
                list = list.child(
                    div()
                        .id("note-outline-empty")
                        .px_2()
                        .text_size(UI_TEXT)
                        .text_color(rgb(p.muted))
                        .aria_label("No headings yet")
                        .child("No headings yet")
                        .test_support(),
                );
            }
            OutlineState::Ready { .. } => {
                let handles = self.outline_entry_focus.borrow();
                for (index, entry) in entries.iter().enumerate() {
                    let Some(target) = state.target(index) else {
                        continue;
                    };
                    let is_current = current == Some(index);
                    let depth = entry.level.saturating_sub(shallowest).min(4);
                    let announced = format!(
                        "{}, heading level {}{}",
                        entry.label,
                        entry.level,
                        if is_current { ", current section" } else { "" }
                    );
                    list = list.child(
                        button(format!("outline-entry-{index}"))
                            .ghost()
                            .compact()
                            .w_full()
                            .h_auto()
                            .py_1()
                            .pl(px(8. + 12. * f32::from(depth)))
                            .track_focus(&handles[index])
                            .selected(is_current)
                            .accessibility_label(announced)
                            .on_click(self.on(cx, WritingRequest::OutlineGo(target)))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_1()
                                    .w_full()
                                    .min_w_0()
                                    // The current section has a bar and a
                                    // heavier label, not color alone.
                                    .child(
                                        div()
                                            .w(px(2.))
                                            .h(px(16.))
                                            .mt(px(2.))
                                            .flex_shrink_0()
                                            .rounded(px(1.))
                                            .when(is_current, |bar| bar.bg(rgb(p.ink))),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .whitespace_normal()
                                            .text_left()
                                            .text_size(px(metrics::TEXT_LABEL + 1.))
                                            .line_height(gpui_kit::relative(1.35))
                                            .text_color(rgb(
                                                if is_current || entry.level <= shallowest {
                                                    p.ink
                                                } else {
                                                    p.muted
                                                },
                                            ))
                                            .when(is_current, |label| {
                                                label.font_weight(gpui_kit::FontWeight::MEDIUM)
                                            })
                                            .child(entry.label.clone()),
                                    ),
                            ),
                    );
                }
            }
        }
        let dispatch = self.dispatch;
        let host = cx.entity().downgrade();
        let keys = move |event: &gpui_kit::KeyDownEvent, window: &mut Window, cx: &mut App| {
            let request = match event.keystroke.key.as_str() {
                "escape" => WritingRequest::CloseOutline,
                "down" => WritingRequest::OutlineStep(1),
                "up" => WritingRequest::OutlineStep(-1),
                _ => return,
            };
            let _ = host.update(cx, |host, cx| {
                dispatch(host, request, window, cx);
                cx.stop_propagation();
            });
        };
        let label = if self.note.is_reading() {
            "Outline"
        } else {
            "Outline · Edit"
        };
        let header = div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(36.))
            .px_3()
            .flex_shrink_0()
            .child(
                div()
                    .flex_1()
                    .text_size(SMALL_TEXT)
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(p.muted))
                    .child(label),
            )
            .child(
                components::icon_button("note-outline-close", IconName::Close, "Close outline")
                    .track_focus(&self.outline_close_focus)
                    .tooltip(if docked {
                        "Close outline"
                    } else {
                        "Back to the document · Esc"
                    })
                    .on_click(self.on(cx, WritingRequest::CloseOutline)),
            );
        let region = div()
            .id(if docked {
                "note-outline-panel"
            } else {
                "note-outline-view"
            })
            .flex()
            .flex_col()
            .h_full()
            .min_h_0()
            .font_family(UI_FONT)
            .on_key_down(keys)
            .aria_label(SharedString::from(if docked {
                "Outline"
            } else {
                "Outline, contents view"
            }))
            .child(header)
            .child(list);
        if docked {
            region
                .w(px(metrics::OUTLINE))
                .flex_shrink_0()
                .border_r_1()
                .border_color(rgb(p.hairline))
                .test_support()
                .into_any_element()
        } else {
            region
                .flex_1()
                .min_w_0()
                .bg(rgb(p.tile))
                .test_support()
                .into_any_element()
        }
    }
}
