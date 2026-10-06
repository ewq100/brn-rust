use super::*;
use std::io::{Cursor, Write};

pub(super) const TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
pub(super) const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
pub(super) const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

pub(super) fn archive(parts: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for &(name, bytes) in parts {
        writer
            .start_file(
                name,
                zip::write::SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
pub(super) fn docx(body: &str, method: zip::CompressionMethod) -> Vec<u8> {
    let doc = format!("<w:document xmlns:w=\"{WORD}\"><w:body>{body}</w:body></w:document>");
    archive(
        &[
            ("[Content_Types].xml", TYPES.as_bytes()),
            ("_rels/.rels", RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
        ],
        method,
    )
}
pub(super) fn paragraph(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}
fn failed(bytes: &[u8], code: &str) {
    assert_eq!(
        convert(bytes, &AtomicBool::new(false)),
        Err(super::super::InboxProcessOutcome::Failed { code: code.into() })
    );
}
fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) as usize
}
fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
fn set16(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes());
}
fn set32(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}
fn central(bytes: &[u8]) -> usize {
    u32_at(bytes, bytes.len() - 22 + 16)
}

#[test]
fn genuine_stored_and_deflate_unicode_paragraphs_ignore_filename_claims() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let bytes = docx(
            &(paragraph("First õ 日本語") + &paragraph("Second preserved")),
            method,
        );
        assert_eq!(
            convert(&bytes, &AtomicBool::new(false)).unwrap(),
            (
                super::super::InboxConversionFormat::DocxTextV1,
                "First õ 日本語\n\nSecond preserved\n".into()
            )
        );
        assert_eq!(
            convert(&docx("", method), &AtomicBool::new(false))
                .unwrap()
                .1,
            ""
        );
    }
    failed(b"%PDF-1.7 opaque input", "binary_unsupported");
    failed(b"PK\x03\x04truncated", "docx_invalid");
    failed(
        &archive(
            &[("word/document.xml", b"not an OPC document")],
            zip::CompressionMethod::Stored,
        ),
        "docx_invalid",
    );
    assert_eq!(
        convert(b"anything", &AtomicBool::new(true)),
        Err(super::super::InboxProcessOutcome::Cancelled)
    );
}

#[test]
fn complete_opc_document_preserves_bound_styles_numbering_links_and_rectangular_table() {
    let types = TYPES.replace("</Types>",concat!(
        "<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>",
        "<Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/></Types>"));
    let doc = format!(
        "<w:document xmlns:w=\"{WORD}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{}</w:body></w:document>",
        concat!(
            "<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Heading õ</w:t></w:r></w:p>",
            "<w:p><w:pPr><w:numPr><w:numId w:val=\"7\"/></w:numPr></w:pPr><w:r><w:t>First</w:t></w:r></w:p>",
            "<w:p><w:pPr><w:numPr><w:numId w:val=\"7\"/></w:numPr></w:pPr><w:hyperlink r:id=\"link\"><w:r><w:t>Reference</w:t></w:r></w:hyperlink></w:p>",
            "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/></w:tblPr><w:tr><w:tc><w:p><w:r><w:t>left</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>right</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"
        )
    );
    let styles = format!(
        "<w:styles xmlns:w=\"{WORD}\"><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:pPr><w:outlineLvl w:val=\"0\"/></w:pPr><w:rPr><w:b/></w:rPr></w:style><w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:tblPr><w:tblBorders><w:top w:val=\"single\"/></w:tblBorders></w:tblPr></w:style></w:styles>"
    );
    let numbering = format!(
        "<w:numbering xmlns:w=\"{WORD}\"><w:abstractNum w:abstractNumId=\"0\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1)\"/></w:lvl></w:abstractNum><w:num w:numId=\"7\"><w:abstractNumId w:val=\"0\"/><w:lvlOverride w:ilvl=\"0\"><w:startOverride w:val=\"4\"/></w:lvlOverride></w:num></w:numbering>"
    );
    let rels = concat!(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
        "<Relationship Id=\"styles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
        "<Relationship Id=\"numbering\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>",
        "<Relationship Id=\"link\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" TargetMode=\"External\" Target=\"https://example.invalid/a?q=one&amp;b=two\"/></Relationships>"
    );
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let bytes = archive(
            &[
                ("[Content_Types].xml", types.as_bytes()),
                ("_rels/.rels", RELS.as_bytes()),
                ("word/document.xml", doc.as_bytes()),
                ("word/styles.xml", styles.as_bytes()),
                ("word/numbering.xml", numbering.as_bytes()),
                ("word/_rels/document.xml.rels", rels.as_bytes()),
            ],
            method,
        );
        assert_eq!(
            convert(&bytes, &AtomicBool::new(false)).unwrap().1,
            concat!(
                "# <strong>Heading õ</strong>\n\n4) First\n\n5) [Reference](<https://example.invalid/a?q=one&amp;b=two>)\n\n",
                "|  |  |\n| --- | --- |\n| left | right |\n"
            )
        );
        let invalid_rels = rels.replace(
            "https://example.invalid/a?q=one&amp;b=two",
            "file:///private/never-read",
        );
        let invalid = archive(
            &[
                ("[Content_Types].xml", types.as_bytes()),
                ("_rels/.rels", RELS.as_bytes()),
                ("word/document.xml", doc.as_bytes()),
                ("word/styles.xml", styles.as_bytes()),
                ("word/numbering.xml", numbering.as_bytes()),
                ("word/_rels/document.xml.rels", invalid_rels.as_bytes()),
            ],
            method,
        );
        failed(&invalid, "docx_unsupported");
    }
}

#[test]
fn footer_inventory_names_and_local_metadata_refuse_ambiguous_interpretations() {
    let original = archive(
        &[("a.xml", b"a"), ("b.xml", b"b")],
        zip::CompressionMethod::Stored,
    );
    let c1 = central(&original);
    let c2 = c1
        + 46
        + u16_at(&original, c1 + 28)
        + u16_at(&original, c1 + 30)
        + u16_at(&original, c1 + 32);
    let l2 = u32_at(&original, c2 + 42);
    let mut duplicate = original.clone();
    duplicate[c2 + 46..c2 + 51].copy_from_slice(b"a.xml");
    duplicate[l2 + 30..l2 + 35].copy_from_slice(b"a.xml");
    assert_eq!(
        package::load(&duplicate, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    let case_alias = archive(
        &[("a.xml", b"a"), ("A.xml", b"b")],
        zip::CompressionMethod::Stored,
    );
    assert_eq!(
        package::load(&case_alias, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    for name in [
        "../a.xml",
        "/a.xml",
        "word//a.xml",
        "word//",
        "word/%61.xml",
        "word\\a.xml",
        "word/é.xml",
    ] {
        let bytes = archive(&[(name, b"a")], zip::CompressionMethod::Stored);
        assert_eq!(
            package::load(&bytes, &AtomicBool::new(false)).unwrap_err(),
            Failure::Invalid,
            "{name}"
        );
    }
    let mut mismatched = original.clone();
    mismatched[30] = b'z';
    assert_eq!(
        package::load(&mismatched, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    let mut overlap = original.clone();
    set32(&mut overlap, c2 + 42, 0);
    assert_eq!(
        package::load(&overlap, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    let mut exaggerated = original.clone();
    let footer = exaggerated.len() - 22;
    set16(&mut exaggerated, footer + 8, 60_000);
    set16(&mut exaggerated, footer + 10, 60_000);
    assert_eq!(
        package::load(&exaggerated, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    let mut trailing = original.clone();
    trailing.push(0);
    assert_eq!(
        package::load(&trailing, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
    let mut offset = original.clone();
    set32(&mut offset, footer + 16, u32::MAX as usize - 1);
    assert_eq!(
        package::load(&offset, &AtomicBool::new(false)).unwrap_err(),
        Failure::Invalid
    );
}

#[test]
fn complete_crc_and_compressed_stream_eof_are_required() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let original = archive(&[("a.xml", &vec![b'a'; 70_000])], method);
        assert_eq!(
            package::load(&original, &AtomicBool::new(false)).unwrap()["a.xml"].len(),
            70_000
        );
        let c = central(&original);
        let mut bad_crc = original.clone();
        set32(&mut bad_crc, 14, 1);
        set32(&mut bad_crc, c + 16, 1);
        assert_eq!(
            package::load(&bad_crc, &AtomicBool::new(false)).unwrap_err(),
            Failure::Invalid
        );
        let mut bad_size = original.clone();
        set32(&mut bad_size, 22, 1);
        set32(&mut bad_size, c + 24, 1);
        assert_eq!(
            package::load(&bad_size, &AtomicBool::new(false)).unwrap_err(),
            Failure::Invalid
        );
        if method == zip::CompressionMethod::Deflated {
            let mut trailing = original.clone();
            let compressed = u32_at(&trailing, 18);
            trailing.insert(c, 0xff);
            set32(&mut trailing, 18, compressed + 1);
            set32(&mut trailing, c + 1 + 20, compressed + 1);
            let footer = trailing.len() - 22;
            set32(&mut trailing, footer + 16, c + 1);
            // Decoder EOF and CRC alone accept this; complete raw stream
            // consumption must independently reject the extra compressed byte.
            assert_eq!(
                package::load(&trailing, &AtomicBool::new(false)).unwrap_err(),
                Failure::Invalid
            );
        }
    }
}

#[test]
fn signed_and_unsigned_descriptors_must_match_and_consume_the_complete_range() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        for signed in [false, true] {
            let original = archive(&[("a.xml", b"complete descriptor")], method);
            let c = central(&original);
            let mut bytes = original.clone();
            let mut descriptor = Vec::new();
            if signed {
                descriptor.extend_from_slice(&0x08074b50u32.to_le_bytes());
            }
            descriptor.extend_from_slice(&original[c + 16..c + 28]);
            bytes.splice(c..c, descriptor.iter().copied());
            set16(&mut bytes, 6, 8);
            set16(&mut bytes, c + descriptor.len() + 8, 8);
            set32(&mut bytes, 14, 0);
            set32(&mut bytes, 18, 0);
            set32(&mut bytes, 22, 0);
            let footer = bytes.len() - 22;
            set32(&mut bytes, footer + 16, c + descriptor.len());
            assert_eq!(
                package::load(&bytes, &AtomicBool::new(false)).unwrap()["a.xml"],
                b"complete descriptor"
            );
            let mut wrong = bytes.clone();
            wrong[c + usize::from(signed) * 4] ^= 1;
            assert_eq!(
                package::load(&wrong, &AtomicBool::new(false)).unwrap_err(),
                Failure::Invalid
            );
            let mut orphan = bytes.clone();
            orphan.insert(c + descriptor.len(), 0);
            let footer = orphan.len() - 22;
            set32(&mut orphan, footer + 16, c + descriptor.len() + 1);
            assert_eq!(
                package::load(&orphan, &AtomicBool::new(false)).unwrap_err(),
                Failure::Invalid
            );
        }
    }
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_directory("word/", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .start_file(
            "word/a.xml",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(b"a").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let parts = package::load(&bytes, &AtomicBool::new(false)).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts["word/a.xml"], b"a");
}

#[test]
fn encryption_methods_zip64_and_unicode_name_extras_are_explicitly_unsupported() {
    let original = archive(&[("a.xml", b"a")], zip::CompressionMethod::Stored);
    let c = central(&original);
    for (local, central_offset, value) in [(6, c + 8, 1), (8, c + 10, 12)] {
        let mut bytes = original.clone();
        set16(&mut bytes, local, value);
        set16(&mut bytes, central_offset, value);
        assert_eq!(
            package::load(&bytes, &AtomicBool::new(false)).unwrap_err(),
            Failure::Unsupported
        );
    }
    let mut zip64 = original.clone();
    let footer = zip64.len() - 22;
    set16(&mut zip64, footer + 10, u16::MAX as usize);
    assert_eq!(
        package::load(&zip64, &AtomicBool::new(false)).unwrap_err(),
        Failure::Unsupported
    );
    // Add a well-formed central Unicode Path extra. Literal aliases are refused
    // before the library can replace both effective name and name_raw.
    let mut alias = original.clone();
    let end = c + 46 + u16_at(&alias, c + 28);
    alias.splice(end..end, [0x75, 0x70, 0, 0]);
    set16(&mut alias, c + 30, 4);
    let footer = alias.len() - 22;
    let size = u32_at(&alias, footer + 12);
    set32(&mut alias, footer + 12, size + 4);
    assert_eq!(
        package::load(&alias, &AtomicBool::new(false)).unwrap_err(),
        Failure::Unsupported
    );
}

#[test]
fn entry_expansion_and_xml_allocation_limits_are_independent_of_claimed_size() {
    let names: Vec<_> = (0..257).map(|i| format!("{i}.xml")).collect();
    let parts: Vec<_> = names
        .iter()
        .map(|n| (n.as_str(), b"a".as_slice()))
        .collect();
    assert_eq!(
        package::load(
            &archive(&parts, zip::CompressionMethod::Stored),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        Failure::Limit
    );
    let mut oversized = archive(&[("a.xml", b"a")], zip::CompressionMethod::Deflated);
    let c = central(&oversized);
    set32(&mut oversized, 22, MAX_PART + 1);
    set32(&mut oversized, c + 24, MAX_PART + 1);
    assert_eq!(
        package::load(&oversized, &AtomicBool::new(false)).unwrap_err(),
        Failure::Limit
    );
    let mut aggregate = archive(
        &[
            ("a.xml", b"a"),
            ("b.xml", b"b"),
            ("c.xml", b"c"),
            ("d.xml", b"d"),
            ("e.xml", b"e"),
        ],
        zip::CompressionMethod::Deflated,
    );
    let mut at = central(&aggregate);
    for _ in 0..5 {
        let local = u32_at(&aggregate, at + 42);
        set32(&mut aggregate, local + 22, MAX_PART);
        set32(&mut aggregate, at + 24, MAX_PART);
        at += 46
            + u16_at(&aggregate, at + 28)
            + u16_at(&aggregate, at + 30)
            + u16_at(&aggregate, at + 32);
    }
    assert_eq!(
        package::load(&aggregate, &AtomicBool::new(false)).unwrap_err(),
        Failure::Limit
    );
    let delimiter_reserve = format!("<r><![CDATA[{}]]></r>", "=".repeat(MAX_XML_ITEMS + 1));
    assert!(matches!(
        xml::Budget::default().parse(delimiter_reserve.as_bytes(), &AtomicBool::new(false)),
        Err(Failure::Limit)
    ));
    let attributes = format!(
        "<r {}/>",
        (0..65)
            .map(|i| format!("a{i}=\"v\""))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(matches!(
        xml::Budget::default().parse(attributes.as_bytes(), &AtomicBool::new(false)),
        Err(Failure::Limit)
    ));
    let depth = format!("{}{}", "<r>".repeat(65), "</r>".repeat(65));
    assert!(matches!(
        xml::Budget::default().parse(depth.as_bytes(), &AtomicBool::new(false)),
        Err(Failure::Limit)
    ));
    let too_many = format!("<r>{}</r>", "<a/>".repeat(MAX_XML_ITEMS));
    assert!(matches!(
        xml::Budget::default().parse(too_many.as_bytes(), &AtomicBool::new(false)),
        Err(Failure::Limit)
    ));
    assert!(matches!(
        xml::Budget::default().parse(&vec![b' '; MAX_XML + 1], &AtomicBool::new(false)),
        Err(Failure::Limit)
    ));
}

#[test]
fn xml_entity_dtd_duplicate_attribute_and_encoding_refusals_preserve_no_success() {
    for input in [
        "<r>&unknown;</r>",
        "<r a='1' a='2'/>",
        "<r><a></r>",
        "<r/><another/>",
    ] {
        assert!(
            matches!(
                xml::Budget::default().parse(input.as_bytes(), &AtomicBool::new(false)),
                Err(Failure::Invalid)
            ),
            "{input}"
        );
    }
    for input in [
        "<!DOCTYPE r><r/>",
        "<!DOCTYPE r [<!ENTITY x SYSTEM 'https://example.invalid/private'>]><r>&x;</r>",
        "<?xml version='1.0' encoding='UTF-16'?><r/>",
    ] {
        assert!(
            matches!(
                xml::Budget::default().parse(input.as_bytes(), &AtomicBool::new(false)),
                Err(Failure::Unsupported)
            ),
            "{input}"
        );
    }
    let docx_dtd = docx("<!DOCTYPE p><w:p/>", zip::CompressionMethod::Stored);
    failed(&docx_dtd, "docx_unsupported");
}

#[test]
fn meaningful_unknown_package_parts_and_false_relationship_roles_are_not_dropped() {
    let doc = format!(
        "<w:document xmlns:w=\"{WORD}\"><w:body>{}</w:body></w:document>",
        paragraph("preserved")
    );
    let image_types = TYPES.replace(
        "</Types>",
        "<Default Extension=\"png\" ContentType=\"image/png\"/></Types>",
    );
    let image = archive(
        &[
            ("[Content_Types].xml", image_types.as_bytes()),
            ("_rels/.rels", RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/media/image.png", b"synthetic image bytes"),
        ],
        zip::CompressionMethod::Stored,
    );
    failed(&image, "docx_unsupported");
    for target in [
        "../word/document.xml",
        "word/%64ocument.xml",
        "https://example.invalid/document.xml",
    ] {
        let rels = RELS.replace("word/document.xml", target);
        let bytes = archive(
            &[
                ("[Content_Types].xml", TYPES.as_bytes()),
                ("_rels/.rels", rels.as_bytes()),
                ("word/document.xml", doc.as_bytes()),
            ],
            zip::CompressionMethod::Stored,
        );
        failed(&bytes, "docx_invalid");
    }
    let extra=RELS.replace("</Relationships>","<Relationship Id=\"other\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"word/document.xml\"/></Relationships>");
    let disguised = archive(
        &[
            ("[Content_Types].xml", TYPES.as_bytes()),
            ("_rels/.rels", extra.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
        ],
        zip::CompressionMethod::Stored,
    );
    failed(&disguised, "docx_unsupported");
    let duplicate = RELS.replace(
        "</Relationships>",
        &format!(
            "{} </Relationships>",
            &RELS[RELS.find("<Relationship Id").unwrap()..RELS.find("</Relationships>").unwrap()]
        ),
    );
    let ambiguous = archive(
        &[
            ("[Content_Types].xml", TYPES.as_bytes()),
            ("_rels/.rels", duplicate.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
        ],
        zip::CompressionMethod::Stored,
    );
    failed(&ambiguous, "docx_invalid");
}
