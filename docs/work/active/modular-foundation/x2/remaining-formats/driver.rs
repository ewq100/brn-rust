//! Source-authored comparison measurements. Current BRN is a comparator only.
use brn_intake::prepared::{BlockContent, PreparedCell, PreparedDocument};
use brn_intake::{Extraction, HelperRequest, IntakeLimits, digest, hex};
use brn_threads_intake::{
    ConversionError, ConversionRequest, ConversionResult, Converter, DocumentKind, ImportIntent,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Cursor, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{OnceLock, atomic::AtomicBool},
};
use xberg::types::{
    document_structure::NodeContent, extraction::ExtractedDocument, metadata::FormatMetadata,
};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, SecurityLimits};

const ROOT: &str = "/private/tmp/brn-x2-remaining-formats-20261018";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PLAIN: &str =
    "Please review the attached Harbor pilot. Extend to Pier B after inspection.\r\n";
const HTML: &str = "<html><body><p>Water use fell 40%.</p><img src=\"cid:water-evidence\"><p>See attached decision.</p></body></html>\r\n";
const NESTED: &str = "From: Child <child@example.test>\r\nTo: Parent <parent@example.test>\r\nSubject: Nested Ω\r\nMessage-ID: <child@example.test>\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\nNested body Ω\r\n";
#[derive(Clone)]
struct Fixture {
    name: &'static str,
    kind: DocumentKind,
    mime: &'static str,
    bytes: Vec<u8>,
}
struct Observation {
    fixture: Fixture,
    candidate: Result<ExtractedDocument, String>,
    current: Result<ConversionResult, ConversionError>,
    prepared: Result<PreparedDocument, ConversionError>,
    envelope: Option<Result<Extraction, String>>,
}
fn save(name: &str, value: &Value) {
    fs::write(
        PathBuf::from(ROOT).join("captures").join(name),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn workbook() -> Vec<u8> {
    let w = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let opc = "http://schemas.openxmlformats.org/package/2006/relationships";
    let parts = vec![
        ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>".into()),
        ("_rels/.rels", format!("<Relationships xmlns=\"{opc}\"><Relationship Id=\"office\" Type=\"{r}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")),
        ("xl/workbook.xml", format!("<workbook xmlns=\"{w}\" xmlns:r=\"{r}\"><sheets><sheet name=\"Facts\" sheetId=\"1\" r:id=\"s1\"/><sheet name=\"Hidden Ω\" sheetId=\"2\" state=\"hidden\" r:id=\"s2\"/><sheet name=\"Empty\" sheetId=\"3\" r:id=\"s3\"/></sheets></workbook>")),
        ("xl/_rels/workbook.xml.rels", format!("<Relationships xmlns=\"{opc}\"><Relationship Id=\"s1\" Type=\"{r}/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"s2\" Type=\"{r}/worksheet\" Target=\"worksheets/sheet2.xml\"/><Relationship Id=\"s3\" Type=\"{r}/worksheet\" Target=\"worksheets/sheet3.xml\"/><Relationship Id=\"styles\" Type=\"{r}/styles\" Target=\"styles.xml\"/></Relationships>")),
        ("xl/styles.xml", format!("<styleSheet xmlns=\"{w}\"><cellXfs count=\"2\"><xf numFmtId=\"0\"/><xf numFmtId=\"14\"/></cellXfs></styleSheet>")),
        ("xl/worksheets/sheet1.xml", format!("<worksheet xmlns=\"{w}\"><dimension ref=\"C3:H5\"/><sheetData><row r=\"3\"><c r=\"C3\" t=\"inlineStr\"><is><t>Café Ω</t></is></c><c r=\"D3\" s=\"1\"><v>45000</v></c><c r=\"E3\" t=\"b\"><v>1</v></c><c r=\"F3\" t=\"e\"><v>#DIV/0!</v></c><c r=\"G3\"><f>SUM(A1:A2)</f><v>99</v></c><c r=\"H3\"><f>1+2</f></c></row><row r=\"5\"><c r=\"C5\" t=\"inlineStr\"><is><t></t></is></c><c r=\"E5\"><v>9007199254740993</v></c></row></sheetData></worksheet>")),
        ("xl/worksheets/sheet2.xml", format!("<worksheet xmlns=\"{w}\"><sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Hidden retained</t></is></c></row></sheetData></worksheet>")),
        ("xl/worksheets/sheet3.xml", format!("<worksheet xmlns=\"{w}\"><sheetData/></worksheet>")),
    ];
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
fn email() -> Vec<u8> {
    format!("From: =?UTF-8?Q?Mira_=CE=A9?= <mira@example.test>\r\nTo: Team <team@example.test>\r\nSubject: =?UTF-8?Q?R=C3=A9sum=C3=A9_=CE=A9?=\r\nDate: Sat, 17 Oct 2026 12:00:00 +0300\r\nMessage-ID: <synthetic@example.test>\r\nX-Trace: first\r\nX-Trace: second\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=outer\r\n\r\n--outer\r\nContent-Type: multipart/alternative; boundary=alternative\r\n\r\n--alternative\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\nPlain Ω\r\n\r\nSecond line\r\n\r\n--alternative\r\nContent-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n<p>HTML Ω</p><p>Second line</p>\r\n\r\n--alternative--\r\n--outer\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=dup.bin\r\nContent-Transfer-Encoding: base64\r\n\r\nYWxwaGE=\r\n--outer\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=dup.bin\r\nContent-Transfer-Encoding: base64\r\n\r\nYmV0YQ==\r\n--outer\r\nContent-Type: message/rfc822\r\nContent-Disposition: attachment; filename=child.eml\r\n\r\n{NESTED}\r\n--outer--\r\n").into_bytes()
}
fn fixtures() -> Vec<Fixture> {
    let mut out = vec![
        Fixture {
            name: "forecast",
            kind: DocumentKind::Xlsx,
            mime: XLSX,
            bytes: include_bytes!("../../fixtures/forecast.xlsx").to_vec(),
        },
        Fixture {
            name: "facts",
            kind: DocumentKind::Xlsx,
            mime: XLSX,
            bytes: workbook(),
        },
        Fixture {
            name: "single",
            kind: DocumentKind::Eml,
            mime: "message/rfc822",
            bytes: include_bytes!("../../fixtures/single.eml").to_vec(),
        },
        Fixture {
            name: "plural",
            kind: DocumentKind::Eml,
            mime: "message/rfc822",
            bytes: include_bytes!("../../fixtures/plural.eml").to_vec(),
        },
        Fixture {
            name: "multipart",
            kind: DocumentKind::Eml,
            mime: "message/rfc822",
            bytes: email(),
        },
        Fixture {
            name: "html-control",
            kind: DocumentKind::Html,
            mime: "text/html",
            bytes: b"<p>Alpha text.</p><p>Beta text.</p>".to_vec(),
        },
    ];
    for (name, kind, text) in [
        ("text-crlf", DocumentKind::Text, " Alpha Ω\r\n\r\nBeta\r\n"),
        ("text-bom", DocumentKind::Text, "\u{feff}Café e\u{301} Ω\n"),
        (
            "text-markup",
            DocumentKind::Text,
            "# Literal heading\n\n[link](https://example.test) <b>literal</b>\n",
        ),
        ("text-no-final", DocumentKind::Text, "No final newline Ω"),
        ("text-blanks", DocumentKind::Text, "\n\nBody\n\n\n"),
        (
            "md-crlf",
            DocumentKind::Markdown,
            "# Title\r\n\r\nA [link](https://example.test).\r\n",
        ),
        (
            "md-fence",
            DocumentKind::Markdown,
            "---\ntitle: Ω\n---\n\n```rust\nlet x = \"<tag>\";\n```\n\n![alt](image.png)\n\n<b>Literal HTML</b>\n",
        ),
        (
            "md-bom",
            DocumentKind::Markdown,
            "\u{feff}# Café Ω\n\nTail\n",
        ),
        (
            "md-blanks",
            DocumentKind::Markdown,
            "\n\n# Title\n\n\nTail\n\n",
        ),
    ] {
        out.push(Fixture {
            name,
            kind,
            mime: if kind == DocumentKind::Text {
                "text/plain"
            } else {
                "text/markdown"
            },
            bytes: text.as_bytes().to_vec(),
        });
    }
    out.push(Fixture {
        name: "text-invalid",
        kind: DocumentKind::Text,
        mime: "text/plain",
        bytes: vec![0xff],
    });
    out.push(Fixture {
        name: "text-nul",
        kind: DocumentKind::Text,
        mime: "text/plain",
        bytes: b"a\0b".to_vec(),
    });
    out
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
fn envelope(f: &Fixture) -> Result<Extraction, String> {
    let mut child = Command::new(PathBuf::from(ROOT).join("bin/current-brn-intake-helper"))
        .env_clear()
        .current_dir(PathBuf::from(ROOT).join("private-tmp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let input = serde_json::to_vec(&HelperRequest {
        limits: None,
        kind: if f.kind == DocumentKind::Html {
            "html"
        } else {
            "eml"
        }
        .into(),
        bytes: f.bytes.clone(),
    })
    .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    fs::write(
        PathBuf::from(ROOT)
            .join("captures")
            .join(format!("{}-current-envelope.raw.json", f.name)),
        &out.stdout,
    )
    .unwrap();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let d: Extraction = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    d.validate()?;
    Ok(d)
}
fn observations() -> &'static [Observation] {
    static OBS: OnceLock<Vec<Observation>> = OnceLock::new();
    OBS.get_or_init(||{
        let root=PathBuf::from(ROOT);let temp=root.join("current-host-tmp");fs::create_dir_all(&temp).unwrap();
        let converter=Converter{helper_path:root.join("bin/current-brn-intake-helper"),pdf_tools:None,temp_root:temp.clone(),limits:IntakeLimits::default()};
        let rt=tokio::runtime::Builder::new_current_thread().max_blocking_threads(2).thread_stack_size(16*1024*1024).build().unwrap();
        let cfg=config();assert!(!cfg.needs_image_data());save("config.json",&serde_json::to_value(&cfg).unwrap());
        let out=rt.block_on(async {
            let mut out=Vec::new();
            for f in fixtures(){
                fs::write(root.join("fixtures").join(format!("{}.input",f.name)),&f.bytes).unwrap();
                let request=||ConversionRequest{kind:f.kind,intent:ImportIntent::FullNote,bytes:&f.bytes,source_reference:f.name,inventory:None};
                let current=converter.convert(request(),&AtomicBool::new(false));
                let prepared=converter.prepare(request(),&AtomicBool::new(false));
                save(&format!("{}-current-convert.json",f.name),&json!({"result":current.as_ref().ok(),"error":current.as_ref().err().map(|e|format!("{e:?}"))}));
                save(&format!("{}-current-prepared.json",f.name),&json!({"result":prepared.as_ref().ok(),"error":prepared.as_ref().err().map(|e|format!("{e:?}"))}));
                let envelope=if matches!(f.kind,DocumentKind::Eml|DocumentKind::Html){Some(envelope(&f))}else{None};
                let result=xberg::extract(ExtractInput::from_bytes(f.bytes.clone(),f.mime,None),&cfg).await;
                let candidate=match result{
                    Ok(mut r)=>{save(&format!("{}-xberg.raw.json",f.name),&serde_json::to_value(&r).unwrap());if !r.errors.is_empty()||r.results.len()!=1{Err(format!("result envelope errors={:?}, results={}",r.errors,r.results.len()))}else{Ok(r.results.remove(0))}},
                    Err(e)=>{save(&format!("{}-xberg.raw.json",f.name),&json!({"error":e.to_string()}));Err(e.to_string())}
                };
                out.push(Observation{fixture:f,candidate,current,prepared,envelope});
            }out
        });drop(rt);assert_eq!(fs::read_dir(temp).unwrap().count(),0,"host transient children cleaned");out
    })
}
fn get(name: &str) -> &'static Observation {
    observations()
        .iter()
        .find(|o| o.fixture.name == name)
        .unwrap()
}
fn candidate(name: &str) -> &'static ExtractedDocument {
    get(name).candidate.as_ref().unwrap()
}
fn cells(name: &str) -> Vec<&'static PreparedCell> {
    get(name)
        .prepared
        .as_ref()
        .unwrap()
        .blocks
        .iter()
        .flat_map(|b| match &b.content {
            BlockContent::Table { rows } => rows.iter().flatten().collect(),
            _ => vec![],
        })
        .collect()
}
fn paragraphs(d: &ExtractedDocument) -> Vec<&str> {
    d.document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|n| {
            if let NodeContent::Paragraph { text } = &n.content {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect()
}
fn email_meta(name: &str) -> &xberg::types::metadata::EmailMetadata {
    match candidate(name).metadata.format.as_ref().unwrap() {
        FormatMetadata::Email(m) => m,
        _ => panic!("expected typed email metadata"),
    }
}
#[test]
fn a_capture_complete_values_evidence_only() {
    assert_eq!(observations().len(), 17, "capture count is not parity");
}
#[test]
fn current_text_and_markdown_keep_complete_bytes() {
    for o in observations().iter().filter(|o| {
        matches!(o.fixture.kind, DocumentKind::Text | DocumentKind::Markdown)
            && !matches!(o.fixture.name, "text-invalid" | "text-nul")
    }) {
        let d = o.current.as_ref().unwrap();
        assert_eq!(d.markdown.as_bytes(), o.fixture.bytes);
        assert_eq!(d.sources.len(), 1);
        assert_eq!(d.sources[0].text.as_bytes(), o.fixture.bytes);
        assert_eq!(d.source_sha256, digest(&o.fixture.bytes));
        assert_eq!(d.converter, "UTF-8 verbatim");
    }
}
#[test]
fn current_route_support_and_decode_policy() {
    for o in observations()
        .iter()
        .filter(|o| matches!(o.fixture.kind, DocumentKind::Text | DocumentKind::Markdown))
    {
        assert_eq!(o.prepared.as_ref().unwrap_err(), &ConversionError::Invalid);
    }
    assert_eq!(
        get("text-invalid").current.as_ref().unwrap_err(),
        &ConversionError::Invalid
    );
    assert_eq!(
        get("text-nul").current.as_ref().unwrap_err(),
        &ConversionError::Budget
    );
    for n in ["forecast", "facts", "html-control"] {
        assert_eq!(
            get(n).current.as_ref().unwrap_err(),
            &ConversionError::Unavailable
        );
    }
}
fn exact(name: &str) {
    assert_eq!(
        candidate(name).content.as_bytes(),
        get(name).fixture.bytes,
        "required verbatim bytes, with no normalization"
    );
}
#[test]
fn text_crlf_exact() {
    exact("text-crlf");
}
#[test]
fn text_bom_exact() {
    exact("text-bom");
}
#[test]
fn text_markup_exact() {
    exact("text-markup");
}
#[test]
fn text_no_final_newline_exact() {
    exact("text-no-final");
}
#[test]
fn text_blank_lines_exact() {
    exact("text-blanks");
}
#[test]
fn markdown_crlf_exact() {
    exact("md-crlf");
}
#[test]
fn markdown_fence_links_images_exact() {
    exact("md-fence");
}
#[test]
fn markdown_bom_exact() {
    exact("md-bom");
}
#[test]
fn markdown_blank_lines_exact() {
    exact("md-blanks");
}
#[test]
fn xlsx_current_source_facts() {
    let a = cells("forecast");
    assert_eq!(
        a.iter()
            .map(|c| (c.address.as_str(), c.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("A1", "Out of scope forecast"),
            ("B1", "EUR"),
            ("A2", "Harbor budget"),
            ("B2", "4000")
        ]
    );
    let a = cells("facts");
    assert_eq!(
        a.iter()
            .map(|c| (c.address.as_str(), c.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("C3", "Café Ω"),
            ("D3", "45000"),
            ("E3", "true"),
            ("F3", "#DIV/0!"),
            ("G3", "99"),
            ("H3", ""),
            ("C5", ""),
            ("E5", "9007199254740993"),
            ("A1", "Hidden retained")
        ]
    );
    assert_eq!(
        (a[4].formula.as_deref(), a[4].result_status.as_deref()),
        (Some("SUM(A1:A2)"), Some("cached"))
    );
    assert_eq!(
        (a[5].formula.as_deref(), a[5].result_status.as_deref()),
        (Some("1+2"), Some("unknown"))
    );
    let d = get("facts").prepared.as_ref().unwrap();
    assert_eq!(
        d.blocks
            .iter()
            .filter_map(|b| if let BlockContent::Heading { text, .. } = &b.content {
                Some(text.as_str())
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec!["Facts", "Hidden Ω", "Empty"]
    );
    assert!(d.blocks.iter().any(|b|matches!(&b.content,BlockContent::Metadata{name,value} if name=="Worksheet state"&&value=="hidden")));
    assert!(
        d.blocks
            .iter()
            .filter(|b| matches!(b.content, BlockContent::Table { .. }))
            .all(|b| b.locator.starts_with("xlsx/sheet-"))
    );
    assert!(
        d.pages.is_empty(),
        "current XLSX exposes typed cells, not page preview rasters"
    );
}
#[test]
fn xlsx_forecast_table_and_page() {
    let d = candidate("forecast");
    assert_eq!(d.tables.len(), 1);
    assert_eq!(
        d.tables[0].cells,
        vec![
            vec!["Out of scope forecast", "EUR"],
            vec!["Harbor budget", "4000"]
        ]
    );
    let p = d.pages.as_ref().unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].sheet_name.as_deref(), Some("Sheet"));
    assert_eq!(p[0].tables[0].cells, d.tables[0].cells);
}
#[test]
fn xlsx_sheet_order_hidden_and_empty() {
    let d = candidate("facts");
    let p = d.pages.as_ref().unwrap();
    assert_eq!(
        p.iter()
            .map(|p| p.sheet_name.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["Facts", "Hidden Ω", "Empty"]
    );
    assert_eq!(p[1].tables[0].cells, vec![vec!["Hidden retained"]]);
    assert!(p[2].tables.is_empty());
    assert_eq!(p[2].is_blank, Some(true));
    assert_eq!(
        d.metadata.additional.get("hidden_sheets"),
        Some(&json!("Hidden Ω"))
    );
}
#[test]
fn xlsx_sparse_absolute_geometry() {
    let d = candidate("facts");
    let grid = d
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
    let c = grid.cells.iter().find(|c| c.content == "Café Ω").unwrap();
    assert_eq!(
        (c.row, c.col),
        (2, 2),
        "source C3; relative rectangle origin is not source position"
    );
}
#[test]
fn xlsx_numeric_identifier_precision() {
    assert_eq!(
        candidate("facts").tables[0].cells[2][2],
        "9007199254740993",
        "exact source decimal identifier; no f64 rounding"
    );
}
#[test]
fn xlsx_formula_absolute_association() {
    assert_eq!(
        candidate("facts").metadata.additional.get("formulas_Facts"),
        Some(&json!("G3=SUM(A1:A2); H3=1+2")),
        "source coordinates, not formula-range-relative references"
    );
}
#[test]
fn eml_current_payloads_and_scope() {
    for name in ["single", "plural", "multipart"] {
        let o = get(name);
        assert!(o.current.is_ok(), "{name}: {:?}", o.current.as_ref().err());
        assert!(
            o.prepared.is_ok(),
            "{name}: {:?}",
            o.prepared.as_ref().err()
        );
        let d = o.envelope.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(d.sources[0].bytes, o.fixture.bytes);
        assert_eq!(d.original_sha256, digest(&o.fixture.bytes));
        if name != "multipart" {
            assert_eq!(
                d.sources
                    .iter()
                    .find(|s| s.media_type == "text/plain")
                    .unwrap()
                    .bytes,
                PLAIN.as_bytes()
            );
            assert_eq!(
                d.sources
                    .iter()
                    .find(|s| s.media_type == "text/html")
                    .unwrap()
                    .bytes,
                HTML.as_bytes()
            );
            for (n, b) in [
                (
                    "harbor.docx",
                    include_bytes!("../../fixtures/harbor.docx").as_slice(),
                ),
                (
                    "inline-water.png",
                    include_bytes!("../../fixtures/water-use.png").as_slice(),
                ),
            ] {
                let s = d.sources.iter().find(|s| s.name == n).unwrap();
                assert_eq!(s.bytes, b);
                assert!(s.parent.is_some());
            }
            if name == "plural" {
                let s = d
                    .sources
                    .iter()
                    .find(|s| s.name == "forecast.xlsx")
                    .unwrap();
                assert_eq!(s.bytes, include_bytes!("../../fixtures/forecast.xlsx"));
                assert_eq!(s.status, "unprocessed");
                assert!(!d.gaps.is_empty());
            }
        } else {
            let dup = d
                .sources
                .iter()
                .filter(|s| s.name == "dup.bin")
                .collect::<Vec<_>>();
            assert_eq!(dup.len(), 2);
            assert_eq!(
                dup.iter().map(|s| s.bytes.as_slice()).collect::<Vec<_>>(),
                vec![b"alpha".as_slice(), b"beta".as_slice()]
            );
            assert_ne!(dup[0].id, dup[1].id);
            assert_ne!(dup[0].locator, dup[1].locator);
            assert!(
                dup.iter()
                    .all(|s| s.status == "unprocessed"
                        && s.parent.as_ref() == Some(&d.sources[0].id))
            );
            let child = d.sources.iter().find(|s| s.name == "child.eml").unwrap();
            assert_eq!(child.bytes, NESTED.as_bytes());
            // A single-part nested message stores its body on this existing
            // message source (helper.rs::MimeContext::visit body_root), not a
            // fabricated extra child. Raw NESTED bytes remain the body oracle.
            assert_eq!(child.parent.as_ref(), Some(&d.sources[0].id));
            assert!(
                child
                    .text
                    .ends_with("\nActual plain-text body\n\n```\nNested body Ω\r\n\n```\n")
            );
            let retained = o
                .current
                .as_ref()
                .unwrap()
                .sources
                .iter()
                .find(|s| s.locator == child.locator)
                .unwrap();
            assert_eq!(retained.text, child.text);
        }
    }
}
#[test]
fn eml_headers_and_attachment_names() {
    for name in ["single", "plural", "multipart"] {
        let d = candidate(name);
        let m = email_meta(name);
        assert_eq!(m.from_email.as_deref(), Some("mira@example.test"));
        let (subject, id, attachments) = if name == "multipart" {
            (
                "Résumé Ω",
                "synthetic@example.test",
                vec!["dup.bin", "dup.bin", "child.eml"],
            )
        } else if name == "single" {
            (
                "Harbor pilot – review",
                "harbor-single@example.test",
                vec!["inline-water.png", "harbor.docx"],
            )
        } else {
            (
                "Harbor pilot – review",
                "harbor-plural@example.test",
                vec!["inline-water.png", "harbor.docx", "forecast.xlsx"],
            )
        };
        assert_eq!(d.metadata.subject.as_deref(), Some(subject));
        assert_eq!(m.message_id.as_deref(), Some(id));
        assert_eq!(m.attachments, attachments);
        let (display_name, recipient, recipient_display, date) = if name == "multipart" {
            (
                "Mira Ω",
                "team@example.test",
                "Team <team@example.test>",
                "Sat, 17 Oct 2026 12:00:00 +0300",
            )
        } else {
            (
                "Mira",
                "ops@example.test",
                "Operations <ops@example.test>",
                "Tue, 06 Oct 2026 16:20:00 +0300",
            )
        };
        assert_eq!(m.from_name.as_deref(), Some(display_name));
        assert_eq!(m.to_emails, vec![recipient]);
        assert!(m.cc_emails.is_empty() && m.bcc_emails.is_empty());
        assert_eq!(d.metadata.created_at.as_deref(), Some(date));
        let prepared = get(name).prepared.as_ref().unwrap();
        let headers: Vec<_> = prepared
            .blocks
            .iter()
            .filter_map(|b| {
                if !b.locator.starts_with("email/header/") {
                    return None;
                }
                if let BlockContent::Metadata { name, value } = &b.content {
                    Some((name.as_str(), value.as_str()))
                } else {
                    None
                }
            })
            .collect();
        let from = format!("{display_name} <mira@example.test>");
        let mut expected = vec![
            ("From", from.as_str()),
            ("To", recipient_display),
            ("Subject", subject),
            // Current HeaderValue::DateTime uses mail-parser0.11.8
            // to_rfc822(), whose day is intentionally not zero-padded.
            // Candidate created_at above retains the original header spelling.
            (
                "Date",
                if name == "multipart" {
                    date
                } else {
                    "Tue, 6 Oct 2026 16:20:00 +0300"
                },
            ),
            ("Message-ID", id),
        ];
        if name == "multipart" {
            expected.extend([("X-Trace", "first"), ("X-Trace", "second")]);
        }
        assert_eq!(
            headers, expected,
            "complete ordered top-level public header projection"
        );
    }
}
#[test]
fn eml_html_body_complete_paragraphs() {
    for name in ["single", "plural", "multipart"] {
        let ps = paragraphs(candidate(name));
        let end = ps.iter().position(|s| *s == "Attachments:").unwrap();
        let expected = if name == "multipart" {
            // Existing eml.rs::collect_nested_message_html also projects the
            // authored nested plain body into this sequence. This verifies
            // wording only; the separate child-identity assertion stays red.
            vec!["HTML Ω", "Second line", "Nested body Ω"]
        } else {
            // The authored CID image has no alt; email.rs::process_node uses
            // this fixed placeholder. No image bytes/identity proof is implied.
            vec!["Water use fell 40%.", "[Image]", "See attached decision."]
        };
        assert_eq!(&ps[..end], expected);
    }
}
#[test]
fn eml_plain_alternative_not_discarded() {
    for name in ["single", "plural", "multipart"] {
        let expected = if name == "multipart" {
            "Plain Ω"
        } else {
            "Please review the attached Harbor pilot. Extend to Pier B after inspection."
        };
        assert!(
            paragraphs(candidate(name)).contains(&expected),
            "{name}: complete real plain-body paragraph required alongside alternative"
        );
    }
}
#[test]
fn eml_nested_source_identity_retained() {
    let d = candidate("multipart");
    let children = d.children.as_deref().unwrap_or(&[]);
    assert_eq!(
        children.len(),
        1,
        "current supported nested message needs a located child, depth0 candidate provides none"
    );
    assert_eq!(children[0].result.content, "Nested body Ω");
}
#[test]
fn html_textual_control_is_not_rich_adoption() {
    let d = get("html-control")
        .envelope
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(d.converter, brn_intake::HTML_CONVERTER);
    assert_eq!(
        paragraphs(candidate("html-control")),
        vec!["Alpha text.", "Beta text."]
    );
    assert!(d.assets.is_empty());
    assert!(d.gaps.iter().all(|g| !g.contains("unprocessed")));
}
#[test]
fn source_input_receipts() {
    for o in observations() {
        assert_eq!(
            fs::read(
                PathBuf::from(ROOT)
                    .join("fixtures")
                    .join(format!("{}.input", o.fixture.name))
            )
            .unwrap(),
            o.fixture.bytes
        );
        save(
            &format!("{}-input-receipt.json", o.fixture.name),
            &json!({"bytes":o.fixture.bytes.len(),"sha256":hex(&digest(&o.fixture.bytes)),"kind":o.fixture.kind,"mime":o.fixture.mime}),
        );
    }
}

#[test]
fn eml_current_prepared_nested_body_is_not_silent_loss() {
    let doc = get("multipart").prepared.as_ref().unwrap();
    assert!(
        doc.blocks.iter().any(|b| matches!(&b.content,
        BlockContent::Paragraph { text } if text == "Nested body Ω\r\n")),
        "source-authored nested body must be represented; preserve current-reader failure if absent"
    );
}
#[test]
fn xlsx_error_value_lexeme() {
    assert_eq!(
        candidate("facts").tables[0].cells[0][3],
        "#DIV/0!",
        "current source error value is retained literally, not renamed to enum debug text"
    );
}
#[test]
fn xlsx_missing_cache_formula_cell_is_retained() {
    assert_eq!(
        candidate("facts").tables[0].cells[0].len(),
        6,
        "source H3 is an actual formula-bearing cell even without a cached value; C3:H3 has six origins"
    );
}
