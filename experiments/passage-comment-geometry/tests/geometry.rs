//! Current BRN components on GPUI's headless test platform; synthetic data only.
use brn_passage_comment_geometry::*;
use brn_threads_core::*;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, Pixels, ScrollDelta, TestAppContext, WindowBounds,
    WindowOptions, point, px, size, test::TestWindowExt,
};
use std::ops::Range;

struct Fixture {
    window: AnyWindowHandle,
    host: Entity<Workshop>,
}
impl Fixture {
    fn new(
        cx: &mut TestAppContext,
        source: &str,
        comments: Vec<Comment>,
        width: f32,
        height: f32,
    ) -> Self {
        cx.update(gpui_kit::init);
        let (window, host) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(width), px(height)),
                    ))),
                    ..Default::default()
                },
                cx,
                |w, cx| cx.new(|cx| Workshop::new(source, comments, w, cx)),
            )
            .unwrap()
        });
        let f = Self { window, host };
        assert!(
            f.measure(cx).passage.is_none(),
            "no invented first-layout position"
        );
        f.frames(cx);
        f
    }
    fn frames(&self, cx: &mut TestAppContext) {
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(self.window, |_, w, cx| w.render_frame(cx))
                .unwrap();
        }
        cx.run_until_parked();
    }
    fn select(
        &self,
        index: usize,
        reveal: bool,
        cx: &mut TestAppContext,
    ) -> std::result::Result<(), Unavailable> {
        let result = self.host.update(cx, |h, cx| {
            let result = h.select_comment(index, reveal, cx);
            assert!(
                h.measurement.borrow().passage.is_none(),
                "previous position invalid before layout"
            );
            result
        });
        self.frames(cx);
        result
    }
    fn measure(&self, cx: &TestAppContext) -> Measurement {
        self.host
            .read_with(cx, |h, _| h.measurement.borrow().clone())
    }
    fn reader(&self, cx: &TestAppContext) -> Entity<gpui_kit::base::TextViewState> {
        self.host.read_with(cx, |h, _| h.reader.clone())
    }
    fn text(&self, cx: &TestAppContext) -> String {
        self.host
            .read_with(cx, |h, cx| h.editor.read(cx).value().to_string())
    }
    fn top(&self, cx: &TestAppContext) -> (usize, Pixels) {
        self.reader(cx).read_with(cx, |r, _| {
            let top = r.list_state().logical_scroll_top();
            (top.item_ix, top.offset_in_item)
        })
    }
    fn click(&self, id: &str, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, w, cx| w.click(id.to_owned(), cx))
            .unwrap();
        self.frames(cx);
    }
    fn press(&self, key: &str, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, w, cx| w.press(key, cx))
            .unwrap();
        self.frames(cx);
    }
    fn arm_reader(&self, range: Range<usize>, cx: &mut TestAppContext) {
        self.reader(cx)
            .update(cx, |r, cx| r.track_range_start(range, cx))
            .unwrap();
        self.frames(cx);
    }
}
fn last(source: &str, quote: &str) -> Range<usize> {
    let a = source.rfind(quote).unwrap();
    a..a + quote.len()
}
fn aligned(m: &Measurement) -> Bounds<Pixels> {
    let passage = m.passage.expect("visible passage-start glyph measured");
    let margin = m.margin.unwrap();
    let card = m.card.expect("card painted in same frame");
    assert!(passage.size.width > px(0.) && passage.size.height > px(0.));
    assert_eq!(card.origin.y, passage.origin.y);
    assert_eq!(m.passage_in_margin.unwrap(), passage.origin - margin.origin);
    assert_eq!(card.origin.x, margin.origin.x);
    assert!(
        margin.origin.x > px(0.) && margin.origin.y > px(0.),
        "nonzero pane origin tests conversion"
    );
    passage
}

#[gpui_kit::test]
fn exact_second_occurrence_unicode_formatting_and_crlf(cx: &mut TestAppContext) {
    for (source, quote, rendered_quote) in [
        (
            "Same paragraph.\n\nSame paragraph.",
            "Same paragraph.",
            "Same paragraph.",
        ),
        (
            "**õun ÕÄÖÜ šž**\n\n**õun ÕÄÖÜ šž**",
            "**õun ÕÄÖÜ šž**",
            "õun ÕÄÖÜ šž",
        ),
        ("*a\u{0308}*\n\n*a\u{0308}*", "a\u{0308}", "a\u{0308}"),
        ("👩🏽‍💻 🦀\n\n👩🏽‍💻 🦀", "👩🏽‍💻 🦀", "👩🏽‍💻 🦀"),
        (
            "**Same õun.**\r\n\r\n**Same õun.**",
            "Same õun.",
            "Same õun.",
        ),
        (
            "[õun](https://example.invalid)\n\n[õun](https://example.invalid)",
            "[õun](https://example.invalid)",
            "õun",
        ),
        ("`õun`\n\n`õun`", "`õun`", "õun"),
    ] {
        let b = last(source, quote);
        let a = source.find(quote).unwrap();
        let f = Fixture::new(
            cx,
            source,
            vec![
                synthetic_comment(source, a..a + quote.len(), "first"),
                synthetic_comment(source, b.clone(), "second"),
            ],
            850.,
            700.,
        );
        f.select(0, false, cx).unwrap();
        let first = aligned(&f.measure(cx));
        f.select(1, false, cx).unwrap();
        let second = aligned(&f.measure(cx));
        assert!(
            second.top() > first.bottom(),
            "must address second occurrence"
        );
        f.host.read_with(cx, |h, cx| {
            assert_eq!(h.valid_range(1, cx), Ok(b.clone()));
            let text = h.reader.read(cx).rendered_text();
            let range = text.range_for_source(b.clone()).unwrap();
            assert_eq!(&text.as_str()[range.clone()], rendered_quote);
            assert_eq!(range.start, text.as_str().rfind(rendered_quote).unwrap());
            assert_eq!(h.editor.read(cx).value().as_ref(), source);
            assert_eq!(h.generation, 0);
        });
        println!("exact {b:?} {quote:?}: first={first:?}; second={second:?}");
    }
}

#[gpui_kit::test]
fn passive_offscreen_none_then_comment_click_reveals_second_without_text_change(
    cx: &mut TestAppContext,
) {
    let (source, comments) = demo_source();
    let f = Fixture::new(cx, &source, comments, 850., 310.);
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    let initial = f.top(cx);
    f.select(1, false, cx).unwrap();
    assert!(f.measure(cx).passage.is_none());
    assert!(f.measure(cx).card.is_none());
    assert_eq!(f.top(cx), initial, "passive measuring never scrolls");
    f.click("comment-1", cx);
    let second = aligned(&f.measure(cx));
    assert!(f.top(cx).0 > 15, "revealed later occurrence");
    let later = f.top(cx);
    f.click("comment-0", cx);
    let first = aligned(&f.measure(cx));
    assert!(f.top(cx).0 < later.0);
    assert_eq!(f.text(cx), source);
    assert_eq!(f.host.read_with(cx, |h, _| h.generation), 0);
    println!(
        "comment clicks: first={first:?}, second={second:?}; list offsets {initial:?} -> {later:?} -> {:?}",
        f.top(cx)
    );
}

#[gpui_kit::test]
fn resize_rewrap_font_scroll_and_coordinate_conversion_use_current_frame(cx: &mut TestAppContext) {
    let source = format!(
        "{}\n\nTarget **õun**.",
        "Long preceding prose for wrapping with a\u{0308} and 🦀. ".repeat(9)
    );
    let range = last(&source, "õun");
    let f = Fixture::new(
        cx,
        &source,
        vec![synthetic_comment(&source, range, "target")],
        1000.,
        1300.,
    );
    f.select(0, false, cx).unwrap();
    let wide = aligned(&f.measure(cx));
    cx.simulate_window_resize(f.window, size(px(620.), px(1300.)));
    f.frames(cx);
    let narrow = aligned(&f.measure(cx));
    assert!(narrow.top() > wide.top());
    f.host.update(cx, |h, cx| {
        h.font_size = 23.;
        cx.notify();
    });
    f.frames(cx);
    let larger = aligned(&f.measure(cx));
    assert!(larger.top() > narrow.top());
    assert!(larger.size.height > narrow.size.height);
    cx.simulate_window_resize(f.window, size(px(620.), px(300.)));
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    assert!(
        f.measure(cx).card.is_none(),
        "never retain old visible anchor after resize"
    );
    f.click("comment-0", cx);
    let shown = aligned(&f.measure(cx));
    cx.update_window(f.window, |_, w, cx| {
        w.scroll(
            "document",
            ScrollDelta::Pixels(point(px(0.), px(10000.))),
            cx,
        )
    })
    .unwrap();
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    assert!(
        f.measure(cx).card.is_none(),
        "offscreen invalidates same-frame card"
    );
    println!(
        "wide={wide:?}; narrow={narrow:?}; larger={larger:?}; revealed={shown:?}; scrolled offscreen=None"
    );
}

#[gpui_kit::test]
fn invalid_identity_quote_version_and_unmapped_syntax_never_search(cx: &mut TestAppContext) {
    let source = "**same**\n\n**same**";
    let good = synthetic_comment(source, last(source, "same"), "second");
    let mut bad = good.clone();
    bad.range = Some((0, 4));
    let mut wrong_note = good.clone();
    wrong_note.note = "other".into();
    let mut stale = good.clone();
    stale.base_version = 2;
    let mut unresolved = good.clone();
    unresolved.unresolved = true;
    let syntax = synthetic_comment(source, 0..2, "syntax");
    let f = Fixture::new(
        cx,
        source,
        vec![good, bad, wrong_note, stale, unresolved, syntax],
        900.,
        500.,
    );
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    for i in 1..6 {
        assert!(f.select(i, false, cx).is_err());
        assert!(f.measure(cx).passage.is_none());
        assert!(f.measure(cx).card.is_none());
    }
    assert_eq!(f.select(5, false, cx), Err(Unavailable::Unmapped));
    println!(
        "wrong note, version, quote, unresolved and source-only Markdown delimiters -> explicit unavailable; no quote search"
    );
}

#[gpui_kit::test]
fn source_change_invalidates_even_when_viewer_remaps_highlights(cx: &mut TestAppContext) {
    let source = "first\n\n**target**";
    let f = Fixture::new(
        cx,
        source,
        vec![synthetic_comment(source, last(source, "target"), "target")],
        850.,
        500.,
    );
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    // Deliberately bypass host invalidation: toolkit snapshot must independently reject stale probes.
    f.reader(cx)
        .update(cx, |r, cx| r.set_text("prefix first\n\n**target**", cx));
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    assert!(f.measure(cx).card.is_none());
    // Source equality alone is insufficient: even reverting needs a validated re-registration.
    f.reader(cx).update(cx, |r, cx| r.set_text(source, cx));
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    f.click("write-toggle", cx);
    let editor = f.host.read_with(cx, |h, _| h.editor.clone());
    cx.update_window(f.window, |_, w, cx| {
        editor.update(cx, |e, cx| e.focus(w, cx))
    })
    .unwrap();
    cx.update_window(f.window, |_, w, cx| w.input("X", cx))
        .unwrap();
    f.frames(cx);
    assert_ne!(f.text(cx), source);
    assert!(f.host.read_with(cx, |h, _| h.generation) > 0);
    assert_eq!(f.select(0, false, cx), Err(Unavailable::DirtyDocument));
    f.click("write-toggle", cx);
    assert!(f.measure(cx).card.is_none());
    println!("source reparse, revert and real current-editor input invalidate; dirty text refused");
}

#[gpui_kit::test]
fn positioning_and_reveal_do_not_modify_current_editor_undo(cx: &mut TestAppContext) {
    let source = "Original **target**.";
    let f = Fixture::new(
        cx,
        source,
        vec![synthetic_comment(source, last(source, "target"), "target")],
        850.,
        500.,
    );
    for _ in 0..3 {
        f.click("comment-0", cx);
        aligned(&f.measure(cx));
    }
    assert_eq!(f.text(cx), source);
    assert_eq!(f.host.read_with(cx, |h, _| h.generation), 0);
    f.click("write-toggle", cx);
    let editor = f.host.read_with(cx, |h, _| h.editor.clone());
    cx.update_window(f.window, |_, w, cx| {
        editor.update(cx, |e, cx| e.focus(w, cx))
    })
    .unwrap();
    f.press("cmd-z", cx);
    assert_eq!(f.text(cx), source, "positioning adds no undo entry");
    cx.update_window(f.window, |_, w, cx| w.input("typed ", cx))
        .unwrap();
    f.frames(cx);
    let typed = f.text(cx);
    assert_ne!(typed, source);
    let generation = f.host.read_with(cx, |h, _| h.generation);
    f.click("write-toggle", cx);
    assert!(f.select(0, true, cx).is_err());
    f.frames(cx);
    assert_eq!(f.text(cx), typed);
    assert_eq!(f.host.read_with(cx, |h, _| h.generation), generation);
    f.click("write-toggle", cx);
    cx.update_window(f.window, |_, w, cx| {
        editor.update(cx, |e, cx| e.focus(w, cx))
    })
    .unwrap();
    f.press("cmd-z", cx);
    assert_eq!(
        f.text(cx),
        source,
        "comment display neither clears nor consumes typing undo"
    );
    f.press("cmd-shift-z", cx);
    assert_eq!(f.text(cx), typed);
    println!(
        "current editor: pristine Undo stays empty; typing Undo/Redo survives viewer/comment operations; only input changes generation"
    );
}

#[gpui_kit::test]
fn nontext_empty_and_newline_probe_never_borrow_neighbor_geometry(cx: &mut TestAppContext) {
    let source = "hello\n\nworld";
    let f = Fixture::new(
        cx,
        source,
        vec![synthetic_comment(source, 0..5, "hello")],
        850.,
        500.,
    );
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    let reader = f.reader(cx);
    assert!(
        reader
            .update(cx, |r, cx| r.track_range_start(5..5, cx))
            .is_err()
    );
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    assert!(
        reader
            .update(cx, |r, cx| r.track_range_start(5..6, cx))
            .is_err()
    );
    f.frames(cx);
    assert!(f.measure(cx).passage.is_none());
    // A hard newline within a text leaf has no glyph: exact passive measurement must report None.
    reader.update(cx, |r, cx| r.set_text("hello  \nworld", cx));
    f.frames(cx);
    let text = reader.read_with(cx, |r, _| r.rendered_text());
    let offset = text.as_str().find('\n').unwrap();
    f.arm_reader(offset..offset + 1, cx);
    assert!(f.measure(cx).passage.is_none());
    println!("empty/separator ranges rejected; hard newline reports None, not nearest glyph/caret");
}

#[gpui_kit::test]
fn checked_store_save_and_comment_mapping_remain_existing_core_behavior(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(dir.path().join("synthetic")).unwrap();
    let op = store.allocate_operation("geometry-fixture").unwrap();
    let note = op.creation_id(0);
    let comment_op = store.allocate_operation("geometry-comment").unwrap();
    let comment = comment_op.creation_id(0);
    let source = "same\n\nsame";
    let mut anchor = synthetic_comment(source, 6..10, "second");
    anchor.note = note.clone();
    let authority = HostAuthority::owner("test");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic note".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: note.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Synthetic", source)),
                }],
            },
            &authority,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &authority).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    store
        .prepare_owner(
            &comment_op,
            &ChangeRequest {
                reason: "Synthetic comment".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: comment.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Comment(anchor.clone()),
                }],
            },
            &authority,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&comment_op, &authority).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let draft = store.begin_edit(&note, 1).unwrap();
    let records = store.records().unwrap();
    let receipts = store.receipts().unwrap();
    let recovery = store.recovery_buffers().unwrap();
    let f = Fixture::new(cx, source, vec![anchor], 850., 500.);
    f.host.update(cx, |h, cx| {
        h.note_id = note.clone();
        cx.notify();
    });
    f.click("comment-0", cx);
    aligned(&f.measure(cx));
    cx.simulate_window_resize(f.window, size(px(650.), px(500.)));
    f.frames(cx);
    assert_eq!(store.records().unwrap(), records);
    assert_eq!(store.receipts().unwrap(), receipts);
    assert_eq!(store.recovery_buffers().unwrap(), recovery);
    assert_eq!(store.edit_session(&draft.id).unwrap(), draft);
    // Only explicit checked Save remaps; geometry doesn't introduce a mapping algorithm.
    let new = "prefix same\n\nsame";
    store.update_buffer(&draft.id, 1, new).unwrap();
    assert!(matches!(
        store.save(&draft.id, 0).unwrap(),
        SaveOutcome::GenerationMismatch(_)
    ));
    assert!(matches!(
        store.save(&draft.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    let RecordData::Comment(mapped) = store.record(&comment).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(exact_writing_range(&mapped, 2, new), Some((13, 17)));
    let draft2 = store.begin_edit(&note, 2).unwrap();
    store
        .update_buffer(&draft2.id, 1, "prefix same\n\nDIFFERENT")
        .unwrap();
    assert!(matches!(
        store.save(&draft2.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    let RecordData::Comment(unresolved) = store.record(&comment).unwrap().unwrap().data else {
        panic!()
    };
    assert!(unresolved.unresolved);
    assert_eq!(
        exact_writing_range(&unresolved, 3, "prefix same\n\nDIFFERENT"),
        None
    );
    println!(
        "real synthetic Store unchanged by display; stale generation Save refused; explicit Save maps second occurrence 6..10 -> 13..17; overlapping Save marks unresolved"
    );
}

#[gpui_kit::test]
fn current_viewer_nested_blocks_table_and_code_use_same_geometry_path(cx: &mut TestAppContext) {
    for source in [
        "# Heading\n\n> First **õun**.\n>\n> Second **õun**.",
        "- First **õun**.\n- Second **õun**.",
        "```text\nfirst õun\nsecond õun\n```",
        "| First | Second |\n|---|---|\n| õun | õun |",
    ] {
        let a = source.find("õun").unwrap();
        let b = last(source, "õun");
        let f = Fixture::new(
            cx,
            source,
            vec![
                synthetic_comment(source, a..a + "õun".len(), "first"),
                synthetic_comment(source, b.clone(), "second"),
            ],
            900.,
            700.,
        );
        f.select(0, false, cx).unwrap();
        let first = aligned(&f.measure(cx));
        f.select(1, false, cx).unwrap();
        let second = aligned(&f.measure(cx));
        assert!(second.top() > first.top() || second.left() > first.left());
        println!("nested/block/table source {b:?}: first={first:?}; second={second:?}");
    }
}

#[gpui_kit::test]
fn brn_read_fixtures_and_literal_html_policy_still_use_one_parser(cx: &mut TestAppContext) {
    for (source, quote) in [
        ("## Café 🦀\n\nA **formatted** paragraph.", "formatted"),
        ("é🦀 same — **é🦀 same**", "é🦀 same"),
        (
            "Intro with <span>inline</span> HTML.\n\n<div>literal block</div>\n\nAfter **formatted** text.",
            "formatted",
        ),
    ] {
        let range = last(source, quote);
        let f = Fixture::new(
            cx,
            source,
            vec![synthetic_comment(source, range.clone(), "comment")],
            900.,
            700.,
        );
        f.select(0, false, cx).unwrap();
        aligned(&f.measure(cx));
        let snapshot = f.reader(cx).read_with(cx, |r, _| r.rendered_text());
        assert_eq!(snapshot.source(), source);
        assert_eq!(
            &snapshot.as_str()[snapshot.range_for_source(range).unwrap()],
            quote
        );
        if source.contains("<div>") {
            assert!(snapshot.as_str().contains("<div>literal block</div>"));
        }
        assert_eq!(f.text(cx), source);
        println!(
            "BRN fixture: source={source:?}; rendered={:?}",
            snapshot.as_str()
        );
    }
}

#[gpui_kit::test]
fn silent_programmatic_editor_edit_invalidates_without_faking_draft_event(cx: &mut TestAppContext) {
    let source = "Original **target**.";
    let f = Fixture::new(
        cx,
        source,
        vec![synthetic_comment(source, last(source, "target"), "comment")],
        850.,
        500.,
    );
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    let editor = f.host.read_with(cx, |h, _| h.editor.clone());
    cx.update_window(f.window, |_, w, cx| {
        editor.update(cx, |e, cx| {
            e.set_value("prefix Original **target**.", w, cx)
        })
    })
    .unwrap();
    f.frames(cx);
    assert_ne!(f.text(cx), source);
    assert_eq!(
        f.host.read_with(cx, |h, _| h.generation),
        0,
        "programmatic notification is not a user Change event"
    );
    assert!(f.measure(cx).passage.is_none());
    assert!(f.measure(cx).card.is_none());
    assert_eq!(f.select(0, true, cx), Err(Unavailable::DirtyDocument));
    println!(
        "silent programmatic set_value: source observer clears anchor; no invented Change/draft generation event"
    );
}

/// Test-only navigation seam matching current open_note's programmatic load.
/// Production navigation itself is not modified by this experiment.
impl Fixture {
    fn switch_saved_document(
        &self,
        note: &str,
        source: &str,
        comments: Vec<Comment>,
        cx: &mut TestAppContext,
    ) {
        cx.update_window(self.window, |_, w, cx| {
            self.host.update(cx, |h, cx| {
                h.clear_comment_anchor(cx);
                h.note_id = note.into();
                h.version = 1;
                h.saved = source.into();
                h.comments = comments;
                h.editor.update(cx, |e, cx| e.set_value(source, w, cx));
                h.reader.update(cx, |r, cx| {
                    r.clear_selection(cx);
                    r.set_text(source, cx);
                });
                assert!(h.measurement.borrow().passage.is_none());
                assert!(h.measurement.borrow().card.is_none());
                cx.notify();
            });
        })
        .unwrap();
        self.frames(cx);
    }
}

#[gpui_kit::test]
fn document_switch_identical_text_and_changed_text_clear_old_anchor(cx: &mut TestAppContext) {
    let source = "**same õun**\n\n**same õun**";
    let a = synthetic_comment(source, last(source, "õun"), "note A second occurrence");
    let f = Fixture::new(cx, source, vec![a.clone()], 850., 500.);
    f.select(0, false, cx).unwrap();
    aligned(&f.measure(cx));
    let reader = f.reader(cx);
    let editor = f.host.read_with(cx, |h, _| h.editor.clone());
    let snapshot = reader.read_with(cx, |r, _| r.rendered_text());
    let mut b = a.clone();
    b.note = "note-B".into();
    // Same source, same version, same range, same quote and same component
    // entities: only document identity and explicit invalidation distinguish B.
    f.switch_saved_document("note-B", source, vec![a.clone(), b], cx);
    assert_eq!(f.reader(cx), reader);
    assert_eq!(f.host.read_with(cx, |h, _| h.editor.clone()), editor);
    assert_eq!(reader.read_with(cx, |r, _| r.rendered_text()), snapshot);
    assert!(f.measure(cx).passage.is_none());
    assert!(f.measure(cx).card.is_none());
    assert_eq!(f.select(0, true, cx), Err(Unavailable::WrongNoteOrVersion));
    f.select(1, true, cx).unwrap();
    let note_b = aligned(&f.measure(cx));
    assert_eq!(f.text(cx), source);
    assert_eq!(f.host.read_with(cx, |h, _| h.generation), 0);

    let changed = "New heading\n\nFirst **same õun**\n\nSecond **same õun**";
    let mut c = synthetic_comment(changed, last(changed, "õun"), "note C second occurrence");
    c.note = "note-C".into();
    f.switch_saved_document("note-C", changed, vec![a, c], cx);
    assert!(f.measure(cx).passage.is_none());
    assert!(f.measure(cx).card.is_none());
    assert_eq!(f.select(0, true, cx), Err(Unavailable::WrongNoteOrVersion));
    f.select(1, true, cx).unwrap();
    let note_c = aligned(&f.measure(cx));
    assert_ne!(note_b, note_c);
    assert_eq!(f.text(cx), changed);
    assert_eq!(f.host.read_with(cx, |h, _| h.generation), 0);
    println!(
        "navigation seam: identical-source note B retained the same parsed snapshot but cleared A's anchor; changed-source C also starts unavailable; old-note comment refused; B={note_b:?}, C={note_c:?}"
    );
}

#[gpui_kit::test]
fn armed_passage_has_no_report_before_first_paint_then_measures(cx: &mut TestAppContext) {
    use gpui_kit::{
        App, Context, IntoElement, Render, Styled, Window,
        base::{TextView, TextViewState},
    };
    use std::{cell::RefCell, rc::Rc};
    struct FirstPaint {
        reader: Entity<TextViewState>,
        reports: Rc<RefCell<Vec<Option<Bounds<Pixels>>>>>,
    }
    impl Render for FirstPaint {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let reports = self.reports.clone();
            TextView::new(&self.reader)
                .w_full()
                .h_full()
                .scrollable(true)
                .on_range_start_layout(move |geometry, _: &mut Window, _: &mut App| {
                    reports.borrow_mut().push(geometry)
                })
        }
    }
    cx.update(gpui_kit::init);
    let source = "First **õun**.\n\nSecond **õun**.";
    let reports = Rc::new(RefCell::new(Vec::new()));
    let reader = cx.new(|cx| TextViewState::markdown(source, cx).scrollable(true));
    reader.update(cx, |r, cx| {
        let text = r.rendered_text();
        assert_eq!(text.source(), source);
        let range = text.range_for_source(last(source, "õun")).unwrap();
        r.track_range_start(range, cx).unwrap();
    });
    assert!(
        reports.borrow().is_empty(),
        "armed probe has no geometry before a window/layout exists"
    );
    let (window, _) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| FirstPaint {
                reader: reader.clone(),
                reports: reports.clone(),
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, w, cx| w.render_frame(cx))
        .unwrap();
    assert!(!reports.borrow().is_empty());
    let first = reports.borrow()[0].expect("first painted frame supplies a visible glyph");
    assert!(first.size.width > px(0.) && first.size.height > px(0.));
    assert_eq!(
        reader.read_with(cx, |r, _| r.rendered_text().source().to_owned()),
        source
    );
    println!(
        "first layout: armed exact range before window creation -> no callback/position; first painted frame -> {first:?}"
    );
}

#[gpui_kit::test]
fn coupled_editor_store_witness_preserves_clean_dirty_generation_save_and_durable_undo(
    cx: &mut TestAppContext,
) {
    use gpui_kit::component::input::InputEvent;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let source = "Saved **õun** passage.";
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(dir.path().join("coupled-synthetic")).unwrap();
    let authority = HostAuthority::owner("geometry-witness");
    let op = store.allocate_operation("coupled-note").unwrap();
    let note = op.creation_id(0);
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic saved note".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: note.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Coupled witness", source)),
                }],
            },
            &authority,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &authority).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let store = Rc::new(RefCell::new(store));
    let session = Rc::new(RefCell::new(None::<EditSession>));
    let events = Rc::new(Cell::new(0usize));
    let f = Fixture::new(
        cx,
        source,
        vec![synthetic_comment(source, last(source, "õun"), "selected")],
        850.,
        500.,
    );
    // This is a coupled test witness, not a copy of ThreadsUi::note_changed.
    // A real editor Change enters the actual checked recovery-buffer APIs;
    // geometry must leave this live callback completely idle.
    let _subscription = f.host.update(cx, |h, cx| {
        let store = store.clone();
        let session = session.clone();
        let events = events.clone();
        let note = note.clone();
        cx.subscribe(&h.editor, move |_, editor, event, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            events.set(events.get() + 1);
            let text = editor.read(cx).value().to_string();
            let mut store = store.borrow_mut();
            let old = session
                .borrow()
                .clone()
                .unwrap_or_else(|| store.begin_edit(&note, 1).unwrap());
            if let BufferOutcome::Updated(next) = store
                .update_buffer(&old.id, old.generation + 1, &text)
                .unwrap()
            {
                *session.borrow_mut() = Some(next);
            }
        })
    });
    let clean_records = store.borrow().records().unwrap();
    let clean_receipts = store.borrow().receipts().unwrap();
    for _ in 0..3 {
        f.click("comment-0", cx);
        aligned(&f.measure(cx));
    }
    cx.simulate_window_resize(f.window, size(px(650.), px(500.)));
    f.frames(cx);
    f.host.update(cx, |h, cx| h.clear_comment_anchor(cx));
    f.frames(cx);
    assert_eq!(f.text(cx), source);
    assert_eq!(events.get(), 0);
    assert!(session.borrow().is_none());
    assert!(
        store.borrow().recovery_buffers().unwrap().is_empty(),
        "clean remains clean, no edit session created"
    );
    assert_eq!(store.borrow().records().unwrap(), clean_records);
    assert_eq!(store.borrow().receipts().unwrap(), clean_receipts);
    assert!(
        store.borrow().revision(&note, 2).unwrap().is_none(),
        "no implicit Save"
    );

    // Positive control: real input exercises the connected callback and SQLite.
    f.click("write-toggle", cx);
    let editor = f.host.read_with(cx, |h, _| h.editor.clone());
    cx.update_window(f.window, |_, w, cx| {
        editor.update(cx, |e, cx| e.focus(w, cx))
    })
    .unwrap();
    cx.update_window(f.window, |_, w, cx| w.input("typed ", cx))
        .unwrap();
    f.frames(cx);
    let typed = f.text(cx);
    assert_ne!(typed, source);
    assert!(events.get() > 0);
    let dirty = session.borrow().clone().unwrap();
    assert_eq!(dirty.markdown, typed);
    assert!(dirty.generation > 0);
    let dirty_events = events.get();
    let dirty_records = store.borrow().records().unwrap();
    let dirty_receipts = store.borrow().receipts().unwrap();
    let recovery = store.borrow().recovery_buffers().unwrap();
    f.click("write-toggle", cx);
    for _ in 0..3 {
        assert_eq!(f.select(0, true, cx), Err(Unavailable::DirtyDocument));
    }
    cx.simulate_window_resize(f.window, size(px(900.), px(450.)));
    f.frames(cx);
    f.host.update(cx, |h, cx| h.clear_comment_anchor(cx));
    f.frames(cx);
    assert_eq!(f.text(cx), typed);
    assert_eq!(events.get(), dirty_events);
    assert_eq!(store.borrow().edit_session(&dirty.id).unwrap(), dirty);
    assert_eq!(
        store.borrow().recovery_buffers().unwrap(),
        recovery,
        "dirty remains recoverable with same generation"
    );
    assert_eq!(store.borrow().records().unwrap(), dirty_records);
    assert_eq!(store.borrow().receipts().unwrap(), dirty_receipts);
    assert!(store.borrow().revision(&note, 2).unwrap().is_none());
    assert!(matches!(
        store
            .borrow_mut()
            .save(&dirty.id, dirty.generation - 1)
            .unwrap(),
        SaveOutcome::GenerationMismatch(_)
    ));
    let SaveOutcome::Saved(saved) = store
        .borrow_mut()
        .save(&dirty.id, dirty.generation)
        .unwrap()
    else {
        panic!("explicit Save must work")
    };
    assert_eq!(store.borrow().record(&note).unwrap().unwrap().version, 2);
    assert_eq!(
        store
            .borrow_mut()
            .save(&dirty.id, dirty.generation)
            .unwrap(),
        SaveOutcome::Saved(saved.clone()),
        "Save replay still idempotent"
    );
    let compensation = store
        .borrow_mut()
        .allocate_operation("undo-explicit-save")
        .unwrap();
    assert!(matches!(
        store
            .borrow_mut()
            .undo(&compensation, &saved.operation, &authority)
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let RecordData::Note(restored) = store.borrow().note(&note).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(
        restored.markdown, source,
        "durable checked Undo restores saved source"
    );
    println!(
        "coupled witness: clean display/reveal/resize/clear -> no Change, no session, no revision/receipt; typing positive control -> persisted generation {}; dirty display -> same text/generation/recovery; stale Save refused, explicit Save/replay/durable Undo passed",
        dirty.generation
    );
}
