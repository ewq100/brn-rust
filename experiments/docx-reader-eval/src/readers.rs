//! One adapter per candidate. Each returns what an adapter in BRN could
//! actually use: the reader's interpreted output (rendered text and/or its
//! public model), image occurrences in document order, image bytes it exposes,
//! and any warnings. Raw package XML is deliberately excluded: re-reading the
//! source is what the evaluation asks a library to replace.
use std::io::Cursor;

pub struct Model {
    /// Searchable interpreted output: rendered Markdown/HTML plus the public
    /// model serialized as JSON or `Debug`.
    pub dump: String,
    /// Image occurrences in document order, as the reader identifies them.
    pub occurrences: Vec<String>,
    /// Image bytes the reader exposes, with the id it uses for them.
    pub images: Vec<(String, Vec<u8>)>,
    /// Reader-reported warnings about content it could not represent.
    pub warnings: Vec<String>,
    /// Items the reader's public API reports as present but not modelled
    /// (qualified names only, never raw bytes). An adapter can refuse on these.
    pub unmodelled: Vec<String>,
}

pub const READERS: &[&str] = &["docx-rs", "rdocx", "office_oxide", "betteroffice"];

/// Budgets matching BRN's DOCX guard where the reader accepts them.
const MAX_ENTRIES: usize = 256;
const MAX_PART: u64 = 8 * 1024 * 1024;
const MAX_EXPANDED: u64 = 32 * 1024 * 1024;

/// A reader's parsed result, before the evaluator inspects it.
pub enum Parsed {
    DocxRs(Box<docx_rs::Docx>),
    Rdocx(Box<rdocx::Document>),
    /// The IR is built inside the timed call: it is the content model.
    OfficeOxide(Box<(office_oxide::Document, office_oxide::ir::DocumentIR)>),
    BetterOffice(Box<docx_parse::S9WireEnvelope>),
}

/// The library call only; this is what is timed and allocation-tracked.
pub fn parse(reader: &str, bytes: &[u8]) -> Result<Parsed, String> {
    match reader {
        "docx-rs" => docx_rs::read_docx_with_options(
            bytes,
            docx_rs::ReadDocxOptions::default().with_image_previews(false),
        )
        .map(|d| Parsed::DocxRs(Box::new(d)))
        .map_err(|e| format!("{e:?}")),
        "rdocx" => {
            let limits = rdocx::PackageReadLimits {
                max_entries: MAX_ENTRIES,
                max_part_uncompressed_bytes: MAX_PART,
                max_total_uncompressed_bytes: MAX_EXPANDED,
            };
            rdocx::Document::from_bytes_with_limits(bytes, limits)
                .map(|d| Parsed::Rdocx(Box::new(d)))
                .map_err(|e| format!("{e:?}"))
        }
        "office_oxide" => {
            // Its package limits are process-global setters.
            office_oxide::limits::set_max_package_entries(MAX_ENTRIES);
            office_oxide::limits::set_max_package_bytes(MAX_EXPANDED);
            office_oxide::Document::from_reader(
                Cursor::new(bytes.to_vec()),
                office_oxide::format::DocumentFormat::Docx,
            )
            .map(|d| {
                let ir = d.to_ir();
                Parsed::OfficeOxide(Box::new((d, ir)))
            })
            .map_err(|e| format!("{e:?}"))
        }
        "betteroffice" => {
            use docx_parse::{ParseLimits, S9ParseOptions, parse_docx_s9_wire_with_limits};
            let limits = ParseLimits {
                max_xml_bytes: MAX_PART as usize,
                max_xml_depth: 64,
                ..ParseLimits::default()
            };
            parse_docx_s9_wire_with_limits(bytes, S9ParseOptions::default(), &limits)
                .map(|e| Parsed::BetterOffice(Box::new(e)))
                .map_err(|e| format!("{e:?}"))
        }
        other => panic!("unknown reader {other}"),
    }
}

/// Everything an adapter could see through the reader's public output.
pub fn model(parsed: Parsed) -> Result<Model, String> {
    match parsed {
        Parsed::DocxRs(d) => docx_rs(*d),
        Parsed::Rdocx(d) => rdocx(*d),
        Parsed::OfficeOxide(d) => office_oxide(d.0, d.1),
        Parsed::BetterOffice(e) => betteroffice(*e),
    }
}

fn compact(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("reader emits JSON")
}

// docx-rs 0.4.22: `read_docx_with_options`, previews off. No limits API.
fn docx_rs(docx: docx_rs::Docx) -> Result<Model, String> {
    let json = compact(&docx.json());
    let mut occurrences = Vec::new();
    collect_pic_ids(&json["document"], &mut occurrences);
    Ok(Model {
        dump: format!("{json}\n{docx:?}"),
        occurrences,
        images: docx
            .images
            .iter()
            .map(|(id, _, image, _)| (id.clone(), image.0.clone()))
            .collect(),
        warnings: Vec::new(),
        unmodelled: Vec::new(),
    })
}
fn collect_pic_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("pic")
                && let Some(id) = map["data"]["id"].as_str()
            {
                out.push(id.to_owned());
            }
            for v in map.values() {
                collect_pic_ids(v, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_pic_ids(v, out)),
        _ => {}
    }
}

// rdocx 0.15.0: `Document::from_bytes_with_limits`, public facade only
// (Markdown, HTML, images(), links(), per-item text).
fn rdocx(doc: rdocx::Document) -> Result<Model, String> {
    let (unmodelled, fields) = rdocx_items(&doc);
    let infos = doc.images();
    let texts: Vec<_> = doc
        .story_item_snapshots()
        .map_err(|e| format!("{e:?}"))?
        .iter()
        .map(|s| s.text().map(str::to_owned))
        .collect();
    let images = infos
        .iter()
        .filter_map(|i| doc.image_data(&i.embed_id).map(|b| (i.embed_id.clone(), b)))
        .collect();
    let revisions: Vec<_> = doc
        .revisions()
        .iter()
        .map(|r| format!("{} {} {:?}", r.id(), r.author(), r.kind()))
        .collect();
    let comments: Vec<_> = doc
        .comments()
        .iter()
        .map(|c| format!("{} {:?} {}", c.id(), c.author(), c.text()))
        .collect();
    Ok(Model {
        dump: format!(
            "{}\n{}\n{infos:?}\n{:?}\n{texts:?}\nfootnotes {:?}\nrevisions {revisions:?}\ncomments {comments:?}\nfacts {fields:?}\nunmodelled {unmodelled:?}",
            doc.to_markdown(),
            doc.to_html(),
            doc.links(),
            doc.footnotes()
        ),
        occurrences: infos.iter().map(|i| i.embed_id.clone()).collect(),
        images,
        warnings: Vec::new(),
        unmodelled,
    })
}

/// rdocx's reader facade: body, paragraph, hyperlink and run items in source
/// order. Unmodelled children surface as `UnsupportedXml` and some elements
/// flag unmodelled attributes; record only names and flags, never raw bytes.
/// Tables, headers and notes are not walked (no case needs them).
fn rdocx_items(doc: &rdocx::Document) -> (Vec<String>, Vec<String>) {
    use rdocx::{BodyItemRef, HyperlinkItemRef, ParagraphItemRef, RunItemRef, RunRef};
    fn name(raw: &[u8]) -> String {
        let item = rdocx::UnsupportedXmlRef::from_bytes(raw);
        format!("UnsupportedXml:{}", item.qualified_name().unwrap_or("?"))
    }
    fn run(run: RunRef<'_>, unmodelled: &mut Vec<String>, facts: &mut Vec<String>) {
        for item in run.items() {
            match item {
                RunItemRef::UnsupportedXml(raw) => unmodelled.push(name(raw)),
                RunItemRef::Field(field) => {
                    facts.push(format!("field {}", field.instruction()));
                    if field.has_unmodeled_semantic_attributes() {
                        unmodelled.push("field attributes".into());
                    }
                }
                _ => {}
            }
        }
    }
    let (mut unmodelled, mut facts) = (Vec::new(), Vec::new());
    for item in doc.body_items() {
        match item {
            BodyItemRef::UnsupportedXml(raw) => unmodelled.push(name(raw)),
            BodyItemRef::Paragraph(paragraph) => {
                for item in paragraph.items() {
                    match item {
                        ParagraphItemRef::UnsupportedXml(raw) => unmodelled.push(name(raw)),
                        ParagraphItemRef::Run(r) => run(r, &mut unmodelled, &mut facts),
                        ParagraphItemRef::Hyperlink(link) => {
                            facts.push(format!("hyperlink tooltip {:?}", link.tooltip()));
                            if link.has_unmodeled_semantic_attributes() {
                                unmodelled.push("hyperlink attributes".into());
                            }
                            for item in link.items() {
                                match item {
                                    HyperlinkItemRef::Run(r) => run(r, &mut unmodelled, &mut facts),
                                    HyperlinkItemRef::UnsupportedXml(raw) => {
                                        unmodelled.push(name(raw))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    (unmodelled, facts)
}

// office_oxide 0.1.13: `Document::from_reader(.., Docx)`, IR + Markdown.
fn office_oxide(
    doc: office_oxide::Document,
    ir: office_oxide::ir::DocumentIR,
) -> Result<Model, String> {
    let json = serde_json::to_value(&ir).expect("IR serializes");
    let mut found = Vec::new();
    collect_ir_images(&json, &mut found);
    // The public DOCX model keeps drawings in place with their relationship id,
    // and the image parts of the main document keyed by that id.
    let docx = doc.as_docx().expect("DOCX document");
    let body = format!("{:?}", docx.body);
    let occurrences = body
        .match_indices("relationship_id: \"")
        .map(|(at, m)| {
            let rest = &body[at + m.len()..];
            rest[..rest.find('"').unwrap_or(0)].to_owned()
        })
        .filter(|id| !id.is_empty())
        .collect();
    let mut images: Vec<_> = docx
        .images
        .iter()
        .map(|(id, (bytes, _))| (id.clone(), bytes.clone()))
        .collect();
    images.extend(found);
    let mut warnings = ir.metadata.warnings.clone();
    warnings.extend(docx.unreadable_parts.iter().cloned());
    Ok(Model {
        dump: format!(
            "{}\n{json}\n{body}\n{:?}",
            doc.to_markdown(),
            docx.headers_footers
        ),
        occurrences,
        images,
        warnings,
        unmodelled: Vec::new(),
    })
}
fn collect_ir_images(value: &serde_json::Value, out: &mut Vec<(String, Vec<u8>)>) {
    use base64::Engine;
    match value {
        serde_json::Value::Object(map) => {
            if let Some(data) = map.get("data").and_then(|d| d.as_str())
                && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data)
            {
                let alt = map.get("alt_text").and_then(|a| a.as_str()).unwrap_or("");
                out.push((alt.to_owned(), bytes));
            }
            for v in map.values() {
                collect_ir_images(v, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_ir_images(v, out)),
        _ => {}
    }
}

// betteroffice-docx-parse 0.3.0: `parse_docx_s9_wire_with_limits` (bounded
// XML on top of betteroffice-opc's ZIP guard), typed wire model.
fn betteroffice(envelope: docx_parse::S9WireEnvelope) -> Result<Model, String> {
    let json = serde_json::to_value(&envelope).expect("wire serializes");
    // Body image occurrences carry their rId and a resolved data URL.
    let mut found = Vec::new();
    collect_wire_images(&json["document"]["package"]["document"], &mut found);
    let occurrences = found.iter().map(|(id, _)| id.clone()).collect();
    let mut images = found;
    for (key, media) in &envelope.document.package.media_entries {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&media.base64)
            .unwrap_or_default();
        images.push((key.clone(), bytes));
    }
    Ok(Model {
        dump: json.to_string(),
        occurrences,
        images,
        warnings: envelope.document.warnings.clone().unwrap_or_default(),
        unmodelled: Vec::new(),
    })
}
fn collect_wire_images(value: &serde_json::Value, out: &mut Vec<(String, Vec<u8>)>) {
    use base64::Engine;
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("image") {
                let id = map
                    .get("rId")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_owned();
                let bytes = map
                    .get("src")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.split_once(";base64,"))
                    .and_then(|(_, data)| {
                        base64::engine::general_purpose::STANDARD.decode(data).ok()
                    })
                    .unwrap_or_default();
                out.push((id, bytes));
            }
            for v in map.values() {
                collect_wire_images(v, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_wire_images(v, out)),
        _ => {}
    }
}
