use brn_store::{
    Error,
    work::{WorkStore, WorkTurnStatus},
};
use rusqlite::{Connection, params};
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn raw(dir: &std::path::Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}

fn counts(conn: &Connection) -> (i64, i64) {
    (
        conn.query_row("SELECT count(*) FROM conversations", [], |r| r.get(0))
            .unwrap(),
        conn.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
            .unwrap(),
    )
}

#[test]
fn terminal_pair_preserves_exact_text_and_selection() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    let question = "  café 🦀\r\n第二行\n";
    let answer = "\r\n答え e\u{301}  \n";
    let started = store
        .begin_turn(id, None, question, "copilot", "gpt-4o")
        .unwrap();
    assert_eq!(started.status, WorkTurnStatus::Running);
    assert_eq!(started.answer, "");
    let ended = store
        .finish_turn(id, WorkTurnStatus::Completed, answer, None)
        .unwrap();
    assert_eq!(ended.question, question);
    assert_eq!(ended.answer, answer);
    let conversations = store.conversations().unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].id, started.conversation_id);
    assert_eq!(conversations[0].title, question);
    assert_eq!(conversations[0].turns, 1);
    let conn = raw(dir.path());
    let rows: Vec<(String, String, String, String, i64)> = conn
        .prepare("SELECT role, provider, model, status, sequence FROM messages ORDER BY role")
        .unwrap()
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows.len(), 2);
    for (_, provider, model, status, sequence) in rows {
        assert_eq!(
            (provider.as_str(), model.as_str(), status.as_str(), sequence),
            ("copilot", "gpt-4o", "completed", 1)
        );
    }
    drop(conn);
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.turns(started.conversation_id).unwrap()[0].answer,
        answer
    );
}

#[test]
fn exact_uuid_replay_returns_running_and_terminal_without_new_rows() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    let first = store
        .begin_turn(id, None, "question\r\n", "chatgpt", "gpt-5.5")
        .unwrap();
    assert_eq!(
        store
            .begin_turn(id, None, "question\r\n", "chatgpt", "gpt-5.5")
            .unwrap()
            .conversation_id,
        first.conversation_id
    );
    store
        .finish_turn(id, WorkTurnStatus::Failed, "partial", Some("network"))
        .unwrap();
    assert_eq!(
        store
            .begin_turn(
                id,
                Some(first.conversation_id),
                "question\r\n",
                "chatgpt",
                "gpt-5.5"
            )
            .unwrap()
            .status,
        WorkTurnStatus::Failed
    );
    assert_eq!(
        store
            .finish_turn(id, WorkTurnStatus::Failed, "partial", Some("network"))
            .unwrap()
            .answer,
        "partial"
    );
    for (conversation, question, provider, model) in [
        (None, "question\n", "chatgpt", "gpt-5.5"),
        (None, "question\r\n", "copilot", "gpt-5.5"),
        (None, "question\r\n", "chatgpt", "different"),
        (Some(Uuid::new_v4()), "question\r\n", "chatgpt", "gpt-5.5"),
    ] {
        assert!(matches!(
            store.begin_turn(id, conversation, question, provider, model),
            Err(Error::OperationConflict(_))
        ));
    }
    for (status, answer, error) in [
        (WorkTurnStatus::Running, "partial", Some("network")),
        (WorkTurnStatus::Completed, "partial", Some("network")),
        (WorkTurnStatus::Failed, "changed", Some("network")),
        (WorkTurnStatus::Failed, "partial", None),
    ] {
        assert!(store.finish_turn(id, status, answer, error).is_err());
    }
    assert_eq!(counts(&raw(dir.path())), (1, 2));
}

#[test]
fn missing_conversation_and_turn_are_typed_and_insert_nothing() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let unknown = Uuid::new_v4();
    assert!(matches!(
        store.begin_turn(Uuid::new_v4(), Some(unknown), "q", "copilot", "model"),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(store.turns(unknown), Err(Error::NotFound(_))));
    assert!(matches!(
        store.finish_turn(unknown, WorkTurnStatus::Failed, "", None),
        Err(Error::NotFound(_))
    ));
    assert_eq!(counts(&raw(dir.path())), (0, 0));
}

#[test]
fn restart_reconciles_running_before_backup_and_never_repeats() {
    let dir = fixture();
    let id = Uuid::new_v4();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = store.begin_turn(id, None, "q", "copilot", "model").unwrap();
    drop(store);
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    let replay = store.begin_turn(id, None, "q", "copilot", "model").unwrap();
    assert_eq!(replay.status, WorkTurnStatus::Interrupted);
    assert_eq!(replay.answer, "");
    assert_eq!(replay.conversation_id, turn.conversation_id);
    let backup = Connection::open(report.backup).unwrap();
    let interrupted: i64 = backup
        .query_row(
            "SELECT count(*) FROM messages WHERE status = 'interrupted'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(interrupted, 2);
    assert_eq!(counts(&raw(dir.path())), (1, 2));
}

#[test]
fn restored_running_backup_retains_only_durable_partial_text() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    let turn = store
        .begin_turn(id, None, "first\r\n", "copilot", "model")
        .unwrap();
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE messages SET text = ?1 WHERE role = 'assistant'",
        ["durable 🦀\r\n"],
    )
    .unwrap();
    let backup = dir.path().join("backups/brn-9999999999999.sqlite");
    conn.backup("main", &backup, None).unwrap();
    drop(conn);
    drop(store);
    for suffix in ["", "-wal", "-shm"] {
        let path = dir.path().join(format!("brn.sqlite{suffix}"));
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }
    std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(backup));
    let replay = store
        .begin_turn(id, None, "first\r\n", "copilot", "model")
        .unwrap();
    assert_eq!(replay.status, WorkTurnStatus::Interrupted);
    assert_eq!(replay.answer, "durable 🦀\r\n");
    assert_eq!(store.conversations().unwrap()[0].title, "first\r\n");
    assert_eq!(store.turns(turn.conversation_id).unwrap().len(), 1);
    let backup = Connection::open(report.backup).unwrap();
    assert_eq!(
        backup
            .query_row(
                "SELECT text FROM messages WHERE role = 'assistant' AND status = 'interrupted'",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap(),
        "durable 🦀\r\n"
    );
}

#[test]
fn twenty_one_local_turns_and_partial_terminal_text_survive_restart() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let mut conversation = None;
    for n in 0..21 {
        let id = Uuid::new_v4();
        let turn = store
            .begin_turn(id, conversation, &format!("q{n}"), "copilot", "model")
            .unwrap();
        conversation = Some(turn.conversation_id);
        let status = if n % 2 == 0 {
            WorkTurnStatus::Interrupted
        } else {
            WorkTurnStatus::Failed
        };
        store
            .finish_turn(id, status, &format!("partial{n}\r\n"), Some("network"))
            .unwrap();
    }
    assert_eq!(store.conversations().unwrap()[0].title, "q0");
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    let turns = store.turns(conversation.unwrap()).unwrap();
    assert_eq!(turns.len(), 21);
    for (n, turn) in turns.iter().enumerate() {
        assert_eq!(turn.question, format!("q{n}"));
        assert_eq!(turn.answer, format!("partial{n}\r\n"));
        assert_eq!(turn.error_code.as_deref(), Some("network"));
    }
    assert_eq!(store.conversations().unwrap()[0].turns, 21);
    let sequences: Vec<i64> = raw(dir.path())
        .prepare("SELECT sequence FROM messages WHERE role = 'user' ORDER BY sequence")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(sequences, (1..=21).collect::<Vec<_>>());
}

#[test]
fn validation_rejects_unsafe_selection_and_error_codes_without_changes() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    for (question, provider, model) in [
        (" \r\n", "copilot", "model"),
        ("q", "other", "model"),
        ("q", "Copilot", "model"),
        ("q", "copilot", ""),
        ("q", "copilot", " "),
        ("q", "copilot", "model\nbody"),
        ("q", "copilot", "模型"),
        ("q", "copilot", &"m".repeat(129)),
    ] {
        assert!(matches!(
            store.begin_turn(Uuid::new_v4(), None, question, provider, model),
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(counts(&raw(dir.path())), (0, 0));
    let id = Uuid::new_v4();
    let turn = store
        .begin_turn(id, None, "q", "copilot", &"m".repeat(128))
        .unwrap();
    for code in [
        "",
        "Network",
        "raw body",
        "token=synthetic",
        "device_code",
        "{\"body\":1}",
    ] {
        assert!(matches!(
            store.finish_turn(id, WorkTurnStatus::Failed, "partial", Some(code)),
            Err(Error::Invalid(_))
        ));
    }
    assert!(
        store
            .finish_turn(id, WorkTurnStatus::Running, "", None)
            .is_err()
    );
    assert_eq!(
        store.turns(turn.conversation_id).unwrap()[0].status,
        WorkTurnStatus::Running
    );
    for code in [
        "reconnect_needed",
        "code_expired",
        "rate_limited",
        "network",
        "model_refused",
        "invalid_tool_use",
        "tool_limit_reached",
        "unsafe_credentials",
        "tool_rejected",
        "index_stale",
        "storage",
        "other",
    ] {
        let id = Uuid::new_v4();
        store
            .begin_turn(
                id,
                Some(turn.conversation_id),
                "q",
                "copilot",
                "org/model:v1.2_3",
            )
            .unwrap();
        assert_eq!(
            store
                .finish_turn(id, WorkTurnStatus::Failed, "", Some(code))
                .unwrap()
                .error_code
                .as_deref(),
            Some(code)
        );
    }
}

#[test]
fn injected_second_row_failure_rolls_back_begin_and_finish() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_assistant BEFORE INSERT ON messages WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    assert!(
        store
            .begin_turn(Uuid::new_v4(), None, "q", "copilot", "model")
            .is_err()
    );
    assert_eq!(counts(&conn), (0, 0));
    conn.execute_batch("DROP TRIGGER fail_assistant").unwrap();
    let id = Uuid::new_v4();
    let turn = store.begin_turn(id, None, "q", "copilot", "model").unwrap();
    conn.execute_batch("CREATE TRIGGER fail_finish BEFORE UPDATE ON messages WHEN NEW.role = 'assistant' BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    assert!(
        store
            .finish_turn(id, WorkTurnStatus::Completed, "answer", None)
            .is_err()
    );
    let retained = store.turns(turn.conversation_id).unwrap();
    assert_eq!(retained[0].status, WorkTurnStatus::Running);
    assert_eq!(retained[0].answer, "");
    assert_eq!(store.conversations().unwrap()[0].title, "");
    conn.execute_batch("DROP TRIGGER fail_finish").unwrap();
    conn.execute_batch(
        "CREATE TRIGGER fail_title BEFORE UPDATE ON conversations
         BEGIN SELECT RAISE(ABORT, 'synthetic title failure'); END;",
    )
    .unwrap();
    assert!(
        store
            .finish_turn(id, WorkTurnStatus::Completed, "answer", None)
            .is_err()
    );
    assert_eq!(
        store.turns(turn.conversation_id).unwrap()[0].status,
        WorkTurnStatus::Running
    );
    assert_eq!(store.turns(turn.conversation_id).unwrap()[0].answer, "");
    conn.execute_batch("DROP TRIGGER fail_title").unwrap();
    store
        .finish_turn(id, WorkTurnStatus::Completed, "answer", None)
        .unwrap();
}

fn v1(conn: &Connection) {
    conn.execute_batch("PRAGMA application_id = 1112690226; PRAGMA user_version = 1;
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE unsaved_edits (path TEXT PRIMARY KEY, base_sha256 BLOB NOT NULL CHECK(length(base_sha256) = 32), text TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);
        INSERT INTO settings VALUES ('selection', 'synthetic');
        INSERT INTO unsaved_edits VALUES ('note.md', zeroblob(32), 'exact\r\n🦀', 123);").unwrap();
}

#[test]
fn v1_upgrade_and_restored_v1_backup_preserve_work() {
    for restored in [false, true] {
        let dir = fixture();
        let backup = dir.path().join("backups/brn-0000000000001.sqlite");
        if restored {
            std::fs::create_dir(dir.path().join("backups")).unwrap();
            v1(&Connection::open(&backup).unwrap());
            std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
        } else {
            v1(&raw(dir.path()));
        }
        let (store, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(report.restored_from, restored.then_some(backup));
        assert_eq!(
            store.setting("selection").unwrap().as_deref(),
            Some("synthetic")
        );
        let edit = store.unsaved_edit("note.md").unwrap().unwrap();
        assert_eq!(edit.text, "exact\r\n🦀");
        assert_eq!(edit.base_sha256, [0; 32]);
        assert_eq!(edit.updated_at_ms, 123);
        assert!(store.conversations().unwrap().is_empty());
        let version: i64 = raw(dir.path())
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 8);
    }
}

#[test]
fn chat_pair_schema_preserves_constraints_with_optional_effort() {
    let dir = fixture();
    drop(WorkStore::open(dir.path()).unwrap());
    let conn = raw(dir.path());
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(messages)")
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        columns,
        [
            "turn_id",
            "conversation_id",
            "sequence",
            "role",
            "text",
            "provider",
            "model",
            "status",
            "error_code",
            "effort",
            "started_at_ms",
            "finished_at_ms"
        ]
    );
    let conversation_columns: Vec<String> = conn
        .prepare("PRAGMA table_info(conversations)")
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        conversation_columns,
        ["id", "title", "created_at_ms", "last_activity_at_ms"]
    );
    let application_id: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    assert_eq!(application_id, 0x4252_4e32);
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = 'messages_conversation'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap(),
        1
    );
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let c = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO conversations(id,title,created_at_ms) VALUES (?1, '', 0)",
        [&c],
    )
    .unwrap();
    let insert = "INSERT INTO messages(turn_id,conversation_id,sequence,role,text,provider,model,status,error_code) VALUES (?1, ?2, 1, ?3, '', ?4, 'model', ?5, NULL)";
    assert!(
        conn.execute(insert, params!["t", "absent", "user", "copilot", "running"])
            .is_err()
    );
    assert!(
        conn.execute(insert, params!["t", c, "tool", "copilot", "running"])
            .is_err()
    );
    assert!(
        conn.execute(insert, params!["t", c, "user", "other", "running"])
            .is_err()
    );
    assert!(
        conn.execute(insert, params!["t", c, "user", "copilot", "unknown"])
            .is_err()
    );
    conn.execute(insert, params!["t", c, "user", "copilot", "running"])
        .unwrap();
    assert!(
        conn.execute(insert, params!["t", c, "user", "copilot", "running"])
            .is_err()
    );
    assert!(
        conn.execute(
            insert,
            params!["different", c, "user", "copilot", "running"]
        )
        .is_err()
    );
    conn.execute("DELETE FROM conversations WHERE id = ?1", [c])
        .unwrap();
    assert_eq!(counts(&conn), (0, 0));
}

#[test]
fn malformed_pairs_are_rejected_not_silently_reconciled() {
    for sql in [
        "DELETE FROM messages WHERE role = 'assistant'",
        "UPDATE messages SET model = 'different' WHERE role = 'assistant'",
        "UPDATE messages SET provider = 'chatgpt' WHERE role = 'assistant'",
        "UPDATE messages SET sequence = 0",
        "UPDATE messages SET error_code = 'raw body'",
        "UPDATE messages SET model = 'unsafe model'",
        "UPDATE messages SET status = 'completed' WHERE role = 'assistant'",
    ] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "q", "copilot", "model")
            .unwrap();
        raw(dir.path()).execute_batch(sql).unwrap();
        assert!(matches!(
            store.turns(turn.conversation_id),
            Err(Error::Invalid(_))
        ));
        drop(store);
        assert!(matches!(
            WorkStore::open(dir.path()),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn newer_schema_is_untouched_and_backups_keep_five() {
    let dir = fixture();
    let mut backups = Vec::new();
    for _ in 0..7 {
        let (_, report) = WorkStore::open(dir.path()).unwrap();
        backups.push(report.backup);
    }
    for (n, backup) in backups.iter().enumerate() {
        assert_eq!(backup.exists(), n >= 2);
    }
    raw(dir.path())
        .pragma_update(None, "user_version", 99)
        .unwrap();
    let before = std::fs::read(dir.path().join("brn.sqlite")).unwrap();
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        std::fs::read(dir.path().join("brn.sqlite")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        5
    );
}
