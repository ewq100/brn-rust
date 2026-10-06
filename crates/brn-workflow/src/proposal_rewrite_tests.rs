use super::*;
use crate::proposal_rewrite::{
    ReasoningEffort, RewriteEvent, RewriteJob, RewriteRequest, RewriteStatus,
};
use crate::proposals::{
    CommentRequest, CommentTarget, DraftNoteChange, DraftRequest, ProposalEdit, ProposalRecord,
    ReviewComment,
};

fn reply(worker: &AppWorker, id: Uuid) -> AppEvent {
    loop {
        let (actual, value) = event(worker);
        if actual == id {
            return value;
        }
    }
}
fn review(worker: &AppWorker, id: Uuid) -> ProposalRecord {
    let query = Uuid::new_v4();
    worker.submit(query, AppCommand::Proposal(id)).unwrap();
    match reply(worker, query) {
        AppEvent::Proposal(record) => record,
        _ => panic!("expected proposal"),
    }
}
fn create(worker: &AppWorker) -> ProposalRecord {
    let open = Uuid::new_v4();
    worker
        .submit(open, AppCommand::OpenEditor("a.md".into()))
        .unwrap();
    let AppEvent::Editor(editor) = reply(worker, open) else {
        panic!("editor")
    };
    let request = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Full rewrite".into(),
        changes: vec![
            DraftNoteChange::Replace {
                path: "a.md".into(),
                expected: editor.record.baseline,
                text: "\u{feff}Draft λ\r\n".into(),
            },
            DraftNoteChange::Create {
                path: "new.md".into(),
                text: "Whole 🦀\r\n".into(),
            },
        ],
        sources: Vec::new(),
    };
    let operation = Uuid::new_v4();
    worker
        .submit(operation, AppCommand::CreateProposal(request))
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, operation) else {
        panic!("create")
    };
    let comment = CommentRequest {
        expected: record.stamp(),
        comment: ReviewComment {
            id: Uuid::new_v4(),
            text: "Temporary private review instruction".into(),
            target: CommentTarget::Proposal,
        },
    };
    let operation = Uuid::new_v4();
    worker
        .submit(operation, AppCommand::AddProposalComment(comment))
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, operation) else {
        panic!("comment")
    };
    record
}
fn request(record: &ProposalRecord) -> RewriteRequest {
    RewriteRequest {
        id: Uuid::new_v4(),
        expected: record.stamp(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-5.5".into(),
        },
        effort: ReasoningEffort::High,
        generation: 73,
    }
}
fn start(worker: &AppWorker, request: &RewriteRequest) -> RewriteJob {
    worker
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    match reply(worker, request.id) {
        AppEvent::Rewrite(RewriteEvent::Started {
            id,
            generation,
            job,
        }) => {
            assert_eq!(id, request.id);
            assert_eq!(generation, request.generation);
            job
        }
        AppEvent::Rewrite(other) => panic!("expected durable admission, got {other:?}"),
        AppEvent::Failed(error) => panic!("expected admission, got {error:?}"),
        _ => panic!("unexpected event before admission"),
    }
}
fn finish(worker: &AppWorker, request: &RewriteRequest) -> RewriteJob {
    loop {
        match reply(worker, request.id) {
            AppEvent::Rewrite(RewriteEvent::Finished {
                id,
                generation,
                job,
            }) => {
                assert_eq!(id, request.id);
                assert_eq!(generation, request.generation);
                return job;
            }
            AppEvent::Rewrite(RewriteEvent::ToolStarted { .. }) => {}
            _ => panic!("unexpected Rewrite event"),
        }
    }
}
fn answer() -> AiAnswer {
    AiAnswer { text: serde_json::json!({"title":"Revised", "texts":["\u{feff}Full λ 🦀\r\n", "Full second\r\n"]}).to_string(), terminal: AiTerminal::Completed }
}

fn asset_review(fixture: &Fixture, worker: &AppWorker) -> ProposalRecord {
    use crate::proposals::{MAX_ASSET_BYTES, SourceVersion};
    fn asset(worker: &AppWorker, path: &str) -> crate::proposals::ProposalAsset {
        let query = Uuid::new_v4();
        worker
            .submit(query, AppCommand::ProposalAsset(path.into()))
            .unwrap();
        match reply(worker, query) {
            AppEvent::ProposalAsset(proof) => *proof,
            _ => panic!("expected complete asset proof"),
        }
    }
    std::fs::write(
        fixture.base.path().join("vault/replace.bin"),
        b"\xff\0Before",
    )
    .unwrap();
    std::fs::write(fixture.base.path().join("vault/trash.bin"), b"\xfe\0Trash").unwrap();
    let before = asset(worker, "replace.bin");
    let trash = asset(worker, "trash.bin");
    let query = Uuid::new_v4();
    worker
        .submit(query, AppCommand::OpenEditor("a.md".into()))
        .unwrap();
    let AppEvent::Editor(editor) = reply(worker, query) else {
        panic!("editor")
    };
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::CreateProposal(DraftRequest {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: vec![],
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Full mixed asset rewrite".into(),
                changes: vec![
                    DraftNoteChange::CreateAsset {
                        path: "new.bin".into(),
                        bytes: b"\0OPAQUE_CREATE_NOT_MODEL_CONTEXT\xff".to_vec(),
                    },
                    DraftNoteChange::ReplaceAsset {
                        path: "replace.bin".into(),
                        expected: before.fingerprint,
                        bytes: vec![0xff; MAX_ASSET_BYTES],
                    },
                    DraftNoteChange::TrashAsset {
                        path: "trash.bin".into(),
                        expected: trash.fingerprint,
                    },
                    DraftNoteChange::Replace {
                        path: "a.md".into(),
                        expected: editor.record.baseline.clone(),
                        text: "Full candidate λ\r\n".into(),
                    },
                    DraftNoteChange::Create {
                        path: "new.md".into(),
                        text: "Whole new 🦀\r\n".into(),
                    },
                ],
                sources: vec![SourceVersion {
                    path: "a.md".into(),
                    fingerprint: editor.record.baseline,
                }],
            }),
        )
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, query) else {
        panic!("asset review")
    };
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::AddProposalComment(CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Preserve all opaque proofs and ordered slots".into(),
                    target: CommentTarget::Proposal,
                },
            }),
        )
        .unwrap();
    let AppEvent::Proposal(record) = reply(worker, query) else {
        panic!("comment")
    };
    record
}

#[test]
fn actual_asset_rewrite_omits_payloads_but_hashes_full_capture_and_preserves_null_slots() {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let (send, captured) = mpsc::channel();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called = calls.clone();
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, prompt, tools, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        send.send(prompt).unwrap();
        Box::pin(async move {
            assert_eq!(tools.read_note("a.md").unwrap().text, "current");
            assert!(tools.read_note("replace.bin").is_err());
            AiAnswer { text: serde_json::json!({"title":"Rewritten mixed review", "texts":[null,null,null,"Rewritten full λ\r\n","Whole rewritten 🦀\r\n"]}).to_string(), terminal: AiTerminal::Completed }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = asset_review(&fixture, &worker);
    let request = request(&original);
    let running = start(&worker, &request);
    assert_eq!(
        running.capture_sha256,
        <[u8; 32]>::from(Sha256::digest(serde_json::to_vec(&original).unwrap()))
    );
    let prompt = captured.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(prompt.len() < 8192);
    assert!(prompt.starts_with("This capture includes immutable ordinary asset members."));
    let projected: serde_json::Value =
        serde_json::from_str(prompt.split_once("\n\n").unwrap().1).unwrap();
    let changes = projected["draft"]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 5);
    for (index, change) in original.draft.changes.iter().take(3).enumerate() {
        assert_eq!(changes[index]["path"], change.path());
        assert_eq!(
            changes[index]["parent"],
            serde_json::to_value(change.parent()).unwrap()
        );
        assert_eq!(
            changes[index]["before"],
            serde_json::to_value(change.before()).unwrap()
        );
        assert_eq!(changes[index]["texts_slot"], serde_json::Value::Null);
        assert!(changes[index].get("bytes").is_none());
        assert!(changes[index].get("before_bytes").is_none());
        if let Some(bytes) = change.candidate_bytes() {
            assert_eq!(changes[index]["candidate"]["byte_len"], bytes.len());
            assert_eq!(
                changes[index]["candidate"]["sha256"],
                serde_json::to_value(<[u8; 32]>::from(Sha256::digest(bytes))).unwrap()
            );
        }
    }
    assert_eq!(
        projected["comments"],
        serde_json::to_value(&original.comments).unwrap()
    );
    assert_eq!(
        projected["draft"]["sources"],
        serde_json::to_value(&original.draft.sources).unwrap()
    );
    let completed = finish(&worker, &request);
    assert_eq!(completed.status, RewriteStatus::Completed);
    let rewritten = review(&worker, original.draft.id);
    assert_eq!(rewritten.draft.changes[..3], original.draft.changes[..3]);
    assert_eq!(
        rewritten.draft.changes[3].text(),
        Some("Rewritten full λ\r\n")
    );
    assert_eq!(
        rewritten.draft.changes[4].text(),
        Some("Whole rewritten 🦀\r\n")
    );
    assert_eq!(rewritten.comments, original.comments);
    worker
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    assert_eq!(finish(&worker, &request), completed);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!fixture.base.path().join("vault/new.bin").exists());
    std::fs::write(
        fixture.base.path().join("vault/replace.bin"),
        b"Changed after Rewrite",
    )
    .unwrap();
    let query = Uuid::new_v4();
    worker
        .submit(
            query,
            AppCommand::ApproveProposal(crate::proposal_apply::ApprovalRequest {
                operation_id: query,
                expected: rewritten.stamp(),
            }),
        )
        .unwrap();
    assert!(
        matches!(reply(&worker, query), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert_eq!(review(&worker, original.draft.id), rewritten);
    assert!(!fixture.base.path().join("vault/new.bin").exists());
    worker.shutdown().unwrap();
}

#[test]
fn nonnull_asset_rewrite_output_is_rejected_as_a_whole_and_replays_without_calls() {
    let fixture = Fixture::new();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called = calls.clone();
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, _, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            AiAnswer { text: serde_json::json!({"title":"Forbidden partial asset edit", "texts":["changed opaque payload",null,null,"Text that must not install","Text that must not install"]}).to_string(), terminal: AiTerminal::Completed }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = asset_review(&fixture, &worker);
    let request = request(&original);
    start(&worker, &request);
    let failed = finish(&worker, &request);
    assert_eq!(failed.status, RewriteStatus::Failed);
    assert_eq!(failed.error_code.as_deref(), Some("tool_rejected"));
    assert_eq!(review(&worker, original.draft.id), original);
    worker
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    assert_eq!(finish(&worker, &request), failed);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!fixture.base.path().join("vault/new.bin").exists());
    worker.shutdown().unwrap();
}

#[test]
fn rewrite_captures_full_review_and_atomically_installs_only_review_text() {
    let fixture = Fixture::new();
    let (entered, capture) = mpsc::channel();
    let hook: crate::proposal_rewrite::RewriteHook =
        Arc::new(move |request, prompt, tools, _, emit| {
            entered.send((request, prompt)).unwrap();
            Box::pin(async move {
                assert_eq!(tools.read_note("a.md").unwrap().text, "current");
                emit(AiEvent::Text(
                    "raw provider text must not become a chat turn".into(),
                ));
                drop(tools);
                answer()
            })
        });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = create(&worker);
    let request = request(&original);
    let running = start(&worker, &request);
    assert_eq!(running.status, RewriteStatus::Running);
    assert_eq!(running.spec.effort, "high");
    let (selected, prompt) = capture.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(selected, request);
    let captured: ProposalRecord = serde_json::from_str(&prompt).unwrap();
    assert_eq!(captured, original);
    let completed = finish(&worker, &request);
    assert_eq!(completed.status, RewriteStatus::Completed);
    let rewritten = review(&worker, original.draft.id);
    assert_eq!(completed.result_stamp, Some(rewritten.stamp()));
    assert_eq!(
        rewritten.draft.changes[0].text(),
        Some("\u{feff}Full λ 🦀\r\n")
    );
    assert_eq!(rewritten.draft.changes[1].text(), Some("Full second\r\n"));
    assert_eq!(rewritten.comments, original.comments);
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
    assert!(!fixture.base.path().join("vault/new.md").exists());
    let query = Uuid::new_v4();
    worker.submit(query, AppCommand::Conversations).unwrap();
    assert!(matches!(reply(&worker, query), AppEvent::Conversations(items) if items.is_empty()));
    worker.shutdown().unwrap();
}

#[test]
fn later_edits_comments_rejection_and_approval_defeat_late_result() {
    for change in 0..4 {
        let fixture = Fixture::new();
        let (release, wait) = tokio::sync::oneshot::channel();
        let wait = Arc::new(Mutex::new(Some(wait)));
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, _, _| {
            let wait = wait.lock().unwrap().take().unwrap();
            Box::pin(async move {
                wait.await.unwrap();
                drop(tools);
                answer()
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let original = create(&worker);
        let request = request(&original);
        start(&worker, &request);
        let operation = Uuid::new_v4();
        let command = match change {
            0 => AppCommand::EditProposal(ProposalEdit {
                action_data: Vec::new(),
                expected: original.stamp(),
                title: "Newer manual work".into(),
                texts: vec![
                    Some("Newer exact text".into()),
                    Some("Second manual text".into()),
                ],
            }),
            1 => AppCommand::AddProposalComment(CommentRequest {
                expected: original.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Later comment".into(),
                    target: CommentTarget::Proposal,
                },
            }),
            2 => AppCommand::RejectProposal(original.stamp()),
            _ => AppCommand::ApproveProposal(crate::proposal_apply::ApprovalRequest {
                operation_id: operation,
                expected: original.stamp(),
            }),
        };
        worker.submit(operation, command).unwrap();
        assert!(matches!(
            reply(&worker, operation),
            AppEvent::Proposal(_) | AppEvent::ProposalApplied(_)
        ));
        let newer = review(&worker, original.draft.id);
        release.send(()).unwrap();
        assert_eq!(finish(&worker, &request).status, RewriteStatus::Stale);
        assert_eq!(review(&worker, original.draft.id), newer);
        worker.shutdown().unwrap();
    }
}

#[test]
fn malformed_and_incomplete_or_oversized_results_settle_without_partial_work() {
    for text in [
        "partial raw private output".into(),
        serde_json::json!({"title":"Extra field", "texts":["one","two"], "path":"other.md"}).to_string(),
        serde_json::json!({"title":"Incomplete", "texts":["one"]}).to_string(),
        serde_json::json!({"title":"Oversized", "texts":["x".repeat(crate::MAX_NOTE_BYTES+1),"two"]}).to_string(),
    ] {
        let fixture = Fixture::new();
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, _, _| {
            let text = text.clone();
            Box::pin(async move { drop(tools); AiAnswer { text, terminal: AiTerminal::Completed } })
        });
        let mut worker = fixture.start(Hooks { rewrite: Some(hook), ..Hooks::default() });
        let original = create(&worker);
        let request = request(&original);
        start(&worker, &request);
        let job = finish(&worker, &request);
        assert_eq!(job.status, RewriteStatus::Failed);
        assert_eq!(job.error_code.as_deref(), Some("tool_rejected"));
        assert_eq!(review(&worker, original.draft.id), original);
        worker.shutdown().unwrap();
    }
}

#[test]
fn stop_waits_for_retained_read_lease_but_editor_recovery_remains_responsive() {
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
                text: "discard partial".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = create(&worker);
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
    let recovery = Uuid::new_v4();
    worker
        .submit(
            recovery,
            AppCommand::RecoverEdit {
                path: "a.md".into(),
                base_sha256: [5; 32],
                text: "Later editor text".into(),
            },
        )
        .unwrap();
    assert!(matches!(reply(&worker, recovery), AppEvent::EditRecovered));
    let query = Uuid::new_v4();
    worker
        .submit(query, AppCommand::ProposalRewrite(request.id))
        .unwrap();
    assert!(
        matches!(reply(&worker, query), AppEvent::ProposalRewrite(Some(job)) if job.status == RewriteStatus::Running)
    );
    release.send(()).unwrap();
    assert_eq!(finish(&worker, &request).status, RewriteStatus::Interrupted);
    assert_eq!(review(&worker, original.draft.id), original);
    worker.shutdown().unwrap();
}

#[test]
fn disconnect_and_shutdown_cancel_owned_rewrite_without_resubmission() {
    for disconnect in [true, false] {
        let fixture = Fixture::new();
        let (entered, ready) = mpsc::channel();
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, cancel, _| {
            entered.send(()).unwrap();
            Box::pin(async move {
                cancel.cancelled().await;
                drop(tools);
                AiAnswer {
                    text: "discard partial".into(),
                    terminal: AiTerminal::Interrupted,
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let original = create(&worker);
        let request = request(&original);
        start(&worker, &request);
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        if disconnect {
            let operation = Uuid::new_v4();
            worker
                .submit(
                    operation,
                    AppCommand::Account {
                        id: operation,
                        command: AccountCommand::Disconnect(Provider::Chatgpt),
                    },
                )
                .unwrap();
            assert_eq!(finish(&worker, &request).status, RewriteStatus::Interrupted);
            assert!(matches!(
                reply(&worker, operation),
                AppEvent::Account(AccountEvent::Finished {
                    reply: AccountReply::Disconnected,
                    ..
                })
            ));
        }
        worker.shutdown().unwrap();
        let mut reopened = fixture.start(Hooks::default());
        let query = Uuid::new_v4();
        reopened
            .submit(query, AppCommand::ProposalRewrite(request.id))
            .unwrap();
        assert!(
            matches!(reply(&reopened, query), AppEvent::ProposalRewrite(Some(job)) if job.status == RewriteStatus::Interrupted)
        );
        reopened
            .submit(
                request.id,
                AppCommand::StartProposalRewrite(request.clone()),
            )
            .unwrap();
        assert_eq!(
            finish(&reopened, &request).status,
            RewriteStatus::Interrupted
        );
        assert_eq!(review(&reopened, original.draft.id), original);
        reopened.shutdown().unwrap();
    }
}

#[test]
fn terminal_replay_is_history_only_after_vault_loss_and_changed_request_conflicts() {
    let fixture = Fixture::new();
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(|_, _, tools, _, _| {
        Box::pin(async move {
            drop(tools);
            answer()
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = create(&worker);
    let request = request(&original);
    start(&worker, &request);
    let completed = finish(&worker, &request);
    worker.shutdown().unwrap();
    std::fs::rename(
        fixture.base.path().join("vault"),
        fixture.base.path().join("moved-vault"),
    )
    .unwrap();
    let mut reopened = fixture.start(Hooks::default());
    let mut replay = request.clone();
    replay.generation += 1;
    reopened
        .submit(replay.id, AppCommand::StartProposalRewrite(replay.clone()))
        .unwrap();
    assert_eq!(finish(&reopened, &replay), completed);
    replay.effort = ReasoningEffort::Low;
    reopened
        .submit(replay.id, AppCommand::StartProposalRewrite(replay.clone()))
        .unwrap();
    assert!(
        matches!(reply(&reopened, replay.id), AppEvent::Rewrite(RewriteEvent::Rejected { error, .. }) if error.kind == ErrorKind::OperationConflict)
    );
    reopened.shutdown().unwrap();
}

#[test]
fn settlement_failure_rolls_back_proposal_and_restart_interrupts_without_retry() {
    let fixture = Fixture::new();
    let (release, wait) = tokio::sync::oneshot::channel();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, _, _| {
        let wait = wait.lock().unwrap().take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            drop(tools);
            answer()
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = create(&worker);
    let request = request(&original);
    start(&worker, &request);
    let conn = rusqlite::Connection::open(fixture.base.path().join("data/brn.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_rewrite_settlement BEFORE UPDATE ON proposal_rewrites BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    release.send(()).unwrap();
    assert!(
        matches!(reply(&worker, request.id), AppEvent::Rewrite(RewriteEvent::PersistenceFailed { error, .. }) if error.kind == ErrorKind::AiStorage)
    );
    assert_eq!(review(&worker, original.draft.id), original);
    assert!(worker.shutdown().is_err());
    conn.execute_batch("DROP TRIGGER reject_rewrite_settlement")
        .unwrap();
    drop(conn);
    let mut reopened = fixture.start(Hooks::default());
    reopened
        .submit(
            request.id,
            AppCommand::StartProposalRewrite(request.clone()),
        )
        .unwrap();
    assert_eq!(
        finish(&reopened, &request).status,
        RewriteStatus::Interrupted
    );
    assert_eq!(review(&reopened, original.draft.id), original);
    reopened.shutdown().unwrap();
}

#[test]
fn owned_panic_and_provider_refusal_have_safe_terminal_history_without_retry() {
    for panic in [true, false] {
        let fixture = Fixture::new();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let called = calls.clone();
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, _, _| {
            called.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                drop(tools);
                assert!(!panic, "synthetic owned job panic");
                AiAnswer {
                    text: "raw refusal must be discarded".into(),
                    terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ModelRefused)),
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let original = create(&worker);
        let request = request(&original);
        start(&worker, &request);
        let failed = finish(&worker, &request);
        assert_eq!(failed.status, RewriteStatus::Failed);
        assert_eq!(
            failed.error_code.as_deref(),
            Some(if panic { "other" } else { "model_refused" })
        );
        worker
            .submit(
                request.id,
                AppCommand::StartProposalRewrite(request.clone()),
            )
            .unwrap();
        assert_eq!(finish(&worker, &request), failed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(review(&worker, original.draft.id), original);
        worker.shutdown().unwrap();
    }
}

#[test]
fn running_replay_does_not_promise_completion_for_a_new_presentation_generation() {
    let fixture = Fixture::new();
    let (release, wait) = tokio::sync::oneshot::channel();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, _, tools, _, _| {
        let wait = wait.lock().unwrap().take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            drop(tools);
            answer()
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: Some(hook),
        ..Hooks::default()
    });
    let original = create(&worker);
    let request = request(&original);
    let running = start(&worker, &request);
    let mut replay = request.clone();
    replay.generation += 1;
    worker
        .submit(replay.id, AppCommand::StartProposalRewrite(replay.clone()))
        .unwrap();
    let event = reply(&worker, replay.id);
    // Started promises this owned request's future terminal event. A Running
    // replay cannot create that promise for a new presentation generation.
    let replay_is_history = matches!(event, AppEvent::Rewrite(RewriteEvent::AlreadyRunning { generation, job, .. }) if generation == replay.generation && job == running);
    release.send(()).unwrap();
    assert_eq!(finish(&worker, &request).status, RewriteStatus::Completed);
    assert_eq!(running.status, RewriteStatus::Running);
    worker.shutdown().unwrap();
    assert!(
        replay_is_history,
        "Running replay must identify existing work without promising a new generation's terminal event"
    );
}

#[path = "action_rewrite_tests.rs"]
mod actions;

#[test]
fn ai_rewrite_preserves_managed_metadata_and_retains_refusals_without_changing_owner_review() {
    for mode in 0..5 {
        let fixture = Fixture::new();
        let hook: crate::proposal_rewrite::RewriteHook = Arc::new(move |_, prompt, tools, _, _| {
            let captured: ProposalRecord = serde_json::from_str(&prompt).unwrap();
            let mut texts = captured
                .draft
                .changes
                .iter()
                .map(|change| change.text().map(str::to_string))
                .collect::<Vec<_>>();
            let original = texts[0].take().unwrap();
            texts[0] = Some(match mode {
                0 => original.replace("Owner body", "Rewritten body"),
                1 => original.replace("brn_kind: knowledge", "brn_kind: source"),
                2 => original.replace("brn_state: current", "brn_state: history"),
                3 => original.replace("brn_provenance: []\r\n", ""),
                4 => original.replace(
                    "brn_kind: knowledge",
                    "brn_kind: knowledge\r\nbrn_inbox_source: {}",
                ),
                _ => unreachable!(),
            });
            drop(tools);
            Box::pin(async move {
                AiAnswer {
                    text: serde_json::json!({"title":"AI suggestion", "texts":texts}).to_string(),
                    terminal: AiTerminal::Completed,
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: Some(hook),
            ..Hooks::default()
        });
        let original = create(&worker);
        let original_text = "\u{feff}---\r\nbrn_kind: knowledge\r\nbrn_state: current\r\nbrn_provenance: []\r\n---\r\nOwner body õ 🦀\r\n";
        let edit = ProposalEdit {
            expected: original.stamp(),
            title: original.draft.title.clone(),
            texts: vec![
                Some(original_text.into()),
                original.draft.changes[1].text().map(str::to_string),
            ],
            action_data: vec![],
        };
        let id = Uuid::new_v4();
        worker.submit(id, AppCommand::EditProposal(edit)).unwrap();
        let AppEvent::Proposal(original) = reply(&worker, id) else {
            panic!("owner metadata edit");
        };
        let request = request(&original);
        start(&worker, &request);
        let job = finish(&worker, &request);
        let after = review(&worker, original.draft.id);
        if mode == 0 {
            assert_eq!(job.status, RewriteStatus::Completed);
            assert_eq!(
                after.draft.changes[0].text(),
                Some(
                    original_text
                        .replace("Owner body", "Rewritten body")
                        .as_str()
                )
            );
        } else {
            assert_eq!(job.status, RewriteStatus::Failed);
            assert_eq!(job.error_code.as_deref(), Some("tool_rejected"));
            assert_eq!(after, original);
        }
        assert_eq!(
            std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
            b"current"
        );
        assert!(!fixture.base.path().join("vault/new.md").exists());
        worker.shutdown().unwrap();
        let mut worker = fixture.start(Hooks::default());
        worker
            .submit(
                request.id,
                AppCommand::StartProposalRewrite(request.clone()),
            )
            .unwrap();
        let replay = finish(&worker, &request);
        assert_eq!(replay, job);
        assert_eq!(review(&worker, original.draft.id), after);
        worker.shutdown().unwrap();
    }
}
