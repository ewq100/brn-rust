//! Owned Ask proposal callbacks; synthetic model hooks, exact ordinary approval.
use super::*;
use crate::{
    actions::{ActionData, ActionListRequest, ActionPriority, ActionState},
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, ProposalRecord, ProposalState},
};
use brn_ai::ActionProposalArgs;
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
fn create_args(proposal: Uuid, action: Uuid, source_paths: Vec<String>) -> ActionProposalArgs {
    ActionProposalArgs {
        id: proposal.to_string(),
        title: "Exact proposal õ\r\n".into(),
        source_paths,
        action_changes: vec![
            serde_json::to_value(ActionChange::Create {
                id: action,
                data: data(),
            })
            .unwrap(),
        ],
    }
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
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
    let proposal_id = Uuid::new_v4();
    let action_id = Uuid::new_v4();
    let args = create_args(proposal_id, action_id, vec![]);
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
    let new_id = Uuid::new_v4();
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
    let mut args = create_args(
        Uuid::new_v4(),
        new_id,
        vec!["source.md".into(), "archive/old.md".into()],
    );
    args.action_changes = vec![
        serde_json::to_value(ActionChange::Create {
            id: new_id,
            data: candidate.clone(),
        })
        .unwrap(),
        serde_json::to_value(ActionChange::Replace {
            before: Box::new(before.clone()),
            data: replacement.clone(),
        })
        .unwrap(),
    ];
    let steps = Arc::new(Mutex::new(vec![args.clone()]));
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(steps.clone())),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.selection.model = "gpt-6-luna".into();
    let (saved, results) = ask(&worker, &request);
    let review = proposal(&worker, Uuid::parse_str(&args.id).unwrap());
    assert_eq!(
        results[0]["ok"]["action_ids"],
        json!([new_id, before.origin.id])
    );
    assert_eq!(
        review.draft.action_changes,
        args.action_changes
            .iter()
            .cloned()
            .map(|v| serde_json::from_value(v).unwrap())
            .collect::<Vec<ActionChange>>()
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
        results[0]["ok"]["sources"],
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
    *steps.lock().unwrap() = vec![args.clone()];
    let mut retry = request.clone();
    retry.id = Uuid::new_v4();
    retry.conversation = Some(saved.conversation_id);
    let (_, results) = ask(&worker, &retry);
    assert_eq!(
        results[0]["ok"]["stamp"],
        serde_json::to_value(edited.stamp()).unwrap()
    );
    assert_eq!(proposal(&worker, edited.draft.id), edited);
    let mut changed_title = args.clone();
    changed_title.title.push('x');
    let mut changed_data = args.clone();
    changed_data.action_changes[0]["data"]["title"] = json!("Different original");
    let mut changed_order = args.clone();
    changed_order.action_changes.reverse();
    let mut changed_path = args.clone();
    changed_path.source_paths.reverse();
    *steps.lock().unwrap() = vec![changed_title, changed_data, changed_order, changed_path];
    retry.id = Uuid::new_v4();
    let (_, results) = ask(&worker, &retry);
    assert!(
        results.iter().all(|r| r["error"] == "tool_rejected"),
        "{results:?}"
    );
    *steps.lock().unwrap() = vec![args];
    retry.id = Uuid::new_v4();
    retry.conversation = None;
    let (_, results) = ask(&worker, &retry);
    assert_eq!(results[0]["error"], "tool_rejected");
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
    let valid = create_args(Uuid::new_v4(), Uuid::new_v4(), vec![]);
    let mut invalid = vec![];
    // Every nullable field must be explicitly present; serde alone would accept these omissions.
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
        let mut args = valid.clone();
        args.id = Uuid::new_v4().to_string();
        args.action_changes[0]["data"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        invalid.push(args);
    }
    for (field, value) in [
        ("extra", json!(true)),
        ("state", json!("completed")),
        ("due_on", json!("2028-02-30")),
        ("owner", json!("õ".repeat(257))),
        ("sources", json!([Uuid::nil()])),
        ("dependencies", json!([Uuid::new_v4()])),
        ("related_person", json!(Uuid::new_v4())),
    ] {
        let mut args = valid.clone();
        args.id = Uuid::new_v4().to_string();
        args.action_changes[0]["data"][field] = value;
        invalid.push(args);
    }
    for id in [Uuid::nil().to_string(), "not a UUID".into()] {
        let mut args = valid.clone();
        args.id = id;
        invalid.push(args);
    }
    let mut args = valid.clone();
    args.action_changes[0]["id"] = json!(Uuid::nil());
    invalid.push(args);
    let mut args = valid.clone();
    args.action_changes.push(args.action_changes[0].clone());
    invalid.push(args);
    let mut args = valid.clone();
    args.action_changes[0]["kind"] = json!("complete");
    invalid.push(args);
    let mut replace = valid.clone();
    replace.action_changes = vec![
        serde_json::to_value(ActionChange::Replace {
            before: Box::new(before.clone()),
            data: data(),
        })
        .unwrap(),
    ];
    for path in [
        "/before/waiting_since_ms",
        "/before/completed_at_ms",
        "/before/origin/data/priority",
    ] {
        let mut args = replace.clone();
        args.id = Uuid::new_v4().to_string();
        let (parent, field) = path.rsplit_once('/').unwrap();
        args.action_changes[0]
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        invalid.push(args);
    }
    let mut args = replace.clone();
    args.action_changes[0]["before"]["data"]["title"] = json!("stale full baseline");
    invalid.push(args);
    let mut args = replace;
    args.action_changes[0]["before"]["data"]["state"] = json!("completed");
    invalid.push(args);
    for path in [
        "../escape.md",
        "/absolute.md",
        ".brn/private.md",
        "missing.md",
    ] {
        let mut args = valid.clone();
        args.id = Uuid::new_v4().to_string();
        args.source_paths = vec![path.into()];
        invalid.push(args);
    }
    let steps = Arc::new(Mutex::new(invalid));
    let expected = steps.lock().unwrap().len();
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(script(steps)),
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
