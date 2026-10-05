use brn_store::{
    Error, WorkStore,
    work::{actions::*, proposals::ProposalStamp},
};
use rusqlite::{Connection, params};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn raw(path: &std::path::Path) -> Connection {
    Connection::open(path.join("brn.sqlite")).unwrap()
}

fn record() -> ActionRecord {
    let data = ActionData {
        title: "  Tähtaeg λ\r\n".into(),
        description: "\u{feff}Exact English and Eesti description. „Tähtaeg on reede.“\r\n".into(),
        state: ActionState::Open,
        owner: Some("  Anna Õun  ".into()),
        related_person: Some(Uuid::new_v4()),
        related_project: Some(Uuid::new_v4()),
        sources: vec![Uuid::new_v4(), Uuid::new_v4()],
        thread: Some(Uuid::new_v4()),
        due_on: Some("2024-02-29".into()),
        follow_up_on: Some("2026-10-04".into()),
        dependencies: vec![Uuid::new_v4()],
        parent: Some(Uuid::new_v4()),
        follows_up: Some(Uuid::new_v4()),
        priority: Some(ActionPriority::High),
    };
    ActionRecord {
        origin: ActionOrigin {
            id: Uuid::new_v4(),
            proposal: ProposalStamp {
                id: Uuid::new_v4(),
                version: 37,
            },
            data: data.clone(),
            created_at_ms: 1000,
        },
        version: 1,
        data,
        updated_at_ms: 1000,
        waiting_since_ms: None,
        completed_at_ms: None,
    }
}

fn state_name(state: ActionState) -> &'static str {
    match state {
        ActionState::Open => "open",
        ActionState::Waiting => "waiting",
        ActionState::Blocked => "blocked",
        ActionState::Completed => "completed",
    }
}

// Synthetic private SQL fixtures do not add a production Action producer.
fn insert(conn: &Connection, record: &ActionRecord) {
    let bytes = serde_json::to_vec(record).unwrap();
    let origin = serde_json::to_vec(&record.origin).unwrap();
    conn.execute("INSERT INTO actions(id,version,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![record.origin.id.to_string(),record.version as i64,state_name(record.data.state),record.origin.created_at_ms as i64,digest(&origin).as_slice(),bytes,digest(&bytes).as_slice()]).unwrap();
}

fn replace_json(conn: &Connection, id: Uuid, bytes: &[u8]) {
    conn.execute(
        "UPDATE actions SET record_json=?2,record_sha256=?3 WHERE id=?1",
        params![id.to_string(), bytes, digest(bytes).as_slice()],
    )
    .unwrap();
}

fn backups(path: &std::path::Path) -> Vec<String> {
    let mut names = std::fs::read_dir(path.join("backups"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn assert_semantic_startup_refusal(path: &std::path::Path) {
    let before = backups(path);
    assert!(WorkStore::open(path).is_err());
    assert_eq!(
        backups(path),
        before,
        "refusal must precede backup creation/pruning"
    );
    assert!(
        !std::fs::read_dir(path).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("corrupt")),
        "semantic refusal must not restore earlier physical work"
    );
}

#[test]
fn fresh_database_has_checked_empty_action_storage() {
    let data = fixture();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    let page = store.action_list(&ActionListRequest::default()).unwrap();
    assert!(page.entries.is_empty());
    assert_eq!(page.next_before, None);
    let conn = Connection::open(data.path().join("brn.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        14
    );
}

#[test]
fn startup_refuses_even_empty_action_schema_damage_before_creating_backup() {
    let data = fixture();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    drop(store);
    let conn = Connection::open(data.path().join("brn.sqlite")).unwrap();
    conn.execute_batch("DROP TABLE IF EXISTS actions; CREATE TABLE actions(id TEXT PRIMARY KEY);")
        .unwrap();
    drop(conn);
    let before = std::fs::read_dir(data.path().join("backups"))
        .unwrap()
        .count();
    assert!(
        WorkStore::open(data.path()).is_err(),
        "malformed Action schema must refuse startup"
    );
    assert_eq!(
        std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn exact_origin_current_bytes_optional_priority_and_all_states_survive_reopen() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    let mut records = Vec::new();
    for state in [
        ActionState::Open,
        ActionState::Waiting,
        ActionState::Blocked,
        ActionState::Completed,
    ] {
        let mut r = record();
        r.version = 2;
        r.updated_at_ms = 2000;
        r.data.state = state;
        r.data
            .description
            .push_str("Current body differs; origin is exact.\r\n");
        r.data.priority = None;
        r.waiting_since_ms = (state == ActionState::Waiting).then_some(1250);
        r.completed_at_ms = (state == ActionState::Completed).then_some(1750);
        r.validate().unwrap();
        insert(&conn, &r);
        assert_eq!(store.action(r.origin.id).unwrap(), Some(r.clone()));
        assert_eq!(
            r.stamp(),
            ActionStamp {
                id: r.origin.id,
                version: 2
            }
        );
        records.push(r);
    }
    drop(conn);
    drop(store);
    let (store, _) = WorkStore::open(path.path()).unwrap();
    for r in records {
        assert_eq!(store.action(r.origin.id).unwrap(), Some(r));
    }
    assert_eq!(store.action(Uuid::new_v4()).unwrap(), None);
    assert!(matches!(store.action(Uuid::nil()), Err(Error::Invalid(_))));
    assert_eq!(ActionListRequest::default().state, None);
    assert_eq!(ActionListRequest::default().limit, 25);
}

#[test]
fn canonical_civil_dates_validate_both_fields_without_changing_bytes() {
    let r = record();
    for date in [
        "0001-01-01",
        "9999-12-31",
        "2000-02-29",
        "2024-02-29",
        "1900-02-28",
        "2026-04-30",
    ] {
        let mut data = r.data.clone();
        data.due_on = Some(date.into());
        data.follow_up_on = Some(date.into());
        data.validate(r.origin.id).unwrap();
        assert_eq!(data.due_on.as_deref(), Some(date));
        assert_eq!(data.follow_up_on.as_deref(), Some(date));
    }
    for date in [
        "0000-01-01",
        "1900-02-29",
        "2100-02-29",
        "2026-02-29",
        "2026-04-31",
        "2026-00-01",
        "2026-13-01",
        "2026-01-00",
        "2026-01-32",
        "2026-1-01",
        " 2026-01-01",
        "2026-01-01\n",
        "２０２６-01-01",
        "",
    ] {
        for follow_up in [false, true] {
            let mut data = r.data.clone();
            if follow_up {
                data.follow_up_on = Some(date.into());
            } else {
                data.due_on = Some(date.into());
            }
            assert!(data.validate(r.origin.id).is_err(), "{date:?}");
        }
    }
}

#[test]
fn byte_uuid_and_self_reference_limits_are_shape_only_and_exact() {
    let r = record();
    let mut data = r.data.clone();
    data.title = "õ".repeat(256);
    data.owner = Some("õ".repeat(256));
    data.description = "õ".repeat(32 * 1024);
    data.sources = (1..=64).map(Uuid::from_u128).collect();
    data.dependencies = (65..=128).map(Uuid::from_u128).collect();
    data.validate(r.origin.id).unwrap();
    // Existence and cycles among other records belong to approval, not reads.
    data.parent = Some(Uuid::from_u128(900));
    data.follows_up = data.parent;
    data.related_person = data.parent;
    data.validate(r.origin.id).unwrap();
    let valid = data.clone();
    data.title.push('x');
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.owner.as_mut().unwrap().push('x');
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.description.push('x');
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.sources.push(Uuid::new_v4());
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.dependencies.push(Uuid::new_v4());
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.sources[1] = data.sources[0];
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.dependencies[1] = data.dependencies[0];
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.sources[0] = Uuid::nil();
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.dependencies[0] = Uuid::nil();
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.dependencies[0] = r.origin.id;
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.parent = Some(r.origin.id);
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.follows_up = Some(r.origin.id);
    assert!(data.validate(r.origin.id).is_err());
    for field in 0..5 {
        data = valid.clone();
        match field {
            0 => data.related_person = Some(Uuid::nil()),
            1 => data.related_project = Some(Uuid::nil()),
            2 => data.thread = Some(Uuid::nil()),
            3 => data.parent = Some(Uuid::nil()),
            _ => data.follows_up = Some(Uuid::nil()),
        }
        assert!(data.validate(r.origin.id).is_err());
    }
    data = valid.clone();
    data.title = " \r\n".into();
    assert!(data.validate(r.origin.id).is_err());
    data = valid.clone();
    data.owner = Some(" \r\n".into());
    assert!(data.validate(r.origin.id).is_err());
    data = valid;
    data.owner = None;
    data.description.clear();
    data.priority = None;
    data.validate(r.origin.id).unwrap();
}

fn reject_change(mut r: ActionRecord, change: impl FnOnce(&mut ActionRecord)) {
    change(&mut r);
    assert!(r.validate().is_err(), "unexpectedly valid: {r:?}");
}

#[test]
fn record_origin_revision_waiting_completion_and_clock_guards_are_checked() {
    let r = record();
    r.validate().unwrap();
    reject_change(r.clone(), |r| r.origin.id = Uuid::nil());
    reject_change(r.clone(), |r| r.origin.proposal.id = Uuid::nil());
    reject_change(r.clone(), |r| r.origin.proposal.version = 0);
    reject_change(r.clone(), |r| {
        r.origin.proposal.version = i64::MAX as u64 + 1
    });
    reject_change(r.clone(), |r| r.origin.data.state = ActionState::Completed);
    reject_change(r.clone(), |r| r.version = 0);
    reject_change(r.clone(), |r| r.version = i64::MAX as u64 + 1);
    reject_change(r.clone(), |r| r.updated_at_ms = i64::MAX as u64 + 1);
    reject_change(r.clone(), |r| r.origin.created_at_ms = i64::MAX as u64 + 1);
    reject_change(r.clone(), |r| r.updated_at_ms = 999);
    reject_change(r.clone(), |r| r.data.title.push('x'));
    reject_change(r.clone(), |r| r.updated_at_ms += 1);
    reject_change(r.clone(), |r| r.waiting_since_ms = Some(1000));
    reject_change(r.clone(), |r| r.completed_at_ms = Some(1000));
    let mut waiting = r.clone();
    waiting.origin.data.state = ActionState::Waiting;
    waiting.data.state = ActionState::Waiting;
    waiting.waiting_since_ms = Some(1000);
    waiting.validate().unwrap();
    reject_change(waiting.clone(), |r| r.waiting_since_ms = None);
    reject_change(waiting.clone(), |r| r.waiting_since_ms = Some(999));
    reject_change(waiting.clone(), |r| r.waiting_since_ms = Some(1001));
    waiting.version = 2;
    waiting.updated_at_ms = 2000;
    waiting.waiting_since_ms = Some(1500);
    waiting.validate().unwrap();
    reject_change(waiting, |r| r.waiting_since_ms = Some(2001));
    let mut completed = r;
    completed.version = 2;
    completed.updated_at_ms = 2000;
    completed.data.state = ActionState::Completed;
    completed.completed_at_ms = Some(1000);
    completed.validate().unwrap();
    reject_change(completed.clone(), |r| r.completed_at_ms = None);
    reject_change(completed.clone(), |r| r.completed_at_ms = Some(999));
    reject_change(completed.clone(), |r| r.completed_at_ms = Some(2001));
    completed.version = i64::MAX as u64;
    completed.updated_at_ms = i64::MAX as u64;
    completed.completed_at_ms = Some(i64::MAX as u64);
    completed.validate().unwrap();
}

#[test]
fn page_filters_and_created_time_id_cursors_survive_state_changes_and_missing_cursor_rows() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    let mut records = Vec::new();
    for (id, time, state) in [
        (1, 1000, ActionState::Waiting),
        (2, 1000, ActionState::Open),
        (3, 1000, ActionState::Blocked),
        (4, 1000, ActionState::Open),
        (5, 2000, ActionState::Open),
    ] {
        let mut r = record();
        r.origin.id = Uuid::from_u128(id);
        r.origin.created_at_ms = time;
        r.updated_at_ms = time;
        r.origin.data.state = state;
        r.data.state = state;
        r.waiting_since_ms = (state == ActionState::Waiting).then_some(time);
        r.validate().unwrap();
        insert(&conn, &r);
        records.push(r);
    }
    let request = ActionListRequest {
        limit: 2,
        ..Default::default()
    };
    let first = store.action_list(&request).unwrap();
    assert_eq!(first.entries, [records[4].clone(), records[3].clone()]);
    let cursor = first.next_before.unwrap();
    assert_eq!(
        cursor,
        ActionCursor {
            created_at_ms: 1000,
            id: Uuid::from_u128(4)
        }
    );
    let mut changed = records[3].clone();
    changed.version = 2;
    changed.data.state = ActionState::Completed;
    changed.updated_at_ms = 3000;
    changed.completed_at_ms = Some(3000);
    changed.validate().unwrap();
    let bytes = serde_json::to_vec(&changed).unwrap();
    conn.execute("UPDATE actions SET version=2,state='completed',record_json=?2,record_sha256=?3 WHERE id=?1",params![changed.origin.id.to_string(),bytes,digest(&bytes).as_slice()]).unwrap();
    let second = store
        .action_list(&ActionListRequest {
            before: Some(cursor),
            ..request.clone()
        })
        .unwrap();
    assert_eq!(second.entries, [records[2].clone(), records[1].clone()]);
    assert_eq!(
        second.next_before,
        Some(ActionCursor {
            created_at_ms: 1000,
            id: Uuid::from_u128(2)
        })
    );
    conn.execute("DELETE FROM actions WHERE id=?1", [cursor.id.to_string()])
        .unwrap();
    assert_eq!(
        store
            .action_list(&ActionListRequest {
                before: Some(cursor),
                ..request.clone()
            })
            .unwrap(),
        second
    );
    let last = store
        .action_list(&ActionListRequest {
            before: second.next_before,
            ..request
        })
        .unwrap();
    assert_eq!(last.entries, [records[0].clone()]);
    assert_eq!(last.next_before, None);
    let open = store
        .action_list(&ActionListRequest {
            state: Some(ActionState::Open),
            limit: 1,
            before: None,
        })
        .unwrap();
    assert_eq!(open.entries, [records[4].clone()]);
    let remaining = store
        .action_list(&ActionListRequest {
            state: Some(ActionState::Open),
            limit: 200,
            before: open.next_before,
        })
        .unwrap();
    assert_eq!(remaining.entries, [records[1].clone()]);
    assert_eq!(remaining.next_before, None);
    for state in [
        ActionState::Waiting,
        ActionState::Blocked,
        ActionState::Completed,
    ] {
        let page = store
            .action_list(&ActionListRequest {
                state: Some(state),
                ..Default::default()
            })
            .unwrap();
        assert!(page.entries.iter().all(|r| r.data.state == state));
        assert_eq!(
            page.entries.len(),
            usize::from(state != ActionState::Completed)
        );
    }
}

#[test]
fn list_admission_bounds_are_checked_before_sql_and_maximum_page_is_exact() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    for id in 1..=201 {
        let mut r = record();
        r.origin.id = Uuid::from_u128(id);
        insert(&conn, &r);
    }
    let default = store.action_list(&Default::default()).unwrap();
    assert_eq!(default.entries.len(), 25);
    let max = store
        .action_list(&ActionListRequest {
            limit: 200,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(max.entries.len(), 200);
    assert_eq!(max.entries[0].origin.id, Uuid::from_u128(201));
    let last = store
        .action_list(&ActionListRequest {
            limit: 200,
            before: max.next_before,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(last.entries.len(), 1);
    assert_eq!(last.entries[0].origin.id, Uuid::from_u128(1));
    assert_eq!(last.next_before, None);
    for limit in [0, 201, usize::MAX] {
        assert!(matches!(
            store.action_list(&ActionListRequest {
                limit,
                ..Default::default()
            }),
            Err(Error::Invalid(_))
        ));
    }
    for cursor in [
        ActionCursor {
            created_at_ms: 1000,
            id: Uuid::nil(),
        },
        ActionCursor {
            created_at_ms: i64::MAX as u64 + 1,
            id: Uuid::new_v4(),
        },
    ] {
        assert!(matches!(
            store.action_list(&ActionListRequest {
                before: Some(cursor),
                ..Default::default()
            }),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn maximum_escaped_text_and_uuid_collections_fit_without_clipping() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    let mut r = record();
    r.data.title = "\0".repeat(512);
    r.data.owner = Some("\0".repeat(512));
    r.data.description = "\0".repeat(64 * 1024);
    r.data.sources = (1..=64).map(Uuid::from_u128).collect();
    r.data.dependencies = (65..=128).map(Uuid::from_u128).collect();
    r.origin.data = r.data.clone();
    r.validate().unwrap();
    insert(&conn, &r);
    assert_eq!(store.action(r.origin.id).unwrap(), Some(r.clone()));
    assert_eq!(
        store.action_list(&Default::default()).unwrap().entries,
        [r.clone()]
    );
    drop(conn);
    drop(store);
    assert_eq!(
        WorkStore::open(path.path())
            .unwrap()
            .0
            .action(r.origin.id)
            .unwrap(),
        Some(r)
    );
}

#[test]
fn bad_record_hash_rehashed_structure_and_index_bindings_refuse_reads_lists_and_startup() {
    for case in 0..9 {
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        let conn = raw(path.path());
        let r = record();
        insert(&conn, &r);
        match case {
            0 => {
                conn.execute("UPDATE actions SET record_sha256=zeroblob(32)", [])
                    .unwrap();
            }
            1 => {
                conn.execute("UPDATE actions SET creation_sha256=zeroblob(32)", [])
                    .unwrap();
            }
            2 => {
                conn.execute("UPDATE actions SET version=2", []).unwrap();
            }
            3 => {
                conn.execute("UPDATE actions SET state='blocked'", [])
                    .unwrap();
            }
            4 => {
                conn.execute("UPDATE actions SET created_at_ms=999", [])
                    .unwrap();
            }
            5 => {
                let mut invalid = r.clone();
                invalid.version = 0;
                replace_json(&conn, r.origin.id, &serde_json::to_vec(&invalid).unwrap());
            }
            6 => {
                let mut invalid = r.clone();
                invalid.version = 2;
                invalid.data.state = ActionState::Completed;
                replace_json(&conn, r.origin.id, &serde_json::to_vec(&invalid).unwrap());
            }
            7 => {
                let mut invalid = r.clone();
                invalid.origin.id = Uuid::new_v4();
                replace_json(&conn, r.origin.id, &serde_json::to_vec(&invalid).unwrap());
            }
            _ => {
                let bytes = vec![b' '; 1024 * 1024];
                replace_json(&conn, r.origin.id, &bytes);
            }
        }
        assert!(store.action(r.origin.id).is_err(), "case {case}");
        assert!(
            store.action_list(&Default::default()).is_err(),
            "case {case}"
        );
        drop(conn);
        drop(store);
        assert_semantic_startup_refusal(path.path());
    }
}

#[test]
fn strict_json_checks_unknown_duplicate_and_missing_fields_at_every_record_layer() {
    let r = record();
    let mut malformed = Vec::new();
    for layer in ["record", "origin", "proposal", "data", "origin_data"] {
        let mut v = serde_json::to_value(&r).unwrap();
        let object = match layer {
            "record" => &mut v,
            "origin" => &mut v["origin"],
            "proposal" => &mut v["origin"]["proposal"],
            "data" => &mut v["data"],
            _ => &mut v["origin"]["data"],
        };
        object
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        malformed.push(serde_json::to_vec(&v).unwrap());
    }
    let encoded = serde_json::to_string(&r).unwrap();
    malformed.push(
        encoded
            .replacen("\"version\":1", "\"version\":1,\"version\":1", 1)
            .into_bytes(),
    );
    malformed.push(
        encoded
            .replacen(
                "\"description\":",
                "\"description\":\"duplicate\",\"description\":",
                1,
            )
            .into_bytes(),
    );
    malformed.push(
        encoded
            .replacen("\"version\":37", "\"version\":37,\"version\":37", 1)
            .into_bytes(),
    );
    let mut missing = serde_json::to_value(&r).unwrap();
    missing["origin"]
        .as_object_mut()
        .unwrap()
        .remove("proposal");
    malformed.push(serde_json::to_vec(&missing).unwrap());
    for bytes in malformed {
        assert!(serde_json::from_slice::<ActionRecord>(&bytes).is_err());
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        let conn = raw(path.path());
        insert(&conn, &r);
        replace_json(&conn, r.origin.id, &bytes);
        assert!(store.action(r.origin.id).is_err());
        assert!(store.action_list(&Default::default()).is_err());
        drop(conn);
        drop(store);
        assert_semantic_startup_refusal(path.path());
    }
    assert!(
        serde_json::from_value::<ActionListRequest>(
            json!({"state":null,"limit":25,"before":null,"extra":true})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ActionCursor>(
            json!({"created_at_ms":1,"id":r.origin.id,"extra":true})
        )
        .is_err()
    );
}

#[test]
fn immutable_creation_digest_remains_checked_after_later_revisions() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    let mut r = record();
    r.version = 9;
    r.updated_at_ms = 5000;
    r.data.title = "Changed current title".into();
    insert(&conn, &r);
    assert_eq!(store.action(r.origin.id).unwrap(), Some(r.clone()));
    r.origin.data.description.push_str("Changed original bytes");
    replace_json(&conn, r.origin.id, &serde_json::to_vec(&r).unwrap());
    assert!(store.action(r.origin.id).is_err());
    assert!(store.action_list(&Default::default()).is_err());
    drop(conn);
    drop(store);
    assert_semantic_startup_refusal(path.path());
}

#[test]
fn malformed_owned_schema_indexes_literal_constraints_and_extra_objects_refuse_startup() {
    for case in 0..6 {
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        drop(store);
        let conn = raw(path.path());
        match case {
            0=>conn.execute_batch("DROP INDEX actions_page;").unwrap(),
            1=>conn.execute_batch("DROP INDEX actions_page; CREATE INDEX actions_page ON actions(id DESC,created_at_ms DESC);").unwrap(),
            2=>conn.execute_batch("DROP INDEX actions_page; CREATE INDEX actions_page ON actions(created_at_ms ASC,id DESC);").unwrap(),
            3=>conn.execute_batch("ALTER TABLE actions ADD COLUMN unexpected TEXT;").unwrap(),
            4=>{
                let table:String=conn.query_row("SELECT sql FROM sqlite_schema WHERE name='actions'",[],|r|r.get(0)).unwrap();
                let index:String=conn.query_row("SELECT sql FROM sqlite_schema WHERE name='actions_page'",[],|r|r.get(0)).unwrap();
                conn.execute_batch("DROP TABLE actions;").unwrap();
                conn.execute_batch(&table.replace("state TEXT NOT NULL,", "state TEXT NOT NULL CHECK(state IN ('o pen','waiting','blocked','completed'))," )).unwrap(); conn.execute_batch(&index).unwrap();
            }
            _=>conn.execute_batch("CREATE TRIGGER actions_unexpected AFTER UPDATE ON actions BEGIN SELECT 1; END;").unwrap(),
        }
        drop(conn);
        assert_semantic_startup_refusal(path.path());
    }
}

#[test]
fn malformed_stored_domains_refuse_without_accidental_constraint_backup_restoration() {
    for sql in [
        "UPDATE actions SET version=0",
        "UPDATE actions SET state='unknown'",
        "UPDATE actions SET creation_sha256=zeroblob(31)",
        "UPDATE actions SET record_sha256=zeroblob(31)",
        "UPDATE actions SET created_at_ms=-1",
    ] {
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        let conn = raw(path.path());
        let r = record();
        insert(&conn, &r);
        conn.execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        conn.execute_batch(sql).unwrap();
        let snapshot: String=conn.query_row("SELECT quote(version)||quote(state)||quote(created_at_ms)||hex(creation_sha256)||hex(record_json)||hex(record_sha256) FROM actions",[],|row|row.get(0)).unwrap();
        assert!(store.action(r.origin.id).is_err());
        assert!(store.action_list(&Default::default()).is_err());
        drop(conn);
        drop(store);
        assert_semantic_startup_refusal(path.path());
        assert_eq!(raw(path.path()).query_row("SELECT quote(version)||quote(state)||quote(created_at_ms)||hex(creation_sha256)||hex(record_json)||hex(record_sha256) FROM actions",[],|row|row.get::<_,String>(0)).unwrap(),snapshot,"invalid semantic row must remain intact");
    }
}

#[test]
fn populated_unexpected_action_constraint_refuses_before_quick_check_backup_restoration() {
    let path = fixture();
    let (store, _) = WorkStore::open(path.path()).unwrap();
    let conn = raw(path.path());
    let table: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='actions'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let index: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='actions_page'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let changed_table = table.replace(
        "state TEXT NOT NULL,",
        "state TEXT NOT NULL CHECK(state='waiting'),",
    );
    assert_ne!(changed_table, table);
    conn.execute_batch("DROP TABLE actions;").unwrap();
    conn.execute_batch(&changed_table).unwrap();
    conn.execute_batch(&index).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints=ON;")
        .unwrap();
    let record = record();
    record.validate().unwrap();
    assert_eq!(record.data.state, ActionState::Open);
    insert(&conn, &record);
    let row_before: String = conn.query_row("SELECT quote(version)||quote(state)||quote(created_at_ms)||hex(creation_sha256)||hex(record_json)||hex(record_sha256) FROM actions", [], |row| row.get(0)).unwrap();
    drop(conn);
    drop(store);

    let conn = raw(path.path());
    assert_eq!(
        conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "CHECK constraint failed in actions"
    );
    let schema_before: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='actions'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);
    assert_semantic_startup_refusal(path.path());
    let conn = raw(path.path());
    assert_eq!(
        conn.query_row(
            "SELECT sql FROM sqlite_schema WHERE name='actions'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        schema_before
    );
    assert_eq!(conn.query_row("SELECT quote(version)||quote(state)||quote(created_at_ms)||hex(creation_sha256)||hex(record_json)||hex(record_sha256) FROM actions", [], |row| row.get::<_, String>(0)).unwrap(), row_before);
}

#[test]
fn noncanonical_and_nil_indexed_row_identities_refuse_lists_and_startup() {
    for id in [
        "NOT-A-UUID".to_owned(),
        Uuid::nil().to_string(),
        "ABCDEF01-2345-4678-9ABC-DEF012345678".to_owned(),
    ] {
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        let conn = raw(path.path());
        let r = record();
        insert(&conn, &r);
        conn.execute("UPDATE actions SET id=?1", [id]).unwrap();
        assert!(store.action_list(&Default::default()).is_err());
        drop(conn);
        drop(store);
        assert_semantic_startup_refusal(path.path());
    }
}

#[test]
fn physical_recovery_skips_action_invalid_backups_and_restores_exact_terminal_work() {
    let mut failures = Vec::new();
    for problem in ["indexed revision", "owned schema"] {
        let path = fixture();
        let (store, _) = WorkStore::open(path.path()).unwrap();
        let mut terminal = record();
        terminal.version = 2;
        terminal.data.state = ActionState::Completed;
        terminal
            .data
            .description
            .push_str("Completed current body. Tehtud.\r\n");
        terminal.updated_at_ms = 2000;
        terminal.completed_at_ms = Some(1750);
        terminal.validate().unwrap();
        let conn = raw(path.path());
        insert(&conn, &terminal);
        assert_eq!(
            store.action(terminal.origin.id).unwrap(),
            Some(terminal.clone())
        );
        drop(conn);
        drop(store);

        // This public startup validates the complete record before backing it up.
        let (store, report) = WorkStore::open(path.path()).unwrap();
        assert_eq!(
            store.action(terminal.origin.id).unwrap(),
            Some(terminal.clone())
        );
        let healthy_backup = report.backup;
        drop(store);
        let healthy_bytes = std::fs::read(&healthy_backup).unwrap();
        let invalid_backup = path.path().join("backups/brn-9999999999999.sqlite");
        let conn = raw(path.path());
        conn.backup("main", &invalid_backup, None).unwrap();
        drop(conn);
        let conn = Connection::open(&invalid_backup).unwrap();
        match problem {
            "indexed revision" => {
                assert_eq!(conn.execute("UPDATE actions SET version=3", []).unwrap(), 1);
            }
            _ => conn
                .execute_batch(
                    "DROP INDEX actions_page; CREATE INDEX actions_page ON actions(id DESC,created_at_ms DESC);",
                )
                .unwrap(),
        }
        drop(conn);

        let conn = Connection::open(&invalid_backup).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok",
            "{problem}: the candidate is physically healthy SQLite"
        );
        let (indexed_version, json, hash): (i64, Vec<u8>, Vec<u8>) = conn
            .query_row(
                "SELECT version,record_json,record_sha256 FROM actions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let decoded: ActionRecord = serde_json::from_slice(&json).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, terminal);
        assert_eq!(hash, digest(&json));
        let healthy = Connection::open(&healthy_backup).unwrap();
        for name in ["actions", "actions_page"] {
            let schema: String = conn
                .query_row(
                    "SELECT sql FROM sqlite_schema WHERE name=?1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            let healthy_schema: String = healthy
                .query_row(
                    "SELECT sql FROM sqlite_schema WHERE name=?1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            if problem == "indexed revision" || name == "actions" {
                assert_eq!(schema, healthy_schema, "{problem}: {name}");
            } else {
                assert_ne!(schema, healthy_schema, "unsupported owned index fixture");
            }
        }
        assert_eq!(
            indexed_version,
            if problem == "indexed revision" { 3 } else { 2 }
        );
        drop(healthy);
        drop(conn);
        let invalid_bytes = std::fs::read(&invalid_backup).unwrap();
        let physical_damage = b"synthetic physical Action database corruption";
        std::fs::write(path.path().join("brn.sqlite"), physical_damage).unwrap();

        match WorkStore::open(path.path()) {
            Ok((store, report)) => {
                assert_eq!(report.restored_from, Some(healthy_backup.clone()), "{problem}");
                assert_eq!(store.action(terminal.origin.id).unwrap(), Some(terminal), "{problem}");
                assert_eq!(
                    std::fs::read(report.corrupt_moved_to.expect("retain physical main")).unwrap(),
                    physical_damage
                );
            }
            Err(error) => failures.push(format!(
                "{problem}: recovery stopped at an invalid newest candidate instead of using the healthy older backup: {error}"
            )),
        }
        assert_eq!(
            std::fs::read(&invalid_backup).unwrap(),
            invalid_bytes,
            "{problem}"
        );
        assert_eq!(
            std::fs::read(&healthy_backup).unwrap(),
            healthy_bytes,
            "{problem}"
        );
        let moved: Vec<_> = std::fs::read_dir(path.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("corrupt")
            })
            .collect();
        assert_eq!(
            moved.len(),
            1,
            "{problem}: preserve the physically damaged main"
        );
        assert_eq!(
            std::fs::read(&moved[0]).unwrap(),
            physical_damage,
            "{problem}"
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn v9_additive_upgrade_and_physical_backup_restore_preserve_all_operational_work() {
    use brn_store::{
        files::{FileFingerprint, VaultIdentity, VaultRecord},
        work::{
            EditRequest, WorkTurnStatus,
            findings::*,
            proposal_apply::ApprovalRequest,
            proposals::{NoteChange, ProposalDraft, SourceVersion},
        },
    };
    let text = "\u{feff}Original English ja Eesti. Õun λ\r\n";
    for restore_v9 in [false, true] {
        let path = fixture();
        let (mut store, _) = WorkStore::open(path.path()).unwrap();
        store.set_setting("synthetic.actions", "retained").unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "Küsimus?\r\n", "chatgpt", "synthetic")
            .unwrap();
        let turn = store
            .finish_turn(
                turn.id,
                WorkTurnStatus::Completed,
                "Exact answer. Vastus.\r\n",
                None,
            )
            .unwrap();
        let fp = FileFingerprint {
            device: 1,
            inode: 2,
            len: text.len() as u64,
            sha256: digest(text.as_bytes()),
        };
        let editor = store.open_editor("current.md", &fp, text).unwrap();
        let editor = store
            .recover_editor(&EditRequest {
                path: editor.path.clone(),
                expected: editor.stamp,
                generation: editor.stamp.generation + 1,
                text: "\u{feff}Unfinished typing. Hilisem töö.\r\n".into(),
            })
            .unwrap();
        let vault = VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/actions-vault".into(),
            identity: VaultIdentity {
                device: 1,
                inode: 1,
            },
        };
        let note_id = Uuid::new_v4();
        let finding = store
            .create_finding(&FindingDraft {
                request: CaptureFindingRequest {
                    id: Uuid::new_v4(),
                    origin: FindingOrigin::IdentityAmbiguity { note_id },
                },
                vault: vault.clone(),
                title: "Retained tentative finding".into(),
                summary: "Two exact source proofs.\r\n".into(),
                evidence: ["a.md", "archive/b.md"]
                    .iter()
                    .enumerate()
                    .map(|(i, path)| FindingEvidence {
                        source: SourceVersion {
                            path: (*path).into(),
                            fingerprint: FileFingerprint {
                                inode: 3 + i as u64,
                                ..fp.clone()
                            },
                        },
                        note_id: Some(note_id),
                        quote: None,
                    })
                    .collect(),
            })
            .unwrap();
        let finding = store
            .close_finding(&CloseFindingRequest {
                expected: finding.stamp(),
                state: FindingState::Dismissed,
            })
            .unwrap();
        let proposal = store
            .create_proposal(&ProposalDraft {
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: Some(turn.conversation_id),
                vault: Some(vault.clone()),
                title: "Protected reviewed bytes".into(),
                changes: vec![NoteChange::Create {
                    path: "new.md".into(),
                    parent: vault.identity,
                    text: text.into(),
                }],
                sources: vec![],
                action_changes: Vec::new(),
            })
            .unwrap();
        let journal = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: proposal.stamp(),
            })
            .unwrap();
        let proposal = store.proposal(proposal.draft.id).unwrap().unwrap();
        drop(store);
        let conn = raw(path.path());
        conn.execute_batch(
            "DROP TABLE inbox_actions; DROP TABLE inbox_processing; DROP TABLE inbox_items; DROP TABLE action_completions; DROP TABLE actions; PRAGMA user_version=9;",
        )
        .unwrap();
        let old_backup = path.path().join("backups/brn-9999999999999.sqlite");
        if restore_v9 {
            conn.backup("main", &old_backup, None).unwrap();
        }
        drop(conn);
        if restore_v9 {
            std::fs::write(
                path.path().join("brn.sqlite"),
                b"synthetic physical V9 corruption",
            )
            .unwrap();
        }
        let (store, report) = WorkStore::open(path.path()).unwrap();
        assert_eq!(report.restored_from, restore_v9.then_some(old_backup));
        assert_eq!(
            raw(path.path())
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            14
        );
        assert!(
            store
                .action_list(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(store.turn(turn.id).unwrap()).unwrap(),
            serde_json::to_value(Some(turn.clone())).unwrap()
        );
        assert_eq!(store.editor(&editor.path).unwrap(), Some(editor.clone()));
        assert_eq!(
            store.finding(finding.draft.request.id).unwrap(),
            Some(finding.clone())
        );
        assert_eq!(
            store.proposal(proposal.draft.id).unwrap(),
            Some(proposal.clone())
        );
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
        let action = record();
        insert(&raw(path.path()), &action);
        assert_eq!(
            store.action(action.origin.id).unwrap(),
            Some(action.clone())
        );
        drop(store);
        let (store, report) = WorkStore::open(path.path()).unwrap();
        assert_eq!(
            store.action(action.origin.id).unwrap(),
            Some(action.clone())
        );
        let backup = report.backup;
        drop(store);
        std::fs::write(
            path.path().join("brn.sqlite"),
            b"synthetic physical V10 corruption",
        )
        .unwrap();
        let (store, report) = WorkStore::open(path.path()).unwrap();
        assert_eq!(report.restored_from, Some(backup));
        assert!(report.corrupt_moved_to.is_some());
        assert_eq!(store.action(action.origin.id).unwrap(), Some(action));
        assert_eq!(
            serde_json::to_value(store.turn(turn.id).unwrap()).unwrap(),
            serde_json::to_value(Some(turn)).unwrap()
        );
        assert_eq!(store.editor(&editor.path).unwrap(), Some(editor));
        assert_eq!(
            store.finding(finding.draft.request.id).unwrap(),
            Some(finding)
        );
        assert_eq!(store.proposal(proposal.draft.id).unwrap(), Some(proposal));
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal)
        );
        assert_eq!(
            store.setting("synthetic.actions").unwrap().as_deref(),
            Some("retained")
        );
    }
}
