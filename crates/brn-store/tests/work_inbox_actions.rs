use brn_store::{
    Error, WorkStore,
    files::FileFingerprint,
    work::{
        WorkTurnStatus,
        actions::{ActionData, ActionState},
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_actions::{InboxActionCapture, InboxActionJob, MAX_INBOX_ACTION_SOURCE_BYTES},
        inbox_processing::{InboxConversionFormat, InboxProcessOutcome, ProcessInboxRequest},
        inbox_source::InboxSourceBinding,
        proposal_rewrite::RewriteSpec,
        proposals::{ActionChange, ProposalDraft, ProposalStamp, SourceVersion},
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn original() -> InboxCapture {
    InboxCapture {
        id: Uuid::new_v4(),
        kind: InboxKind::Email,
        title: "Synthetic mail õ".into(),
        original_name: None,
        copy: InboxCopy {
            directory: "/synthetic/inbox".into(),
            directory_device: 1,
            directory_inode: 2,
            file_device: 1,
            file_inode: 3,
            byte_len: 4,
            sha256: digest(b"body"),
        },
    }
}
fn capture() -> InboxActionCapture {
    let binding = InboxSourceBinding {
        batch_id: Uuid::new_v4(),
        index: 0,
        original: InboxItem {
            capture: original(),
            received_at_ms: 1,
        },
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: 17,
        sha256: digest(b"```text\nbody\n```\n"),
        note_id: Uuid::new_v4(),
    };
    let source_text = binding.markdown("```text\nbody\n```\n").unwrap();
    InboxActionCapture {
        id: Uuid::new_v4(),
        conversation: None,
        source: SourceVersion {
            path: "Sources/exact.md".into(),
            fingerprint: FileFingerprint {
                device: 1,
                inode: 2,
                len: source_text.len() as u64,
                sha256: digest(source_text.as_bytes()),
            },
        },
        source_text,
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    }
}
fn rebind_text(capture: &mut InboxActionCapture) {
    capture.source.fingerprint.len = capture.source_text.len() as u64;
    capture.source.fingerprint.sha256 = digest(capture.source_text.as_bytes());
}
fn review(store: &mut WorkStore) -> ProposalStamp {
    store
        .create_proposal(&ProposalDraft {
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Synthetic candidate".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id: Uuid::new_v4(),
                data: ActionData {
                    title: "Ask for the exact reply".into(),
                    description: String::new(),
                    state: ActionState::Open,
                    owner: None,
                    related_person: None,
                    related_project: None,
                    sources: vec![],
                    thread: None,
                    due_on: None,
                    follow_up_on: None,
                    dependencies: vec![],
                    parent: None,
                    follows_up: None,
                    priority: None,
                },
            }],
        })
        .unwrap()
        .stamp()
}

#[test]
fn complete_source_identity_provenance_selection_and_encoded_bounds_are_checked() {
    let valid = capture();
    valid.validate().unwrap();
    assert!(!valid.note_id().unwrap().is_nil());
    for mode in 0..16 {
        let mut bad = valid.clone();
        match mode {
            0 => bad.id = Uuid::nil(),
            1 => bad.conversation = Some(Uuid::nil()),
            2 => bad.source.path = "../escape.md".into(),
            3 => bad.source.path = ".hidden.md".into(),
            4 => bad.source.fingerprint.len += 1,
            5 => bad.source.fingerprint.sha256[0] ^= 1,
            6 => {
                bad.source_text = bad.source_text.replace(
                    &bad.note_id().unwrap().to_string(),
                    &Uuid::nil().to_string(),
                );
                rebind_text(&mut bad);
            }
            7 => {
                bad.source_text = bad
                    .source_text
                    .replace("brn_kind: source", "brn_kind: knowledge");
                rebind_text(&mut bad);
            }
            8 => {
                bad.source_text = bad
                    .source_text
                    .lines()
                    .filter(|line| !line.starts_with("brn_inbox_source:"))
                    .collect::<Vec<_>>()
                    .join("\n");
                rebind_text(&mut bad);
            }
            9 => {
                bad.source_text = bad
                    .source_text
                    .replace("\"kind\":\"email\"", "\"kind\":\"markdown\"");
                rebind_text(&mut bad);
            }
            10 => bad.provider = "automatic".into(),
            11 => bad.model = "choose the best".into(),
            12 => bad.effort.clear(),
            13 => {
                bad.source_text
                    .push_str(&"x".repeat(MAX_INBOX_ACTION_SOURCE_BYTES));
                rebind_text(&mut bad);
            }
            14 => {
                bad.source_text = bad
                    .source_text
                    .replace("brn_state: current", "brn_state: unknown");
                rebind_text(&mut bad);
            }
            _ => {
                bad.source_text = bad.source_text.replace("brn_id:", "other_id:");
                rebind_text(&mut bad);
            }
        }
        assert!(
            matches!(bad.validate(), Err(Error::Invalid(_))),
            "mode {mode}"
        );
    }
    let mut historic = valid.clone();
    historic.source_text = historic
        .source_text
        .replace("brn_state: current", "brn_state: history");
    rebind_text(&mut historic);
    historic.validate().unwrap();
    let mut boundary = valid.clone();
    boundary
        .source_text
        .push_str(&"x".repeat(MAX_INBOX_ACTION_SOURCE_BYTES - boundary.source_text.len()));
    rebind_text(&mut boundary);
    boundary.validate().unwrap();
    let good = InboxActionJob {
        capture: valid,
        question: "Analyze exact Source".into(),
        created_at_ms: 1,
    };
    good.validate().unwrap();
    for question in [" ".into(), "x".repeat(512 * 1024 + 1), "\0".repeat(200_000)] {
        let mut bad = good.clone();
        bad.question = question;
        assert!(matches!(bad.validate(), Err(Error::Invalid(_))));
    }
    let mut bad = good;
    bad.created_at_ms = i64::MAX as u64 + 1;
    assert!(bad.validate().is_err());
}

#[test]
fn immutable_full_reservation_replay_preserves_time_and_refuses_changed_input() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    let job = store
        .reserve_inbox_action(&capture, "Analyze õ\r\nexact")
        .unwrap();
    assert_eq!(store.inbox_action(capture.id).unwrap(), Some(job.clone()));
    assert_eq!(
        store.reserve_inbox_action(&capture, &job.question).unwrap(),
        job
    );
    assert!(store.turn(capture.id).unwrap().is_none());
    assert!(store.conversations().unwrap().is_empty());
    for mode in 0..8 {
        let mut changed = capture.clone();
        match mode {
            0 => changed.source.path = "Sources/renamed.md".into(),
            1 => changed.source.fingerprint.inode += 1,
            2 => changed.source.fingerprint.device += 1,
            3 => {
                changed.source_text.push('õ');
                rebind_text(&mut changed);
            }
            4 => changed.conversation = Some(Uuid::new_v4()),
            5 => changed.provider = "copilot".into(),
            6 => changed.model = "other-model".into(),
            _ => changed.effort = "high".into(),
        }
        assert!(
            matches!(
                store.reserve_inbox_action(&changed, &job.question),
                Err(Error::OperationConflict(_))
            ),
            "mode {mode}"
        );
    }
    // Replay checks the whole request before fresh invalid-input handling.
    assert!(matches!(
        store.reserve_inbox_action(&capture, " "),
        Err(Error::OperationConflict(_))
    ));
    drop(store);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.reserve_inbox_action(&capture, &job.question).unwrap(),
        job
    );
    assert_eq!(store.inbox_action(capture.id).unwrap(), Some(job));
}

#[test]
fn only_exact_bound_turn_can_use_the_reserved_namespace_and_replay_is_terminal() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    let parent = store
        .begin_turn(Uuid::new_v4(), None, "Earlier", "chatgpt", "gpt-6-luna")
        .unwrap();
    store
        .finish_turn(parent.id, WorkTurnStatus::Completed, "Done", None)
        .unwrap();
    capture.conversation = Some(parent.conversation_id);
    let job = store
        .reserve_inbox_action(&capture, "Analyze this Source only")
        .unwrap();
    assert!(matches!(
        store.begin_turn_with_effort(
            capture.id,
            capture.conversation,
            &job.question,
            &capture.provider,
            &capture.model,
            Some(&capture.effort)
        ),
        Err(Error::OperationConflict(_))
    ));
    let stamp = review(&mut store);
    let spec = RewriteSpec {
        id: capture.id,
        expected: stamp,
        provider: capture.provider.clone(),
        model: capture.model.clone(),
        effort: capture.effort.clone(),
    };
    assert!(matches!(
        store.begin_proposal_rewrite(&spec),
        Err(Error::OperationConflict(_))
    ));
    for mode in 0..3 {
        let mut wrong = job.clone();
        match mode {
            0 => wrong.question.push('x'),
            1 => wrong.created_at_ms += 1,
            _ => wrong.capture.source.fingerprint.inode += 1,
        }
        assert!(matches!(
            store.begin_inbox_action_turn(&wrong),
            Err(Error::OperationConflict(_))
        ));
    }
    let mut chat = store.chat_connection().unwrap();
    let running = chat.begin_inbox_action_turn(&job).unwrap();
    assert_eq!(running.conversation_id, parent.conversation_id);
    assert_eq!(running.question, job.question);
    assert_eq!(running.effort.as_deref(), Some("medium"));
    assert_eq!(
        store.begin_inbox_action_turn(&job).unwrap().started_at_ms,
        running.started_at_ms
    );
    let done = chat
        .finish_turn(
            capture.id,
            WorkTurnStatus::Completed,
            "No certain Action",
            None,
        )
        .unwrap();
    let replay = chat.begin_inbox_action_turn(&job).unwrap();
    assert_eq!(replay.status, WorkTurnStatus::Completed);
    assert_eq!(replay.answer, done.answer);
    assert_eq!(replay.finished_at_ms, done.finished_at_ms);
    assert_eq!(
        store.reserve_inbox_action(&capture, &job.question).unwrap(),
        job
    );
    assert_eq!(store.turns(parent.conversation_id).unwrap().len(), 2);
    let mut collision = capture.clone();
    collision.id = parent.id;
    assert!(matches!(
        store.reserve_inbox_action(&collision, "New analysis"),
        Err(Error::OperationConflict(_))
    ));
    let rewrite_id = Uuid::new_v4();
    store
        .begin_proposal_rewrite(&RewriteSpec {
            id: rewrite_id,
            ..spec
        })
        .unwrap();
    collision.id = rewrite_id;
    assert!(matches!(
        store.reserve_inbox_action(&collision, "New analysis"),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn pending_reservation_survives_restart_while_running_bound_turn_is_interrupted() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let pending = store
        .reserve_inbox_action(&capture(), "Not started")
        .unwrap();
    let running = store.reserve_inbox_action(&capture(), "Started").unwrap();
    store.begin_inbox_action_turn(&running).unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.inbox_action(pending.capture.id).unwrap(),
        Some(pending.clone())
    );
    assert!(store.turn(pending.capture.id).unwrap().is_none());
    assert_eq!(
        store.inbox_action(running.capture.id).unwrap(),
        Some(running.clone())
    );
    let interrupted = store.begin_inbox_action_turn(&running).unwrap();
    assert_eq!(interrupted.status, WorkTurnStatus::Interrupted);
    assert_eq!(interrupted.question, running.question);
    assert_eq!(
        store.begin_inbox_action_turn(&pending).unwrap().status,
        WorkTurnStatus::Running
    );
}

#[test]
fn failed_bound_admission_rolls_back_chat_and_preserves_the_pending_exact_job() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let job = store
        .reserve_inbox_action(&capture(), "Admit once")
        .unwrap();
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    raw.execute_batch(&format!(
        "CREATE TRIGGER abort_bound_assistant BEFORE INSERT ON messages WHEN NEW.turn_id='{}' AND NEW.role='assistant' BEGIN SELECT RAISE(ABORT,'synthetic bound failure'); END;",
        job.capture.id
    )).unwrap();
    let mut chat = store.chat_connection().unwrap();
    assert!(chat.begin_inbox_action_turn(&job).is_err());
    assert!(store.turn(job.capture.id).unwrap().is_none());
    assert!(store.conversations().unwrap().is_empty());
    assert_eq!(
        store.inbox_action(job.capture.id).unwrap(),
        Some(job.clone())
    );
    raw.execute_batch("DROP TRIGGER abort_bound_assistant;")
        .unwrap();
    let started = chat.begin_inbox_action_turn(&job).unwrap();
    assert_eq!(started.status, WorkTurnStatus::Running);
    assert_eq!(store.conversations().unwrap().len(), 1);
}

#[test]
fn readable_rewrite_namespace_collision_refuses_both_reads_and_startup() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let job = store
        .reserve_inbox_action(&capture(), "Reserved only")
        .unwrap();
    let stamp = review(&mut store);
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: stamp,
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    };
    let (mut rewrite, _) = store.begin_proposal_rewrite(&spec).unwrap();
    rewrite.spec.id = job.capture.id;
    let bytes = serde_json::to_vec(&rewrite).unwrap();
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    raw.execute(
        "UPDATE proposal_rewrites SET id=?1,job_json=?2,job_sha256=?3",
        params![job.capture.id.to_string(), bytes, digest(&bytes).as_slice()],
    )
    .unwrap();
    assert!(matches!(
        store.inbox_action(job.capture.id),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.proposal_rewrite(job.capture.id),
        Err(Error::Invalid(_))
    ));
    drop(raw);
    drop(store);
    let bytes = std::fs::read(data.path().join("brn.sqlite")).unwrap();
    let backups = std::fs::read_dir(data.path().join("backups"))
        .unwrap()
        .count();
    assert!(matches!(
        WorkStore::open(data.path()),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        std::fs::read(data.path().join("brn.sqlite")).unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count(),
        backups
    );
}

fn upgrade_v13(restored: bool) {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    store.set_setting("synthetic", "\u{feff}õ\r\nλ").unwrap();
    store
        .put_unsaved_edit("unfinished.md", [3; 32], "exact\r\nÕun")
        .unwrap();
    let unfinished = store.unsaved_edit("unfinished.md").unwrap();
    let item = store.capture_inbox(&original()).unwrap();
    let process = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![item.clone()],
    };
    store.process_inbox(&process).unwrap();
    store.start_inbox_processing(process.id, 0).unwrap();
    let processed = store
        .finish_inbox_processing(
            process.id,
            0,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::LiteralTextV1,
                byte_len: 17,
                sha256: digest(b"```text\nbody\n```\n"),
            },
        )
        .unwrap();
    let turn = store
        .begin_turn_with_effort(
            Uuid::new_v4(),
            None,
            "Exact retained question õ",
            "copilot",
            "synthetic-model",
            Some("high"),
        )
        .unwrap();
    let turn = store
        .finish_turn(
            turn.id,
            WorkTurnStatus::Completed,
            "Exact retained answer\r\nλ",
            None,
        )
        .unwrap();
    drop(store);
    let db = data.path().join("brn.sqlite");
    let raw = Connection::open(&db).unwrap();
    raw.execute_batch("DROP TABLE inbox_actions; PRAGMA user_version=13;")
        .unwrap();
    let backup = data.path().join("backups/brn-9999999999999.sqlite");
    if restored {
        raw.backup("main", &backup, None).unwrap();
    }
    drop(raw);
    let backup_bytes = restored.then(|| std::fs::read(&backup).unwrap());
    if restored {
        std::fs::write(&db, b"synthetic physical V13 damage").unwrap();
    }
    let (mut store, report) = WorkStore::open(data.path()).unwrap();
    assert_eq!(report.restored_from, restored.then_some(backup.clone()));
    assert_eq!(
        store.setting("synthetic").unwrap().as_deref(),
        Some("\u{feff}õ\r\nλ")
    );
    assert_eq!(store.unsaved_edit("unfinished.md").unwrap(), unfinished);
    assert_eq!(store.inbox_item(item.capture.id).unwrap(), Some(item));
    assert_eq!(store.inbox_processing(process.id).unwrap(), Some(processed));
    let saved_turn = store.turn(turn.id).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(saved_turn).unwrap(),
        serde_json::to_value(turn).unwrap()
    );
    let job = store
        .reserve_inbox_action(&capture(), "New V14 work")
        .unwrap();
    assert_eq!(store.inbox_action(job.capture.id).unwrap(), Some(job));
    let raw = Connection::open(&db).unwrap();
    assert_eq!(
        raw.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
    if let Some(bytes) = backup_bytes {
        assert_eq!(std::fs::read(backup).unwrap(), bytes);
    }
}

#[test]
fn additive_v13_upgrade_preserves_inbox_processing_chat_and_unfinished_work() {
    upgrade_v13(false);
}

#[test]
fn validated_v13_backup_upgrades_without_losing_retained_work() {
    upgrade_v13(true);
}

fn write_job(raw: &Connection, job: &InboxActionJob) {
    let bytes = serde_json::to_vec(job).unwrap();
    raw.execute(
        "UPDATE inbox_actions SET record_json=?1,record_sha256=?2",
        params![bytes, digest(&bytes).as_slice()],
    )
    .unwrap();
}

#[test]
fn readable_record_index_schema_and_bound_turn_damage_refuse_without_backup_or_replacement() {
    for mode in 0..15 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let parent = store
            .begin_turn(
                Uuid::new_v4(),
                None,
                "Conversation one",
                "chatgpt",
                "gpt-6-luna",
            )
            .unwrap();
        store
            .finish_turn(parent.id, WorkTurnStatus::Completed, "Done", None)
            .unwrap();
        let other = store
            .begin_turn(
                Uuid::new_v4(),
                None,
                "Conversation two",
                "chatgpt",
                "gpt-6-luna",
            )
            .unwrap();
        store
            .finish_turn(other.id, WorkTurnStatus::Completed, "Done", None)
            .unwrap();
        let mut capture = capture();
        capture.conversation = Some(parent.conversation_id);
        let mut job = store
            .reserve_inbox_action(&capture, "Exact bound question")
            .unwrap();
        store.begin_inbox_action_turn(&job).unwrap();
        store
            .finish_turn(
                capture.id,
                WorkTurnStatus::Completed,
                "No certain Action",
                None,
            )
            .unwrap();
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        match mode {
            0 => {
                raw.execute("UPDATE inbox_actions SET record_sha256=?1", [vec![0u8; 32]])
                    .unwrap();
            }
            1 => {
                job.capture.source.fingerprint.len += 1;
                write_job(&raw, &job);
            }
            2 => {
                job.capture.id = Uuid::new_v4();
                write_job(&raw, &job);
            }
            3 => {
                raw.execute("UPDATE inbox_actions SET created_at_ms=created_at_ms+1", [])
                    .unwrap();
            }
            4 => {
                raw.execute(
                    "UPDATE messages SET text='Forked question' WHERE turn_id=?1 AND role='user'",
                    [capture.id.to_string()],
                )
                .unwrap();
            }
            5 => {
                raw.execute(
                    "UPDATE messages SET provider='copilot' WHERE turn_id=?1",
                    [capture.id.to_string()],
                )
                .unwrap();
            }
            6 => {
                raw.execute(
                    "UPDATE messages SET effort='high' WHERE turn_id=?1",
                    [capture.id.to_string()],
                )
                .unwrap();
            }
            7 => {
                raw.execute(
                    "UPDATE messages SET conversation_id=?1,sequence=2 WHERE turn_id=?2",
                    params![other.conversation_id.to_string(), capture.id.to_string()],
                )
                .unwrap();
            }
            8 => {
                let bytes = serde_json::to_vec_pretty(&job).unwrap();
                raw.execute(
                    "UPDATE inbox_actions SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            9 => {
                let mut value = serde_json::to_value(&job).unwrap();
                value["unexpected"] = true.into();
                let bytes = serde_json::to_vec(&value).unwrap();
                raw.execute(
                    "UPDATE inbox_actions SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            10 => {
                raw.execute("UPDATE inbox_actions SET id=upper(id)", [])
                    .unwrap();
            }
            11 => {
                raw.execute_batch("ALTER TABLE inbox_actions ADD COLUMN status TEXT;")
                    .unwrap();
            }
            12 => {
                raw.execute_batch(
                    "CREATE INDEX unwanted_inbox_action_index ON inbox_actions(created_at_ms);",
                )
                .unwrap();
            }
            13 => {
                raw.execute(
                    "UPDATE inbox_actions SET record_json=?1",
                    [vec![b'x'; 1024 * 1024 + 1]],
                )
                .unwrap();
            }
            _ => {
                raw.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE inbox_actions SET created_at_ms=-1;").unwrap();
            }
        }
        // Current reads must detect their own bindings, as well as startup.
        if mode == 10 {
            // A noncanonical row key is no longer this exact lookup's row;
            // full startup validation below must still refuse the hidden work.
            assert!(store.inbox_action(capture.id).unwrap().is_none());
        } else {
            assert!(store.inbox_action(capture.id).is_err(), "read mode {mode}");
        }
        drop(raw);
        drop(store);
        let bytes = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "startup mode {mode}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            bytes,
            "bytes mode {mode}"
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups,
            "backup mode {mode}"
        );
    }
}

#[test]
fn validated_backup_restores_exact_source_reservations_and_completed_bound_turn() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let pending = store
        .reserve_inbox_action(&capture(), "Prepared only")
        .unwrap();
    let done = store
        .reserve_inbox_action(&capture(), "Analyzed exact Source")
        .unwrap();
    store.begin_inbox_action_turn(&done).unwrap();
    let turn = store
        .finish_turn(
            done.capture.id,
            WorkTurnStatus::Completed,
            "Action candidates remain proposals",
            None,
        )
        .unwrap();
    drop(store);
    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.inbox_action(done.capture.id).unwrap(),
        Some(done.clone())
    );
    drop(store);
    let invalid = data.path().join("backups/brn-9999999999999.sqlite");
    std::fs::copy(&report.backup, &invalid).unwrap();
    let bad = Connection::open(&invalid).unwrap();
    bad.execute("UPDATE inbox_actions SET record_sha256=?1", [vec![0u8; 32]])
        .unwrap();
    drop(bad);
    let invalid_bytes = std::fs::read(&invalid).unwrap();
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(report.backup));
    assert_eq!(
        store.inbox_action(pending.capture.id).unwrap(),
        Some(pending.clone())
    );
    assert!(store.turn(pending.capture.id).unwrap().is_none());
    assert_eq!(
        store.inbox_action(done.capture.id).unwrap(),
        Some(done.clone())
    );
    assert_eq!(
        serde_json::to_value(store.begin_inbox_action_turn(&done).unwrap()).unwrap(),
        serde_json::to_value(turn).unwrap()
    );
    assert_eq!(
        store
            .reserve_inbox_action(&pending.capture, &pending.question)
            .unwrap(),
        pending
    );
    assert_eq!(std::fs::read(invalid).unwrap(), invalid_bytes);
}
