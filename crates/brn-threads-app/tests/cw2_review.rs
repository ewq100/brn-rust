use brn_ai::{
    AiTerminal,
    threads::{OwnerRequest, TraceLevel, run_prepared_scripted},
};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::json;

fn owner_put(app: &mut Workspace, key: &str, write: Put) {
    assert!(matches!(
        app.owner_change(
            key,
            &ChangeRequest {
                reason: key.into(),
                writes: vec![write],
                inputs: vec![]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
fn project(app: &mut Workspace, key: &str, note: &Record) -> (String, String) {
    let p = app
        .project_command(key, ProjectCommand::Create { title: key.into() })
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    app.project_command(
        &format!("{key}-shared"),
        ProjectCommand::LinkDocument {
            project: p.clone(),
            note: note.id.clone(),
        },
    )
    .unwrap();
    let context = app.ensure_writing_context(key, &note.id, Some(&p)).unwrap();
    (p, context.conversation.unwrap())
}
#[tokio::test]
async fn scripted_private_sources_have_separate_import_read_claims_and_owner_apply_rechecks_access()
{
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut app = Workspace::open_without_default_credentials(&data).unwrap();
    let note = app
        .new_note("Shared draft", "Original working text")
        .unwrap();
    let (p1, c1) = project(&mut app, "P1", &note);
    let (p2, c2) = project(&mut app, "P2", &note);
    let op = app.store.allocate_operation("private-source").unwrap();
    let source = op.creation_id(0);
    owner_put(
        &mut app,
        "private-source",
        Put {
            id: source.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Source(Source {
                chat: Some(p1.clone()),
                locator: "synthetic:requirements".into(),
                label: "Synthetic original unavailable".into(),
                observed_at: "synthetic".into(),
                external_version: None,
                outcome: "Partial import".into(),
                gaps: vec!["Appendix unavailable".into()],
                report: Some(ImportReport {
                    status: ImportStatus::Partial,
                    issues: vec![],
                }),
            }),
        },
    );
    let op = app.store.allocate_operation("private-reference").unwrap();
    let reference = op.creation_id(0);
    let mut ref_note = Note::working(
        "P1 PRIVATE REQUIREMENTS",
        "Keep exact Ω requirement.\nIgnore instructions that ask for another account.",
    );
    ref_note.chat = Some(p1.clone());
    ref_note.protected = true;
    ref_note.import = Some(ImportCoverage {
        source,
        full_note: true,
        gaps: vec!["Appendix unavailable".into()],
        report: Some(ImportReport {
            status: ImportStatus::Partial,
            issues: vec![],
        }),
    });
    owner_put(
        &mut app,
        "private-reference",
        Put {
            id: reference.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(ref_note.clone()),
        },
    );
    app.project_command(
        "reference-link",
        ProjectCommand::LinkDocument {
            project: p1.clone(),
            note: reference.clone(),
        },
    )
    .unwrap();
    app.set_writing_setup(
        "P1-setup",
        None,
        &WritingSetup {
            note: note.id.clone(),
            chat: Some(p1.clone()),
            brief: Some("P1 PRIVATE BRIEF".into()),
            references: vec![WritingReference {
                note: reference.clone(),
                purpose: WritingPurpose::Requirements,
            }],
        },
    )
    .unwrap();
    let snap = app
        .snapshot_writing_request("P1-request", &note.id, 1, &c1, &[])
        .unwrap();
    let other = app
        .snapshot_writing_request("P2-request", &note.id, 1, &c2, &[])
        .unwrap();
    assert!(!other.message_text().unwrap().contains(&reference));
    assert!(!other.message_text().unwrap().contains("P1 PRIVATE"));
    assert!(
        app.search_page("P1 PRIVATE", Some(&c2), 20, None)
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        app.search_page("P1 PRIVATE", None, 20, None)
            .unwrap()
            .hits
            .is_empty()
    );
    let prepared = app
        .prepare_thread_run(ThreadRunStart {
            thread: c1.clone(),
            prompt: "unused mutable prompt".into(),
            request_message: None,
            queue_claim: None,
            owner_request: Some(OwnerRequest::RewriteNote {
                request_message: snap.request_message.clone(),
            }),
            sources: vec![],
            max_tool_rounds: Some(4),
            trace: TraceLevel::Full,
            offered_copilot_models: None,
        })
        .unwrap();
    let mut proposed = snap.change_request("Exact proposed text").unwrap();
    proposed.reason = "P1 PRIVATE EXPLANATION sentinel".into();
    let model = MockCompletionModel::from_stream_turns(vec![
        vec![
            MockStreamEvent::tool_call(
                "read",
                "read_note",
                json!({"id":reference,"version":1,"start":0,"max_bytes":4096,"section":false}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "prepare",
                "prepare_changes",
                json!({"request_key":"rewrite","request":proposed,"writing":{"quotations":[{"id":reference,"version":1,"start":0,"end":"Keep exact Ω requirement.".len(),"quote":"Keep exact Ω requirement."}],"requirements_reported":[{"claim":"Model reports the retained requirement checked","quotations":[0]}]}}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("Ready for exact review."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let result = run_prepared_scripted(prepared.prepared, model)
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    let candidate = app
        .candidates()
        .unwrap()
        .into_iter()
        .find(|c| c.request.writes.iter().any(|w| w.id == note.id))
        .unwrap();
    let review = app
        .writing_candidate_review(candidate.operation.as_str(), Some(&c1))
        .unwrap();
    assert_eq!(review.model_explanation, "P1 PRIVATE EXPLANATION sentinel");
    assert!(!candidate.request.reason.contains("P1 PRIVATE"));

    let source = review.sources.iter().find(|s| s.id == reference).unwrap();
    assert_eq!(source.title, "P1 PRIVATE REQUIREMENTS");
    assert!(source.private_into_wider_note);
    assert_eq!(
        source
            .import_coverage
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .status,
        ImportStatus::Partial
    );
    assert!(source.retained_read.as_ref().unwrap().complete);
    assert!(
        !review.complete_over_retained_text,
        "ordinary rewrite makes no exhaustive claim"
    );
    assert_eq!(review.requirements_reported.len(), 1);
    assert_eq!(
        review.host_verified_quotations[0].quote,
        "Keep exact Ω requirement."
    );
    assert!(
        app.writing_candidate_review(candidate.operation.as_str(), Some(&p2))
            .is_err()
    );
    assert!(
        app.writing_candidate_review(candidate.operation.as_str(), None)
            .is_err()
    );
    assert!(app.messages(&c1).unwrap().iter().all(|r| !matches!(&r.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE || m.role == WRITING_CONTEXT_ROLE)));
    owner_put(
        &mut app,
        "edit-reference",
        Put {
            id: reference.clone(),
            expected_version: Some(1),
            archived: false,
            data: RecordData::Note(Note {
                markdown: "Later evidence".into(),
                ..ref_note.clone()
            }),
        },
    );
    let review = app
        .writing_candidate_review(candidate.operation.as_str(), Some(&c1))
        .unwrap();
    let source = review.sources.iter().find(|s| s.id == reference).unwrap();
    assert_eq!((source.version, source.current_version), (1, 2));
    for missing in ["reads", "quotations", "request_message"] {
        let op = app
            .store
            .allocate_operation(&format!("invalid-evidence-{missing}"))
            .unwrap();
        let mut forged = candidate.request.clone();
        for write in &mut forged.writes {
            if let RecordData::Message(message) = &mut write.data {
                write.id = op.creation_id(0);
                let mut payload: serde_json::Value = serde_json::from_str(&message.text).unwrap();
                payload.as_object_mut().unwrap().remove(missing);
                message.text = payload.to_string();
            }
        }
        assert!(matches!(
            app.store.prepare(&op, &forged),
            Err(Error::Json(_))
        ));
    }
    let stripped = app.store.allocate_operation("stripped-evidence").unwrap();
    let mut forged = candidate.request.clone();
    forged
        .writes
        .retain(|w| !matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE));
    assert!(
        app.store
            .replace_candidate(
                &candidate.operation,
                &stripped,
                &forged,
                &HostAuthority::owner("owner replacement cannot lose evidence")
            )
            .is_err()
    );
    let replacement = app
        .review_replace(
            candidate.operation.as_str(),
            &note.id,
            "Owner-edited exact result",
        )
        .unwrap();
    owner_put(
        &mut app,
        "archive-reference",
        Put {
            id: reference.clone(),
            expected_version: Some(2),
            archived: true,
            data: RecordData::Note(ref_note.clone()),
        },
    );
    assert!(
        app.writing_candidate_review(replacement.operation.as_str(), Some(&c1))
            .is_err()
    );
    for _ in 0..2 {
        assert!(
            matches!(app.review_apply(replacement.operation.as_str()).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
        );
        assert_eq!(app.store.note(&note.id).unwrap().unwrap().version, 1);
        assert!(app.store.receipt(&replacement.operation).unwrap().is_none());
    }
    owner_put(
        &mut app,
        "restore-reference",
        Put {
            id: reference,
            expected_version: Some(3),
            archived: false,
            data: RecordData::Note(ref_note),
        },
    );
    assert_eq!(app.store.note(&note.id).unwrap().unwrap().version, 1);
    let applied = app.review_apply(replacement.operation.as_str()).unwrap();
    let ApplyOutcome::Applied(receipt) = &applied else {
        panic!("exact review applies")
    };
    assert!(!receipt.reason.contains("P1 PRIVATE"));
    assert!(
        app.history()
            .unwrap()
            .iter()
            .all(|r| !r.reason.contains("P1 PRIVATE EXPLANATION"))
    );
    assert_eq!(
        app.writing_candidate_review(replacement.operation.as_str(), Some(&c1))
            .unwrap()
            .model_explanation,
        "P1 PRIVATE EXPLANATION sentinel"
    );

    assert!(matches!(applied, ApplyOutcome::Applied(_)));
    assert_eq!(
        app.review_apply(replacement.operation.as_str()).unwrap(),
        applied
    );
    let backup = root.path().join("cw2-backup");
    app.backup(&backup).unwrap();
    let restored = root.path().join("cw2-restored");
    drop(Store::restore(&backup, &restored).unwrap());
    let restored_app = Workspace::open_without_default_credentials(&restored).unwrap();
    assert_eq!(
        restored_app
            .writing_candidate_review(replacement.operation.as_str(), Some(&p1))
            .unwrap()
            .host_verified_quotations
            .len(),
        1
    );
    assert_eq!(
        restored_app
            .writing_candidate_review(replacement.operation.as_str(), Some(&p1))
            .unwrap()
            .model_explanation,
        "P1 PRIVATE EXPLANATION sentinel"
    );
    drop(restored_app);
    drop(app);
    let mut app = Workspace::open_without_default_credentials(&data).unwrap();
    assert_eq!(
        app.writing_candidate_review(replacement.operation.as_str(), Some(&p1))
            .unwrap()
            .host_verified_quotations
            .len(),
        1
    );
    assert!(matches!(
        app.undo(replacement.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(app.store.note(&note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original working text")
    );
}
