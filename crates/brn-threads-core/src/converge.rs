//! No-op detection for converge commands (owner decision B4): archive,
//! unarchive, chat move and project link/unlink return `Unchanged` when the
//! requested state already holds, recording no receipt and bumping no version.
//! The hook runs right before preparing, so binding, receipt replay and every
//! refusal run first.
use crate::store::record_on;
use crate::*;

/// What a converging command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Converged<T> {
    Changed(T),
    Unchanged(Unchanged),
}
/// The requested state already held: nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unchanged {
    /// The thread (archive, unarchive), chat (move) or project (link, unlink).
    pub record: String,
    /// Its current version.
    pub version: u64,
}

/// Project chats and documents compare as sets; a chat move ignores the
/// context epoch it always bumps.
fn normalized(data: &RecordData, ignore_epoch: bool) -> RecordData {
    let mut data = data.clone();
    match &mut data {
        RecordData::Project(project) => {
            project.chats.sort();
            project.chats.dedup();
            project.documents.sort();
            project.documents.dedup();
        }
        RecordData::Thread(thread) if ignore_epoch => thread.context_epoch = 0,
        _ => {}
    }
    data
}

/// `Some` iff every write targets an existing record at its current version
/// and leaves its archived flag and data as they are.
pub(crate) fn detect_unchanged(
    tx: &rusqlite::Connection,
    record: &str,
    writes: &[Put],
    ignore_epoch: bool,
) -> Result<Option<Unchanged>> {
    for put in writes {
        let Some(expected) = put.expected_version else {
            return Ok(None);
        };
        let Some(current) = record_on(tx, &put.id)? else {
            return Ok(None);
        };
        if current.version != expected
            || current.archived != put.archived
            || normalized(&current.data, ignore_epoch) != normalized(&put.data, ignore_epoch)
        {
            return Ok(None);
        }
    }
    let current = record_on(tx, record)?.ok_or_else(|| Error::Missing(record.into()))?;
    Ok(Some(Unchanged {
        record: record.into(),
        version: current.version,
    }))
}
