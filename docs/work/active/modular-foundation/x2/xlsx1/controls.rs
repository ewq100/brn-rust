//! Fixed source facts for two maintained Excel projections, plus limited Office union controls.
use serde_json::json;
use std::{
    fs,
    io::{Cursor, Write},
    path::PathBuf,
    sync::OnceLock,
};
use xberg::types::{document_structure::NodeContent, extraction::ExtractedDocument};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, SecurityLimits};
const ROOT: &str = "/private/tmp/brn-x2-xlsx-references1-20261018";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const CLASSIC: [&str; 7] = [
    "#DIV/0!", "#N/A", "#NAME?", "#NULL!", "#NUM!", "#REF!", "#VALUE!",
];
fn capture_mime(name: &str, bytes: &[u8], mime: &str) -> Result<ExtractedDocument, String> {
    let root = PathBuf::from(ROOT);
    fs::write(
        root.join("fixtures").join(format!("{name}.xlsx1-input")),
        bytes,
    )
    .unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(2)
        .thread_stack_size(16 * 1024 * 1024)
        .build()
        .unwrap();
    let cfg = config();
    assert!(!cfg.needs_image_data());
    let result = rt.block_on(xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), mime, None),
        &cfg,
    ));
    drop(rt);
    fs::write(root.join("captures").join(format!("xlsx1-{name}.json")),
        serde_json::to_vec_pretty(&json!({"result":result.as_ref().ok(), "error":result.as_ref().err().map(|e|e.to_string())})).unwrap()).unwrap();
    let mut result = result.map_err(|e| e.to_string())?;
    if !result.errors.is_empty() || result.results.len() != 1 {
        return Err(format!(
            "envelope errors={:?}, results={}",
            result.errors,
            result.results.len()
        ));
    }
    Ok(result.results.remove(0))
}
fn capture(name: &str, bytes: &[u8]) -> ExtractedDocument {
    let mime = if name.starts_with("ns3-") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    } else {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    };
    capture_mime(name, bytes, mime).unwrap()
}
fn workbook(sheets: Vec<(&str, String)>) -> Vec<u8> {
    let s = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let opc = "http://schemas.openxmlformats.org/package/2006/relationships";
    let declarations: String = sheets
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            format!(
                "<sheet name=\"{name}\" sheetId=\"{}\" r:id=\"s{}\"/>",
                i + 1,
                i + 1
            )
        })
        .collect();
    let relationships: String = sheets.iter().enumerate().map(|(i,_)|format!("<Relationship Id=\"s{}\" Type=\"{r}/worksheet\" Target=\"worksheets/sheet{}.xml\"/>",i+1,i+1)).collect();
    let mut parts = vec![
        ("[Content_Types].xml".to_string(), "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>".to_string()),
        ("_rels/.rels".into(),format!("<Relationships xmlns=\"{opc}\"><Relationship Id=\"office\" Type=\"{r}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")),
        ("xl/workbook.xml".into(),format!("<workbook xmlns=\"{s}\" xmlns:r=\"{r}\"><sheets>{declarations}</sheets></workbook>")),
        ("xl/_rels/workbook.xml.rels".into(),format!("<Relationships xmlns=\"{opc}\">{relationships}</Relationships>")),
    ];
    for (i, (_, data)) in sheets.into_iter().enumerate() {
        parts.push((
            format!("xl/worksheets/sheet{}.xml", i + 1),
            format!("<worksheet xmlns=\"{s}\"><sheetData>{data}</sheetData></worksheet>"),
        ));
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in parts {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(data.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn formula_document() -> &'static ExtractedDocument {
    static DOC: OnceLock<ExtractedDocument> = OnceLock::new();
    DOC.get_or_init(|| {
        let cap: String = (12..=212).map(|row|format!("<row r=\"{row}\"><c r=\"AA{row}\"><f>1+1</f><v>2</v></c></row>")).collect();
        let bytes = workbook(vec![
            ("Offsets","<row r=\"12\"><c r=\"AA12\"><f>1+2</f><v>3</v></c><c r=\"AC12\"><f>SUM(AA12:AB12)</f><v>3</v></c></row><row r=\"14\"><c r=\"AB14\"><f>4+5</f><v>9</v></c></row>".into()),
            ("NoFormula","<row r=\"3\"><c r=\"C3\" t=\"inlineStr\"><is><t>Unchanged Ω</t></is></c></row>".into()),
            ("Empty",String::new()),
            ("A1","<row r=\"1\"><c r=\"A1\"><f>2*3</f><v>6</v></c></row>".into()),
            ("Last","<row r=\"1048576\"><c r=\"XFD1048576\"><f>1</f><v>1</v></c></row>".into()),
            ("Capped",cap),
        ]);
        capture_mime("formulas",&bytes,XLSX).unwrap()
    })
}
#[test]
fn xlsx_existing_non_a1_formulas() {
    let d = capture_mime(
        "existing-facts",
        include_bytes!("../../fixtures/facts.input"),
        XLSX,
    )
    .unwrap();
    assert_eq!(
        d.metadata.additional.get("formulas_Facts"),
        Some(&json!("G3=SUM(A1:A2); H3=1+2"))
    );
}
#[test]
fn xlsx_sparse_beyond_z_formula_order() {
    assert_eq!(
        formula_document()
            .metadata
            .additional
            .get("formulas_Offsets"),
        Some(&json!("AA12=1+2; AC12=SUM(AA12:AB12); AB14=4+5"))
    );
}
#[test]
fn xlsx_formula_empty_and_a1_controls() {
    let d = formula_document();
    assert!(!d.metadata.additional.contains_key("formulas_NoFormula"));
    assert!(!d.metadata.additional.contains_key("formulas_Empty"));
    assert_eq!(
        d.metadata.additional.get("formulas_A1"),
        Some(&json!("A1=2*3"))
    );
    assert_eq!(d.pages.as_ref().unwrap()[2].is_blank, Some(true));
}
#[test]
fn xlsx_formula_format_edge() {
    assert_eq!(
        formula_document().metadata.additional.get("formulas_Last"),
        Some(&json!("XFD1048576=1"))
    );
}
#[test]
fn xlsx_formula_metadata_cap_unchanged() {
    let expected = (12..=211)
        .map(|row| format!("AA{row}=1+1"))
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(
        formula_document()
            .metadata
            .additional
            .get("formulas_Capped"),
        Some(&json!(expected)),
        "first200 in source row order; source AA212 omitted by unchanged cap"
    );
}
#[test]
fn xlsx_seven_classic_error_spellings() {
    let cells: String = CLASSIC
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "<c r=\"{}1\" t=\"e\"><v>{e}</v></c>",
                char::from(b'A' + i as u8)
            )
        })
        .collect();
    let bytes = workbook(vec![("Errors", format!("<row r=\"1\">{cells}</row>"))]);
    let d = capture_mime("classic-errors", &bytes, XLSX).unwrap();
    assert_eq!(d.tables.len(), 1);
    assert_eq!(d.tables[0].cells, vec![CLASSIC.to_vec()]);
    let grids: Vec<_> = d
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
    assert_eq!(grids.len(), 1);
    assert_eq!((grids[0].rows, grids[0].cols), (1, 7));
    assert_eq!(
        grids[0]
            .cells
            .iter()
            .map(|c| (c.row, c.col, c.row_span, c.col_span, c.content.as_str()))
            .collect::<Vec<_>>(),
        CLASSIC
            .iter()
            .enumerate()
            .map(|(i, s)| (0, i as u32, 1, 1, *s))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        d.pages.as_ref().unwrap()[0].tables[0].cells,
        d.tables[0].cells
    );
}
#[test]
fn xlsx_getting_data_source_spelling_requirement() {
    // A source requirement, not a claim that pinned Calamine supports this raw lexeme.
    // Its CellErrorType::Display is #DATA!; XLSX FromStr separately accepts only seven classics.
    let bytes = workbook(vec![(
        "GettingData",
        "<row r=\"1\"><c r=\"A1\" t=\"e\"><v>#GETTING_DATA</v></c></row>".into(),
    )]);
    let d = capture_mime("getting-data", &bytes, XLSX).unwrap();
    assert_eq!(
        d.tables.iter().map(|t| &t.cells).collect::<Vec<_>>(),
        vec![&vec![vec!["#GETTING_DATA".to_string()]]],
        "known raw-error limitation remains open, not covered by seven-classic Display proof"
    );
}

fn config() -> ExtractionConfig {
    let e = 16 * 1024 * 1024;
    ExtractionConfig {
        use_cache: false,
        cache_namespace: None,
        enable_quality_processing: false,
        extraction_timeout_secs: None,
        max_concurrent_extractions: Some(1),
        content_filter: None,
        output_format: OutputFormat::Markdown,
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
        pages: Some(xberg::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        include_document_structure: true,
        max_archive_depth: 0,
        max_embedded_file_bytes: Some(e as u64),
        security_limits: Some(SecurityLimits {
            max_archive_size: e,
            max_compression_ratio: e,
            max_files_in_archive: 512,
            max_nesting_depth: 128,
            max_entity_length: e,
            max_content_size: e,
            max_iterations: e + 512,
            max_xml_depth: 128,
            max_table_cells: e,
            max_pages: Some(512),
        }),
        url: xberg::UrlExtractionConfig {
            allow_local_file_inputs: false,
            allow_file_uris: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn office_docx_namespace_control() {
    // Full wording/order from the frozen hand-authored inventory. 15 source p = 6 body + 9 cell p;
    // table/figure DTOs carry the visible projection; no invented 15-paragraph public locator.
    let inv: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/prefix-variation.inventory.json")).unwrap();
    let d = capture(
        "ns3-real-prefix",
        include_bytes!("../fixtures/prefix-variation.docx"),
    );
    let nodes = &d.document.as_ref().unwrap().nodes;
    let substantive: Vec<_> = nodes
        .iter()
        .filter_map(|n| match &n.content {
            NodeContent::Group { .. } => None,
            NodeContent::Heading { level, text } => {
                Some(json!(["heading", n.content_layer, level, text]))
            }
            NodeContent::Paragraph { text } => Some(json!(["paragraph", n.content_layer, text])),
            NodeContent::Table { grid } => Some(json!([
                "table",
                n.content_layer,
                grid.rows,
                grid.cols,
                grid.cells
                    .iter()
                    .map(|c| json!([c.row, c.col, c.row_span, c.col_span, c.content]))
                    .collect::<Vec<_>>()
            ])),
            NodeContent::Image {
                description,
                image_index,
                ..
            } => Some(json!([
                "figure",
                n.content_layer,
                image_index,
                description,
                n.attributes.as_ref().unwrap().get("image_uri")
            ])),
            _ => panic!("unrepresented substantive public node"),
        })
        .collect();
    assert_eq!(substantive,json!([
        ["heading","body",1,"Harbor water-saving pilot"],
        ["paragraph","body","Decision: extend the pilot to Pier B on 15 October 2026. Owner: Mira. The chart is evidence, not decoration."],
        ["table","body",3,3,[[0,0,1,1,"Site"],[0,1,1,1,"Before"],[0,2,1,1,"After"],[1,0,1,1,"Pier A"],[1,1,1,1,"120 L/day"],[1,2,1,1,"72 L/day"],[2,0,1,1,"Pier B"],[2,1,1,1,"Pending"],[2,2,1,1,"Pending"]]],
        ["paragraph","body","Water use before and after"],
        ["figure","body",0,"120 to 72 litres/day, a 40% reduction","media/image1.png"],
        ["paragraph","body","Same chart for operations handoff"],
        ["figure","body",1,"120 to 72 litres/day, a 40% reduction","media/image1.png"],
        ["paragraph","header","Internal synthetic briefing — review date 12 October 2026"],
        ["paragraph","footer","Footer constraint: budget capped at EUR 4,000."]
    ]).as_array().unwrap().clone());
    assert_eq!(inv["tables"][0]["cells"].as_array().unwrap().len(), 9);
    assert_eq!(inv["figures"].as_array().unwrap().len(), 2);
    let canonical = capture("ns3-real-harbor", include_bytes!("../fixtures/harbor.docx"));
    assert_eq!(
        serde_json::to_value(&d).unwrap(),
        serde_json::to_value(canonical).unwrap(),
        "no public exceptions are pre-authorized"
    );
}

fn hidden(d: &ExtractedDocument) -> Vec<(u32, Option<bool>)> {
    d.metadata
        .pages
        .as_ref()
        .unwrap()
        .pages
        .as_ref()
        .unwrap()
        .iter()
        .map(|p| (p.number, p.hidden))
        .collect()
}
#[test]
fn office_pptx_quay_order_control() {
    let d = capture("quay-order", include_bytes!("../fixtures/quay.pptx"));
    let p = d.pages.as_ref().unwrap();
    assert_eq!(p.len(), 2);
    assert!(
        p[0].content.contains("Inspect pier A"),
        "first declared source slide9 must be first: {p:?}"
    );
    assert!(!p[1].content.contains("Inspect pier A"));
    assert_eq!(hidden(&d), vec![(1, Some(true)), (2, Some(false))]);
    let rows: Vec<Vec<&str>> = p[0]
        .content
        .lines()
        .filter(|l| l.starts_with('|') && !l.contains("---"))
        .map(|l| l.trim_matches('|').split('|').map(str::trim).collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec!["Pier", "Owner", "State"],
            vec!["A", "Leena", "Pending"]
        ]
    );
    assert!(p[0].content.contains("14 October"));
    let nodes: Vec<_> = d
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|n| json!({"page":n.page,"content":n.content}))
        .collect();
    let heading = |page| json!({"page":page,"content":{"node_type":"heading","level":2,"text":"Dock review is pending."}});
    let group = |page| json!({"page":page,"content":{"node_type":"group","heading_level":2,"heading_text":"Dock review is pending."}});
    let paragraph =
        |page, text: &str| json!({"page":page,"content":{"node_type":"paragraph","text":text}});
    let expected_cells:Vec<_>=[["Pier","Owner","State"],["A","Leena","Pending"]].into_iter().enumerate().flat_map(|(row,cells)|cells.into_iter().enumerate().map(move |(col,text)|json!({"content":text,"row":row,"col":col,"row_span":1,"col_span":1,"is_header":row==0}))).collect();
    assert_eq!(
        nodes,
        vec![
            group(1),
            heading(1),
            json!({"page":1,"content":{"node_type":"table","grid":{"rows":2,"cols":3,"cells":expected_cells}}}),
            paragraph(1, "Inspect pier A"),
            paragraph(1, "before cargo arrival."),
            paragraph(1, "Display date: 14 October"),
            paragraph(
                1,
                "![Authored pier capacity illustration](../media/image1.png)"
            ),
            group(2),
            heading(2),
            paragraph(
                2,
                "![Authored pier capacity illustration](../media/image1.png)"
            ),
            paragraph(2, "![image.jpg](../media/image2.jpg)")
        ]
    );

    // Three direct picture occurrences are retained in their source pages; image bytes stay disabled.
    assert_eq!(
        p.iter()
            .map(|p| p.content.matches("![").count())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn office_pptx_quay_notes_control() {
    let d = capture("quay-notes", include_bytes!("../fixtures/quay.pptx"));
    assert_eq!(
        d.pages
            .as_ref()
            .unwrap()
            .iter()
            .map(|p| p.speaker_notes.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("Notes only: Leena must confirm the crane by 14 October."),
            Some("Do not treat a berth count as arrival authorization.")
        ]
    );
}
