//! Current Note/comments/review/Compare presentation, extracted without a second workflow.
use crate::markdown::markdown;
use crate::native::*;
use crate::{
    components, diff, layout,
    theme::pal,
    workbench::{RunOutcome, WritingRow},
};
use brn_threads_app::*;
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::{Context, SharedString, TestSupportExt, Window, div, prelude::*, px, rgb};
use std::collections::BTreeSet;
impl<T: 'static> WritingView<'_, T> {
    /// What each Note toolbar arrangement needs for the open note: each
    /// label's measured text plus its control's fixed chrome, as measured on
    /// the toolbar (Write 16 and Read 17 pt, Format ▾ and Outline 41, the
    /// status chip 38, the comment toggle 40 with a count or 24 without,
    /// Edit note and Connections 17), 50 pt of gaps, padding, spacer and
    /// borders, and 8 pt of slack. The full row's five 24 pt formatting
    /// buttons and their gaps (136 pt) replace Format ▾.
    pub fn note_toolbar_needs(&self, window: &Window) -> layout::NoteNeeds {
        let Some(open) = self.note.as_ref() else {
            return layout::NoteNeeds::WIDEST;
        };
        let RecordData::Note(note) = &open.record().data else {
            return layout::NoteNeeds::WIDEST;
        };
        let text = |label: &str, size| text_width(window, label, size);
        let kind = if note.protected {
            "Protected"
        } else {
            "Working"
        };
        let chip = text(&format!("{kind} · v{}", open.record().version), SMALL_TEXT) + 38.;
        let pending = self.pending_comments(&open.record().id);
        let toggle = if pending > 0 {
            text(&pending.to_string(), UI_TEXT) + 40.
        } else {
            24.
        };
        let format = text("Format", UI_TEXT) + 41.;
        let edit = text("Edit note", UI_TEXT) + 17.;
        let connections = text("Connections", UI_TEXT) + 17.;
        let format_menu = 50.
            + 8.
            + text("Write", UI_TEXT)
            + 16.
            + text("Read", UI_TEXT)
            + 17.
            + format
            + text("Outline", UI_TEXT)
            + 41.
            + chip
            + toggle
            + edit
            + connections;
        layout::NoteNeeds {
            full: format_menu - format + 136.,
            format_menu,
            compact: format_menu - edit - connections - 8.,
        }
    }

    /// The Note toolbar, one row: Write | Read, formatting and Outline on the
    /// left; the status chip, comments, Edit note and Connections on the
    /// right. `note_fit` folds controls into menus as the panel narrows.
    pub fn note_controls(&self, cx: &mut Context<T>) -> Vec<gpui_kit::AnyElement> {
        let Some(open) = self.note.as_ref() else {
            return vec![];
        };
        let RecordData::Note(note) = &open.record().data else {
            return vec![];
        };
        // During review the review bar replaces the toolbar; its Proposal
        // label and Protected chip state the review.
        if self.review.is_some() {
            return vec![];
        }
        let fit = self.note_fit;
        let mut controls = self.document_controls(fit, cx);
        controls.push(div().flex_1().min_w(px(4.)).into_any_element());
        // Protection and version stay visible in every state, dirty included.
        let (tone, icon, kind) = if note.protected {
            (Some(Tone::Info), IconName::Lock, "Protected")
        } else {
            (None, IconName::FileText, "Working")
        };
        let version = format!("v{}", open.record().version);
        let status = format!("{kind} · {version}");
        controls.push(
            if fit.icons() {
                components::short_status_chip("note-status-chip", tone, Some(icon), version, status)
            } else {
                components::status_chip("note-status-chip", tone, Some(icon), status)
            }
            .into_any_element(),
        );
        controls.extend(self.comments_toggle(cx));
        if !fit.actions_in_menu() {
            let blocked = self.edit_note_blocked();
            controls.push(
                pill("edit-note-context", "Edit note")
                    .disabled(blocked.is_some())
                    .accessibility_label(match blocked {
                        Some(reason) => format!("Edit note, unavailable: {reason}"),
                        None => "Edit note".into(),
                    })
                    .tooltip(match blocked {
                        Some(reason) => format!("Edit note is unavailable: {reason}"),
                        None => "Give BRN an instruction for this saved note".into(),
                    })
                    .on_click(self.on(cx, WritingRequest::EditNote))
                    .into_any_element(),
            );
            controls.push(
                pill("note-connections", "Connections")
                    .tooltip("Linked notes and sources of this note")
                    .on_click(self.on(cx, WritingRequest::Connections))
                    .into_any_element(),
            );
        }
        controls
    }

    /// The Note ⋯'s own items: the review's during review, otherwise the
    /// open note's. The panel header renders the ⋯ (`note-menu`) and adds its
    /// Panel section.
    pub fn note_menu_section(
        &self,
        cx: &mut Context<T>,
    ) -> Option<Box<dyn Fn(PopupMenu) -> PopupMenu + 'static>> {
        if let Some(review) = self.review.as_ref() {
            return Some(Box::new(self.review_menu_section(review, cx)));
        }
        let open = self.note.as_ref()?;
        let RecordData::Note(note) = &open.record().data else {
            return None;
        };
        let this = cx.entity().downgrade();
        let dispatch = self.dispatch;
        let dirty = open.session().is_some();
        let busy = self.worker.is_some();
        let imported = note.import.is_some();
        let protected = note.protected;
        // A narrow Note keeps Edit note and Connections here instead.
        let collapsed = self.note_fit.actions_in_menu();
        let edit_blocked = self.edit_note_blocked();
        Some(Box::new(move |menu: PopupMenu| {
            let mut menu = menu
                .item(item(
                    &this,
                    "Save",
                    Some(Box::new(Save)),
                    !dirty,
                    false,
                    WritingRequest::Save,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Reload saved note",
                    None,
                    dirty,
                    false,
                    WritingRequest::Reload,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Save text as a new note",
                    None,
                    !dirty,
                    false,
                    WritingRequest::SaveCopy,
                    dispatch,
                ))
                .separator()
                .item(item(
                    &this,
                    if dirty {
                        "Comment on selection · save first"
                    } else {
                        "Comment on selection"
                    },
                    Some(Box::new(CommentSelection)),
                    dirty,
                    false,
                    WritingRequest::CommentSelection,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Rewrite with AI",
                    Some(Box::new(Rewrite)),
                    dirty || busy,
                    false,
                    WritingRequest::Rewrite,
                    dispatch,
                ));
            if collapsed {
                menu = menu
                    .item(item(
                        &this,
                        match edit_blocked {
                            Some(reason) => format!("Edit note… · {reason}"),
                            None => "Edit note…".into(),
                        },
                        None,
                        edit_blocked.is_some(),
                        false,
                        WritingRequest::EditNote,
                        dispatch,
                    ))
                    .item(item(
                        &this,
                        "Connections",
                        None,
                        false,
                        false,
                        WritingRequest::Connections,
                        dispatch,
                    ));
            }
            let mut menu = menu
                .separator()
                .item(item(
                    &this,
                    "Brief & references…",
                    None,
                    false,
                    false,
                    WritingRequest::OpenSetup,
                    dispatch,
                ))
                .item(item(
                    &this,
                    if protected {
                        "Make working note"
                    } else {
                        "Protect note"
                    },
                    None,
                    dirty,
                    false,
                    WritingRequest::ToggleProtection,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Export Markdown snapshot…",
                    None,
                    busy,
                    false,
                    WritingRequest::Export,
                    dispatch,
                ));
            if imported {
                menu = menu.item(item(
                    &this,
                    "Coverage details",
                    None,
                    false,
                    false,
                    WritingRequest::ToggleCoverage,
                    dispatch,
                ));
            }
            // The destructive item comes last and confirms in a dialog.
            menu.separator()
                .item(item(
                    &this,
                    "New chat for this note",
                    None,
                    false,
                    false,
                    WritingRequest::NewWritingThread,
                    dispatch,
                ))
                .separator()
                .item(item(
                    &this,
                    "Discard edit…",
                    None,
                    !dirty,
                    false,
                    WritingRequest::ConfirmDiscard,
                    dispatch,
                ))
        }))
    }

    /// The Note ⋯ during review: the side-by-side and editing routes and the
    /// saved inputs, then Keep for later and Dismiss when the review bar is
    /// too narrow to show them.
    fn review_menu_section(
        &self,
        review: &Review,
        cx: &mut Context<T>,
    ) -> impl Fn(PopupMenu) -> PopupMenu + 'static + use<T> {
        let this = cx.entity().downgrade();
        let dispatch = self.dispatch;
        let note = review.note.is_some();
        let editing = self.review.is_editing();
        let inputs = self.review.inputs_shown() || !note;
        let collapsed = self.review_fit.actions_in_menu();
        move |menu: PopupMenu| {
            let menu = menu
                .item(item(
                    &this,
                    "Compare side by side",
                    None,
                    !note,
                    false,
                    WritingRequest::Compare,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Edit final text",
                    None,
                    !note || editing,
                    false,
                    WritingRequest::StartReviewEdit,
                    dispatch,
                ))
                // A proposal without a note always shows its inputs.
                .item(item(
                    &this,
                    "Inputs",
                    None,
                    !note,
                    inputs,
                    WritingRequest::ToggleReviewInputs,
                    dispatch,
                ));
            if !collapsed {
                return menu;
            }
            menu.separator()
                .item(item(
                    &this,
                    "Keep for later",
                    None,
                    false,
                    false,
                    WritingRequest::KeepReview,
                    dispatch,
                ))
                .item(item(
                    &this,
                    "Dismiss",
                    None,
                    false,
                    false,
                    WritingRequest::DismissReview,
                    dispatch,
                ))
        }
    }

    /// The other open edit of the open note (U4), found by its note: the
    /// typed error's session is never used.
    pub fn open_edit_elsewhere(&self) -> Option<(EditSession, u64)> {
        let open = self.note.as_ref()?;
        let (note, base) = self.note.open_edit_elsewhere()?;
        if note != open.record().id || !self.recovery_blocked() {
            return None;
        }
        let buffer = self.buffers.iter().find(|s| s.note == note)?;
        Some((buffer.clone(), base))
    }

    pub fn note_tile(&self, window: &mut Window, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        if self.review.is_some() {
            return self.review_body(cx);
        }
        let p = pal();
        let Some(open) = self.note.as_ref() else {
            return div()
                .id("note-empty-state")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(components::empty_state(
                    "note-empty",
                    IconName::FileText,
                    "No note open",
                    "Open one from Notes or ⌘K.",
                    Some(
                        button("new-note")
                            .label("New note")
                            .outline()
                            .tooltip("New note (⌘N)")
                            .on_click(self.on(cx, WritingRequest::NewNote)),
                    ),
                ))
                .into_any_element();
        };
        let _ = window;
        let dirty = open.session().is_some();
        let blocked = self.recovery_blocked();
        let mut content = div()
            .id("note-tile")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0();
        // Banners sit between the toolbar and the text and never move into a
        // menu; hairlines separate adjacent ones. In a short Note the
        // saved-version preview gives up height first; past that the banners
        // after the first scroll inside their own box, which keeps
        // BANNER_FLOOR, so no banner is painted over the text, the text
        // keeps its minimum and the writing bar stays in the Note.
        let mut banners = self.note_banners(cx).into_iter();
        let banner_shown = banners.len() > 0;
        if let Some(lead) = banners.next() {
            // The first banner (a kept edit, or the stale edit with its
            // Save as new note and Open current version) never shrinks, so
            // its actions stay in view. The rest share a box below it that
            // keeps its first line (the saved version's label, or the next
            // banner's opening line) and scrolls.
            // `note-banners` opens the stack with the lead banner;
            // `note-banners-more` is the shared box below it.
            let rest: Vec<_> = banners.collect();
            let more = !rest.is_empty();
            content = content
                .child(
                    div()
                        .id("note-banners")
                        .flex()
                        .flex_col()
                        .flex_shrink(1.)
                        .min_h(components::BANNER_PINNED_MIN)
                        .bg(rgb(p.hairline))
                        .border_b_1()
                        .border_color(rgb(p.hairline))
                        .child(lead)
                        .test_support(),
                )
                // The saved version and later banners give way first.
                .when(more, |content| {
                    content.child(
                        div()
                            .id("note-banners-more")
                            .flex()
                            .flex_col()
                            .flex_shrink(YIELD_FIRST)
                            .min_h(BANNER_FLOOR)
                            .overflow_y_scroll()
                            .gap(px(1.))
                            .bg(rgb(p.hairline))
                            .border_b_1()
                            .border_color(rgb(p.hairline))
                            .children(rest)
                            .test_support(),
                    )
                });
        }
        if let Some((_, quote)) = &self.selection
            && !blocked
        {
            let mut strip = div()
                .id("selection-toolbar")
                .flex()
                .flex_wrap()
                .flex_shrink_0()
                .items_center()
                .gap_1()
                .min_h(components::tokens::TOOLBAR)
                .px_3()
                .py_1()
                .bg(rgb(p.alt))
                .border_b_1()
                .border_color(rgb(p.hairline))
                .font_family(UI_FONT)
                .text_size(SMALL_TEXT)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(rgb(p.muted))
                        .child(format!("“{}”", short(quote))),
                )
                .child(
                    pill(
                        "selection-comment",
                        if dirty {
                            "Save and comment"
                        } else {
                            "Comment ⌘/"
                        },
                    )
                    .tooltip(if dirty {
                        "Save this edit, then comment on the same passage"
                    } else {
                        "Comment on this passage"
                    })
                    .on_click(self.on(cx, WritingRequest::CommentOrSave)),
                )
                .child(
                    pill("selection-ask", "Ask in chat")
                        .on_click(self.on(cx, WritingRequest::AskSelection)),
                );
            if dirty {
                let reason =
                    "Save and comment saves your edit first, then comments on this passage.";
                strip = strip.child(
                    div()
                        .id("selection-comment-reason")
                        .w_full()
                        .text_color(rgb(p.muted))
                        .aria_label(reason)
                        .child(reason)
                        .test_support(),
                );
            }
            content = content.child(strip.test_support());
        }
        let editor = if self.note.is_reading() {
            crate::document::readable_document(&self.note_read_view, self.note_assets())
                .into_any_element()
        } else {
            prose_editor(&self.note_editor)
                .h_full()
                .readonly(blocked)
                .aria_label("Note text")
                .context_menu(move |menu, _, _| {
                    menu.menu_with_disabled(
                        if dirty {
                            "Comment on selection · save first"
                        } else {
                            "Comment on selection"
                        },
                        dirty,
                        Box::new(CommentSelection),
                    )
                    .menu("Ask in chat", Box::new(AskAboutSelection))
                    .separator()
                    .menu_with_disabled("Cut", blocked, Box::new(gpui_kit::base::input::Cut))
                    .menu("Copy", Box::new(gpui_kit::base::input::Copy))
                    .menu_with_disabled("Paste", blocked, Box::new(gpui_kit::base::input::Paste))
                    .separator()
                    .menu("Select all", Box::new(gpui_kit::base::input::SelectAll))
                })
                .into_any_element()
        };
        let centered = self.centered;
        // The text keeps a few lines however many banners a short Note shows,
        // and 120 px beside an open comments section. The section asks for
        // its floor after the reply below gives way, and gives way first,
        // before the text. Banners are never hidden to meet that 120 px:
        // with a banner shown the text still asks for 120 px and gives way
        // last, below it only when the rows that never hide (the banners'
        // actions, a failed run's reason, the reply's label and the whole
        // writing bar) leave less, down to its few lines.
        // An open stacked draft keeps its whole input, so beside it the text
        // gives way to its few lines too and the writing bar stays whole
        // (R1, 900×600).
        let stacked = self.comments_stacked();
        let text_floor = text_min(stacked && !banner_shown && !self.comment_drafting());
        let text_prefer = text_min(stacked);
        // A shrink factor of 1 beside the section's YIELD_FIRST: the
        // section gives way first, and once it is at its floor the text
        // takes up the whole remaining overflow (a factor below 1 would
        // leave part of it painted over the rows below).
        let mut body = div()
            .flex()
            .flex_grow(1.)
            .flex_basis(if stacked { text_prefer } else { px(0.) })
            .flex_shrink(1.)
            .min_h(text_floor)
            .child(
                div()
                    .id("note-body")
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .when(centered, |body| body.justify_center())
                    .child(
                        reading_column()
                            .id("note-column")
                            .child(editor)
                            .test_support(),
                    )
                    .on_mouse_up(
                        gpui_kit::MouseButton::Left,
                        self.on(cx, WritingRequest::TrackSelection),
                    )
                    .on_key_up(self.on(cx, WritingRequest::TrackSelection))
                    .test_support(),
            );
        if self.comments_beside() {
            body = body.child(self.margin_column(cx));
        }
        // Below the text: comments, then the run's status and reply, then the
        // writing bar. An open draft keeps its whole input: the area's floor
        // holds the text box plus the draft, and the rows around give way
        // first.
        let (prefer, min) = if stacked {
            let floor = self.stacked_floor();
            let min = if self.comment_drafting() {
                text_floor + floor
            } else {
                text_floor
            };
            (text_prefer + floor, min)
        } else {
            (px(0.), text_floor)
        };
        content
            .child(self.setup_drawer(cx))
            .child(self.text_and_comments(body, prefer, min, stacked, cx))
            .child(self.writing_activity(cx))
            .child(self.writing_run_status())
            .child(self.writing_proposals(cx))
            .child(self.writing_result())
            .children(self.note_bar(cx))
            .test_support()
            .into_any_element()
    }

    /// The Note's banners, top to bottom: a kept edit, the edit's stale base
    /// (with the saved version) or a newer saved version, and the import
    /// summary with its coverage details.
    fn note_banners(&self, cx: &mut Context<T>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let mut banners: Vec<gpui_kit::AnyElement> = vec![];
        // The first banner leads the stack: its actions never leave view.
        let banner = |lead: bool,
                      id: &'static str,
                      tone: Tone,
                      text: SharedString,
                      actions: Vec<gpui_kit::AnyElement>|
         -> gpui_kit::AnyElement {
            if lead {
                components::banner_pinned(id, tone, text, actions).into_any_element()
            } else {
                components::banner(id, tone, text, actions, None).into_any_element()
            }
        };
        let Some(open) = self.note.as_ref() else {
            return banners;
        };
        let RecordData::Note(note) = &open.record().data else {
            return banners;
        };
        if let Some((buffer, base)) = self.open_edit_elsewhere() {
            // Starting this view's edit found another open edit (U4): name
            // its base, keep it, and offer Compare and Discard beside the
            // usual Recover edit.
            let id = buffer.note.clone();
            banners.push(banner(
                banners.is_empty(),
                "note-edit-open-banner",
                Tone::Warning,
                format!(
                    "An edit of this note is already open from v{base} and is kept. What you typed here isn't saved. Compare it with the saved note, recover it, or discard it."
                )
                .into(),
                vec![
                    button("recover-note-edit")
                        .label("Recover edit")
                        .compact()
                        .primary()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip("Reopen the open edit here so you can save or discard it")
                        .on_click(self.on(cx, WritingRequest::OpenNote { id: id.clone(), recovery: Some(buffer.clone()) }))
                        .into_any_element(),
                    banner_button("note-edit-open-compare", "Compare")
                        .tooltip("Show the open edit beside the saved note")
                        .on_click(self.on(cx, WritingRequest::CompareOpenEdit))
                        .into_any_element(),
                    banner_button("note-edit-open-discard", "Discard edit…")
                        .tooltip("Discard the open edit; what you typed here is kept")
                        .on_click(self.on(cx, WritingRequest::ConfirmDiscardOpenEdit))
                        .into_any_element(),
                ],
            ));
        } else if self.recovery_blocked()
            && let Some(buffer) = self.buffers.iter().find(|s| s.note == open.record().id)
        {
            let (buffer, id) = (buffer.clone(), buffer.note.clone());
            banners.push(banner(
                banners.is_empty(),
                "note-recover-banner",
                Tone::Warning,
                "An unsaved edit was kept. This note is read-only until you recover it.".into(),
                // The kept edit blocks the note, so recovering it is the
                // panel's one filled primary.
                vec![
                    button("recover-note-edit")
                        .label("Recover edit")
                        .compact()
                        .primary()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip("Reopen the kept edit so you can save or discard it")
                        .on_click(self.on(
                            cx,
                            WritingRequest::OpenNote {
                                id: id.clone(),
                                recovery: Some(buffer.clone()),
                            },
                        ))
                        .into_any_element(),
                ],
            ));
        }
        let id = open.record().id.clone();
        let current = move |cx: &mut Context<T>| {
            let id = id.clone();
            banner_button("show-current-note", "Open current version")
                .on_click(self.on(
                    cx,
                    WritingRequest::OpenNote {
                        id: id.clone(),
                        recovery: None,
                    },
                ))
                .into_any_element()
        };
        // An open edit usually makes both version facts true; the stale-edit
        // banner states both, so Open current version renders once.
        if self.edit_is_stale() {
            banners.push(banner(
                banners.is_empty(),
                "note-stale-banner",
                Tone::Warning,
                "This note changed since your edit began. Your text is kept. Compare it with the saved version below, or save your text as a new note.".into(),
                vec![
                    button("save-buffer-copy")
                        .label("Save as new note")
                        .compact()
                        .primary()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip("Save your text as a separate working note")
                        .on_click(self.on(cx, WritingRequest::SaveCopy))
                        .into_any_element(),
                    current(cx),
                ],
            ));
            banners.push(
                div()
                    .id("note-stale-preview")
                    .flex()
                    .flex_col()
                    .h(px(204.))
                    .min_h(px(84.))
                    .flex_shrink(1.)
                    .bg(rgb(p.alt))
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .font_family(UI_FONT)
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .border_b_1()
                            .border_color(rgb(p.hairline))
                            .child("Saved version"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .px_3()
                            .child(markdown(&self.current_view, self.note_assets())),
                    )
                    .test_support()
                    .into_any_element(),
            );
        } else if self
            .records
            .iter()
            .any(|r| r.id == open.record().id && r.version != open.record().version)
        {
            banners.push(banner(
                banners.is_empty(),
                "note-newer-banner",
                Tone::Warning,
                "A newer version of this note is saved. Your visible text is kept.".into(),
                vec![current(cx)],
            ));
        }
        if let Some(coverage) = &note.import {
            let intent = if coverage.full_note {
                "Full imported reference"
            } else {
                "Useful information"
            };
            // Info only when the report records full coverage.
            let (summary, details, checked) = if let Some(report) = &coverage.report {
                (
                    report.summary(),
                    import_report_details(report),
                    report.status == ImportStatus::Complete
                        && report.issues.iter().all(|issue| {
                            !matches!(
                                issue.kind,
                                ImportIssueKind::Omission | ImportIssueKind::Unverified
                            )
                        }),
                )
            } else {
                (
                    if coverage.gaps.is_empty() {
                        "Coverage not recorded"
                    } else {
                        "Partial / coverage requires inspection"
                    },
                    coverage.gaps.clone(),
                    false,
                )
            };
            let mut actions = vec![];
            if let Some(record) = self.records.iter().find(|r| r.id == coverage.source)
                && let RecordData::Source(source) = &record.data
            {
                let locator = source.locator.clone();
                actions.push(
                    banner_button("open-import-original", "Open original")
                        .on_click(move |_, _, cx| {
                            if std::path::Path::new(&locator).is_absolute() {
                                cx.open_with_system(std::path::Path::new(&locator));
                            } else if locator.starts_with("https://")
                                || locator.starts_with("http://")
                            {
                                cx.open_url(&locator);
                            }
                        })
                        .into_any_element(),
                );
            }
            if !details.is_empty() {
                actions.push(
                    pill(
                        "toggle-coverage",
                        format!("Coverage details ({})", details.len()),
                    )
                    .selected(self.show_coverage)
                    .on_click(self.on(cx, WritingRequest::ToggleCoverage))
                    .into_any_element(),
                );
            }
            banners.push(banner(
                banners.is_empty(),
                "note-import-summary",
                if checked { Tone::Info } else { Tone::Warning },
                format!("{intent} · {summary}").into(),
                actions,
            ));
            if self.show_coverage && !details.is_empty() {
                // Provenance may contain long paths and many qualifications.
                // Keep every issue scrollable without consuming the note body.
                banners.push(
                    div()
                        .id("note-coverage-details")
                        .w_full()
                        .max_h(px(120.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .bg(rgb(p.alt))
                        .px_3()
                        .py_2()
                        .font_family(UI_FONT)
                        .child(
                            div()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(p.muted))
                                .child(format!("Coverage details ({})", details.len())),
                        )
                        .children(details.into_iter().enumerate().map(|(index, detail)| {
                            div()
                                .id(format!("note-coverage-detail-{index}"))
                                .flex_shrink_0()
                                .text_size(UI_TEXT)
                                .child(detail)
                                .test_support()
                        }))
                        .test_support()
                        .into_any_element(),
                );
            }
        }
        banners
    }

    /// The note whose comments the Note shows: the open note, unless a
    /// review shows another note's text.
    fn comments_note(&self) -> Option<&OpenNote> {
        let open = self.note.as_ref()?;
        match self.review.as_ref() {
            Some(review) if review.note.as_deref() != Some(open.record().id.as_str()) => None,
            _ => Some(open),
        }
    }

    /// A draft, a comment or a load error to show for that note.
    fn comments_have_content(&self) -> bool {
        self.comments_note().is_some_and(|open| {
            self.comment_anchor
                .as_ref()
                .is_some_and(|(note, ..)| note == &open.record().id)
                || self
                    .workspace
                    .writing_comments(&open.record().id)
                    .map_or(true, |comments| !comments.is_empty())
        })
    }

    /// Whether the comments are open where they sit now. Beside the text
    /// they follow the note until the toggle decides; the stacked section
    /// stays collapsed until opened.
    pub fn comments_open(&self) -> bool {
        if self.margin_mode == layout::MarginMode::Stacked {
            self.comments_expanded
        } else {
            self.margin.unwrap_or_else(|| self.comments_have_content())
        }
    }

    /// The margin renders beside the text.
    pub fn comments_beside(&self) -> bool {
        self.margin_mode != layout::MarginMode::Stacked
            && self.comments_note().is_some()
            && self.comments_open()
    }

    /// The in-tile section renders below the text.
    pub fn comments_stacked(&self) -> bool {
        self.margin_mode == layout::MarginMode::Stacked
            && self.comments_note().is_some()
            && self.comments_expanded
    }

    /// The open stacked section's floor: the label and a card's first
    /// line, or the whole draft while one is open.
    fn stacked_floor(&self) -> gpui_kit::Pixels {
        if self.comment_drafting() {
            STACKED_DRAFT_FLOOR
        } else {
            COMMENTS_FLOOR
        }
    }

    /// The comment toggle with its open count: the same control, handler and
    /// state in the Note toolbar and the review bar, which never render
    /// together.
    fn comments_toggle(&self, cx: &mut Context<T>) -> Option<gpui_kit::AnyElement> {
        let open = self.comments_note()?;
        let pending = self.pending_comments(&open.record().id);
        let label = if pending > 0 {
            format!("Comments · {pending}")
        } else {
            "Comments".into()
        };
        let shown = self.comments_open();
        let reviewing = self.review.is_some();
        Some(
            if pending > 0 {
                pill("toggle-margin", pending.to_string())
                    .icon(IconName::MessageSquare)
                    .accessibility_label(label)
            } else {
                components::icon_button("toggle-margin", IconName::MessageSquare, label)
            }
            .selected(shown)
            .tooltip(match (shown, reviewing) {
                (true, true) => "Hide comments",
                (false, true) => "Show comments",
                (true, false) => "Hide comments · select text to comment (⌘/)",
                (false, false) => "Show comments · select text to comment (⌘/)",
            })
            .on_click(self.on(cx, WritingRequest::ToggleComments))
            .into_any_element(),
        )
    }

    fn pending_comments(&self, note: &str) -> usize {
        self.workspace
            .writing_comments(note)
            .map(|comments| comments.iter().filter(|entry| !entry.addressed).count())
            .unwrap_or_default()
    }

    /// "Comments · N", 12 px, above the list in both placements.
    fn comments_label(&self, note: &str) -> gpui_kit::AnyElement {
        let pending = self.pending_comments(note);
        div()
            .flex_shrink_0()
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .text_color(rgb(pal().muted))
            .child(if pending > 0 {
                format!("Comments · {pending}")
            } else {
                "Comments".into()
            })
            .into_any_element()
    }

    /// Text above, comments below. The area asks for `prefer` (the text's
    /// 120 px plus the open section's floor) and gives way only after the
    /// rows around it reach their minimums (the reply in Write, the details
    /// in review), down to `min`. Inside, the section (at most 40 % of the
    /// area, clipped) shrinks before the text.
    fn text_and_comments(
        &self,
        text: impl IntoElement,
        prefer: gpui_kit::Pixels,
        min: gpui_kit::Pixels,
        stacked: bool,
        cx: &mut Context<T>,
    ) -> gpui_kit::AnyElement {
        div()
            .id("text-and-comments")
            .flex()
            .flex_col()
            .flex_grow(1.)
            .flex_basis(prefer)
            .flex_shrink(YIELD_LAST)
            .min_h(min)
            .child(text)
            .when(stacked, |area| area.child(self.comments_section(cx)))
            .test_support()
            .into_any_element()
    }

    /// Wide and Narrow: the margin beside the text, at the mode's width.
    fn margin_column(&self, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        let Some(open) = self.comments_note() else {
            return div().into_any_element();
        };
        div()
            .id("writing-feedback")
            .w(px(self.margin_mode.width()))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.comments_scroll)
            .flex()
            .flex_col()
            .gap_2()
            .px_2()
            .py_2()
            .border_l_1()
            .border_color(rgb(pal().hairline))
            .child(self.comments_label(&open.record().id))
            .children(self.comment_items(true, cx))
            .test_support()
            .into_any_element()
    }

    /// Stacked: the docked section below the text, with the same list,
    /// cards, draft and IDs as the margin. Escape inside collapses it.
    fn comments_section(&self, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        let Some(open) = self.comments_note() else {
            return div().into_any_element();
        };
        // Clipped, so a squeezed section reads as its one-line label row
        // and never paints over the rows below. An open draft sits above
        // the list and keeps the section at least as tall as itself, so
        // its input is never cut off; the cards scroll below it.
        let drafting = self.comment_drafting();
        // Beside the text's 120 px ask the section gives way first.
        div()
            .id("comments-section")
            .flex()
            .flex_col()
            .flex_shrink(YIELD_FIRST)
            .min_h(if drafting {
                STACKED_DRAFT_FLOOR
            } else {
                px(0.)
            })
            .max_h(gpui_kit::relative(0.4))
            .overflow_hidden()
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .on_key_down(self.collapse_listener(cx))
            .child(
                div()
                    .id("comments-section-label")
                    .flex_shrink_0()
                    .px_3()
                    .py_1()
                    .child(self.comments_label(&open.record().id))
                    .test_support(),
            )
            .children(
                self.comment_draft(cx)
                    .map(|draft| div().flex_shrink_0().px_3().pb_2().child(draft)),
            )
            .child(
                div()
                    .id("writing-feedback")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_shrink(1.)
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.comments_scroll)
                    .px_3()
                    .pb_2()
                    .children(self.comment_items(false, cx))
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }

    /// The draft, a load error or the empty hint, then one card per comment.
    /// Only one placement renders at a time, so the IDs stay unique. The
    /// stacked section renders the draft itself, above its scroller.
    fn comment_items(&self, with_draft: bool, cx: &mut Context<T>) -> Vec<gpui_kit::AnyElement> {
        let mut items = vec![];
        let Some(open) = self.comments_note() else {
            return items;
        };
        let note_id = open.record().id.clone();
        let drafting = self.comment_drafting();
        if with_draft {
            items.extend(self.comment_draft(cx));
        }
        let (comments, feedback_error) = match self.workspace.writing_comments(&note_id) {
            Ok(comments) => (comments, None),
            Err(error) => (
                vec![],
                Some(format!(
                    "Couldn't load comments: {error}. Reload saved note to try again."
                )),
            ),
        };
        self.comment_cards(items, comments, feedback_error, drafting, cx)
    }

    /// The passage draft: its passage, Cancel and Add comment, then the
    /// input. The stacked section keeps all of it in view.
    fn comment_draft(&self, cx: &mut Context<T>) -> Option<gpui_kit::AnyElement> {
        let p = pal();
        let open = self.comments_note()?;
        let dirty = open.session().is_some();
        let (_, _, _, quote) = self
            .comment_anchor
            .as_ref()
            .filter(|(anchor_note, ..)| anchor_note == &open.record().id)?;
        Some(
            div()
                .id("comment-composer")
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap_1()
                .p_2()
                .rounded_md()
                .border_l_2()
                .border_color(rgb(p.accent))
                .bg(rgb(p.tile))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(80.))
                                .truncate()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(p.muted))
                                .italic()
                                .child(format!("“{}”", short(quote))),
                        )
                        .child(
                            pill("cancel-comment", "Cancel")
                                .on_click(self.on(cx, WritingRequest::CancelComment)),
                        )
                        .child(
                            // Outline: the panel's one filled primary
                            // stays in the writing or review bar.
                            button("post-comment")
                                .label("Add comment")
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .outline()
                                .disabled(dirty)
                                .when(dirty, |post| {
                                    post.accessibility_label(
                                        "Add comment, unavailable: Save your edit first",
                                    )
                                })
                                .tooltip(if dirty {
                                    "Save your edit first; comments anchor to the saved note"
                                } else {
                                    "Add this comment to the note's feedback"
                                })
                                .on_click(self.on(cx, WritingRequest::PostComment)),
                        ),
                )
                .child(
                    div()
                        .id("comment-input-box")
                        .h(px(if self.margin_mode == layout::MarginMode::Stacked {
                            STACKED_DRAFT_INPUT
                        } else {
                            90.
                        }))
                        .child(
                            prose_editor(&self.comment_input)
                                .h_full()
                                .aria_label("Comment"),
                        )
                        .test_support(),
                )
                .test_support()
                .into_any_element(),
        )
    }

    /// A passage draft is open for the note the comments show.
    fn comment_drafting(&self) -> bool {
        self.comments_note().is_some_and(|open| {
            self.comment_anchor
                .as_ref()
                .is_some_and(|(anchor_note, ..)| anchor_note == &open.record().id)
        })
    }

    /// The load error or the empty hint, then one card per comment.
    fn comment_cards(
        &self,
        mut items: Vec<gpui_kit::AnyElement>,
        mut comments: Vec<WritingComment>,
        feedback_error: Option<String>,
        drafting: bool,
        cx: &mut Context<T>,
    ) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let Some(open) = self.comments_note() else {
            return items;
        };
        let RecordData::Note(note) = &open.record().data else {
            return items;
        };
        let note_id = open.record().id.clone();
        let dirty = open.session().is_some();
        let failed = feedback_error.is_some();
        if let Some(error) = feedback_error {
            items.push(
                components::status_line("comments-error", Some(Tone::Error), error)
                    .into_any_element(),
            );
        }
        comments.sort_by_key(|entry| {
            let start = match &entry.record.data {
                RecordData::Comment(comment) => {
                    comment_location(comment, open.record().version, &note.markdown)
                        .map_or(usize::MAX, |range| range.start)
                }
                _ => usize::MAX,
            };
            (entry.addressed, start, entry.record.id.clone())
        });
        let applied = self
            .applied_rewrite
            .get(&note_id)
            .and_then(|(request, operation)| {
                self.workspace
                    .writing_request(request)
                    .ok()
                    .filter(|snapshot| snapshot.note.id == note_id)
                    .map(|snapshot| (request.clone(), operation.clone(), snapshot))
            });
        if comments.is_empty() && !drafting && !failed {
            items.push(
                components::empty_state(
                    "margin-empty",
                    IconName::MessageSquare,
                    "No comments yet",
                    "Select a passage and press ⌘/.",
                    None,
                )
                .into_any_element(),
            );
        }
        for entry in &comments {
            let RecordData::Comment(comment) = &entry.record.data else {
                continue;
            };
            let id = entry.record.id.clone();
            let location = comment_location(comment, open.record().version, &note.markdown);
            let checked = !entry.addressed && !self.excluded_comments.contains(&id);
            // An unlocated comment is kept as written; the owner excludes it
            // or comments on the passage again to reattach the feedback.
            let unlocated = unlocated_reason(comment, open.record().version);
            let status = if entry.addressed {
                "Addressed · history kept".to_owned()
            } else if location.is_none() {
                unlocated.status.clone()
            } else if !checked {
                "Excluded from rewrite".into()
            } else {
                String::new()
            };
            let quote_label = format!(
                "“{}{}”",
                comment.quote.chars().take(28).collect::<String>(),
                if comment.quote.chars().count() > 28 {
                    "…"
                } else {
                    ""
                }
            );
            // A disabled Show passage keeps its quote in the announced name,
            // so each comment's control stays distinct, and states why.
            let disabled_names = |why: &str| {
                (
                    components::unavailable(&format!("Show passage {quote_label}"), Some(why)).0,
                    components::unavailable("Show passage", Some(why)).1,
                )
            };
            let quote = if let Some(range) = location.clone() {
                let blocked = if dirty {
                    Some("save your edit first")
                } else if self.review.is_some() {
                    Some("finish the review first")
                } else {
                    None
                };
                let reveal = button(format!("reveal-comment-{id}"))
                    .label(quote_label.clone())
                    .ghost()
                    .compact()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .disabled(blocked.is_some())
                    .on_click(self.on(cx, WritingRequest::Reveal(range.clone())));
                match blocked {
                    Some(why) => {
                        let (announced, tooltip) = disabled_names(why);
                        reveal.accessibility_label(announced).tooltip(tooltip)
                    }
                    None => reveal.tooltip("Show passage in note"),
                }
            } else {
                let (announced, tooltip) = disabled_names(&unlocated.reason);
                button(format!("comment-quote-{id}"))
                    .label(quote_label.clone())
                    .ghost()
                    .compact()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .disabled(true)
                    .accessibility_label(announced)
                    .tooltip(tooltip)
            };
            let mut actions = div().flex().flex_wrap().gap_1();
            if !entry.addressed {
                let toggle = id.clone();
                actions = actions.child(
                    pill(
                        format!("include-comment-{id}"),
                        if checked { "Included" } else { "Include" },
                    )
                    .when(checked, |toggle| toggle.icon(IconName::Check))
                    .selected(checked)
                    .tooltip(if location.is_none() && !checked {
                        components::unavailable("Include", Some(&unlocated.reason)).1
                    } else if location.is_none() {
                        format!("Exclude this comment; {}", unlocated.reason)
                    } else if checked {
                        "Exclude from the next rewrite".into()
                    } else {
                        "Include in the next rewrite".into()
                    })
                    .when(!checked && location.is_none(), |toggle| {
                        toggle.accessibility_label(
                            components::unavailable("Include", Some(&unlocated.reason)).0,
                        )
                    })
                    .disabled(!checked && location.is_none())
                    .on_click(self.on(cx, WritingRequest::ToggleExcluded(toggle.clone()))),
                );
            }
            if let Some((request, operation, snapshot)) = &applied
                && !entry.addressed
                && snapshot.comments.iter().any(|captured| captured.id == id)
            {
                let (request, operation, record) = (request.clone(), operation.clone(), id.clone());
                actions = actions.child(
                    pill(format!("address-comment-{id}"), "Mark addressed").on_click(self.on(
                        cx,
                        WritingRequest::ConfirmAddressed {
                            request: request.clone(),
                            operation: operation.clone(),
                            comments: vec![record.clone()],
                        },
                    )),
                );
            }
            // Body first; the passage and the card's controls share a row
            // that wraps in a narrow margin. Cards keep only the left rule.
            let card = div()
                .id(format!("comment-card-{id}"))
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded_md()
                .border_l_2()
                .border_color(rgb(p.border))
                .bg(rgb(if entry.addressed { p.alt } else { p.tile }))
                .child(div().text_size(UI_TEXT).child(comment.body.clone()))
                .when(!status.is_empty(), |card| {
                    card.child(
                        div()
                            .id(format!("comment-status-{id}"))
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .aria_label(status.clone())
                            .child(status)
                            .test_support(),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(quote)
                        .child(div().flex_1())
                        .child(actions),
                );
            items.push(card.test_support().into_any_element());
        }
        items
    }

    /// The expanded saved inputs, when shown, then the writing bar.
    fn note_bar(&self, cx: &mut Context<T>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let Some(open) = self.note.as_ref() else {
            return vec![];
        };
        let note_id = open.record().id.clone();
        let dirty = open.session().is_some();
        let mut bar = div()
            .id("note-bar")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .px_3()
            .py_1()
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.hairline));
        let comments = self.workspace.writing_comments(&note_id);
        let feedback_error = comments.is_err();
        let comments = comments.unwrap_or_default();
        let applied = self
            .applied_rewrite
            .get(&note_id)
            .and_then(|(request, operation)| {
                self.workspace
                    .writing_request(request)
                    .ok()
                    .filter(|snapshot| {
                        snapshot.note.id == note_id && self.latest_rewrite_applied(&note_id)
                    })
                    .map(|snapshot| (request.clone(), operation.clone(), snapshot))
            });
        let selected = self.writing_threads.get(&note_id).cloned();
        let latest_request = self.latest_note_writing_request(&note_id);
        // The expanded saved rewrite inputs sit above the bar, outside it:
        // a short Note scrolls them down to their heading line before the
        // text gives up height or the bar is cut off.
        let mut saved_inputs = None;
        if let Some(record) = &latest_request
            && self.expanded_requests.contains(&record.id)
            && let Ok(snapshot) = self.workspace.writing_request(&record.id)
        {
            let mut disclosure = div()
                .id("writing-requests")
                .flex_shrink(1.)
                .min_h(px(40.))
                .max_h(px(220.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1()
                .px_3()
                .py_1()
                .bg(rgb(p.alt))
                .border_t_1()
                .border_color(rgb(p.hairline))
                .font_family(UI_FONT)
                .text_size(PROSE_TEXT)
                .child(subheading("Saved rewrite inputs"))
                .child(format!(
                    "Saved note v{} · {} comments · {} posted chat messages. Later additions stay pending.",
                    snapshot.note.version,
                    snapshot.comments.len(),
                    snapshot.messages.len()
                ));
            if let RecordData::Note(captured) = &snapshot.note.data {
                disclosure = disclosure.child(subheading("Captured saved note")).child(
                    self.safe_message_for_thread(
                        format!("captured-note-{}", record.id),
                        captured.markdown.clone(),
                        Some(&snapshot.thread),
                    ),
                );
            }
            if !snapshot.general_feedback.is_empty() {
                disclosure = disclosure
                    .child(subheading("Overall feedback"))
                    .child(snapshot.general_feedback.clone());
            }
            disclosure = disclosure.child(subheading("Captured chat"));
            for message in &snapshot.messages {
                if let RecordData::Message(body) = &message.data {
                    disclosure = disclosure
                        .child(if body.role == "owner" { "You" } else { "BRN" })
                        .child(self.safe_message_for_thread(
                            format!("captured-chat-{}", message.id),
                            body.text.clone(),
                            Some(&snapshot.thread),
                        ));
                }
            }
            saved_inputs = Some(disclosure.test_support());
        }
        bar = bar.child(self.overall_feedback());
        // The first matching row picks the one filled primary; controls of
        // lower matching rows still render once, outline or ghost.
        let busy = self.worker.is_some();
        let proposals = self
            .candidates
            .iter()
            .filter(|candidate| !candidate.request.writes.iter().any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
                || self.writing_reviews.contains_key(candidate.operation.as_str()))
            .any(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(&note_id));
        let Some(row) = self.writing_bar_row() else {
            return vec![];
        };
        let mut info: Vec<gpui_kit::AnyElement> = vec![];
        let mut secondary: Vec<gpui_kit::AnyElement> = vec![];
        let mut primary: Option<gpui_kit::AnyElement> = None;
        let mut reason: Option<&'static str> = None;
        if dirty {
            info.push(
                components::status_chip(
                    "note-unsaved-chip",
                    Some(Tone::Warning),
                    None,
                    "Unsaved edit · recoverable",
                )
                .into_any_element(),
            );
            secondary.push(
                pill("discard-note", "Discard…")
                    .tooltip("Discard your unsaved edit; asks first")
                    .on_click(self.on(cx, WritingRequest::ConfirmDiscard))
                    .into_any_element(),
            );
            // While the note changed since the edit began, Save as new note
            // is the stale-edit banner's primary and renders only there.
            if row != WritingRow::DirtyStale {
                secondary.push(
                    pill("save-buffer-copy", "Save as new note")
                        .tooltip("Save your text as a separate working note")
                        .on_click(self.on(cx, WritingRequest::SaveCopy))
                        .into_any_element(),
                );
            }
            let save = button("save-note")
                .label("Save ⌘S")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .tooltip("Save this edit (⌘S)")
                .on_click(self.on(cx, WritingRequest::Save));
            if row == WritingRow::Dirty {
                primary = Some(save.primary().into_any_element());
            } else {
                secondary.push(save.ghost().into_any_element());
            }
        }
        if let Some((request, operation, snapshot)) = &applied {
            let open_ids: Vec<String> = snapshot
                .comments
                .iter()
                .filter(|captured| {
                    !comments
                        .iter()
                        .any(|entry| entry.record.id == captured.id && entry.addressed)
                })
                .map(|record| record.id.clone())
                .collect();
            let (request, operation_mark, undo) =
                (request.clone(), operation.clone(), operation.clone());
            let clear = note_id.clone();
            info.push(
                components::status_chip(
                    "applied-state",
                    Some(Tone::Success),
                    Some(IconName::CircleCheck),
                    format!(
                        "Applied · v{}{}",
                        open.record().version,
                        if open_ids.is_empty() && !snapshot.comments.is_empty() {
                            format!(
                                " · {} addressed",
                                plural(snapshot.comments.len(), "comment")
                            )
                        } else {
                            String::new()
                        }
                    ),
                )
                .into_any_element(),
            );
            info.push(
                components::icon_button("clear-applied", IconName::Close, "Hide")
                    .tooltip("Hide; Changes keeps it and its Undo")
                    .on_click(self.on(cx, WritingRequest::HideApplied(clear.clone())))
                    .into_any_element(),
            );
            // As on the base, an open edit hides Undo; Changes keeps it.
            if !dirty {
                secondary.push(
                    pill("undo-writing-apply", "Undo")
                        .tooltip("Undo the applied rewrite; Changes keeps it")
                        .on_click(self.on(cx, WritingRequest::Undo(undo.clone())))
                        .into_any_element(),
                );
            }
            let again_blocked = if busy {
                Some("BRN is working")
            } else if dirty {
                Some("save your edit first")
            } else {
                None
            };
            secondary.push(
                pill("rewrite-again", "Rewrite again")
                    .disabled(again_blocked.is_some())
                    .when_some(again_blocked, |button, why| {
                        button
                            .accessibility_label(format!("Rewrite again, unavailable: {why}"))
                            .tooltip(format!("Rewrite again is unavailable: {why}"))
                    })
                    .on_click(self.on(cx, WritingRequest::Rewrite))
                    .into_any_element(),
            );
            if !open_ids.is_empty() {
                let mark = button("confirm-addressed")
                    .label(format!(
                        "Mark {} addressed",
                        plural(open_ids.len(), "comment")
                    ))
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .on_click(self.on(
                        cx,
                        WritingRequest::ConfirmAddressed {
                            request: request.clone(),
                            operation: operation_mark.clone(),
                            comments: open_ids.clone(),
                        },
                    ));
                if row == WritingRow::Applied {
                    primary = Some(mark.primary().into_any_element());
                } else {
                    secondary.push(mark.outline().into_any_element());
                }
            }
        } else {
            let mut included = 0;
            let mut unresolved_included = false;
            if let RecordData::Note(note) = &open.record().data {
                for entry in &comments {
                    if let RecordData::Comment(comment) = &entry.record.data
                        && !entry.addressed
                        && !self.excluded_comments.contains(&entry.record.id)
                    {
                        included += 1;
                        unresolved_included |=
                            comment_location(comment, open.record().version, &note.markdown)
                                .is_none();
                    }
                }
            }
            let messages = selected
                .as_ref()
                .and_then(|thread| self.workspace.messages(thread).ok())
                .map(|messages| {
                    messages
                        .iter()
                        .filter(|record| matches!(&record.data, RecordData::Message(m) if matches!(m.role.as_str(), "owner" | "assistant")))
                        .count()
                })
                .unwrap_or_default();
            let scope = format!(
                "Whole note · {} · {}",
                plural(included, "comment"),
                plural(messages, "saved context message")
            );
            info.push(
                // The scope shares the actions' row and truncates (its full
                // text is in the tooltip) instead of taking a row of its own
                // in a narrow Note.
                div()
                    .id("rewrite-scope")
                    .flex_1()
                    .min_w(px(120.))
                    .truncate()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .aria_label(scope.clone())
                    .tooltip({
                        let scope = SharedString::from(scope.clone());
                        move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(scope.clone())
                                .build(window, cx)
                        }
                    })
                    .child(scope)
                    .test_support()
                    .into_any_element(),
            );
            // Why a rewrite can't start now; shown inline when it blocks the
            // primary, and always in the control's tooltip and label.
            let recovering = row == WritingRow::Recover;
            let why = if recovering {
                Some("Recover the kept edit first")
            } else if busy {
                Some("BRN is working")
            } else if feedback_error {
                Some("Feedback couldn't load")
            } else if unresolved_included {
                Some("Exclude or re-select unlocated comments")
            } else if dirty {
                Some("Save your edit first")
            } else {
                None
            };
            if let Some(record) = latest_request {
                let toggle = record.id.clone();
                let retry = record.id.clone();
                let expanded = self.expanded_requests.contains(&record.id);
                secondary.push(
                    components::icon_button(
                        "inspect-rewrite-inputs",
                        IconName::Info,
                        if expanded {
                            "Hide saved inputs"
                        } else {
                            "Saved inputs"
                        },
                    )
                    .selected(expanded)
                    .on_click(self.on(cx, WritingRequest::ToggleRequest(toggle.clone())))
                    .into_any_element(),
                );
                let retry_blocked = if recovering {
                    Some("Recover the kept edit first")
                } else if busy {
                    Some("BRN is working")
                } else if proposals {
                    Some("Review or dismiss the ready rewrite first")
                } else if dirty {
                    Some("Save your edit first")
                } else {
                    None
                };
                let retry = button("retry-rewrite")
                    .label("Retry saved rewrite")
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .disabled(retry_blocked.is_some())
                    .accessibility_label(match retry_blocked {
                        Some(why) => format!("Retry saved rewrite, unavailable: {why}"),
                        None => "Retry saved rewrite".into(),
                    })
                    .tooltip(match retry_blocked {
                        Some(why) => format!("Retry is unavailable: {why}"),
                        None => "Run the rewrite again with the same saved inputs".into(),
                    })
                    .on_click(self.on(cx, WritingRequest::Retry(retry.clone())));
                if row == WritingRow::Retry {
                    reason = retry_blocked;
                    primary = Some(retry.primary().into_any_element());
                } else {
                    secondary.push(retry.ghost().into_any_element());
                }
            }
            // As on the base, an open edit hides Undo; Changes keeps it.
            if let Some((_, operation)) = self.applied_rewrite.get(&note_id)
                && !dirty
            {
                let undo = operation.clone();
                secondary.push(
                    pill("undo-writing-apply", "Undo previous rewrite")
                        .tooltip("Undo the last applied rewrite; Changes keeps it")
                        .on_click(self.on(cx, WritingRequest::Undo(undo.clone())))
                        .into_any_element(),
                );
            }
            let rewrite = button("rewrite-note")
                .label("Rewrite with AI ⌘⇧R")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .disabled(why.is_some())
                .accessibility_label(match why {
                    Some(why) => format!("Rewrite with AI, unavailable: {why}"),
                    None => "Rewrite with AI ⌘⇧R".into(),
                })
                .tooltip(
                    "Captures this note, included comments, overall feedback and saved context",
                )
                .on_click(self.on(cx, WritingRequest::Rewrite));
            if matches!(row, WritingRow::Finished | WritingRow::Idle) {
                reason = why;
                primary = Some(rewrite.primary().into_any_element());
            } else {
                secondary.push(rewrite.ghost().into_any_element());
            }
            // A rewrite saved as Working that this window does not hold may
            // still run elsewhere: Stop… asks first. When the visible chat's
            // recovery card already offers Stop… for this run, it is there.
            if let Some(RunOutcome::Elsewhere(run)) = self.writing_run_outcome()
                && self.worker.as_ref().and_then(|w| w.run_id) != Some(run.as_str())
                && self.chat_card_stop().as_deref() != Some(run.as_str())
            {
                secondary.push(self.stop_saved_run_button(&run, cx).into_any_element());
            }
        }
        let line = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .children(info)
            .child(div().flex_1())
            .children(secondary)
            .when_some(reason, |line, why| {
                line.child(
                    div()
                        .id("writing-blocked-reason")
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child(why)
                        .test_support(),
                )
            })
            .children(primary);
        // The inputs give way down to their heading; the bar never does.
        saved_inputs
            .map(|inputs| inputs.into_any_element())
            .into_iter()
            .chain([bar.child(line).test_support().into_any_element()])
            .collect()
    }

    fn review_body(&self, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(review) = self.review.as_ref() else {
            return div().into_any_element();
        };
        let has_evidence =
            review.candidate.request.writes.iter().any(
                |w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE),
            );
        let writing_evidence = if has_evidence {
            match self.writing_reviews.get(review.candidate.operation.as_str()) {
                Some(evidence) => Some(evidence),
                None => return div().id("writing-evidence-unavailable").flex().flex_col().gap_2().p_3()
                    .aria_label("Writing proposal unavailable here. Open and review it in its matching writing context, or Refresh after restoring access.")
                    .child("Writing proposal unavailable here. Open and review it in its matching writing context, or Refresh after restoring access.")
                    .child(pill("close-review", "Keep for later").on_click(self.on(cx, WritingRequest::KeepReview)))
                    .test_support().into_any_element(),
            }
        } else {
            None
        };
        let fit = self.review_fit;
        let writes = &review.candidate.request.writes;
        let shape = ProposalShape::of(writes);
        // Approval wording whenever the proposal writes a protected note.
        let protected = writes.iter().any(|write| {
            matches!(&write.data, RecordData::Note(note) if note.protected)
                || self.records.iter().any(|record| {
                    record.id == write.id
                        && matches!(&record.data, RecordData::Note(note) if note.protected)
                })
        });
        let counts = review.note.as_ref().map(|id| {
            let current = self
                .workspace
                .store
                .note(id)
                .ok()
                .flatten()
                .and_then(|record| match record.data {
                    RecordData::Note(note) => Some(note.markdown),
                    _ => None,
                })
                .unwrap_or_default();
            let tracked = diff::tracked(&current, &review.original);
            format!("+{} −{}", tracked.added, tracked.removed)
        });
        let editing = self.review.is_editing();
        let edited = editing || self.review.is_edited();
        let grouped = review.groups.groups.len() > 1;
        let partial = review.selected.len() != review.groups.groups.len();
        let snapshot = review
            .candidate
            .request
            .inputs
            .iter()
            .find_map(|input| self.workspace.writing_request(&input.record).ok());
        let open_comments = snapshot.as_ref().map_or(0, |snapshot| {
            let addressed: BTreeSet<String> = self
                .workspace
                .writing_comments(&snapshot.note.id)
                .map(|entries| {
                    entries
                        .into_iter()
                        .filter(|entry| entry.addressed)
                        .map(|entry| entry.record.id)
                        .collect()
                })
                .unwrap_or_default();
            snapshot
                .comments
                .iter()
                .filter(|comment| !addressed.contains(&comment.id))
                .count()
        });
        let mut footer = review_footer(shape, protected, open_comments);
        let selected_comments = review.selected_comments.len();
        if selected_comments > 0 {
            footer = format!(
                "{} Apply also marks {} addressed.",
                shape.binding(protected),
                plural(selected_comments, "captured comment")
            );
        }

        // The Proposal label plus the Protected chip states the review. A
        // narrower bar keeps the counts, then the shape, in the label's
        // tooltip and accessible name.
        let described = match &counts {
            Some(counts) => format!("Proposal · {} · {counts}", shape.label()),
            None => format!("Proposal · {}", shape.label()),
        };
        let shown_counts = counts.clone().filter(|_| !fit.actions_in_menu());
        let folded = fit.icons() || (counts.is_some() && shown_counts.is_none());
        let label = div()
            .id("review-proposal")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .aria_label(described.clone())
            .when(folded, |label| {
                let tip = SharedString::from(described.clone());
                label.tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
                })
            })
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("Proposal"),
            )
            .when(!fit.icons(), |label| {
                label.child(
                    div()
                        .text_color(rgb(p.muted))
                        .child(format!("· {}", shape.label())),
                )
            })
            .when_some(shown_counts, |label, counts| {
                label.child(div().text_color(rgb(p.muted)).child(counts))
            })
            .test_support();
        let protected_chip = protected.then(|| {
            if fit.icons() {
                components::short_status_chip(
                    "review-protected",
                    Some(Tone::Info),
                    Some(IconName::Lock),
                    "",
                    "Protected",
                )
            } else {
                components::status_chip(
                    "review-protected",
                    Some(Tone::Info),
                    Some(IconName::Lock),
                    "Protected",
                )
            }
        });
        // Changes | Result: the old toggle's ID stays on Result.
        let mode = review.note.is_some().then(|| {
            let why = edited.then_some("Finish or cancel the edit first");
            components::segmented(
                "review-mode",
                vec![
                    button("review-show-changes")
                        .label("Changes")
                        .selected(!self.review.is_reading())
                        .disabled(edited)
                        .when_some(why, |segment, why| {
                            segment.accessibility_label(format!("Changes, unavailable: {why}"))
                        })
                        .tooltip(why.unwrap_or("Show the tracked changes"))
                        .on_click(self.on(cx, WritingRequest::ReviewReading(false))),
                    button("review-read-result")
                        .label("Result")
                        .selected(self.review.is_reading())
                        .disabled(edited)
                        .when_some(why, |segment, why| {
                            segment.accessibility_label(format!("Result, unavailable: {why}"))
                        })
                        .tooltip(why.unwrap_or("Read the proposed text, formatted"))
                        .on_click(self.on(cx, WritingRequest::ReviewReading(true))),
                ],
            )
        });
        // A proposal without note text names its reason here; the details
        // below list what it writes.
        let middle = if review.note.is_none() {
            let reason = review.candidate.request.reason.clone();
            let tip = SharedString::from(reason.clone());
            div()
                .id("review-reason")
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(rgb(p.muted))
                .aria_label(reason.clone())
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
                })
                .child(reason)
                .test_support()
                .into_any_element()
        } else {
            div().flex_1().into_any_element()
        };
        let mut secondary: Vec<gpui_kit::AnyElement> = vec![];
        if editing {
            secondary.push(
                pill("cancel-final", "Cancel edit")
                    .tooltip("Discard your edit to the final text and show the proposal again")
                    .on_click(self.on(cx, WritingRequest::CancelReviewEdit))
                    .into_any_element(),
            );
        }
        if !fit.actions_in_menu() {
            secondary.push(
                pill("close-review", "Keep for later")
                    .tooltip("Close the review; the proposal stays ready in the writing bar")
                    .on_click(self.on(cx, WritingRequest::KeepReview))
                    .into_any_element(),
            );
            secondary.push(
                pill("dismiss-proposal", "Dismiss")
                    .tooltip("Dismiss this proposal; the note stays unchanged")
                    .on_click(self.on(cx, WritingRequest::DismissReview))
                    .into_any_element(),
            );
        }
        // The one primary, by state; blocked controls stay in place with
        // their reason.
        let full = shape.primary(protected);
        let apply_blocked = if edited {
            Some("Finish or cancel the edit first")
        } else if partial {
            Some("Prepare your selection first")
        } else {
            None
        };
        // The full label shows only where Apply leads and the bar has room;
        // it stays the tooltip and accessible name.
        let apply_leads = !editing && !(grouped && partial);
        let apply = button("apply-final")
            .label(if fit.icons() || !apply_leads {
                if protected { "Approve" } else { "Apply" }.to_owned()
            } else {
                full.clone()
            })
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .disabled(apply_blocked.is_some())
            .accessibility_label(match apply_blocked {
                Some(why) => format!("{full}, unavailable: {why}"),
                None => full.clone(),
            })
            .tooltip(match apply_blocked {
                Some(why) => format!("{full} is unavailable: {why}"),
                None => format!("{full} · {}", shape.binding(protected)),
            })
            .on_click(self.on(cx, WritingRequest::ApplyReview));
        let select_blocked = if edited {
            Some("Finish or cancel the edit first")
        } else if review.selected.is_empty() {
            Some("Include at least one group")
        } else if !partial {
            Some("Omit a group first")
        } else {
            None
        };
        let prepare_selected = button("prepare-selected")
            .label("Prepare selection")
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .disabled(select_blocked.is_some())
            .accessibility_label(match select_blocked {
                Some(why) => format!("Prepare selection, unavailable: {why}"),
                None => "Prepare selection".into(),
            })
            .tooltip(select_blocked.unwrap_or("Review only the included groups as a new proposal"))
            .on_click(self.on(cx, WritingRequest::PrepareReviewSelection));
        // A Prepare selection that waits sits in the bar only when the full
        // bar has room; otherwise it moves beside the group toggles, where
        // its reason applies, so the bar keeps one row with the primary
        // last.
        let mut waiting_selection = None;
        let mut prepare_selected = Some(prepare_selected);
        if grouped && (editing || !partial) {
            let waiting = prepare_selected.take().map(|control| control.ghost());
            if fit.actions_in_menu() {
                waiting_selection = waiting;
            } else {
                secondary.extend(waiting.map(IntoElement::into_any_element));
            }
        }
        let primary = if editing {
            secondary.push(apply.outline().into_any_element());
            button("prepare-final")
                .label("Use this text")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .disabled(!self.review.is_edited())
                .accessibility_label(if self.review.is_edited() {
                    "Use this text"
                } else {
                    "Use this text, unavailable: Edit the text first"
                })
                .tooltip(if self.review.is_edited() {
                    "Review your text as a new proposal"
                } else {
                    "Edit the text first"
                })
                .on_click(self.on(cx, WritingRequest::PrepareReviewEdit))
                .primary()
        } else if let Some(selection) = prepare_selected.filter(|_| grouped && partial) {
            secondary.push(apply.outline().into_any_element());
            selection.primary()
        } else {
            apply.primary()
        };
        // One row; a crowded state wraps rather than clipping a control.
        let bar = components::toolbar("review-bar")
            .h_auto()
            .min_h(components::tokens::TOOLBAR)
            .flex_wrap()
            .py(px(3.5))
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .child(label)
            .children(protected_chip)
            .children(mode)
            .child(middle)
            .children(secondary)
            .children(self.comments_toggle(cx))
            .child(primary);
        let blocked_reason = if editing {
            Some(
                "Finish or cancel the edit first. Use this text reviews your text as a new proposal.",
            )
        } else if grouped && review.selected.is_empty() {
            Some("Include at least one group, then prepare your selection.")
        } else if grouped && partial {
            Some(
                "Prepare your selection first. Only the included groups become a new proposal to review.",
            )
        } else {
            None
        };
        // The details scroll; with several rows they give way first, down to
        // a line or two, so the review text and the footer keep their room.
        let note_writes = writes
            .iter()
            .filter(|write| matches!(write.data, RecordData::Note(_)))
            .count();
        let tall =
            self.review.inputs_shown() || review.note.is_none() || grouped || note_writes > 1;
        let mut details = div()
            .id("review-details")
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_1()
            .flex_shrink_0()
            .when(tall, |details| details.flex_shrink(1.).min_h(px(44.)))
            .text_size(UI_TEXT)
            .max_h(px(220.))
            .overflow_y_scroll()
            .when_some(blocked_reason, |details, why| {
                details.child(
                    div()
                        .id("review-blocked-reason")
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .aria_label(why)
                        .child(why)
                        .test_support(),
                )
            });
        if let Some(evidence) = &writing_evidence {
            let private = evidence
                .sources
                .iter()
                .filter(|source| source.private_into_wider_note)
                .map(|source| format!("{} (v{})", source.title, source.version))
                .collect::<Vec<_>>();
            if !private.is_empty() {
                let disclosure = format!(
                    "Private sources used in this more widely visible Note: {}. Apply requires your exact review and retains private provenance.",
                    private.join(", ")
                );
                details = details.child(
                    div()
                        .id("writing-private-disclosure")
                        .aria_label(disclosure.clone())
                        .child(disclosure)
                        .test_support(),
                );
            }
        }
        if let Some(evidence) = &writing_evidence {
            // New attachment annotations consume only the host's checked context batch;
            // the legacy snapshot reader above remains unchanged for existing review UI.
            let captured = snapshot
                .as_ref()
                .filter(|saved| self.writing_requests.get(&saved.request_message) == Some(*saved));
            details = details.child(self.sources_used(evidence, captured));
        }
        if self.review.inputs_shown() || review.note.is_none() {
            // The bar names the reason when the proposal has no note text.
            if review.note.is_some() {
                details = details.child(review.candidate.request.reason.clone());
            }
            for write in &review.candidate.request.writes {
                details = details.child(metadata(match &write.data {
                    RecordData::Note(n) => format!(
                        "Note: {}{}",
                        n.title,
                        if n.protected { " · protected" } else { "" }
                    ),
                    RecordData::Action(a) => format!("Task: {} · {:?}", a.description, a.state),
                    RecordData::Thread(t) => format!("Chat: {}", t.title),
                    RecordData::Message(m) => format!("Message: {}", short(&m.text)),
                    RecordData::Link(_) => "Connect related work".into(),
                    RecordData::Comment(c) => format!("Comment: {}", short(&c.body)),
                    RecordData::Source(s) => format!("Source: {}", s.label),
                    RecordData::Asset(_) => "Retain a meaningful figure".into(),
                    RecordData::Run(_) => "Save work progress".into(),
                    RecordData::Settings(_) => "Change workspace settings".into(),
                    RecordData::WritingSetup(_) => "Change writing setup".into(),
                    RecordData::ChatControl(_) => "Change chat controls".into(),
                    RecordData::Project(_) => "Change project configuration".into(),
                }));
            }
            if let Some(snapshot) = &snapshot {
                details = details.child(metadata(format!(
                    "Inputs: saved v{} · {} · {}",
                    snapshot.note.version,
                    plural(snapshot.comments.len(), "comment"),
                    plural(snapshot.messages.len(), "chat message")
                )));
                if !snapshot.general_feedback.is_empty() {
                    details = details
                        .child(subheading("Overall feedback"))
                        .child(snapshot.general_feedback.clone());
                }
            }
            details = details.child(metadata(
                "A note that changed since, or has an open edit, stays unchanged until that is resolved.",
            ));
        }
        if grouped {
            details = details.child(metadata("Include or omit whole notes and tasks."));
            let mut groups = div().flex().flex_wrap().gap_1();
            for group in &review.groups.groups {
                let id = group.id.clone();
                let included = review.selected.contains(&id);
                let label = format!(
                    "{} {}: {}",
                    if included { "Include" } else { "Omit" },
                    if group.kind == "Action" {
                        "task"
                    } else {
                        "note"
                    },
                    short(&group.title)
                );
                let blocked = if !review.groups.ungrouped_writes.is_empty() {
                    Some("These changes can't be separated")
                } else if edited {
                    Some("Finish or cancel the edit first")
                } else {
                    None
                };
                groups = groups.child(
                    pill(format!("review-group-{id}"), label.clone())
                        .selected(included)
                        .disabled(blocked.is_some())
                        .when_some(blocked, |toggle, why| {
                            toggle
                                .accessibility_label(format!("{label}, unavailable: {why}"))
                                .tooltip(why)
                        })
                        .on_click(self.on(cx, WritingRequest::ToggleGroup(id.clone()))),
                );
                if !group.required_groups.is_empty() {
                    let required = group
                        .required_groups
                        .iter()
                        .filter_map(|id| review.groups.groups.iter().find(|g| &g.id == id))
                        .map(|g| g.title.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    groups = groups.child(metadata(format!("Requires: {required}")));
                }
            }
            details = details.child(groups.children(waiting_selection));
            if !review.groups.ungrouped_writes.is_empty() {
                let line = "These changes can't be separated. Apply all or ask for a new rewrite.";
                details = details.child(
                    div()
                        .id("review-inseparable")
                        .aria_label(line)
                        .child(line)
                        .test_support(),
                );
            }
        }
        let notes: Vec<_> = review
            .candidate
            .request
            .writes
            .iter()
            .filter_map(|write| match &write.data {
                RecordData::Note(note) => Some((write.id.clone(), note.title.clone())),
                _ => None,
            })
            .collect();
        if notes.len() > 1 {
            // "Showing: A · B", the shown note selected.
            let mut members = div()
                .id("review-members")
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child("Showing:"),
                );
            for (index, (id, name)) in notes.into_iter().enumerate() {
                let selected = review.note.as_ref() == Some(&id);
                if index > 0 {
                    members = members.child(div().text_color(rgb(p.muted)).child("·"));
                }
                members = members.child(
                    pill(format!("review-member-{id}"), short(&name))
                        .tooltip(format!("Show {name}"))
                        .accessibility_label(format!("Show {name}"))
                        .selected(selected)
                        .on_click(self.on(cx, WritingRequest::SelectReviewNote(id.clone()))),
                );
            }
            details = details.child(members.test_support());
        }
        // Display only the host's currently verified writing-context batch.
        if let Some(snapshot) = review
            .feedback
            .as_ref()
            .and_then(|snapshot| self.writing_requests.get(&snapshot.request_message))
            .filter(|snapshot| review.note.as_ref() == Some(&snapshot.note.id))
            && !snapshot.comments.is_empty()
        {
            let mut feedback = div()
                .id("review-feedback")
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .child("Mark selected comments addressed after Apply (optional)");
            for captured in &snapshot.comments {
                let RecordData::Comment(comment) = &captured.data else {
                    continue;
                };
                let id = captured.id.clone();
                let checked = review.selected_comments.contains(&id);
                feedback = feedback.child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(SMALL_TEXT)
                                .child(format!("“{}” — {}", comment.quote, comment.body)),
                        )
                        .child(
                            pill(
                                format!("review-address-comment-{id}"),
                                if checked {
                                    "Selected"
                                } else {
                                    "Mark addressed with Apply"
                                },
                            )
                            .selected(checked)
                            .when(checked, |button| button.icon(IconName::Check))
                            .disabled(edited)
                            .accessibility_label(format!(
                                "Mark captured comment addressed with Apply: {}",
                                comment.body
                            ))
                            .on_click(self.on(cx, WritingRequest::ToggleReviewComment(id))),
                        ),
                );
            }
            details = details.child(feedback.test_support());
        }
        let body = if review.note.is_none() {
            div().into_any_element()
        } else if self.review.is_editing() {
            prose_editor(&self.review_editor)
                .h_full()
                .aria_label("Final proposed note text")
                .into_any_element()
        } else if self.review.is_reading() {
            crate::document::readable_document(&self.candidate_view, self.note_assets())
                .into_any_element()
        } else {
            prose_editor(&self.diff_editor)
                .h_full()
                .readonly(true)
                .aria_label("Proposed changes")
                .into_any_element()
        };
        let centered = self.centered;
        // The review text, then comments when they're stacked, then the
        // footer that names exactly what Apply saves; it never folds and
        // never moves. The text asks for 120 px (plus the open section's
        // floor) after the details give way, and yields to a few lines
        // before the footer would.
        let stacked = self.comments_stacked();
        let text = review.note.is_some();
        let (prefer, min) = match (text, stacked) {
            (false, _) => (px(0.), px(0.)),
            (true, false) => (text_min(true), text_min(false)),
            (true, true) => (text_min(true) + self.stacked_floor(), text_min(false)),
        };
        let mut document = div()
            .id("review-body")
            .flex()
            .flex_grow(1.)
            .flex_basis(if text { text_min(true) } else { px(0.) })
            // Inside the area the open section gives way before the text.
            .flex_shrink(YIELD_LAST)
            .min_h(min)
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .when(centered, |text| text.justify_center())
                    .child(reading_column().child(body)),
            );
        if self.comments_beside() {
            document = document.child(self.margin_column(cx));
        }
        div()
            .id("review-panel")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(bar.test_support())
            .child(details.test_support())
            .child(self.text_and_comments(document.test_support(), prefer, min, stacked, cx))
            .child(self.writing_activity(cx))
            .child(
                div()
                    .id("note-bar")
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .font_family(UI_FONT)
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.ink))
                    .bg(rgb(p.alt))
                    .border_t_1()
                    .border_color(rgb(p.hairline))
                    .aria_label(footer.clone())
                    .child(footer)
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }

    pub fn compare_tile(&self, _: &mut Context<T>) -> gpui_kit::AnyElement {
        if let Some(review) = self.review.as_ref()
            && review.candidate.request.writes.iter().any(
                |w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE),
            )
            && !self
                .writing_reviews
                .contains_key(review.candidate.operation.as_str())
        {
            return div().id("compare-evidence-unavailable").p_3()
                .aria_label("Writing comparison unavailable in this context. Open and review the matching writing context or Refresh after restoring access.")
                .child("Writing comparison unavailable in this context. Open and review the matching writing context or Refresh after restoring access.")
                .test_support().into_any_element();
        }
        if self
            .review
            .as_ref()
            .is_none_or(|review| review.note.is_none())
        {
            return scroll("compare-content")
                .child(components::empty_state(
                    "compare-empty",
                    IconName::Columns2,
                    "Nothing to compare",
                    "Compare shows a whole-note proposal beside the saved note.",
                    None,
                ))
                .into_any_element();
        }
        let version = self
            .review
            .as_ref()
            .and_then(|review| review.note.as_ref())
            .and_then(|id| self.records.iter().find(|record| &record.id == id))
            .map_or(0, |record| record.version);
        div()
            .id("compare-content")
            .size_full()
            .flex()
            .gap_2()
            .p_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(section(format!("Current · v{version}")))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(markdown(&self.current_view, self.note_assets())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(section("Proposal · exact text under review"))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(markdown(&self.candidate_view, self.note_assets())),
                    ),
            )
            .test_support()
            .into_any_element()
    }
}

/// A one-line label's width in the UI font, as the text system lays it out.
fn text_width(window: &Window, text: &str, size: gpui_kit::Pixels) -> f32 {
    let mut style = window.text_style();
    style.font_family = UI_FONT.into();
    let run = style.to_run(text.len());
    f32::from(
        window
            .text_system()
            .shape_line(text.to_owned().into(), size, &[run], None)
            .width,
    )
}

/// The widest line of note or review text; wider panels keep 24 px gutters
/// and leave the rest as margin.
pub const READING_MEASURE: f32 = 680.;

/// The note and review text column: at most 680 px of text with 24 px
/// gutters, filling a narrower panel. Long words and URLs wrap.
fn reading_column() -> gpui_kit::Div {
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .max_w(px(READING_MEASURE) + components::tokens::SPACE_6 * 2.)
        .px(components::tokens::SPACE_6)
        .py_2()
}

/// A banner's secondary action: a 28 px outline button.
fn banner_button(id: impl Into<gpui_kit::ElementId>, label: &'static str) -> Button {
    button(id)
        .label(label)
        .compact()
        .outline()
        .h(components::tokens::COMPACT_CONTROL)
}

/// "1 comment", "3 comments".
fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}
/// The text's few lines, or the 120 px it keeps beside an open comments
/// section. With a banner shown it asks for the 120 px and keeps the few
/// lines as its floor.
fn text_min(stacked: bool) -> gpui_kit::Pixels {
    px(if stacked { 120. } else { 48. })
}

/// The open comments section's floor: its label and the first card's
/// first line.
pub const COMMENTS_FLOOR: gpui_kit::Pixels = px(64.);

/// The stacked draft's input height.
const STACKED_DRAFT_INPUT: f32 = 56.;

/// The open section's floor while a draft is open: the label row (26),
/// the draft (8 + 28 control row + 4 + input + 8) with its 8 px below,
/// and the 1 px rule above.
pub const STACKED_DRAFT_FLOOR: gpui_kit::Pixels =
    px(26. + 8. + 28. + 4. + STACKED_DRAFT_INPUT + 8. + 8. + 1.);

/// The floor of the box that holds the banners after the first one in a
/// short Note: its first line (the saved version's label, or the next
/// banner's opening line) stays in view and the rest scrolls. The first
/// banner's actions are always in view above it.
const BANNER_FLOOR: gpui_kit::Pixels = px(28.);

/// The shrink factor of the review or note text area: next to the reply's
/// and the details' factor of 1, it gives way only once they reach their
/// minimums.
const YIELD_LAST: f32 = 0.01;

/// The shrink factor of rows that give way before every other one: a
/// large factor on the yielding row rather than a small one on the rest,
/// so the rest still take up the remaining overflow once it reached its
/// floor.
const YIELD_FIRST: f32 = 1000.;

/// What a proposal writes. It names the review bar's label and primary and
/// the footer's binding sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalShape {
    /// One note and nothing else.
    Note,
    /// Several notes and nothing else.
    Notes(usize),
    /// Anything else, counted as changes.
    Changes(usize),
}
impl ProposalShape {
    pub fn of(writes: &[Put]) -> Self {
        let notes = writes
            .iter()
            .filter(|write| matches!(write.data, RecordData::Note(_)))
            .count();
        match (notes, writes.len()) {
            (1, 1) => Self::Note,
            (notes, total) if notes == total && notes > 1 => Self::Notes(notes),
            (_, total) => Self::Changes(total),
        }
    }
    pub fn label(self) -> String {
        match self {
            Self::Note => "whole note".into(),
            Self::Notes(count) => plural(count, "note"),
            Self::Changes(count) => plural(count, "change"),
        }
    }
    /// "Apply whole note", or "Approve …" when a protected note is written.
    pub fn primary(self, protected: bool) -> String {
        format!(
            "{} {}",
            if protected { "Approve" } else { "Apply" },
            self.label()
        )
    }
    /// The sentence that says exactly what Apply saves.
    pub fn binding(self, protected: bool) -> String {
        match (self, protected) {
            (Self::Note, false) => "Applying saves exactly this text.".into(),
            (Self::Note, true) => "Approving saves exactly this text to a protected note.".into(),
            (Self::Notes(count), false) => format!("Applying saves exactly these {count} notes."),
            (Self::Notes(count), true) => {
                format!("Approving saves exactly these {count} notes, including protected notes.")
            }
            (Self::Changes(1), false) => "Applying saves exactly this change.".into(),
            (Self::Changes(1), true) => {
                "Approving saves exactly this change to a protected note.".into()
            }
            (Self::Changes(count), false) => {
                format!("Applying saves exactly these {count} changes.")
            }
            (Self::Changes(count), true) => format!(
                "Approving saves exactly these {count} changes, including a protected note."
            ),
        }
    }
}

/// The review footer: the binding sentence first, then the comments that
/// stay open (omitted when none).
pub fn review_footer(shape: ProposalShape, protected: bool, open: usize) -> String {
    let binding = shape.binding(protected);
    match open {
        0 => binding,
        1 => format!("{binding} 1 comment stays open until you mark it addressed."),
        open => format!("{binding} {open} comments stay open until you mark them addressed."),
    }
}

/// The existing open-edit comparison dialog, over the host's selected saved/buffer text.
pub fn compare_open_edit_dialog(
    dialog: gpui_kit::component::dialog::Dialog,
    window: &mut Window,
    base: u64,
    version: u64,
    edit: &str,
    saved: &str,
) -> gpui_kit::component::dialog::Dialog {
    let side = |id: &'static str, label: String, text: &str| {
        div().flex().flex_col().gap_1().child(section(label)).child(
            div()
                .id(id)
                .p_2()
                .rounded(components::tokens::RADIUS_CONTROL)
                .bg(rgb(pal().alt))
                .text_size(UI_TEXT)
                .aria_label(text.to_owned())
                .child(if text.trim().is_empty() {
                    "(empty)".to_owned()
                } else {
                    text.to_owned()
                })
                .test_support(),
        )
    };
    dialog
        .title("Compare the open edit")
        .on_ok(|_, _, _| false)
        .child(
            components::dialog_body("compare-open-edit-body", window)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(side(
                            "compare-open-edit-buffer",
                            format!("Open edit · from v{base}"),
                            edit,
                        ))
                        .child(side(
                            "compare-open-edit-saved",
                            format!("Saved · v{version}"),
                            saved,
                        )),
                )
                .test_support(),
        )
        .footer(
            components::dialog_footer().child(
                button("compare-open-edit-close")
                    .label("Close")
                    .outline()
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            ),
        )
}
