use super::{Checked, check, check_with_flags, now_ms};
use crate::{Result, invalid};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const KEEP: usize = 5;
const COPY_BUDGET: Duration = Duration::from_secs(2);
const PAGES_PER_STEP: i32 = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackupCheckpointOutcome {
    Unchanged,
    Created {
        path: PathBuf,
        completed_at_ms: u64,
        retention_warning: Option<String>,
    },
}

// Serde's internally tagged unit variant otherwise ignores unexpected fields,
// even with deny_unknown_fields. Decode its wire form as an empty struct variant.
impl<'de> Deserialize<'de> for BackupCheckpointOutcome {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Unchanged {},
            Created {
                path: PathBuf,
                completed_at_ms: u64,
                retention_warning: Option<String>,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Unchanged {} => Self::Unchanged,
            Wire::Created {
                path,
                completed_at_ms,
                retention_warning,
            } => Self::Created {
                path,
                completed_at_ms,
                retention_warning,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChangeToken {
    own_changes: u64,
    data_version: i64,
}

pub(super) fn change_token(conn: &Connection) -> Result<ChangeToken> {
    Ok(ChangeToken {
        own_changes: conn.total_changes(),
        data_version: conn.query_row("PRAGMA main.data_version", [], |row| row.get(0))?,
    })
}

pub(super) struct Created {
    pub path: PathBuf,
    pub completed_at_ms: u64,
    pub retention_warning: Option<String>,
}

pub(super) fn checkpoint_if_changed(
    store: &mut super::WorkStore,
) -> Result<BackupCheckpointOutcome> {
    checkpoint_with(store, COPY_BUDGET, |_, _| Ok(()))
}

fn checkpoint_with(
    store: &mut super::WorkStore,
    budget: Duration,
    hook: impl FnMut(Phase, &Path) -> Result<()>,
) -> Result<BackupCheckpointOutcome> {
    let before_copy = change_token(&store.conn)?;
    if before_copy == store.checkpoint_baseline {
        return Ok(BackupCheckpointOutcome::Unchanged);
    }
    let created = create_with(&store.dir, &store.conn, budget, hook)?;
    // Keep the pre-copy token: even a late commit included by SQLite's restart
    // stays eligible for another conservative copy.
    store.checkpoint_baseline = before_copy;
    Ok(BackupCheckpointOutcome::Created {
        path: created.path,
        completed_at_ms: created.completed_at_ms,
        retention_warning: created.retention_warning,
    })
}

/// Small private fault/progress seam; production supplies an empty callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    BeforeTemporary,
    Temporary,
    Step(StepResult),
    Validate,
    FileSync,
    Publish,
    DirectorySync,
    Prune,
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = remove_with_sidecars(&self.0);
    }
}

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

/// Creates a standalone validated snapshot before publishing any restore candidate.
pub(super) fn create(data_dir: &Path, conn: &Connection) -> Result<Created> {
    create_with(data_dir, conn, COPY_BUDGET, |_, _| Ok(()))
}

fn create_with(
    data_dir: &Path,
    conn: &Connection,
    budget: Duration,
    mut hook: impl FnMut(Phase, &Path) -> Result<()>,
) -> Result<Created> {
    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    if !std::fs::symlink_metadata(&dir)?.file_type().is_dir() {
        return Err(invalid("backup directory must be a real directory"));
    }
    let after_last = list(data_dir)?
        .last()
        .and_then(|path| stamp(path))
        .map(|last| {
            last.checked_add(1)
                .ok_or_else(|| invalid("backup timestamp exhausted"))
        })
        .transpose()?
        .unwrap_or(0);
    let path = dir.join(format!("brn-{:013}.sqlite", now_ms().max(after_last)));
    let temporary_path = dir.join(format!(".checkpoint-{}.tmp", uuid::Uuid::new_v4()));
    // Ownership starts only after exclusive creation; never clean up someone
    // else's occupied path. UUID collisions fail instead of adopting a file.
    hook(Phase::BeforeTemporary, &temporary_path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary_path)?;
    let temporary = Temporary(temporary_path);
    hook(Phase::Temporary, &temporary.0)?;
    let mut destination = Connection::open_with_flags(
        &temporary.0,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    destination.busy_timeout(Duration::ZERO)?;
    {
        let backup = Backup::new(conn, &mut destination)?;
        bounded_steps(Instant::now() + budget, || {
            let step = backup.step(PAGES_PER_STEP)?;
            hook(Phase::Step(step), &temporary.0)?;
            Ok(step)
        })?;
    }
    // A restore candidate must not depend on temporary-name WAL/SHM sidecars.
    let mode: String =
        destination.pragma_update_and_check(None, "journal_mode", "DELETE", |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("delete") {
        return Err(invalid("backup could not become a standalone database"));
    }
    destination.close().map_err(|(_, error)| error)?;
    hook(Phase::Validate, &temporary.0)?;
    validate_snapshot(&temporary.0)?;
    hook(Phase::FileSync, &temporary.0)?;
    file.sync_all()?;
    drop(file);
    hook(Phase::Publish, &path)?;
    // Same-directory hard-link installation is atomic and refuses every occupied
    // path (including symlinks); unlike rename it cannot overwrite a foreign file.
    std::fs::hard_link(&temporary.0, &path)?;
    let synced = (|| -> Result<()> {
        std::fs::remove_file(&temporary.0)?;
        hook(Phase::DirectorySync, &path)?;
        File::open(&dir)?.sync_all()?;
        File::open(data_dir)?.sync_all()?;
        Ok(())
    })();
    if let Err(error) = synced {
        // The candidate is not durable yet. Leave earlier usable copies alone.
        let _ = std::fs::remove_file(&path);
        let _ = File::open(&dir).and_then(|directory| directory.sync_all());
        return Err(error);
    }
    let completed_at_ms = now_ms();
    let retention_warning = hook(Phase::Prune, &path)
        .and_then(|()| prune(data_dir))
        .err()
        .map(|error| error.to_string());
    Ok(Created {
        path,
        completed_at_ms,
        retention_warning,
    })
}

fn bounded_steps(deadline: Instant, mut step: impl FnMut() -> Result<StepResult>) -> Result<()> {
    loop {
        if Instant::now() >= deadline {
            return Err(invalid("backup copy deadline exceeded"));
        }
        match step()? {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            StepResult::Busy => {
                return Err(invalid("backup source is busy; checkpoint may be retried"));
            }
            StepResult::Locked => {
                return Err(invalid(
                    "backup source is locked; checkpoint may be retried",
                ));
            }
            _ => return Err(invalid("unsupported SQLite backup step result")),
        }
    }
}

/// Read-only validation, deliberately without WorkStore::open, migrations,
/// reconciliation or another backup. Running work remains Running in the copy.
fn validate_snapshot(path: &Path) -> Result<()> {
    let conn = match check_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )? {
        Checked::Brn(conn) => conn,
        Checked::Invalid(error) => return Err(error),
        _ => return Err(invalid("backup snapshot is not a valid BRN database")),
    };
    validate_current_snapshot(&conn)
}

fn validate_current_snapshot(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != super::MIGRATIONS.len() as i64 {
        return Err(invalid("backup snapshot has an unexpected schema version"));
    }
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(invalid("backup snapshot failed integrity validation"));
    }
    let mut foreign = conn.prepare("PRAGMA foreign_key_check")?;
    if foreign.query([])?.next()?.is_some() {
        return Err(invalid("backup snapshot has broken foreign-key bindings"));
    }
    super::editor::check_all(conn)?;
    // V1 unfinished edits have no JSON reader, but their typed fields still
    // must satisfy the public write bounds.
    let bad_edits: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM unsaved_edits WHERE length(path)=0 OR length(base_sha256)!=32 OR length(CAST(text AS BLOB))>?1)",
        [super::MAX_NOTE_BYTES as i64], |row| row.get(0),
    )?;
    if bad_edits {
        return Err(invalid("invalid backup unfinished edit"));
    }
    super::proposals::check_all(conn)?;
    super::chat::conversations(conn)?;
    for id in keys(conn, "proposal_applies", "operation_id")? {
        if super::proposal_apply::read_journal(conn, canonical_id(&id)?)?.is_none() {
            return Err(invalid("backup approval journal is missing"));
        }
    }
    for id in keys(conn, "proposal_rewrites", "id")? {
        if super::proposal_rewrite::read_job(conn, canonical_id(&id)?)?.is_none() {
            return Err(invalid("backup Rewrite job is missing"));
        }
    }
    Ok(())
}

fn canonical_id(id: &str) -> Result<uuid::Uuid> {
    let uuid = uuid::Uuid::parse_str(id).map_err(|_| invalid("invalid backup row UUID"))?;
    if uuid.to_string() != id {
        return Err(invalid("noncanonical backup row UUID"));
    }
    Ok(uuid)
}

pub(super) fn keys(conn: &Connection, table: &str, key: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare(&format!("SELECT {key} FROM {table} ORDER BY rowid"))?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
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
    for path in [
        db.to_path_buf(),
        with_suffix(db, "-wal"),
        with_suffix(db, "-shm"),
        with_suffix(db, "-journal"),
    ] {
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
            let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            if version < super::MIGRATIONS.len() as i64 || validate_current_snapshot(&conn).is_ok()
            {
                return Ok((conn, Some(backup)));
            }
        }
        remove_with_sidecars(db)?;
    }
    Ok((Connection::open(db)?, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{WorkBudget, WorkStore, WorkTurnStatus, conversations::*};
    use rusqlite::params;
    use std::os::unix::fs::PermissionsExt;
    use uuid::Uuid;

    fn assert_same_turn(left: crate::work::WorkTurn, right: crate::work::WorkTurn) {
        assert_eq!(
            serde_json::to_vec(&left).unwrap(),
            serde_json::to_vec(&right).unwrap()
        );
    }

    fn fixture() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
    }

    fn created(outcome: BackupCheckpointOutcome) -> PathBuf {
        let BackupCheckpointOutcome::Created { path, .. } = outcome else {
            panic!("expected a changed-state checkpoint")
        };
        path
    }

    fn copies(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        list(dir)
            .unwrap()
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }

    fn assert_no_temporary(dir: &Path) {
        assert!(std::fs::read_dir(backups_dir(dir)).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".checkpoint-")
        }));
    }

    #[test]
    fn strict_outcomes_and_read_only_or_exact_replay_do_not_copy() {
        assert!(
            serde_json::from_str::<BackupCheckpointOutcome>(r#"{"kind":"unchanged","extra":true}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<BackupCheckpointOutcome>(r#"{"kind":"created","path":"/synthetic","completed_at_ms":1,"retention_warning":null,"extra":true}"#).is_err());
        for outcome in [
            BackupCheckpointOutcome::Unchanged,
            BackupCheckpointOutcome::Created {
                path: "/synthetic/backup.sqlite".into(),
                completed_at_ms: 7,
                retention_warning: Some("retention warning".into()),
            },
        ] {
            assert_eq!(
                serde_json::from_slice::<BackupCheckpointOutcome>(
                    &serde_json::to_vec(&outcome).unwrap()
                )
                .unwrap(),
                outcome
            );
        }
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
        store.set_setting("synthetic", "exact λ\r\n").unwrap();
        let first = created(store.checkpoint_if_changed().unwrap());
        let before = copies(dir.path());
        assert_eq!(
            store.setting("synthetic").unwrap().as_deref(),
            Some("exact λ\r\n")
        );
        assert!(store.conversations().unwrap().is_empty());
        store.set_setting("synthetic", "exact λ\r\n").unwrap();
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
        assert_eq!(copies(dir.path()), before);
        assert_eq!(
            std::fs::metadata(first).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let mut chat = store.chat_connection().unwrap();
        let id = Uuid::new_v4();
        let started = chat
            .begin_turn(id, None, "Question", "chatgpt", "synthetic")
            .unwrap();
        let ended = chat
            .finish_turn(id, WorkTurnStatus::Completed, "Answer", None)
            .unwrap();
        created(store.checkpoint_if_changed().unwrap());
        assert_same_turn(
            chat.begin_turn(id, None, "Question", "chatgpt", "synthetic")
                .unwrap(),
            ended.clone(),
        );
        assert_same_turn(
            chat.finish_turn(id, WorkTurnStatus::Completed, "Answer", None)
                .unwrap(),
            ended.clone(),
        );
        assert_same_turn(store.turn(id).unwrap().unwrap(), ended);
        assert_eq!(
            store.conversations().unwrap()[0].id,
            started.conversation_id
        );
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
    }

    #[test]
    fn attached_commits_during_and_after_snapshot_stay_dirty_and_copies_are_consistent() {
        for after_done in [false, true] {
            let dir = fixture();
            let (mut store, _) = WorkStore::open(dir.path()).unwrap();
            store
                .set_setting("synthetic.large", &"x".repeat(2 * 1024 * 1024))
                .unwrap();
            let mut chat = store.chat_connection().unwrap();
            let id = Uuid::new_v4();
            let running = chat
                .begin_turn(id, None, "Question", "chatgpt", "synthetic")
                .unwrap();
            let mut committed = None;
            let path = created(
                checkpoint_with(&mut store, COPY_BUDGET, |phase, _| {
                    let due = if after_done {
                        phase == Phase::Step(StepResult::Done)
                    } else {
                        phase == Phase::Step(StepResult::More)
                    };
                    if due && committed.is_none() {
                        committed = Some(chat.finish_turn(
                            id,
                            WorkTurnStatus::Failed,
                            "Retained partial λ",
                            Some("network"),
                        )?);
                    }
                    Ok(())
                })
                .unwrap(),
            );
            let terminal = committed.expect("fixture must cross a real copy step");
            let snapshot =
                Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            let captured = super::super::chat::read_turn(&snapshot, id)
                .unwrap()
                .unwrap()
                .0;
            assert_same_turn(
                captured.clone(),
                if after_done {
                    running
                } else {
                    terminal.clone()
                },
            );
            assert!(matches!(
                captured.status,
                WorkTurnStatus::Running | WorkTurnStatus::Failed
            ));
            if after_done {
                let recovery = fixture();
                std::fs::copy(&path, recovery.path().join("brn.sqlite")).unwrap();
                let (recovered, _) = WorkStore::open(recovery.path()).unwrap();
                let interrupted = recovered.turn(id).unwrap().unwrap();
                assert_eq!(interrupted.status, WorkTurnStatus::Interrupted);
                assert_eq!(interrupted.question, captured.question);
                assert_eq!(interrupted.answer, captured.answer);
            }
            let next = created(store.checkpoint_if_changed().unwrap());
            let next = Connection::open_with_flags(next, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_same_turn(
                super::super::chat::read_turn(&next, id).unwrap().unwrap().0,
                terminal,
            );
            assert_eq!(
                store.checkpoint_if_changed().unwrap(),
                BackupCheckpointOutcome::Unchanged
            );
            assert_no_temporary(dir.path());
        }
    }

    #[test]
    fn each_failed_copy_phase_preserves_baseline_prior_copies_and_retry() {
        for fail_at in [
            Phase::Temporary,
            Phase::Step(StepResult::Done),
            Phase::Validate,
            Phase::FileSync,
            Phase::Publish,
            Phase::DirectorySync,
        ] {
            let dir = fixture();
            let (mut store, _) = WorkStore::open(dir.path()).unwrap();
            store.set_setting("synthetic", "kept").unwrap();
            let before = copies(dir.path());
            let baseline = store.checkpoint_baseline;
            let error = checkpoint_with(&mut store, COPY_BUDGET, |phase, _| {
                if phase == fail_at {
                    return Err(invalid("injected checkpoint failure"));
                }
                Ok(())
            })
            .expect_err("injected phase must fail");
            assert!(error.to_string().contains("injected"));
            assert_eq!(store.checkpoint_baseline, baseline);
            assert_eq!(copies(dir.path()), before);
            assert_no_temporary(dir.path());
            created(store.checkpoint_if_changed().unwrap());
        }
    }

    #[test]
    fn occupied_private_temporary_is_never_opened_or_cleaned_up() {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("synthetic", "retry me").unwrap();
        let before = copies(dir.path());
        let baseline = store.checkpoint_baseline;
        let mut occupied = None;
        assert!(
            checkpoint_with(&mut store, COPY_BUDGET, |phase, path| {
                if phase == Phase::BeforeTemporary {
                    std::fs::write(path, b"foreign private temp collision")?;
                    occupied = Some(path.to_path_buf());
                }
                Ok(())
            })
            .is_err()
        );
        let occupied = occupied.unwrap();
        assert_eq!(
            std::fs::read(&occupied).unwrap(),
            b"foreign private temp collision"
        );
        assert_eq!(store.checkpoint_baseline, baseline);
        assert_eq!(copies(dir.path()), before);
        // The occupied path is foreign test evidence; only its owner removes it.
        std::fs::remove_file(occupied).unwrap();
        created(store.checkpoint_if_changed().unwrap());
    }

    #[test]
    fn occupied_publication_is_never_overwritten_or_adopted() {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("synthetic", "kept").unwrap();
        let before = copies(dir.path());
        let baseline = store.checkpoint_baseline;
        let mut occupied = None;
        assert!(
            checkpoint_with(&mut store, COPY_BUDGET, |phase, path| {
                if phase == Phase::Publish {
                    std::fs::write(path, b"foreign occupied destination")?;
                    occupied = Some(path.to_path_buf());
                }
                Ok(())
            })
            .is_err()
        );
        let occupied = occupied.unwrap();
        assert_eq!(
            std::fs::read(&occupied).unwrap(),
            b"foreign occupied destination"
        );
        assert_eq!(store.checkpoint_baseline, baseline);
        for (path, bytes) in before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert_no_temporary(dir.path());
        let next = created(store.checkpoint_if_changed().unwrap());
        assert_ne!(next, occupied);
    }

    #[test]
    fn prune_failure_acknowledges_usable_copy_and_retention_recovers() {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        for n in 0..KEEP {
            store.set_setting("synthetic", &n.to_string()).unwrap();
            created(store.checkpoint_if_changed().unwrap());
        }
        assert_eq!(list(dir.path()).unwrap().len(), KEEP);
        let before = copies(dir.path());
        store.set_setting("synthetic", "newest").unwrap();
        let outcome = checkpoint_with(&mut store, COPY_BUDGET, |phase, _| {
            if phase == Phase::Prune {
                return Err(invalid("injected retention failure"));
            }
            Ok(())
        })
        .unwrap();
        let BackupCheckpointOutcome::Created {
            path,
            retention_warning,
            ..
        } = outcome
        else {
            panic!("expected valid checkpoint")
        };
        assert!(retention_warning.unwrap().contains("retention"));
        validate_snapshot(&path).unwrap();
        assert_eq!(list(dir.path()).unwrap().len(), KEEP + 1);
        for (path, bytes) in before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
        store.set_setting("synthetic", "later").unwrap();
        created(store.checkpoint_if_changed().unwrap());
        assert_eq!(list(dir.path()).unwrap().len(), KEEP);
    }

    #[test]
    fn bounded_steps_refuse_busy_locked_and_elapsed_deadlines_without_spinning() {
        for outcome in [StepResult::Busy, StepResult::Locked] {
            let mut attempts = 0;
            assert!(
                bounded_steps(Instant::now() + COPY_BUDGET, || {
                    attempts += 1;
                    Ok(outcome)
                })
                .is_err()
            );
            assert_eq!(attempts, 1);
        }
        let mut attempts = 0;
        assert!(
            bounded_steps(Instant::now(), || {
                attempts += 1;
                Ok(StepResult::Done)
            })
            .is_err()
        );
        assert_eq!(attempts, 0);
        let mut attempts = 0;
        assert!(
            bounded_steps(Instant::now() + Duration::from_millis(1), || {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(2));
                Ok(StepResult::More)
            })
            .is_err()
        );
        assert_eq!(attempts, 1);
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("synthetic", "retry me").unwrap();
        let baseline = store.checkpoint_baseline;
        let before = copies(dir.path());
        assert!(checkpoint_with(&mut store, Duration::ZERO, |_, _| Ok(())).is_err());
        store
            .conn
            .execute_batch(
                "BEGIN IMMEDIATE; UPDATE settings SET value='uncommitted' WHERE key='synthetic'",
            )
            .unwrap();
        assert!(
            store.checkpoint_if_changed().is_err(),
            "live source write transaction must refuse backup"
        );
        store.conn.execute_batch("ROLLBACK").unwrap();
        assert_eq!(store.checkpoint_baseline, baseline);
        assert_eq!(copies(dir.path()), before);
        assert_no_temporary(dir.path());
        created(store.checkpoint_if_changed().unwrap());
    }

    #[test]
    fn invalid_temporary_or_live_semantic_damage_never_publishes() {
        for corrupt_temp in [false, true] {
            let dir = fixture();
            let (mut store, _) = WorkStore::open(dir.path()).unwrap();
            store.set_setting("synthetic", "retry me").unwrap();
            let before = copies(dir.path());
            let baseline = store.checkpoint_baseline;
            let result = checkpoint_with(&mut store, COPY_BUDGET, |phase, path| {
                if phase == Phase::Validate {
                    if corrupt_temp {
                        std::fs::write(path, b"synthetic corrupt copy")?;
                    } else {
                        let conn = Connection::open(path)?;
                        conn.execute(
                            "INSERT INTO unsaved_edits VALUES('bad.md',?1,?2,0)",
                            params![vec![0_u8; 32], "x".repeat(super::super::MAX_NOTE_BYTES + 1)],
                        )?;
                    }
                }
                Ok(())
            });
            assert!(result.is_err());
            assert_eq!(store.checkpoint_baseline, baseline);
            assert_eq!(copies(dir.path()), before);
            assert_no_temporary(dir.path());
        }
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "Question", "chatgpt", "synthetic")
            .unwrap();
        created(store.checkpoint_if_changed().unwrap());
        let before = copies(dir.path());
        let baseline = store.checkpoint_baseline;
        store
            .conn
            .execute(
                "UPDATE conversation_lifecycle SET version=2 WHERE conversation_id=?1",
                [turn.conversation_id.to_string()],
            )
            .unwrap();
        assert!(store.checkpoint_if_changed().is_err());
        assert_eq!(store.checkpoint_baseline, baseline);
        assert_eq!(copies(dir.path()), before);
        assert_no_temporary(dir.path());
    }

    #[test]
    fn readable_primary_editor_damage_refuses_without_replacing_work_or_backups() {
        use crate::{files::FileFingerprint, hash};
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let text = "Newer owner work λ\r\n";
        let fingerprint = FileFingerprint {
            device: 1,
            inode: 4,
            len: text.len() as u64,
            sha256: hash(text.as_bytes()),
        };
        store.open_editor("note.md", &fingerprint, text).unwrap();
        created(store.checkpoint_if_changed().unwrap());
        store
            .conn
            .execute("UPDATE editors SET record_sha256=?1", [vec![0_u8; 32]])
            .unwrap();
        drop(store);
        let before = copies(dir.path());
        assert!(WorkStore::open(dir.path()).is_err());
        assert_eq!(copies(dir.path()), before);
        let conn = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        let digest: Vec<u8> = conn
            .query_row(
                "SELECT record_sha256 FROM editors WHERE path='note.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            digest,
            vec![0_u8; 32],
            "damaged current work must never be replaced by older healthy work"
        );
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("brn.sqlite.corrupt-")
        }));
        assert_no_temporary(dir.path());
    }

    #[test]
    fn newest_editor_invalid_candidate_falls_back_without_modifying_old_copies() {
        use crate::{files::FileFingerprint, hash};
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let text = "Exact saved baseline λ\r\n";
        let fingerprint = FileFingerprint {
            device: 1,
            inode: 4,
            len: text.len() as u64,
            sha256: hash(text.as_bytes()),
        };
        let editor = store.open_editor("note.md", &fingerprint, text).unwrap();
        let healthy = created(store.checkpoint_if_changed().unwrap());
        drop(store);
        let damaged = backups_dir(dir.path()).join("brn-9999999999999.sqlite");
        std::fs::copy(&healthy, &damaged).unwrap();
        let conn = Connection::open(&damaged).unwrap();
        conn.execute("UPDATE editors SET record_sha256=?1", [vec![0_u8; 32]])
            .unwrap();
        drop(conn);
        let prior = copies(dir.path());
        std::fs::write(
            dir.path().join("brn.sqlite"),
            b"synthetic physical corruption",
        )
        .unwrap();
        let (restored, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(report.restored_from, Some(healthy));
        assert_eq!(restored.editor("note.md").unwrap(), Some(editor));
        for (path, bytes) in prior {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert_no_temporary(dir.path());
    }

    #[test]
    fn large_synthetic_checkpoint_reports_measured_latency_and_complete_bytes() {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let body = "synthetic large internal state λ\r\n".repeat(16 * 1024);
        let mut conversation = None;
        for _ in 0..64 {
            let turn = store
                .begin_turn(Uuid::new_v4(), conversation, &body, "chatgpt", "synthetic")
                .unwrap();
            conversation = Some(turn.conversation_id);
            store
                .finish_turn(turn.id, WorkTurnStatus::Completed, &body, None)
                .unwrap();
        }
        let started = Instant::now();
        let path = created(store.checkpoint_if_changed().unwrap());
        let elapsed = started.elapsed();
        let size = std::fs::metadata(&path).unwrap().len();
        eprintln!(
            "synthetic operational checkpoint (64 complete chat turns): {size} bytes, {} ms including validation/fsync/publication; per-step wall time is not guaranteed",
            elapsed.as_millis()
        );
        assert!(size > 64 * 1024 * 1024);
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages WHERE text=?1",
                [&body],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 128);
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
        assert_no_temporary(dir.path());
    }
    fn all_rows(conn: &Connection) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
        let tables = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        tables
            .into_iter()
            .map(|table| {
                let mut query = conn
                    .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                    .unwrap();
                let columns = query.column_count();
                let rows = query
                    .query_map([], |row| {
                        (0..columns)
                            .map(|column| row.get(column))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap();
                (table, rows)
            })
            .collect()
    }

    #[test]
    fn restored_v18_operational_checkpoint_keeps_every_table_and_exact_replay() {
        use crate::work::{
            actions::*, editor::EditRequest, findings::*, inbox::*, intake::IntakeSnapshot,
            proposals::*,
        };
        use crate::{
            files::{FileFingerprint, VaultIdentity, VaultRecord},
            hash,
        };
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let budget = WorkBudget {
            max_tool_rounds: 3,
            timeout_seconds: 90,
        };
        let (turn, _) = store
            .begin_turn_with_effort_and_budget(
                Uuid::new_v4(),
                None,
                "Exact question λ\r\n",
                "chatgpt",
                "synthetic",
                Some("medium"),
                Some(budget),
            )
            .unwrap();
        let terminal = store
            .finish_turn(
                turn.id,
                WorkTurnStatus::Failed,
                "Retained partial 日本語\r\n",
                Some("network"),
            )
            .unwrap();
        let action_id = Uuid::new_v4();
        let action_data = ActionData {
            title: "Clarify exact source".into(),
            description: "Owner work λ".into(),
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
        };
        let draft = ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: Some(turn.conversation_id),
            vault: None,
            title: "Owner review".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id: action_id,
                data: action_data.clone(),
            }],
        };
        let proposal = store.create_proposal(&draft).unwrap();
        let edited = store
            .edit_proposal(&ProposalEdit {
                expected: proposal.stamp(),
                title: "Edited owner review λ".into(),
                texts: vec![],
                action_data: vec![action_data.clone()],
            })
            .unwrap();
        let commented = store
            .add_proposal_comment(&CommentRequest {
                expected: edited.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Exact unresolved review comment 日本語".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        // Private synthetic insertion creates a genuine checked retained record;
        // it does not add a production Action producer or filesystem effect.
        let origin = ActionOrigin {
            id: action_id,
            proposal: proposal.stamp(),
            data: action_data.clone(),
            created_at_ms: 7,
        };
        let action = ActionRecord {
            origin: origin.clone(),
            version: 1,
            data: action_data,
            updated_at_ms: 7,
            waiting_since_ms: None,
            completed_at_ms: None,
        };
        let bytes = serde_json::to_vec(&action).unwrap();
        store
            .conn
            .execute(
                "INSERT INTO actions VALUES(?1,1,'open',7,?2,?3,?4)",
                params![
                    action_id.to_string(),
                    hash(&serde_json::to_vec(&origin).unwrap()).as_slice(),
                    bytes,
                    hash(&bytes).as_slice()
                ],
            )
            .unwrap();
        let text = "Exact saved baseline λ\r\n";
        let fingerprint = FileFingerprint {
            device: 1,
            inode: 4,
            len: text.len() as u64,
            sha256: hash(text.as_bytes()),
        };
        let editor = store.open_editor("note.md", &fingerprint, text).unwrap();
        let recovered = store
            .recover_editor(&EditRequest {
                path: "note.md".into(),
                expected: editor.stamp,
                generation: 1,
                text: "Retained unsaved owner text 日本語".into(),
            })
            .unwrap();
        let note = Uuid::new_v4();
        let finding_draft = FindingDraft {
            request: CaptureFindingRequest {
                id: Uuid::new_v4(),
                origin: FindingOrigin::IdentityAmbiguity { note_id: note },
            },
            vault: VaultRecord {
                id: Uuid::new_v4(),
                root: "/synthetic/vault".into(),
                identity: VaultIdentity {
                    device: 1,
                    inode: 1,
                },
            },
            title: "Conflicting identity".into(),
            summary: "Keep exact evidence".into(),
            evidence: ["note.md", "other.md"]
                .iter()
                .map(|path| FindingEvidence {
                    source: SourceVersion {
                        path: (*path).into(),
                        fingerprint: fingerprint.clone(),
                    },
                    note_id: Some(note),
                    quote: Some(FindingQuote {
                        start_byte: 0,
                        end_byte: text.len(),
                        quote: text.into(),
                    }),
                })
                .collect(),
        };
        let finding = store.create_finding(&finding_draft).unwrap();
        let original_bytes = b"From: synthetic@example.test\r\n\r\nRetained opaque \x00\xff";
        let original = InboxItem {
            received_at_ms: 7,
            capture: InboxCapture {
                id: Uuid::new_v4(),
                kind: InboxKind::Email,
                title: "Synthetic retained mail".into(),
                original_name: Some("mail.eml".into()),
                copy: InboxCopy {
                    directory: "/synthetic/originals".into(),
                    directory_device: 1,
                    directory_inode: 2,
                    file_device: 1,
                    file_inode: 3,
                    byte_len: original_bytes.len() as u64,
                    sha256: hash(original_bytes),
                },
            },
        };
        store.restore_inbox(&original).unwrap();
        let intake = IntakeSnapshot {
            id: Uuid::new_v4(),
            batch_id: Uuid::new_v4(),
            index: 0,
            original,
            extraction: brn_intake::Extraction {
                limits: Default::default(),
                consumed: None,
                schema: 1,
                converter: "synthetic-adapter/1".into(),
                original_sha256: hash(original_bytes),
                markdown: "Extracted mail λ".into(),
                sources: vec![brn_intake::SourceNode {
                    id: "source-1".into(),
                    parent: None,
                    name: "mail.eml".into(),
                    media_type: "message/rfc822".into(),
                    locator: "original".into(),
                    status: "partial".into(),
                    bytes: original_bytes.to_vec(),
                    text: "Extracted mail λ".into(),
                }],
                assets: vec![],
                occurrences: vec![],
                gaps: vec!["Synthetic retained opaque suffix".into()],
            },
        };
        store.restore_intake_snapshot(&intake).unwrap();
        let archive = ConversationLifecycleRequest {
            operation_id: Uuid::new_v4(),
            expected: store
                .conversation_lifecycle(turn.conversation_id)
                .unwrap()
                .stamp,
            target: ConversationState::Archived,
        };
        let archived = store.set_conversation_lifecycle(&archive).unwrap();
        let rows = all_rows(&store.conn);
        let path = created(store.checkpoint_if_changed().unwrap());
        assert_eq!(
            store.set_conversation_lifecycle(&archive).unwrap(),
            archived
        );
        assert_eq!(store.restore_intake_snapshot(&intake).unwrap(), intake);
        assert_eq!(
            store.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
        drop(store);
        std::fs::write(
            dir.path().join("brn.sqlite"),
            b"synthetic physical corruption",
        )
        .unwrap();
        let (mut restored, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(report.restored_from, Some(path));
        assert_eq!(all_rows(&restored.conn), rows);
        assert_same_turn(restored.turn(turn.id).unwrap().unwrap(), terminal);
        assert_eq!(restored.run_budget(turn.id).unwrap(), Some(budget));
        assert_eq!(restored.proposal(draft.id).unwrap(), Some(commented));
        assert_eq!(restored.action(action_id).unwrap(), Some(action));
        assert_eq!(
            restored.finding(finding_draft.request.id).unwrap(),
            Some(finding)
        );
        assert_eq!(restored.editor("note.md").unwrap(), Some(recovered));
        assert_eq!(
            restored.intake_snapshot(intake.id).unwrap(),
            Some(intake.clone())
        );
        assert_eq!(
            restored.conversation_lifecycle_replay(&archive).unwrap(),
            Some(archived.clone())
        );
        assert_eq!(
            restored.set_conversation_lifecycle(&archive).unwrap(),
            archived
        );
        assert_eq!(restored.restore_intake_snapshot(&intake).unwrap(), intake);
        assert_eq!(
            restored.checkpoint_if_changed().unwrap(),
            BackupCheckpointOutcome::Unchanged
        );
    }
}
