//! Document history (AC-17): the open note's accepted versions, a clearly
//! labelled read-only preview of an earlier one, and Restore through the
//! owner's ordinary checked edit. Previewing never changes the current note;
//! Restore places the earlier text in the editor as an unsaved edit, so the
//! usual Save makes it a new current version with its own receipt and Undo,
//! and every earlier version stays in history.
use super::tiles::{PAGE_WIDTH, column};
use super::*;
use brn_threads_app::RevisionSummary;
use gpui_kit::base::TextViewState;
use gpui_kit::component::button::ButtonVariants;

/// At most this many pages of 32 summaries load at once.
const MAX_PAGES: usize = 16;

pub(super) struct DocHistory {
    pub(super) note: String,
    /// Accepted versions, newest first.
    pub(super) revisions: Vec<RevisionSummary>,
    /// The version under preview and its full text.
    pub(super) preview: Option<(u64, String)>,
    view: Entity<TextViewState>,
}

impl ThreadsUi {
    /// Open the open note's history in place of the document.
    pub(super) fn open_doc_history(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.open_note_id().map(str::to_owned) else {
            return;
        };
        self.preserve_visible_note_input(cx);
        let mut revisions = Vec::new();
        let mut after = 0;
        for _ in 0..MAX_PAGES {
            match self.workspace.store.note_revisions(&id, after) {
                Ok(page) => {
                    revisions.extend(page.revisions);
                    match page.next_after {
                        Some(next) => after = next,
                        None => break,
                    }
                }
                Err(error) => {
                    self.fail(format!("Could not read this document's history: {error}"));
                    cx.notify();
                    return;
                }
            }
        }
        revisions.reverse();
        let view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        self.doc_history = Some(DocHistory {
            note: id,
            revisions,
            preview: None,
            view,
        });
        cx.notify();
    }

    /// Whether the open note's history shows in place of the document.
    pub(super) fn doc_history_open(&self) -> bool {
        self.doc_history
            .as_ref()
            .is_some_and(|history| Some(history.note.as_str()) == self.open_note_id())
    }

    pub(super) fn close_doc_history(&mut self, cx: &mut Context<Self>) {
        self.doc_history = None;
        cx.notify();
    }

    /// Preview `version` read-only; the current note is untouched.
    pub(super) fn preview_doc_version(&mut self, version: u64, cx: &mut Context<Self>) {
        let Some(history) = self.doc_history.as_mut() else {
            return;
        };
        match self.workspace.store.revision(&history.note, version) {
            Ok(Some(Record {
                data: RecordData::Note(note),
                ..
            })) => {
                history
                    .view
                    .update(cx, |view, cx| view.set_text(&note.markdown, cx));
                history.preview = Some((version, note.markdown));
            }
            Ok(_) => self.fail(format!("Version {version} is not available.")),
            Err(error) => self.fail(format!("Could not read version {version}: {error}")),
        }
        cx.notify();
    }

    /// Back from a preview to the list, or from the list to the document.
    pub(super) fn doc_history_back(&mut self, cx: &mut Context<Self>) {
        match self.doc_history.as_mut() {
            Some(history) if history.preview.is_some() => history.preview = None,
            _ => self.doc_history = None,
        }
        cx.notify();
    }

    /// Why Restore is refused now, if it is.
    pub(super) fn doc_restore_refusal(&self) -> Option<&'static str> {
        let open = self.writing.note.as_ref()?;
        if self.writing.review.is_some() {
            return Some("Finish reviewing the proposal first.");
        }
        if self.writing.note.open_edit_elsewhere().is_some() {
            return Some("Another edit of this note is open. Resolve it first.");
        }
        if open.session().is_some() {
            return Some("Save or discard your current edit first.");
        }
        None
    }

    /// Restore through the checked owner edit: the earlier text becomes an
    /// unsaved edit of the current version, which Save makes current.
    pub(super) fn restore_doc_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(why) = self.doc_restore_refusal() {
            self.warn(why);
            cx.notify();
            return;
        }
        let Some((version, markdown)) = self
            .doc_history
            .as_ref()
            .and_then(|history| history.preview.clone())
        else {
            return;
        };
        let current = self.writing.note.as_ref().map(|open| open.record().version);
        if current == Some(version) {
            self.inform(format!("Version {version} is already current."));
            cx.notify();
            return;
        }
        self.doc_history = None;
        self.set_note_reading(false, window, cx);
        self.writing.note_editor.update(cx, |editor, cx| {
            editor.set_value(markdown.clone(), window, cx)
        });
        match self.writing.note_changed(&mut self.workspace, markdown) {
            Ok(_) => self.inform(format!(
                "Version {version} is in the editor as an unsaved edit. Save (⌘S) to make it current; Discard keeps the current version."
            )),
            Err(error) => self.fail(format!(
                "Could not keep the restored text: {error}. The current version is unchanged."
            )),
        }
        cx.notify();
    }

    pub(super) fn doc_history_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(history) = self.doc_history.as_ref() else {
            return div().into_any_element();
        };
        let current = self.writing.note.as_ref().map(|open| open.record().version);
        let name = self
            .writing
            .note
            .as_ref()
            .map(|open| super::shell::one_line(&title(open.record())))
            .unwrap_or_default();
        let back_label = if history.preview.is_some() {
            "Back to versions"
        } else {
            "Back to current"
        };
        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .pb(components::tokens::SPACE_2)
            .child(
                button("doc-history-back")
                    .label(back_label)
                    .icon(IconName::ArrowLeft)
                    .ghost()
                    .compact()
                    .accessibility_label(back_label)
                    .on_click(cx.listener(|this, _, _, cx| this.doc_history_back(cx))),
            )
            .child(
                div()
                    .id("doc-history-title")
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(rgb(p.ink))
                    .aria_label(format!("History · {name}"))
                    .child(format!("History · {name}"))
                    .test_support(),
            );
        let mut view = column().child(header);
        if let Some((version, _)) = &history.preview {
            let version = *version;
            let is_current = current == Some(version);
            let label = if is_current {
                format!("Version {version} · current")
            } else {
                format!(
                    "Earlier version {version} · read-only. The current document (v{}) is unchanged.",
                    current.unwrap_or_default()
                )
            };
            let refusal = self.doc_restore_refusal();
            let mut actions = div().flex().flex_wrap().items_center().gap_2().child(
                button("doc-history-current")
                    .label("Back to current")
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| this.close_doc_history(cx))),
            );
            if !is_current {
                actions =
                    actions.child(
                        button("doc-history-restore")
                            .label(format!("Restore version {version}"))
                            .primary()
                            .disabled(refusal.is_some())
                            .tooltip(refusal.unwrap_or(
                                "Opens this text as an unsaved edit; Save makes it current",
                            ))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.restore_doc_version(window, cx)
                            })),
                    );
            }
            view = view
                .child(
                    div()
                        .id("doc-history-preview-label")
                        .px_3()
                        .py_2()
                        .rounded(components::tokens::RADIUS_CONTROL)
                        .bg(rgb(if is_current { p.tile } else { p.warning_bg }))
                        .text_color(rgb(p.ink))
                        .aria_label(label.clone())
                        .child(label)
                        .test_support(),
                )
                .child(actions.py_2())
                .child(
                    div()
                        .id("doc-history-preview")
                        .h(px(480.))
                        .min_h_0()
                        .border_1()
                        .border_color(rgb(p.hairline))
                        .rounded(components::tokens::RADIUS_CONTROL)
                        .p_3()
                        .aria_label(format!("Version {version} text"))
                        .child(brn_writing_ui::markdown::markdown(
                            &history.view,
                            self.writing_view().note_assets(),
                        ))
                        .test_support(),
                );
        } else {
            let mut list = super::inbox_page::page_group("doc-history-versions")
                .role(gpui_kit::Role::Group)
                .aria_label(format!("Versions · {}", history.revisions.len()))
                .child(super::inbox_page::group_heading(
                    "Versions",
                    Some(history.revisions.len()),
                ));
            for revision in &history.revisions {
                let version = revision.version;
                let is_current = current == Some(version);
                let reason = lists::receipt_reason(&revision.reason);
                let label = if is_current {
                    format!("Version {version} · current")
                } else {
                    format!("Version {version}")
                };
                list =
                    list.child(
                        button(format!("doc-version-{version}"))
                            .ghost()
                            .w_full()
                            .h_auto()
                            .py_1()
                            .accessibility_label(format!(
                                "{label}: {reason}, {}",
                                lists::human_time(&revision.committed_at)
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.preview_doc_version(version, cx)
                            }))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .w_full()
                                    .min_w_0()
                                    .items_start()
                                    .child(
                                        div()
                                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                                            .text_color(rgb(p.ink))
                                            .child(label),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .text_left()
                                            .text_size(SMALL_TEXT)
                                            .text_color(rgb(p.muted))
                                            .child(format!(
                                                "{reason} · {}",
                                                lists::human_time(&revision.committed_at)
                                            )),
                                    ),
                            ),
                    );
            }
            view = view
                .child(super::inbox_page::lead(
                    "doc-history-lead",
                    "Accepted versions of this document, newest first. Choose one to read it; reading never changes the current document.",
                ))
                .child(list);
        }
        super::inbox_page::page_frame(
            "doc-history-content",
            PAGE_WIDTH,
            self.arrangement.primary < 720.,
            view,
        )
        .test_support()
        .into_any_element()
    }
}
