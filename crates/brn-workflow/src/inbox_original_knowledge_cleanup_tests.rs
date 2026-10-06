//! Genuine Knowledge capture-companion recovery after confirmed original removal.
use crate::{
    app::{App, AppConfig},
    inbox::{InboxAvailability, InboxListRequest, InboxOriginal},
    inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
    inbox_original_operations::{InboxRemovalConfirmation, RemoveInboxOriginalRequest},
    inbox_removal::tests::Fixture,
    proposal_apply::{ApplyOutcome, ApprovalRequest},
};
use brn_ai::{KnowledgeProposalArgs, KnowledgeQuoteArgs};
use brn_store::work::WorkTurnStatus;
use std::{fs, os::unix::fs::MetadataExt};
use uuid::Uuid;

#[test]
fn removed_original_bootstraps_before_genuine_knowledge_companion_recovery_without_chat() {
    let mut f = Fixture::new();
    let original = f.app.inbox_item(f.item).unwrap().item;
    let original_path = f.data.join("inbox").join(format!("{}.txt", f.item));
    let original_bytes = fs::read(&original_path).unwrap();
    let original_inode = fs::metadata(&original_path).unwrap().ino();
    let source_operation = f.source();
    let source_journal = f.app.proposal_apply(source_operation).unwrap().unwrap();
    let source = f.app.proposal_source("source.md").unwrap();
    let capture = InboxActionCapture {
        visual_asset: None,
        purpose: InboxAnalysisPurpose::KnowledgeAndActions,
        id: Uuid::new_v4(),
        conversation: None,
        source: source.source,
        source_text: source.text,
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    };
    // Exercise the real captured-job/turn lifecycle with synthetic completed
    // output. No provider or model is invoked by these Store operations.
    let job = f
        .app
        .store
        .reserve_inbox_action(&capture, "Synthetic exact original interpretation")
        .unwrap();
    let turn = f.app.store.begin_inbox_action_turn(&job).unwrap();
    f.app
        .store
        .finish_turn(
            turn.id,
            WorkTurnStatus::Completed,
            "Synthetic interpretation of the preserved Source",
            None,
        )
        .unwrap();
    let args = KnowledgeProposalArgs {
        supersedes: None,
        title: "Reviewed synthetic knowledge".into(),
        path: "knowledge.md".into(),
        source_paths: vec![],
        text: "# Reviewed interpretation\nThe captured body contains Japanese text.\n".into(),
        quotes: vec![KnowledgeQuoteArgs {
            quote: "Exact body 日本語".into(),
            occurrence: None,
        }],
    };
    let draft = f
        .app
        .prepare_inbox_knowledge(&job, &args, turn.conversation_id)
        .unwrap();
    let proposal = f.app.create_proposal(&draft).unwrap();
    let knowledge_approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    let knowledge_receipt = f.app.approve_proposal(&knowledge_approval).unwrap();
    assert_eq!(knowledge_receipt.outcome, ApplyOutcome::Applied);
    let knowledge_journal = f
        .app
        .proposal_apply(knowledge_approval.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        f.app
            .application_records()
            .unwrap()
            .read_inbox_capture(&knowledge_journal)
            .unwrap(),
        Some(job.clone())
    );
    let preserved_files = [
        f.vault.join("source.md"),
        f.vault.join("knowledge.md"),
        f.data
            .join(format!(".brn-apply-{source_operation}.receipt")),
        f.data.join(format!(
            ".brn-apply-{}.receipt",
            knowledge_approval.operation_id
        )),
        f.data.join(format!(
            ".brn-apply-{}.inbox-capture",
            knowledge_approval.operation_id
        )),
    ]
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        let inode = fs::metadata(&path).unwrap().ino();
        (path, bytes, inode)
    });
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    assert!(preview.evidence.blockers.is_empty());
    assert_eq!(
        preview.evidence.source.as_ref().unwrap().approval,
        source_journal
    );
    let request = RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: f.item,
        preview_digest: preview.digest,
        previous_restore: None,
        confirmation: InboxRemovalConfirmation {
            version: 1,
            exact_copy_removal_intended: true,
        },
    };
    let removed = f.app.remove_inbox_original(&request).unwrap();
    assert!(removed.removed_at_ms.is_some());
    let retained_path = f.data.join("inbox").join(format!(
        ".brn-inbox-removed-{}.original",
        request.operation_id
    ));
    assert_eq!(fs::read(&retained_path).unwrap(), original_bytes);
    assert_eq!(fs::metadata(&retained_path).unwrap().ino(), original_inode);
    assert!(!original_path.exists());

    let Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    } = f;
    drop(app);
    // Retire only this fixture's operational SQL and backups. The checked
    // removal mirrors and ordinary Source/Knowledge recovery families remain.
    for name in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
        let path = data.join(name);
        if path.exists() {
            fs::rename(&path, _owner.path().join(format!("retired-{name}"))).unwrap();
        }
    }
    fs::rename(data.join("backups"), _owner.path().join("retired-backups")).unwrap();
    let mut app = App::open(
        &data,
        AppConfig {
            vault_root: Some(vault),
            credentials_dir: Some(_owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(app.open_report().restored_from.is_none());
    let recovered_original = app.inbox_item(item).unwrap();
    assert_eq!(recovered_original.item, original);
    assert_eq!(
        recovered_original.original,
        InboxOriginal::RemovedRetained {
            operation_id: request.operation_id
        }
    );
    assert_eq!(
        app.inbox_items(&InboxListRequest::default())
            .unwrap()
            .entries[0]
            .availability,
        InboxAvailability::RemovedRetained
    );
    assert!(!original_path.exists());
    assert!(
        !data
            .join("inbox")
            .join(format!(".brn-inbox-{item}.stage"))
            .exists()
    );
    assert_eq!(fs::read(&retained_path).unwrap(), original_bytes);
    assert_eq!(fs::metadata(&retained_path).unwrap().ino(), original_inode);
    let recovered_removal = app
        .inbox_original_removal(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(recovered_removal.format(), 2);
    assert_eq!(
        recovered_removal.digest().unwrap(),
        removed.digest().unwrap()
    );
    assert_eq!(
        app.proposal_apply(source_operation).unwrap(),
        Some(source_journal.clone())
    );
    assert_eq!(
        app.proposal_apply(knowledge_approval.operation_id).unwrap(),
        Some(knowledge_journal.clone())
    );
    let recovered_analysis = app.inbox_action_analysis(job.capture.id).unwrap();
    assert_eq!(recovered_analysis.job, job);
    assert!(recovered_analysis.turn.is_none());
    assert_eq!(recovered_analysis.proposals.len(), 1);
    assert_eq!(
        recovered_analysis.proposals[0].draft,
        knowledge_journal.approved.draft
    );
    assert!(app.store.turn(job.capture.id).unwrap().is_none());
    assert!(app.store.conversations().unwrap().is_empty());
    assert_eq!(
        app.store
            .reserve_inbox_action(&job.capture, &job.question)
            .unwrap(),
        job
    );
    assert!(app.store.begin_inbox_action_turn(&job).is_err());
    assert!(
        app.store
            .reserve_inbox_action(&job.capture, "Changed analysis question")
            .is_err()
    );
    assert_eq!(
        app.approve_proposal(&source_journal.request).unwrap(),
        source_journal.receipt.unwrap()
    );
    assert_eq!(
        app.approve_proposal(&knowledge_approval).unwrap(),
        knowledge_receipt
    );
    assert_eq!(
        app.remove_inbox_original(&request)
            .unwrap()
            .digest()
            .unwrap(),
        removed.digest().unwrap()
    );
    assert!(app.store.turn(job.capture.id).unwrap().is_none());
    assert!(app.store.conversations().unwrap().is_empty());
    assert!(!original_path.exists());
    for (path, bytes, inode) in preserved_files {
        assert_eq!(fs::read(&path).unwrap(), bytes, "{}", path.display());
        assert_eq!(
            fs::metadata(&path).unwrap().ino(),
            inode,
            "{}",
            path.display()
        );
    }
    assert_eq!(
        fs::read_dir(_owner.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}
