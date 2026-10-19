use brn_threads_core::*;
use serde_json::json;

/// Uses only the pre-D4 Store/Settings API: a new API compile failure is not red
/// evidence. Schema8 silently discarded this authority-bearing configuration.
#[test]
fn checked_settings_and_backup_retain_explicit_disabled_mcp_policy() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let policy =
        json!({"endpoint":"http://127.0.0.1:8077/mcp","enabled":false,"tools":[],"access":[]});
    let settings: WorkspaceSettings = serde_json::from_value(json!({
        "mcp":policy,"provider":"Codex","model":"gpt-6.1-sol","effort":"medium",
        "auth_file":null,"credentials_dir":null,"maintenance":false,"review_first":true
    }))
    .unwrap();
    let op = store
        .allocate_operation("synthetic mcp format control")
        .unwrap();
    let id = op.creation_id(0);
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Owner configures disabled local connection".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Settings(settings),
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
    let current = serde_json::to_value(store.record(&id).unwrap().unwrap()).unwrap();
    assert_eq!(
        current["data"]["Settings"]["mcp"], policy,
        "canonical Settings must not silently discard external-data policy"
    );
    let snapshot = root.path().join("snapshot.sqlite3");
    store.backup(&snapshot).unwrap();
    Store::restore(&snapshot, root.path().join("restored")).unwrap();
    let restored = Store::open(root.path().join("restored")).unwrap();
    assert_eq!(
        serde_json::to_value(restored.record(&id).unwrap().unwrap()).unwrap(),
        current
    );
    assert!(restored.receipt(&op).unwrap().is_some());
}
