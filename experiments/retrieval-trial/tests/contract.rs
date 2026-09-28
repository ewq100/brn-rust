use brn_retrieval_trial::{fixtures, fuse, search_keyword, Filters, Profile, Request};

#[test]
fn keyword_obeys_current_approved_filter_before_limit() {
    let docs = fixtures();
    let hits = search_keyword(
        &docs,
        &Request::new("launch window", Profile::Keyword, 1, Filters::default()).unwrap(),
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source_id, "launch-current");
    assert_eq!(hits[0].version_id, "v2");
}

#[test]
fn rejects_invalid_requests() {
    assert!(Request::new("  ", Profile::Keyword, 2, Filters::default()).is_err());
    assert!(Request::new("x", Profile::Keyword, 0, Filters::default()).is_err());
    assert!(Request::new("x", Profile::Keyword, 101, Filters::default()).is_err());
    assert!(Request::new(
        "x",
        Profile::Keyword,
        1,
        Filters {
            source_ids: Some(vec![]),
            ..Filters::default()
        }
    )
    .is_err());
}

#[test]
fn evidence_checks_unicode_byte_offsets_and_hash() {
    let docs = fixtures();
    let hits = search_keyword(
        &docs,
        &Request::new("café", Profile::Keyword, 3, Filters::default()).unwrap(),
    )
    .unwrap();
    assert_eq!(hits[0].source_id, "unicode-note");
    hits[0].verify(&docs).unwrap();
    let mut bad = hits[0].clone();
    bad.start_byte = bad.quote.find('é').unwrap() + 1;
    assert!(bad.verify(&docs).is_err());
    let mut bad = hits[0].clone();
    bad.source_hash = "0".repeat(64);
    assert!(bad.verify(&docs).is_err());
}

#[test]
fn fusion_deduplicates_and_keeps_provenance() {
    let docs = fixtures();
    let keyword = search_keyword(
        &docs,
        &Request::new("BRN-482", Profile::Keyword, 5, Filters::default()).unwrap(),
    )
    .unwrap();
    let combined = fuse(&keyword, &keyword, 5);
    assert_eq!(combined.len(), keyword.len());
    for hit in combined {
        hit.verify(&docs).unwrap();
    }
}

#[test]
fn explicit_version_and_status_filters_find_historical_draft_and_withdrawn() {
    use brn_retrieval_trial::Approval;
    let docs = fixtures();
    let old = Filters {
        source_ids: Some(vec!["launch-current".into()]),
        version_ids: Some(vec!["v1".into()]),
        current_only: false,
        approvals: vec![Approval::Approved],
    };
    let hits = search_keyword(
        &docs,
        &Request::new("March", Profile::Keyword, 3, old).unwrap(),
    )
    .unwrap();
    assert_eq!(
        hits.iter()
            .map(|h| h.version_id.as_str())
            .collect::<Vec<_>>(),
        ["v1"]
    );
    let draft = Filters {
        source_ids: Some(vec!["draft-brief".into()]),
        approvals: vec![Approval::Draft],
        ..Filters::default()
    };
    assert_eq!(
        search_keyword(
            &docs,
            &Request::new("January", Profile::Keyword, 3, draft).unwrap()
        )
        .unwrap()[0]
            .source_id,
        "draft-brief"
    );
    let withdrawn = Filters {
        source_ids: Some(vec!["withdrawn-brief".into()]),
        approvals: vec![Approval::Withdrawn],
        ..Filters::default()
    };
    assert_eq!(
        search_keyword(
            &docs,
            &Request::new("February", Profile::Keyword, 3, withdrawn).unwrap()
        )
        .unwrap()[0]
            .source_id,
        "withdrawn-brief"
    );
}

#[test]
fn public_mutable_request_and_duplicate_document_are_rejected() {
    let docs = fixtures();
    let mut request = Request::new("launch", Profile::Keyword, 1, Filters::default()).unwrap();
    request.limit = 0;
    assert!(search_keyword(&docs, &request).is_err());
    request.limit = 1;
    let mut duplicate = docs.clone();
    duplicate.push(docs[0].clone());
    assert!(search_keyword(&duplicate, &request).is_err());
}

#[test]
fn empty_eligibility_yields_no_hits() {
    let docs = fixtures();
    let filters = Filters {
        source_ids: Some(vec!["absent".into()]),
        ..Filters::default()
    };
    assert!(search_keyword(
        &docs,
        &Request::new("launch", Profile::Keyword, 3, filters).unwrap()
    )
    .unwrap()
    .is_empty());
}

#[test]
fn keyword_adapter_rejects_semantic_profile() {
    let docs = fixtures();
    let req = Request::new("launch", Profile::Semantic, 3, Filters::default()).unwrap();
    assert!(search_keyword(&docs, &req).is_err());
}
