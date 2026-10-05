//! Owned Action suggestions over the actual shared worker, with synthetic provider hooks.
use super::*;

fn action_data() -> crate::actions::ActionData {
    crate::actions::ActionData {
        title: "  Approved waiting λ\r\n".into(),
        description: "\u{feff}Whole õ 🦀\r\n".into(),
        state: crate::actions::ActionState::Waiting,
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
        priority: None,
    }
}

fn action(worker: &AppWorker, id: Uuid) -> crate::actions::ActionRecord {
    let query = Uuid::new_v4();
    worker.submit(query, AppCommand::Action(id)).unwrap();
    match reply(worker, query) {
        AppEvent::Action(record) => *record,
        _ => panic!("Action"),
    }
}

fn action_review(
    worker: &AppWorker,
    mixed: bool,
) -> (ProposalRecord, crate::actions::ActionRecord) {
    use crate::proposals::ActionChange;
    let seed = Uuid::new_v4();
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::CreateProposal(DraftRequest {
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Seed review".into(),
                changes: vec![],
                sources: vec![],
                action_changes: vec![ActionChange::Create {
                    id: seed,
                    data: action_data(),
                }],
            }),
        )
        .unwrap();
    let AppEvent::Proposal(seed_review) = reply(worker, query) else {
        panic!("seed review")
    };
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::ApproveProposal(crate::proposal_apply::ApprovalRequest {
                operation_id: query,
                expected: seed_review.stamp(),
            }),
        )
        .unwrap();
    assert!(matches!(reply(worker, query), AppEvent::ProposalApplied(_)));
    let before = action(worker, seed);
    let note_id = Uuid::new_v4();
    let mut data = action_data();
    data.dependencies = vec![seed];
    data.parent = Some(seed);
    data.follows_up = Some(seed);
    if mixed {
        data.related_person = Some(note_id);
        data.related_project = Some(note_id);
        data.sources = vec![note_id];
        data.thread = Some(note_id);
    }
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::CreateProposal(DraftRequest {
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Whole Action rewrite".into(),
                changes: if mixed {
                    vec![DraftNoteChange::Create {
                        path: "action-note.md".into(),
                        text: format!("\u{feff}---\r\nbrn_id: {note_id}\r\n---\r\nExact õ\r\n"),
                    }]
                } else {
                    vec![]
                },
                sources: vec![],
                action_changes: vec![
                    ActionChange::Create {
                        id: Uuid::new_v4(),
                        data,
                    },
                    ActionChange::Replace {
                        before: Box::new(before.clone()),
                        data: before.data.clone(),
                    },
                ],
            }),
        )
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, query) else {
        panic!("Action draft")
    };
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::AddProposalComment(CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Keep all full Action evidence õ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            }),
        )
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, query) else {
        panic!("Action comment")
    };
    (record, before)
}

fn action_answer(record: &ProposalRecord) -> AiAnswer {
    let mut data: Vec<_> = record
        .draft
        .action_changes
        .iter()
        .map(|c| c.data().clone())
        .collect();
    for (index, value) in data.iter_mut().enumerate() {
        value.title = format!("\u{feff}Full suggestion {index} Õ\r\n");
        value.description.push_str("  Suggested 日本語\r\n");
        value.state = crate::actions::ActionState::Blocked;
        value.owner = Some("  Revised owner λ  ".into());
        value.due_on = Some("2028-03-01".into());
        value.follow_up_on = Some("2028-03-02".into());
        value.priority = Some(crate::actions::ActionPriority::High);
    }
    AiAnswer {
        text: serde_json::json!({"title":"Revised complete Actions", "texts": record.draft.changes.iter().map(|c| c.text()).collect::<Vec<_>>(), "action_data": data}).to_string(),
        terminal: AiTerminal::Completed,
    }
}

#[test]
fn owned_action_only_and_mixed_rewrite_require_exact_approval_then_replay_without_provider() {
    for mixed in [false, true] {
        let fixture = Fixture::new();
        let (entered, captured) = mpsc::channel();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = calls.clone();
        let hook: crate::proposal_rewrite::RewriteHook =
            Arc::new(move |selected, prompt, tools, _, _| {
                count.fetch_add(1, Ordering::SeqCst);
                let record: ProposalRecord = serde_json::from_str(&prompt).unwrap();
                entered.send((selected, record.clone())).unwrap();
                Box::pin(async move {
                    drop(tools);
                    action_answer(&record)
                })
            });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let (original, before) = action_review(&worker, mixed);
        assert_eq!(original.draft.vault.is_some(), mixed);
        let request = request(&original);
        start(&worker, &request);
        let (selected, captured) = captured.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(selected, request);
        assert_eq!(captured, original);
        let completed = finish(&worker, &request);
        assert_eq!(completed.status, RewriteStatus::Completed);
        let revised = review(&worker, original.draft.id);
        assert_eq!(revised.comments, original.comments);
        assert_eq!(revised.draft.vault, original.draft.vault);
        assert_eq!(revised.draft.sources, original.draft.sources);
        let result: serde_json::Value =
            serde_json::from_str(&action_answer(&original).text).unwrap();
        assert_eq!(
            serde_json::to_value(
                revised
                    .draft
                    .action_changes
                    .iter()
                    .map(|c| c.data())
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            result["action_data"]
        );
        assert_eq!(action(&worker, before.origin.id), before);
        let created_id = revised.draft.action_changes[0].id();
        let query = Uuid::new_v4();
        worker
            .submit(query, AppCommand::Action(created_id))
            .unwrap();
        assert!(
            matches!(reply(&worker, query), AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
        );
        assert!(!fixture.base.path().join("vault/action-note.md").exists());
        assert_eq!(
            std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
            b"current"
        );
        match (
            &original.draft.action_changes[1],
            &revised.draft.action_changes[1],
        ) {
            (
                crate::proposals::ActionChange::Replace { before: a, .. },
                crate::proposals::ActionChange::Replace { before: b, .. },
            ) => assert_eq!(a, b),
            _ => panic!("Replace binding"),
        }
        let query = Uuid::new_v4();
        worker
            .submit(
                query,
                AppCommand::ApproveProposal(crate::proposal_apply::ApprovalRequest {
                    operation_id: query,
                    expected: revised.stamp(),
                }),
            )
            .unwrap();
        assert!(matches!(
            reply(&worker, query),
            AppEvent::ProposalApplied(_)
        ));
        assert_eq!(
            action(&worker, created_id).data,
            revised.draft.action_changes[0].data().clone()
        );
        let changed = action(&worker, before.origin.id);
        assert_eq!(changed.origin, before.origin);
        assert_eq!(changed.version, before.version + 1);
        assert_eq!(changed.data, revised.draft.action_changes[1].data().clone());
        worker.shutdown().unwrap();
        let mut reopened = fixture.start(Hooks::default());
        reopened
            .submit(
                request.id,
                AppCommand::StartProposalRewrite(request.clone()),
            )
            .unwrap();
        assert_eq!(finish(&reopened, &request), completed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(action(&reopened, before.origin.id), changed);
        assert_eq!(
            std::fs::read_dir(fixture.base.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
        reopened.shutdown().unwrap();
    }
}

#[test]
fn owned_action_rewrite_incomplete_invalid_and_completed_results_leave_everything_unchanged() {
    for case in 0..7 {
        let fixture = Fixture::new();
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, prompt, tools, _, _| {
            let record: ProposalRecord = serde_json::from_str(&prompt).unwrap();
            let mut value: serde_json::Value =
                serde_json::from_str(&action_answer(&record).text).unwrap();
            match case {
                0 => {
                    value.as_object_mut().unwrap().remove("action_data");
                }
                1 => {
                    value["action_data"].as_array_mut().unwrap().pop();
                }
                2 => {
                    let extra = value["action_data"][0].clone();
                    value["action_data"].as_array_mut().unwrap().push(extra);
                }
                3 => value["action_data"][1]["state"] = serde_json::json!("completed"),
                4 => {
                    value["action_data"][1]
                        .as_object_mut()
                        .unwrap()
                        .remove("owner");
                }
                5 => value["action_data"][1]["before"] = serde_json::json!({}),
                _ => {
                    value["action_data"][1]["description"] =
                        serde_json::json!("x".repeat(64 * 1024 + 1))
                }
            }
            Box::pin(async move {
                drop(tools);
                AiAnswer {
                    text: value.to_string(),
                    terminal: AiTerminal::Completed,
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let (original, before) = action_review(&worker, true);
        let request = request(&original);
        start(&worker, &request);
        let failed = finish(&worker, &request);
        assert_eq!(failed.status, RewriteStatus::Failed, "case {case}");
        assert_eq!(failed.error_code.as_deref(), Some("tool_rejected"));
        assert_eq!(review(&worker, original.draft.id), original);
        assert_eq!(action(&worker, before.origin.id), before);
        assert!(!fixture.base.path().join("vault/action-note.md").exists());
        worker.shutdown().unwrap();
    }
}

#[test]
fn owned_action_rewrite_later_edit_or_comment_defeats_full_suggestion() {
    for comment in [false, true] {
        let fixture = Fixture::new();
        let (release, wait) = tokio::sync::oneshot::channel();
        let wait = Arc::new(Mutex::new(Some(wait)));
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, prompt, tools, _, _| {
            let wait = wait.lock().unwrap().take().unwrap();
            let record: ProposalRecord = serde_json::from_str(&prompt).unwrap();
            Box::pin(async move {
                wait.await.unwrap();
                drop(tools);
                action_answer(&record)
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let (original, before) = action_review(&worker, false);
        let request = request(&original);
        start(&worker, &request);
        let query = Uuid::new_v4();
        let command = if comment {
            AppCommand::AddProposalComment(CommentRequest {
                expected: original.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Later exact Action comment õ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
        } else {
            let mut data: Vec<_> = original
                .draft
                .action_changes
                .iter()
                .map(|c| c.data().clone())
                .collect();
            data[1].description = "Later explicit full Action edit õ\r\n".into();
            AppCommand::EditProposal(ProposalEdit {
                expected: original.stamp(),
                title: original.draft.title.clone(),
                texts: vec![],
                action_data: data,
            })
        };
        worker.submit(query, command).unwrap();
        let AppEvent::Proposal(later) = reply(&worker, query) else {
            panic!("later review")
        };
        release.send(()).unwrap();
        assert_eq!(finish(&worker, &request).status, RewriteStatus::Stale);
        assert_eq!(review(&worker, original.draft.id), later);
        assert_eq!(action(&worker, before.origin.id), before);
        worker.shutdown().unwrap();
    }
}

#[test]
fn owned_action_rewrite_stop_drains_retained_lease_and_restart_replays_interrupted_only() {
    let fixture = Fixture::new();
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, cancel, _| {
        let wait = wait.lock().unwrap().take().unwrap();
        let entered = entered.clone();
        std::thread::spawn(move || {
            entered.send(()).unwrap();
            wait.recv().unwrap();
            drop(tools);
        });
        Box::pin(async move {
            cancel.cancelled().await;
            AiAnswer {
                text: "Discard incomplete Actions".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let (original, before) = action_review(&worker, false);
    let request = request(&original);
    start(&worker, &request);
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let stop = Uuid::new_v4();
    worker
        .submit(stop, AppCommand::CancelTurn(request.id))
        .unwrap();
    assert!(matches!(
        reply(&worker, stop),
        AppEvent::TurnCancelRequested { accepted: true, .. }
    ));
    let query = Uuid::new_v4();
    worker
        .submit(query, AppCommand::ProposalRewrite(request.id))
        .unwrap();
    assert!(
        matches!(reply(&worker,query),AppEvent::ProposalRewrite(Some(job)) if job.status == RewriteStatus::Running)
    );
    assert_eq!(action(&worker, before.origin.id), before);
    release.send(()).unwrap();
    let interrupted = finish(&worker, &request);
    assert_eq!(interrupted.status, RewriteStatus::Interrupted);
    assert_eq!(review(&worker, original.draft.id), original);
    worker.shutdown().unwrap();
    let mut reopened = fixture.start(Hooks::default());
    reopened
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    assert_eq!(finish(&reopened, &request), interrupted);
    assert_eq!(action(&reopened, before.origin.id), before);
    reopened.shutdown().unwrap();
}

#[test]
fn fresh_source_free_action_rewrite_still_requires_an_ai_vault_without_provider_calls() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.vault_root = None;
    let hook: crate::proposal_rewrite::RewriteHook =
        Arc::new(|_, _, _, _, _| panic!("vaultless fresh Rewrite must not call provider"));
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        config,
        Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        },
        None,
        None,
    )
    .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Ready {
            vault_bound: false,
            ..
        }
    ));
    let (original, before) = action_review(&worker, false);
    let request = request(&original);
    worker
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    assert!(
        matches!(reply(&worker,request.id),AppEvent::Rewrite(RewriteEvent::Rejected { error,.. }) if error.kind == ErrorKind::VaultNotBound)
    );
    let query = Uuid::new_v4();
    worker
        .submit(query, AppCommand::ProposalRewrite(request.id))
        .unwrap();
    assert!(matches!(
        reply(&worker, query),
        AppEvent::ProposalRewrite(None)
    ));
    assert_eq!(review(&worker, original.draft.id), original);
    assert_eq!(action(&worker, before.origin.id), before);
    worker.shutdown().unwrap();
}

#[test]
fn owned_action_rewrite_uses_full_application_action_reads_and_keeps_real_records_unchanged() {
    let fixture = Fixture::new();
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, prompt, tools, _, _| {
        let captured: ProposalRecord = serde_json::from_str(&prompt).unwrap();
        Box::pin(async move {
            let copy = captured.clone();
            tokio::task::spawn_blocking(move || {
                let crate::proposals::ActionChange::Replace { before, .. } =
                    &copy.draft.action_changes[1]
                else {
                    panic!("captured baseline")
                };
                assert_eq!(
                    tools.read_action(&before.origin.id.to_string()).unwrap(),
                    serde_json::json!(before)
                );
                let page = tools.list_actions(Some("waiting"), 20, None).unwrap();
                assert_eq!(page["entries"], serde_json::json!([before]));
                assert!(page["next_cursor"].is_null());
                assert_eq!(tools.read_note("a.md").unwrap().text, "current");
            })
            .await
            .unwrap();
            action_answer(&captured)
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let (original, before) = action_review(&worker, false);
    let request = request(&original);
    start(&worker, &request);
    assert_eq!(finish(&worker, &request).status, RewriteStatus::Completed);
    assert_eq!(
        action(&worker, before.origin.id),
        before,
        "Suggestion is not approval"
    );
    assert_ne!(review(&worker, original.draft.id), original);
    worker.shutdown().unwrap();
}
