//! Exact inverse derivation and atomic Undo admission. No filesystem effects.
use super::{
    WorkStore, actions, now_ms,
    proposal_apply::{
        self, ApplyJournal, ApplyMember, ApplyOutcome, ApprovalRequest, UndoBinding, UndoOriginal,
        UndoPreview, UndoRequest,
    },
    proposals::{
        self, ActionChange, NoteChange, ProposalDraft, ProposalRecord, ProposalState,
        StoredProposal,
    },
};
use crate::{Error, Result, hash, invalid};
use rusqlite::Connection;
use std::collections::HashSet;
use uuid::Uuid;

fn validate_request(request: &UndoRequest) -> Result<()> {
    proposals::nonnil(request.operation_id)?;
    proposals::nonnil(request.target_operation_id)?;
    if request.operation_id == request.target_operation_id {
        return Err(invalid(
            "Undo requires distinct new and source operation UUIDs",
        ));
    }
    if request
        .trash_member
        .is_some_and(|index| index >= proposals::MAX_PROPOSAL_CHANGES)
    {
        return Err(invalid("Trash member exceeds the bounded proposal domain"));
    }
    Ok(())
}

fn replay(conn: &Connection, request: &UndoRequest) -> Result<Option<ApplyJournal>> {
    let journal = proposal_apply::read_journal(conn, request.operation_id)?;
    if let Some(journal) = &journal
        && !journal.undo.as_ref().is_some_and(|binding| {
            binding.operation_id == request.target_operation_id
                && binding.trash_member == request.trash_member
        })
    {
        return Err(Error::OperationConflict(
            "Undo operation UUID has another request".into(),
        ));
    }
    Ok(journal)
}

fn derive(conn: &Connection, request: &UndoRequest) -> Result<UndoPreview> {
    let source = proposal_apply::read_journal(conn, request.target_operation_id)?
        .ok_or_else(|| Error::NotFound("Undo source operation is absent".into()))?;
    if source.receipt.as_ref().map(|receipt| receipt.outcome) != Some(ApplyOutcome::Applied) {
        return Err(Error::StateChanged(
            "Undo source must be a terminal Applied operation".into(),
        ));
    }
    let action_changes = if source.approved.draft.action_changes.is_empty() {
        Vec::new()
    } else {
        if !source.approved.draft.changes.is_empty()
            || request.trash_member.is_some()
            || !source
                .approved
                .draft
                .action_changes
                .iter()
                .all(|change| matches!(change, ActionChange::Replace { .. }))
        {
            return Err(invalid(
                "Action Undo requires an Action-only whole operation of replacements",
            ));
        }
        source
            .approved
            .draft
            .action_changes
            .iter()
            .zip(&source.action_records)
            .map(|(change, installed)| {
                let ActionChange::Replace { before, .. } = change else {
                    unreachable!("checked all-Replace operation")
                };
                ActionChange::Replace {
                    before: Box::new(installed.clone()),
                    data: before.data.clone(),
                }
            })
            .collect()
    };
    let prepared = source
        .prepared
        .as_ref()
        .ok_or_else(|| invalid("Undo source has no complete prepared proofs"))?;
    if let Some(index) = request.trash_member
        && !source
            .approved
            .draft
            .changes
            .get(index)
            .is_some_and(|change| {
                matches!(
                    change,
                    NoteChange::Trash { .. } | NoteChange::TrashAsset { .. }
                )
            })
    {
        return Err(invalid(
            "Trash restore must identify an original Trash member",
        ));
    }
    let mut changes = Vec::with_capacity(source.members.len());
    let mut originals = Vec::with_capacity(source.members.len());
    for (index, ((change, member), installed)) in source
        .approved
        .draft
        .changes
        .iter()
        .zip(&source.members)
        .zip(prepared)
        .enumerate()
    {
        if request
            .trash_member
            .is_some_and(|selected| selected != index)
        {
            continue;
        }
        match change {
            NoteChange::Create { path, parent, text } => {
                changes.push(NoteChange::Trash {
                    path: path.clone(),
                    parent: parent.clone(),
                    before: installed.clone(),
                    before_text: text.clone(),
                });
                originals.push(None);
            }
            NoteChange::Replace {
                path,
                parent,
                before,
                before_text,
                text,
            } => {
                changes.push(NoteChange::Replace {
                    path: path.clone(),
                    parent: parent.clone(),
                    before: installed.clone(),
                    before_text: text.clone(),
                    text: before_text.clone(),
                });
                originals.push(Some(UndoOriginal {
                    member_id: member.id,
                    fingerprint: before.clone(),
                }));
            }
            NoteChange::Trash {
                path,
                parent,
                before,
                before_text,
            } => {
                changes.push(NoteChange::Create {
                    path: path.clone(),
                    parent: parent.clone(),
                    text: before_text.clone(),
                });
                originals.push(Some(UndoOriginal {
                    member_id: member.id,
                    fingerprint: before.clone(),
                }));
            }
            NoteChange::CreateAsset {
                path,
                parent,
                bytes,
            } => {
                changes.push(NoteChange::TrashAsset {
                    path: path.clone(),
                    parent: parent.clone(),
                    before: installed.clone(),
                    before_bytes: bytes.clone(),
                });
                originals.push(None);
            }
            NoteChange::ReplaceAsset {
                path,
                parent,
                before,
                before_bytes,
                bytes,
            } => {
                changes.push(NoteChange::ReplaceAsset {
                    path: path.clone(),
                    parent: parent.clone(),
                    before: installed.clone(),
                    before_bytes: bytes.clone(),
                    bytes: before_bytes.clone(),
                });
                originals.push(Some(UndoOriginal {
                    member_id: member.id,
                    fingerprint: before.clone(),
                }));
            }
            NoteChange::TrashAsset {
                path,
                parent,
                before,
                before_bytes,
            } => {
                changes.push(NoteChange::CreateAsset {
                    path: path.clone(),
                    parent: parent.clone(),
                    bytes: before_bytes.clone(),
                });
                originals.push(Some(UndoOriginal {
                    member_id: member.id,
                    fingerprint: before.clone(),
                }));
            }
        }
    }
    let original_title = &source.approved.draft.title;
    let budget = original_title.len().min(512);
    let mut title = if budget < "Undo:".len() {
        original_title.clone()
    } else {
        format!("Undo: {original_title}")
    };
    if title.len() > budget {
        let mut end = budget;
        while !title.is_char_boundary(end) {
            end -= 1;
        }
        title.truncate(end);
    }
    let draft = ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: request.operation_id,
        group_id: None,
        session_id: source.approved.draft.session_id,
        vault: source.approved.draft.vault,
        title,
        changes,
        sources: Vec::new(),
        action_changes,
    };
    let preview = UndoPreview {
        draft,
        binding: UndoBinding {
            operation_id: request.target_operation_id,
            trash_member: request.trash_member,
            originals,
        },
    };
    // Enforce the same bounded proposal domain even before admission.
    let now = now_ms();
    proposals::validate_record(&ProposalRecord {
        draft: preview.draft.clone(),
        version: 1,
        state: ProposalState::Draft,
        comments: Vec::new(),
        created_at_ms: now,
        updated_at_ms: now,
    })?;
    Ok(preview)
}

impl WorkStore {
    /// Read-only exact inverse. A matching admitted operation keeps its snapshot.
    pub fn preview_proposal_undo(&self, request: &UndoRequest) -> Result<UndoPreview> {
        validate_request(request)?;
        if let Some(journal) = replay(&self.conn, request)? {
            return Ok(UndoPreview {
                draft: journal.approved.draft,
                binding: journal.undo.expect("checked Undo replay"),
            });
        }
        derive(&self.conn, request)
    }

    /// Admits an explicitly identified Undo without installing or moving files.
    pub fn begin_proposal_undo(&mut self, request: &UndoRequest) -> Result<ApplyJournal> {
        validate_request(request)?;
        let tx = self.conn.transaction()?;
        if let Some(journal) = replay(&tx, request)? {
            return Ok(journal);
        }
        let preview = derive(&tx, request)?;
        if proposals::read_proposal(&tx, request.operation_id)?.is_some() {
            return Err(Error::OperationConflict(
                "Undo proposal UUID already belongs to review work".into(),
            ));
        }
        proposal_apply::require_clear_apply_lane(&tx)?;
        actions::check_changes(&tx, &preview.draft.action_changes)?;
        let now = preview
            .draft
            .action_changes
            .iter()
            .fold(now_ms(), |time, change| match change {
                ActionChange::Replace { before, .. } => time.max(before.updated_at_ms),
                ActionChange::Create { .. } => time,
            });
        let approved = ProposalRecord {
            draft: preview.draft,
            version: 1,
            state: ProposalState::Draft,
            comments: Vec::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        let creation_sha256 = hash(&proposal_apply::encode(&approved.draft)?);
        let mut ids: HashSet<_> = preview
            .binding
            .originals
            .iter()
            .flatten()
            .map(|original| original.member_id)
            .collect();
        let members = approved
            .draft
            .changes
            .iter()
            .zip(&preview.binding.originals)
            .map(|(change, original)| {
                let id = if let Some(original) = original {
                    original.member_id
                } else {
                    loop {
                        let id = Uuid::new_v4();
                        if ids.insert(id) {
                            break id;
                        }
                    }
                };
                ApplyMember {
                    id,
                    staging: proposal_apply::stages_for(id, change),
                }
            })
            .collect();
        let action_records = proposal_apply::action_records(&approved, now)?;
        let journal = ApplyJournal {
            request: ApprovalRequest {
                operation_id: request.operation_id,
                expected: approved.stamp(),
            },
            approved: approved.clone(),
            creation_sha256,
            members,
            prepared: None,
            receipt: None,
            observations: None,
            no_effects: false,
            undo: Some(preview.binding),
            repair: None,
            started_at_ms: now,
            action_records,
        };
        journal.validate()?;
        let mut record = approved;
        record.state = ProposalState::Applying;
        proposals::advance(&mut record)?;
        proposal_apply::insert_review(
            &tx,
            &StoredProposal {
                creation_sha256,
                record,
            },
        )?;
        proposal_apply::insert_journal(&tx, &journal)?;
        tx.commit()?;
        Ok(journal)
    }
}
