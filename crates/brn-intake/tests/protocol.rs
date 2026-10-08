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
    let image = include_bytes!(
        "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png"
    );
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

fn repeated_image_fixture() -> Extraction {
    let mut extraction = fixture();
    let bytes = b"image bytes qualified separately by the helper".to_vec();
    let sha256 = digest(&bytes);
    let asset = ImageAsset {
        id: format!("asset-{}", hex(&sha256)),
        sha256,
        width: 1,
        height: 1,
        media_type: "image/png".into(),
        bytes,
    };
    let link = format!("![exact alt]({})", asset_file_name(&asset).unwrap());
    let body = format!("Before λ\n{link}\nBetween images\n{link}\nAfter both images λ");
    extraction.markdown = format!("Email header\n{body}");
    extraction.sources[0].text = extraction.markdown.clone();
    extraction.sources.push(SourceNode {
        id: "document".into(),
        parent: Some("source-1".into()),
        name: "document.docx".into(),
        media_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            .into(),
        locator: "mime/attachment/1".into(),
        status: "complete".into(),
        bytes: b"exact document bytes".to_vec(),
        text: body,
    });
    for (index, (start, _)) in extraction.markdown.match_indices(&link).enumerate() {
        extraction.occurrences.push(ImageOccurrence {
            id: format!("occurrence-{index}"),
            source_id: "document".into(),
            asset_id: asset.id.clone(),
            locator: format!("document/paragraph/{index}/image/0"),
            alt: Some("exact alt".into()),
            start,
            end: start + link.len(),
        });
    }
    extraction.assets.push(asset);
    extraction.consumed = Some(IntakeUsage {
        input_bytes: extraction.sources[0].bytes.len(),
        decoded_bytes: extraction.sources[1].bytes.len(),
        image_pixels: 1,
        ..Default::default()
    });
    loop {
        let output_bytes = serde_json::to_vec(&extraction).unwrap().len();
        let usage = extraction.consumed.as_mut().unwrap();
        if usage.output_bytes == output_bytes {
            break;
        }
        usage.output_bytes = output_bytes;
    }
    extraction.validate().unwrap();
    extraction
}

#[test]
fn separate_sources_materialize_shared_image_bytes_without_destination_conflicts() {
    let original = repeated_image_fixture();
    let unchanged = original.clone();
    let first_id = "12345678-1234-1234-1234-123456789abc";
    let second_id = "87654321-4321-4321-4321-cba987654321";
    let first = original.materialize_for_source(first_id).unwrap();
    let second = original.materialize_for_source(second_id).unwrap();
    assert_ne!(first.markdown, second.markdown);
    assert_eq!(original, unchanged);
    assert!(original.consumed.is_some());
    assert!(first.consumed.is_none());
    assert_eq!(first.assets, original.assets);
    assert_eq!(second.assets, original.assets);
    for (derived, namespace) in [(&first, first_id), (&second, second_id)] {
        derived.validate().unwrap();
        assert_eq!(derived.occurrences.len(), 2);
        for (source, old_source) in derived.sources.iter().zip(&original.sources) {
            assert_eq!(source.bytes, old_source.bytes);
            assert_eq!(source.id, old_source.id);
            assert_eq!(source.locator, old_source.locator);
            assert!(derived.markdown.contains(&source.text));
        }
        for (occurrence, old_occurrence) in derived.occurrences.iter().zip(&original.occurrences) {
            assert_eq!(occurrence.id, old_occurrence.id);
            assert_eq!(occurrence.asset_id, old_occurrence.asset_id);
            assert_eq!(occurrence.locator, old_occurrence.locator);
            assert_eq!(occurrence.alt, old_occurrence.alt);
            let destination = asset_file_name_for_source(&derived.assets[0], namespace).unwrap();
            assert_eq!(
                &derived.markdown[occurrence.start..occurrence.end],
                format!("![exact alt]({destination})")
            );
        }
    }
}

#[test]
fn citations_after_repeated_images_map_to_exact_immutable_source_ranges() {
    let original = repeated_image_fixture();
    let namespace = "12345678123412341234123456789abc";
    let derived = original.materialize_for_source(namespace).unwrap();
    for source_index in [0, 1] {
        for quote in [
            "Before λ",
            "Between images",
            "After both images λ",
            "exact alt",
        ] {
            let source = &derived.sources[source_index];
            let start = source.text.find(quote).unwrap();
            let end = start + quote.len();
            let (old_start, old_end) = original
                .original_source_range(namespace, &source.id, start, end)
                .unwrap();
            assert_eq!(
                &original.sources[source_index].text[old_start..old_end],
                quote
            );
        }
    }
    let source = &derived.sources[1];
    let destination = asset_file_name_for_source(&derived.assets[0], namespace).unwrap();
    let start = source.text.find(&destination).unwrap();
    assert!(
        original
            .original_source_range(namespace, &source.id, start, start + destination.len())
            .is_err()
    );
    assert!(
        original
            .original_source_range(namespace, &source.id, 0, source.text.len())
            .is_err()
    );
    let split = source.text.find('λ').unwrap() + 1;
    assert!(
        original
            .original_source_range(namespace, &source.id, split, split + 1)
            .is_err()
    );
}

#[test]
fn source_materialization_rejects_unsafe_names_and_unqualified_or_partial_destinations() {
    let original = repeated_image_fixture();
    for namespace in [
        "",
        "../source",
        "00000000-0000-0000-0000-000000000000",
        "12345678_1234-1234-1234-123456789abc",
    ] {
        assert!(original.materialize_for_source(namespace).is_err());
        assert!(asset_file_name_for_source(&original.assets[0], namespace).is_err());
    }
    let lower = "12345678123412341234123456789abc";
    assert_eq!(
        asset_file_name_for_source(&original.assets[0], lower).unwrap(),
        asset_file_name_for_source(&original.assets[0], "12345678-1234-1234-1234-123456789ABC")
            .unwrap()
    );
    let mut unqualified = original.clone();
    unqualified.consumed = None;
    unqualified.markdown = unqualified
        .markdown
        .replace("](", "](https://example.invalid/");
    unqualified.sources.truncate(1);
    unqualified.sources[0].text = unqualified.markdown.clone();
    for occurrence in &mut unqualified.occurrences {
        occurrence.source_id = "source-1".into();
        occurrence.start = unqualified.markdown.find("![exact alt]").unwrap();
        occurrence.end = unqualified.markdown.len();
    }
    unqualified.occurrences.truncate(1);
    unqualified.validate().unwrap();
    assert!(
        unqualified
            .materialize_for_source(lower)
            .unwrap_err()
            .contains("qualified")
    );
    let mut partial = original;
    partial.consumed = None;
    partial.sources[1].text = asset_file_name(&partial.assets[0]).unwrap()[1..].to_string();
    partial.validate().unwrap();
    assert!(
        partial
            .materialize_for_source(lower)
            .unwrap_err()
            .contains("cuts through")
    );
}

#[test]
fn source_materialization_refuses_ambiguous_text_correspondence() {
    let mut original = repeated_image_fixture();
    original.consumed = None;
    let link =
        original.markdown[original.occurrences[0].start..original.occurrences[0].end].to_string();
    // The same text appears as qualified image evidence and unassociated prose.
    original
        .markdown
        .push_str(&format!("\nUnassociated: {link}"));
    original.sources[0].text = original.markdown.clone();
    original.sources[1].text = link;
    original.validate().unwrap();
    assert!(
        original
            .materialize_for_source("12345678123412341234123456789abc")
            .unwrap_err()
            .contains("ambiguous")
    );
}
