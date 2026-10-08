use brn_store::{
    Error, WorkStore,
    work::{WorkTurn, WorkTurnStatus},
};
use rusqlite::{Connection, params};
use std::path::Path;
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn raw(data: &Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}

fn effort_pair(conn: &Connection, id: Uuid) -> Vec<Option<String>> {
    conn.prepare("SELECT effort FROM messages WHERE turn_id=?1 ORDER BY role")
        .unwrap()
        .query_map([id.to_string()], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn count(conn: &Connection, table: &str) -> i64 {
    let query = match table {
        "messages" => "SELECT count(*) FROM messages",
        "conversations" => "SELECT count(*) FROM conversations",
        _ => unreachable!(),
    };
    conn.query_row(query, [], |row| row.get(0)).unwrap()
}

#[test]
fn explicit_effort_is_exact_in_both_rows_terminal_history_and_restored_backup() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut attached = store.chat_connection().unwrap();
    let mut conversation = None;
    let mut expected = vec![];
    for (index, effort) in ["low", "medium", "high"].into_iter().enumerate() {
        let id = Uuid::new_v4();
        let question = format!("  café 🦀\r\n第二行 {index}\n");
        let answer = format!("\r\n答え e\u{301} {index}  \n");
        let started = if index == 1 {
            attached
                .begin_turn_with_effort(
                    id,
                    conversation,
                    &question,
                    "copilot",
                    "explicit-model",
                    Some(effort),
                )
                .unwrap()
        } else {
            store
                .begin_turn_with_effort(
                    id,
                    conversation,
                    &question,
                    "copilot",
                    "explicit-model",
                    Some(effort),
                )
                .unwrap()
        };
        conversation = Some(started.conversation_id);
        assert_eq!(started.effort.as_deref(), Some(effort));
        assert_eq!(started.status, WorkTurnStatus::Running);
        assert_eq!(
            effort_pair(&raw(data.path()), id),
            vec![Some(effort.into()); 2]
        );
        let finished = attached
            .finish_turn(id, WorkTurnStatus::Completed, &answer, None)
            .unwrap();
        assert_eq!(finished.effort, started.effort);
        assert_eq!(finished.question, question);
        assert_eq!(finished.answer, answer);
        assert_eq!(serde_json::to_value(&finished).unwrap()["effort"], effort);
        expected.push(serde_json::to_value(finished).unwrap());
    }
    drop(attached);
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        serde_json::to_value(store.turns(conversation.unwrap()).unwrap()).unwrap(),
        serde_json::Value::Array(expected.clone())
    );
    drop(store); // last validated startup backup includes the exact terminal pairs
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert!(report.restored_from.is_some());
    assert_eq!(
        serde_json::to_value(store.turns(conversation.unwrap()).unwrap()).unwrap(),
        serde_json::Value::Array(expected)
    );
}

#[test]
fn unknown_historical_effort_keeps_absent_json_shape_and_is_never_defaulted() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let started = store
        .begin_turn(Uuid::new_v4(), None, "historical", "chatgpt", "gpt-5.5")
        .unwrap();
    assert!(started.effort.is_none());
    assert_eq!(effort_pair(&raw(data.path()), started.id), vec![None, None]);
    let finished = store
        .finish_turn(started.id, WorkTurnStatus::Completed, "old answer", None)
        .unwrap();
    let historical = serde_json::to_value(&finished).unwrap();
    assert!(historical.get("effort").is_none());
    let restored: WorkTurn = serde_json::from_value(historical.clone()).unwrap();
    assert!(restored.effort.is_none());
    assert_eq!(serde_json::to_value(restored).unwrap(), historical);
    let replay = store
        .begin_turn_with_effort(started.id, None, "historical", "chatgpt", "gpt-5.5", None)
        .unwrap();
    assert!(replay.effort.is_none());
    assert_eq!(replay.status, WorkTurnStatus::Completed);
    assert!(matches!(
        store.begin_turn_with_effort(
            started.id,
            None,
            "historical",
            "chatgpt",
            "gpt-5.5",
            Some("low")
        ),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn running_and_terminal_replay_bind_some_none_and_the_exact_effort_value() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut attached = store.chat_connection().unwrap();
    let id = Uuid::new_v4();
    let first = attached
        .begin_turn_with_effort(
            id,
            None,
            "exact request\r\n",
            "chatgpt",
            "gpt-5.5",
            Some("medium"),
        )
        .unwrap();
    assert_eq!(
        store
            .begin_turn_with_effort(
                id,
                None,
                &first.question,
                &first.provider,
                &first.model,
                Some("medium")
            )
            .unwrap()
            .conversation_id,
        first.conversation_id
    );
    for effort in [None, Some("low"), Some("high")] {
        assert!(matches!(
            store.begin_turn_with_effort(
                id,
                None,
                &first.question,
                &first.provider,
                &first.model,
                effort
            ),
            Err(Error::OperationConflict(_))
        ));
    }
    assert!(matches!(
        store.begin_turn(id, None, &first.question, &first.provider, &first.model),
        Err(Error::OperationConflict(_))
    ));
    let final_turn = attached
        .finish_turn(id, WorkTurnStatus::Failed, "partial\r\n🦀", Some("network"))
        .unwrap();
    assert_eq!(final_turn.effort.as_deref(), Some("medium"));
    assert_eq!(
        serde_json::to_value(
            store
                .begin_turn_with_effort(
                    id,
                    Some(first.conversation_id),
                    &first.question,
                    &first.provider,
                    &first.model,
                    Some("medium")
                )
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&final_turn).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            store
                .finish_turn(
                    id,
                    WorkTurnStatus::Failed,
                    &final_turn.answer,
                    Some("network")
                )
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&final_turn).unwrap()
    );
    assert!(matches!(
        attached.begin_turn_with_effort(
            id,
            None,
            &first.question,
            &first.provider,
            &first.model,
            Some("low")
        ),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(count(&raw(data.path()), "messages"), 2);
    assert_eq!(count(&raw(data.path()), "conversations"), 1);
    assert_eq!(
        effort_pair(&raw(data.path()), id),
        vec![Some("medium".into()); 2]
    );
}

#[test]
fn invalid_effort_is_refused_by_admission_and_schema_without_partial_pairs() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let conn = raw(data.path());
    for effort in [
        "",
        "LOW",
        "normal",
        "high\n",
        "ultra",
        "provider secret message",
    ] {
        assert!(matches!(
            store.begin_turn_with_effort(
                Uuid::new_v4(),
                None,
                "question",
                "copilot",
                "model",
                Some(effort)
            ),
            Err(Error::Invalid(_))
        ));
        assert_eq!(count(&conn, "messages"), 0);
        assert_eq!(count(&conn, "conversations"), 0);
    }
    let turn = store
        .begin_turn_with_effort(
            Uuid::new_v4(),
            None,
            "question",
            "copilot",
            "model",
            Some("high"),
        )
        .unwrap();
    assert!(
        conn.execute(
            "UPDATE messages SET effort='ultra' WHERE turn_id=?1",
            [turn.id.to_string()]
        )
        .is_err()
    );
    assert_eq!(effort_pair(&conn, turn.id), vec![Some("high".into()); 2]);
}

#[test]
fn cross_role_drift_and_invalid_values_fail_reads_and_checked_startup() {
    for mutation in [
        "UPDATE messages SET effort='high' WHERE role='assistant'",
        "UPDATE messages SET effort=NULL WHERE role='assistant'",
        "PRAGMA ignore_check_constraints=ON; UPDATE messages SET effort='ultra';",
    ] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let turn = store
            .begin_turn_with_effort(
                Uuid::new_v4(),
                None,
                "question",
                "copilot",
                "model",
                Some("low"),
            )
            .unwrap();
        let attached = store.chat_connection().unwrap();
        let conn = raw(data.path());
        conn.execute_batch(mutation).unwrap();
        assert!(matches!(store.turn(turn.id), Err(Error::Invalid(_))));
        assert!(matches!(
            store.turns(turn.conversation_id),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(store.conversations(), Err(Error::Invalid(_))));
        assert!(matches!(attached.turn(turn.id), Err(Error::Invalid(_))));
        assert!(matches!(
            attached.turns(turn.conversation_id),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.begin_turn_with_effort(
                turn.id,
                None,
                "question",
                "copilot",
                "model",
                Some("low")
            ),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.finish_turn(turn.id, WorkTurnStatus::Interrupted, "", None),
            Err(Error::Invalid(_))
        ));
        drop(attached);
        drop(store);
        let before = effort_pair(&conn, turn.id);
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        drop(conn);
        // Invalid CHECK data may trigger the existing integrity/backup recovery
        // path. Drift between individually valid rows must refuse unchanged.
        if mutation.contains("ultra") {
            let (recovered, report) = WorkStore::open(data.path()).unwrap();
            assert!(report.restored_from.is_some());
            assert!(recovered.turn(turn.id).unwrap().is_none());
            let preserved = Connection::open(report.corrupt_moved_to.unwrap()).unwrap();
            assert_eq!(effort_pair(&preserved, turn.id), before);
            assert_eq!(before, vec![Some("ultra".into()); 2]);
        } else {
            assert!(matches!(
                WorkStore::open(data.path()),
                Err(Error::Invalid(_))
            ));
            assert_eq!(effort_pair(&raw(data.path()), turn.id), before);
            assert_eq!(
                std::fs::read_dir(data.path().join("backups"))
                    .unwrap()
                    .count(),
                backups
            );
        }
    }
}

#[test]
fn explicit_admission_and_terminal_failures_roll_back_both_effort_rows() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let conn = raw(data.path());
    conn.execute_batch("CREATE TRIGGER refuse_assistant BEFORE INSERT ON messages WHEN NEW.role='assistant' BEGIN SELECT RAISE(ABORT,'synthetic insert failure'); END;").unwrap();
    assert!(matches!(
        store.begin_turn_with_effort(
            Uuid::new_v4(),
            None,
            "question",
            "copilot",
            "model",
            Some("medium")
        ),
        Err(Error::Sql(_))
    ));
    assert_eq!(count(&conn, "messages"), 0);
    assert_eq!(count(&conn, "conversations"), 0);
    conn.execute_batch("DROP TRIGGER refuse_assistant;")
        .unwrap();
    let turn = store
        .begin_turn_with_effort(
            Uuid::new_v4(),
            None,
            "question",
            "copilot",
            "model",
            Some("medium"),
        )
        .unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_assistant_finish BEFORE UPDATE ON messages WHEN NEW.role='assistant' BEGIN SELECT RAISE(ABORT,'synthetic finish failure'); END;").unwrap();
    assert!(matches!(
        store.finish_turn(turn.id, WorkTurnStatus::Completed, "answer", None),
        Err(Error::Sql(_))
    ));
    let retained = store.turn(turn.id).unwrap().unwrap();
    assert_eq!(retained.status, WorkTurnStatus::Running);
    assert_eq!(retained.answer, "");
    assert_eq!(retained.effort.as_deref(), Some("medium"));
    assert_eq!(effort_pair(&conn, turn.id), vec![Some("medium".into()); 2]);
    conn.execute_batch("DROP TRIGGER refuse_assistant_finish;")
        .unwrap();
    assert_eq!(
        store
            .finish_turn(turn.id, WorkTurnStatus::Completed, "answer", None)
            .unwrap()
            .effort
            .as_deref(),
        Some("medium")
    );
}

#[test]
fn v6_upgrade_and_restored_v6_backup_preserve_old_history_and_interrupt_running() {
    for restored in [false, true] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        store.set_setting("synthetic", "kept").unwrap();
        store
            .put_unsaved_edit("old.md", [1; 32], "protected typing 🦀\r\n")
            .unwrap();
        let completed = store
            .begin_turn(
                Uuid::new_v4(),
                None,
                "old question\r\n",
                "copilot",
                "historical-model",
            )
            .unwrap();
        let completed = store
            .finish_turn(
                completed.id,
                WorkTurnStatus::Completed,
                "old answer 🦀\r\n",
                None,
            )
            .unwrap();
        let running = store
            .begin_turn(
                Uuid::new_v4(),
                Some(completed.conversation_id),
                "interrupted old request",
                "chatgpt",
                "gpt-5.5",
            )
            .unwrap();
        let conn = raw(data.path());
        conn.execute(
            "UPDATE messages SET text=?2 WHERE turn_id=?1 AND role='assistant'",
            params![running.id.to_string(), "durable prior partial\r\n"],
        )
        .unwrap();
        drop(store);
        conn.execute_batch("DROP TABLE conversation_lifecycle_operations; DROP TABLE conversation_lifecycle; DROP TABLE ai_run_budgets; DROP TABLE intake_snapshots; DROP TABLE inbox_original_operations; DROP TABLE inbox_actions; DROP TABLE inbox_processing; DROP TABLE inbox_items; DROP TABLE action_completions; DROP TABLE actions; DROP TABLE findings; ALTER TABLE messages DROP COLUMN started_at_ms; ALTER TABLE messages DROP COLUMN finished_at_ms; ALTER TABLE conversations DROP COLUMN last_activity_at_ms; ALTER TABLE messages DROP COLUMN effort; PRAGMA user_version=6;")
            .unwrap();
        let backup = data.path().join("backups/brn-9999999999999.sqlite");
        if restored {
            conn.backup("main", &backup, None).unwrap();
        }
        drop(conn);
        if restored {
            std::fs::write(data.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
        }
        let (mut store, report) = WorkStore::open(data.path()).unwrap();
        assert_eq!(report.restored_from, restored.then_some(backup));
        let mut expected_legacy = serde_json::to_value(&completed).unwrap();
        expected_legacy["started_at_ms"] = serde_json::Value::Null;
        expected_legacy["finished_at_ms"] = serde_json::Value::Null;
        assert_eq!(
            serde_json::to_value(store.turn(completed.id).unwrap().unwrap()).unwrap(),
            expected_legacy
        );
        let resumed = store.turn(running.id).unwrap().unwrap();
        assert_eq!(resumed.status, WorkTurnStatus::Interrupted);
        assert_eq!(resumed.answer, "durable prior partial\r\n");
        assert!(resumed.effort.is_none());
        assert_eq!(
            store
                .begin_turn(
                    running.id,
                    Some(completed.conversation_id),
                    &running.question,
                    &running.provider,
                    &running.model
                )
                .unwrap()
                .status,
            WorkTurnStatus::Interrupted
        );
        assert_eq!(store.setting("synthetic").unwrap().as_deref(), Some("kept"));
        assert_eq!(
            store.unsaved_edit("old.md").unwrap().unwrap().text,
            "protected typing 🦀\r\n"
        );
        let conn = raw(data.path());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            18
        );
        assert_eq!(effort_pair(&conn, completed.id), vec![None, None]);
        assert_eq!(effort_pair(&conn, running.id), vec![None, None]);
        let startup_backup = Connection::open(report.backup).unwrap();
        assert_eq!(effort_pair(&startup_backup, running.id), vec![None, None]);
        assert_eq!(
            startup_backup
                .query_row(
                    "SELECT status FROM messages WHERE turn_id=?1 AND role='assistant'",
                    [running.id.to_string()],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "interrupted"
        );
    }
}
