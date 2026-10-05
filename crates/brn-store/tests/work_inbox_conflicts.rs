use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        findings::*,
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
        inbox_processing::InboxConversionFormat,
        inbox_source::InboxSourceBinding,
        proposals::SourceVersion,
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn proof(path: &str, text: &str, inode: u64) -> SourceVersion {
    SourceVersion {
        path: path.into(),
        fingerprint: FileFingerprint {
            device: 1,
            inode,
            len: text.len() as u64,
            sha256: digest(text.as_bytes()),
        },
    }
}
fn quote(text: &str, wording: &str) -> FindingQuote {
    let start_byte = text.find(wording).unwrap();
    FindingQuote {
        start_byte,
        end_byte: start_byte + wording.len(),
        quote: wording.into(),
    }
}
fn capture() -> InboxActionCapture {
    let body = "```text\nBlue õ 🦀\n```\n";
    let binding = InboxSourceBinding {
        batch_id: Uuid::new_v4(),
        index: 0,
        original: InboxItem {
            capture: InboxCapture {
                id: Uuid::new_v4(),
                kind: InboxKind::Email,
                title: "Synthetic color mail".into(),
                original_name: None,
                copy: InboxCopy {
                    directory: "/synthetic/inbox".into(),
                    directory_device: 1,
                    directory_inode: 2,
                    file_device: 1,
                    file_inode: 3,
                    byte_len: 4,
                    sha256: digest(b"blue"),
                },
            },
            received_at_ms: 1,
        },
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: body.len() as u64,
        sha256: digest(body.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    let text = binding.markdown(body).unwrap();
    InboxActionCapture {
        purpose: InboxAnalysisPurpose::KnowledgeAndActions,
        id: Uuid::new_v4(),
        conversation: None,
        source: proof("Sources/color.md", &text, 11),
        source_text: text,
        provider: "chatgpt".into(),
        model: "gpt-6-luna".into(),
        effort: "medium".into(),
    }
}
fn draft(capture: &InboxActionCapture) -> FindingDraft {
    let other_id = Uuid::new_v4();
    let other_text = format!("---\nbrn_id: {other_id}\n---\n# Decision\nGreen 日本語\n");
    let source_quote = quote(&capture.source_text, "Blue õ 🦀");
    let other_quote = quote(&other_text, "Green 日本語");
    let title = "Unresolved color õ".to_owned();
    let summary = "The saved sources disagree; neither is established as the winner. 🦀".to_owned();
    FindingDraft {
        request: CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::InboxConflict {
                analysis_id: capture.id,
                title: title.clone(),
                summary: summary.clone(),
                source_quote: source_quote.clone(),
                other_path: "decision.md".into(),
                other_quote: other_quote.clone(),
            },
        },
        vault: VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: VaultIdentity {
                device: 1,
                inode: 1,
            },
        },
        title,
        summary,
        evidence: vec![
            FindingEvidence {
                source: capture.source.clone(),
                note_id: Some(capture.note_id().unwrap()),
                quote: Some(source_quote),
            },
            FindingEvidence {
                source: proof("decision.md", &other_text, 12),
                note_id: Some(other_id),
                quote: Some(other_quote),
            },
        ],
    }
}
fn reserve(store: &mut WorkStore, capture: &InboxActionCapture) {
    store
        .reserve_inbox_action(capture, "Compare these synthetic saved sources")
        .unwrap();
}

#[test]
fn conflict_creation_keeps_two_exact_proofs_and_replays_closed_work_after_reopen_and_backup() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let input = draft(&capture);
    let created = store.create_finding(&input).unwrap();
    assert_eq!(created.draft, input);
    assert_eq!(created.state, FindingState::Open);
    assert_eq!(
        store.inbox_conflicts(capture.id).unwrap(),
        vec![created.clone()]
    );
    let other_id = input.evidence[1].note_id.unwrap();
    let by_source = store
        .note_conflicts(
            &input.vault,
            &input.evidence[0].source.path,
            input.evidence[0].note_id.unwrap(),
            &FindingListRequest::default(),
        )
        .unwrap();
    let by_other = store
        .note_conflicts(
            &input.vault,
            "renamed.md",
            other_id,
            &FindingListRequest::default(),
        )
        .unwrap();
    assert_eq!(by_source, by_other);
    assert_eq!(by_other.entries, vec![created.clone()]);
    assert_eq!(by_other.open_count, 1);
    let closed = store
        .close_finding(&CloseFindingRequest {
            expected: created.stamp(),
            state: FindingState::Dismissed,
        })
        .unwrap();
    assert_eq!(store.create_finding(&input).unwrap(), closed);
    assert!(
        store
            .note_conflicts(
                &input.vault,
                "decision.md",
                other_id,
                &FindingListRequest::default()
            )
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        store.inbox_conflicts(capture.id).unwrap(),
        vec![closed.clone()]
    );
    let mut changed = input.clone();
    changed.summary.push_str(" Changed intent");
    if let FindingOrigin::InboxConflict { summary, .. } = &mut changed.request.origin {
        *summary = changed.summary.clone();
    }
    assert!(matches!(
        store.create_finding(&changed),
        Err(Error::OperationConflict(_))
    ));
    drop(store);
    let (mut store, checked) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.create_finding(&input).unwrap(), closed);
    drop(store);
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (mut store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup));
    assert_eq!(store.create_finding(&input).unwrap(), closed);
}

fn origin_mut(
    input: &mut FindingDraft,
) -> (
    &mut Uuid,
    &mut String,
    &mut String,
    &mut FindingQuote,
    &mut String,
    &mut FindingQuote,
) {
    let FindingOrigin::InboxConflict {
        analysis_id,
        title,
        summary,
        source_quote,
        other_path,
        other_quote,
    } = &mut input.request.origin
    else {
        panic!("conflict fixture")
    };
    (
        analysis_id,
        title,
        summary,
        source_quote,
        other_path,
        other_quote,
    )
}
fn raw(data: &std::path::Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}
// Recompute both hashes so semantic tests reach the contextual validator rather
// than merely failing a cryptographic envelope check.
fn rewrite(conn: &Connection, record: &FindingRecord) {
    let bytes = serde_json::to_vec(record).unwrap();
    let creation = serde_json::to_vec(&record.draft).unwrap();
    conn.execute("UPDATE findings SET created_at_ms=?2,record_json=?3,record_sha256=?4,creation_sha256=?5 WHERE id=?1", params![record.draft.request.id.to_string(), record.created_at_ms as i64, bytes, digest(&bytes).as_slice(), digest(&creation).as_slice()]).unwrap();
}

#[test]
fn conflict_shape_and_complete_intent_refuse_atomically() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let input = draft(&capture);
    for change in 0..20 {
        let mut bad = input.clone();
        match change {
            0 => bad.evidence.clear(),
            1 => {
                bad.evidence.pop();
            }
            2 => bad.evidence.push(bad.evidence[1].clone()),
            3 => bad.evidence.swap(0, 1),
            4 => bad.evidence[1].note_id = None,
            5 => bad.evidence[1].note_id = Some(Uuid::nil()),
            6 => bad.evidence[1].note_id = bad.evidence[0].note_id,
            7 => {
                bad.evidence[1].source.path = bad.evidence[0].source.path.clone();
                origin_mut(&mut bad).4.clone_from(&capture.source.path);
            }
            8 => bad.evidence[0].quote = None,
            9 => bad.evidence[1].quote.as_mut().unwrap().quote = "Different".into(),
            10 => bad.title.push('!'),
            11 => bad.summary.push('!'),
            12 => *origin_mut(&mut bad).0 = Uuid::nil(),
            13 => *origin_mut(&mut bad).1 = "x".repeat(513),
            14 => *origin_mut(&mut bad).2 = "x".repeat(16 * 1024 + 1),
            15 => *origin_mut(&mut bad).4 = "../decision.md".into(),
            16 => origin_mut(&mut bad).3.end_byte += 1,
            17 => origin_mut(&mut bad).5.quote.clear(),
            18 => origin_mut(&mut bad).5.end_byte = brn_store::MAX_NOTE_BYTES + 1,
            _ => {
                let q = origin_mut(&mut bad).5;
                q.start_byte = q.end_byte;
            }
        }
        assert!(bad.validate().is_err(), "shape change {change}");
        assert!(
            matches!(store.create_finding(&bad), Err(Error::Invalid(_))),
            "write change {change}"
        );
    }
    assert!(store.inbox_conflicts(capture.id).unwrap().is_empty());
    // Origin strings/quotes are retained twice; their duplication counts toward
    // the same whole-work cap, even when every individual field is valid.
    let mut capped = input.clone();
    let unduplicated = input.title.len()
        + input.summary.len()
        + input
            .evidence
            .iter()
            .map(|e| e.source.path.len() + e.quote.as_ref().unwrap().quote.len())
            .sum::<usize>();
    capped.vault.root = format!(
        "/{}",
        "x".repeat(brn_store::MAX_NOTE_BYTES - unduplicated - 2)
    )
    .into();
    assert!(capped.validate().is_err());
}

#[test]
fn contextual_source_body_capture_proof_and_purpose_are_checked_without_a_turn() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let input = draft(&capture);
    for change in 0..6 {
        let mut bad = input.clone();
        bad.request.id = Uuid::new_v4();
        match change {
            0 => *origin_mut(&mut bad).0 = Uuid::new_v4(),
            1 => bad.evidence[0].source.fingerprint.inode += 1,
            2 => bad.evidence[0].source.fingerprint.sha256 = [0; 32],
            3 => bad.evidence[0].note_id = Some(Uuid::new_v4()),
            4 => {
                let q = quote(&capture.source_text, "brn_kind");
                *origin_mut(&mut bad).3 = q.clone();
                bad.evidence[0].quote = Some(q);
            }
            _ => {
                let mut q = bad.evidence[0].quote.clone().unwrap();
                q.start_byte += 1;
                q.end_byte += 1;
                *origin_mut(&mut bad).3 = q.clone();
                bad.evidence[0].quote = Some(q);
            }
        }
        bad.validate().unwrap();
        assert!(
            matches!(store.create_finding(&bad), Err(Error::Invalid(_))),
            "context change {change}"
        );
    }
    // A same-length invented wording cannot be accepted as captured evidence.
    let mut bad = input.clone();
    let q = origin_mut(&mut bad).3;
    q.quote = "X".repeat(q.quote.len());
    bad.evidence[0].quote = Some(origin_mut(&mut bad).3.clone());
    assert!(matches!(store.create_finding(&bad), Err(Error::Invalid(_))));
    for kind in 0..3 {
        let mut captured = capture.clone();
        captured.id = Uuid::new_v4();
        match kind {
            0 => captured.purpose = InboxAnalysisPurpose::Actions,
            1 => {
                captured.source_text =
                    captured
                        .source_text
                        .replacen("brn_state: current", "brn_state: history", 1);
                captured.source = proof("Sources/history.md", &captured.source_text, 31);
            }
            _ => captured.source.path = "Archive/source.md".into(),
        }
        reserve(&mut store, &captured);
        let bad = draft(&captured);
        bad.validate().unwrap();
        assert!(
            matches!(store.create_finding(&bad), Err(Error::Invalid(_))),
            "capture kind {kind}"
        );
    }
    assert!(store.inbox_conflicts(capture.id).unwrap().is_empty());
    assert_eq!(store.create_finding(&input).unwrap().draft, input);
}

#[test]
fn hash_valid_contextual_damage_refuses_read_and_startup_without_replacing_main_or_backup() {
    for change in 0..4 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let capture = capture();
        reserve(&mut store, &capture);
        let input = draft(&capture);
        store.create_finding(&input).unwrap();
        drop(store);
        let (mut store, checked) = WorkStore::open(data.path()).unwrap();
        let mut record = store.finding(input.request.id).unwrap().unwrap();
        match change {
            0 => *origin_mut(&mut record.draft).0 = Uuid::new_v4(),
            1 => record.draft.evidence[0].source.fingerprint.inode += 1,
            2 => record.draft.evidence[0].note_id = Some(Uuid::new_v4()),
            _ => {
                let q = quote(&capture.source_text, "brn_kind");
                *origin_mut(&mut record.draft).3 = q.clone();
                record.draft.evidence[0].quote = Some(q);
            }
        }
        record.draft.validate().unwrap();
        rewrite(&raw(data.path()), &record);
        assert!(matches!(
            store.finding(input.request.id),
            Err(Error::Invalid(_))
        ));
        assert!(store.inbox_conflicts(capture.id).is_err());
        assert!(
            store
                .close_finding(&CloseFindingRequest {
                    expected: record.stamp(),
                    state: FindingState::Resolved
                })
                .is_err()
        );
        drop(store);
        let main = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backup = std::fs::read(&checked.backup).unwrap();
        assert!(
            matches!(WorkStore::open(data.path()), Err(Error::Invalid(_))),
            "semantic change {change}"
        );
        assert_eq!(std::fs::read(data.path().join("brn.sqlite")).unwrap(), main);
        assert_eq!(std::fs::read(&checked.backup).unwrap(), backup);
    }
}

#[test]
fn conflict_strict_json_rejects_unrecognized_or_duplicated_intent_and_quote_fields() {
    let input = draft(&capture());
    for pointer in [
        "/request/origin",
        "/request/origin/source_quote",
        "/request/origin/other_quote",
    ] {
        let mut bad = serde_json::to_value(&input).unwrap();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future_field".into(), true.into());
        assert!(
            serde_json::from_value::<FindingDraft>(bad).is_err(),
            "{pointer}"
        );
    }
    let encoded = serde_json::to_string(&input).unwrap();
    let duplicated = encoded.replacen(
        "\"other_path\":",
        "\"other_path\":\"duplicate.md\",\"other_path\":",
        1,
    );
    assert!(serde_json::from_str::<FindingDraft>(&duplicated).is_err());
    for text in [
        r#"{"kind":"identity_ambiguity","note_id":"11111111-1111-4111-8111-111111111111"}"#,
        r#"{"kind":"unresolved_link","path":"a.md","source_sha256":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"destination":"b.md","start_byte":1}"#,
    ] {
        let origin: FindingOrigin = serde_json::from_str(text).unwrap();
        assert_eq!(
            serde_json::to_string(&origin).unwrap(),
            text,
            "legacy canonical bytes must not change"
        );
    }
}

#[test]
fn note_lookup_matches_exact_vault_either_path_or_identity_and_has_bound_closed_cursors() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let base = draft(&capture);
    let mut records = Vec::new();
    for low in 1..=3 {
        let mut input = base.clone();
        input.request.id = Uuid::from_u128(low);
        let mut record = store.create_finding(&input).unwrap();
        record.created_at_ms = 1;
        record.updated_at_ms = 1;
        rewrite(&raw(data.path()), &record);
        records.push(record);
    }
    let mut foreign = base.clone();
    foreign.request.id = Uuid::new_v4();
    foreign.vault.identity.inode += 1;
    let foreign = store.create_finding(&foreign).unwrap();
    let source_id = capture.note_id().unwrap();
    let request = FindingListRequest {
        limit: 1,
        ..Default::default()
    };
    let first = store
        .note_conflicts(&base.vault, "renamed.md", source_id, &request)
        .unwrap();
    assert_eq!(first.entries, vec![records[2].clone()]);
    assert_eq!(first.open_count, 3);
    assert_eq!(first.next_before, Some(records[2].draft.request.id));
    store
        .close_finding(&CloseFindingRequest {
            expected: records[2].stamp(),
            state: FindingState::Resolved,
        })
        .unwrap();
    let next = FindingListRequest {
        before: first.next_before,
        ..request.clone()
    };
    let second = store
        .note_conflicts(&base.vault, "renamed.md", source_id, &next)
        .unwrap();
    assert_eq!(second.entries, vec![records[1].clone()]);
    assert_eq!(second.open_count, 2);
    let last = store
        .note_conflicts(
            &base.vault,
            "renamed.md",
            source_id,
            &FindingListRequest {
                before: second.next_before,
                ..request.clone()
            },
        )
        .unwrap();
    assert_eq!(last.entries, vec![records[0].clone()]);
    assert_eq!(last.next_before, None);
    let path_only = store
        .note_conflicts(
            &base.vault,
            "decision.md",
            Uuid::new_v4(),
            &FindingListRequest::default(),
        )
        .unwrap();
    assert_eq!(path_only.open_count, 2);
    assert_eq!(
        store
            .note_conflicts(
                &base.vault,
                "renamed.md",
                base.evidence[1].note_id.unwrap(),
                &FindingListRequest::default()
            )
            .unwrap(),
        path_only
    );
    for change in 0..3 {
        let mut vault = base.vault.clone();
        match change {
            0 => vault.id = Uuid::new_v4(),
            1 => vault.root = "/another/vault".into(),
            _ => vault.identity.device += 1,
        };
        assert!(
            store
                .note_conflicts(
                    &vault,
                    "decision.md",
                    source_id,
                    &FindingListRequest::default()
                )
                .unwrap()
                .entries
                .is_empty()
        );
    }
    for cursor in [foreign.draft.request.id, Uuid::new_v4()] {
        assert!(
            store
                .note_conflicts(
                    &base.vault,
                    "decision.md",
                    source_id,
                    &FindingListRequest {
                        before: Some(cursor),
                        ..request.clone()
                    }
                )
                .is_err()
        );
    }
    assert!(
        store
            .note_conflicts(&base.vault, "unrelated.md", Uuid::new_v4(), &next)
            .is_err()
    );
    for state in [
        None,
        Some(FindingState::Dismissed),
        Some(FindingState::Resolved),
    ] {
        assert!(
            store
                .note_conflicts(
                    &base.vault,
                    "decision.md",
                    source_id,
                    &FindingListRequest {
                        state,
                        ..request.clone()
                    }
                )
                .is_err()
        );
    }
    assert!(
        store
            .note_conflicts(&base.vault, "../decision.md", source_id, &request)
            .is_err()
    );
    assert!(
        store
            .note_conflicts(&base.vault, "decision.md", Uuid::nil(), &request)
            .is_err()
    );
    assert!(store.inbox_conflicts(Uuid::nil()).is_err());
    assert!(store.inbox_conflicts(Uuid::new_v4()).unwrap().is_empty());
    assert_eq!(store.inbox_conflicts(capture.id).unwrap().len(), 4);
}

#[test]
fn identical_request_id_refuses_each_changed_original_conflict_intent_after_closure() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let input = draft(&capture);
    let record = store.create_finding(&input).unwrap();
    let closed = store
        .close_finding(&CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Resolved,
        })
        .unwrap();
    for change in 0..6 {
        let mut changed = input.clone();
        match change {
            0 => {
                changed.title.push('!');
                *origin_mut(&mut changed).1 = changed.title.clone();
            }
            1 => {
                changed.summary.push('!');
                *origin_mut(&mut changed).2 = changed.summary.clone();
            }
            2 => *origin_mut(&mut changed).0 = Uuid::new_v4(),
            3 => {
                changed.evidence[1].source.path = "renamed.md".into();
                *origin_mut(&mut changed).4 = "renamed.md".into();
            }
            4 => {
                let q = quote(&capture.source_text, "Blue");
                *origin_mut(&mut changed).3 = q.clone();
                changed.evidence[0].quote = Some(q);
            }
            _ => {
                let q = origin_mut(&mut changed).5;
                q.quote = "Green".into();
                q.end_byte = q.start_byte + q.quote.len();
                changed.evidence[1].quote = Some(origin_mut(&mut changed).5.clone());
            }
        }
        changed.validate().unwrap();
        assert!(
            matches!(
                store.create_finding(&changed),
                Err(Error::OperationConflict(_))
            ),
            "intent change {change}"
        );
        assert_eq!(
            store.finding(input.request.id).unwrap(),
            Some(closed.clone())
        );
    }
}

#[test]
fn exact_case_paths_are_preserved_and_generic_findings_are_excluded_from_conflict_lookup() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut capture = capture();
    capture.source.path = "Note.md".into();
    reserve(&mut store, &capture);
    let mut input = draft(&capture);
    input.evidence[1].source.path = "note.md".into();
    *origin_mut(&mut input).4 = "note.md".into();
    let conflict = store.create_finding(&input).unwrap();
    let mut generic = input.clone();
    generic.request.id = Uuid::new_v4();
    generic.request.origin = FindingOrigin::IdentityAmbiguity {
        note_id: capture.note_id().unwrap(),
    };
    generic.evidence[1].note_id = generic.evidence[0].note_id;
    let generic = store.create_finding(&generic).unwrap();
    let by_case = store
        .note_conflicts(
            &input.vault,
            "note.md",
            Uuid::new_v4(),
            &FindingListRequest::default(),
        )
        .unwrap();
    assert_eq!(by_case.entries, vec![conflict.clone()]);
    assert_eq!(by_case.open_count, 1);
    assert!(
        store
            .note_conflicts(
                &input.vault,
                "NOTE.md",
                Uuid::new_v4(),
                &FindingListRequest::default()
            )
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(
        store
            .note_conflicts(
                &input.vault,
                "Note.md",
                capture.note_id().unwrap(),
                &FindingListRequest {
                    before: Some(generic.draft.request.id),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(store.inbox_conflicts(capture.id).unwrap(), vec![conflict]);
    // A range whose byte length is valid but begins inside a multibyte codepoint
    // is refused against the retained original, rather than repaired/guessed.
    let mut bad = input;
    bad.request.id = Uuid::new_v4();
    let start = capture.source_text.find('õ').unwrap() + 1;
    let q = FindingQuote {
        start_byte: start,
        end_byte: start + 1,
        quote: "X".into(),
    };
    *origin_mut(&mut bad).3 = q.clone();
    bad.evidence[0].quote = Some(q);
    bad.validate().unwrap();
    assert!(matches!(store.create_finding(&bad), Err(Error::Invalid(_))));
}

#[test]
fn hash_valid_conflict_context_damage_skips_newer_backup_during_physical_recovery() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let capture = capture();
    reserve(&mut store, &capture);
    let input = draft(&capture);
    let record = store.create_finding(&input).unwrap();
    let terminal = store
        .close_finding(&CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Dismissed,
        })
        .unwrap();
    drop(store);
    let (store, checked) = WorkStore::open(data.path()).unwrap();
    drop(store);
    let healthy_bytes = std::fs::read(&checked.backup).unwrap();
    let damaged_backup = data.path().join("backups/brn-9999999999999.sqlite");
    raw(data.path())
        .backup("main", &damaged_backup, None)
        .unwrap();
    let mut damaged = terminal.clone();
    *origin_mut(&mut damaged.draft).0 = Uuid::new_v4();
    rewrite(&Connection::open(&damaged_backup).unwrap(), &damaged);
    let damaged_bytes = std::fs::read(&damaged_backup).unwrap();
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(checked.backup.clone()));
    assert_eq!(store.finding(input.request.id).unwrap(), Some(terminal));
    assert_eq!(std::fs::read(&damaged_backup).unwrap(), damaged_bytes);
    assert_eq!(std::fs::read(&checked.backup).unwrap(), healthy_bytes);
}
