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
        writing: None,
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
                reference: "outside-snapshot".into(),
                start: 0,
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
fn v2_model_request_uses_canonical_metadata_without_content_and_excludes_revoked_references() {
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
        requirements: WritingReadRequirements::default(),
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
    let (scope, input) = begin(&request).unwrap();
    assert!(!input.contains(&reference_op.creation_id(0)));
    assert!(!input.contains("REFERENCE CONTENT NOT CAPTURED"));
    assert!(
        read_note(
            &scope,
            ReadNoteArgs {
                id: reference_op.creation_id(0),
                version: 1,
                start: 0,
                max_bytes: 1024,
                section: false
            }
        )
        .is_err()
    );
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
                writing: None,
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

fn contract_scope(mode: &str) -> (tempfile::TempDir, ThreadRunRequest, Arc<Scope>, String) {
    let (root, request) = match mode {
        "ordinary" => {
            let (root, request, _) = tests::fixture();
            (root, request)
        }
        "v1" => {
            let (root, request, _) = rewrite_fixture(true, 0);
            (root, request)
        }
        "v2" => {
            let (root, request, _, _) =
                cw2_tests::fixture("First Ω requirement.\n", WritingReadRequirements::default());
            (root, request)
        }
        _ => panic!("unknown synthetic mode"),
    };
    Store::open(&request.data_dir)
        .unwrap()
        .set_workspace_purpose(WorkspacePurpose::Test)
        .unwrap();
    let (scope, input) = begin(&request).unwrap();
    (root, request, scope, input)
}
async fn offered_prepare_contract(mode: &str) {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let (_root, _, scope, input) = contract_scope(mode);
    let model = MockCompletionModel::from_stream_turns([vec![
        MockStreamEvent::final_response_with_total_tokens(1),
    ]]);
    let _answer = run_model(
        model.clone(),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert_eq!(model.request_count(), 1);
    let requests = model.requests();
    assert_eq!(
        serde_json::to_value(&requests[0].tools).unwrap(),
        json!(budget_tool_definitions(&scope)),
        "budget and runtime must use the same definitions"
    );
    let offered = &requests[0]
        .tools
        .iter()
        .find(|t| t.name == "prepare_changes")
        .unwrap()
        .parameters;
    let full = schema("prepare_changes");
    let mut expected = full.clone();
    if mode != "v2" {
        expected["properties"]["writing"] = json!({"type":"null"});
    }
    assert_eq!(
        *offered, expected,
        "actual {mode} CompletionRequest must match its supported writing contract"
    );
}
#[tokio::test]
async fn ordinary_offered_prepare_contract() {
    offered_prepare_contract("ordinary").await;
}
#[tokio::test]
async fn v1_offered_prepare_contract() {
    offered_prepare_contract("v1").await;
}
#[tokio::test]
async fn v2_offered_prepare_contract() {
    offered_prepare_contract("v2").await;
}

#[tokio::test]
async fn prepare_contract_serializes_on_codex_and_strict_copilot_responses() {
    use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
    for mode in ["ordinary", "v1", "v2"] {
        for copilot in [false, true] {
            let (_root, request, scope, input) = contract_scope(mode);
            let mut responses = Vec::new();
            if mode == "v2" {
                let snapshot = scope.writing_snapshot.as_ref().unwrap();
                let args = json!({"request_key":"wire-empty-suggestions", "request":snapshot.change_request("Tuesday").unwrap(), "writing":{"quotations":[],"requirements_reported":[],"proposed_addressed_comment_ids":[]}});
                let call = json!({"type":"function_call","id":"fc_prepare","call_id":"call_prepare","name":"prepare_changes","arguments":args.to_string(),"status":"completed"});
                responses.push(MockHttpResponse::success_typed(cache_tests::sse(&[
                    json!({"type":"response.output_item.done","output_index":0,"sequence_number":1,"item":call}),
                    json!({"type":"response.completed","sequence_number":2,"response":{"id":"resp_prepare","object":"response","created_at":1,"status":"completed","model":"gpt-6.1-sol","output":[call],"tools":[],"usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11}}}),
                ]), "text/event-stream"));
            }
            responses.push(MockHttpResponse::success_typed(
                cache_tests::final_turn(),
                "text/event-stream",
            ));
            let http = SequencedHttpClient::new(responses);
            let model = if copilot {
                let client = rig::providers::copilot::CopilotConfig::new("synthetic-session-token")
                    .with_base_url("https://copilot.example.invalid")
                    .connect(DynHttpClient::new(http.clone()));
                crate::auth::copilot_completion(&client, request.selection.model.clone(), true)
                    .into()
            } else {
                let client = rig::providers::openai::OpenAIConfig::with_key(
                    &rig::providers::chatgpt::DIALECT,
                    "synthetic-not-a-credential",
                )
                .connect(DynHttpClient::new(http.clone()));
                rig_core::DynModel::<rig_core::operation::Completion>::from(
                    client.responses(request.selection.model.clone()),
                )
            };
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
            assert_eq!(http.requests().len(), if mode == "v2" { 2 } else { 1 });
            if mode == "v2" {
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{:?}",
                    answer.terminal
                );
                let store = scope.store.lock().unwrap();
                assert_eq!(
                    store.candidates().unwrap().len(),
                    1,
                    "explicit [] must execute through the actual provider transport"
                );
                assert!(store.receipts().unwrap().iter().all(|receipt| {
                    !receipt.writes.iter().any(|write| {
                        write.after.id == scope.writing_snapshot.as_ref().unwrap().note.id
                            && write.after.version
                                > scope.writing_snapshot.as_ref().unwrap().note.version
                    })
                }));
            }
            let requests = http.requests();
            assert!(requests[0].uri.ends_with("/responses"));
            let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
            assert_eq!(body["model"], "gpt-6.1-sol");
            assert_eq!(body["reasoning"]["effort"], "medium");
            let tool = body["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == "prepare_changes")
                .unwrap();
            assert_eq!(tool["strict"], copilot);
            if mode == "v2" {
                // Independent contract checks, in addition to whole-schema equality.
                let writing = &tool["parameters"]["properties"]["writing"];
                let suggestion = &writing["properties"]["proposed_addressed_comment_ids"];
                assert_eq!(suggestion["type"], "array");
                assert_eq!(suggestion["items"]["type"], "string");
                assert_eq!(suggestion["maxItems"], 128);
                assert_eq!(writing["additionalProperties"], false);
                let required = writing["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<std::collections::BTreeSet<_>>();
                let expected = if copilot {
                    vec![
                        "quotations",
                        "requirements_reported",
                        "proposed_addressed_comment_ids",
                    ]
                } else {
                    vec!["quotations", "requirements_reported"]
                };
                assert_eq!(required, expected.into_iter().collect());
                let mut expected_writing =
                    schema("prepare_changes")["properties"]["writing"].clone();
                if copilot {
                    expected_writing["required"] = expected_writing["properties"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .into();
                }
                assert_eq!(
                    tool["parameters"]["properties"]["writing"],
                    expected_writing
                );
            } else {
                assert_eq!(
                    tool["parameters"]["properties"]["writing"],
                    json!({"type":"null"})
                );
            }
            assert_eq!(
                tool["parameters"]["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "writing"),
                copilot
            );
            if !copilot {
                assert_eq!(
                    tool["parameters"],
                    PrepareChanges::for_scope(&scope).parameters()
                );
            }
            let refresh = body["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == "refresh_candidate")
                .unwrap();
            assert_eq!(refresh["strict"], copilot);
            assert_eq!(
                refresh["parameters"]["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                ["operation", "request_key", "request"]
                    .into_iter()
                    .collect()
            );
            assert_eq!(
                refresh["parameters"]["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<std::collections::BTreeSet<_>>(),
                ["operation", "request_key", "request"]
                    .into_iter()
                    .collect()
            );
            assert_eq!(
                refresh["parameters"]["properties"]["request"],
                tool["parameters"]["properties"]["request"]
            );
            assert_eq!(
                refresh["parameters"]["properties"]["request_key"],
                tool["parameters"]["properties"]["request_key"]
            );
            assert_eq!(
                refresh["parameters"]["properties"]["operation"]["type"],
                "string"
            );
            for field in ["max_output_tokens", "max_tokens", "max_completion_tokens"] {
                assert!(body.get(field).is_none());
            }
        }
    }
}

#[tokio::test]
async fn absent_and_null_prepare_still_register_protected_candidates_without_apply_authority() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    for mode in ["ordinary", "v1"] {
        for explicit_null in [false, true] {
            let (root, request, note, change) = if mode == "ordinary" {
                let (root, request, note) = tests::fixture();
                let mut store = Store::open(&request.data_dir).unwrap();
                store.set_workspace_purpose(WorkspacePurpose::Test).unwrap();
                let record = store.record(&note).unwrap().unwrap();
                let RecordData::Note(mut data) = record.data else {
                    panic!()
                };
                data.protected = true;
                owner_write(
                    &mut store,
                    "protect-contract",
                    vec![Put {
                        id: note.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Note(data),
                    }],
                );
                let record = store.record(&note).unwrap().unwrap();
                let RecordData::Note(mut data) = record.data else {
                    panic!()
                };
                data.markdown = "Tuesday proposed only".into();
                let change = ChangeRequest {
                    reason: "Synthetic protected proposal".into(),
                    writes: vec![Put {
                        id: note.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Note(data),
                    }],
                    inputs: vec![],
                };
                (root, request, note, change)
            } else {
                let (root, request, snapshot) = rewrite_fixture(true, 0);
                Store::open(&request.data_dir)
                    .unwrap()
                    .set_workspace_purpose(WorkspacePurpose::Test)
                    .unwrap();
                let change = snapshot.change_request("Tuesday proposed only").unwrap();
                (root, request, snapshot.note.id.clone(), change)
            };
            let _root = root;
            let (scope, input) = begin(&request).unwrap();
            let original = scope.store.lock().unwrap().record(&note).unwrap().unwrap();
            let operation = scope
                .store
                .lock()
                .unwrap()
                .allocate_operation(&format!("{}:proposal:supported-contract", scope.run))
                .unwrap();
            let mut args = json!({"request_key":"supported-contract","request":change});
            if explicit_null {
                args["writing"] = Value::Null;
            }
            let mut turns = vec![];
            if mode == "ordinary" {
                turns.push(vec![
                    MockStreamEvent::tool_call(
                        "read",
                        "read_records",
                        json!({"ids":[note],"query":null}),
                    ),
                    MockStreamEvent::final_response_with_total_tokens(1),
                ]);
            }
            turns.push(vec![
                MockStreamEvent::tool_call("prepare", "prepare_changes", args),
                MockStreamEvent::final_response_with_total_tokens(1),
            ]);
            turns.push(vec![
                MockStreamEvent::tool_call(
                    "apply",
                    "apply_changes",
                    json!({"operation":operation.as_str()}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ]);
            turns.push(vec![MockStreamEvent::final_response_with_total_tokens(1)]);
            let model = MockCompletionModel::from_stream_turns(turns);
            let activity = Arc::new(Mutex::new(Vec::new()));
            let emitted = activity.clone();
            let answer = run_model(
                model.clone(),
                scope.clone(),
                &input,
                ReasoningEffort::Medium,
                4,
                true,
                CancellationToken::new(),
                Arc::new(move |event| {
                    if let AiEvent::Activity(message) = event {
                        emitted.lock().unwrap().push(message);
                    }
                }),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{mode}/{explicit_null}: {:?}",
                answer.terminal
            );
            assert_eq!(
                model.request_count(),
                if mode == "ordinary" { 4 } else { 3 }
            );
            assert_eq!(
                activity
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|message| message.as_str() == "< apply_changes · NeedsReview")
                    .count(),
                1,
                "{mode}/{explicit_null}: expected one actual NeedsReview outcome"
            );
            let store = scope.store.lock().unwrap();
            let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
                panic!()
            };
            assert_eq!(run.operations, vec![operation.clone()]);
            assert_eq!(
                store.prepared(&operation).unwrap().request.writes,
                change.writes
            );
            assert!(store.receipt(&operation).unwrap().is_none());
            assert_eq!(store.record(&note).unwrap().unwrap(), original);
        }
    }
}
