use brn_store::{
    Error, WorkStore,
    work::{
        WorkBudget, WorkTurn, WorkTurnStatus,
        actions::{ActionData, ActionState},
        conversations::*,
        proposal_rewrite::{RewriteOutcome, RewriteSpec},
        proposals::{ActionChange, ProposalDraft, ProposalRecord},
    },
};
use rusqlite::Connection;
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn raw(dir: &std::path::Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}
fn completed(store: &mut WorkStore) -> WorkTurn {
    let (turn, _) = store
        .begin_turn_with_effort_and_budget(
            Uuid::new_v4(),
            None,
            "Exact question õ\r\n",
            "chatgpt",
            "model",
            Some("medium"),
            Some(WorkBudget {
                max_tool_rounds: 4,
                timeout_seconds: 60,
            }),
        )
        .unwrap();
    store
        .finish_turn(
            turn.id,
            WorkTurnStatus::Completed,
            "Exact original answer 日本語\r\n",
            None,
        )
        .unwrap()
}
fn request(store: &WorkStore, id: Uuid, target: ConversationState) -> ConversationLifecycleRequest {
    ConversationLifecycleRequest {
        operation_id: Uuid::new_v4(),
        expected: store.conversation_lifecycle(id).unwrap().stamp,
        target,
    }
}
fn review(store: &mut WorkStore, session: Option<Uuid>) -> ProposalRecord {
    store
        .create_proposal(&ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: session,
            vault: None,
            title: "Clarification for owner review".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id: Uuid::new_v4(),
                data: ActionData {
                    title: "Ask for clarification".into(),
                    description: "Unassigned review draft; no execution authority.".into(),
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
}
fn spec(record: &ProposalRecord) -> RewriteSpec {
    RewriteSpec {
        id: Uuid::new_v4(),
        expected: record.stamp(),
        provider: "chatgpt".into(),
        model: "model".into(),
        effort: "medium".into(),
    }
}
fn canonical<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn preserved(conn: &Connection) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    let tables = conn.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT IN ('conversation_lifecycle','conversation_lifecycle_operations') ORDER BY name").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    tables
        .into_iter()
        .map(|name| {
            let mut query = conn
                .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY rowid"))
                .unwrap();
            let columns = query.column_count();
            let values = query
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get(i))
                        .collect::<rusqlite::Result<Vec<rusqlite::types::Value>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            (name, values)
        })
        .collect()
}

#[test]
fn archive_restart_restore_preserves_canonical_history_budgets_times_and_all_other_tables() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    let id = turn.conversation_id;
    let proposal = review(&mut store, Some(id));
    store
        .set_setting("owner.synthetic", "kept exactly õ\r\n")
        .unwrap();
    let original = preserved(&raw(dir.path()));
    let original_conversation = canonical(&store.conversations().unwrap());
    let archive = request(&store, id, ConversationState::Archived);
    assert!(
        store
            .conversation_lifecycle_replay(&archive)
            .unwrap()
            .is_none()
    );
    let archived = store.set_conversation_lifecycle(&archive).unwrap();
    archived.validate().unwrap();
    assert_eq!(archived.receipt.after.state, ConversationState::Archived);
    assert!(
        store
            .conversation_summaries(ConversationFilter::Active)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .conversation_summaries(ConversationFilter::Archived)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(preserved(&raw(dir.path())), original);
    assert_eq!(
        canonical(&store.conversations().unwrap()),
        original_conversation
    );
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.conversation_lifecycle(id).unwrap(), archived.current);
    let restore = request(&store, id, ConversationState::Active);
    let restored = store.set_conversation_lifecycle(&restore).unwrap();
    assert_eq!(restored.current.stamp.version, 3);
    assert_eq!(
        store
            .conversation_summaries(ConversationFilter::Active)
            .unwrap()
            .len(),
        1
    );
    let replay = store.set_conversation_lifecycle(&archive).unwrap();
    assert_eq!(replay.receipt, archived.receipt);
    assert_eq!(replay.current, restored.current);
    assert_eq!(
        store.conversation_lifecycle_replay(&archive).unwrap(),
        Some(replay)
    );
    assert_eq!(preserved(&raw(dir.path())), original);
    assert_eq!(
        canonical(&store.turn(turn.id).unwrap().unwrap()),
        canonical(&turn)
    );
    assert_eq!(store.proposal(proposal.draft.id).unwrap(), Some(proposal));
    assert_eq!(
        store.run_budget(turn.id).unwrap(),
        Some(WorkBudget {
            max_tool_rounds: 4,
            timeout_seconds: 60
        })
    );
}

#[test]
fn stale_changed_same_state_invalid_and_overflow_requests_have_no_effect() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    let mut archive = request(&store, turn.conversation_id, ConversationState::Archived);
    let same = request(&store, turn.conversation_id, ConversationState::Active);
    assert!(matches!(
        store.set_conversation_lifecycle(&same),
        Err(Error::StateChanged(_))
    ));
    let result = store.set_conversation_lifecycle(&archive).unwrap();
    let mut stale = archive.clone();
    stale.operation_id = Uuid::new_v4();
    assert!(matches!(
        store.set_conversation_lifecycle(&stale),
        Err(Error::StateChanged(_))
    ));
    archive.target = ConversationState::Active;
    assert!(matches!(
        store.conversation_lifecycle_replay(&archive),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.set_conversation_lifecycle(&archive),
        Err(Error::OperationConflict(_))
    ));
    for version in [0, i64::MAX as u64, u64::MAX] {
        let mut invalid = stale.clone();
        invalid.expected.version = version;
        assert!(invalid.validate().is_err());
        assert!(store.set_conversation_lifecycle(&invalid).is_err());
    }
    let mut invalid = stale;
    invalid.operation_id = Uuid::nil();
    assert!(invalid.validate().is_err());
    assert_eq!(
        store.conversation_lifecycle(turn.conversation_id).unwrap(),
        result.current
    );
    let encoded = canonical(&result);
    let mut bad: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    bad["current"]["state"] = serde_json::json!("active");
    assert!(
        serde_json::from_value::<ConversationLifecycleResult>(bad)
            .unwrap()
            .validate()
            .is_err()
    );
    let json = String::from_utf8(canonical(&archive)).unwrap();
    assert!(
        serde_json::from_str::<ConversationLifecycleRequest>(
            &json.replace("\"target\":", "\"extra\":0,\"target\":")
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<ConversationLifecycleRequest>(
            &json.replace("\"target\":", "\"target\":\"active\",\"target\":")
        )
        .is_err()
    );
}

#[test]
fn archive_fences_fresh_ask_but_exact_completed_replay_and_settlement_remain_available() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    let mut chat = store.chat_connection().unwrap();
    let archive = request(&store, turn.conversation_id, ConversationState::Archived);
    chat.set_conversation_lifecycle(&archive).unwrap();
    let fresh = Uuid::new_v4();
    assert!(matches!(
        chat.begin_turn_with_effort_and_budget(
            fresh,
            Some(turn.conversation_id),
            "new",
            "chatgpt",
            "model",
            Some("low"),
            None
        ),
        Err(Error::StateChanged(_))
    ));
    assert!(store.turn(fresh).unwrap().is_none());
    assert!(store.run_budget(fresh).unwrap().is_none());
    let (replay, budget) = chat
        .begin_turn_with_effort_and_budget(
            turn.id,
            Some(turn.conversation_id),
            &turn.question,
            &turn.provider,
            &turn.model,
            turn.effort.as_deref(),
            None,
        )
        .unwrap();
    assert_eq!(canonical(&replay), canonical(&turn));
    assert_eq!(budget, store.run_budget(turn.id).unwrap());
    assert_eq!(
        canonical(
            &chat
                .finish_turn(turn.id, WorkTurnStatus::Completed, &turn.answer, None)
                .unwrap()
        ),
        canonical(&turn)
    );
    assert_eq!(
        chat.conversation_summaries(ConversationFilter::All)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn running_ask_wins_archive_race_and_failed_transition_is_retryable_after_settlement() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    let archive = request(&store, turn.conversation_id, ConversationState::Archived);
    let mut chat = store.chat_connection().unwrap();
    let running = chat
        .begin_turn(
            Uuid::new_v4(),
            Some(turn.conversation_id),
            "running",
            "chatgpt",
            "model",
        )
        .unwrap();
    assert!(matches!(
        store.set_conversation_lifecycle(&archive),
        Err(Error::WorkspaceBusy(_))
    ));
    assert!(
        store
            .conversation_lifecycle_replay(&archive)
            .unwrap()
            .is_none()
    );
    chat.finish_turn(
        running.id,
        WorkTurnStatus::Interrupted,
        "retained partial",
        None,
    )
    .unwrap();
    store.set_conversation_lifecycle(&archive).unwrap();
    assert_eq!(
        store.turn(running.id).unwrap().unwrap().answer,
        "retained partial"
    );
}

#[test]
fn rewrite_admission_and_archive_fence_each_other_while_legacy_session_rewrite_stays_eligible() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    let proposal = review(&mut store, Some(turn.conversation_id));
    let request_spec = spec(&proposal);
    let mut chat = store.chat_connection().unwrap();
    let (job, _) = chat.begin_proposal_rewrite(&request_spec).unwrap();
    let archive = request(&store, turn.conversation_id, ConversationState::Archived);
    assert!(matches!(
        store.set_conversation_lifecycle(&archive),
        Err(Error::WorkspaceBusy(_))
    ));
    chat.finish_proposal_rewrite(job.spec.id, &RewriteOutcome::Interrupted)
        .unwrap();
    store.set_conversation_lifecycle(&archive).unwrap();
    let fresh = spec(&proposal);
    assert!(matches!(
        chat.begin_proposal_rewrite(&fresh),
        Err(Error::StateChanged(_))
    ));
    assert!(store.proposal_rewrite(fresh.id).unwrap().is_none());
    let (replay, capture) = chat.begin_proposal_rewrite(&request_spec).unwrap();
    assert!(capture.is_none());
    assert_eq!(replay.spec, request_spec);
    for session in [None, Some(Uuid::new_v4())] {
        let legacy = review(&mut store, session);
        let spec = spec(&legacy);
        chat.begin_proposal_rewrite(&spec).unwrap();
        chat.finish_proposal_rewrite(spec.id, &RewriteOutcome::Interrupted)
            .unwrap();
    }
    assert_eq!(store.conversations().unwrap().len(), 1);
}

#[test]
fn v17_migration_backfills_only_lifecycle_and_backup_restores_the_recorded_checkpoint() {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let turn = completed(&mut store);
    drop(store);
    let conn = raw(dir.path());
    conn.execute_batch("DROP TABLE conversation_lifecycle_operations; DROP TABLE conversation_lifecycle; PRAGMA user_version=17;").unwrap();
    let before = preserved(&conn);
    drop(conn);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(preserved(&raw(dir.path())), before);
    assert_eq!(
        store.conversation_lifecycle(turn.conversation_id).unwrap(),
        ConversationLifecycle {
            stamp: ConversationStamp {
                id: turn.conversation_id,
                version: 1
            },
            state: ConversationState::Active
        }
    );
    let archive = request(&store, turn.conversation_id, ConversationState::Archived);
    let result = store.set_conversation_lifecycle(&archive).unwrap();
    drop(store);
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    let checkpoint = report.backup;
    drop(store);
    std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(checkpoint));
    assert_eq!(
        store.conversation_lifecycle(turn.conversation_id).unwrap(),
        result.current
    );
    assert_eq!(
        store.conversation_lifecycle_replay(&archive).unwrap(),
        Some(result)
    );
    assert_eq!(
        canonical(&store.turn(turn.id).unwrap().unwrap()),
        canonical(&turn)
    );
    assert_eq!(preserved(&raw(dir.path())), before);
}

#[test]
fn readable_lifecycle_damage_refuses_main_without_restoring_older_backups() {
    for mutation in [
        "DELETE FROM conversation_lifecycle",
        "UPDATE conversation_lifecycle SET version=3",
        "UPDATE conversation_lifecycle SET state='active'",
        "UPDATE conversation_lifecycle_operations SET receipt_sha256=zeroblob(32)",
        "DELETE FROM conversation_lifecycle_operations",
        "CREATE TRIGGER unexpected_lifecycle AFTER UPDATE ON conversation_lifecycle BEGIN SELECT 1; END",
    ] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = completed(&mut store);
        let archive = request(&store, turn.conversation_id, ConversationState::Archived);
        store.set_conversation_lifecycle(&archive).unwrap();
        drop(store);
        raw(dir.path()).execute_batch(mutation).unwrap();
        let before = std::fs::read(dir.path().join("brn.sqlite")).unwrap();
        assert!(WorkStore::open(dir.path()).is_err(), "{mutation}");
        assert_eq!(
            std::fs::read(dir.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("corrupt")
        }));
    }
}

#[test]
fn concurrent_attached_admission_and_archive_have_one_atomic_winner() {
    use std::sync::{Arc, Barrier};
    for _ in 0..8 {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let previous = completed(&mut store);
        let archive = request(
            &store,
            previous.conversation_id,
            ConversationState::Archived,
        );
        let mut chat = store.chat_connection().unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let other = barrier.clone();
        let admission = std::thread::spawn(move || {
            other.wait();
            chat.begin_turn(
                Uuid::new_v4(),
                Some(previous.conversation_id),
                "racing admission",
                "chatgpt",
                "model",
            )
        });
        barrier.wait();
        let transition = store.set_conversation_lifecycle(&archive);
        let admission = admission.join().unwrap();
        match (transition, admission) {
            (Ok(result), Err(Error::StateChanged(_))) => {
                assert_eq!(result.current.state, ConversationState::Archived)
            }
            (Err(Error::WorkspaceBusy(_)), Ok(turn)) => {
                assert_eq!(
                    store
                        .conversation_lifecycle(turn.conversation_id)
                        .unwrap()
                        .state,
                    ConversationState::Active
                );
                store
                    .finish_turn(turn.id, WorkTurnStatus::Interrupted, "race retained", None)
                    .unwrap();
            }
            other => panic!("archive/admission must have one atomic winner: {other:?}"),
        }
    }
}

#[test]
fn rehashed_invalid_receipt_or_row_binding_is_refused_before_open() {
    use sha2::{Digest, Sha256};
    for mutation in ["identity", "transition", "chain"] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = completed(&mut store);
        let archive = request(&store, turn.conversation_id, ConversationState::Archived);
        store.set_conversation_lifecycle(&archive).unwrap();
        drop(store);
        let conn = raw(dir.path());
        let bytes: Vec<u8> = conn
            .query_row(
                "SELECT receipt_json FROM conversation_lifecycle_operations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut receipt: ConversationLifecycleReceipt = serde_json::from_slice(&bytes).unwrap();
        match mutation {
            "identity" => receipt.request.operation_id = Uuid::new_v4(),
            "transition" => receipt.after.state = ConversationState::Active,
            "chain" => {
                receipt.before.stamp.version = 3;
                receipt.request.expected.version = 3;
                receipt.after.stamp.version = 4;
            }
            _ => unreachable!(),
        }
        let bytes = canonical(&receipt);
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        conn.execute(
            "UPDATE conversation_lifecycle_operations SET receipt_json=?1,receipt_sha256=?2",
            rusqlite::params![bytes, hash.as_slice()],
        )
        .unwrap();
        drop(conn);
        assert!(WorkStore::open(dir.path()).is_err(), "{mutation}");
    }
}
