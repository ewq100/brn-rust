use super::*;

fn owner_write(store: &mut Store, key: &str, writes: Vec<Put>) {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic writing owner");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

pub(super) fn rewrite_fixture(
    protected: bool,
    chat_count: usize,
) -> (tempfile::TempDir, ThreadRunRequest, WritingSnapshot) {
    let (root, mut request, note) = super::tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    if protected {
        let mut data = Note::working("Decision", "Monday");
        data.protected = true;
        owner_write(
            &mut store,
            "protect",
            vec![Put {
                id: note.clone(),
                expected_version: Some(1),
                archived: false,
                data: RecordData::Note(data),
            }],
        );
    }
    let note = store.record(&note).unwrap().unwrap();
    let op = store.allocate_operation("writing-feedback").unwrap();
    let comment = op.creation_id(0);
    let mut writes = vec![Put {
        id: comment.clone(),
        expected_version: None,
        archived: false,
        data: RecordData::Comment(Comment {
            note: note.id.clone(),
            base_version: note.version,
            quote: "Monday".into(),
            range: Some((0, 6)),
            mapped_version: None,
            mapped_range: None,
            body: "Replace the weekday".into(),
            unresolved: false,
        }),
    }];
    let mut message_ids = Vec::new();
    for i in 0..chat_count {
        let id = op.creation_id(i + 1);
        message_ids.push(id.clone());
        writes.push(Put {
            id,
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: request.thread.clone(),
                role: if i % 2 == 0 { "owner" } else { "assistant" }.into(),
                text: format!("captured-post-{i:03}"),
            }),
        });
    }
    owner_write(&mut store, "writing-feedback", writes);
    let op = store.allocate_operation("saved-writing-request").unwrap();
    let snapshot = WritingSnapshot {
        setup: None,
        request_message: op.creation_id(0),
        thread: request.thread.clone(),
        note,
        comments: vec![store.record(&comment).unwrap().unwrap()],
        messages: message_ids
            .iter()
            .map(|id| store.record(id).unwrap().unwrap())
            .collect(),
        general_feedback: "Keep the rest of the note unchanged; clarify the weekday".into(),
    };
    owner_write(
        &mut store,
        "saved-writing-request",
        vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: request.thread.clone(),
                role: WRITING_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
    );
    request.owner_request = Some(OwnerRequest::RewriteNote {
        request_message: snapshot.request_message.clone(),
    });
    request.prompt = "THIS MUTABLE PROMPT MUST NOT BECOME REWRITE CONTEXT".into();
    (root, request, snapshot)
}

fn proposal(snapshot: &WritingSnapshot) -> PrepareArgs {
    PrepareArgs {
        request_key: "whole-note".into(),
        request: snapshot.change_request("Tuesday").unwrap(),
    }
}

#[test]
fn rewrite_uses_full_saved_chat_and_never_posts_synthetic_owner_messages_on_retry() {
    let (_root, request, snapshot) = rewrite_fixture(false, 45);
    let mut store = Store::open(&request.data_dir).unwrap();
    let op = store.allocate_operation("later-chat").unwrap();
    owner_write(
        &mut store,
        "later-chat",
        vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: request.thread.clone(),
                role: "owner".into(),
                text: "LATER FEEDBACK OUTSIDE SNAPSHOT".into(),
            }),
        }],
    );
    let before = store
        .records()
        .unwrap()
        .iter()
        .filter(|r| matches!(&r.data, RecordData::Message(_)))
        .count();
    for _ in 0..2 {
        let (_, input) = begin(&request).unwrap();
        assert!(input.contains("captured-post-000"));
        assert!(input.contains("captured-post-044"));
        assert!(input.contains("Replace the weekday"));
        assert!(input.contains(&snapshot.general_feedback));
        assert!(!input.contains("LATER FEEDBACK"));
        assert!(!input.contains(&request.prompt));
        assert!(input.contains(&snapshot.request_message));
    }
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(|r| matches!(&r.data, RecordData::Message(_)))
            .count(),
        before
    );
}

#[test]
fn rewrite_forces_all_current_inputs_and_review_for_ordinary_and_protected_notes() {
    for protected in [false, true] {
        let (_root, request, snapshot) = rewrite_fixture(protected, 3);
        let (scope, _) = begin(&request).unwrap();
        let mut args = proposal(&snapshot);
        args.request.inputs = vec![RevisionInput {
            record: snapshot.note.id.clone(),
            version: snapshot.note.version,
            expectation: InputExpectation::Historical,
        }];
        let prepared: Prepared = serde_json::from_value(prepare(&scope, args).unwrap()).unwrap();
        assert_eq!(
            prepared
                .request
                .inputs
                .iter()
                .filter(|i| i.expectation != InputExpectation::ChatContext)
                .cloned()
                .collect::<Vec<_>>(),
            snapshot.current_inputs()
        );
        assert!(
            prepared
                .request
                .inputs
                .iter()
                .any(|i| i.record == snapshot.thread
                    && i.expectation == InputExpectation::ChatContext)
        );
        let result = apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into(),
            },
        )
        .unwrap();
        assert_eq!(result["outcome"], "NeedsReview");
        let store = scope.store.lock().unwrap();
        assert_eq!(
            store.record(&snapshot.note.id).unwrap().unwrap(),
            snapshot.note
        );
        assert!(store.receipt(&prepared.operation).unwrap().is_none());
        assert!(
            matches!(store.record(&snapshot.thread).unwrap().unwrap().data,
            RecordData::Thread(thread) if thread.attention.iter().any(|attention|
                attention.kind == AttentionKind::Review && attention.record.as_deref() == Some(prepared.operation.as_str())))
        );
    }
}

#[test]
fn rewrite_rejects_target_metadata_and_extra_writes() {
    for bad in ["target", "version", "title", "protect", "archive", "extra"] {
        let (_root, request, snapshot) = rewrite_fixture(false, 1);
        let (scope, _) = begin(&request).unwrap();
        let mut args = proposal(&snapshot);
        match bad {
            "target" => args.request.writes[0].id = "another-note".into(),
            "version" => args.request.writes[0].expected_version = Some(snapshot.note.version + 1),
            "archive" => args.request.writes[0].archived = true,
            "extra" => args.request.writes.push(Put {
                id: "new:0".into(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working("Other", "scope expansion")),
            }),
            "title" | "protect" => {
                if let RecordData::Note(note) = &mut args.request.writes[0].data {
                    if bad == "title" {
                        note.title = "Changed title".into();
                    } else {
                        note.protected = true;
                    }
                }
            }
            _ => unreachable!(),
        }
        assert!(prepare(&scope, args).is_err(), "accepted {bad}");
        assert_eq!(
            scope
                .store
                .lock()
                .unwrap()
                .record(&snapshot.note.id)
                .unwrap()
                .unwrap(),
            snapshot.note
        );
    }
}

#[test]
fn rewrite_rejects_forged_snapshot_foreign_thread_and_transient_sources() {
    for bad in ["invented-note", "invented-chat", "foreign-thread", "source"] {
        let (_root, mut request, mut snapshot) = rewrite_fixture(false, 1);
        if bad == "source" {
            request.sources.push(TransientSource {
                reference: "new source".into(),
                text: "extra context".into(),
                fingerprint: None,
                report: None,
                assets: vec![],
            });
        } else {
            let mut store = Store::open(&request.data_dir).unwrap();
            let op = store.allocate_operation("forged-writing-request").unwrap();
            snapshot.request_message = op.creation_id(0);
            if bad == "invented-note"
                && let RecordData::Note(note) = &mut snapshot.note.data
            {
                note.markdown = "Monday invented accepted content".into();
            }
            if bad == "invented-chat"
                && let RecordData::Message(message) = &mut snapshot.messages[0].data
            {
                message.text = "Invented owner instruction".into();
            }
            owner_write(
                &mut store,
                "forged-writing-request",
                vec![Put {
                    id: snapshot.request_message.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: request.thread.clone(),
                        role: WRITING_REQUEST_ROLE.into(),
                        text: snapshot.message_text().unwrap(),
                    }),
                }],
            );
            request.owner_request = Some(OwnerRequest::RewriteNote {
                request_message: snapshot.request_message,
            });
            if bad == "foreign-thread" {
                let op = store.allocate_operation("other-writing-thread").unwrap();
                request.thread = op.creation_id(0);
                owner_write(
                    &mut store,
                    "other-writing-thread",
                    vec![Put {
                        id: request.thread.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Thread(Thread {
                            private_context: false,
                            context_epoch: 0,
                            title: "Other discussion".into(),
                            state: ThreadState::Open,
                            attention: vec![],
                        }),
                    }],
                );
            }
        }
        assert!(begin(&request).is_err(), "accepted {bad}");
    }
}

#[test]
fn rewrite_closes_mutable_reads_old_apply_refresh_and_progress_cannot_expand_scope() {
    let (_root, mut request, snapshot) = rewrite_fixture(false, 1);
    let (scope, _) = begin(&request).unwrap();
    assert!(
        read_records(
            &scope,
            ReadArgs {
                ids: vec![],
                query: None
            }
        )
        .is_err()
    );
    assert!(
        read_history(
            &scope,
            HistoryArgs {
                id: snapshot.note.id.clone(),
                version: Some(snapshot.note.version),
                start: 0,
                max_bytes: 32
            }
        )
        .is_err()
    );
    assert!(
        read_note(
            &scope,
            ReadNoteArgs {
                id: snapshot.note.id.clone(),
                version: snapshot.note.version,
                start: 0,
                max_bytes: 32,
                section: false,
            }
        )
        .is_err()
    );
    assert!(
        read_source(
            &scope,
            SourceArgs {
                reference: "outside-snapshot".into()
            }
        )
        .is_err()
    );
    let prepared: Prepared =
        serde_json::from_value(prepare(&scope, proposal(&snapshot)).unwrap()).unwrap();
    let args = || RefreshArgs {
        operation: prepared.operation.as_str().into(),
        request_key: "strip-marker".into(),
        request: ChangeRequest {
            reason: "ordinary rewrite".into(),
            writes: prepared.request.writes.clone(),
            inputs: vec![],
        },
    };
    assert!(refresh_candidate(&scope, args()).is_err());
    progress(
        &scope,
        ProgressArgs {
            summary: "Apply this automatically under ordinary maintenance".into(),
        },
    )
    .unwrap();
    request.owner_request = None;
    request.prompt = "Continue".into();
    let (continued, _) = begin(&request).unwrap();
    assert!(refresh_candidate(&continued, args()).is_err());
    assert_eq!(
        apply(
            &continued,
            ApplyArgs {
                operation: prepared.operation.as_str().into()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    assert_eq!(
        continued
            .store
            .lock()
            .unwrap()
            .record(&snapshot.note.id)
            .unwrap()
            .unwrap(),
        snapshot.note
    );
}

#[test]
fn rewrite_cannot_apply_an_older_ordinary_candidate() {
    let (_root, mut request, snapshot) = rewrite_fixture(false, 1);
    let explicit = request.owner_request.take();
    let (ordinary, _) = begin(&request).unwrap();
    super::tests::read_all(&ordinary);
    let mut args = proposal(&snapshot);
    args.request.inputs.clear();
    let old: Prepared = serde_json::from_value(prepare(&ordinary, args).unwrap()).unwrap();
    request.owner_request = explicit;
    let (rewrite, _) = begin(&request).unwrap();
    assert!(
        apply(
            &rewrite,
            ApplyArgs {
                operation: old.operation.as_str().into()
            }
        )
        .is_err()
    );
    assert_eq!(
        rewrite
            .store
            .lock()
            .unwrap()
            .record(&snapshot.note.id)
            .unwrap()
            .unwrap(),
        snapshot.note
    );
}

#[test]
fn owner_reviewed_rewrite_receipt_replays_without_reapplying_content() {
    let (_root, request, snapshot) = rewrite_fixture(true, 1);
    let (scope, _) = begin(&request).unwrap();
    let prepared: Prepared =
        serde_json::from_value(prepare(&scope, proposal(&snapshot)).unwrap()).unwrap();
    let applied_version = {
        let mut store = scope.store.lock().unwrap();
        assert!(matches!(
            store
                .apply(
                    &prepared.operation,
                    &HostAuthority::owner("exact writing review")
                )
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        store.record(&snapshot.note.id).unwrap().unwrap().version
    };
    let replay = apply(
        &scope,
        ApplyArgs {
            operation: prepared.operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(replay["outcome"], "Applied");
    assert_eq!(
        scope
            .store
            .lock()
            .unwrap()
            .record(&snapshot.note.id)
            .unwrap()
            .unwrap()
            .version,
        applied_version
    );
}

#[test]
fn captured_note_comment_or_chat_changes_stale_owner_reviewed_apply() {
    for captured_kind in ["note", "comment", "message"] {
        let (_root, request, snapshot) = rewrite_fixture(false, 1);
        let (scope, _) = begin(&request).unwrap();
        let prepared: Prepared =
            serde_json::from_value(prepare(&scope, proposal(&snapshot)).unwrap()).unwrap();
        let mut changed = match captured_kind {
            "note" => snapshot.note.clone(),
            "comment" => snapshot.comments[0].clone(),
            _ => snapshot.messages[0].clone(),
        };
        match &mut changed.data {
            RecordData::Note(note) => note.markdown = "Wednesday".into(),
            RecordData::Comment(comment) => comment.body = "New feedback".into(),
            RecordData::Message(message) => message.text = "Changed posted instruction".into(),
            _ => unreachable!(),
        }
        let mut store = scope.store.lock().unwrap();
        owner_write(
            &mut store,
            "concurrent-captured-change",
            vec![Put {
                id: changed.id.clone(),
                expected_version: Some(changed.version),
                archived: false,
                data: changed.data,
            }],
        );
        assert!(
            matches!(store.apply(&prepared.operation, &HostAuthority::owner("exact candidate review")).unwrap(), ApplyOutcome::Stale { records } if records.contains(&changed.id))
        );
        assert!(store.receipt(&prepared.operation).unwrap().is_none());
    }
}

#[test]
fn rewrite_rejects_oversized_saved_context_and_preserves_request_after_failed_startup() {
    let (_root, mut request, snapshot) = rewrite_fixture(false, 1);
    request.max_tool_rounds = 0;
    assert!(begin(&request).is_err());
    let mut store = Store::open(&request.data_dir).unwrap();
    let saved = store.record(&snapshot.request_message).unwrap().unwrap();
    assert_eq!(WritingSnapshot::from_message(&saved).unwrap(), snapshot);
    request.max_tool_rounds = 4;
    assert!(begin(&request).is_ok());
    let op = store.allocate_operation("oversized-request").unwrap();
    let id = op.creation_id(0);
    let mut oversized = snapshot.clone();
    oversized.request_message = id.clone();
    if let RecordData::Message(message) = &mut oversized.messages[0].data {
        message.text = "x".repeat(MAX_WRITING_REQUEST_BYTES);
    }
    assert!(oversized.message_text().is_err());
    owner_write(
        &mut store,
        "oversized-request",
        vec![Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: snapshot.thread.clone(),
                role: WRITING_REQUEST_ROLE.into(),
                text: serde_json::to_string(&json!({"version":1,"snapshot":oversized})).unwrap(),
            }),
        }],
    );
    request.owner_request = Some(OwnerRequest::RewriteNote {
        request_message: id,
    });
    assert!(begin(&request).is_err());
}

#[tokio::test]
async fn mock_rewrite_prepares_review_without_apply_and_finishes_without_changing_the_note() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let (_root, request, snapshot) = rewrite_fixture(false, 1);
    let (scope, input) = begin(&request).unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "prepare",
                "prepare_changes",
                json!({"request_key":"whole-note","request":snapshot.change_request("Tuesday").unwrap()}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("The whole-note proposal is ready for your review."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let answer = run_model(
        model,
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(matches!(answer.terminal, AiTerminal::Completed));
    assert!(finish(&scope, &answer).unwrap());
    let store = scope.store.lock().unwrap();
    assert_eq!(
        store.record(&snapshot.note.id).unwrap().unwrap(),
        snapshot.note
    );
    let candidates = store.candidates().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0]
            .request
            .inputs
            .iter()
            .filter(|i| i.expectation != InputExpectation::ChatContext)
            .cloned()
            .collect::<Vec<_>>(),
        snapshot.current_inputs()
    );
    assert!(
        matches!(store.record(&snapshot.thread).unwrap().unwrap().data,
        RecordData::Thread(thread) if thread.attention.iter().any(|attention|
            attention.kind == AttentionKind::Review && attention.record.as_deref() == Some(candidates[0].operation.as_str())))
    );
}

#[tokio::test]
async fn prose_only_rewrite_fails_and_saved_request_remains_available_for_retry() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let (_root, request, snapshot) = rewrite_fixture(false, 1);
    let (scope, input) = begin(&request).unwrap();
    let model = MockCompletionModel::from_stream_turns([vec![
        MockStreamEvent::text("I rewrote the note."),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]]);
    let answer = run_model(
        model,
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(
        matches!(answer.terminal, AiTerminal::Failed(ref error) if error.kind == AiErrorKind::InvalidToolUse)
    );
    assert!(finish(&scope, &answer).unwrap());
    assert!(begin(&request).is_ok());
    let store = scope.store.lock().unwrap();
    assert_eq!(
        store.record(&snapshot.note.id).unwrap().unwrap(),
        snapshot.note
    );
    assert_eq!(
        WritingSnapshot::from_message(&store.record(&snapshot.request_message).unwrap().unwrap())
            .unwrap(),
        snapshot
    );
    assert!(store.candidates().unwrap().is_empty());
}

#[test]
fn setup_configuration_is_never_model_readable_or_model_writable() {
    let (_root, request, note) = super::tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let op = store.allocate_operation("private-configuration").unwrap();
    let setup = WritingSetup {
        note,
        chat: None,
        brief: Some("Owner setup metadata".into()),
        references: vec![],
    };
    owner_write(
        &mut store,
        "private-configuration",
        vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(setup.clone()),
        }],
    );
    let (scope, _) = begin(&request).unwrap();
    for ids in [vec![], vec![op.creation_id(0)]] {
        let records = read_records(&scope, ReadArgs { ids, query: None }).unwrap();
        assert!(!records.to_string().contains("Owner setup metadata"));
        assert!(!records.to_string().contains("WritingSetup"));
    }
    assert!(!parse_valid(
        "prepare_changes",
        &json!({"request_key":"forged-config","request":{"reason":"model config","writes":[{"id":"new","expected_version":null,"archived":false,"data":{"WritingSetup":setup}}],"inputs":[]}})
    ));
}

#[test]
fn v2_model_request_uses_canonical_metadata_without_reference_content_and_refuses_revocation() {
    let (_root, mut request, mut snapshot) = rewrite_fixture(false, 0);
    let mut store = Store::open(&request.data_dir).unwrap();
    let reference_op = store.allocate_operation("v2-reference").unwrap();
    owner_write(
        &mut store,
        "v2-reference",
        vec![Put {
            id: reference_op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working(
                "Secret reference title",
                "REFERENCE CONTENT NOT CAPTURED",
            )),
        }],
    );
    let setup_op = store.allocate_operation("v2-setup").unwrap();
    owner_write(
        &mut store,
        "v2-setup",
        vec![Put {
            id: setup_op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(WritingSetup {
                note: snapshot.note.id.clone(),
                chat: None,
                brief: Some("Use concise prose".into()),
                references: vec![WritingReference {
                    note: reference_op.creation_id(0),
                    purpose: WritingPurpose::Style,
                }],
            }),
        }],
    );
    let marker = store.allocate_operation("v2-marker").unwrap();
    snapshot.request_message = marker.creation_id(0);
    snapshot.setup = Some(WritingSnapshotSetup {
        chat: None,
        input: RevisionInput {
            record: setup_op.creation_id(0),
            version: 1,
            expectation: InputExpectation::Current,
        },
        brief: Some("Use concise prose".into()),
        references: vec![WritingReferenceVersion {
            note: reference_op.creation_id(0),
            version: 1,
            purpose: WritingPurpose::Style,
        }],
    });
    owner_write(
        &mut store,
        "v2-marker",
        vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: request.thread.clone(),
                role: WRITING_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
    );
    request.owner_request = Some(OwnerRequest::RewriteNote {
        request_message: snapshot.request_message.clone(),
    });
    let (scope, input) = begin(&request).unwrap();
    assert!(input.contains("Use concise prose"));
    assert!(input.contains(&reference_op.creation_id(0)));
    assert!(!input.contains("REFERENCE CONTENT NOT CAPTURED"));
    assert!(!input.contains("Secret reference title"));
    drop(scope);
    let writing_request = request.owner_request.take();
    request.prompt = "Ordinary chat without writing setup".into();
    let (ordinary_scope, history) = begin(&request).unwrap();
    assert!(!history.contains("Use concise prose"));
    let read = read_records(
        &ordinary_scope,
        ReadArgs {
            ids: vec![snapshot.request_message.clone()],
            query: None,
        },
    )
    .unwrap();
    assert!(!read.to_string().contains("Use concise prose"));
    drop(ordinary_scope);
    request.owner_request = writing_request;
    let reference = store.record(&reference_op.creation_id(0)).unwrap().unwrap();
    owner_write(
        &mut store,
        "archive-reference",
        vec![Put {
            id: reference.id,
            expected_version: Some(1),
            archived: true,
            data: reference.data,
        }],
    );
    assert!(begin(&request).is_err());
}

#[test]
fn ordinary_chat_setup_preserves_runtime_source_scope_and_delegated_maintenance() {
    let (_root, mut request, note) = super::tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let op = store.allocate_operation("ordinary-context-setup").unwrap();
    let scope = writing_setup_scope(&store.records().unwrap(), Some(&request.thread)).unwrap();
    owner_write(
        &mut store,
        "ordinary-context-setup",
        vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(WritingSetup {
                note: note.clone(),
                chat: scope,
                brief: Some("Ordinary brief".into()),
                references: vec![],
            }),
        }],
    );
    request.sources = vec![TransientSource::text(
        "https://example.invalid/cw1-public-source",
        "Synthetic source, no fetching",
    )];
    let (scope, _) = begin(&request).unwrap();
    assert!(!scope.private_context);
    let source = scope
        .store
        .lock()
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .find(
            |r| matches!(&r.data,RecordData::Source(s) if s.locator==request.sources[0].reference),
        )
        .unwrap();
    assert!(matches!(source.data,RecordData::Source(s) if s.chat.is_none()));
    super::tests::read_all(&scope);
    let record = scope.store.lock().unwrap().record(&note).unwrap().unwrap();
    let mut data = record.data;
    let RecordData::Note(n) = &mut data else {
        panic!()
    };
    n.markdown = "Preserved ordinary delegated maintenance".into();
    let prepared: Prepared = serde_json::from_value(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "ordinary-maintenance".into(),
                request: ChangeRequest {
                    reason: "Ordinary scoped setup must not change delegation".into(),
                    writes: vec![Put {
                        id: note,
                        expected_version: Some(record.version),
                        archived: false,
                        data,
                    }],
                    inputs: vec![],
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    let result = apply(
        &scope,
        ApplyArgs {
            operation: prepared.operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(result["outcome"], "Applied");
}
