#![cfg(feature = "office-differential")]
#[path = "support/office.rs"]
mod office;
use brn_intake::{helper, *};
use office::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use xberg::types::{
    document_structure::{ContentLayer, NodeContent},
    extraction::ExtractedDocument,
};

// Candidate allowances are captured before current helper admission charges this package.
// E+P iteration, XML acceptance and repeated-pass equivalence remain hypotheses.
fn candidate(
    limits: &IntakeLimits,
    prior_bytes: usize,
    prior_parts: usize,
) -> xberg::SecurityLimits {
    let e = limits.max_expanded_bytes.checked_sub(prior_bytes).unwrap();
    let p = limits.max_package_parts.checked_sub(prior_parts).unwrap();
    xberg::SecurityLimits {
        max_archive_size: e,
        max_compression_ratio: e,
        max_files_in_archive: p,
        max_nesting_depth: 128,
        max_entity_length: e,
        max_content_size: e,
        max_iterations: e.checked_add(p).unwrap(),
        max_xml_depth: 128,
        max_table_cells: e,
        max_pages: Some(p),
    }
}
fn config(limits: xberg::SecurityLimits) -> xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        use_cache: false,
        cache_namespace: None,
        enable_quality_processing: false,
        extraction_timeout_secs: None,
        max_concurrent_extractions: Some(1),
        content_filter: None,
        output_format: xberg::OutputFormat::Markdown,
        concurrency: Some(xberg::ConcurrencyConfig {
            max_threads: Some(1),
            max_concurrent_ocr: Some(1),
        }),
        postprocessor: Some(xberg::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        disable_ocr: true,
        ocr: None,
        force_ocr: false,
        force_ocr_pages: None,
        images: None,
        captioning: None,
        qr_codes: Some(false),
        chunking: None,
        language_detection: None,
        token_reduction: None,
        pages: None,
        include_document_structure: true,
        max_archive_depth: 0,
        security_limits: Some(limits),
        url: xberg::UrlExtractionConfig {
            allow_local_file_inputs: false,
            allow_file_uris: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn run(bytes: &[u8], limits: xberg::SecurityLimits) -> Result<ExtractedDocument, String> {
    println!(
        "O1_INPUT {}",
        json!({"api":"xberg::extract","mime":DOCX_MIME,"sha256":hex(&digest(bytes)),"bytes":bytes.len(),"limits":&limits})
    );
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let cfg = config(limits);
    assert!(!cfg.needs_image_data());
    assert_eq!(cfg.max_archive_depth, 0);
    let mut future = pin!(xberg::extract(
        xberg::ExtractInput::from_bytes(bytes.to_vec(), DOCX_MIME, None),
        &cfg
    ));
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    for _ in 0..32 {
        match future.as_mut().poll(&mut cx) {
            Poll::Pending => std::thread::park_timeout(Duration::from_millis(50)),
            Poll::Ready(result) => {
                let mut result = result.map_err(|e| e.to_string())?;
                if !result.errors.is_empty() || result.results.len() != 1 {
                    return Err(format!(
                        "Xberg envelope {:?}; documents={}",
                        result.errors,
                        result.results.len()
                    ));
                }
                let doc = result.results.remove(0);
                assert_eq!(doc.mime_type, DOCX_MIME);
                assert!(doc.children.as_ref().is_none_or(Vec::is_empty));
                assert!(
                    doc.images
                        .as_ref()
                        .is_none_or(|images| images.iter().all(|i| i.data.is_empty()))
                );
                return Ok(doc);
            }
        }
    }
    panic!("test executor ceiling reached: infrastructure failure, never an admission refusal")
}
fn extract(bytes: &[u8]) -> ExtractedDocument {
    run(bytes, candidate(&IntakeLimits::default(), 0, 0)).unwrap()
}
fn atoms(doc: &ExtractedDocument, body_only: bool) -> Vec<Atom> {
    let mut out = Vec::new();
    for node in &doc.document.as_ref().expect("structure requested").nodes {
        if body_only && node.content_layer != ContentLayer::Body {
            continue;
        }
        match &node.content {
            NodeContent::Title { text }
            | NodeContent::Heading { text, .. }
            | NodeContent::Paragraph { text }
            | NodeContent::ListItem { text }
            | NodeContent::Footnote { text }
            | NodeContent::Comment { text } => out.push(Atom::Text(text.clone())),
            NodeContent::Table { grid } => {
                for c in &grid.cells {
                    out.push(Atom::Cell {
                        address: format!("R{}C{}", c.row + 1, c.col + 1),
                        text: c.content.clone(),
                        row_span: c.row_span,
                        column_span: c.col_span,
                    });
                }
            }
            _ => (),
        }
    }
    out
}
fn expected_body(case: &Case) -> Vec<Atom> {
    let mut oracle = case.oracle.clone();
    oracle["stories"] = json!([oracle["stories"][0].clone()]);
    expected(
        &Case {
            name: case.name,
            bytes: vec![],
            oracle,
        },
        &BTreeMap::new(),
        false,
        false,
    )
}
#[derive(serde::Serialize)]
enum WarningPolicy {
    MissingReferenceGap,
    MissingCommentGap,
    SymbolLossGap,
    FieldLossGap,
    StoryLossGap,
    UnmappedPromotionGate,
}
fn warning_classes(doc: &ExtractedDocument) -> Vec<Value> {
    doc.processing_warnings.iter().map(|w| {
        let message=w.message.as_ref();
        let policy=match w.source.as_ref() {
            "relationships" if message.contains(" cross-reference target(s) could not be resolved and were dropped from the document structure: ") => WarningPolicy::MissingReferenceGap,
            "docx" if message.starts_with("Comment reference id ") && message.ends_with(" has no matching entry in comments.xml; the comment text was dropped") => WarningPolicy::MissingCommentGap,
            "docx" if message.starts_with("Could not map w:sym character code ") && message.ends_with("; inserted a placeholder") => WarningPolicy::SymbolLossGap,
            "docx" if message.starts_with("A DOCX field instruction (w:fldChar begin) was never closed with a matching separate/end; ") => WarningPolicy::FieldLossGap,
            "docx" if (message.starts_with("Header relationship target '") || message.starts_with("Footer relationship target '")) && message.ends_with(" was dropped") => WarningPolicy::StoryLossGap,
            _ => WarningPolicy::UnmappedPromotionGate,
        };
        json!({"source":w.source,"message":w.message,"candidate_current_policy":policy,"location":"only supplied source/message; no guessed source range"})
    }).collect()
}
fn witness(doc: &ExtractedDocument) -> Value {
    json!({"document":doc.document,"revisions":doc.revisions,"warnings":doc.processing_warnings,"warning_classes":warning_classes(doc),"images":doc.images,"metadata":doc.metadata})
}
#[test]
fn office_xberg_docx_full_independent_inventory_and_prefix() {
    for case in cases() {
        let doc = extract(&case.bytes);
        if case.name == "prefix-variation" {
            assert!(
                atoms(&doc, true).is_empty(),
                "observed prefix body loss, never expected source absence"
            );
            let stories: Vec<_> = doc
                .document
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .filter_map(|n| match &n.content {
                    NodeContent::Paragraph { text } => Some((n.content_layer, text.as_str())),
                    _ => None,
                })
                .collect();
            assert_eq!(
                stories,
                vec![
                    (
                        ContentLayer::Header,
                        "Internal synthetic briefing — review date 12 October 2026"
                    ),
                    (
                        ContentLayer::Footer,
                        "Footer constraint: budget capped at EUR 4,000."
                    )
                ]
            );
        }
        if case.name == "footnotes" {
            let grid = doc
                .document
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find_map(|n| {
                    if let NodeContent::Table { grid } = &n.content {
                        Some(grid)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(
                grid.cells.len(),
                12,
                "observed expanded grid, independent source has ten origin cells"
            );
            assert_eq!(
                (grid.cells[0].content.as_str(), grid.cells[0].col_span),
                ("Acceptance checks", 1)
            );
            assert!(grid.cells[1].content.is_empty() && grid.cells[2].content.is_empty());
        }
        compare(
            if case.name == "prefix-variation" {
                "S7"
            } else {
                "S6"
            },
            &format!("{}-body", case.name),
            json!(expected_body(&case)),
            json!(atoms(&doc, true)),
        );
        record(
            "S6",
            &format!("{}-all-stories", case.name),
            json!({"atoms":expected(&case,&BTreeMap::new(),false,false),"stories":case.oracle["stories"],"conditional_gaps":case.oracle["conditional_gaps"]}),
            witness(&doc),
        );
        // Image indices and paths are public evidence; no joins recovered from Markdown.
        let wanted:Vec<_>=array(&case.oracle,"figures").iter().enumerate().map(|(i,f)|json!({"image_index":i,"relationship_target":f["relationship_target"],"source_rid":f["relationship_id"],"content_range_required":true})).collect();
        let actual:Vec<_>=doc.document.as_ref().unwrap().nodes.iter().filter_map(|n|match &n.content {
            NodeContent::Image{image_index,description,src}=>Some(json!({"image_index":image_index,"description":description,"src":src,"attributes":n.attributes,"content_range":null})),_=>None
        }).collect();
        record(
            "S1",
            &format!("{}-images", case.name),
            json!(wanted),
            json!(actual),
        );
        if !array(&case.oracle, "tables").is_empty() {
            let wanted: Vec<_> = expected(&case, &BTreeMap::new(), false, false)
                .into_iter()
                .filter(|x| matches!(x, Atom::Cell { .. }))
                .collect();
            let actual: Vec<_> = atoms(&doc, false)
                .into_iter()
                .filter(|x| matches!(x, Atom::Cell { .. }))
                .collect();
            compare(
                "S3",
                &format!("{}-cells", case.name),
                json!(wanted),
                json!(actual),
            );
        }
        if !array(&case.oracle, "notes").is_empty() {
            record(
                "S4",
                "footnotes-typed-associations",
                json!({"notes":case.oracle["notes"],"fields":case.oracle["fields"],"bookmarks":case.oracle["bookmarks"]}),
                witness(&doc),
            );
        }
    }
}
#[test]
fn office_xberg_docx_authored_revisions_comments_notes_and_warnings() {
    let bytes = revisions_comments();
    let doc = extract(&bytes);
    assert_eq!(
        atoms(&doc, true),
        vec![
            Atom::Text("Keep accepted.".into()),
            Atom::Text("Reviewed.[cmt:7]".into())
        ]
    );
    let structure = doc.document.as_ref().unwrap();
    let comments: Vec<_> = structure
        .nodes
        .iter()
        .filter_map(|n| {
            if let NodeContent::Comment { text } = &n.content {
                assert!(
                    n.attributes.is_none(),
                    "observed comment context is not emitted"
                );
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(comments, ["Reviewer note.", "Reply note."]);
    let revisions = doc.revisions.as_ref().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].author.as_deref(), Some("Iris"));
    assert_eq!(
        serde_json::to_value(&revisions[0]).unwrap()["delta"]["content"],
        json!([{"kind":"added","text":"accepted"}])
    );
    assert_eq!(
        serde_json::to_value(&revisions[1]).unwrap()["delta"]["content"],
        json!([{"kind":"removed","text":"removed"}])
    );
    record(
        "S2",
        "xberg-comments",
        json!({"comments":[{"id":"7","author":"Iris","date":"2026-10-18T10:00:00Z","range_text":"Reviewed."},{"id":"8","author":"Mara","date":"2026-10-18T11:00:00Z","parent":"7","resolved":true}],"accepted_body":["Keep accepted.","Reviewed."]}),
        witness(&doc),
    );
    record(
        "S6",
        "xberg-revisions",
        json!({"accepted":"Keep accepted.","inserted":{"text":"accepted","author":"Iris","date":"2026-10-18T10:00:00Z"},"deleted":"removed"}),
        json!({"body":atoms(&doc,true),"revisions":doc.revisions}),
    );
    let notes = extract(&note_collisions());
    assert_eq!(
        atoms(&notes, true),
        vec![
            Atom::Text("Literal [^1] and [cmt:7]. [^1][^1]".into()),
            Atom::Text("1".into()),
            Atom::Cell {
                address: "R1C1".into(),
                text: "Cell[^2]".into(),
                row_span: 1,
                column_span: 1
            }
        ]
    );
    let ns = notes.document.as_ref().unwrap();
    let definitions: Vec<_> = ns
        .nodes
        .iter()
        .filter_map(|n| {
            if let NodeContent::Footnote { text } = &n.content {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(definitions, ["Footnote one.", "Cell note.", "Endnote one."]);
    let associations = serde_json::to_value(&ns.relationships).unwrap();
    assert_eq!(
        associations,
        json!([{"source":0,"target":3,"kind":"footnote_reference"},{"source":0,"target":5,"kind":"footnote_reference"}])
    );
    record(
        "S4",
        "xberg-note-collisions",
        json!({"literal":"Literal [^1] and [cmt:7]. ","typed_refs":["footnote:1","endnote:1","table/footnote:2"],"definitions":["Footnote one.","Cell note.","Endnote one."],"field":{"kind":"NOTEREF","bookmark":"rule","cached":"1"}}),
        witness(&notes),
    );
    let escaped = package(
        &paragraph("Escape target."),
        &relationship("h", "header", "../../outside.xml"),
        &[],
    );
    record(
        "S6",
        "xberg-escaping-relationship",
        json!({"target":"../../outside.xml","policy":"no filesystem resolution; measure existing gap/refusal before promotion"}),
        json!({"current":helper::extract(request(&escaped,IntakeLimits::default())).map(|e|json!({"wording":markdown_wording(&e.markdown),"gaps":e.gaps})),"xberg":run(&escaped,candidate(&IntakeLimits::default(),0,0)).map(|d|witness(&d))}),
    );
    let warning_bytes = warning_document();
    let current_warning =
        helper::extract(request(&warning_bytes, IntakeLimits::default())).unwrap();
    let current_prepared =
        helper::prepared::prepare(request(&warning_bytes, IntakeLimits::default())).unwrap();
    current_warning.validate().unwrap();
    current_prepared.validate().unwrap();
    assert_eq!(
        markdown_wording(&current_warning.markdown),
        ["Bookmark label."]
    );
    println!(
        "O1_CURRENT_WARNING_POLICY {}",
        json!({"schema1_gaps":current_warning.gaps,"schema2_blocks":current_prepared.blocks,"schema2_gaps":current_prepared.gaps})
    );
    let warnings = extract(&warning_bytes);
    let classes = warning_classes(&warnings);
    assert_eq!(
        classes
            .iter()
            .map(|c| c["candidate_current_policy"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["MissingCommentGap", "MissingReferenceGap"]
    );
    record(
        "S6",
        "xberg-warning-policy",
        json!({"visible_text":"Bookmark label.","missing_bookmark":"located gap, ordinary partial input","missing_comment":"located missing-reference gap","unknown_warning":"promotion gate, no generic production refusal"}),
        json!({"body":atoms(&warnings,true),"warnings":warnings.processing_warnings,"classes":warning_classes(&warnings)}),
    );
}
#[test]
fn office_xberg_docx_orphan_multitable_spans_and_repeated_figures() {
    let orphan = extract(&empty_orphan());
    assert_eq!(
        atoms(&orphan, false),
        vec![
            Atom::Text("Visible.".into()),
            Atom::Text("Bound header.".into()),
            Atom::Text("Orphan header.".into())
        ]
    );
    record(
        "S6",
        "xberg-empty-orphan",
        json!({"body":["","Visible."],"related_header":"Bound header.","orphan_header":"Orphan header."}),
        witness(&orphan),
    );
    let grids = extract(&grids());
    let tables: Vec<_> = grids
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|n| {
            if let NodeContent::Table { grid } = &n.content {
                Some(grid)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(tables.len(), 3);
    assert!(
        tables
            .iter()
            .flat_map(|t| &t.cells)
            .all(|c| c.row_span == 1 && c.col_span == 1),
        "observed span loss, source spans remain required"
    );
    assert_eq!(
        tables[0].cells[4].content, "Outer Inner",
        "observed nested-table flattening"
    );
    assert_eq!(tables[2].cells[0].content, "Vertical");
    assert!(tables[2].cells[2].content.is_empty());
    record("S3", "xberg-grids", grid_source(), witness(&grids));
    let images = extract(&repeated_figures());
    let nodes: Vec<_> = images
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|n| match &n.content {
            NodeContent::Image { image_index, .. } => {
                Some(json!({"index":image_index,"attributes":n.attributes}))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        nodes.len(),
        2,
        "two independently authored drawing occurrences"
    );
    assert_ne!(nodes[0]["index"], nodes[1]["index"]);
    record(
        "S1",
        "xberg-repeated-images",
        json!({"source_docpr_ids":[31,32],"relationship":"image","target":"media/same.png","caption_order":["Same caption.","Same caption."],"content_ranges_required":2}),
        json!({"nodes":nodes,"body":atoms(&images,true),"images":images.images}),
    );
}
fn refusal(name: &str, bytes: &[u8], limits: xberg::SecurityLimits, cause: &str) {
    let error = run(bytes, limits).expect_err("intended bounded operation must refuse");
    record(
        "S6",
        name,
        json!({"required_error_class":cause}),
        json!({"error":error}),
    );
    assert!(
        error.to_ascii_lowercase().contains(cause),
        "wrong refusal for {name}: {error}"
    );
}
fn boundary_package() -> Vec<u8> {
    let notes = "<w:footnotes xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:footnote w:id=\"1\"><w:p><w:r><w:t>Optional note.</w:t></w:r></w:p></w:footnote></w:footnotes>";
    package(
        &paragraph("Boundary."),
        &relationship("fn", "footnotes", "footnotes.xml"),
        &[("word/footnotes.xml", notes)],
    )
}
#[test]
fn office_limits_pre_admission_allowances_and_actual_iteration_boundary() {
    let bytes = boundary_package();
    let parts = ooxml_opc::unzip_parts(&bytes).unwrap();
    let expanded: usize = parts.iter().map(|(_, b)| b.len()).sum();
    let limits = IntakeLimits {
        max_expanded_bytes: expanded,
        max_package_parts: parts.len(),
        ..Default::default()
    };
    let before = candidate(&limits, 0, 0);
    assert_eq!(before.max_archive_size, expanded);
    assert_eq!(before.max_files_in_archive, parts.len());
    let admitted = helper::extract(request(&bytes, limits.clone()))
        .unwrap()
        .consumed
        .unwrap();
    assert_eq!(admitted.expanded_bytes, expanded);
    assert_eq!(
        atoms(&run(&bytes, before.clone()).unwrap(), true),
        vec![Atom::Text("Boundary.".into())]
    );
    let wrongly_after = candidate(&limits, admitted.expanded_bytes, admitted.package_parts);
    assert_eq!(wrongly_after.max_archive_size, 0);
    assert!(run(&bytes, wrongly_after).is_err());
    // Finite binary search on a fixed synthetic source measures consumed budget steps.
    let mut lo = 0;
    let mut hi = before.max_iterations;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        let mut l = before.clone();
        l.max_iterations = mid;
        if run(&bytes, l).is_ok() {
            hi = mid
        } else {
            lo = mid
        }
    }
    let mut at = before.clone();
    at.max_iterations = hi;
    assert!(run(&bytes, at).is_ok());
    let mut below = before.clone();
    below.max_iterations = lo;
    assert!(run(&bytes, below).is_err());
    record(
        "S6",
        "iteration-budget-fixed-source",
        json!({"candidate":expanded+parts.len(),"hypothesis":"E+P, not heap proof"}),
        json!({"first_success":hi,"last_refused":lo,"expanded":expanded,"parts":parts.len()}),
    );
    let mut over = before.clone();
    over.max_files_in_archive = parts.len() - 1;
    assert!(run(&bytes, over).is_err());
    let mut over = before;
    over.max_archive_size = expanded - 1;
    assert!(run(&bytes, over).is_err());
}
#[test]
fn office_limits_central_actual_lies_zero_compressed_empty_and_descriptor() {
    let bytes = boundary_package();
    for valid in [
        stored(&bytes, true),
        descriptor(&bytes, "word/document.xml"),
    ] {
        let doc = extract(&valid);
        assert_eq!(atoms(&doc, true), vec![Atom::Text("Boundary.".into())]);
    }
    assert!(
        run(
            &size_lie(&bytes, "word/document.xml", 0, true),
            candidate(&IntakeLimits::default(), 0, 0)
        )
        .is_err()
    );
    let history: Value = serde_json::from_str(include_str!(
        "../../../docs/work/active/modular-foundation/x2/office1/admission1/historical-size-observations.json"
    )).unwrap();
    let mut observed = Vec::new();
    for (member, source) in [
        ("word/document.xml", bytes.clone()),
        ("word/footnotes.xml", bytes.clone()),
        ("word/media/same.png", repeated_figures()),
    ] {
        for delta in [-1, 1] {
            let malformed = size_lie(&source, member, delta, false);
            let current = helper::extract(request(&malformed, IntakeLimits::default()));
            let maintained = run(&malformed, candidate(&IntakeLimits::default(), 0, 0));
            assert_eq!(
                current.as_ref().unwrap_err(),
                "Office package member-size-mismatch"
            );
            let key = format!("xberg-size-{member}-{delta}");
            let xberg = json!(maintained.map(|x| witness(&x)));
            // Value equality compares complete content and array order; object key
            // ordering is immaterial. This is historical drift detection, not parity.
            assert_eq!(
                xberg, history["xberg_values"][&key],
                "bare backend changed: {key}"
            );
            let expected = json!({"member":member,"central_minus_actual":delta,"required":member=="word/document.xml","policy":"reject inconsistent members before future promotion, valid descriptor is separate"});
            let actual = json!({"current":current.map(|x|json!({"wording":markdown_wording(&x.markdown),"gaps":x.gaps})),"xberg":xberg});
            println!(
                "A1_PAIRED_SIZE {}",
                json!({"class":"S6","witness":key,"measurement":"EvidenceOnly","expected":expected,"actual":actual})
            );
            observed.push((key, expected, actual));
        }
    }
    // Capture every pair before checking the updated current-result pins.
    for (key, expected, actual) in observed {
        record("S6", &key, expected, actual);
    }
}
#[test]
fn office_limits_content_cells_depth_and_repeated_story_passes() {
    let bytes = package(
        &format!("{}{}", paragraph("One."), paragraph("Two.")),
        "",
        &[],
    );
    let limits = candidate(&IntakeLimits::default(), 0, 0);
    assert_eq!(
        atoms(&extract(&bytes), true),
        vec![Atom::Text("One.".into()), Atom::Text("Two.".into())]
    );
    let mut l = limits.clone();
    l.max_content_size = 7;
    refusal("content-cap", &bytes, l, "content");
    let mut l = limits.clone();
    l.max_entity_length = 3;
    refusal("entity-cap", &bytes, l, "entity");
    let table = "<w:tbl><w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>";
    let bytes = package(&table.repeat(2), "", &[]);
    assert_eq!(
        atoms(&extract(&bytes), true)
            .iter()
            .filter(|a| matches!(a, Atom::Cell { .. }))
            .count(),
        2
    );
    let mut l = limits.clone();
    l.max_table_cells = 1;
    refusal("aggregate-cell-cap", &bytes, l, "table");
    let body = format!(
        "{}{}{}",
        "<w:sdt><w:sdtContent>".repeat(130),
        paragraph("Deep"),
        "</w:sdtContent></w:sdt>".repeat(130)
    );
    refusal(
        "xml-depth-cap",
        &package(&body, "", &[]),
        limits.clone(),
        "nesting too deep",
    );
    let mut dtd = ooxml_opc::unzip_parts(&bytes).unwrap();
    let main = &mut dtd
        .iter_mut()
        .find(|(name, _)| name == "word/document.xml")
        .unwrap()
        .1;
    let mut with_dtd = b"<!DOCTYPE document [<!ENTITY value \"forbidden\">]>".to_vec();
    with_dtd.extend_from_slice(main);
    *main = with_dtd;
    let dtd_bytes = ooxml_opc::rezip_parts(&dtd).unwrap();
    let dtd_result = run(&dtd_bytes, limits.clone());
    record(
        "S6",
        "xberg-dtd-admission",
        json!({"required":"checked Xberg adapter must reject DTD before backend invocation","declaration":"internal entity, unused in body","bare_backend":"accepts with two ordered R1C1 Cell cells, each span 1x1","promotion":"OPEN; checked Xberg adapter does not exist"}),
        json!(dtd_result.as_ref().map(witness)),
    );
    let dtd_doc = dtd_result.expect("bare Xberg accepts this unused declaration");
    let expected_cell = Atom::Cell {
        address: "R1C1".into(),
        text: "Cell".into(),
        row_span: 1,
        column_span: 1,
    };
    assert_eq!(
        atoms(&dtd_doc, true),
        vec![expected_cell.clone(), expected_cell]
    );
    // ZIP admission does not validate XML. Both current public schemas subsequently
    // reach BetterOffice parse_xml's Event::DocType refusal on these same bytes.
    let schema1_error = helper::extract(request(&dtd_bytes, IntakeLimits::default())).unwrap_err();
    let schema2_error =
        helper::prepared::prepare(request(&dtd_bytes, IntakeLimits::default())).unwrap_err();
    let expected_error =
        "DOCX parse: unsafe XML in word/document.xml: DTD/entity declarations are forbidden";
    assert_eq!(schema1_error, expected_error);
    assert_eq!(schema2_error, expected_error);
    println!(
        "O1_DTD_BOUNDARIES {}",
        json!({"input_sha256":hex(&digest(&dtd_bytes)),"bare_cells":atoms(&dtd_doc,true),"schema1_error":schema1_error,"schema2_error":schema2_error})
    );
    // The future checked Xberg adapter must refuse before calling its backend.
    // No such adapter exists; these controls leave that promotion gate OPEN.
}
#[test]
fn office_limits_xml_event_and_repeated_story_passes() {
    let limits = candidate(&IntakeLimits::default(), 0, 0);
    let events = package(&format!("<w:p>{}</w:p>", "<w:r/>".repeat(200_001)), "", &[]);
    let current = helper::extract(request(&events, IntakeLimits::default()));
    assert!(current.is_err(), "existing 200,000 XML event policy");
    record(
        "S6",
        "xml-event-policy",
        json!({"empty_run_elements":200001,"current_event_limit":200000,"candidate":"E+P may admit more events; not equivalence proof"}),
        json!({"current":current,"xberg":run(&events,limits.clone()).map(|d|witness(&d))}),
    );
    let hdr = format!(
        "<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{}</w:hdr>",
        paragraph("Shared.").repeat(100)
    );
    let rels = (0..16)
        .map(|i| relationship(&format!("h{i}"), "header", "header1.xml"))
        .collect::<String>();
    let bytes = package(&paragraph("Body."), &rels, &[("word/header1.xml", &hdr)]);
    let parts = ooxml_opc::unzip_parts(&bytes).unwrap();
    let expanded: usize = parts.iter().map(|(_, b)| b.len()).sum();
    let l = candidate(
        &IntakeLimits {
            max_expanded_bytes: expanded,
            max_package_parts: parts.len(),
            ..Default::default()
        },
        0,
        0,
    );
    record(
        "S6",
        "repeated-header-pass-candidate",
        json!({"source_header_paragraphs":100,"relationships_to_same_target":16,"expanded":expanded,"candidate_iterations":l.max_iterations,"proof":"unproven acceptance/pass count, no automatic quota raising"}),
        json!(run(&bytes, l).map(|d| witness(&d))),
    );
}
