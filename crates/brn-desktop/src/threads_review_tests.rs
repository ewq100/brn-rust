//! Review bar and comment placement journeys (S4) on synthetic data with
//! the headless GPUI harness. Pixels and pointer feel remain native checks.
use super::layout::{MarginMode, ReviewFit};
use super::polish_tests::{
    Fixture, assert_in, choose, fixture, menu_rows, on_window, type_edit, update, window_bounds,
};
use super::writing::{COMMENTS_FLOOR, ProposalShape, review_footer};
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

const TEXT: &str = "# Pilot\n\nThe quick brown fox jumps.\n\nKeep this line.";
const REVISED: &str = "# Pilot\n\nThe swift brown fox leaps.\n\nKeep this line.";
const PROTECTED_FOOTER: &str = "Approving saves exactly this text to a protected note. 2 comments stay open until you mark them addressed.";

/// Give the open note two passage comments in a linked writing chat, after
/// protecting it when asked. Returns the chat.
fn commented(cx: &mut TestAppContext, f: &Fixture, protected: bool) -> String {
    update(cx, f, |view, window, cx| {
        let mut note = view.workspace.store.note(&f.note.id).unwrap().unwrap();
        if protected {
            let RecordData::Note(mut data) = note.data.clone() else {
                unreachable!()
            };
            data.protected = true;
            view.workspace
                .owner_change(
                    "s4-protect",
                    &ChangeRequest {
                        reason: "Protect the review witness".into(),
                        writes: vec![Put {
                            id: note.id.clone(),
                            expected_version: Some(note.version),
                            archived: false,
                            data: RecordData::Note(data),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            note = view.workspace.store.note(&f.note.id).unwrap().unwrap();
        }
        let thread = view.workspace.new_thread("Pilot chat").unwrap().id;
        view.workspace
            .link_writing_thread("s4-link", &note.id, note.version, &thread)
            .unwrap();
        for (index, (word, body)) in [("quick", "Sharper verb"), ("Keep", "Say why")]
            .into_iter()
            .enumerate()
        {
            let start = TEXT.find(word).unwrap();
            view.workspace
                .add_writing_comment(
                    &format!("s4-comment-{index}"),
                    &WritingCommentInput {
                        note: note.id.clone(),
                        version: note.version,
                        thread: thread.clone(),
                        range: (start, start + word.len()),
                        quote: word.into(),
                        body: body.into(),
                    },
                )
                .unwrap();
        }
        view.refresh();
        view.open_note(&note.id, None, window, cx);
        view.select_writing_thread(note.id.clone(), thread.clone(), window, cx);
        thread
    })
}

/// The comment on this quoted passage.
fn comment_on(cx: &mut TestAppContext, f: &Fixture, quote: &str) -> String {
    cx.read(|cx| {
        f.view
            .read(cx)
            .workspace
            .writing_comments(&f.note.id)
            .unwrap()
            .into_iter()
            .find(|entry| matches!(&entry.record.data, RecordData::Comment(c) if c.quote == quote))
            .unwrap()
            .record
            .id
    })
}

fn comment_ids(cx: &mut TestAppContext, f: &Fixture) -> Vec<String> {
    cx.read(|cx| {
        f.view
            .read(cx)
            .workspace
            .writing_comments(&f.note.id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.record.id)
            .collect()
    })
}

/// A rewrite proposal for the note that captures every comment, opened in
/// review.
fn review_rewrite(cx: &mut TestAppContext, f: &Fixture, thread: &str, text: &str) -> Prepared {
    let ids = comment_ids(cx, f);
    update(cx, f, |view, window, cx| {
        let note = view.workspace.store.note(&f.note.id).unwrap().unwrap();
        let key = format!("s4-request-{}", uuid::Uuid::new_v4());
        let snapshot = view
            .workspace
            .snapshot_writing_request(&key, &note.id, note.version, thread, &ids)
            .unwrap();
        let operation = view
            .workspace
            .store
            .allocate_operation(&format!("s4-proposal-{}", uuid::Uuid::new_v4()))
            .unwrap();
        let candidate = view
            .workspace
            .store
            .prepare(&operation, &snapshot.change_request(text).unwrap())
            .unwrap();
        view.refresh();
        view.open_review(candidate.clone(), window, cx);
        candidate
    })
}

/// Prepare and open a proposal with exactly these writes; new records take
/// their IDs from the proposal's operation.
fn review_writes(
    cx: &mut TestAppContext,
    f: &Fixture,
    reason: &str,
    writes: impl FnOnce(&OperationId) -> Vec<Put>,
) {
    update(cx, f, |view, window, cx| {
        let operation = view
            .workspace
            .store
            .allocate_operation(&format!("s4-writes-{}", uuid::Uuid::new_v4()))
            .unwrap();
        let writes = writes(&operation);
        let candidate = view
            .workspace
            .store
            .prepare(
                &operation,
                &ChangeRequest {
                    reason: reason.into(),
                    writes,
                    inputs: vec![],
                },
            )
            .unwrap();
        view.refresh();
        view.open_review(candidate, window, cx);
    });
}

fn note_put(record: &Record, markdown: &str) -> Put {
    let RecordData::Note(mut note) = record.data.clone() else {
        unreachable!()
    };
    note.markdown = markdown.into();
    Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived: false,
        data: RecordData::Note(note),
    }
}

/// Two open tasks, saved, as proposal targets.
fn tasks(cx: &mut TestAppContext, f: &Fixture) -> Vec<Record> {
    update(cx, f, |view, _, _| {
        ["Call the venue", "Book the room"]
            .into_iter()
            .map(|description| {
                let key = format!("s4-task-{description}");
                let id = view
                    .workspace
                    .store
                    .allocate_operation(&key)
                    .unwrap()
                    .creation_id(0);
                view.workspace
                    .owner_change(
                        &key,
                        &ChangeRequest {
                            reason: "Synthetic task".into(),
                            writes: vec![Put {
                                id: id.clone(),
                                expected_version: None,
                                archived: false,
                                data: RecordData::Action(Action {
                                    kept_outside_project: false,
                                    chat: None,
                                    description: description.into(),
                                    state: ActionState::Open,
                                    internal: false,
                                    evidence: None,
                                    due: None,
                                }),
                            }],
                            inputs: vec![],
                        },
                    )
                    .unwrap();
                view.workspace.store.record(&id).unwrap().unwrap()
            })
            .collect()
    })
}

fn done(task: &Record) -> Put {
    let RecordData::Action(mut action) = task.data.clone() else {
        unreachable!()
    };
    action.state = ActionState::Done;
    action.evidence = Some("Owner confirmed it".into());
    Put {
        id: task.id.clone(),
        expected_version: Some(task.version),
        archived: false,
        data: RecordData::Action(action),
    }
}

fn px_of(value: gpui_kit::Pixels) -> f32 {
    f32::from(value)
}

/// Every control is inside the bar, which has one 36 px row.
fn assert_one_row(window: &Window, ids: &[&str]) {
    let bar = window.find("review-bar").bounds();
    assert_eq!(bar.size.height, px(36.), "{bar:?}");
    for id in ids {
        assert_in(window, id, bar);
    }
}

/// Every card renders in the section's list: the topmost in full, the rest
/// below it in the capped, scrolling list.
fn assert_cards_in(window: &Window, ids: &[String], section: Bounds<gpui_kit::Pixels>) {
    let cards: Vec<_> = ids
        .iter()
        .map(|id| (id, window.find(format!("comment-card-{id}")).bounds()))
        .collect();
    for (id, card) in &cards {
        assert!(
            card.left() >= section.left()
                && card.right() <= section.right()
                && card.top() >= section.top(),
            "{id} {card:?} in {section:?}"
        );
    }
    let (top, _) = cards
        .iter()
        .min_by(|a, b| a.1.top().partial_cmp(&b.1.top()).unwrap())
        .unwrap();
    assert_in(window, &format!("comment-card-{top}"), section);
}

/// The draft's passage and actions are in view at the top of the section.
fn assert_draft_in(window: &Window, section: Bounds<gpui_kit::Pixels>) {
    let composer = window.find("comment-composer").bounds();
    assert!(composer.top() >= section.top(), "{composer:?} {section:?}");
    assert_in(window, "cancel-comment", section);
    assert_in(window, "post-comment", section);
}

/// `primary` is the bar's last control: right after `before` on its row, or
/// on the next row when a crowded state wraps.
fn assert_after(window: &Window, before: &str, primary: &str) {
    let (a, b) = (
        window.find(before.to_owned()).bounds(),
        window.find(primary.to_owned()).bounds(),
    );
    let same_row = b.top() < a.bottom() && a.top() < b.bottom();
    assert!(
        (same_row && b.left() >= a.right()) || b.top() >= a.bottom(),
        "{before} {a:?} then {primary} {b:?}"
    );
}

fn select(cx: &mut TestAppContext, f: &Fixture, word: &str) {
    update(cx, f, |view, _, cx| {
        let text = view.note_editor.read(cx).value().to_string();
        let start = text.find(word).unwrap();
        view.note_editor.update(cx, |editor, cx| {
            editor.set_selected_range(start..start + word.len(), cx)
        });
        view.track_selection(cx);
    });
}

#[test]
fn proposal_shapes_name_the_label_primary_and_exact_footer() {
    let note = |id: &str| Put {
        id: id.into(),
        expected_version: Some(1),
        archived: false,
        data: RecordData::Note(Note::working("A", "text")),
    };
    let task = Put {
        id: "task".into(),
        expected_version: Some(1),
        archived: false,
        data: RecordData::Action(Action {
            kept_outside_project: false,
            chat: None,
            description: "Task".into(),
            state: ActionState::Done,
            internal: false,
            evidence: None,
            due: None,
        }),
    };
    assert_eq!(ProposalShape::of(&[note("a")]), ProposalShape::Note);
    assert_eq!(
        ProposalShape::of(&[note("a"), note("b")]),
        ProposalShape::Notes(2)
    );
    assert_eq!(
        ProposalShape::of(&[note("a"), task.clone()]),
        ProposalShape::Changes(2)
    );
    assert_eq!(
        ProposalShape::of(std::slice::from_ref(&task)),
        ProposalShape::Changes(1)
    );
    for (shape, protected, label, primary, footer) in [
        (
            ProposalShape::Note,
            false,
            "whole note",
            "Apply whole note",
            "Applying saves exactly this text.",
        ),
        (
            ProposalShape::Note,
            true,
            "whole note",
            "Approve whole note",
            "Approving saves exactly this text to a protected note.",
        ),
        (
            ProposalShape::Notes(3),
            false,
            "3 notes",
            "Apply 3 notes",
            "Applying saves exactly these 3 notes.",
        ),
        (
            ProposalShape::Notes(3),
            true,
            "3 notes",
            "Approve 3 notes",
            "Approving saves exactly these 3 notes, including protected notes.",
        ),
        (
            ProposalShape::Changes(2),
            false,
            "2 changes",
            "Apply 2 changes",
            "Applying saves exactly these 2 changes.",
        ),
        (
            ProposalShape::Changes(2),
            true,
            "2 changes",
            "Approve 2 changes",
            "Approving saves exactly these 2 changes, including a protected note.",
        ),
    ] {
        assert_eq!(shape.label(), label);
        assert_eq!(shape.primary(protected), primary);
        assert_eq!(review_footer(shape, protected, 0), footer);
    }
    assert_eq!(
        review_footer(ProposalShape::Note, false, 1),
        "Applying saves exactly this text. 1 comment stays open until you mark it addressed."
    );
    assert_eq!(
        review_footer(ProposalShape::Note, true, 2),
        PROTECTED_FOOTER
    );
}

/// M1 and N2 at 900×600 in Write: the comment toggle sits in the one-row
/// review bar, opens the stacked section between the review text and the
/// footer, and Escape closes it again with focus back on the review text.
#[gpui_kit::test]
fn review_bar_keeps_one_row_and_opens_stacked_comments_above_the_footer_at_900x600(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, 900., 600., TEXT);
    cx.update(|cx| cx.bind_keys(bindings()));
    let thread = commented(cx, &f, true);
    let comments = comment_ids(cx, &f);
    review_rewrite(cx, &f, &thread, REVISED);
    let screen = window_bounds(900., 600.);
    let bar_ids = [
        "review-proposal",
        "review-protected",
        "review-show-changes",
        "review-read-result",
        "toggle-margin",
        "apply-final",
    ];
    on_window(cx, &f, |window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.margin_mode, MarginMode::Stacked);
        assert_eq!(view.review_fit, ReviewFit::Icons);
        // The review bar replaces the toolbar; only one of them renders.
        assert!(window.try_find("note-toolbar").is_none());
        assert_one_row(window, &bar_ids);
        let bar = window.find("review-bar").bounds();
        // The toggle is the last control before the primary.
        let toggle = window.find("toggle-margin").bounds();
        let apply = window.find("apply-final").bounds();
        assert!((px_of(apply.left() - toggle.right()) - 4.).abs() < 0.5);
        assert_eq!(window.find("toggle-margin").label(), Some("Comments · 2"));
        // Measured fit: the controls and gaps, without the flexible space,
        // against the width inside the bar's padding.
        let first = window.find("review-proposal").bounds().left();
        let space = toggle.left() - window.find("review-read-result").bounds().right() - px(8.);
        let used = px_of(apply.right() - first - space);
        let usable = px_of(bar.size.width) - 16.;
        assert!(used <= usable, "{used} of {usable}");
        // Keep for later and Dismiss live in the Note ⋯ at this width.
        assert!(window.try_find("close-review").is_none());
        assert!(window.try_find("dismiss-proposal").is_none());
        // Narrow labels keep their full names.
        let proposal = window.find("review-proposal");
        assert!(
            proposal
                .label()
                .is_some_and(|label| label.starts_with("Proposal · whole note · +")),
            "{proposal:?}"
        );
        assert_eq!(window.find("review-protected").label(), Some("Protected"));
        assert_eq!(
            window.find("apply-final").label(),
            Some("Approve whole note")
        );
        // The footer says exactly what Approve saves, whole.
        assert_eq!(window.find("note-bar").label(), Some(PROTECTED_FOOTER));
        assert_in(window, "note-bar", window.find("tile-note").bounds());
        assert_in(window, "note-bar", screen);
        assert!(window.try_find("comments-section").is_none());
        assert!(window.try_find("writing-feedback").is_none());
    });
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    let rows = menu_rows(cx, &f);
    assert_eq!(
        rows[..5],
        [
            "Compare side by side",
            "Edit final text",
            "Inputs",
            "Keep for later",
            "Dismiss"
        ],
        "{rows:?}"
    );
    on_window(cx, &f, |window, cx| window.press("escape", cx));

    // The toggle opens the section between the review text and the footer.
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).comments_expanded);
        assert_one_row(window, &bar_ids);
        let text = window.find("review-body").bounds();
        let section = window.find("comments-section").bounds();
        let footer = window.find("note-bar").bounds();
        assert!(text.bottom() <= section.top(), "{text:?} {section:?}");
        assert!(section.bottom() <= footer.top(), "{section:?} {footer:?}");
        assert_cards_in(window, &comments, section);
        assert_in(window, "note-bar", window.find("tile-note").bounds());
        assert_eq!(window.find("note-bar").label(), Some(PROTECTED_FOOTER));
        // Comments never render in Chat.
        let chat = window.within("tile-chat");
        assert!(
            comments
                .iter()
                .all(|id| chat.try_find(format!("comment-card-{id}")).is_none())
        );
    });

    // A kept draft has focus inside the section; Escape collapses it, keeps
    // the draft and returns focus to the review text.
    update(cx, &f, |view, window, cx| {
        let start = TEXT.find("brown").unwrap();
        let version = view.note.as_ref().unwrap().record.version;
        view.comment_anchor = Some((f.note.id.clone(), version, start..start + 5, "brown".into()));
        view.comment_input.update(cx, |editor, cx| {
            editor.set_value("Draft kept", window, cx);
            editor.focus(window, cx);
        });
    });
    on_window(cx, &f, |window, _| {
        assert_draft_in(window, window.find("comments-section").bounds())
    });
    on_window(cx, &f, |window, cx| window.press("escape", cx));
    update(cx, &f, |view, window, cx| {
        assert!(!view.comments_expanded);
        assert!(view.review.is_some(), "Escape never leaves the review");
        assert!(
            view.diff_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        assert!(view.comment_anchor.is_some());
        assert_eq!(view.comment_input.read(cx).value().as_ref(), "Draft kept");
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("comments-section").is_none())
    });
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.find("comments-section").visible());
        assert!(window.find("comment-composer").visible());
        assert_in(window, "note-bar", window.find("tile-note").bounds());
    });

    // In Result, Escape inside the section returns focus to the formatted
    // proposal.
    on_window(cx, &f, |window, cx| window.click("review-read-result", cx));
    escape_from_comments(cx, &f);
    update(cx, &f, |view, window, cx| {
        assert!(view.review_reading && !view.comments_expanded);
        assert!(
            view.candidate_view
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(view.comment_input.read(cx).value().as_ref(), "Draft kept");
    });
    // While editing the final text, it returns focus to the text being
    // edited.
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Edit final text");
    escape_from_comments(cx, &f);
    update(cx, &f, |view, window, cx| {
        assert!(view.review_editing && !view.comments_expanded);
        assert!(
            view.review_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(view.comment_input.read(cx).value().as_ref(), "Draft kept");
    });
}

/// Open the stacked section, focus the kept draft inside it and press
/// Escape.
fn escape_from_comments(cx: &mut TestAppContext, f: &Fixture) {
    if !cx.read(|cx| f.view.read(cx).comments_expanded) {
        on_window(cx, f, |window, cx| window.click("toggle-margin", cx));
    }
    update(cx, f, |view, window, cx| {
        assert!(view.comments_expanded);
        view.comment_input
            .update(cx, |editor, cx| editor.focus(window, cx));
    });
    on_window(cx, f, |window, _| {
        assert!(window.find("comment-composer").visible())
    });
    on_window(cx, f, |window, cx| window.press("escape", cx));
}

/// The icon arrangement still fits one row at 900×600 with a two-digit
/// comment count on a protected note.
#[gpui_kit::test]
fn review_bar_keeps_one_row_with_twelve_comments_at_900x600(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., TEXT);
    let thread = commented(cx, &f, true);
    update(cx, &f, |view, _, cx| {
        let version = view
            .workspace
            .store
            .note(&f.note.id)
            .unwrap()
            .unwrap()
            .version;
        for index in 0..10 {
            view.workspace
                .add_writing_comment(
                    &format!("s4-more-{index}"),
                    &WritingCommentInput {
                        note: f.note.id.clone(),
                        version,
                        thread: thread.clone(),
                        range: (2, 7),
                        quote: "Pilot".into(),
                        body: format!("More {index}"),
                    },
                )
                .unwrap();
        }
        view.refresh();
        cx.notify();
    });
    review_rewrite(cx, &f, &thread, REVISED);
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("toggle-margin").label(), Some("Comments · 12"));
        assert_one_row(
            window,
            &[
                "review-proposal",
                "review-protected",
                "review-show-changes",
                "review-read-result",
                "toggle-margin",
                "apply-final",
            ],
        );
        let bar = window.find("review-bar").bounds();
        let toggle = window.find("toggle-margin").bounds();
        let apply = window.find("apply-final").bounds();
        let first = window.find("review-proposal").bounds().left();
        let space = toggle.left() - window.find("review-read-result").bounds().right() - px(8.);
        let used = px_of(apply.right() - first - space);
        assert!(used <= px_of(bar.size.width) - 16., "{used} of {bar:?}");
    });
}

/// A whole-note rewrite of the open note with overall feedback, plus the
/// other writes given, opened in review.
fn review_rewrite_with(
    cx: &mut TestAppContext,
    f: &Fixture,
    thread: &str,
    feedback: &str,
    also: Vec<Put>,
) {
    let ids = comment_ids(cx, f);
    update(cx, f, |view, window, cx| {
        let note = view.workspace.store.note(&f.note.id).unwrap().unwrap();
        let key = format!("s4-request-{}", uuid::Uuid::new_v4());
        let snapshot = view
            .workspace
            .snapshot_writing_request_with_feedback(
                &key,
                &note.id,
                note.version,
                thread,
                &ids,
                feedback,
            )
            .unwrap();
        let mut request = snapshot.change_request(REVISED).unwrap();
        request.writes.extend(also);
        let operation = view
            .workspace
            .store
            .allocate_operation(&format!("s4-proposal-{}", uuid::Uuid::new_v4()))
            .unwrap();
        let candidate = view.workspace.store.prepare(&operation, &request).unwrap();
        view.refresh();
        view.open_review(candidate, window, cx);
    });
}

/// The exact-text footer is whole inside the Note panel and the window,
/// with its full text.
fn assert_footer_whole(window: &Window, footer: &str, case: &str) {
    assert_eq!(window.find("note-bar").label(), Some(footer), "{case}");
    assert_in(window, "note-bar", window.find("tile-note").bounds());
    assert!(
        window.find("note-bar").bounds().bottom() <= px(600.),
        "{case}"
    );
}

/// The details give way, then the text, while the open section keeps its
/// label and first card's first line.
fn assert_shared_height(window: &Window, comments: &[String], case: &str) {
    let details = window.find("review-details").bounds();
    assert!(details.size.height >= px(44.), "{case}: {details:?}");
    let text = window.find("review-body").bounds();
    assert!(text.size.height >= px(119.), "{case}: {text:?}");
    let section = window.find("comments-section").bounds();
    assert!(
        section.size.height >= COMMENTS_FLOOR - px(1.),
        "{case}: {section:?}"
    );
    let first = comments
        .iter()
        .map(|id| window.find(format!("comment-card-{id}")).bounds())
        .min_by(|a, b| a.top().partial_cmp(&b.top()).unwrap())
        .unwrap();
    assert!(
        first.top() + px(28.) <= section.bottom(),
        "{case}: {first:?} in {section:?}"
    );
    let footer = window.find("note-bar").bounds();
    assert!(
        details.bottom() <= text.top() && text.bottom() <= section.top(),
        "{case}"
    );
    assert!(section.bottom() <= footer.top(), "{case}");
}

/// The footer never moves: at 900×600 and 850×600, with Inputs, long
/// overall feedback and the stacked comments open, the details give way
/// first, so the exact-text footer stays whole for a several-notes
/// protected review and while editing the final text.
#[gpui_kit::test]
fn review_footer_stays_whole_with_inputs_and_comments_open_at_600_tall(cx: &mut TestAppContext) {
    let feedback = "Tighten the opening, keep the pilot's dates and owners exactly as written, name the venue once, and say who confirms the booking and by when. ".repeat(3);
    for width in [900., 850.] {
        // Several notes, one of them protected, every group included.
        let f = fixture(cx, width, 600., TEXT);
        let thread = commented(cx, &f, true);
        let comments = comment_ids(cx, &f);
        let other = update(cx, &f, |view, _, _| {
            view.workspace.new_note("Second", "Second text").unwrap()
        });
        review_rewrite_with(
            cx,
            &f,
            &thread,
            &feedback,
            vec![note_put(&other, "Second text, tidied")],
        );
        on_window(cx, &f, |window, cx| window.click("note-menu", cx));
        choose(cx, &f, "Inputs");
        on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
        let case = format!("{width}x600, 2 notes");
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            assert!(view.review_inputs && view.comments_expanded, "{case}");
            assert_eq!(view.review_fit, ReviewFit::Icons, "{case}");
            assert_footer_whole(
                window,
                "Approving saves exactly these 2 notes, including protected notes. 2 comments stay open until you mark them addressed.",
                &case,
            );
            assert_shared_height(window, &comments, &case);
            // The waiting Prepare selection sits with the group toggles in
            // the scrolling details, so the bar keeps one row at 900×600.
            assert_eq!(
                window.find("prepare-selected").label(),
                Some("Prepare selection, unavailable: Omit a group first")
            );
            let details = window.within("review-details");
            assert!(details.try_find("prepare-selected").is_some(), "{case}");
            assert!(details.try_find("review-members").is_some(), "{case}");
            assert!(
                window
                    .within("review-bar")
                    .try_find("prepare-selected")
                    .is_none(),
                "{case}"
            );
            assert_eq!(window.find("apply-final").label(), Some("Approve 2 notes"));
            if width == 900. {
                assert_one_row(
                    window,
                    &[
                        "review-proposal",
                        "review-protected",
                        "review-show-changes",
                        "review-read-result",
                        "toggle-margin",
                        "apply-final",
                    ],
                );
            }
        });

        // Editing the final text with Inputs open.
        let f = fixture(cx, width, 600., TEXT);
        let thread = commented(cx, &f, true);
        let comments = comment_ids(cx, &f);
        review_rewrite_with(cx, &f, &thread, &feedback, vec![]);
        on_window(cx, &f, |window, cx| window.click("note-menu", cx));
        choose(cx, &f, "Inputs");
        on_window(cx, &f, |window, cx| window.click("note-menu", cx));
        choose(cx, &f, "Edit final text");
        on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
        let case = format!("{width}x600, editing");
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            assert!(view.review_inputs && view.review_editing, "{case}");
            assert!(view.comments_expanded, "{case}");
            assert_footer_whole(window, PROTECTED_FOOTER, &case);
            assert_shared_height(window, &comments, &case);
            assert!(window.find("review-blocked-reason").visible(), "{case}");
            assert_in(window, "prepare-final", window.find("review-bar").bounds());
            assert_after(window, "toggle-margin", "prepare-final");
        });
    }
}

/// At 1100×760 and 1250×860 the review shows the comments beside the text,
/// and the same toggle in the review bar hides and shows them.
#[gpui_kit::test]
fn review_comments_render_beside_the_text_at_1250x860(cx: &mut TestAppContext) {
    for (width, height, mode, fit) in [
        (1100., 760., MarginMode::Narrow, ReviewFit::Icons),
        (1250., 860., MarginMode::Wide, ReviewFit::Compact),
    ] {
        let f = fixture(cx, width, height, TEXT);
        let thread = commented(cx, &f, false);
        let comments = comment_ids(cx, &f);
        review_rewrite(cx, &f, &thread, REVISED);
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            assert_eq!((view.margin_mode, view.review_fit), (mode, fit), "{width}");
            let margin = window.find("writing-feedback").bounds();
            assert_eq!(margin.size.width, px(mode.width()));
            assert!(margin.left() >= window.find("review-body").bounds().left());
            for id in &comments {
                assert_in(window, &format!("comment-card-{id}"), margin);
            }
            assert!(window.try_find("comments-section").is_none());
            assert_one_row(
                window,
                &[
                    "review-proposal",
                    "review-show-changes",
                    "review-read-result",
                    "toggle-margin",
                    "apply-final",
                ],
            );
            assert_after(window, "toggle-margin", "apply-final");
            assert_eq!(window.find("apply-final").label(), Some("Apply whole note"));
            assert_eq!(
                window.find("note-bar").label(),
                Some(
                    "Applying saves exactly this text. 2 comments stay open until you mark them addressed."
                )
            );
        });
        on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
        on_window(cx, &f, |window, _| {
            assert!(window.try_find("writing-feedback").is_none())
        });
        on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
        on_window(cx, &f, |window, _| {
            assert!(window.find("writing-feedback").visible())
        });
    }
}

/// The review bar's state table: one primary per state, blocked controls
/// with their reasons, and the footer in every state.
#[gpui_kit::test]
fn review_states_keep_one_primary_and_the_exact_footer(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., TEXT);
    let thread = commented(cx, &f, true);
    review_rewrite(cx, &f, &thread, REVISED);
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("apply-final").label(),
            Some("Approve whole note")
        );
        assert_eq!(window.find("review-protected").label(), Some("Protected"));
        assert!(window.try_find("review-blocked-reason").is_none());
    });
    // Editing: Use this text is the primary, disabled until edited, and
    // Apply waits; the footer stays.
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Edit final text");
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("prepare-final").label(),
            Some("Use this text, unavailable: Edit the text first")
        );
        assert_after(window, "toggle-margin", "prepare-final");
        assert_eq!(
            window.find("apply-final").label(),
            Some("Approve whole note, unavailable: Finish or cancel the edit first")
        );
        assert_eq!(
            window.find("review-show-changes").label(),
            Some("Changes, unavailable: Finish or cancel the edit first")
        );
        assert!(window.find("cancel-final").visible());
        assert!(window.find("review-blocked-reason").visible());
        assert_eq!(window.find("note-bar").label(), Some(PROTECTED_FOOTER));
    });
    update(cx, &f, |view, window, cx| {
        view.review_editor.update(cx, |editor, cx| {
            editor.set_value(format!("{REVISED}\n\nAdded."), window, cx)
        });
        if let Some(review) = &mut view.review {
            review.edited = true;
        }
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("prepare-final").label(), Some("Use this text"));
    });
    on_window(cx, &f, |window, cx| window.click("cancel-final", cx));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(!view.review_editing && !view.review.as_ref().unwrap().edited);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("prepare-final").is_none());
        assert_eq!(
            window.find("apply-final").label(),
            Some("Approve whole note")
        );
    });
    // Changes | Result keep their focus.
    on_window(cx, &f, |window, cx| window.click("review-read-result", cx));
    update(cx, &f, |view, window, cx| {
        assert!(view.review_reading);
        assert!(
            view.candidate_view
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    on_window(cx, &f, |window, cx| window.click("review-show-changes", cx));
    update(cx, &f, |view, window, cx| {
        assert!(!view.review_reading);
        assert!(
            view.diff_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    // Keep for later closes the review and keeps the proposal; Dismiss
    // removes it and leaves the note unchanged. Both are in the Note ⋯ here.
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Keep for later");
    let kept = cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.review.is_none());
        assert_eq!(view.candidates.len(), 1);
        view.candidates[0].clone()
    });
    update(cx, &f, |view, window, cx| {
        view.open_review(kept, window, cx)
    });
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Dismiss");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.review.is_none());
        assert!(view.candidates.is_empty());
        let RecordData::Note(saved) = view.workspace.store.note(&f.note.id).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(saved.markdown, TEXT);
    });

    // Several notes: a member switcher and "these N notes".
    let other = update(cx, &f, |view, _, _| {
        view.workspace.new_note("Second", "Second text").unwrap()
    });
    let first = cx.read(|cx| {
        f.view
            .read(cx)
            .workspace
            .store
            .note(&f.note.id)
            .unwrap()
            .unwrap()
    });
    review_writes(cx, &f, "Tidy both notes", |_| {
        vec![
            note_put(&first, REVISED),
            note_put(&other, "Second text, tidied"),
        ]
    });
    on_window(cx, &f, |window, _| {
        assert!(
            window
                .find("review-proposal")
                .label()
                .is_some_and(|label| label.starts_with("Proposal · 2 notes"))
        );
        assert_eq!(window.find("apply-final").label(), Some("Approve 2 notes"));
        assert_eq!(
            window.find("note-bar").label(),
            Some("Approving saves exactly these 2 notes, including protected notes.")
        );
        assert!(window.find("review-members").visible());
        assert!(window.find(format!("review-member-{}", other.id)).visible());
        // Every note is included, so Prepare selection waits and Apply leads.
        assert_eq!(
            window.find("prepare-selected").label(),
            Some("Prepare selection, unavailable: Omit a group first")
        );
        assert_after(window, "toggle-margin", "apply-final");
    });

    // A proposal without note text: N changes, its reason and details.
    let tasks = tasks(cx, &f);
    review_writes(cx, &f, "Close both tasks", |_| {
        tasks.iter().map(done).collect()
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("review-proposal").label(),
            Some("Proposal · 2 changes")
        );
        assert_eq!(
            window.find("review-reason").label(),
            Some("Close both tasks")
        );
        assert_eq!(window.find("apply-final").label(), Some("Apply 2 changes"));
        assert_eq!(
            window.find("note-bar").label(),
            Some("Applying saves exactly these 2 changes.")
        );
        assert!(window.find("review-details").visible());
        assert!(window.try_find("review-show-changes").is_none());
    });
    // Omitting a group makes Prepare selection the primary; Apply waits.
    on_window(cx, &f, |window, cx| {
        window.click(format!("review-group-{}", tasks[1].id), cx)
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("prepare-selected").label(),
            Some("Prepare selection")
        );
        assert_after(window, "apply-final", "prepare-selected");
        assert_eq!(
            window.find("apply-final").label(),
            Some("Apply 2 changes, unavailable: Prepare your selection first")
        );
        assert!(window.find("review-blocked-reason").visible());
        assert!(window.find("note-bar").visible());
    });
    on_window(cx, &f, |window, cx| window.click("prepare-selected", cx));
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("apply-final").label(), Some("Apply 1 change"));
        assert_eq!(
            window.find("note-bar").label(),
            Some("Applying saves exactly this change.")
        );
    });

    // Shared changes that can't be separated keep the toggles off.
    let tasks = update(cx, &f, |view, _, _| {
        tasks
            .iter()
            .map(|task| view.workspace.store.record(&task.id).unwrap().unwrap())
            .collect::<Vec<_>>()
    });
    review_writes(cx, &f, "Close both and say so", |operation| {
        let mut writes: Vec<Put> = tasks.iter().map(done).collect();
        writes.push(Put {
            id: operation.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread,
                role: "assistant".into(),
                text: "Both tasks closed together.".into(),
            }),
        });
        writes
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("review-inseparable").label(),
            Some("These changes can't be separated. Apply all or ask for a new rewrite.")
        );
        assert!(
            window
                .find(format!("review-group-{}", tasks[0].id))
                .label()
                .is_some_and(
                    |label| label.ends_with("unavailable: These changes can't be separated")
                )
        );
        assert_eq!(window.find("apply-final").label(), Some("Apply 3 changes"));
        assert_eq!(
            window.find("note-bar").label(),
            Some("Applying saves exactly these 3 changes.")
        );
    });
}

/// Comments sit beside the text when the Note has room and stack below it
/// otherwise; they never render in Chat.
#[gpui_kit::test]
fn comment_placement_follows_the_note_width(cx: &mut TestAppContext) {
    for (width, height, mode) in [
        (900., 600., MarginMode::Stacked),
        (1050., 700., MarginMode::Stacked),
        (1100., 760., MarginMode::Narrow),
        (1250., 860., MarginMode::Wide),
    ] {
        let f = fixture(cx, width, height, TEXT);
        commented(cx, &f, false);
        let comments = comment_ids(cx, &f);
        let case = format!("{width}x{height}");
        on_window(cx, &f, |window, cx| {
            assert_eq!(f.view.read(cx).margin_mode, mode, "{case}");
            let chat = window.within("tile-chat");
            assert!(
                comments
                    .iter()
                    .all(|id| chat.try_find(format!("comment-card-{id}")).is_none()),
                "{case}: no card in Chat"
            );
            if mode == MarginMode::Stacked {
                assert!(window.try_find("writing-feedback").is_none(), "{case}");
                assert!(window.try_find("comments-section").is_none(), "{case}");
            } else {
                let margin = window.find("writing-feedback").bounds();
                assert_eq!(margin.size.width, px(mode.width()), "{case}");
                assert!(
                    margin.left() >= window.find("note-body").bounds().right(),
                    "{case}: beside the text"
                );
                for id in &comments {
                    assert_in(window, &format!("comment-card-{id}"), margin);
                }
            }
        });
        if mode == MarginMode::Stacked {
            on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
            on_window(cx, &f, |window, _| {
                let text = window.find("note-body").bounds();
                let section = window.find("comments-section").bounds();
                assert!(text.bottom() <= section.top(), "{case}: below the text");
                // At most 40 % of the area it shares with the text, which
                // keeps at least 120 px.
                assert!(text.size.height >= px(120.), "{case}: {text:?}");
                let shared = px_of(section.bottom() - text.top());
                assert!(
                    px_of(section.size.height) <= shared * 0.4 + 0.5,
                    "{case}: {section:?} of {shared}"
                );
                assert!(section.bottom() <= window.find("note-bar").bounds().top());
                assert_cards_in(window, &comments, section);
                assert!(
                    window
                        .within("tile-chat")
                        .try_find("comments-section")
                        .is_none()
                );
            });
        }
    }
    // Solo gives the Note the whole row, so comments go beside the text.
    let f = fixture(cx, 900., 600., TEXT);
    commented(cx, &f, false);
    update(cx, &f, |view, window, cx| {
        view.layout.set_focus(TileKind::Note);
        view.apply_layout(Preset::Solo, window, cx);
    });
    on_window(cx, &f, |window, cx| {
        assert_eq!(f.view.read(cx).margin_mode, MarginMode::Wide);
        assert!(window.find("writing-feedback").visible());
    });
    // Without comments or a draft the margin stays closed; the toggle opens it.
    let f = fixture(cx, 1250., 860., TEXT);
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("writing-feedback").is_none());
        assert_eq!(window.find("toggle-margin").label(), Some("Comments"));
    });
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.find("writing-feedback").visible());
        assert!(window.find("margin-empty").visible());
    });
}

/// Every comment capability works in the stacked section at 900×600.
#[gpui_kit::test]
fn stacked_comments_keep_every_comment_capability(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., TEXT);
    cx.update(|cx| cx.bind_keys(bindings()));
    commented(cx, &f, false);
    let quick = comment_on(cx, &f, "quick");
    // A selected passage plus ⌘/ opens the section with a focused draft.
    select(cx, &f, "brown");
    on_window(cx, &f, |window, cx| window.press("cmd-/", cx));
    update(cx, &f, |view, window, cx| {
        assert_eq!(view.margin_mode, MarginMode::Stacked);
        assert!(view.comments_expanded);
        assert_eq!(view.comment_anchor.as_ref().unwrap().3, "brown");
        assert!(
            view.comment_input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        view.comment_input.update(cx, |editor, cx| {
            editor.set_value("Name the colour", window, cx)
        });
    });
    on_window(cx, &f, |window, _| {
        assert_draft_in(window, window.find("comments-section").bounds())
    });
    on_window(cx, &f, |window, cx| window.click("post-comment", cx));
    let all = comment_ids(cx, &f);
    assert_eq!(all.len(), 3);
    on_window(cx, &f, |window, _| {
        assert!(window.find("comments-section").visible());
        for id in &all {
            assert!(window.try_find(format!("comment-card-{id}")).is_some());
        }
    });

    // Escape inside the section collapses it, keeps the draft and returns
    // focus to the note text.
    select(cx, &f, "fox");
    on_window(cx, &f, |window, cx| window.press("cmd-/", cx));
    update(cx, &f, |view, window, cx| {
        view.comment_input
            .update(cx, |editor, cx| editor.set_value("Draft kept", window, cx));
    });
    on_window(cx, &f, |window, cx| window.press("escape", cx));
    update(cx, &f, |view, window, cx| {
        assert!(!view.comments_expanded);
        assert!(
            view.note_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(view.comment_anchor.as_ref().unwrap().3, "fox");
        assert_eq!(view.comment_input.read(cx).value().as_ref(), "Draft kept");
        assert!(view.worker.is_none(), "Escape never starts or stops work");
    });
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    on_window(cx, &f, |window, cx| {
        assert!(window.find("comment-composer").visible());
        window.click("cancel-comment", cx);
    });

    // Collapsing keeps the list's scroll position.
    update(cx, &f, |view, _, cx| {
        view.comments_scroll.scroll_to_bottom();
        cx.notify();
    });
    let offset = cx.read(|cx| f.view.read(cx).comments_scroll.offset());
    assert!(offset.y < px(0.), "{offset:?}");
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("comments-section").is_none())
    });
    on_window(cx, &f, |window, cx| window.click("toggle-margin", cx));
    cx.read(|cx| assert_eq!(f.view.read(cx).comments_scroll.offset(), offset));
    update(cx, &f, |view, _, cx| {
        view.comments_scroll.scroll_to_item(0);
        cx.notify();
    });

    // Show passage selects the passage above and leaves the section open.
    on_window(cx, &f, |window, cx| {
        window.click(format!("reveal-comment-{}", quick), cx)
    });
    update(cx, &f, |view, _, cx| {
        let editor = view.note_editor.read(cx);
        assert_eq!(editor.value().get(editor.selected_range()), Some("quick"));
        assert!(view.comments_expanded);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("comments-section").visible())
    });

    // Save and comment works while the note has an unsaved edit.
    type_edit(cx, &f, " Added.");
    select(cx, &f, "Added");
    on_window(cx, &f, |window, cx| {
        assert_eq!(
            window.find("selection-comment").label(),
            Some("Save and comment")
        );
        window.click("selection-comment", cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.note.as_ref().unwrap().session.is_none(), "saved first");
        assert_eq!(view.comment_anchor.as_ref().unwrap().3, "Added");
    });
    on_window(cx, &f, |window, cx| {
        assert_draft_in(window, window.find("comments-section").bounds());
        window.click("cancel-comment", cx);
    });

    // An edit that removes a passage leaves its comment unlocated: it is
    // kept, blocks the rewrite until excluded, and the owner reattaches the
    // feedback by commenting on the passage again.
    update(cx, &f, |view, window, cx| {
        let text = view.note_editor.read(cx).value().replace("quick", "slow");
        view.note_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        view.preserve_visible_note_input(cx);
        view.save(window, cx);
    });
    let version = cx.read(|cx| f.view.read(cx).note.as_ref().unwrap().record.version);
    on_window(cx, &f, |window, _| {
        let card = window.within(format!("comment-card-{}", quick));
        assert!(card.try_find(format!("comment-quote-{}", quick)).is_some());
        assert!(window.find("writing-blocked-reason").visible());
        assert_eq!(
            window.find("rewrite-note").label(),
            Some("Rewrite with AI, unavailable: Exclude or re-select unlocated comments")
        );
    });
    // Unlocated comments sort last; the owner scrolls the section to it.
    update(cx, &f, |view, _, cx| {
        view.comments_scroll.scroll_to_bottom();
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("comment-card-{}", quick)).visible());
        assert_eq!(
            window.find(format!("comment-status-{}", quick)).label(),
            Some(
                format!(
                    "Passage not found in v{version}. Exclude it, or select the passage and comment again."
                )
                .as_str()
            )
        );
    });
    on_window(cx, &f, |window, cx| {
        window.click(format!("include-comment-{}", quick), cx)
    });
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).excluded_comments.contains(&quick));
        assert!(window.try_find("writing-blocked-reason").is_none());
        assert_eq!(
            window.find("rewrite-note").label(),
            Some("Rewrite with AI ⌘⇧R")
        );
    });
    select(cx, &f, "slow");
    on_window(cx, &f, |window, cx| window.press("cmd-/", cx));
    update(cx, &f, |view, window, cx| {
        assert_eq!(view.comment_anchor.as_ref().unwrap().3, "slow");
        view.comment_input.update(cx, |editor, cx| {
            editor.set_value("Sharper verb", window, cx)
        });
    });
    on_window(cx, &f, |window, cx| window.click("post-comment", cx));

    // Rewrite with AI captures every included comment and the overall
    // feedback (offline, so no AI starts).
    update(cx, &f, |view, window, cx| {
        view.general_feedback.update(cx, |editor, cx| {
            editor.set_value("Tighten the opening.", window, cx)
        });
    });
    on_window(cx, &f, |window, cx| window.click("rewrite-note", cx));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_none());
        let request = view.latest_note_writing_request(&f.note.id).unwrap();
        let snapshot = view.workspace.writing_request(&request.id).unwrap();
        assert_eq!(snapshot.general_feedback, "Tighten the opening.");
        let captured: BTreeSet<String> = snapshot.comments.iter().map(|c| c.id.clone()).collect();
        let included: BTreeSet<String> = view
            .workspace
            .writing_comments(&f.note.id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.record.id)
            .filter(|id| !view.excluded_comments.contains(id))
            .collect();
        assert_eq!(captured, included);
        // Quick (excluded) stays out; Keep, brown and slow go in.
        assert_eq!(captured.len(), 3);
    });
}

/// N3: below the text come the comments, the run's status and reply, then
/// the writing bar; the text keeps 120 px and the section gives way first.
#[gpui_kit::test]
fn a_failed_run_and_open_comments_leave_the_text_120_px_at_900x600(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., TEXT);
    let thread = commented(cx, &f, false);
    update(cx, &f, |view, _, cx| view.rewrite_note(cx));
    let reason = "Run failed: Synthetic provider stopped before finishing the rewrite.";
    update(cx, &f, |view, _, cx| {
        let request = view.latest_note_writing_request(&f.note.id).unwrap().id;
        let key = "s4-failed-run";
        let operation = view.workspace.store.allocate_operation(key).unwrap();
        let settings = view.workspace.settings().unwrap();
        view.workspace
            .owner_change(
                key,
                &ChangeRequest {
                    reason: "Synthetic failed rewrite with a partial reply".into(),
                    inputs: vec![],
                    writes: vec![
                        Put {
                            id: operation.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Run(Run {
                                instruction: Some(request),
                                queue_claim: None,
                                thread: thread.clone(),
                                provider: "offline-fixture".into(),
                                model: settings.model,
                                guide_identity: "s4-fixture".into(),
                                budget: 1,
                                effort: settings.effort,
                                loaded_skills: vec![],
                                fence: 0,
                                state: RunState::Failed,
                                progress: reason.into(),
                                operations: vec![],
                                sources: vec![],
                            }),
                        },
                        Put {
                            id: operation.creation_id(1),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Message(Message {
                                thread: thread.clone(),
                                role: "assistant-partial".into(),
                                text: "A partial reply that stopped mid-sentence, kept for you to read before you retry the rewrite.".into(),
                            }),
                        },
                    ],
                },
            )
            .unwrap();
        view.refresh();
        view.comments_expanded = true;
        cx.notify();
    });
    let comments = comment_ids(cx, &f);
    let check = |window: &Window| {
        let tile = window.find("tile-note").bounds();
        let order = [
            "note-body",
            "comments-section",
            "writing-run-status",
            "writing-result",
            "note-bar",
        ];
        for pair in order.windows(2) {
            let (above, below) = (window.find(pair[0]).bounds(), window.find(pair[1]).bounds());
            assert!(
                above.bottom() <= below.top() + px(0.5),
                "{} {above:?} above {} {below:?}",
                pair[0],
                pair[1]
            );
        }
        // Everything but the section stays whole; the open section gave way
        // first.
        for id in order.into_iter().filter(|id| *id != "comments-section") {
            assert_in(window, id, tile);
        }
        let text = window.find("note-body").bounds();
        let section = window.find("comments-section").bounds();
        assert!(text.size.height >= px(120.), "{text:?}");
        // The section is at most 40 % of the area it shares with the text.
        let shared = section.bottom() - text.top();
        assert!(
            px_of(section.size.height) <= px_of(shared) * 0.4 + 0.5,
            "{section:?} of {shared:?}"
        );
        // An open section is never empty: at least its "Comments · 2" row
        // is in view, and the reply keeps its label.
        let label = window.find("comments-section-label").bounds();
        assert!(section.size.height >= px(24.), "{section:?}");
        assert!(label.top() >= section.top(), "{label:?} in {section:?}");
        assert!(window.find("writing-result").bounds().size.height >= px(32.));
        assert_eq!(
            window.find("writing-run-status").label(),
            Some(format!("Rewrite failed · {reason}").as_str())
        );
        section
    };
    // With the offline setup's two-line notice the Note has 358 px: the
    // reply gives way to its label and the section to its label row.
    on_window(cx, &f, |window, _| {
        assert!(window.find("workspace-notice").visible());
        check(window);
    });
    // Without it the section has its floor, the label and the first card's
    // first line, and the reply keeps a line.
    on_window(cx, &f, |window, cx| window.click("dismiss-notice", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("workspace-notice").is_none());
        let section = check(window);
        assert!(
            section.size.height >= COMMENTS_FLOOR - px(1.),
            "{section:?}"
        );
        let first = comments
            .iter()
            .map(|id| window.find(format!("comment-card-{id}")).bounds())
            .min_by(|a, b| a.top().partial_cmp(&b.top()).unwrap())
            .unwrap();
        assert!(
            first.top() + px(28.) <= section.bottom(),
            "{first:?} in {section:?}"
        );
        assert!(window.find("writing-result").bounds().size.height >= px(44.));
    });
}
