use brn_store::{
    MAX_NOTE_BYTES, note_identity, note_metadata,
    note_provenance::{self, MAX_CITATIONS, MAX_QUOTE_BYTES, VaultCitation},
};
use uuid::Uuid;

fn citation(quote: &str) -> VaultCitation {
    VaultCitation {
        note_id: Uuid::parse_str("ac012deb-f344-4af4-a93c-c0d8bd86e3de").unwrap(),
        sha256: [37; 32],
        start_byte: 101,
        end_byte: 101 + quote.len(),
        quote: quote.into(),
    }
}

fn field(citations: &[VaultCitation]) -> String {
    format!(
        "brn_provenance: {}",
        serde_json::to_string(citations).unwrap()
    )
}

#[test]
fn exact_quotes_and_unrelated_bom_crlf_bytes_survive_insert_read_and_replace() {
    let original = "\u{feff}---\r\ncustom: 'õäöü'\r\nsummary: |\r\n  brn_provenance: literal text\r\n  ---\r\nnested:\r\n  brn_provenance: unrelated\r\n---\r\nBody 日本語\r\n";
    let first = vec![citation("Original \"õäöü\" \\ λ\r\n\t")];
    let assigned = note_provenance::write(original, &first).unwrap();
    let expected = original.replacen("---\r\n", &format!("---\r\n{}\r\n", field(&first)), 1);
    assert_eq!(assigned.as_bytes(), expected.as_bytes());
    assert_eq!(note_provenance::read(&assigned).unwrap(), first);
    assert_eq!(note_provenance::write(&assigned, &first).unwrap(), assigned);

    let next = vec![citation("A different 原文\n"), citation("Second quote")];
    let replaced = note_provenance::write(&assigned, &next).unwrap();
    assert_eq!(
        replaced,
        assigned.replacen(&field(&first), &field(&next), 1)
    );
    assert_eq!(note_provenance::read(&replaced).unwrap(), next);
    let cleared = note_provenance::write(&replaced, &[]).unwrap();
    assert_eq!(
        cleared,
        replaced.replacen(&field(&next), "brn_provenance: []", 1)
    );
    assert!(note_provenance::read(&cleared).unwrap().is_empty());
}

#[test]
fn absent_empty_is_exact_and_new_headers_keep_existing_body_and_newline_style() {
    for original in [
        "",
        "Body λ",
        "Body λ\n",
        "\u{feff}Body λ\r\n",
        "---\n# A thematic break\n",
    ] {
        assert!(note_provenance::read(original).unwrap().is_empty());
        assert_eq!(note_provenance::write(original, &[]).unwrap(), original);
    }
    let cites = vec![citation("Evidence")];
    for (original, prefix) in [
        ("Body λ\n", "---\n"),
        ("\u{feff}Body λ\r\n", "\u{feff}---\r\n"),
    ] {
        let proposed = note_provenance::write(original, &cites).unwrap();
        assert!(proposed.starts_with(prefix));
        assert!(proposed.ends_with(original.trim_start_matches('\u{feff}')));
        assert_eq!(note_provenance::read(&proposed).unwrap(), cites);
    }
    assert!(note_provenance::write("---\n# Ambiguous incomplete header\n", &cites).is_err());
}

#[test]
fn identical_typed_json_is_an_exact_noop_and_replacement_keeps_field_line_ending() {
    let cites = vec![citation("õäöü")];
    let json = serde_json::to_string(&cites).unwrap().replace(':', ": ");
    let original =
        format!("---\r\ncustom: unchanged\nbrn_provenance:\t {json} \n...\r\nBody λ\r\n");
    assert_eq!(note_provenance::read(&original).unwrap(), cites);
    assert_eq!(note_provenance::write(&original, &cites).unwrap(), original);
    let next = vec![citation("New quote")];
    let replaced = note_provenance::write(&original, &next).unwrap();
    assert_eq!(
        replaced,
        format!(
            "---\r\ncustom: unchanged\n{}\n...\r\nBody λ\r\n",
            field(&next)
        )
    );
}

#[test]
fn unsupported_managed_frontmatter_is_refused_without_automatic_repair() {
    for metadata in [
        "brn_provenance: []\nbrn_provenance: []",
        "brn_provenance:[]",
        "brn_provenance: [] # trailing comment",
        "brn_provenance: '[]'",
        "brn_provenance: \"[]\"",
        "brn_provenance: []\n  continued text",
        "brn_provenance: []\n  # comment\n \t\n  continued text",
        "brn_provenance: [\n]",
        "brn_provenance: |\n  []",
        " brn_provenance: []",
        "'brn_provenance': []",
        "\"brn_provenance\": []",
        "? brn_provenance: []",
        "- brn_provenance: []",
        "brn_provenance = []",
        "{title: unrelated, brn_provenance: []}",
    ] {
        let text = format!("---\n{metadata}\n---\nBody λ\n");
        assert!(note_provenance::read(&text).is_err(), "{metadata:?}");
        assert!(note_provenance::write(&text, &[]).is_err(), "{metadata:?}");
        assert!(
            note_provenance::write(&text, &[citation("quote")]).is_err(),
            "{metadata:?}"
        );
    }
    for text in [
        "---\nbrn_provenance: []\n",
        "--- \nbrn_provenance: []\n---\n",
        "---\nbrn_provenance: []\n--- # comment\n",
    ] {
        assert!(note_provenance::read(text).is_err(), "{text:?}");
    }
}

#[test]
fn absent_provenance_does_not_add_a_legacy_yaml_or_body_constraint() {
    for text in [
        "---\n{title: unrelated}\n---\nBody brn_provenance: []\n",
        "---\n[unrelated, values]\n---\nBody\n",
        "---\n{title: \"brn_provenance: []\"}\n---\nBody\n",
        "---\n{nested: {brn_provenance: []}}\n---\nBody\n",
        "---\nnested:\n  brn_provenance: []\n---\nBody\n",
        "---\nsummary: |\n  brn_provenance: []\n  {brn_provenance: []}\n---\nBody\n",
        "---\nsummary: |\n\n# retained comment\n  brn_provenance: literal\n---\nBody\n",
        "---\ncustom: ordinary\n---\nbrn_provenance: []\n",
        "brn_provenance: []\n",
    ] {
        assert!(note_provenance::read(text).unwrap().is_empty(), "{text:?}");
        assert_eq!(note_provenance::write(text, &[]).unwrap(), text);
    }
    for flow in [
        "{title: unrelated, brn_provenance: []}",
        "{title: unrelated, 'brn_provenance': []}",
        "{\"brn_provenance\": []}",
        "[brn_provenance: []]",
    ] {
        assert!(
            note_provenance::read(&format!("---\n{flow}\n---\nBody\n")).is_err(),
            "{flow}"
        );
    }
    assert!(note_provenance::read("  ---\nbrn_provenance: []\n---\n").is_err());
}

#[test]
fn strict_citation_json_refuses_duplicate_keys_unknown_members_and_wrong_shapes() {
    let json = serde_json::to_string(&citation("quote")).unwrap();
    let duplicate = json.replacen("{", "{\"quote\":\"quote\",", 1);
    let unknown = json.replacen("{", "{\"path\":\"source.md\",", 1);
    for json in [
        duplicate,
        unknown,
        "null".into(),
        "{}".into(),
        "\"citation\"".into(),
        json.replace("101", "-1"),
    ] {
        let text = format!("---\nbrn_provenance: [{json}]\n---\nBody\n");
        assert!(note_provenance::read(&text).is_err(), "{json}");
    }
    for json in [
        "null",
        "{}",
        "[{}]",
        "[true]",
        "[[]]",
        "[] trailing",
        "[] // comment",
    ] {
        assert!(
            note_provenance::read(&format!("---\nbrn_provenance: {json}\n---\n")).is_err(),
            "{json}"
        );
    }
}

#[test]
fn citation_ranges_quotes_counts_and_duplicates_have_exact_bounds() {
    let valid = citation("🦀õäöü\r\n");
    valid.validate().unwrap();
    for invalid in [
        VaultCitation {
            note_id: Uuid::nil(),
            ..valid.clone()
        },
        VaultCitation {
            quote: String::new(),
            ..valid.clone()
        },
        VaultCitation {
            start_byte: valid.end_byte,
            ..valid.clone()
        },
        VaultCitation {
            start_byte: valid.end_byte + 1,
            ..valid.clone()
        },
        VaultCitation {
            end_byte: MAX_NOTE_BYTES + 1,
            ..valid.clone()
        },
        VaultCitation {
            end_byte: valid.end_byte + 1,
            ..valid.clone()
        },
        citation(&"a".repeat(MAX_QUOTE_BYTES + 1)),
    ] {
        assert!(invalid.validate().is_err(), "{invalid:?}");
        assert!(note_provenance::read(&format!("---\n{}\n---\n", field(&[invalid]))).is_err());
    }
    let bounded = citation(&"a".repeat(MAX_QUOTE_BYTES));
    bounded.validate().unwrap();
    let end_bound = VaultCitation {
        start_byte: MAX_NOTE_BYTES - 1,
        end_byte: MAX_NOTE_BYTES,
        ..citation("a")
    };
    end_bound.validate().unwrap();
    let many: Vec<_> = (0..MAX_CITATIONS)
        .map(|index| VaultCitation {
            start_byte: index,
            end_byte: index + 1,
            ..citation("a")
        })
        .collect();
    note_provenance::validate(&many).unwrap();
    let mut too_many = many.clone();
    too_many.push(VaultCitation {
        start_byte: MAX_CITATIONS,
        end_byte: MAX_CITATIONS + 1,
        ..citation("a")
    });
    assert!(note_provenance::validate(&too_many).is_err());
    assert!(note_provenance::validate(&[valid.clone(), valid]).is_err());
    assert!(note_provenance::read(&format!("---\n{}\n---\n", field(&too_many))).is_err());
}

#[test]
fn complete_note_byte_budget_includes_encoded_quotes_and_replacement() {
    let cites = vec![citation("quote")];
    let header = format!("---\n{}\n---\n", field(&cites));
    let body = "a".repeat(MAX_NOTE_BYTES - header.len());
    let exact = note_provenance::write(&body, &cites).unwrap();
    assert_eq!(exact.len(), MAX_NOTE_BYTES);
    assert_eq!(note_provenance::read(&exact).unwrap(), cites);
    assert_eq!(note_provenance::write(&exact, &cites).unwrap(), exact);
    assert!(note_provenance::write(&(body + "a"), &cites).is_err());
    assert!(note_provenance::write(&exact, &[citation("larger quote")]).is_err());
    let shorter = note_provenance::write(&exact, &[]).unwrap();
    assert!(shorter.len() < MAX_NOTE_BYTES);
    assert!(note_provenance::write(&"x".repeat(MAX_NOTE_BYTES + 1), &[]).is_err());
}

#[test]
fn identity_and_classification_keep_their_own_scalar_and_opaque_metadata_semantics() {
    let id = Uuid::new_v4();
    let original = format!(
        "\u{feff}---\r\nbrn_id: '{id}' # retained\r\nbrn_kind: \"source\" # retained\r\nbrn_state: history\r\nsummary: |\r\n  brn_provenance: literal\r\n  brn_id: opaque\r\ncustom: {{brn_state: unrelated}}\r\n---\r\nBody õäöü\r\n"
    );
    let proposed = note_provenance::write(&original, &[citation("source\r\n")]).unwrap();
    assert_eq!(note_identity::read(&proposed).unwrap(), Some(id));
    assert_eq!(note_identity::assign(&proposed, id).unwrap(), proposed);
    assert_eq!(
        note_metadata::classify(&proposed).unwrap(),
        note_metadata::NoteClassification {
            source: true,
            history: true
        }
    );
    let malformed_provenance = original.replacen(
        "summary: |",
        "brn_provenance: historic unsupported text\r\nsummary: |",
        1,
    );
    assert!(note_provenance::read(&malformed_provenance).is_err());
    assert_eq!(
        note_identity::read(&malformed_provenance).unwrap(),
        Some(id)
    );
    assert!(
        note_metadata::classify(&malformed_provenance)
            .unwrap()
            .source
    );
}
