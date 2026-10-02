# Step 2: Store and vault

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the new user-work database (`brn.sqlite`: open, integrity check, restore, backups, settings, unsaved edits) and the vault module (path rules, scan, read) beside the existing code.

**Architecture:** `brn_store::work::WorkStore` owns a data folder (owner lock, `brn.sqlite`, `backups/`), separate from the old `Store` and its `brn.sqlite3`. `brn_workflow::vault` decides which files are notes and reads them without following symlinks. Nothing old is changed or removed; wiring into CLI/desktop happens in steps 3–4.

**Tech Stack:** Rust `1.98.1`, rusqlite `=0.40.2` (bundled), sha2, tempfile for tests.

**Spec:** [Simple Rig-based notes app](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md), sections 3, 4 and 11; [roadmap](plan.md).

## Global Constraints

- Inherit the [roadmap](plan.md) constraints.
- Notes: regular `.md` files up to `1_048_576` bytes; hidden files/folders (name starts with `.`) and the top-level `archive/` folder (ASCII case-insensitive) are excluded; symlinks are never followed; paths use `/` separators.
- `brn.sqlite`: `PRAGMA quick_check` on every open; on corruption, move the file aside to `brn.sqlite.corrupt-<millis>` and restore the newest backup; then back up to `backups/brn-<millis>.sqlite`, keeping the 5 newest.
- A newer schema or a foreign database is an error, never moved aside or replaced.
- No credentials in `brn.sqlite`.

## File map

| Path | Responsibility |
| --- | --- |
| `crates/brn-store/src/work/mod.rs` | `WorkStore`: lock, open/check/migrate, settings |
| `crates/brn-store/src/work/backup.rs` | Create, list, prune and restore backups; move a corrupt file aside |
| `crates/brn-store/src/work/edits.rs` | Unsaved editor text per note path |
| `crates/brn-store/tests/work.rs` | Store behaviour tests |
| `crates/brn-workflow/src/vault/mod.rs` | Module exports |
| `crates/brn-workflow/src/vault/path.rs` | `VaultPath` rules |
| `crates/brn-workflow/src/vault/scan.rs` | Listing notes in the vault |
| `crates/brn-workflow/src/vault/read.rs` | Reading one note safely |
| `crates/brn-workflow/tests/vault.rs` | Vault behaviour tests |

---

### Task 1: WorkStore open, migrations and settings

**Files:**
- Create: `crates/brn-store/src/work/mod.rs`
- Create: `crates/brn-store/src/work/backup.rs` (minimal in this task, completed in Task 2)
- Create: `crates/brn-store/tests/work.rs`
- Modify: `crates/brn-store/Cargo.toml` (rusqlite features become `["bundled", "backup"]`; no new packages, so `--locked` still works)
- Modify: `crates/brn-store/src/lib.rs` (add module and exports after line 18 `mod workflow;`)

**Interfaces:**
- Consumes: private crate-root helpers in `crates/brn-store/src/lib.rs`: `acquire_owner_lock`, `check_regular_single_link`, `invalid`, plus `Error`/`Result`.
- Produces:

```rust
pub struct WorkStore { /* private */ }
pub struct OpenReport {
    pub corrupt_moved_to: Option<std::path::PathBuf>,
    pub restored_from: Option<std::path::PathBuf>,
    pub backup: std::path::PathBuf,
}
pub const MAX_NOTE_BYTES: usize = 1024 * 1024;
impl WorkStore {
    pub fn open(data_dir: &std::path::Path) -> brn_store::Result<(WorkStore, OpenReport)>;
    pub fn data_dir(&self) -> &std::path::Path;
    pub fn setting(&self, key: &str) -> brn_store::Result<Option<String>>;
    pub fn set_setting(&mut self, key: &str, value: &str) -> brn_store::Result<()>;
    pub fn remove_setting(&mut self, key: &str) -> brn_store::Result<()>;
}
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-store/tests/work.rs`:

```rust
use brn_store::{Error, WorkStore};

#[test]
fn settings_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, None);
    assert_eq!(report.corrupt_moved_to, None);
    store.set_setting("vault", "/synthetic/vault").unwrap();
    store.set_setting("vault", "/synthetic/vault2").unwrap();
    store.set_setting("model", "gpt").unwrap();
    store.remove_setting("model").unwrap();
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.setting("vault").unwrap().as_deref(), Some("/synthetic/vault2"));
    assert_eq!(store.setting("model").unwrap(), None);
    assert_eq!(store.data_dir(), dir.path());
}

#[test]
fn second_owner_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (_store, _) = WorkStore::open(dir.path()).unwrap();
    assert!(matches!(WorkStore::open(dir.path()), Err(Error::WorkspaceBusy(_))));
}

#[test]
fn missing_data_dir_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        WorkStore::open(&dir.path().join("absent")),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn newer_schema_is_refused_and_left_in_place() {
    let dir = tempfile::tempdir().unwrap();
    drop(WorkStore::open(dir.path()).unwrap());
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.pragma_update(None, "user_version", 99).unwrap();
    drop(raw);
    assert!(matches!(WorkStore::open(dir.path()), Err(Error::Invalid(_))));
    assert!(dir.path().join("brn.sqlite").exists());
    let moved = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| {
            e.as_ref().unwrap().file_name().to_string_lossy().contains("corrupt")
        })
        .count();
    assert_eq!(moved, 0);
}

#[test]
fn foreign_database_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.pragma_update(None, "application_id", 12345).unwrap();
    drop(raw);
    assert!(matches!(WorkStore::open(dir.path()), Err(Error::Invalid(_))));
}

#[test]
fn unbranded_database_is_refused_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.execute_batch("CREATE TABLE other (x INTEGER)").unwrap();
    drop(raw);
    assert!(matches!(WorkStore::open(dir.path()), Err(Error::Invalid(_))));
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let tables: Vec<String> = raw
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tables, vec!["other".to_string()]);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-store --locked --test work`
Expected: compile error, `WorkStore` not found in `brn_store`.

- [ ] **Step 3: Create `crates/brn-store/src/work/mod.rs`**

```rust
//! User-work database for the simple notes app (`brn.sqlite`). It is checked
//! on every open, restored from the newest backup when corrupt, and backed
//! up after every successful open. Notes themselves live in the vault.
mod backup;
mod edits;

use crate::{Result, acquire_owner_lock, check_regular_single_link, invalid};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub use edits::UnsavedEdit;

/// Largest note, unsaved edit or proposal text, in bytes.
pub const MAX_NOTE_BYTES: usize = 1024 * 1024;
const APPLICATION_ID: i64 = 0x4252_4e32; // BRN2
const DB_NAME: &str = "brn.sqlite";
/// Each entry upgrades the schema by one version; `user_version` is the number applied.
const MIGRATIONS: &[&str] = &["CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE unsaved_edits (
        path TEXT PRIMARY KEY,
        base_sha256 BLOB NOT NULL CHECK(length(base_sha256) = 32),
        text TEXT NOT NULL,
        updated_at_ms INTEGER NOT NULL
    );"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    /// Where a corrupt `brn.sqlite` was moved, if one was found.
    pub corrupt_moved_to: Option<PathBuf>,
    /// The backup that replaced it, or `None` if a fresh database was created.
    pub restored_from: Option<PathBuf>,
    /// The backup made by this open.
    pub backup: PathBuf,
}

pub struct WorkStore {
    conn: Connection,
    dir: PathBuf,
    _owner_lock: File,
}

/// What an existing database file turned out to be.
enum Checked {
    /// A BRN work database with a supported schema.
    Brn(Connection),
    /// A healthy but empty SQLite file, for example left by an interrupted first start.
    Empty(Connection),
    /// Damaged, or not SQLite at all.
    Corrupt,
    /// A healthy database BRN must not touch (another application's, or a newer BRN's).
    Foreign(&'static str),
}

impl WorkStore {
    pub fn open(data_dir: &Path) -> Result<(Self, OpenReport)> {
        if !data_dir.is_dir() {
            return Err(invalid("data directory must already exist"));
        }
        let lock = lock_dir(data_dir)?;
        let db = data_dir.join(DB_NAME);
        let existing = if db.exists() {
            check_regular_single_link(&db)?;
            Some(check(&db)?)
        } else {
            None
        };
        let mut corrupt_moved_to = None;
        let mut restored_from = None;
        let mut conn = match existing {
            Some(Checked::Brn(conn) | Checked::Empty(conn)) => conn,
            Some(Checked::Foreign(reason)) => return Err(invalid(reason)),
            // Missing or corrupt: restore the newest usable backup, or start fresh.
            Some(Checked::Corrupt) | None => {
                corrupt_moved_to = backup::move_aside(&db)?;
                let (conn, from) = backup::restore_newest(data_dir, &db)?;
                restored_from = from;
                conn
            }
        };
        configure(&conn)?;
        migrate(&mut conn)?;
        let backup = backup::create(data_dir, &conn)?;
        backup::prune(data_dir)?;
        Ok((
            Self {
                conn,
                dir: data_dir.to_path_buf(),
                _owner_lock: lock,
            },
            OpenReport {
                corrupt_moved_to,
                restored_from,
                backup,
            },
        ))
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&mut self, key: &str) -> Result<()> {
        self.conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }
}

fn lock_dir(dir: &Path) -> Result<File> {
    let path = dir.join("brn.owner.lock");
    if path.exists() {
        check_regular_single_link(&path)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    acquire_owner_lock(|| file.try_lock(), Instant::now() + Duration::from_secs(1))?;
    Ok(file)
}

fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt)
    )
}

/// Opens and checks an existing database without changing it.
fn check(db: &Path) -> Result<Checked> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = match Connection::open_with_flags(db, flags) {
        Ok(conn) => conn,
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    };
    match conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(result) if result == "ok" => {}
        Ok(_) => return Ok(Checked::Corrupt),
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    }
    let application: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if application == APPLICATION_ID {
        if (1..=MIGRATIONS.len() as i64).contains(&version) {
            return Ok(Checked::Brn(conn));
        }
        return Ok(Checked::Foreign("brn.sqlite has an unsupported (newer?) BRN schema"));
    }
    let objects: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
    if application == 0 && version == 0 && objects == 0 {
        return Ok(Checked::Empty(conn));
    }
    Ok(Checked::Foreign("brn.sqlite belongs to another application"))
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "trusted_schema", "OFF")?;
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(invalid("brn.sqlite could not enable WAL mode"));
    }
    conn.pragma_update(None, "synchronous", "FULL")?;
    #[cfg(target_os = "macos")]
    conn.pragma_update(None, "fullfsync", "ON")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let tx = conn.transaction()?;
    for sql in &MIGRATIONS[version as usize..] {
        tx.execute_batch(sql)?;
    }
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", MIGRATIONS.len() as i64)?;
    tx.commit()?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
```

- [ ] **Step 4: Create `crates/brn-store/src/work/backup.rs`** (complete version; Task 2 adds its tests):

```rust
use super::{Checked, check, now_ms};
use crate::{Result, invalid};
use rusqlite::Connection;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

const KEEP: usize = 5;

fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// The number in a backup file name `brn-<number>.sqlite`.
fn stamp(path: &Path) -> Option<u64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("brn-")?
        .strip_suffix(".sqlite")?
        .parse()
        .ok()
}

/// Backups, oldest first.
pub(super) fn list(data_dir: &Path) -> Result<Vec<PathBuf>> {
    let dir = backups_dir(data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(number) = stamp(&path)
            && entry.file_type()?.is_file()
        {
            backups.push((number, path));
        }
    }
    backups.sort();
    Ok(backups.into_iter().map(|(_, path)| path).collect())
}

/// Writes a consistent copy with SQLite's online backup API. The number is the
/// current time in milliseconds, but always above every existing backup, so the
/// new copy is the newest even if the clock went backwards.
pub(super) fn create(data_dir: &Path, conn: &Connection) -> Result<PathBuf> {
    std::fs::create_dir_all(backups_dir(data_dir))?;
    let after_last = list(data_dir)?
        .last()
        .and_then(|path| stamp(path))
        .map_or(0, |last| last + 1);
    let path = backups_dir(data_dir).join(format!("brn-{:013}.sqlite", now_ms().max(after_last)));
    conn.backup("main", &path, None)?;
    Ok(path)
}

pub(super) fn prune(data_dir: &Path) -> Result<()> {
    let backups = list(data_dir)?;
    if backups.len() > KEEP {
        for old in &backups[..backups.len() - KEEP] {
            std::fs::remove_file(old)?;
        }
    }
    Ok(())
}

/// Renames a database and its WAL/SHM files out of the way. Returns the new
/// name of the database file, or `None` if only leftover WAL/SHM files existed.
pub(super) fn move_aside(db: &Path) -> Result<Option<PathBuf>> {
    let parent = db
        .parent()
        .ok_or_else(|| invalid("database path has no parent folder"))?;
    let mut ms = now_ms();
    let target = loop {
        let candidate = parent.join(format!("brn.sqlite.corrupt-{ms:013}"));
        if !candidate.exists() {
            break candidate;
        }
        ms += 1;
    };
    let moved = if db.exists() {
        std::fs::rename(db, &target)?;
        Some(target.clone())
    } else {
        None
    };
    for suffix in ["-wal", "-shm"] {
        let side = with_suffix(db, suffix);
        if side.exists() {
            std::fs::rename(&side, with_suffix(&target, suffix))?;
        }
    }
    Ok(moved)
}

fn remove_with_sidecars(db: &Path) -> Result<()> {
    for path in [db.to_path_buf(), with_suffix(db, "-wal"), with_suffix(db, "-shm")] {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Copies the newest usable BRN backup to `db`, skipping damaged, empty or
/// foreign files. Falls back to a fresh, empty database.
pub(super) fn restore_newest(data_dir: &Path, db: &Path) -> Result<(Connection, Option<PathBuf>)> {
    for backup in list(data_dir)?.into_iter().rev() {
        std::fs::copy(&backup, db)?;
        if let Checked::Brn(conn) = check(db)? {
            return Ok((conn, Some(backup)));
        }
        remove_with_sidecars(db)?;
    }
    Ok((Connection::open(db)?, None))
}
```

- [ ] **Step 5: Create an empty `crates/brn-store/src/work/edits.rs`** so the module compiles (Task 3 fills it):

```rust
/// Unsaved editor text for one note (filled in by Task 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedEdit;
```

- [ ] **Step 6: Export the module and enable the backup API.** In `crates/brn-store/src/lib.rs`, directly after `mod workflow;`:

```rust
pub mod work;
pub use work::{MAX_NOTE_BYTES, OpenReport, UnsavedEdit, WorkStore};
```

In `crates/brn-store/Cargo.toml`, change the rusqlite line to:

```toml
rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled", "backup"] }
```

- [ ] **Step 7: Run the tests**

Run: `cargo test -p brn-store --locked --test work`
Expected: 6 passed. `second_owner_is_rejected` takes about 1 second (lock retry window).

- [ ] **Step 8: Run the existing store tests and Clippy**

Run: `cargo test -p brn-store --locked && cargo clippy -p brn-store --all-targets --locked -- -D warnings`
Expected: all pass, no warnings. If Clippy flags the placeholder `UnsavedEdit` as unused, leave it; Task 3 replaces it.

- [ ] **Step 9: Commit**

```bash
git add crates/brn-store/Cargo.toml crates/brn-store/src/work crates/brn-store/src/lib.rs crates/brn-store/tests/work.rs
git commit -m "feat(store): add brn.sqlite work store with settings

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Backups and corruption recovery

**Files:**
- Modify: `crates/brn-store/tests/work.rs` (add tests)
- `crates/brn-store/src/work/backup.rs` already contains the implementation from Task 1; fix it here if a test fails.

**Interfaces:**
- Consumes: Task 1 `WorkStore::open`, `OpenReport`.
- Produces: tested behaviour only.

- [ ] **Step 1: Add the tests** to `crates/brn-store/tests/work.rs`:

```rust
fn backups(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found: Vec<_> = std::fs::read_dir(dir.join("backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("brn-") && name.ends_with(".sqlite")
        })
        .collect();
    found.sort();
    found
}

fn corrupt(dir: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.join(format!("brn.sqlite{suffix}")));
    }
    std::fs::write(dir.join("brn.sqlite"), vec![0x42u8; 8192]).unwrap();
}

#[test]
fn each_open_backs_up_and_keeps_five() {
    let dir = tempfile::tempdir().unwrap();
    let mut made = Vec::new();
    for _ in 0..7 {
        let (_store, report) = WorkStore::open(dir.path()).unwrap();
        assert!(report.backup.exists());
        made.push(report.backup);
    }
    assert_eq!(backups(dir.path()), made[2..].to_vec());
}

#[test]
fn corrupt_database_is_restored_from_newest_backup() {
    let dir = tempfile::tempdir().unwrap();
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "first").unwrap();
    }
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "second").unwrap();
    }
    let newest = WorkStore::open(dir.path()).unwrap().1.backup;
    corrupt(dir.path());
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.setting("k").unwrap().as_deref(), Some("second"));
    assert_eq!(report.restored_from, Some(newest));
    let moved = report.corrupt_moved_to.unwrap();
    assert!(moved.exists());
    assert!(moved.file_name().unwrap().to_string_lossy().starts_with("brn.sqlite.corrupt-"));
}

#[test]
fn corrupt_database_without_backups_starts_fresh() {
    let dir = tempfile::tempdir().unwrap();
    corrupt(dir.path());
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, None);
    assert!(report.corrupt_moved_to.unwrap().exists());
    assert_eq!(store.setting("k").unwrap(), None);
}

#[test]
fn unusable_newest_backup_falls_back_to_older_one() {
    for damage in [vec![0x42u8; 8192], Vec::new()] {
        let dir = tempfile::tempdir().unwrap();
        {
            let (mut store, _) = WorkStore::open(dir.path()).unwrap();
            store.set_setting("k", "kept").unwrap();
        }
        let older = WorkStore::open(dir.path()).unwrap().1.backup;
        let newest = WorkStore::open(dir.path()).unwrap().1.backup;
        std::fs::write(&newest, &damage).unwrap();
        corrupt(dir.path());
        let (store, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(report.restored_from, Some(older), "damage of {} bytes", damage.len());
        assert_eq!(store.setting("k").unwrap().as_deref(), Some("kept"));
    }
}

#[test]
fn missing_database_is_restored_from_newest_backup() {
    let dir = tempfile::tempdir().unwrap();
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "kept").unwrap();
    }
    let newest = WorkStore::open(dir.path()).unwrap().1.backup;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.path().join(format!("brn.sqlite{suffix}")));
    }
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(newest));
    assert_eq!(report.corrupt_moved_to, None);
    assert_eq!(store.setting("k").unwrap().as_deref(), Some("kept"));
}

#[test]
fn new_backup_sorts_after_existing_ones_even_with_future_names() {
    let dir = tempfile::tempdir().unwrap();
    let first = WorkStore::open(dir.path()).unwrap().1.backup;
    let future = dir.path().join("backups/brn-9000000000000.sqlite");
    std::fs::copy(&first, &future).unwrap();
    let (_store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        report.backup.file_name().unwrap().to_string_lossy(),
        "brn-9000000000001.sqlite"
    );
    assert!(report.backup.exists());
    assert_eq!(backups(dir.path()).last(), Some(&report.backup));
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p brn-store --locked --test work`
Expected: 12 passed. If `unusable_newest_backup_falls_back_to_older_one` fails because `prune` deleted `older`, check that only 5 are kept and `older` is among them (it is: only 3 backups exist at that point).

- [ ] **Step 3: Commit**

```bash
git add crates/brn-store/tests/work.rs crates/brn-store/src/work/backup.rs
git commit -m "test(store): cover brn.sqlite backups and corruption recovery

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Unsaved edits

**Files:**
- Modify: `crates/brn-store/src/work/edits.rs`
- Modify: `crates/brn-store/tests/work.rs`

**Interfaces:**
- Consumes: Task 1 `WorkStore` (private `conn` field is visible to the child module), `MAX_NOTE_BYTES`, `now_ms`.
- Produces:

```rust
pub struct UnsavedEdit {
    pub path: String,
    pub base_sha256: [u8; 32],
    pub text: String,
    pub updated_at_ms: u64,
}
impl WorkStore {
    pub fn put_unsaved_edit(&mut self, path: &str, base_sha256: [u8; 32], text: &str) -> brn_store::Result<()>;
    pub fn unsaved_edit(&self, path: &str) -> brn_store::Result<Option<UnsavedEdit>>;
    pub fn unsaved_edits(&self) -> brn_store::Result<Vec<UnsavedEdit>>; // ordered by path
    pub fn clear_unsaved_edit(&mut self, path: &str) -> brn_store::Result<()>;
}
```

`path` is a vault-relative note path already validated by `brn_workflow::vault::VaultPath`; the store only rejects an empty one.

- [ ] **Step 1: Add the failing tests** to `crates/brn-store/tests/work.rs`:

```rust
#[test]
fn unsaved_edits_round_trip_replace_and_clear() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store.put_unsaved_edit("notes/a.md", [7; 32], "draft é\r\n").unwrap();
    store.put_unsaved_edit("b.md", [1; 32], "").unwrap();
    let edit = store.unsaved_edit("notes/a.md").unwrap().unwrap();
    assert_eq!(edit.path, "notes/a.md");
    assert_eq!(edit.base_sha256, [7; 32]);
    assert_eq!(edit.text, "draft é\r\n");
    assert!(edit.updated_at_ms > 0);
    store.put_unsaved_edit("notes/a.md", [8; 32], "newer").unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let all = store.unsaved_edits().unwrap();
    assert_eq!(
        all.iter().map(|e| (e.path.as_str(), e.text.as_str())).collect::<Vec<_>>(),
        vec![("b.md", ""), ("notes/a.md", "newer")]
    );
    store.clear_unsaved_edit("notes/a.md").unwrap();
    assert_eq!(store.unsaved_edit("notes/a.md").unwrap(), None);
}

#[test]
fn unsaved_edit_limits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let at_limit = "a".repeat(brn_store::MAX_NOTE_BYTES);
    store.put_unsaved_edit("a.md", [0; 32], &at_limit).unwrap();
    let over = "a".repeat(brn_store::MAX_NOTE_BYTES + 1);
    assert!(matches!(
        store.put_unsaved_edit("a.md", [0; 32], &over),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(store.put_unsaved_edit("", [0; 32], "x"), Err(Error::Invalid(_))));
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-store --locked --test work unsaved`
Expected: compile error, no method `put_unsaved_edit`.

- [ ] **Step 3: Replace `crates/brn-store/src/work/edits.rs`**

```rust
use super::{MAX_NOTE_BYTES, WorkStore, now_ms};
use crate::{Result, invalid};
use rusqlite::{OptionalExtension, Row, params};

/// Unsaved editor text for one note, kept so it can be offered back after a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedEdit {
    pub path: String,
    /// SHA-256 of the saved note the edit started from.
    pub base_sha256: [u8; 32],
    pub text: String,
    pub updated_at_ms: u64,
}

fn read(row: &Row<'_>) -> rusqlite::Result<(String, Vec<u8>, String, i64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn convert((path, base, text, updated): (String, Vec<u8>, String, i64)) -> Result<UnsavedEdit> {
    let base_sha256 = base
        .try_into()
        .map_err(|_| invalid("stored unsaved edit has an invalid hash"))?;
    Ok(UnsavedEdit {
        path,
        base_sha256,
        text,
        updated_at_ms: updated.max(0) as u64,
    })
}

impl WorkStore {
    pub fn put_unsaved_edit(&mut self, path: &str, base_sha256: [u8; 32], text: &str) -> Result<()> {
        if path.is_empty() {
            return Err(invalid("unsaved edit needs a note path"));
        }
        if text.len() > MAX_NOTE_BYTES {
            return Err(invalid("unsaved edit exceeds the 1 MiB note limit"));
        }
        self.conn.execute(
            "INSERT INTO unsaved_edits(path, base_sha256, text, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET base_sha256 = excluded.base_sha256,
                 text = excluded.text, updated_at_ms = excluded.updated_at_ms",
            params![path, &base_sha256[..], text, now_ms() as i64],
        )?;
        Ok(())
    }

    pub fn unsaved_edit(&self, path: &str) -> Result<Option<UnsavedEdit>> {
        self.conn
            .query_row(
                "SELECT path, base_sha256, text, updated_at_ms FROM unsaved_edits WHERE path = ?1",
                [path],
                read,
            )
            .optional()?
            .map(convert)
            .transpose()
    }

    pub fn unsaved_edits(&self) -> Result<Vec<UnsavedEdit>> {
        let mut statement = self.conn.prepare(
            "SELECT path, base_sha256, text, updated_at_ms FROM unsaved_edits ORDER BY path",
        )?;
        let rows = statement.query_map([], read)?;
        rows.map(|row| convert(row?)).collect()
    }

    pub fn clear_unsaved_edit(&mut self, path: &str) -> Result<()> {
        self.conn.execute("DELETE FROM unsaved_edits WHERE path = ?1", [path])?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run all store tests and Clippy**

Run: `cargo test -p brn-store --locked && cargo clippy -p brn-store --all-targets --locked -- -D warnings`
Expected: all pass (14 in `work`), no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/brn-store/src/work/edits.rs crates/brn-store/tests/work.rs
git commit -m "feat(store): keep unsaved editor text in brn.sqlite

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Vault path rules

**Files:**
- Create: `crates/brn-workflow/src/vault/mod.rs`
- Create: `crates/brn-workflow/src/vault/path.rs`
- Create: `crates/brn-workflow/tests/vault.rs`
- Modify: `crates/brn-workflow/src/lib.rs` (add `pub mod vault;` after line 6 `pub mod worker;`)

**Interfaces:**
- Produces:

```rust
pub struct VaultPath(/* private String */); // Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display
pub enum VaultPathError { Empty, Absolute, InvalidComponent, Traversal, Hidden, Archived, NotMarkdown }
impl VaultPath {
    pub fn parse(raw: &str) -> Result<VaultPath, VaultPathError>;
    pub fn as_str(&self) -> &str;
    pub fn to_fs_path(&self, root: &std::path::Path) -> std::path::PathBuf;
}
pub(crate) fn is_markdown_name(name: &str) -> bool;
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-workflow/tests/vault.rs`:

```rust
use brn_workflow::vault::{VaultPath, VaultPathError};

#[test]
fn accepts_note_paths() {
    for raw in [
        "a.md",
        "notes/plan.md",
        "deep/er/x.MD",
        "archive.md",
        "archived/x.md",
        "sub/archive/x.md",
        "unicode/é note.md",
    ] {
        assert_eq!(VaultPath::parse(raw).unwrap().as_str(), raw, "{raw}");
    }
}

#[test]
fn rejects_non_note_paths() {
    use VaultPathError::*;
    for (raw, expected) in [
        ("", Empty),
        ("/abs.md", Absolute),
        ("a.txt", NotMarkdown),
        ("noext", NotMarkdown),
        ("notes/", InvalidComponent),
        ("a//b.md", InvalidComponent),
        ("./a.md", InvalidComponent),
        ("a\\b.md", InvalidComponent),
        ("a\0.md", InvalidComponent),
        ("../a.md", Traversal),
        ("x/../a.md", Traversal),
        (".hidden/a.md", Hidden),
        ("x/.a.md", Hidden),
        (".md", Hidden),
        ("archive/a.md", Archived),
        ("Archive/sub/a.md", Archived),
    ] {
        assert_eq!(VaultPath::parse(raw), Err(expected), "{raw:?}");
    }
}

#[test]
fn builds_filesystem_paths() {
    let path = VaultPath::parse("notes/plan.md").unwrap();
    assert_eq!(
        path.to_fs_path(std::path::Path::new("/vault")),
        std::path::PathBuf::from("/vault/notes/plan.md")
    );
    assert_eq!(path.to_string(), "notes/plan.md");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-workflow --locked --test vault`
Expected: compile error, unresolved module `vault`.

- [ ] **Step 3: Create `crates/brn-workflow/src/vault/path.rs`**

```rust
use std::path::{Path, PathBuf};

/// A vault-relative path to a note: `/`-separated, ends in `.md`, never hidden,
/// never under the top-level `archive/` folder and never escaping the vault.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultPath(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultPathError {
    Empty,
    Absolute,
    InvalidComponent,
    Traversal,
    Hidden,
    Archived,
    NotMarkdown,
}

impl std::fmt::Display for VaultPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "note path is empty",
            Self::Absolute => "note path must be relative to the vault",
            Self::InvalidComponent => "note path has an empty or invalid part",
            Self::Traversal => "note path must stay inside the vault",
            Self::Hidden => "hidden files and folders are not notes",
            Self::Archived => "notes in archive/ are not available",
            Self::NotMarkdown => "notes must be .md files",
        })
    }
}

impl std::error::Error for VaultPathError {}

pub(crate) fn is_markdown_name(name: &str) -> bool {
    name.len() > 3
        && name
            .get(name.len() - 3..)
            .is_some_and(|ext| ext.eq_ignore_ascii_case(".md"))
}

impl VaultPath {
    pub fn parse(raw: &str) -> Result<Self, VaultPathError> {
        if raw.is_empty() {
            return Err(VaultPathError::Empty);
        }
        if raw.starts_with('/') {
            return Err(VaultPathError::Absolute);
        }
        if raw.contains('\\') || raw.contains('\0') {
            return Err(VaultPathError::InvalidComponent);
        }
        let parts: Vec<&str> = raw.split('/').collect();
        for part in &parts {
            match *part {
                "" | "." => return Err(VaultPathError::InvalidComponent),
                ".." => return Err(VaultPathError::Traversal),
                p if p.starts_with('.') => return Err(VaultPathError::Hidden),
                _ => {}
            }
        }
        if parts.len() > 1 && parts[0].eq_ignore_ascii_case("archive") {
            return Err(VaultPathError::Archived);
        }
        if !is_markdown_name(parts[parts.len() - 1]) {
            return Err(VaultPathError::NotMarkdown);
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_fs_path(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        for part in self.0.split('/') {
            path.push(part);
        }
        path
    }
}

impl std::fmt::Display for VaultPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

- [ ] **Step 4: Create `crates/brn-workflow/src/vault/mod.rs`**

```rust
//! The vault folder: which files are notes, listing them and reading them.
mod path;

pub use path::{VaultPath, VaultPathError};
```

- [ ] **Step 5: Add `pub mod vault;`** to `crates/brn-workflow/src/lib.rs` after `pub mod worker;`.

- [ ] **Step 6: Run**

Run: `cargo test -p brn-workflow --locked --test vault`
Expected: 3 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/brn-workflow/src/vault crates/brn-workflow/src/lib.rs crates/brn-workflow/tests/vault.rs
git commit -m "feat(workflow): add vault note path rules

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 5: Scanning and reading the vault

**Files:**
- Create: `crates/brn-workflow/src/vault/scan.rs`
- Create: `crates/brn-workflow/src/vault/read.rs`
- Modify: `crates/brn-workflow/src/vault/mod.rs`
- Modify: `crates/brn-workflow/tests/vault.rs`

**Interfaces:**
- Consumes: Task 4 `VaultPath`, `is_markdown_name`; `crate::MAX_IMPORT_BYTES` (1 MiB, `crates/brn-workflow/src/lib.rs:106`).
- Produces:

```rust
pub struct NoteFile { pub path: VaultPath, pub size: u64, pub modified_ns: i64 }
pub enum SkipReason { TooLarge, InvalidName }
pub struct Skipped { pub path: String, pub reason: SkipReason }
pub struct Scan { pub notes: Vec<NoteFile>, pub skipped: Vec<Skipped> } // both sorted by path
pub fn scan(root: &std::path::Path) -> std::io::Result<Scan>;

pub struct NoteText { pub text: String, pub sha256: [u8; 32] }
pub enum ReadError { Missing, NotAFile, TooLarge, NotUtf8, Io(std::io::Error) }
pub fn read_note(root: &std::path::Path, path: &VaultPath) -> Result<NoteText, ReadError>;
```

`scan` uses file metadata only, so a `.md` file with non-UTF-8 content still appears in `Scan.notes`. Step 3 (search) reads every new or changed note for indexing and lists those returning `ReadError::NotUtf8` as unreadable (spec section 3). Known limit: a folder swapped for a symlink in the middle of a scan can make `scan` list paths outside the vault; `read_note` refuses them, so their content is never read.

- [ ] **Step 1: Add the failing tests** to `crates/brn-workflow/tests/vault.rs`:

```rust
use brn_workflow::vault::{ReadError, SkipReason, read_note, scan};
use sha2::{Digest, Sha256};
use std::os::unix::fs::symlink;

const LIMIT: usize = 1024 * 1024;

fn write(root: &std::path::Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn scan_lists_notes_and_skips_excluded_files() {
    let vault = tempfile::tempdir().unwrap();
    let root = vault.path();
    write(root, "a.md", b"alpha");
    write(root, "sub/b.md", b"beta");
    write(root, "sub/c.txt", b"not a note");
    write(root, ".obsidian/d.md", b"hidden");
    write(root, "sub/.e.md", b"hidden");
    write(root, "archive/f.md", b"archived");
    write(root, "sub/archive/g.md", b"nested archive is fine");
    write(root, "big.md", &vec![b'x'; LIMIT + 1]);
    write(root, "exact.md", &vec![b'x'; LIMIT]);
    symlink(root.join("a.md"), root.join("link.md")).unwrap();
    symlink(root.join("sub"), root.join("linkdir")).unwrap();

    let found = scan(root).unwrap();
    let paths: Vec<&str> = found.notes.iter().map(|n| n.path.as_str()).collect();
    assert_eq!(paths, vec!["a.md", "exact.md", "sub/archive/g.md", "sub/b.md"]);
    assert_eq!(found.notes[0].size, 5);
    assert!(found.notes[0].modified_ns > 0);
    assert_eq!(found.skipped.len(), 1);
    assert_eq!(found.skipped[0].path, "big.md");
    assert_eq!(found.skipped[0].reason, SkipReason::TooLarge);
}

#[test]
fn read_returns_exact_text_and_hash() {
    let vault = tempfile::tempdir().unwrap();
    let bytes = "\u{feff}# Plan é\r\nline\r\n".as_bytes();
    write(vault.path(), "notes/plan.md", bytes);
    let path = VaultPath::parse("notes/plan.md").unwrap();
    let note = read_note(vault.path(), &path).unwrap();
    assert_eq!(note.text.as_bytes(), bytes);
    assert_eq!(note.sha256, <[u8; 32]>::from(Sha256::digest(bytes)));
}

#[test]
fn read_rejects_unsafe_or_unusable_files() {
    let vault = tempfile::tempdir().unwrap();
    let root = vault.path();
    write(root, "real/a.md", b"a");
    write(root, "big.md", &vec![b'x'; LIMIT + 1]);
    write(root, "binary.md", &[0xff, 0xfe, 0x00]);
    std::fs::create_dir(root.join("dir.md")).unwrap();
    symlink(root.join("real/a.md"), root.join("link.md")).unwrap();
    symlink(root.join("real"), root.join("linked")).unwrap();
    let read = |raw: &str| read_note(root, &VaultPath::parse(raw).unwrap());
    assert!(matches!(read("absent.md"), Err(ReadError::Missing)));
    assert!(matches!(read("link.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("linked/a.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("dir.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("big.md"), Err(ReadError::TooLarge)));
    assert!(matches!(read("binary.md"), Err(ReadError::NotUtf8)));
    assert_eq!(read("real/a.md").unwrap().text, "a");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-workflow --locked --test vault`
Expected: compile error, unresolved `scan` / `read_note`.

- [ ] **Step 3: Create `crates/brn-workflow/src/vault/scan.rs`**

```rust
use super::path::{VaultPath, is_markdown_name};
use crate::MAX_IMPORT_BYTES;
use std::{
    fs::Metadata,
    path::Path,
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteFile {
    pub path: VaultPath,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch (0 if unavailable).
    pub modified_ns: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TooLarge,
    InvalidName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: String,
    pub reason: SkipReason,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub notes: Vec<NoteFile>,
    pub skipped: Vec<Skipped>,
}

/// Lists every note in the vault. Symlinks, hidden entries and the top-level
/// `archive/` folder are skipped silently; unusable `.md` files are reported.
pub fn scan(root: &Path) -> std::io::Result<Scan> {
    let mut found = Scan::default();
    visit(root, "", &mut found)?;
    found.notes.sort_by(|a, b| a.path.cmp(&b.path));
    found.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn modified_ns(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn visit(dir: &Path, prefix: &str, found: &mut Scan) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let os_name = entry.file_name();
        let Some(name) = os_name.to_str() else {
            let lossy = os_name.to_string_lossy();
            if file_type.is_file() && is_markdown_name(&lossy) {
                found.skipped.push(Skipped {
                    path: join(prefix, &lossy),
                    reason: SkipReason::InvalidName,
                });
            }
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let relative = join(prefix, name);
        if file_type.is_dir() {
            if prefix.is_empty() && name.eq_ignore_ascii_case("archive") {
                continue;
            }
            visit(&entry.path(), &relative, found)?;
        } else if file_type.is_file() && is_markdown_name(name) {
            let Ok(path) = VaultPath::parse(&relative) else {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::InvalidName,
                });
                continue;
            };
            let meta = entry.metadata()?;
            if meta.len() > MAX_IMPORT_BYTES as u64 {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::TooLarge,
                });
                continue;
            }
            found.notes.push(NoteFile {
                path,
                size: meta.len(),
                modified_ns: modified_ns(&meta),
            });
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Create `crates/brn-workflow/src/vault/read.rs`**

```rust
use super::path::VaultPath;
use crate::MAX_IMPORT_BYTES;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{ErrorKind, Read},
    os::unix::fs::MetadataExt,
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteText {
    pub text: String,
    pub sha256: [u8; 32],
}

#[derive(Debug)]
pub enum ReadError {
    Missing,
    /// A symlink, a folder or another non-regular file.
    NotAFile,
    TooLarge,
    NotUtf8,
    Io(std::io::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => f.write_str("note does not exist"),
            Self::NotAFile => f.write_str("note is not a regular file"),
            Self::TooLarge => f.write_str("note is larger than 1 MiB"),
            Self::NotUtf8 => f.write_str("note is not valid UTF-8"),
            Self::Io(e) => write!(f, "could not read note: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == ErrorKind::NotFound {
            Self::Missing
        } else {
            Self::Io(e)
        }
    }
}

/// Reads a note's exact bytes without following symlinks anywhere below `root`.
pub fn read_note(root: &Path, path: &VaultPath) -> Result<NoteText, ReadError> {
    let mut current = root.to_path_buf();
    let mut checked = None;
    for part in path.as_str().split('/') {
        current.push(part);
        let meta = std::fs::symlink_metadata(&current)?;
        if meta.file_type().is_symlink() {
            return Err(ReadError::NotAFile);
        }
        checked = Some(meta);
    }
    let Some(checked) = checked else {
        return Err(ReadError::Missing);
    };
    if !checked.is_file() {
        return Err(ReadError::NotAFile);
    }
    if checked.len() > MAX_IMPORT_BYTES as u64 {
        return Err(ReadError::TooLarge);
    }
    let file = File::open(&current)?;
    let opened = file.metadata()?;
    // If a folder or the file was swapped for a symlink after the checks
    // above, `open` reached a different file: refuse it.
    if !opened.is_file() || (opened.dev(), opened.ino()) != (checked.dev(), checked.ino()) {
        return Err(ReadError::NotAFile);
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(MAX_IMPORT_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ReadError::TooLarge);
    }
    let sha256 = Sha256::digest(&bytes).into();
    let text = String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    Ok(NoteText { text, sha256 })
}
```

- [ ] **Step 5: Update `crates/brn-workflow/src/vault/mod.rs`**

```rust
//! The vault folder: which files are notes, listing them and reading them.
mod path;
mod read;
mod scan;

pub use path::{VaultPath, VaultPathError};
pub use read::{NoteText, ReadError, read_note};
pub use scan::{NoteFile, Scan, SkipReason, Skipped, scan};
```

- [ ] **Step 6: Run tests and Clippy**

Run: `cargo test -p brn-workflow --locked --test vault && cargo clippy -p brn-workflow --all-targets --locked -- -D warnings`
Expected: 6 passed, no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/brn-workflow/src/vault crates/brn-workflow/tests/vault.rs
git commit -m "feat(workflow): scan and read vault notes safely

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 6: Repository rules and evidence

**Files:**
- Modify: `AGENTS.md` (section "Rules to preserve", lines 27–35)
- Modify: `docs/architecture/invariants.md` (insert after the introduction paragraph, line 3)
- Modify: `docs/work/active/README.md` (task table)
- Modify: `docs/work/active/simple-rig-notes/evidence.md`

**Interfaces:** documentation only.

- [ ] **Step 1: Replace the "Rules to preserve" bullets in `AGENTS.md`** with:

```markdown
The repository is moving to the [simple Rig-based notes app](docs/superpowers/specs/2026-10-02-simple-rig-notes-design.md) ([roadmap](docs/work/active/simple-rig-notes/plan.md)). New code follows these rules; existing code keeps working until the cleanup step removes it.

- Notes are vault Markdown files; preserve their exact bytes. `index.sqlite` is disposable; `brn.sqlite` holds user work, is checked at start and backed up.
- The AI writes only proposals; only the user's Approve writes AI text to the vault. No automatic fallback between providers, models or accounts.
- Comments are temporary review notes, deleted when the note's review is approved. Never re-anchor a comment by guessing.
- UI and CLI go through `brn-workflow`; keep provider and retrieval details out of UI state.
- Use disposable explicit data directories and synthetic fixtures for checks. Preserve the original vault, old data folders and existing trial workspaces.
- Do not log or commit credentials. Live provider checks and model downloads require the user's request; deterministic offline checks are the default.
- Respect the pinned toolchain and lockfiles. Optional native features and standalone experiments need separate verification.
```

- [ ] **Step 2: Add a transition note to `docs/architecture/invariants.md`** after its first paragraph:

```markdown
> **Transition:** the [simple Rig-based notes app](../superpowers/specs/2026-10-02-simple-rig-notes-design.md) replaces these invariants step by step ([roadmap](../work/active/simple-rig-notes/plan.md)). New code in `brn_store::work` and `brn_workflow::vault` follows that specification; the invariants below still describe the existing code until the cleanup step removes it.
```

- [ ] **Step 3: Update `docs/work/active/README.md`.** Change the "Rig-first architecture reset" row's state to `Superseded on 2026-10-02 by the simple Rig-based notes app; not executed.` Add a row:

```markdown
| Simple Rig-based notes app | Specification approved 2026-10-02. Step 1 (spike) and step 2 (store and vault) planned; steps 3–6 planned after their inputs exist. | [Roadmap](simple-rig-notes/plan.md), [evidence](simple-rig-notes/evidence.md), [specification](../../superpowers/specs/2026-10-02-simple-rig-notes-design.md) |
```

This file may contain the user's uncommitted edits; keep them and stage only after checking `git diff docs/work/active/README.md` with the user if anything unexpected is present.

- [ ] **Step 4: Record evidence.** Append to `docs/work/active/simple-rig-notes/evidence.md` a "Step 2: store and vault" section with date, commit, platform/toolchain and the actual results of:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p brn-store -p brn-workflow --locked
git diff --check
```

- [ ] **Step 5: Run the checks above.** Expected: all pass. Record actual counts, not expected ones.

- [ ] **Step 6: Commit**

```bash
git add AGENTS.md docs/architecture/invariants.md docs/work/active/README.md docs/work/active/simple-rig-notes/evidence.md
git commit -m "docs: adopt simple notes app rules for new code

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```
