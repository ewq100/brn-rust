use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    note_identity,
    work::{proposal_apply::*, proposal_rewrite::*, proposals::*},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn draft(create: &str, replace: &str) -> ProposalDraft {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    let original = "\u{feff}Original 日本語\r\n";
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        },
        title: "Exact identity review λ".into(),
        changes: vec![
            NoteChange::Create {
                path: "new.md".into(),
                parent: parent.clone(),
                text: create.into(),
            },
            NoteChange::Replace {
                path: "existing.md".into(),
                parent,
                before: fingerprint(original, 2),
                before_text: original.into(),
                text: replace.into(),
            },
        ],
        sources: vec![],
    }
}

fn edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
    }
}

fn comment(store: &mut WorkStore, record: &ProposalRecord) -> ProposalRecord {
    let text = record.draft.changes[0].text().unwrap();
    let start = text.find("日本語").unwrap();
    store
        .add_proposal_comment(&CommentRequest {
            expected: record.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Preserve this wording 🦀\r\n".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start,
                    end: start + "日本語".len(),
                    quote: "日本語".into(),
                }),
            },
        })
        .unwrap()
}

#[test]
fn refused_full_edit_keeps_all_members_version_comments_and_restart_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let original = "\u{feff}---\r\ncustom: '日本語 🦀'\r\n---\r\nBody 日本語 λ\r\n";
    let create = note_identity::assign(original, first_id).unwrap();
    let replace = note_identity::assign(original, second_id).unwrap();
    let draft = draft(&create, &replace);
    let first = store.create_proposal(&draft).unwrap();
    let current = comment(&mut store, &first);

    let mut refused = edit(&current);
    refused.title = "Should not be installed".into();
    refused.texts[0] = Some(create.replace("Body 日本語", "Valid later 日本語"));
    refused.texts[1] = Some(original.into());
    assert!(matches!(
        store.edit_proposal(&refused),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current.clone()));
    refused = edit(&current);
    refused.texts[0] = Some(create.replace(&first_id.to_string(), &Uuid::new_v4().to_string()));
    assert!(matches!(
        store.rewrite_proposal(&refused),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current.clone()));

    let mut accepted = edit(&current);
    accepted.texts[0] = Some(create.replace("Body 日本語", "Reviewed 日本語"));
    let updated = store.edit_proposal(&accepted).unwrap();
    assert_eq!(updated.version, current.version + 1);
    assert_eq!(
        updated.draft.changes[0].text(),
        accepted.texts[0].as_deref()
    );
    assert_eq!(updated.draft.changes[1], current.draft.changes[1]);
    assert_eq!(updated.comments[0].text, current.comments[0].text);
    let CommentTarget::Text(anchor) = &current.comments[0].target else {
        panic!("expected original anchor");
    };
    assert_eq!(
        updated.comments[0].target,
        CommentTarget::Unresolved(anchor.clone())
    );
    assert_eq!(
        note_identity::read(updated.draft.changes[0].text().unwrap()).unwrap(),
        Some(first_id)
    );
    assert_eq!(
        note_identity::read(updated.draft.changes[1].text().unwrap()).unwrap(),
        Some(second_id)
    );
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(updated.clone()));
    assert_eq!(store.create_proposal(&draft).unwrap(), updated);
}

#[test]
fn legacy_malformed_records_allow_exact_text_and_unmanaged_records_can_gain_identity() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let malformed = "---\nbrn_id: historic malformed value\n---\n日本語 body\n";
    let original = draft(malformed, "unmanaged replacement λ\n");
    let current = store.create_proposal(&original).unwrap();
    assert_eq!(store.edit_proposal(&edit(&current)).unwrap(), current);
    let mut title_only = edit(&current);
    title_only.title = "Review exact legacy bytes".into();
    let current = store.edit_proposal(&title_only).unwrap();
    assert_eq!(current.draft.changes[0].text(), Some(malformed));
    let mut refused = edit(&current);
    refused.texts[0] = Some(malformed.replace("body", "changed body"));
    assert!(store.edit_proposal(&refused).is_err());
    assert_eq!(store.proposal(original.id).unwrap(), Some(current.clone()));

    let id = Uuid::new_v4();
    let mut assigned = edit(&current);
    assigned.texts[1] = Some(note_identity::assign("unmanaged replacement λ\n", id).unwrap());
    let updated = store.edit_proposal(&assigned).unwrap();
    let mut dropped = edit(&updated);
    dropped.texts[1] = Some("unmanaged replacement λ\n".into());
    assert!(store.edit_proposal(&dropped).is_err());
    assert_eq!(store.proposal(original.id).unwrap(), Some(updated.clone()));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.create_proposal(&original).unwrap(), updated);
}

#[test]
fn owned_rewrite_validation_and_atomic_settlement_preserve_identity() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let text = note_identity::assign("日本語 original λ\r\n", Uuid::new_v4()).unwrap();
    let draft = draft(&text, "unmanaged replacement\r\n");
    let current = store.create_proposal(&draft).unwrap();
    let current = comment(&mut store, &current);
    let request = RewriteSpec {
        id: Uuid::new_v4(),
        expected: current.stamp(),
        provider: "chatgpt".into(),
        model: "gpt-5.5".into(),
        effort: "high".into(),
    };
    let (running, capture) = store.begin_proposal_rewrite(&request).unwrap();
    assert_eq!(capture, Some(current.clone()));
    let mut refused = edit(&current);
    refused.texts[0] = Some("日本語 identity accidentally omitted\r\n".into());
    assert!(validate_result(&current, &refused).is_err());
    assert!(
        store
            .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(refused))
            .is_err()
    );
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current.clone()));
    assert_eq!(store.proposal_rewrite(request.id).unwrap(), Some(running));

    let mut accepted = edit(&current);
    accepted.texts[0] = Some(text.replace("original λ", "reviewed 🦀"));
    validate_result(&current, &accepted).unwrap();
    let outcome = RewriteOutcome::Completed(accepted.clone());
    let completed = store.finish_proposal_rewrite(request.id, &outcome).unwrap();
    assert_eq!(completed.status, RewriteStatus::Completed);
    let updated = store.proposal(draft.id).unwrap().unwrap();
    assert_eq!(completed.result_stamp, Some(updated.stamp()));
    assert_eq!(updated.version, current.version + 1);
    assert_eq!(
        updated.draft.changes[0].text(),
        accepted.texts[0].as_deref()
    );
    assert!(matches!(
        updated.comments[0].target,
        CommentTarget::Unresolved(_)
    ));
    assert_eq!(
        store.finish_proposal_rewrite(request.id, &outcome).unwrap(),
        completed
    );
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(updated));
    assert_eq!(store.proposal_rewrite(request.id).unwrap(), Some(completed));
}

#[test]
fn ordinary_review_edits_keep_raw_markdown_and_opaque_metadata_readable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let raw = "---\n# Work 日本語\n";
    let opaque = "\u{feff}---\r\nsummary: |\r\n  brn_id: literal text\r\n  ---\r\n  ...\r\nnested:\r\n  brn_id: unrelated metadata\r\n---\r\nBody λ\r\n";
    let original = draft(raw, opaque);
    let current = store.create_proposal(&original).unwrap();
    let mut changed = edit(&current);
    changed.texts[0] = Some(format!("{raw}more ordinary text λ\n"));
    changed.texts[1] = Some(opaque.replace("Body λ", "Reviewed body 🦀"));
    validate_result(&current, &changed).unwrap();
    let updated = store.edit_proposal(&changed).unwrap();
    assert_eq!(updated.version, current.version + 1);
    assert_eq!(updated.draft.changes[0].text(), changed.texts[0].as_deref());
    assert_eq!(updated.draft.changes[1].text(), changed.texts[1].as_deref());
    let id = Uuid::new_v4();
    let mut assigned = edit(&updated);
    assigned.texts[1] =
        Some(note_identity::assign(updated.draft.changes[1].text().unwrap(), id).unwrap());
    let assigned = store.edit_proposal(&assigned).unwrap();
    assert_eq!(
        note_identity::read(assigned.draft.changes[1].text().unwrap()).unwrap(),
        Some(id)
    );
    assert!(
        assigned.draft.changes[1]
            .text()
            .unwrap()
            .contains("  brn_id: literal text\r\n  ---\r\n  ...\r\n")
    );
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(original.id).unwrap(), Some(assigned));
}

#[test]
fn scalar_continuation_is_refused_without_mutating_review_or_rewrite_capture() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let text = note_identity::assign("日本語 body λ\n", Uuid::new_v4()).unwrap();
    let draft = draft(&text, "unmanaged replacement\n");
    let current = store.create_proposal(&draft).unwrap();
    let current = comment(&mut store, &current);
    let mut continued = edit(&current);
    continued.texts[0] = Some(text.replacen("\n---\n", "\n  extra scalar text\n---\n", 1));
    assert!(note_identity::read(continued.texts[0].as_deref().unwrap()).is_err());
    assert!(validate_result(&current, &continued).is_err());
    assert!(matches!(
        store.edit_proposal(&continued),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current.clone()));

    let request = RewriteSpec {
        id: Uuid::new_v4(),
        expected: current.stamp(),
        provider: "chatgpt".into(),
        model: "gpt-5.5".into(),
        effort: "high".into(),
    };
    let (running, capture) = store.begin_proposal_rewrite(&request).unwrap();
    assert_eq!(capture, Some(current.clone()));
    assert!(
        store
            .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(continued))
            .is_err()
    );
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current.clone()));
    assert_eq!(store.proposal_rewrite(request.id).unwrap(), Some(running));
    store
        .finish_proposal_rewrite(request.id, &RewriteOutcome::Failed("other".into()))
        .unwrap();
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(current));
}

#[test]
fn exact_undo_can_remove_assignment_and_keeps_historical_receipts_valid() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let original = "\u{feff}---\r\ncustom: 日本語 🦀\r\n---\r\nBody λ\r\n";
    let assigned = note_identity::assign(original, Uuid::new_v4()).unwrap();
    let original_proof = fingerprint(original, 2);
    let assigned_proof = fingerprint(&assigned, 100);
    let mut draft = draft("unused", &assigned);
    draft.changes = vec![NoteChange::Replace {
        path: "existing.md".into(),
        parent: draft.vault.identity.clone(),
        before: original_proof.clone(),
        before_text: original.into(),
        text: assigned.clone(),
    }];
    let current = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: current.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(request.operation_id, std::slice::from_ref(&assigned_proof))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(assigned_proof.clone()),
                staging: Some(original_proof.clone()),
            }]),
        )
        .unwrap();
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: request.operation_id,
        trash_member: None,
    };
    let preview = store.preview_proposal_undo(&undo).unwrap();
    assert_eq!(preview.draft.changes[0].text(), Some(original));
    assert_eq!(
        note_identity::read(preview.draft.changes[0].text().unwrap()).unwrap(),
        None
    );
    assert!(note_identity::protect(&assigned, original).is_err());
    let admitted = store.begin_proposal_undo(&undo).unwrap();
    assert_eq!(admitted.approved.draft, preview.draft);
    store
        .record_proposal_prepared(undo.operation_id, std::slice::from_ref(&original_proof))
        .unwrap();
    store
        .finish_proposal_apply(
            undo.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(original_proof),
                staging: Some(assigned_proof),
            }]),
        )
        .unwrap();
    let settled = store.proposal_apply(undo.operation_id).unwrap().unwrap();
    assert_eq!(
        settled.receipt.as_ref().unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(settled.approved.draft.changes[0].text(), Some(original));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.begin_proposal_undo(&undo).unwrap(), settled);
    assert_eq!(
        store.proposal_apply(undo.operation_id).unwrap(),
        Some(settled)
    );
    assert_eq!(
        store
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap()
            .approved
            .draft
            .changes[0]
            .text(),
        Some(assigned.as_str())
    );
}
