use brn_store::{
    Error, WorkStore,
    work::{WorkBudget, WorkTurnStatus},
};
use rusqlite::Connection;
use serde_json::json;
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn raw(data: &std::path::Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}
fn counts(conn: &Connection) -> (i64, i64, i64) {
    conn.query_row("SELECT (SELECT count(*) FROM conversations),(SELECT count(*) FROM messages),(SELECT count(*) FROM ai_run_budgets)", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
}

#[test]
fn budget_wire_is_closed_bounded_and_requires_complete_explicit_values() {
    assert_eq!(
        WorkBudget::default(),
        WorkBudget {
            max_tool_rounds: 8,
            timeout_seconds: 300
        }
    );
    for (rounds, seconds) in [(1, 1), (32, 3600), (8, 180)] {
        let budget = WorkBudget {
            max_tool_rounds: rounds,
            timeout_seconds: seconds,
        };
        assert_eq!(
            serde_json::from_value::<WorkBudget>(serde_json::to_value(budget).unwrap()).unwrap(),
            budget
        );
    }
    for value in [
        json!({}),
        json!({"max_tool_rounds":8}),
        json!({"max_tool_rounds":8,"timeout_seconds":300,"extra":true}),
        json!({"max_tool_rounds":0,"timeout_seconds":300}),
        json!({"max_tool_rounds":33,"timeout_seconds":300}),
        json!({"max_tool_rounds":8,"timeout_seconds":0}),
        json!({"max_tool_rounds":8,"timeout_seconds":3601}),
        json!({"max_tool_rounds":-1,"timeout_seconds":300}),
        json!({"max_tool_rounds":1.5,"timeout_seconds":300}),
    ] {
        assert!(serde_json::from_value::<WorkBudget>(value).is_err());
    }
}

#[test]
fn frozen_budget_admission_replay_and_readonly_resolution_preserve_canonical_turns() {
    let data = fixture();
    let (owner, _) = WorkStore::open(data.path()).unwrap();
    let mut chat = owner.chat_connection().unwrap();
    let id = Uuid::new_v4();
    assert_eq!(
        owner.resolve_run_budget(id, None).unwrap(),
        Some(WorkBudget::default())
    );
    assert_eq!(
        chat.resolve_run_budget(id, None).unwrap(),
        Some(WorkBudget::default())
    );
    assert_eq!(counts(&raw(data.path())), (0, 0, 0));
    let budget = WorkBudget {
        max_tool_rounds: 16,
        timeout_seconds: 180,
    };
    let (turn, recorded) = chat
        .begin_turn_with_effort_and_budget(
            id,
            None,
            "exact õ\r\n",
            "chatgpt",
            "model",
            Some("medium"),
            Some(budget),
        )
        .unwrap();
    assert_eq!(recorded, Some(budget));
    assert_eq!(owner.run_budget(id).unwrap(), Some(budget));
    assert_eq!(chat.run_budget(id).unwrap(), Some(budget));
    let bytes = serde_json::to_vec(&turn).unwrap();
    assert!(
        !serde_json::to_value(&turn)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("budget")
    );
    for requested in [None, Some(budget)] {
        assert_eq!(
            owner.resolve_run_budget(id, requested).unwrap(),
            Some(budget)
        );
        let (replay, same) = chat
            .begin_turn_with_effort_and_budget(
                id,
                None,
                "exact õ\r\n",
                "chatgpt",
                "model",
                Some("medium"),
                requested,
            )
            .unwrap();
        assert_eq!(serde_json::to_vec(&replay).unwrap(), bytes);
        assert_eq!(same, Some(budget));
    }
    assert!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "exact õ\r\n",
            "chatgpt",
            "model",
            Some("medium"),
            None
        )
        .is_ok()
    );
    assert!(matches!(
        owner.resolve_run_budget(id, Some(WorkBudget::default())),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "exact õ\r\n",
            "chatgpt",
            "model",
            Some("medium"),
            Some(WorkBudget::default())
        ),
        Err(Error::OperationConflict(_))
    ));
    assert!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "changed",
            "chatgpt",
            "model",
            Some("medium"),
            None
        )
        .is_err()
    );
    assert_eq!(counts(&raw(data.path())), (1, 2, 1));
    assert_eq!(
        serde_json::to_vec(
            &chat
                .begin_turn_with_effort(id, None, "exact õ\r\n", "chatgpt", "model", Some("medium"))
                .unwrap()
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn invalid_admission_and_late_budget_insert_failure_roll_back_all_related_rows() {
    let data = fixture();
    let (owner, _) = WorkStore::open(data.path()).unwrap();
    let mut chat = owner.chat_connection().unwrap();
    for budget in [
        WorkBudget {
            max_tool_rounds: 0,
            timeout_seconds: 300,
        },
        WorkBudget {
            max_tool_rounds: 8,
            timeout_seconds: 3601,
        },
    ] {
        assert!(
            chat.begin_turn_with_effort_and_budget(
                Uuid::new_v4(),
                None,
                "q",
                "chatgpt",
                "model",
                Some("low"),
                Some(budget)
            )
            .is_err()
        );
        assert_eq!(counts(&raw(data.path())), (0, 0, 0));
    }
    assert!(
        chat.begin_turn_with_effort_and_budget(
            Uuid::new_v4(),
            None,
            " ",
            "chatgpt",
            "model",
            Some("low"),
            None
        )
        .is_err()
    );
    assert_eq!(counts(&raw(data.path())), (0, 0, 0));
    // Force a failure after the chat pair has been inserted, at budget insertion.
    // This is a test-only transaction fault, not a production hook or schema.
    let conn = raw(data.path());
    conn.execute_batch("CREATE TRIGGER inject_budget_collision AFTER INSERT ON messages WHEN NEW.role='assistant' BEGIN INSERT INTO ai_run_budgets VALUES(NEW.turn_id,8,300); END;").unwrap();
    let id = Uuid::new_v4();
    assert!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "q",
            "chatgpt",
            "model",
            Some("low"),
            None
        )
        .is_err()
    );
    assert_eq!(counts(&conn), (0, 0, 0));
    conn.execute_batch("DROP TRIGGER inject_budget_collision;")
        .unwrap();
    let (_, budget) = chat
        .begin_turn_with_effort_and_budget(id, None, "q", "chatgpt", "model", Some("low"), None)
        .unwrap();
    assert_eq!(budget, Some(WorkBudget::default()));
    assert_eq!(counts(&conn), (1, 2, 1));
}

#[test]
fn legacy_turn_replay_and_v16_upgrade_never_invent_a_budget() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let id = Uuid::new_v4();
    store
        .begin_turn_with_effort(id, None, "old", "chatgpt", "model", Some("low"))
        .unwrap();
    let turn = store
        .finish_turn(id, WorkTurnStatus::Completed, "exact old answer", None)
        .unwrap();
    let before = serde_json::to_vec(&turn).unwrap();
    drop(store);
    raw(data.path())
        .execute_batch("DROP TABLE conversation_lifecycle_operations; DROP TABLE conversation_lifecycle; DROP TABLE ai_run_budgets; PRAGMA user_version=16;")
        .unwrap();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    let mut chat = store.chat_connection().unwrap();
    assert_eq!(store.run_budget(id).unwrap(), None);
    assert_eq!(store.resolve_run_budget(id, None).unwrap(), None);
    let (replay, budget) = chat
        .begin_turn_with_effort_and_budget(id, None, "old", "chatgpt", "model", Some("low"), None)
        .unwrap();
    assert_eq!(budget, None);
    assert_eq!(serde_json::to_vec(&replay).unwrap(), before);
    assert!(matches!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "old",
            "chatgpt",
            "model",
            Some("low"),
            Some(WorkBudget::default())
        ),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(counts(&raw(data.path())), (1, 2, 0));
}

#[test]
fn restart_and_database_backup_retain_budgets_and_timeout_reason() {
    let data = fixture();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    let mut chat = store.chat_connection().unwrap();
    let id = Uuid::new_v4();
    let budget = WorkBudget {
        max_tool_rounds: 3,
        timeout_seconds: 180,
    };
    chat.begin_turn_with_effort_and_budget(
        id,
        None,
        "q",
        "chatgpt",
        "model",
        Some("high"),
        Some(budget),
    )
    .unwrap();
    drop(chat);
    drop(store);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.turn(id).unwrap().unwrap().status,
        WorkTurnStatus::Interrupted
    );
    assert_eq!(store.run_budget(id).unwrap(), Some(budget));
    let timed = Uuid::new_v4();
    store
        .begin_turn_with_effort_and_budget(
            timed,
            None,
            "timed",
            "copilot",
            "model",
            Some("high"),
            None,
        )
        .unwrap();
    let timed_turn = store
        .finish_turn(
            timed,
            WorkTurnStatus::Failed,
            "partial",
            Some("time_limit_reached"),
        )
        .unwrap();
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.run_budget(id).unwrap(), Some(budget));
    assert_eq!(
        store.run_budget(timed).unwrap(),
        Some(WorkBudget::default())
    );
    drop(store);
    std::fs::write(
        data.path().join("brn.sqlite"),
        b"synthetic physical corruption",
    )
    .unwrap();
    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert!(report.restored_from.is_some());
    assert_eq!(store.run_budget(id).unwrap(), Some(budget));
    assert_eq!(
        store.run_budget(timed).unwrap(),
        Some(WorkBudget::default())
    );
    assert_eq!(
        serde_json::to_vec(&store.turn(timed).unwrap().unwrap()).unwrap(),
        serde_json::to_vec(&timed_turn).unwrap()
    );
}

#[test]
fn retained_metadata_is_inspectable_but_cannot_recreate_a_deleted_run() {
    let data = fixture();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    let mut chat = store.chat_connection().unwrap();
    let id = Uuid::new_v4();
    chat.begin_turn_with_effort_and_budget(id, None, "q", "chatgpt", "model", Some("low"), None)
        .unwrap();
    let conn = raw(data.path());
    conn.execute("DELETE FROM messages WHERE turn_id=?1", [id.to_string()])
        .unwrap();
    conn.execute("DELETE FROM conversation_lifecycle", [])
        .unwrap();
    conn.execute("DELETE FROM conversations", []).unwrap();
    assert_eq!(
        store.resolve_run_budget(id, None).unwrap(),
        Some(WorkBudget::default())
    );
    assert!(
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "q",
            "chatgpt",
            "model",
            Some("low"),
            None
        )
        .is_err()
    );
    assert!(
        chat.begin_turn_with_effort(id, None, "q", "chatgpt", "model", Some("low"))
            .is_err()
    );
    assert_eq!(counts(&conn), (0, 0, 1));
    drop(conn);
    drop(chat);
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.run_budget(id).unwrap(), Some(WorkBudget::default()));
}

#[test]
fn readable_budget_damage_refuses_before_backup_or_older_restore() {
    for case in ["bounds", "uuid", "nil", "fraction", "schema", "extra"] {
        let data = fixture();
        let (store, _) = WorkStore::open(data.path()).unwrap();
        let mut chat = store.chat_connection().unwrap();
        let id = Uuid::new_v4();
        chat.begin_turn_with_effort_and_budget(
            id,
            None,
            "q",
            "chatgpt",
            "model",
            Some("low"),
            None,
        )
        .unwrap();
        drop(chat);
        drop(store);
        let conn = raw(data.path());
        match case {
            "bounds" => conn.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE ai_run_budgets SET max_tool_rounds=33;").unwrap(),
            "uuid" => { conn.execute("UPDATE ai_run_budgets SET run_id=?1", ["ABCDEFAB-CDEF-4ABC-8DEF-ABCDEFABCDEF"]).unwrap(); },
            "nil" => { conn.execute("UPDATE ai_run_budgets SET run_id=?1", [Uuid::nil().to_string()]).unwrap(); },
            "fraction" => conn.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE ai_run_budgets SET timeout_seconds=1.5;").unwrap(),
            "schema" => conn.execute_batch("ALTER TABLE ai_run_budgets ADD COLUMN hidden INTEGER;").unwrap(),
            "extra" => conn.execute_batch("CREATE INDEX unexpected_budget_index ON ai_run_budgets(timeout_seconds);").unwrap(),
            _ => unreachable!(),
        }
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        drop(conn);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(WorkStore::open(data.path()).is_err(), "{case}");
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before,
            "{case}"
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups,
            "{case}"
        );
    }
    // Querying an unknown ID never manufactures a row.
    let data = fixture();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.run_budget(Uuid::new_v4()).unwrap(), None);
    assert!(store.run_budget(Uuid::nil()).is_err());
    assert_eq!(counts(&raw(data.path())), (0, 0, 0));
}
