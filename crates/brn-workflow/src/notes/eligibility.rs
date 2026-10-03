//! One live-note boundary for every current-source and provider surface.
use super::{NoteAvailability, NoteErrorCode, NoteResult, NoteSearchReceipt, note_failure};
use crate::{ErrorKind, Result, SourceDocument, WorkflowError, Workspace, digest, error};
use brn_retrieval::Evidence;
use brn_store::{Approval, notes::NoteSearchRequest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceCurrentState {
    Current,
    Shadowed,
    Changed,
    Missing,
    Unavailable,
    OwnedElsewhere,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceStateSummary {
    pub source_id: Uuid,
    pub title: String,
    pub note_id: Option<Uuid>,
    pub current_state: SourceCurrentState,
    pub message: Option<String>,
}

pub(crate) fn note_workflow_error(e: super::NoteFailure) -> WorkflowError {
    WorkflowError {
        kind: match e.code {
            NoteErrorCode::OperationConflict => ErrorKind::OperationConflict,
            NoteErrorCode::WorkspaceBusy => ErrorKind::WorkspaceBusy,
            NoteErrorCode::Storage => ErrorKind::Other,
            _ => ErrorKind::EvidenceStale,
        },
        message: e.message,
    }
}

pub(crate) fn stale(message: impl Into<String>) -> WorkflowError {
    WorkflowError {
        kind: ErrorKind::EvidenceStale,
        message: message.into(),
    }
}

impl Workspace {
    pub fn approve_note_snapshot(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected_file_state: Uuid,
    ) -> NoteResult<NoteSearchReceipt> {
        self.note_snapshot_operation(
            op,
            &NoteSearchRequest::Approve {
                note_id: id,
                file_state: expected_file_state,
            },
        )
        .map(|(receipt, _)| receipt)
    }

    pub(crate) fn note_snapshot_operation(
        &mut self,
        op: Uuid,
        request: &NoteSearchRequest,
    ) -> NoteResult<(NoteSearchReceipt, bool)> {
        let result = (|| {
            if let Some(result) = self.store.note_search_replay(op, request)? {
                return Ok(result);
            }
            let id = request.note_id();
            let view = self.note(id)?;
            if view.availability != NoteAvailability::Available {
                return Err(note_failure(
                    NoteErrorCode::StateChanged,
                    view.availability_message
                        .unwrap_or_else(|| "reconcile the saved note before approval".into()),
                ));
            }
            let record = self.store.note_record(id)?;
            let (state, file) = record
                .observed
                .ok_or_else(|| note_failure(NoteErrorCode::Storage, "note observation missing"))?;
            if let NoteSearchRequest::SetApproval {
                source_id,
                version_id,
                ..
            } = request
            {
                let snapshot = self.store.note_search_snapshot(id)?.ok_or_else(|| {
                    note_failure(NoteErrorCode::Storage, "managed snapshot missing")
                })?;
                if snapshot.receipt.source_id != *source_id
                    || snapshot.receipt.version_id != *version_id
                    || snapshot.fingerprint != file
                {
                    return Err(note_failure(
                        NoteErrorCode::StateChanged,
                        "old snapshot cannot approve new saved bytes; explicitly approve the current note",
                    ));
                }
            }
            self.store.freeze_note_search_snapshot(
                op,
                request,
                state,
                &file,
                view.saved.as_deref().ok_or_else(|| {
                    note_failure(NoteErrorCode::Storage, "saved observation missing")
                })?,
            )
        })();
        result.map_err(|mut e: super::NoteFailure| {
            e.operation_id.get_or_insert(op);
            e.note_id.get_or_insert(request.note_id());
            e
        })
    }

    pub(crate) fn managed_note_for_path(&self, path: &Path) -> Result<Option<Uuid>> {
        let Some(vault) = self.store.registered_vault().map_err(note_workflow_error)? else {
            return Ok(None);
        };
        for recovery in self.store.note_recoveries().map_err(note_workflow_error)? {
            let record = self
                .store
                .note_record(recovery.note_id)
                .map_err(note_workflow_error)?;
            let registered = vault.root.join(record.relative_path);
            if registered == path || std::fs::canonicalize(&registered).is_ok_and(|p| p == path) {
                return Ok(Some(record.id));
            }
        }
        let associated = self
            .store
            .note_source_associations()
            .map_err(note_workflow_error)?;
        if self.store.documents().map_err(error)?.iter().any(|doc| {
            Path::new(&doc.origin) == path
                && associated
                    .iter()
                    .any(|(_, source, shadowed)| *source == doc.source_id && *shadowed)
        }) {
            return Err(stale(
                "origin belongs to a shadowed historical import; use the managed note at its reconciled location",
            ));
        }
        Ok(None)
    }

    /// Observes each enrolled note once and never returns excluded snapshot bytes.
    pub fn source_projection(&mut self) -> Result<(Vec<SourceDocument>, Vec<SourceStateSummary>)> {
        self.store
            .reconcile_note_sources()
            .map_err(note_workflow_error)?;
        let associations = self
            .store
            .note_source_associations()
            .map_err(note_workflow_error)?;
        let mut live = BTreeMap::new();
        for (id, _, _) in &associations {
            if !live.contains_key(id) {
                live.insert(*id, self.note(*id).map_err(note_workflow_error)?);
            }
        }
        let mut current = Vec::new();
        let mut states = Vec::new();
        for mut doc in self.store.documents().map_err(error)? {
            let matches: Vec<_> = associations
                .iter()
                .filter(|(_, source, _)| *source == doc.source_id)
                .collect();
            let mut summary = SourceStateSummary {
                source_id: doc.source_id,
                title: doc.title.clone(),
                note_id: None,
                current_state: SourceCurrentState::Current,
                message: None,
            };
            if matches.len() > 1 {
                summary.current_state = SourceCurrentState::Uncertain;
                summary.message = Some(
                    "ambiguous registered-path associations; explicit reconciliation required"
                        .into(),
                );
            } else if let Some((id, _, shadowed)) = matches.first() {
                summary.note_id = Some(*id);
                let view = &live[id];
                if *shadowed {
                    summary.current_state = SourceCurrentState::Shadowed;
                    summary.message = Some(format!(
                        "historical imported copy; use managed note {id} ({:?})",
                        view.availability
                    ));
                } else {
                    summary.current_state = match view.availability {
                        NoteAvailability::Available => SourceCurrentState::Current,
                        NoteAvailability::Missing => SourceCurrentState::Missing,
                        NoteAvailability::Conflict => {
                            if self
                                .store
                                .note_original_save_blocker(*id)
                                .map_err(note_workflow_error)?
                                .is_some()
                            {
                                SourceCurrentState::Uncertain
                            } else {
                                SourceCurrentState::Changed
                            }
                        }
                        NoteAvailability::Uncertain => SourceCurrentState::Uncertain,
                        NoteAvailability::OwnedElsewhere => SourceCurrentState::OwnedElsewhere,
                        NoteAvailability::Unsupported | NoteAvailability::Unavailable => {
                            SourceCurrentState::Unavailable
                        }
                    };
                    summary.message = view.availability_message.clone();
                    if summary.current_state == SourceCurrentState::Current {
                        let snapshot = self
                            .store
                            .note_search_snapshot(*id)
                            .map_err(note_workflow_error)?
                            .ok_or_else(|| error("managed source lacks snapshot association"))?;
                        let record = self.store.note_record(*id).map_err(note_workflow_error)?;
                        if snapshot.receipt.source_id != doc.source_id
                            || snapshot.receipt.version_id != doc.version_id
                            || snapshot.fingerprint.sha256 != doc.sha256
                        {
                            return Err(error("managed source snapshot integrity mismatch"));
                        }
                        if record.observed.as_ref().map(|(_, file)| file)
                            != Some(&snapshot.fingerprint)
                        {
                            summary.current_state = SourceCurrentState::Changed;
                            summary.message = Some("saved file differs from search snapshot; explicitly approve and rebuild".into());
                        } else {
                            doc.approval = view.search_approval;
                            if doc.approval != Approval::Approved {
                                summary.message =
                                    Some("saved snapshot is not approved for search".into());
                            }
                        }
                    }
                }
            }
            if summary.current_state == SourceCurrentState::Current {
                current.push(doc);
            }
            states.push(summary);
        }
        Ok((current, states))
    }

    pub fn source_states(&mut self) -> Result<Vec<SourceStateSummary>> {
        self.source_projection().map(|(_, states)| states)
    }

    pub fn sources(&mut self) -> Result<Vec<SourceDocument>> {
        let (docs, states) = self.source_projection()?;
        if let Some(state) = states
            .iter()
            .find(|s| s.current_state != SourceCurrentState::Current)
        {
            return Err(stale(format!(
                "source {} is {:?}: {}",
                state.source_id,
                state.current_state,
                state.message.as_deref().unwrap_or("not current")
            )));
        }
        Ok(docs)
    }

    pub(crate) fn validate_current_note_evidence(&mut self, evidence: &Evidence) -> Result<()> {
        let source = Uuid::parse_str(&evidence.source_id).map_err(error)?;
        let version = Uuid::parse_str(&evidence.version_id).map_err(error)?;
        let (docs, states) = self.source_projection()?;
        if let Some(state) = states.iter().find(|s| {
            s.source_id == source
                && (s.note_id.is_some() || s.current_state != SourceCurrentState::Current)
        }) && (state.current_state != SourceCurrentState::Current
            || !docs.iter().any(|d| {
                d.source_id == source && d.version_id == version && d.approval == Approval::Approved
            }))
        {
            return Err(stale(format!(
                "managed evidence {} is no longer current and approved: {:?}; {}",
                source,
                state.current_state,
                state.message.as_deref().unwrap_or("snapshot superseded")
            )));
        }
        Ok(())
    }

    pub(crate) fn evidence_epochs(
        &self,
        source_ids: impl IntoIterator<Item = Uuid>,
    ) -> Result<Vec<(Uuid, u64, u64)>> {
        let associations = self
            .store
            .note_source_associations()
            .map_err(note_workflow_error)?;
        let mut epochs = BTreeMap::new();
        for source in source_ids {
            for (note, id, _) in &associations {
                if *id == source {
                    epochs.insert(
                        *note,
                        self.store
                            .note_evidence_epochs(*note)
                            .map_err(note_workflow_error)?,
                    );
                }
            }
        }
        Ok(epochs.into_iter().map(|(id, (c, a))| (id, c, a)).collect())
    }

    pub(crate) fn eligibility_fingerprint(
        &self,
        docs: &[brn_retrieval::Document],
    ) -> Result<String> {
        let ids = docs
            .iter()
            .map(|d| Uuid::parse_str(&d.source_id).map_err(error))
            .collect::<Result<Vec<_>>>()?;
        serde_json::to_vec(&self.evidence_epochs(ids)?)
            .map(|bytes| digest(&bytes))
            .map_err(error)
    }
}
