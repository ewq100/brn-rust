//! Conservative Markdown formatting over the one canonical UTF-8 buffer.

use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub(super) enum Format {
    Bold,
    Italic,
    Heading,
    List,
    Quote,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Edit {
    pub(super) range: Range<usize>,
    pub(super) replacement: String,
    /// Absolute byte range in the buffer after applying this edit.
    pub(super) selection: Range<usize>,
}

/// Return a local Markdown edit, or refuse an unsupported/unsafe selection.
///
/// Inline actions require selected single-line prose. Block actions affect
/// complete touched lines (the active line for a caret), excluding a following
/// line when the selection ends immediately after a newline. Markdown markers
/// remain source text; this helper neither parses nor normalizes the document.
pub(super) fn format(text: &str, selection: Range<usize>, kind: Format) -> Option<Edit> {
    if selection.start > selection.end
        || selection.end > text.len()
        || !text.is_char_boundary(selection.start)
        || !text.is_char_boundary(selection.end)
    {
        return None;
    }
    match kind {
        Format::Bold => inline(text, selection, "**"),
        Format::Italic => inline(text, selection, "*"),
        Format::Heading | Format::List | Format::Quote => block(text, selection, kind),
    }
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map_or(text.len(), |i| offset + i)
}

fn inline(text: &str, selection: Range<usize>, marker: &str) -> Option<Edit> {
    let selected = text.get(selection.clone())?;
    if selected.trim().is_empty() || selected.contains(['\n', '\r']) {
        return None;
    }
    // A delimiter next to whitespace does not open/close CommonMark emphasis.
    // Keep selected edge whitespace in the buffer, outside the local edit.
    let leading = selected.len() - selected.trim_start().len();
    let selected = selected.trim();
    let selection = selection.start + leading..selection.start + leading + selected.len();
    let line = line_start(text, selection.start)..line_end(text, selection.end);
    if !safe_lines(text, line.clone()) {
        return None;
    }

    let n = marker.len();
    let (range, content, removing) =
        if selected.len() > 2 * n && selected.starts_with(marker) && selected.ends_with(marker) {
            (selection.clone(), &selected[n..selected.len() - n], true)
        } else if selection.start >= line.start + n
            && selection.end + n <= line.end
            && text.get(selection.start - n..selection.start) == Some(marker)
            && text.get(selection.end..selection.end + n) == Some(marker)
        {
            (selection.start - n..selection.end + n, selected, true)
        } else {
            (selection.clone(), selected, false)
        };

    // Without a Markdown parser, refuse uncertain nesting, escaping, links and
    // code spans rather than create a selection across their syntax. Exact
    // marker toggles above are the only emphasis syntax accepted on this line.
    let uncertain = |s: &str| s.contains(['*', '_', '`', '[', ']', '\\']);
    if content.trim().is_empty()
        || uncertain(content)
        || uncertain(&text[line.start..range.start])
        || uncertain(&text[range.end..line.end])
    {
        return None;
    }
    let replacement = if removing {
        content.to_owned()
    } else {
        format!("{marker}{content}{marker}")
    };
    let start = range.start + if removing { 0 } else { n };
    Some(Edit {
        range,
        replacement,
        selection: start..start + content.len(),
    })
}

fn block(text: &str, selection: Range<usize>, kind: Format) -> Option<Edit> {
    let end_point = if selection.end > selection.start
        && text.as_bytes().get(selection.end - 1) == Some(&b'\n')
    {
        selection.end - 1
    } else {
        selection.end
    };
    let range = line_start(text, selection.start)..line_end(text, end_point);
    if !safe_lines(text, range.clone()) {
        return None;
    }
    let original = &text[range.clone()];
    let prefix = match kind {
        Format::Heading => "# ",
        Format::List => "- ",
        Format::Quote => "> ",
        Format::Bold | Format::Italic => return None,
    };
    let nonblank = original.lines().filter(|line| !line.trim().is_empty());
    let mut count = 0;
    let mut remove = true;
    for line in nonblank {
        count += 1;
        remove &= prefix_len(line.trim_start_matches(' '), kind).is_some();
    }
    remove &= count > 0;

    let mut replacement = String::with_capacity(original.len() + 2 * (count + 1));
    let mut changes = Vec::new();
    let mut offset = range.start;
    // split, rather than lines, retains CRLF and the empty active final line.
    for (i, line) in original.split('\n').enumerate() {
        if i > 0 {
            replacement.push('\n');
            offset += 1;
        }
        let indent = line.bytes().take_while(|b| *b == b' ').count();
        let body = &line[indent..];
        let blank = body.trim().is_empty();
        let old_len = prefix_len(body, kind);
        let modify_blank = selection.is_empty() && original.trim().is_empty();
        if (!blank || modify_blank) && (remove || old_len.is_none()) {
            let old_len = if remove { old_len? } else { 0 };
            let new_prefix = if remove { "" } else { prefix };
            replacement.push_str(&line[..indent]);
            replacement.push_str(new_prefix);
            replacement.push_str(&body[old_len..]);
            changes.push((offset + indent..offset + indent + old_len, new_prefix.len()));
        } else {
            replacement.push_str(line);
        }
        offset += line.len();
    }
    if replacement == original {
        return None;
    }
    Some(Edit {
        selection: map_offset(selection.start, &changes)..map_offset(selection.end, &changes),
        range,
        replacement,
    })
}

fn prefix_len(line: &str, kind: Format) -> Option<usize> {
    match kind {
        Format::Heading => {
            let hashes = line.bytes().take_while(|b| *b == b'#').count();
            (1..=6)
                .contains(&hashes)
                .then_some(())
                .filter(|_| line.as_bytes().get(hashes) == Some(&b' '))
                .map(|_| hashes + 1)
        }
        Format::List => line
            .as_bytes()
            .first()
            .filter(|b| matches!(b, b'-' | b'*' | b'+'))
            .filter(|_| line.as_bytes().get(1) == Some(&b' '))
            .map(|_| 2),
        Format::Quote => line.starts_with("> ").then_some(2),
        Format::Bold | Format::Italic => None,
    }
}

fn map_offset(offset: usize, changes: &[(Range<usize>, usize)]) -> usize {
    let mut shift = 0isize;
    for (range, new_len) in changes {
        if offset < range.start {
            break;
        }
        if offset < range.end {
            return range.start.checked_add_signed(shift).unwrap() + new_len;
        }
        shift += *new_len as isize - range.len() as isize;
    }
    offset.checked_add_signed(shift).unwrap()
}

/// Reject any touched fenced/indented code line, including fence delimiters.
fn safe_lines(text: &str, target: Range<usize>) -> bool {
    let mut fence = None;
    let mut offset = 0;
    for line in text.split('\n') {
        let indent = line.bytes().take_while(|b| *b == b' ').count();
        let body = &line[indent..];
        let touched = offset <= target.end && offset + line.len() >= target.start;
        let code_body = strip_containers(body);
        let code_indent = code_body.bytes().take_while(|b| *b == b' ').count();
        let code_body = &code_body[code_indent..];
        let delimiter = if indent <= 3 && code_indent <= 3 {
            let ch = code_body.as_bytes().first().copied();
            ch.filter(|ch| matches!(ch, b'`' | b'~'))
                .map(|ch| (ch, code_body.bytes().take_while(|b| *b == ch).count()))
                .filter(|(_, len)| *len >= 3)
        } else {
            None
        };
        if touched
            && (fence.is_some()
                || delimiter.is_some()
                || indent >= 4
                || code_indent >= 4
                || code_body.starts_with('\t'))
        {
            return false;
        }
        if let Some((ch, len)) = delimiter {
            match fence {
                Some((open_ch, open_len))
                    if ch == open_ch && len >= open_len && code_body[len..].trim().is_empty() =>
                {
                    fence = None;
                }
                None => fence = Some((ch, len)),
                _ => {}
            }
        }
        offset += line.len() + 1;
        if offset > target.end {
            break;
        }
    }
    true
}

fn strip_containers(mut body: &str) -> &str {
    loop {
        let indent = body.bytes().take_while(|b| *b == b' ').count();
        if indent > 3 {
            return body;
        }
        let candidate = &body[indent..];
        if let Some(rest) = candidate.strip_prefix('>') {
            body = rest.strip_prefix(' ').unwrap_or(rest);
        } else if let Some(len) = prefix_len(candidate, Format::List) {
            body = &candidate[len..];
        } else if let Some(rest) = strip_ordered_list(candidate) {
            body = rest;
        } else {
            return body;
        }
    }
}

fn strip_ordered_list(body: &str) -> Option<&str> {
    let digits = body.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &body[digits..];
    rest.strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))
        .or_else(|| rest.strip_prefix(".\t"))
        .or_else(|| rest.strip_prefix(")\t"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_selection(text: &str, edit: &Edit) -> String {
        let result = apply(text, edit);
        result[edit.selection.clone()].to_owned()
    }

    fn apply(text: &str, edit: &Edit) -> String {
        let mut result = text.to_owned();
        result.replace_range(edit.range.clone(), &edit.replacement);
        assert_eq!(&result[..edit.range.start], &text[..edit.range.start]);
        assert_eq!(
            &result[edit.range.start + edit.replacement.len()..],
            &text[edit.range.end..]
        );
        assert!(result.is_char_boundary(edit.selection.start));
        assert!(result.is_char_boundary(edit.selection.end));
        result
    }

    #[test]
    fn unicode_and_repeated_text_keep_the_selected_occurrence() {
        let text = "é🦀 same — é🦀 same";
        let start = text.rfind("é🦀").unwrap();
        let edit = format(text, start..text.len(), Format::Bold).unwrap();
        let result = apply(text, &edit);
        assert_eq!(result, "é🦀 same — **é🦀 same**");
        assert_eq!(&result[edit.selection], "é🦀 same");
    }

    #[test]
    fn inline_toggle_accepts_content_or_complete_markers() {
        for (kind, marker) in [(Format::Bold, "**"), (Format::Italic, "*")] {
            let initial = format("before café after", 7..12, kind).unwrap();
            let marked = apply("before café after", &initial);
            let unwrapped = format(&marked, initial.selection.clone(), kind).unwrap();
            assert_eq!(apply(&marked, &unwrapped), "before café after");
            assert_eq!(unwrapped.selection, 7..12);
            let full = 7..12 + 2 * marker.len();
            let unwrapped = format(&marked, full, kind).unwrap();
            assert_eq!(apply(&marked, &unwrapped), "before café after");
        }
    }

    #[test]
    fn inline_refuses_empty_multiline_and_whitespace() {
        for kind in [Format::Bold, Format::Italic] {
            assert!(format("text", 0..0, kind).is_none());
            assert!(format(" \t ", 0..3, kind).is_none());
            assert!(format("one\ntwo", 0..7, kind).is_none());
            assert!(format("one\rtwo", 0..7, kind).is_none());
        }
    }

    #[test]
    fn unsafe_inline_syntax_is_a_noop() {
        for text in [
            "[link](url)",
            "`code`",
            "escaped \\*star",
            "*italic*",
            "word_with_gap",
        ] {
            assert!(format(text, 0..text.len(), Format::Bold).is_none());
        }
        assert!(format("**bold**", 2..6, Format::Italic).is_none());
        assert!(format("a *word* b", 4..6, Format::Bold).is_none());
    }

    #[test]
    fn invalid_ranges_never_slice_or_panic() {
        for kind in [
            Format::Bold,
            Format::Italic,
            Format::Heading,
            Format::List,
            Format::Quote,
        ] {
            for selection in [
                Range { start: 2, end: 1 },
                0..99,
                1..2,
                0..1,
                usize::MAX..usize::MAX,
            ] {
                assert!(format("é🦀", selection, kind).is_none());
            }
        }
    }

    #[test]
    fn block_touches_whole_lines_but_excludes_the_next_line() {
        let text = "before\nalpha\nbéta\nafter";
        for (kind, prefix) in [
            (Format::Heading, "# "),
            (Format::List, "- "),
            (Format::Quote, "> "),
        ] {
            let edit = format(text, 9..19, kind).unwrap();
            let result = apply(text, &edit);
            assert_eq!(
                result,
                format!("before\n{prefix}alpha\n{prefix}béta\nafter")
            );
            assert_eq!(&result[edit.selection], format!("pha\n{prefix}béta\n"));
        }
    }

    #[test]
    fn block_toggle_preserves_unicode_indent_blank_lines_and_crlf() {
        let text = "outside\r\n  café\r\n\r\n🦀\r\nafter";
        let selection = 9..text.find("after").unwrap();
        for kind in [Format::Heading, Format::List, Format::Quote] {
            let edit = format(text, selection.clone(), kind).unwrap();
            let marked = apply(text, &edit);
            let undo = format(&marked, edit.selection.clone(), kind).unwrap();
            assert_eq!(apply(&marked, &undo), text);
            assert_eq!(undo.selection, selection);
        }
    }

    #[test]
    fn active_line_and_empty_final_line_map_the_caret() {
        let edit = format("one\n🦀 two\n", 8..8, Format::Quote).unwrap();
        assert_eq!(apply("one\n🦀 two\n", &edit), "one\n> 🦀 two\n");
        assert_eq!(edit.selection, 10..10);
        let edit = format("one\n", 4..4, Format::Heading).unwrap();
        assert_eq!(apply("one\n", &edit), "one\n# ");
        assert_eq!(edit.selection, 6..6);
        let edit = format("", 0..0, Format::List).unwrap();
        assert_eq!(apply("", &edit), "- ");
        assert_eq!(edit.selection, 2..2);
    }

    #[test]
    fn mixed_block_prefixes_do_not_double_format_existing_lines() {
        let text = "# one\ntwo";
        let edit = format(text, 0..text.len(), Format::Heading).unwrap();
        assert_eq!(apply(text, &edit), "# one\n# two");
        let text = "* one\n+ two\n- three";
        let edit = format(text, 0..text.len(), Format::List).unwrap();
        assert_eq!(apply(text, &edit), "one\ntwo\nthree");
        let text = "## one\n### two";
        let edit = format(text, 0..text.len(), Format::Heading).unwrap();
        assert_eq!(apply(text, &edit), "one\ntwo");
    }

    #[test]
    fn fenced_and_indented_code_are_untouched() {
        for text in [
            "```rust\nword\n```",
            "~~~\nword\n~~~",
            "````\n```\nword\n````",
            "> ```\n> word\n> ```",
            "- ```\n  word\n  ```",
            ">     word",
            "    word",
            "\tword",
        ] {
            let start = text.find("word").unwrap();
            for kind in [
                Format::Bold,
                Format::Italic,
                Format::Heading,
                Format::List,
                Format::Quote,
            ] {
                assert!(format(text, start..start + 4, kind).is_none(), "{text:?}");
            }
        }
        let text = "```\ncode\n```\nword";
        let start = text.find("word").unwrap();
        let edit = format(text, start..text.len(), Format::Bold).unwrap();
        assert_eq!(apply(text, &edit), "```\ncode\n```\n**word**");
    }

    #[test]
    fn block_prefixes_preserve_link_and_code_contents() {
        let text = "[é](https://example.test) and `🦀`";
        let edit = format(text, 0..text.len(), Format::Quote).unwrap();
        assert_eq!(apply(text, &edit), format!("> {text}"));
    }

    #[test]
    fn whitespace_only_multiline_selection_is_a_noop() {
        assert!(format(" \n  \n", 0..5, Format::List).is_none());
    }

    #[test]
    fn repeated_quoted_passages_keep_the_unselected_occurrence() {
        let text = "“café”\n“café”\nlast";
        let start = text.rfind("“café”").unwrap();
        let edit = format(text, start..start + "“café”".len(), Format::Quote).unwrap();
        let result = apply(text, &edit);
        assert_eq!(result, "“café”\n> “café”\nlast");
        assert_eq!(&result[edit.selection], "“café”");
    }

    #[test]
    fn inline_edge_whitespace_stays_outside_valid_markers() {
        for (leading, trailing) in [("", " "), (" ", ""), (" \t", "\t "), ("\u{a0}", "\u{2003}")] {
            let text = format!("left:{leading}café{trailing}:right");
            let selection = 5..5 + leading.len() + "café".len() + trailing.len();
            for (kind, marker) in [(Format::Bold, "**"), (Format::Italic, "*")] {
                let edit = format(&text, selection.clone(), kind).unwrap();
                assert_eq!(
                    edit.range,
                    5 + leading.len()..5 + leading.len() + "café".len()
                );
                let result = apply(&text, &edit);
                assert_eq!(
                    result,
                    format!("left:{leading}{marker}café{marker}{trailing}:right")
                );
                assert_eq!(&result[edit.selection.clone()], "café");
                let undo = format(&result, edit.selection, kind).unwrap();
                assert_eq!(apply(&result, &undo), text);
                assert_eq!(&text[undo.selection], "café");
            }
        }
    }

    #[test]
    fn mid_line_selection_on_a_non_final_line_formats_in_place() {
        // The most common case: a word inside the first of several lines.
        let text = "first word here\nsecond line";
        let edit = format(text, 6..10, Format::Bold).unwrap();
        assert_eq!(apply(text, &edit), "first **word** here\nsecond line");
        assert_eq!(result_selection(text, &edit), "word");
        let edit = format(text, 6..10, Format::Italic).unwrap();
        assert_eq!(apply(text, &edit), "first *word* here\nsecond line");
    }

    #[test]
    fn one_sided_or_exact_markers_are_refused_not_mangled() {
        // A single marker edge is uncertain syntax, never a toggle.
        for (text, selection) in [
            ("**partial", 0..9),
            ("partial**", 0..9),
            ("*partial", 0..8),
            ("partial*", 0..8),
            ("**", 0..2),
            ("*", 0..1),
        ] {
            let kind = if text.starts_with("**") || text.ends_with("**") {
                Format::Bold
            } else {
                Format::Italic
            };
            assert!(format(text, selection, kind).is_none(), "{text:?}");
        }
        // The same refusal applies when only the inner word is selected next
        // to a one-sided marker, on either side.
        assert!(format("say word** now", 4..8, Format::Bold).is_none());
        assert!(format("say **word now", 6..10, Format::Bold).is_none());
    }

    #[test]
    fn complete_markers_unwrap_at_line_start_and_on_later_lines() {
        let edit = format("**bold**", 0..8, Format::Bold).unwrap();
        assert_eq!(apply("**bold**", &edit), "bold");
        assert_eq!(result_selection("**bold**", &edit), "bold");
        let text = "abc\n**word**";
        let edit = format(text, 6..10, Format::Bold).unwrap();
        assert_eq!(apply(text, &edit), "abc\nword");
        // Trailing text after the closing marker still unwraps.
        let edit = format("**word**!", 2..6, Format::Bold).unwrap();
        assert_eq!(apply("**word**!", &edit), "word!");
    }

    #[test]
    fn reversed_selection_at_valid_boundaries_is_refused() {
        // Crossing line boundaries at char boundaries must not panic.
        for reversed in [Range { start: 5, end: 2 }, Range { start: 4, end: 1 }] {
            for kind in [Format::Bold, Format::Heading, Format::Quote] {
                assert!(format("abc\ndef", reversed.clone(), kind).is_none());
            }
        }
    }

    #[test]
    fn emphasis_syntax_elsewhere_on_the_line_is_refused() {
        // Clean selection, but a code span follows on the same line.
        assert!(format("plain `code`", 0..5, Format::Bold).is_none());
        assert!(format("`code` plain", 7..12, Format::Bold).is_none());
    }

    #[test]
    fn four_space_content_inside_a_quote_is_not_a_fence_opener() {
        // CommonMark: within a blockquote, four spaces of content indent make
        // the fence literal code text, so a following plain quote line stays
        // formattable instead of being treated as fenced.
        let text = ">     ```\n> word";
        let start = text.find("word").unwrap();
        let edit = format(text, start..text.len(), Format::Bold).unwrap();
        assert_eq!(apply(text, &edit), ">     ```\n> **word**");
    }

    #[test]
    fn fence_with_info_string_encloses_following_lines() {
        // An untouched opening fence with an info string still starts a fence.
        let text = "```rust\nmore text";
        let start = text.find("more text").unwrap();
        assert!(format(text, start..text.len(), Format::Bold).is_none());
        let text = "aaa\n```rust\nword";
        let start = text.find("word").unwrap();
        assert!(format(text, start..text.len(), Format::Bold).is_none());
    }

    #[test]
    fn deeply_indented_container_content_is_untouched() {
        // Four spaces of content indent inside a container is indented code,
        // whatever container prefix follows it; stripping must stop there.
        for text in [
            ">     - word",
            ">      > word",
            ">     1. word",
            "  >     - word",
        ] {
            let start = text.find("word").unwrap();
            for kind in [Format::Bold, Format::Quote] {
                assert!(format(text, start..text.len(), kind).is_none(), "{text:?}");
            }
        }
    }

    #[test]
    fn three_space_indented_container_fence_still_encloses() {
        // Up to three spaces of indent still belong to the blockquote, so
        // its fence encloses the touched word.
        let text = "   > ```\n   > word\n   > ```";
        let start = text.find("word").unwrap();
        for kind in [Format::Bold, Format::Heading] {
            assert!(format(text, start..start + 4, kind).is_none(), "{kind:?}");
        }
    }

    #[test]
    fn selection_ending_inside_an_existing_prefix_anchors_at_its_start() {
        // Removing "# one" with the selection ending inside "# " maps the
        // selection to where the prefix began, never to a multiplied or
        // boundary-shifted offset.
        let text = "x\n# one";
        let edit = format(text, 2..4, Format::Heading).unwrap();
        assert_eq!(apply(text, &edit), "x\none");
        assert_eq!(edit.selection, 2..2);
    }

    #[test]
    fn multi_line_prefixes_map_selections_across_every_newline() {
        let text = "aaa\nbbb\nccc";
        let edit = format(text, 0..7, Format::Quote).unwrap();
        assert_eq!(apply(text, &edit), "> aaa\n> bbb\nccc");
        assert_eq!(&apply(text, &edit)[edit.selection.clone()], "aaa\n> bbb");
        // A selection anchored exactly at the line break still maps through
        // every prefix change, including the one after its own line.
        let edit = format(text, 3..7, Format::Quote).unwrap();
        assert_eq!(apply(text, &edit), "> aaa\n> bbb\nccc");
        assert_eq!(&apply(text, &edit)[edit.selection.clone()], "\n> bbb");
    }

    #[test]
    fn ordered_and_nested_container_fences_are_untouched() {
        for text in [
            "1. ```\n   word\n   ```",
            "-\t```\n    word\n    ```",
            "*\t~~~\n    word\n    ~~~",
            "+\t```\n    word\n    ```",
            "> -\t```\n>     word\n>     ```",
            "2) ~~~\n   word\n   ~~~",
            "> 1. ```\n>    word\n>    ```",
            ">   1. - ```\n>   word\n>   ```",
            "- 1. > ```\n  word\n  ```",
        ] {
            let start = text.find("word").unwrap();
            for kind in [
                Format::Bold,
                Format::Italic,
                Format::Heading,
                Format::List,
                Format::Quote,
            ] {
                assert!(format(text, start..start + 4, kind).is_none(), "{text:?}");
            }
        }
        let text = "1. ```\n   code\n   ```\nword";
        let start = text.find("word").unwrap();
        let edit = format(text, start..text.len(), Format::Bold).unwrap();
        assert_eq!(apply(text, &edit), "1. ```\n   code\n   ```\n**word**");
    }
}
