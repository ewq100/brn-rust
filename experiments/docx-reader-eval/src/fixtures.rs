//! Synthetic DOCX fixtures. Each case ID maps to one package; every reader
//! sees identical bytes.
use std::io::{Cursor, Write};

pub const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
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

// ------------------------------------------------------------- case packages

pub const PNG_A: [u8; 4] = [255, 0, 0, 255];
pub const PNG_B: [u8; 4] = [0, 0, 255, 255];
pub const PNG_ONE: [u8; 4] = [0, 255, 0, 128];
pub const PNG_SHARED: [u8; 4] = [1, 2, 3, 255];
pub const PNG_BODY: [u8; 4] = [10, 10, 10, 255];
pub const PNG_HEADER: [u8; 4] = [200, 200, 200, 255];
/// Declared uncompressed size patched into B1's main part.
pub const B1_DECLARED: usize = 0xF000_0000;
pub fn jpeg() -> &'static [u8] {
    JPEG
}
pub fn png_bytes(pixel: [u8; 4]) -> Vec<u8> {
    png(pixel)
}

pub fn build(id: &str) -> Vec<u8> {
    match id {
        "S0" => {
            let mut package = Package::new(&p("Minimal õ"));
            package.rels = None;
            package.bytes()
        }
        "S1a" | "S1b" => {
            let mut package = Package::new(&(p("First õ 日本語") + &p("Second preserved")));
            package.method = if id == "S1a" {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            };
            package.bytes()
        }
        "S2" => {
            let body = concat!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Heading õ</w:t></w:r></w:p>"#,
                r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>First item</w:t></w:r></w:p>"#,
                r#"<w:p><w:hyperlink r:id="link"><w:r><w:t>Reference</w:t></w:r></w:hyperlink></w:p>"#,
                r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>left cell</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>right cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#
            );
            let styles = format!(
                r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style></w:styles>"#
            );
            let numbering = format!(
                r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1)"/></w:lvl></w:abstractNum><w:num w:numId="7"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
            );
            Package::new(body)
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
                .bytes()
        }
        "S4" => Package::new(r#"<w:p><w:hyperlink r:id="link" w:tooltip="TOOLTIP-SENTINEL"><w:r><w:t>Linked words</w:t></w:r></w:hyperlink></w:p>"#)
            .rels(&[("link", "hyperlink", "https://example.invalid/tip")])
            .bytes(),
        "S3" => {
            let run = format!(
                "<w:p><w:r><w:t>Before õ</w:t>{}<w:t>After</w:t></w:r></w:p>",
                drawing_titled("image1", "ALT-SENTINEL 日本語", "TITLE-SENTINEL")
            );
            Package::new(&run)
                .rels(&[("image1", "image", "media/picture.png")])
                .part("word/media/picture.png", png(PNG_ONE))
                .bytes()
        }
        "M1" => {
            let body = p("Body")
                + r#"<w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
            Package::new(&body)
                .types(HEADER_CT)
                .rels(&[("h1", "header", "header1.xml")])
                .part("word/header1.xml", header(&p("HEADER-SENTINEL")))
                .bytes()
        }
        "M2" => {
            let body = concat!(
                r#"<w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rIdEarlyHdr7"/></w:sectPr></w:pPr><w:r><w:t>Section one</w:t></w:r></w:p>"#,
                r#"<w:p><w:r><w:t>Section two</w:t></w:r></w:p><w:sectPr/>"#
            );
            Package::new(body)
                .types(HEADER_CT)
                .rels(&[("rIdEarlyHdr7", "header", "header1.xml")])
                .part("word/header1.xml", header(&p("EARLY-HEADER-SENTINEL")))
                .bytes()
        }
        "M3" => {
            let body = p("Body")
                + r#"<w:sectPr><w:footerReference w:type="default" r:id="f1"/></w:sectPr>"#;
            Package::new(&body)
                .types(r#"<Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>"#)
                .rels(&[("f1", "footer", "footer1.xml")])
                .part(
                    "word/footer1.xml",
                    format!("<w:ftr {NS}>{}</w:ftr>", p("FOOTER-SENTINEL")),
                )
                .bytes()
        }
        "M4" => {
            let body = r#"<w:p><w:r><w:t>Claim</w:t><w:footnoteReference w:id="7731"/></w:r></w:p>"#;
            let notes = format!(
                r#"<w:footnotes xmlns:w="{W}"><w:footnote w:id="7731"><w:p><w:r><w:t>FOOTNOTE-SENTINEL</w:t></w:r></w:p></w:footnote></w:footnotes>"#
            );
            Package::new(body)
                .types(r#"<Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>"#)
                .rels(&[("fn", "footnotes", "footnotes.xml")])
                .part("word/footnotes.xml", notes)
                .bytes()
        }
        "M5" => {
            let body = r#"<w:p><w:commentRangeStart w:id="0"/><w:r><w:t>Commented</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r></w:p>"#;
            let comments = format!(
                r#"<w:comments xmlns:w="{W}"><w:comment w:id="0" w:author="Synthetic"><w:p><w:r><w:t>COMMENT-SENTINEL</w:t></w:r></w:p></w:comment></w:comments>"#
            );
            Package::new(body)
                .types(r#"<Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>"#)
                .rels(&[("c", "comments", "comments.xml")])
                .part("word/comments.xml", comments)
                .bytes()
        }
        "M6" => Package::new(r#"<w:p><w:ins w:id="1" w:author="ReviserZ9"><w:r><w:t>INSERTED-SENTINEL</w:t></w:r></w:ins><w:del w:id="2" w:author="ReviserZ9"><w:r><w:delText>DELETED-SENTINEL</w:delText></w:r></w:del></w:p>"#).bytes(),
        "M7" => {
            let mut package = Package::new("");
            package.document = format!(
                r#"<w:document xmlns:w="{W}" xmlns:x="{W}"><w:body><w:p><w:r><w:t>Visible </w:t><x:t>PREFIX-SENTINEL</x:t></w:r></w:p></w:body></w:document>"#
            )
            .into_bytes();
            package.bytes()
        }
        "M8" => {
            let mut package = Package::new("");
            package.document = format!(
                r#"<document xmlns="{W}"><body><p><r><t>DEFAULT-NS-SENTINEL</t></r></p></body></document>"#
            )
            .into_bytes();
            package.bytes()
        }
        "M9" => {
            let mut package = Package::new("");
            package.document = r#"<w:document xmlns:w="urn:example:not-wordprocessingml"><w:body><w:p><w:r><w:t>FOREIGN-SENTINEL</w:t></w:r></w:p></w:body></w:document>"#.as_bytes().to_vec();
            package.bytes()
        }
        "M10" => {
            let body = p("Before chunk") + r#"<w:altChunk r:id="rIdAltChunk9"/>"#;
            Package::new(&body)
                .rels(&[("rIdAltChunk9", "aFChunk", "afchunk.html")])
                .part("word/afchunk.html", "<html><body>ALTCHUNK-SENTINEL</body></html>")
                .bytes()
        }
        "M11" => Package::new(concat!(
            r#"<w:p><w:r><w:t>Anchor</w:t></w:r><w:r><w:pict><v:shape id="VmlShape42" style="width:101pt;height:41pt">"#,
            r#"<v:textbox><w:txbxContent><w:p><w:r><w:t>VML-SENTINEL</w:t></w:r></w:p></w:txbxContent></v:textbox>"#,
            r#"</v:shape></w:pict></w:r></w:p>"#
        ))
        .bytes(),
        "M12" => Package::new(concat!(
            r#"<w:p><w:r><w:ruby><w:rubyPr/><w:rt><w:r><w:t>RUBY-TEXT</w:t></w:r></w:rt>"#,
            r#"<w:rubyBase><w:r><w:t>RUBY-BASE</w:t></w:r></w:rubyBase></w:ruby></w:r>"#,
            r#"<w:r><w:t>Tail</w:t></w:r></w:p>"#
        ))
        .bytes(),
        "M13" => Package::new(r#"<w:p><w:fldSimple w:instr=" HYPERLINK &quot;https://field.invalid/&quot; "><w:r><w:t>FIELD-RESULT</w:t></w:r></w:fldSimple></w:p>"#).bytes(),
        "M14" => Package::new(r#"<w:p><w:customXml w:uri="urn:example:tags" w:element="secret"><w:r><w:t>CUSTOMXML-WORDING</w:t></w:r></w:customXml></w:p>"#).bytes(),
        "M15" => Package::new(
            r#"<w:p><w:r><w:t>Seen</w:t></w:r><w:unknownWrapper w:val="x"><w:r><w:t>UNKNOWN-WRAPPED</w:t></w:r></w:unknownWrapper></w:p>"#,
        )
        .bytes(),
        "B1" => {
            let mut bytes = Package::new(&p("tiny")).bytes();
            patch_entry(&mut bytes, "word/document.xml", |b, local, central| {
                set32(b, local + 22, B1_DECLARED);
                set32(b, central + 24, B1_DECLARED);
            });
            bytes
        }
        "B2" => {
            let mut package = Package::new(&p("CRC-SENTINEL"));
            package.method = zip::CompressionMethod::Stored;
            let mut bytes = package.bytes();
            let at = find(&bytes, b"CRC-SENTINEL").unwrap();
            bytes[at] = b'X';
            bytes
        }
        "B3" => {
            let mut bytes = Package::new(&p("BACKSLASH-SENTINEL")).bytes();
            rename_entry(&mut bytes, "word/document.xml", "word\\document.xml");
            bytes
        }
        "B4" => Package::new(&p("LOWER-SENTINEL"))
            .part("WORD/document.xml", document(&p("UPPER-SENTINEL")))
            .bytes(),
        "B5" => {
            let mut package = Package::new("");
            package.document = format!(
                r#"<!DOCTYPE w:document [<!ENTITY s "ENTITY-SENTINEL">]><w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t>A&s;B</w:t></w:r></w:p></w:body></w:document>"#
            )
            .into_bytes();
            package.bytes()
        }
        "B6" => {
            let xml = format!(
                r#"<?xml version="1.0" encoding="UTF-16"?><w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t>UTF16-SENTINEL</w:t></w:r></w:p></w:body></w:document>"#
            );
            let mut encoded = vec![0xff, 0xfe];
            for unit in xml.encode_utf16() {
                encoded.extend(unit.to_le_bytes());
            }
            let mut package = Package::new("");
            package.document = encoded;
            package.bytes()
        }
        "B7" => Package::new(&b7_body()).bytes(),
        "I1" => Package::new(&(image_run("imgA", "A") + &image_run("imgB", "B")))
            .rels(&[("imgA", "image", "media/a.png"), ("imgB", "image", "media/b.png")])
            .part("word/media/a.png", png(PNG_A))
            .part("word/media/b.png", png(PNG_B))
            .bytes(),
        "I2" => Package::new(&(image_run("img", "first") + &image_run("img", "second")))
            .rels(&[("img", "image", "media/one.png")])
            .part("word/media/one.png", png(PNG_SHARED))
            .bytes(),
        "I3" => Package::new(&image_run("jpg", "photo"))
            .rels(&[("jpg", "image", "media/photo.jpg")])
            .part("word/media/photo.jpg", JPEG)
            .bytes(),
        "I5" => {
            let body = image_run("rId1", "body")
                + r#"<w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
            Package::new(&body)
                .types(HEADER_CT)
                .rels(&[("rId1", "image", "media/body.png"), ("h1", "header", "header1.xml")])
                .part("word/header1.xml", header(&image_run("rId1", "header")))
                .part(
                    "word/_rels/header1.xml.rels",
                    rels(&[("rId1", "image", "media/header.png")]),
                )
                .part("word/media/body.png", png(PNG_BODY))
                .part("word/media/header.png", png(PNG_HEADER))
                .bytes()
        }
        "I6" => Package::new(&image_run("gone", "MISSING-ALT"))
            .rels(&[("gone", "image", "media/missing.png")])
            .bytes(),
        other => panic!("unknown case {other}"),
    }
}

/// B7 body: inside BRN's XML guard (<= 8 MiB XML, <= 50,000 raw `<`/`=`).
pub fn b7_body() -> String {
    let line = format!("<w:p><w:r><w:t>{}</w:t></w:r></w:p>", "a".repeat(900));
    line.repeat(8_000) + &p("LAST-SENTINEL")
}
pub fn b7_stats() -> (usize, usize) {
    let xml = document(&b7_body());
    let delimiters = xml.bytes().filter(|b| matches!(b, b'<' | b'=')).count();
    (xml.len(), delimiters)
}
