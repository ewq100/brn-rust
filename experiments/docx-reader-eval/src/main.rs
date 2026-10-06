//! H4 evaluator: does published docx-rs 0.4.22 retain enough of a synthetic DOCX
//! for a small BRN admission/mapping adapter? Every fixture is synthetic. Each
//! case records the observed verdict and fails the run if it differs from the
//! recorded expectation, so the findings are reproducible assertions.
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::{Cursor, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

use docx_rs::{Docx, ReadDocxOptions, read_docx, read_docx_with_options};

// ---------------------------------------------------------------- allocation

struct Counting;
static TRACK: AtomicBool = AtomicBool::new(false);
static LARGEST: AtomicUsize = AtomicUsize::new(0);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn note(size: usize) {
    if TRACK.load(Relaxed) {
        LARGEST.fetch_max(size, Relaxed);
        let now = CURRENT.fetch_add(size, Relaxed) + size;
        PEAK.fetch_max(now, Relaxed);
    }
}
fn forget(size: usize) {
    if TRACK.load(Relaxed) {
        let _ = CURRENT.fetch_update(Relaxed, Relaxed, |c| Some(c.saturating_sub(size)));
    }
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        forget(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        forget(layout.size());
        note(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;

// ------------------------------------------------------------------ fixtures

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const NS: &str = concat!(
    r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" "#,
    r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" "#,
    r#"xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" "#,
    r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" "#,
    r#"xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" "#,
    r#"xmlns:v="urn:schemas-microsoft-com:vml""#
);
const ROOT_RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
const JPEG: &[u8] = include_bytes!("../fixtures/ordinary.jpg");

fn archive(parts: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for &(name, bytes) in parts {
        let options = zip::write::SimpleFileOptions::default().compression_method(method);
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn types(extra: &str) -> String {
    format!(
        concat!(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
            r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
            r#"<Default Extension="xml" ContentType="application/xml"/>"#,
            r#"<Default Extension="png" ContentType="image/png"/>"#,
            r#"<Default Extension="jpg" ContentType="image/jpeg"/>"#,
            r#"<Default Extension="html" ContentType="text/html"/>"#,
            r#"<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#,
            "{}</Types>"
        ),
        extra
    )
}
fn document(body: &str) -> String {
    format!("<w:document {NS}><w:body>{body}</w:body></w:document>")
}
fn rels(entries: &[(&str, &str, &str)]) -> String {
    let mut out = String::from(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (id, kind, target) in entries {
        let mode = if *kind == "hyperlink" {
            r#" TargetMode="External""#
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<Relationship Id="{id}" Type="{REL}/{kind}" Target="{target}"{mode}/>"#
        ));
    }
    out + "</Relationships>"
}

/// Ordinary package: content types, root rels, document, document rels, extra parts.
struct Package {
    types: String,
    document: Vec<u8>,
    rels: Option<String>,
    extra: Vec<(String, Vec<u8>)>,
    method: zip::CompressionMethod,
}
impl Package {
    fn new(body: &str) -> Self {
        Self {
            types: types(""),
            document: document(body).into_bytes(),
            rels: Some(rels(&[])),
            extra: Vec::new(),
            method: zip::CompressionMethod::Deflated,
        }
    }
    fn rels(mut self, entries: &[(&str, &str, &str)]) -> Self {
        self.rels = Some(rels(entries));
        self
    }
    fn part(mut self, name: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.extra.push((name.into(), bytes.into()));
        self
    }
    fn types(mut self, overrides: &str) -> Self {
        self.types = types(overrides);
        self
    }
    fn bytes(&self) -> Vec<u8> {
        let mut parts: Vec<(&str, &[u8])> = vec![
            ("[Content_Types].xml", self.types.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", &self.document),
        ];
        if let Some(rels) = &self.rels {
            parts.push(("word/_rels/document.xml.rels", rels.as_bytes()));
        }
        for (name, bytes) in &self.extra {
            parts.push((name, bytes));
        }
        archive(&parts, self.method)
    }
}

fn p(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}
fn png(pixel: [u8; 4]) -> Vec<u8> {
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32fast::hash(&out[4..]).to_be_bytes());
        out
    }
    let mut header = 1u32.to_be_bytes().to_vec();
    header.extend(1u32.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&[0, pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    out.extend(chunk(b"IHDR", &header));
    out.extend(chunk(b"IDAT", &z.finish().unwrap()));
    out.extend(chunk(b"IEND", &[]));
    out
}
fn drawing(rid: &str, descr: &str) -> String {
    drawing_titled(rid, descr, "Picture caption")
}
fn drawing_titled(rid: &str, descr: &str, title: &str) -> String {
    format!(
        concat!(
            r#"<w:drawing><wp:inline><wp:extent cx="9525" cy="9525"/><wp:docPr id="1" name="Picture" descr="{descr}" title="{title}"/>"#,
            r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic>"#,
            r#"<pic:nvPicPr><pic:cNvPr id="0" name="Picture"/><pic:cNvPicPr/></pic:nvPicPr>"#,
            r#"<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>"#,
            r#"<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9525" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>"#,
            r#"</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#
        ),
        rid = rid,
        descr = descr,
        title = title
    )
}
fn image_run(rid: &str, descr: &str) -> String {
    format!(
        "<w:p><w:r><w:t>Before õ</w:t>{}<w:t>After</w:t></w:r></w:p>",
        drawing(rid, descr)
    )
}
const HEADER_CT: &str = r#"<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>"#;
fn header(body: &str) -> String {
    format!("<w:hdr {NS}>{body}</w:hdr>")
}

// ------------------------------------------------------------------- reading

enum Read {
    Ok(Box<Docx>),
    Err(String),
    Panic(String),
}
struct Measured {
    read: Read,
    largest: usize,
    peak: usize,
    millis: u128,
}
fn measure(f: impl FnOnce() -> Result<Docx, docx_rs::ReaderError>) -> Measured {
    LARGEST.store(0, Relaxed);
    CURRENT.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    let start = Instant::now();
    TRACK.store(true, Relaxed);
    // Silence only the reader's own panic; evaluator panics stay visible.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    TRACK.store(false, Relaxed);
    let millis = start.elapsed().as_millis();
    let read = match result {
        Ok(Ok(docx)) => Read::Ok(Box::new(docx)),
        Ok(Err(e)) => Read::Err(format!("{e:?}")),
        Err(payload) => Read::Panic(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        ),
    };
    Measured {
        read,
        largest: LARGEST.load(Relaxed),
        peak: PEAK.load(Relaxed),
        millis,
    }
}
thread_local! {
    static CASE: std::cell::RefCell<(&'static str, usize)> = const { std::cell::RefCell::new(("", 0)) };
}
/// `DOCX_EVAL_FIXTURES=DIR` writes every evaluated package to DIR/<case>-<n>.docx
/// so the identical bytes can be replayed through BRN's current converter.
fn keep(bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("DOCX_EVAL_FIXTURES") {
        let name = CASE.with_borrow_mut(|(id, n)| {
            *n += 1;
            format!("{id}-{n}.docx")
        });
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}
fn originals(bytes: &[u8]) -> Measured {
    keep(bytes);
    let m = measure(|| {
        read_docx_with_options(bytes, ReadDocxOptions::default().with_image_previews(false))
    });
    // `DOCX_EVAL_DUMP=1` prints each serialized read model for inspection.
    if std::env::var_os("DOCX_EVAL_DUMP").is_some()
        && let Some(model) = json(&m.read)
    {
        eprintln!("{model}");
    }
    m
}

// ------------------------------------------------------------------ verdicts

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Content/asset retained in the read model.
    Retained,
    /// Reader returned an error (an explicit refusal BRN could map).
    Refused,
    /// Read succeeded; the meaningful content is absent from the model and
    /// nothing in the model shows that it was dropped.
    SilentLoss,
    /// Read succeeded and gives a meaning BRN's admission rejects
    /// (non-DOCX namespace, alias, encoding) without signalling it.
    Misread,
    /// Wording survives but structure/semantics are flattened unobservably.
    Flattened,
    /// Asset/occurrence mapping cannot be resolved from the model alone.
    Ambiguous,
    /// The gap is visible in the model (adapter can refuse).
    DetectableGap,
    /// Reader panicked on malformed input.
    Panic,
    /// Allocation is driven by advertised, untrusted sizes.
    AdvertisedAllocation,
}

struct Case {
    id: &'static str,
    group: &'static str,
    title: &'static str,
    brn: &'static str,
    expected: Verdict,
    run: fn() -> (Verdict, String),
}

/// Compact JSON of only the `document` subtree (body content and drawings),
/// excluding the top-level `images` list.
fn document_json(read: &Read) -> Option<String> {
    match read {
        Read::Ok(docx) => {
            Some(
                serde_json::from_str::<serde_json::Value>(&docx.json())
                    .expect("docx-rs emits JSON")["document"]
                    .to_string(),
            )
        }
        _ => None,
    }
}
fn json(read: &Read) -> Option<String> {
    match read {
        // Compact form so needles do not depend on pretty-printing.
        Read::Ok(docx) => Some(
            serde_json::from_str::<serde_json::Value>(&docx.json())
                .expect("docx-rs emits JSON")
                .to_string(),
        ),
        _ => None,
    }
}
fn summary(m: &Measured) -> String {
    match &m.read {
        Read::Ok(_) => "Ok".into(),
        Read::Err(e) => format!("Err({e})"),
        Read::Panic(p) => format!("panic: {p}"),
    }
}
/// Generic sentinel oracle over the complete serialized read model.
fn sentinel(bytes: &[u8], wanted: &[&str]) -> (Verdict, String) {
    traced(bytes, wanted, None)
}
/// As `sentinel`, but a missing sentinel with `trace` still present in the
/// model is a gap an adapter could detect and refuse.
fn traced(bytes: &[u8], wanted: &[&str], trace: Option<&str>) -> (Verdict, String) {
    let m = originals(bytes);
    let Some(model) = json(&m.read) else {
        return match m.read {
            Read::Panic(_) => (Verdict::Panic, summary(&m)),
            _ => (Verdict::Refused, summary(&m)),
        };
    };
    // docx-rs serializes through hand-written `Serialize` impls; also search the
    // derived `Debug` form so data retained but not serialized is not "lost".
    let debug = match &m.read {
        Read::Ok(docx) => format!("{docx:?}"),
        _ => String::new(),
    };
    let missing: Vec<_> = wanted
        .iter()
        .filter(|s| !model.contains(**s) && !debug.contains(**s))
        .collect();
    if missing.is_empty() {
        (Verdict::Retained, "Ok; all sentinels in model".into())
    } else if let Some(trace) = trace.filter(|t| model.contains(*t)) {
        (
            Verdict::DetectableGap,
            format!("Ok; absent from model: {missing:?}; trace kept: {trace}"),
        )
    } else {
        (
            Verdict::SilentLoss,
            format!("Ok; absent from model: {missing:?}"),
        )
    }
}
fn images(read: &Read) -> Vec<(String, String, Vec<u8>)> {
    match read {
        Read::Ok(docx) => docx
            .images
            .iter()
            .map(|(id, path, image, _)| (id.clone(), path.clone(), image.0.clone()))
            .collect(),
        _ => Vec::new(),
    }
}
fn count(model: &str, needle: &str) -> usize {
    model.matches(needle).count()
}

// --------------------------------------------------------------------- cases

fn cases() -> Vec<Case> {
    vec![
        // ---- supported BRN profile ----
        Case {
            id: "S0",
            group: "supported",
            title: "Minimal package without word/_rels/document.xml.rels",
            brn: "Ok DocxTextV1",
            expected: Verdict::Refused,
            run: || {
                let mut package = Package::new(&p("Minimal õ"));
                package.rels = None;
                sentinel(&package.bytes(), &["Minimal õ"])
            },
        },
        Case {
            id: "S1",
            group: "supported",
            title: "Unicode paragraphs, Stored and Deflate",
            brn: "Ok DocxTextV1 (exact)",
            expected: Verdict::Retained,
            run: || {
                let mut out = Vec::new();
                for method in [
                    zip::CompressionMethod::Stored,
                    zip::CompressionMethod::Deflated,
                ] {
                    let mut package = Package::new(&(p("First õ 日本語") + &p("Second preserved")));
                    package.method = method;
                    let (v, d) =
                        sentinel(&package.bytes(), &["First õ 日本語", "Second preserved"]);
                    if v != Verdict::Retained {
                        return (v, format!("{method:?}: {d}"));
                    }
                    out.push(format!("{method:?} ok"));
                }
                (Verdict::Retained, out.join(", "))
            },
        },
        Case {
            id: "S2",
            group: "supported",
            title: "Heading style, numbered list, external link, simple table",
            brn: "Ok DocxTextV1 (exact)",
            expected: Verdict::Retained,
            run: || {
                let body = concat!(
                    r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Heading õ</w:t></w:r></w:p>"#,
                    r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>First item</w:t></w:r></w:p>"#,
                    r#"<w:p><w:hyperlink r:id="link"><w:r><w:t>Reference</w:t></w:r></w:hyperlink></w:p>"#,
                    r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>left cell</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>right cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#
                );
                let styles = format!(
                    r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Heading1"><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style></w:styles>"#
                );
                let numbering = format!(
                    r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1)"/></w:lvl></w:abstractNum><w:num w:numId="7"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
                );
                let bytes = Package::new(body)
                    .types(concat!(
                        r#"<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>"#,
                        r#"<Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>"#
                    ))
                    .rels(&[
                        ("styles", "styles", "styles.xml"),
                        ("numbering", "numbering", "numbering.xml"),
                        ("link", "hyperlink", "https://example.invalid/a?q=one&amp;b=two"),
                    ])
                    .part("word/styles.xml", styles)
                    .part("word/numbering.xml", numbering)
                    .bytes();
                sentinel(
                    &bytes,
                    &[
                        "Heading õ",
                        "\"Heading1\"",
                        "First item",
                        "%1)",
                        "Reference",
                        "https://example.invalid/a?q=one&b=two",
                        "left cell",
                        "right cell",
                    ],
                )
            },
        },
        Case {
            id: "S3",
            group: "supported",
            title: "One inline PNG: exact bytes, occurrence order, alt/title wording",
            brn: "Ok DocxInlinePngV1 (bytes, position, alt, title)",
            expected: Verdict::SilentLoss,
            run: || {
                let image = png([0, 255, 0, 128]);
                let run = format!(
                    "<w:p><w:r><w:t>Before õ</w:t>{}<w:t>After</w:t></w:r></w:p>",
                    drawing_titled("image1", "ALT-SENTINEL 日本語", "TITLE-SENTINEL")
                );
                let bytes = Package::new(&run)
                    .rels(&[("image1", "image", "media/picture.png")])
                    .part("word/media/picture.png", image.clone())
                    .bytes();
                let m = originals(&bytes);
                let Some(model) = document_json(&m.read) else {
                    return (Verdict::Refused, summary(&m));
                };
                let debug = match &m.read {
                    Read::Ok(docx) => format!("{docx:?}"),
                    _ => String::new(),
                };
                let found = images(&m.read);
                let exact = found.len() == 1 && found[0].0 == "image1" && found[0].2 == image;
                let before = model.find("Before õ");
                let pic = model.find("\"image1\"");
                let after = model.find("After");
                let ordered =
                    matches!((before, pic, after), (Some(b), Some(i), Some(a)) if b < i && i < a);
                let alt = model.contains("ALT-SENTINEL 日本語") || debug.contains("ALT-SENTINEL");
                let title = model.contains("TITLE-SENTINEL") || debug.contains("TITLE-SENTINEL");
                let detail = format!(
                    "Ok; exact bytes={exact}, rId in run order={ordered}; docPr alt text in model={alt}, title in model={title}"
                );
                if exact && ordered && alt && title {
                    (Verdict::Retained, detail)
                } else {
                    (Verdict::SilentLoss, detail)
                }
            },
        },
        // ---- meaningful content outside the BRN profile ----
        Case {
            id: "M1",
            group: "meaningful",
            title: "Final-section default header",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let body = p("Body")
                    + r#"<w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
                let bytes = Package::new(&body)
                    .types(HEADER_CT)
                    .rels(&[("h1", "header", "header1.xml")])
                    .part("word/header1.xml", header(&p("HEADER-SENTINEL")))
                    .bytes();
                sentinel(&bytes, &["HEADER-SENTINEL"])
            },
        },
        Case {
            id: "M2",
            group: "meaningful",
            title: "Header referenced only by an earlier section",
            brn: "docx_unsupported",
            expected: Verdict::DetectableGap,
            run: || {
                let body = concat!(
                    r#"<w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr></w:pPr><w:r><w:t>Section one</w:t></w:r></w:p>"#,
                    r#"<w:p><w:r><w:t>Section two</w:t></w:r></w:p><w:sectPr/>"#
                );
                let bytes = Package::new(body)
                    .types(HEADER_CT)
                    .rels(&[("h1", "header", "header1.xml")])
                    .part("word/header1.xml", header(&p("EARLY-HEADER-SENTINEL")))
                    .bytes();
                traced(
                    &bytes,
                    &["Section one", "EARLY-HEADER-SENTINEL"],
                    Some(r#""headerReference":{"headerType":"default","id":"h1"}"#),
                )
            },
        },
        Case {
            id: "M3",
            group: "meaningful",
            title: "Final-section default footer",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let body = p("Body")
                    + r#"<w:sectPr><w:footerReference w:type="default" r:id="f1"/></w:sectPr>"#;
                let bytes = Package::new(&body)
                    .types(r#"<Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>"#)
                    .rels(&[("f1", "footer", "footer1.xml")])
                    .part("word/footer1.xml", format!("<w:ftr {NS}>{}</w:ftr>", p("FOOTER-SENTINEL")))
                    .bytes();
                sentinel(&bytes, &["Body", "FOOTER-SENTINEL"])
            },
        },
        Case {
            id: "M4",
            group: "meaningful",
            title: "Footnote reference (same run as text) and footnotes part",
            brn: "docx_unsupported",
            expected: Verdict::SilentLoss,
            run: || {
                let body =
                    r#"<w:p><w:r><w:t>Claim</w:t><w:footnoteReference w:id="1"/></w:r></w:p>"#;
                let notes = format!(
                    r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="1"><w:p><w:r><w:t>FOOTNOTE-SENTINEL</w:t></w:r></w:p></w:footnote></w:footnotes>"#
                );
                let bytes = Package::new(body)
                    .types(r#"<Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>"#)
                    .rels(&[("fn", "footnotes", "footnotes.xml")])
                    .part("word/footnotes.xml", notes)
                    .bytes();
                sentinel(&bytes, &["Claim", "FOOTNOTE-SENTINEL"])
            },
        },
        Case {
            id: "M5",
            group: "meaningful",
            title: "Comment range, reference and comments part",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let body = r#"<w:p><w:commentRangeStart w:id="0"/><w:r><w:t>Commented</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r></w:p>"#;
                let comments = format!(
                    r#"<w:comments xmlns:w="{W}"><w:comment w:id="0" w:author="Synthetic"><w:p><w:r><w:t>COMMENT-SENTINEL</w:t></w:r></w:p></w:comment></w:comments>"#
                );
                let bytes = Package::new(body)
                    .types(r#"<Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>"#)
                    .rels(&[("c", "comments", "comments.xml")])
                    .part("word/comments.xml", comments)
                    .bytes();
                sentinel(&bytes, &["Commented", "COMMENT-SENTINEL"])
            },
        },
        Case {
            id: "M6",
            group: "meaningful",
            title: "Tracked insertion and deletion",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let body = r#"<w:p><w:ins w:id="1" w:author="A"><w:r><w:t>INSERTED-SENTINEL</w:t></w:r></w:ins><w:del w:id="2" w:author="A"><w:r><w:delText>DELETED-SENTINEL</w:delText></w:r></w:del></w:p>"#;
                sentinel(
                    &Package::new(body).bytes(),
                    &["INSERTED-SENTINEL", "DELETED-SENTINEL"],
                )
            },
        },
        Case {
            id: "M7",
            group: "meaningful",
            title: "Second prefix bound to WordprocessingML inside a `w:` run",
            brn: "Ok DocxTextV1 (both wordings)",
            expected: Verdict::SilentLoss,
            run: || {
                let xml = format!(
                    r#"<w:document xmlns:w="{W}" xmlns:x="{W}"><w:body><w:p><w:r><w:t>Visible </w:t><x:t>PREFIX-SENTINEL</x:t></w:r></w:p></w:body></w:document>"#
                );
                let mut package = Package::new("");
                package.document = xml.into_bytes();
                sentinel(&package.bytes(), &["Visible ", "PREFIX-SENTINEL"])
            },
        },
        Case {
            id: "M8",
            group: "meaningful",
            title: "WordprocessingML as the default namespace (no prefix)",
            brn: "Ok DocxTextV1 (wording kept)",
            expected: Verdict::SilentLoss,
            run: || {
                let xml = format!(
                    r#"<document xmlns="{W}"><body><p><r><t>DEFAULT-NS-SENTINEL</t></r></p></body></document>"#
                );
                let mut package = Package::new("");
                package.document = xml.into_bytes();
                sentinel(&package.bytes(), &["DEFAULT-NS-SENTINEL"])
            },
        },
        Case {
            id: "M9",
            group: "meaningful",
            title: "`w` prefix bound to a non-WordprocessingML namespace",
            brn: "docx_unsupported",
            expected: Verdict::Misread,
            run: || {
                let xml = r#"<w:document xmlns:w="urn:example:not-wordprocessingml"><w:body><w:p><w:r><w:t>FOREIGN-SENTINEL</w:t></w:r></w:p></w:body></w:document>"#;
                let mut package = Package::new("");
                package.document = xml.as_bytes().to_vec();
                match sentinel(&package.bytes(), &["FOREIGN-SENTINEL"]) {
                    (Verdict::Retained, _) => (
                        Verdict::Misread,
                        "Ok; foreign-namespace wording read as document text".into(),
                    ),
                    other => other,
                }
            },
        },
        Case {
            id: "M10",
            group: "meaningful",
            title: "Body-level altChunk imported part",
            brn: "docx_unsupported",
            expected: Verdict::SilentLoss,
            run: || {
                let body = p("Before chunk") + r#"<w:altChunk r:id="alt"/>"#;
                let bytes = Package::new(&body)
                    .rels(&[("alt", "aFChunk", "afchunk.html")])
                    .part(
                        "word/afchunk.html",
                        "<html><body>ALTCHUNK-SENTINEL</body></html>",
                    )
                    .bytes();
                sentinel(&bytes, &["Before chunk", "ALTCHUNK-SENTINEL"])
            },
        },
        Case {
            id: "M11",
            group: "meaningful",
            title: "Legacy VML text box (w:pict/v:shape/v:textbox)",
            brn: "docx_unsupported",
            expected: Verdict::DetectableGap,
            run: || {
                let body = concat!(
                    r#"<w:p><w:r><w:t>Anchor</w:t></w:r><w:r><w:pict><v:shape id="s1" style="width:100pt;height:40pt">"#,
                    r#"<v:textbox><w:txbxContent><w:p><w:r><w:t>VML-SENTINEL</w:t></w:r></w:p></w:txbxContent></v:textbox>"#,
                    r#"</v:shape></w:pict></w:r></w:p>"#
                );
                traced(
                    &Package::new(body).bytes(),
                    &["Anchor", "VML-SENTINEL"],
                    Some(r#""type":"shape""#),
                )
            },
        },
        Case {
            id: "M12",
            group: "meaningful",
            title: "Ruby annotation (base and phonetic text)",
            brn: "docx_unsupported",
            expected: Verdict::Flattened,
            run: || {
                let body = concat!(
                    r#"<w:p><w:r><w:ruby><w:rubyPr/><w:rt><w:r><w:t>RUBY-TEXT</w:t></w:r></w:rt>"#,
                    r#"<w:rubyBase><w:r><w:t>RUBY-BASE</w:t></w:r></w:rubyBase></w:ruby></w:r>"#,
                    r#"<w:r><w:t>Tail</w:t></w:r></w:p>"#
                );
                match sentinel(
                    &Package::new(body).bytes(),
                    &["RUBY-TEXT", "RUBY-BASE", "Tail", "ruby"],
                ) {
                    (Verdict::SilentLoss, d) if d.contains("\"ruby\"") && !d.contains("RUBY-") => (
                        Verdict::Flattened,
                        "Ok; phonetic guide and base become consecutive plain runs".into(),
                    ),
                    other => other,
                }
            },
        },
        Case {
            id: "M13",
            group: "meaningful",
            title: "Simple HYPERLINK field: destination vs cached result",
            brn: "docx_unsupported",
            expected: Verdict::SilentLoss,
            run: || {
                let body = r#"<w:p><w:fldSimple w:instr=" HYPERLINK &quot;https://field.invalid/&quot; "><w:r><w:t>FIELD-RESULT</w:t></w:r></w:fldSimple></w:p>"#;
                match sentinel(&Package::new(body).bytes(), &["FIELD-RESULT", "field.invalid"]) {
                    (Verdict::SilentLoss, d) if d.contains("field.invalid") && !d.contains("FIELD-RESULT") => (
                        Verdict::SilentLoss,
                        "Ok; result wording kept as plain run; instruction and link destination absent".into(),
                    ),
                    other => other,
                }
            },
        },
        Case {
            id: "M14",
            group: "meaningful",
            title: "Inline customXml wrapper around a run",
            brn: "docx_unsupported",
            expected: Verdict::Flattened,
            run: || {
                let body = r#"<w:p><w:customXml w:uri="urn:example:tags" w:element="secret"><w:r><w:t>CUSTOMXML-WORDING</w:t></w:r></w:customXml></w:p>"#;
                match sentinel(
                    &Package::new(body).bytes(),
                    &["CUSTOMXML-WORDING", "urn:example:tags"],
                ) {
                    (Verdict::SilentLoss, d) if !d.contains("CUSTOMXML-WORDING") => (
                        Verdict::Flattened,
                        "Ok; wording kept as plain run, wrapper semantics absent".into(),
                    ),
                    other => other,
                }
            },
        },
        // ---- budgets, integrity and package aliases ----
        Case {
            id: "B1",
            group: "budget",
            title: "Document entry advertising 3.75 GiB (0xF0000000) uncompressed size",
            brn: "docx_limit",
            expected: Verdict::AdvertisedAllocation,
            run: || {
                let mut bytes = Package::new(&p("tiny")).bytes();
                let declared = 0xF000_0000usize;
                patch_entry(&mut bytes, "word/document.xml", |b, local, central| {
                    set32(b, local + 22, declared);
                    set32(b, central + 24, declared);
                });
                let m = originals(&bytes);
                let detail = format!(
                    "{}; largest single allocation {} bytes",
                    summary(&m),
                    m.largest
                );
                if m.largest >= declared {
                    (Verdict::AdvertisedAllocation, detail)
                } else {
                    (Verdict::Refused, detail)
                }
            },
        },
        Case {
            id: "B2",
            group: "budget",
            title: "Stored document entry with corrupted bytes (CRC mismatch)",
            brn: "docx_invalid",
            expected: Verdict::Panic,
            run: || {
                let mut package = Package::new(&p("CRC-SENTINEL"));
                package.method = zip::CompressionMethod::Stored;
                let mut bytes = package.bytes();
                let at = find(&bytes, b"CRC-SENTINEL").unwrap();
                bytes[at] = b'X';
                sentinel(&bytes, &["XRC-SENTINEL"])
            },
        },
        Case {
            id: "B3",
            group: "budget",
            title: "Main part stored under a backslash name",
            brn: "docx_invalid",
            expected: Verdict::Refused,
            run: || {
                let mut bytes = Package::new(&p("BACKSLASH-SENTINEL")).bytes();
                rename_entry(&mut bytes, "word/document.xml", "word\\document.xml");
                match sentinel(&bytes, &["BACKSLASH-SENTINEL"]) {
                    (Verdict::Retained, _) => (
                        Verdict::Misread,
                        "Ok; backslash-named entry read as the main part".into(),
                    ),
                    other => other,
                }
            },
        },
        Case {
            id: "B4",
            group: "budget",
            title: "Case-alias second main part (word/ vs WORD/)",
            brn: "docx_invalid",
            expected: Verdict::Misread,
            run: || {
                let bytes = Package::new(&p("LOWER-SENTINEL"))
                    .part("WORD/document.xml", document(&p("UPPER-SENTINEL")))
                    .bytes();
                let (v, d) = sentinel(&bytes, &["LOWER-SENTINEL"]);
                if v == Verdict::Retained {
                    (
                        Verdict::Misread,
                        "Ok; one alias silently chosen, the other ignored".into(),
                    )
                } else {
                    (v, d)
                }
            },
        },
        Case {
            id: "B5",
            group: "budget",
            title: "Internal DTD entity in document text",
            brn: "docx_unsupported",
            expected: Verdict::Refused,
            run: || {
                let xml = format!(
                    r#"<!DOCTYPE w:document [<!ENTITY s "ENTITY-SENTINEL">]><w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t>A&s;B</w:t></w:r></w:p></w:body></w:document>"#
                );
                let mut package = Package::new("");
                package.document = xml.into_bytes();
                sentinel(&package.bytes(), &["AENTITY-SENTINELB"])
            },
        },
        Case {
            id: "B6",
            group: "budget",
            title: "UTF-16 encoded main part",
            brn: "docx_unsupported",
            expected: Verdict::Refused,
            run: || {
                let xml = format!(
                    r#"<?xml version="1.0" encoding="UTF-16"?><w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t>UTF16-SENTINEL</w:t></w:r></w:p></w:body></w:document>"#
                );
                let mut encoded = vec![0xff, 0xfe];
                for unit in xml.encode_utf16() {
                    encoded.extend(unit.to_le_bytes());
                }
                let mut package = Package::new("");
                package.document = encoded;
                match sentinel(&package.bytes(), &["UTF16-SENTINEL"]) {
                    (Verdict::Retained, _) => (Verdict::Misread, "Ok; UTF-16 accepted".into()),
                    other => other,
                }
            },
        },
        Case {
            id: "B7",
            group: "budget",
            title: "Body inside BRN's XML budget (8,001 paragraphs, ~7.1 MiB XML): uncancellable read",
            brn: "docx_limit (1 MiB output cap; XML budget passes)",
            expected: Verdict::Retained,
            run: || {
                // BRN's XML guard: <= 8 MiB total XML and <= 50,000 raw `<`/`=`
                // delimiters per part. This body passes both, so BRN refuses only
                // at its 1 MiB rendered-output cap.
                let line = format!("<w:p><w:r><w:t>{}</w:t></w:r></w:p>", "a".repeat(900));
                let body = line.repeat(8_000) + &p("LAST-SENTINEL");
                let xml = document(&body);
                let delimiters = xml.bytes().filter(|b| matches!(b, b'<' | b'=')).count();
                assert!(delimiters <= 50_000 && xml.len() < 8 * 1024 * 1024 - 4096);
                let bytes = Package::new(&body).bytes();
                let m = originals(&bytes);
                let ok = json(&m.read).is_some_and(|j| j.contains("LAST-SENTINEL"));
                let detail = format!(
                    "{}; {} ms in one call with no cancellation hook; peak heap {} MiB; xml {} bytes, {} delimiters",
                    summary(&m),
                    m.millis,
                    m.peak / (1024 * 1024),
                    xml.len(),
                    delimiters
                );
                (
                    if ok {
                        Verdict::Retained
                    } else {
                        Verdict::SilentLoss
                    },
                    detail,
                )
            },
        },
        // ---- images ----
        Case {
            id: "I1",
            group: "images",
            title: "Two distinct PNGs, two relationships, two occurrences",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let (a, b) = (png([255, 0, 0, 255]), png([0, 0, 255, 255]));
                let bytes = Package::new(&(image_run("imgA", "A") + &image_run("imgB", "B")))
                    .rels(&[
                        ("imgA", "image", "media/a.png"),
                        ("imgB", "image", "media/b.png"),
                    ])
                    .part("word/media/a.png", a.clone())
                    .part("word/media/b.png", b.clone())
                    .bytes();
                let m = originals(&bytes);
                let found = images(&m.read);
                let model = document_json(&m.read).unwrap_or_default();
                let exact = found.iter().any(|i| i.0 == "imgA" && i.2 == a)
                    && found.iter().any(|i| i.0 == "imgB" && i.2 == b);
                let ordered = matches!((model.find("\"imgA\""), model.find("\"imgB\"")), (Some(x), Some(y)) if x < y);
                if exact && ordered && found.len() == 2 {
                    (
                        Verdict::Retained,
                        "exact bytes; occurrences in document order by rId".into(),
                    )
                } else {
                    (
                        Verdict::SilentLoss,
                        format!("{} images exact={exact} ordered={ordered}", found.len()),
                    )
                }
            },
        },
        Case {
            id: "I2",
            group: "images",
            title: "One relationship used by two occurrences",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let image = png([1, 2, 3, 255]);
                let bytes =
                    Package::new(&(image_run("img", "first") + &image_run("img", "second")))
                        .rels(&[("img", "image", "media/one.png")])
                        .part("word/media/one.png", image.clone())
                        .bytes();
                let m = originals(&bytes);
                let found = images(&m.read);
                let model = document_json(&m.read).unwrap_or_default();
                let refs = count(&model, "\"img\"");
                if found.len() == 1 && found[0].2 == image && refs == 2 {
                    (
                        Verdict::Retained,
                        format!("one asset; {refs} drawings in the document name its rId"),
                    )
                } else {
                    (
                        Verdict::SilentLoss,
                        format!("{} images, {refs} references", found.len()),
                    )
                }
            },
        },
        Case {
            id: "I3",
            group: "images",
            title: "Ordinary JPEG with previews disabled",
            brn: "docx_unsupported",
            expected: Verdict::Retained,
            run: || {
                let bytes = Package::new(&image_run("jpg", "photo"))
                    .rels(&[("jpg", "image", "media/photo.jpg")])
                    .part("word/media/photo.jpg", JPEG)
                    .bytes();
                let m = originals(&bytes);
                let found = images(&m.read);
                if found.len() == 1 && found[0].2 == JPEG {
                    (
                        Verdict::Retained,
                        "exact original JPEG bytes and rId retained".into(),
                    )
                } else {
                    (Verdict::SilentLoss, format!("{} images", found.len()))
                }
            },
        },
        Case {
            id: "I4",
            group: "images",
            title: "Same JPEG through default read_docx without the `image` feature",
            brn: "docx_unsupported",
            expected: Verdict::SilentLoss,
            run: || {
                let bytes = Package::new(&image_run("jpg", "photo"))
                    .rels(&[("jpg", "image", "media/photo.jpg")])
                    .part("word/media/photo.jpg", JPEG)
                    .bytes();
                keep(&bytes);
                let m = measure(|| read_docx(&bytes));
                let found = images(&m.read);
                match &m.read {
                    Read::Ok(_) if found.is_empty() => (
                        Verdict::SilentLoss,
                        "Ok; image omitted from Docx::images while the drawing remains".into(),
                    ),
                    Read::Ok(_) => (Verdict::Retained, format!("{} images", found.len())),
                    _ => (Verdict::Refused, summary(&m)),
                }
            },
        },
        Case {
            id: "I5",
            group: "images",
            title: "Header image and body image both use relationship id rId1",
            brn: "docx_unsupported",
            expected: Verdict::Ambiguous,
            run: || {
                let (body_png, header_png) = (png([10, 10, 10, 255]), png([200, 200, 200, 255]));
                let body = image_run("rId1", "body")
                    + r#"<w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
                let hdr = header(&image_run("rId1", "header"));
                let bytes = Package::new(&body)
                    .types(HEADER_CT)
                    .rels(&[
                        ("rId1", "image", "media/body.png"),
                        ("h1", "header", "header1.xml"),
                    ])
                    .part("word/header1.xml", hdr)
                    .part(
                        "word/_rels/header1.xml.rels",
                        rels(&[("rId1", "image", "media/header.png")]),
                    )
                    .part("word/media/body.png", body_png.clone())
                    .part("word/media/header.png", header_png.clone())
                    .bytes();
                let m = originals(&bytes);
                let found = images(&m.read);
                let matching: Vec<_> = found.iter().filter(|i| i.0 == "rId1").collect();
                let first_is_body = matching.first().is_some_and(|i| i.2 == body_png);
                if matching.len() > 1 {
                    (
                        Verdict::Ambiguous,
                        format!(
                            "{} images share id rId1 (first is body image: {first_is_body}); paths {:?}",
                            matching.len(),
                            matching.iter().map(|i| i.1.as_str()).collect::<Vec<_>>()
                        ),
                    )
                } else {
                    (Verdict::Retained, format!("{} images", found.len()))
                }
            },
        },
        Case {
            id: "I6",
            group: "images",
            title: "Image relationship whose part is missing",
            brn: "docx_invalid",
            expected: Verdict::DetectableGap,
            run: || {
                let bytes = Package::new(&image_run("gone", "missing"))
                    .rels(&[("gone", "image", "media/missing.png")])
                    .bytes();
                let m = originals(&bytes);
                let found = images(&m.read);
                let model = json(&m.read).unwrap_or_default();
                match &m.read {
                    Read::Ok(_) if found.is_empty() && model.contains("\"gone\"") => (
                        Verdict::DetectableGap,
                        "Ok; no image entry, drawing still names rId (adapter can refuse)".into(),
                    ),
                    _ => (Verdict::Refused, summary(&m)),
                }
            },
        },
    ]
}

// --------------------------------------------------------------- zip patches

fn u16_at(b: &[u8], at: usize) -> usize {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap()) as usize
}
fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize
}
fn set32(b: &mut [u8], at: usize, value: usize) {
    b[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
/// Calls `f(bytes, local_header, central_header)` for the named entry.
fn patch_entry(b: &mut Vec<u8>, name: &str, f: impl FnOnce(&mut Vec<u8>, usize, usize)) {
    let footer = b.len() - 22;
    let count = u16_at(b, footer + 10);
    let mut at = u32_at(b, footer + 16);
    for _ in 0..count {
        let len = u16_at(b, at + 28);
        let next = at + 46 + len + u16_at(b, at + 30) + u16_at(b, at + 32);
        if &b[at + 46..at + 46 + len] == name.as_bytes() {
            let local = u32_at(b, at + 42);
            f(b, local, at);
            return;
        }
        at = next;
    }
    panic!("entry {name} not found");
}
/// Same-length rename in both local and central headers.
fn rename_entry(b: &mut Vec<u8>, from: &str, to: &str) {
    assert_eq!(from.len(), to.len());
    patch_entry(b, from, |b, local, central| {
        b[local + 30..local + 30 + to.len()].copy_from_slice(to.as_bytes());
        b[central + 46..central + 46 + to.len()].copy_from_slice(to.as_bytes());
    });
}

// ---------------------------------------------------------------------- main

fn main() {
    println!(
        "| ID | Group | Case | BRN converter at 450eaa2 | docx-rs 0.4.22 verdict | Observation |"
    );
    println!("| --- | --- | --- | --- | --- | --- |");
    let mut mismatches = 0;
    // Optional argument: run only the listed case IDs.
    let only: Vec<String> = std::env::args().skip(1).collect();
    for case in cases() {
        if !only.is_empty() && !only.iter().any(|id| id == case.id) {
            continue;
        }
        CASE.with_borrow_mut(|c| *c = (case.id, 0));
        let (verdict, detail) = (case.run)();
        let flag = if verdict == case.expected {
            ""
        } else {
            mismatches += 1;
            " **(unexpected)**"
        };
        println!(
            "| {} | {} | {} | {} | {:?}{} | {} |",
            case.id,
            case.group,
            case.title,
            case.brn,
            verdict,
            flag,
            detail.replace('|', "\\|")
        );
    }
    if mismatches > 0 {
        eprintln!("{mismatches} case(s) differ from the recorded expectation");
        std::process::exit(1);
    }
}
