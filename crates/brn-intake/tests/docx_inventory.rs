#![cfg(feature = "helper")]
#[path = "support/office.rs"]
mod office;
use brn_intake::{helper, prepared::BlockContent, *};
use office::*;
use serde_json::json;

#[test]
fn current_docx_inventory_complete_ordered_schema1_and_schema2() {
    for case in cases() {
        check_current(&case);
    }
}
#[test]
fn current_docx_inventory_comparator_detects_loss_and_rebinding() {
    // Mutation of synthetic evidence, not a compiler-error or keyword-only witness.
    let case = cases().remove(0);
    let doc = helper::prepared::prepare(request(&case.bytes, IntakeLimits::default())).unwrap();
    let original = prepared_atoms(&doc);
    let mut changed = doc.clone();
    changed.blocks.remove(1);
    assert_ne!(prepared_atoms(&changed), original);
    let mut changed = doc.clone();
    for block in &mut changed.blocks {
        if let BlockContent::Table { rows } = &mut block.content {
            rows[1][1].text = "wrong number".into();
            rows[0][0].column_span = 2;
            break;
        }
    }
    assert_ne!(prepared_atoms(&changed), original);
    let mut changed = doc.clone();
    let images: Vec<_> = changed
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b.content, BlockContent::Figure { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(images.len(), 2);
    changed.blocks.remove(images[1]);
    assert_ne!(prepared_atoms(&changed), original);
    let mut evidence = helper::extract(request(&case.bytes, IntakeLimits::default())).unwrap();
    evidence.occurrences[0].source_id = "nonexistent-source".into();
    assert!(
        evidence.validate().is_err(),
        "source rebinding must fail DTO validation"
    );
}
#[test]
fn current_docx_inventory_revisions_comments_source_context() {
    let bytes = revisions_comments();
    let e = helper::extract(request(&bytes, IntakeLimits::default())).unwrap();
    let d = helper::prepared::prepare(request(&bytes, IntakeLimits::default())).unwrap();
    let wording: Vec<_> = prepared_atoms(&d).iter().filter_map(text_of).collect();
    assert_eq!(
        wording,
        [
            "Keep accepted.",
            "Reviewed.",
            "Reviewer note.",
            "Reply note."
        ]
    );
    assert_eq!(markdown_wording(&e.markdown), wording);
    let context: Vec<_> = d
        .blocks
        .iter()
        .filter_map(|b| match &b.content {
            BlockContent::Metadata { name, value } if name == "Source comment context" => {
                Some(json!({"locator":b.locator,"value":value}))
            }
            _ => None,
        })
        .collect();
    // Exact opaque source-range formatting awaits measurement; never reconstruct it from Markdown.
    record(
        "S2",
        "current-comments",
        json!({"author":"Iris","id":"7","date":"2026-10-18T10:00:00Z","range_text":"Reviewed.","reply":{"id":"8","parent":"7","resolved":true}}),
        json!(context),
    );
}
#[test]
fn current_docx_inventory_repeated_targets_keep_two_exact_occurrences() {
    let bytes = repeated_figures();
    let e = helper::extract(request(&bytes, IntakeLimits::default())).unwrap();
    let d = helper::prepared::prepare(request(&bytes, IntakeLimits::default())).unwrap();
    assert_eq!(e.assets.len(), 1);
    assert_eq!(e.assets[0].bytes, include_bytes!("fixtures/water-use.png"));
    assert_eq!(e.occurrences.len(), 2);
    assert_eq!(e.occurrences[0].asset_id, e.occurrences[1].asset_id);
    assert_ne!(e.occurrences[0].locator, e.occurrences[1].locator);
    assert!(e.occurrences[0].end < e.occurrences[1].start);
    assert_eq!(
        prepared_atoms(&d),
        vec![
            Atom::Text("Same caption.".into()),
            Atom::Figure(hex(&e.assets[0].sha256)),
            Atom::Text("Same caption.".into()),
            Atom::Figure(hex(&e.assets[0].sha256))
        ]
    );
}
#[test]
fn current_docx_inventory_empty_orphan_notes_and_grid_boundaries() {
    for (name, bytes, source) in [
        (
            "current-orphan",
            empty_orphan(),
            json!({"body":["","Visible."],"bound_header":"Bound header.","orphan_header":"Orphan header."}),
        ),
        (
            "current-notes",
            note_collisions(),
            json!({"literal":"Literal [^1] and [cmt:7]. ","footnote_1":"Footnote one.","endnote_1":"Endnote one.","table_note_2":"Cell note.","noteref":{"bookmark":"rule","cached":"1"}}),
        ),
        ("current-grids", grids(), grid_source()),
    ] {
        let d = helper::prepared::prepare(request(&bytes, IntakeLimits::default())).unwrap();
        d.validate().unwrap();
        let e = helper::extract(request(&bytes, IntakeLimits::default())).unwrap();
        e.validate().unwrap();
        record(
            "S6",
            name,
            source,
            json!({"atoms":prepared_atoms(&d),"blocks":d.blocks,"gaps":d.gaps,"schema1":e.markdown,"schema1_gaps":e.gaps}),
        );
    }
}
fn current_refusal(bytes: &[u8], limits: IntakeLimits, class: &str) {
    let errors = [
        helper::extract(request(bytes, limits.clone())).unwrap_err(),
        helper::prepared::prepare(request(bytes, limits)).unwrap_err(),
    ];
    for error in errors {
        let lower = error.to_ascii_lowercase();
        let correct = match class {
            "dtd" => lower.contains("dtd") || lower.contains("doctype"),
            "path" => lower.contains("path"),
            "expanded" => lower.contains("inflated size exceeds"),
            "parts" => lower.contains("package member budget"),
            "depth" => lower.contains("depth"),
            _ => panic!("unallocated refusal class"),
        };
        assert!(correct, "wrong {class} refusal: {error}");
    }
}
#[test]
fn current_office_admission_existing_hostile_inputs_and_job_limits() {
    for (bytes, class) in [
        (
            include_bytes!("fixtures/hostile-dtd.docx").as_slice(),
            "dtd",
        ),
        (
            include_bytes!("fixtures/hostile-path.docx").as_slice(),
            "path",
        ),
        (
            include_bytes!("fixtures/hostile-inflate.docx").as_slice(),
            "expanded",
        ),
        (
            include_bytes!("fixtures/hostile-members.docx").as_slice(),
            "parts",
        ),
    ] {
        current_refusal(bytes, IntakeLimits::default(), class);
    }
    let bytes = package(&paragraph("Boundary."), "", &[]);
    let count = ooxml_opc::unzip_parts(&bytes).unwrap().len();
    let expanded: usize = ooxml_opc::unzip_parts(&bytes)
        .unwrap()
        .iter()
        .map(|(_, b)| b.len())
        .sum();
    let at = IntakeLimits {
        max_package_parts: count,
        max_expanded_bytes: expanded,
        ..Default::default()
    };
    let e = helper::extract(request(&bytes, at.clone())).unwrap();
    assert_eq!(e.consumed.unwrap().expanded_bytes, expanded);
    for (limits, class) in [
        (
            IntakeLimits {
                max_package_parts: count - 1,
                ..at.clone()
            },
            "parts",
        ),
        (
            IntakeLimits {
                max_expanded_bytes: expanded - 1,
                ..at
            },
            "expanded",
        ),
    ] {
        current_refusal(&bytes, limits, class);
    }
    for kind in ["docm", "dotx", "pptm", "ppsx", "application/msword"] {
        assert!(
            helper::extract(HelperRequest {
                kind: kind.into(),
                bytes: bytes.clone(),
                limits: None
            })
            .is_err()
        );
    }
}
#[test]
fn current_office_admission_descriptors_empty_members_and_malformed_baseline() {
    let note = "<w:footnotes xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:footnote w:id=\"1\"><w:p><w:r><w:t>Optional note.</w:t></w:r></w:p></w:footnote></w:footnotes>";
    let bytes = package(
        &paragraph("Boundary."),
        &relationship("fn", "footnotes", "footnotes.xml"),
        &[("word/footnotes.xml", note)],
    );
    for valid in [
        descriptor(&bytes, "word/document.xml"),
        stored(&bytes, true),
    ] {
        let e = helper::extract(request(&valid, IntakeLimits::default())).unwrap();
        let d = helper::prepared::prepare(request(&valid, IntakeLimits::default())).unwrap();
        assert_eq!(
            markdown_wording(&e.markdown),
            ["Boundary.", "Optional note."]
        );
        assert_eq!(
            prepared_atoms(&d),
            vec![
                Atom::Text("Boundary.".into()),
                Atom::Text("Optional note.".into())
            ]
        );
    }
    for (member, source) in [
        ("word/document.xml", bytes.clone()),
        ("word/footnotes.xml", bytes.clone()),
        ("word/media/same.png", repeated_figures()),
    ] {
        for delta in [-1, 1] {
            let malformed = size_lie(&source, member, delta, false);
            let a = helper::extract(request(&malformed, IntakeLimits::default()));
            let b = helper::prepared::prepare(request(&malformed, IntakeLimits::default()));
            assert_eq!(a.as_ref().unwrap_err(), A1_SIZE_ERROR);
            assert_eq!(b.as_ref().unwrap_err(), A1_SIZE_ERROR);
            println!(
                "A1_CURRENT_SIZE {}",
                json!({"class":"S6","witness":format!("current-size-{member}-{delta}"),"measurement":"EvidenceOnly","expected":{"member":member,"central_minus_actual":delta,"contract":"reject inconsistent member with size-specific refusal after bounded inflation"},"actual":{"schema1":a.as_ref().map(|e|json!({"wording":markdown_wording(&e.markdown),"usage":e.consumed})),"schema2":b.as_ref().map(prepared_atoms)}})
            );
            record(
                "S6",
                &format!("current-size-{member}-{delta}"),
                json!({"member":member,"central_minus_actual":delta,"contract":"reject inconsistent member with size-specific refusal after bounded inflation"}),
                json!({"schema1":a.map(|e|json!({"wording":markdown_wording(&e.markdown),"usage":e.consumed})),"schema2":b.map(|d|prepared_atoms(&d))}),
            );
        }
    }
    assert!(
        helper::extract(request(
            &size_lie(&bytes, "word/document.xml", 0, true),
            IntakeLimits::default()
        ))
        .is_err()
    );
}
#[test]
fn current_office_admission_alias_dtd_depth_and_relationship_count() {
    let bytes = package(&paragraph("Admitted."), "", &[]);
    let original = ooxml_opc::unzip_parts(&bytes).unwrap();
    for alias in ["word/./document.xml", "WORD/document.xml", "../escape.xml"] {
        let mut parts = original.clone();
        parts.push((alias.into(), b"opaque".to_vec()));
        let bad = zip_parts(parts);
        current_refusal(&bad, IntakeLimits::default(), "path");
    }
    let mut dtd = original.clone();
    dtd.iter_mut().find(|(n,_)|n=="word/document.xml").unwrap().1=br#"<!DOCTYPE document [<!ENTITY value "not allowed">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>&value;</w:t></w:r></w:p></w:body></w:document>"#.to_vec();
    current_refusal(
        &ooxml_opc::rezip_parts(&dtd).unwrap(),
        IntakeLimits::default(),
        "dtd",
    );
    let body = format!(
        "{}{}{}",
        "<w:sdt><w:sdtContent>".repeat(130),
        paragraph("Deep"),
        "</w:sdtContent></w:sdt>".repeat(130)
    );
    current_refusal(&package(&body, "", &[]), IntakeLimits::default(), "depth");
    let rels = (0..513)
        .map(|i| relationship(&format!("r{i}"), "image", "media/missing.png"))
        .collect::<String>();
    let bytes = package(&paragraph("Many relationships."), &rels, &[]);
    record("S6","current-relationship-count",json!({"relationship_count":513,"policy":"measure existing parse/count behavior; no new relationship quota invented"}),json!(helper::extract(request(&bytes,IntakeLimits::default())).map(|e|json!({"usage":e.consumed,"gaps":e.gaps,"wording":markdown_wording(&e.markdown)}))));
}

const A1_SIZE_ERROR: &str = "Office package member-size-mismatch";
fn a1_request(kind: &str, bytes: &[u8], limits: IntakeLimits) -> HelperRequest {
    let mut r = request(bytes, limits);
    r.kind = kind.into();
    r
}
fn a1_stats(bytes: &[u8]) -> (usize, usize) {
    let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let expanded = ooxml_opc::unzip_parts(bytes)
        .unwrap()
        .iter()
        .map(|(_, b)| b.len())
        .sum();
    (archive.len(), expanded)
}
fn a1_valid(kind: &str, original: &[u8], variant: &[u8]) {
    let (parts, expanded) = a1_stats(variant);
    let limits = IntakeLimits {
        max_package_parts: parts,
        max_expanded_bytes: expanded,
        ..Default::default()
    };
    let base =
        helper::prepared::prepare(a1_request(kind, original, IntakeLimits::default())).unwrap();
    let d = helper::prepared::prepare(a1_request(kind, variant, limits.clone())).unwrap();
    d.validate().unwrap();
    assert_eq!(d.source_sha256, digest(variant));
    assert_eq!(d.format, kind);
    assert_eq!(d.blocks, base.blocks);
    assert_eq!(d.assets, base.assets);
    assert_eq!(d.pages, base.pages);
    assert_eq!(d.gaps, base.gaps);
    assert_eq!(
        (d.consumed.package_parts, d.consumed.expanded_bytes),
        (parts, expanded)
    );
    if kind != "xlsx" {
        let mut base =
            helper::extract(a1_request(kind, original, IntakeLimits::default())).unwrap();
        let e = helper::extract(a1_request(kind, variant, limits)).unwrap();
        e.validate().unwrap();
        assert_eq!(e.original_sha256, digest(variant));
        assert_eq!(e.sources[0].bytes, variant);
        base.sources[0].bytes = variant.to_vec();
        assert_eq!(e.sources, base.sources);
        assert_eq!(e.markdown, base.markdown);
        assert_eq!(e.assets, base.assets);
        assert_eq!(e.occurrences, base.occurrences);
        assert_eq!(e.gaps, base.gaps);
        let usage = e.consumed.unwrap();
        assert_eq!(
            (usage.package_parts, usage.expanded_bytes),
            (parts, expanded)
        );
    } else {
        let cells: Vec<_> = d
            .blocks
            .iter()
            .flat_map(|b| match &b.content {
                BlockContent::Table { rows } => rows
                    .iter()
                    .flatten()
                    .map(|c| {
                        (
                            c.address.as_str(),
                            c.text.as_str(),
                            c.row_span,
                            c.column_span,
                        )
                    })
                    .collect(),
                _ => vec![],
            })
            .collect();
        assert_eq!(
            cells,
            [
                ("A1", "Out of scope forecast", 1, 1),
                ("B1", "EUR", 1, 1),
                ("A2", "Harbor budget", 1, 1),
                ("B2", "4000", 1, 1)
            ]
        );
    }
}
fn a1_boundary() -> Vec<u8> {
    package(
        &paragraph("Boundary."),
        &relationship("fn", "footnotes", "footnotes.xml"),
        &[(
            "word/footnotes.xml",
            "<w:footnotes xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:footnote w:id=\"1\"><w:p><w:r><w:t>Optional note.</w:t></w:r></w:p></w:footnote></w:footnotes>",
        )],
    )
}
fn a1_sizes(kind: &str, source: &[u8], members: &[&str]) -> Vec<serde_json::Value> {
    a1_valid(kind, source, source);
    let actual: std::collections::BTreeMap<_, _> = ooxml_opc::unzip_parts(source)
        .unwrap()
        .into_iter()
        .collect();
    let mut failures = vec![];
    for member in members {
        for delta in [-1, 1] {
            let bad = size_lie(source, member, delta, false);
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bad)).unwrap();
            let entry = (0..archive.len())
                .find_map(|i| {
                    let e = archive.by_index_raw(i).unwrap();
                    (e.name() == *member).then_some((e.size(), e.is_dir()))
                })
                .unwrap();
            assert!(!entry.1);
            assert_eq!(
                entry.0 as i64,
                actual[*member].len() as i64 + i64::from(delta)
            );
            let mut errors = vec![];
            if kind != "xlsx" {
                errors.push(
                    helper::extract(a1_request(kind, &bad, IntakeLimits::default()))
                        .map(|_| "ACCEPTED"),
                );
            }
            errors.push(
                helper::prepared::prepare(a1_request(kind, &bad, IntakeLimits::default()))
                    .map(|_| "ACCEPTED"),
            );
            let observation = json!({"kind":kind,"member":member,"central_minus_actual":delta,"input_sha256":hex(&digest(&bad)),"results":errors});
            println!("A1_SIZE_CHECK {observation}");
            if errors
                .iter()
                .any(|r| !matches!(r,Err(e) if e==A1_SIZE_ERROR))
            {
                failures.push(observation);
            }
        }
    }
    failures
}
#[test]
fn a1_docx_member_sizes_required_optional_both_schemas() {
    let mut failures = a1_sizes(
        "docx",
        &a1_boundary(),
        &["word/document.xml", "word/footnotes.xml"],
    );
    failures.extend(a1_sizes(
        "docx",
        &repeated_figures(),
        &["word/media/same.png"],
    ));
    assert!(
        failures.is_empty(),
        "size-specific refusals missing: {failures:#?}"
    );
}
#[test]
fn a1_valid_descriptors_empty_files_and_directories() {
    for (kind, bytes, main) in [
        ("docx", a1_boundary(), "word/document.xml"),
        (
            "pptx",
            include_bytes!("fixtures/quay.pptx").to_vec(),
            "ppt/presentation.xml",
        ),
        (
            "xlsx",
            include_bytes!("fixtures/forecast.xlsx").to_vec(),
            "xl/workbook.xml",
        ),
    ] {
        let dirs = empty_members(&bytes);
        let (count, expanded) = a1_stats(&dirs);
        assert_eq!(count, a1_stats(&bytes).0 + 3);
        assert_eq!(expanded, a1_stats(&bytes).1);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&dirs)).unwrap();
        let dir = (0..archive.len())
            .find_map(|i| {
                let e = archive.by_index_raw(i).unwrap();
                (e.name() == "opaque/deflated/").then_some((
                    e.size(),
                    e.compressed_size(),
                    e.is_dir(),
                    e.header_start() as usize,
                ))
            })
            .unwrap();
        assert_eq!((dir.0, dir.2), (0, true));
        assert!(dir.1 > 0, "empty deflate stream still has compressed bytes");
        assert_ne!(dirs[dir.3 + 6] & 8, 0);
        assert_eq!(&dirs[dir.3 + 14..dir.3 + 26], &[0; 12]);
        for valid in [
            bytes.clone(),
            descriptor(&bytes, main),
            stored(&bytes, true),
            dirs,
        ] {
            a1_valid(kind, &bytes, &valid);
        }
    }
}
#[test]
fn a1_shared_pptx_and_xlsx_required_and_unread_optional_sizes() {
    let mut failures = a1_sizes(
        "pptx",
        include_bytes!("fixtures/quay.pptx"),
        &["ppt/presentation.xml", "docProps/thumbnail.jpeg"],
    );
    failures.extend(a1_sizes(
        "xlsx",
        include_bytes!("fixtures/forecast.xlsx"),
        &["xl/workbook.xml", "docProps/app.xml"],
    ));
    assert!(
        failures.is_empty(),
        "shared size-specific refusals missing: {failures:#?}"
    );
}
#[test]
fn a1_nested_eml_refusals_preserve_global_actual_charges() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let valid = package(&paragraph("Attachment boundary."), "", &[]);
    let bad = size_lie(&valid, "word/document.xml", 1, false);
    let mut inner = "MIME-Version: 1.0\r\nSubject: Inner\r\nContent-Type: multipart/mixed; boundary=i\r\n\r\n--i\r\nContent-Type: text/plain\r\n\r\nInner body.\r\n".to_owned();
    for (name, bytes) in [
        ("bad1.docx", &bad),
        ("bad2.docx", &bad),
        ("valid.docx", &valid),
    ] {
        inner.push_str(&format!("--i\r\nContent-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document\r\nContent-Disposition: attachment; filename={name}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",STANDARD.encode(bytes)));
    }
    inner.push_str("--i--\r\n");
    let outer = format!("MIME-Version: 1.0\r\nSubject: Outer\r\nContent-Type: multipart/mixed; boundary=o\r\n\r\n--o\r\nContent-Type: text/plain\r\n\r\nOuter body.\r\n--o\r\nContent-Type: message/rfc822\r\nContent-Disposition: attachment; filename=inner.eml\r\n\r\n{inner}\r\n--o--\r\n").into_bytes();
    let (parts, expanded) = a1_stats(&valid);
    for (max_parts, max_bytes, processed, refusal) in [
        (3 * parts, 3 * expanded, true, ""),
        (2 * parts, 3 * expanded, false, "package member budget"),
        (3 * parts, 2 * expanded, false, "inflated size exceeds"),
    ] {
        let limits = IntakeLimits {
            max_package_parts: max_parts,
            max_expanded_bytes: max_bytes,
            ..Default::default()
        };
        let e = helper::extract(a1_request("eml", &outer, limits.clone())).unwrap();
        let d = helper::prepared::prepare(a1_request("eml", &outer, limits)).unwrap();
        e.validate().unwrap();
        d.validate().unwrap();
        let charged = if processed { 3 } else { 2 };
        for usage in [e.consumed.as_ref().unwrap(), &d.consumed] {
            assert_eq!(
                (usage.package_parts, usage.expanded_bytes),
                (charged * parts, charged * expanded)
            );
        }
        assert_eq!(e.sources[0].bytes, outer);
        for name in ["bad1.docx", "bad2.docx"] {
            let s = e.sources.iter().find(|s| s.name == name).unwrap();
            assert_eq!(s.bytes, bad);
            assert_eq!(s.status, "unprocessed");
            assert!(s.text.is_empty());
            assert!(
                e.gaps
                    .iter()
                    .any(|g| g == &format!("{}: DOCX unprocessed: {A1_SIZE_ERROR}", s.id))
            );
            assert!(d.gaps.iter().any(|g|g.detail.contains(name)&&g.detail.contains("Unprocessed attachment")));
        }
        let s = e.sources.iter().find(|s| s.name == "valid.docx").unwrap();
        assert_eq!(s.bytes, valid);
        assert_eq!(s.status, if processed { "partial" } else { "unprocessed" });
        assert_eq!(
            e.markdown.matches("Attachment boundary.").count(),
            usize::from(processed)
        );
        if !processed {
            assert!(
                e.gaps
                    .iter()
                    .any(|g| g.contains(&s.id) && g.contains(refusal))
            );
        }
        println!(
            "A1_EML_CHECK {}",
            json!({"input_sha256":hex(&digest(&outer)),"processed_valid":processed,"usage":e.consumed,"gaps":e.gaps})
        );
    }
}
