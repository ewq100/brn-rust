use super::{LinkEvidence, LinkTarget, RawMarkdownLink, rejected, target};
use crate::{MAX_NOTE_BYTES, Result};
use markdown::{ParseOptions, mdast::Node};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

const MAX_LINKS: usize = 4096;
const MAX_RETURNED_BYTES: usize = 4 * 1024 * 1024;

/// Approval compares identities, without imposing public evidence-output limits
/// on unchanged historical links.
pub(super) fn stable_ids(text: &str, body_start: usize) -> Result<BTreeSet<Uuid>> {
    let root = parse(text, body_start)?;
    let definitions = definitions(&root);
    let mut ids = BTreeSet::new();
    for node in nodes(&root) {
        if let Some((destination, _)) = destination(node, &definitions)?
            && let (Some(LinkTarget::Identity(id)), _) = target("", destination)
        {
            ids.insert(id);
        }
    }
    Ok(ids)
}

pub(super) fn extract(text: &str, body_start: usize) -> Result<Vec<RawMarkdownLink>> {
    let root = parse(text, body_start)?;
    let definitions = definitions(&root);
    let mut links = Vec::new();
    let mut returned_bytes = 0usize;
    for node in nodes(&root) {
        let Some((destination, definition)) = destination(node, &definitions)? else {
            continue;
        };
        if links.len() == MAX_LINKS {
            return Err(rejected(
                "Saved link extraction exceeds the 4096 link limit.",
            ));
        }
        let mut evidence = vec![proof(text, body_start, node)?];
        if let Some(definition) = definition {
            evidence.push(proof(text, body_start, definition)?);
        }
        returned_bytes = returned_bytes
            .checked_add(destination.len())
            .ok_or_else(|| rejected("Saved link extraction exceeds its output limit."))?;
        for item in &evidence {
            returned_bytes = returned_bytes
                .checked_add(item.quote.len())
                .ok_or_else(|| rejected("Saved link extraction exceeds its output limit."))?;
        }
        if returned_bytes > MAX_RETURNED_BYTES {
            return Err(rejected(
                "Saved link extraction exceeds its 4 MiB output limit.",
            ));
        }
        links.push(RawMarkdownLink {
            destination: destination.into(),
            evidence,
        });
    }
    Ok(links)
}

fn parse(text: &str, body_start: usize) -> Result<Node> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(rejected(
            "Saved link extraction exceeds the 1 MiB note limit.",
        ));
    }
    let body = text.get(body_start..).ok_or_else(|| {
        rejected("Saved link body offset is outside the note or splits a UTF-8 character.")
    })?;
    markdown::to_mdast(body, &ParseOptions::default())
        .map_err(|_| rejected("Saved Markdown links could not be parsed."))
}

fn definitions(root: &Node) -> HashMap<&str, (&str, &Node)> {
    let mut definitions = HashMap::new();
    for node in nodes(root) {
        if let Node::Definition(definition) = node {
            // The parser supplies CommonMark-normalized identifiers. The first
            // matching definition is used even if it follows the occurrence.
            definitions
                .entry(definition.identifier.as_str())
                .or_insert((definition.url.as_str(), node));
        }
    }
    definitions
}

fn destination<'a>(
    node: &'a Node,
    definitions: &HashMap<&'a str, (&'a str, &'a Node)>,
) -> Result<Option<(&'a str, Option<&'a Node>)>> {
    match node {
        Node::Link(link) => Ok(Some((link.url.as_str(), None))),
        Node::LinkReference(reference) => {
            let (destination, definition) = definitions
                .get(reference.identifier.as_str())
                .ok_or_else(|| rejected("Saved link reference has no matching definition."))?;
            Ok(Some((*destination, Some(*definition))))
        }
        _ => Ok(None),
    }
}

fn nodes(root: &Node) -> impl Iterator<Item = &Node> {
    let mut pending = vec![root];
    std::iter::from_fn(move || {
        let node = pending.pop()?;
        if let Some(children) = node.children() {
            pending.extend(children.iter().rev());
        }
        Some(node)
    })
}

fn proof(text: &str, body_start: usize, node: &Node) -> Result<LinkEvidence> {
    let invalid = || rejected("Saved link evidence has an invalid exact UTF-8 byte span.");
    let position = node.position().ok_or_else(invalid)?;
    let start_byte = body_start
        .checked_add(position.start.offset)
        .ok_or_else(invalid)?;
    let end_byte = body_start
        .checked_add(position.end.offset)
        .ok_or_else(invalid)?;
    if start_byte >= end_byte {
        return Err(invalid());
    }
    let quote = text.get(start_byte..end_byte).ok_or_else(invalid)?;
    Ok(LinkEvidence {
        start_byte,
        end_byte,
        quote: quote.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    fn assert_proof(text: &str, proof: &super::super::LinkEvidence, quote: &str) {
        assert_eq!(proof.quote, quote);
        assert_eq!(text.get(proof.start_byte..proof.end_byte), Some(quote));
        assert_eq!(proof.end_byte - proof.start_byte, quote.len());
    }

    #[test]
    fn inline_links_keep_ast_destinations_and_exact_full_source_byte_evidence() {
        let prefix = "\u{feff}---\r\ncustom: '[metadata](ignore.md)'\r\n...\r\n";
        let body = "# Eesti 🦀\r\n[A](Märkus%20üks.md#päis) and [B](foo\\(bar\\).md)\r\n[C](<allikad/A&amp;B.md>)\r\n";
        let text = format!("{prefix}{body}");
        let links = extract(&text, prefix.len()).unwrap();
        assert_eq!(links.len(), 3);
        for (link, destination, quote) in [
            (
                &links[0],
                "Märkus%20üks.md#päis",
                "[A](Märkus%20üks.md#päis)",
            ),
            (&links[1], "foo(bar).md", "[B](foo\\(bar\\).md)"),
            (&links[2], "allikad/A&B.md", "[C](<allikad/A&amp;B.md>)"),
        ] {
            assert_eq!(link.destination, destination);
            assert_eq!(link.evidence.len(), 1);
            assert_proof(&text, &link.evidence[0], quote);
            assert!(link.evidence[0].start_byte >= prefix.len());
        }
    }

    #[test]
    fn reference_occurrences_use_normalized_identifiers_and_first_definition_in_source_order() {
        let prefix = "\u{feff}---\r\nbrn_kind: knowledge\r\n---\r\n";
        let body = "[VÕTI]: <first&amp;one.md>\r\n\r\n[x][  võti  ] [VÕTI][] [võti] [ß] [x][later]\r\n\r\n[võti]: ignored.md\r\n[SS]: sharp.md\r\n[later]: last.md\r\n";
        let text = format!("{prefix}{body}");
        let links = extract(&text, prefix.len()).unwrap();
        assert_eq!(links.len(), 5);
        for (link, destination, occurrence, definition) in [
            (
                &links[0],
                "first&one.md",
                "[x][  võti  ]",
                "[VÕTI]: <first&amp;one.md>",
            ),
            (
                &links[1],
                "first&one.md",
                "[VÕTI][]",
                "[VÕTI]: <first&amp;one.md>",
            ),
            (
                &links[2],
                "first&one.md",
                "[võti]",
                "[VÕTI]: <first&amp;one.md>",
            ),
            (&links[3], "sharp.md", "[ß]", "[SS]: sharp.md"),
            (&links[4], "last.md", "[x][later]", "[later]: last.md"),
        ] {
            assert_eq!(link.destination, destination);
            assert_eq!(link.evidence.len(), 2);
            assert_proof(&text, &link.evidence[0], occurrence);
            assert_proof(&text, &link.evidence[1], definition);
        }
        assert!(
            links
                .windows(2)
                .all(|pair| pair[0].evidence[0].start_byte < pair[1].evidence[0].start_byte)
        );
    }

    #[test]
    fn only_ordinary_links_are_returned_across_markdown_containers() {
        let text = "# [heading](h.md)\n\n> [quote][q]\n\n- *[list](l.md)*\n- [![nested image](image.md)](outer.md)\n\n`[code](inline.md)` ![image](image.md) ![reference image][q]\n\n```\n[code](fenced.md)\n```\n\n    [code](indented.md)\n\n<a href=\"html.md\">raw</a>\n\n<!-- [comment](comment.md) -->\n\n<div>\n[raw block](raw.md)\n</div>\n\n[missing][unknown]\n\n[q]: quote.md\n[unused]: unused.md\n";
        let links = extract(text, 0).unwrap();
        assert_eq!(
            links
                .iter()
                .map(|link| link.destination.as_str())
                .collect::<Vec<_>>(),
            ["h.md", "quote.md", "l.md", "outer.md"]
        );
        assert_proof(
            text,
            &links[3].evidence[0],
            "[![nested image](image.md)](outer.md)",
        );
    }

    #[test]
    fn empty_body_is_valid_but_outside_or_split_utf8_body_offsets_are_rejected() {
        let text = "õ\r\n";
        assert!(extract("", 0).unwrap().is_empty());
        assert!(extract(text, text.len()).unwrap().is_empty());
        for offset in [1, text.len() + 1, usize::MAX] {
            assert_eq!(
                extract(text, offset).err().unwrap().kind,
                ErrorKind::ToolRejected
            );
        }
        let oversized = "x".repeat(MAX_NOTE_BYTES + 1);
        assert_eq!(
            extract(&oversized, oversized.len()).err().unwrap().kind,
            ErrorKind::ToolRejected
        );
    }

    #[test]
    fn invalid_evidence_positions_are_refused_without_guessing_or_panicking() {
        let text = "õ[x](a.md)";
        for position in [
            None,
            Some((2, 2)),
            Some((3, 2)),
            Some((2, text.len() + 1)),
            Some((1, text.len())),
        ] {
            let node = Node::Link(markdown::mdast::Link {
                children: vec![],
                position: position
                    .map(|(start, end)| markdown::unist::Position::new(1, 1, start, 1, 1, end)),
                url: "a.md".into(),
                title: None,
            });
            assert_eq!(
                proof(text, 0, &node).err().unwrap().kind,
                ErrorKind::ToolRejected
            );
        }
        let node = Node::Link(markdown::mdast::Link {
            children: vec![],
            position: Some(markdown::unist::Position::new(1, 1, 2, 1, 1, text.len())),
            url: "a.md".into(),
            title: None,
        });
        assert_eq!(
            proof(text, usize::MAX, &node).err().unwrap().kind,
            ErrorKind::ToolRejected
        );
    }

    #[test]
    fn link_count_limit_rejects_overflow_instead_of_truncating() {
        let allowed = "[x](a.md)\n".repeat(4096);
        assert_eq!(extract(&allowed, 0).unwrap().len(), 4096);
        let too_many = format!("{allowed}[x](b.md)\n");
        assert_eq!(
            extract(&too_many, 0).err().unwrap().kind,
            ErrorKind::ToolRejected
        );
    }

    #[test]
    fn output_budget_counts_repeated_reference_definitions_and_destinations() {
        let references = "[x][r]\n".repeat(1024);
        let within = format!("{references}\n[r]: {}.md\n", "a".repeat(2039));
        let links = extract(&within, 0).unwrap();
        assert_eq!(links.len(), 1024);
        let returned_bytes: usize = links
            .iter()
            .map(|link| {
                link.destination.len()
                    + link
                        .evidence
                        .iter()
                        .map(|proof| proof.quote.len())
                        .sum::<usize>()
            })
            .sum();
        assert_eq!(returned_bytes, 4095 * 1024);
        let over = format!("{references}\n[r]: {}.md\n", "a".repeat(2040));
        assert_eq!(
            extract(&over, 0).err().unwrap().kind,
            ErrorKind::ToolRejected
        );
    }

    #[test]
    fn stable_ids_share_inline_reference_normalization_and_canonical_destination_rules() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let ignored = Uuid::new_v4();
        let prefix = "\u{feff}---\r\ncustom: links\r\n...\r\n";
        let body = format!(
            "[inline](BRN://note/{first}#heading) [query](brn://note/{first}?view=x)\r\n\r\n[reference][  võti  ] [VÕTI][] [ß]\r\n\r\n[VÕTI]: brn://note/{second}\r\n[võti]: brn://note/{ignored}\r\n[SS]: brn://note/{first}\r\n\r\n`[code](brn://note/{ignored})` ![image](brn://note/{ignored})\r\n\r\n~~~\r\n[fence](brn://note/{ignored})\r\n~~~\r\n\r\n    [indented](brn://note/{ignored})\r\n\r\n<!-- [html](brn://note/{ignored}) -->\r\n\r\n<div>\r\n[block](brn://note/{ignored})\r\n</div>\r\n\r\n[relative](a.md) [external](https://example.invalid) [nil](brn://note/{}) [noncanonical](brn://note/{}) [encoded](brn://note/%7B{ignored}%7D) [missing][unknown]\r\n",
            Uuid::nil(),
            ignored.simple(),
        );
        let text = format!("{prefix}{body}");
        assert_eq!(
            stable_ids(&text, prefix.len()).unwrap(),
            BTreeSet::from([first, second])
        );
        let upper_uuid = first.to_string().to_uppercase();
        assert_eq!(
            stable_ids(&format!("[x](BrN://note/{upper_uuid})"), 0).unwrap(),
            BTreeSet::from([first])
        );
    }

    #[test]
    fn stable_id_approval_scan_has_no_public_link_count_or_returned_quote_cap() {
        let id = Uuid::new_v4();
        let many = format!("[x](brn://note/{id})\n").repeat(4097);
        assert_eq!(stable_ids(&many, 0).unwrap(), BTreeSet::from([id]));
        assert!(extract(&many, 0).is_err());
        let references = "[x][r]\n".repeat(1024);
        let huge_definition = format!(
            "{references}\n[r]: <brn://note/{id}#{}>\n",
            "a".repeat(5000)
        );
        assert_eq!(
            stable_ids(&huge_definition, 0).unwrap(),
            BTreeSet::from([id])
        );
        assert!(extract(&huge_definition, 0).is_err());
    }

    #[test]
    fn stable_id_scan_enforces_note_and_utf8_body_limits_without_truncating() {
        assert!(stable_ids("", 0).unwrap().is_empty());
        assert!(stable_ids("õ", "õ".len()).unwrap().is_empty());
        for offset in [1, 3, usize::MAX] {
            assert_eq!(
                stable_ids("õ", offset).err().unwrap().kind,
                ErrorKind::ToolRejected
            );
        }
        let oversized = "x".repeat(MAX_NOTE_BYTES + 1);
        assert_eq!(
            stable_ids(&oversized, 0).err().unwrap().kind,
            ErrorKind::ToolRejected
        );
        assert!(
            stable_ids(&"x".repeat(MAX_NOTE_BYTES), 0)
                .unwrap()
                .is_empty()
        );
    }
}
