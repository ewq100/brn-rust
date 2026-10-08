//! Exact same-folder Markdown Create revision and compact creation replay evidence.
use super::*;

const MAX_ORIGINAL_CREATE_PATH_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalCreatePath {
    pub change_index: usize,
    pub path: String,
}

fn rename_eligible(record: &ProposalRecord) -> Result<()> {
    if record.draft.inbox_source.is_some()
        || record.draft.inbox_visual.is_some()
        || record.draft.intake.is_some()
    {
        return Err(invalid(
            "bound Source, visual and direct intake destinations cannot be renamed",
        ));
    }
    Ok(())
}

pub(in crate::work) fn validate_original_create_paths(
    record: &ProposalRecord,
    originals: &[OriginalCreatePath],
) -> Result<()> {
    if originals.is_empty() {
        return Ok(());
    }
    rename_eligible(record)?;
    if record.version == 1 || originals.len() > MAX_PROPOSAL_CHANGES {
        return Err(invalid(
            "invalid original Create path count or initial version",
        ));
    }
    let bytes = encode(&originals)?.len();
    if bytes > MAX_ORIGINAL_CREATE_PATH_BYTES {
        return Err(invalid(
            "original Create paths exceed their 64 KiB encoded limit",
        ));
    }
    let mut previous = None;
    for original in originals {
        if previous.is_some_and(|index| index >= original.change_index) {
            return Err(invalid(
                "original Create path indices must be sorted and unique",
            ));
        }
        previous = Some(original.change_index);
        let Some(NoteChange::Create { path, .. }) = record.draft.changes.get(original.change_index)
        else {
            return Err(invalid(
                "original Create path must identify a Markdown Create",
            ));
        };
        validate_path(&original.path)?;
        if Path::new(&original.path).parent() != Path::new(path).parent() {
            return Err(invalid(
                "original Create path differs from its current folder",
            ));
        }
    }
    super::super::proposal_apply::reserve_asset_review_with_extra(record, bytes)?;
    Ok(())
}

pub(in crate::work) fn normalize_create_paths(
    draft: &mut ProposalDraft,
    originals: &[OriginalCreatePath],
) {
    for original in originals {
        if let Some(NoteChange::Create { path, .. }) = draft.changes.get_mut(original.change_index)
        {
            *path = original.path.clone();
        }
    }
}

fn renamed_review(
    before: &ProposalRecord,
    change_index: usize,
    path: &str,
) -> Result<ProposalRecord> {
    validate_record(before)?;
    if before.state != ProposalState::Draft {
        return Err(Error::StateChanged(
            "only a Draft Create may be renamed".into(),
        ));
    }
    rename_eligible(before)?;
    validate_path(path)?;
    let Some(NoteChange::Create { path: current, .. }) = before.draft.changes.get(change_index)
    else {
        return Err(invalid("rename needs an indexed Markdown Create"));
    };
    if Path::new(current).parent() != Path::new(path).parent() {
        return Err(invalid("Create rename must remain in its existing folder"));
    }
    let mut revised = before.clone();
    let NoteChange::Create {
        path: destination, ..
    } = &mut revised.draft.changes[change_index]
    else {
        unreachable!("checked Create")
    };
    *destination = path.into();
    validate_record(&revised)?;
    Ok(revised)
}

/// Verify the complete acknowledgement without filesystem or Store access.
/// No-op acknowledgements remain byte-exact; changed paths advance exactly once.
pub fn validate_create_rename_transition(
    before: &ProposalRecord,
    after: &ProposalRecord,
    change_index: usize,
    path: &str,
) -> Result<()> {
    validate_record(after)?;
    let mut expected = renamed_review(before, change_index, path)?;
    if expected != *before {
        expected.version = before
            .version
            .checked_add(1)
            .ok_or_else(|| invalid("proposal review version overflow"))?;
        if after.updated_at_ms < before.updated_at_ms {
            return Err(invalid("Create rename timestamp moved backwards"));
        }
        expected.updated_at_ms = after.updated_at_ms;
    }
    if expected != *after {
        return Err(invalid("Create rename changed preserved review work"));
    }
    Ok(())
}

impl WorkStore {
    /// Changes only one current Draft Create filename and its first-path evidence.
    /// The workflow owns fresh destination/parent/editor/evidence qualification.
    pub fn rename_proposal_create(
        &mut self,
        expected: ProposalStamp,
        change_index: usize,
        path: &str,
    ) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, expected)?;
        let before = stored.record;
        let mut revised = renamed_review(&before, change_index, path)?;
        if revised == before {
            return Ok(before);
        }
        if let Err(index) = stored
            .original_create_paths
            .binary_search_by_key(&change_index, |entry| entry.change_index)
        {
            stored.original_create_paths.insert(
                index,
                OriginalCreatePath {
                    change_index,
                    path: before.draft.changes[change_index].path().into(),
                },
            );
        }
        advance(&mut revised)?;
        validate_create_rename_transition(&before, &revised, change_index, path)?;
        stored.record = revised;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }

    /// Checked compact original paths for reconstruction of the initial request.
    pub fn proposal_original_create_paths(&self, id: Uuid) -> Result<Vec<OriginalCreatePath>> {
        read_proposal(&self.conn, id)?
            .map(|stored| stored.original_create_paths)
            .ok_or_else(|| Error::NotFound("proposal does not exist".into()))
    }
}
