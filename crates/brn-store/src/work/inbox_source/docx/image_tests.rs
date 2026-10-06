use super::tests::{RELS, TYPES, WORD, archive, paragraph};
use super::*;
use std::io::Write;
const DRAWING: &str = r#"<w:drawing><wp:inline><wp:extent cx="9525" cy="9525"/><wp:docPr id="1" name="Picture" descr="A &amp; [B] 日本語" title="T &quot;Q&quot; &lt;tag&gt;"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="Picture"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="image1"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9525" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#;
const IMAGE_RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="image1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/picture.png"/></Relationships>"#;
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32fast::hash(&out[4..]).to_be_bytes());
    out
}
fn png(width: u32, height: u32, depth: u8, color: u8, raw: &[u8]) -> Vec<u8> {
    let mut header = width.to_be_bytes().to_vec();
    header.extend(height.to_be_bytes());
    header.extend([depth, color, 0, 0, 0]);
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(raw).unwrap();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    out.extend(chunk(b"IHDR", &header));
    out.extend(chunk(b"IDAT", &z.finish().unwrap()));
    out.extend(chunk(b"IEND", &[]));
    out
}
fn small_png() -> Vec<u8> {
    png(1, 1, 8, 6, &[0, 255, 0, 128, 255])
}
#[test]
fn valid_consecutive_empty_idat_chunks_preserve_the_complete_inline_png() {
    let valid = small_png();
    let size = u32::from_be_bytes(valid[33..37].try_into().unwrap()) as usize;
    let idat_end = 33 + 12 + size;
    for at in [33, idat_end] {
        let mut image = valid[..at].to_vec();
        image.extend(chunk(b"IDAT", &[]));
        image.extend_from_slice(&valid[at..]);
        assert_eq!(
            image::validate_png(&image, &AtomicBool::new(false)).unwrap(),
            super::super::PngImageFacts {
                width: 1,
                height: 1
            },
        );
        let converted = convert_source(
            &package(
                &run(DRAWING),
                &image,
                IMAGE_RELS,
                zip::CompressionMethod::Stored,
            ),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(converted.visual.unwrap().bytes, image);
    }
}
fn package(body: &str, image: &[u8], rels: &str, method: zip::CompressionMethod) -> Vec<u8> {
    let types = TYPES.replace(
        "</Types>",
        "<Default Extension=\"png\" ContentType=\"image/png\"/></Types>",
    );
    let document = format!(
        r#"<w:document xmlns:w="{WORD}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{body}</w:body></w:document>"#
    );
    archive(
        &[
            ("[Content_Types].xml", types.as_bytes()),
            ("_rels/.rels", RELS.as_bytes()),
            ("word/document.xml", document.as_bytes()),
            ("word/_rels/document.xml.rels", rels.as_bytes()),
            ("word/media/picture.png", image),
        ],
        method,
    )
}
fn run(drawing: &str) -> String {
    format!("<w:p><w:r><w:t>Before õ</w:t>{drawing}<w:t>After</w:t></w:r></w:p>")
}
#[test]
fn stored_deflate_inline_png_preserves_bytes_wording_occurrence_and_raw_metadata() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let bytes = package(
            &(paragraph("First") + &run(DRAWING) + &paragraph("Caption &amp; last")),
            &small_png(),
            IMAGE_RELS,
            method,
        );
        assert!(
            matches!(convert(&bytes,&AtomicBool::new(false)),Err(super::super::InboxProcessOutcome::Failed{code}) if code=="docx_unsupported")
        );
        let converted = convert_source(&bytes, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            converted.format,
            super::super::InboxConversionFormat::DocxInlinePngV1
        );
        let image = converted.visual.unwrap();
        let digest: String = crate::hash(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(image.asset_name, format!("brn-inbox-image-{digest}-1.png"));
        assert_eq!(image.part_name, "word/media/picture.png");
        assert_eq!(image.relationship_id, "image1");
        assert_eq!(image.bytes, small_png());
        assert_eq!(image.sha256, crate::hash(&small_png()));
        assert_eq!(image.byte_len, small_png().len() as u64);
        assert_eq!((image.width, image.height), (1, 1));
        assert_eq!(image.alt_text.as_deref(), Some("A & [B] 日本語"));
        assert_eq!(image.title.as_deref(), Some("T \"Q\" <tag>"));
        let markup = image_markdown(
            &image.asset_name,
            image.alt_text.as_deref(),
            image.title.as_deref(),
        )
        .unwrap();
        assert_eq!(&converted.body[image.image_start..image.image_end], markup);
        assert!(
            converted.body.is_char_boundary(image.image_start)
                && converted.body.is_char_boundary(image.image_end)
        );
        assert_eq!(
            converted.body,
            format!("First\n\nBefore õ{markup}After\n\nCaption &amp; last\n")
        );
        assert!(markup.starts_with("![A &amp; \\[B\\] 日本語](brn-inbox-image-"));
        assert!(markup.ends_with(" \"T &quot;Q&quot; &lt;tag&gt;\")"));
    }
}
#[test]
fn range_is_compositional_through_emphasis_heading_and_table_cells() {
    for body in [
        format!(
            "<w:p><w:pPr><w:outlineLvl w:val=\"2\"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Pre</w:t>{DRAWING}<w:t>Post</w:t></w:r></w:p>"
        ),
        format!(
            "<w:tbl><w:tr><w:tc>{}</w:tc><w:tc>{}{}</w:tc></w:tr></w:tbl>",
            paragraph("Left"),
            paragraph("Earlier"),
            run(DRAWING)
        ),
    ] {
        let bytes = package(
            &body,
            &small_png(),
            IMAGE_RELS,
            zip::CompressionMethod::Stored,
        );
        let converted = convert_source(&bytes, &AtomicBool::new(false)).unwrap();
        let v = converted.visual.unwrap();
        let markup =
            image_markdown(&v.asset_name, v.alt_text.as_deref(), v.title.as_deref()).unwrap();
        assert_eq!(&converted.body[v.image_start..v.image_end], markup);
        assert!(
            converted.body[..v.image_start].contains("Pre")
                || converted.body[..v.image_start].contains("Earlier<br><br>Before õ")
        );
        assert!(
            converted.body[v.image_end..].contains("Post")
                || converted.body[v.image_end..].contains("After")
        );
    }
}
#[test]
fn unsupported_drawings_references_and_effects_refuse_complete_conversion() {
    for drawing in [
        DRAWING.replace("wp:inline", "wp:anchor"),
        DRAWING.repeat(2),
        DRAWING.replace("image1", "missing"),
        DRAWING.replace("<a:stretch>", "<a:srcRect l=\"1\"/><a:stretch>"),
        DRAWING.replace("<a:xfrm>", "<a:xfrm rot=\"1\">"),
        DRAWING.replace("<a:xfrm>", "<a:xfrm rot=\"false\">"),
        DRAWING.replace("<a:xfrm>", "<a:xfrm flipH=\"1\">"),
        DRAWING.replace("<a:off x=\"0\"", "<a:off x=\"1\""),
        DRAWING.replace("<a:fillRect/>", "<a:fillRect l=\"1\"/>"),
        DRAWING.replace(
            "<a:blip r:embed=\"image1\"/>",
            "<a:blip r:embed=\"image1\"><a:alphaModFix amt=\"50000\"/></a:blip>",
        ),
        DRAWING.replace("<pic:spPr>", "<pic:spPr><a:effectLst/>"),
        DRAWING.replace("<a:graphicData uri=", "<a:graphicData other=\"x\" uri="),
        DRAWING.replace("<w:drawing>", "<w:drawing><w:t>Hidden</w:t>"),
    ] {
        assert!(
            convert_source(
                &package(
                    &run(&drawing),
                    &small_png(),
                    IMAGE_RELS,
                    zip::CompressionMethod::Stored
                ),
                &AtomicBool::new(false)
            )
            .is_err(),
            "{drawing}"
        );
    }
    for rels in [IMAGE_RELS.replace("Target=", "TargetMode=\"External\" Target="),IMAGE_RELS.replace("/image\"","/chart\""),IMAGE_RELS.replace("</Relationships>","<Relationship Id=\"image2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/picture.png\"/></Relationships>"),IMAGE_RELS.replace("media/picture.png","media/missing.png")] {
        assert!(convert_source(&package(&run(DRAWING),&small_png(),&rels,zip::CompressionMethod::Stored),&AtomicBool::new(false)).is_err());
    }
    assert!(
        convert_source(
            &package(
                &paragraph("No occurrence"),
                &small_png(),
                IMAGE_RELS,
                zip::CompressionMethod::Stored
            ),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
fn rewrite_idat(bytes: &[u8], f: impl FnOnce(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let size = u32::from_be_bytes(bytes[33..37].try_into().unwrap()) as usize;
    let mut out = bytes[..33].to_vec();
    out.extend(chunk(b"IDAT", &f(&bytes[41..41 + size])));
    out.extend_from_slice(&bytes[45 + size..]);
    out
}
#[test]
fn png_integrity_crc_stream_eof_chunk_order_animation_and_budgets() {
    let valid = small_png();
    assert_eq!(
        image::validate_png(&valid, &AtomicBool::new(false)).unwrap(),
        super::super::PngImageFacts {
            width: 1,
            height: 1
        }
    );
    let mut bad_crc = valid.clone();
    bad_crc[29] ^= 1;
    let mut trailing = valid.clone();
    trailing.push(0);
    let mut apng = valid[..33].to_vec();
    apng.extend(chunk(b"acTL", &[0; 8]));
    apng.extend_from_slice(&valid[33..]);
    let mut bad_reserved = valid[..33].to_vec();
    bad_reserved.extend(chunk(b"text", b"x"));
    bad_reserved.extend_from_slice(&valid[33..]);
    let mut split = valid[..33].to_vec();
    split.extend(chunk(b"IDAT", b""));
    split.extend(chunk(b"tEXt", b"k\0v"));
    split.extend_from_slice(&valid[33..]);
    for bad in [
        bad_crc,
        trailing,
        apng,
        bad_reserved,
        split,
        valid[..valid.len() - 1].to_vec(),
        valid[..valid.len() - 12].to_vec(),
        rewrite_idat(&valid, |v| v[..v.len() - 1].to_vec()),
        rewrite_idat(&valid, |v| [v, b"junk"].concat()),
        rewrite_idat(&valid, |v| {
            let mut v = v.to_vec();
            let end = v.len() - 1;
            v[end] ^= 1;
            v
        }),
        png(1, 1, 8, 6, &[0, 1, 2, 3]),
        png(1, 1, 8, 6, &[0, 1, 2, 3, 4, 5]),
        png(1, 1, 8, 6, &[7, 1, 2, 3, 4]),
    ] {
        assert!(image::validate_png(&bad, &AtomicBool::new(false)).is_err());
    }
    for bad in [
        png(4097, 1, 8, 6, &[]),
        png(4096, 1025, 8, 6, &[]),
        vec![0; 1024 * 1024 + 1],
    ] {
        assert_eq!(
            image::validate_png(&bad, &AtomicBool::new(false)),
            Err(Failure::Limit)
        );
    }
    // The pinned decoder's charged metadata budget is tested with an ICC profile
    // that fits the encoded limit but expands beyond8MiB before any pixels.
    let mut compressor =
        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    compressor.write_all(&vec![0; 8 * 1024 * 1024 + 1]).unwrap();
    let mut iccp = b"profile\0\0".to_vec();
    iccp.extend(compressor.finish().unwrap());
    let mut bad = valid[..33].to_vec();
    bad.extend(chunk(b"iCCP", &iccp));
    bad.extend_from_slice(&valid[33..]);
    assert_eq!(
        image::validate_png(&bad, &AtomicBool::new(false)),
        Err(Failure::Limit)
    );
    assert_eq!(
        image::validate_png(&valid, &AtomicBool::new(true)),
        Err(Failure::Cancelled)
    );
    assert_eq!(
        convert_source(
            &package(
                &run(DRAWING),
                &valid,
                IMAGE_RELS,
                zip::CompressionMethod::Stored
            ),
            &AtomicBool::new(true)
        ),
        Err(super::super::InboxProcessOutcome::Cancelled)
    );
}
#[test]
fn text_profile_bytes_and_literal_markup_guards_stay_exact() {
    let bytes = super::tests::docx(
        &paragraph("Text õ &amp; literal"),
        zip::CompressionMethod::Stored,
    );
    let legacy = convert(&bytes, &AtomicBool::new(false)).unwrap();
    let next = convert_source(&bytes, &AtomicBool::new(false)).unwrap();
    assert_eq!((next.format, next.body), legacy);
    assert!(next.visual.is_none());
    let name = format!("brn-inbox-image-{}-1.png", "a".repeat(64));
    assert!(image_markdown(&name, Some(&"a".repeat(2049)), None).is_err());
    for bad in [
        "../picture.png".to_string(),
        name.to_uppercase(),
        format!("brn-inbox-image-{}-1.png", "g".repeat(64)),
    ] {
        assert!(image_markdown(&bad, None, None).is_err());
    }
    assert_eq!(
        image_markdown(&name, None, Some("")).unwrap(),
        format!("![]({name} \"\")")
    );
}
fn repack(
    bytes: &[u8],
    modify: impl FnOnce(&mut std::collections::BTreeMap<String, Vec<u8>>),
) -> Vec<u8> {
    let mut parts = package::load(bytes, &AtomicBool::new(false)).unwrap();
    modify(&mut parts);
    let parts: Vec<_> = parts
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    archive(&parts, zip::CompressionMethod::Stored)
}
#[test]
fn image_in_nested_list_keeps_full_ordinary_list_and_caption_order() {
    let numbered = |level, text: &str| {
        format!(
            "<w:p><w:pPr><w:numPr><w:ilvl w:val=\"{level}\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r>{text}</w:r></w:p>"
        )
    };
    let body = numbered(0, "<w:t>Parent</w:t>")
        + &numbered(1, &format!("<w:t>Before</w:t>{DRAWING}<w:t>After</w:t>"))
        + &paragraph("Caption");
    let bytes = package(
        &body,
        &small_png(),
        IMAGE_RELS,
        zip::CompressionMethod::Deflated,
    );
    let bytes = repack(&bytes, |parts| {
        let types=String::from_utf8(parts["[Content_Types].xml"].clone()).unwrap().replace("</Types>","<Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/></Types>");
        parts.insert("[Content_Types].xml".into(), types.into_bytes());
        let rels=IMAGE_RELS.replace("</Relationships>","<Relationship Id=\"num\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/></Relationships>");
        parts.insert("word/_rels/document.xml.rels".into(), rels.into_bytes());
        let num = format!(
            r#"<w:numbering xmlns:w="{WORD}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
        );
        parts.insert("word/numbering.xml".into(), num.into_bytes());
    });
    let converted = convert_source(&bytes, &AtomicBool::new(false)).unwrap();
    let v = converted.visual.unwrap();
    let markup = image_markdown(&v.asset_name, v.alt_text.as_deref(), v.title.as_deref()).unwrap();
    assert_eq!(
        converted.body,
        format!("1. Parent\n\n    - Before{markup}After\n\nCaption\n")
    );
    assert_eq!(&converted.body[v.image_start..v.image_end], markup);
}
#[test]
fn extra_unreferenced_image_wrong_mime_and_alias_relationships_refuse() {
    let original = package(
        &run(DRAWING),
        &small_png(),
        IMAGE_RELS,
        zip::CompressionMethod::Stored,
    );
    for bytes in [
        repack(&original, |p| {
            p.insert("word/media/extra.png".into(), small_png());
        }),
        repack(&original, |p| {
            let t = String::from_utf8(p["[Content_Types].xml"].clone())
                .unwrap()
                .replace("image/png", "image/jpeg");
            p.insert("[Content_Types].xml".into(), t.into_bytes());
        }),
        repack(&original, |p| {
            let r=IMAGE_RELS.replace("</Relationships>","<Relationship Id=\"image1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/picture.png\"/></Relationships>");
            p.insert("word/_rels/document.xml.rels".into(), r.into_bytes());
        }),
    ] {
        assert!(convert_source(&bytes, &AtomicBool::new(false)).is_err());
    }
    let invalid = package(
        &run(&DRAWING.replace("<a:ext cx=\"9525\"", "<a:ext cx=\"19525\"")),
        &small_png(),
        IMAGE_RELS,
        zip::CompressionMethod::Stored,
    );
    assert!(convert_source(&invalid, &AtomicBool::new(false)).is_err());
}
#[test]
fn compressed_ancillary_crc_zlib_and_stream_consumption_are_required() {
    let valid = small_png();
    for kind in [*b"iCCP", *b"zTXt"] {
        for data in [
            b"profile\0\0broken".to_vec(),
            b"profile\0\x01invalid".to_vec(),
        ] {
            let mut bad = valid[..33].to_vec();
            bad.extend(chunk(&kind, &data));
            bad.extend_from_slice(&valid[33..]);
            assert!(image::validate_png(&bad, &AtomicBool::new(false)).is_err());
        }
    }
    let size = u32::from_be_bytes(valid[33..37].try_into().unwrap()) as usize;
    let mut data = b"profile\0\0".to_vec();
    data.extend_from_slice(&valid[41..41 + size]);
    data.extend_from_slice(b"junk");
    let mut bad = valid[..33].to_vec();
    bad.extend(chunk(b"iCCP", &data));
    bad.extend_from_slice(&valid[33..]);
    assert!(image::validate_png(&bad, &AtomicBool::new(false)).is_err());
    let mut bad = valid[..33].to_vec();
    bad.extend(chunk(b"ABCD", b""));
    bad.extend_from_slice(&valid[33..]);
    assert_eq!(
        image::validate_png(&bad, &AtomicBool::new(false)),
        Err(Failure::Unsupported)
    );
}
#[test]
fn valid_packed_palette_adam7_and_maximum_pixel_16bit_rows_decode_completely() {
    let indexed = png(1, 1, 1, 3, &[0, 0]);
    let mut valid = indexed[..33].to_vec();
    valid.extend(chunk(b"PLTE", &[255, 0, 0]));
    valid.extend_from_slice(&indexed[33..]);
    assert!(image::validate_png(&valid, &AtomicBool::new(false)).is_ok());
    let bad = png(1, 1, 1, 3, &[0, 128]);
    let mut invalid = bad[..33].to_vec();
    invalid.extend(chunk(b"PLTE", &[255, 0, 0]));
    invalid.extend_from_slice(&bad[33..]);
    assert_eq!(
        image::validate_png(&invalid, &AtomicBool::new(false)),
        Err(Failure::Invalid)
    );
    let small = small_png();
    let mut ihdr = small[16..29].to_vec();
    ihdr[12] = 1;
    let mut interlaced = small[..8].to_vec();
    interlaced.extend(chunk(b"IHDR", &ihdr));
    interlaced.extend_from_slice(&small[33..]);
    assert!(image::validate_png(&interlaced, &AtomicBool::new(false)).is_ok());
    let mut header = 2048u32.to_be_bytes().to_vec();
    header.extend(2048u32.to_be_bytes());
    header.extend([16, 6, 0, 0, 0]);
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let row = vec![0; 2048 * 8 + 1];
    for _ in 0..2048 {
        z.write_all(&row).unwrap();
    }
    let mut maximum = b"\x89PNG\r\n\x1a\n".to_vec();
    maximum.extend(chunk(b"IHDR", &header));
    maximum.extend(chunk(b"IDAT", &z.finish().unwrap()));
    maximum.extend(chunk(b"IEND", &[]));
    assert_eq!(
        image::validate_png(&maximum, &AtomicBool::new(false)).unwrap(),
        super::super::PngImageFacts {
            width: 2048,
            height: 2048
        }
    );
    let raw = png(4096, 1, 1, 0, &vec![0; 513]);
    assert!(image::validate_png(&raw, &AtomicBool::new(false)).is_ok());
}
#[test]
fn malformed_auxiliary_palette_lengths_values_order_and_itxt_are_refused() {
    let base = small_png();
    for (kind, data) in [
        (*b"PLTE", vec![1, 2]),
        (*b"gAMA", vec![0; 4]),
        (*b"sRGB", vec![4]),
        (*b"pHYs", vec![0; 8]),
        (*b"sBIT", vec![9; 4]),
        (*b"tIME", vec![0; 7]),
        (*b"tEXt", b"missing-separator".to_vec()),
        (*b"tRNS", vec![0; 6]),
    ] {
        let mut bad = base[..33].to_vec();
        bad.extend(chunk(&kind, &data));
        bad.extend_from_slice(&base[33..]);
        assert!(
            image::validate_png(&bad, &AtomicBool::new(false)).is_err(),
            "{kind:?}"
        );
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&[0xff]).unwrap();
    let mut data = b"k\0\x01\0\0\0".to_vec();
    data.extend(z.finish().unwrap());
    let mut bad = base[..33].to_vec();
    bad.extend(chunk(b"iTXt", &data));
    bad.extend_from_slice(&base[33..]);
    assert!(image::validate_png(&bad, &AtomicBool::new(false)).is_err());
}
#[test]
fn image_title_table_delimiters_are_literal_without_changing_the_raw_title() {
    let drawing = DRAWING.replace(
        "T &quot;Q&quot; &lt;tag&gt;",
        "Left|Right &#10; [literal](fake.png)",
    );
    let body = format!(
        "<w:tbl><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr></w:tbl>",
        paragraph("Left"),
        run(&drawing)
    );
    let converted = convert_source(
        &package(
            &body,
            &small_png(),
            IMAGE_RELS,
            zip::CompressionMethod::Stored,
        ),
        &AtomicBool::new(false),
    )
    .unwrap();
    let v = converted.visual.unwrap();
    assert_eq!(
        v.title.as_deref(),
        Some("Left|Right \n [literal](fake.png)")
    );
    let markup = image_markdown(&v.asset_name, v.alt_text.as_deref(), v.title.as_deref()).unwrap();
    assert_eq!(&converted.body[v.image_start..v.image_end], markup);
    assert!(markup.ends_with(" \"Left&#124;Right &#10; [literal](fake.png)\")"));
    assert_eq!(
        converted.body.lines().nth(2).unwrap().matches('|').count(),
        3
    );
}
