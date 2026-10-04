//! Pure additive Markdown text; durable changes still require exact approval.
use super::{LinkTarget, extract, rejected, target};
use crate::{MAX_NOTE_BYTES, Result};
use uuid::Uuid;

pub(super) fn append(
    text: &str,
    body_start: usize,
    label: &str,
    target_id: Uuid,
) -> Result<String> {
    if text.len() > MAX_NOTE_BYTES || text.get(body_start..).is_none() {
        return Err(rejected(
            "Link preparation needs a note within 1 MiB and an exact UTF-8 body offset.",
        ));
    }
    if target_id.is_nil()
        || label.trim().is_empty()
        || label.len() > 512
        || label.chars().any(char::is_control)
    {
        return Err(rejected(
            "Link preparation needs a nonnil target UUID and a single-line label of 1 to 512 bytes.",
        ));
    }
    let newline = match text.find('\n') {
        Some(offset) if offset > 0 && text.as_bytes()[offset - 1] == b'\r' => "\r\n",
        _ => "\n",
    };
    let mut escaped = String::new();
    for character in label.chars() {
        if character.is_ascii_punctuation() {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    let link = format!("[{escaped}](brn://note/{target_id})");
    let size = text
        .len()
        .checked_add(newline.len() * 3)
        .and_then(|bytes| bytes.checked_add(link.len()))
        .filter(|bytes| *bytes <= MAX_NOTE_BYTES)
        .ok_or_else(|| rejected("Prepared link text exceeds the 1 MiB note limit."))?;
    let mut prepared = String::with_capacity(size);
    prepared.push_str(text);
    prepared.push_str(newline);
    prepared.push_str(newline);
    let start_byte = prepared.len();
    prepared.push_str(&link);
    let end_byte = prepared.len();
    prepared.push_str(newline);
    let links = extract::extract(&prepared, body_start)?;
    if !links.iter().any(|candidate| {
        matches!(target("", &candidate.destination).0, Some(LinkTarget::Identity(id)) if id == target_id)
            && candidate.evidence.first().is_some_and(|proof| {
                proof.start_byte == start_byte && proof.end_byte == end_byte && proof.quote == link
            })
    }) {
        return Err(rejected("Appended stable link is hidden by the existing Markdown context."));
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;
    use markdown::{ParseOptions, mdast::Node};

    fn visible_label(node: &Node) -> String {
        match node {
            Node::Text(text) => text.value.clone(),
            node => node
                .children()
                .map(|children| children.iter().map(visible_label).collect())
                .unwrap_or_default(),
        }
    }

    #[test]
    fn literal_punctuation_backslashes_and_unicode_remain_one_plain_link_label() {
        let id = Uuid::new_v4();
        for label in [
            "Eesti õ / 日本語 🦀",
            "literal [brackets] (parens) \\ backslash *emphasis* _other_ `code` &amp; <tag>",
            "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
            "  exact spaces  ",
        ] {
            let original = "# Saved note\nbody";
            let prepared = append(original, 0, label, id).unwrap();
            assert!(prepared.starts_with(original));
            let ast =
                markdown::to_mdast(&prepared[original.len()..], &ParseOptions::default()).unwrap();
            let Node::Root(root) = ast else {
                panic!("root")
            };
            assert_eq!(root.children.len(), 1);
            let Node::Paragraph(paragraph) = &root.children[0] else {
                panic!("paragraph")
            };
            assert_eq!(paragraph.children.len(), 1);
            let Node::Link(link) = &paragraph.children[0] else {
                panic!("link")
            };
            assert_eq!(link.url, format!("brn://note/{id}"));
            assert!(
                link.children
                    .iter()
                    .all(|node| matches!(node, Node::Text(_)))
            );
            assert_eq!(visible_label(&paragraph.children[0]), label);
        }
    }

    #[test]
    fn complete_original_bom_frontmatter_and_mixed_eof_prefix_is_preserved_exactly() {
        let id = Uuid::new_v4();
        for (original, body_start, newline) in [
            ("", 0, "\n"),
            ("\u{feff}no newline", 3, "\n"),
            ("first\r\nthen\nlast\r", 0, "\r\n"),
            ("first\nthen\r\nlast", 0, "\n"),
            (
                "\u{feff}---\r\ncustom: untouched\r\n...\r\nbody\n",
                "\u{feff}---\r\ncustom: untouched\r\n...\r\n".len(),
                "\r\n",
            ),
        ] {
            let prepared = append(original, body_start, "plain", id).unwrap();
            assert_eq!(
                prepared,
                format!("{original}{newline}{newline}[plain](brn://note/{id}){newline}")
            );
            assert_eq!(&prepared.as_bytes()[..original.len()], original.as_bytes());
        }
    }

    #[test]
    fn appended_occurrence_must_be_a_real_link_outside_unfinished_code_and_html() {
        let id = Uuid::new_v4();
        for text in [
            "```rust\nunfinished",
            "~~~\nunfinished",
            "<!-- unfinished",
            "<script>\nunfinished",
            "<? unfinished",
            "<![CDATA[unfinished",
        ] {
            assert_eq!(
                append(text, 0, "next", id).err().unwrap().kind,
                ErrorKind::ToolRejected,
                "{text}"
            );
        }
        for text in [
            "```\ncode\n```",
            "<!-- complete -->",
            "<script>\ncomplete\n</script>",
            "<div>\nblank line ends this raw block",
            "    indented code",
            "[next]: ignored.md",
        ] {
            let prepared = append(text, 0, "next", id).unwrap();
            let links = super::super::extract::extract(&prepared, 0).unwrap();
            let last = links.last().unwrap();
            assert_eq!(last.destination, format!("brn://note/{id}"));
            assert_eq!(last.evidence[0].start_byte, text.len() + 2);
        }
    }

    #[test]
    fn label_uuid_body_offset_note_size_and_existing_extraction_limits_are_strict() {
        let id = Uuid::new_v4();
        for label in [
            "",
            "   ",
            "line\nline",
            "tab\there",
            "null\0",
            "del\u{7f}",
            "c1\u{85}",
        ] {
            assert_eq!(
                append("", 0, label, id).err().unwrap().kind,
                ErrorKind::ToolRejected
            );
        }
        assert!(append("", 0, &"õ".repeat(256), id).is_ok());
        assert!(append("", 0, &"õ".repeat(257), id).is_err());
        assert!(append("", 0, &"x".repeat(513), id).is_err());
        assert!(append("", 0, "valid", Uuid::nil()).is_err());
        for offset in [1, 3, usize::MAX] {
            assert!(append("õ", offset, "valid", id).is_err());
        }
        let suffix_len = format!("\n\n[x](brn://note/{id})\n").len();
        let within = "a".repeat(MAX_NOTE_BYTES - suffix_len);
        assert_eq!(append(&within, 0, "x", id).unwrap().len(), MAX_NOTE_BYTES);
        assert!(append(&format!("{within}a"), 0, "x", id).is_err());
        assert!(append(&"x".repeat(MAX_NOTE_BYTES + 1), 0, "x", id).is_err());
        let too_many = "[x](a.md)\n".repeat(4096);
        assert!(append(&too_many, 0, "x", id).is_err());
        let references = "[x][r]\n".repeat(1024);
        let excessive_output = format!("{references}\n[r]: {}.md\n", "a".repeat(2040));
        assert!(append(&excessive_output, 0, "x", id).is_err());
    }
}
