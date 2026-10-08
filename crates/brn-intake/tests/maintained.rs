#![cfg(feature = "helper")]
use brn_intake::*;
const DOCX: &[u8] = include_bytes!(
    "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx"
);
const PNG: &[u8] = include_bytes!(
    "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png"
);
fn convert(kind: &str, bytes: &[u8]) -> Extraction {
    let result = helper::extract(HelperRequest {
        limits: None,
        kind: kind.into(),
        bytes: bytes.to_vec(),
    })
    .unwrap();
    result.validate().unwrap();
    result
}
#[test]
fn useful_docx_maintains_exact_assets_and_repeated_occurrence_identity() {
    let result = convert("docx", DOCX);
    assert!(result.markdown.contains("Owner: Mira"));
    assert!(result.markdown.contains("EUR 4,000"));
    assert_eq!(result.markdown.matches("Pending").count(), 2);
    assert_eq!(result.assets.len(), 1);
    assert_eq!(result.assets[0].bytes, PNG);
    assert_eq!(result.occurrences.len(), 2);
    assert_ne!(result.occurrences[0].locator, result.occurrences[1].locator);
    assert_ne!(result.occurrences[0].start, result.occurrences[1].start);
    assert_eq!(
        result.occurrences[0].asset_id,
        result.occurrences[1].asset_id
    );
    assert!(result.gaps.iter().any(|g| g.contains("titles unavailable")));
}
#[test]
fn actual_single_and_plural_eml_preserve_bodies_ids_parents_and_unsupported_bytes() {
    for (bytes, plural) in [
        (
            include_bytes!(
                "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml"
            )
            .as_slice(),
            false,
        ),
        (
            include_bytes!(
                "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
            )
            .as_slice(),
            true,
        ),
    ] {
        let result = convert("eml", bytes);
        assert_eq!(result.sources[0].bytes, bytes);
        assert!(result.markdown.contains("Harbor pilot – review"));
        assert!(result.markdown.contains("Mira <mira@example.test>"));
        assert!(result.markdown.contains("Operations <ops@example.test>"));
        assert!(!result.markdown.contains("{\"Address\""));
        assert!(
            result
                .sources
                .iter()
                .filter(|s| !s.text.is_empty())
                .all(|s| result.markdown.contains(&s.text))
        );
        assert!(result.markdown.contains("+0300"));
        assert!(result.markdown.contains(if plural {
            "harbor-plural@example.test"
        } else {
            "harbor-single@example.test"
        }));
        assert!(result.markdown.contains("Actual plain-text body"));
        assert!(
            result
                .markdown
                .contains("Actual HTML alternative (inert quoted source)")
        );
        let document = result
            .sources
            .iter()
            .find(|s| s.name == "harbor.docx")
            .unwrap();
        assert_eq!(document.bytes, DOCX);
        assert!(document.parent.is_some());
        assert_eq!(
            result
                .occurrences
                .iter()
                .filter(|o| o.source_id == document.id)
                .count(),
            2
        );
        assert_eq!(result.occurrences.len(), 3);
        assert_eq!(result.assets.len(), 1);
        if plural {
            let xlsx = result
                .sources
                .iter()
                .find(|s| s.name == "forecast.xlsx")
                .unwrap();
            assert_eq!(
                xlsx.bytes,
                include_bytes!(
                    "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/forecast.xlsx"
                )
            );
            assert_eq!(xlsx.status, "unprocessed");
            assert!(xlsx.text.is_empty());
        }
    }
}
#[test]
fn hostile_packages_refuse_without_bespoke_package_or_xml_interpreter() {
    for bytes in [include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-path.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-inflate.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-members.docx").as_slice(), include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-dtd.docx").as_slice()] {
        assert!(helper::extract(HelperRequest { limits: None, kind: "docx".into(), bytes: bytes.to_vec() }).is_err());
    }
}
#[test]
fn namespace_prefix_gap_is_honest_and_never_guesses_asset_joins() {
    let result = convert(
        "docx",
        include_bytes!(
            "../../../experiments/architecture-reassessment/p2-api-check/prefix-variation.docx"
        ),
    );
    assert!(result.occurrences.is_empty());
    assert!(result.assets.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("unresolved-reference"))
    );
    assert!(result.markdown.contains("image unavailable"));
}
#[test]
fn chart_omission_is_visible_and_has_no_invented_chart_facts() {
    let result = convert(
        "docx",
        include_bytes!("../../../experiments/architecture-reassessment/p2-api-check/features.docx"),
    );
    assert!(result.gaps.iter().any(|g| g.contains("c:chart")));
    assert!(result.gaps.iter().any(|g| g.contains("charts/chart")));
}
#[test]
fn missing_and_ambiguous_cid_never_cross_related_scope() {
    let missing = b"MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=x\r\n\r\n--x\r\nContent-Type: text/html\r\n\r\n<img src='cid:missing'><img src='https://remote.invalid/pic.png'>\r\n--x--\r\n";
    let result = convert("eml", missing);
    assert!(result.occurrences.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("missing/ambiguous CID missing"))
    );
    assert!(result.gaps.iter().any(|g| g.contains("remote/non-CID")));
    let ambiguous = b"MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=x\r\n\r\n--x\r\nContent-Type: text/html\r\n\r\n<img src='cid:duplicate'>\r\n--x\r\nContent-Type: image/png\r\nContent-ID: <duplicate>\r\n\r\nfirst\r\n--x\r\nContent-Type: image/png\r\nContent-ID: <duplicate>\r\n\r\nsecond\r\n--x--\r\n";
    let result = convert("eml", ambiguous);
    assert!(result.occurrences.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("missing/ambiguous CID duplicate"))
    );
}

#[test]
fn cid_resolution_is_scoped_to_its_own_related_container() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let message = format!(
        "MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=outer\r\n\r\n--outer\r\nContent-Type: multipart/related; boundary=first\r\n\r\n--first\r\nContent-Type: text/html\r\n\r\n<img src='cid:shared'>\r\n--first\r\nContent-Type: image/png\r\nContent-ID: <shared>\r\n\r\ninvalid image\r\n--first--\r\n--outer\r\nContent-Type: multipart/related; boundary=second\r\n\r\n--second\r\nContent-Type: text/html\r\n\r\n<img src='cid:shared'>\r\n--second\r\nContent-Type: image/png\r\nContent-ID: <shared>\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--second--\r\n--outer--\r\n",
        STANDARD.encode(PNG)
    );
    let result = convert("eml", message.as_bytes());
    assert_eq!(result.occurrences.len(), 1);
    assert_eq!(result.assets[0].bytes, PNG);
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("CID shared: image signature"))
    );
}

#[test]
fn unknown_attachment_and_malformed_docx_remain_exact_unprocessed_children() {
    let message = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=broken.docx\r\n\r\nnot a ZIP\r\n--x--\r\n";
    let result = convert("eml", message);
    let child = result
        .sources
        .iter()
        .find(|s| s.name == "broken.docx")
        .unwrap();
    assert_eq!(child.status, "unprocessed");
    assert_eq!(child.bytes, b"not a ZIP");
    assert!(child.text.is_empty());
    assert!(result.gaps.iter().any(|g| g.contains("DOCX unprocessed")));
}

#[test]
fn package_members_and_mime_parts_have_aggregate_job_budgets() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut parts = ooxml_opc::unzip_parts(DOCX).unwrap();
    for i in 0..260 {
        parts.push((format!("opaque/item-{i}.bin"), vec![0]));
    }
    let docx = ooxml_opc::rezip_parts(&parts).unwrap();
    let mut message =
        String::from("MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n");
    for name in ["first.docx", "second.docx"] {
        message.push_str(&format!("--x\r\nContent-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document\r\nContent-Disposition: attachment; filename={name}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n", STANDARD.encode(&docx)));
    }
    message.push_str("--x--\r\n");
    let result = convert("eml", message.as_bytes());
    assert_eq!(
        result
            .sources
            .iter()
            .find(|s| s.name == "second.docx")
            .unwrap()
            .status,
        "unprocessed"
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|g| g.contains("package member budget"))
    );
    let mut excessive =
        String::from("MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n");
    for _ in 0..513 {
        excessive.push_str("--x\r\nContent-Type: text/plain\r\n\r\npart\r\n");
    }
    excessive.push_str("--x--\r\n");
    assert!(
        helper::extract(HelperRequest {
            limits: None,
            kind: "eml".into(),
            bytes: excessive.into_bytes()
        })
        .unwrap_err()
        .contains("MIME depth/part budget")
    );
}

#[test]
fn configured_quotas_and_actual_consumption_survive_protocol_round_trip() {
    let output = convert(
        "eml",
        include_bytes!(
            "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
        ),
    );
    let usage = output.consumed.as_ref().unwrap();
    assert_eq!(usage.input_bytes, output.sources[0].bytes.len());
    assert_eq!(
        usage.decoded_bytes,
        output
            .sources
            .iter()
            .skip(1)
            .map(|s| s.bytes.len())
            .sum::<usize>()
    );
    assert_eq!(
        usage.image_pixels,
        output
            .assets
            .iter()
            .map(|a| u64::from(a.width) * u64::from(a.height))
            .sum::<u64>()
    );
    assert!(usage.expanded_bytes > 0 && usage.package_parts > 0 && usage.mime_parts > 0);
    assert_eq!(
        usage.output_bytes,
        serde_json::to_vec(&output).unwrap().len()
    );
    for limits in [
        IntakeLimits {
            max_input_bytes: 1,
            ..Default::default()
        },
        IntakeLimits {
            max_package_parts: 1,
            ..Default::default()
        },
        IntakeLimits {
            max_expanded_bytes: 1,
            ..Default::default()
        },
        IntakeLimits {
            max_output_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(
            helper::extract(HelperRequest {
                limits: Some(limits),
                kind: "docx".into(),
                bytes: DOCX.to_vec()
            })
            .is_err()
        );
    }
    let limited = helper::extract(HelperRequest {
        limits: Some(IntakeLimits {
            max_image_pixels: 1,
            ..Default::default()
        }),
        kind: "docx".into(),
        bytes: DOCX.to_vec(),
    })
    .unwrap();
    assert!(limited.assets.is_empty() && limited.occurrences.is_empty());
    assert!(
        limited
            .gaps
            .iter()
            .any(|g| g.contains("limit") || g.contains("budget"))
    );
    for limits in [
        IntakeLimits {
            max_mime_parts: 1,
            ..Default::default()
        },
        IntakeLimits {
            max_mime_depth: 1,
            ..Default::default()
        },
        IntakeLimits {
            max_decoded_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(helper::extract(HelperRequest { limits: Some(limits), kind: "eml".into(), bytes: include_bytes!("../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml").to_vec() }).is_err());
    }
    let mut forged = output;
    forged.consumed.as_mut().unwrap().decoded_bytes += 1;
    assert!(forged.validate().is_err());
}

#[test]
fn resolved_exact_images_do_not_retain_upstream_byte_omission_diagnostics() {
    let resolved = convert("docx", DOCX);
    assert!(
        !resolved
            .gaps
            .iter()
            .any(|g| g.contains("image-data-omitted"))
    );
    assert!(!resolved.gaps.iter().any(|g| g.starts_with('{')));
    let unresolved = convert(
        "docx",
        include_bytes!(
            "../../../experiments/architecture-reassessment/p2-api-check/prefix-variation.docx"
        ),
    );
    assert!(
        unresolved
            .gaps
            .iter()
            .any(|g| g.contains("unresolved-reference"))
    );
    assert!(
        unresolved
            .gaps
            .iter()
            .any(|g| g.contains("image-data-omitted"))
    );
}

const EMAIL_CAVEAT: &str = "source-0: decoded email headers are source claims; sender authenticity and thread relationships have not been independently verified";
const EMAIL_IDENTIFIERS: [(&str, &str, &str); 3] = [
    (
        "Message-ID: <service-window@orchard.example.test>\r\n",
        "Message-ID",
        "service-window@orchard.example.test",
    ),
    (
        "In-Reply-To: <prior-request@orchard.example.test>\r\n",
        "In-Reply-To",
        "prior-request@orchard.example.test",
    ),
    (
        "References: <first-request@orchard.example.test> <prior-request@orchard.example.test>\r\n",
        "References",
        "first-request@orchard.example.test prior-request@orchard.example.test",
    ),
];
fn synthetic_email(headers: &str, media: &str, body: &str) -> Vec<u8> {
    format!("From: Orchard Service <service@orchard.example.test>\r\nTo: Review Desk <review@orchard.example.test>\r\nSubject: Service-window review\r\nDate: Wed, 7 Oct 2026 14:45:00 +0200\r\n{headers}MIME-Version: 1.0\r\nContent-Type: {media}; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}").into_bytes()
}

#[test]
fn email_plain_headers_preserve_all_identifiers_without_false_absence_or_html_gaps() {
    let headers = EMAIL_IDENTIFIERS
        .iter()
        .map(|(raw, _, _)| *raw)
        .collect::<String>();
    let bytes = synthetic_email(
        &headers,
        "text/plain",
        "Please review the recorded service window. Õun 日本語\r\n",
    );
    let result = convert("eml", &bytes);
    assert_eq!(result.sources[0].bytes, bytes);
    assert_eq!(result.sources[0].status, "partial");
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.gaps, vec![EMAIL_CAVEAT]);
    for (_, label, value) in EMAIL_IDENTIFIERS {
        assert!(
            result
                .markdown
                .contains(&format!("\n{label}\n\n```\n{value}\n```\n"))
        );
        assert!(result.sources[0].text.contains(value));
    }
    assert!(result.markdown.contains("Actual plain-text body"));
    assert!(!result.markdown.contains("Actual HTML alternative"));
    assert!(result.assets.is_empty());
    assert!(result.occurrences.is_empty());
}

#[test]
fn email_missing_identifiers_individually_and_together_remain_unknown_without_inferred_threads() {
    // Each combination includes individual omissions, absent reply headers on a
    // new message, and all identifiers unavailable. Subject is intentionally unchanged.
    for mask in 0..8 {
        let headers = EMAIL_IDENTIFIERS
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, (raw, _, _))| *raw)
            .collect::<String>();
        let bytes = synthetic_email(&headers, "text/plain", "Retain this exact body.\r\n");
        let result = convert("eml", &bytes);
        assert_eq!(result.sources[0].bytes, bytes);
        assert_eq!(result.sources[0].status, "partial");
        assert_eq!(result.gaps, vec![EMAIL_CAVEAT]);
        for (index, (_, label, value)) in EMAIL_IDENTIFIERS.iter().enumerate() {
            let expected = if mask & (1 << index) != 0 {
                *value
            } else {
                "unknown"
            };
            assert!(
                result
                    .markdown
                    .contains(&format!("\n{label}\n\n```\n{expected}\n```\n")),
                "mask {mask}: {label}"
            );
        }
        assert!(!result.markdown.contains("Actual HTML alternative"));
    }
}

#[test]
fn email_html_labels_and_remote_or_cid_gaps_describe_only_actual_content() {
    for (body, remote, missing_cid) in [
        ("<p>Exact HTML wording õ.</p>", false, false),
        (
            "<p>Exact HTML wording õ.</p><img src='https://remote.example.test/picture.png'>",
            true,
            false,
        ),
        (
            "<p>Exact HTML wording õ.</p><img src='cid:missing'>",
            false,
            true,
        ),
        (
            "<img src='cid:missing'><img src='https://remote.example.test/picture.png'>",
            true,
            true,
        ),
    ] {
        let bytes = synthetic_email(EMAIL_IDENTIFIERS[0].0, "text/html", body);
        let result = convert("eml", &bytes);
        assert_eq!(result.sources[0].bytes, bytes);
        assert_eq!(result.sources[0].status, "partial");
        assert!(
            result
                .markdown
                .contains("Actual HTML alternative (inert quoted source)")
        );
        assert!(result.markdown.contains(body));
        assert!(result.gaps.iter().any(|gap| gap == EMAIL_CAVEAT));
        assert_eq!(result.gaps.iter().any(|gap| gap.contains("remote/non-CID HTML images unavailable; no network fetch")), remote);
        assert_eq!(
            result
                .gaps
                .iter()
                .any(|gap| gap.contains("missing/ambiguous CID missing")),
            missing_cid
        );
        assert_eq!(
            result.gaps.len(),
            1 + usize::from(remote) + usize::from(missing_cid)
        );
        assert!(result.assets.is_empty());
        assert!(result.occurrences.is_empty());
    }
}

#[test]
fn email_authentication_results_remain_original_claims_without_verified_identity() {
    let ordinary_headers = EMAIL_IDENTIFIERS
        .iter()
        .map(|(raw, _, _)| *raw)
        .collect::<String>();
    let claimed_authentication = format!(
        "{ordinary_headers}Authentication-Results: claimed.example.test; dkim=pass header.d=orchard.example.test; spf=pass smtp.mailfrom=service@orchard.example.test\r\n"
    );
    let ordinary = convert(
        "eml",
        &synthetic_email(&ordinary_headers, "text/plain", "Exact body.\r\n"),
    );
    let bytes = synthetic_email(&claimed_authentication, "text/plain", "Exact body.\r\n");
    let claimed = convert("eml", &bytes);
    assert_eq!(claimed.sources[0].bytes, bytes);
    assert_eq!(claimed.markdown, ordinary.markdown);
    assert_eq!(claimed.gaps, ordinary.gaps);
    assert_eq!(claimed.gaps, vec![EMAIL_CAVEAT]);
    assert_eq!(claimed.sources[0].status, "partial");
}

#[test]
fn email_multiple_reply_identifiers_keep_parser_order_and_repetitions_without_verified_threading() {
    let headers = "Message-ID: <followup@orchard.example.test>\r\nIn-Reply-To: <second@orchard.example.test> <first@orchard.example.test>\r\nReferences: <third@orchard.example.test>\r\n\t<first@orchard.example.test> <third@orchard.example.test>\r\n";
    let bytes = synthetic_email(
        headers,
        "text/plain",
        "Retain header claims without inferring a verified thread.\r\n",
    );
    let result = convert("eml", &bytes);
    assert_eq!(result.sources[0].bytes, bytes);
    assert_eq!(result.sources[0].status, "partial");
    assert_eq!(result.gaps, vec![EMAIL_CAVEAT]);
    for (label, value) in [
        (
            "In-Reply-To",
            "second@orchard.example.test first@orchard.example.test",
        ),
        (
            "References",
            "third@orchard.example.test first@orchard.example.test third@orchard.example.test",
        ),
    ] {
        let rendered = format!("\n{label}\n\n```\n{value}\n```\n");
        assert!(result.markdown.contains(&rendered));
        assert!(result.sources[0].text.contains(&rendered));
    }
}

#[test]
fn email_repeated_physical_reply_fields_preserve_mixed_scalar_lists_in_source_order() {
    let headers = "Message-ID: <physical-fields@orchard.example.test>\r\nIn-Reply-To: <scalar@orchard.example.test>\r\nReferences: <z-first@orchard.example.test> <a-next@orchard.example.test>\r\nIn-Reply-To: <middle@orchard.example.test> <scalar@orchard.example.test>\r\nReferences: <tail@orchard.example.test>\r\nIn-Reply-To: <end@orchard.example.test>\r\nReferences: <z-first@orchard.example.test>\r\n\t<final@orchard.example.test>\r\n";
    let bytes = synthetic_email(
        headers,
        "text/plain",
        "Every decoded physical reply field is a retained source claim.\r\n",
    );
    let result = convert("eml", &bytes);
    assert_eq!(result.sources[0].bytes, bytes);
    assert_eq!(result.sources[0].status, "partial");
    assert_eq!(result.gaps, vec![EMAIL_CAVEAT]);
    for (label, value) in [
        (
            "In-Reply-To",
            "scalar@orchard.example.test middle@orchard.example.test scalar@orchard.example.test end@orchard.example.test",
        ),
        (
            "References",
            "z-first@orchard.example.test a-next@orchard.example.test tail@orchard.example.test z-first@orchard.example.test final@orchard.example.test",
        ),
    ] {
        let rendered = format!("\n{label}\n\n```\n{value}\n```\n");
        assert!(
            result.markdown.contains(&rendered),
            "{label}: {}",
            result.markdown
        );
        assert!(result.sources[0].text.contains(&rendered));
    }
}
