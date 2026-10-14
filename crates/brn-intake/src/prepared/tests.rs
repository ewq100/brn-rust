use super::*;
fn document() -> PreparedDocument {
    PreparedDocument {
        schema: 2,
        profile: PROFILE.into(),
        source_sha256: crate::digest(b"original"),
        format: "docx".into(),
        limits: IntakeLimits::default(),
        consumed: IntakeUsage {
            input_bytes: 8,
            ..Default::default()
        },
        assets: vec![],
        pages: vec![],
        gaps: vec![],
        blocks: vec![
            PreparedBlock {
                id: "heading".into(),
                locator: "body/heading".into(),
                content: BlockContent::Heading {
                    text: "Process".into(),
                    level: 1,
                },
            },
            PreparedBlock {
                id: "parent".into(),
                locator: "body/list/1".into(),
                content: BlockContent::ListItem {
                    text: "Run the check".into(),
                    level: 0,
                    label: "7.".into(),
                },
            },
            PreparedBlock {
                id: "child".into(),
                locator: "body/list/2".into(),
                content: BlockContent::ListItem {
                    text: "Use identifier 00123".into(),
                    level: 1,
                    label: "z.".into(),
                },
            },
            PreparedBlock {
                id: "table".into(),
                locator: "sheet/Sheet1".into(),
                content: BlockContent::Table {
                    rows: vec![vec![
                        PreparedCell {
                            address: "A1".into(),
                            column: 0,
                            text: "00123".into(),
                            formula: None,
                            result_status: None,
                            comment: None,
                            links: vec![],
                            row_span: 1,
                            column_span: 1,
                        },
                        PreparedCell {
                            address: "B1".into(),
                            column: 1,
                            text: String::new(),
                            formula: Some("SUM(A2:A3)".into()),
                            result_status: Some("uncached".into()),
                            comment: Some("Keep this blank cell note".into()),
                            links: vec!["https://example.org/reference".into()],
                            row_span: 1,
                            column_span: 1,
                        },
                    ]],
                },
            },
            PreparedBlock {
                id: "body".into(),
                locator: "body/p".into(),
                content: BlockContent::Paragraph {
                    text: "Qualification[^footnote-1] remains in force.".into(),
                },
            },
            PreparedBlock {
                id: "footnote".into(),
                locator: "footnotes/1".into(),
                content: BlockContent::Footnote {
                    label: "footnote-1".into(),
                    text: "Apply only after review.".into(),
                },
            },
            PreparedBlock {
                id: "header".into(),
                locator: "message/header/1".into(),
                content: BlockContent::Metadata {
                    name: "X-Trace".into(),
                    value: "abc/01".into(),
                },
            },
        ],
    }
}
fn candidate(doc: &PreparedDocument) -> ConversionCandidate {
    ConversionCandidate {
        schema: 2,
        source_sha256: doc.source_sha256,
        markdown: format(doc).unwrap(),
        page_reviews: vec![],
        crops: vec![],
        table_reviews: doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let BlockContent::Table { rows } = &b.content {
                    Some(TableReview {
                        block_id: b.id.clone(),
                        source_sha256: table_hash(rows).unwrap(),
                        uncertainty: String::new(),
                    })
                } else {
                    None
                }
            })
            .collect(),
    }
}
#[test]
fn prepared_draft_preserves_readable_structure() {
    let doc = document();
    let candidate = candidate(&doc);
    check_candidate(&doc, &candidate).unwrap();
    let html =
        markdown::to_html_with_options(&candidate.markdown, &markdown::Options::gfm()).unwrap();
    assert!(html.contains("<ul>\n<li>7. Run the check"));
    assert!(html.contains("Keep this blank cell note"));
    assert!(html.contains("footnote-1"));
}

#[test]
fn literal_punctuation_is_inert_and_internal_bindings_do_not_reach_note_prose() {
    let mut doc = document();
    let BlockContent::Table { rows } = &mut doc.blocks[3].content else {
        unreachable!()
    };
    rows[0][0].text = "--".into();
    doc.blocks.push(PreparedBlock {
        id: "punctuation".into(),
        locator: "body/literal".into(),
        content: BlockContent::Paragraph {
            text: "M-0110\n--\n+ Keep = value ~~exact~~\nhttps://example.org/slip-10#section-4 (reference)\n<!-- source:not-a-binding -->".into(),
        },
    });
    let bound = candidate(&doc);
    check_candidate(&doc, &bound).unwrap();
    let clean = readable_markdown(&doc, &bound).unwrap();
    assert!(!clean.contains("<!-- source:") && !clean.contains("<!-- cell:"));
    let html = markdown::to_html_with_options(&clean, &markdown::Options::gfm()).unwrap();
    assert!(html.contains("--") && html.contains("~~exact~~"));
    assert!(html.contains("&lt;!-- source:not-a-binding --&gt;"));
    assert!(html.contains("https://example.org/slip-10#section-4 (reference)"));
    assert!(!html.contains("%5C"));
    assert!(clean.contains("<br>Comment:"));
    let mut corrupt = bound;
    corrupt.markdown = corrupt
        .markdown
        .replace("\\-\\- <!-- cell:", "0 <!-- cell:");
    assert!(readable_markdown(&doc, &corrupt).is_err());
}
#[test]
fn changed_table_binding_and_leading_zero_are_rejected() {
    let doc = document();
    let original = candidate(&doc);
    for (from, to) in [
        ("00123", "123"),
        ("cell:table:A1", "cell:table:B1"),
        ("Keep this blank cell note", ""),
        ("A2\\:A3", "A3\\:A4"),
    ] {
        let mut changed = original.clone();
        changed.markdown = changed.markdown.replace(from, to);
        assert!(check_candidate(&doc, &changed).is_err(), "{from}");
    }
}
#[test]
fn nested_list_heading_footnote_and_metadata_changes_rejected() {
    let doc = document();
    let original = candidate(&doc);
    for (from, to) in [
        ("  - z", "- z"),
        ("# Process", "## Process"),
        ("[^footnote-1]:", "[^footnote-2]:"),
        ("abc/01", "abc/02"),
    ] {
        let mut changed = original.clone();
        changed.markdown = changed.markdown.replace(from, to);
        assert!(check_candidate(&doc, &changed).is_err(), "{from}");
    }
}
#[test]
fn unknown_images_and_mismatched_identity_rejected() {
    let doc = document();
    let mut changed = candidate(&doc);
    changed
        .markdown
        .push_str("\n![Injected](https://example.org/x.png)");
    assert!(check_candidate(&doc, &changed).is_err());
    changed = candidate(&doc);
    changed.source_sha256 = crate::digest(b"different");
    assert!(check_candidate(&doc, &changed).is_err());
}
#[test]
fn literal_control_text_is_not_markdown_structure() {
    let mut doc = document();
    doc.blocks = vec![PreparedBlock {
        id: "literal".into(),
        locator: "p".into(),
        content: BlockContent::Paragraph {
            text: "12. Keep <script> and **literal** [text].".into(),
        },
    }];
    let draft = candidate(&doc);
    check_candidate(&doc, &draft).unwrap();
    let html = markdown::to_html_with_options(&draft.markdown, &markdown::Options::gfm()).unwrap();
    assert!(!html.contains("<ol>"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("**literal**"));
}
#[test]
fn quotas_and_duplicate_coordinates_fail_before_provider() {
    let mut doc = document();
    doc.limits.max_input_bytes = 0;
    assert!(format(&doc).is_err());
    doc = document();
    if let BlockContent::Table { rows } = &mut doc.blocks[3].content {
        rows[0][1].address = "A1".into();
    }
    assert!(format(&doc).is_err());
}

#[test]
fn numeric_extensions_signs_decimals_and_added_qualifications_fail() {
    let doc = document();
    let original = candidate(&doc);
    for (from, to) in [
        ("00123", "001230"),
        ("00123", "-00123"),
        ("00123", "00123.5"),
        ("remains in force", "does not remain in force"),
        ("in force", "in force, except when convenient"),
    ] {
        let mut changed = original.clone();
        changed.markdown = changed.markdown.replace(from, to);
        assert_ne!(changed.markdown, original.markdown, "mutation must apply");
        assert!(check_candidate(&doc, &changed).is_err(), "{from} -> {to}");
    }
}

#[test]
fn host_plan_writes_all_nested_grammar_and_table_values() {
    let mut doc = document();
    doc.blocks.insert(
        3,
        PreparedBlock {
            id: "grandchild".into(),
            locator: "body/list/3".into(),
            content: BlockContent::ListItem {
                text: "Inspect the third level".into(),
                level: 2,
                label: "ab.".into(),
            },
        },
    );
    let plan = ConversionPlan {
        schema: 2,
        source_sha256: doc.source_sha256,
        pages: vec![],
        tables: doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let BlockContent::Table { rows } = &b.content {
                    Some(TableReview {
                        block_id: b.id.clone(),
                        source_sha256: table_hash(rows).unwrap(),
                        uncertainty: String::new(),
                    })
                } else {
                    None
                }
            })
            .collect(),
        crops: vec![],
    };
    let candidate = apply_plan(&doc, &plan).unwrap();
    assert_eq!(candidate.markdown, format(&doc).unwrap());
    check_candidate(&doc, &candidate).unwrap();
    let mut invalid = plan.clone();
    invalid.tables[0].source_sha256 = [0; 32];
    assert!(apply_plan(&doc, &invalid).is_err());
    invalid.tables.clear();
    assert!(apply_plan(&doc, &invalid).is_err());
}
#[test]
fn unbound_prose_raw_html_and_role_swaps_fail() {
    let doc = document();
    let original = candidate(&doc);
    let mut changed = original.clone();
    changed.markdown.push_str("Release approved.");
    assert!(check_candidate(&doc, &changed).is_err());
    changed = original.clone();
    changed
        .markdown
        .push_str("<img src=\"https://example.org/tracker\">");
    assert!(check_candidate(&doc, &changed).is_err());
    changed = original.clone();
    changed.markdown = changed.markdown.replace(
        "Formula: SUM(A2\\:A3\\)<br>Result status: uncached",
        "Result status: SUM(A2\\:A3\\)<br>Formula: uncached",
    );
    assert_ne!(changed.markdown, original.markdown);
    assert!(check_candidate(&doc, &changed).is_err());
}
