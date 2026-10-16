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
    // Direct core replacement, not the app wrapper which already normalizes reason.
    let archive_evidence = app
        .store
        .allocate_operation("archived-evidence-replacement")
        .unwrap();
    let before_records = app.store.records().unwrap();
    let before_candidates = app.store.candidates().unwrap();
    let original = app.store.prepared(&candidate.operation).unwrap();
    let mut forged = candidate.request.clone();
    forged.reason = "P1 PRIVATE EXPLANATION sentinel".into();
    for write in &mut forged.writes {
        if matches!(&write.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE) {
            write.id = archive_evidence.creation_id(0);
            write.archived = true;
        }
    }
    let refused = app.store.replace_candidate(
        &candidate.operation,
        &archive_evidence,
        &forged,
        &HostAuthority::owner("replacement must retain active immutable evidence"),
    );
    assert!(
        refused.is_err(),
        "archiving evidence must not bypass immutable binding and the public reason gate"
    );
    assert_eq!(app.store.records().unwrap(), before_records);
    assert_eq!(app.store.candidates().unwrap(), before_candidates);
    assert_eq!(app.store.prepared(&candidate.operation).unwrap(), original);
    assert!(app.store.prepared(&archive_evidence).is_err());
    assert!(app.store.receipt(&archive_evidence).unwrap().is_none());
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
            id: reference.clone(),
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
    let saved = app.store.note(&reference).unwrap().unwrap();
    owner_put(
        &mut app,
        "revoke-before-safe-undo",
        Put {
            id: reference.clone(),
            expected_version: Some(saved.version),
            archived: true,
            data: saved.data,
        },
    );
    let ApplyOutcome::Applied(undone) = app.undo(replacement.operation.as_str()).unwrap() else {
        panic!("safe compensation restores pre-AI text after revocation");
    };
    assert!(
        matches!(app.store.note(&note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original working text")
    );
    let redo = app.store.allocate_operation("new-style-reapply").unwrap();
    assert!(
        matches!(app.store.undo(&redo, &undone.operation, &HostAuthority::owner("explicit reapply")).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
    );
    assert!(app.store.receipt(&redo).unwrap().is_none());
    let saved = app.store.note(&reference).unwrap().unwrap();
    owner_put(
        &mut app,
        "restore-before-exact-reapply",
        Put {
            id: reference.clone(),
            expected_version: Some(saved.version),
            archived: false,
            data: saved.data,
        },
    );
    let reapplied = app
        .store
        .undo(
            &redo,
            &undone.operation,
            &HostAuthority::owner("new exact restored-access reapply"),
        )
        .unwrap();
    assert!(matches!(&reapplied, ApplyOutcome::Applied(_)));
    let saved = app.store.note(&reference).unwrap().unwrap();
    owner_put(
        &mut app,
        "revoke-before-reapply-replay",
        Put {
            id: reference,
            expected_version: Some(saved.version),
            archived: true,
            data: saved.data,
        },
    );
    assert_eq!(
        app.store
            .undo(
                &redo,
                &undone.operation,
                &HostAuthority::owner("lost response replay")
            )
            .unwrap(),
        reapplied
    );
    let third = app
        .store
        .allocate_operation("new-style-safe-compensation-again")
        .unwrap();
    assert!(matches!(
        app.store
            .undo(&third, &redo, &HostAuthority::owner("safe compensation"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(app.store.note(&note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original working text")
    );
}

#[tokio::test]
async fn general_writing_uses_real_search_counts_and_never_delivers_new_private_retained_material()
{
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let note = app
        .new_note("Working draft", "WORKING POSITIVE CONTROL")
        .unwrap();
    let reference = app
        .new_note("Beacon public", "Beacon permitted reference")
        .unwrap();
    let thread = app
        .ensure_writing_context("general-first", &note.id, None)
        .unwrap()
        .conversation
        .unwrap();
    app.set_writing_setup(
        "general-setup",
        None,
        &WritingSetup {
            note: note.id.clone(),
            chat: None,
            brief: None,
            references: vec![WritingReference {
                note: reference.id.clone(),
                purpose: WritingPurpose::Evidence,
            }],
        },
    )
    .unwrap();
    let snapshot = app
        .snapshot_writing_request("general-request", &note.id, 1, &thread, &[])
        .unwrap();
    let prepared = app
        .prepare_thread_run(ThreadRunStart {
            thread: thread.clone(),
            prompt: "unused".into(),
            request_message: None,
            queue_claim: None,
            owner_request: Some(OwnerRequest::RewriteNote {
                request_message: snapshot.request_message.clone(),
            }),
            sources: vec![],
            max_tool_rounds: Some(4),
            trace: TraceLevel::Full,
            offered_copilot_models: None,
        })
        .unwrap();
    // A general Note created DURING the run retains a conversation-private
    // source. It is visible in the selected chat, never canonical general.
    let op = app.store.allocate_operation("new-private-source").unwrap();
    let source = op.creation_id(0);
    owner_put(
        &mut app,
        "new-private-source",
        Put {
            id: source.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Source(Source {
                chat: Some(thread.clone()),
                locator: "synthetic:beacon-private".into(),
                label: "Private retained source".into(),
                observed_at: "synthetic".into(),
                external_version: None,
                outcome: "Synthetic retained text".into(),
                gaps: vec![],
                report: None,
            }),
        },
    );
    let op = app
        .store
        .allocate_operation("new-general-note-private-source")
        .unwrap();
    let other = op.creation_id(0);
    let mut private = Note::working("PRIVATE BEACON TITLE", "Beacon OTHER PRIVATE SENTINEL");
    private.import = Some(ImportCoverage {
        source,
        full_note: false,
        gaps: vec![],
        report: None,
    });
    owner_put(
        &mut app,
        "new-general-note-private-source",
        Put {
            id: other.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(private),
        },
    );
    assert_eq!(
        app.search_page("Beacon", Some(&thread), 10, None)
            .unwrap()
            .total,
        2
    );
    assert_eq!(app.search_page("Beacon", None, 10, None).unwrap().total, 1);
    let model = MockCompletionModel::from_stream_turns(vec![
        vec![
            MockStreamEvent::tool_call(
                "search",
                "search_notes",
                json!({"query":"Beacon","limit":10,"cursor":null,"mode":"keyword"}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "read",
                "read_records",
                json!({"ids":[note.id,other],"query":null}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "prepare",
                "prepare_changes",
                json!({"request_key":"general-proposal","request":snapshot.change_request("Permitted rewrite").unwrap(),"writing":null}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("Ready for exact review."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let result = run_prepared_scripted(prepared.prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    let delivery = serde_json::to_value(model.requests()).unwrap();
    let delivered = delivery.to_string();
    assert!(delivered.contains("WORKING POSITIVE CONTROL"));
    assert!(!delivered.contains("OTHER PRIVATE SENTINEL"));
    assert!(!delivered.contains("PRIVATE BEACON TITLE"));
    fn search_pages(value: &serde_json::Value, found: &mut Vec<serde_json::Value>) {
        if value
            .get("results")
            .is_some_and(serde_json::Value::is_array)
            && value.get("total").is_some()
        {
            found.push(value.clone());
        }
        match value {
            serde_json::Value::String(s) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                    search_pages(&parsed, found);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    search_pages(v, found);
                }
            }
            serde_json::Value::Object(o) => {
                for v in o.values() {
                    search_pages(v, found);
                }
            }
            _ => {}
        }
    }
    let mut pages = vec![];
    search_pages(&delivery, &mut pages);
    assert!(
        !pages.is_empty(),
        "actual host search tool result reached the scripted model"
    );
    assert!(pages.iter().all(|p| {
        p["total"] == 1
            && p["results"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["id"] != other)
    }));
    let candidate = app
        .candidates()
        .unwrap()
        .into_iter()
        .find(|c| c.request.writes.iter().any(|w| w.id == note.id))
        .unwrap();
    assert!(!candidate.request.inputs.iter().any(|i| i.record == other));
    let review = app
        .writing_candidate_review(candidate.operation.as_str(), None)
        .unwrap();
    assert!(review.sources.iter().all(|s| s.id != other));
}
