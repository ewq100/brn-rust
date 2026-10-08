//! Historical receipt lineage for intake-backed saved Finding evidence.
use super::*;
use brn_store::{
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        findings::*,
        inbox_actions::{InboxActionCapture, InboxAnalysisPurpose, InboxIntakeBinding},
        inbox_processing::InboxConversionFormat,
        inbox_source::{ExtractionAsset, ExtractionBinding, InboxSourceBinding},
        proposal_apply::{ApplyJournal, ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposals::{NoteChange, ProposalDraft, SourceVersion},
    },
};

struct Setup {
    dir: tempfile::TempDir,
    store: WorkStore,
    capture: InboxActionCapture,
    source: SourceVersion,
    vault: VaultRecord,
    journal: Option<ApplyJournal>,
}

fn setup(applied: bool) -> Setup {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let snapshot = admitted(&mut store);
    store.restore_intake_snapshot(&snapshot).unwrap();
    let note_id = Uuid::new_v4();
    let materialized = snapshot
        .extraction
        .materialize_for_source(&note_id.to_string())
        .unwrap();
    let binding = InboxSourceBinding {
        extraction: Some(ExtractionBinding {
            snapshot_id: snapshot.id,
            snapshot_sha256: snapshot.digest().unwrap(),
            assets: snapshot
                .extraction
                .assets
                .iter()
                .map(|asset| ExtractionAsset {
                    name: brn_intake::asset_file_name_for_source(asset, &note_id.to_string())
                        .unwrap(),
                    byte_len: asset.bytes.len() as u64,
                    sha256: asset.sha256,
                })
                .collect(),
        }),
        visual: None,
        batch_id: snapshot.batch_id,
        index: snapshot.index,
        original: snapshot.original.clone(),
        format: InboxConversionFormat::MaintainedExtractionV1,
        byte_len: materialized.markdown.len() as u64,
        sha256: digest(materialized.markdown.as_bytes()),
        note_id,
    };
    let text = binding.markdown(&materialized.markdown).unwrap();
    let parent = VaultIdentity {
        device: 1,
        inode: 10,
    };
    let vault = VaultRecord {
        id: Uuid::new_v4(),
        root: "/synthetic/vault".into(),
        identity: parent.clone(),
    };
    let path = "Sources/retained.md";
    let mut changes = vec![NoteChange::Create {
        path: path.into(),
        parent: parent.clone(),
        text: text.clone(),
    }];
    for (asset, bytes) in binding
        .extraction
        .as_ref()
        .unwrap()
        .assets
        .iter()
        .zip(&snapshot.extraction.assets)
    {
        changes.push(NoteChange::CreateAsset {
            path: binding
                .extraction
                .as_ref()
                .unwrap()
                .asset_path(path, &asset.name)
                .unwrap(),
            parent: parent.clone(),
            bytes: bytes.bytes.clone(),
        });
    }
    let draft = ProposalDraft {
        intake: None,
        inbox_knowledge: None,
        inbox_visual: None,
        inbox_source: Some(Box::new(binding)),
        action_changes: vec![],
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(vault.clone()),
        title: "Retain synthetic extracted Source".into(),
        changes,
        sources: vec![],
    };
    let review = store.create_proposal(&draft).unwrap();
    let installed = draft
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let bytes = change.candidate_bytes().unwrap();
            FileFingerprint {
                device: 1,
                inode: 100 + i as u64,
                len: bytes.len() as u64,
                sha256: digest(bytes),
            }
        })
        .collect::<Vec<_>>();
    let source = SourceVersion {
        path: path.into(),
        fingerprint: installed[0].clone(),
    };
    let journal = if applied {
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        store.begin_proposal_apply(&request).unwrap();
        store
            .record_proposal_prepared(request.operation_id, &installed)
            .unwrap();
        store
            .finish_proposal_apply(
                request.operation_id,
                ApplyOutcome::Applied,
                Some(
                    &installed
                        .into_iter()
                        .map(|proof| ApplyMemberProof {
                            destination: Some(proof),
                            staging: None,
                        })
                        .collect::<Vec<_>>(),
                ),
            )
            .unwrap();
        store.proposal_apply(request.operation_id).unwrap()
    } else {
        None
    };
    let capture = InboxActionCapture {
        source: None,
        intake: Some(InboxIntakeBinding {
            snapshot_id: snapshot.id,
            snapshot_sha256: snapshot.digest().unwrap(),
            source_proposal: review.stamp(),
            source_path: path.into(),
            source_note_id: note_id,
            source_text_sha256: digest(text.as_bytes()),
            assets: snapshot
                .extraction
                .assets
                .iter()
                .map(|a| a.id.clone())
                .collect(),
            occurrences: snapshot
                .extraction
                .occurrences
                .iter()
                .map(|o| o.id.clone())
                .collect(),
        }),
        visual_asset: None,
        purpose: InboxAnalysisPurpose::KnowledgeAndActions,
        id: Uuid::new_v4(),
        conversation: None,
        source_text: text,
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    };
    store
        .reserve_inbox_action(&capture, "Original private question retained exactly.\r\n")
        .unwrap();
    Setup {
        dir,
        store,
        capture,
        source,
        vault,
        journal,
    }
}

fn quote(text: &str, selected: &str) -> FindingQuote {
    let start_byte = text.find(selected).unwrap();
    FindingQuote {
        start_byte,
        end_byte: start_byte + selected.len(),
        quote: selected.into(),
    }
}
fn finding(s: &Setup) -> FindingDraft {
    let other_id = Uuid::new_v4();
    let other = format!("---\nbrn_id: {other_id}\n---\nOpposing exact context 日本語\n");
    let source_quote = quote(&s.capture.source_text, "Exact extracted body λ");
    let other_quote = quote(&other, "Opposing exact context 日本語");
    let title = "Unresolved retained evidence".to_owned();
    let summary = "Tentative conflicting evidence; approval is not semantic truth.".to_owned();
    FindingDraft {
        request: CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::InboxConflict {
                analysis_id: s.capture.id,
                title: title.clone(),
                summary: summary.clone(),
                source_quote: source_quote.clone(),
                other_path: "context.md".into(),
                other_quote: other_quote.clone(),
            },
        },
        vault: s.vault.clone(),
        title,
        summary,
        evidence: vec![
            FindingEvidence {
                source: s.source.clone(),
                note_id: Some(s.capture.note_id().unwrap()),
                quote: Some(source_quote),
            },
            FindingEvidence {
                source: SourceVersion {
                    path: "context.md".into(),
                    fingerprint: FileFingerprint {
                        device: 1,
                        inode: 200,
                        len: other.len() as u64,
                        sha256: digest(other.as_bytes()),
                    },
                },
                note_id: Some(other_id),
                quote: Some(other_quote),
            },
        ],
    }
}

#[test]
fn applied_intake_conflict_uses_historical_installation_and_retains_canonical_job_through_backup() {
    let mut s = setup(true);
    let canonical =
        serde_json::to_vec(&s.store.inbox_action(s.capture.id).unwrap().unwrap()).unwrap();
    assert_eq!(
        s.store.inbox_conflict_source(s.capture.id).unwrap(),
        s.source
    );
    let input = finding(&s);
    let created = s.store.create_finding(&input).unwrap();
    assert_eq!(created.draft.evidence[0].source, s.source);
    let original_id = brn_store::work::inbox_source::read_provenance(&s.capture.source_text)
        .unwrap()
        .unwrap()
        .item_id;
    assert!(
        s.store.inbox_removal_snapshot(original_id).is_err(),
        "legacy cleanup certificates must still refuse private intake"
    );
    let mut copied = input.clone();
    copied.request.id = Uuid::new_v4();
    copied.evidence[0].source.fingerprint.inode += 1;
    assert!(
        s.store.create_finding(&copied).is_err(),
        "equal bytes cannot renew an installation"
    );
    let closed = s
        .store
        .close_finding(&CloseFindingRequest {
            expected: created.stamp(),
            state: FindingState::Resolved,
        })
        .unwrap();
    assert_eq!(s.store.create_finding(&input).unwrap(), closed);
    assert_eq!(
        serde_json::to_vec(&s.store.inbox_action(s.capture.id).unwrap().unwrap()).unwrap(),
        canonical
    );
    let Setup {
        dir,
        store,
        capture,
        source,
        ..
    } = s;
    drop(store);
    let (store, checked) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.finding(closed.draft.request.id).unwrap(),
        Some(closed.clone())
    );
    drop(store);
    std::fs::write(dir.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    assert_eq!(store.inbox_conflict_source(capture.id).unwrap(), source);
    assert_eq!(store.create_finding(&input).unwrap(), closed);
    assert_eq!(
        serde_json::to_vec(&store.inbox_action(capture.id).unwrap().unwrap()).unwrap(),
        canonical
    );
}

fn replace_journal(s: &Setup, journal: &ApplyJournal) {
    let bytes = serde_json::to_vec(journal).unwrap();
    raw(s.dir.path())
        .execute(
            "UPDATE proposal_applies SET journal_json=?2,journal_sha256=?3 WHERE operation_id=?1",
            params![
                journal.request.operation_id.to_string(),
                bytes,
                digest(&bytes).as_slice()
            ],
        )
        .unwrap();
}

#[test]
fn applied_intake_conflict_refuses_pending_missing_ambiguous_incomplete_and_mismatched_lineage() {
    for mode in [
        "pending",
        "missing",
        "ambiguous",
        "incomplete",
        "stamp",
        "text",
        "path",
    ] {
        let mut s = setup(mode != "pending");
        match mode {
            "pending" => {}
            "missing" => {
                raw(s.dir.path())
                    .execute(
                        "DELETE FROM proposal_applies WHERE operation_id=?1",
                        [s.journal.as_ref().unwrap().request.operation_id.to_string()],
                    )
                    .unwrap();
            }
            "incomplete" => {
                let mut broken = s.journal.clone().unwrap();
                broken.prepared = None;
                replace_journal(&s, &broken);
            }
            "ambiguous" => {
                let mut duplicate = s.journal.clone().unwrap();
                duplicate.request.operation_id = Uuid::new_v4();
                duplicate.receipt.as_mut().unwrap().operation_id = duplicate.request.operation_id;
                let bytes = serde_json::to_vec(&duplicate).unwrap();
                let request = serde_json::to_vec(&duplicate.request).unwrap();
                raw(s.dir.path()).execute("INSERT INTO proposal_applies(operation_id,proposal_id,outcome,request_sha256,journal_json,journal_sha256) VALUES(?1,?2,'applied',?3,?4,?5)", params![duplicate.request.operation_id.to_string(), duplicate.approved.draft.id.to_string(), digest(&request).as_slice(), bytes, digest(&bytes).as_slice()]).unwrap();
            }
            _ => {
                s.capture.id = Uuid::new_v4();
                let intake = s.capture.intake.as_mut().unwrap();
                match mode {
                    "stamp" => intake.source_proposal.version += 1,
                    "text" => {
                        s.capture.source_text.push_str("Later changed text\n");
                        intake.source_text_sha256 = digest(s.capture.source_text.as_bytes());
                    }
                    "path" => intake.source_path = "Sources/other.md".into(),
                    _ => unreachable!(),
                }
                s.store
                    .reserve_inbox_action(&s.capture, "Different exact synthetic capture")
                    .unwrap();
            }
        }
        assert!(
            s.store.inbox_conflict_source(s.capture.id).is_err(),
            "{mode}"
        );
        let draft = finding(&s);
        assert!(s.store.create_finding(&draft).is_err(), "{mode}");
        assert!(
            s.store.finding(draft.request.id).unwrap().is_none(),
            "{mode}"
        );
        assert!(
            s.store
                .findings(&FindingListRequest::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }
}

#[test]
fn applied_intake_conflict_binds_its_source_vault_and_preserves_historical_receipt_after_current_review_changes()
 {
    let mut s = setup(true);
    let input = finding(&s);
    let mut wrong = input.clone();
    wrong.vault.id = Uuid::new_v4();
    assert!(s.store.create_finding(&wrong).is_err());
    let created = s.store.create_finding(&input).unwrap();
    let proposal_id = s.capture.intake.as_ref().unwrap().source_proposal.id;
    let conn = raw(s.dir.path());
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT record_json FROM proposals WHERE id=?1",
            [proposal_id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    // Synthetically valid later operational state: historical qualification must
    // use the immutable journal rather than a current eligibility decision.
    stored["record"]["state"] = serde_json::json!("rejected");
    let bytes = serde_json::to_vec(&stored).unwrap();
    conn.execute(
        "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
        params![proposal_id.to_string(), bytes, digest(&bytes).as_slice()],
    )
    .unwrap();
    assert_eq!(
        s.store.inbox_conflict_source(s.capture.id).unwrap(),
        s.source
    );
    assert_eq!(
        s.store.finding(input.request.id).unwrap(),
        Some(created.clone())
    );
    assert!(
        s.store
            .close_finding(&CloseFindingRequest {
                expected: created.stamp(),
                state: FindingState::Dismissed
            })
            .is_ok()
    );
}
