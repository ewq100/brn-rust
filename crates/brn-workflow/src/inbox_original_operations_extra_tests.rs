use super::*;
use crate::{
    app::AppConfig,
    inbox::{CaptureInboxRequest, InboxKind, InboxOriginal},
    inbox_removal::tests::Fixture,
};
use brn_store::work::{
    WorkTurnStatus,
    inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
    inbox_original_operations::legacy,
};
use std::{fs, os::unix::fs::MetadataExt};

fn request(
    f: &mut Fixture,
    previous_restore: Option<InboxOriginalParent>,
) -> RemoveInboxOriginalRequest {
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    assert!(preview.evidence.blockers.is_empty());
    RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: f.item,
        preview_digest: preview.digest,
        previous_restore,
        confirmation: InboxRemovalConfirmation {
            version: 1,
            exact_copy_removal_intended: true,
        },
    }
}
fn config(f: &Fixture) -> AppConfig {
    AppConfig {
        vault_root: Some(f.vault.clone()),
        credentials_dir: Some(f.data.parent().unwrap().join("credentials")),
        model_dir: None,
    }
}

#[test]
fn oversized_intake_without_operation_history_stays_an_inbox_local_issue() {
    let mut f = Fixture::new();
    f.source();
    // Existing intake intentionally refuses this bounded inventory. Its local
    // issue must not become a new global vault-startup failure during bootstrap.
    for index in 0..16_384 {
        fs::write(f.data.join("inbox").join(format!("unknown-{index}")), b"").unwrap();
    }
    let unknown = f.data.join("inbox").join("unknown-0");
    let config = config(&f);
    let Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    } = f;
    drop(app);
    let mut app =
        App::open(&data, config).expect("ordinary intake issues must not fence the vault");
    assert!(fs::read(unknown).unwrap().is_empty());
    assert!(
        !app.inbox_items(&Default::default())
            .unwrap()
            .issues
            .is_empty()
    );
    assert!(matches!(
        app.inbox_item(item).unwrap().original,
        InboxOriginal::Unavailable { .. }
    ));
    assert!(
        app.store
            .inbox_original_operations(&[])
            .unwrap()
            .history
            .is_empty()
    );
    assert!(vault.join("source.md").exists());
    assert_eq!(
        app.proposal_source("source.md").unwrap().text,
        fs::read_to_string(vault.join("source.md")).unwrap()
    );
    drop((app, _owner));
}
#[test]
fn failed_analysis_and_pending_consequence_do_not_veto_confirmed_exact_copy_cleanup() {
    let mut f = Fixture::new();
    f.source();
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    let source = preview.evidence.source.unwrap().saved;
    let job = f
        .app
        .store
        .reserve_inbox_action(
            &InboxActionCapture {
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: Uuid::new_v4(),
                conversation: None,
                source: source.source.clone(),
                source_text: source.text,
                provider: "chatgpt".into(),
                model: "gpt-6-luna".into(),
                effort: "medium".into(),
            },
            "Synthetic retained failure; no provider route",
        )
        .unwrap();
    f.app.store.begin_inbox_action_turn(&job).unwrap();
    f.app
        .store
        .finish_turn(job.capture.id, WorkTurnStatus::Failed, "", Some("network"))
        .unwrap();
    let draft = f
        .app
        .create_proposal(&crate::proposals::DraftRequest {
            inbox_source: None,
            inbox_knowledge: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Independent pending consequence".into(),
            changes: vec![crate::proposals::DraftNoteChange::Create {
                path: "pending.md".into(),
                text: "Candidate".into(),
            }],
            sources: vec![source.source],
            action_changes: vec![],
        })
        .unwrap();
    let removal = request(&mut f, None);
    assert_eq!(removal.preview_digest, preview.digest);
    f.app.remove_inbox_original(&removal).unwrap();
    assert!(!f.vault.join("pending.md").exists());
    assert_eq!(
        f.app
            .store
            .proposal(draft.draft.id)
            .unwrap()
            .unwrap()
            .stamp(),
        draft.stamp()
    );
    assert_eq!(
        f.app.store.turn(job.capture.id).unwrap().unwrap().status,
        WorkTurnStatus::Failed
    );
    assert_eq!(
        f.app.inbox_item(f.item).unwrap().original,
        InboxOriginal::RemovedRetained {
            operation_id: removal.operation_id
        }
    );
}

fn legacy_record(f: &mut Fixture) -> legacy::InboxOriginalRemovalRecord {
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    let source = preview.evidence.source.unwrap();
    let binding = source
        .approval
        .approved
        .draft
        .inbox_source
        .as_ref()
        .unwrap();
    let evidence = legacy::InboxQualifiedRemovalEvidence {
        snapshot: legacy::InboxRemovalSnapshot {
            original_operations: vec![],
            review: f.app.store.inbox_review_manifest(f.item).unwrap(),
            related_reviews: vec![],
            actions: vec![],
            completions: vec![],
            rewrites: vec![],
            lineage: vec![],
        },
        original: match preview.evidence.original {
            InboxOriginal::Available { text } => legacy::InboxQualifiedOriginal::Available { text },
            _ => panic!("available original"),
        },
        sources: vec![legacy::InboxApprovedSource {
            operation_id: source.approval.request.operation_id,
            proposal_id: source.approval.approved.draft.id,
            note_id: binding.note_id,
            saved: legacy::InboxSourceProof {
                source: source.saved.source,
                text: source.saved.text,
            },
        }],
        saved_consequences: vec![],
        blockers: [],
        needs_owner_attestation: true,
    };
    let request = legacy::RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: f.item,
        preview_digest: evidence.digest().unwrap(),
        previous_restore: None,
        attestation: legacy::InboxRemovalAttestation {
            version: 1,
            copy_disposable: true,
            meaningful_content_preserved: true,
            consequences_reviewed: true,
            conflicts_acknowledged: true,
            exact_copy_removal_intended: true,
        },
    };
    let record = legacy::InboxOriginalRemovalRecord {
        prepared_at_ms: now_ms(evidence.snapshot.review.original.received_at_ms),
        request,
        evidence,
        namespace: f.app.inbox.files.as_ref().unwrap().original_namespace(),
        removed_at_ms: None,
    };
    record.validate().unwrap();
    record
}

#[test]
fn legacy_removal_bootstrap_then_new_restore_and_removal_preserve_mirror_bytes_and_archived_analysis()
 {
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
    let job = f
        .app
        .store
        .reserve_inbox_action(
            &InboxActionCapture {
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: Uuid::new_v4(),
                conversation: None,
                source: source.source,
                source_text: source.text,
                provider: "chatgpt".into(),
                model: "gpt-6-luna".into(),
                effort: "medium".into(),
            },
            "Synthetic historical analysis; no provider route",
        )
        .unwrap();
    f.app.store.begin_inbox_action_turn(&job).unwrap();
    f.app
        .store
        .finish_turn(
            job.capture.id,
            WorkTurnStatus::Completed,
            "Retained historical answer",
            None,
        )
        .unwrap();
    let mut old = legacy_record(&mut f);
    let files = f.app.inbox.files.as_ref().unwrap();
    let operation = OriginalOperationFile::LegacyRemove(Box::new(old.clone()));
    files.publish_operation(&operation).unwrap();
    let intent_path = f.data.join("inbox").join(operation.name());
    let bytes = fs::read(&intent_path).unwrap();
    let inode = fs::metadata(&intent_path).unwrap().ino();
    files.move_original(&operation).unwrap();
    old.removed_at_ms = Some(now_ms(old.prepared_at_ms));
    files
        .publish_operation(&OriginalOperationFile::LegacyRemove(Box::new(old.clone())))
        .unwrap();
    let config = config(&f);
    let Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    } = f;
    drop(app);
    // Reconstruct from genuine mirrors/ordinary approval receipts, not a copied
    // database or synthesized WorkTurn. Bootstrap precedes approval recovery.
    fs::remove_file(data.join("brn.sqlite")).unwrap();
    if data.join("backups").exists() {
        fs::remove_dir_all(data.join("backups")).unwrap();
    }
    let app = App::open(&data, config).unwrap();
    let mut f = Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    };
    let inspected = f
        .app
        .inbox_original_removal(old.request.operation_id)
        .unwrap()
        .unwrap();
    assert!(matches!(inspected, InboxOriginalOperation::LegacyRemove(_)));
    assert_eq!(inspected.digest().unwrap(), old.digest().unwrap());
    assert_eq!(fs::read(&intent_path).unwrap(), bytes);
    assert_eq!(fs::metadata(&intent_path).unwrap().ino(), inode);
    assert!(f.app.store.turn(job.capture.id).unwrap().is_none());
    assert_eq!(
        f.app
            .archived_inbox_analysis(job.capture.id)
            .unwrap()
            .unwrap()
            .analysis
            .turn
            .unwrap()
            .answer,
        "Retained historical answer"
    );
    assert!(matches!(
        f.app.store.begin_inbox_action_turn(&job),
        Err(brn_store::Error::StateChanged(_))
    ));
    let restore = f
        .app
        .restore_inbox_original(&RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: old.request.operation_id,
            removal_digest: old.digest().unwrap(),
        })
        .unwrap();
    let next = request(
        &mut f,
        Some(InboxOriginalParent {
            operation_id: restore.request.operation_id,
            record_sha256: restore.digest().unwrap(),
        }),
    );
    f.app.remove_inbox_original(&next).unwrap();
    assert_eq!(
        f.app
            .inbox_original_operations(f.item)
            .unwrap()
            .iter()
            .map(|s| s.operation_id)
            .collect::<Vec<_>>(),
        vec![
            old.request.operation_id,
            restore.request.operation_id,
            next.operation_id
        ]
    );
    assert_eq!(fs::read(&intent_path).unwrap(), bytes);
    assert_eq!(fs::metadata(&intent_path).unwrap().ino(), inode);
}

#[test]
#[ignore = "expensive escaped-size mirror recovery; run explicitly for lifecycle qualification"]
fn aggregate_over_64_mib_mirrors_import_and_sql_only_repair_stream_without_selected_bodies() {
    let mut f = Fixture::new();
    f.item = f
        .app
        .capture_inbox(&CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Near-limit synthetic original".into(),
            original_name: None,
            text: "\u{0001}".repeat(crate::MAX_NOTE_BYTES - 4096),
        })
        .unwrap()
        .capture
        .id;
    f.source();
    let first = request(&mut f, None);
    let evidence = f.app.qualify_original_removal(&first).unwrap();
    let files = f.app.inbox.files.as_ref().unwrap();
    let namespace = files.original_namespace();
    let mut previous = None;
    let mut receipt_bytes = 0_u64;
    for cycle in 0..4 {
        let mut remove = InboxOriginalRemovalRecord {
            request: RemoveInboxOriginalRequest {
                operation_id: Uuid::new_v4(),
                previous_restore: previous.clone(),
                ..first.clone()
            },
            evidence: evidence.clone(),
            namespace: namespace.clone(),
            prepared_at_ms: now_ms(evidence.item.received_at_ms),
            removed_at_ms: None,
        };
        let operation = OriginalOperationFile::Remove(Box::new(remove.clone()));
        files.publish_operation(&operation).unwrap();
        files.move_original(&operation).unwrap();
        remove.removed_at_ms = Some(now_ms(remove.prepared_at_ms));
        let operation = OriginalOperationFile::Remove(Box::new(remove.clone()));
        files.publish_operation(&operation).unwrap();
        receipt_bytes += fs::metadata(f.data.join("inbox").join(operation.name()))
            .unwrap()
            .len();
        if cycle < 3 {
            let mut restore = InboxOriginalRestoreRecord {
                request: RestoreInboxOriginalRequest {
                    operation_id: Uuid::new_v4(),
                    removal_operation_id: remove.request.operation_id,
                    removal_digest: remove.digest().unwrap(),
                },
                original: evidence.item.clone(),
                namespace: namespace.clone(),
                prepared_at_ms: now_ms(remove.removed_at_ms.unwrap()),
                restored_at_ms: None,
            };
            let operation = OriginalOperationFile::Restore(Box::new(restore.clone()));
            files.publish_operation(&operation).unwrap();
            files.move_original(&operation).unwrap();
            restore.restored_at_ms = Some(now_ms(restore.prepared_at_ms));
            files
                .publish_operation(&OriginalOperationFile::Restore(Box::new(restore.clone())))
                .unwrap();
            previous = Some(InboxOriginalParent {
                operation_id: restore.request.operation_id,
                record_sha256: restore.digest().unwrap(),
            });
        }
    }
    assert!(
        receipt_bytes > 64 * 1024 * 1024,
        "actual aggregate {receipt_bytes}"
    );
    let started = std::time::Instant::now();
    let imported = import_mirrors(&mut f.app.store, files, &files.names().unwrap()).unwrap();
    assert_eq!(imported.history.len(), 7);
    assert!(imported.selected.is_empty());
    let imported_elapsed = started.elapsed();
    for name in files
        .names()
        .unwrap()
        .into_iter()
        .filter(|name| original_operation_name(name))
    {
        fs::remove_file(f.data.join("inbox").join(name)).unwrap();
    }
    let started = std::time::Instant::now();
    let recovered = restore_records(&mut f.app.store, files, &files.names().unwrap()).unwrap();
    assert_eq!(recovered.heads.len(), 1);
    assert_eq!(recovered.removed.len(), 1);
    assert_eq!(
        recovered
            .known
            .iter()
            .filter(|name| original_operation_name(name))
            .count(),
        14
    );
    eprintln!(
        "streamed mirror receipts={receipt_bytes} history=7 import={imported_elapsed:?} SQL-only repair={:?}",
        started.elapsed()
    );
}
