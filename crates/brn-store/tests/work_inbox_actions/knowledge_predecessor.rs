//! Owner structural revision over real Store rows; no vault/provider operations.
use super::*;
use brn_store::work::proposals::{
    CommentRequest, CommentTarget, ProposalRecord, ReviewComment, TextAnchor,
    validate_knowledge_predecessor_transition,
};

fn setup() -> (
    tempfile::TempDir,
    WorkStore,
    InboxActionCapture,
    ProposalDraft,
    NoteChange,
) {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let mut captured = capture();
    captured.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    store
        .reserve_inbox_action(&captured, "Synthetic retained interpretation")
        .unwrap();
    let draft = knowledge_draft(&captured);
    let history = supersession_draft(&captured).changes.remove(1);
    (dir, store, captured, draft, history)
}
fn predecessor(history: &NoteChange) -> SourceVersion {
    SourceVersion {
        path: history.path().into(),
        fingerprint: history.before().unwrap().clone(),
    }
}
fn context(path: &str, inode: u64) -> SourceVersion {
    SourceVersion {
        path: path.into(),
        fingerprint: FileFingerprint {
            device: 1,
            inode,
            len: 7,
            sha256: digest(b"context"),
        },
    }
}
fn original_hash(dir: &tempfile::TempDir, id: Uuid) -> Vec<u8> {
    Connection::open(dir.path().join("brn.sqlite"))
        .unwrap()
        .query_row(
            "SELECT creation_sha256 FROM proposals WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap()
}
fn row_bytes(dir: &tempfile::TempDir, id: Uuid) -> Vec<u8> {
    Connection::open(dir.path().join("brn.sqlite"))
        .unwrap()
        .query_row(
            "SELECT record_json FROM proposals WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap()
}
fn edit(record: &ProposalRecord, replacement: &str) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: "Owner-reviewed interpretation õ".into(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|c| {
                c.text()
                    .map(|text| text.replace("Interpreted summary", replacement))
            })
            .collect(),
        action_data: vec![],
    }
}
fn replace_baseline(history: &mut NoteChange, text: String) {
    let NoteChange::Replace {
        before,
        before_text,
        text: after,
        ..
    } = history
    else {
        unreachable!()
    };
    before.len = text.len() as u64;
    before.sha256 = digest(text.as_bytes());
    *after = brn_store::note_metadata::to_history(&text).unwrap_or_else(|_| text.clone());
    *before_text = text;
}

#[test]
fn predecessor_attachment_preserves_owner_edits_comments_and_creation_hash_across_restart() {
    let (dir, mut store, captured, draft, history) = setup();
    let first = store.create_proposal(&draft).unwrap();
    let edited = store
        .edit_proposal(&edit(&first, "Owner choice 日本語 🦀\r\n"))
        .unwrap();
    let text = edited.draft.changes[0].text().unwrap();
    let quote = "Owner choice 日本語 🦀";
    let start = text.find(quote).unwrap();
    let commented = store
        .add_proposal_comment(&CommentRequest {
            expected: edited.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Preserve this exact owner wording".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start,
                    end: start + quote.len(),
                    quote: quote.into(),
                }),
            },
        })
        .unwrap();
    let before = store
        .add_proposal_comment(&CommentRequest {
            expected: commented.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "A whole-proposal comment also survives".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let creation = original_hash(&dir, draft.id);
    assert_eq!(creation, digest(&serde_json::to_vec(&draft).unwrap()));
    let attached = store
        .attach_inbox_knowledge_predecessor(before.stamp(), &history)
        .unwrap();
    validate_knowledge_predecessor_transition(&before, &attached).unwrap();
    assert_eq!(attached.version, before.version + 1);
    assert_eq!(attached.comments, before.comments);
    assert_eq!(attached.draft.changes[1], history);
    assert!(
        attached.draft.changes[0]
            .text()
            .unwrap()
            .starts_with(before.draft.changes[0].text().unwrap())
    );
    let mut binding = attached.draft.inbox_knowledge.clone().unwrap();
    assert!(binding.supersedes.take().is_some());
    assert_eq!(Some(binding), before.draft.inbox_knowledge);
    assert_eq!(attached.draft.sources[0], captured.source.unwrap());
    assert_eq!(attached.draft.sources[1], predecessor(&history));
    assert_eq!(original_hash(&dir, draft.id), creation);
    assert_eq!(store.create_proposal(&draft).unwrap(), attached);
    assert!(matches!(
        store.attach_inbox_knowledge_predecessor(before.stamp(), &history),
        Err(Error::StateChanged(_))
    ));
    let stale = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: before.stamp(),
    };
    assert!(matches!(
        store.begin_proposal_apply(&stale),
        Err(Error::StateChanged(_))
    ));
    assert!(store.proposal_apply(stale.operation_id).unwrap().is_none());
    let mut change = edit(&attached, "unused");
    change.texts[0] = Some(
        attached.draft.changes[0]
            .text()
            .unwrap()
            .replace("Owner choice", "Later owner choice"),
    );
    let latest = store.edit_proposal(&change).unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(draft.id).unwrap(), Some(latest.clone()));
    assert_eq!(store.create_proposal(&draft).unwrap(), latest);
    assert_eq!(original_hash(&dir, draft.id), creation);
    let mut changed_initial = draft.clone();
    changed_initial.title.push('!');
    assert!(matches!(
        store.create_proposal(&changed_initial),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.create_proposal(&latest.draft),
        Err(Error::OperationConflict(_))
    ));
    let admitted = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: latest.stamp(),
        })
        .unwrap();
    assert_eq!(admitted.approved, latest);
    assert_eq!(admitted.creation_sha256.as_slice(), creation);
    assert_eq!(admitted.members.len(), 2);
    assert!(!std::path::Path::new("/synthetic/vault/knowledge/new.md").exists());
}

#[test]
fn predecessor_attachment_promotes_exact_context_without_refresh_and_refuses_a_different_proof() {
    for stale in [false, true] {
        let (dir, mut store, _, mut draft, history) = setup();
        let captured_predecessor = predecessor(&history);
        let left = context("knowledge/left.md", 51);
        let right = context("knowledge/right.md", 52);
        draft
            .sources
            .extend([left.clone(), captured_predecessor.clone(), right.clone()]);
        let before = store.create_proposal(&draft).unwrap();
        let bytes = row_bytes(&dir, draft.id);
        let mut supplied = history.clone();
        if stale && let NoteChange::Replace { before, .. } = &mut supplied {
            before.inode += 1;
        }
        let result = store.attach_inbox_knowledge_predecessor(before.stamp(), &supplied);
        if stale {
            assert!(matches!(result, Err(Error::StateChanged(_))));
            assert_eq!(store.proposal(draft.id).unwrap(), Some(before));
            assert_eq!(row_bytes(&dir, draft.id), bytes);
        } else {
            let after = result.unwrap();
            assert_eq!(
                after.draft.sources,
                vec![draft.sources[0].clone(), captured_predecessor, left, right]
            );
            assert_eq!(after.draft.sources.len(), draft.sources.len());
            validate_knowledge_predecessor_transition(&before, &after).unwrap();
            assert_eq!(store.create_proposal(&draft).unwrap(), after);
        }
    }
}

#[test]
fn predecessor_attachment_invalid_history_and_repeated_attachment_preserve_the_row() {
    let (dir, mut store, captured, draft, history) = setup();
    let before = store.create_proposal(&draft).unwrap();
    let bytes = row_bytes(&dir, draft.id);
    for mode in 0..12 {
        let mut bad = history.clone();
        match mode {
            0 => bad = draft.changes[0].clone(),
            1 => {
                if let NoteChange::Replace { path, .. } = &mut bad {
                    *path = "../escape.md".into();
                }
            }
            2 => {
                if let NoteChange::Replace { path, .. } = &mut bad {
                    *path = draft.changes[0].path().into();
                }
            }
            3 => {
                if let NoteChange::Replace { before, .. } = &mut bad {
                    before.sha256 = [0; 32];
                }
            }
            4 => {
                if let NoteChange::Replace { text, .. } = &mut bad {
                    text.push_str("Changed protected History");
                }
            }
            5 => replace_baseline(&mut bad, "Unmanaged predecessor".into()),
            6 => replace_baseline(
                &mut bad,
                format!(
                    "---\nbrn_id: {}\n---\nSelf\n",
                    draft.inbox_knowledge.as_ref().unwrap().note_id
                ),
            ),
            7 => replace_baseline(
                &mut bad,
                format!(
                    "---\nbrn_id: {}\n---\nSource identity\n",
                    captured.note_id().unwrap()
                ),
            ),
            8 => replace_baseline(
                &mut bad,
                format!(
                    "---\nbrn_id: {}\nbrn_state: history\n---\nHistory\n",
                    Uuid::new_v4()
                ),
            ),
            9 => replace_baseline(
                &mut bad,
                format!(
                    "---\nbrn_id: {}\nbrn_kind: source\n---\nSource\n",
                    Uuid::new_v4()
                ),
            ),
            10 => {
                if let NoteChange::Replace { path, .. } = &mut bad {
                    *path = captured.source.as_ref().unwrap().path.clone();
                }
            }
            11 => replace_baseline(
                &mut bad,
                format!(
                    "---\nbrn_id: {}\nbrn_provenance: invalid\n---\nInvalid provenance\n",
                    Uuid::new_v4()
                ),
            ),
            _ => unreachable!(),
        }
        assert!(
            store
                .attach_inbox_knowledge_predecessor(before.stamp(), &bad)
                .is_err(),
            "mode {mode}"
        );
        assert_eq!(row_bytes(&dir, draft.id), bytes, "mode {mode}");
        assert_eq!(store.proposal(draft.id).unwrap(), Some(before.clone()));
    }
    let after = store
        .attach_inbox_knowledge_predecessor(before.stamp(), &history)
        .unwrap();
    assert!(
        store
            .attach_inbox_knowledge_predecessor(after.stamp(), &history)
            .is_err()
    );
    assert_eq!(store.proposal(draft.id).unwrap(), Some(after));
}

#[test]
fn predecessor_transition_validator_refuses_preserved_field_changes_and_wrong_versions() {
    let (_dir, mut store, _, draft, history) = setup();
    let first = store.create_proposal(&draft).unwrap();
    let before = store
        .add_proposal_comment(&CommentRequest {
            expected: first.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Exact comment".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let after = store
        .attach_inbox_knowledge_predecessor(before.stamp(), &history)
        .unwrap();
    for mode in 0..18 {
        let mut bad = after.clone();
        match mode {
            0 => bad.version += 1,
            1 => bad.version = before.version,
            2 => bad.updated_at_ms = before.updated_at_ms - 1,
            3 => bad.created_at_ms -= 1,
            4 => bad.state = ProposalState::Rejected,
            5 => bad.draft.title.push('!'),
            6 => bad.comments[0].text.push('!'),
            7 => bad.comments.clear(),
            8 => bad.draft.id = Uuid::new_v4(),
            9 => bad.draft.session_id = Some(Uuid::new_v4()),
            10 => bad.draft.vault.as_mut().unwrap().id = Uuid::new_v4(),
            11 => {
                if let NoteChange::Create { path, .. } = &mut bad.draft.changes[0] {
                    *path = "knowledge/retargeted.md".into();
                }
            }
            12 => {
                if let NoteChange::Create { parent, .. } = &mut bad.draft.changes[0] {
                    parent.inode += 1;
                }
            }
            13 => {
                if let NoteChange::Create { text, .. } = &mut bad.draft.changes[0] {
                    *text = text.replace("Interpreted summary", "Unexpected rewriting");
                }
            }
            14 => bad.draft.sources.swap(0, 1),
            15 => bad.draft.sources.push(context("knowledge/extra.md", 80)),
            16 => {
                if let NoteChange::Replace { text, .. } = &mut bad.draft.changes[1] {
                    text.push('!');
                }
            }
            17 => bad.draft.inbox_knowledge.as_mut().unwrap().citations[0]
                .quote
                .push('!'),
            _ => unreachable!(),
        }
        assert!(
            validate_knowledge_predecessor_transition(&before, &bad).is_err(),
            "mode {mode}"
        );
    }
    let mut later_timestamp = after.clone();
    later_timestamp.updated_at_ms += 100;
    validate_knowledge_predecessor_transition(&before, &later_timestamp).unwrap();
    assert!(validate_knowledge_predecessor_transition(&after, &after).is_err());
}

#[test]
fn predecessor_attachment_sql_failure_rolls_back_before_any_acknowledgement() {
    let (dir, mut store, _, draft, history) = setup();
    let before = store.create_proposal(&draft).unwrap();
    let bytes = row_bytes(&dir, draft.id);
    let creation = original_hash(&dir, draft.id);
    let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.execute_batch("CREATE TRIGGER abort_predecessor AFTER UPDATE ON proposals BEGIN SELECT RAISE(ABORT,'synthetic revision failure'); END;").unwrap();
    assert!(matches!(
        store.attach_inbox_knowledge_predecessor(before.stamp(), &history),
        Err(Error::Sql(_))
    ));
    assert_eq!(row_bytes(&dir, draft.id), bytes);
    assert_eq!(original_hash(&dir, draft.id), creation);
    assert_eq!(store.proposal(draft.id).unwrap(), Some(before.clone()));
    raw.execute_batch("DROP TRIGGER abort_predecessor;")
        .unwrap();
    drop(raw);
    let after = store
        .attach_inbox_knowledge_predecessor(before.stamp(), &history)
        .unwrap();
    assert_eq!(after.version, before.version + 1);
}

#[test]
fn predecessor_attachment_requires_an_unapproved_inbox_knowledge_draft() {
    let (dir, mut store, _, mut draft, history) = setup();
    let first = store.create_proposal(&draft).unwrap();
    let rejected = store.reject_proposal(first.stamp()).unwrap();
    let bytes = row_bytes(&dir, draft.id);
    assert!(matches!(
        store.attach_inbox_knowledge_predecessor(rejected.stamp(), &history),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(row_bytes(&dir, draft.id), bytes);
    draft.id = Uuid::new_v4();
    draft.inbox_knowledge = None;
    let ordinary = store.create_proposal(&draft).unwrap();
    let bytes = row_bytes(&dir, draft.id);
    assert!(
        store
            .attach_inbox_knowledge_predecessor(ordinary.stamp(), &history)
            .is_err()
    );
    assert_eq!(row_bytes(&dir, draft.id), bytes);
}

#[test]
fn predecessor_attachment_respects_full_source_and_note_capacity_without_partial_changes() {
    for promote in [false, true] {
        let (dir, mut store, _, mut draft, history) = setup();
        draft
            .sources
            .extend((1..64).map(|i| context(&format!("knowledge/context-{i}.md"), 100 + i)));
        if promote {
            draft.sources[63] = predecessor(&history);
        }
        let before = store.create_proposal(&draft).unwrap();
        let bytes = row_bytes(&dir, draft.id);
        let attached = store.attach_inbox_knowledge_predecessor(before.stamp(), &history);
        if promote {
            let after = attached.unwrap();
            assert_eq!(after.draft.sources.len(), 64);
            assert_eq!(after.draft.sources[1], predecessor(&history));
        } else {
            assert!(attached.is_err());
            assert_eq!(row_bytes(&dir, draft.id), bytes);
        }
    }
    let (dir, mut store, _, mut draft, history) = setup();
    if let NoteChange::Create { text, .. } = &mut draft.changes[0] {
        text.push_str(&"x".repeat(brn_store::MAX_NOTE_BYTES - text.len()));
    }
    let before = store.create_proposal(&draft).unwrap();
    let bytes = row_bytes(&dir, draft.id);
    assert!(
        store
            .attach_inbox_knowledge_predecessor(before.stamp(), &history)
            .is_err()
    );
    assert_eq!(row_bytes(&dir, draft.id), bytes);
    assert_eq!(store.proposal(draft.id).unwrap(), Some(before));
}
