use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        WorkTurnStatus,
        inbox::{InboxCapture, InboxCopy, InboxKind},
        inbox_actions::{InboxActionCapture, InboxAnalysisPurpose, InboxKnowledgeBinding},
        inbox_original_operations::{InboxOriginalOperation, legacy::*},
        inbox_processing::{InboxConversionFormat, InboxProcessOutcome, ProcessInboxRequest},
        inbox_source::InboxSourceBinding,
        proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposals::{NoteChange, ProposalDraft, SourceVersion},
    },
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn attestation() -> InboxRemovalAttestation {
    InboxRemovalAttestation {
        version: 1,
        copy_disposable: true,
        meaningful_content_preserved: true,
        consequences_reviewed: true,
        conflicts_acknowledged: true,
        exact_copy_removal_intended: true,
    }
}
fn namespace() -> InboxOriginalNamespace {
    InboxOriginalNamespace {
        data_device: 1,
        data_inode: 99,
    }
}
fn evidence(store: &mut WorkStore, analysis: bool) -> InboxQualifiedRemovalEvidence {
    let original = store
        .capture_inbox(&InboxCapture {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Synthetic original õ".into(),
            original_name: None,
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: 4,
                sha256: hash(b"body"),
            },
        })
        .unwrap();
    let process = ProcessInboxRequest {
        limits: None,

        id: Uuid::new_v4(),
        items: vec![original.clone()],
    };
    store.process_inbox(&process).unwrap();
    store.start_inbox_processing(process.id, 0).unwrap();
    let converted = "```text\nbody\n```\n";
    store
        .finish_inbox_processing(
            process.id,
            0,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::LiteralTextV1,
                byte_len: converted.len() as u64,
                sha256: hash(converted.as_bytes()),
            },
        )
        .unwrap();
    let binding = InboxSourceBinding {
        extraction: None,
        visual: None,
        batch_id: process.id,
        index: 0,
        original: original.clone(),
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: converted.len() as u64,
        sha256: hash(converted.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    let text = binding.markdown(converted).unwrap();
    let draft = ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: Some(Box::new(binding.clone())),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: VaultIdentity {
                device: 1,
                inode: 10,
            },
        }),
        title: "Preserve exact Source".into(),
        changes: vec![NoteChange::Create {
            path: "Sources/exact.md".into(),
            parent: VaultIdentity {
                device: 1,
                inode: 11,
            },
            text: text.clone(),
        }],
        sources: vec![],
        action_changes: vec![],
    };
    let review = store.create_proposal(&draft).unwrap();
    let apply = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&apply).unwrap();
    let fp = FileFingerprint {
        device: 1,
        inode: 12,
        len: text.len() as u64,
        sha256: hash(text.as_bytes()),
    };
    store
        .record_proposal_prepared(apply.operation_id, std::slice::from_ref(&fp))
        .unwrap();
    store
        .finish_proposal_apply(
            apply.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(fp.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    let source = SourceVersion {
        path: "Sources/exact.md".into(),
        fingerprint: fp,
    };
    if analysis {
        let job = store
            .reserve_inbox_action(
                &InboxActionCapture {
                    intake: None,
                    visual_asset: None,
                    purpose: Default::default(),
                    id: Uuid::new_v4(),
                    conversation: None,
                    source: Some(source.clone()),
                    source_text: text.clone(),
                    provider: "chatgpt".into(),
                    model: "gpt-6-luna".into(),
                    effort: "medium".into(),
                },
                "Retain this exact analysis",
            )
            .unwrap();
        store.begin_inbox_action_turn(&job).unwrap();
        store
            .finish_turn(
                job.capture.id,
                WorkTurnStatus::Completed,
                "Historical answer õ",
                None,
            )
            .unwrap();
    }
    InboxQualifiedRemovalEvidence {
        snapshot: serde_json::from_value(
            serde_json::to_value(store.inbox_removal_snapshot(original.capture.id).unwrap())
                .unwrap(),
        )
        .unwrap(),
        original: InboxQualifiedOriginal::Available {
            text: "body".into(),
        },
        sources: vec![InboxApprovedSource {
            operation_id: apply.operation_id,
            proposal_id: draft.id,
            note_id: binding.note_id,
            saved: InboxSourceProof { source, text },
        }],
        saved_consequences: vec![],
        blockers: [],
        needs_owner_attestation: true,
    }
}
fn request(
    e: &InboxQualifiedRemovalEvidence,
    previous: Option<Uuid>,
) -> RemoveInboxOriginalRequest {
    RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: e.snapshot.review.original.capture.id,
        preview_digest: e.digest().unwrap(),
        previous_restore: previous,
        attestation: attestation(),
    }
}
fn bytes<T: serde::Serialize>(v: &T) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn terminal(
    _store: &mut WorkStore,
    e: &InboxQualifiedRemovalEvidence,
) -> InboxOriginalRemovalRecord {
    let at = e.snapshot.review.original.received_at_ms;
    InboxOriginalRemovalRecord {
        request: request(e, None),
        evidence: e.clone(),
        namespace: namespace(),
        prepared_at_ms: at,
        removed_at_ms: Some(at),
    }
}
fn knowledge_evidence(store: &mut WorkStore) -> InboxQualifiedRemovalEvidence {
    knowledge_evidence_with_status(store, WorkTurnStatus::Completed)
}
fn knowledge_evidence_with_status(
    store: &mut WorkStore,
    status: WorkTurnStatus,
) -> InboxQualifiedRemovalEvidence {
    let mut e = evidence(store, false);
    let source = &e.sources[0].saved;
    let job = store
        .reserve_inbox_action(
            &InboxActionCapture {
                intake: None,
                visual_asset: None,
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: Uuid::new_v4(),
                conversation: None,
                source: Some(source.source.clone()),
                source_text: source.text.clone(),
                provider: "chatgpt".into(),
                model: "gpt-6-luna".into(),
                effort: "medium".into(),
            },
            "Interpret this exact Source",
        )
        .unwrap();
    store.begin_inbox_action_turn(&job).unwrap();
    store
        .finish_turn(
            job.capture.id,
            status,
            "Synthetic interpretation",
            (status == WorkTurnStatus::Failed).then_some("network"),
        )
        .unwrap();
    let note_id = Uuid::new_v4();
    let start_byte = source.text.find("body").unwrap();
    let citation = brn_store::note_provenance::VaultCitation {
        note_id: e.sources[0].note_id,
        sha256: source.source.fingerprint.sha256,
        start_byte,
        end_byte: start_byte + 4,
        quote: "body".into(),
    };
    let text = brn_store::note_provenance::write(&format!("---\nbrn_id: {note_id}\nbrn_kind: knowledge\nbrn_state: current\n---\n# Interpreted knowledge\n"), std::slice::from_ref(&citation)).unwrap();
    let draft = ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: Some(Box::new(InboxKnowledgeBinding {
            intake_citations: Vec::new(),
            intake: None,
            analysis_id: job.capture.id,
            note_id,
            source: Some(source.source.clone()),
            supersedes: None,
            citations: vec![citation],
        })),
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: Some(job.capture.id),
        session_id: None,
        vault: e.snapshot.review.proposals[0].record.draft.vault.clone(),
        title: "Preserve interpreted knowledge".into(),
        changes: vec![NoteChange::Create {
            path: "knowledge/exact.md".into(),
            parent: VaultIdentity {
                device: 1,
                inode: 11,
            },
            text: text.clone(),
        }],
        sources: vec![source.source.clone()],
        action_changes: vec![],
    };
    let review = store.create_proposal(&draft).unwrap();
    let req = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&req).unwrap();
    let fp = FileFingerprint {
        device: 1,
        inode: 13,
        len: text.len() as u64,
        sha256: hash(text.as_bytes()),
    };
    store
        .record_proposal_prepared(req.operation_id, std::slice::from_ref(&fp))
        .unwrap();
    store
        .finish_proposal_apply(
            req.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(fp.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    e.saved_consequences.push(InboxSavedConsequence {
        operation_id: req.operation_id,
        member_index: 0,
        saved: InboxSourceProof {
            source: SourceVersion {
                path: "knowledge/exact.md".into(),
                fingerprint: fp,
            },
            text,
        },
    });
    e.snapshot = serde_json::from_value(
        serde_json::to_value(
            store
                .inbox_removal_snapshot(e.snapshot.review.original.capture.id)
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    e
}

fn rebind_knowledge_certificate(record: &mut InboxOriginalRemovalRecord, mode: usize) {
    let evidence = &mut record.evidence;
    let operation = evidence.saved_consequences[0].operation_id;
    let proposal = evidence
        .snapshot
        .lineage
        .iter()
        .find(|j| j.request.operation_id == operation)
        .unwrap()
        .approved
        .draft
        .id;
    if mode == 0 {
        evidence.snapshot.review.analyses[0].job.capture.purpose = InboxAnalysisPurpose::Actions;
    } else {
        let draft = &mut evidence
            .snapshot
            .review
            .proposals
            .iter_mut()
            .find(|p| p.record.draft.id == proposal)
            .unwrap()
            .record
            .draft;
        let binding = draft.inbox_knowledge.as_mut().unwrap();
        match mode {
            1 => binding.citations[0].quote = "fork".into(),
            2 => {
                binding.citations[0].start_byte += 1;
                binding.citations[0].end_byte += 1;
            }
            _ => {
                let mut forged = binding.citations[0].clone();
                forged.quote = "fork".into();
                binding.citations.push(forged);
            }
        }
        binding.validate().unwrap();
        let citations = binding.citations.clone();
        let NoteChange::Create { text, .. } = &mut draft.changes[0] else {
            panic!("expected one Knowledge Create")
        };
        *text = brn_store::note_provenance::write(text, &citations).unwrap();
        let saved = &mut evidence.saved_consequences[0].saved;
        saved.text = text.clone();
        saved.source.fingerprint.len = text.len() as u64;
        saved.source.fingerprint.sha256 = hash(text.as_bytes());
        let changed_draft = draft.clone();
        let creation_sha256 = hash(&bytes(&changed_draft));
        let review = evidence
            .snapshot
            .review
            .proposals
            .iter_mut()
            .find(|p| p.record.draft.id == proposal)
            .unwrap();
        review.creation_sha256 = creation_sha256;
        let applied_stamp = review.record.stamp();
        for journal in evidence
            .snapshot
            .review
            .approvals
            .iter_mut()
            .chain(&mut evidence.snapshot.lineage)
        {
            if journal.request.operation_id == operation {
                journal.approved.draft = changed_draft.clone();
                journal.creation_sha256 = creation_sha256;
                journal.request.expected = journal.approved.stamp();
                journal.receipt.as_mut().unwrap().stamp = applied_stamp;
                journal.prepared = Some(vec![saved.source.fingerprint.clone()]);
                journal.observations.as_mut().unwrap()[0].destination =
                    Some(saved.source.fingerprint.clone());
                journal.validate().unwrap();
            }
        }
        saved.validate().unwrap();
    }
    evidence.snapshot.review.analyses[0].job.validate().unwrap();
    record.request.preview_digest = hash(&bytes(evidence));
}

#[test]
fn legacy_applied_knowledge_keeps_purpose_and_every_exact_quote_checked() {
    let dir = fixture();
    let (mut creator, _) = WorkStore::open(dir.path()).unwrap();
    let e = knowledge_evidence(&mut creator);
    let valid = terminal(&mut creator, &e);
    valid.validate().unwrap();
    let target = fixture();
    let (mut store, _) = WorkStore::open(target.path()).unwrap();
    store
        .restore_inbox_original_operations(
            &[InboxOriginalOperation::LegacyRemove(Box::new(
                valid.clone(),
            ))],
            &[],
        )
        .unwrap();
    assert!(store.conversations().unwrap().is_empty());
    let job = &e.snapshot.review.analyses[0].job;
    assert_eq!(
        store.inbox_action(job.capture.id).unwrap().as_ref(),
        Some(job)
    );
    for mode in 0..4 {
        let mut bad = valid.clone();
        rebind_knowledge_certificate(&mut bad, mode);
        assert!(bad.validate().is_err(), "mode {mode}");
        let fresh = fixture();
        let (mut store, _) = WorkStore::open(fresh.path()).unwrap();
        assert!(
            store
                .restore_inbox_original_operations(
                    &[InboxOriginalOperation::LegacyRemove(Box::new(bad))],
                    &[]
                )
                .is_err()
        );
        assert!(store.inbox_item(valid.request.item_id).unwrap().is_none());
        assert!(store.inbox_action(job.capture.id).unwrap().is_none());
    }
}
// Pinned once from the copied 7c4f668 serde layout and genuine synthetic work.
// The complete fixed bytes, record digest and envelope digest guard future drift.
const GOLDEN: &str = include_str!("fixtures/inbox-original-v1.json");
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FixedEnvelope {
    operation: brn_store::work::inbox_original_legacy::Operation,
    sha256: [u8; 32],
}
fn golden() -> InboxOriginalRemovalRecord {
    let env: FixedEnvelope = serde_json::from_str(GOLDEN).unwrap();
    assert_eq!(bytes(&env), GOLDEN.as_bytes());
    assert_eq!(hash(&bytes(&env.operation)), env.sha256);
    assert_eq!(
        hex(&hash(GOLDEN.as_bytes())),
        "380a30650a5a8e605283ccc83e6a23c0eb8ca9c1cf40ceae5790753dd90e303b"
    );
    let brn_store::work::inbox_original_legacy::Operation::Remove(r) = env.operation else {
        panic!("fixed Remove")
    };
    assert_eq!(
        hex(&r.digest().unwrap()),
        "acc1203cb8773872226b7d0ecfc4856999aa97322a5f55f6c84776bda95f4630"
    );
    *r
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn fixed_legacy_v14_setting_migrates_and_restores_backup_without_rewriting_bytes() {
    let r = golden();
    let analysis_ids: Vec<_> = r
        .evidence
        .snapshot
        .review
        .analyses
        .iter()
        .map(|analysis| analysis.job.capture.id)
        .collect();
    let id = r.request.operation_id;
    let original = r.evidence.snapshot.review.original.clone();
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .restore_inbox_original_operations(
            &[InboxOriginalOperation::LegacyRemove(Box::new(r))],
            &[id],
        )
        .unwrap();
    for &analysis in &analysis_ids {
        assert_eq!(store.run_budget(analysis).unwrap(), None);
        assert_eq!(store.resolve_run_budget(analysis, None).unwrap(), None);
        assert!(matches!(
            store.resolve_run_budget(analysis, Some(brn_store::work::WorkBudget::default())),
            Err(Error::OperationConflict(_))
        ));
    }
    assert!(
        store
            .set_setting(&format!("{ORIGINAL_OPERATION_PREFIX}{id}"), "x")
            .is_err()
    );
    assert!(
        store
            .remove_setting(&format!("{ORIGINAL_OPERATION_PREFIX}{id}"))
            .is_err()
    );
    drop(store);
    let db = dir.path().join("brn.sqlite");
    let raw = Connection::open(&db).unwrap();
    raw.execute_batch("DROP TABLE ai_run_budgets; DROP TABLE intake_snapshots; DROP TABLE inbox_original_operations; PRAGMA user_version=14;")
        .unwrap();
    assert_eq!(
        raw.query_row(
            "SELECT value FROM settings WHERE key=?1",
            [format!("{ORIGINAL_OPERATION_PREFIX}{id}")],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        GOLDEN
    );
    raw.backup(
        "main",
        dir.path().join("backups/brn-9999999999999.sqlite"),
        None,
    )
    .unwrap();
    drop(raw);
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.restored_from.is_none());
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
    assert_eq!(
        store
            .setting(&format!("{ORIGINAL_OPERATION_PREFIX}{id}"))
            .unwrap()
            .as_deref(),
        Some(GOLDEN)
    );
    drop(store);
    std::fs::write(&db, b"synthetic physical corruption").unwrap();
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.restored_from.is_some());
    for &analysis in &analysis_ids {
        assert_eq!(store.run_budget(analysis).unwrap(), None);
        assert_eq!(store.resolve_run_budget(analysis, None).unwrap(), None);
    }
    assert_eq!(
        store
            .setting(&format!("{ORIGINAL_OPERATION_PREFIX}{id}"))
            .unwrap()
            .as_deref(),
        Some(GOLDEN)
    );
    assert_eq!(
        store.inbox_original_operations(&[id]).unwrap().selected[0].format(),
        1
    );
}
#[test]
fn legacy_turn_and_reservation_forks_are_atomic_with_mixed_new_restore() {
    use brn_store::work::inbox_original_operations as new;
    let a = golden();
    let original = a.evidence.snapshot.review.original.clone();
    let at = a.removed_at_ms.unwrap();
    let b = new::InboxOriginalRestoreRecord {
        request: RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: a.request.operation_id,
            removal_digest: a.digest().unwrap(),
        },
        original: original.clone(),
        namespace: a.namespace.clone(),
        prepared_at_ms: at + 1,
        restored_at_ms: Some(at + 1),
    };
    let first = InboxOriginalOperation::LegacyRemove(Box::new(a.clone()));
    let second = InboxOriginalOperation::Restore(Box::new(b));
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .restore_inbox_original_operations(&[second.clone(), first.clone()], &[])
        .unwrap();
    for mode in 0..2 {
        let mut c = a.clone();
        c.request.operation_id = Uuid::new_v4();
        c.request.previous_restore = Some(second.summary().unwrap().operation_id);
        c.prepared_at_ms = at + 2;
        c.removed_at_ms = Some(at + 2);
        c.evidence.snapshot.original_operations =
            vec![first.summary().unwrap(), second.summary().unwrap()];
        let analysis = &mut c.evidence.snapshot.review.analyses[0];
        if mode == 0 {
            analysis.turn.as_mut().unwrap().answer = "Forked archive".into();
        } else {
            analysis.job.question.push('x');
            analysis.turn.as_mut().unwrap().question.push('x');
        }
        c.request.preview_digest = c.evidence.digest().unwrap();
        c.validate().unwrap();
        assert!(
            store
                .restore_inbox_original_operations(
                    &[InboxOriginalOperation::LegacyRemove(Box::new(c))],
                    &[]
                )
                .is_err(),
            "mode {mode}"
        );
        assert_eq!(
            store.inbox_original_operations(&[]).unwrap().history.len(),
            2
        );
    }
    // A lean new Remove can extend this old family without cumulative history.
    let source = a.evidence.sources[0].clone();
    let approval = a
        .evidence
        .snapshot
        .review
        .approvals
        .iter()
        .find(|j| j.request.operation_id == source.operation_id)
        .unwrap()
        .clone();
    let e = new::InboxQualifiedRemovalEvidence {
        item: original,
        original: a.evidence.original.clone(),
        source: Some(new::InboxApprovedSource {
            approval,
            saved: source.saved,
        }),
        blockers: [],
        needs_owner_confirmation: true,
    };
    let c = new::InboxOriginalRemovalRecord {
        request: new::RemoveInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            item_id: e.item.capture.id,
            preview_digest: e.digest().unwrap(),
            previous_restore: Some(new::InboxOriginalParent {
                operation_id: second.summary().unwrap().operation_id,
                record_sha256: second.summary().unwrap().record_sha256,
            }),
            confirmation: new::InboxRemovalConfirmation {
                version: 1,
                exact_copy_removal_intended: true,
            },
        },
        evidence: e,
        namespace: a.namespace,
        prepared_at_ms: at + 2,
        removed_at_ms: Some(at + 2),
    };
    assert_eq!(
        store
            .restore_inbox_original_operations(&[InboxOriginalOperation::Remove(Box::new(c))], &[])
            .unwrap()
            .history
            .len(),
        3
    );
}
#[test]
fn malformed_readable_v14_legacy_authority_refuses_before_migration_or_backup() {
    let a = golden();
    let id = a.request.operation_id;
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .restore_inbox_original_operations(
            &[InboxOriginalOperation::LegacyRemove(Box::new(a))],
            &[],
        )
        .unwrap();
    drop(store);
    let db = dir.path().join("brn.sqlite");
    let raw = Connection::open(&db).unwrap();
    raw.execute_batch("DROP TABLE ai_run_budgets; DROP TABLE intake_snapshots; DROP TABLE inbox_original_operations; PRAGMA user_version=14;")
        .unwrap();
    raw.execute(
        "UPDATE settings SET value='{}' WHERE key=?1",
        [format!("{ORIGINAL_OPERATION_PREFIX}{id}")],
    )
    .unwrap();
    drop(raw);
    let before = std::fs::read(&db).unwrap();
    let backups = std::fs::read_dir(dir.path().join("backups"))
        .unwrap()
        .count();
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::Invalid(_))
    ));
    assert_eq!(std::fs::read(&db).unwrap(), before);
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        backups
    );
    let raw = Connection::open(db).unwrap();
    assert_eq!(
        raw.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
}

#[test]
fn historical_legacy_only_recovery_cannot_reopen_provider_turn_after_restart() {
    let r = golden();
    let job = r.evidence.snapshot.review.analyses[0].job.clone();
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .restore_inbox_original_operations(
            &[InboxOriginalOperation::LegacyRemove(Box::new(r))],
            &[],
        )
        .unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert!(store.conversations().unwrap().is_empty());
    assert!(store.turn(job.capture.id).unwrap().is_none());
    assert_eq!(store.run_budget(job.capture.id).unwrap(), None);
    assert_eq!(
        store.resolve_run_budget(job.capture.id, None).unwrap(),
        None
    );
    assert!(matches!(
        store.begin_turn_with_effort_and_budget(
            job.capture.id,
            job.capture.conversation,
            &job.question,
            &job.capture.provider,
            &job.capture.model,
            Some(&job.capture.effort),
            None,
        ),
        Err(Error::OperationConflict(_) | Error::StateChanged(_))
    ));
    assert!(matches!(
        store.begin_inbox_action_turn(&job),
        Err(Error::StateChanged(_))
    ));
    assert!(store.conversations().unwrap().is_empty());
    assert!(store.turn(job.capture.id).unwrap().is_none());
    let mut capture = job.capture.clone();
    capture.id = Uuid::new_v4();
    let unissued = store.reserve_inbox_action(&capture, &job.question).unwrap();
    assert_eq!(
        store.begin_inbox_action_turn(&unissued).unwrap().status,
        WorkTurnStatus::Running
    );
}

#[test]
fn actual_failed_and_completed_turn_replay_precedes_legacy_historical_fence() {
    for status in [WorkTurnStatus::Failed, WorkTurnStatus::Completed] {
        let dir = fixture();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let e = knowledge_evidence_with_status(&mut store, status);
        let job = e.snapshot.review.analyses[0].job.clone();
        let actual = store.turn(job.capture.id).unwrap().unwrap();
        assert_eq!(actual.status, status);
        let record = terminal(&mut store, &e);
        store
            .restore_inbox_original_operations(
                &[InboxOriginalOperation::LegacyRemove(Box::new(record))],
                &[],
            )
            .unwrap();
        assert_eq!(
            bytes(&store.begin_inbox_action_turn(&job).unwrap()),
            bytes(&actual)
        );
        let conversations = store.conversations().unwrap().len();
        drop(store);
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            bytes(&store.begin_inbox_action_turn(&job).unwrap()),
            bytes(&actual)
        );
        assert_eq!(store.conversations().unwrap().len(), conversations);
    }
}

#[test]
fn binary_forged_legacy_remove_and_restore_records_are_unsupported() {
    let mut remove = golden();
    let mut restored = InboxOriginalRestoreRecord {
        request: RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: remove.request.operation_id,
            removal_digest: remove.digest().unwrap(),
        },
        original: remove.evidence.snapshot.review.original.clone(),
        namespace: remove.namespace.clone(),
        prepared_at_ms: remove.prepared_at_ms,
        restored_at_ms: None,
    };
    restored.original.capture.kind = brn_store::work::inbox::InboxKind::Binary;
    assert!(
        restored
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
    remove.evidence.snapshot.review.original.capture.kind =
        brn_store::work::inbox::InboxKind::Binary;
    assert!(
        remove
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
}
