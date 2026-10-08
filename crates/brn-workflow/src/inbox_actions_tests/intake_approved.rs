//! Retained collection investigation after offline Source approval and restart.
use super::*;
use crate::app::App;

fn approve_source(
    worker: &AppWorker,
) -> (
    crate::inbox_processing::InboxConversionPreview,
    ProposalRecord,
) {
    let (preview, source) = intake(
        worker,
        include_bytes!(
            "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
        ),
        "plural.eml",
    );
    assert_eq!(approved(worker, &source).outcome, ApplyOutcome::Applied);
    (preview, source)
}

fn binding(
    worker: &AppWorker,
    source: &ProposalRecord,
) -> crate::inbox_actions::InboxIntakeBinding {
    let event = reply(
        worker,
        AppCommand::InboxIntakeBinding {
            source_proposal_id: source.draft.id,
        },
    );
    let AppEvent::InboxIntakeBinding(binding) = event else {
        panic!("approved retained Source must be investigable");
    };
    *binding
}

fn request(binding: crate::inbox_actions::InboxIntakeBinding) -> InboxActionRequest {
    InboxActionRequest {
        intake: Some(binding),
        source: None,
        visual_asset: None,
        purpose: InboxAnalysisPurpose::KnowledgeAndActions,
        id: Uuid::new_v4(),
        conversation: None,
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6-luna".into(),
        },
        effort: crate::ReasoningEffort::Medium,
        generation: 801,
    }
}

#[test]
fn approved_retained_source_investigation_preserves_images_and_approves_only_new_consequences() {
    let f = Fixture::new();
    let mut worker = f.start(Hooks::default());
    let (preview, source) = approve_source(&worker);
    let extraction = preview.extraction.as_ref().unwrap();
    let docx = extraction
        .sources
        .iter()
        .find(|s| s.name == "harbor.docx")
        .unwrap();
    let source_path = f.base.path().join("vault/harbor-source.md");
    let source_bytes = std::fs::read(&source_path).unwrap();
    let assets = source
        .draft
        .changes
        .iter()
        .skip(1)
        .map(|change| {
            let path = f.base.path().join("vault").join(change.path());
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    worker.shutdown().unwrap();

    let knowledge: KnowledgeProposalArgs = serde_json::from_value(json!({
        "supersedes": null, "title": "Harbor extension after retained evidence review",
        "path": "harbor-reviewed.md", "source_paths": [],
        "text": "# Harbor extension\n\nThe document proposes extending the pilot to Pier B on 15 October 2026, owned by Mira. The retained chart reports 120 to 72 litres/day, a 40% reduction.\n",
        "quotes": [{"quote": "Decision: extend the pilot to Pier B on 15 October 2026. Owner: Mira.", "source_id": docx.id}]
    })).unwrap();
    let mut action = args();
    candidate_data_mut(&mut action).title = "Review the Harbor extension with Mira".into();
    candidate_data_mut(&mut action).owner = Some("Mira".into());
    let script = scripted(Arc::new(Mutex::new(vec![
        Step::Knowledge(knowledge),
        Step::Action(action),
    ])));
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let hook: ProposalAnswerHook = Arc::new(move |ask, history, read, tools, cancel, emit| {
        called.fetch_add(1, Ordering::SeqCst);
        let evidence: Value =
            serde_json::from_str(ask.question.rsplit_once("\n\n").unwrap().1).unwrap();
        assert_eq!(evidence["source_approval"], "applied");
        assert_eq!(evidence["assets"].as_array().unwrap().len(), 1);
        assert_eq!(evidence["image_occurrences"].as_array().unwrap().len(), 3);
        assert_eq!(evidence["source_nodes"].as_array().unwrap().len(), 8);
        assert_eq!(evidence["assets"][0]["included_in_visual_input"], true);
        script(ask, history, read, tools, cancel, emit)
    });
    let mut worker = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let binding = binding(&worker, &source);
    assert_eq!(
        binding.source_proposal,
        source.stamp(),
        "use the exact approved Draft stamp, not the later Applied review stamp"
    );
    assert_eq!(binding.assets.len(), 1);
    assert_eq!(binding.occurrences.len(), 3);
    let request = request(binding);
    let turn = analyze(&worker, &request).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    assert!(
        results(&turn).iter().all(|r| r.get("ok").is_some()),
        "{}",
        turn.answer
    );
    let proposals = analysis(&worker, request.id).proposals;
    assert_eq!(proposals.len(), 2);
    assert!(
        proposals
            .iter()
            .all(|p| p.draft.id != source.draft.id && p.draft.inbox_source.is_none())
    );
    assert_eq!(std::fs::read(&source_path).unwrap(), source_bytes);
    // Approval must work in a fresh process without analysis/editor preparation
    // having opened the vault adapter. Put the vault-free Action first too.
    worker.shutdown().unwrap();
    let mut worker = f.start(Hooks::default());
    let mut consequences = proposals.iter().collect::<Vec<_>>();
    consequences.sort_by_key(|p| p.draft.action_changes.is_empty());
    let group = GroupApprovalRequest {
        group_id: request.id,
        approvals: consequences
            .into_iter()
            .map(|p| ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: p.stamp(),
            })
            .collect(),
    };
    let AppEvent::ProposalGroupApplied(applied) =
        reply(&worker, AppCommand::ApproveProposalGroup(group))
    else {
        panic!("consequence group");
    };
    assert!(applied.stopped.is_none(), "{applied:?}");
    assert_eq!(applied.receipts.len(), 2);
    assert!(
        applied
            .receipts
            .iter()
            .all(|r| r.outcome == ApplyOutcome::Applied)
    );
    assert!(
        std::fs::read_to_string(f.base.path().join("vault/harbor-reviewed.md"))
            .unwrap()
            .contains("40% reduction")
    );
    let action = proposals
        .iter()
        .find(|p| !p.draft.action_changes.is_empty())
        .unwrap();
    let AppEvent::Action(saved) = reply(
        &worker,
        AppCommand::Action(action.draft.action_changes[0].id()),
    ) else {
        panic!("saved Action");
    };
    assert_eq!(saved.data.owner.as_deref(), Some("Mira"));
    worker.shutdown().unwrap();
    let mut worker = f.start(Hooks {
        proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
            panic!("retained analysis must not rerun inference")
        })),
        ..Hooks::default()
    });
    let replay = analyze(&worker, &request).unwrap();
    assert_eq!(replay.id, turn.id);
    assert_eq!(replay.status, turn.status);
    assert_eq!(replay.question, turn.question);
    assert_eq!(replay.answer, turn.answer);
    assert_eq!(std::fs::read(source_path).unwrap(), source_bytes);
    for (path, bytes) in assets {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.shutdown().unwrap();
    let app = App::open(&f.base.path().join("data"), f.config()).unwrap();
    assert_eq!(
        app.proposals(None).unwrap().len(),
        3,
        "one original Source and two new consequence proposals only"
    );
}

#[test]
fn approved_retained_source_rechecks_source_image_original_and_identity_before_inference() {
    for damage in ["source", "image", "original", "identity"] {
        let f = Fixture::new();
        let mut worker = f.start(Hooks {
            proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
                panic!("changed evidence must refuse before inference")
            })),
            ..Hooks::default()
        });
        let (preview, source) = approve_source(&worker);
        let request = request(binding(&worker, &source));
        let source_path = f.base.path().join("vault/harbor-source.md");
        match damage {
            "source" => {
                let mut text = std::fs::read(&source_path).unwrap();
                text.extend_from_slice(b"\nChanged since approval.\n");
                std::fs::write(source_path, text).unwrap();
            }
            "image" => std::fs::write(
                f.base
                    .path()
                    .join("vault")
                    .join(source.draft.changes[1].path()),
                b"changed image",
            )
            .unwrap(),
            "original" => std::fs::write(
                preview
                    .original
                    .capture
                    .copy
                    .directory
                    .join(preview.original.capture.copy_name()),
                b"changed original",
            )
            .unwrap(),
            "identity" => {
                std::fs::copy(source_path, f.base.path().join("vault/duplicate-source.md"))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let error = analyze(&worker, &request).expect_err(damage);
        assert_eq!(error.kind, ErrorKind::ContextStale, "{damage}: {error:?}");
        worker.shutdown().unwrap();
    }
}

#[test]
fn approved_retained_source_individual_action_approval_after_restart_checks_exact_evidence() {
    for damage in [None, Some("image"), Some("source")] {
        let f = Fixture::new();
        let mut worker = f.start(Hooks {
            proposal_answer: Some(scripted(Arc::new(Mutex::new(vec![Step::Action(args())])))),
            ..Hooks::default()
        });
        let (_, source) = approve_source(&worker);
        let request = request(binding(&worker, &source));
        assert_eq!(
            analyze(&worker, &request).unwrap().status,
            WorkTurnStatus::Completed
        );
        let proposals = analysis(&worker, request.id).proposals;
        assert_eq!(proposals.len(), 1);
        let action = &proposals[0];
        worker.shutdown().unwrap();
        let mut app = App::open(&f.base.path().join("data"), f.config()).unwrap();
        assert!(app.editor.files.is_none());
        if damage.is_none() {
            // A fresh manually submitted retained Action draft needs the same
            // acquisition, independently of provider preparation or approval.
            let draft = &action.draft;
            let created = app
                .create_proposal(&crate::proposals::DraftRequest {
                    id: Uuid::new_v4(),
                    group_id: draft.group_id,
                    session_id: draft.session_id,
                    title: draft.title.clone(),
                    changes: Vec::new(),
                    sources: draft.sources.clone(),
                    action_changes: draft.action_changes.clone(),
                    intake: draft.intake.clone(),
                    inbox_knowledge: None,
                    inbox_visual: None,
                    inbox_source: None,
                })
                .unwrap();
            assert_eq!(created.state, ProposalState::Draft);
            assert!(app.action(draft.action_changes[0].id()).is_err());
            drop(app);
            app = App::open(&f.base.path().join("data"), f.config()).unwrap();
            assert!(app.editor.files.is_none());
        }
        if let Some(damage) = damage {
            let path = if damage == "image" {
                source.draft.changes[1].path()
            } else {
                source.draft.changes[0].path()
            };
            // Keep the Source UUID readable so the byte check, not identity loss,
            // distinguishes the changed Source from the uninitialized adapter.
            let path = f.base.path().join("vault").join(path);
            let mut bytes = std::fs::read(&path).unwrap();
            bytes.extend_from_slice(b"\nchanged after investigation\n");
            std::fs::write(path, bytes).unwrap();
        }
        let approve = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: action.stamp(),
        };
        let result = app.approve_proposal(&approve);
        match damage {
            None => {
                assert_eq!(result.unwrap().outcome, ApplyOutcome::Applied);
                assert!(app.action(action.draft.action_changes[0].id()).is_ok());
            }
            Some(damage) => {
                let error = result.unwrap_err();
                assert_eq!(error.kind, ErrorKind::ContextStale);
                assert!(
                    error.message.contains(if damage == "image" {
                        "Applied Source asset differs"
                    } else {
                        "Applied Source differs"
                    }),
                    "{error:?}"
                );
                assert!(
                    app.store
                        .proposal_apply(approve.operation_id)
                        .unwrap()
                        .is_none()
                );
                assert!(app.action(action.draft.action_changes[0].id()).is_err());
                assert_eq!(
                    app.proposal(action.draft.id).unwrap().state,
                    ProposalState::Draft
                );
            }
        }
    }
}

#[test]
fn approved_retained_source_binding_requires_exact_receipt_and_refuses_unsettled_states() {
    let f = Fixture::new();
    let mut worker = f.start(Hooks::default());
    let (_, source) = approve_source(&worker);
    worker.shutdown().unwrap();
    let app = App::open(&f.base.path().join("data"), f.config()).unwrap();
    assert!(app.intake_analysis_binding(source.draft.id).is_ok());
    // Simulate a missing checked authority record after startup. The binding
    // query must not invent a receipt from an Applied label or pick history.
    let conn = rusqlite::Connection::open(f.base.path().join("data/brn.sqlite")).unwrap();
    conn.execute(
        "DELETE FROM proposal_applies WHERE proposal_id=?1",
        [source.draft.id.to_string()],
    )
    .unwrap();
    assert!(app.intake_analysis_binding(source.draft.id).is_err());
    drop((conn, app));

    for applying in [false, true] {
        let f = Fixture::new();
        let mut worker = f.start(Hooks::default());
        let (_, source) = intake(
            &worker,
            include_bytes!(
                "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
            ),
            "plural.eml",
        );
        worker.shutdown().unwrap();
        let mut app = App::open(&f.base.path().join("data"), f.config()).unwrap();
        if applying {
            app.store
                .begin_proposal_apply(&ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: source.stamp(),
                })
                .unwrap();
        } else {
            app.reject_proposal(source.stamp()).unwrap();
        }
        assert!(app.intake_analysis_binding(source.draft.id).is_err());
    }
}
