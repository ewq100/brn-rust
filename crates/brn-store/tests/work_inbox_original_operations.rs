use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        WorkTurnStatus,
        inbox::{InboxCapture, InboxCopy, InboxKind},
        inbox_actions::{InboxActionCapture, InboxAnalysisPurpose, InboxKnowledgeBinding},
        inbox_original_operations::*,
        inbox_processing::{InboxConversionFormat, InboxProcessOutcome, ProcessInboxRequest},
        inbox_source::InboxSourceBinding,
        proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposals::{NoteChange, ProposalDraft, SourceVersion},
    },
};
use rusqlite::{Connection, params};
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
                    purpose: Default::default(),
                    id: Uuid::new_v4(),
                    conversation: None,
                    source: source.clone(),
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
        snapshot: store.inbox_removal_snapshot(original.capture.id).unwrap(),
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
    store: &mut WorkStore,
    e: &InboxQualifiedRemovalEvidence,
) -> InboxOriginalRemovalRecord {
    let mut r = store
        .prepare_inbox_original_removal(&request(e, None), e, &namespace())
        .unwrap();
    r.removed_at_ms = Some(r.prepared_at_ms + 1);
    store.settle_inbox_original_removal(&r).unwrap()
}
#[test]
fn full_attestation_preview_and_member_bindings_refuse_changed_evidence() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut store, false);
    e.validate().unwrap();
    for mode in 0..6 {
        let mut a = attestation();
        match mode {
            0 => a.version = 2,
            1 => a.copy_disposable = false,
            2 => a.meaningful_content_preserved = false,
            3 => a.consequences_reviewed = false,
            4 => a.conflicts_acknowledged = false,
            _ => a.exact_copy_removal_intended = false,
        };
        assert!(a.validate().is_err());
    }
    for mode in 0..10 {
        let mut bad = e.clone();
        match mode {
            0 => bad.sources.clear(),
            1 => bad.sources[0].saved.source.path = "../escape.md".into(),
            2 => bad.sources[0].saved.text.push('x'),
            3 => bad.sources[0].saved.source.fingerprint.inode += 1,
            4 => bad.sources[0].note_id = Uuid::new_v4(),
            5 => bad.sources[0].proposal_id = Uuid::new_v4(),
            6 => bad.needs_owner_attestation = false,
            7 => {
                bad.original = InboxQualifiedOriginal::Available {
                    text: "fork".into(),
                }
            }
            8 => bad.snapshot.review.processing.clear(),
            _ => bad.snapshot.review.proposals[0].creation_sha256[0] ^= 1,
        };
        assert!(bad.validate().is_err(), "mode {mode}");
    }
    let mut req = request(&e, None);
    req.preview_digest[0] ^= 1;
    assert!(
        store
            .prepare_inbox_original_removal(&req, &e, &namespace())
            .is_err()
    );
    assert!(store.inbox_original_operation_ids().unwrap().is_empty());
}
#[test]
fn exact_replay_precedes_new_eligibility_and_snapshot_cas_refuses_change() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut store, false);
    let req = request(&e, None);
    let intent = store
        .prepare_inbox_original_removal(&req, &e, &namespace())
        .unwrap();
    let mut invalid = e.clone();
    invalid.sources.clear();
    assert_eq!(
        bytes(
            &store
                .prepare_inbox_original_removal(
                    &req,
                    &invalid,
                    &InboxOriginalNamespace {
                        data_device: 0,
                        data_inode: 0
                    }
                )
                .unwrap()
        ),
        bytes(&intent)
    );
    let mut changed = req.clone();
    changed.preview_digest[0] ^= 1;
    assert!(matches!(
        store.prepare_inbox_original_removal(&changed, &e, &namespace()),
        Err(Error::OperationConflict(_))
    ));
    assert!(store.settle_inbox_original_removal(&intent).is_err());
    let data2 = fixture();
    let (mut second, _) = WorkStore::open(data2.path()).unwrap();
    let stale = evidence(&mut second, false);
    second
        .process_inbox(&ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![stale.snapshot.review.original.clone()],
        })
        .unwrap();
    assert!(matches!(
        second.prepare_inbox_original_removal(&request(&stale, None), &stale, &namespace()),
        Err(Error::StateChanged(_))
    ));
}
#[test]
fn causal_remove_restore_remove_uses_exact_terminal_identity_and_pending_head() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut e = evidence(&mut store, false);
    let removed = terminal(&mut store, &e);
    let req = RestoreInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        removal_operation_id: removed.request.operation_id,
        removal_digest: removed.digest().unwrap(),
    };
    let restore = store.prepare_inbox_original_restore(&req).unwrap();
    e.snapshot = store
        .inbox_removal_snapshot(removed.request.item_id)
        .unwrap();
    assert!(
        store
            .prepare_inbox_original_removal(&request(&e, Some(req.operation_id)), &e, &namespace())
            .is_err()
    );
    let mut competing = req.clone();
    competing.operation_id = Uuid::new_v4();
    assert!(store.prepare_inbox_original_restore(&competing).is_err());
    let mut settled = restore.clone();
    settled.restored_at_ms = Some(settled.prepared_at_ms + 1);
    store.settle_inbox_original_restore(&settled).unwrap();
    e.snapshot = store
        .inbox_removal_snapshot(removed.request.item_id)
        .unwrap();
    let next = store
        .prepare_inbox_original_removal(&request(&e, Some(req.operation_id)), &e, &namespace())
        .unwrap();
    assert_eq!(
        store
            .inbox_original_operation_head(removed.request.item_id)
            .unwrap()
            .unwrap()
            .operation_id,
        next.request.operation_id
    );
    assert_eq!(
        store
            .inbox_original_operation_summaries(removed.request.item_id)
            .unwrap()
            .iter()
            .map(|s| s.operation_id)
            .collect::<Vec<_>>(),
        vec![
            removed.request.operation_id,
            req.operation_id,
            next.request.operation_id
        ]
    );
    assert_eq!(
        bytes(&store.settle_inbox_original_removal(&removed).unwrap()),
        bytes(&removed)
    );
    let mut fork = removed.clone();
    fork.removed_at_ms = Some(removed.removed_at_ms.unwrap() + 1);
    assert!(matches!(
        store.settle_inbox_original_removal(&fork),
        Err(Error::OperationConflict(_))
    ));
    let mut body = removed.clone();
    body.namespace.data_inode += 1;
    assert!(store.restore_inbox_original_removal(&body).is_err());
    assert_eq!(
        bytes(
            &store
                .restore_inbox_original_removal(&InboxOriginalRemovalRecord {
                    removed_at_ms: None,
                    ..removed.clone()
                })
                .unwrap()
        ),
        bytes(&removed)
    );
    let collision = RestoreInboxOriginalRequest {
        operation_id: removed.request.operation_id,
        ..req
    };
    assert!(store.prepare_inbox_original_restore(&collision).is_err());
}
#[test]
fn certified_recovery_imports_exact_capture_times_without_live_turn_or_sessions() {
    let data = fixture();
    let (mut old, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut old, true);
    let removed = terminal(&mut old, &e);
    let job = e.snapshot.review.analyses[0].job.clone();
    let fresh = fixture();
    let (mut recovered, _) = WorkStore::open(fresh.path()).unwrap();
    recovered.restore_inbox_original_removal(&removed).unwrap();
    assert_eq!(
        recovered.inbox_item(removed.request.item_id).unwrap(),
        Some(e.snapshot.review.original.clone())
    );
    assert_eq!(
        recovered.inbox_action(job.capture.id).unwrap(),
        Some(job.clone())
    );
    assert!(recovered.turn(job.capture.id).unwrap().is_none());
    assert!(recovered.conversations().unwrap().is_empty());
    let archived = recovered
        .archived_inbox_analysis(job.capture.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        bytes(&archived.analysis),
        bytes(&e.snapshot.review.analyses[0])
    );
    assert!(matches!(
        recovered.begin_inbox_action_turn(&job),
        Err(Error::StateChanged(_))
    ));
    let mut new = job.capture.clone();
    new.id = Uuid::new_v4();
    let next = recovered.reserve_inbox_action(&new, &job.question).unwrap();
    assert_eq!(
        recovered.begin_inbox_action_turn(&next).unwrap().status,
        WorkTurnStatus::Running
    );
    assert!(
        old.archived_inbox_analysis(job.capture.id)
            .unwrap()
            .is_none()
    );
    recovered
        .restore_inbox_original_removal(&InboxOriginalRemovalRecord {
            removed_at_ms: None,
            ..removed.clone()
        })
        .unwrap();
    assert_eq!(
        bytes(
            &recovered
                .inbox_original_removal(removed.request.operation_id)
                .unwrap()
                .unwrap()
        ),
        bytes(&removed)
    );
    // Bootstrap restores no processing or approvals; certificates retain these.
    let raw = Connection::open(fresh.path().join("brn.sqlite")).unwrap();
    for table in ["inbox_processing", "proposals", "proposal_applies"] {
        assert_eq!(
            raw.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn ordinary_restore_chain_import_requires_exact_parent_and_rejects_forks() {
    let data = fixture();
    let (mut old, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut old, false);
    let removed = terminal(&mut old, &e);
    let mut restored = old
        .prepare_inbox_original_restore(&RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: removed.request.operation_id,
            removal_digest: removed.digest().unwrap(),
        })
        .unwrap();
    restored.restored_at_ms = Some(restored.prepared_at_ms);
    old.settle_inbox_original_restore(&restored).unwrap();
    let fresh = fixture();
    let (mut target, _) = WorkStore::open(fresh.path()).unwrap();
    assert!(target.restore_inbox_original_restore(&restored).is_err());
    target.restore_inbox_original_removal(&removed).unwrap();
    target.restore_inbox_original_restore(&restored).unwrap();
    let mut fork = restored.clone();
    fork.request.operation_id = Uuid::new_v4();
    assert!(target.restore_inbox_original_restore(&fork).is_err());
    let mut unknown = restored.clone();
    unknown.request.operation_id = Uuid::new_v4();
    unknown.request.removal_operation_id = Uuid::new_v4();
    assert!(target.restore_inbox_original_restore(&unknown).is_err());
    let mut wrong = restored.clone();
    wrong.request.removal_digest[0] ^= 1;
    assert!(target.restore_inbox_original_restore(&wrong).is_err());
    assert_eq!(
        bytes(
            &target
                .restore_inbox_original_restore(&InboxOriginalRestoreRecord {
                    restored_at_ms: None,
                    ..restored.clone()
                })
                .unwrap()
        ),
        bytes(&restored)
    );
}
#[test]
fn owned_namespace_guards_and_semantic_damage_preserve_database_and_backups() {
    for mode in 0..5 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let e = evidence(&mut store, false);
        let removed = terminal(&mut store, &e);
        let key = format!(
            "{ORIGINAL_OPERATION_PREFIX}{}",
            removed.request.operation_id
        );
        assert!(store.set_setting(&key, "replacement").is_err());
        assert!(store.remove_setting(&key).is_err());
        drop(store);
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        let value: String = raw
            .query_row("SELECT value FROM settings WHERE key=?1", [&key], |r| {
                r.get(0)
            })
            .unwrap();
        let mut json: serde_json::Value = serde_json::from_str(&value).unwrap();
        match mode {
            0 => json["sha256"][0] = serde_json::json!(json["sha256"][0].as_u64().unwrap() ^ 1),
            1 => json["extra"] = serde_json::json!(true),
            2 => json["operation"]["record"]["removed_at_ms"] = serde_json::json!(0),
            3 => {
                raw.execute(
                    "UPDATE settings SET key=?1 WHERE key=?2",
                    params![
                        format!(
                            "{ORIGINAL_OPERATION_PREFIX}{}",
                            removed.request.operation_id.to_string().to_uppercase()
                        ),
                        &key
                    ],
                )
                .unwrap();
            }
            _ => {
                raw.execute_batch("CREATE TRIGGER erase_original AFTER INSERT ON settings BEGIN DELETE FROM settings WHERE key=NEW.key; END;").unwrap();
            }
        }
        if mode < 3 {
            let text = if mode == 1 {
                serde_json::to_string_pretty(&json).unwrap()
            } else {
                json.to_string()
            };
            raw.execute(
                "UPDATE settings SET value=?1 WHERE key=?2",
                params![text, &key],
            )
            .unwrap();
        }
        drop(raw);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "mode {mode}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}
#[test]
fn related_other_original_sources_are_retained_and_all_applied_members_are_required() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut e = evidence(&mut store, false);
    let other = evidence(&mut store, false);
    e.snapshot
        .related_reviews
        .extend(other.snapshot.review.proposals.clone());
    e.snapshot.lineage.extend(other.snapshot.lineage.clone());
    assert!(e.validate().is_err());
    let saved = &other.sources[0];
    e.saved_consequences.push(InboxSavedConsequence {
        operation_id: saved.operation_id,
        member_index: 0,
        saved: saved.saved.clone(),
    });
    e.validate().unwrap();
    let mut bad = e.clone();
    bad.saved_consequences[0].saved.source.path = "Sources/renamed.md".into();
    assert!(bad.validate().is_err());
    let mut duplicate = e.clone();
    duplicate
        .saved_consequences
        .push(e.saved_consequences[0].clone());
    assert!(duplicate.validate().is_err());
}
#[test]
fn archive_never_overrides_live_failed_turn_and_conflicting_capture_import_is_atomic() {
    let data = fixture();
    let (mut old, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut old, true);
    let removed = terminal(&mut old, &e);
    let job = e.snapshot.review.analyses[0].job.clone();
    let fresh = fixture();
    let (mut target, _) = WorkStore::open(fresh.path()).unwrap();
    let mut conflicting = target
        .reserve_inbox_action(&job.capture, &job.question)
        .unwrap();
    // Make the timestamp difference deterministic without sampling/waiting.
    conflicting.created_at_ms = job.created_at_ms + 1;
    let raw = Connection::open(fresh.path().join("brn.sqlite")).unwrap();
    let encoded = bytes(&conflicting);
    raw.execute(
        "UPDATE inbox_actions SET created_at_ms=?1,record_json=?2,record_sha256=?3 WHERE id=?4",
        params![
            conflicting.created_at_ms as i64,
            &encoded,
            hash(&encoded).as_slice(),
            job.capture.id.to_string()
        ],
    )
    .unwrap();
    drop(raw);
    // Timestamp is immutable too; even otherwise identical reservation conflicts.
    assert!(target.restore_inbox_original_removal(&removed).is_err());
    assert!(
        target
            .inbox_item(removed.request.item_id)
            .unwrap()
            .is_none()
    );
    assert!(target.inbox_original_operation_ids().unwrap().is_empty());
    let mut altered = job.capture.clone();
    altered.id = Uuid::new_v4();
    let live = old
        .reserve_inbox_action(&altered, "New live failed analysis")
        .unwrap();
    old.begin_inbox_action_turn(&live).unwrap();
    old.finish_turn(live.capture.id, WorkTurnStatus::Failed, "", Some("network"))
        .unwrap();
    assert!(
        old.archived_inbox_analysis(live.capture.id)
            .unwrap()
            .is_none()
    );
    // A fresh recovery has history; a live same-ID turn takes precedence even when failed.
    let second = fixture();
    let (mut target, _) = WorkStore::open(second.path()).unwrap();
    target.restore_inbox_original_removal(&removed).unwrap();
    let raw = Connection::open(second.path().join("brn.sqlite")).unwrap();
    let conversation = Uuid::new_v4();
    raw.execute("INSERT INTO conversations(id,title,created_at_ms,last_activity_at_ms) VALUES(?1,'Synthetic live failure',?2,?2)",params![conversation.to_string(),job.created_at_ms as i64]).unwrap();
    for (role, text) in [("user", job.question.as_str()), ("assistant", "")] {
        raw.execute("INSERT INTO messages(turn_id,conversation_id,sequence,role,text,provider,model,effort,status,error_code,started_at_ms,finished_at_ms) VALUES(?1,?2,1,?3,?4,?5,?6,?7,'failed','network',?8,?8)",params![job.capture.id.to_string(),conversation.to_string(),role,text,&job.capture.provider,&job.capture.model,&job.capture.effort,job.created_at_ms as i64]).unwrap();
    }
    assert!(
        target
            .archived_inbox_analysis(job.capture.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        target.begin_inbox_action_turn(&job).unwrap().status,
        WorkTurnStatus::Failed
    );
}
#[test]
fn noncanonical_but_valid_hash_rows_and_complete_oversize_or_row_limit_refuse() {
    for mode in 0..3 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let e = evidence(&mut store, false);
        let removed = terminal(&mut store, &e);
        drop(store);
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        let key = format!(
            "{ORIGINAL_OPERATION_PREFIX}{}",
            removed.request.operation_id
        );
        match mode {
            0 => {
                let value: String = raw
                    .query_row("SELECT value FROM settings WHERE key=?1", [&key], |r| {
                        r.get(0)
                    })
                    .unwrap();
                let value: serde_json::Value = serde_json::from_str(&value).unwrap();
                raw.execute(
                    "UPDATE settings SET value=?1 WHERE key=?2",
                    params![serde_json::to_string_pretty(&value).unwrap(), &key],
                )
                .unwrap();
            }
            1 => {
                raw.execute(
                    "UPDATE settings SET value=?1 WHERE key=?2",
                    params!["x".repeat(MAX_ORIGINAL_OPERATION_BYTES + 1), &key],
                )
                .unwrap();
            }
            _ => {
                raw.execute_batch("BEGIN").unwrap();
                for _ in 0..16384 {
                    raw.execute(
                        "INSERT INTO settings(key,value) VALUES(?1,'{}')",
                        [format!("{ORIGINAL_OPERATION_PREFIX}{}", Uuid::new_v4())],
                    )
                    .unwrap();
                }
                raw.execute_batch("COMMIT").unwrap();
            }
        }
        drop(raw);
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "mode {mode}"
        );
    }
}
#[test]
fn same_id_conflicting_archived_turns_refuse_second_certificate_atomically() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut e = evidence(&mut store, true);
    let removed = terminal(&mut store, &e);
    let mut restore = store
        .prepare_inbox_original_restore(&RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: removed.request.operation_id,
            removal_digest: removed.digest().unwrap(),
        })
        .unwrap();
    restore.restored_at_ms = Some(restore.prepared_at_ms);
    store.settle_inbox_original_restore(&restore).unwrap();
    e.snapshot = store
        .inbox_removal_snapshot(removed.request.item_id)
        .unwrap();
    e.snapshot.review.analyses[0].turn.as_mut().unwrap().answer = "Forked historical answer".into();
    let record = InboxOriginalRemovalRecord {
        request: request(&e, Some(restore.request.operation_id)),
        evidence: e,
        namespace: namespace(),
        prepared_at_ms: restore.restored_at_ms.unwrap(),
        removed_at_ms: None,
    };
    record.validate().unwrap();
    assert!(store.restore_inbox_original_removal(&record).is_err());
    assert_eq!(store.inbox_original_operation_ids().unwrap().len(), 2);
    assert_eq!(
        bytes(
            &store
                .inbox_original_removal(removed.request.operation_id)
                .unwrap()
                .unwrap()
        ),
        bytes(&removed)
    );
}
#[test]
fn fresh_recovery_restart_keeps_certificate_without_fabricating_later_removal_eligibility() {
    let data = fixture();
    let (mut old, _) = WorkStore::open(data.path()).unwrap();
    let e = evidence(&mut old, true);
    let removed = terminal(&mut old, &e);
    let fresh = fixture();
    let (mut target, _) = WorkStore::open(fresh.path()).unwrap();
    target.restore_inbox_original_removal(&removed).unwrap();
    drop(target);
    let (mut target, report) = WorkStore::open(fresh.path()).unwrap();
    assert!(report.restored_from.is_none());
    assert!(target.conversations().unwrap().is_empty());
    let mut restore = target
        .prepare_inbox_original_restore(&RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: removed.request.operation_id,
            removal_digest: removed.digest().unwrap(),
        })
        .unwrap();
    restore.restored_at_ms = Some(restore.prepared_at_ms);
    target.settle_inbox_original_restore(&restore).unwrap();
    let mut later = e.clone();
    later.snapshot = target
        .inbox_removal_snapshot(removed.request.item_id)
        .unwrap();
    assert!(later.snapshot.review.processing.is_empty());
    assert!(later.snapshot.review.analyses[0].turn.is_none());
    assert!(later.validate().is_err());
    assert!(
        target
            .archived_inbox_analysis(e.snapshot.review.analyses[0].job.capture.id)
            .unwrap()
            .unwrap()
            .analysis
            .turn
            .is_some()
    );
    assert_eq!(
        bytes(
            &target
                .inbox_original_removal(removed.request.operation_id)
                .unwrap()
                .unwrap()
        ),
        bytes(&removed)
    );
}

fn knowledge_evidence(store: &mut WorkStore) -> InboxQualifiedRemovalEvidence {
    let mut e = evidence(store, false);
    let source = &e.sources[0].saved;
    let job = store
        .reserve_inbox_action(
            &InboxActionCapture {
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: Uuid::new_v4(),
                conversation: None,
                source: source.source.clone(),
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
            WorkTurnStatus::Completed,
            "Synthetic interpretation",
            None,
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
        inbox_knowledge: Some(Box::new(InboxKnowledgeBinding {
            analysis_id: job.capture.id,
            note_id,
            source: source.source.clone(),
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
    e.snapshot = store
        .inbox_removal_snapshot(e.snapshot.review.original.capture.id)
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
fn applied_knowledge_certificates_check_purpose_and_every_exact_citation_before_recovery() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let e = knowledge_evidence(&mut store);
    e.validate().unwrap();
    let valid = terminal(&mut store, &e);
    valid.validate().unwrap();
    let fresh = fixture();
    let (mut target, _) = WorkStore::open(fresh.path()).unwrap();
    target.restore_inbox_original_removal(&valid).unwrap();
    assert_eq!(
        bytes(
            &target
                .inbox_original_removal(valid.request.operation_id)
                .unwrap()
                .unwrap()
        ),
        bytes(&valid)
    );
    for mode in 0..4 {
        let mut bad = valid.clone();
        rebind_knowledge_certificate(&mut bad, mode);
        assert_eq!(bad.request.preview_digest, hash(&bytes(&bad.evidence)));
        assert!(
            matches!(bad.validate(), Err(Error::Invalid(_))),
            "validate mode {mode}"
        );
        let fresh = fixture();
        let (mut target, _) = WorkStore::open(fresh.path()).unwrap();
        assert!(
            matches!(
                target.restore_inbox_original_removal(&bad),
                Err(Error::Invalid(_))
            ),
            "import mode {mode}"
        );
        assert!(target.inbox_item(valid.request.item_id).unwrap().is_none());
        assert!(target.inbox_original_operation_ids().unwrap().is_empty());
        assert!(
            target
                .inbox_action(e.snapshot.review.analyses[0].job.capture.id)
                .unwrap()
                .is_none()
        );
        assert!(target.conversations().unwrap().is_empty());
    }
}
