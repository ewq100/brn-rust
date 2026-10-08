//! Exact unfinished editor work and a narrow Markdown-save journal. Filesystem
//! observation, installation and reconciliation proofs belong to the workflow.
use super::{MAX_NOTE_BYTES, WorkStore};
use crate::{
    Error, Result,
    files::{FileFingerprint, PreparedFile, VaultIdentity},
    hash, invalid,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub(super) const V3: &str = "
CREATE TABLE editors (
    path TEXT PRIMARY KEY,
    record_json BLOB NOT NULL,
    record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);
CREATE TABLE editor_saves (
    operation_id TEXT PRIMARY KEY,
    path TEXT NOT NULL REFERENCES editors(path),
    original INTEGER NOT NULL CHECK(original IN (0,1)),
    outcome TEXT CHECK(outcome IN ('applied','not_applied','uncertain')),
    request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
    intent_json BLOB NOT NULL,
    intent_sha256 BLOB NOT NULL CHECK(length(intent_sha256)=32),
    installed_json BLOB,
    installed_sha256 BLOB CHECK(installed_sha256 IS NULL OR length(installed_sha256)=32),
    CHECK((installed_json IS NULL)=(installed_sha256 IS NULL))
);
CREATE UNIQUE INDEX one_unresolved_editor_save ON editor_saves(path)
    WHERE original=1 AND (outcome IS NULL OR outcome='uncertain');
CREATE TABLE editor_previous (
    path TEXT PRIMARY KEY REFERENCES editors(path),
    pair_json BLOB NOT NULL,
    pair_sha256 BLOB NOT NULL CHECK(length(pair_sha256)=32)
);
CREATE TABLE editor_completed (
    operation_id TEXT PRIMARY KEY,
    request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
    receipt_json BLOB NOT NULL,
    receipt_sha256 BLOB NOT NULL CHECK(length(receipt_sha256)=32),
    no_op INTEGER NOT NULL CHECK(no_op IN (0,1))
);";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditStamp {
    pub baseline: Uuid,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorRecord {
    pub path: String,
    pub stamp: EditStamp,
    pub baseline: FileFingerprint,
    pub baseline_text: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditRequest {
    pub path: String,
    pub expected: EditStamp,
    pub generation: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRequest {
    pub operation_id: Uuid,
    pub edit: EditRequest,
    /// An independent copy; None means the editor's original path.
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveOutcome {
    Applied,
    NotApplied,
    Uncertain,
}

impl SaveOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::NotApplied => "not_applied",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveReceipt {
    pub operation_id: Uuid,
    pub path: String,
    pub destination: Option<String>,
    pub submitted_generation: u64,
    pub stamp: EditStamp,
    pub outcome: SaveOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveIntent {
    pub request: SaveRequest,
    pub baseline: FileFingerprint,
    pub baseline_text: String,
    pub staging: PathBuf,
    #[serde(default)]
    pub parent: Option<VaultIdentity>,
    /// The workflow recorded a coordinated unchanged-save branch.
    #[serde(default)]
    pub no_op: bool,
    pub prepared: Option<PreparedFile>,
    pub receipt: Option<SaveReceipt>,
}

#[derive(Serialize, Deserialize)]
struct CompletedSave {
    receipt: SaveReceipt,
    no_op: bool,
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode editor record"))
}

fn decode<T: DeserializeOwned>(bytes: Vec<u8>, digest: Vec<u8>) -> Result<T> {
    if digest.as_slice() != hash(&bytes) {
        return Err(invalid("stored editor record failed its hash check"));
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored editor record"))
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || !Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(invalid("editor needs a contained relative Markdown path"));
    }
    Ok(())
}

fn validate_text(text: &str) -> Result<()> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(invalid("editor text exceeds the 1 MiB note limit"));
    }
    Ok(())
}

fn validate_fingerprint(fingerprint: &FileFingerprint, text: &str) -> Result<()> {
    validate_text(text)?;
    if fingerprint.len != text.len() as u64 || fingerprint.sha256 != hash(text.as_bytes()) {
        return Err(invalid("editor fingerprint does not match exact bytes"));
    }
    Ok(())
}

fn read_editor(conn: &Connection, path: &str) -> Result<Option<EditorRecord>> {
    let row: Option<(Vec<u8>, Vec<u8>)> = conn
        .query_row(
            "SELECT record_json,record_sha256 FROM editors WHERE path=?1",
            [path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    row.map(|(bytes, digest)| {
        let record: EditorRecord = decode(bytes, digest)?;
        if record.path != path {
            return Err(invalid("stored editor path does not match its key"));
        }
        validate_path(&record.path)?;
        validate_fingerprint(&record.baseline, &record.baseline_text)?;
        validate_text(&record.text)?;
        Ok(record)
    })
    .transpose()
}

fn write_editor(conn: &Connection, record: &EditorRecord) -> Result<()> {
    let bytes = encode(record)?;
    conn.execute(
        "INSERT INTO editors(path,record_json,record_sha256) VALUES(?1,?2,?3)
         ON CONFLICT(path) DO UPDATE SET record_json=excluded.record_json,record_sha256=excluded.record_sha256",
        params![record.path, bytes, hash(&bytes).as_slice()],
    )?;
    Ok(())
}

fn accept_edit(conn: &Connection, request: &EditRequest) -> Result<EditorRecord> {
    validate_path(&request.path)?;
    validate_text(&request.text)?;
    let mut record = read_editor(conn, &request.path)?
        .ok_or_else(|| Error::NotFound("editor is not open".into()))?;
    if request.expected.baseline != record.stamp.baseline
        || request.expected.generation > record.stamp.generation
        || request.generation < record.stamp.generation
        || request.generation == record.stamp.generation && request.text != record.text
    {
        return Err(Error::StateChanged(
            "editor baseline or generation changed".into(),
        ));
    }
    record.stamp.generation = request.generation;
    record.text = request.text.clone();
    write_editor(conn, &record)?;
    Ok(record)
}

fn read_save(conn: &Connection, id: Uuid) -> Result<Option<SaveIntent>> {
    type SaveRow = (String, bool, Option<String>, Vec<u8>, Vec<u8>, Vec<u8>);
    let row: Option<SaveRow> = conn.query_row(
        "SELECT path,original,outcome,request_sha256,intent_json,intent_sha256 FROM editor_saves WHERE operation_id=?1",
        [id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).optional()?;
    row.map(|(path, original, outcome, request_hash, bytes, digest)| {
        let intent: SaveIntent = decode(bytes, digest)?;
        if intent.request.operation_id != id
            || intent.request.edit.path != path
            || original != intent.request.destination.is_none()
            || request_hash.as_slice() != hash(&encode(&(&intent.request, &intent.staging))?)
            || outcome.as_deref()
                != intent
                    .receipt
                    .as_ref()
                    .map(|receipt| receipt.outcome.as_str())
        {
            return Err(invalid(
                "stored save intent does not match its key or binding",
            ));
        }
        validate_fingerprint(&intent.baseline, &intent.baseline_text)?;
        validate_path(&intent.request.edit.path)?;
        validate_text(&intent.request.edit.text)?;
        if let Some(destination) = &intent.request.destination {
            validate_path(destination)?;
        }
        if intent.no_op
            && (intent.request.destination.is_some()
                || intent.request.edit.text != intent.baseline_text
                || intent.prepared.is_some())
        {
            return Err(invalid(
                "recorded unchanged save has incompatible write state",
            ));
        }
        if let Some(prepared) = &intent.prepared {
            if prepared.relative != intent.staging {
                return Err(invalid("stored prepared path differs from save staging"));
            }
            validate_fingerprint(&prepared.fingerprint, &intent.request.edit.text)?;
        }
        if let Some(receipt) = &intent.receipt
            && (receipt.operation_id != id
                || receipt.path != path
                || receipt.destination != intent.request.destination
                || receipt.submitted_generation != intent.request.edit.generation
                || receipt.stamp.generation != intent.request.edit.generation)
        {
            return Err(invalid("stored save receipt differs from its submission"));
        }
        Ok(intent)
    })
    .transpose()
}

fn write_save(conn: &Connection, intent: &SaveIntent) -> Result<()> {
    let bytes = encode(intent)?;
    conn.execute(
        "UPDATE editor_saves SET outcome=?2,intent_json=?3,intent_sha256=?4 WHERE operation_id=?1",
        params![
            intent.request.operation_id.to_string(),
            intent.receipt.as_ref().map(|r| r.outcome.as_str()),
            bytes,
            hash(&bytes).as_slice()
        ],
    )?;
    Ok(())
}

fn read_completed(conn: &Connection, id: Uuid) -> Result<Option<([u8; 32], CompletedSave)>> {
    type CompletedRow = (Vec<u8>, Vec<u8>, Vec<u8>, bool);
    let row: Option<CompletedRow> = conn.query_row(
        "SELECT request_sha256,receipt_json,receipt_sha256,no_op FROM editor_completed WHERE operation_id=?1",
        [id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional()?;
    row.map(|(request_hash, bytes, digest, no_op)| {
        let completed: CompletedSave = decode(bytes, digest)?;
        let request_hash = request_hash
            .try_into()
            .map_err(|_| invalid("completed save request hash has an invalid length"))?;
        if completed.receipt.operation_id != id
            || completed.no_op != no_op
            || completed.receipt.outcome == SaveOutcome::Uncertain
            || completed.receipt.stamp.generation != completed.receipt.submitted_generation
            || completed.no_op
                && (completed.receipt.outcome != SaveOutcome::NotApplied
                    || completed.receipt.destination.is_some())
        {
            return Err(invalid(
                "completed save receipt does not match its key or outcome",
            ));
        }
        validate_path(&completed.receipt.path)?;
        if let Some(destination) = &completed.receipt.destination {
            validate_path(destination)?;
        }
        Ok((request_hash, completed))
    })
    .transpose()
}

/// Validate internal editor recovery without observing or installing vault files.
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    for path in super::backup::keys(conn, "editors", "path")? {
        read_editor(conn, &path)?.ok_or_else(|| invalid("listed editor is missing"))?;
    }
    for id in super::backup::keys(conn, "editor_saves", "operation_id")? {
        let uuid = crate::parse_id(id.clone())?;
        if uuid.to_string() != id {
            return Err(invalid("noncanonical save UUID"));
        }
        let intent = read_save(conn, uuid)?.ok_or_else(|| invalid("listed save is missing"))?;
        let installed: Option<(Vec<u8>, Vec<u8>)> = conn.query_row(
            "SELECT installed_json,installed_sha256 FROM editor_saves WHERE operation_id=?1 AND installed_json IS NOT NULL",
            [&id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        let installed: Option<FileFingerprint> = installed
            .map(|(bytes, digest)| decode(bytes, digest))
            .transpose()?;
        if let Some(proof) = installed {
            validate_fingerprint(&proof, &intent.request.edit.text)?;
            if intent.receipt.as_ref().map(|receipt| receipt.outcome) != Some(SaveOutcome::Applied)
                || intent
                    .prepared
                    .as_ref()
                    .map(|prepared| &prepared.fingerprint)
                    != Some(&proof)
            {
                return Err(invalid("save installation proof differs from its journal"));
            }
        } else if intent.receipt.as_ref().map(|receipt| receipt.outcome)
            == Some(SaveOutcome::Applied)
        {
            return Err(invalid("applied save lacks installation proof"));
        }
    }
    for id in super::backup::keys(conn, "editor_completed", "operation_id")? {
        let uuid = crate::parse_id(id.clone())?;
        if uuid.to_string() != id {
            return Err(invalid("noncanonical completed save UUID"));
        }
        read_completed(conn, uuid)?.ok_or_else(|| invalid("listed completed save is missing"))?;
    }
    let mut previous = conn.prepare("SELECT path,pair_json,pair_sha256 FROM editor_previous")?;
    let mut rows = previous.query([])?;
    while let Some(row) = rows.next()? {
        validate_path(&row.get::<_, String>(0)?)?;
        let pair: (String, String) = decode(row.get(1)?, row.get(2)?)?;
        validate_text(&pair.0)?;
        validate_text(&pair.1)?;
    }
    Ok(())
}

impl WorkStore {
    pub fn editor(&self, path: &str) -> Result<Option<EditorRecord>> {
        read_editor(&self.conn, path)
    }

    pub fn editors(&self) -> Result<Vec<EditorRecord>> {
        let mut statement = self
            .conn
            .prepare("SELECT path FROM editors ORDER BY path")?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .map(|path| {
                read_editor(&self.conn, &path?)?.ok_or_else(|| invalid("editor disappeared"))
            })
            .collect()
    }

    /// Existing recovery is never replaced by a fresh observation.
    pub fn open_editor(
        &mut self,
        path: &str,
        fingerprint: &FileFingerprint,
        text: &str,
    ) -> Result<EditorRecord> {
        if let Some(record) = self.editor(path)? {
            return Ok(record);
        }
        validate_path(path)?;
        validate_fingerprint(fingerprint, text)?;
        let recovered = self.unsaved_edit(path)?;
        if recovered
            .as_ref()
            .is_some_and(|edit| edit.base_sha256 != fingerprint.sha256)
        {
            return Err(Error::StateChanged("legacy recovered text has a different saved baseline; inspect it before adopting current bytes".into()));
        }
        let record = EditorRecord {
            path: path.into(),
            stamp: EditStamp {
                baseline: Uuid::new_v4(),
                generation: u64::from(recovered.is_some()),
            },
            baseline: fingerprint.clone(),
            baseline_text: text.into(),
            text: recovered.map_or_else(|| text.into(), |edit| edit.text),
        };
        let tx = self.conn.transaction()?;
        write_editor(&tx, &record)?;
        // The exact work now has a generation-aware durable owner; remove only
        // this promoted row in the same transaction, never a conflicting row.
        tx.execute("DELETE FROM unsaved_edits WHERE path=?1", [path])?;
        tx.commit()?;
        Ok(record)
    }

    /// Adopts freshly revalidated disk bytes after explicit discard of local
    /// changes. A prior uncertain original write needs its separate resolution.
    pub fn reload_editor(
        &mut self,
        path: &str,
        expected: EditStamp,
        observed: &FileFingerprint,
        text: &str,
        discard: bool,
    ) -> Result<EditorRecord> {
        validate_fingerprint(observed, text)?;
        let tx = self.conn.transaction()?;
        let mut record =
            read_editor(&tx, path)?.ok_or_else(|| Error::NotFound("editor is not open".into()))?;
        if record.stamp != expected {
            return Err(Error::StateChanged(
                "editor stamp changed before reload".into(),
            ));
        }
        let blocked: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM editor_saves WHERE path=?1 AND original=1 AND (outcome IS NULL OR outcome='uncertain'))",
            [path], |row| row.get(0),
        )?;
        if blocked {
            return Err(Error::SaveUncertain(
                "reconcile the original save before reloading".into(),
            ));
        }
        if record.text != record.baseline_text && !discard {
            return Err(Error::StateChanged(
                "reload requires explicit discard of unfinished edits".into(),
            ));
        }
        record.stamp.baseline = Uuid::new_v4();
        record.baseline = observed.clone();
        record.baseline_text = text.into();
        record.text = text.into();
        write_editor(&tx, &record)?;
        tx.commit()?;
        Ok(record)
    }

    pub fn recover_editor(&mut self, request: &EditRequest) -> Result<EditorRecord> {
        let tx = self.conn.transaction()?;
        let record = accept_edit(&tx, request)?;
        tx.commit()?;
        Ok(record)
    }

    /// Returns a bound existing intent before validating current editor state.
    /// Its existence is recovery/replay information, never permission to repeat I/O.
    pub fn begin_editor_save(
        &mut self,
        request: &SaveRequest,
        staging: &Path,
    ) -> Result<SaveIntent> {
        if self.editor_save_replay(request, staging)?.is_some() {
            return Err(Error::StateChanged(
                "save operation is already settled; use its compact receipt".into(),
            ));
        }
        if let Some(intent) = self.editor_save(request.operation_id)? {
            if intent.request != *request || intent.staging != staging {
                return Err(Error::OperationConflict(
                    "save operation ID has another payload".into(),
                ));
            }
            return Ok(intent);
        }
        let destination = request.destination.as_deref().unwrap_or(&request.edit.path);
        validate_path(destination)?;
        if request.destination.as_deref() == Some(&request.edit.path) {
            return Err(invalid("copy destination must differ from the original"));
        }
        let expected_stage =
            Path::new(destination).with_file_name(format!(".brn-{}.stage", request.operation_id));
        if staging != expected_stage {
            return Err(invalid("save staging must be its operation-owned sibling"));
        }
        let tx = self.conn.transaction()?;
        if request.destination.is_none() {
            let blocked: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM editor_saves WHERE path=?1 AND original=1 AND (outcome IS NULL OR outcome='uncertain'))",
                [&request.edit.path], |row| row.get(0),
            )?;
            if blocked {
                return Err(Error::SaveUncertain(
                    "original save requires reconciliation of its prior intent".into(),
                ));
            }
        }
        let editor = accept_edit(&tx, &request.edit)?;
        let intent = SaveIntent {
            request: request.clone(),
            baseline: editor.baseline,
            baseline_text: editor.baseline_text,
            staging: staging.into(),
            parent: None,
            no_op: false,
            prepared: None,
            receipt: None,
        };
        let bytes = encode(&intent)?;
        tx.execute(
            "INSERT INTO editor_saves(operation_id,path,original,request_sha256,intent_json,intent_sha256) VALUES(?1,?2,?3,?4,?5,?6)",
            params![request.operation_id.to_string(), request.edit.path, request.destination.is_none(), hash(&encode(&(request, staging))?).as_slice(), bytes, hash(&bytes).as_slice()],
        )?;
        tx.commit()?;
        Ok(intent)
    }

    pub fn editor_save(&self, id: Uuid) -> Result<Option<SaveIntent>> {
        read_save(&self.conn, id)
    }

    /// UUID-only inspection remains available after full payload retirement.
    pub fn editor_save_receipt(&self, id: Uuid) -> Result<Option<SaveReceipt>> {
        if let Some((_, completed)) = read_completed(&self.conn, id)? {
            return Ok(Some(completed.receipt));
        }
        Ok(read_save(&self.conn, id)?.and_then(|intent| intent.receipt))
    }

    /// Validates an old UUID's exact submission before any current-state gate.
    /// A compact receipt can never authorize repeating filesystem installation.
    pub fn editor_save_replay(
        &self,
        request: &SaveRequest,
        staging: &Path,
    ) -> Result<Option<(SaveReceipt, bool)>> {
        let Some((request_hash, completed)) = read_completed(&self.conn, request.operation_id)?
        else {
            return Ok(None);
        };
        if request_hash != hash(&encode(&(request, staging))?) {
            return Err(Error::OperationConflict(
                "save operation ID has another payload".into(),
            ));
        }
        if completed.receipt.path != request.edit.path
            || completed.receipt.destination != request.destination
            || completed.receipt.submitted_generation != request.edit.generation
        {
            return Err(invalid(
                "completed save receipt differs from its bound request",
            ));
        }
        Ok(Some((completed.receipt, completed.no_op)))
    }

    /// The workflow calls this only after it has retired any proven obsolete
    /// artifacts. Pending, uncertain and the latest Applied original retain
    /// their full journal; this method never observes or removes files.
    pub fn compact_editor_save(&mut self, id: Uuid) -> Result<()> {
        let tx = self.conn.transaction()?;
        if read_completed(&tx, id)?.is_some() {
            return Ok(());
        }
        let intent =
            read_save(&tx, id)?.ok_or_else(|| Error::NotFound("save intent is absent".into()))?;
        let Some(receipt) = &intent.receipt else {
            return Ok(());
        };
        if receipt.outcome == SaveOutcome::Uncertain {
            return Ok(());
        }
        if receipt.outcome == SaveOutcome::Applied && intent.request.destination.is_none() {
            let latest: String = tx.query_row(
                "SELECT operation_id FROM editor_saves WHERE path=?1 AND original=1 AND outcome='applied' ORDER BY rowid DESC LIMIT 1",
                [&intent.request.edit.path], |row| row.get(0),
            )?;
            if latest == id.to_string() {
                return Ok(());
            }
        }
        let completed = CompletedSave {
            receipt: receipt.clone(),
            no_op: intent.no_op,
        };
        let bytes = encode(&completed)?;
        tx.execute(
            "INSERT INTO editor_completed(operation_id,request_sha256,receipt_json,receipt_sha256,no_op) VALUES(?1,?2,?3,?4,?5)",
            params![id.to_string(), hash(&encode(&(&intent.request, &intent.staging))?).as_slice(), bytes, hash(&bytes).as_slice(), completed.no_op],
        )?;
        tx.execute(
            "DELETE FROM editor_saves WHERE operation_id=?1",
            [id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn editor_saves(&self) -> Result<Vec<SaveIntent>> {
        let mut statement = self
            .conn
            .prepare("SELECT operation_id FROM editor_saves ORDER BY rowid")?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .map(|id| {
                read_save(&self.conn, crate::parse_id(id?)?)?
                    .ok_or_else(|| invalid("save intent disappeared"))
            })
            .collect()
    }

    /// Binds the observed destination directory before staging. A later parent
    /// replacement cannot inherit this intent's permission or recovery proof.
    pub fn bind_editor_save_parent(&mut self, id: Uuid, parent: &VaultIdentity) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut intent =
            read_save(&tx, id)?.ok_or_else(|| Error::NotFound("save intent is absent".into()))?;
        if intent.receipt.is_some() || intent.prepared.is_some() {
            return Err(Error::StateChanged(
                "save parent must be bound before preparation or completion".into(),
            ));
        }
        if let Some(existing) = &intent.parent {
            if existing == parent {
                return Ok(());
            }
            return Err(Error::OperationConflict(
                "save parent identity is already bound".into(),
            ));
        }
        intent.parent = Some(parent.clone());
        write_save(&tx, &intent)?;
        tx.commit()?;
        Ok(())
    }

    pub fn prepare_editor_save(&mut self, id: Uuid, prepared: &PreparedFile) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut intent =
            read_save(&tx, id)?.ok_or_else(|| Error::NotFound("save intent is absent".into()))?;
        if intent.prepared.as_ref() == Some(prepared) {
            return Ok(());
        }
        if intent.receipt.is_some() || intent.prepared.is_some() || intent.no_op {
            return Err(invalid(
                "save intent already has a prepared identity, unchanged branch or receipt",
            ));
        }
        if prepared.relative != intent.staging {
            return Err(invalid("prepared path differs from save staging"));
        }
        validate_fingerprint(&prepared.fingerprint, &intent.request.edit.text)?;
        intent.prepared = Some(prepared.clone());
        write_save(&tx, &intent)?;
        tx.commit()?;
        Ok(())
    }

    /// Marks only the workflow's coordinated, freshly verified unchanged branch.
    /// Equal submitted text alone does not establish a successful no-op.
    pub fn mark_editor_save_noop(&mut self, id: Uuid) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut intent =
            read_save(&tx, id)?.ok_or_else(|| Error::NotFound("save intent is absent".into()))?;
        if intent.receipt.is_some() {
            return Err(Error::StateChanged(
                "unchanged save must be marked before completion".into(),
            ));
        }
        if intent.request.destination.is_some()
            || intent.request.edit.text != intent.baseline_text
            || intent.prepared.is_some()
        {
            return Err(Error::StateChanged(
                "unchanged save requires an unprepared original with exact baseline text".into(),
            ));
        }
        if intent.no_op {
            return Ok(());
        }
        intent.no_op = true;
        write_save(&tx, &intent)?;
        tx.commit()?;
        Ok(())
    }

    /// The workflow supplies the observed installation proof. This commits the
    /// receipt, original baseline and protected recovery together.
    pub fn finish_editor_save(
        &mut self,
        id: Uuid,
        outcome: SaveOutcome,
        installed: Option<&FileFingerprint>,
    ) -> Result<SaveReceipt> {
        let tx = self.conn.transaction()?;
        let mut intent =
            read_save(&tx, id)?.ok_or_else(|| Error::NotFound("save intent is absent".into()))?;
        if let Some(receipt) = &intent.receipt {
            if receipt.outcome == outcome {
                let stored: Option<(Vec<u8>, Vec<u8>)> = tx.query_row(
                    "SELECT installed_json,installed_sha256 FROM editor_saves WHERE operation_id=?1 AND installed_json IS NOT NULL",
                    [id.to_string()], |row| Ok((row.get(0)?, row.get(1)?)),
                ).optional()?;
                let stored: Option<FileFingerprint> = stored
                    .map(|(bytes, digest)| decode(bytes, digest))
                    .transpose()?;
                if stored.as_ref() != installed {
                    return Err(Error::OperationConflict(
                        "save completion proof differs".into(),
                    ));
                }
                return Ok(receipt.clone());
            }
            if receipt.outcome != SaveOutcome::Uncertain || outcome == SaveOutcome::Uncertain {
                return Err(Error::OperationConflict(
                    "save completion conflicts with its receipt".into(),
                ));
            }
        }
        if outcome != SaveOutcome::Applied && installed.is_some() {
            return Err(invalid("only an applied save can carry installation proof"));
        }
        let mut editor = read_editor(&tx, &intent.request.edit.path)?
            .ok_or_else(|| invalid("save editor is absent"))?;
        let mut stamp = EditStamp {
            baseline: intent.request.edit.expected.baseline,
            generation: intent.request.edit.generation,
        };
        if outcome == SaveOutcome::Applied {
            let installed =
                installed.ok_or_else(|| invalid("applied save requires installation proof"))?;
            let prepared = intent
                .prepared
                .as_ref()
                .ok_or_else(|| invalid("applied save lacks a prepared identity"))?;
            if &prepared.fingerprint != installed {
                return Err(invalid("installed file differs from prepared identity"));
            }
            validate_fingerprint(installed, &intent.request.edit.text)?;
            if intent.request.destination.is_none() {
                if editor.stamp.baseline != intent.request.edit.expected.baseline
                    || editor.baseline != intent.baseline
                    || editor.baseline_text != intent.baseline_text
                    || editor.stamp.generation < intent.request.edit.generation
                {
                    return Err(Error::StateChanged(
                        "original save baseline changed before completion".into(),
                    ));
                }
                stamp.baseline = Uuid::new_v4();
                editor.stamp.baseline = stamp.baseline;
                editor.baseline = installed.clone();
                editor.baseline_text = intent.request.edit.text.clone();
                write_editor(&tx, &editor)?;
                let pair = encode(&(&intent.baseline_text, &intent.request.edit.text))?;
                tx.execute(
                    "INSERT INTO editor_previous(path,pair_json,pair_sha256) VALUES(?1,?2,?3)
                     ON CONFLICT(path) DO UPDATE SET pair_json=excluded.pair_json,pair_sha256=excluded.pair_sha256",
                    params![editor.path, pair, hash(&pair).as_slice()],
                )?;
            }
        }
        let receipt = SaveReceipt {
            operation_id: id,
            path: intent.request.edit.path.clone(),
            destination: intent.request.destination.clone(),
            submitted_generation: intent.request.edit.generation,
            stamp,
            outcome,
        };
        intent.receipt = Some(receipt.clone());
        write_save(&tx, &intent)?;
        let proof = installed.map(encode).transpose()?;
        tx.execute(
            "UPDATE editor_saves SET installed_json=?2,installed_sha256=?3 WHERE operation_id=?1",
            params![
                id.to_string(),
                proof,
                proof.as_ref().map(|bytes| hash(bytes).to_vec())
            ],
        )?;
        tx.commit()?;
        Ok(receipt)
    }

    pub fn editor_previous(&self, path: &str) -> Result<Option<(String, String)>> {
        let row: Option<(Vec<u8>, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT pair_json,pair_sha256 FROM editor_previous WHERE path=?1",
                [path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(bytes, digest)| decode(bytes, digest)).transpose()
    }
}
