use brn_intake::*;
fn fixture() -> Extraction {
    let bytes = b"synthetic original".to_vec();
    Extraction {
        limits: Default::default(),
        consumed: None,
        schema: 1,
        converter: CONVERTER.into(),
        original_sha256: digest(&bytes),
        markdown: "actual words".into(),
        sources: vec![SourceNode {
            id: "source-1".into(),
            parent: None,
            name: "mail.eml".into(),
            media_type: "message/rfc822".into(),
            locator: "original".into(),
            status: "complete".into(),
            bytes,
            text: "actual words".into(),
        }],
        assets: vec![],
        occurrences: vec![],
        gaps: vec![],
    }
}
#[test]
fn exact_retained_bytes_round_trip_and_root_integrity() {
    let extraction = fixture();
    extraction.validate().unwrap();
    let encoded = serde_json::to_vec(&extraction).unwrap();
    let decoded: Extraction = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, extraction);
    let mut forged = extraction;
    forged.sources[0].bytes.push(0);
    assert!(forged.validate().unwrap_err().contains("root original"));
}
#[test]
fn refuses_cyclic_sources_and_factual_unsupported_content() {
    let mut extraction = fixture();
    let mut child = extraction.sources[0].clone();
    child.id = "child".into();
    child.parent = Some("child".into());
    extraction.sources.push(child);
    assert!(extraction.validate().is_err());
    extraction.sources.pop();
    extraction.sources[0].status = "unprocessed".into();
    assert!(extraction.validate().unwrap_err().contains("factual"));
}
#[test]
fn rejects_hash_aliases_utf8_splits_and_duplicate_occurrence_locators() {
    let mut extraction = fixture();
    let bytes = b"image bytes validated in helper".to_vec();
    let sha256 = digest(&bytes);
    let asset = ImageAsset {
        id: format!("asset-{}", hex(&sha256)),
        sha256,
        width: 1,
        height: 1,
        media_type: "image/png".into(),
        bytes,
    };
    let link = format!("![image](assets/{})", asset_file_name(&asset).unwrap());
    extraction.markdown = format!("é{link}\n{link}");
    extraction.sources[0].text = extraction.markdown.clone();
    extraction.assets.push(asset.clone());
    extraction.occurrences.push(ImageOccurrence {
        id: "o1".into(),
        source_id: "source-1".into(),
        asset_id: asset.id.clone(),
        locator: "part/image/1".into(),
        alt: None,
        start: 2,
        end: 2 + link.len(),
    });
    extraction.validate().unwrap();
    extraction.occurrences[0].start = 1;
    assert!(extraction.validate().is_err());
    extraction.occurrences[0].start = 2;
    let mut second = extraction.occurrences[0].clone();
    second.id = "o2".into();
    second.start = 3 + link.len();
    second.end = extraction.markdown.len();
    extraction.occurrences.push(second);
    assert!(extraction.validate().is_err());
    extraction.occurrences[1].locator = "part/image/2".into();
    extraction.validate().unwrap();
    extraction.assets[0].bytes.push(0);
    assert!(extraction.validate().is_err());
}

#[test]
fn maintained_png_decoder_requires_complete_terminal_chunks_and_checksums() {
    let image = include_bytes!("fixtures/water-use.png");
    let dimensions = validate_png_image(image).unwrap();
    assert!(dimensions.0 > 0 && dimensions.1 > 0);
    assert!(validate_png_image(&image[..image.len() - 1]).is_err());
    assert!(validate_png_image(&image[..image.len() / 2]).is_err());
    let mut damaged = image.to_vec();
    damaged[40] ^= 1;
    assert!(validate_png_image(&damaged).is_err());
    let mut trailing = image.to_vec();
    trailing.push(0);
    assert!(validate_png_image(&trailing).is_err());
}

#[test]
fn retained_old_email_caveat_and_partial_snapshot_remain_exact_and_readable() {
    // This synthetic historical snapshot is independent of the current helper.
    // Reading it must neither rerun conversion nor replace its old diagnostic.
    const OLD_GAP: &str = "source-0: authentication and absent message/thread identifiers remain unknown; HTML is inert quoted source, remote resources unavailable";
    let bytes = b"Message-ID: <historical@archive.example.test>\r\nContent-Type: text/plain\r\n\r\nExact retained wording.\r\n".to_vec();
    let text = "\nMessage-ID\n\n```\nhistorical@archive.example.test\n```\n\nActual plain-text body\n\n```\nExact retained wording.\r\n```\n".to_owned();
    let historical = Extraction {
        limits: Default::default(),
        consumed: None,
        schema: 1,
        converter: "brn-intake-v1/betteroffice-0.3.0/mail-parser-0.11.8".into(),
        original_sha256: digest(&bytes),
        markdown: text.clone(),
        sources: vec![SourceNode {
            id: "source-0".into(),
            parent: None,
            name: "original.eml".into(),
            media_type: "message/rfc822".into(),
            locator: "original".into(),
            status: "partial".into(),
            bytes,
            text,
        }],
        assets: vec![],
        occurrences: vec![],
        gaps: vec![OLD_GAP.into()],
    };
    let stored_bytes = serde_json::to_vec(&historical).unwrap();
    let stored_hash = digest(&stored_bytes);
    let decoded: Extraction = serde_json::from_slice(&stored_bytes).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, historical);
    assert_eq!(decoded.gaps, vec![OLD_GAP]);
    assert_eq!(decoded.sources[0].status, "partial");
    let read_back = serde_json::to_vec(&decoded).unwrap();
    assert_eq!(read_back, stored_bytes);
    assert_eq!(digest(&read_back), stored_hash);
}
