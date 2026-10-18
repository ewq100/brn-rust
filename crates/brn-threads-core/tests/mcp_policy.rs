use brn_threads_core::*;
use serde_json::json;
fn checked(store: &mut Store, data: RecordData) -> Record {
    let op = store
        .allocate_operation(&uuid::Uuid::new_v4().to_string())
        .unwrap();
    let id = op.creation_id(0);
    let request = ChangeRequest {
        reason: "synthetic owner fixture".into(),
        writes: vec![Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data,
        }],
        inputs: vec![],
    };
    store
        .prepare_owner(&op, &request, &HostAuthority::owner("fixture"))
        .unwrap();
    assert!(matches!(
        store.apply(&op, &HostAuthority::owner("fixture")).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    store.record(&id).unwrap().unwrap()
}
fn replace(store: &mut Store, record: &Record, data: RecordData) {
    let op = store
        .allocate_operation(&uuid::Uuid::new_v4().to_string())
        .unwrap();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "synthetic owner update".into(),
                writes: vec![Put {
                    id: record.id.clone(),
                    expected_version: Some(record.version),
                    archived: record.archived,
                    data,
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &HostAuthority::owner("fixture")).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
fn settings(store: &Store) -> WorkspaceSettings {
    store
        .records()
        .unwrap()
        .into_iter()
        .find_map(|r| {
            if let RecordData::Settings(s) = r.data {
                Some(s)
            } else {
                None
            }
        })
        .unwrap()
}
fn initial(store: &mut Store) {
    checked(store,RecordData::Settings(serde_json::from_value(json!({"provider":"Codex","model":"gpt-6.1-sol","effort":"medium","auth_file":null,"credentials_dir":null,"maintenance":true,"review_first":false,"delegation_chosen":true})).unwrap()));
}
fn policy(access: Vec<McpAccess>) -> McpPolicy {
    McpPolicy {
        endpoint: "http://127.0.0.1:8077/mcp".into(),
        server_digest: "a".repeat(64),
        enabled: true,
        tools: vec![McpApprovedTool {
            name: "public_read".into(),
            metadata_digest: "b".repeat(64),
            schema_digest: mcp_input_schema_digest(McpInputContract::Query),
            contract: McpInputContract::Query,
        }],
        access,
    }
}
fn set(store: &mut Store, p: &McpPolicy) {
    let op = store
        .allocate_operation(&uuid::Uuid::new_v4().to_string())
        .unwrap();
    assert!(matches!(
        store
            .change_mcp_policy(&op, Some(p), &HostAuthority::owner("owner"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
fn chat(store: &mut Store, private: bool) -> String {
    checked(
        store,
        RecordData::Thread(Thread {
            private_context: private,
            context_epoch: 0,
            title: "synthetic chat".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    )
    .id
}
fn run(store: &mut Store, chat: &str) -> String {
    let message = checked(
        store,
        RecordData::Message(Message {
            thread: chat.into(),
            role: "owner".into(),
            text: "Read the explicitly reviewed external input".into(),
        }),
    );
    let s = settings(store);
    checked(
        store,
        RecordData::Run(Run {
            instruction: Some(message.id),
            queue_claim: None,
            thread: chat.into(),
            provider: s.provider,
            model: s.model,
            effort: s.effort,
            guide_identity: "frozen synthetic guide".into(),
            budget: 8,
            loaded_skills: vec![],
            loaded_skill_details: vec![],
            fence: 1,
            state: RunState::Working,
            progress: String::new(),
            operations: vec![],
            sources: vec![],
        }),
    )
    .id
}
#[test]
fn policy_is_explicit_replay_bound_preserves_configure_and_fences_current_runs() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    assert!(settings(&store).mcp.is_none());
    let c = chat(&mut store, false);
    let r = run(&mut store, &c);
    let stale = settings(&store);
    let p = policy(vec![McpAccess::General]);
    let op = store
        .allocate_operation("approve known local service")
        .unwrap();
    let outcome = store
        .change_mcp_policy(&op, Some(&p), &HostAuthority::owner("owner"))
        .unwrap();
    let RecordData::Run(ended) = store.record(&r).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!((ended.state, ended.fence), (RunState::Interrupted, 2));
    assert_eq!(
        store
            .change_mcp_policy(&op, Some(&p), &HostAuthority::owner("retry"))
            .unwrap(),
        outcome
    );
    let mut different = p.clone();
    different.endpoint = "http://127.0.0.1:8078/mcp".into();
    assert!(matches!(
        store.change_mcp_policy(&op, Some(&different), &HostAuthority::owner("retry")),
        Err(Error::ImmutableRequest)
    ));
    let config = store
        .allocate_operation("unrelated provider setting from stale form")
        .unwrap();
    let mut next = stale;
    next.model = "other".into();
    store
        .configure_workspace(&config, &next, &HostAuthority::owner("owner"))
        .unwrap();
    assert_eq!(settings(&store).mcp, Some(p.clone()));
    store
        .configure_workspace(&config, &next, &HostAuthority::owner("retry"))
        .unwrap();
    assert_eq!(settings(&store).mcp, Some(p.clone()));
    let denied = store
        .allocate_operation("no model policy authority")
        .unwrap();
    assert!(
        store
            .change_mcp_policy(&denied, None, &HostAuthority::maintenance("model"))
            .is_err()
    );
    assert_eq!(settings(&store).mcp, Some(p));
}
#[test]
fn exact_reviewed_input_scope_instruction_and_run_fence_are_checked_fresh() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    let p = policy(vec![McpAccess::General]);
    set(&mut store, &p);
    let c = chat(&mut store, false);
    let r = run(&mut store, &c);
    let input = json!({"query":"EXTERNAL_INPUT_CANARY_õ"});
    assert!(
        store
            .grant_external_data(
                &r,
                "public_read",
                &input,
                &HostAuthority::runtime("model", &r)
            )
            .is_err()
    );
    assert!(
        store
            .grant_external_data(
                &r,
                "public_read",
                &json!({"query":"x","credential":"never"}),
                &HostAuthority::owner("owner")
            )
            .is_err()
    );
    let grant = store
        .grant_external_data(&r, "public_read", &input, &HostAuthority::owner("owner"))
        .unwrap();
    let valid = |s: &Store| {
        s.validate_external_dispatch(
            &grant,
            &p.endpoint,
            "public_read",
            &p.tools[0].metadata_digest,
            &input,
        )
    };
    valid(&store).unwrap();
    assert!(
        store
            .validate_external_dispatch(
                &grant,
                "http://127.0.0.1:8078/mcp",
                "public_read",
                &p.tools[0].metadata_digest,
                &input
            )
            .is_err()
    );
    assert!(
        store
            .validate_external_dispatch(
                &grant,
                &p.endpoint,
                "new_tool",
                &p.tools[0].metadata_digest,
                &input
            )
            .is_err()
    );
    assert!(
        store
            .validate_external_dispatch(&grant, &p.endpoint, "public_read", &"c".repeat(64), &input)
            .is_err()
    );
    assert!(
        store
            .validate_external_dispatch(
                &grant,
                &p.endpoint,
                "public_read",
                &p.tools[0].metadata_digest,
                &json!({"query":"different private input"})
            )
            .is_err()
    );
    // Progress alone does not silently invalidate or refresh the exact fence.
    let before = store.record(&r).unwrap().unwrap();
    let RecordData::Run(mut updated) = before.data.clone() else {
        panic!()
    };
    updated.progress = "reported progress".into();
    replace(&mut store, &before, RecordData::Run(updated));
    valid(&store).unwrap();
    // A legitimate lazy skill load changes this control fence: old grant dies.
    let before = store.record(&r).unwrap().unwrap();
    let RecordData::Run(mut updated) = before.data.clone() else {
        panic!()
    };
    updated.loaded_skills.push("maintain-notes".into());
    updated.fence += 1;
    replace(&mut store, &before, RecordData::Run(updated));
    assert!(valid(&store).is_err());
    let fresh = store
        .grant_external_data(
            &r,
            "public_read",
            &input,
            &HostAuthority::owner("fresh explicit review"),
        )
        .unwrap();
    store.validate_external_channel(&fresh).unwrap();
    let rr = store.record(&r).unwrap().unwrap();
    let RecordData::Run(run) = rr.data else {
        panic!()
    };
    let instruction = store
        .record(run.instruction.as_ref().unwrap())
        .unwrap()
        .unwrap();
    let RecordData::Message(mut m) = instruction.data.clone() else {
        panic!()
    };
    m.text = "Changed owner instruction".into();
    replace(&mut store, &instruction, RecordData::Message(m));
    assert!(store.validate_external_channel(&fresh).is_err());
    let backup = root.path().join("copy.sqlite3");
    store.backup(&backup).unwrap();
    assert!(
        !std::fs::read(&backup)
            .unwrap()
            .windows("EXTERNAL_INPUT_CANARY".len())
            .any(|w| w == b"EXTERNAL_INPUT_CANARY")
    );
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(settings(&restored).mcp, Some(p));
    assert!(restored.validate_external_channel(&fresh).is_err());
}
#[test]
fn private_and_project_access_are_explicit_and_raw_owner_policy_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    set(&mut store, &policy(vec![McpAccess::General]));
    let private = chat(&mut store, true);
    let r = run(&mut store, &private);
    assert!(
        store
            .grant_external_data(
                &r,
                "public_read",
                &json!({"query":"x"}),
                &HostAuthority::owner("owner")
            )
            .is_err()
    );
    set(&mut store, &policy(vec![McpAccess::Chat(private.clone())]));
    let r = run(&mut store, &private);
    let grant = store
        .grant_external_data(
            &r,
            "public_read",
            &json!({"query":"x"}),
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    store.validate_external_channel(&grant).unwrap();
    let thread = store.record(&private).unwrap().unwrap();
    let RecordData::Thread(mut t) = thread.data.clone() else {
        panic!()
    };
    t.context_epoch += 1;
    replace(&mut store, &thread, RecordData::Thread(t));
    assert!(store.validate_external_channel(&grant).is_err());
    let baseline = settings(&store);
    let settings_record = store
        .records()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.data, RecordData::Settings(_)))
        .unwrap();
    for bad in [
        policy(vec![McpAccess::Project("missing".into())]),
        policy(vec![McpAccess::General, McpAccess::General]),
        {
            let mut p = policy(vec![McpAccess::General]);
            p.endpoint = "http://127.0.0.1:8077/mcp?secret=x".into();
            p
        },
        {
            let mut p = policy(vec![McpAccess::General]);
            p.tools[0].schema_digest = "a".repeat(64);
            p
        },
    ] {
        let mut next = baseline.clone();
        next.mcp = Some(bad);
        let op = store
            .allocate_operation(&uuid::Uuid::new_v4().to_string())
            .unwrap();
        let request = ChangeRequest {
            reason: "raw owner validation cannot bypass MCP limits".into(),
            writes: vec![Put {
                id: settings_record.id.clone(),
                expected_version: Some(settings_record.version),
                archived: false,
                data: RecordData::Settings(next),
            }],
            inputs: vec![],
        };
        if store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))
            .is_ok()
        {
            assert!(store.apply(&op, &HostAuthority::owner("owner")).is_err());
        }
        assert_eq!(settings(&store), baseline);
    }
}

#[test]
fn still_valid_grant_cannot_cross_store_reopen_or_unchanged_backup_restore() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    initial(&mut store);
    let p = policy(vec![McpAccess::General]);
    set(&mut store, &p);
    let c = chat(&mut store, false);
    let r = run(&mut store, &c);
    let input = json!({"query":"EPHEMERAL_GRANT_INPUT_CANARY"});
    let grant = store
        .grant_external_data(
            &r,
            "public_read",
            &input,
            &HostAuthority::owner("exact owner review"),
        )
        .unwrap();
    let checks = |s: &Store| {
        [
            s.validate_external_channel(&grant).is_ok(),
            s.validate_external_dispatch(
                &grant,
                &p.endpoint,
                "public_read",
                &p.tools[0].metadata_digest,
                &input,
            )
            .is_ok(),
        ]
    };
    assert_eq!(checks(&store), [true, true]);
    let before = store.record(&r).unwrap().unwrap();
    let RecordData::Run(mut updated) = before.data.clone() else {
        panic!()
    };
    updated.progress = "same invocation non-control progress".into();
    replace(&mut store, &before, RecordData::Run(updated));
    assert_eq!(checks(&store), [true, true]);
    let backup = root.path().join("unchanged.sqlite3");
    store.backup(&backup).unwrap();
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(restored.record(&r).unwrap(), store.record(&r).unwrap());
    assert_eq!(settings(&restored).mcp, Some(p.clone()));
    let second = Store::open(&data).unwrap();
    let same_directory = checks(&second);
    let restored_context = checks(&restored);
    assert_eq!(
        checks(&store),
        [true, true],
        "issuing context is still valid; unrelated refusal is not epoch proof"
    );
    drop(second);
    drop(store);
    let reopened = Store::open(&data).unwrap();
    let after_drop = checks(&reopened);
    assert_eq!(
        (same_directory, restored_context, after_drop),
        ([false, false], [false, false], [false, false]),
        "channel/dispatch must refuse a held grant on a different Store even with unchanged canonical context"
    );
}
