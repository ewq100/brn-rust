//! One explicit structural revision; the workflow owns fresh filesystem capture.
use super::*;
use crate::work::inbox_actions::InboxSupersedesBinding;

fn attached_review(before: &ProposalRecord, history: &NoteChange) -> Result<ProposalRecord> {
    validate_record(before)?;
    let binding = before
        .draft
        .inbox_knowledge
        .as_ref()
        .filter(|binding| binding.supersedes.is_none())
        .ok_or_else(|| invalid("predecessor attachment needs supplemental Inbox knowledge"))?;
    let [NoteChange::Create { .. }] = before.draft.changes.as_slice() else {
        return Err(invalid("predecessor attachment needs exactly one Create"));
    };
    if before.state != ProposalState::Draft {
        return Err(Error::StateChanged(
            "only a Draft may attach a predecessor".into(),
        ));
    }
    let NoteChange::Replace {
        path,
        before: proof,
        before_text,
        text,
        ..
    } = history
    else {
        return Err(invalid(
            "predecessor attachment needs an exact History Replace",
        ));
    };
    let note_id = crate::note_identity::read(before_text)?
        .ok_or_else(|| invalid("predecessor attachment needs managed Current knowledge"))?;
    let predecessor = SourceVersion {
        path: path.clone(),
        fingerprint: proof.clone(),
    };
    let mut binding = binding.clone();
    binding.supersedes = Some(InboxSupersedesBinding {
        note_id,
        source: predecessor.clone(),
    });
    binding.validate_history(path, proof, before_text, text)?;

    let mut revised = before.clone();
    let offset = usize::from(binding.source.is_some());
    if let Some(index) = revised
        .draft
        .sources
        .iter()
        .position(|source| source.path.eq_ignore_ascii_case(path))
    {
        if revised.draft.sources[index] != predecessor {
            return Err(Error::StateChanged(
                "captured predecessor source changed".into(),
            ));
        }
        revised.draft.sources.remove(index);
    }
    revised.draft.sources.insert(offset, predecessor);
    let NoteChange::Create { text, .. } = &mut revised.draft.changes[0] else {
        unreachable!("checked sole Create")
    };
    text.push_str(&format!(
        "\n\nPrevious version: [History](brn://note/{note_id})\n"
    ));
    revised.draft.changes.push(history.clone());
    revised.draft.inbox_knowledge = Some(binding);
    validate_record(&revised)?;
    Ok(revised)
}

/// Check the sole permitted None-to-Some predecessor revision without I/O.
/// All original fields, comment anchors and text-prefix bytes remain exact.
/// Readable Markdown and fresh filesystem authority belong to the workflow.
pub fn validate_knowledge_predecessor_transition(
    before: &ProposalRecord,
    after: &ProposalRecord,
) -> Result<()> {
    validate_record(after)?;
    let [_, history] = after.draft.changes.as_slice() else {
        return Err(invalid(
            "attached predecessor needs exactly Create and History",
        ));
    };
    let mut expected = attached_review(before, history)?;
    expected.version = before
        .version
        .checked_add(1)
        .ok_or_else(|| invalid("proposal review version overflow"))?;
    if after.updated_at_ms < before.updated_at_ms {
        return Err(invalid("predecessor attachment timestamp moved backwards"));
    }
    expected.updated_at_ms = after.updated_at_ms;
    if expected != *after {
        return Err(invalid(
            "predecessor attachment changed preserved review work",
        ));
    }
    Ok(())
}

impl WorkStore {
    /// Attach one workflow-captured predecessor in the existing proposal row.
    /// The immutable initial creation hash and all original review work survive.
    pub fn attach_inbox_knowledge_predecessor(
        &mut self,
        expected: ProposalStamp,
        history: &NoteChange,
    ) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, expected)?;
        let before = stored.record;
        let mut revised = attached_review(&before, history)?;
        advance(&mut revised)?;
        validate_knowledge_predecessor_transition(&before, &revised)?;
        stored.record = revised;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }
}
