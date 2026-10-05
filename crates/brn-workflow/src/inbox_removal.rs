//! Read-only qualification before explicit semantic approval and recoverable
//! original-copy removal. No namespace effects or inferred completeness.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    inbox::InboxOriginal,
    proposals::{NoteChange, ProposalSource, ProposalState},
};
pub use brn_store::work::inbox_removal::InboxRemovalSnapshot;
use brn_store::work::{
    chat::WorkTurnStatus, proposal_apply::ApplyOutcome, proposal_rewrite::RewriteStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxApprovedSource {
    pub operation_id: Uuid,
    pub proposal_id: Uuid,
    pub note_id: Uuid,
    pub saved: ProposalSource,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSavedConsequence {
    pub operation_id: Uuid,
    pub member_index: usize,
    pub saved: ProposalSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InboxRemovalBlocker {
    OriginalUnavailable,
    ConsequenceUnavailable {
        operation_id: Uuid,
        member_index: usize,
        reason: String,
    },
    SourceRequired,
    ProcessingPending {
        batch_id: Uuid,
        index: usize,
    },
    AnalysisUnsettled {
        analysis_id: Uuid,
    },
    ProposalUnsettled {
        proposal_id: Uuid,
    },
    ApprovalUnsettled {
        operation_id: Uuid,
    },
    RewriteRunning {
        rewrite_id: Uuid,
    },
    AppliedUndo {
        operation_id: Uuid,
    },
    SourceUnavailable {
        proposal_id: Uuid,
        reason: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalEvidence {
    pub snapshot: InboxRemovalSnapshot,
    pub original: InboxOriginal,
    pub sources: Vec<InboxApprovedSource>,
    pub saved_consequences: Vec<InboxSavedConsequence>,
    pub blockers: Vec<InboxRemovalBlocker>,
    /// Even an empty blockers list grants no semantic approval or removal.
    pub needs_owner_attestation: bool,
}
impl InboxRemovalEvidence {
    pub fn digest(&self) -> Result<[u8; 32]> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| WorkflowError::msg("could not encode original-removal evidence"))?;
        // Reserve outer digest/field metadata within the same complete 64 MiB cap.
        if bytes.len() > brn_store::work::inbox_review::MAX_INBOX_REVIEW_BYTES - 512 {
            return Err(WorkflowError::msg(
                "complete original-removal evidence exceeds its encoded bound",
            ));
        }
        Ok(Sha256::digest(bytes).into())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalPreview {
    pub evidence: InboxRemovalEvidence,
    pub digest: [u8; 32],
}
impl App {
    /// Exact full backend observation, never a durable approval. Confirmation
    /// must requalify this evidence and separately bind the owner's attestations.
    pub fn preview_inbox_removal(&mut self, id: Uuid) -> Result<InboxRemovalPreview> {
        let snapshot = self.store.inbox_removal_snapshot(id)?;
        let original = self.inbox.original(&snapshot.review.original);
        let mut blockers = Vec::new();
        if !matches!(original, InboxOriginal::Available { .. }) {
            blockers.push(InboxRemovalBlocker::OriginalUnavailable);
        }
        for batch in &snapshot.review.processing {
            for (index, item) in batch.request.items.iter().enumerate() {
                if item.capture.id == id && batch.entries[index].outcome.pending() {
                    blockers.push(InboxRemovalBlocker::ProcessingPending {
                        batch_id: batch.request.id,
                        index,
                    });
                }
            }
        }
        for a in &snapshot.review.analyses {
            if a.turn
                .as_ref()
                .is_none_or(|turn| turn.status != WorkTurnStatus::Completed)
            {
                blockers.push(InboxRemovalBlocker::AnalysisUnsettled {
                    analysis_id: a.job.capture.id,
                });
            }
        }
        for p in snapshot
            .review
            .proposals
            .iter()
            .chain(&snapshot.related_reviews)
        {
            if !matches!(
                p.record.state,
                ProposalState::Applied | ProposalState::Rejected
            ) {
                blockers.push(InboxRemovalBlocker::ProposalUnsettled {
                    proposal_id: p.record.draft.id,
                });
            }
        }
        for a in &snapshot.lineage {
            let outcome = a.receipt.as_ref().map(|r| r.outcome);
            if !matches!(
                outcome,
                Some(ApplyOutcome::Applied | ApplyOutcome::NotApplied)
            ) {
                blockers.push(InboxRemovalBlocker::ApprovalUnsettled {
                    operation_id: a.request.operation_id,
                });
            }
            if a.undo.is_some() && outcome == Some(ApplyOutcome::Applied) {
                blockers.push(InboxRemovalBlocker::AppliedUndo {
                    operation_id: a.request.operation_id,
                });
            }
        }
        for job in &snapshot.rewrites {
            if job.status == RewriteStatus::Running {
                blockers.push(InboxRemovalBlocker::RewriteRunning {
                    rewrite_id: job.spec.id,
                });
            }
        }
        let mut sources = Vec::new();
        for a in &snapshot.review.approvals {
            let Some(binding) = &a.approved.draft.inbox_source else {
                continue;
            };
            if a.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied) {
                continue;
            }
            let qualified = (|| -> Result<ProposalSource> {
                if binding.original != snapshot.review.original {
                    return Err(stale("approved Source belongs to a different capture"));
                }
                self.validate_inbox_source(Some(binding))?;
                let [NoteChange::Create { path, text, .. }] = a.approved.draft.changes.as_slice()
                else {
                    return Err(stale("approved Source is not one exact Create"));
                };
                let saved = self.qualify_inbox_saved_change(a, 0)?;
                if &saved.text != text {
                    return Err(stale("approved Source bytes changed"));
                }
                binding.validate_markdown(&saved.text)?;
                let resolution = self.resolve_note_identity(binding.note_id)?;
                if resolution.outcome != crate::knowledge::IdentityOutcome::Unique
                    || resolution.matches.len() != 1
                    || resolution.matches[0].path != *path
                {
                    return Err(stale(
                        "approved Source UUID is ambiguous or incompletely inspected",
                    ));
                }
                Ok(saved)
            })();
            match qualified {
                Ok(saved) => sources.push(InboxApprovedSource {
                    operation_id: a.request.operation_id,
                    proposal_id: a.approved.draft.id,
                    note_id: binding.note_id,
                    saved,
                }),
                Err(e) => blockers.push(InboxRemovalBlocker::SourceUnavailable {
                    proposal_id: a.approved.draft.id,
                    reason: e.message,
                }),
            }
        }
        if sources.is_empty() {
            blockers.push(InboxRemovalBlocker::SourceRequired);
        }
        let mut saved_consequences = Vec::new();
        for journal in &snapshot.lineage {
            if journal.undo.is_some()
                || journal.approved.draft.inbox_source.is_some()
                || journal.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied)
            {
                continue;
            }
            for index in 0..journal.approved.draft.changes.len() {
                match self.qualify_inbox_saved_change(journal, index) {
                    Ok(saved) => saved_consequences.push(InboxSavedConsequence {
                        operation_id: journal.request.operation_id,
                        member_index: index,
                        saved,
                    }),
                    Err(e) => blockers.push(InboxRemovalBlocker::ConsequenceUnavailable {
                        operation_id: journal.request.operation_id,
                        member_index: index,
                        reason: e.message,
                    }),
                }
            }
        }
        // Filesystem observations must not quietly mix with later operational
        // mutations. No lock spans caller review; eventual confirmation rereads.
        if self.store.inbox_removal_snapshot(id)?.digest()? != snapshot.digest()? {
            return Err(stale(
                "retained original-removal work changed during qualification",
            ));
        }
        let evidence = InboxRemovalEvidence {
            snapshot,
            original,
            sources,
            saved_consequences,
            blockers,
            needs_owner_attestation: true,
        };
        let digest = evidence.digest()?;
        Ok(InboxRemovalPreview { evidence, digest })
    }
    fn qualify_inbox_saved_change(
        &mut self,
        journal: &brn_store::work::proposal_apply::ApplyJournal,
        index: usize,
    ) -> Result<ProposalSource> {
        let change = &journal.approved.draft.changes[index];
        let Some(text) = change.text() else {
            return Err(stale(
                "removed knowledge needs a separately qualified retained-copy review",
            ));
        };
        self.editor_files()?;
        let current = self
            .store
            .setting("vault.editor_identity")?
            .ok_or_else(|| stale("current vault identity is missing"))?;
        let bound: brn_store::files::VaultRecord = serde_json::from_str(&current)
            .map_err(|_| stale("current vault identity is malformed"))?;
        if journal.approved.draft.vault.as_ref() != Some(&bound) {
            return Err(stale("approved outcome vault changed"));
        }
        let saved = self.proposal_evidence_source(change.path())?;
        let proof = journal
            .observations
            .as_ref()
            .and_then(|proofs| proofs.get(index))
            .and_then(|p| p.destination.as_ref())
            .ok_or_else(|| stale("approval has no terminal saved proof"))?;
        if &saved.source.fingerprint != proof || saved.text != text {
            return Err(stale("approved outcome bytes or identity changed"));
        }
        for editor in self.store.editors()? {
            if (editor.path == change.path()
                || (editor.baseline.device == proof.device && editor.baseline.inode == proof.inode))
                && editor.text != editor.baseline_text
            {
                return Err(stale("approved outcome has retained unsaved editor work"));
            }
        }
        if let Some(id) = brn_store::note_identity::read(&saved.text)? {
            let resolution = self.resolve_note_identity(id)?;
            if resolution.outcome != crate::knowledge::IdentityOutcome::Unique
                || resolution.matches.len() != 1
                || resolution.matches[0].path != change.path()
            {
                return Err(stale(
                    "approved outcome UUID is ambiguous or incompletely inspected",
                ));
            }
        }
        Ok(saved)
    }
}
fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{
        app::AppConfig,
        inbox::{CaptureInboxRequest, InboxKind},
        inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
        proposal_apply::{ApprovalRequest, UndoRequest},
    };
    use std::{fs, sync::atomic::AtomicBool};
    struct Fixture {
        _owner: tempfile::TempDir,
        data: std::path::PathBuf,
        vault: std::path::PathBuf,
        app: App,
        item: Uuid,
    }
    impl Fixture {
        fn new() -> Self {
            let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = owner.path().join("data");
            let vault = owner.path().join("vault");
            fs::create_dir(&data).unwrap();
            fs::create_dir(&vault).unwrap();
            let mut app = App::open(
                &data,
                AppConfig {
                    vault_root: Some(vault.clone()),
                    credentials_dir: Some(owner.path().join("credentials")),
                    model_dir: None,
                },
            )
            .unwrap();
            let item = app
                .capture_inbox(&CaptureInboxRequest {
                    id: Uuid::new_v4(),
                    kind: InboxKind::Email,
                    title: "Synthetic copy õ".into(),
                    original_name: None,
                    text: "\u{feff}Exact body 日本語\r\n\u{0001}".into(),
                })
                .unwrap();
            Self {
                _owner: owner,
                data,
                vault,
                app,
                item: item.capture.id,
            }
        }
        fn source(&mut self) -> Uuid {
            let item = self.app.inbox_item(self.item).unwrap().item;
            let batch = self
                .app
                .process_inbox(&ProcessInboxRequest {
                    id: Uuid::new_v4(),
                    items: vec![item],
                })
                .unwrap();
            self.app
                .advance_inbox_processing(batch.request.id, &AtomicBool::new(false))
                .unwrap();
            let draft = self
                .app
                .prepare_inbox_source(&InboxSourceRequest {
                    candidate: InboxCandidateRequest {
                        batch_id: batch.request.id,
                        index: 0,
                    },
                    proposal_id: Uuid::new_v4(),
                    note_id: Uuid::new_v4(),
                    path: "source.md".into(),
                    title: "Exact Source".into(),
                })
                .unwrap();
            let p = self.app.create_proposal(&draft).unwrap();
            let r = self
                .app
                .approve_proposal(&ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: p.stamp(),
                })
                .unwrap();
            assert_eq!(r.outcome, ApplyOutcome::Applied);
            r.operation_id
        }
    }
    #[test]
    fn conversion_and_applied_source_never_attest_semantic_completeness_or_remove_original() {
        let mut f = Fixture::new();
        let empty = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            empty
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::SourceRequired)
        );
        let original = f.data.join("inbox").join(format!("{}.txt", f.item));
        let bytes = fs::read(&original).unwrap();
        let operation = f.source();
        let preview = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            preview.evidence.blockers.is_empty(),
            "{:?}",
            preview.evidence.blockers
        );
        assert!(preview.evidence.needs_owner_attestation);
        assert_eq!(preview.evidence.sources.len(), 1);
        assert_eq!(preview.evidence.sources[0].operation_id, operation);
        assert_ne!(preview.digest, empty.digest);
        assert_eq!(
            preview.evidence.sources[0].saved.text.as_bytes(),
            fs::read(f.vault.join("source.md")).unwrap()
        );
        assert_eq!(fs::read(original).unwrap(), bytes);
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            preview.digest
        );
    }
    #[test]
    fn stale_duplicate_missing_source_and_changed_original_refuse_without_touching_bytes() {
        for mode in ["edit", "duplicate", "missing", "original"] {
            let mut f = Fixture::new();
            f.source();
            let before = f.app.preview_inbox_removal(f.item).unwrap();
            let source = f.vault.join("source.md");
            let exact = fs::read(&source).unwrap();
            let original = f.data.join("inbox").join(format!("{}.txt", f.item));
            match mode {
                "edit" => {
                    let mut changed = exact.clone();
                    changed.extend_from_slice(b"\nEdited");
                    fs::write(&source, changed).unwrap();
                }
                "duplicate" => fs::write(f.vault.join("duplicate.md"), &exact).unwrap(),
                "missing" => fs::rename(&source, f.vault.join("retained-copy")).unwrap(),
                "original" => fs::write(&original, b"changed synthetic original").unwrap(),
                _ => unreachable!(),
            }
            let bytes = fs::read(&original).unwrap();
            let preview = f.app.preview_inbox_removal(f.item).unwrap();
            assert!(!preview.evidence.blockers.is_empty(), "{mode}");
            assert!(preview.evidence.sources.is_empty(), "{mode}");
            assert_ne!(preview.digest, before.digest);
            assert_eq!(fs::read(original).unwrap(), bytes);
        }
    }
    #[test]
    fn pending_related_processing_and_undo_remain_explicit_blockers_in_complete_evidence() {
        let mut f = Fixture::new();
        let operation = f.source();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        let item = good.evidence.snapshot.review.original.clone();
        let batch = f
            .app
            .process_inbox(&ProcessInboxRequest {
                id: Uuid::new_v4(),
                items: vec![item.clone()],
            })
            .unwrap();
        let pending = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            pending
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::ProcessingPending {
                    batch_id: batch.request.id,
                    index: 0
                })
        );
        assert_ne!(pending.digest, good.digest);
        f.app.cancel_inbox_processing(batch.request.id).unwrap();
        let undo = f
            .app
            .undo_proposal(&UndoRequest {
                operation_id: Uuid::new_v4(),
                target_operation_id: operation,
                trash_member: None,
            })
            .unwrap();
        assert_eq!(undo.outcome, ApplyOutcome::Applied);
        let undone = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            undone
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::AppliedUndo {
                    operation_id: undo.operation_id
                })
        );
        assert!(
            undone
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::SourceRequired)
        );
        assert_eq!(undone.evidence.snapshot.lineage.len(), 2);
        assert!(fs::exists(item.capture.copy.directory.join(item.capture.copy_name())).unwrap());
    }

    #[test]
    fn changed_applied_consequence_is_a_visible_blocker() {
        let mut f = Fixture::new();
        f.source();
        let saved = f
            .app
            .preview_inbox_removal(f.item)
            .unwrap()
            .evidence
            .sources
            .remove(0)
            .saved;
        let draft = crate::proposals::DraftRequest {
            inbox_source: None,
            inbox_knowledge: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Selected durable consequence".into(),
            changes: vec![crate::proposals::DraftNoteChange::Create {
                path: "consequence.md".into(),
                text: "# Preserve this consequence 日本語\n".into(),
            }],
            sources: vec![saved.source],
            action_changes: vec![],
        };
        let proposal = f.app.create_proposal(&draft).unwrap();
        f.app
            .approve_proposal(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: proposal.stamp(),
            })
            .unwrap();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            good.evidence.blockers.is_empty(),
            "{:?}",
            good.evidence.blockers
        );
        assert_eq!(good.evidence.saved_consequences.len(), 1);
        let retained = f.vault.join("consequence-retained");
        fs::rename(f.vault.join("consequence.md"), &retained).unwrap();
        let missing = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            missing
                .evidence
                .blockers
                .iter()
                .any(|b| matches!(b, InboxRemovalBlocker::ConsequenceUnavailable { .. }))
        );
        assert_ne!(missing.digest, good.digest);
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn absent_running_and_failed_analysis_or_running_rewrite_cannot_qualify_as_completed() {
        use brn_store::work::{
            inbox_actions::InboxActionCapture,
            proposal_rewrite::{RewriteOutcome, RewriteSpec},
        };
        let mut f = Fixture::new();
        f.source();
        let source = f
            .app
            .preview_inbox_removal(f.item)
            .unwrap()
            .evidence
            .sources
            .remove(0)
            .saved;
        let capture = InboxActionCapture {
            purpose: Default::default(),
            id: Uuid::new_v4(),
            conversation: None,
            source: source.source.clone(),
            source_text: source.text,
            provider: "chatgpt".into(),
            model: "gpt-6-luna".into(),
            effort: "medium".into(),
        };
        let job = f
            .app
            .store
            .reserve_inbox_action(&capture, "Synthetic analysis; no provider route")
            .unwrap();
        let expected = InboxRemovalBlocker::AnalysisUnsettled {
            analysis_id: capture.id,
        };
        assert!(
            f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .blockers
                .contains(&expected)
        );
        f.app.store.begin_inbox_action_turn(&job).unwrap();
        assert!(
            f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .blockers
                .contains(&expected)
        );
        f.app
            .store
            .finish_turn(capture.id, WorkTurnStatus::Failed, "", Some("network"))
            .unwrap();
        assert!(
            f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .blockers
                .contains(&expected)
        );
        let draft = crate::proposals::DraftRequest {
            inbox_source: None,
            inbox_knowledge: None,
            id: Uuid::new_v4(),
            group_id: Some(capture.id),
            session_id: None,
            title: "Related review".into(),
            changes: vec![crate::proposals::DraftNoteChange::Create {
                path: "review.md".into(),
                text: "# Candidate".into(),
            }],
            sources: vec![source.source],
            action_changes: vec![],
        };
        let p = f.app.create_proposal(&draft).unwrap();
        let spec = RewriteSpec {
            id: Uuid::new_v4(),
            expected: p.stamp(),
            provider: "chatgpt".into(),
            model: "gpt-6-luna".into(),
            effort: "medium".into(),
        };
        f.app.store.begin_proposal_rewrite(&spec).unwrap();
        f.app.store.reject_proposal(p.stamp()).unwrap();
        assert!(
            f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::RewriteRunning {
                    rewrite_id: spec.id
                })
        );
        f.app
            .store
            .finish_proposal_rewrite(spec.id, &RewriteOutcome::Interrupted)
            .unwrap();
        assert!(
            !f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::RewriteRunning {
                    rewrite_id: spec.id
                })
        );
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn retained_unsaved_source_editor_blocks_even_when_approved_file_is_unchanged() {
        let mut f = Fixture::new();
        f.source();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        let saved = fs::read(f.vault.join("source.md")).unwrap();
        let editor = f.app.open_editor("source.md").unwrap().record;
        f.app
            .recover_editor(&crate::editor::EditRequest {
                path: editor.path,
                expected: editor.stamp,
                generation: editor.stamp.generation + 1,
                text: editor.text + "\nUnfinished review",
            })
            .unwrap();
        let preview = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(preview.evidence.blockers.iter().any(|b|matches!(b,InboxRemovalBlocker::SourceUnavailable{reason,..} if reason.contains("unsaved editor"))));
        assert_ne!(preview.digest, good.digest);
        assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), saved);
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn not_applied_undo_draft_and_later_review_cannot_disappear_from_qualification() {
        let mut f = Fixture::new();
        let applied = f.source();
        let op = Uuid::new_v4();
        f.app
            .store
            .begin_proposal_undo(&UndoRequest {
                operation_id: op,
                target_operation_id: applied,
                trash_member: None,
            })
            .unwrap();
        f.app
            .store
            .refuse_proposal_before_effects(op, None)
            .unwrap();
        let proposal = f.app.store.proposal(op).unwrap().unwrap();
        assert_eq!(proposal.state, ProposalState::Draft);
        let preview = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(
            preview
                .evidence
                .blockers
                .contains(&InboxRemovalBlocker::ProposalUnsettled { proposal_id: op }),
            "related Undo Draft was omitted: {:?}",
            preview.evidence.blockers
        );
        f.app
            .store
            .edit_proposal(&crate::proposals::ProposalEdit {
                expected: proposal.stamp(),
                title: proposal.draft.title + " reviewed",
                texts: vec![None],
                action_data: vec![],
            })
            .unwrap();
        assert_ne!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            preview.digest,
            "Undo review revision was omitted"
        );
    }
}
