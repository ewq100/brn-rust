use super::citation_review_state::NeedsReviewMode;
use super::*;
use brn_workflow::knowledge::*;

pub(crate) const EXACT: &str = "\u{feff}Original 日本語 õ\r\nsecond λ\r";
pub(crate) fn ready() -> AiState {
    AiState {
        ready: true,
        vault_bound: true,
        ..Default::default()
    }
}
pub(crate) fn detail() -> CitationReviewDetail {
    let citation = VaultCitation {
        note_id: Uuid::from_u128(17),
        sha256: [19; 32],
        start_byte: 0,
        end_byte: EXACT.len(),
        quote: EXACT.into(),
    };
    let text = brn_store::note_provenance::write(
        "\u{feff}# Consumer 日本語\r\nSaved body õ\r\n",
        std::slice::from_ref(&citation),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("current.md"), &text).unwrap();
    let sha256 = brn_workflow::vault::read_evidence(
        directory.path(),
        &brn_workflow::vault::EvidencePath::parse("current.md").unwrap(),
    )
    .unwrap()
    .sha256;
    let detail = CitationReviewDetail {
        path: "current.md".into(),
        title: "Consumer 日本語".into(),
        note_id: None,
        sha256,
        text,
        provenance: NoteProvenance {
            path: "current.md".into(),
            inbox_source: None,
            citations: vec![ResolvedCitation {
                citation,
                outcome: CitationOutcome::Incomplete,
                matches: vec![],
                issues: vec![IdentityIssue {
                    path: "source.md".into(),
                    reason: "Synthetic unavailable lookup".into(),
                }],
            }],
        },
    };
    detail
        .validate_for(&CitationReviewDetailRequest {
            path: detail.path.clone(),
            expected_sha256: detail.sha256,
        })
        .unwrap();
    detail
}
pub(crate) fn entry(detail: &CitationReviewDetail) -> CitationReviewEntry {
    CitationReviewEntry {
        path: detail.path.clone(),
        title: detail.title.clone(),
        note_id: detail.note_id,
        sha256: detail.sha256,
        citations: vec![CitationReviewIssue {
            index: 0,
            outcome: CitationOutcome::Incomplete,
        }],
    }
}
pub(crate) fn page(entries: Vec<CitationReviewEntry>) -> CitationReviewPage {
    CitationReviewPage {
        inspected_count: entries.len(),
        entries,
        next_cursor: None,
        coverage: CitationReviewCoverage {
            incomplete: false,
            diagnostics: vec![],
            diagnostic_count: 0,
            diagnostics_truncated: false,
        },
    }
}
pub(crate) fn cursor() -> CitationReviewCursor {
    CitationReviewCursor { vault: serde_json::from_value(serde_json::json!({"id":Uuid::from_u128(18),"root":"/synthetic/vault","identity":{"device":741,"inode":800}})).unwrap(), observation_digest: [23; 32], after_path: "current.md".into() }
}
fn open(state: &mut AiState, page: CitationReviewPage) {
    let (id, _) = state.open_citation_review().unwrap();
    assert!(
        state
            .apply(id, AppEvent::CitationReview(Box::new(page)))
            .is_empty()
    );
}
#[test]
fn modes_default_and_sparse_cursor_advancement_are_explicit() {
    let mut state = ready();
    assert_eq!(state.needs_review_mode, NeedsReviewMode::Findings);
    let mut sparse = page(vec![]);
    sparse.inspected_count = 25;
    sparse.next_cursor = Some(cursor());
    open(&mut state, sparse);
    assert!(state.citation_review.visible);
    assert!(!state.finding_queue.visible);
    let (id, AppCommand::CitationReview(request)) = state.load_more_citation_review().unwrap()
    else {
        panic!()
    };
    assert_eq!(request.cursor, Some(cursor()));
    assert!(state.citation_review_loading());
    assert!(state.load_more_citation_review().is_none());
    let mut next = page(vec![]);
    next.inspected_count = 25;
    let mut progress = cursor();
    progress.after_path = "z.md".into();
    next.next_cursor = Some(progress.clone());
    state.apply(id, AppEvent::CitationReview(Box::new(next)));
    let (_, AppCommand::CitationReview(request)) = state.load_more_citation_review().unwrap()
    else {
        panic!()
    };
    assert_eq!(request.cursor, Some(progress));
    let (id, AppCommand::CitationReview(request)) = state.refresh_citation_review().unwrap() else {
        panic!()
    };
    assert_eq!(request.cursor, None);
    state.apply(id, AppEvent::CitationReview(Box::new(page(vec![]))));
    assert!(state.load_more_citation_review().is_none());
    state.open_findings().unwrap();
    assert_eq!(state.needs_review_mode, NeedsReviewMode::Findings);
    assert!(!state.citation_review.visible);
}
#[test]
fn refresh_mode_close_and_operation_ids_reject_old_pages_and_errors() {
    let mut state = ready();
    let expected = page(vec![entry(&detail())]);
    let (old, _) = state.open_citation_review().unwrap();
    let (new, _) = state.refresh_citation_review().unwrap();
    state.apply(
        Uuid::new_v4(),
        AppEvent::CitationReview(Box::new(expected.clone())),
    );
    assert!(state.citation_review.page.is_none());
    state.apply(new, AppEvent::CitationReview(Box::new(expected.clone())));
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old failure")),
    );
    assert_eq!(state.citation_review.page, Some(expected.clone()));
    assert!(state.citation_review.error.is_none());
    let (old, _) = state.refresh_citation_review().unwrap();
    state.open_findings().unwrap();
    state.apply(old, AppEvent::CitationReview(Box::new(expected)));
    assert!(state.citation_review.page.is_none());
    let (old, _) = state.open_citation_review().unwrap();
    state.close_findings();
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("closed failure")),
    );
    assert!(state.citation_review.error.is_none());
}
#[test]
fn malformed_page_wrong_hash_and_stale_row_never_open_an_unbound_consumer() {
    let mut state = ready();
    let detail = detail();
    let expected = entry(&detail);
    open(&mut state, page(vec![expected.clone()]));
    let row = state.citation_review_row_capture(&expected).unwrap();
    let (old, AppCommand::CitationReviewDetail(request)) =
        state.select_citation_review(row.clone()).unwrap()
    else {
        panic!()
    };
    assert_eq!(request.expected_sha256, detail.sha256);
    assert_eq!(request.path, detail.path);
    let (new, _) = state.select_citation_review(row.clone()).unwrap();
    state.apply(
        old,
        AppEvent::CitationReviewDetail(Box::new(detail.clone())),
    );
    assert!(state.citation_review.detail.is_none());
    let mut wrong = detail.clone();
    wrong.sha256 = [1; 32];
    state.apply(new, AppEvent::CitationReviewDetail(Box::new(wrong)));
    assert!(state.citation_review.detail.is_none());
    assert!(
        state
            .citation_review
            .error
            .as_ref()
            .unwrap()
            .contains("Refresh")
    );
    for invalid in 0..3 {
        let (id, _) = state.select_citation_review(row.clone()).unwrap();
        let mut wrong = detail.clone();
        match invalid {
            0 => wrong.path = "other.md".into(),
            1 => wrong.provenance.path = "other.md".into(),
            _ => wrong.text.push_str("Partial or different consumer"),
        }
        state.apply(id, AppEvent::CitationReviewDetail(Box::new(wrong)));
        assert!(state.citation_review.detail.is_none());
        assert!(
            state
                .citation_review
                .error
                .as_ref()
                .unwrap()
                .contains("Refresh")
        );
    }
    let (late_detail, _) = state.select_citation_review(row.clone()).unwrap();
    let (id, _) = state.refresh_citation_review().unwrap();
    state.apply(
        late_detail,
        AppEvent::CitationReviewDetail(Box::new(detail.clone())),
    );
    assert!(state.citation_review.detail.is_none());
    assert!(state.select_citation_review(row.clone()).is_none());
    let mut malformed = page(vec![expected.clone()]);
    malformed.entries[0].citations[0].outcome = CitationOutcome::Matched;
    state.apply(id, AppEvent::CitationReview(Box::new(malformed)));
    assert!(state.citation_review.page.is_none());
    assert!(state.citation_review.error.is_some());
    let (id, _) = state.refresh_citation_review().unwrap();
    state.apply(
        id,
        AppEvent::CitationReview(Box::new(page(vec![expected.clone()]))),
    );
    assert!(
        state.select_citation_review(row).is_none(),
        "old identical row still belongs to an old page generation"
    );
    let row = state.citation_review_row_capture(&expected).unwrap();
    let (id, _) = state.select_citation_review(row).unwrap();
    state.apply(id, AppEvent::CitationReviewDetail(Box::new(detail.clone())));
    assert_eq!(state.citation_review.detail, Some(detail));
}
#[test]
fn inspection_preserves_owner_buffers_and_note_review_generations() {
    let mut state = ready();
    owner_editor(&mut state);
    owner_review(&mut state);
    state.notice = "Owner notice".into();
    state.note_generation = 41;
    state.review_generation = 52;
    state.draft = Some(crate::draft::DraftForm::new(None).unwrap());
    state.draft.as_mut().unwrap().edit(
        "Owner title".into(),
        "new.md".into(),
        "Owner unsaved draft õ\r\n".into(),
        crate::draft::DraftKind::Create,
    );
    let detail = detail();
    let expected = entry(&detail);
    open(&mut state, page(vec![expected.clone()]));
    let row = state.citation_review_row_capture(&expected).unwrap();
    let (id, _) = state.select_citation_review(row).unwrap();
    state.apply(id, AppEvent::CitationReviewDetail(Box::new(detail)));
    assert_eq!(
        state.draft.as_ref().unwrap().text,
        "Owner unsaved draft õ\r\n"
    );
    assert_eq!(state.note_generation, 41);
    assert_eq!(state.review_generation, 52);
    assert_eq!(state.notice, "Owner notice");
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Owner unsaved note õ\r\n"
    );
    assert!(state.evidence.is_none());
    assert_eq!(
        state.review.as_ref().unwrap().text(0),
        Some("Owner unsaved review õ\r\n")
    );
    assert!(!state.review_can_leave());
    assert!(state.proposals.is_empty());
    assert!(state.active.is_none());
}

pub(crate) fn owner_editor(state: &mut AiState) {
    let (id, _) = state.open_editor("owner.md".into());
    state.apply(
        id,
        AppEvent::Editor(super::tests::editor_view("owner.md", "Saved owner note")),
    );
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Owner unsaved note õ\r\n".into(), Instant::now())
        .unwrap();
}

pub(crate) fn owner_review(state: &mut AiState) {
    let record = serde_json::from_value(serde_json::json!({
        "draft": {
            "id": Uuid::new_v4(), "group_id": null, "session_id": null,
            "vault": {"id":Uuid::new_v4(),"root":"/synthetic/review-vault","identity":{"device":1,"inode":2}},
            "title": "Owner review", "changes": [{"kind":"create","path":"new.md","parent":{"device":1,"inode":3},"text":"Original reviewed text"}], "sources":[]
        },
        "version":1, "state":"draft", "created_at_ms":1, "updated_at_ms":1, "comments":[]
    })).unwrap();
    let mut review = crate::review::ProposalReview::new(record);
    review
        .edit_text(0, "Owner unsaved review õ\r\n".into(), Instant::now())
        .unwrap();
    state.review = Some(review);
}

#[test]
fn vault_rebind_and_changed_root_invalidate_derived_proofs_but_same_root_status_is_stable() {
    let mut state = ready();
    state.vault_root = Some("/synthetic/first-vault".into());
    owner_editor(&mut state);
    owner_review(&mut state);
    let detail = detail();
    let expected = entry(&detail);
    open(&mut state, page(vec![expected.clone()]));
    let row = state.citation_review_row_capture(&expected).unwrap();
    let (late_detail, _) = state.select_citation_review(row).unwrap();
    let (bind, _) = state.command(
        Pending::Bind,
        AppCommand::BindVault("/synthetic/second-vault".into()),
    );
    state.apply(bind, AppEvent::VaultBound);
    assert_eq!(state.needs_review_mode, NeedsReviewMode::CitationEvidence);
    assert!(state.citation_review.visible);
    assert!(state.citation_review.page.is_none());
    assert!(state.citation_review.selected.is_none());
    state.apply(
        late_detail,
        AppEvent::CitationReviewDetail(Box::new(detail.clone())),
    );
    assert!(state.citation_review.detail.is_none());
    let status = |root: &str| {
        AppEvent::Status(brn_workflow::app_worker::AppStatus {
            vault_root: Some(root.into()),
            model_installed: false,
            model_download: None,
        })
    };
    let (late_page, _) = state.refresh_citation_review().unwrap();
    state.apply(Uuid::new_v4(), status("/synthetic/second-vault"));
    state.apply(
        late_page,
        AppEvent::CitationReview(Box::new(page(vec![expected.clone()]))),
    );
    assert!(state.citation_review.page.is_none());
    assert!(
        state
            .citation_review
            .error
            .as_ref()
            .unwrap()
            .contains("Refresh")
    );
    let (id, _) = state.refresh_citation_review().unwrap();
    state.apply(
        id,
        AppEvent::CitationReview(Box::new(page(vec![expected.clone()]))),
    );
    let row = state.citation_review_row_capture(&expected).unwrap();
    let (id, _) = state.select_citation_review(row).unwrap();
    state.apply(id, AppEvent::CitationReviewDetail(Box::new(detail.clone())));
    state.apply(Uuid::new_v4(), status("/synthetic/second-vault"));
    assert_eq!(state.citation_review.detail, Some(detail));
    assert!(state.citation_review.page.is_some());
    assert!(state.citation_review.error.is_none());
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Owner unsaved note õ\r\n"
    );
    assert_eq!(
        state.review.as_ref().unwrap().text(0),
        Some("Owner unsaved review õ\r\n")
    );
}
