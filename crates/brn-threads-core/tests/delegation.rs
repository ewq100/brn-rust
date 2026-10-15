use brn_threads_core::*;

fn fixture() -> (tempfile::TempDir, Store) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let store = Store::open(root.path().join("fresh")).unwrap();
    (root, store)
}

fn candidate(store: &mut Store, key: &str) -> OperationId {
    let op = store.allocate_operation(key).unwrap();
    store
        .prepare(
            &op,
            &ChangeRequest {
                reason: "Synthetic ordinary note".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Summary", "Synthetic content")),
                }],
            },
        )
        .unwrap();
    op
}

fn legacy_settings(store: &mut Store, archived: bool) -> (OperationId, ChangeRequest, Prepared) {
    let op = store.allocate_operation("legacy settings").unwrap();
    let body = format!(
        r#"{{"reason":"Legacy configuration","writes":[{{"id":"{}","expected_version":null,"archived":{},"data":{{"Settings":{{"provider":"Codex","model":"gpt-6.1-sol","effort":"medium","auth_file":null,"credentials_dir":null,"maintenance":true,"review_first":false}}}}}}],"inputs":[]}}"#,
        op.creation_id(0),
        archived
    );
    let request: ChangeRequest = serde_json::from_str(&body).unwrap();
    assert_eq!(serde_json::to_string(&request).unwrap(), body);
    let prepared = store
        .prepare_owner(&op, &request, &HostAuthority::owner("fixture"))
        .unwrap();
    use sha2::{Digest, Sha256};
    assert_eq!(
        prepared.request_hash,
        format!("{:x}", Sha256::digest(body.as_bytes()))
    );
    assert!(matches!(
        store.apply(&op, &HostAuthority::owner("fixture")).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    (op, request, prepared)
}

fn settings(store: &Store) -> WorkspaceSettings {
    store
        .records()
        .unwrap()
        .into_iter()
        .find_map(|record| {
            if let RecordData::Settings(settings) = record.data {
                Some(settings)
            } else {
                None
            }
        })
        .unwrap()
}

#[test]
fn legacy_and_archived_settings_deny_standing_authority() {
    for archived in [false, true] {
        let (_root, mut store) = fixture();
        legacy_settings(&mut store, archived);
        let op = candidate(&mut store, "legacy standing attempt");
        assert!(matches!(
            store
                .apply(&op, &HostAuthority::maintenance("agent"))
                .unwrap(),
            ApplyOutcome::NeedsReview { .. }
        ));
        assert!(matches!(
            store.apply(&op, &HostAuthority::owner("review")).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }
}

#[test]
fn legacy_request_bytes_hash_and_receipt_replay_after_choice_and_reopen() {
    let (root, mut store) = fixture();
    let (legacy, request, prepared) = legacy_settings(&mut store, false);
    let receipt = store.receipt(&legacy).unwrap().unwrap();
    let choice = store.allocate_operation("owner choice").unwrap();
    store
        .choose_delegation(
            &choice,
            DelegationChoice::MaintainOrdinaryNotes,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    assert_eq!(
        store
            .prepare_owner(&legacy, &request, &HostAuthority::owner("retry"))
            .unwrap(),
        prepared
    );
    assert_eq!(
        store
            .apply(&legacy, &HostAuthority::owner("retry"))
            .unwrap(),
        ApplyOutcome::Applied(receipt)
    );
    assert!(settings(&store).delegation_chosen);
}

#[test]
fn choices_bind_intent_preserve_concurrent_provider_and_replay_without_reapplying() {
    let (root, mut store) = fixture();
    legacy_settings(&mut store, false);
    let choice = store
        .allocate_operation("choice allocated before provider update")
        .unwrap();
    let mut other = Store::open(store.directory()).unwrap();
    let mut selected = settings(&other);
    selected.provider = "Offline".into();
    selected.model = "synthetic".into();
    selected.effort = "high".into();
    selected.auth_file = Some("/synthetic/empty-auth".into());
    selected.credentials_dir = Some("/synthetic/empty-credentials".into());
    let configure = other.allocate_operation("new provider selection").unwrap();
    other
        .configure_workspace(&configure, &selected, &HostAuthority::owner("owner"))
        .unwrap();
    let receipt = store
        .choose_delegation(
            &choice,
            DelegationChoice::MaintainOrdinaryNotes,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    selected.delegation_chosen = true;
    assert_eq!(settings(&store), selected);
    assert!(matches!(
        store.choose_delegation(
            &choice,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("retry")
        ),
        Err(Error::ImmutableRequest)
    ));
    let review = store.allocate_operation("later review first").unwrap();
    store
        .choose_delegation(
            &review,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let denied = candidate(&mut store, "review first attempt");
    assert!(matches!(
        store
            .apply(&denied, &HostAuthority::maintenance("agent"))
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    drop(other);
    drop(store);
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    assert_eq!(
        store
            .choose_delegation(
                &choice,
                DelegationChoice::MaintainOrdinaryNotes,
                &HostAuthority::owner("retry")
            )
            .unwrap(),
        receipt
    );
    assert!(settings(&store).review_first);
    assert!(matches!(
        store
            .apply(&denied, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn generic_configuration_cannot_forge_or_clear_choice() {
    let (_root, mut store) = fixture();
    legacy_settings(&mut store, false);
    for chosen in [false, true] {
        let mut desired = settings(&store);
        desired.delegation_chosen = chosen;
        let configure = store
            .allocate_operation(&format!("forge {chosen}"))
            .unwrap();
        store
            .configure_workspace(&configure, &desired, &HostAuthority::owner("owner"))
            .unwrap();
        assert!(!settings(&store).delegation_chosen);
    }
    let choice = store.allocate_operation("explicit maintain").unwrap();
    store
        .choose_delegation(
            &choice,
            DelegationChoice::MaintainOrdinaryNotes,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let mut desired = settings(&store);
    desired.delegation_chosen = false;
    let configure = store.allocate_operation("cannot clear choice").unwrap();
    store
        .configure_workspace(&configure, &desired, &HostAuthority::owner("owner"))
        .unwrap();
    assert!(settings(&store).delegation_chosen);
    let note = candidate(&mut store, "authorized ordinary");
    assert!(matches!(
        store
            .apply(&note, &HostAuthority::maintenance("agent"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn stale_provider_form_preserves_new_review_choice_and_pause() {
    let (_root, mut store) = fixture();
    legacy_settings(&mut store, false);
    let mut stale = settings(&store);
    stale.provider = "Offline".into();
    let mut other = Store::open(store.directory()).unwrap();
    let choice = other
        .allocate_operation("choose review after form read")
        .unwrap();
    other
        .choose_delegation(
            &choice,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let pause = other.allocate_operation("pause after choice").unwrap();
    other
        .pause_workspace(&pause, true, &HostAuthority::owner("owner"))
        .unwrap();
    let configure = store
        .allocate_operation("save older provider form")
        .unwrap();
    let receipt = store
        .configure_workspace(&configure, &stale, &HostAuthority::owner("owner"))
        .unwrap();
    let current = settings(&other);
    assert!(current.delegation_chosen && current.review_first);
    assert!(!current.maintenance);
    assert_eq!(current.provider, "Offline");
    assert_eq!(
        store
            .configure_workspace(&configure, &stale, &HostAuthority::owner("retry"))
            .unwrap(),
        receipt
    );
}

fn working_run(store: &mut Store, key: &str) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let thread = op.creation_id(0);
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic working invocation".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: thread.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Thread(Thread {
                            private_context: false,
                            context_epoch: 0,
                            title: key.into(),
                            state: ThreadState::Open,
                            attention: vec![],
                        }),
                    },
                    Put {
                        id: op.creation_id(1),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            instruction: None,
                            queue_claim: None,
                            thread,
                            provider: "Codex".into(),
                            model: "gpt-6.1-sol".into(),
                            effort: "medium".into(),
                            guide_identity: "fixture".into(),
                            budget: 3,
                            loaded_skills: vec![],
                            fence: 1,
                            state: RunState::Working,
                            progress: String::new(),
                            operations: vec![],
                            sources: vec![],
                        }),
                    },
                ],
            },
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    store.apply(&op, &HostAuthority::owner("fixture")).unwrap();
    store.record(&op.creation_id(1)).unwrap().unwrap()
}

fn chat_attention(store: &Store, run: &Record) -> Vec<Attention> {
    let RecordData::Run(run) = &run.data else {
        panic!("Run")
    };
    match store.record(&run.thread).unwrap().unwrap().data {
        RecordData::Thread(thread) => thread.attention,
        _ => panic!("Thread"),
    }
}

fn interrupted(run: &str) -> Attention {
    Attention {
        kind: AttentionKind::Blocker,
        reason: run_attention::INTERRUPTED_ATTENTION.into(),
        record: Some(run.into()),
    }
}

#[test]
fn both_choices_and_pause_fence_every_working_run_and_block_stale_connection() {
    for choice in [
        DelegationChoice::MaintainOrdinaryNotes,
        DelegationChoice::ReviewFirst,
    ] {
        let (_root, mut store) = fixture();
        legacy_settings(&mut store, false);
        let mut stale = Store::open(store.directory()).unwrap();
        let selection = store
            .allocate_operation("choice allocated before runs start")
            .unwrap();
        let runs = [
            working_run(&mut stale, "run A"),
            working_run(&mut stale, "run B"),
        ];
        let op = candidate(&mut stale, "old run candidate");
        let receipt = store
            .choose_delegation(&selection, choice, &HostAuthority::owner("owner"))
            .unwrap();
        let ApplyOutcome::Applied(receipt) = receipt else {
            panic!()
        };
        // Settings, both Runs and both of their chats, each with its notice.
        assert_eq!(receipt.writes.len(), 5);
        for old in &runs {
            let RecordData::Run(run) = stale.record(&old.id).unwrap().unwrap().data else {
                panic!()
            };
            assert_eq!(run.state, RunState::Cancelled);
            assert_eq!(run.fence, 2);
            assert_eq!(chat_attention(&stale, old), [interrupted(&old.id)]);
        }
        assert_eq!(
            stale
                .apply(
                    &op,
                    &HostAuthority::maintenance("stale agent").for_run(&runs[0].id, 1)
                )
                .unwrap(),
            ApplyOutcome::Superseded {
                run: runs[0].id.clone()
            }
        );
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
        let fresh = working_run(&mut stale, "run C after choice");
        let pause = store.allocate_operation("pause choice").unwrap();
        let paused = store
            .pause_workspace(&pause, true, &HostAuthority::owner("owner"))
            .unwrap();
        assert_eq!(
            store
                .pause_workspace(&pause, true, &HostAuthority::owner("retry"))
                .unwrap(),
            paused
        );
        assert!(matches!(
            store.pause_workspace(&pause, false, &HostAuthority::owner("changed retry")),
            Err(Error::ImmutableRequest)
        ));
        let RecordData::Run(run) = stale.record(&fresh.id).unwrap().unwrap().data else {
            panic!()
        };
        assert_eq!(run.state, RunState::Cancelled);
        assert_eq!(chat_attention(&stale, &fresh), [interrupted(&fresh.id)]);
        assert!(!settings(&store).maintenance);
        assert_eq!(
            settings(&store).review_first,
            choice == DelegationChoice::ReviewFirst
        );
    }
}

#[test]
fn missing_settings_deny_standing_writes_but_allow_scoped_and_owner_writes() {
    let (_root, mut store) = fixture();
    let op = candidate(&mut store, "scoped pending note");
    assert!(matches!(
        store
            .apply(&op, &HostAuthority::maintenance("agent"))
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let scoped = HostAuthority::instruction(
        "owner",
        "Create this summary",
        op.clone(),
        [op.creation_id(0)],
        [Capability::EditContent],
    );
    assert!(matches!(
        store.apply(&op, &scoped).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let op = candidate(&mut store, "owner pending note");
    assert!(matches!(
        store.apply(&op, &HostAuthority::owner("owner")).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
