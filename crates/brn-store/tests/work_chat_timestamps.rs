use brn_store::{WorkStore, work::WorkTurnStatus};
use rusqlite::{Connection, params};
use std::sync::{Arc, Barrier};
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn raw(dir: &std::path::Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}
type StoredTimes = (i64, Option<i64>, Vec<(Option<i64>, Option<i64>)>);
fn times(conn: &Connection, conversation: Uuid, turn: Uuid) -> StoredTimes {
    let (created, active) = conn
        .query_row(
            "SELECT created_at_ms,last_activity_at_ms FROM conversations WHERE id=?1",
            [conversation.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let pairs = conn
        .prepare("SELECT started_at_ms,finished_at_ms FROM messages WHERE turn_id=?1 ORDER BY role")
        .unwrap()
        .query_map([turn.to_string()], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    (created, active, pairs)
}
fn downgrade_v7(conn: &Connection) {
    conn.execute_batch("DROP TABLE findings; ALTER TABLE messages DROP COLUMN started_at_ms; ALTER TABLE messages DROP COLUMN finished_at_ms; ALTER TABLE conversations DROP COLUMN last_activity_at_ms; PRAGMA user_version=7;").unwrap();
}

#[test]
fn fresh_capture_has_equal_pair_times_and_exact_replay_never_refreshes_known_stamps() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    let first = store
        .begin_turn_with_effort(
            id,
            None,
            "Question õäöü\r\n",
            "chatgpt",
            "gpt-5.5",
            Some("high"),
        )
        .unwrap();
    let conversation = &store.conversations().unwrap()[0];
    assert_eq!(first.started_at_ms, Some(conversation.created_at_ms));
    assert_eq!(conversation.last_activity_at_ms, first.started_at_ms);
    assert!(first.finished_at_ms.is_none());
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE conversations SET created_at_ms=1,last_activity_at_ms=20 WHERE id=?1",
        [first.conversation_id.to_string()],
    )
    .unwrap();
    conn.execute(
        "UPDATE messages SET started_at_ms=10 WHERE turn_id=?1",
        [id.to_string()],
    )
    .unwrap();
    let before = times(&conn, first.conversation_id, id);
    let replay = store
        .begin_turn_with_effort(
            id,
            None,
            &first.question,
            &first.provider,
            &first.model,
            Some("high"),
        )
        .unwrap();
    assert_eq!(replay.started_at_ms, Some(10));
    assert_eq!(times(&conn, first.conversation_id, id), before);
    let finished = store
        .finish_turn(id, WorkTurnStatus::Completed, "Exact answer λ\r\n", None)
        .unwrap();
    assert!(finished.finished_at_ms.unwrap() >= finished.started_at_ms.unwrap());
    assert_eq!(
        store.conversations().unwrap()[0].last_activity_at_ms,
        finished.finished_at_ms
    );
    conn.execute(
        "UPDATE conversations SET last_activity_at_ms=30 WHERE id=?1",
        [first.conversation_id.to_string()],
    )
    .unwrap();
    conn.execute(
        "UPDATE messages SET finished_at_ms=30 WHERE turn_id=?1",
        [id.to_string()],
    )
    .unwrap();
    let before = times(&conn, first.conversation_id, id);
    let replay = store
        .finish_turn(id, WorkTurnStatus::Completed, &finished.answer, None)
        .unwrap();
    assert_eq!(replay.finished_at_ms, Some(30));
    assert_eq!(replay.question, first.question);
    assert_eq!(replay.answer, finished.answer);
    assert_eq!(replay.effort.as_deref(), Some("high"));
    assert_eq!(times(&conn, first.conversation_id, id), before);
    let replay = store
        .begin_turn_with_effort(
            id,
            None,
            &first.question,
            &first.provider,
            &first.model,
            Some("high"),
        )
        .unwrap();
    assert_eq!(replay.finished_at_ms, Some(30));
    assert_eq!(times(&conn, first.conversation_id, id), before);
}

#[test]
fn restart_interruption_does_not_fabricate_finish_or_refresh_activity() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = store
        .begin_turn(
            Uuid::new_v4(),
            None,
            "Interrupted õäöü\r\n",
            "copilot",
            "gpt-5.3-codex",
        )
        .unwrap();
    let before = times(&raw(dir.path()), turn.conversation_id, turn.id);
    drop(store);
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    let interrupted = store.turn(turn.id).unwrap().unwrap();
    assert_eq!(interrupted.status, WorkTurnStatus::Interrupted);
    assert_eq!(interrupted.started_at_ms, turn.started_at_ms);
    assert!(interrupted.finished_at_ms.is_none());
    assert_eq!(
        times(&raw(dir.path()), turn.conversation_id, turn.id),
        before
    );
    assert_eq!(
        times(
            &Connection::open(report.backup).unwrap(),
            turn.conversation_id,
            turn.id
        ),
        before
    );
    let replay = store
        .finish_turn(turn.id, WorkTurnStatus::Interrupted, "", None)
        .unwrap();
    assert!(replay.finished_at_ms.is_none());
    assert_eq!(
        times(&raw(dir.path()), turn.conversation_id, turn.id),
        before
    );
}

#[test]
fn fresh_admission_and_finalization_clamp_clock_rollback_against_known_activity_and_turns() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let first = store
        .begin_turn(Uuid::new_v4(), None, "First", "chatgpt", "gpt-5.5")
        .unwrap();
    store
        .finish_turn(first.id, WorkTurnStatus::Completed, "First answer", None)
        .unwrap();
    let future = i64::MAX;
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE messages SET started_at_ms=?2,finished_at_ms=?2 WHERE turn_id=?1",
        params![first.id.to_string(), future],
    )
    .unwrap();
    conn.execute(
        "UPDATE conversations SET last_activity_at_ms=NULL WHERE id=?1",
        [first.conversation_id.to_string()],
    )
    .unwrap();
    let next = store
        .begin_turn(
            Uuid::new_v4(),
            Some(first.conversation_id),
            "Next",
            "chatgpt",
            "gpt-5.5",
        )
        .unwrap();
    assert_eq!(next.started_at_ms, Some(future as u64));
    assert_eq!(
        store.conversations().unwrap()[0].last_activity_at_ms,
        Some(future as u64)
    );
    let completed = store
        .finish_turn(next.id, WorkTurnStatus::Completed, "Next answer", None)
        .unwrap();
    assert_eq!(completed.finished_at_ms, Some(future as u64));
    assert_eq!(
        store.conversations().unwrap()[0].last_activity_at_ms,
        Some(future as u64)
    );
}

#[test]
fn failed_admission_or_finalization_rolls_back_timestamps_and_content_together() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let first = store
        .begin_turn(Uuid::new_v4(), None, "First", "chatgpt", "gpt-5.5")
        .unwrap();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER refuse_activity BEFORE UPDATE ON conversations BEGIN SELECT RAISE(ABORT,'synthetic activity failure'); END;").unwrap();
    let before = times(&conn, first.conversation_id, first.id);
    let next = Uuid::new_v4();
    assert!(
        store
            .begin_turn(
                next,
                Some(first.conversation_id),
                "Refused",
                "chatgpt",
                "gpt-5.5"
            )
            .is_err()
    );
    assert!(store.turn(next).unwrap().is_none());
    assert_eq!(times(&conn, first.conversation_id, first.id), before);
    assert!(
        store
            .finish_turn(first.id, WorkTurnStatus::Completed, "Refused answer", None)
            .is_err()
    );
    let retained = store.turn(first.id).unwrap().unwrap();
    assert_eq!(retained.status, WorkTurnStatus::Running);
    assert_eq!(retained.answer, "");
    assert_eq!(times(&conn, first.conversation_id, first.id), before);
    conn.execute_batch("DROP TRIGGER refuse_activity;").unwrap();
    store
        .finish_turn(first.id, WorkTurnStatus::Completed, "Accepted answer", None)
        .unwrap();
}

#[test]
fn legacy_v7_upgrade_and_restored_backup_keep_unknown_activity_and_turn_times() {
    for restore in [false, true] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let first = store
            .begin_turn_with_effort(
                Uuid::new_v4(),
                None,
                "Old õäöü\r\n",
                "chatgpt",
                "gpt-5.5",
                Some("medium"),
            )
            .unwrap();
        let ended = store
            .finish_turn(first.id, WorkTurnStatus::Completed, "Old exact λ\r\n", None)
            .unwrap();
        let running = store
            .begin_turn(
                Uuid::new_v4(),
                Some(first.conversation_id),
                "Old running",
                "copilot",
                "gpt-5.3-codex",
            )
            .unwrap();
        let created = store.conversations().unwrap()[0].created_at_ms;
        drop(store);
        let conn = raw(dir.path());
        downgrade_v7(&conn);
        let backup = dir.path().join("backups/brn-9999999999999.sqlite");
        if restore {
            conn.backup("main", &backup, None).unwrap();
        }
        drop(conn);
        if restore {
            std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
        }
        let (mut store, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(report.restored_from, restore.then_some(backup));
        let legacy = store.turn(first.id).unwrap().unwrap();
        assert_eq!(legacy.question, ended.question);
        assert_eq!(legacy.answer, ended.answer);
        assert_eq!(legacy.effort, ended.effort);
        assert!(legacy.started_at_ms.is_none());
        assert!(legacy.finished_at_ms.is_none());
        let conversation = &store.conversations().unwrap()[0];
        assert_eq!(conversation.created_at_ms, created);
        assert!(conversation.last_activity_at_ms.is_none());
        let recovered = store.turn(running.id).unwrap().unwrap();
        assert_eq!(recovered.status, WorkTurnStatus::Interrupted);
        assert!(recovered.started_at_ms.is_none());
        assert!(recovered.finished_at_ms.is_none());
        let replay = store
            .finish_turn(first.id, WorkTurnStatus::Completed, &ended.answer, None)
            .unwrap();
        assert!(replay.finished_at_ms.is_none());
        assert!(
            store.conversations().unwrap()[0]
                .last_activity_at_ms
                .is_none()
        );
        let json = serde_json::to_value(&legacy).unwrap();
        assert_eq!(json["started_at_ms"], serde_json::Value::Null);
        assert_eq!(json["finished_at_ms"], serde_json::Value::Null);
        let backup_conn = Connection::open(report.backup).unwrap();
        assert_eq!(times(&backup_conn, first.conversation_id, first.id).1, None);
    }
}

#[test]
fn malformed_pair_or_conversation_times_refuse_reads_and_startup_without_repair() {
    for mutation in [
        "UPDATE messages SET started_at_ms=-1",
        "UPDATE messages SET finished_at_ms=-1",
        "UPDATE messages SET started_at_ms=started_at_ms+1 WHERE role='assistant'",
        "UPDATE messages SET finished_at_ms=started_at_ms",
        "UPDATE messages SET started_at_ms=0",
        "UPDATE conversations SET created_at_ms=-1",
        "UPDATE conversations SET last_activity_at_ms=-1",
        "UPDATE conversations SET last_activity_at_ms=created_at_ms-1",
        "UPDATE messages SET started_at_ms=started_at_ms+1000",
    ] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "Synthetic", "chatgpt", "gpt-5.5")
            .unwrap();
        let conn = raw(dir.path());
        conn.execute_batch(mutation).unwrap();
        assert!(store.turn(turn.id).is_err(), "{mutation}");
        assert!(store.conversations().is_err(), "{mutation}");
        drop(store);
        assert!(WorkStore::open(dir.path()).is_err(), "{mutation}");
        let status: String = conn
            .query_row("SELECT status FROM messages LIMIT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            status, "running",
            "startup must refuse before interruption: {mutation}"
        );
    }
}

#[test]
fn malformed_empty_conversations_are_validated_at_startup() {
    for (created, active) in [(-1, None), (10, Some(-1)), (10, Some(9))] {
        let dir = fixture();
        drop(WorkStore::open(dir.path()).unwrap());
        let conn = raw(dir.path());
        conn.execute("INSERT INTO conversations(id,title,created_at_ms,last_activity_at_ms) VALUES(?1,'empty',?2,?3)",params![Uuid::new_v4().to_string(),created,active]).unwrap();
        assert!(WorkStore::open(dir.path()).is_err(), "{created}/{active:?}");
    }
}

#[test]
fn terminal_order_and_pair_finish_agreement_are_checked_without_requiring_known_legacy_times() {
    for mutation in [
        "UPDATE messages SET finished_at_ms=started_at_ms-1",
        "UPDATE messages SET finished_at_ms=finished_at_ms+1 WHERE role='assistant'",
        "UPDATE messages SET finished_at_ms=finished_at_ms+1000",
    ] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "Synthetic", "chatgpt", "gpt-5.5")
            .unwrap();
        store
            .finish_turn(turn.id, WorkTurnStatus::Completed, "Exact", None)
            .unwrap();
        let conn = raw(dir.path());
        conn.execute_batch(mutation).unwrap();
        assert!(store.turn(turn.id).is_err(), "{mutation}");
        drop(store);
        assert!(WorkStore::open(dir.path()).is_err(), "{mutation}");
    }
}

#[test]
fn attached_and_owner_chat_writers_preserve_monotonic_atomic_pair_times() {
    let dir = fixture();
    let (mut owner, _) = WorkStore::open(dir.path()).unwrap();
    let first = owner
        .begin_turn(Uuid::new_v4(), None, "First", "chatgpt", "gpt-5.5")
        .unwrap();
    owner
        .finish_turn(first.id, WorkTurnStatus::Completed, "First answer", None)
        .unwrap();
    let conversation = first.conversation_id;
    let future = 9_000_000_000_000i64;
    raw(dir.path())
        .execute(
            "UPDATE conversations SET last_activity_at_ms=?2 WHERE id=?1",
            params![conversation.to_string(), future],
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = vec![];
    for index in 0..2 {
        let mut attached = owner.chat_connection().unwrap();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            let id = Uuid::new_v4();
            let turn = attached
                .begin_turn(
                    id,
                    Some(conversation),
                    &format!("Attached {index}"),
                    "chatgpt",
                    "gpt-5.5",
                )
                .unwrap();
            assert_eq!(turn.started_at_ms, Some(future as u64));
            let ended = attached
                .finish_turn(id, WorkTurnStatus::Interrupted, "Exact partial", None)
                .unwrap();
            assert_eq!(ended.finished_at_ms, Some(future as u64));
        }));
    }
    barrier.wait();
    let own = owner
        .begin_turn(
            Uuid::new_v4(),
            Some(conversation),
            "Owner",
            "chatgpt",
            "gpt-5.5",
        )
        .unwrap();
    owner
        .finish_turn(
            own.id,
            WorkTurnStatus::Failed,
            "Exact owner partial",
            Some("network"),
        )
        .unwrap();
    for thread in threads {
        thread.join().unwrap();
    }
    let turns = owner.turns(conversation).unwrap();
    assert_eq!(turns.len(), 4);
    for turn in turns.iter().skip(1) {
        assert_eq!(turn.started_at_ms, Some(future as u64));
        assert_eq!(turn.finished_at_ms, Some(future as u64));
        assert_eq!(
            times(&raw(dir.path()), conversation, turn.id).2,
            vec![(Some(future), Some(future)); 2]
        );
    }
    assert_eq!(
        owner.conversations().unwrap()[0].last_activity_at_ms,
        Some(future as u64)
    );
}

#[test]
fn conversation_projection_uses_one_snapshot_during_attached_admission() {
    let dir = fixture();
    let (mut owner, _) = WorkStore::open(dir.path()).unwrap();
    let first = owner
        .begin_turn(Uuid::new_v4(), None, "First", "chatgpt", "gpt-5.5")
        .unwrap();
    let conversation = first.conversation_id;
    let mut attached = owner.chat_connection().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = barrier.clone();
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut starts = vec![first.started_at_ms.unwrap()];
        for index in 0..300 {
            let turn = attached
                .begin_turn(
                    Uuid::new_v4(),
                    Some(conversation),
                    &format!("Attached {index}"),
                    "chatgpt",
                    "gpt-5.5",
                )
                .unwrap();
            starts.push(turn.started_at_ms.unwrap());
        }
        starts
    });
    barrier.wait();
    let mut samples = Vec::new();
    // Each projection is real Store work. Bound sampling even if a writer
    // fails; join then reports its error instead of spinning on a done flag.
    for _ in 0..1000 {
        samples.push(owner.conversations().unwrap().remove(0));
        if writer.is_finished() {
            break;
        }
    }
    let starts = writer.join().unwrap();
    assert!(!samples.is_empty());
    let final_projection = owner.conversations().unwrap().remove(0);
    assert_eq!(final_projection.turns, 301);
    samples.push(final_projection);
    for sample in samples {
        assert_eq!(sample.id, conversation);
        assert!(sample.turns > 0 && sample.turns <= starts.len());
        assert!(
            sample.last_activity_at_ms.unwrap() >= starts[sample.turns - 1],
            "activity {:?} predates counted turn {} start {}",
            sample.last_activity_at_ms,
            sample.turns,
            starts[sample.turns - 1]
        );
    }
}

#[test]
fn historical_dto_omissions_deserialize_as_explicit_unknown_times() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = store
        .begin_turn(Uuid::new_v4(), None, "Question", "chatgpt", "gpt-5.5")
        .unwrap();
    let mut historical_turn = serde_json::to_value(&turn).unwrap();
    historical_turn
        .as_object_mut()
        .unwrap()
        .remove("started_at_ms");
    historical_turn
        .as_object_mut()
        .unwrap()
        .remove("finished_at_ms");
    let historical: brn_store::work::WorkTurn = serde_json::from_value(historical_turn).unwrap();
    assert!(historical.started_at_ms.is_none());
    assert!(historical.finished_at_ms.is_none());
    assert_eq!(
        serde_json::to_value(&historical).unwrap()["started_at_ms"],
        serde_json::Value::Null
    );
    assert_eq!(
        serde_json::to_value(&historical).unwrap()["finished_at_ms"],
        serde_json::Value::Null
    );
    let conversation = store.conversations().unwrap().remove(0);
    let mut historical_conversation = serde_json::to_value(&conversation).unwrap();
    historical_conversation
        .as_object_mut()
        .unwrap()
        .remove("last_activity_at_ms");
    let historical: brn_store::work::WorkConversation =
        serde_json::from_value(historical_conversation).unwrap();
    assert_eq!(historical.created_at_ms, conversation.created_at_ms);
    assert!(historical.last_activity_at_ms.is_none());
    assert_eq!(
        serde_json::to_value(&historical).unwrap()["last_activity_at_ms"],
        serde_json::Value::Null
    );
}
