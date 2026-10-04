use brn_store::{MAX_NOTE_BYTES, note_identity};

const ID: &str = "9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d";
const BOM: &str = "\u{feff}";

#[test]
fn plain_markdown_starts_after_only_its_optional_leading_bom() {
    for body in [
        "",
        "# Eesti õäöü λ\r\n[Link](source.md)\r\n",
        "```yaml\nbrn_id: literal\n---\n```\n",
        "\n---\nordinary thematic break\n",
        "...\n[Link](source.md)\n",
    ] {
        for bom in ["", BOM] {
            let text = format!("{bom}{body}");
            let before = text.as_bytes().to_vec();
            let offset = note_identity::body_start(&text).unwrap();
            assert_eq!(offset, bom.len());
            assert_eq!(&text.as_bytes()[offset..], body.as_bytes());
            assert_eq!(text.as_bytes(), before);
        }
    }
    assert_eq!(
        note_identity::body_start(&format!("{BOM}{BOM}Body")).unwrap(),
        BOM.len()
    );
}

#[test]
fn complete_frontmatter_returns_exact_utf8_body_offset_for_both_fences_and_newlines() {
    for newline in ["\n", "\r\n"] {
        for closing in ["---", "..."] {
            for bom in ["", BOM] {
                let header = format!(
                    "{bom}---{newline}custom: '日本語 🦀'{newline}brn_id: {ID}{newline}brn_kind: source{newline}brn_state: history{newline}brn_provenance: []{newline}{closing}{newline}"
                );
                let body = format!(
                    "# Body õäöü λ{newline}[Note](target.md){newline}```yaml{newline}---{newline}brn_id: invalid body text{newline}...{newline}```{newline}"
                );
                let text = format!("{header}{body}");
                let before = text.as_bytes().to_vec();
                let offset = note_identity::body_start(&text).unwrap();
                assert_eq!(offset, header.len());
                assert!(text.is_char_boundary(offset));
                assert_eq!(&text.as_bytes()[offset..], body.as_bytes());
                assert_eq!(text.as_bytes(), before);
                assert_eq!(note_identity::read(&text).unwrap().unwrap().to_string(), ID);
            }
        }
    }
}

#[test]
fn a_closing_fence_without_a_final_newline_has_an_empty_body() {
    for newline in ["\n", "\r\n"] {
        for closing in ["---", "..."] {
            for bom in ["", BOM] {
                let text = format!("{bom}---{newline}custom: λ{newline}{closing}");
                let offset = note_identity::body_start(&text).unwrap();
                assert_eq!(offset, text.len());
                assert_eq!(&text[offset..], "");
            }
        }
    }
    assert_eq!(note_identity::body_start("---\n---").unwrap(), 7);
}

#[test]
fn opaque_indented_metadata_does_not_end_frontmatter_or_become_a_managed_field() {
    for newline in ["\n", "\r\n"] {
        let header = format!(
            "{BOM}---{newline}summary: |{newline}  ---{newline}  ...{newline}  brn_id: literal{newline}  brn_kind: literal{newline}nested:{newline}\tbrn_state: unrelated{newline}\tbrn_provenance: opaque{newline}\t---{newline}\t...{newline}'custom': >-{newline}  {{brn_id: unrelated}}{newline}...{newline}"
        );
        let body = format!("[Exact λ](target.md){newline}");
        let text = format!("{header}{body}");
        assert_eq!(note_identity::body_start(&text).unwrap(), header.len());
        assert_eq!(&text[header.len()..], body);
        assert_eq!(note_identity::read(&text).unwrap(), None);
    }
}

#[test]
fn incomplete_thematic_break_without_managed_metadata_remains_body() {
    for body in [
        "---",
        "---\n",
        "---\n# Work\nText [Exact](target.md)\n",
        "---\r\ncustom: retained\r\n",
        "---\n# brn_id: comment, not a managed root field\n",
        "---\nsummary: |\n  brn_id: opaque\n  ---\n",
    ] {
        for bom in ["", BOM] {
            let text = format!("{bom}{body}");
            let offset = note_identity::body_start(&text).unwrap();
            assert_eq!(offset, bom.len());
            assert_eq!(&text[offset..], body);
        }
    }
}

#[test]
fn naked_link_after_unmanaged_thematic_break_is_body_but_not_assignable_frontmatter() {
    let id = uuid::Uuid::parse_str(ID).unwrap();
    for newline in ["\n", "\r\n"] {
        for bom in ["", BOM] {
            let body = format!("---{newline}[ordinary](../archive/t%C3%B5end.md){newline}");
            let text = format!("{bom}{body}");
            let before = text.as_bytes().to_vec();
            assert_eq!(note_identity::read(&text).unwrap(), None);
            assert_eq!(note_identity::body_start(&text).unwrap(), bom.len());
            assert_eq!(&text[bom.len()..], body);
            assert!(note_identity::assign(&text, id).is_err());
            assert_eq!(text.as_bytes(), before);
        }
    }
    for key in ["brn_id", "brn_kind", "brn_state", "brn_provenance"] {
        for managed in [
            format!("{key}: opaque"),
            format!("'{key}': opaque"),
            format!("{{title: literal, {key}: opaque}}"),
        ] {
            let text = format!("---\n[ordinary](target.md)\n{managed}\n");
            assert!(note_identity::body_start(&text).is_err(), "{managed}");
            assert!(note_identity::assign(&text, id).is_err(), "{managed}");
        }
    }
    let invalid_closer = "---\n[ordinary](target.md)\n... # unsupported closer\n";
    assert!(note_identity::body_start(invalid_closer).is_err());
    assert!(note_identity::read(invalid_closer).is_err());
}

#[test]
fn managed_layouts_stay_structurally_strict_without_interpreting_their_values() {
    for key in ["brn_id", "brn_kind", "brn_state", "brn_provenance"] {
        for unsupported in [
            format!("---\n{key}: opaque\n"),
            format!("---\n{key}: opaque\n{key}: opaque\n---\nBody"),
            format!("---\n{key}: opaque\n  continuation\n---\nBody"),
            format!("---\n'{key}': opaque\n---\nBody"),
            format!("---\n{key}:opaque\n---\nBody"),
            format!("---\n{key} : opaque\n---\nBody"),
            format!("---\n {key}: opaque\n---\nBody"),
        ] {
            assert!(
                note_identity::body_start(&unsupported).is_err(),
                "{unsupported}"
            );
        }
    }
    let header = "---\nbrn_id: invalid UUID\nbrn_kind: custom\nbrn_state: unknown\nbrn_provenance: invalid JSON\n---\n";
    let text = format!("{header}[Body](target.md)");
    assert_eq!(note_identity::body_start(&text).unwrap(), header.len());
    assert!(note_identity::read(&text).is_err());
    assert!(brn_store::note_metadata::classify(&text).is_err());
    assert!(brn_store::note_provenance::read(&text).is_err());
}

#[test]
fn unsupported_delimiters_and_root_flow_preserve_existing_refusal() {
    for text in [
        "--- \ncustom: retained\n---\nBody",
        " ---\ncustom: retained\n---\nBody",
        "---\ncustom: retained\n--- # unsupported closer\nBody",
        "---\ncustom: retained\n... \nBody",
        "---\n{title: unrelated root flow}\n---\nBody",
        "---\n[unrelated, root, flow]\n---\nBody",
    ] {
        assert!(note_identity::body_start(text).is_err(), "{text}");
        assert!(note_identity::read(text).is_err(), "{text}");
    }
}

#[test]
fn inspection_enforces_the_complete_utf8_note_byte_limit() {
    let header = format!("{BOM}---\r\nbrn_id: {ID}\r\n...\r\n");
    let at_limit = format!("{header}{}", "a".repeat(MAX_NOTE_BYTES - header.len()));
    assert_eq!(at_limit.len(), MAX_NOTE_BYTES);
    assert_eq!(note_identity::body_start(&at_limit).unwrap(), header.len());
    assert!(note_identity::body_start(&(at_limit + "λ")).is_err());
    assert_eq!(
        note_identity::body_start(&"a".repeat(MAX_NOTE_BYTES)).unwrap(),
        0
    );
    assert!(note_identity::body_start(&"a".repeat(MAX_NOTE_BYTES + 1)).is_err());
}
