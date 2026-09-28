use brn_store::{OperationStatus, Store};
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn version_bytes_are_exact_and_retries_are_idempotent() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let op = Uuid::new_v4();
    let source = store.create_source(op, "notes").unwrap();
    let version_op = Uuid::new_v4();
    let version = store
        .add_version(version_op, source, None, "naïve 🌍\r\n".as_bytes())
        .unwrap();
    assert_eq!(store.create_source(op, "notes").unwrap(), source);
    assert_eq!(
        store
            .add_version(version_op, source, None, "naïve 🌍\r\n".as_bytes())
            .unwrap(),
        version
    );
    assert_eq!(
        store.version(version).unwrap().unwrap().bytes,
        "naïve 🌍\r\n".as_bytes()
    );
    assert!(store.create_source(op, "changed").is_err());
    drop(store);
    let (store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.interrupted_operations, 0);
    assert_eq!(
        store.operation(version_op).unwrap().unwrap().status,
        OperationStatus::Completed
    );
}

#[test]
fn opening_interrupts_nonterminal_operations_and_locks_owner() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let op = Uuid::new_v4();
    store.begin_operation(op, "work", b"x").unwrap();
    store.mark_running(op).unwrap();
    assert!(Store::open(dir.path()).is_err());
    drop(store);
    let (store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.interrupted_operations, 1);
    assert_eq!(
        store.operation(op).unwrap().unwrap().status,
        OperationStatus::Interrupted
    );
}

#[test]
fn parent_must_be_same_source_and_version_hash_is_checked_on_read() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let a = store.create_source(Uuid::new_v4(), "a").unwrap();
    let b = store.create_source(Uuid::new_v4(), "b").unwrap();
    let original = store.add_version(Uuid::new_v4(), a, None, b"one").unwrap();
    assert!(
        store
            .add_version(Uuid::new_v4(), b, Some(original), b"two")
            .is_err()
    );
    let next = store
        .add_version(Uuid::new_v4(), a, Some(original), b"two")
        .unwrap();
    assert_eq!(
        store.version(next).unwrap().unwrap().parent_id,
        Some(original)
    );
    let db = store.database_path().to_owned();
    drop(store);
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute(
        "UPDATE versions SET bytes=?1 WHERE id=?2",
        rusqlite::params![b"bad".as_slice(), next.to_string()],
    )
    .unwrap();
    drop(conn);
    let (store, _) = Store::open(dir.path()).unwrap();
    assert!(store.version(next).is_err());
}

#[test]
fn operation_state_is_monotonic_and_terminal_repeats_are_exact() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let op = Uuid::new_v4();
    assert_eq!(
        store
            .begin_operation(op, "provider.turn", b"request")
            .unwrap(),
        brn_store::BeginOperation::New
    );
    assert_eq!(
        store
            .begin_operation(op, "provider.turn", b"request")
            .unwrap(),
        brn_store::BeginOperation::Existing
    );
    assert!(
        store
            .begin_operation(op, "provider.turn", b"other")
            .is_err()
    );
    store.mark_running(op).unwrap();
    store
        .finish_operation(op, OperationStatus::Failed, b"unknown")
        .unwrap();
    store
        .finish_operation(op, OperationStatus::Failed, b"unknown")
        .unwrap();
    assert!(
        store
            .finish_operation(op, OperationStatus::Failed, b"changed")
            .is_err()
    );
    assert!(store.mark_running(op).is_err());
    assert!(
        store
            .begin_operation(Uuid::new_v4(), "source.create", b"x")
            .is_err()
    );
}

#[test]
fn sessions_and_messages_are_local_projection_with_replay() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let sop = Uuid::new_v4();
    let sid = store
        .create_session(
            sop,
            "codex",
            "store-A",
            Some("account"),
            Some("thread"),
            b"metadata",
        )
        .unwrap();
    assert_eq!(
        store
            .create_session(
                sop,
                "codex",
                "store-A",
                Some("account"),
                Some("thread"),
                b"metadata"
            )
            .unwrap(),
        sid
    );
    assert!(
        store
            .create_session(sop, "codex", "store-A", None, Some("thread"), b"metadata")
            .is_err()
    );
    let mop = Uuid::new_v4();
    let mid = store
        .add_message(mop, sid, "assistant", "hello\r\n🌍".as_bytes())
        .unwrap();
    assert_eq!(
        store
            .add_message(mop, sid, "assistant", "hello\r\n🌍".as_bytes())
            .unwrap(),
        mid
    );
    assert_eq!(store.messages(sid).unwrap().len(), 1);
    assert_eq!(
        store.messages(sid).unwrap()[0].content,
        "hello\r\n🌍".as_bytes()
    );
    assert_eq!(
        store.session(sid).unwrap().unwrap().provider_store,
        "store-A"
    );
}

#[test]
fn refuses_foreign_newer_and_unexpected_schema_without_replacing_bytes() {
    for flavor in [
        "foreign",
        "newer",
        "unexpected",
        "prefix_trick",
        "corrupt",
        "sidecar",
    ] {
        let dir = tempdir().unwrap();
        let db = dir.path().join("brn.sqlite3");
        if flavor == "sidecar" {
            std::fs::write(dir.path().join("brn.sqlite3-journal"), b"orphan").unwrap();
        } else {
            let (store, _) = Store::open(dir.path()).unwrap();
            drop(store);
            if flavor == "corrupt" {
                std::fs::write(&db, b"this is not sqlite").unwrap();
            } else {
                let conn = rusqlite::Connection::open(&db).unwrap();
                match flavor {
                    "foreign" => conn
                        .pragma_update(None, "application_id", 0x12345678u32)
                        .unwrap(),
                    "newer" => conn.pragma_update(None, "user_version", 999u32).unwrap(),
                    "unexpected" => conn.execute_batch("CREATE TABLE surprise(x);").unwrap(),
                    "prefix_trick" => conn.execute_batch("CREATE TABLE sqliteXevil(x);").unwrap(),
                    _ => unreachable!(),
                }
            }
        }
        let before = if db.exists() {
            Some(std::fs::read(&db).unwrap())
        } else {
            None
        };
        assert!(Store::open(dir.path()).is_err(), "{flavor}");
        assert_eq!(
            if db.exists() {
                Some(std::fs::read(&db).unwrap())
            } else {
                None
            },
            before,
            "{flavor}"
        );
    }
}

#[test]
fn aliases_to_database_and_owner_lock_are_rejected() {
    use std::os::unix::fs::symlink;
    for alias in ["db_link", "db_hardlink", "lock_link"] {
        let dir = tempdir().unwrap();
        let (store, _) = Store::open(dir.path()).unwrap();
        drop(store);
        let db = dir.path().join("brn.sqlite3");
        let lock = dir.path().join("brn.owner.lock");
        match alias {
            "db_link" => {
                std::fs::rename(&db, dir.path().join("actual.db")).unwrap();
                symlink(dir.path().join("actual.db"), &db).unwrap();
            }
            "db_hardlink" => {
                std::fs::hard_link(&db, dir.path().join("alias.db")).unwrap();
            }
            "lock_link" => {
                std::fs::rename(&lock, dir.path().join("actual.lock")).unwrap();
                symlink(dir.path().join("actual.lock"), &lock).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(Store::open(dir.path()).is_err(), "{alias}");
    }
}

#[test]
fn restart_can_discover_records_without_prior_ids() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let source = store.create_source(Uuid::new_v4(), "notes").unwrap();
    store
        .add_version(Uuid::new_v4(), source, None, b"hello")
        .unwrap();
    store
        .create_session(Uuid::new_v4(), "codex", "store-A", None, None, b"{}")
        .unwrap();
    let op = Uuid::new_v4();
    store.begin_operation(op, "provider.turn", b"x").unwrap();
    drop(store);
    let (store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.interrupted_operations, 1);
    assert_eq!(store.sources().unwrap()[0].0, source);
    assert_eq!(store.versions(source).unwrap()[0].bytes, b"hello");
    assert_eq!(store.sessions().unwrap().len(), 1);
    assert_eq!(store.interrupted_operations().unwrap(), vec![op]);
}

#[test]
fn killed_process_preserves_acknowledged_rows_and_interrupts_pending_work() {
    let dir = tempdir().unwrap();
    let (store, _) = Store::open(dir.path()).unwrap();
    drop(store);
    kill_after_ack(dir.path(), "ack", None);
    let (mut store, report) = Store::open(dir.path()).unwrap();
    let sources = store.sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        store.versions(sources[0].0).unwrap()[0].bytes,
        b"acknowledged version"
    );
    let sessions = store.sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        store.messages(sessions[0].id).unwrap()[0].content,
        b"acknowledged message"
    );
    assert_eq!(report.interrupted_operations, 2);
    assert_eq!(store.interrupted_operations().unwrap().len(), 2);
    for (id, payload) in [
        (Uuid::from_u128(1), b"pending".as_slice()),
        (Uuid::from_u128(2), b"running".as_slice()),
    ] {
        assert_eq!(
            store.begin_operation(id, "provider.turn", payload).unwrap(),
            brn_store::BeginOperation::Existing
        );
        assert!(store.mark_running(id).is_err());
    }
}

#[test]
fn killed_uncommitted_spilled_transaction_recovers_original_bytes() {
    let dir = tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let source = store.create_source(Uuid::new_v4(), "large").unwrap();
    let original = vec![b'a'; 3_000_000];
    let version = store
        .add_version(Uuid::new_v4(), source, None, &original)
        .unwrap();
    drop(store);
    kill_after_ack(dir.path(), "spill", Some(version));
    let (store, _) = Store::open(dir.path()).unwrap();
    assert_eq!(store.version(version).unwrap().unwrap().bytes, original);
}

#[test]
fn killed_migration_recovers_header_and_then_upgrades_v1() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("brn.sqlite3");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE sources (id TEXT PRIMARY KEY, title TEXT NOT NULL);CREATE TABLE versions (id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id), parent_id TEXT REFERENCES versions(id), bytes BLOB NOT NULL, sha256 BLOB NOT NULL);").unwrap();
    conn.pragma_update(None, "application_id", 0x4252_4e31u32)
        .unwrap();
    conn.pragma_update(None, "user_version", 1).unwrap();
    conn.pragma_update(None, "journal_mode", "DELETE").unwrap();
    let source = Uuid::new_v4();
    let original = "a".repeat(3_000_000);
    conn.execute(
        "INSERT INTO sources(id,title) VALUES(?1,?2)",
        rusqlite::params![source.to_string(), original],
    )
    .unwrap();
    drop(conn);
    kill_after_ack(dir.path(), "migration", None);
    let (store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.migrated_from, Some(1));
    assert_eq!(store.source_title(source).unwrap().unwrap(), original);
}

fn kill_after_ack(dir: &std::path::Path, mode: &str, version: Option<Uuid>) {
    use sha2::{Digest, Sha256};
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    let baseline = format!(
        "{:x}",
        Sha256::digest(std::fs::read(dir.join("brn.sqlite3")).unwrap())
    );
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "child_worker", "--nocapture"])
        .env("BRN_CHILD_MODE", mode)
        .env("BRN_CHILD_DIR", dir)
        .env("BRN_CHILD_DBHASH", baseline)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());
    if let Some(id) = version {
        cmd.env("BRN_CHILD_VERSION", id.to_string());
    }
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = ChildGuard(cmd.spawn().unwrap());
    let stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            if line.contains("ACK") {
                let _ = tx.send(());
                return;
            }
        }
    });
    if rx.recv_timeout(Duration::from_secs(10)).is_err() {
        let mut stderr = String::new();
        use std::io::Read;
        let _ = child.0.kill();
        let _ = child.0.stderr.take().unwrap().read_to_string(&mut stderr);
        panic!("child did not acknowledge: {stderr}");
    }
    assert!(
        Store::open(dir).is_err(),
        "child process must retain directory ownership"
    );
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    assert!(!status.success());
    reader.join().unwrap();
}

#[test]
fn child_worker() {
    use std::io::{Read, Write};
    let Ok(mode) = std::env::var("BRN_CHILD_MODE") else {
        return;
    };
    let dir = std::env::var("BRN_CHILD_DIR").unwrap();
    use sha2::{Digest, Sha256};
    let db = std::path::Path::new(&dir).join("brn.sqlite3");
    let _migration_owner = if mode == "migration" {
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(std::path::Path::new(&dir).join("brn.owner.lock"))
            .unwrap();
        lock.try_lock().unwrap();
        Some(lock)
    } else {
        None
    };
    let mut store = if mode == "migration" {
        None
    } else {
        Some(Store::open(&dir).unwrap().0)
    };
    match mode.as_str() {
        "ack" => {
            let st = store.as_mut().unwrap();
            let source = st.create_source(Uuid::new_v4(), "acknowledged").unwrap();
            st.add_version(Uuid::new_v4(), source, None, b"acknowledged version")
                .unwrap();
            let session = st
                .create_session(Uuid::new_v4(), "codex", "store-A", None, None, b"{}")
                .unwrap();
            st.add_message(
                Uuid::new_v4(),
                session,
                "assistant",
                b"acknowledged message",
            )
            .unwrap();
            st.begin_operation(Uuid::from_u128(1), "provider.turn", b"pending")
                .unwrap();
            let running = Uuid::from_u128(2);
            st.begin_operation(running, "provider.turn", b"running")
                .unwrap();
            st.mark_running(running).unwrap();
        }
        "spill" => {
            let id = std::env::var("BRN_CHILD_VERSION").unwrap();
            let conn = rusqlite::Connection::open(&db).unwrap();
            conn.execute_batch("PRAGMA cache_size=10; PRAGMA cache_spill=ON; BEGIN IMMEDIATE;")
                .unwrap();
            conn.execute(
                "UPDATE versions SET bytes=zeroblob(3000000) WHERE id=?1",
                [id],
            )
            .unwrap();
            let journal = dir.to_string() + "/brn.sqlite3-journal";
            assert!(std::fs::metadata(journal).unwrap().len() > 512);
            std::mem::forget(conn);
        }
        "migration" => {
            let conn = rusqlite::Connection::open(&db).unwrap();
            conn.execute_batch("PRAGMA cache_size=1; PRAGMA cache_spill=ON; BEGIN IMMEDIATE; CREATE TABLE operations (id TEXT PRIMARY KEY, kind TEXT NOT NULL); PRAGMA user_version=2;").unwrap();
            conn.execute("UPDATE sources SET title=?1", ["b".repeat(3_000_000)])
                .unwrap();
            let journal = dir.to_string() + "/brn.sqlite3-journal";
            assert!(std::fs::metadata(journal).unwrap().len() > 512);
            // SQLite may keep page 1 in cache until commit. Simulate its page-1
            // write after the valid rollback journal is durable, then kill.
            std::mem::forget(conn);
            use std::io::{Seek, SeekFrom};
            let mut file = std::fs::OpenOptions::new().write(true).open(&db).unwrap();
            file.seek(SeekFrom::Start(60)).unwrap();
            file.write_all(&2u32.to_be_bytes()).unwrap();
            file.sync_all().unwrap();
            let bytes = std::fs::read(&db).unwrap();
            assert_eq!(u32::from_be_bytes(bytes[60..64].try_into().unwrap()), 2);
        }
        _ => panic!("unknown child mode"),
    }
    let actual = format!("{:x}", Sha256::digest(std::fs::read(&db).unwrap()));
    assert_ne!(
        actual,
        std::env::var("BRN_CHILD_DBHASH").unwrap(),
        "main database must have changed before kill"
    );
    println!("ACK");
    std::io::stdout().flush().unwrap();
    let mut input = [0u8; 1];
    let _ = std::io::stdin().read(&mut input);
}

#[test]
fn recognized_header_with_corrupt_body_or_foreign_keys_is_refused_unchanged() {
    for flavor in ["truncated", "orphan"] {
        let dir = tempdir().unwrap();
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.create_source(Uuid::new_v4(), "seed").unwrap();
        let db = store.database_path().to_owned();
        drop(store);
        match flavor {
            "truncated" => {
                let file = std::fs::OpenOptions::new().write(true).open(&db).unwrap();
                file.set_len(100).unwrap();
            }
            "orphan" => {
                let conn = rusqlite::Connection::open(&db).unwrap();
                conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
                conn.execute("INSERT INTO versions(id,source_id,parent_id,bytes,sha256) VALUES(?1,?2,NULL,?3,?4)",rusqlite::params![Uuid::new_v4().to_string(),Uuid::new_v4().to_string(),b"x".as_slice(),[0u8;32].as_slice()]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = std::fs::read(&db).unwrap();
        assert!(Store::open(dir.path()).is_err(), "{flavor}");
        assert_eq!(std::fs::read(&db).unwrap(), before, "{flavor}");
    }
}
