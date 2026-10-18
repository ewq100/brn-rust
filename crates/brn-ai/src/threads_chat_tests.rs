use super::*;
use sha2::{Digest, Sha256};
fn control(store: &mut Store, key: &str, command: ChatCommand) -> ChatCommandOutcome {
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&command).unwrap())
    );
    let op = store.allocate_bound_operation(key, &hash).unwrap();
    store
        .chat_command(&op, &command, &HostAuthority::owner("fixture chat control"))
        .unwrap()
}
fn pending(store: &Store) -> QueuedMessage {
    let RecordData::ChatControl(c) = store.chat_control().unwrap().unwrap().data else {
        panic!()
    };
    c.pending.unwrap()
}
fn owner_record(store: &mut Store, key: &str, data: RecordData) -> String {
    let op = store.allocate_operation(key).unwrap();
    let id = op.creation_id(0);
    let owner = HostAuthority::owner("fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                inputs: vec![],
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data,
                }],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    id
}
#[test]
fn queued_partial_failure_and_stop_resume_reuse_exact_instruction_and_reject_old_claim() {
    for stopped in [false, true] {
        let (_root, mut request, _) = tests::fixture();
        let mut store = Store::open(&request.data_dir).unwrap();
        control(
            &mut store,
            "queue",
            ChatCommand::Enqueue {
                thread: request.thread.clone(),
                text: request.prompt.clone(),
            },
        );
        let q = control(&mut store, "dispatch", ChatCommand::Dispatch)
            .dispatched
            .unwrap();
        request.request_message = q.message.clone();
        request.queue_claim = q.dispatch.clone();
        let (scope, _) = begin(&request).unwrap();
        let bound = pending(&store);
        assert_eq!(bound.run.as_ref(), Some(&scope.run));
        assert!(begin(&request).is_err(), "one claimant starts once");
        if stopped {
            control(
                &mut store,
                "stop",
                ChatCommand::Pause {
                    run: Some(scope.run.clone()),
                    partial: Some("Useful partial reply".into()),
                },
            );
        } else {
            assert!(
                finish(
                    &scope,
                    &AiAnswer {
                        text: "Useful partial reply".into(),
                        terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected))
                    }
                )
                .unwrap()
            );
        }
        let RecordData::Run(r) = store.record(&scope.run).unwrap().unwrap().data else {
            panic!()
        };
        control(
            &mut store,
            "failed",
            ChatCommand::Finish {
                item: q.id.clone(),
                message: q.message.clone().unwrap(),
                run: scope.run.clone(),
                fence: r.fence,
                success: false,
            },
        );
        control(&mut store, "resume", ChatCommand::Resume);
        let old = ChatCommand::Finish {
            item: q.id.clone(),
            message: q.message.clone().unwrap(),
            run: scope.run.clone(),
            fence: r.fence,
            success: false,
        };
        let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&old).unwrap()));
        let op = store.allocate_bound_operation("late", &hash).unwrap();
        assert!(
            store
                .chat_command(&op, &old, &HostAuthority::owner("fixture"))
                .is_err()
        );
        let retry = control(&mut store, "retry", ChatCommand::Dispatch)
            .dispatched
            .unwrap();
        assert_eq!(q.message, retry.message);
        assert!(begin(&request).is_err(), "stale claim invalid after Resume");
        request.queue_claim = retry.dispatch;
        let (fresh, _) = begin(&request).unwrap();
        assert_ne!(fresh.run, scope.run);
        let messages = store
            .records()
            .unwrap()
            .into_iter()
            .filter(|r| matches!(&r.data,RecordData::Message(m) if m.role=="owner"))
            .count();
        assert_eq!(messages, 1);
        assert!(
            !finish(
                &scope,
                &AiAnswer {
                    text: "Late".into(),
                    terminal: AiTerminal::Completed
                }
            )
            .unwrap()
        );
    }
}
#[test]
fn unrelated_chat_cannot_read_private_ids_history_or_apply_new_foreign_refs() {
    let (_root, request, general) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let other = owner_record(
        &mut store,
        "other",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Other".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let source = owner_record(
        &mut store,
        "private-source",
        RecordData::Source(Source {
            chat: Some(other.clone()),
            locator: "private://file".into(),
            label: "Private label".into(),
            observed_at: "fixture".into(),
            external_version: None,
            outcome: "Private observation".into(),
            gaps: vec![],
            report: None,
        }),
    );
    let mut n = Note::working("Private", "private text");
    n.chat = Some(other);
    let note = owner_record(&mut store, "private-note", RecordData::Note(n));
    let (scope, _) = begin(&request).unwrap();
    assert_eq!(
        read_records(
            &scope,
            ReadArgs {
                ids: vec![source.clone(), note.clone()],
                query: None
            }
        )
        .unwrap()["records"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        read_note(
            &scope,
            ReadNoteArgs {
                id: note.clone(),
                version: 1,
                start: 0,
                max_bytes: 4096,
                section: false
            }
        )
        .is_err()
    );
    assert!(
        history::read_history(
            &scope,
            HistoryArgs {
                id: note.clone(),
                version: Some(1),
                start: 0,
                max_bytes: 4096
            }
        )
        .is_err()
    );
    for (index, data) in [
        RecordData::Link(Link {
            from: general.clone(),
            to: source.clone(),
            relation: "source".into(),
        }),
        RecordData::Comment(Comment {
            note: note.clone(),
            body: "foreign".into(),
            base_version: 1,
            quote: "private".into(),
            range: None,
            mapped_version: None,
            mapped_range: None,
            unresolved: true,
        }),
    ]
    .into_iter()
    .enumerate()
    {
        let prepared = prepare(
            &scope,
            PrepareArgs {
                writing: None,
                request_key: format!("foreign-{index}"),
                request: ChangeRequest {
                    reason: "Foreign reference attempt".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: "new:0".into(),
                        expected_version: None,
                        archived: false,
                        data,
                    }],
                },
            },
        )
        .unwrap();
        let outcome = apply(
            &scope,
            ApplyArgs {
                operation: prepared["operation"].as_str().unwrap().into(),
            },
        )
        .unwrap();
        assert_eq!(outcome["outcome"], "NeedsReview");
    }
}
#[test]
fn private_context_scopes_generated_notes_and_binds_exact_local_source_observation() {
    let (_root, mut request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let mut n = Note::working("Attachment", "private attachment text");
    n.chat = Some(request.thread.clone());
    owner_record(&mut store, "attachment", RecordData::Note(n));
    let global = owner_record(
        &mut store,
        "global-source",
        RecordData::Source(Source {
            chat: None,
            locator: "same://file".into(),
            label: "Old general source".into(),
            observed_at: "old".into(),
            external_version: Some("old".into()),
            outcome: "old".into(),
            gaps: vec![],
            report: None,
        }),
    );
    request.sources.push(TransientSource {
        reference: "same://file".into(),
        text: "new private source".into(),
        fingerprint: Some("new".into()),
        report: None,
        assets: vec![],
    });
    let (scope, _) = begin(&request).unwrap();
    let metadata = read_source(
        &scope,
        SourceArgs {
            reference: "same://file".into(),
            start: 0,
        },
    )
    .unwrap();
    assert_ne!(metadata["source_record"]["id"], global);
    let source = metadata["source_record"]["id"].as_str().unwrap();
    assert!(
        matches!(store.record(source).unwrap().unwrap().data,RecordData::Source(s) if s.chat.as_ref()==Some(&request.thread))
    );
    let p = prepare(
        &scope,
        PrepareArgs {
            writing: None,
            request_key: "local-note".into(),
            request: ChangeRequest {
                reason: "Local generated note".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: "new:0".into(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Note(Note::working(
                            "Generated",
                            "supported private source information",
                        )),
                    },
                    Put {
                        id: "new:1".into(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: "new:0".into(),
                            to: source.into(),
                            relation: "source".into(),
                        }),
                    },
                ],
            },
        },
    )
    .unwrap();
    let op = store.operation(p["operation"].as_str().unwrap()).unwrap();
    let prepared = store.prepared(&op).unwrap();
    assert!(
        prepared.request.writes.iter().any(
            |w| matches!(&w.data,RecordData::Note(n) if n.chat.as_ref()==Some(&request.thread))
        )
    );
    assert_eq!(store.record(&global).unwrap().unwrap().version, 1);
}

#[test]
fn foreign_inline_asset_urls_cannot_be_written_or_finalized_in_an_unrelated_chat() {
    let (_root, request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let other = owner_record(
        &mut store,
        "asset-owner-chat",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Private chat".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let mut private = Note::working("Private file", "Figure");
    private.chat = Some(other);
    let note = owner_record(&mut store, "private-figure-note", RecordData::Note(private));
    let op = store.allocate_operation("private-figure").unwrap();
    let asset = op.creation_id(0);
    let current = store.record(&note).unwrap().unwrap();
    let owner = HostAuthority::owner("fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "private figure".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: note.clone(),
                        expected_version: Some(current.version),
                        archived: false,
                        data: current.data,
                    },
                    Put {
                        id: asset.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Asset(Asset {
                            note,
                            source: None,
                            media_type: "image/png".into(),
                            bytes: vec![1, 2, 3],
                        }),
                    },
                ],
            },
            &owner,
        )
        .unwrap();
    store.apply(&op, &owner).unwrap();
    let (scope, _) = begin(&request).unwrap();
    let text = format!("![Private figure](brn-asset://{asset})");
    let p = prepare(
        &scope,
        PrepareArgs {
            writing: None,
            request_key: "foreign-picture".into(),
            request: ChangeRequest {
                reason: "Unrelated picture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: "new:0".into(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("New general note", &text)),
                }],
            },
        },
    )
    .unwrap();
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: p["operation"].as_str().unwrap().into()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    assert!(
        !finish(
            &scope,
            &AiAnswer {
                text,
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
}

#[test]
fn a_queue_claim_excludes_nonqueued_start_in_another_chat_until_acknowledgement() {
    let (_root, mut request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let other = owner_record(
        &mut store,
        "other-chat",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Other chat".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    control(
        &mut store,
        "q",
        ChatCommand::Enqueue {
            thread: request.thread.clone(),
            text: request.prompt.clone(),
        },
    );
    let q = control(&mut store, "dispatch", ChatCommand::Dispatch)
        .dispatched
        .unwrap();
    let original = request.thread.clone();
    request.thread = other.clone();
    assert!(
        begin(&request).is_err(),
        "ordinary start cannot steal a claimed workspace"
    );
    request.thread = original;
    request.request_message = q.message.clone();
    request.queue_claim = q.dispatch.clone();
    let (scope, _) = begin(&request).unwrap();
    assert!(
        finish(
            &scope,
            &AiAnswer {
                text: "Complete".into(),
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
    let RecordData::Run(r) = store.record(&scope.run).unwrap().unwrap().data else {
        panic!()
    };
    control(
        &mut store,
        "ack",
        ChatCommand::Finish {
            item: q.id,
            message: q.message.unwrap(),
            run: scope.run.clone(),
            fence: r.fence,
            success: true,
        },
    );
    request.thread = other;
    request.request_message = None;
    request.queue_claim = None;
    assert!(begin(&request).is_ok());
}
