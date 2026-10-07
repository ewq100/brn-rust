use std::time::Instant;
fn picture_titles(xml: &[u8]) -> Vec<String> {
    use quick_xml::{NsReader, events::Event, name::ResolveResult};
    let mut reader = NsReader::from_reader(xml);
    let mut titles = Vec::new();
    loop {
        match reader.read_resolved_event().unwrap() {
            (ResolveResult::Bound(ns), Event::Start(e) | Event::Empty(e))
                if ns.as_ref() == b"http://schemas.openxmlformats.org/presentationml/2006/main"
                    && e.local_name().as_ref() == b"cNvPr" =>
            {
                for attr in e.attributes() {
                    let attr = attr.unwrap();
                    if attr.key.as_ref() == b"title" {
                        titles.push(
                            attr.decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .unwrap()
                            .into_owned(),
                        );
                    }
                }
            }
            (_, Event::Eof) => break,
            _ => (),
        }
    }
    titles
}
fn main() {
    let start = Instant::now();
    let docx = docx_parse::parse_docx_s9_wire(include_bytes!("../sample.docx"), Default::default())
        .unwrap();
    let json = serde_json::to_string(&docx).unwrap();
    assert!(json.contains("data:image/png;base64,"));
    println!(
        "DOCX image data URL exposed: yes; wire bytes: {}; elapsed_us: {}",
        json.len(),
        start.elapsed().as_micros()
    );
    let (gaps, parts) = docx_parse::parse_docx_s9_wire_parts_with_limits(
        include_bytes!("../sample-with-gaps.docx"),
        Default::default(),
        &Default::default(),
    )
    .unwrap();
    let gaps_json = serde_json::to_string(&gaps).unwrap();
    let raw = parts
        .iter()
        .find(|(name, _)| name == "word/document.xml")
        .unwrap();
    let raw = std::str::from_utf8(&raw.1).unwrap();
    for sentinel in [
        "RUBY_GUIDE_SENTINEL",
        "RUBY_BASE_SENTINEL",
        "UNKNOWN_SENTINEL",
    ] {
        assert!(raw.contains(sentinel));
        assert!(!gaps_json.contains(sentinel));
    }
    println!(
        "DOCX H4 ruby/unknown-wrapper content loss independently reproduced; original part remains available: yes"
    );
    let start = Instant::now();
    let pptx = pptx_parse::parse_pptx(include_bytes!("../sample.pptx")).unwrap();
    let json = serde_json::to_string(&pptx).unwrap();
    assert!(json.contains("Slide sentinel"));
    assert!(json.contains("Notes sentinel"));
    assert!(json.contains("Image alt"));
    assert_eq!(
        pptx.media[0].bytes.as_slice(),
        include_bytes!("../image.png")
    );
    println!(
        "PPTX slide text, notes, image alt and exact asset: yes; image title exposed: {}; elapsed_us: {}",
        json.contains("Image title"),
        start.elapsed().as_micros()
    );
    let start = Instant::now();
    let titles = picture_titles(pptx.part_bytes("ppt/slides/slide1.xml").unwrap());
    assert_eq!(titles, ["Image title"]);
    println!(
        "Targeted title supplement from retained PPTX part: yes; elapsed_us: {}",
        start.elapsed().as_micros()
    );
    let start = Instant::now();
    let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let parts = vec![
      ("xl/workbook.xml".into(),format!(r#"<workbook xmlns="{ns}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet sentinel" sheetId="1" r:id="rId1"/></sheets></workbook>"#).into_bytes()),
      ("xl/_rels/workbook.xml.rels".into(),br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.to_vec()),
      ("xl/worksheets/sheet1.xml".into(),format!(r#"<worksheet xmlns="{ns}"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Cell sentinel</t></is></c><c r="B1"><v>42</v></c></row></sheetData></worksheet>"#).into_bytes()),
    ];
    let xlsx = xlsx_parse::parse_workbook(&parts).unwrap();
    let debug = format!("{xlsx:?}");
    assert!(debug.contains("Sheet sentinel"));
    assert!(debug.contains("Cell sentinel"));
    assert!(debug.contains("Number { value: 42.0 }"));
    println!(
        "XLSX sheet and string/number model: yes; elapsed_us: {}",
        start.elapsed().as_micros()
    );
}
