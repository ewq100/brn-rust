use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    note_provenance::{self, VaultCitation},
    work::{
        WorkTurnStatus,
        actions::{ActionData, ActionState},
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_actions::{
            InboxActionCapture, InboxActionJob, InboxAnalysisPurpose, InboxKnowledgeBinding,
            MAX_INBOX_ACTION_SOURCE_BYTES,
        },
        inbox_processing::{InboxConversionFormat, InboxProcessOutcome, ProcessInboxRequest},
        inbox_source::InboxSourceBinding,
        proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest, UndoRequest},
        proposal_rewrite::RewriteSpec,
        proposals::{
            ActionChange, NoteChange, ProposalDraft, ProposalEdit, ProposalStamp, ProposalState,
            SourceVersion,
        },
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[path = "work_inbox_actions/knowledge_capture_recovery.rs"]
mod knowledge_capture_recovery;

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
        visual: None,
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
        purpose: Default::default(),
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
            inbox_knowledge: None,
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
    raw.execute_batch(
        "DROP TABLE inbox_original_operations; DROP TABLE inbox_actions; PRAGMA user_version=13;",
    )
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
        15
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

// This independent old-schema encoder deliberately has no purpose field. It
// witnesses byte compatibility with records written before opt-in knowledge.
#[derive(serde::Serialize)]
struct LegacyCapture<'a> {
    id: Uuid,
    conversation: Option<Uuid>,
    source: &'a SourceVersion,
    source_text: &'a str,
    provider: &'a str,
    model: &'a str,
    effort: &'a str,
}
#[derive(serde::Serialize)]
struct LegacyJob<'a> {
    capture: LegacyCapture<'a>,
    question: &'a str,
    created_at_ms: u64,
}
fn legacy_bytes(job: &InboxActionJob) -> Vec<u8> {
    serde_json::to_vec(&LegacyJob {
        capture: LegacyCapture {
            id: job.capture.id,
            conversation: job.capture.conversation,
            source: &job.capture.source,
            source_text: &job.capture.source_text,
            provider: &job.capture.provider,
            model: &job.capture.model,
            effort: &job.capture.effort,
        },
        question: &job.question,
        created_at_ms: job.created_at_ms,
    })
    .unwrap()
}

#[test]
fn legacy_action_bytes_and_question_survive_bound_chat_shutdown_and_restart() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let job = store
        .reserve_inbox_action(&capture(), "Original Action-only question õ\r\n")
        .unwrap();
    let legacy = legacy_bytes(&job);
    assert_eq!(serde_json::to_vec(&job).unwrap(), legacy);
    assert_eq!(
        serde_json::from_slice::<InboxActionJob>(&legacy).unwrap(),
        job
    );
    assert_eq!(job.capture.purpose, InboxAnalysisPurpose::Actions);
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    let retained: (Vec<u8>, Vec<u8>) = raw
        .query_row(
            "SELECT record_json,record_sha256 FROM inbox_actions WHERE id=?1",
            [job.capture.id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(retained, (legacy.clone(), digest(&legacy).to_vec()));
    drop(raw);
    let mut chat = store.chat_connection().unwrap();
    let started = chat.begin_inbox_action_turn(&job).unwrap();
    assert_eq!(started.status, WorkTurnStatus::Running);
    drop(chat);
    drop(store);

    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.inbox_action(job.capture.id).unwrap(),
        Some(job.clone())
    );
    assert_eq!(
        store
            .reserve_inbox_action(&job.capture, &job.question)
            .unwrap(),
        job
    );
    let interrupted = store.begin_inbox_action_turn(&job).unwrap();
    assert_eq!(interrupted.status, WorkTurnStatus::Interrupted);
    assert_eq!(interrupted.question, job.question);
    assert_eq!(store.turns(started.conversation_id).unwrap().len(), 1);
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    assert_eq!(
        raw.query_row(
            "SELECT record_json FROM inbox_actions WHERE id=?1",
            [job.capture.id.to_string()],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .unwrap(),
        legacy
    );
    assert_eq!(
        raw.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
}

#[test]
fn knowledge_purpose_is_immutable_and_survives_checked_backup_restoration() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let old = store
        .reserve_inbox_action(&capture(), "Action-only saved question")
        .unwrap();
    let mut semantic = capture();
    semantic.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    let knowledge = store
        .reserve_inbox_action(&semantic, "Knowledge and Actions saved question")
        .unwrap();
    for job in [&old, &knowledge] {
        let mut changed = job.capture.clone();
        changed.purpose = if changed.purpose == InboxAnalysisPurpose::Actions {
            InboxAnalysisPurpose::KnowledgeAndActions
        } else {
            InboxAnalysisPurpose::Actions
        };
        assert!(matches!(
            store.reserve_inbox_action(&changed, &job.question),
            Err(Error::OperationConflict(_))
        ));
        assert_eq!(
            store
                .reserve_inbox_action(&job.capture, &job.question)
                .unwrap(),
            *job
        );
        store.begin_inbox_action_turn(job).unwrap();
        store
            .finish_turn(
                job.capture.id,
                WorkTurnStatus::Completed,
                "Drafts only",
                None,
            )
            .unwrap();
    }
    let encoded = serde_json::to_vec(&knowledge).unwrap();
    assert!(
        String::from_utf8(encoded.clone())
            .unwrap()
            .contains("\"purpose\":\"knowledge_and_actions\"")
    );
    assert_eq!(
        serde_json::from_slice::<InboxActionJob>(&encoded).unwrap(),
        knowledge
    );
    drop(store);
    let (store, checked) = WorkStore::open(data.path()).unwrap();
    drop(store);
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    for job in [old, knowledge] {
        assert_eq!(
            store.inbox_action(job.capture.id).unwrap(),
            Some(job.clone())
        );
        assert_eq!(
            store
                .reserve_inbox_action(&job.capture, &job.question)
                .unwrap(),
            job
        );
        let turn = store.begin_inbox_action_turn(&job).unwrap();
        assert_eq!(turn.status, WorkTurnStatus::Completed);
        assert_eq!(turn.question, job.question);
    }
}

#[test]
fn unknown_noncanonical_or_hash_damaged_purpose_refuses_without_backup_or_replacement() {
    for mode in 0..3 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let job = store
            .reserve_inbox_action(&capture(), "Old saved question")
            .unwrap();
        let legacy = String::from_utf8(legacy_bytes(&job)).unwrap();
        let bytes = match mode {
            0 => legacy
                .replacen(
                    "{\"capture\":{",
                    "{\"capture\":{\"purpose\":\"future_unknown\",",
                    1,
                )
                .into_bytes(),
            1 => legacy
                .replacen(
                    "{\"capture\":{",
                    "{\"capture\":{\"purpose\":\"actions\",",
                    1,
                )
                .into_bytes(),
            _ => legacy.into_bytes(),
        };
        if mode == 0 {
            assert!(serde_json::from_slice::<InboxActionJob>(&bytes).is_err());
        } else {
            assert_eq!(
                serde_json::from_slice::<InboxActionJob>(&bytes).unwrap(),
                job
            );
        }
        let hash = if mode == 2 {
            vec![0u8; 32]
        } else {
            digest(&bytes).to_vec()
        };
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        raw.execute(
            "UPDATE inbox_actions SET record_json=?1,record_sha256=?2 WHERE id=?3",
            params![bytes, hash, job.capture.id.to_string()],
        )
        .unwrap();
        assert!(
            store.inbox_action(job.capture.id).is_err(),
            "read mode {mode}"
        );
        drop(raw);
        drop(store);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "mode {mode}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}

fn knowledge_draft(capture: &InboxActionCapture) -> ProposalDraft {
    let note_id = Uuid::new_v4();
    let start_byte = capture.source_text.find("body").unwrap();
    let citation = VaultCitation {
        note_id: capture.note_id().unwrap(),
        sha256: capture.source.fingerprint.sha256,
        start_byte,
        end_byte: start_byte + 4,
        quote: "body".into(),
    };
    let text = note_provenance::write(
        &format!("\u{feff}---\r\nbrn_id: {note_id}\r\nbrn_kind: knowledge\r\nbrn_state: current\r\ncustom: unchanged õ\r\n---\r\n# Candidate\r\nInterpreted summary\r\n"),
        std::slice::from_ref(&citation),
    ).unwrap();
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    ProposalDraft {
        inbox_knowledge: Some(Box::new(InboxKnowledgeBinding {
            analysis_id: capture.id,
            note_id,
            source: capture.source.clone(),
            supersedes: None,
            citations: vec![citation],
        })),
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: Some(capture.id),
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        }),
        title: "Interpret exact saved Source".into(),
        changes: vec![NoteChange::Create {
            path: "knowledge/new.md".into(),
            parent,
            text,
        }],
        sources: vec![capture.source.clone()],
        action_changes: vec![],
    }
}

#[test]
fn knowledge_review_edits_and_rewrite_preserve_identity_classification_and_selected_citations() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Analyze this exact Source")
        .unwrap();
    let draft = knowledge_draft(&capture);
    let created = store.create_proposal(&draft).unwrap();
    let original_text = draft.changes[0].text().unwrap();
    let edited_text = original_text.replace("Interpreted summary", "Human reviewed summary");
    let edited = store
        .edit_proposal(&ProposalEdit {
            expected: created.stamp(),
            title: "Reviewed interpretation".into(),
            texts: vec![Some(edited_text.clone())],
            action_data: vec![],
        })
        .unwrap();
    assert_eq!(edited.version, created.version + 1);
    let rewritten = store
        .rewrite_proposal(&ProposalEdit {
            expected: edited.stamp(),
            title: "Rewritten interpretation".into(),
            texts: vec![Some(
                edited_text.replace("Human reviewed summary", "Rewritten summary"),
            )],
            action_data: vec![],
        })
        .unwrap();
    assert_eq!(rewritten.version, edited.version + 1);
    assert_eq!(rewritten.draft.inbox_knowledge, draft.inbox_knowledge);
    assert_eq!(rewritten.draft.sources, draft.sources);
    assert_eq!(rewritten.draft.group_id, draft.group_id);
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    let retained_text = rewritten.draft.changes[0].text().unwrap();
    assert!(retained_text.starts_with("\u{feff}---\r\n"));
    assert!(retained_text.contains("custom: unchanged õ\r\n"));
    let binding = draft.inbox_knowledge.as_ref().unwrap();
    assert_eq!(
        note_provenance::read(retained_text).unwrap(),
        binding.citations
    );

    let source_field = capture
        .source_text
        .lines()
        .find(|line| line.starts_with("brn_inbox_source:"))
        .unwrap();
    for mode in 0..7 {
        let bad_text = match mode {
            0 => retained_text.replace(&binding.note_id.to_string(), &Uuid::new_v4().to_string()),
            1 => retained_text.replace(&format!("brn_id: {}\r\n", binding.note_id), ""),
            2 => retained_text.replace("brn_kind: knowledge", "brn_kind: source"),
            3 => retained_text.replace("brn_state: current", "brn_state: history"),
            4 => note_provenance::write(retained_text, &[]).unwrap(),
            5 => {
                let mut changed = binding.citations.clone();
                changed[0].quote = "fork".into();
                note_provenance::write(retained_text, &changed).unwrap()
            }
            _ => retained_text.replacen(
                "custom: unchanged õ",
                &format!("{source_field}\r\ncustom: unchanged õ"),
                1,
            ),
        };
        let edit = ProposalEdit {
            expected: rewritten.stamp(),
            title: "Must not take effect".into(),
            texts: vec![Some(bad_text)],
            action_data: vec![],
        };
        assert!(store.edit_proposal(&edit).is_err(), "Edit mode {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
        assert!(
            store.rewrite_proposal(&edit).is_err(),
            "Rewrite mode {mode}"
        );
        assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
    }
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten));
}

#[test]
fn knowledge_ordered_target_proofs_survive_review_replay_restart_and_checked_backup() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    let job = store
        .reserve_inbox_action(&capture, "Interpret with captured saved targets")
        .unwrap();
    let mut draft = knowledge_draft(&capture);
    for (i, path) in [
        "projects/synthetic.md",
        "people/synthetic.md",
        "threads/synthetic.md",
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = format!("Synthetic target {i} õ");
        draft.sources.push(SourceVersion {
            path: path.into(),
            fingerprint: FileFingerprint {
                device: 7,
                inode: 20 + i as u64,
                len: bytes.len() as u64,
                sha256: digest(bytes.as_bytes()),
            },
        });
    }
    let created = store.create_proposal(&draft).unwrap();
    assert_eq!(created.draft.sources, draft.sources);
    assert_eq!(store.proposal(draft.id).unwrap(), Some(created.clone()));
    let edited = store
        .edit_proposal(&ProposalEdit {
            expected: created.stamp(),
            title: "Reviewed targets".into(),
            texts: vec![Some(
                draft.changes[0]
                    .text()
                    .unwrap()
                    .replace("Interpreted summary", "Human reviewed summary"),
            )],
            action_data: vec![],
        })
        .unwrap();
    let rewritten = store
        .rewrite_proposal(&ProposalEdit {
            expected: edited.stamp(),
            title: "Rewritten with original targets".into(),
            texts: vec![Some(
                edited.draft.changes[0]
                    .text()
                    .unwrap()
                    .replace("Human reviewed summary", "Rewritten summary"),
            )],
            action_data: vec![],
        })
        .unwrap();
    assert_eq!(rewritten.version, created.version + 2);
    assert_eq!(rewritten.draft.sources, draft.sources);
    assert_eq!(rewritten.draft.inbox_knowledge, draft.inbox_knowledge);
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    for mode in 0..7 {
        let mut changed = draft.clone();
        match mode {
            0 => changed.sources.swap(1, 2),
            1 => changed.sources[1].path = "projects/renamed.md".into(),
            2 => changed.sources[1].fingerprint.device += 1,
            3 => changed.sources[1].fingerprint.inode += 1,
            4 => changed.sources[1].fingerprint.len += 1,
            5 => changed.sources[1].fingerprint.sha256[0] ^= 1,
            _ => {
                changed.sources.pop();
            }
        }
        assert!(
            matches!(
                store.create_proposal(&changed),
                Err(Error::OperationConflict(_))
            ),
            "immutable replay mode {mode}"
        );
        assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
    }
    drop(store);
    let (mut store, checked) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    assert_eq!(store.inbox_action(capture.id).unwrap(), Some(job.clone()));
    drop(store);
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    assert_eq!(store.inbox_action(capture.id).unwrap(), Some(job));
    assert_eq!(
        Connection::open(data.path().join("brn.sqlite"))
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
}

#[test]
fn knowledge_target_proof_bounds_and_malformed_lists_refuse_before_admission() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Exact Source and targets")
        .unwrap();
    let mut draft = knowledge_draft(&capture);
    for i in 1..64 {
        let mut target = capture.source.clone();
        target.path = format!("targets/{i}.md");
        target.fingerprint.inode += i;
        draft.sources.push(target);
    }
    draft.sources[1].fingerprint.len = 1024 * 1024;
    let valid = store.create_proposal(&draft).unwrap();
    assert_eq!(valid.draft.sources, draft.sources);
    for mode in 0..12 {
        let mut bad = draft.clone();
        bad.id = Uuid::new_v4();
        match mode {
            0 => bad.sources.clear(),
            1 => bad.sources.swap(0, 1),
            2 => bad.sources[0].fingerprint.inode += 1,
            3 => bad.sources[1] = bad.sources[0].clone(),
            4 => {
                bad.sources[1] = bad.sources[0].clone();
                bad.sources[1].path = bad.sources[1].path.to_ascii_uppercase();
            }
            5 => {
                bad.sources[2] = bad.sources[1].clone();
                bad.sources[2].path = bad.sources[2].path.to_ascii_uppercase();
            }
            6 => bad.sources[1].path = ".hidden/target.md".into(),
            7 => bad.sources[1].path = "targets/../target.md".into(),
            8 => bad.sources[1].path = "/targets/target.md".into(),
            9 => bad.sources[1].path = "targets/target.txt".into(),
            10 => bad.sources[1].fingerprint.len += 1,
            _ => {
                let mut extra = capture.source.clone();
                extra.path = "targets/65.md".into();
                bad.sources.push(extra);
            }
        }
        assert!(
            matches!(store.create_proposal(&bad), Err(Error::Invalid(_))),
            "source-list mode {mode}"
        );
        assert!(store.proposal(bad.id).unwrap().is_none(), "row mode {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(valid.clone()));
    }
    assert_eq!(store.proposals(Some(capture.id)).unwrap(), vec![valid]);
}

#[test]
fn knowledge_hash_valid_target_list_damage_refuses_read_and_startup_without_backup() {
    for mode in 0..5 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let mut capture = capture();
        capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
        store
            .reserve_inbox_action(&capture, "Retain selected Source")
            .unwrap();
        let mut draft = knowledge_draft(&capture);
        let mut target = capture.source.clone();
        target.path = "targets/synthetic.md".into();
        target.fingerprint.inode += 1;
        draft.sources.push(target);
        let created = store.create_proposal(&draft).unwrap();
        store
            .edit_proposal(&ProposalEdit {
                expected: created.stamp(),
                title: "Newer review retains original ordered targets".into(),
                texts: vec![Some(draft.changes[0].text().unwrap().to_owned())],
                action_data: vec![],
            })
            .unwrap();
        let mut damaged_sources = draft.sources.clone();
        match mode {
            0 => damaged_sources.swap(0, 1),
            1 => {
                damaged_sources[1] = damaged_sources[0].clone();
                damaged_sources[1].path = damaged_sources[1].path.to_ascii_uppercase();
            }
            2 => damaged_sources[1].fingerprint.len = 1024 * 1024 + 1,
            3 => {
                damaged_sources.remove(0);
            }
            _ => damaged_sources[1].path = "targets/.hidden.md".into(),
        }
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        let bytes: Vec<u8> = raw
            .query_row(
                "SELECT record_json FROM proposals WHERE id=?1",
                [draft.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        json["record"]["draft"]["sources"] = serde_json::to_value(damaged_sources).unwrap();
        let changed = serde_json::to_vec(&json).unwrap();
        raw.execute(
            "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
            params![draft.id.to_string(), changed, digest(&changed).as_slice()],
        )
        .unwrap();
        assert!(
            matches!(store.proposal(draft.id), Err(Error::Invalid(_))),
            "read mode {mode}"
        );
        drop(raw);
        drop(store);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "startup mode {mode}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}

#[test]
fn knowledge_create_refuses_mixed_members_or_changed_typed_binding_before_effects() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Analyze this exact Source")
        .unwrap();
    let draft = knowledge_draft(&capture);
    let valid = store.create_proposal(&draft).unwrap();
    for mode in 0..16 {
        let mut bad = draft.clone();
        bad.id = Uuid::new_v4();
        match mode {
            0 => bad.group_id = Some(Uuid::new_v4()),
            1 => bad.sources.clear(),
            2 => bad.sources[0].fingerprint.sha256[0] ^= 1,
            3 => bad.changes.push(NoteChange::Create {
                path: "knowledge/another.md".into(),
                parent: bad.vault.as_ref().unwrap().identity.clone(),
                text: "Additional member".into(),
            }),
            4 => {
                let action_stamp = review(&mut store);
                bad.action_changes = store
                    .proposal(action_stamp.id)
                    .unwrap()
                    .unwrap()
                    .draft
                    .action_changes;
            }
            5 => {
                bad.inbox_source = Some(Box::new(InboxSourceBinding {
                    visual: None,
                    batch_id: Uuid::new_v4(),
                    index: 0,
                    original: InboxItem {
                        capture: original(),
                        received_at_ms: 1,
                    },
                    format: InboxConversionFormat::LiteralTextV1,
                    byte_len: 17,
                    sha256: digest(b"body"),
                    note_id: Uuid::new_v4(),
                }));
            }
            6 => bad.inbox_knowledge.as_mut().unwrap().analysis_id = Uuid::nil(),
            7 => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                let old_id = binding.note_id;
                binding.note_id = binding.citations[0].note_id;
                if let NoteChange::Create { text, .. } = &mut bad.changes[0] {
                    *text = text.replace(&old_id.to_string(), &binding.note_id.to_string());
                }
            }
            8 => bad.inbox_knowledge.as_mut().unwrap().citations.clear(),
            9 => bad.inbox_knowledge.as_mut().unwrap().citations[0].sha256[0] ^= 1,
            10 => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                let quote_len = binding.citations[0].quote.len();
                binding.citations[0].start_byte = binding.source.fingerprint.len as usize;
                binding.citations[0].end_byte = binding.citations[0].start_byte + quote_len;
            }
            11 => {
                let absent = Uuid::new_v4();
                bad.group_id = Some(absent);
                bad.inbox_knowledge.as_mut().unwrap().analysis_id = absent;
            }
            12 => {
                let mut old_capture = capture.clone();
                old_capture.id = Uuid::new_v4();
                old_capture.purpose = InboxAnalysisPurpose::Actions;
                let old_job = store
                    .reserve_inbox_action(&old_capture, "Actions only")
                    .unwrap();
                bad = knowledge_draft(&old_job.capture);
            }
            13 => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                binding.source.fingerprint.sha256[0] ^= 1;
                binding.citations[0].sha256 = binding.source.fingerprint.sha256;
                bad.sources = vec![binding.source.clone()];
                if let NoteChange::Create { text, .. } = &mut bad.changes[0] {
                    *text = note_provenance::write(text, &binding.citations).unwrap();
                }
            }
            14 => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                binding.citations[0].note_id = Uuid::new_v4();
                if let NoteChange::Create { text, .. } = &mut bad.changes[0] {
                    *text = note_provenance::write(text, &binding.citations).unwrap();
                }
            }
            _ => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                binding.citations[0].quote = "fork".into();
                if let NoteChange::Create { text, .. } = &mut bad.changes[0] {
                    *text = note_provenance::write(text, &binding.citations).unwrap();
                }
            }
        }
        assert!(
            matches!(store.create_proposal(&bad), Err(Error::Invalid(_))),
            "mode {mode}"
        );
        assert!(store.proposal(bad.id).unwrap().is_none(), "row mode {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(valid.clone()));
    }
}

#[test]
fn knowledge_proposal_reads_and_startup_refuse_changed_or_missing_retained_capture() {
    for mode in 0..4 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let mut source_capture = capture();
        source_capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
        let mut job = store
            .reserve_inbox_action(&source_capture, "Analyze exact Source")
            .unwrap();
        let draft = knowledge_draft(&source_capture);
        store.create_proposal(&draft).unwrap();
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        match mode {
            0 => {
                raw.execute(
                    "DELETE FROM inbox_actions WHERE id=?1",
                    [source_capture.id.to_string()],
                )
                .unwrap();
            }
            1 => {
                job.capture.purpose = InboxAnalysisPurpose::Actions;
                write_job(&raw, &job);
            }
            2 => {
                job.capture.source.path = "Sources/other.md".into();
                write_job(&raw, &job);
            }
            _ => {
                job.capture.source_text = job.capture.source_text.replace("body", "fork");
                rebind_text(&mut job.capture);
                write_job(&raw, &job);
            }
        }
        assert!(store.proposal(draft.id).is_err(), "read mode {mode}");
        drop(raw);
        drop(store);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "startup mode {mode}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}

#[test]
fn knowledge_binding_uses_existing_apply_and_exact_create_undo_journals() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Analyze this exact Source")
        .unwrap();
    let draft = knowledge_draft(&capture);
    let reviewed = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: reviewed.stamp(),
    };
    let applying = store.begin_proposal_apply(&request).unwrap();
    assert_eq!(
        applying.approved.draft.inbox_knowledge,
        draft.inbox_knowledge
    );
    let text = draft.changes[0].text().unwrap();
    let installed = FileFingerprint {
        device: 1,
        inode: 100,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    };
    store
        .record_proposal_prepared(request.operation_id, std::slice::from_ref(&installed))
        .unwrap();
    let receipt = store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(installed.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    let applied = store.proposal_apply(request.operation_id).unwrap().unwrap();
    assert_eq!(
        applied.approved.draft.inbox_knowledge,
        draft.inbox_knowledge
    );
    assert_eq!(
        store.proposal(draft.id).unwrap().unwrap().state,
        ProposalState::Applied
    );
    assert_eq!(store.begin_proposal_apply(&request).unwrap(), applied);

    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: request.operation_id,
        trash_member: None,
    };
    let preview = store.preview_proposal_undo(&undo).unwrap();
    assert!(preview.draft.inbox_knowledge.is_none());
    assert!(preview.draft.inbox_source.is_none());
    assert_eq!(
        preview.draft.changes,
        vec![NoteChange::Trash {
            path: draft.changes[0].path().into(),
            parent: draft.vault.as_ref().unwrap().identity.clone(),
            before: installed.clone(),
            before_text: text.into(),
        }]
    );
    store.begin_proposal_undo(&undo).unwrap();
    store
        .record_proposal_prepared(undo.operation_id, std::slice::from_ref(&installed))
        .unwrap();
    let undo_receipt = store
        .finish_proposal_apply(
            undo.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: None,
                staging: Some(installed),
            }]),
        )
        .unwrap();
    assert_eq!(
        store.begin_proposal_undo(&undo).unwrap().receipt,
        Some(undo_receipt.clone())
    );
    // These Store witnesses settle supplied proofs only; actual vault/source
    // qualification and filesystem effects remain the workflow's responsibility.
    drop(store);
    let (store, checked) = WorkStore::open(data.path()).unwrap();
    let retained = store.proposal_apply(request.operation_id).unwrap().unwrap();
    assert_eq!(
        retained.approved.draft.inbox_knowledge,
        draft.inbox_knowledge
    );
    assert_eq!(retained.receipt, Some(receipt.clone()));
    drop(store);
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    assert_eq!(
        store.inbox_action(capture.id).unwrap().unwrap().capture,
        capture
    );
    let retained = store.proposal_apply(request.operation_id).unwrap().unwrap();
    assert_eq!(
        retained.approved.draft.inbox_knowledge,
        draft.inbox_knowledge
    );
    assert_eq!(retained.receipt, Some(receipt));
    assert_eq!(
        store.begin_proposal_undo(&undo).unwrap().receipt,
        Some(undo_receipt)
    );
}

fn supersession_draft(capture: &InboxActionCapture) -> ProposalDraft {
    let mut draft = knowledge_draft(capture);
    let old_id = Uuid::new_v4();
    let before_text = format!(
        "\u{feff}---\r\nbrn_id: {old_id}\r\nbrn_state: 'current'\t# exact comment\r\ncustom: λ\r\nbrn_provenance: []\r\n---\r\nPrevious wording 日本語 🦀\r\n"
    );
    let before = FileFingerprint {
        device: 1,
        inode: 41,
        len: before_text.len() as u64,
        sha256: digest(before_text.as_bytes()),
    };
    let source = SourceVersion {
        path: "knowledge/previous.md".into(),
        fingerprint: before.clone(),
    };
    let binding = draft.inbox_knowledge.as_mut().unwrap();
    let mut encoded = serde_json::to_value(&**binding).unwrap();
    encoded["supersedes"] = serde_json::json!({"note_id": old_id, "source": source});
    **binding = serde_json::from_value(encoded).unwrap();
    if let NoteChange::Create { text, .. } = &mut draft.changes[0] {
        text.push_str(&format!(
            "\n\nPrevious version: [History](brn://note/{old_id})\n"
        ));
    }
    draft.sources.push(source);
    draft.changes.push(NoteChange::Replace {
        path: "knowledge/previous.md".into(),
        parent: VaultIdentity {
            device: 1,
            inode: 1,
        },
        before,
        before_text: before_text.clone(),
        text: brn_store::note_metadata::to_history(&before_text).unwrap(),
    });
    draft
}

#[test]
fn supersession_review_and_rewrite_allow_current_edits_but_protect_exact_history_and_footer() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Review likely supersession")
        .unwrap();
    let draft = supersession_draft(&capture);
    let created = store.create_proposal(&draft).unwrap();
    let mut edit = ProposalEdit {
        expected: created.stamp(),
        title: "Reviewed supersession".into(),
        texts: draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
        action_data: vec![],
    };
    edit.texts[0] = Some(
        edit.texts[0]
            .as_ref()
            .unwrap()
            .replace("Interpreted summary", "Reviewed current wording"),
    );
    let reviewed = store.edit_proposal(&edit).unwrap();
    edit.expected = reviewed.stamp();
    edit.texts[0] = Some(
        edit.texts[0]
            .as_ref()
            .unwrap()
            .replace("Reviewed current wording", "Rewritten current wording"),
    );
    let rewritten = store.rewrite_proposal(&edit).unwrap();
    assert_eq!(rewritten.draft.changes[1], draft.changes[1]);
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    edit.expected = rewritten.stamp();
    for mode in 0..4 {
        let mut bad = edit.clone();
        match mode {
            0 => {
                bad.texts[1] = Some(
                    bad.texts[1]
                        .as_ref()
                        .unwrap()
                        .replace("Previous wording", "Rewritten history"),
                )
            }
            1 => {
                bad.texts[1] = Some(
                    bad.texts[1]
                        .as_ref()
                        .unwrap()
                        .replace("'history'", "'current'"),
                )
            }
            2 => {
                bad.texts[0] = Some(
                    bad.texts[0]
                        .as_ref()
                        .unwrap()
                        .replace("Previous version: [History]", "Previous version: [Changed]"),
                )
            }
            _ => bad.texts[0]
                .as_mut()
                .unwrap()
                .push_str("After required footer"),
        }
        assert!(store.edit_proposal(&bad).is_err(), "edit {mode}");
        assert!(store.rewrite_proposal(&bad).is_err(), "rewrite {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(rewritten.clone()));
    }
    drop(store);
    let (mut store, checked) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
    drop(store);
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    assert_eq!(store.create_proposal(&draft).unwrap(), rewritten);
}

#[test]
fn supersession_binding_refuses_inexact_predecessors_shapes_sources_and_identities() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Bound supersession")
        .unwrap();
    let draft = supersession_draft(&capture);
    let valid = store.create_proposal(&draft).unwrap();
    for mode in 0..17 {
        let mut bad = draft.clone();
        bad.id = Uuid::new_v4();
        match mode {
            0 => {
                bad.inbox_knowledge
                    .as_mut()
                    .unwrap()
                    .supersedes
                    .as_mut()
                    .unwrap()
                    .note_id = Uuid::nil()
            }
            1 => {
                let binding = bad.inbox_knowledge.as_mut().unwrap();
                binding.supersedes.as_mut().unwrap().note_id = binding.note_id;
            }
            2 => {
                bad.inbox_knowledge
                    .as_mut()
                    .unwrap()
                    .supersedes
                    .as_mut()
                    .unwrap()
                    .note_id = capture.note_id().unwrap()
            }
            3 => {
                bad.inbox_knowledge
                    .as_mut()
                    .unwrap()
                    .supersedes
                    .as_mut()
                    .unwrap()
                    .source
                    .path = capture.source.path.to_ascii_uppercase()
            }
            4 => {
                bad.inbox_knowledge
                    .as_mut()
                    .unwrap()
                    .supersedes
                    .as_mut()
                    .unwrap()
                    .source
                    .fingerprint
                    .len = 1024 * 1024 + 1
            }
            5 => bad.sources.swap(0, 1),
            6 => {
                bad.sources.remove(1);
            }
            7 => bad.sources[1].fingerprint.inode += 1,
            8 => {
                bad.changes.pop();
            }
            9 => bad.changes.swap(0, 1),
            10 => bad.inbox_knowledge.as_mut().unwrap().supersedes = None,
            11 => {
                if let NoteChange::Replace { path, .. } = &mut bad.changes[1] {
                    *path = path.to_ascii_uppercase();
                }
            }
            12 => {
                if let NoteChange::Replace { before, .. } = &mut bad.changes[1] {
                    before.inode += 1;
                }
            }
            13 => {
                if let NoteChange::Replace { before_text, .. } = &mut bad.changes[1] {
                    before_text.push('x');
                }
            }
            14 => {
                if let NoteChange::Replace { text, .. } = &mut bad.changes[1] {
                    *text = text.replace("Previous wording", "Lost wording");
                }
            }
            15 => {
                if let NoteChange::Create { text, .. } = &mut bad.changes[0] {
                    *text =
                        text.replace("Previous version: [History]", "Previous version: [Changed]");
                }
            }
            _ => {
                if let NoteChange::Replace {
                    before,
                    before_text,
                    text,
                    ..
                } = &mut bad.changes[1]
                {
                    *before_text = before_text.replace("'current'", "'history'");
                    *before = FileFingerprint {
                        device: before.device,
                        inode: before.inode,
                        len: before_text.len() as u64,
                        sha256: digest(before_text.as_bytes()),
                    };
                    *text = before_text.clone();
                    bad.sources[1].fingerprint = before.clone();
                    bad.inbox_knowledge
                        .as_mut()
                        .unwrap()
                        .supersedes
                        .as_mut()
                        .unwrap()
                        .source
                        .fingerprint = before.clone();
                }
            }
        }
        assert!(
            matches!(store.create_proposal(&bad), Err(Error::Invalid(_))),
            "mode {mode}"
        );
        assert!(store.proposal(bad.id).unwrap().is_none(), "new row {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(valid.clone()));
    }
}

#[test]
fn legacy_knowledge_binding_omits_supersedes_canonically_and_keeps_original_replay_hash() {
    #[derive(serde::Serialize)]
    struct LegacyBinding<'a> {
        analysis_id: Uuid,
        note_id: Uuid,
        source: &'a SourceVersion,
        citations: &'a [VaultCitation],
    }
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&capture, "Legacy Create")
        .unwrap();
    let draft = knowledge_draft(&capture);
    let binding = draft.inbox_knowledge.as_ref().unwrap();
    let original = serde_json::to_vec(&LegacyBinding {
        analysis_id: binding.analysis_id,
        note_id: binding.note_id,
        source: &binding.source,
        citations: &binding.citations,
    })
    .unwrap();
    assert_eq!(serde_json::to_vec(&**binding).unwrap(), original);
    let decoded: InboxKnowledgeBinding = serde_json::from_slice(&original).unwrap();
    assert_eq!(&decoded, &**binding);
    let created = store.create_proposal(&draft).unwrap();
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    let before_hash: Vec<u8> = raw
        .query_row(
            "SELECT creation_sha256 FROM proposals WHERE id=?1",
            [draft.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(before_hash, digest(&serde_json::to_vec(&draft).unwrap()));
    drop(raw);
    let edit = ProposalEdit {
        expected: created.stamp(),
        title: "Newer legacy review".into(),
        texts: vec![Some(
            draft.changes[0]
                .text()
                .unwrap()
                .replace("Interpreted summary", "Reviewed later"),
        )],
        action_data: vec![],
    };
    let edited = store.edit_proposal(&edit).unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.create_proposal(&draft).unwrap(), edited);
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    let after_hash: Vec<u8> = raw
        .query_row(
            "SELECT creation_sha256 FROM proposals WHERE id=?1",
            [draft.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after_hash, before_hash);
}

#[test]
fn supersession_hash_valid_semantic_and_owned_schema_damage_refuse_without_restoring_or_backup() {
    for mode in 0..6 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let mut capture = capture();
        capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
        store
            .reserve_inbox_action(&capture, "Exact supersession")
            .unwrap();
        let draft = supersession_draft(&capture);
        let created = store.create_proposal(&draft).unwrap();
        store
            .edit_proposal(&ProposalEdit {
                expected: created.stamp(),
                title: "Retained reviewed pair".into(),
                texts: draft
                    .changes
                    .iter()
                    .map(|change| change.text().map(str::to_owned))
                    .collect(),
                action_data: vec![],
            })
            .unwrap();
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        if mode == 5 {
            raw.execute_batch("CREATE TRIGGER inbox_actions_unexpected AFTER INSERT ON inbox_actions BEGIN SELECT 1; END;").unwrap();
        } else {
            let bytes: Vec<u8> = raw
                .query_row(
                    "SELECT record_json FROM proposals WHERE id=?1",
                    [draft.id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            match mode {
                0 => {
                    json["record"]["draft"]["inbox_knowledge"]["supersedes"]["note_id"] =
                        serde_json::json!(Uuid::new_v4())
                }
                1 => {
                    json["record"]["draft"]["changes"][1]["text"] =
                        serde_json::json!("Lost history")
                }
                2 => {
                    json["record"]["draft"]["sources"][1]["fingerprint"]["inode"] =
                        serde_json::json!(999)
                }
                3 => {
                    json["record"]["draft"]["inbox_knowledge"]["supersedes"]["unknown"] =
                        serde_json::json!(true)
                }
                _ => {
                    json["record"]["draft"]["inbox_knowledge"]["supersedes"] =
                        serde_json::Value::Null
                }
            }
            let changed = serde_json::to_vec(&json).unwrap();
            raw.execute(
                "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
                params![draft.id.to_string(), changed, digest(&changed).as_slice()],
            )
            .unwrap();
            assert!(
                matches!(store.proposal(draft.id), Err(Error::Invalid(_))),
                "read mode {mode}"
            );
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
            "backups mode {mode}"
        );
        assert!(
            !std::fs::read_dir(data.path()).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("corrupt")),
            "restore mode {mode}"
        );
    }
}

fn review_capture(store: &mut WorkStore) -> (InboxItem, InboxActionCapture) {
    let original = store.capture_inbox(&original()).unwrap();
    let mut capture = capture();
    let binding = InboxSourceBinding {
        visual: None,
        batch_id: Uuid::new_v4(),
        index: 0,
        original: original.clone(),
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: 17,
        sha256: digest(b"```text\nbody\n```\n"),
        note_id: capture.note_id().unwrap(),
    };
    capture.source_text = binding.markdown("```text\nbody\n```\n").unwrap();
    capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    rebind_text(&mut capture);
    (original, capture)
}

#[test]
fn original_review_is_complete_across_processing_analyses_manual_work_and_rejection() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    let empty = store.inbox_review_manifest(original.capture.id).unwrap();
    let empty_digest = empty.digest().unwrap();
    assert!(empty.analyses.is_empty() && empty.processing.is_empty());
    assert_eq!(empty.digest().unwrap(), empty_digest);
    let processing = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![original.clone()],
    };
    store.process_inbox(&processing).unwrap();
    store
        .reserve_inbox_action(&capture, "Interpret the exact saved Source")
        .unwrap();
    // Another analysis of the same original with a different retained Source.
    let mut later = capture.clone();
    later.id = Uuid::new_v4();
    later.source.path = "Sources/second.md".into();
    store
        .reserve_inbox_action(&later, "Inspect the second Source")
        .unwrap();
    let unrelated = crate::capture();
    store
        .reserve_inbox_action(&unrelated, "Unrelated original")
        .unwrap();
    let initial = store.create_proposal(&knowledge_draft(&capture)).unwrap();
    let creation = digest(&serde_json::to_vec(&initial.draft).unwrap());
    let mut texts: Vec<Option<String>> = initial
        .draft
        .changes
        .iter()
        .map(|c| c.text().map(str::to_owned))
        .collect();
    texts[0]
        .as_mut()
        .unwrap()
        .push_str("\nNewer review wording\n");
    let edited = store
        .edit_proposal(&ProposalEdit {
            expected: initial.stamp(),
            title: initial.draft.title.clone(),
            texts,
            action_data: vec![],
        })
        .unwrap();
    let rejected = store.reject_proposal(edited.stamp()).unwrap();
    let mut manual = knowledge_draft(&later);
    manual.inbox_knowledge = None;
    manual.group_id = None;
    let manual = store.create_proposal(&manual).unwrap();
    // A genuinely unrelated Action proposal is not swept into this review.
    review(&mut store);
    let snapshot = store.inbox_review_manifest(original.capture.id).unwrap();
    assert_eq!(snapshot.original, original);
    assert_eq!(snapshot.processing.len(), 1);
    assert_eq!(snapshot.analyses.len(), 2);
    assert_eq!(snapshot.proposals.len(), 2);
    assert!(
        snapshot
            .analyses
            .iter()
            .all(|analysis| analysis.turn.is_none())
    );
    let saved = snapshot
        .proposals
        .iter()
        .find(|p| p.record.draft.id == rejected.draft.id)
        .unwrap();
    assert_eq!(saved.record, rejected);
    assert_eq!(saved.creation_sha256, creation);
    assert_ne!(
        saved.creation_sha256,
        digest(&serde_json::to_vec(&rejected.draft).unwrap())
    );
    assert!(snapshot.proposals.iter().any(|p| p.record == manual));
    assert_ne!(snapshot.digest().unwrap(), empty_digest);
    assert_eq!(
        snapshot.digest().unwrap(),
        store
            .inbox_review_manifest(original.capture.id)
            .unwrap()
            .digest()
            .unwrap()
    );
}

#[test]
fn original_review_identity_changes_for_turn_progress_late_work_and_apply_intent() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    let job = store
        .reserve_inbox_action(&capture, "Inspect saved evidence")
        .unwrap();
    let reserved = store
        .inbox_review_manifest(original.capture.id)
        .unwrap()
        .digest()
        .unwrap();
    store.begin_inbox_action_turn(&job).unwrap();
    let running = store.inbox_review_manifest(original.capture.id).unwrap();
    assert_eq!(
        running.analyses[0].turn.as_ref().unwrap().status,
        WorkTurnStatus::Running
    );
    assert_ne!(reserved, running.digest().unwrap());
    store
        .finish_turn(
            job.capture.id,
            WorkTurnStatus::Failed,
            "Partial interpretation",
            Some("other"),
        )
        .unwrap();
    let failed = store.inbox_review_manifest(original.capture.id).unwrap();
    assert_eq!(
        failed.analyses[0].turn.as_ref().unwrap().answer,
        "Partial interpretation"
    );
    assert_ne!(running.digest().unwrap(), failed.digest().unwrap());
    let proposal = store.create_proposal(&knowledge_draft(&capture)).unwrap();
    let draft = store.inbox_review_manifest(original.capture.id).unwrap();
    assert_ne!(failed.digest().unwrap(), draft.digest().unwrap());
    let journal = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        })
        .unwrap();
    let applying = store.inbox_review_manifest(original.capture.id).unwrap();
    assert_eq!(applying.approvals, vec![journal]);
    assert_eq!(applying.proposals[0].record.state, ProposalState::Applying);
    assert_ne!(applying.digest().unwrap(), draft.digest().unwrap());
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
}

#[test]
fn original_review_rejects_forked_provenance_and_malformed_members_without_clipping() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, mut capture) = review_capture(&mut store);
    let mut provenance = brn_store::work::inbox_source::read_provenance(&capture.source_text)
        .unwrap()
        .unwrap();
    let encoded = serde_json::to_string(&provenance).unwrap();
    provenance.title.push_str(" different capture");
    capture.source_text = capture
        .source_text
        .replace(&encoded, &serde_json::to_string(&provenance).unwrap());
    rebind_text(&mut capture);
    store
        .reserve_inbox_action(&capture, "Forked evidence remains visible")
        .unwrap();
    assert!(matches!(
        store.inbox_review_manifest(original.capture.id),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
    assert!(store.inbox_review_manifest(Uuid::nil()).is_err());
    assert!(matches!(
        store.inbox_review_manifest(Uuid::new_v4()),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn original_review_whole_encoded_bound_refuses_escaped_complete_members() {
    use brn_store::work::inbox_review::{InboxAnalysisReview, MAX_INBOX_REVIEW_BYTES};
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    let mut manifest = store.inbox_review_manifest(original.capture.id).unwrap();
    // Every retained job is individually valid; escaping makes the complete set
    // exceed its bound even though its raw text is considerably smaller.
    let question = "\0".repeat(150_000);
    let mut encoded = 0;
    while encoded <= MAX_INBOX_REVIEW_BYTES {
        let mut capture = capture.clone();
        capture.id = Uuid::new_v4();
        let job = InboxActionJob {
            capture,
            question: question.clone(),
            created_at_ms: original.received_at_ms,
        };
        job.validate().unwrap();
        encoded += serde_json::to_vec(&job).unwrap().len();
        manifest
            .analyses
            .push(InboxAnalysisReview { job, turn: None });
    }
    assert!(manifest.digest().is_err());
    assert!(
        store
            .inbox_review_manifest(original.capture.id)
            .unwrap()
            .analyses
            .is_empty()
    );
}

#[test]
fn original_review_refuses_a_hash_valid_malformed_related_record() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Retained source")
        .unwrap();
    let malformed = b"{}";
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    raw.execute(
        "UPDATE inbox_actions SET record_json=?1,record_sha256=?2 WHERE id=?3",
        params![
            malformed.as_slice(),
            digest(malformed).as_slice(),
            capture.id.to_string()
        ],
    )
    .unwrap();
    assert!(matches!(
        store.inbox_review_manifest(original.capture.id),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
}

#[test]
fn original_review_includes_manual_action_unlinks_from_before_and_origin() {
    use brn_store::work::actions::{ActionOrigin, ActionRecord};
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Inspect Source")
        .unwrap();
    let template_stamp = review(&mut store);
    let template = store.proposal(template_stamp.id).unwrap().unwrap().draft;
    for origin_only in [false, true] {
        let mut linked = template.action_changes[0].data().clone();
        linked.sources = vec![capture.note_id().unwrap()];
        let mut current = linked.clone();
        current.sources.clear();
        let before = ActionRecord {
            origin: ActionOrigin {
                id: Uuid::new_v4(),
                proposal: ProposalStamp {
                    id: Uuid::new_v4(),
                    version: 1,
                },
                data: linked.clone(),
                created_at_ms: 1,
            },
            version: if origin_only { 2 } else { 1 },
            data: if origin_only { current.clone() } else { linked },
            updated_at_ms: if origin_only { 2 } else { 1 },
            waiting_since_ms: None,
            completed_at_ms: None,
        };
        before.validate().unwrap();
        let mut draft = template.clone();
        draft.id = Uuid::new_v4();
        draft.action_changes = vec![ActionChange::Replace {
            before: Box::new(before),
            data: current,
        }];
        let review = store.create_proposal(&draft).unwrap();
        let snapshot = store.inbox_review_manifest(original.capture.id).unwrap();
        assert!(
            snapshot.proposals.iter().any(|p| p.record == review),
            "immutable baseline Source link was omitted (origin_only={origin_only})"
        );
    }
}

#[test]
fn original_review_includes_historical_not_applied_action_link_after_edit_and_rejection() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Inspect Source")
        .unwrap();
    let template_stamp = review(&mut store);
    let mut draft = store.proposal(template_stamp.id).unwrap().unwrap().draft;
    draft.id = Uuid::new_v4();
    draft.action_changes[0].data_mut().sources = vec![capture.note_id().unwrap()];
    let created = store.create_proposal(&draft).unwrap();
    let operation_id = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id,
            expected: created.stamp(),
        })
        .unwrap();
    store
        .refuse_proposal_before_effects(operation_id, Some(&[]))
        .unwrap();
    let current = store.proposal(draft.id).unwrap().unwrap();
    let mut candidate = current.draft.action_changes[0].data().clone();
    candidate.sources.clear();
    let edited = store
        .edit_proposal(&ProposalEdit {
            expected: current.stamp(),
            title: current.draft.title.clone(),
            texts: vec![],
            action_data: vec![candidate],
        })
        .unwrap();
    let rejected = store.reject_proposal(edited.stamp()).unwrap();
    let snapshot = store.inbox_review_manifest(original.capture.id).unwrap();
    assert!(
        snapshot.proposals.iter().any(|p| p.record == rejected),
        "historically linked proposal was omitted"
    );
    assert!(
        snapshot
            .approvals
            .iter()
            .any(|j| j.request.operation_id == operation_id),
        "retained full journal was omitted"
    );
}

#[test]
fn removal_snapshot_tracks_current_actions_completions_and_running_rewrite_without_clipping() {
    use brn_store::work::{
        action_completion::CompleteActionRequest,
        proposal_rewrite::{RewriteOutcome, RewriteStatus},
    };
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Inspect exact Source")
        .unwrap();
    let template = review(&mut store);
    let mut draft = store.proposal(template.id).unwrap().unwrap().draft;
    draft.id = Uuid::new_v4();
    draft.group_id = Some(capture.id);
    draft.action_changes[0].data_mut().sources = vec![capture.note_id().unwrap()];
    let created = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: created.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(request.operation_id, &[])
        .unwrap();
    store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    let before = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_eq!(before.actions.len(), 1);
    assert_eq!(before.lineage.len(), 1);
    let old_digest = before.digest().unwrap();
    let completion = store
        .complete_action_with(
            &CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: Box::new(before.actions[0].clone()),
            },
            before.actions[0].updated_at_ms + 1,
            |_| Ok(()),
        )
        .unwrap();
    let after = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_eq!(after.completions, vec![completion.clone()]);
    assert_eq!(after.actions, vec![completion.after]);
    assert_ne!(after.digest().unwrap(), old_digest);
    assert_eq!(
        after.review.digest().unwrap(),
        before.review.digest().unwrap(),
        "completion is separate from proposal history"
    );
    let mut rewrite_draft = draft.clone();
    rewrite_draft.id = Uuid::new_v4();
    rewrite_draft.action_changes[0] = ActionChange::Create {
        id: Uuid::new_v4(),
        data: draft.action_changes[0].data().clone(),
    };
    let proposal = store.create_proposal(&rewrite_draft).unwrap();
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: proposal.stamp(),
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    };
    let (running, _) = store.begin_proposal_rewrite(&spec).unwrap();
    store.reject_proposal(proposal.stamp()).unwrap();
    let running_snapshot = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_eq!(running_snapshot.rewrites, vec![running]);
    let running_digest = running_snapshot.digest().unwrap();
    store
        .finish_proposal_rewrite(spec.id, &RewriteOutcome::Interrupted)
        .unwrap();
    let final_snapshot = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_eq!(
        final_snapshot.rewrites[0].status,
        RewriteStatus::Interrupted
    );
    assert_ne!(final_snapshot.digest().unwrap(), running_digest);
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
}

#[test]
fn removal_snapshot_retains_undo_lineage_even_when_undo_draft_has_no_source_links() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Inspect exact Source")
        .unwrap();
    let draft = knowledge_draft(&capture);
    let created = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: created.stamp(),
    };
    let journal = store.begin_proposal_apply(&request).unwrap();
    let text = draft.changes[0].text().unwrap();
    let installed = FileFingerprint {
        device: 1,
        inode: 71,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    };
    store
        .record_proposal_prepared(request.operation_id, std::slice::from_ref(&installed))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(installed),
                staging: None,
            }]),
        )
        .unwrap();
    let before = store.inbox_removal_snapshot(original.capture.id).unwrap();
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: request.operation_id,
        trash_member: None,
    };
    let undo_journal = store.begin_proposal_undo(&undo).unwrap();
    assert!(undo_journal.approved.draft.sources.is_empty());
    assert!(undo_journal.approved.draft.inbox_knowledge.is_none());
    let after = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_eq!(after.lineage.len(), 2);
    assert!(
        after
            .lineage
            .iter()
            .any(|a| a.request.operation_id == undo.operation_id)
    );
    assert_ne!(before.digest().unwrap(), after.digest().unwrap());
    assert_eq!(
        after
            .lineage
            .iter()
            .find(|a| a.request.operation_id == request.operation_id)
            .unwrap()
            .approved,
        journal.approved
    );
}

fn settle_removal_action(
    store: &mut WorkStore,
    draft: &ProposalDraft,
) -> brn_store::work::actions::ActionRecord {
    let proposal = store.create_proposal(draft).unwrap();
    let op = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: op,
            expected: proposal.stamp(),
        })
        .unwrap();
    store.record_proposal_prepared(op, &[]).unwrap();
    store
        .finish_proposal_apply(op, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    store.action(draft.action_changes[0].id()).unwrap().unwrap()
}
#[test]
fn removal_snapshot_retains_action_family_after_source_links_are_cleared() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let (original, capture) = review_capture(&mut store);
    store
        .reserve_inbox_action(&capture, "Inspect Source")
        .unwrap();
    let template = review(&mut store);
    let mut draft = store.proposal(template.id).unwrap().unwrap().draft;
    draft.id = Uuid::new_v4();
    let mut action = settle_removal_action(&mut store, &draft);
    for linked in [true, false] {
        draft.id = Uuid::new_v4();
        let mut candidate = action.data.clone();
        candidate.sources = if linked {
            vec![capture.note_id().unwrap()]
        } else {
            vec![]
        };
        draft.action_changes = vec![ActionChange::Replace {
            before: Box::new(action),
            data: candidate,
        }];
        action = settle_removal_action(&mut store, &draft);
    }
    draft.id = Uuid::new_v4();
    let mut candidate = action.data.clone();
    candidate.title = "Later pending Action review".into();
    draft.action_changes = vec![ActionChange::Replace {
        before: Box::new(action),
        data: candidate,
    }];
    let pending = store.create_proposal(&draft).unwrap();
    let before = store.inbox_removal_snapshot(original.capture.id).unwrap();
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: pending.stamp(),
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    };
    let (rewrite, _) = store.begin_proposal_rewrite(&spec).unwrap();
    let after = store.inbox_removal_snapshot(original.capture.id).unwrap();
    assert_ne!(
        before.digest().unwrap(),
        after.digest().unwrap(),
        "Source-unlinked pending Action Rewrite was omitted"
    );
    assert!(after.rewrites.contains(&rewrite));
    assert!(after.related_reviews.iter().any(|p| p.record == pending
        && p.creation_sha256 == digest(&serde_json::to_vec(&pending.draft).unwrap())));
}
#[test]
fn removal_snapshot_refuses_missing_known_applied_action_or_completion_authority() {
    use brn_store::work::action_completion::CompleteActionRequest;
    let mut missed = vec![];
    for mode in ["open_action", "completed_action", "completion", "proposal"] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let (original, capture) = review_capture(&mut store);
        store
            .reserve_inbox_action(&capture, "Inspect Source")
            .unwrap();
        let template = review(&mut store);
        let mut draft = store.proposal(template.id).unwrap().unwrap().draft;
        draft.id = Uuid::new_v4();
        draft.group_id = Some(capture.id);
        draft.action_changes[0].data_mut().sources = vec![capture.note_id().unwrap()];
        let action = settle_removal_action(&mut store, &draft);
        if mode != "open_action" {
            store
                .complete_action_with(
                    &CompleteActionRequest {
                        operation_id: Uuid::new_v4(),
                        before: Box::new(action.clone()),
                    },
                    action.updated_at_ms + 1,
                    |_| Ok(()),
                )
                .unwrap();
        }
        let good = store.inbox_removal_snapshot(original.capture.id).unwrap();
        assert_eq!(good.actions.len(), 1);
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        if mode == "proposal" {
            // Synthetic integrity fault, never an application mutation.
            raw.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
            raw.execute("DELETE FROM proposals WHERE id=?1", [draft.id.to_string()])
                .unwrap();
        } else if mode == "completion" {
            raw.execute(
                "DELETE FROM action_completions WHERE action_id=?1",
                [action.origin.id.to_string()],
            )
            .unwrap();
        } else {
            raw.execute(
                "DELETE FROM actions WHERE id=?1",
                [action.origin.id.to_string()],
            )
            .unwrap();
        }
        drop(raw);
        if store.inbox_removal_snapshot(original.capture.id).is_ok() {
            missed.push(mode);
        }
    }
    assert!(
        missed.is_empty(),
        "Known durable authority disappeared silently: {missed:?}"
    );
}
