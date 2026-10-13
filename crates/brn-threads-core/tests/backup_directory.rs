use brn_threads_core::*;
use std::path::Path;

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
fn fixture() -> (
    tempfile::TempDir,
    Store,
    OperationId,
    ChangeRequest,
    Prepared,
) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("workspace")).unwrap();
    let op = store.allocate_operation("legacy").unwrap();
    let body = format!(
        r#"{{"reason":"Legacy settings","writes":[{{"id":"{}","expected_version":null,"archived":false,"data":{{"Settings":{{"provider":"offline","model":"synthetic","effort":"medium","auth_file":null,"credentials_dir":null,"maintenance":false,"review_first":false}}}}}}],"inputs":[]}}"#,
        op.creation_id(0)
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
    store.apply(&op, &HostAuthority::owner("fixture")).unwrap();
    (root, store, op, request, prepared)
}
fn choose(store: &mut Store, key: &str, directory: &Path) -> Result<ApplyOutcome> {
    let op = store.allocate_operation(key)?;
    store.choose_backup_directory(&op, directory, &HostAuthority::owner("fixture"))
}

#[test]
fn folder_patch_preserves_current_settings_fences_runs_and_replays_raw_intent_without_filesystem() {
    let (root, mut store, legacy, request, prepared) = fixture();
    let folder = root.path().join("backups");
    std::fs::create_dir(&folder).unwrap();
    let selection = store
        .allocate_operation("selection allocated before config")
        .unwrap();
    let mut other = Store::open(store.directory()).unwrap();
    let mut selected = settings(&other);
    selected.provider = "other-offline".into();
    selected.model = "other-model".into();
    selected.effort = "high".into();
    selected.auth_file = Some("/synthetic/empty-auth".into());
    selected.credentials_dir = Some("/synthetic/empty-credentials".into());
    let config = other.allocate_operation("config").unwrap();
    other
        .configure_workspace(&config, &selected, &HostAuthority::owner("fixture"))
        .unwrap();
    let delegation = other.allocate_operation("delegation").unwrap();
    other
        .choose_delegation(
            &delegation,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    selected = settings(&other);
    let run_op = other.allocate_operation("run").unwrap();
    let run_id = run_op.creation_id(0);
    let run_request = ChangeRequest {
        reason: "fixture run".into(),
        inputs: vec![],
        writes: vec![Put {
            id: run_id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Run(Run {
                instruction: None,
                queue_claim: None,
                thread: "synthetic-thread".into(),
                provider: selected.provider.clone(),
                model: selected.model.clone(),
                guide_identity: "fixture".into(),
                budget: 1,
                effort: selected.effort.clone(),
                loaded_skills: vec![],
                fence: 0,
                state: RunState::Working,
                progress: "fixture".into(),
                operations: vec![],
                sources: vec![],
            }),
        }],
    };
    // A real thread is required by core validation.
    let thread_op = other.allocate_operation("thread").unwrap();
    let thread_id = thread_op.creation_id(0);
    let thread_request = ChangeRequest {
        reason: "fixture thread".into(),
        inputs: vec![],
        writes: vec![Put {
            id: thread_id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Thread(Thread {
                title: "fixture".into(),
                state: ThreadState::Open,
                attention: vec![],
            }),
        }],
    };
    other
        .prepare_owner(
            &thread_op,
            &thread_request,
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    other
        .apply(&thread_op, &HostAuthority::owner("fixture"))
        .unwrap();
    let mut run_request = run_request;
    if let RecordData::Run(run) = &mut run_request.writes[0].data {
        run.thread = thread_id;
    }
    other
        .prepare_owner(&run_op, &run_request, &HostAuthority::owner("fixture"))
        .unwrap();
    other
        .apply(&run_op, &HostAuthority::owner("fixture"))
        .unwrap();
    let raw = folder.join(".");
    let receipt = store
        .choose_backup_directory(&selection, &raw, &HostAuthority::owner("fixture"))
        .unwrap();
    selected.backup_directory = Some(folder.to_str().unwrap().into());
    assert_eq!(settings(&store), selected);
    let RecordData::Run(run) = store.record(&run_id).unwrap().unwrap().data else {
        panic!("run")
    };
    assert_eq!(run.state, RunState::Cancelled);
    assert_eq!(run.fence, 1);
    assert!(matches!(
        store.choose_backup_directory(&selection, &folder, &HostAuthority::owner("retry")),
        Err(Error::ImmutableRequest)
    ));
    std::fs::remove_dir(&folder).unwrap();
    drop(other);
    drop(store);
    let mut store = Store::open(root.path().join("workspace")).unwrap();
    assert_eq!(
        store
            .choose_backup_directory(&selection, &raw, &HostAuthority::owner("retry"))
            .unwrap(),
        receipt
    );
    assert_eq!(
        store
            .prepare_owner(&legacy, &request, &HostAuthority::owner("retry"))
            .unwrap(),
        prepared
    );
    assert!(matches!(
        store
            .apply(&legacy, &HostAuthority::owner("retry"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(settings(&store), selected);
}

#[test]
fn stale_configure_and_its_replay_preserve_saved_folder() {
    let (root, mut store, _, _, _) = fixture();
    let stale = settings(&store);
    let folder = root.path().join("backups");
    std::fs::create_dir(&folder).unwrap();
    choose(&mut store, "first folder", &folder).unwrap();
    let config = store.allocate_operation("stale configure").unwrap();
    let receipt = store
        .configure_workspace(&config, &stale, &HostAuthority::owner("owner"))
        .unwrap();
    assert_eq!(
        settings(&store).backup_directory.as_deref(),
        folder.to_str()
    );
    let other = root.path().join("other");
    std::fs::create_dir(&other).unwrap();
    choose(&mut store, "other folder", &other).unwrap();
    assert_eq!(
        store
            .configure_workspace(&config, &stale, &HostAuthority::owner("retry"))
            .unwrap(),
        receipt
    );
    assert_eq!(settings(&store).backup_directory.as_deref(), other.to_str());
}

#[test]
fn selection_requires_owner_and_existing_absolute_external_directory_including_aliases() {
    let (root, mut store, _, _, _) = fixture();
    let folder = root.path().join("backups");
    std::fs::create_dir(&folder).unwrap();
    let op = store.allocate_operation("agent attempts folder").unwrap();
    assert!(
        store
            .choose_backup_directory(&op, &folder, &HostAuthority::maintenance("agent"))
            .is_err()
    );
    for (key, bad) in [
        ("relative", "relative".into()),
        ("missing", root.path().join("missing")),
        ("workspace", store.directory().to_owned()),
        ("child", store.directory().join("child")),
    ] {
        if key == "child" {
            std::fs::create_dir(&bad).unwrap();
        }
        assert!(choose(&mut store, key, &bad).is_err());
    }
    let file = root.path().join("file");
    std::fs::write(&file, "fixture").unwrap();
    assert!(choose(&mut store, "file", &file).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::{ffi::OsStringExt, fs::symlink};
        let alias = root.path().join("alias");
        symlink(store.directory(), &alias).unwrap();
        assert!(choose(&mut store, "alias", &alias).is_err());
        let external = root.path().join("external-alias");
        symlink(&folder, &external).unwrap();
        choose(&mut store, "external alias", &external).unwrap();
        assert_eq!(
            settings(&store).backup_directory.as_deref(),
            folder.to_str()
        );
        let invalid = root.path().join(std::ffi::OsString::from_vec(vec![0xff]));
        assert!(choose(&mut store, "non utf8", &invalid).is_err());
    }
}

#[test]
fn explicit_backup_refuses_workspace_and_alias_and_preserves_reopen_while_parent_is_allowed() {
    let (root, store, _, _, _) = fixture();
    let records = store.records().unwrap();
    let inside = store.directory().join("backup.sqlite3");
    assert!(matches!(
        store.backup(&inside),
        Err(Error::UnsafeDirectory(_))
    ));
    assert!(!inside.exists());
    #[cfg(unix)]
    {
        let alias = root.path().join("workspace-alias");
        std::os::unix::fs::symlink(store.directory(), &alias).unwrap();
        assert!(store.backup(alias.join("backup.sqlite3")).is_err());
    }
    let outside = root.path().join("backup.sqlite3");
    store.backup(&outside).unwrap();
    assert!(outside.is_file());
    let data = store.directory().to_owned();
    drop(store);
    assert_eq!(Store::open(data).unwrap().records().unwrap(), records);
}

#[test]
fn backup_selection_cannot_replay_a_delegation_receipt_with_matching_raw_string() {
    let (_, mut store, _, _, _) = fixture();
    let operation = store.allocate_operation("owner choice").unwrap();
    store
        .choose_delegation(
            &operation,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    let settings_before = settings(&store);
    assert!(matches!(
        store.choose_backup_directory(
            &operation,
            Path::new("ReviewFirst"),
            &HostAuthority::owner("retry")
        ),
        Err(Error::ImmutableRequest)
    ));
    assert_eq!(settings(&store), settings_before);
}
