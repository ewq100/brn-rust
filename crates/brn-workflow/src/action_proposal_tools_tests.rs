//! Owned Ask proposal callbacks; synthetic model hooks, exact ordinary approval.
use super::*;
use crate::{
    actions::{ActionData, ActionListRequest, ActionPriority, ActionState},
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, ProposalRecord, ProposalState},
};
use brn_ai::{ActionCandidate, ActionCandidateData, ActionProposalArgs, ActionRef};
use serde_json::{Value, json};
use sha2::Digest;

fn data() -> ActionData {
    ActionData {
        title: "  Complete õ  ".into(),
        description: "\u{feff}Exact 🦀\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("  Anna Õ  ".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(ActionPriority::High),
    }
}
fn candidate_data(data: &ActionData) -> ActionCandidateData {
    let mut value = serde_json::to_value(data).unwrap();
    for field in ["dependencies", "parent", "follows_up"] {
        if field == "dependencies" {
            value[field] = json!(
                data.dependencies
                    .iter()
                    .map(|id| ActionRef::Existing { id: id.to_string() })
                    .collect::<Vec<_>>()
            );
        } else {
            let id = if field == "parent" {
                data.parent
            } else {
                data.follows_up
            };
            value[field] = json!(id.map(|id| ActionRef::Existing { id: id.to_string() }));
        }
    }
    serde_json::from_value(value).unwrap()
}
fn create_args(source_paths: Vec<String>) -> ActionProposalArgs {
    ActionProposalArgs {
        title: "Exact proposal õ\r\n".into(),
        source_paths,
        action_changes: vec![ActionCandidate::Create {
            data: candidate_data(&data()),
        }],
    }
}
fn checked_ref(record: &crate::actions::ActionRecord) -> brn_ai::CheckedActionRef {
    brn_ai::CheckedActionRef {
        id: record.origin.id.to_string(),
        version: record.version,
        sha256: format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(record).unwrap())
        ),
    }
}
fn receipt_id(receipt: &Value) -> Uuid {
    Uuid::parse_str(receipt["stamp"]["id"].as_str().unwrap()).unwrap()
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    reply_at(worker, Uuid::new_v4(), command)
}
fn reply_at(worker: &AppWorker, id: Uuid, command: AppCommand) -> AppEvent {
    worker.submit(id, command).unwrap();
    loop {
        let (actual, value) = event(worker);
        if actual == id {
            return value;
        }
    }
}
fn proposal(worker: &AppWorker, id: Uuid) -> ProposalRecord {
    match reply(worker, AppCommand::Proposal(id)) {
        AppEvent::Proposal(p) => p,
        AppEvent::Failed(e) => panic!("{e:?}"),
        _ => panic!("wrong reply"),
    }
}
#[test]
fn ask_action_proposal_creates_only_full_review_with_inferred_session_then_exact_approval_and_replay()
 {
    let fixture = Fixture::new();
    let args = create_args(vec![]);
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called = calls.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, reads, proposals, _, _| {
        let args = args.clone();
        let called = called.clone();
        Box::pin(async move {
            let receipt = tokio::task::spawn_blocking(move || {
                called.fetch_add(1, Ordering::SeqCst);
                assert!(
                    reads.list_actions(None, 20, None).unwrap()["entries"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(reads.read_note("a.md").unwrap().text, "current");
                proposals.propose_actions(args).unwrap()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&receipt).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let saved = terminal(&worker, request.id);
    assert_eq!(saved.status, WorkTurnStatus::Completed);
    let receipt: Value = serde_json::from_str(&saved.answer).unwrap();
    let proposal_id = receipt_id(&receipt);
    let action_id = Uuid::parse_str(receipt["action_ids"][0].as_str().unwrap()).unwrap();
    assert_ne!(proposal_id, action_id);
    assert_eq!(proposal_id.get_version_num(), 8);
    assert_eq!(action_id.get_version_num(), 8);
    let review = proposal(&worker, proposal_id);
    assert_eq!(
        receipt["stamp"],
        serde_json::to_value(review.stamp()).unwrap()
    );
    assert_eq!(receipt["state"], "draft");
    assert_eq!(review.state, ProposalState::Draft);
    assert_eq!(review.draft.session_id, Some(saved.conversation_id));
    assert_eq!(receipt["session_id"], saved.conversation_id.to_string());
    assert_eq!(receipt["action_ids"], json!([action_id]));
    assert_eq!(
        review.draft.action_changes,
        vec![ActionChange::Create {
            id: action_id,
            data: data()
        }]
    );
    assert!(review.draft.changes.is_empty());
    assert!(review.draft.sources.is_empty());
    assert!(
        matches!(reply(&worker,AppCommand::Actions(ActionListRequest::default())),AppEvent::Actions(page)if page.entries.is_empty())
    );
    let operation_id = Uuid::new_v4();
    worker
        .submit(
            operation_id,
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id,
                expected: review.stamp(),
            }),
        )
        .unwrap();
    loop {
        let (id, event) = event(&worker);
        if id == operation_id {
            assert!(matches!(event, AppEvent::ProposalApplied(_)));
            break;
        }
    }
    match reply(&worker, AppCommand::Action(action_id)) {
        AppEvent::Action(record) => {
            assert_eq!(record.data, data());
            assert_eq!(record.origin.data, data());
            assert_eq!(record.origin.proposal, review.stamp());
        }
        AppEvent::Failed(e) => panic!("{e:?}"),
        _ => panic!("wrong reply"),
    }
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks::default());
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(
        serde_json::to_value(terminal(&worker, request.id)).unwrap(),
        serde_json::to_value(&saved).unwrap()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        matches!(reply(&worker,AppCommand::Proposals(None)),AppEvent::Proposals(rows)if rows.len()==1)
    );
    worker.shutdown().unwrap();
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
    assert_eq!(
        std::fs::read_dir(fixture.base.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}

fn script(steps: Arc<Mutex<Vec<ActionProposalArgs>>>) -> ProposalAnswerHook {
    Arc::new(move |_, _, _, proposals, _, _| {
        let steps = std::mem::take(&mut *steps.lock().unwrap());
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                steps
                    .into_iter()
                    .map(|args| match proposals.propose_actions(args) {
                        Ok(value) => json!({"ok": value}),
                        Err(error) => json!({"error": error.kind}),
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    })
}
fn ask(worker: &AppWorker, request: &AskRequest) -> (WorkTurn, Vec<Value>) {
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let saved = terminal(worker, request.id);
    assert_eq!(saved.status, WorkTurnStatus::Completed);
    let results = serde_json::from_str(&saved.answer).unwrap();
    (saved, results)
}
fn seed(fixture: &Fixture) -> crate::actions::ActionRecord {
    let mut app =
        crate::app::App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    let id = Uuid::new_v4();
    let draft = app
        .create_proposal(&crate::proposals::DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Approved baseline".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create { id, data: data() }],
        })
        .unwrap();
    app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: draft.stamp(),
    })
    .unwrap();
    app.action(id).unwrap()
}
#[test]
fn action_proposal_full_replace_sources_and_original_replay_preserve_newer_review_after_source_loss()
 {
    let fixture = Fixture::new();
    let before = seed(&fixture);
    let source_id = Uuid::new_v4();
    let full_source = format!(
        "\u{feff}---\r\nbrn_id: {source_id}\r\n---\r\n{}",
        "Full õ 🦀\r\n".repeat(6_000)
    );
    assert!(full_source.len() > brn_ai::READ_NOTE_BYTES);
    std::fs::write(fixture.base.path().join("vault/source.md"), &full_source).unwrap();
    std::fs::create_dir(fixture.base.path().join("vault/archive")).unwrap();
    let historical = "\u{feff}---\r\nbrn_state: history\r\n---\r\nOriginal evidence õ\r\n";
    std::fs::write(fixture.base.path().join("vault/archive/old.md"), historical).unwrap();
    let mut candidate = data();
    candidate.related_person = Some(source_id);
    candidate.related_project = Some(source_id);
    candidate.sources = vec![source_id];
    candidate.thread = Some(source_id);
    candidate.dependencies = vec![before.origin.id];
    candidate.parent = Some(before.origin.id);
    candidate.follows_up = Some(before.origin.id);
    let mut replacement = data();
    replacement.title = "\u{feff}Exact replacement õ\r\n".into();
    replacement.state = ActionState::Blocked;
    let mut args = create_args(vec!["source.md".into(), "archive/old.md".into()]);
    let reference = checked_ref(&before);
    args.action_changes = vec![
        ActionCandidate::Create {
            data: candidate_data(&candidate),
        },
        ActionCandidate::Replace {
            target: reference,
            data: candidate_data(&replacement),
        },
    ];
    // Keep the actual owned turn active while owner review and saved evidence
    // change. Original replay is scoped to that turn, never a later Ask.
    let (sent, received) = std::sync::mpsc::channel();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, cancel, _| {
        sent.send(proposals).unwrap();
        Box::pin(async move {
            cancel.cancelled().await;
            AiAnswer {
                text: "Stopped".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let lease = received
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let receipt = lease.propose_actions(args.clone()).unwrap();
    let review = proposal(&worker, receipt_id(&receipt));
    let new_id = Uuid::parse_str(receipt["action_ids"][0].as_str().unwrap()).unwrap();
    assert_eq!(receipt["action_ids"], json!([new_id, before.origin.id]));
    assert_eq!(
        review.draft.action_changes,
        vec![
            ActionChange::Create {
                id: new_id,
                data: candidate.clone()
            },
            ActionChange::Replace {
                before: Box::new(before.clone()),
                data: replacement.clone()
            },
        ]
    );
    for (source, text) in review
        .draft
        .sources
        .iter()
        .zip([full_source.as_str(), historical])
    {
        assert_eq!(source.fingerprint.len, text.len() as u64);
        assert_eq!(
            source.fingerprint.sha256,
            <[u8; 32]>::from(sha2::Sha256::digest(text.as_bytes()))
        );
    }
    assert_eq!(
        receipt["sources"],
        serde_json::to_value(&review.draft.sources).unwrap()
    );
    assert!(
        matches!(reply(&worker, AppCommand::Action(before.origin.id)),AppEvent::Action(r)if *r==before)
    );
    assert!(
        matches!(reply(&worker, AppCommand::Action(new_id)),AppEvent::Failed(e)if e.kind==ErrorKind::NotFound)
    );
    let mut newer = candidate;
    newer.description = "Later manual whole work õ\r\n".into();
    let edited = match reply(
        &worker,
        AppCommand::EditProposal(crate::proposals::ProposalEdit {
            expected: review.stamp(),
            title: "Later manual title".into(),
            texts: vec![],
            action_data: vec![newer, replacement],
        }),
    ) {
        AppEvent::Proposal(p) => p,
        _ => panic!("edit failed"),
    };
    std::fs::remove_file(fixture.base.path().join("vault/source.md")).unwrap();
    std::fs::remove_file(fixture.base.path().join("vault/archive/old.md")).unwrap();
    let completion = crate::action_completion::CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before.clone()),
    };
    assert!(matches!(
        reply_at(
            &worker,
            completion.operation_id,
            AppCommand::CompleteAction(completion)
        ),
        AppEvent::ActionCompleted(_)
    ));
    let replay = lease.propose_actions(args.clone()).unwrap();
    assert_eq!(
        replay["stamp"],
        serde_json::to_value(edited.stamp()).unwrap()
    );
    assert_eq!(proposal(&worker, edited.draft.id), edited);
    let mut changed_title = args.clone();
    changed_title.title.push('x');
    let mut changed_data = args.clone();
    if let ActionCandidate::Create { data } = &mut changed_data.action_changes[0] {
        data.title = "Different original".into();
    }
    let mut changed_order = args.clone();
    changed_order.action_changes.reverse();
    let mut changed_path = args.clone();
    changed_path.source_paths.reverse();
    for changed in [changed_title, changed_data, changed_order, changed_path] {
        assert_eq!(
            lease.propose_actions(changed).unwrap_err().kind,
            brn_ai::AiErrorKind::IndexStale
        );
    }
    reply(&worker, AppCommand::CancelTurn(request.id));
    drop(lease);
    let saved = terminal(&worker, request.id);
    assert_eq!(saved.status, WorkTurnStatus::Interrupted);
    worker.shutdown().unwrap();
    let steps = Arc::new(Mutex::new(vec![args]));
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(steps)),
        ..Hooks::default()
    });
    let mut retry = request;
    retry.id = Uuid::new_v4();
    retry.conversation = Some(saved.conversation_id);
    let (_, results) = ask(&worker, &retry);
    assert_eq!(results[0]["error"], "index_stale");
    assert_eq!(proposal(&worker, edited.draft.id), edited);
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks::default());
    assert_eq!(proposal(&worker, edited.draft.id), edited);
    worker.shutdown().unwrap();
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
    assert_eq!(
        std::fs::read_dir(fixture.base.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn action_proposal_partial_unknown_nil_stale_and_completed_inputs_refuse_without_durable_effects() {
    let fixture = Fixture::new();
    let before = seed(&fixture);
    let valid = create_args(vec![]);
    let value = serde_json::to_value(&valid).unwrap();
    // Strict protocol rejects legacy model IDs/full before records, omitted
    // nullable fields, unknown fields and Completed before owner dispatch.
    for field in [
        "owner",
        "related_person",
        "related_project",
        "thread",
        "due_on",
        "follow_up_on",
        "parent",
        "follows_up",
        "priority",
    ] {
        let mut input = value.clone();
        input["action_changes"][0]["data"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<ActionProposalArgs>(input).is_err());
    }
    for (path, injected) in [("id", json!(Uuid::new_v4())), ("legacy", json!(true))] {
        let mut input = value.clone();
        input[path] = injected;
        assert!(serde_json::from_value::<ActionProposalArgs>(input).is_err());
    }
    for (field, injected) in [
        ("id", json!(Uuid::new_v4())),
        ("before", json!(before)),
        ("extra", json!(true)),
    ] {
        let mut input = value.clone();
        input["action_changes"][0][field] = injected;
        assert!(serde_json::from_value::<ActionProposalArgs>(input).is_err());
    }
    let mut completed = value;
    completed["action_changes"][0]["data"]["state"] = json!("completed");
    assert!(serde_json::from_value::<ActionProposalArgs>(completed).is_err());
    let mut invalid = vec![];
    for (field, value) in [
        ("due_on", json!("2028-02-30")),
        ("owner", json!("õ".repeat(257))),
        ("sources", json!([Uuid::nil()])),
        (
            "dependencies",
            json!([ActionRef::Existing {
                id: Uuid::new_v4().to_string()
            }]),
        ),
        ("related_person", json!(Uuid::new_v4())),
        ("thread", json!("not a UUID")),
        ("dependencies", json!([ActionRef::Member { index: 2 }])),
        ("parent", json!(ActionRef::Member { index: 1 })),
    ] {
        let mut input = serde_json::to_value(&valid).unwrap();
        input["action_changes"][0]["data"][field] = value;
        invalid.push(serde_json::from_value(input).unwrap());
    }
    let target = checked_ref(&before);
    let mut stale_version = target.clone();
    stale_version.version += 1;
    let mut divergent = before.clone();
    divergent.data.title.push_str(" different exact bytes");
    let mut nil = target.clone();
    nil.id = Uuid::nil().to_string();
    for target in [stale_version, checked_ref(&divergent), nil] {
        let mut input = valid.clone();
        input.action_changes = vec![ActionCandidate::Replace {
            target,
            data: candidate_data(&data()),
        }];
        invalid.push(input);
    }
    let mut duplicate = valid.clone();
    let replace = ActionCandidate::Replace {
        target,
        data: candidate_data(&data()),
    };
    duplicate.action_changes = vec![replace.clone(), replace];
    invalid.push(duplicate);
    for path in [
        "../escape.md",
        "/absolute.md",
        ".brn/private.md",
        "missing.md",
    ] {
        let mut args = valid.clone();
        args.source_paths = vec![path.into()];
        invalid.push(args);
    }
    let expected = invalid.len();
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(Arc::new(Mutex::new(invalid)))),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    let (_, results) = ask(&worker, &request);
    assert_eq!(results.len(), expected);
    assert!(
        results.iter().all(|r| r.get("error").is_some()),
        "{results:?}"
    );
    assert!(
        matches!(reply(&worker,AppCommand::Proposals(None)),AppEvent::Proposals(p)if p.len()==1)
    );
    assert!(
        matches!(reply(&worker,AppCommand::Action(before.origin.id)),AppEvent::Action(r)if *r==before)
    );
    worker.shutdown().unwrap();
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
    assert_eq!(
        std::fs::read_dir(fixture.base.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn action_member_references_resolve_mixed_drafts_and_full_approval_still_checks_the_baseline() {
    let fixture = Fixture::new();
    let before = seed(&fixture);
    let mut input = create_args(vec![]);
    let mut first = candidate_data(&data());
    first.dependencies = vec![ActionRef::Member { index: 2 }];
    first.parent = Some(ActionRef::Member { index: 2 });
    first.follows_up = Some(ActionRef::Member { index: 2 });
    input.action_changes = vec![
        ActionCandidate::Create { data: first },
        ActionCandidate::Replace {
            target: checked_ref(&before),
            data: candidate_data(&data()),
        },
    ];
    let mut cycle = input.clone();
    if let ActionCandidate::Replace { data, .. } = &mut cycle.action_changes[1] {
        data.dependencies.push(ActionRef::Member { index: 1 });
    }
    let mut hierarchy_cycle = input.clone();
    if let ActionCandidate::Replace { data, .. } = &mut hierarchy_cycle.action_changes[1] {
        data.parent = Some(ActionRef::Member { index: 1 });
    }
    let mut changed = input.clone();
    changed.title.push_str(" distinct intent");
    let steps = Arc::new(Mutex::new(vec![
        input.clone(),
        input.clone(),
        cycle,
        hierarchy_cycle,
        changed,
    ]));
    let later = steps.clone();
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(steps)),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    let (_, results) = ask(&worker, &request);
    assert_eq!(results[0], results[1]);
    assert!(
        results[2..4]
            .iter()
            .all(|result| result["error"] == "tool_rejected")
    );
    assert_ne!(
        results[4]["ok"]["stamp"]["id"],
        results[0]["ok"]["stamp"]["id"]
    );
    *later.lock().unwrap() = vec![input];
    let mut other = request.clone();
    other.id = Uuid::new_v4();
    let (_, another) = ask(&worker, &other);
    assert_ne!(
        another[0]["ok"]["stamp"]["id"],
        results[0]["ok"]["stamp"]["id"]
    );
    assert_ne!(
        another[0]["ok"]["action_ids"][0],
        results[0]["ok"]["action_ids"][0]
    );
    let review = proposal(&worker, receipt_id(&results[0]["ok"]));
    assert_eq!(
        review.draft.action_changes[0].data().dependencies,
        vec![before.origin.id]
    );
    assert_eq!(
        review.draft.action_changes[0].data().parent,
        Some(before.origin.id)
    );
    assert_eq!(
        review.draft.action_changes[0].data().follows_up,
        Some(before.origin.id)
    );
    assert_eq!(review.draft.action_changes[1].id(), before.origin.id);
    let completion = crate::action_completion::CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before),
    };
    assert!(matches!(
        reply_at(
            &worker,
            completion.operation_id,
            AppCommand::CompleteAction(completion)
        ),
        AppEvent::ActionCompleted(_)
    ));
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert!(matches!(
        reply_at(
            &worker,
            approval.operation_id,
            AppCommand::ApproveProposal(approval)
        ),
        AppEvent::Failed(_)
    ));
    assert!(
        matches!(reply(&worker, AppCommand::Actions(ActionListRequest::default())),AppEvent::Actions(page)if page.entries.len()==1)
    );
    worker.shutdown().unwrap();
}

#[test]
fn ordinary_twenty_member_semantic_draft_resolves_forward_and_backward_member_links() {
    let fixture = Fixture::new();
    let mut input = create_args(vec![]);
    input.action_changes = (0..20)
        .map(|index| {
            let mut after = candidate_data(&data());
            after.title = format!("Owned member {index}");
            if index > 0 {
                after.dependencies = vec![ActionRef::Member { index }];
            }
            if index == 0 {
                after.follows_up = Some(ActionRef::Member { index: 20 });
            }
            ActionCandidate::Create { data: after }
        })
        .collect();
    let mut oversized = input.clone();
    oversized
        .action_changes
        .push(oversized.action_changes[0].clone());
    let steps = Arc::new(Mutex::new(vec![input.clone(), input, oversized]));
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(steps)),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    let (_, results) = ask(&worker, &request);
    assert_eq!(results[0], results[1]);
    assert_eq!(results[2]["error"], "tool_rejected");
    let review = proposal(&worker, receipt_id(&results[0]["ok"]));
    let ids: Vec<_> = review
        .draft
        .action_changes
        .iter()
        .map(ActionChange::id)
        .collect();
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        20
    );
    assert_eq!(
        review.draft.action_changes[0].data().follows_up,
        Some(ids[19])
    );
    for (index, change) in review.draft.action_changes.iter().enumerate().skip(1) {
        assert_eq!(change.data().dependencies, vec![ids[index - 1]]);
    }
    assert!(
        matches!(reply(&worker,AppCommand::Actions(ActionListRequest::default())),AppEvent::Actions(page)if page.entries.is_empty())
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert!(matches!(
        reply_at(
            &worker,
            approval.operation_id,
            AppCommand::ApproveProposal(approval)
        ),
        AppEvent::ProposalApplied(_)
    ));
    assert!(
        matches!(reply(&worker,AppCommand::Actions(ActionListRequest::default())),AppEvent::Actions(page)if page.entries.len()==20)
    );
    worker.shutdown().unwrap();
}
