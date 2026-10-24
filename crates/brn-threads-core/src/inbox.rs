//! Machine-local manual-inbox authorization and recovery metadata; never source bytes.
use crate::*;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

pub(crate) const INBOX_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS inbox_state(id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL, body TEXT NOT NULL);";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InboxDestination {
    General,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InboxStatus {
    Pending,
    Imported,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InboxEntry {
    pub name: String,
    pub source_sha256: Option<[u8; 32]>,
    pub operation_key: Option<String>,
    pub status: InboxStatus,
    pub detail: String,
    pub note: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InboxState {
    pub revision: u64,
    pub generation: String,
    pub folder: String,
    pub folder_identity: String,
    pub destination: InboxDestination,
    pub entries: Vec<InboxEntry>,
}

impl Store {
    /// Keep the explicit folder generation and captured conversion profile current
    /// through the ordinary checked
    /// writer's transaction. Unrelated theme/provider settings do not change it.
    pub fn apply_inbox_import(
        &mut self,
        operation: &OperationId,
        settings: &DocumentConversionSettings,
        generation: &str,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "Inbox import requires owner authority".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inbox_body: Option<String> = tx
            .query_row("SELECT body FROM inbox_state WHERE id=1", [], |r| r.get(0))
            .optional()?;
        let inbox = inbox_body
            .map(|body| serde_json::from_str::<InboxState>(&body))
            .transpose()?;
        if !inbox.as_ref().is_some_and(|state| {
            state.generation == generation && state.destination == InboxDestination::General
        }) {
            return Err(Error::Invalid(
                "Inbox folder choice changed; scan again".into(),
            ));
        }
        let body:String=tx.query_row("SELECT body FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL AND json_extract(body,'$.archived')=0",[],|r|r.get(0))?;
        let record: Record = serde_json::from_str(&body)?;
        if !matches!(&record.data,RecordData::Settings(current) if &current.document_conversion==settings)
        {
            return Err(Error::Invalid(
                "Document conversion settings changed; scan again".into(),
            ));
        }
        let outcome = crate::store::apply_on(&tx, operation, authority, None)?;
        tx.commit()?;
        Ok(outcome)
    }
    pub fn inbox_state(&self) -> Result<Option<InboxState>> {
        let body: Option<String> = self
            .conn
            .query_row("SELECT body FROM inbox_state WHERE id=1", [], |r| r.get(0))
            .optional()?;
        body.map(|body| serde_json::from_str(&body).map_err(Error::from))
            .transpose()
    }
    /// Compare-and-swap the complete bounded local ledger. Owner-host only;
    /// knowledge writes continue to use ordinary import operations/receipts.
    pub fn save_inbox_state(
        &mut self,
        expected: Option<u64>,
        state: &InboxState,
        authority: &HostAuthority,
    ) -> Result<InboxState> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "Inbox configuration requires owner authority".into(),
            ));
        }
        if state.generation.is_empty()
            || state.generation.len() > 128
            || !std::path::Path::new(&state.folder).is_absolute()
            || state.folder.len() > 4096
            || state.folder_identity.len() > 128
            || state.entries.len() > 256
        {
            return Err(Error::Invalid("Invalid inbox configuration".into()));
        }
        let mut names = std::collections::BTreeSet::new();
        for entry in &state.entries {
            let name = std::path::Path::new(&entry.name);
            if entry.name.len() > 255
                || name.components().count() != 1
                || !matches!(
                    name.components().next(),
                    Some(std::path::Component::Normal(_))
                )
                || !names.insert(&entry.name)
                || entry.detail.len() > 1024
                || entry.operation_key.as_ref().is_some_and(|k| k.len() > 256)
                || entry.note.as_ref().is_some_and(|n| n.len() > 256)
            {
                return Err(Error::Invalid("Invalid inbox entry".into()));
            }
            if entry.status == InboxStatus::Imported
                && (entry.note.is_none()
                    || entry.source_sha256.is_none()
                    || entry.operation_key.is_none())
            {
                return Err(Error::Invalid(
                    "Imported inbox entries need a retained note and source identity".into(),
                ));
            }
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actual: Option<i64> = tx
            .query_row("SELECT revision FROM inbox_state WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()?;
        let actual = actual
            .map(u64::try_from)
            .transpose()
            .map_err(|_| Error::Invalid("Invalid inbox revision".into()))?;
        if actual != expected {
            return Err(Error::Invalid(
                "Inbox changed; refresh before scanning again".into(),
            ));
        }
        if actual.is_some() {
            let prior: String =
                tx.query_row("SELECT body FROM inbox_state WHERE id=1", [], |r| r.get(0))?;
            let prior: InboxState = serde_json::from_str(&prior)?;
            if prior.generation == state.generation
                && (prior.folder != state.folder
                    || prior.folder_identity != state.folder_identity
                    || prior.destination != state.destination)
            {
                return Err(Error::Invalid(
                    "A changed inbox folder requires a fresh explicit choice".into(),
                ));
            }
            if prior.generation != state.generation && !state.entries.is_empty() {
                return Err(Error::Invalid(
                    "A fresh inbox choice cannot inherit another folder's ledger".into(),
                ));
            }
        }
        let mut state = state.clone();
        state.revision = actual
            .unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("Inbox revision exhausted".into()))?;
        tx.execute("INSERT INTO inbox_state(id,revision,body) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,body=excluded.body",params![crate::integer(state.revision)?,serde_json::to_string(&state)?])?;
        tx.commit()?;
        Ok(state)
    }
}
/// Restored knowledge and import receipts survive; local folder authority does not.
pub(crate) fn reset_on_restore(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM inbox_state", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inbox_is_bounded_owner_only_cas_state_and_reopens() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("data");
        let mut store = Store::open(&data).unwrap();
        let state = InboxState {
            revision: 0,
            generation: "synthetic-generation".into(),
            folder: root.path().join("inbox").display().to_string(),
            folder_identity: "synthetic-inode".into(),
            destination: InboxDestination::General,
            entries: vec![],
        };
        assert!(
            store
                .save_inbox_state(None, &state, &HostAuthority::maintenance("untrusted"))
                .is_err()
        );
        let saved = store
            .save_inbox_state(None, &state, &HostAuthority::owner("explicit choice"))
            .unwrap();
        assert!(
            store
                .save_inbox_state(None, &state, &HostAuthority::owner("stale"))
                .is_err()
        );
        drop(store);
        let mut store = Store::open_existing_without_upgrade(&data).unwrap();
        assert_eq!(store.inbox_state().unwrap(), Some(saved.clone()));
        let mut invalid = saved.clone();
        invalid.entries.push(InboxEntry {
            name: "../escape.txt".into(),
            source_sha256: None,
            operation_key: None,
            status: InboxStatus::Pending,
            detail: String::new(),
            note: None,
        });
        assert!(
            store
                .save_inbox_state(
                    Some(saved.revision),
                    &invalid,
                    &HostAuthority::owner("bad filename")
                )
                .is_err()
        );
        let tx = store.conn.transaction().unwrap();
        reset_on_restore(&tx).unwrap();
        tx.commit().unwrap();
        assert!(store.inbox_state().unwrap().is_none());
    }
    #[test]
    fn schema10_and11_upgrade_preserve_record_revision_and_receipt_bytes() {
        for version in [10, 11] {
            let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = root.path().join("data");
            let mut store = Store::open(&data).unwrap();
            let op = store
                .allocate_operation("synthetic-inbox-upgrade-note")
                .unwrap();
            let authority = HostAuthority::owner("synthetic upgrade fixture");
            store
                .prepare_owner(
                    &op,
                    &ChangeRequest {
                        reason: "Seed exact bytes".into(),
                        writes: vec![Put {
                            id: op.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Note(Note::working(
                                "Before inbox",
                                "Exact old note Ω\n",
                            )),
                        }],
                        inputs: vec![],
                    },
                    &authority,
                )
                .unwrap();
            store.apply(&op, &authority).unwrap();
            fn bodies(conn: &rusqlite::Connection) -> Vec<String> {
                [
                    "SELECT body FROM records ORDER BY id",
                    "SELECT body FROM revisions ORDER BY id,version",
                    "SELECT body FROM receipts ORDER BY operation",
                ]
                .into_iter()
                .flat_map(|sql| {
                    conn.prepare(sql)
                        .unwrap()
                        .query_map([], |r| r.get::<_, String>(0))
                        .unwrap()
                        .map(|r| r.unwrap())
                        .collect::<Vec<_>>()
                })
                .collect()
            }
            let prior = bodies(&store.conn);
            store.conn.execute_batch("DROP TABLE inbox_state").unwrap();
            store
                .conn
                .pragma_update(None, "user_version", version)
                .unwrap();
            drop(store);
            let before = std::fs::read(data.join(DATABASE_FILENAME)).unwrap();
            assert!(
                matches!(Store::open_existing_without_upgrade(&data),Err(Error::UpgradeRequired{schema_found,schema_required:12}) if schema_found==version)
            );
            assert_eq!(std::fs::read(data.join(DATABASE_FILENAME)).unwrap(), before);
            let backups = root.path().join("backups");
            std::fs::create_dir(&backups).unwrap();
            Store::upgrade(&data, &backups).unwrap();
            let store = Store::open_existing_without_upgrade(&data).unwrap();
            assert_eq!(bodies(&store.conn), prior);
            assert!(store.inbox_state().unwrap().is_none());
            let snapshot = std::fs::read_dir(backups)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let backup = rusqlite::Connection::open_with_flags(
                snapshot,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(
                backup
                    .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                version
            );
            assert_eq!(bodies(&backup), prior);
        }
    }
}
