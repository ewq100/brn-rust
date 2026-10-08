//! Full approved Action tools on the real worker; synthetic model hooks only.
use super::*;
use crate::{
    actions::{ActionData, ActionListRequest, ActionRecord, ActionState},
    app::App,
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, DraftRequest},
};
use serde_json::json;

fn ai_record(record: &ActionRecord) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(record).unwrap()));
    let mut result = serde_json::to_value(record).unwrap();
    result["checked_ref"] = json!({"id":record.origin.id,"version":record.version,"sha256":hash});
    result
}

fn seed(fixture: &Fixture, large: bool) -> Vec<ActionRecord> {
    let mut app = App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    for (index, state) in [
        ActionState::Open,
        ActionState::Waiting,
        ActionState::Blocked,
        ActionState::Open,
    ]
    .into_iter()
    .enumerate()
    // Two large records suffice for the whole-page limit; the small fixture covers all states.
    .take(if large { 2 } else { 4 })
    {
        let id = Uuid::new_v4();
        let draft = app
            .create_proposal(&DraftRequest {
                intake: None,
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Explicit approved seed".into(),
                changes: vec![],
                sources: vec![],
                action_changes: vec![ActionChange::Create {
                    id,
                    data: ActionData {
                        title: "  Exact approved õ\r\n".into(),
                        description: if large {
                            "\u{1}".repeat(64 * 1024)
                        } else {
                            "\u{feff}Whole 🦀\r\n".into()
                        },
                        state,
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
                        priority: Some(crate::actions::ActionPriority::High),
                    },
                }],
            })
            .unwrap();
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: draft.stamp(),
        })
        .unwrap();
        if index == 3 {
            app.complete_action(&crate::action_completion::CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: Box::new(app.action(id).unwrap()),
            })
            .unwrap();
        }
    }
    app.actions(&ActionListRequest::default()).unwrap().entries
}

#[test]
fn action_tools_read_exact_approved_records_and_pages_without_knowledge_mutations() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.base.path().join("vault/source.md"),
        "\u{feff}---\r\nbrn_kind: source\r\n---\r\ncurrent algne allikas\r\n",
    )
    .unwrap();
    std::fs::write(
        fixture.base.path().join("vault/history.md"),
        "---\nbrn_state: history\n---\ncurrent previous knowledge",
    )
    .unwrap();
    let before = seed(&fixture, false);
    let expected = before.clone();
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, _| {
        let expected = expected.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                for record in &expected {
                    assert_eq!(
                        tools.read_action(&record.origin.id.to_string()).unwrap(),
                        ai_record(record)
                    );
                }
                let first = tools.list_actions(None, 1, None).unwrap();
                assert_eq!(first["entries"], json!([expected[0]]));
                let cursor = first["next_cursor"].as_str().unwrap();
                let rest = tools.list_actions(None, 20, Some(cursor)).unwrap();
                assert_eq!(rest["entries"], json!(&expected[1..]));
                assert!(rest["next_cursor"].is_null());
                let waiting = tools.list_actions(Some("waiting"), 20, None).unwrap();
                assert_eq!(
                    waiting["entries"],
                    json!(
                        expected
                            .iter()
                            .filter(|r| r.data.state == ActionState::Waiting)
                            .collect::<Vec<_>>()
                    )
                );
                assert_eq!(tools.read_note("a.md").unwrap().text, "current");
                for (scope, expected) in [
                    (brn_ai::ReadScope::Current, vec!["a.md"]),
                    (brn_ai::ReadScope::Source, vec!["source.md"]),
                    (brn_ai::ReadScope::History, vec!["history.md"]),
                    (
                        brn_ai::ReadScope::All,
                        vec!["a.md", "history.md", "source.md"],
                    ),
                ] {
                    assert_eq!(
                        tools
                            .list_notes_scoped(None, None, scope)
                            .unwrap()
                            .notes
                            .iter()
                            .map(|n| n.path.as_str())
                            .collect::<Vec<_>>(),
                        expected
                    );
                    let mut hits = tools
                        .search_notes_scoped("current", 10, scope)
                        .unwrap()
                        .hits;
                    hits.sort_by(|a, b| a.path.cmp(&b.path));
                    assert_eq!(
                        hits.iter().map(|h| h.path.as_str()).collect::<Vec<_>>(),
                        expected
                    );
                    for path in expected {
                        let read = tools.read_note_scoped(path, scope).unwrap();
                        assert!(!read.truncated);
                        if path == "a.md" {
                            assert_eq!(read.text, "current");
                        } else {
                            assert!(read.text.contains("current"));
                        }
                    }
                }
                assert_eq!(tools.list_notes(None, None).unwrap().notes.len(), 1);
                assert_eq!(tools.search_notes("current", 10).unwrap().hits.len(), 1);
            })
            .await
            .unwrap();
            AiAnswer {
                text: "Full Action evidence read".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let query = Uuid::new_v4();
    worker
        .submit(query, AppCommand::Action(before[0].origin.id))
        .unwrap();
    loop {
        let (id, event) = event(&worker);
        if id == query {
            let AppEvent::Action(record) = event else {
                panic!("Expected owner Action record");
            };
            assert_eq!(*record, before[0]);
            assert!(
                serde_json::to_value(record)
                    .unwrap()
                    .get("checked_ref")
                    .is_none()
            );
            break;
        }
    }
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let result = terminal(&worker, request.id);
    assert_eq!(result.status, WorkTurnStatus::Completed);
    worker.shutdown().unwrap();
    let app = App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    assert_eq!(
        app.actions(&ActionListRequest::default()).unwrap().entries,
        before
    );
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
fn action_tools_refuse_malformed_and_oversized_whole_pages_but_single_records_remain_full() {
    let fixture = Fixture::new();
    let before = seed(&fixture, true);
    assert_eq!(before.len(), 2);
    for record in &before {
        assert!(serde_json::to_vec(record).unwrap().len() <= brn_ai::READ_ACTION_BYTES);
    }
    assert!(
        serde_json::to_vec(&json!({"entries": before, "next_cursor": null}))
            .unwrap()
            .len()
            > brn_ai::READ_ACTION_BYTES
    );
    let expected = before.clone();
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, _| {
        let expected = expected.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move||{
                for id in ["", "not-a-uuid", "00000000-0000-0000-0000-000000000000", "22222222-2222-4222-8222-222222222222"] {
                    assert_eq!(tools.read_action(id).unwrap_err().kind,AiErrorKind::ToolRejected);
                }
                for (state,limit,cursor) in [
                    (None,0,None),(None,21,None),(Some("Waiting"),1,None),(Some(""),1,None),
                    (None,1,Some("")),(None,1,Some("{}")),(None,1,Some("not-json")),
                    (None,1,Some(r#"{"created_at_ms":1,"id":"00000000-0000-0000-0000-000000000000"}"#)),
                    (None,1,Some(r#"{"created_at_ms":9223372036854775808,"id":"22222222-2222-4222-8222-222222222222"}"#)),
                    (None,1,Some(r#"{"created_at_ms":1,"id":"22222222-2222-4222-8222-222222222222","unknown":true}"#)),
                ]{assert_eq!(tools.list_actions(state,limit,cursor).unwrap_err().kind,AiErrorKind::ToolRejected);}
                assert_eq!(tools.list_actions(None,1,Some(&"x".repeat(257))).unwrap_err().kind,AiErrorKind::ToolRejected);
                assert_eq!(tools.list_actions(None,2,None).unwrap_err().kind,AiErrorKind::ToolRejected,"No truncated page or invented cursor");
                let mut cursor=None;
                for record in expected {
                    assert_eq!(tools.read_action(&record.origin.id.to_string()).unwrap(),ai_record(&record));
                    let page=tools.list_actions(None,1,cursor.as_deref()).unwrap();
                    assert_eq!(page["entries"],json!([record]));
                    cursor=page["next_cursor"].as_str().map(str::to_owned);
                }
                assert!(cursor.is_none());
            }).await.unwrap();
            AiAnswer {
                text: "Full bounded evidence".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    // Startup verifies retained full drafts, approvals and origins before Ready.
    // This escaped-byte fixture needs a bounded allowance under parallel CI load;
    // tool/event waits and the cancellation/deadlock witnesses keep their own deadlines.
    let mut worker = fixture.start_with_startup_timeout(
        Hooks {
            answer: Some(hook),
            ..Hooks::default()
        },
        Duration::from_secs(60),
    );
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(
        terminal(&worker, request.id).status,
        WorkTurnStatus::Completed
    );
    worker.shutdown().unwrap();
    let app = App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    assert_eq!(
        app.actions(&ActionListRequest::default()).unwrap().entries,
        before
    );
}

#[test]
fn action_tool_stop_retains_read_lease_public_events_and_restart_history() {
    let fixture = Fixture::new();
    let before = seed(&fixture, false);
    let expected = before.clone();
    let (release, wait) = mpsc::channel();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let (started, ready) = mpsc::channel();
    let (canceled, cancellation) = mpsc::channel();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called = calls.clone();
    let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        called.fetch_add(1, Ordering::SeqCst);
        let wait = wait.lock().unwrap().take().unwrap();
        let expected = expected.clone();
        let started = started.clone();
        let canceled = canceled.clone();
        Box::pin(async move {
            let _read = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                let a = tools.clone();
                let b = tools.clone();
                let first = std::thread::spawn(move || {
                    a.read_action(&expected[0].origin.id.to_string()).unwrap()
                });
                let second =
                    std::thread::spawn(move || b.list_actions(Some("waiting"), 20, None).unwrap());
                assert!(first.join().unwrap()["origin"].is_object());
                assert_eq!(
                    second.join().unwrap()["entries"].as_array().unwrap().len(),
                    1
                );
            });
            cancel.cancelled().await;
            canceled.send(()).unwrap();
            AiAnswer {
                text: "retained Action read".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let stop = Uuid::new_v4();
    worker
        .submit(stop, AppCommand::CancelTurn(request.id))
        .unwrap();
    cancellation.recv_timeout(Duration::from_secs(10)).unwrap();
    let status = Uuid::new_v4();
    worker.submit(status, AppCommand::Status).unwrap();
    let mut status_seen = false;
    let mut stop_seen = false;
    while !(status_seen && stop_seen) {
        let (id, value) = event(&worker);
        match value {
            AppEvent::Status(_) => {
                assert_eq!(id, status);
                status_seen = true;
            }
            AppEvent::TurnCancelRequested { accepted, .. } => {
                assert_eq!(id, stop);
                assert!(accepted);
                stop_seen = true;
            }
            AppEvent::Chat(ChatEvent::Finished { .. }) => {
                panic!("must retain blocking Action tools until release")
            }
            _ => {}
        }
    }
    let query = Uuid::new_v4();
    worker.submit(query, AppCommand::Turn(request.id)).unwrap();
    loop {
        let (id, value) = event(&worker);
        if id == query {
            assert!(
                matches!(value,AppEvent::Turn(Some(turn))if turn.status==WorkTurnStatus::Running)
            );
            break;
        }
    }
    release.send(()).unwrap();
    let saved = terminal(&worker, request.id);
    assert_eq!(saved.status, WorkTurnStatus::Interrupted);
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks::default());
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(terminal(&worker, request.id).answer, saved.answer);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.shutdown().unwrap();
    let app = App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    assert_eq!(
        app.actions(&ActionListRequest::default()).unwrap().entries,
        before
    );
}
