use crate::*;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub const DATABASE_FILENAME: &str = "threads.sqlite3";
pub const DATA_MARKER: &str = ".brn-threads-data";
/// The workspace's only directory for rebuildable derived data, such as the
/// search index. It is never backed up or restored, and may be deleted.
pub const DERIVED_DIRECTORY: &str = "derived";
const IDENTITY: &str = "BRN Threads SQLite v1\n";
pub(crate) const APPLICATION_ID: i64 = 0x42525448;
pub(crate) const SCHEMA_VERSION: i64 = 4;
pub struct Store {
    pub(crate) conn: Connection,
    directory: PathBuf,
}
enum SettingsUpdate<'a> {
    Configure(&'a WorkspaceSettings),
    Choose(DelegationChoice),
    Pause(bool),
    BackupDirectory(&'a str),
    Theme(&'a str),
    TopicsNote(&'a str),
}
impl Store {
    pub fn open(directory: impl AsRef<Path>) -> Result<Self> {
        let directory = directory.as_ref();
        if !directory.is_absolute() {
            return Err(Error::UnsafeDirectory(
                "an absolute explicit directory is required".into(),
            ));
        }
        if has_symlink_ancestor(directory)? {
            return Err(Error::UnsafeDirectory("symlink directory".into()));
        }
        fs::create_dir_all(directory)?;
        let marker = directory.join(DATA_MARKER);
        let entries: Vec<_> = fs::read_dir(directory)?.collect::<std::result::Result<_, _>>()?;
        let existing = marker.exists();
        if !existing && !entries.is_empty() {
            return Err(Error::UnsafeDirectory(
                "nonempty directory without Threads marker".into(),
            ));
        }
        if existing {
            if fs::symlink_metadata(&marker)?.file_type().is_symlink()
                || fs::read_to_string(&marker)? != IDENTITY
            {
                return Err(Error::UnsafeDirectory("foreign marker".into()));
            }
            for entry in entries {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let kind = entry.file_type()?;
                if ![
                    DATA_MARKER,
                    DATABASE_FILENAME,
                    "threads.sqlite3-wal",
                    "threads.sqlite3-shm",
                    DERIVED_DIRECTORY,
                ]
                .contains(&name.as_ref())
                    || kind.is_symlink()
                    || (name == DERIVED_DIRECTORY) != kind.is_dir()
                {
                    return Err(Error::UnsafeDirectory(format!(
                        "unexpected directory entry: {name}"
                    )));
                }
            }
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)?
                .write_all(IDENTITY.as_bytes())?;
        }
        let database = directory.join(DATABASE_FILENAME);
        let mut conn = if existing {
            if !database.exists() {
                return Err(missing_storage());
            }
            // Do not allow a disappearing database to become a fresh one. The
            // same connection is validated before any mutating SQLite pragma.
            open_existing_database(&database)?
        } else {
            Connection::open(&database)?
        };
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        if !existing {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE operations(id TEXT PRIMARY KEY, host_key TEXT UNIQUE NOT NULL, input_hash TEXT, hash TEXT, request TEXT, undo_of TEXT, replacement_of TEXT, retired INTEGER NOT NULL DEFAULT 0); CREATE TABLE candidate_targets(note TEXT PRIMARY KEY, operation TEXT NOT NULL REFERENCES operations(id)); CREATE TABLE records(id TEXT PRIMARY KEY, version INTEGER NOT NULL, body TEXT NOT NULL); CREATE UNIQUE INDEX one_workspace_settings ON records((1)) WHERE json_type(body,'$.data.Settings') IS NOT NULL; CREATE UNIQUE INDEX source_locator ON records(json_extract(body,'$.data.Source.locator')) WHERE json_type(body,'$.data.Source') IS NOT NULL; CREATE TABLE revisions(id TEXT NOT NULL, version INTEGER NOT NULL, operation TEXT NOT NULL REFERENCES operations(id), body TEXT NOT NULL, PRIMARY KEY(id,version)); CREATE TABLE receipts(operation TEXT PRIMARY KEY REFERENCES operations(id), body TEXT NOT NULL); CREATE TABLE edit_sessions(id TEXT PRIMARY KEY, note TEXT NOT NULL, base INTEGER NOT NULL, generation INTEGER NOT NULL, markdown TEXT NOT NULL, closed INTEGER NOT NULL DEFAULT 0); CREATE UNIQUE INDEX one_dirty_session ON edit_sessions(note) WHERE closed=0; CREATE TABLE derivation_inputs(operation TEXT NOT NULL, record TEXT NOT NULL, version INTEGER NOT NULL);")?;
            tx.execute_batch(crate::upgrade::CHAT_SCHEMA)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            tx.commit()?;
        }
        Ok(Self {
            conn,
            directory: directory.to_owned(),
        })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Creates (if needed) and returns the real `derived/` directory. Refuses a
    /// symlink or non-directory there, or a symlink among its ancestors.
    pub fn derived_directory(&self) -> Result<PathBuf> {
        ensure_derived(&self.directory)
    }
    /// Host key is a stable command/run request identity, saved before prepare.
    pub fn allocate_operation(&mut self, host_request_key: &str) -> Result<OperationId> {
        self.allocate_with_binding(host_request_key, None)
    }
    /// Persist the host input identity before preparing effects, closing the
    /// allocation-to-prepare retry gap for intake and other bounded host work.
    pub fn allocate_bound_operation(
        &mut self,
        host_request_key: &str,
        input_hash: &str,
    ) -> Result<OperationId> {
        if input_hash.len() != 64 || !input_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::Invalid(
                "input binding must be a SHA-256 digest".into(),
            ));
        }
        self.allocate_with_binding(host_request_key, Some(input_hash))
    }
    fn allocate_with_binding(&mut self, key: &str, binding: Option<&str>) -> Result<OperationId> {
        if key.trim().is_empty() {
            return Err(Error::Invalid("empty host request key".into()));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT id,input_hash FROM operations WHERE host_key=?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let id = if let Some((id, existing)) = prior {
            if existing.as_deref() != binding {
                return Err(Error::ImmutableRequest);
            }
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO operations(id,host_key,input_hash) VALUES (?1,?2,?3)",
                params![id, key, binding],
            )?;
            id
        };
        tx.commit()?;
        Ok(OperationId(id))
    }
    pub fn operation(&self, id: &str) -> Result<OperationId> {
        let found: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1)",
            [id],
            |r| r.get(0),
        )?;
        if found {
            Ok(OperationId(id.into()))
        } else {
            Err(Error::Missing(id.into()))
        }
    }
    /// Retire an unapplied candidate before preparing a replacement. Receipts remain.
    pub fn retire(&mut self, operation: &OperationId) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if receipt_on(&tx, operation)?.is_some() {
            return Err(Error::Invalid(
                "applied operations cannot be retired".into(),
            ));
        }
        if tx.execute(
            "UPDATE operations SET retired=1 WHERE id=?1",
            [&operation.0],
        )? == 0
        {
            return Err(Error::Missing(operation.0.clone()));
        }
        tx.execute(
            "DELETE FROM candidate_targets WHERE operation=?1",
            [&operation.0],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn prepare(
        &mut self,
        operation: &OperationId,
        request: &ChangeRequest,
    ) -> Result<Prepared> {
        self.prepare_inner(operation, request, false)
    }
    /// Owner direct actions bypass the pending-review slot, but use precisely
    /// the same immutable candidate and checked application boundary.
    pub fn prepare_owner(
        &mut self,
        operation: &OperationId,
        request: &ChangeRequest,
        authority: &HostAuthority,
    ) -> Result<Prepared> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "direct preparation requires owner host authority".into(),
            ));
        }
        self.prepare_inner(operation, request, true)
    }
    fn prepare_inner(
        &mut self,
        operation: &OperationId,
        request: &ChangeRequest,
        owner_direct: bool,
    ) -> Result<Prepared> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prepared = prepare_on(&tx, operation, request, owner_direct)?;
        tx.commit()?;
        Ok(prepared)
    }
    /// Replace an owner-edited review candidate atomically. A failed preparation
    /// rolls retirement and note-slot changes back with the same transaction.
    pub fn replace_candidate(
        &mut self,
        original: &OperationId,
        replacement: &OperationId,
        request: &ChangeRequest,
        authority: &HostAuthority,
    ) -> Result<Prepared> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "candidate replacement requires owner authority".into(),
            ));
        }
        self.replace_candidate_inner(original, replacement, request, authority, false)
    }
    /// Prepare a fresh candidate in this runtime's Thread family. This grants
    /// no permission to apply its content; the normal checked boundary remains.
    pub fn refresh_candidate(
        &mut self,
        original: &OperationId,
        replacement: &OperationId,
        request: &ChangeRequest,
        authority: &HostAuthority,
    ) -> Result<Prepared> {
        if !matches!(authority.scope, crate::types::Scope::Runtime { .. }) {
            return Err(Error::Invalid(
                "candidate refresh requires fenced runtime authority".into(),
            ));
        }
        self.replace_candidate_inner(original, replacement, request, authority, true)
    }
    fn replace_candidate_inner(
        &mut self,
        original: &OperationId,
        replacement: &OperationId,
        request: &ChangeRequest,
        authority: &HostAuthority,
        runtime_refresh: bool,
    ) -> Result<Prepared> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let refresh_thread = if runtime_refresh {
            Some(refresh_scope_on(&tx, original, replacement, authority)?)
        } else {
            None
        };
        // Replacement identity binds both the original and the complete immutable
        // request. Replay precedes retirement, applied receipts, and current bases.
        let (existing_hash, replacement_of): (Option<String>, Option<String>) = tx.query_row(
            "SELECT hash,replacement_of FROM operations WHERE id=?1",
            [replacement.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if let Some(bound) = replacement_of {
            let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(request)?));
            if bound != original.as_str() || existing_hash.as_deref() != Some(&hash) {
                return Err(Error::ImmutableRequest);
            }
            let prepared = prepared_on(&tx, replacement)?;
            tx.commit()?;
            return Ok(prepared);
        }
        if existing_hash.is_some() {
            return Err(Error::ImmutableRequest);
        }
        if original == replacement {
            return Err(Error::Invalid(
                "replacement must have a fresh identity".into(),
            ));
        }
        let retired: bool = tx.query_row(
            "SELECT retired FROM operations WHERE id=?1",
            [original.as_str()],
            |r| r.get(0),
        )?;
        if retired || receipt_on(&tx, original)?.is_some() {
            return Err(Error::Invalid(
                "only active unapplied candidates can be replaced".into(),
            ));
        }
        let original_prepared = prepared_on(&tx, original)?;
        for marker in writing_markers_on(&tx, &original_prepared.request)? {
            if !request.inputs.contains(&marker) {
                return Err(Error::Invalid(
                    "writing review context cannot be removed from a replacement".into(),
                ));
            }
        }
        if runtime_refresh {
            for write in &request.writes {
                if matches!(write.data, RecordData::Run(_) | RecordData::Settings(_)) {
                    return Err(Error::Invalid(
                        "candidate refresh cannot alter host records".into(),
                    ));
                }
                if record_on(&tx, &write.id)?
                    .as_ref()
                    .map(|record| record.version)
                    != write.expected_version
                {
                    return Err(Error::Invalid(
                        "candidate refresh requires current target versions".into(),
                    ));
                }
            }
            for input in &request.inputs {
                if input.expectation == InputExpectation::Current
                    && !current_input_matches(&tx, input)?
                {
                    return Err(Error::Invalid(
                        "candidate refresh requires current input versions".into(),
                    ));
                }
            }
        }
        tx.execute(
            "UPDATE operations SET retired=1 WHERE id=?1",
            [original.as_str()],
        )?;
        tx.execute(
            "DELETE FROM candidate_targets WHERE operation=?1",
            [original.as_str()],
        )?;
        let prepared = prepare_on(&tx, replacement, request, false)?;
        tx.execute(
            "UPDATE operations SET replacement_of=?2 WHERE id=?1",
            params![replacement.as_str(), original.as_str()],
        )?;
        let mut attention_writes = vec![];
        {
            let mut stmt = tx.prepare(
                "SELECT body FROM records WHERE json_type(body,'$.data.Thread') IS NOT NULL",
            )?;
            for body in stmt.query_map([], |row| row.get::<_, String>(0))? {
                let record: Record = serde_json::from_str(&body?)?;
                if refresh_thread
                    .as_ref()
                    .is_some_and(|thread| record.id != *thread)
                    || request.writes.iter().any(|write| write.id == record.id)
                {
                    continue;
                }
                let RecordData::Thread(mut thread) = record.data else {
                    continue;
                };
                let mut changed = false;
                for attention in &mut thread.attention {
                    if (attention.kind == AttentionKind::Review
                        || runtime_refresh && attention.kind == AttentionKind::Conflict)
                        && attention.record.as_deref() == Some(original.as_str())
                    {
                        attention.record = Some(replacement.as_str().into());
                        changed = true;
                    }
                }
                if changed {
                    attention_writes.push(Put {
                        id: record.id,
                        expected_version: Some(record.version),
                        archived: record.archived,
                        data: RecordData::Thread(thread),
                    });
                }
            }
        }
        if !attention_writes.is_empty() {
            let metadata = OperationId(uuid::Uuid::new_v4().to_string());
            let key = format!("replacement-attention:{}", replacement.as_str());
            tx.execute(
                "INSERT INTO operations(id,host_key) VALUES (?1,?2)",
                params![metadata.0, key],
            )?;
            let request = ChangeRequest {
                reason: "Attention follows immutable replacement candidate".into(),
                writes: attention_writes,
                inputs: vec![],
            };
            prepare_on(&tx, &metadata, &request, true)?;
            // This host-selected metadata request never includes model content.
            let metadata_authority = HostAuthority::owner("candidate attention bookkeeping");
            let metadata_authority = if let Some((run, fence)) = &authority.run_binding {
                metadata_authority.for_run(run, *fence)
            } else {
                metadata_authority
            };
            if !matches!(
                apply_on(&tx, &metadata, &metadata_authority, None)?,
                ApplyOutcome::Applied(_)
            ) {
                return Err(Error::Invalid(
                    "replacement attention could not be committed".into(),
                ));
            }
        }
        tx.commit()?;
        Ok(prepared)
    }
    pub fn prepared(&self, operation: &OperationId) -> Result<Prepared> {
        prepared_on(&self.conn, operation)
    }
    pub fn record(&self, id: &str) -> Result<Option<Record>> {
        record_on(&self.conn, id)
    }
    pub fn note(&self, id: &str) -> Result<Option<Record>> {
        match self.record(id)? {
            Some(record) if matches!(record.data, RecordData::Note(_)) => Ok(Some(record)),
            None => Ok(None),
            _ => Err(Error::Invalid("record is not a note".into())),
        }
    }
    pub fn records(&self) -> Result<Vec<Record>> {
        let mut stmt = self.conn.prepare("SELECT body FROM records ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub fn revision(&self, id: &str, version: u64) -> Result<Option<Record>> {
        let body: Option<String> = self
            .conn
            .query_row(
                "SELECT body FROM revisions WHERE id=?1 AND version=?2",
                params![id, crate::integer(version)?],
                |r| r.get(0),
            )
            .optional()?;
        body.map(|body| Ok(serde_json::from_str(&body)?))
            .transpose()
    }
    pub fn receipt(&self, operation: &OperationId) -> Result<Option<Receipt>> {
        receipt_on(&self.conn, operation)
    }
    pub fn receipts(&self) -> Result<Vec<Receipt>> {
        let mut stmt = self
            .conn
            .prepare("SELECT body FROM receipts ORDER BY rowid")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect()
    }
    pub fn candidates(&self) -> Result<Vec<Prepared>> {
        let mut stmt=self.conn.prepare("SELECT id FROM operations WHERE hash IS NOT NULL AND retired=0 AND NOT EXISTS(SELECT 1 FROM receipts WHERE operation=operations.id) ORDER BY rowid")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .map(|row| prepared_on(&self.conn, &OperationId(row?)))
            .collect()
    }
    pub fn apply(
        &mut self,
        operation: &OperationId,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let outcome = apply_on(&tx, operation, authority, None)?;
        tx.commit()?;
        Ok(outcome)
    }
    pub fn undo(
        &mut self,
        compensation: &OperationId,
        original: &OperationId,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Bind compensation identity before replaying its existing receipt.
        let prior: Option<String> = tx.query_row(
            "SELECT undo_of FROM operations WHERE id=?1",
            [&compensation.0],
            |r| r.get(0),
        )?;
        if prior.as_deref().is_some_and(|prior| prior != original.0) {
            return Err(Error::ImmutableRequest);
        }
        if receipt_on(&tx, compensation)?.is_some() {
            if prior.as_deref() != Some(original.as_str()) {
                return Err(Error::ImmutableRequest);
            }
            let outcome = apply_on(&tx, compensation, authority, None)?;
            tx.commit()?;
            return Ok(outcome);
        }
        let receipt =
            receipt_on(&tx, original)?.ok_or_else(|| Error::Missing(original.0.clone()))?;
        if receipt.writes.iter().any(run_control_written) {
            return Err(Error::Invalid(
                "run execution control cannot be compensated; continue with a fresh Run".into(),
            ));
        }
        let mut request = ChangeRequest {
            reason: format!("Undo: {}", receipt.reason),
            writes: receipt
                .writes
                .iter()
                .map(|written| {
                    let restored = written.before.as_ref().unwrap_or(&written.after);
                    Put {
                        id: restored.id.clone(),
                        expected_version: Some(written.after.version),
                        archived: if written.before.is_none() {
                            true
                        } else {
                            restored.archived
                        },
                        data: restored.data.clone(),
                    }
                })
                .collect(),
            inputs: vec![],
        };
        let restored_notes = request
            .writes
            .iter()
            .filter_map(|write| match &write.data {
                RecordData::Note(note) => Some((
                    write.id.clone(),
                    (
                        write.expected_version.unwrap_or(0) + 1,
                        note.markdown.clone(),
                    ),
                )),
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for write in &mut request.writes {
            if !write.archived
                && let RecordData::Comment(comment) = &mut write.data
                && let Some((version, text)) = restored_notes.get(&comment.note)
            {
                let range = comment.mapped_range.or(comment.range);
                if !comment.unresolved
                    && range.is_some_and(|(start, end)| {
                        text.get(start..end) == Some(comment.quote.as_str())
                    })
                {
                    comment.mapped_version = Some(*version);
                    comment.mapped_range = range;
                } else if comment.range.is_some() || !comment.quote.is_empty() {
                    comment.mapped_version = Some(*version);
                    comment.mapped_range = None;
                    comment.unresolved = true;
                }
            }
        }
        let body = serde_json::to_string(&request)?;
        let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
        let prepared_hash: Option<String> = tx.query_row(
            "SELECT hash FROM operations WHERE id=?1",
            [&compensation.0],
            |r| r.get(0),
        )?;
        if prepared_hash.as_ref().is_some_and(|prior| prior != &hash) {
            return Err(Error::ImmutableRequest);
        }
        tx.execute(
            "UPDATE operations SET hash=?2,request=?3,undo_of=?4 WHERE id=?1",
            params![compensation.0, hash, body, original.0],
        )?;
        let outcome = apply_on(&tx, compensation, authority, None)?;
        tx.commit()?;
        Ok(outcome)
    }
    /// Owner configuration and stopping of every current invocation share the
    /// writer transaction, so a concurrently created run cannot escape the fence.
    pub fn configure_workspace(
        &mut self,
        operation: &OperationId,
        settings: &WorkspaceSettings,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        self.update_workspace_settings(operation, SettingsUpdate::Configure(settings), authority)
    }

    /// Apply only the owner's choice against current Settings, with immutable
    /// intent and run fencing in the same checked writer transaction.
    pub fn choose_delegation(
        &mut self,
        operation: &OperationId,
        choice: DelegationChoice,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        self.update_workspace_settings(operation, SettingsUpdate::Choose(choice), authority)
    }

    pub fn pause_workspace(
        &mut self,
        operation: &OperationId,
        paused: bool,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        self.update_workspace_settings(operation, SettingsUpdate::Pause(paused), authority)
    }

    /// Save only the owner's backup destination. Bind the original UTF-8 path
    /// before normalization so committed retries survive an unavailable drive.
    pub fn choose_backup_directory(
        &mut self,
        operation: &OperationId,
        directory: &Path,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        let directory = directory
            .to_str()
            .ok_or_else(|| Error::UnsafeDirectory("backup folder must be a UTF-8 path".into()))?;
        self.update_workspace_settings(
            operation,
            SettingsUpdate::BackupDirectory(directory),
            authority,
        )
    }

    /// Save the owner's appearance preference. Settings writes change the version
    /// that running invocations are bound to, so this is deferred, inside the
    /// writer transaction, while any run is Working instead of stopping it.
    pub fn choose_theme(
        &mut self,
        operation: &OperationId,
        theme: &str,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        if theme.is_empty()
            || theme.len() > 40
            || !theme
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        {
            return Err(Error::Invalid("unsupported theme name".into()));
        }
        self.update_workspace_settings(operation, SettingsUpdate::Theme(theme), authority)
    }

    /// Points the workspace at its root Topics note (D6). The target must be a
    /// current, non-archived note outside any chat. Owner configuration only;
    /// deferred while a run is working, like the theme.
    pub fn choose_topics_note(
        &mut self,
        operation: &OperationId,
        note: &str,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        if note.is_empty() || note.len() > 256 {
            return Err(Error::Invalid("unsupported Topics note ID".into()));
        }
        self.update_workspace_settings(operation, SettingsUpdate::TopicsNote(note), authority)
    }

    /// Recheck the saved destination against this actual workspace at dispatch.
    pub fn validate_backup_directory(&self, directory: &Path) -> Result<PathBuf> {
        validate_backup_directory(&self.directory, directory)
    }

    fn update_workspace_settings(
        &mut self,
        operation: &OperationId,
        update: SettingsUpdate<'_>,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "workspace configuration requires owner authority".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (hash, prior_binding): (Option<String>, Option<String>) = tx.query_row(
            "SELECT hash,input_hash FROM operations WHERE id=?1",
            [operation.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let intent = match update {
            SettingsUpdate::Choose(choice) => Some(serde_json::to_vec(&choice)?),
            SettingsUpdate::Pause(paused) => Some(serde_json::to_vec(&paused)?),
            SettingsUpdate::BackupDirectory(directory) => {
                Some(serde_json::to_vec(&("backup-directory", directory))?)
            }
            SettingsUpdate::Theme(theme) => Some(serde_json::to_vec(&("theme", theme))?),
            SettingsUpdate::TopicsNote(note) => Some(serde_json::to_vec(&("topics-note", note))?),
            SettingsUpdate::Configure(_) => None,
        };
        if let Some(intent) = intent {
            let binding = format!("{:x}", Sha256::digest(intent));
            if prior_binding
                .as_ref()
                .is_some_and(|prior| prior != &binding)
                || (hash.is_some() && prior_binding.is_none())
            {
                return Err(Error::ImmutableRequest);
            }
            tx.execute(
                "UPDATE operations SET input_hash=?2 WHERE id=?1 AND input_hash IS NULL",
                params![operation.as_str(), binding],
            )?;
        }
        if hash.is_some() {
            let prepared = prepared_on(&tx, operation)?;
            let configured = prepared
                .request
                .writes
                .iter()
                .filter_map(|write| {
                    if let RecordData::Settings(value) = &write.data {
                        Some(value)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let [prior] = configured.as_slice() else {
                return Err(Error::ImmutableRequest);
            };
            match update {
                SettingsUpdate::Configure(settings) => {
                    let mut requested = settings.clone();
                    requested.delegation_chosen = prior.delegation_chosen;
                    requested.maintenance = prior.maintenance;
                    requested.review_first = prior.review_first;
                    requested.backup_directory = prior.backup_directory.clone();
                    requested.theme = prior.theme.clone();
                    requested.topics_note = prior.topics_note.clone();
                    if &requested != *prior {
                        return Err(Error::ImmutableRequest);
                    }
                }
                SettingsUpdate::Choose(choice) => {
                    if !prior.delegation_chosen
                        || !prior.maintenance
                        || prior.review_first != (choice == DelegationChoice::ReviewFirst)
                    {
                        return Err(Error::ImmutableRequest);
                    }
                }
                SettingsUpdate::Pause(paused) => {
                    if prior.maintenance == paused {
                        return Err(Error::ImmutableRequest);
                    }
                }
                SettingsUpdate::BackupDirectory(_) => {}
                SettingsUpdate::Theme(theme) => {
                    if prior.theme.as_deref() != Some(theme) {
                        return Err(Error::ImmutableRequest);
                    }
                }
                SettingsUpdate::TopicsNote(note) => {
                    if prior.topics_note.as_deref() != Some(note) {
                        return Err(Error::ImmutableRequest);
                    }
                }
            }
            let outcome = apply_on(&tx, operation, authority, None)?;
            tx.commit()?;
            return Ok(outcome);
        }
        let body: String = tx.query_row(
            "SELECT body FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL AND json_extract(body,'$.archived')=0",
            [], |row| row.get(0),
        )?;
        let current: Record = serde_json::from_str(&body)?;
        let RecordData::Settings(current_settings) = &current.data else {
            return Err(Error::Invalid("workspace settings kind mismatch".into()));
        };
        let theme_only = matches!(update, SettingsUpdate::Theme(_));
        // Preferences never change authority, so they wait for working runs
        // instead of stopping them.
        if matches!(
            update,
            SettingsUpdate::Theme(_) | SettingsUpdate::TopicsNote(_)
        ) {
            let mut guarded = Vec::new();
            let mut stmt = tx.prepare(
                "SELECT body FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL",
            )?;
            for body in stmt.query_map([], |row| row.get::<_, String>(0))? {
                let record: Record = serde_json::from_str(&body?)?;
                if matches!(&record.data, RecordData::Run(run) if run.state == RunState::Working) {
                    guarded.push(record.id);
                }
            }
            if !guarded.is_empty() {
                return Ok(ApplyOutcome::Deferred { guarded });
            }
        }
        let mut next = match update {
            SettingsUpdate::Configure(settings) => {
                let mut next = settings.clone();
                next.delegation_chosen = current_settings.delegation_chosen;
                next.maintenance = current_settings.maintenance;
                next.review_first = current_settings.review_first;
                next.backup_directory = current_settings.backup_directory.clone();
                next.theme = current_settings.theme.clone();
                next.topics_note = current_settings.topics_note.clone();
                next
            }
            _ => current_settings.clone(),
        };
        match update {
            SettingsUpdate::Choose(choice) => {
                next.delegation_chosen = true;
                next.maintenance = true;
                next.review_first = choice == DelegationChoice::ReviewFirst;
            }
            SettingsUpdate::Pause(paused) => {
                if !paused && !current_settings.delegation_chosen {
                    return Err(Error::Invalid(
                        "Choose how BRN should maintain notes before resuming".into(),
                    ));
                }
                next.maintenance = !paused;
            }
            SettingsUpdate::BackupDirectory(directory) => {
                let normalized = validate_backup_directory(&self.directory, Path::new(directory))?;
                next.backup_directory = Some(
                    normalized
                        .to_str()
                        .ok_or_else(|| {
                            Error::UnsafeDirectory("backup folder must be a UTF-8 path".into())
                        })?
                        .to_owned(),
                );
            }
            SettingsUpdate::Theme(theme) => next.theme = Some(theme.to_owned()),
            SettingsUpdate::TopicsNote(note) => {
                let body: Option<String> = tx
                    .query_row("SELECT body FROM records WHERE id=?1", [note], |row| {
                        row.get(0)
                    })
                    .optional()?;
                let target = body
                    .map(|b| serde_json::from_str::<Record>(&b))
                    .transpose()?;
                if !matches!(&target, Some(Record { archived: false, data: RecordData::Note(n), .. })
                    if n.chat.is_none() && n.superseded_by.is_none())
                {
                    return Err(Error::Invalid(
                        "the Topics note must be a current note outside any chat".into(),
                    ));
                }
                next.topics_note = Some(note.to_owned());
            }
            SettingsUpdate::Configure(_) => {}
        }
        let mut writes = vec![Put {
            id: current.id,
            expected_version: Some(current.version),
            archived: false,
            data: RecordData::Settings(next),
        }];
        {
            let mut stmt = tx.prepare(
                "SELECT body FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL",
            )?;
            for body in stmt.query_map([], |row| row.get::<_, String>(0))? {
                let record: Record = serde_json::from_str(&body?)?;
                if let RecordData::Run(mut run) = record.data
                    && run.state == RunState::Working
                {
                    run.state = RunState::Cancelled;
                    run.fence = run
                        .fence
                        .checked_add(1)
                        .ok_or_else(|| Error::Invalid("run fence exhausted".into()))?;
                    writes.push(Put {
                        id: record.id,
                        expected_version: Some(record.version),
                        archived: record.archived,
                        data: RecordData::Run(run),
                    });
                }
            }
        }
        let request = ChangeRequest {
            reason: if theme_only {
                "Owner chose the workspace theme".into()
            } else if matches!(update, SettingsUpdate::TopicsNote(_)) {
                "Owner chose the root Topics note".into()
            } else {
                "Owner configured workspace and stopped prior invocations".into()
            },
            writes,
            inputs: vec![],
        };
        prepare_on(&tx, operation, &request, true)?;
        let outcome = apply_on(&tx, operation, authority, None)?;
        tx.commit()?;
        Ok(outcome)
    }
}
fn open_existing_database(database: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(database, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|error| {
        if !database.exists() {
            missing_storage()
        } else {
            error.into()
        }
    })?;
    let schema: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if schema == 3 {
        crate::upgrade::upgrade_connection(&conn, database, None)?;
    }
    validate_connection(&conn)?;
    Ok(conn)
}
fn missing_storage() -> Error {
    Error::UnsafeDirectory(
        "workspace database threads.sqlite3 is missing; preserve this directory and recover from a backup into a fresh directory".into(),
    )
}
pub(crate) fn validate_database(path: &Path) -> Result<()> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    validate_connection(&conn)
}
pub(crate) fn validate_connection(conn: &Connection) -> Result<()> {
    validate_schema(conn, &[SCHEMA_VERSION])
}
pub(crate) fn validate_schema(conn: &Connection, versions: &[i64]) -> Result<()> {
    let app: i64 = conn.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let schema: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if app != APPLICATION_ID || !versions.contains(&schema) {
        return Err(Error::UnsafeDirectory(
            "database application/schema identity mismatch".into(),
        ));
    }
    let integrity: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(Error::Invalid(integrity));
    }
    Ok(())
}
pub(crate) fn record_on(conn: &Connection, id: &str) -> Result<Option<Record>> {
    let body: Option<String> = conn
        .query_row("SELECT body FROM records WHERE id=?1", [id], |r| r.get(0))
        .optional()?;
    body.map(|body| Ok(serde_json::from_str(&body)?))
        .transpose()
}
pub(crate) fn receipt_on(conn: &Connection, operation: &OperationId) -> Result<Option<Receipt>> {
    let body: Option<String> = conn
        .query_row(
            "SELECT body FROM receipts WHERE operation=?1",
            [&operation.0],
            |r| r.get(0),
        )
        .optional()?;
    body.map(|body| Ok(serde_json::from_str(&body)?))
        .transpose()
}
fn prepared_on(conn: &Connection, operation: &OperationId) -> Result<Prepared> {
    let (hash, body): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT hash,request FROM operations WHERE id=?1",
            [&operation.0],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Error::Missing(operation.0.clone()))?;
    Ok(Prepared {
        operation: operation.clone(),
        request_hash: hash.ok_or_else(|| Error::Invalid("operation not prepared".into()))?,
        request: serde_json::from_str(
            &body.ok_or_else(|| Error::Invalid("operation not prepared".into()))?,
        )?,
    })
}
fn validate_request(operation: &OperationId, request: &ChangeRequest) -> Result<()> {
    if request.reason.trim().is_empty() || request.writes.is_empty() {
        return Err(Error::Invalid("reason and writes required".into()));
    }
    let mut ids = BTreeSet::new();
    for write in &request.writes {
        if !ids.insert(&write.id) {
            return Err(Error::Invalid("duplicate write target".into()));
        }
        if write.expected_version.is_none()
            && !write
                .id
                .strip_prefix(&format!("{}:", operation.0))
                .is_some_and(|index| index.parse::<usize>().is_ok())
        {
            return Err(Error::Invalid(
                "creation ID must be preassigned by this operation".into(),
            ));
        }
        if let RecordData::Note(note) = &write.data
            && note.import.as_ref().is_some_and(|import| import.full_note)
            && write.expected_version.is_none()
            && !note.protected
        {
            return Err(Error::Invalid("full imports begin protected".into()));
        }
        if let RecordData::Action(action) = &write.data
            && action.state == ActionState::Done
            && action
                .evidence
                .as_deref()
                .is_none_or(|text| text.trim().is_empty())
        {
            return Err(Error::Invalid("Done requires completion evidence".into()));
        }
    }
    Ok(())
}
pub(crate) fn apply_on(
    tx: &Transaction<'_>,
    operation: &OperationId,
    authority: &HostAuthority,
    saving_session: Option<&str>,
) -> Result<ApplyOutcome> {
    // Replaying a receipt precedes bases, guards, and current policy checks.
    if let crate::types::Scope::Grant {
        operation: bound, ..
    } = &authority.scope
        && bound != operation
    {
        return Ok(ApplyOutcome::NeedsReview {
            denied: vec![operation.0.clone()],
        });
    }
    if let Some(receipt) = receipt_on(tx, operation)? {
        if let crate::types::Scope::Runtime { run } = &authority.scope {
            let denied = receipt
                .writes
                .iter()
                .filter(|write| {
                    write.after.id != *run || !matches!(write.after.data, RecordData::Run(_))
                })
                .map(|write| write.after.id.clone())
                .collect::<Vec<_>>();
            if !denied.is_empty() {
                return Ok(ApplyOutcome::NeedsReview { denied });
            }
        }
        return Ok(ApplyOutcome::Applied(receipt));
    }
    if let Some((id, version)) = &authority.settings_binding
        && !matches!(record_on(tx, id)?, Some(Record { version: current, archived: false, data: RecordData::Settings(_), .. }) if current == *version)
    {
        return Err(Error::Invalid(
            "workspace configuration snapshot changed".into(),
        ));
    }
    if let Some((id, fence)) = &authority.run_binding {
        let valid = match record_on(tx, id)? {
            Some(Record {
                archived: false,
                data: RecordData::Run(run),
                ..
            }) if run.state == RunState::Working
                && run.fence == *fence
                && workspace_settings_on(tx)?
                    .as_ref()
                    .is_none_or(|settings| run_matches_settings(&run, settings)) =>
            {
                matches!(
                    record_on(tx, &run.thread)?,
                    Some(Record {
                        archived: false,
                        data: RecordData::Thread(Thread {
                            state: ThreadState::Open,
                            ..
                        }),
                        ..
                    })
                )
            }
            _ => false,
        };
        if !valid {
            return Ok(ApplyOutcome::Superseded { run: id.clone() });
        }
    }
    let retired: bool = tx.query_row(
        "SELECT retired FROM operations WHERE id=?1",
        [&operation.0],
        |r| r.get(0),
    )?;
    if retired {
        return Err(Error::Invalid("candidate retired".into()));
    }
    let prepared = prepared_on(tx, operation)?;
    let context_rows = if authority.chat_binding.is_some() {
        let mut query = tx.prepare("SELECT body FROM records ORDER BY rowid")?;
        query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|r| Ok(serde_json::from_str::<Record>(&r?)?))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };
    let mut projected_scope = context_rows.clone();
    if authority.chat_binding.is_some() {
        for w in &prepared.request.writes {
            projected_scope.retain(|r| r.id != w.id);
            projected_scope.push(Record {
                id: w.id.clone(),
                version: w.expected_version.unwrap_or(0),
                archived: w.archived,
                data: w.data.clone(),
            });
        }
    }
    if let Some((chat, _)) = &authority.chat_binding {
        if let Some((run, _)) = &authority.run_binding
            && !matches!(record_on(tx, run)?, Some(Record { data: RecordData::Run(r), .. }) if r.thread == *chat)
        {
            return Err(Error::Invalid("run and chat context disagree".into()));
        }
        let mut denied_inputs = vec![];
        for input in &prepared.request.inputs {
            let current = context_rows.iter().find(|r| r.id == input.record);
            let body: Option<String> = tx
                .query_row(
                    "SELECT body FROM revisions WHERE id=?1 AND version=?2",
                    params![input.record, crate::integer(input.version)?],
                    |r| r.get(0),
                )
                .optional()?;
            let historical = body
                .map(|body| serde_json::from_str::<Record>(&body))
                .transpose()?;
            if current.is_some_and(|r| !crate::context::visible(r, &context_rows, Some(chat)))
                || historical
                    .as_ref()
                    .is_some_and(|r| !crate::context::visible(r, &context_rows, Some(chat)))
            {
                denied_inputs.push(input.record.clone());
            }
        }
        if !denied_inputs.is_empty() {
            return Ok(ApplyOutcome::NeedsReview {
                denied: denied_inputs,
            });
        }
    }
    if !matches!(authority.scope, crate::types::Scope::Owner)
        && !writing_markers_on(tx, &prepared.request)?.is_empty()
    {
        return Ok(ApplyOutcome::NeedsReview {
            denied: prepared
                .request
                .writes
                .iter()
                .map(|write| write.id.clone())
                .collect(),
        });
    }
    let undo_of: Option<String> = tx.query_row(
        "SELECT undo_of FROM operations WHERE id=?1",
        [&operation.0],
        |r| r.get(0),
    )?;
    let standing_enabled = workspace_settings_on(tx)?.is_some_and(|settings| {
        settings.delegation_chosen && settings.maintenance && !settings.review_first
    });
    if !standing_enabled && matches!(authority.scope, crate::types::Scope::Maintenance) {
        let denied = prepared
            .request
            .writes
            .iter()
            .filter(|write| !matches!(write.data, RecordData::Run(_)))
            .map(|write| write.id.clone())
            .collect::<Vec<_>>();
        if !denied.is_empty() {
            return Ok(ApplyOutcome::NeedsReview { denied });
        }
    }
    let strict_instruction =
        !standing_enabled && matches!(authority.scope, crate::types::Scope::Grant { .. });
    let mut asset_owners = BTreeSet::new();
    for write in &prepared.request.writes {
        if let RecordData::Asset(asset) = &write.data {
            if let Some(Record {
                data: RecordData::Asset(old),
                ..
            }) = record_on(tx, &write.id)?
                && old.note != asset.note
            {
                return Err(Error::Invalid("asset ownership is immutable".into()));
            }
            if !prepared
                .request
                .writes
                .iter()
                .any(|owner| owner.id == asset.note && matches!(owner.data, RecordData::Note(_)))
            {
                return Err(Error::Invalid(
                    "asset mutations require an owning Note write in the same group".into(),
                ));
            }
            asset_owners.insert(asset.note.clone());
        }
    }
    let mut stale = vec![];
    let mut guarded = vec![];
    let mut denied = vec![];
    let mut before = vec![];
    for write in &prepared.request.writes {
        let current = record_on(tx, &write.id)?;
        if current.as_ref().map(|r| r.version) != write.expected_version {
            stale.push(write.id.clone());
        }
        let guard: Option<String> = tx
            .query_row(
                "SELECT id FROM edit_sessions WHERE note=?1 AND closed=0",
                [&write.id],
                |r| r.get(0),
            )
            .optional()?;
        if guard
            .as_deref()
            .is_some_and(|guard| Some(guard) != saving_session)
        {
            guarded.push(write.id.clone());
        }
        if let Some(old) = &current
            && std::mem::discriminant(&old.data) != std::mem::discriminant(&write.data)
        {
            return Err(Error::Invalid("record kind is immutable".into()));
        }
        let mut required = BTreeSet::new();
        if matches!(write.data, RecordData::ChatControl(_)) {
            required.insert(Capability::Configure);
        }
        let before_context = current.as_ref().and_then(|r| match &r.data {
            RecordData::Note(n) => n.chat.as_deref(),
            RecordData::Source(s) => s.chat.as_deref(),
            _ => None,
        });
        let after_context = match &write.data {
            RecordData::Note(n) => n.chat.as_deref(),
            RecordData::Source(s) => s.chat.as_deref(),
            _ => None,
        };
        if before_context != after_context
            && (current.is_some()
                || after_context.is_some_and(|chat| {
                    authority
                        .chat_binding
                        .as_ref()
                        .is_none_or(|(bound, _)| bound != chat)
                }))
        {
            required.insert(Capability::Configure);
        }
        if let Some(chat) = after_context
            && !matches!(
                record_on(tx, chat)?,
                Some(Record {
                    data: RecordData::Thread(_),
                    ..
                })
            )
        {
            return Err(Error::Invalid(
                "chat context must name an existing conversation".into(),
            ));
        }
        if let Some((chat, private)) = &authority.chat_binding {
            if current
                .as_ref()
                .is_some_and(|r| !crate::context::visible(r, &context_rows, Some(chat)))
            {
                denied.push(write.id.clone());
            }
            if !crate::context::visible(
                &Record {
                    id: write.id.clone(),
                    version: write.expected_version.unwrap_or(0),
                    archived: write.archived,
                    data: write.data.clone(),
                },
                &projected_scope,
                Some(chat),
            ) {
                denied.push(write.id.clone());
            }
            let text = match &write.data {
                RecordData::Note(n) => Some(n.markdown.as_str()),
                RecordData::Message(m) => Some(m.text.as_str()),
                RecordData::Comment(c) => Some(c.body.as_str()),
                _ => None,
            };
            if text.is_some_and(|text| {
                crate::managed_asset_ids(text).any(|id| {
                    !projected_scope
                        .iter()
                        .find(|r| r.id == id)
                        .is_some_and(|r| {
                            matches!(r.data, RecordData::Asset(_))
                                && crate::context::visible(r, &projected_scope, Some(chat))
                        })
                })
            }) {
                denied.push(write.id.clone());
            }
            let private = *private
                || context_rows.iter().any(
                    |r| matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(chat)),
                );
            if private && matches!(authority.scope, crate::types::Scope::Maintenance) {
                let local = match &write.data {
                    RecordData::Note(n) => n.chat.as_deref() == Some(chat),
                    RecordData::Source(s) => s.chat.as_deref() == Some(chat),
                    RecordData::Thread(_) => write.id == *chat,
                    RecordData::Run(r) => r.thread == *chat,
                    RecordData::Asset(a) => projected_scope.iter().any(|r| r.id == a.note && matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(chat))),
                    RecordData::Comment(c) => projected_scope.iter().any(|r| r.id == c.note && matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(chat))),
                    RecordData::Link(l) => {
                        let local_end = |id: &str| id == chat || projected_scope.iter().any(|r| r.id == id && matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(chat)));
                        (local_end(&l.from) || local_end(&l.to)) && projected_scope.iter().find(|r| r.id == l.from).is_some_and(|r| crate::context::visible(r, &projected_scope, Some(chat))) && projected_scope.iter().find(|r| r.id == l.to).is_some_and(|r| crate::context::visible(r, &projected_scope, Some(chat)))
                    },
                    _ => false,
                };
                if !local {
                    denied.push(write.id.clone());
                }
            }
        }
        if let crate::types::Scope::Runtime { run } = &authority.scope
            && (run != &write.id || !matches!(write.data, RecordData::Run(_)))
        {
            denied.push(write.id.clone());
            before.push(current);
            continue;
        }
        if matches!(write.data, RecordData::Settings(_)) {
            required.insert(Capability::Configure);
        }
        if matches!(write.data, RecordData::Run(_)) {
            required.insert(Capability::ManageRun);
        }
        if let (Some(old), RecordData::Note(next)) = (&current, &write.data) {
            if let RecordData::Note(prior) = &old.data {
                if prior.protected || strict_instruction {
                    if asset_owners.contains(&write.id)
                        || prior.title != next.title
                        || prior.markdown != next.markdown
                        || prior.import != next.import
                    {
                        required.insert(Capability::EditContent);
                    }
                    if prior.superseded_by != next.superseded_by {
                        required.insert(Capability::Supersede);
                    }
                    if old.archived != write.archived {
                        required.insert(Capability::Archive);
                    }
                    if prior.protected && !next.protected {
                        required.insert(Capability::Unprotect);
                    }
                }
                if !prior.protected && next.protected {
                    required.insert(Capability::Protect);
                }
                let changed_content = prior.title != next.title
                    || prior.markdown != next.markdown
                    || prior.import != next.import
                    || asset_owners.contains(&write.id);
                if prior.confirmed != next.confirmed && (next.confirmed || !changed_content) {
                    required.insert(Capability::Confirm);
                }
                if undo_of.is_some() && (prior.protected || strict_instruction) {
                    required.insert(Capability::Undo);
                }
            }
        } else if let RecordData::Note(next) = &write.data
            && next.confirmed
        {
            required.insert(Capability::Confirm);
        }
        if let RecordData::Action(next) = &write.data {
            let prior = current.as_ref().and_then(|record| {
                if let RecordData::Action(action) = &record.data {
                    Some(action)
                } else {
                    None
                }
            });
            let commitment_changed = prior.is_some_and(|prior| {
                prior.state != ActionState::Suggested
                    && (prior.due != next.due
                        || prior.description != next.description
                        || prior.state != next.state
                        || current
                            .as_ref()
                            .is_some_and(|record| record.archived != write.archived))
            });
            let new_commitment = prior.is_none_or(|prior| prior.state == ActionState::Suggested)
                && next.state != ActionState::Suggested;
            let routine_internal_completion = next.internal
                && next.state == ActionState::Done
                && prior.is_some_and(|prior| {
                    prior.internal
                        && prior.due == next.due
                        && prior.description == next.description
                        && current
                            .as_ref()
                            .is_some_and(|record| record.archived == write.archived)
                });
            // Waiting is routine progress, while recording/changing an agreed
            // commitment or cancelling it requires an explicit host instruction.
            let routine_wait = prior.is_some_and(|prior| {
                matches!(prior.state, ActionState::Open | ActionState::Waiting)
                    && matches!(next.state, ActionState::Open | ActionState::Waiting)
                    && prior.due == next.due
                    && prior.description == next.description
                    && prior.internal == next.internal
                    && current
                        .as_ref()
                        .is_some_and(|record| record.archived == write.archived)
            });
            if (new_commitment || commitment_changed)
                && !routine_internal_completion
                && !routine_wait
            {
                required.insert(Capability::Commitment);
            }
            if next.state == ActionState::Done
                && !next.internal
                && prior.is_none_or(|prior| {
                    prior.state != ActionState::Done || prior.evidence != next.evidence
                })
            {
                required.insert(Capability::CompleteHuman);
            }
            if prior.is_some_and(|prior| !prior.internal) && next.internal {
                required.insert(Capability::Commitment);
            }
        }
        if strict_instruction {
            if current
                .as_ref()
                .is_some_and(|old| old.archived != write.archived)
            {
                required.insert(Capability::Archive);
            }
            // Without standing delegation, otherwise ordinary creation/content
            // changes need an applicable action grant as well as a named target.
            let content_changed = current.as_ref().is_none_or(|old| old.data != write.data)
                || asset_owners.contains(&write.id);
            match &write.data {
                RecordData::Note(_) => {
                    if current.is_none() {
                        required.insert(Capability::EditContent);
                    }
                }
                RecordData::Action(_) => {
                    if content_changed
                        && !required.contains(&Capability::Commitment)
                        && !required.contains(&Capability::CompleteHuman)
                    {
                        required.insert(Capability::EditContent);
                    }
                }
                RecordData::Run(_) | RecordData::Settings(_) => {}
                _ => {
                    if content_changed {
                        required.insert(Capability::EditContent);
                    }
                }
            }
        }
        if let crate::types::Scope::Grant { targets, .. } = &authority.scope {
            // Instruction scope is a concrete target set. An unrelated linked
            // record cannot silently inherit that owner's instruction.
            if !targets.contains(&write.id)
                && (strict_instruction
                    || required
                        .iter()
                        .any(|capability| *capability != Capability::Confirm))
            {
                denied.push(write.id.clone());
                before.push(current);
                continue;
            }
        }
        if required
            .iter()
            .any(|capability| !authority.allows(operation, &write.id, *capability))
        {
            denied.push(write.id.clone());
        }
        before.push(current);
    }
    if !denied.is_empty() {
        return Ok(ApplyOutcome::NeedsReview { denied });
    }
    if !guarded.is_empty() {
        return Ok(ApplyOutcome::Deferred { guarded });
    }
    // Check all reads against the transaction's prestate, including self-inputs.
    // Receipt replay above precedes these mutable checks; no grouped write has
    // happened yet. Historical provenance is validated separately below.
    for input in &prepared.request.inputs {
        if input.expectation == InputExpectation::Current
            && !current_input_matches(tx, input)?
            && !stale.contains(&input.record)
        {
            stale.push(input.record.clone());
        }
    }
    if !stale.is_empty() {
        return Ok(ApplyOutcome::Stale { records: stale });
    }
    for write in &prepared.request.writes {
        if let RecordData::Run(next) = &write.data
            && let Some(prior) = record_on(tx, &write.id)?
        {
            let RecordData::Run(old) = &prior.data else {
                return Err(Error::Invalid("record kind is immutable".into()));
            };
            if old.instruction != next.instruction || old.queue_claim != next.queue_claim {
                return Err(Error::Invalid(
                    "Run instruction identity is immutable; start fresh".into(),
                ));
            }
            if next.fence < old.fence {
                return Err(Error::Invalid("run fence cannot decrease".into()));
            }
            if old.state != RunState::Working && next.state == RunState::Working {
                return Err(Error::Invalid(
                    "terminal Run cannot reactivate; create a fresh Run".into(),
                ));
            }
            if run_control_changed(old, next, prior.archived, write.archived)
                && next.fence <= old.fence
            {
                return Err(Error::Invalid(
                    "run control changes must advance the fence".into(),
                ));
            }
        }
    }
    for write in &prepared.request.writes {
        if let RecordData::Comment(comment) = &write.data {
            let base: Option<String> = tx
                .query_row(
                    "SELECT body FROM revisions WHERE id=?1 AND version=?2",
                    params![comment.note, crate::integer(comment.base_version)?],
                    |r| r.get(0),
                )
                .optional()?;
            let base: Record = serde_json::from_str(
                &base.ok_or_else(|| Error::Invalid("comment base revision missing".into()))?,
            )?;
            let RecordData::Note(note) = base.data else {
                return Err(Error::Invalid("comment target must be note".into()));
            };
            if let Some((start, end)) = comment.range
                && note.markdown.get(start..end) != Some(comment.quote.as_str())
            {
                return Err(Error::Invalid(
                    "comment UTF-8 range does not match quote".into(),
                ));
            }
        }
    }
    for input in &prepared.request.inputs {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM revisions WHERE id=?1 AND version=?2)",
            params![input.record, crate::integer(input.version)?],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(Error::Invalid("derivation input revision missing".into()));
        }
    }
    let selected = prepared
        .request
        .writes
        .iter()
        .find_map(|write| {
            if !write.archived
                && let RecordData::Settings(settings) = &write.data
            {
                Some(settings.clone())
            } else {
                None
            }
        })
        .or(workspace_settings_on(tx)?);
    if matches!(authority.scope, crate::types::Scope::Runtime { .. })
        && authority.settings_binding.is_none()
        && selected.is_some()
        && prepared.request.writes.iter().any(|write| {
            write.expected_version.is_none()
                && matches!(&write.data, RecordData::Run(run) if run.state == RunState::Working)
        })
    {
        return Err(Error::Invalid(
            "runtime start requires captured workspace configuration".into(),
        ));
    }
    for write in &prepared.request.writes {
        if !write.archived
            && let RecordData::Run(run) = &write.data
            && run.state == RunState::Working
            && let Some(settings) = &selected
            && !run_matches_settings(run, settings)
        {
            return Err(Error::Invalid(
                "Working Run selection must match current workspace configuration".into(),
            ));
        }
    }
    let mut run_rows = std::collections::BTreeMap::new();
    let mut stmt =
        tx.prepare("SELECT body FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL")?;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        let record: Record = serde_json::from_str(&row?)?;
        run_rows.insert(record.id.clone(), record);
    }
    for write in &prepared.request.writes {
        if matches!(write.data, RecordData::Run(_)) {
            run_rows.insert(
                write.id.clone(),
                Record {
                    id: write.id.clone(),
                    version: 0,
                    archived: write.archived,
                    data: write.data.clone(),
                },
            );
        }
    }
    let control = prepared
        .request
        .writes
        .iter()
        .find_map(|w| match &w.data {
            RecordData::ChatControl(c) => Some(c.clone()),
            _ => None,
        })
        .or(crate::chat::control_on(tx)?.and_then(|r| match r.data {
            RecordData::ChatControl(c) => Some(c),
            _ => None,
        }));
    for w in &prepared.request.writes {
        if w.expected_version.is_none()
            && !w.archived
            && let RecordData::Run(run) = &w.data
            && run.state == RunState::Working
        {
            if let Some(claim) = &run.queue_claim {
                if !control.as_ref().is_some_and(|c| {
                    !c.paused
                        && c.pending.as_ref().is_some_and(|q| {
                            q.run.as_ref() == Some(&w.id)
                                && q.dispatch.as_ref() == Some(claim)
                                && q.message == run.instruction
                                && q.thread == run.thread
                        })
                }) {
                    return Err(Error::Invalid(
                        "queued start requires its current checked instruction claim".into(),
                    ));
                }
                if run_rows.values().any(|r| {
                    r.id != w.id
                        && !r.archived
                        && matches!(&r.data,RecordData::Run(r) if r.state==RunState::Working)
                }) {
                    return Err(Error::Invalid(
                        "queued work waits for all current runs".into(),
                    ));
                }
            } else if control
                .as_ref()
                .is_some_and(|c| c.pending.as_ref().is_some_and(|q| q.dispatch.is_some()))
            {
                return Err(Error::Invalid(
                    "queued instruction is claimed; use the saved queue controls".into(),
                ));
            }
        }
    }
    let mut working_threads = BTreeSet::new();
    for record in run_rows.values() {
        if !record.archived
            && let RecordData::Run(run) = &record.data
            && run.state == RunState::Working
            && !working_threads.insert(&run.thread)
        {
            return Err(Error::Invalid(
                "a Thread can have only one Working Run".into(),
            ));
        }
    }
    for write in &prepared.request.writes {
        let refs: Vec<(&str, Option<&str>)> = match &write.data {
            RecordData::Asset(asset) => std::iter::once((asset.note.as_str(), Some("note")))
                .chain(asset.source.as_deref().map(|id| (id, Some("source"))))
                .collect(),
            RecordData::Link(link) => vec![(&link.from, None), (&link.to, None)],
            RecordData::Comment(comment) => vec![(&comment.note, Some("note"))],
            RecordData::Message(message) => vec![(&message.thread, Some("thread"))],
            RecordData::Run(run) => std::iter::once((run.thread.as_str(), Some("thread")))
                .chain(run.sources.iter().map(|id| (id.as_str(), Some("source"))))
                .collect(),
            RecordData::Note(note) => note
                .import
                .as_ref()
                .map(|import| (import.source.as_str(), Some("source")))
                .into_iter()
                .chain(note.superseded_by.as_deref().map(|id| (id, Some("note"))))
                .collect(),
            RecordData::Thread(thread) => thread
                .attention
                .iter()
                .filter_map(|a| a.record.as_deref().map(|id| (id, None)))
                .collect(),
            _ => vec![],
        };
        for (reference, kind) in refs {
            let current = record_on(tx, reference)?;
            let data = prepared
                .request
                .writes
                .iter()
                .find(|planned| planned.id == reference)
                .map(|planned| &planned.data)
                .or(current.as_ref().map(|r| &r.data));
            if data.is_none() && matches!(write.data, RecordData::Thread(_)) {
                let known: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1)",
                    [reference],
                    |r| r.get(0),
                )?;
                if known {
                    continue;
                }
            }
            let data =
                data.ok_or_else(|| Error::Invalid(format!("missing reference {reference}")))?;
            let actual = match data {
                RecordData::Note(_) => "note",
                RecordData::Thread(_) => "thread",
                RecordData::Source(_) => "source",
                _ => "other",
            };
            if kind.is_some_and(|expected| expected != actual) {
                return Err(Error::Invalid(format!(
                    "reference {reference} requires {}",
                    kind.unwrap()
                )));
            }
        }
        if let RecordData::Asset(asset) = &write.data
            && (asset.bytes.is_empty() || asset.media_type.trim().is_empty())
        {
            return Err(Error::Invalid("asset bytes and media type required".into()));
        }
    }
    let mut written = vec![];
    for (write, prior) in prepared.request.writes.iter().zip(before) {
        let version = prior.as_ref().map_or(1, |r| r.version + 1);
        let mut data = write.data.clone();
        if let RecordData::Note(next) = &mut data {
            let content_changed = prior.as_ref().is_some_and(|record| match &record.data {
                RecordData::Note(old) => {
                    old.title != next.title
                        || old.markdown != next.markdown
                        || old.import != next.import
                        || asset_owners.contains(&write.id)
                }
                _ => false,
            });
            if content_changed && !authority.allows(operation, &write.id, Capability::Confirm) {
                next.confirmed = false;
            }
        }
        let after = Record {
            id: write.id.clone(),
            version,
            archived: write.archived,
            data,
        };
        let body = serde_json::to_string(&after)?;
        tx.execute("INSERT INTO records(id,version,body) VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET version=excluded.version,body=excluded.body",params![after.id,crate::integer(version)?,body])?;
        tx.execute(
            "INSERT INTO revisions(id,version,operation,body) VALUES (?1,?2,?3,?4)",
            params![after.id, crate::integer(version)?, operation.0, body],
        )?;
        written.push(Written {
            before: prior,
            after,
        });
    }
    for input in &prepared.request.inputs {
        tx.execute(
            "INSERT INTO derivation_inputs(operation,record,version) VALUES (?1,?2,?3)",
            params![operation.0, input.record, crate::integer(input.version)?],
        )?;
    }
    let mut refresh = BTreeSet::new();
    for input_change in &written {
        let Some(prior) = &input_change.before else {
            continue;
        };
        if !meaningful_input_change(prior, &input_change.after) {
            continue;
        }
        // Only current directly derived revisions are affected. An observation-only
        // version may intervene, so compare every recorded older input version.
        // Targets recomputed in this same atomic group are already refreshed; a
        // target's edit base is not a dependency on itself.
        let mut stmt = tx.prepare("SELECT DISTINCT c.body FROM derivation_inputs d JOIN revisions r ON r.operation=d.operation JOIN records c ON c.id=r.id WHERE d.record=?1 AND d.version<?2 AND json_extract(c.body,'$.archived')=0 AND json_extract(c.body,'$.data.Note.superseded_by') IS NULL")?;
        for id in stmt.query_map(
            params![prior.id, crate::integer(input_change.after.version)?],
            |r| r.get::<_, String>(0),
        )? {
            let current: Record = serde_json::from_str(&id?)?;
            if written.iter().any(|write| write.after.id == current.id) {
                continue;
            }
            let Some(derived_operation) = derivation_operation_on(tx, &current)? else {
                continue;
            };
            let uses_older_input = tx.query_row("SELECT EXISTS(SELECT 1 FROM derivation_inputs WHERE operation=?1 AND record=?2 AND version<?3)", params![derived_operation.as_str(), prior.id, crate::integer(input_change.after.version)?], |row| row.get::<_, bool>(0))?;
            if uses_older_input {
                refresh.insert(current.id);
            }
        }
    }
    let committed_at = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
        r.get(0)
    })?;
    let receipt = Receipt {
        operation: operation.clone(),
        request_hash: prepared.request_hash,
        reason: prepared.request.reason,
        writes: written,
        authority: authority.audit(),
        committed_at,
        undo_of: undo_of.map(OperationId),
        needs_refresh: refresh.into_iter().collect(),
    };
    tx.execute(
        "DELETE FROM candidate_targets WHERE operation=?1",
        [&operation.0],
    )?;
    tx.execute(
        "INSERT INTO receipts(operation,body) VALUES (?1,?2)",
        params![operation.0, serde_json::to_string(&receipt)?],
    )?;
    Ok(ApplyOutcome::Applied(receipt))
}

fn current_input_matches(conn: &Connection, input: &RevisionInput) -> Result<bool> {
    Ok(record_on(conn, &input.record)?.is_some_and(|record| {
        record.version == input.version
            && !record.archived
            && !matches!(record.data, RecordData::Note(note) if note.superseded_by.is_some())
    }))
}

/// Version changes for editor guards, comment mapping and observation timestamps
/// alone do not imply new substantive evidence. No semantic dependency inference.
fn meaningful_input_change(before: &Record, after: &Record) -> bool {
    if before.archived != after.archived {
        return true;
    }
    match (&before.data, &after.data) {
        (RecordData::Note(old), RecordData::Note(new)) => {
            let mut old = old.clone();
            old.protected = new.protected;
            old.confirmed = new.confirmed;
            old != *new
        }
        (RecordData::Source(old), RecordData::Source(new)) => {
            let mut old = old.clone();
            old.observed_at = new.observed_at.clone();
            old.label = new.label.clone();
            old != *new
        }
        (RecordData::Action(old), RecordData::Action(new)) => old != new,
        (RecordData::Asset(old), RecordData::Asset(new)) => old != new,
        _ => false,
    }
}

fn derivation_is_current(derived: &Record, current: &Record) -> bool {
    match (&derived.data, &current.data) {
        // Progress/completion changes do not erase where the task itself came
        // from. A later input may warrant a follow-up even for completed work.
        (RecordData::Action(old), RecordData::Action(new)) => {
            old.description == new.description && old.due == new.due && old.internal == new.internal
        }
        (RecordData::Note(_), RecordData::Note(_))
        | (RecordData::Source(_), RecordData::Source(_))
        | (RecordData::Asset(_), RecordData::Asset(_)) => {
            !meaningful_input_change(derived, current)
        }
        _ => derived.version == current.version,
    }
}

/// Recover the exact direct-input footing through existing durable receipts.
/// Metadata/progress inherits its prior footing; Undo restores the original
/// write's before revision, including when two derived revisions had identical
/// text but different sources. No new dependency graph or schema is needed.
fn derivation_operation_on(conn: &Connection, current: &Record) -> Result<Option<OperationId>> {
    let mut revision = current.clone();
    loop {
        let operation: String = conn.query_row(
            "SELECT operation FROM revisions WHERE id=?1 AND version=?2",
            params![revision.id, crate::integer(revision.version)?],
            |row| row.get(0),
        )?;
        let operation = OperationId(operation);
        let receipt = receipt_on(conn, &operation)?
            .ok_or_else(|| Error::Missing(operation.as_str().into()))?;
        if let Some(original) = receipt.undo_of {
            let original = receipt_on(conn, &original)?
                .ok_or_else(|| Error::Missing(original.as_str().into()))?;
            let Some(prior) = original
                .writes
                .iter()
                .find(|write| write.after.id == revision.id)
                .and_then(|write| write.before.as_ref())
            else {
                return Ok(None);
            };
            revision = prior.clone();
            continue;
        }
        if conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM derivation_inputs WHERE operation=?1)",
            [operation.as_str()],
            |row| row.get::<_, bool>(0),
        )? {
            return Ok(Some(operation));
        }
        let Some(prior) = receipt
            .writes
            .iter()
            .find(|write| write.after.id == revision.id)
            .and_then(|write| write.before.as_ref())
        else {
            return Ok(None);
        };
        if !derivation_is_current(prior, &revision) {
            return Ok(None);
        }
        revision = prior.clone();
    }
}

/// Creates (if needed) and returns `directory`'s real `derived/` directory.
pub(crate) fn ensure_derived(directory: &Path) -> Result<PathBuf> {
    let path = directory.join(DERIVED_DIRECTORY);
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let kind = fs::symlink_metadata(&path)?.file_type();
    if kind.is_symlink() || !kind.is_dir() || has_symlink_ancestor(&path)? {
        return Err(Error::UnsafeDirectory(
            "derived data must be a real directory inside the workspace".into(),
        ));
    }
    Ok(path)
}

pub(crate) fn has_symlink_ancestor(path: &Path) -> Result<bool> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}

fn run_control_changed(old: &Run, next: &Run, old_archived: bool, next_archived: bool) -> bool {
    old.state != next.state
        || old.thread != next.thread
        || old.provider != next.provider
        || old.model != next.model
        || old.guide_identity != next.guide_identity
        || old.budget != next.budget
        || old.effort != next.effort
        || old.loaded_skills != next.loaded_skills
        || old_archived != next_archived
}
fn run_control_written(written: &Written) -> bool {
    if let RecordData::ChatControl(after) = &written.after.data {
        return match written.before.as_ref().map(|r| &r.data) {
            Some(RecordData::ChatControl(before)) => {
                before.queue != after.queue
                    || before.pending != after.pending
                    || before.paused != after.paused
            }
            _ => true,
        };
    }

    if let RecordData::Run(after) = &written.after.data {
        match written.before.as_ref() {
            Some(Record {
                data: RecordData::Run(before),
                archived,
                ..
            }) => {
                before.fence != after.fence
                    || run_control_changed(before, after, *archived, written.after.archived)
            }
            _ => true,
        }
    } else {
        false
    }
}

fn refresh_scope_on(
    tx: &Transaction<'_>,
    original: &OperationId,
    replacement: &OperationId,
    authority: &HostAuthority,
) -> Result<String> {
    let crate::types::Scope::Runtime { run: scoped } = &authority.scope else {
        return Err(Error::Invalid("runtime refresh scope required".into()));
    };
    let Some((run_id, fence)) = &authority.run_binding else {
        return Err(Error::Invalid(
            "runtime refresh requires a run fence".into(),
        ));
    };
    if scoped != run_id {
        return Err(Error::Invalid("runtime refresh scope mismatch".into()));
    }
    let record = record_on(tx, run_id)?.ok_or_else(|| Error::Missing(run_id.clone()))?;
    let RecordData::Run(run) = record.data else {
        return Err(Error::Invalid("not a run".into()));
    };
    if record.archived
        || run.state != RunState::Working
        || run.fence != *fence
        || !run.operations.contains(replacement)
        || !matches!(
            record_on(tx, &run.thread)?,
            Some(Record {
                archived: false,
                data: RecordData::Thread(Thread {
                    state: ThreadState::Open,
                    ..
                }),
                ..
            })
        )
        || workspace_settings_on(tx)?
            .as_ref()
            .is_some_and(|settings| !run_matches_settings(&run, settings))
    {
        return Err(Error::Invalid("runtime refresh is superseded".into()));
    }
    if let Some((id, version)) = &authority.settings_binding
        && !matches!(record_on(tx, id)?, Some(Record { version: current, archived: false, data: RecordData::Settings(_), .. }) if current == *version)
    {
        return Err(Error::Invalid(
            "runtime refresh configuration changed".into(),
        ));
    }
    let mut statement =
        tx.prepare("SELECT body FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL")?;
    let mut belongs = false;
    for body in statement.query_map([], |row| row.get::<_, String>(0))? {
        let record: Record = serde_json::from_str(&body?)?;
        if !record.archived
            && matches!(record.data, RecordData::Run(previous) if previous.thread == run.thread && previous.operations.contains(original))
        {
            belongs = true;
            break;
        }
    }
    if !belongs {
        return Err(Error::Invalid("candidate belongs to another Thread".into()));
    }
    Ok(run.thread)
}

pub(crate) fn prepare_on(
    tx: &Transaction<'_>,
    operation: &OperationId,
    request: &ChangeRequest,
    owner_direct: bool,
) -> Result<Prepared> {
    validate_request(operation, request)?;
    let body = serde_json::to_string(request)?;
    let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
    let prior: Option<Option<String>> = tx
        .query_row(
            "SELECT hash FROM operations WHERE id=?1",
            [&operation.0],
            |r| r.get(0),
        )
        .optional()?;
    match prior {
        None => return Err(Error::Missing(operation.0.clone())),
        Some(Some(prior)) if prior != hash => return Err(Error::ImmutableRequest),
        _ => {}
    }
    let retired: bool = tx.query_row(
        "SELECT retired FROM operations WHERE id=?1",
        [&operation.0],
        |r| r.get(0),
    )?;
    if retired {
        return Err(Error::Invalid("candidate retired".into()));
    }
    if !owner_direct && receipt_on(tx, operation)?.is_none() {
        for write in &request.writes {
            if matches!(write.data, RecordData::Note(_)) {
                let active: Option<String> = tx
                    .query_row(
                        "SELECT operation FROM candidate_targets WHERE note=?1",
                        [&write.id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if active
                    .as_deref()
                    .is_some_and(|active| active != operation.0)
                {
                    return Err(Error::Invalid(format!(
                        "note {} already has an active candidate",
                        write.id
                    )));
                }
                tx.execute(
                    "INSERT OR IGNORE INTO candidate_targets(note,operation) VALUES (?1,?2)",
                    params![write.id, operation.0],
                )?;
            }
        }
    }
    tx.execute(
        "UPDATE operations SET hash=?2,request=?3 WHERE id=?1 AND hash IS NULL",
        params![operation.0, hash, body],
    )?;
    Ok(Prepared {
        operation: operation.clone(),
        request_hash: hash,
        request: request.clone(),
    })
}

fn workspace_settings_on(conn: &Connection) -> Result<Option<WorkspaceSettings>> {
    let body:Option<String>=conn.query_row("SELECT body FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL AND json_extract(body,'$.archived')=0",[],|r|r.get(0)).optional()?;
    body.map(|body| match serde_json::from_str::<Record>(&body)?.data {
        RecordData::Settings(settings) => Ok(settings),
        _ => Err(Error::Invalid("workspace settings kind mismatch".into())),
    })
    .transpose()
}
fn run_matches_settings(run: &Run, settings: &WorkspaceSettings) -> bool {
    (run.provider.eq_ignore_ascii_case(&settings.provider)
        || (subscription_provider(&run.provider) && subscription_provider(&settings.provider)))
        && run.model == settings.model
        && run.effort == settings.effort
}

fn subscription_provider(provider: &str) -> bool {
    provider.eq_ignore_ascii_case("codex") || provider.eq_ignore_ascii_case("chatgpt")
}

pub(crate) fn validate_backup_directory(workspace: &Path, directory: &Path) -> Result<PathBuf> {
    if !directory.is_absolute() || directory.to_str().is_none() {
        return Err(Error::UnsafeDirectory(
            "backup folder must be an absolute UTF-8 path".into(),
        ));
    }
    let directory = fs::canonicalize(directory)?;
    if directory.to_str().is_none() || !directory.is_dir() {
        return Err(Error::UnsafeDirectory(
            "backup folder must be an existing UTF-8 directory".into(),
        ));
    }
    if directory.starts_with(fs::canonicalize(workspace)?) {
        return Err(Error::UnsafeDirectory(
            "backup folder must be outside the workspace storage directory".into(),
        ));
    }
    Ok(directory)
}

// Historical evidence keeps the review gate even if the current Message changes.
fn writing_markers_on(conn: &Connection, request: &ChangeRequest) -> Result<Vec<RevisionInput>> {
    let mut markers = Vec::new();
    for input in &request.inputs {
        if input.expectation != InputExpectation::Current {
            continue;
        }
        let body: Option<String> = conn
            .query_row(
                "SELECT body FROM revisions WHERE id=?1 AND version=?2",
                params![input.record, crate::integer(input.version)?],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(body) = body {
            let record: Record = serde_json::from_str(&body)?;
            if matches!(record.data, RecordData::Message(message) if message.role == crate::WRITING_REQUEST_ROLE)
            {
                markers.push(input.clone());
            }
        }
    }
    Ok(markers)
}

#[cfg(test)]
mod storage_open_tests {
    use super::*;
    #[test]
    fn existing_connection_path_never_creates_a_database_after_path_check() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let directory = root.path().join("data");
        drop(Store::open(&directory).unwrap());
        let database = directory.join(DATABASE_FILENAME);
        assert!(database.exists());
        // Exercise the production connection seam after deletion, bypassing the
        // friendly path precheck that cannot close a race on its own.
        fs::remove_file(&database).unwrap();
        assert!(
            matches!(open_existing_database(&database), Err(Error::UnsafeDirectory(message)) if message.contains("missing"))
        );
        assert!(!database.exists());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    }
}
