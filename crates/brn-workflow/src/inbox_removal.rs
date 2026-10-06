//! Read-only exact Source preservation qualification before explicit owner
//! confirmation and recoverable original-copy removal. No namespace effects.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    inbox::{InboxItem, InboxOriginal},
    proposals::ProposalSource,
};
use brn_store::work::{inbox_source::InboxSourcePreservation, proposal_apply::ApplyJournal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// Three complete bounded texts (original, approved Source, saved Source), each
// with worst-case six-byte JSON escaping, plus fixed journal/metadata overhead.
const MAX_PRESERVATION_EVIDENCE_BYTES: usize = 18 * crate::MAX_NOTE_BYTES + 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxApprovedSource {
    pub approval: ApplyJournal,
    pub saved: ProposalSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InboxRemovalBlocker {
    OriginalUnavailable,
    SourceRequired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalEvidence {
    pub item: InboxItem,
    pub original: InboxOriginal,
    /// One selected, fully checked approval and its freshly observed Source.
    /// Alternatives and derived work confer no cleanup authority.
    pub source: Option<InboxApprovedSource>,
    pub blockers: Vec<InboxRemovalBlocker>,
    /// A qualified preview never grants confirmation or removal authority.
    pub needs_owner_confirmation: bool,
}
impl InboxRemovalEvidence {
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.item.validate()?;
        let mut blockers = Vec::new();
        match &self.original {
            InboxOriginal::Available { text } => {
                if text.len() as u64 != self.item.capture.copy.byte_len
                    || <[u8; 32]>::from(Sha256::digest(text.as_bytes()))
                        != self.item.capture.copy.sha256
                {
                    return Err(stale("original bytes differ from their retained capture"));
                }
                if let Some(source) = &self.source {
                    InboxSourcePreservation {
                        original: &self.item,
                        original_text: text,
                        approval: &source.approval,
                        saved: &source.saved.source,
                        saved_text: &source.saved.text,
                    }
                    .validate()?;
                }
            }
            _ => {
                blockers.push(InboxRemovalBlocker::OriginalUnavailable);
                if self.source.is_some() {
                    return Err(stale(
                        "Source qualification requires the exact available original",
                    ));
                }
            }
        }
        if self.source.is_none() {
            blockers.push(InboxRemovalBlocker::SourceRequired);
        }
        if !self.needs_owner_confirmation || self.blockers != blockers {
            return Err(stale(
                "original-removal evidence has inconsistent admission fields",
            ));
        }
        let bytes = serde_json::to_vec(self)
            .map_err(|_| WorkflowError::msg("could not encode original-removal evidence"))?;
        if bytes.len() > MAX_PRESERVATION_EVIDENCE_BYTES {
            return Err(WorkflowError::msg(
                "complete Source-preservation evidence exceeds its encoded bound",
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
    /// Select one approved Source still preserving the complete exact original.
    /// Pending/failed analysis, drafts and later derived work are independent.
    /// Confirmation must bind and freshly recheck this same selected witness.
    pub fn preview_inbox_removal(&mut self, id: Uuid) -> Result<InboxRemovalPreview> {
        self.require_current_evidence()?;
        let item = self
            .store
            .inbox_item(id)?
            .ok_or_else(|| stale("retained original catalog entry is missing"))?;
        let original = self.inbox.original(&item);
        let mut source = None;
        if let InboxOriginal::Available { text } = &original {
            for operation_id in self.store.inbox_source_approval_ids(id)? {
                let approval = self
                    .store
                    .proposal_apply(operation_id)?
                    .ok_or_else(|| stale("listed Source approval disappeared"))?;
                match self.qualify_inbox_preservation(&item, text, &approval) {
                    Ok(saved) => {
                        source = Some(InboxApprovedSource { approval, saved });
                        break;
                    }
                    // Another stale Source is not a veto on a valid witness. Actual
                    // pending Save/Apply authority remains a global error below.
                    Err(error) if error.kind == ErrorKind::SaveUncertain => return Err(error),
                    Err(_) => {}
                }
            }
        }
        self.require_current_evidence()?;
        if self.store.inbox_item(id)?.as_ref() != Some(&item)
            || self.inbox.original(&item) != original
        {
            return Err(stale("retained original changed during qualification"));
        }
        if let Some(selected) = &source {
            let current = self
                .store
                .proposal_apply(selected.approval.request.operation_id)?
                .ok_or_else(|| stale("selected Source approval disappeared"))?;
            let InboxOriginal::Available { text } = &original else {
                unreachable!()
            };
            if current != selected.approval
                || self.qualify_inbox_preservation(&item, text, &current)? != selected.saved
            {
                return Err(stale(
                    "selected Source preservation changed during qualification",
                ));
            }
        }
        let mut blockers = Vec::new();
        if !matches!(original, InboxOriginal::Available { .. }) {
            blockers.push(InboxRemovalBlocker::OriginalUnavailable);
        }
        if source.is_none() {
            blockers.push(InboxRemovalBlocker::SourceRequired);
        }
        let evidence = InboxRemovalEvidence {
            item,
            original,
            source,
            blockers,
            needs_owner_confirmation: true,
        };
        let digest = evidence.digest()?;
        Ok(InboxRemovalPreview { evidence, digest })
    }

    fn qualify_inbox_preservation(
        &mut self,
        item: &InboxItem,
        original_text: &str,
        approval: &ApplyJournal,
    ) -> Result<ProposalSource> {
        let binding = approval
            .approved
            .draft
            .inbox_source
            .as_ref()
            .ok_or_else(|| stale("approval has no bound Inbox Source"))?;
        self.editor_files()?;
        let current = self
            .store
            .setting("vault.editor_identity")?
            .ok_or_else(|| stale("current vault identity is missing"))?;
        let bound: brn_store::files::VaultRecord = serde_json::from_str(&current)
            .map_err(|_| stale("current vault identity is malformed"))?;
        if approval.approved.draft.vault.as_ref() != Some(&bound) {
            return Err(stale("approved Source vault changed"));
        }
        let resolution = self.resolve_note_identity(binding.note_id)?;
        if resolution.outcome != crate::knowledge::IdentityOutcome::Unique
            || resolution.matches.len() != 1
        {
            return Err(stale(
                "approved Source UUID is absent, ambiguous or incompletely inspected",
            ));
        }
        let saved = self.proposal_evidence_source(&resolution.matches[0].path)?;
        if saved.source.fingerprint.sha256 != resolution.matches[0].sha256 {
            return Err(stale("saved Source changed during identity resolution"));
        }
        InboxSourcePreservation {
            original: item,
            original_text,
            approval,
            saved: &saved.source,
            saved_text: &saved.text,
        }
        .validate()?;
        Ok(saved)
    }
}
fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) mod tests {
    use super::*;
    use crate::{
        app::AppConfig,
        inbox::{CaptureInboxRequest, InboxKind},
        inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
        proposal_apply::{ApprovalRequest, UndoRequest},
        proposals::ProposalState,
    };
    use brn_store::work::{chat::WorkTurnStatus, proposal_apply::ApplyOutcome};
    use std::{fs, sync::atomic::AtomicBool};
    pub(crate) struct Fixture {
        pub(crate) _owner: tempfile::TempDir,
        pub(crate) data: std::path::PathBuf,
        pub(crate) vault: std::path::PathBuf,
        pub(crate) app: App,
        pub(crate) item: Uuid,
    }
    impl Fixture {
        pub(crate) fn new() -> Self {
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
        pub(crate) fn source(&mut self) -> Uuid {
            self.source_at("source.md")
        }
        pub(crate) fn source_at(&mut self, path: &str) -> Uuid {
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
                    path: path.into(),
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
    fn conversion_and_applied_source_still_require_confirmation_and_never_remove_original() {
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
        assert!(preview.evidence.needs_owner_confirmation);
        assert_eq!(usize::from(preview.evidence.source.is_some()), 1);
        assert_eq!(
            preview
                .evidence
                .source
                .as_ref()
                .unwrap()
                .approval
                .request
                .operation_id,
            operation
        );
        assert_ne!(preview.digest, empty.digest);
        assert_eq!(
            preview
                .evidence
                .source
                .as_ref()
                .unwrap()
                .saved
                .text
                .as_bytes(),
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
            assert!(preview.evidence.source.is_none(), "{mode}");
            assert_ne!(preview.digest, before.digest);
            assert_eq!(fs::read(original).unwrap(), bytes);
        }
    }
    #[test]
    fn pending_processing_is_independent_but_undo_removing_the_only_source_refuses() {
        let mut f = Fixture::new();
        let operation = f.source();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        let item = good.evidence.item.clone();
        let batch = f
            .app
            .process_inbox(&ProcessInboxRequest {
                id: Uuid::new_v4(),
                items: vec![item.clone()],
            })
            .unwrap();
        let pending = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(pending.evidence.blockers.is_empty());
        assert_eq!(pending.digest, good.digest);
        f.app.cancel_inbox_processing(batch.request.id).unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
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
        assert_eq!(
            undone.evidence.blockers,
            vec![InboxRemovalBlocker::SourceRequired]
        );
        assert!(undone.evidence.source.is_none());
        assert!(fs::exists(item.capture.copy.directory.join(item.capture.copy_name())).unwrap());
    }

    #[test]
    fn changed_applied_consequence_does_not_change_selected_preservation() {
        let mut f = Fixture::new();
        f.source();
        let saved = f
            .app
            .preview_inbox_removal(f.item)
            .unwrap()
            .evidence
            .source
            .unwrap()
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
        let retained = f.vault.join("consequence-retained");
        fs::rename(f.vault.join("consequence.md"), &retained).unwrap();
        let missing = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(missing.evidence.blockers.is_empty());
        assert_eq!(missing.digest, good.digest);
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn absent_running_failed_analysis_pending_drafts_and_rewrite_do_not_block_preservation() {
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
            .source
            .unwrap()
            .saved;
        let good = f.app.preview_inbox_removal(f.item).unwrap();
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
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
        assert!(good.evidence.blockers.is_empty());
        f.app.store.begin_inbox_action_turn(&job).unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
        f.app
            .store
            .finish_turn(capture.id, WorkTurnStatus::Failed, "", Some("network"))
            .unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
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
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
        f.app
            .store
            .finish_proposal_rewrite(spec.id, &RewriteOutcome::Interrupted)
            .unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn retained_unsaved_editor_is_not_saved_preservation_authority() {
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
        assert!(preview.evidence.blockers.is_empty());
        assert_eq!(preview.digest, good.digest);
        assert_eq!(
            f.app.store.editor("source.md").unwrap().unwrap().text,
            good.evidence.source.as_ref().unwrap().saved.text.clone() + "\nUnfinished review"
        );
        assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), saved);
        fs::write(
            f.vault.join("source.md"),
            b"Saved Source no longer preserves anything",
        )
        .unwrap();
        assert!(
            f.app
                .preview_inbox_removal(f.item)
                .unwrap()
                .evidence
                .source
                .is_none()
        );
        assert!(
            f.app
                .store
                .editor("source.md")
                .unwrap()
                .unwrap()
                .text
                .ends_with("Unfinished review")
        );
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }
    #[test]
    fn not_applied_undo_and_later_review_do_not_change_preservation() {
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
        assert!(preview.evidence.blockers.is_empty());
        f.app
            .store
            .edit_proposal(&crate::proposals::ProposalEdit {
                expected: proposal.stamp(),
                title: proposal.draft.title + " reviewed",
                texts: vec![None],
                action_data: vec![],
            })
            .unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            preview.digest
        );
    }
    #[test]
    fn owner_save_header_history_archive_and_restart_still_prove_the_exact_body() {
        use crate::editor::{EditRequest, SaveOutcome, SaveRequest};
        let mut f = Fixture::new();
        let operation = f.source();
        let before = f.app.preview_inbox_removal(f.item).unwrap();
        let old = before.evidence.source.as_ref().unwrap();
        let body = brn_store::note_identity::body_start(&old.saved.text).unwrap();
        let edited_header = old.saved.text[..body]
            .replace("brn_state: current", "brn_state: history")
            .replace("---\n", "---\ncustom: owner metadata 日本語\n")
            .replace('\n', "\r\n");
        // Remove the closing-delimiter custom field introduced by replace, keeping
        // only unrelated metadata inside the outer header and exact body bytes.
        let edited_header = edited_header
            .strip_suffix("custom: owner metadata 日本語\r\n")
            .unwrap();
        let text = format!("\u{feff}{edited_header}{}", &old.saved.text[body..]);
        let editor = f.app.open_editor("source.md").unwrap().record;
        let receipt = f
            .app
            .save_editor(&SaveRequest {
                operation_id: Uuid::new_v4(),
                edit: EditRequest {
                    path: editor.path,
                    expected: editor.stamp,
                    generation: editor.stamp.generation + 1,
                    text: text.clone(),
                },
                destination: None,
            })
            .unwrap();
        assert_eq!(receipt.outcome, SaveOutcome::Applied);
        let after = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(after.evidence.blockers.is_empty());
        let source = after.evidence.source.as_ref().unwrap();
        assert_eq!(source.approval.request.operation_id, operation);
        assert_eq!(source.approval, old.approval);
        assert_ne!(
            source.saved.source.fingerprint.inode,
            old.saved.source.fingerprint.inode
        );
        assert_eq!(source.saved.text, text);
        assert_ne!(after.digest, before.digest);
        fs::create_dir(f.vault.join("archive")).unwrap();
        fs::rename(f.vault.join("source.md"), f.vault.join("archive/moved.md")).unwrap();
        let moved = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(moved.evidence.blockers.is_empty());
        assert_eq!(
            moved.evidence.source.as_ref().unwrap().saved.source.path,
            "archive/moved.md"
        );
        assert_ne!(moved.digest, after.digest);
        drop(f.app);
        let mut app = App::open(
            &f.data,
            AppConfig {
                vault_root: Some(f.vault),
                credentials_dir: Some(f._owner.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            app.preview_inbox_removal(f.item).unwrap().digest,
            moved.digest
        );
        assert!(
            f.data
                .join("inbox")
                .join(format!("{}.txt", f.item))
                .exists()
        );
    }

    #[test]
    fn one_valid_source_qualifies_even_when_an_alternative_is_stale() {
        let mut f = Fixture::new();
        f.source_at("first.md");
        let second = f.source_at("second.md");
        fs::write(f.vault.join("first.md"), "stale alternative").unwrap();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(good.evidence.blockers.is_empty());
        assert_eq!(
            good.evidence
                .source
                .as_ref()
                .unwrap()
                .approval
                .request
                .operation_id,
            second
        );
        fs::write(f.vault.join("first.md"), "different stale alternative").unwrap();
        assert_eq!(
            f.app.preview_inbox_removal(f.item).unwrap().digest,
            good.digest
        );
        fs::write(f.vault.join("second.md"), "changed selected body").unwrap();
        let refused = f.app.preview_inbox_removal(f.item).unwrap();
        assert_eq!(
            refused.evidence.blockers,
            vec![InboxRemovalBlocker::SourceRequired]
        );
        assert!(refused.evidence.source.is_none());
    }

    #[test]
    fn identity_provenance_classification_and_complete_inventory_remain_required() {
        for mode in [
            "identity",
            "provenance",
            "kind",
            "state",
            "oversized_inventory",
            "unreadable_inventory",
        ] {
            let mut f = Fixture::new();
            f.source();
            let good = f.app.preview_inbox_removal(f.item).unwrap();
            let path = f.vault.join("source.md");
            let text = fs::read_to_string(&path).unwrap();
            match mode {
                "identity" => {
                    let id = brn_store::note_identity::read(&text).unwrap().unwrap();
                    fs::write(
                        &path,
                        text.replace(&id.to_string(), &Uuid::new_v4().to_string()),
                    )
                    .unwrap();
                }
                "provenance" => {
                    fs::write(&path, text.replace("Synthetic copy õ", "Another capture")).unwrap()
                }
                "kind" => fs::write(
                    &path,
                    text.replace("brn_kind: source", "brn_kind: knowledge"),
                )
                .unwrap(),
                "state" => fs::write(
                    &path,
                    text.replace("brn_state: current", "brn_state: guessed"),
                )
                .unwrap(),
                "oversized_inventory" => fs::write(
                    f.vault.join("unknown.md"),
                    vec![b'x'; crate::MAX_NOTE_BYTES + 1],
                )
                .unwrap(),
                "unreadable_inventory" => fs::write(f.vault.join("unknown.md"), [0xff]).unwrap(),
                _ => unreachable!(),
            }
            let preview = f.app.preview_inbox_removal(f.item).unwrap();
            assert!(preview.evidence.source.is_none(), "{mode}");
            assert_eq!(
                preview.evidence.blockers,
                vec![InboxRemovalBlocker::SourceRequired],
                "{mode}"
            );
            assert_ne!(preview.digest, good.digest);
            assert!(
                f.data
                    .join("inbox")
                    .join(format!("{}.txt", f.item))
                    .exists()
            );
        }
    }

    #[test]
    fn interrupted_save_or_apply_fences_preservation_and_never_becomes_a_pending_draft_gate() {
        use crate::editor::{EditRequest, SaveRequest};
        for mode in ["save", "apply"] {
            let mut f = Fixture::new();
            f.source();
            let good = f.app.preview_inbox_removal(f.item).unwrap();
            if mode == "save" {
                let editor = f.app.open_editor("source.md").unwrap().record;
                let request = SaveRequest {
                    operation_id: Uuid::new_v4(),
                    edit: EditRequest {
                        path: editor.path,
                        expected: editor.stamp,
                        generation: editor.stamp.generation + 1,
                        text: editor.text + "\nPending Save",
                    },
                    destination: None,
                };
                let stage =
                    std::path::PathBuf::from(format!(".brn-{}.stage", request.operation_id));
                f.app.store.begin_editor_save(&request, &stage).unwrap();
            } else {
                let source = good.evidence.source.as_ref().unwrap().saved.source.clone();
                let draft = crate::proposals::DraftRequest {
                    inbox_source: None,
                    inbox_knowledge: None,
                    id: Uuid::new_v4(),
                    group_id: None,
                    session_id: None,
                    title: "Pending consequence".into(),
                    changes: vec![crate::proposals::DraftNoteChange::Create {
                        path: "later.md".into(),
                        text: "Later".into(),
                    }],
                    sources: vec![source],
                    action_changes: vec![],
                };
                let p = f.app.create_proposal(&draft).unwrap();
                assert_eq!(
                    f.app.preview_inbox_removal(f.item).unwrap().digest,
                    good.digest
                );
                f.app
                    .store
                    .begin_proposal_apply(&ApprovalRequest {
                        operation_id: Uuid::new_v4(),
                        expected: p.stamp(),
                    })
                    .unwrap();
            }
            assert_eq!(
                f.app.preview_inbox_removal(f.item).unwrap_err().kind,
                ErrorKind::SaveUncertain,
                "{mode}"
            );
            assert!(
                f.data
                    .join("inbox")
                    .join(format!("{}.txt", f.item))
                    .exists()
            );
        }
    }

    #[test]
    fn client_evidence_digest_rejects_forged_readiness_and_saved_proofs() {
        let mut f = Fixture::new();
        f.source();
        let good = f.app.preview_inbox_removal(f.item).unwrap();
        let mut e = good.evidence.clone();
        e.needs_owner_confirmation = false;
        assert!(e.digest().is_err());
        let mut e = good.evidence.clone();
        e.blockers.push(InboxRemovalBlocker::SourceRequired);
        assert!(e.digest().is_err());
        let mut e = good.evidence.clone();
        e.source.as_mut().unwrap().saved.text.push('x');
        assert!(e.digest().is_err());
        let mut e = good.evidence;
        e.original = InboxOriginal::Missing;
        assert!(e.digest().is_err());
    }
    #[test]
    fn qualified_source_preview_matches_the_lean_store_record_without_extra_admission_gates() {
        use brn_store::work::inbox_original_operations::InboxQualifiedRemovalEvidence;

        let mut f = Fixture::new();
        f.source();
        let preview = f.app.preview_inbox_removal(f.item).unwrap();
        let encoded = serde_json::to_vec(&preview.evidence).unwrap();
        let qualified: InboxQualifiedRemovalEvidence = serde_json::from_slice(&encoded).unwrap();
        qualified.validate().unwrap();
        assert_eq!(qualified.digest().unwrap(), preview.digest);
        assert_eq!(serde_json::to_vec(&qualified).unwrap(), encoded);

        let mut changed = serde_json::to_value(&qualified).unwrap();
        changed["needs_owner_confirmation"] = serde_json::json!(false);
        let changed: InboxQualifiedRemovalEvidence = serde_json::from_value(changed).unwrap();
        assert!(changed.validate().is_err());

        let mut f = Fixture::new();
        let blocked = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(!blocked.evidence.blockers.is_empty());
        assert!(
            serde_json::from_value::<InboxQualifiedRemovalEvidence>(
                serde_json::to_value(blocked.evidence).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn worst_case_escaped_original_and_sources_remain_complete_within_fixed_evidence_bound() {
        let mut f = Fixture::new();
        let text = "\u{0001}".repeat(crate::MAX_NOTE_BYTES - 4096);
        let item = f
            .app
            .capture_inbox(&CaptureInboxRequest {
                id: Uuid::new_v4(),
                kind: InboxKind::Text,
                title: "Worst-case synthetic JSON".into(),
                original_name: None,
                text: text.clone(),
            })
            .unwrap();
        f.item = item.capture.id;
        f.source();
        let preview = f.app.preview_inbox_removal(f.item).unwrap();
        assert!(preview.evidence.blockers.is_empty());
        assert_eq!(preview.evidence.original, InboxOriginal::Available { text });
        let encoded = serde_json::to_vec(&preview.evidence).unwrap();
        assert!(encoded.len() > 17 * crate::MAX_NOTE_BYTES);
        assert!(encoded.len() < MAX_PRESERVATION_EVIDENCE_BYTES);
        let replay: InboxRemovalEvidence = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(replay.digest().unwrap(), preview.digest);
        let qualified: brn_store::work::inbox_original_operations::InboxQualifiedRemovalEvidence =
            serde_json::from_slice(&encoded).unwrap();
        assert_eq!(qualified.digest().unwrap(), preview.digest);
        assert_eq!(serde_json::to_vec(&qualified).unwrap(), encoded);
    }
}
