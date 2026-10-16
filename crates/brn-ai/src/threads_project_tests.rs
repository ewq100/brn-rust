//! Offline project boundaries exercise the actual runtime handlers and fences.
use super::*;
use sha2::{Digest, Sha256};
fn project_command(store: &mut Store, command: ProjectCommand) -> Receipt {
    let key = unique_host_key();
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&command).unwrap())
    );
    let op = store.allocate_bound_operation(&key, &hash).unwrap();
    store
        .project_command(&op, &command, &HostAuthority::owner("fixture"))
        .unwrap()
}
fn saved(store: &mut Store, data: RecordData) -> Record {
    let op = store.allocate_operation(&unique_host_key()).unwrap();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "fixture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data,
                }],
            },
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    let ApplyOutcome::Applied(r) = store.apply(&op, &HostAuthority::owner("fixture")).unwrap()
    else {
        panic!()
    };
    r.writes[0].after.clone()
}
#[test]
fn project_instructions_discussion_and_current_document_context_use_real_handlers() {
    let (_root, request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let project = project_command(
        &mut store,
        ProjectCommand::Create {
            title: "Japan trip".into(),
        },
    )
    .writes[0]
        .after
        .clone();
    project_command(
        &mut store,
        ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(project.id.clone()),
        },
    );
    let earlier = project_command(
        &mut store,
        ProjectCommand::CreateChat {
            project: project.id.clone(),
            title: "Earlier route".into(),
        },
    )
    .writes
    .into_iter()
    .find(|w| matches!(w.after.data, RecordData::Thread(_)))
    .unwrap()
    .after;
    let discussion = saved(
        &mut store,
        RecordData::Message(Message {
            thread: earlier.id,
            role: "assistant".into(),
            text: "Suggested Hakone route, not adopted".into(),
        }),
    );
    let document = project_command(
        &mut store,
        ProjectCommand::CreateDocument {
            project: project.id.clone(),
            title: "Itinerary".into(),
            markdown: "Adopted Hakone itinerary".into(),
        },
    )
    .writes
    .into_iter()
    .find(|w| matches!(w.after.data, RecordData::Note(_)))
    .unwrap()
    .after;
    let general = saved(
        &mut store,
        RecordData::Note(Note::working("General Hakone", "Private global lookup")),
    );
    let r = store.record(&project.id).unwrap().unwrap();
    project_command(
        &mut store,
        ProjectCommand::Configure {
            project: project.id.clone(),
            version: r.version,
            title: "Japan trip".into(),
            instructions: "For beginners, in Estonian".into(),
            use_brn_knowledge: false,
        },
    );
    let (scope, input) = begin(&request).unwrap();
    assert!(input.contains("For beginners, in Estonian"));
    assert!(input.contains(&document.id));
    assert!(!input.contains(&general.id));
    let rows = read_records(
        &scope,
        ReadArgs {
            ids: vec![],
            query: Some("Hakone".into()),
        },
    )
    .unwrap();
    let encoded = rows.to_string();
    assert!(encoded.contains(&discussion.id));
    assert!(encoded.contains("Earlier discussion"));
    assert!(!encoded.contains(&general.id));
    assert!(
        read_records(
            &scope,
            ReadArgs {
                ids: vec![general.id.clone()],
                query: None
            }
        )
        .unwrap()["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        read_note(
            &scope,
            ReadNoteArgs {
                id: general.id,
                version: 1,
                start: 0,
                max_bytes: 100,
                section: false
            }
        )
        .is_err()
    );
    let mut proposal = ChangeRequest {
        reason: "Propose".into(),
        inputs: vec![],
        writes: vec![Put {
            id: "new:0".into(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working("Local result", "Hakone")),
        }],
    };
    proposal.writes.push(Put {
        id: "new:1".into(),
        expected_version: None,
        archived: false,
        data: RecordData::Action(Action {
            chat: None,
            kept_outside_project: false,
            description: "Private proposed follow-up".into(),
            state: ActionState::Suggested,
            internal: false,
            evidence: None,
            due: None,
        }),
    });
    let guard = scope.store.lock().unwrap();
    scope_request(&scope, &guard, &mut proposal).unwrap();
    bind_project_inputs(&scope, &guard, &mut proposal).unwrap();
    drop(guard);
    assert!(
        matches!(&proposal.writes[0].data, RecordData::Note(n) if n.chat.as_ref() == Some(&request.thread))
    );
    assert!(
        matches!(&proposal.writes[1].data, RecordData::Action(a) if a.chat.as_ref() == Some(&request.thread) && !a.kept_outside_project)
    );
    let mut forged = proposal.clone();
    if let RecordData::Action(a) = &mut forged.writes[1].data {
        a.kept_outside_project = true;
    }
    assert!(scope_request(&scope, &scope.store.lock().unwrap(), &mut forged).is_err());
    assert!(proposal.inputs.iter().any(|i| i.record == project.id));
    let r = store.record(&project.id).unwrap().unwrap();
    project_command(
        &mut store,
        ProjectCommand::Configure {
            project: project.id,
            version: r.version,
            title: "Japan trip".into(),
            instructions: "For advanced readers".into(),
            use_brn_knowledge: true,
        },
    );
    assert!(
        read_records(
            &scope,
            ReadArgs {
                ids: vec![document.id],
                query: None
            }
        )
        .is_err()
    );
    assert!(
        !finish(
            &scope,
            &AiAnswer {
                text: "Late stale answer".into(),
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
    assert!(
        !store
            .records()
            .unwrap()
            .iter()
            .any(|r| matches!(&r.data, RecordData::Message(m) if m.text == "Late stale answer"))
    );
}
#[test]
fn moving_a_live_chat_fences_runtime_and_preserves_its_history_and_paused_queue() {
    let (_root, request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let p = project_command(
        &mut store,
        ProjectCommand::Create {
            title: "Travel".into(),
        },
    )
    .writes[0]
        .after
        .id
        .clone();
    let (scope, _) = begin(&request).unwrap();
    project_command(
        &mut store,
        ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(p),
        },
    );
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
        !finish(
            &scope,
            &AiAnswer {
                text: "late".into(),
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
    assert!(store.records().unwrap().iter().any(|r| matches!(&r.data, RecordData::Message(m) if m.thread == request.thread && m.role == "owner")));
}

#[test]
fn reviewed_project_rewrite_applies_after_attention_but_context_movement_invalidates_it() {
    for move_after in [false, true] {
        let (_root, request, snapshot) = writing_tests::rewrite_fixture(true, 2);
        let mut store = Store::open(&request.data_dir).unwrap();
        let p = project_command(
            &mut store,
            ProjectCommand::Create {
                title: "Writing".into(),
            },
        )
        .writes[0]
            .after
            .id
            .clone();
        project_command(
            &mut store,
            ProjectCommand::MoveChat {
                chat: request.thread.clone(),
                project: Some(p),
            },
        );
        let (scope, _) = begin(&request).unwrap();
        let proposal = prepare(
            &scope,
            PrepareArgs {
                writing: None,
                request_key: "rewrite".into(),
                request: snapshot.change_request("Tuesday").unwrap(),
            },
        )
        .unwrap();
        let prepared: Prepared = serde_json::from_value(proposal).unwrap();
        let thread = store.record(&request.thread).unwrap().unwrap();
        assert!(
            matches!(&thread.data, RecordData::Thread(t) if t.attention.iter().any(|a| a.kind == AttentionKind::Review))
        );
        if move_after {
            project_command(
                &mut store,
                ProjectCommand::MoveChat {
                    chat: request.thread.clone(),
                    project: None,
                },
            );
        }
        let applied = store
            .apply(
                &prepared.operation,
                &HostAuthority::owner("owner accepted exact candidate"),
            )
            .unwrap();
        if move_after {
            assert!(matches!(applied, ApplyOutcome::Stale { .. }));
        } else {
            assert!(matches!(applied, ApplyOutcome::Applied(_)));
        }
    }
}

#[test]
fn deleting_working_project_retains_checked_progress_and_displayed_partial_answer_through_trash() {
    let (_root, request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let p = project_command(
        &mut store,
        ProjectCommand::Create {
            title: "Working project".into(),
        },
    )
    .writes[0]
        .after
        .id
        .clone();
    project_command(
        &mut store,
        ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(p.clone()),
        },
    );
    let (scope, _) = begin(&request).unwrap();
    update_run(&scope, &mut store, |r| {
        r.progress = "Useful saved investigation summary".into()
    })
    .unwrap();
    let preview = store.project_delete_preview(&p).unwrap();
    let r = project_command(
        &mut store,
        ProjectCommand::WithProgress {
            command: Box::new(ProjectCommand::Delete {
                preview,
                actions: BTreeMap::new(),
            }),
            run: scope.run.clone(),
            partial: "Useful displayed partial answer · õun".into(),
        },
    );
    let partial = r
        .writes
        .iter()
        .find(|w| matches!(&w.after.data, RecordData::Message(m) if m.role == "assistant-partial"))
        .unwrap()
        .after
        .clone();
    assert!(partial.archived);
    assert!(
        matches!(store.record(&scope.run).unwrap().unwrap().data, RecordData::Run(r) if r.progress == "Useful saved investigation summary" && r.state == RunState::Interrupted)
    );
    let deleted = store.record(&p).unwrap().unwrap();
    project_command(
        &mut store,
        ProjectCommand::Restore {
            project: p,
            version: deleted.version,
        },
    );
    let retained = store.record(&partial.id).unwrap().unwrap();
    assert!(!retained.archived);
    assert!(
        matches!(retained.data, RecordData::Message(m) if m.role == "assistant-partial" && m.text == "Useful displayed partial answer · õun")
    );
    assert!(
        !finish(
            &scope,
            &AiAnswer {
                text: "Late finished answer".into(),
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
}

#[test]
fn model_authority_cannot_change_the_host_context_epoch() {
    let (_root, request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let p = project_command(
        &mut store,
        ProjectCommand::Create {
            title: "Private".into(),
        },
    )
    .writes[0]
        .after
        .id
        .clone();
    project_command(
        &mut store,
        ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(p),
        },
    );
    let (scope, _) = begin(&request).unwrap();
    let record = store.record(&request.thread).unwrap().unwrap();
    for next_epoch in [0, 99] {
        let mut data = record.data.clone();
        if let RecordData::Thread(t) = &mut data {
            t.context_epoch = next_epoch;
        }
        let op = store.allocate_operation(&unique_host_key()).unwrap();
        store
            .prepare(
                &op,
                &ChangeRequest {
                    reason: "Forged context fence".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: record.id.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data,
                    }],
                },
            )
            .unwrap();
        let outcome = store.apply(&op, &bind(&scope, HostAuthority::maintenance("model")));
        assert!(matches!(
            outcome,
            Err(_) | Ok(ApplyOutcome::NeedsReview { .. })
        ));
        assert_eq!(store.record(&record.id).unwrap().unwrap(), record);
    }
}
