#![cfg(feature = "helper")]
use brn_intake::*;
use std::collections::BTreeMap;
const HARBOR: &[u8] = include_bytes!(
    "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.pptx"
);
const QUAY: &[u8] = include_bytes!("fixtures/quay.pptx");
fn extract(bytes: &[u8], limits: Option<IntakeLimits>) -> Result<Extraction, String> {
    helper::extract(HelperRequest {
        kind: "pptx".into(),
        bytes: bytes.into(),
        limits,
    })
}
fn parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    ooxml_opc::unzip_parts(bytes).unwrap().into_iter().collect()
}
fn changed(mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>)) -> Vec<u8> {
    let mut entries = parts(QUAY);
    mutate(&mut entries);
    ooxml_opc::rezip_parts(&entries.into_iter().collect::<Vec<_>>()).unwrap()
}
fn replace(entries: &mut BTreeMap<String, Vec<u8>>, part: &str, before: &str, after: &str) {
    let text = String::from_utf8(entries[part].clone()).unwrap();
    assert!(text.contains(before), "fixture did not contain {before}");
    entries.insert(part.into(), text.replacen(before, after, 1).into_bytes());
}
fn email(attachments: &[(&str, &[u8])], limits: Option<IntakeLimits>) -> Extraction {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut message = String::from(
        "MIME-Version: 1.0\r\nSubject: Quay source review\r\nContent-Type: multipart/mixed; boundary=q\r\n\r\n--q\r\nContent-Type: text/plain\r\n\r\nReview both attached presentations independently.\r\n",
    );
    for (name, bytes) in attachments {
        message.push_str(&format!("--q\r\nContent-Type: application/vnd.openxmlformats-officedocument.presentationml.presentation\r\nContent-Disposition: attachment; filename={name}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n", STANDARD.encode(bytes)));
    }
    message.push_str("--q--\r\n");
    let result = helper::extract(HelperRequest {
        kind: "eml".into(),
        bytes: message.into_bytes(),
        limits,
    })
    .unwrap();
    result.validate().unwrap();
    result
}
#[test]
fn harbor_preserves_prerequisite_notes_titles_and_repeated_picture_without_chart_facts() {
    let result = extract(HARBOR, None).unwrap();
    result.validate().unwrap();
    assert_eq!(result.sources[0].bytes, HARBOR);
    assert_eq!(result.sources[0].status, "partial");
    assert!(
        result
            .markdown
            .contains("CRITICAL: installation may start only after valve inspection.")
    );
    assert!(
        result
            .markdown
            .contains("Presenter note: ask Mira to book the inspection by 12 October.")
    );
    assert_eq!(result.assets.len(), 1);
    assert_eq!(
        result.assets[0].bytes,
        include_bytes!(
            "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png"
        )
    );
    assert_eq!(result.occurrences.len(), 2);
    assert_ne!(
        result.occurrences[0].source_id,
        result.occurrences[1].source_id
    );
    assert_ne!(result.occurrences[0].locator, result.occurrences[1].locator);
    assert_eq!(
        result.occurrences[0].asset_id,
        result.occurrences[1].asset_id
    );
    assert!(
        result.occurrences[0]
            .alt
            .as_ref()
            .unwrap()
            .contains("Primary water chart")
    );
    assert!(
        result.occurrences[1]
            .alt
            .as_ref()
            .unwrap()
            .contains("Repeated water chart")
    );
    assert!(
        !result
            .occurrences
            .iter()
            .any(|o| o.alt.as_ref().unwrap().contains("dependency"))
    );
    assert!(result.gaps.iter().any(|g| g.contains("shape=3")
        && g.contains("charts/chart1.xml")
        && g.contains("factual chart content unavailable")));
    assert!(!result.markdown.contains("2 slots"));
    assert!(!result.markdown.contains("13 October"));
    assert!(!result.sources.iter().any(|s| s.text.contains("13 October")));
}
#[test]
fn unrelated_deck_keeps_presentation_order_exact_parts_groups_cells_hidden_content_and_notes() {
    let result = extract(QUAY, None).unwrap();
    let original = parts(QUAY);
    let slides: Vec<_> = result
        .sources
        .iter()
        .filter(|s| s.media_type.ends_with(".slide+xml"))
        .collect();
    assert_eq!(slides.len(), 2);
    assert!(slides[0].locator.contains("slide9.xml"));
    assert!(slides[1].locator.contains("slide2.xml"));
    assert!(slides[0].name.contains("hidden"));
    for slide in &slides {
        let path = slide.locator.split("part=").nth(1).unwrap();
        assert_eq!(slide.bytes, original[path]);
        assert_eq!(slide.parent.as_deref(), Some("source-0"));
    }
    assert!(
        result
            .markdown
            .contains("Inspect pier A\nbefore cargo arrival.\nDisplay date: 14 October")
    );
    assert!(result.markdown.contains("(hidden)"));
    for (word, row, cell) in [("Pier", 1, 1), ("Leena", 2, 2), ("Pending", 2, 3)] {
        let source = result
            .sources
            .iter()
            .find(|s| {
                s.locator
                    .ends_with(&format!("/table/row={row}/cell={cell}"))
            })
            .unwrap();
        assert!(source.text.contains(word));
    }
    let notes = result
        .sources
        .iter()
        .find(|s| {
            s.text
                .contains("Notes only: Leena must confirm the crane by 14 October.")
        })
        .unwrap();
    assert!(notes.media_type.ends_with(".notesSlide+xml"));
    assert_eq!(notes.parent.as_deref(), Some(slides[0].id.as_str()));
    assert_eq!(
        notes.bytes,
        original[notes.locator.split("part=").nth(1).unwrap()]
    );
    assert_eq!(result.occurrences.len(), 3);
    assert_eq!(result.assets.len(), 2);
    assert!(
        result
            .assets
            .iter()
            .any(|a| a.bytes == include_bytes!("fixtures/quay-map.png"))
    );
    assert!(
        result
            .assets
            .iter()
            .any(|a| a.bytes == include_bytes!("fixtures/quay-map.jpg"))
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("contentPart") && g.contains("element-path="))
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("unsupported") && g.contains("paragraph"))
    );
    assert!(result.gaps.iter().any(|g| g.contains("Inherited layout")));
    assert!(result.gaps.iter().any(|g| g.contains("Inherited master")));
    let usage = result.consumed.as_ref().unwrap();
    assert_eq!(usage.package_parts, original.len());
    assert_eq!(
        usage.expanded_bytes,
        original.values().map(Vec::len).sum::<usize>()
    );
}
#[test]
fn identical_email_attachments_retain_distinct_descendants_occurrences_and_quote_locations() {
    let result = email(&[("first.pptx", QUAY), ("second.pptx", QUAY)], None);
    let attachments: Vec<_> = result
        .sources
        .iter()
        .filter(|s| s.name.ends_with(".pptx"))
        .collect();
    assert_eq!(attachments.len(), 2);
    assert_eq!(result.assets.len(), 2);
    assert_eq!(result.occurrences.len(), 6);
    let mut groups = Vec::new();
    for attachment in &attachments {
        assert_eq!(attachment.bytes, QUAY);
        assert_eq!(attachment.status, "partial");
        let mut descendants = vec![attachment.id.clone()];
        for source in &result.sources {
            if source
                .parent
                .as_ref()
                .is_some_and(|p| descendants.contains(p))
            {
                descendants.push(source.id.clone());
            }
        }
        assert_eq!(
            result
                .occurrences
                .iter()
                .filter(|o| descendants.contains(&o.source_id))
                .count(),
            3
        );
        let note = result
            .sources
            .iter()
            .find(|s| descendants.contains(&s.id) && s.text.contains("Notes only:"))
            .unwrap();
        groups.push((descendants, note));
    }
    assert!(!groups[0].0.iter().any(|id| groups[1].0.contains(id)));
    assert_ne!(groups[0].1.text, groups[1].1.text);
    let namespace = "00112233-4455-6677-8899-aabbccddeeff";
    let derived = result.materialize_for_source(namespace).unwrap();
    derived.validate().unwrap();
    for (_, note) in groups {
        let transformed = derived.sources.iter().find(|s| s.id == note.id).unwrap();
        let quote = "Notes only: Leena must confirm the crane by 14 October.";
        let start = transformed.text.find(quote).unwrap();
        let range = result
            .original_source_range(namespace, &note.id, start, start + quote.len())
            .unwrap();
        assert_eq!(&note.text[range.0..range.1], quote);
        assert_eq!(derived.markdown.matches(&transformed.text).count(), 1);
    }
}
#[test]
fn missing_external_damaged_and_unsupported_rasters_have_located_gaps() {
    let missing = changed(|entries| {
        entries.remove("ppt/media/image1.png").unwrap();
    });
    let result = extract(&missing, None).unwrap();
    assert_eq!(result.assets.len(), 1);
    assert_eq!(result.occurrences.len(), 1);
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("shape=") && g.contains("Missing embedded picture part"))
    );
    let external = changed(|entries| {
        for part in [
            "ppt/slides/_rels/slide9.xml.rels",
            "ppt/slides/_rels/slide2.xml.rels",
        ] {
            replace(
                entries,
                part,
                "Target=\"../media/image1.png\"",
                "Target=\"https://example.invalid/private.png\" TargetMode=\"External\"",
            );
        }
    });
    let result = extract(&external, None).unwrap();
    assert_eq!(result.occurrences.len(), 1);
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("External picture unavailable; no network fetch"))
    );
    for bytes in [b"not an image".as_slice(), b"GIF89a\x01\x00\x01\x00"] {
        let damaged = changed(|entries| {
            entries.insert("ppt/media/image1.png".into(), bytes.into());
        });
        let result = extract(&damaged, None).unwrap();
        assert_eq!(result.occurrences.len(), 1);
        assert!(result.gaps.iter().any(|g| g.contains("shape=")
            && (g.contains("image signature") || g.contains("image format unprocessed"))));
    }
}
#[test]
fn duplicate_shape_and_relationship_identity_refuse_instead_of_guessing() {
    let duplicate_shape =
        changed(|entries| replace(entries, "ppt/slides/slide9.xml", "id=\"3\"", "id=\"2\""));
    assert!(
        extract(&duplicate_shape, None)
            .unwrap_err()
            .contains("ambiguous")
    );
    let duplicate_relation = changed(|entries| {
        replace(
            entries,
            "ppt/slides/_rels/slide9.xml.rels",
            "Id=\"rId2\"",
            "Id=\"rId1\"",
        )
    });
    assert!(
        extract(&duplicate_relation, None)
            .unwrap_err()
            .contains("ambiguous")
    );
}
#[test]
fn hostile_packages_and_tight_quotas_refuse_without_partial_output() {
    for bytes in [include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-path.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-inflate.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-members.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-dtd.docx").as_slice()] {
        assert!(extract(bytes, None).is_err());
    }
    for limits in [
        IntakeLimits {
            max_input_bytes: QUAY.len() - 1,
            ..Default::default()
        },
        IntakeLimits {
            max_expanded_bytes: 128,
            ..Default::default()
        },
        IntakeLimits {
            max_package_parts: 2,
            ..Default::default()
        },
        IntakeLimits {
            max_decoded_bytes: 10,
            ..Default::default()
        },
        IntakeLimits {
            max_output_bytes: 128,
            ..Default::default()
        },
    ] {
        assert!(extract(QUAY, Some(limits)).is_err());
    }
    let dtd = changed(|entries| {
        replace(
            entries,
            "ppt/presentation.xml",
            "?>",
            "?><!DOCTYPE presentation [<!ENTITY forbidden 'source substitution'>]>",
        )
    });
    assert!(extract(&dtd, None).unwrap_err().contains("DTD"));
    let deep_xml = changed(|entries| {
        replace(
            entries,
            "ppt/slides/slide9.xml",
            "Dock review is pending.",
            &format!(
                "{}deep{}",
                "<a:unqualified>".repeat(129),
                "</a:unqualified>".repeat(129)
            ),
        )
    });
    assert!(extract(&deep_xml, None).unwrap_err().contains("xmlDepth"));
    let oversized_xml = changed(|entries| {
        replace(
            entries,
            "ppt/slides/slide9.xml",
            "Dock review is pending.",
            &"word ".repeat(1_700_000),
        );
    });
    assert!(extract(&oversized_xml, None).is_err());
}
#[test]
fn failed_attachment_merge_is_atomic_and_keeps_exact_unprocessed_original() {
    let result = email(
        &[("bounded.pptx", QUAY)],
        Some(IntakeLimits {
            max_decoded_bytes: QUAY.len() + 1000,
            ..Default::default()
        }),
    );
    let attachment = result
        .sources
        .iter()
        .find(|s| s.name == "bounded.pptx")
        .unwrap();
    assert_eq!(attachment.status, "unprocessed");
    assert_eq!(attachment.bytes, QUAY);
    assert!(attachment.text.is_empty());
    assert!(result.assets.is_empty());
    assert!(result.occurrences.is_empty());
    assert!(
        !result
            .sources
            .iter()
            .any(|s| s.locator.starts_with("pptx/"))
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("PPTX unprocessed") && g.contains("quota"))
    );
    let result = email(&[("valid.pptx", QUAY), ("broken.pptx", b"not a ZIP")], None);
    assert_eq!(result.occurrences.len(), 3);
    let broken = result
        .sources
        .iter()
        .find(|s| s.name == "broken.pptx")
        .unwrap();
    assert_eq!(broken.bytes, b"not a ZIP");
    assert_eq!(broken.status, "unprocessed");
    assert!(
        !result
            .sources
            .iter()
            .any(|s| s.parent.as_deref() == Some(&broken.id))
    );
}

#[test]
fn single_part_email_attachment_keeps_email_headers_separate_from_document_source() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    for (kind, media, bytes) in [
        (
            "pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            QUAY,
        ),
        (
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            include_bytes!(
                "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx"
            )
            .as_slice(),
        ),
    ] {
        let message = format!(
            "MIME-Version: 1.0\r\nSubject: Root attachment\r\nContent-Type: {media}\r\nContent-Disposition: attachment; filename=attached.{kind}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
            STANDARD.encode(bytes)
        );
        let result = helper::extract(HelperRequest {
            kind: "eml".into(),
            limits: None,
            bytes: message.as_bytes().into(),
        })
        .unwrap();
        assert_eq!(result.sources[0].bytes, message.as_bytes());
        assert!(result.sources[0].text.contains("Root attachment"));
        let attachment = result
            .sources
            .iter()
            .find(|s| s.name == format!("attached.{kind}"))
            .unwrap();
        assert_eq!(attachment.bytes, bytes);
        assert_eq!(
            attachment.parent.as_deref(),
            Some(result.sources[0].id.as_str())
        );
        assert_eq!(attachment.status, "partial");
        assert_eq!(result.occurrences.len(), if kind == "pptx" { 3 } else { 2 });
    }
}

#[test]
fn unsupported_graphics_media_and_missing_notes_are_located_without_factual_projection() {
    // A typed media frame's poster cannot stand in for its audio/video content.
    let media = changed(|entries| {
        replace(
            entries,
            "ppt/slides/slide2.xml",
            "</p:cNvPicPr><p:nvPr/>",
            "</p:cNvPicPr><p:nvPr><a:audioFile r:link=\"unresolved\"/></p:nvPr>",
        )
    });
    let result = extract(&media, None).unwrap();
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("Audio") && g.contains("shape="))
    );
    let missing_notes = changed(|entries| {
        entries.remove("ppt/notesSlides/notesSlide1.xml").unwrap();
    });
    let result = extract(&missing_notes, None).unwrap();
    assert!(!result.markdown.contains("Notes only:"));
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("notes page is missing") && g.contains("slide9.xml"))
    );
    let mut harbor = parts(HARBOR);
    let raw = String::from_utf8(harbor["ppt/slides/slide2.xml"].clone()).unwrap();
    let start = raw.find("<c:chart").unwrap();
    let end = start + raw[start..].find("/>").unwrap() + 2;
    for (replacement, expected) in [
        (
            "<dgm:relIds xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" r:dm=\"unresolved\"/>",
            "SmartArt relationships=",
        ),
        (
            "<a:unsupportedGraphic/>",
            "Unsupported drawing graphic URI=",
        ),
    ] {
        let changed = format!("{}{}{}", &raw[..start], replacement, &raw[end..]);
        harbor.insert("ppt/slides/slide2.xml".into(), changed.into_bytes());
        let bytes =
            ooxml_opc::rezip_parts(&harbor.clone().into_iter().collect::<Vec<_>>()).unwrap();
        let result = extract(&bytes, None).unwrap();
        assert!(
            result
                .gaps
                .iter()
                .any(|g| g.contains(expected) && g.contains("slide2.xml") && g.contains("shape=3"))
        );
        assert!(!result.markdown.contains("13 October"));
    }
}

#[test]
fn review_generic_mime_office_attachments_keep_validated_package_type_and_raw_email() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    for (kind, canonical, bytes) in [
        (
            "pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            QUAY,
        ),
        (
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            include_bytes!(
                "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx"
            )
            .as_slice(),
        ),
    ] {
        for multipart in [false, true] {
            let attachment = format!(
                "Content-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=generic.{kind}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
                STANDARD.encode(bytes)
            );
            let message = if multipart {
                format!(
                    "MIME-Version: 1.0\r\nSubject: Declared generic attachment\r\nContent-Type: multipart/mixed; boundary=generic\r\n\r\n--generic\r\n{attachment}\r\n--generic--\r\n"
                )
            } else {
                format!("MIME-Version: 1.0\r\nSubject: Declared generic attachment\r\n{attachment}")
            };
            let result = helper::extract(HelperRequest {
                kind: "eml".into(),
                limits: None,
                bytes: message.as_bytes().into(),
            })
            .unwrap();
            assert_eq!(result.sources[0].bytes, message.as_bytes());
            assert_eq!(result.sources[0].media_type, "message/rfc822");
            assert!(
                String::from_utf8_lossy(&result.sources[0].bytes)
                    .contains("Content-Type: application/octet-stream")
            );
            let document = result
                .sources
                .iter()
                .find(|s| s.name == format!("generic.{kind}"))
                .unwrap();
            assert_eq!(document.bytes, bytes);
            assert_eq!(document.media_type, canonical);
            assert_eq!(
                document.parent.as_deref(),
                Some(result.sources[0].id.as_str())
            );
            assert_eq!(document.status, "partial");
            if kind == "pptx" {
                assert_eq!(
                    result
                        .sources
                        .iter()
                        .filter(|s| s.parent.as_deref() == Some(&document.id)
                            && s.media_type.ends_with(".slide+xml"))
                        .count(),
                    2
                );
            } else {
                assert!(document.text.contains("Owner: Mira"));
            }
        }
    }
}

#[test]
fn review_distinct_notes_relationship_ids_refuse_ambiguous_page_selection() {
    let bytes = changed(|entries| {
        replace(
            entries,
            "ppt/slides/_rels/slide9.xml.rels",
            "</Relationships>",
            "<Relationship Id=\"rIdSecondNotes\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide2.xml\"/></Relationships>",
        )
    });
    let result = extract(&bytes, None);
    assert!(result.is_err(), "two notes pages were silently accepted");
    assert!(result.unwrap_err().contains("ambiguous notesSlide"));
    let retained = email(&[("ambiguous.pptx", &bytes)], None);
    let attachment = retained
        .sources
        .iter()
        .find(|s| s.name == "ambiguous.pptx")
        .unwrap();
    assert_eq!(attachment.bytes, bytes);
    assert_eq!(attachment.status, "unprocessed");
    assert!(retained.assets.is_empty() && retained.occurrences.is_empty());
    assert!(
        !retained
            .sources
            .iter()
            .any(|s| s.locator.starts_with("pptx/"))
    );
}

#[test]
fn review_consumed_root_layout_and_master_singleton_edges_refuse_distinct_id_ambiguity() {
    let cases = [
        ("_rels/.rels", "officeDocument", "ppt/presentation.xml"),
        (
            "ppt/slides/_rels/slide9.xml.rels",
            "slideLayout",
            "../slideLayouts/slideLayout6.xml",
        ),
        (
            "ppt/slideLayouts/_rels/slideLayout7.xml.rels",
            "slideMaster",
            "../slideMasters/slideMaster1.xml",
        ),
    ];
    let mut accepted = Vec::new();
    for (part, kind, target) in cases {
        let bytes = changed(|entries| {
            replace(
                entries,
                part,
                "</Relationships>",
                &format!(
                    "<Relationship Id=\"rIdAmbiguous\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/></Relationships>"
                ),
            )
        });
        match extract(&bytes, None) {
            Ok(_) => accepted.push(kind),
            Err(error) => assert!(error.contains(&format!("ambiguous {kind}")), "{error}"),
        }
    }
    assert!(
        accepted.is_empty(),
        "silently selected singleton edges: {accepted:?}"
    );
}
