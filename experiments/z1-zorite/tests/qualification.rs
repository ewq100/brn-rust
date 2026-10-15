//! Gates 1–2 only: GPUI's test platform, never an interactive application.
mod common;
use common::*;
use gpui_kit::{
    AppContext, ClipboardItem, EntityInputHandler, TestAppContext, point, px, size,
    test::TestWindowExt,
};

#[gpui_kit::test]
fn byte_ranges_repeated_estonian_combining_emoji_and_crlf(cx: &mut TestAppContext) {
    for (source, quote) in [
        (
            "Identical paragraph.\n\nIdentical paragraph.",
            "Identical paragraph.",
        ),
        ("tere tere õun — ÕÄÖÜ šž", "tere"),
        ("tere tere õun — ÕÄÖÜ šž", "õun — ÕÄÖÜ šž"),
        ("a\u{0308} before a\u{0308}", "a\u{0308}"),
        ("👩🏽‍💻 before 👩🏽‍💻", "👩🏽‍💻"),
        ("First õun.\r\n\r\nSame 🦀.\r\n\r\nSame 🦀.", "Same 🦀."),
    ] {
        let f = Fixture::new(cx, source);
        assert_eq!(f.text(cx), source, "load must retain exact bytes");
        let start = source.rfind(quote).unwrap();
        let wanted = start..start + quote.len();
        f.select(cx, wanted.clone());
        assert_eq!(f.selection(cx), wanted);
        assert_eq!(&f.text(cx)[wanted.clone()], quote);
        println!("raw selection {source:?}: {wanted:?} = {quote:?}");
        f.press(cx, "cmd-a");
        assert_eq!(f.selection(cx), 0..source.len());
    }
}

#[gpui_kit::test]
fn programmatic_replacement_events_undo_redo_and_set_text(cx: &mut TestAppContext) {
    let f = Fixture::new(cx, "tere tere õun");
    let notified = f.host.read_with(cx, |h, _| h.notifications);
    f.editor
        .update(cx, |e, cx| e.replace_range(5..9, "teine", cx));
    f.frame(cx);
    assert_eq!(f.text(cx), "tere teine õun");
    assert_eq!(f.changed(cx), 0, "replace_range is silent");
    assert!(f.host.read_with(cx, |h, _| h.notifications) > notified);
    f.press(cx, "cmd-z");
    assert_eq!(f.text(cx), "tere tere õun");
    assert_eq!(f.changed(cx), 0, "Undo is also silent!");
    f.press(cx, "cmd-shift-z");
    assert_eq!(f.text(cx), "tere teine õun");
    assert_eq!(f.changed(cx), 0, "Redo is also silent!");
    cx.update_window(f.window, |_, w, cx| w.input("!", cx))
        .unwrap();
    f.frame(cx);
    assert_eq!(f.text(cx), "tere teine! õun");
    assert_eq!(f.changed(cx), 1, "typing is the positive event control");
    f.press(cx, "cmd-z");
    assert_eq!(
        f.text(cx),
        "tere teine õun",
        "typed edit is a separate undo step"
    );
    assert_eq!(f.changed(cx), 1);
    f.editor
        .update(cx, |e, cx| e.set_text("A different saved revision.", cx));
    f.frame(cx);
    f.press(cx, "cmd-z");
    f.press(cx, "cmd-shift-z");
    assert_eq!(f.text(cx), "A different saved revision.");
    assert_eq!(f.changed(cx), 1, "set_text/undo/redo emit no Changed");
    println!(
        "replace_range, Undo, Redo, set_text: no Changed; typing positive control: one event; set_text clears both histories"
    );
}

#[gpui_kit::test]
fn capture_wrapper_blocks_keys_and_paste_but_input_callbacks_bypass_it(cx: &mut TestAppContext) {
    let f = Fixture::new(cx, "protected draft");
    f.host.update(cx, |h, cx| {
        h.capture_read_only = true;
        cx.notify();
    });
    f.frame(cx);
    cx.update_window(f.window, |_, w, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string("PASTED".into()));
        w.input("x", cx);
        for key in [
            "backspace",
            "delete",
            "enter",
            "tab",
            "cmd-v",
            "cmd-x",
            "cmd-b",
            "cmd-z",
        ] {
            w.press(key, cx);
        }
    })
    .unwrap();
    f.frame(cx);
    assert_eq!(
        f.text(cx),
        "protected draft",
        "key capture positive control"
    );
    // Directly exercise the public callbacks registered by EditorElement::paint.
    // Native text systems can call them without a GPUI key/action event.
    cx.update_window(f.window, |_, w, cx| {
        f.editor.update(cx, |e, cx| {
            EntityInputHandler::paste(e, ClipboardItem::new_string("PASTED".into()), w, cx);
        })
    })
    .unwrap();
    f.frame(cx);
    assert_eq!(f.text(cx), "PASTEDprotected draft");
    cx.update_window(f.window, |_, w, cx| {
        f.editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "õ", Some(1..1), w, cx);
            assert!(e.marked_text_range(w, cx).is_some());
        })
    })
    .unwrap();
    f.frame(cx);
    assert_eq!(f.text(cx), "PASTEDõprotected draft");
    assert_eq!(f.changed(cx), 2);
    println!(
        "FAIL read-only qualification: key/action capture blocks typing and cmd-v; direct input-handler paste and marked-text callbacks mutate content"
    );
}

#[gpui_kit::test]
fn geometry_first_paint_resize_rewrap_and_font(cx: &mut TestAppContext) {
    let source = format!(
        "{}\n\nSecond paragraph for a margin comment.",
        "A long sentence with õun and 🦀 to rewrap. ".repeat(5)
    );
    let f = Fixture::new(cx, &source);
    let start = source.find("Second paragraph").unwrap();
    let wide = f.bounds(cx, start);
    assert_eq!(
        wide.size.width,
        px(0.),
        "API returns a zero-width caret, not a range rectangle"
    );
    assert!(wide.size.height > px(0.));
    cx.simulate_window_resize(f.window, size(px(240.), px(1000.)));
    f.frame(cx);
    let narrow = f.bounds(cx, start);
    assert!(
        narrow.origin.y > wide.origin.y,
        "preceding paragraph must rewrap"
    );
    f.host.update(cx, |h, cx| {
        h.font_size = 24.;
        cx.notify();
    });
    f.frame(cx);
    let larger = f.bounds(cx, start);
    assert!(larger.origin.y > narrow.origin.y);
    assert!(larger.size.height > narrow.size.height);
    let margin_x = px(260.);
    let comment_anchor = point(margin_x, larger.origin.y);
    assert_eq!(comment_anchor.y, larger.origin.y);
    println!(
        "caret offset {start}: wide={wide:?}; narrow={narrow:?}; larger={larger:?}; comment={comment_anchor:?}"
    );
}

#[gpui_kit::test]
fn wysiwyg_source_mapping_against_brn_read_fixtures(cx: &mut TestAppContext) {
    for (source, quote) in [
        ("# Café 🦀\n\nFirst **echo**.\n\nSecond echo.", "echo"),
        ("## Café 🦀\n\nA **formatted** paragraph.", "formatted"),
        ("é🦀 same — é🦀 same", "é🦀 same"),
        (
            "# Õun\r\n\r\nA **täpne** a\u{0308} 🦀.\r\n\r\nA **täpne** a\u{0308} 🦀.",
            "täpne",
        ),
    ] {
        let f = Fixture::new(cx, source);
        f.markdown(cx);
        let start = source.rfind(quote).unwrap();
        let wanted = start..start + quote.len();
        f.select(cx, wanted.clone());
        let actual = f.selection(cx);
        assert_eq!(&source[actual.clone()], quote);
        f.show_reader(cx);
        let rendered = f.reader.read_with(cx, |r, _| r.rendered_text());
        assert_eq!(rendered.source(), source);
        let read_range = rendered.range_for_source(wanted.clone()).unwrap();
        assert_eq!(&rendered.as_str()[read_range.clone()], quote);
        println!(
            "WYSIWYG {source:?}: editor source={actual:?}, BRN rendered={read_range:?}, quote={quote:?}"
        );
        cx.update_window(f.window, |_, w, cx| {
            f.reader.read(cx).focus_handle().clone().focus(w, cx);
            w.press("cmd-a", cx);
        })
        .unwrap();
        f.frame(cx);
        assert_eq!(
            f.reader.read_with(cx, |r, _| r.selected_source_range()),
            Some(0..source.len())
        );
    }
}

#[gpui_kit::test]
fn math_load_normalization_is_a_source_fidelity_counterexample(cx: &mut TestAppContext) {
    let source = "before $$x^2$$ after";
    let f = Fixture::new(cx, source);
    let loaded = f.text(cx);
    assert_ne!(
        loaded, source,
        "published load normalizes math even before Markdown styling"
    );
    assert_eq!(f.changed(cx), 0);
    f.show_reader(cx);
    assert_eq!(
        f.reader
            .read_with(cx, |r, _| r.rendered_text().source().to_owned()),
        source
    );
    println!("FAIL exact source identity: loaded {source:?} as {loaded:?} without Changed");
}

#[gpui_kit::test]
fn ime_api_surface_utf16_marked_text_and_input_commit_only(cx: &mut TestAppContext) {
    let f = Fixture::new(cx, "õ 🦀 ");
    f.cursor(cx, "õ 🦀 ".len());
    cx.update_window(f.window, |_, w, cx| {
        f.editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "に🦀", Some(3..3), w, cx);
            assert_eq!(e.text(), "õ 🦀 に🦀");
            assert_eq!(e.marked_text_range(w, cx), Some(5..8));
            assert_eq!(e.selected_text_range(false, w, cx).unwrap().range, 8..8);
            e.replace_text_in_range(None, "日本", w, cx);
            assert_eq!(e.text(), "õ 🦀 日本");
            assert_eq!(e.marked_text_range(w, cx), None);
            e.unmark_text(w, cx);
        })
    })
    .unwrap();
    f.frame(cx);
    assert_eq!(f.changed(cx), 2);
    println!(
        "IME API only: UTF-16 marked range 5..8 and caret 8; replacement commit clears mark; no native IME claim"
    );
}

#[gpui_kit::test]
fn wysiwyg_pointer_mapping_counterexamples(cx: &mut TestAppContext) {
    for (source, quote) in [
        ("# Café 🦀\n\nFirst **echo**.\n\nSecond echo.", "echo"),
        ("## Café 🦀\n\nA **formatted** paragraph.", "formatted"),
        (
            "# Õun\r\n\r\nA **täpne** a\u{0308} 🦀.\r\n\r\nA **täpne** a\u{0308} 🦀.",
            "täpne",
        ),
    ] {
        let f = Fixture::new(cx, source);
        f.markdown(cx);
        let start = source.rfind(quote).unwrap();
        let wanted = start..start + quote.len();
        // Caret stays in the heading so the selected paragraph is rendered
        // with its Markdown markers hidden, not in source-editing presentation.
        let a = f.bounds(cx, wanted.start);
        let b = f.bounds(cx, wanted.end);
        let from = point(a.origin.x, a.origin.y + a.size.height / 2.);
        let to = point(b.origin.x, b.origin.y + b.size.height / 2.);
        let endpoints = cx
            .update_window(f.window, |_, w, cx| {
                f.editor.update(cx, |e, cx| {
                    let left = e.character_index_for_point(from, w, cx).unwrap();
                    let right = e.character_index_for_point(to, w, cx).unwrap();
                    utf16_to_byte(e.text(), left)..utf16_to_byte(e.text(), right)
                })
            })
            .unwrap();
        println!("WYSIWYG hit mapping: wanted={wanted:?}, hit={endpoints:?}, source={source:?}");
        cx.update_window(f.window, |_, w, cx| w.drag(from, to, cx))
            .unwrap();
        f.frame(cx);
        let selected = f.selection(cx);
        println!(
            "WYSIWYG mouse selection: wanted={wanted:?}, selected={selected:?}, quote={:?}",
            &source[selected.clone()]
        );
        if quote == "echo" {
            assert_eq!(endpoints, wanted);
            assert_eq!(selected, wanted);
        } else {
            // A visible formatting boundary maps past the hidden closing **.
            // Characterize the published behavior without trimming or patching.
            assert_eq!(endpoints, wanted.start..wanted.end + 2);
            assert_eq!(selected, wanted.start..wanted.end + 2);
            println!("FAIL agreement with BRN minimal source range: selected hidden closing **");
        }
    }
}

#[gpui_kit::test]
fn margin_comment_element_tracks_caret_after_resize(cx: &mut TestAppContext) {
    let source = format!(
        "{}\n\nSelected paragraph.",
        "Long text to wrap õun 🦀. ".repeat(8)
    );
    let f = Fixture::new(cx, &source);
    let start = source.find("Selected").unwrap();
    f.host.update(cx, |h, cx| {
        h.comment_offset = Some(start);
        cx.notify();
    });
    f.frame(cx);
    let wide = f.bounds(cx, start);
    let marker = cx
        .update_window(f.window, |_, w, _| w.find("margin-comment").bounds())
        .unwrap();
    assert!(
        (marker.origin.y - wide.origin.y).abs() <= px(0.5),
        "GPUI rounds layout to physical pixels"
    );
    cx.simulate_window_resize(f.window, size(px(240.), px(1000.)));
    f.frame(cx);
    // bounds_for_offset reads the LAST paint: a second host frame uses the
    // newly wrapped geometry. The production host must schedule that update.
    f.frame(cx);
    let narrow = f.bounds(cx, start);
    let marker = cx
        .update_window(f.window, |_, w, _| w.find("margin-comment").bounds())
        .unwrap();
    assert!(narrow.origin.y > wide.origin.y);
    assert!(
        (marker.origin.y - narrow.origin.y).abs() <= px(0.5),
        "GPUI rounds layout to physical pixels"
    );
    println!(
        "laid-out margin element: wide y={:?}, narrow y={:?}; marker={marker:?}",
        wide.origin.y, narrow.origin.y
    );
}

#[gpui_kit::test]
fn capture_wrapper_does_not_block_mouse_checkbox_mutation(cx: &mut TestAppContext) {
    let source = "Header\n\n- [ ] task";
    let f = Fixture::new(cx, source);
    f.markdown(cx);
    f.host.update(cx, |h, cx| {
        h.capture_read_only = true;
        cx.notify();
    });
    f.frame(cx);
    let caret = f.bounds(cx, source.find("task").unwrap());
    let checkbox = point(px(6.), caret.origin.y + caret.size.height / 2.);
    cx.update_window(f.window, |_, w, cx| w.drag(checkbox, checkbox, cx))
        .unwrap();
    f.frame(cx);
    println!("read-only checkbox click at {checkbox:?}: {:?}", f.text(cx));
    assert_eq!(f.text(cx), "Header\n\n- [x] task");
    assert_eq!(f.changed(cx), 1);
}

#[gpui_kit::test]
fn brn_read_pointer_ranges_exclude_hidden_emphasis_delimiters(cx: &mut TestAppContext) {
    for (source, quote) in [
        ("## Café 🦀\n\nA **formatted** paragraph.", "formatted"),
        (
            "# Õun\r\n\r\nA **täpne** a\u{0308} 🦀.\r\n\r\nA **täpne** a\u{0308} 🦀.",
            "täpne",
        ),
    ] {
        let f = Fixture::new(cx, source);
        f.show_reader(cx);
        let bounds = f.reader.read_with(cx, |r, _| r.bounds());
        // The GPUI test text system uses a 0.6 em advance. The paragraph
        // prefix is "A ". Scan vertical bands to hit every repeated paragraph;
        // only accept bands whose actual selected plain text is the whole word.
        let left = bounds.origin.x + px(2. * 16. * 0.6);
        let right = left + px(quote.chars().count() as f32 * 16. * 0.6);
        let mut ranges = Vec::new();
        let mut samples = Vec::new();
        for dy in (1..f32::from(bounds.size.height).ceil() as usize).step_by(2) {
            let y = bounds.origin.y + px(dy as f32);
            cx.update_window(f.window, |_, w, cx| {
                w.drag(point(left, y), point(right, y), cx)
            })
            .unwrap();
            f.frame(cx);
            f.reader.read_with(cx, |r, _| {
                let sample = (r.selected_text(), r.selected_source_range());
                if !samples.contains(&sample) {
                    samples.push(sample);
                }
                if r.selected_text().trim_end_matches('\n') == quote {
                    let range = r.selected_source_range().unwrap();
                    if !ranges.contains(&range) {
                        ranges.push(range);
                    }
                }
            });
        }
        println!("BRN bounds={bounds:?}, x={left:?}..{right:?}, selected samples={samples:?}");
        let last = source.rfind(quote).unwrap();
        println!("BRN Read actual pointer selection {quote:?}: {ranges:?}; source={source:?}");
        assert!(ranges.contains(&(last..last + quote.len())));
        assert!(ranges.iter().all(|r| &source[r.clone()] == quote));
    }
}
