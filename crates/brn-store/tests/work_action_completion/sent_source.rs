use super::*;
use brn_store::{
    MAX_NOTE_BYTES,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        action_completion::SentSourceBinding,
        inbox::{InboxCapture, InboxCopy, InboxKind},
        inbox_processing::InboxConversionFormat,
        inbox_source::InboxSourceBinding,
        proposal_apply::{ApplyJournal, ApplyMemberProof},
        proposals::{NoteChange, SourceVersion},
    },
};

fn source_draft(store: &mut WorkStore, kind: InboxKind, original_text: &str) -> ProposalDraft {
    let original = store
        .capture_inbox(&InboxCapture {
            id: Uuid::new_v4(),
            kind,
            title: "Actual sent text".into(),
            original_name: None,
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: original_text.len() as u64,
                sha256: Sha256::digest(original_text.as_bytes()).into(),
            },
        })
        .unwrap();
    let converted = format!("```text\n{original_text}\n```\n");
    let binding = InboxSourceBinding {
        extraction: None,
        visual: None,
        batch_id: Uuid::new_v4(),
        index: 0,
        original,
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: converted.len() as u64,
        sha256: Sha256::digest(converted.as_bytes()).into(),
        note_id: Uuid::new_v4(),
    };
    let text = binding.markdown(&converted).unwrap();
    ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: Some(Box::new(binding)),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: VaultIdentity {
                device: 1,
                inode: 2,
            },
        }),
        title: "Retain exact actual text".into(),
        changes: vec![NoteChange::Create {
            path: "Sources/sent.md".into(),
            parent: VaultIdentity {
                device: 1,
                inode: 2,
            },
            text,
        }],
        sources: vec![],
        action_changes: vec![],
    }
}
fn apply_source(store: &mut WorkStore, draft: &ProposalDraft) -> (SourceVersion, ApplyJournal) {
    let reviewed = store.create_proposal(draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: reviewed.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    let text = draft.changes[0].text().unwrap();
    let source = SourceVersion {
        path: draft.changes[0].path().to_owned(),
        fingerprint: FileFingerprint {
            device: 1,
            inode: 80,
            len: text.len() as u64,
            sha256: Sha256::digest(text.as_bytes()).into(),
        },
    };
    store
        .record_proposal_prepared(
            request.operation_id,
            std::slice::from_ref(&source.fingerprint),
        )
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(source.fingerprint.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    let journal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    (source, journal)
}
fn sent_request(store: &mut WorkStore) -> (CompleteActionRequest, ApplyJournal) {
    let draft = source_draft(
        store,
        InboxKind::Text,
        "Different from my approved reply draft. λ",
    );
    let (source, journal) = apply_source(store, &draft);
    let before = approved(store);
    assert_eq!(before.data.state, ActionState::Waiting);
    let mut request = complete_request(&before);
    request.sent_source = Some(store.sent_source_binding(&source).unwrap());
    (request, journal)
}
fn change_current_source_title(dir: &Path, journal: &ApplyJournal) {
    let raw = Connection::open(dir.join("brn.sqlite")).unwrap();
    let bytes: Vec<u8> = raw
        .query_row(
            "SELECT record_json FROM proposals WHERE id=?1",
            [journal.approved.draft.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    #[derive(serde::Deserialize, serde::Serialize)]
    struct StoredProposal {
        creation_sha256: [u8; 32],
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        original_create_paths: Vec<brn_store::work::proposals::OriginalCreatePath>,
        record: brn_store::work::proposals::ProposalRecord,
    }
    let mut stored: StoredProposal = serde_json::from_slice(&bytes).unwrap();
    stored.record.draft.title.push_str(" later title");
    stored.record.version += 1;
    let bytes = serde_json::to_vec(&stored).unwrap();
    raw.execute(
        "UPDATE proposals SET record_json=?1,record_sha256=?2 WHERE id=?3",
        params![
            &bytes,
            Sha256::digest(&bytes).as_slice(),
            stored.record.draft.id.to_string()
        ],
    )
    .unwrap();
}

#[test]
fn omitted_and_none_keep_historical_canonical_bytes_and_hashes() {
    let (_, mut store) = fixture();
    let before = approved(&mut store);
    let request = complete_request(&before);
    let historical = format!(
        "{{\"operation_id\":\"{}\",\"before\":{}}}",
        request.operation_id,
        serde_json::to_string(&before).unwrap()
    );
    assert_eq!(serde_json::to_vec(&request).unwrap(), historical.as_bytes());
    let omitted: CompleteActionRequest = serde_json::from_str(&historical).unwrap();
    assert_eq!(omitted, request);
    let mut with_null = serde_json::to_value(&request).unwrap();
    with_null["sent_source"] = serde_json::Value::Null;
    let with_null: CompleteActionRequest = serde_json::from_value(with_null).unwrap();
    assert_eq!(
        serde_json::to_vec(&with_null).unwrap(),
        historical.as_bytes()
    );
    let completion = store.complete_action_with(&omitted, 0, |_| Ok(())).unwrap();
    assert_eq!(
        store.action_completion_for(&with_null).unwrap(),
        Some(completion)
    );
    assert_eq!(
        Sha256::digest(serde_json::to_vec(&request).unwrap()),
        Sha256::digest(historical.as_bytes())
    );
}

#[test]
fn sent_completion_appends_once_and_preserves_thread_every_other_field_and_origin() {
    for already_linked in [false, true] {
        let (dir, mut store) = fixture();
        let (mut request, _) = sent_request(&mut store);
        let binding = request.sent_source.clone().unwrap();
        store.validate_sent_source_binding(&binding).unwrap();
        if already_linked {
            request.before.version += 1;
            request.before.data.sources = (0..63).map(|_| Uuid::new_v4()).collect();
            request.before.data.sources.push(binding.note_id);
            put_action(dir.path(), &request.before);
        }
        let before = request.before.as_ref().clone();
        let completion = store
            .complete_action_with(&request, before.updated_at_ms + 1, |_| Ok(()))
            .unwrap();
        let mut expected = before;
        expected.version += 1;
        if !already_linked {
            expected.data.sources.push(binding.note_id);
        }
        expected.data.state = ActionState::Completed;
        expected.updated_at_ms += 1;
        expected.waiting_since_ms = None;
        expected.completed_at_ms = Some(expected.updated_at_ms);
        assert_eq!(completion.after, expected);
        assert_eq!(
            completion
                .after
                .data
                .sources
                .iter()
                .filter(|id| **id == binding.note_id)
                .count(),
            1
        );
        assert_eq!(store.sent_source_binding(&binding.source).unwrap(), binding);
    }
}

#[test]
fn exact_replay_precedes_fresh_source_check_but_changed_binding_conflicts() {
    let (dir, mut store) = fixture();
    let (request, journal) = sent_request(&mut store);
    let completion = store.complete_action_with(&request, 0, |_| Ok(())).unwrap();
    change_current_source_title(dir.path(), &journal);
    assert!(
        store
            .validate_sent_source_binding(request.sent_source.as_ref().unwrap())
            .is_err()
    );
    assert_eq!(
        store
            .complete_action_with(&request, u64::MAX, |_| panic!("replay published"))
            .unwrap(),
        completion
    );
    let mut changed = request.clone();
    changed
        .sent_source
        .as_mut()
        .unwrap()
        .source
        .fingerprint
        .inode += 1;
    assert!(matches!(
        store.complete_action_with(&changed, 0, |_| panic!("changed replay published")),
        Err(Error::OperationConflict(_))
    ));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store
            .complete_action_with(&request, u64::MAX, |_| panic!("restart replay published"))
            .unwrap(),
        completion
    );
}

#[test]
fn malformed_binding_wrong_approval_and_source_limit_refuse_before_publication() {
    for case in [
        "thread",
        "count",
        "path",
        "path-bound",
        "nil",
        "approval",
        "version",
        "id",
        "hash",
        "length",
        "current",
    ] {
        let (dir, mut store) = fixture();
        let (mut request, journal) = sent_request(&mut store);
        match case {
            "thread" => {
                request.before.version += 1;
                request.before.data.thread = None;
                put_action(dir.path(), &request.before);
            }
            "count" => {
                request.before.version += 1;
                request.before.data.sources = (0..64).map(|_| Uuid::new_v4()).collect();
                put_action(dir.path(), &request.before);
            }
            "path" => request.sent_source.as_mut().unwrap().source.path = "../sent.md".into(),
            "path-bound" => {
                request.sent_source.as_mut().unwrap().source.path =
                    format!("{}.md", "λ".repeat(255))
            }
            "nil" => {
                request
                    .sent_source
                    .as_mut()
                    .unwrap()
                    .source_approval
                    .operation_id = Uuid::nil()
            }
            "approval" => {
                request
                    .sent_source
                    .as_mut()
                    .unwrap()
                    .source_approval
                    .operation_id = Uuid::new_v4()
            }
            "version" => {
                request
                    .sent_source
                    .as_mut()
                    .unwrap()
                    .source_approval
                    .expected
                    .version += 1
            }
            "id" => request.sent_source.as_mut().unwrap().note_id = Uuid::new_v4(),
            "hash" => {
                request
                    .sent_source
                    .as_mut()
                    .unwrap()
                    .source
                    .fingerprint
                    .sha256[0] ^= 1
            }
            "length" => request.sent_source.as_mut().unwrap().source.fingerprint.len += 1,
            "current" => change_current_source_title(dir.path(), &journal),
            _ => unreachable!(),
        }
        let before = store.action(request.before.origin.id).unwrap();
        assert!(
            store
                .complete_action_with(&request, 0, |_| panic!("{case} published"))
                .is_err(),
            "{case}"
        );
        assert_eq!(store.action(request.before.origin.id).unwrap(), before);
        assert_eq!(count(dir.path()), 0);
    }
}

#[test]
fn binding_requires_plain_text_and_one_exact_applied_receipt() {
    let (_, mut store) = fixture();
    let email = source_draft(
        &mut store,
        InboxKind::Email,
        "email is not explicit Text capture",
    );
    let (email, _) = apply_source(&mut store, &email);
    assert!(store.sent_source_binding(&email).is_err());
    let draft = source_draft(&mut store, InboxKind::Text, "actual sent text");
    let reviewed = store.create_proposal(&draft).unwrap();
    let source = SourceVersion {
        path: draft.changes[0].path().to_owned(),
        fingerprint: FileFingerprint {
            device: 1,
            inode: 2,
            len: draft.changes[0].text().unwrap().len() as u64,
            sha256: Sha256::digest(draft.changes[0].text().unwrap().as_bytes()).into(),
        },
    };
    assert!(
        store.sent_source_binding(&source).is_err(),
        "Draft source has no Applied receipt"
    );
    store.reject_proposal(reviewed.stamp()).unwrap();
    let mut first = draft.clone();
    first.id = Uuid::new_v4();
    let (source, _) = apply_source(&mut store, &first);
    assert!(store.sent_source_binding(&source).is_ok());
    let mut second = draft;
    second.id = Uuid::new_v4();
    apply_source(&mut store, &second);
    assert!(
        store.sent_source_binding(&source).is_err(),
        "duplicate exact approvals must refuse"
    );
}

#[test]
fn historical_restore_requires_source_lineage_and_preserves_equal_fork_checks() {
    let (_, mut source_store) = fixture();
    let (request, journal) = sent_request(&mut source_store);
    let completion = source_store
        .complete_action_with(&request, 0, |_| Ok(()))
        .unwrap();
    for older in [false, true] {
        let (dir, mut target) = fixture();
        if older {
            put_action(dir.path(), &request.before);
        }
        assert!(target.restore_action_completion(&completion).is_err());
        assert_eq!(count(dir.path()), 0);
        target.restore_proposal_apply(&journal).unwrap();
        change_current_source_title(dir.path(), &journal);
        assert_eq!(
            target.restore_action_completion(&completion).unwrap(),
            completion
        );
        assert_eq!(
            target.restore_action_completion(&completion).unwrap(),
            completion
        );
        assert_eq!(
            target.action_completion_for(&request).unwrap(),
            Some(completion.clone())
        );
        drop(target);
        let (target, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            target.action(request.before.origin.id).unwrap(),
            Some(completion.after.clone())
        );
    }
    let (dir, mut target) = fixture();
    target.restore_proposal_apply(&journal).unwrap();
    let mut fork = completion.after.clone();
    fork.data.description.push('x');
    put_action(dir.path(), &fork);
    assert!(target.restore_action_completion(&completion).is_err());
    assert_eq!(target.action(fork.origin.id).unwrap(), Some(fork));
    assert_eq!(count(dir.path()), 0);
}

#[test]
fn failed_publication_retains_source_without_completing_and_historical_import_recovers() {
    let (dir, mut store) = fixture();
    let (request, _) = sent_request(&mut store);
    let mut published = None;
    assert!(
        store
            .complete_action_with(&request, 0, |candidate| {
                published = Some(candidate.clone());
                Err(Error::StateChanged(
                    "synthetic after-publication failure".into(),
                ))
            })
            .is_err()
    );
    assert_eq!(
        store.action(request.before.origin.id).unwrap(),
        Some(*request.before.clone())
    );
    assert_eq!(count(dir.path()), 0);
    assert_eq!(
        store
            .sent_source_binding(&request.sent_source.as_ref().unwrap().source)
            .unwrap(),
        *request.sent_source.as_ref().unwrap()
    );
    let completion = published.unwrap();
    assert_eq!(
        store.restore_action_completion(&completion).unwrap(),
        completion
    );
    assert_eq!(count(dir.path()), 1);
}

#[test]
fn full_escaped_action_pair_and_near_one_mib_source_remain_complete() {
    let (dir, mut store) = fixture();
    let draft = source_draft(
        &mut store,
        InboxKind::Text,
        &"x".repeat(MAX_NOTE_BYTES - 4096),
    );
    let (source, journal) = apply_source(&mut store, &draft);
    assert!(source.fingerprint.len > (MAX_NOTE_BYTES - 4096) as u64);
    let mut before = approved(&mut store);
    before.data.description = "\u{1}".repeat(64 * 1024);
    before.data.title = "\u{1}".repeat(512);
    before.data.owner = Some("\u{1}".repeat(512));
    before.origin.data = before.data.clone();
    put_action(dir.path(), &before);
    let mut request = complete_request(&before);
    request.sent_source = Some(store.sent_source_binding(&source).unwrap());
    let completion = store
        .complete_action_with(&request, i64::MAX as u64, |_| Ok(()))
        .unwrap();
    let bytes = serde_json::to_vec(&completion).unwrap();
    assert!(
        bytes.len() > 1_590_000,
        "full worst-case escaped pair was reduced: {}",
        bytes.len()
    );
    assert!(bytes.len() < 4 * 1024 * 1024);
    assert!(
        serde_json::to_vec(request.sent_source.as_ref().unwrap())
            .unwrap()
            .len()
            <= 8192
    );
    let (_, mut target) = fixture();
    target.restore_proposal_apply(&journal).unwrap();
    assert_eq!(
        target.restore_action_completion(&completion).unwrap(),
        completion
    );
    assert_eq!(
        target.action(before.origin.id).unwrap(),
        Some(completion.after)
    );
}

#[test]
fn shape_bounds_cover_worst_encoded_path_and_strict_unknown_fields() {
    let (_, mut store) = fixture();
    let (request, _) = sent_request(&mut store);
    let mut binding = request.sent_source.unwrap();
    binding.source.path = format!("{}.md", "\u{1}".repeat(509));
    binding.source.fingerprint.device = u64::MAX;
    binding.source.fingerprint.inode = u64::MAX;
    binding.source.fingerprint.len = MAX_NOTE_BYTES as u64;
    binding.source.fingerprint.sha256 = [255; 32];
    binding.source_approval.expected.version = u64::MAX - 3;
    binding.validate().unwrap();
    let bytes = serde_json::to_vec(&binding).unwrap();
    assert!(bytes.len() > 3000 && bytes.len() <= 8192);
    binding.source.path.insert(0, 'x');
    assert!(binding.validate().is_err());
    binding.source.path = "sent.md".into();
    for len in [0, MAX_NOTE_BYTES as u64 + 1] {
        binding.source.fingerprint.len = len;
        assert!(binding.validate().is_err());
    }
    binding.source.fingerprint.len = 1;
    for version in [0, u64::MAX - 2, u64::MAX] {
        binding.source_approval.expected.version = version;
        assert!(binding.validate().is_err());
    }
    let mut value = serde_json::to_value(&binding).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<SentSourceBinding>(value).is_err());
}

#[test]
fn historical_restore_refuses_forged_source_bindings_without_action_effects() {
    let (_, mut source_store) = fixture();
    let (request, journal) = sent_request(&mut source_store);
    let completion = source_store
        .complete_action_with(&request, 0, |_| Ok(()))
        .unwrap();
    for case in ["approval", "stamp", "path", "id", "hash", "length"] {
        let (dir, mut target) = fixture();
        target.restore_proposal_apply(&journal).unwrap();
        let mut forged = completion.clone();
        let binding = forged.request.sent_source.as_mut().unwrap();
        match case {
            "approval" => binding.source_approval.operation_id = Uuid::new_v4(),
            "stamp" => binding.source_approval.expected.version += 1,
            "path" => binding.source.path = "Sources/other.md".into(),
            "id" => {
                let id = Uuid::new_v4();
                *forged.after.data.sources.last_mut().unwrap() = id;
                binding.note_id = id;
            }
            "hash" => binding.source.fingerprint.sha256[0] ^= 1,
            "length" => binding.source.fingerprint.len += 1,
            _ => unreachable!(),
        }
        forged.validate().unwrap();
        assert!(target.restore_action_completion(&forged).is_err(), "{case}");
        assert!(target.action(request.before.origin.id).unwrap().is_none());
        assert_eq!(count(dir.path()), 0);
    }
}
