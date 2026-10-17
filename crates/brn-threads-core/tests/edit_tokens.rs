//! Round 3: caller-bound edit sessions, workspace purpose, receipt actors,
//! and schema 6 (upgrade from 5, restore of older and test backups).
//!
//! The schema-5 fixtures downgrade a fresh store by dropping the schema-6
//! tables, the downgrade-fixture pattern of `project_format.rs`.
use brn_threads_core::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn owner_record(store: &mut Store, key: &str, data: RecordData) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data,
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!("fixture write")
    };
    receipt.writes[0].after.clone()
}

fn note(store: &mut Store, key: &str, text: &str) -> Record {
    owner_record(
        store,
        key,
        RecordData::Note(Note::working("Synthetic", text)),
    )
}

/// Every file in the folder (recursively) with its bytes.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn user_version(path: &Path) -> i64 {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

/// A schema-5 workspace: a current store with the schema-6 tables dropped.
fn schema5(store: Store) -> PathBuf {
    let data = store.directory().to_owned();
    drop(store);
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    conn.execute_batch(
        "DROP INDEX one_writing_setup; DROP TABLE edit_session_bindings; DROP TABLE workspace_profile; DROP TABLE receipt_actors; PRAGMA user_version=5;",
    )
    .unwrap();
    drop(conn);
    data
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

// Tokens

#[test]
fn tokens_match_the_pattern_differ_and_only_their_hash_is_stored() {
    let root = root();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let a = note(&mut store, "a", "alpha");
    let b = note(&mut store, "b", "beta");
    let first = store.begin_caller_edit(&a.id, a.version).unwrap();
    let second = store.begin_caller_edit(&b.id, b.version).unwrap();
    for issued in [&first, &second] {
        let token = issued.token.expose();
        assert!(EditToken::parse(token).is_some(), "{}", token.len());
        assert_eq!(token.len(), 71);
        assert!(token.starts_with("brnet1_"));
        assert_eq!(issued.session.holder, EditHolder::Caller);
        assert_eq!(format!("{:?}", issued.token), "EditToken(..)");
        assert!(!format!("{issued:?}").contains(token));
    }
    assert_ne!(first.token.expose(), second.token.expose());
    for bad in [
        "",
        "brnet1_",
        &format!("brnet1_{}", "A".repeat(64)),
        &format!("brnet2_{}", "a".repeat(64)),
        &format!("brnet1_{}", "a".repeat(63)),
        &format!("{} ", first.token.expose()),
    ] {
        assert!(EditToken::parse(bad).is_none(), "{bad:?}");
    }
    drop(store);
    let mut bytes = vec![];
    for (_, file) in snapshot(&data) {
        bytes.extend(file);
    }
    use sha2::{Digest, Sha256};
    for issued in [&first, &second] {
        let token = issued.token.expose();
        assert!(!contains(&bytes, token), "the token is stored");
        let hash = format!("{:x}", Sha256::digest(token.as_bytes()));
        assert!(contains(&bytes, &hash), "the hash is not stored");
    }
}

#[test]
fn bound_writes_need_their_own_token_and_refusals_write_nothing() {
    let root = root();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let a = note(&mut store, "a", "alpha");
    let b = note(&mut store, "b", "beta");
    let mine = store.begin_caller_edit(&a.id, a.version).unwrap();
    let other = store.begin_caller_edit(&b.id, b.version).unwrap();
    let desktop = {
        let c = note(&mut store, "c", "gamma");
        store.begin_edit(&c.id, c.version).unwrap()
    };
    assert_eq!(desktop.holder, EditHolder::OwnerHost);
    let id = &mine.session.id;
    // Another session's token: Mismatch, nothing written.
    for result in [
        store
            .update_caller_buffer(id, &other.token, 1, "taken over")
            .map(|_| ()),
        store.save_caller_edit(id, &other.token, 0).map(|_| ()),
        store.discard_caller_edit(id, &other.token, 0).map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(Error::EditToken { problem: EditTokenProblem::Mismatch, ref session }) if session == id
        ));
    }
    assert_eq!(store.edit_session(id).unwrap(), mine.session);
    // A desktop-held session takes no token at all.
    assert!(matches!(
        store.update_caller_buffer(&desktop.id, &mine.token, 1, "x"),
        Err(Error::EditToken {
            problem: EditTokenProblem::OwnerHost,
            ..
        })
    ));
    assert_eq!(store.edit_session(&desktop.id).unwrap(), desktop);
    // Unknown session: Missing.
    assert!(matches!(
        store.save_caller_edit("no-such-session", &mine.token, 0),
        Err(Error::Missing(_))
    ));
    // The right token writes.
    let BufferOutcome::Updated(updated) = store
        .update_caller_buffer(id, &mine.token, 1, "alpha, edited")
        .unwrap()
    else {
        panic!("update")
    };
    assert_eq!(updated.holder, EditHolder::Caller);
    let SaveOutcome::Saved(receipt) = store.save_caller_edit(id, &mine.token, 1).unwrap() else {
        panic!("save")
    };
    assert!(
        receipt.authority.contains("editor"),
        "{}",
        receipt.authority
    );
    // A retry replays the same receipt.
    assert_eq!(
        store.save_caller_edit(id, &mine.token, 1).unwrap(),
        SaveOutcome::Saved(receipt.clone())
    );
    // Closed: the binding stays, so the token still checks; then today's rules.
    assert!(matches!(
        store
            .update_caller_buffer(id, &mine.token, 2, "late")
            .unwrap(),
        BufferOutcome::Ignored(_)
    ));
    assert!(matches!(
        store.save_caller_edit(id, &other.token, 1),
        Err(Error::EditToken { .. })
    ));
}

#[test]
fn a_bound_stale_save_keeps_the_buffer_and_the_token_survives_reopen() {
    let root = root();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let a = note(&mut store, "a", "alpha");
    // The note moves on to version 2 before the caller's session opens on 1.
    let RecordData::Note(mut changed) = a.data.clone() else {
        panic!()
    };
    changed.markdown = "alpha, changed elsewhere".into();
    let op = store.allocate_operation("other-writer").unwrap();
    let owner = HostAuthority::owner("synthetic");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "elsewhere".into(),
                writes: vec![Put {
                    id: a.id.clone(),
                    expected_version: Some(a.version),
                    archived: false,
                    data: RecordData::Note(changed),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    let id = issued.session.id.clone();
    store
        .update_caller_buffer(&id, &issued.token, 1, "my typing")
        .unwrap();
    let SaveOutcome::Stale(kept) = store.save_caller_edit(&id, &issued.token, 1).unwrap() else {
        panic!("stale")
    };
    assert_eq!(kept.markdown, "my typing");
    assert!(!kept.closed);
    drop(store);
    let mut store = Store::open(&data).unwrap();
    let session = store.edit_session(&id).unwrap();
    assert_eq!(session.markdown, "my typing");
    assert_eq!(session.holder, EditHolder::Caller);
    assert!(matches!(
        store
            .update_caller_buffer(&id, &issued.token, 2, "my typing, more")
            .unwrap(),
        BufferOutcome::Updated(_)
    ));
    assert!(matches!(
        store.discard_caller_edit(&id, &issued.token, 2).unwrap(),
        BufferOutcome::Updated(_)
    ));
}

// Owner host on caller sessions

#[test]
fn owner_typing_adopts_a_caller_session_and_the_caller_token_stops() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let a = note(&mut store, "a", "alpha");
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    let id = issued.session.id.clone();
    assert_eq!(
        store.recovery_buffers().unwrap()[0].holder,
        EditHolder::Caller
    );
    // The owner host reattaches by ID and types the next generation.
    let BufferOutcome::Updated(adopted) = store.update_buffer(&id, 1, "owner typing").unwrap()
    else {
        panic!("adopt")
    };
    assert_eq!(adopted.holder, EditHolder::OwnerHost);
    assert_eq!(
        store.edit_session(&id).unwrap().holder,
        EditHolder::OwnerHost
    );
    // A delayed caller write can no longer land over the owner's typing.
    assert!(matches!(
        store.update_caller_buffer(&id, &issued.token, 5, "late caller"),
        Err(Error::EditToken {
            problem: EditTokenProblem::OwnerHost,
            ..
        })
    ));
    assert_eq!(store.edit_session(&id).unwrap().markdown, "owner typing");
    // An ignored owner write (old generation) adopts nothing.
    let b = note(&mut store, "b", "beta");
    let other = store.begin_caller_edit(&b.id, b.version).unwrap();
    assert!(matches!(
        store.update_buffer(&other.session.id, 0, "stale").unwrap(),
        BufferOutcome::Ignored(_)
    ));
    assert_eq!(
        store.edit_session(&other.session.id).unwrap().holder,
        EditHolder::Caller
    );
}

#[test]
fn owner_discard_and_save_close_caller_sessions_without_a_token() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let a = note(&mut store, "a", "alpha");
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    let id = issued.session.id.clone();
    // U4: the owner discards the listed session by ID.
    assert!(matches!(
        store.discard(&id, 0).unwrap(),
        BufferOutcome::Updated(_)
    ));
    assert!(matches!(
        store.save_caller_edit(&id, &issued.token, 0).unwrap(),
        SaveOutcome::Closed(_)
    ));
    assert!(matches!(
        store.discard_caller_edit(&id, &issued.token, 0).unwrap(),
        BufferOutcome::Ignored(_)
    ));
    // The owner saves a caller's session; the caller's save replay returns it.
    let b = note(&mut store, "b", "beta");
    let issued = store.begin_caller_edit(&b.id, b.version).unwrap();
    let id = issued.session.id.clone();
    store
        .update_caller_buffer(&id, &issued.token, 1, "beta, by the caller")
        .unwrap();
    let SaveOutcome::Saved(receipt) = store.save(&id, 1).unwrap() else {
        panic!("owner save")
    };
    assert_eq!(
        store.save_caller_edit(&id, &issued.token, 1).unwrap(),
        SaveOutcome::Saved(receipt)
    );
}

// Reclaim

#[test]
fn reclaim_is_refused_in_owner_workspaces_and_rotates_in_test_workspaces() {
    let root = root();
    let owner_data = root.path().join("owner");
    let mut store = Store::open(&owner_data).unwrap();
    let a = note(&mut store, "a", "alpha");
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    drop(store);
    let before = snapshot(&owner_data);
    let mut store = Store::open(&owner_data).unwrap();
    assert!(matches!(
        store.reclaim_caller_edit(&issued.session.id),
        Err(Error::TestWorkspaceRequired)
    ));
    drop(store);
    assert_eq!(snapshot(&owner_data), before);

    let mut store = Store::create(root.path().join("test"), WorkspacePurpose::Test).unwrap();
    let a = note(&mut store, "a", "alpha");
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    let id = issued.session.id.clone();
    store
        .update_caller_buffer(&id, &issued.token, 3, "kept")
        .unwrap();
    let reclaimed = store.reclaim_caller_edit(&id).unwrap();
    assert_ne!(reclaimed.token.expose(), issued.token.expose());
    assert_eq!(reclaimed.session.generation, 3);
    assert_eq!(reclaimed.session.markdown, "kept");
    assert_eq!(reclaimed.session.holder, EditHolder::Caller);
    assert!(matches!(
        store.update_caller_buffer(&id, &issued.token, 4, "old token"),
        Err(Error::EditToken {
            problem: EditTokenProblem::Mismatch,
            ..
        })
    ));
    assert!(matches!(
        store
            .update_caller_buffer(&id, &reclaimed.token, 4, "new token")
            .unwrap(),
        BufferOutcome::Updated(_)
    ));
    // Owner-held sessions in a test workspace can be reclaimed too.
    let b = note(&mut store, "b", "beta");
    let desktop = store.begin_edit(&b.id, b.version).unwrap();
    let taken = store.reclaim_caller_edit(&desktop.id).unwrap();
    assert_eq!(taken.session.holder, EditHolder::Caller);
    store
        .discard_caller_edit(&desktop.id, &taken.token, 0)
        .unwrap();
    assert!(matches!(
        store.reclaim_caller_edit(&desktop.id),
        Err(Error::Invalid(message)) if message == "This edit is closed"
    ));
}

// Purpose

#[test]
fn purpose_is_set_at_creation_only_and_owner_by_default() {
    let root = root();
    let test = root.path().join("test");
    let store = Store::create(&test, WorkspacePurpose::Test).unwrap();
    assert_eq!(store.workspace_purpose().unwrap(), WorkspacePurpose::Test);
    drop(store);
    let store = Store::open(&test).unwrap();
    assert_eq!(store.workspace_purpose().unwrap(), WorkspacePurpose::Test);
    drop(store);
    // An existing workspace is refused unchanged.
    let before = snapshot(&test);
    for purpose in [WorkspacePurpose::Test, WorkspacePurpose::Owner] {
        assert!(matches!(
            Store::create(&test, purpose),
            Err(Error::Invalid(message)) if message == "A test workspace needs a new data folder"
        ));
    }
    assert_eq!(snapshot(&test), before);
    // A non-empty folder without a marker stays an unsafe directory.
    let foreign = root.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("file"), "x").unwrap();
    assert!(matches!(
        Store::create(&foreign, WorkspacePurpose::Test),
        Err(Error::UnsafeDirectory(_))
    ));
    // An empty folder is a new folder.
    let empty = root.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert_eq!(
        Store::create(&empty, WorkspacePurpose::Test)
            .unwrap()
            .workspace_purpose()
            .unwrap(),
        WorkspacePurpose::Test
    );
    // Plain open and the no-upgrade open create owner workspaces.
    let mut owner = Store::open(root.path().join("owner")).unwrap();
    assert_eq!(owner.workspace_purpose().unwrap(), WorkspacePurpose::Owner);
    let quiet = Store::open_existing_without_upgrade(root.path().join("quiet")).unwrap();
    assert_eq!(quiet.workspace_purpose().unwrap(), WorkspacePurpose::Owner);
    // The owner host may change it.
    owner.set_workspace_purpose(WorkspacePurpose::Test).unwrap();
    assert_eq!(owner.workspace_purpose().unwrap(), WorkspacePurpose::Test);
    owner
        .set_workspace_purpose(WorkspacePurpose::Owner)
        .unwrap();
    assert_eq!(owner.workspace_purpose().unwrap(), WorkspacePurpose::Owner);
    assert!(
        owner.receipts().unwrap().is_empty(),
        "no receipt in phase 1"
    );
}

// Actor

#[test]
fn actors_commit_with_receipts_and_never_change_identity() {
    let root = root();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    assert!(matches!(
        store.set_actor(Some("bad actor")),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.set_actor(Some(&"a".repeat(65))),
        Err(Error::Invalid(_))
    ));
    store.set_actor(Some("agent-7")).unwrap();
    let first = note(&mut store, "keyed", "same request");
    let op = store.receipts().unwrap()[0].operation.clone();
    assert_eq!(
        store.receipt_actor(&op).unwrap().as_deref(),
        Some("agent-7")
    );
    // A replay under another actor keeps the first.
    let mut other = Store::open(&data).unwrap();
    other.set_actor(Some("agent-8")).unwrap();
    let replayed = other.allocate_operation("keyed").unwrap();
    assert_eq!(replayed, op);
    let ApplyOutcome::Applied(receipt) = other
        .apply(&replayed, &HostAuthority::owner("synthetic fixture"))
        .unwrap()
    else {
        panic!("replay")
    };
    assert_eq!(receipt.writes[0].after, first);
    assert_eq!(
        other.receipt_actor(&op).unwrap().as_deref(),
        Some("agent-7")
    );
    // No actor records none; the request hash does not depend on the actor.
    other.set_actor(None).unwrap();
    let unnamed = owner_record(
        &mut other,
        "unnamed",
        RecordData::Note(Note::working("Same", "body")),
    );
    other.set_actor(Some("agent-9")).unwrap();
    let named = owner_record(
        &mut other,
        "named",
        RecordData::Note(Note::working("Same", "body")),
    );
    let receipts = other.receipts().unwrap();
    let find = |id: &str| {
        receipts
            .iter()
            .find(|r| r.writes[0].after.id == id)
            .unwrap()
            .clone()
    };
    let (a, b) = (find(&unnamed.id), find(&named.id));
    assert_eq!(other.receipt_actor(&a.operation).unwrap(), None);
    assert_eq!(
        other.receipt_actor(&b.operation).unwrap().as_deref(),
        Some("agent-9")
    );
    // Creation IDs differ per operation, so compare a fixed request.
    let fixed = |store: &mut Store, key: &str| {
        let op = store.allocate_operation(key).unwrap();
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "fixed".into(),
                    writes: vec![Put {
                        id: first.id.clone(),
                        expected_version: Some(1),
                        archived: false,
                        data: first.data.clone(),
                    }],
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.prepared(&op).unwrap().request_hash
    };
    other.set_actor(Some("x")).unwrap();
    let with_x = fixed(&mut other, "fixed-x");
    other.set_actor(None).unwrap();
    let without = fixed(&mut other, "fixed-none");
    assert_eq!(with_x, without);
}

// Schema 5 to 6

#[test]
fn the_no_upgrade_open_refuses_schema_5_and_changes_nothing() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    note(&mut store, "a", "schema five");
    let data = schema5(store);
    let before = snapshot(&data);
    assert!(matches!(
        Store::open_existing_without_upgrade(&data),
        Err(Error::UpgradeRequired {
            schema_found: 5,
            schema_required: 9
        })
    ));
    assert_eq!(snapshot(&data), before);
    assert_eq!(user_version(&data.join(DATABASE_FILENAME)), 5);
    // A missing or empty folder is creation, not an upgrade.
    let fresh = Store::open_existing_without_upgrade(root.path().join("fresh")).unwrap();
    assert_eq!(fresh.workspace_purpose().unwrap(), WorkspacePurpose::Owner);
    assert_eq!(user_version(&fresh.directory().join(DATABASE_FILENAME)), 9);
}

#[test]
fn schema_5_upgrade_is_refused_with_an_open_edit_or_without_a_folder() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let a = note(&mut store, "a", "schema five");
    store.begin_edit(&a.id, a.version).unwrap();
    let data = schema5(store);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    assert!(matches!(
        Store::upgrade(&data, &backups),
        Err(Error::UnsafeDirectory(message)) if message.contains("save/discard edits")
    ));
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    assert_eq!(user_version(&data.join(DATABASE_FILENAME)), 5);

    let mut store = Store::open(root.path().join("plain")).unwrap();
    note(&mut store, "a", "schema five");
    let plain = schema5(store);
    // The owner host's implicit upgrade needs a remembered folder: the hint
    // keeps the schema 3/4 wording and adds a schema 5 sentence.
    assert!(matches!(
        Store::open(&plain),
        Err(Error::UnsafeDirectory(message)) if message.starts_with("Schema 5 needs an external pre-upgrade backup folder. Run: brn --data-dir ")
    ));
    assert_eq!(user_version(&plain.join(DATABASE_FILENAME)), 5);
}

#[test]
fn schema_5_upgrade_is_refused_with_a_working_run() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let thread = owner_record(
        &mut store,
        "thread",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "fixture".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    owner_record(
        &mut store,
        "run",
        RecordData::Run(Run {
            loaded_skill_details: vec![],
            instruction: None,
            queue_claim: None,
            thread: thread.id,
            provider: "offline".into(),
            model: "fixture".into(),
            guide_identity: "fixture".into(),
            budget: 1,
            effort: "low".into(),
            loaded_skills: vec![],
            fence: 0,
            state: RunState::Working,
            progress: "fixture".into(),
            operations: vec![],
            sources: vec![],
        }),
    );
    let data = schema5(store);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    assert!(matches!(
        Store::upgrade(&data, &backups),
        Err(Error::UnsafeDirectory(message)) if message.starts_with("Stop work")
    ));
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    assert_eq!(user_version(&data.join(DATABASE_FILENAME)), 5);
}

#[test]
fn schema_5_upgrade_backs_up_first_and_ends_at_7_as_an_owner_workspace() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let a = note(&mut store, "a", "schema five");
    let data = schema5(store);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    // A competing writer holds the lock: the upgrade cannot start, and
    // nothing is backed up or migrated.
    let competitor = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    competitor.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(Store::upgrade(&data, &backups).is_err());
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    competitor.execute_batch("ROLLBACK").unwrap();
    drop(competitor);
    assert_eq!(user_version(&data.join(DATABASE_FILENAME)), 5);

    Store::upgrade(&data, &backups).unwrap();
    let files: Vec<_> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let name = files[0].file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("brn-before-schema-9-") && name.ends_with(".sqlite3"));
    assert_eq!(user_version(&files[0]), 5);
    assert_eq!(user_version(&data.join(DATABASE_FILENAME)), 9);
    let store = Store::open_existing_without_upgrade(&data).unwrap();
    assert_eq!(store.workspace_purpose().unwrap(), WorkspacePurpose::Owner);
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
    drop(store);
    // A second upgrade of a current workspace records nothing.
    Store::upgrade(&data, &backups).unwrap();
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 1);
    // The published backup restores (older backups upgrade only their copy).
    let restored = Store::restore(&files[0], root.path().join("restored")).unwrap();
    assert_eq!(restored.record(&a.id).unwrap().unwrap(), a);
    assert_eq!(user_version(&files[0]), 5);
}

// Restore

#[test]
fn older_backups_restore_as_schema_9_owner_workspaces() {
    let root = root();
    for version in [3, 4, 5] {
        let mut store = Store::open(root.path().join(format!("v{version}"))).unwrap();
        let a = note(&mut store, "a", "older");
        let data = schema5(store);
        let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
        if version < 5 {
            conn.execute_batch("DROP INDEX project_records; DROP INDEX context_link_from; DROP INDEX context_link_to; DROP INDEX context_import_source; DROP INDEX context_asset_source;").unwrap();
        }
        if version == 3 {
            conn.execute_batch("DROP TABLE chat_drafts; DROP INDEX one_chat_control;")
                .unwrap();
        }
        conn.pragma_update(None, "user_version", version).unwrap();
        drop(conn);
        let source = data.join(DATABASE_FILENAME);
        let before = std::fs::read(&source).unwrap();
        let restored =
            Store::restore(&source, root.path().join(format!("restored-{version}"))).unwrap();
        assert_eq!(
            restored.workspace_purpose().unwrap(),
            WorkspacePurpose::Owner
        );
        assert_eq!(
            user_version(&restored.directory().join(DATABASE_FILENAME)),
            9
        );
        assert_eq!(restored.record(&a.id).unwrap().unwrap(), a);
        assert_eq!(std::fs::read(&source).unwrap(), before, "v{version}");
    }
}

#[test]
fn a_test_backup_with_a_caller_session_restores_as_an_owner_workspace() {
    let root = root();
    let mut store = Store::create(root.path().join("test"), WorkspacePurpose::Test).unwrap();
    let a = note(&mut store, "a", "alpha");
    let issued = store.begin_caller_edit(&a.id, a.version).unwrap();
    store
        .update_caller_buffer(&issued.session.id, &issued.token, 1, "unsaved")
        .unwrap();
    let backup = root.path().join("backup.sqlite3");
    store.backup(&backup).unwrap();
    drop(store);
    let before = std::fs::read(&backup).unwrap();
    let mut restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(std::fs::read(&backup).unwrap(), before);
    assert_eq!(
        restored.workspace_purpose().unwrap(),
        WorkspacePurpose::Owner
    );
    let sessions = restored.recovery_buffers().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].holder, EditHolder::OwnerHost);
    assert_eq!(sessions[0].markdown, "unsaved");
    assert_eq!(sessions[0].generation, 1);
    assert!(matches!(
        restored.update_caller_buffer(&issued.session.id, &issued.token, 2, "x"),
        Err(Error::EditToken {
            problem: EditTokenProblem::OwnerHost,
            ..
        })
    ));
    assert!(matches!(
        restored.reclaim_caller_edit(&issued.session.id),
        Err(Error::TestWorkspaceRequired)
    ));
}
