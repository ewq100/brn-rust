use super::*;

fn credential_probe(request: &mut ThreadRunRequest, root: &tempfile::TempDir) -> PathBuf {
    // Auth::open would create this synthetic directory. Its absence proves
    // startup fencing/cancellation precedes even credential-store access.
    let dir = root.path().join("must-not-open-credentials");
    request.credentials_dir = Some(dir.clone());
    let mut store = Store::open(&request.data_dir).unwrap();
    let RecordData::Settings(mut settings) = store
        .records()
        .unwrap()
        .into_iter()
        .find_map(|r| matches!(r.data, RecordData::Settings(_)).then_some(r.data))
        .unwrap()
    else {
        panic!()
    };
    settings.credentials_dir = Some(dir.to_str().unwrap().into());
    let op = store
        .allocate_operation("synthetic credential location")
        .unwrap();
    store
        .configure_workspace(&op, &settings, &HostAuthority::owner("fixture"))
        .unwrap();
    dir
}

#[tokio::test]
async fn prepared_run_id_exists_before_worker_and_precancel_skips_credentials() {
    let (root, mut request, _) = tests::fixture();
    let dir = credential_probe(&mut request, &root);
    let data = request.data_dir.clone();
    let prepared = prepare_run(request).unwrap();
    let id = prepared.run_id().to_owned();
    assert!(
        matches!(Store::open(&data).unwrap().record(&id).unwrap().unwrap().data, RecordData::Run(r) if r.state==RunState::Working)
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = run_prepared(
        prepared,
        cancel,
        Arc::new(|_| panic!("no provider events expected")),
    )
    .await
    .unwrap();
    assert_eq!(result.run_id, id);
    assert!(matches!(result.answer.terminal, AiTerminal::Interrupted));
    assert!(
        !dir.exists(),
        "credential store was opened despite pre-cancel"
    );
    assert!(
        matches!(Store::open(&data).unwrap().record(&id).unwrap().unwrap().data, RecordData::Run(r) if r.state==RunState::Interrupted)
    );
}

#[tokio::test]
async fn stop_before_worker_skips_credentials_stale_tools_and_completion() {
    let (root, mut request, note) = tests::fixture();
    let dir = credential_probe(&mut request, &root);
    let data = request.data_dir.clone();
    let thread = request.thread.clone();
    let prepared = prepare_run(request).unwrap();
    let id = prepared.run_id().to_owned();
    let scope = prepared.scope.clone();
    let mut store = Store::open(&data).unwrap();
    let op = store.allocate_operation("stop during startup").unwrap();
    store
        .stop_and_post(
            &op,
            &id,
            &thread,
            "New direction",
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let before = store.records().unwrap();
    assert!(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "late old tool".into(),
                request: ChangeRequest {
                    reason: "Stale output".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: note,
                        expected_version: Some(1),
                        archived: false,
                        data: RecordData::Note(Note::working("Decision", "Stale output"))
                    }]
                },
            }
        )
        .is_err()
    );
    assert!(
        !finish(
            &scope,
            &AiAnswer {
                text: "Stale final answer".into(),
                terminal: AiTerminal::Completed
            }
        )
        .unwrap()
    );
    let result = run_prepared(
        prepared,
        CancellationToken::new(),
        Arc::new(|_| panic!("no provider events expected")),
    )
    .await
    .unwrap();
    assert_eq!(result.run_id, id);
    assert!(matches!(result.answer.terminal, AiTerminal::Interrupted));
    assert!(
        !dir.exists(),
        "credential store was opened despite durable fence"
    );
    assert_eq!(store.records().unwrap(), before);
}

#[tokio::test]
async fn configuration_change_after_preparation_skips_credentials() {
    let (root, mut request, _) = tests::fixture();
    let dir = credential_probe(&mut request, &root);
    let data = request.data_dir.clone();
    let prepared = prepare_run(request).unwrap();
    let mut store = Store::open(&data).unwrap();
    let op = store.allocate_operation("new owner policy").unwrap();
    store
        .choose_delegation(
            &op,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let result = run_prepared(prepared, CancellationToken::new(), Arc::new(|_| {}))
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Interrupted));
    assert!(!dir.exists());
}

#[test]
fn fresh_queued_preparation_has_no_previous_owner_scope_and_uses_current_input() {
    let (_root, mut old, note) = tests::fixture();
    old.owner_request = Some(OwnerRequest::EditNote {
        id: note,
        version: 1,
    });
    let mut store = Store::open(&old.data_dir).unwrap();
    let thread = old.thread.clone();
    let data = old.data_dir.clone();
    let prepared = prepare_run(old).unwrap();
    assert!(matches!(
        prepared.scope.owner_request,
        Some(OwnerRequest::EditNote { .. })
    ));
    let op = store.allocate_operation("new direction").unwrap();
    store
        .stop_and_post(
            &op,
            prepared.run_id(),
            &thread,
            "Use the new direction",
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let fresh = prepare_run(ThreadRunRequest {
        owner_request: None,
        data_dir: data,
        thread,
        prompt: "Use the new direction".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 4,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
    })
    .unwrap();
    assert_ne!(fresh.run_id(), prepared.run_id());
    assert!(fresh.scope.owner_request.is_none());
    assert!(fresh.input.contains("Use the new direction"));
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(
                |r| matches!(&r.data, RecordData::Message(m) if m.text=="Use the new direction")
            )
            .count(),
        1
    );
    assert!(live(&fresh.scope, &fresh.scope.store.lock().unwrap()).is_ok());
    assert!(live(&prepared.scope, &prepared.scope.store.lock().unwrap()).is_err());
}

#[tokio::test]
async fn archived_destination_after_preparation_skips_credentials() {
    let (root, mut request, _) = tests::fixture();
    let dir = credential_probe(&mut request, &root);
    let data = request.data_dir.clone();
    let thread = request.thread.clone();
    let prepared = prepare_run(request).unwrap();
    let mut store = Store::open(&data).unwrap();
    let record = store.record(&thread).unwrap().unwrap();
    let op = store.allocate_operation("archive destination").unwrap();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Archive synthetic destination".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: thread,
                    expected_version: Some(record.version),
                    archived: true,
                    data: record.data,
                }],
            },
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    store.apply(&op, &HostAuthority::owner("owner")).unwrap();
    let result = run_prepared(prepared, CancellationToken::new(), Arc::new(|_| {}))
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Interrupted));
    assert!(!dir.exists());
}
