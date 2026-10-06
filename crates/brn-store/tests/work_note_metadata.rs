use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    note_identity,
    note_metadata::{NoteClassification, classify},
    work::proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest, UndoRequest},
    work::proposals::{
        CommentRequest, CommentTarget, NoteChange, ProposalDraft, ProposalEdit, ReviewComment,
        TextAnchor,
    },
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ID: &str = "9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d";

#[test]
fn classification_defaults_and_scalars_leave_exact_note_bytes_unchanged() {
    for text in [
        "",
        "\u{feff}# Body 日本語 🦀\r\n",
        "---",
        "---\n# Raw Markdown\n",
        "---\ncustom: retained\n---\nbrn_kind: source\nbrn_state: history\n",
        "---\n# brn_kind: source\n# brn_state: history\n---\nBody\n",
    ] {
        assert_eq!(classify(text).unwrap(), NoteClassification::default());
    }
    for newline in ["\n", "\r\n"] {
        for closing in ["---", "..."] {
            for scalar in ["source", "'source'", "\"source\"", "source # comment 🦀"] {
                let text = format!(
                    "\u{feff}---{newline}brn_id: {ID}{newline}custom: '日本語 🦀'{newline}brn_kind: {scalar}{newline}brn_state: 'history'\t# exact comment{newline}{closing}{newline}Body λ{newline}"
                );
                let original = text.as_bytes().to_owned();
                assert_eq!(
                    classify(&text).unwrap(),
                    NoteClassification {
                        source: true,
                        history: true
                    }
                );
                assert_eq!(text.as_bytes(), original);
                assert_eq!(
                    note_identity::read(&text).unwrap(),
                    Some(Uuid::parse_str(ID).unwrap())
                );
                assert_eq!(
                    note_identity::assign(&text, Uuid::parse_str(ID).unwrap()).unwrap(),
                    text
                );
            }
        }
    }
    assert_eq!(
        classify("---\nbrn_kind: 'knowledge'\nbrn_state: \"current\"\n...\n").unwrap(),
        NoteClassification::default()
    );
    assert_eq!(
        classify("---\nbrn_state: history\n---\n").unwrap(),
        NoteClassification {
            source: false,
            history: true
        }
    );
    assert_eq!(
        classify("---\nbrn_kind: source\n---\n").unwrap(),
        NoteClassification {
            source: true,
            history: false
        }
    );
}

#[test]
fn managed_class_fields_refuse_malformed_duplicate_and_incomplete_syntax() {
    for metadata in [
        "brn_kind:",
        "brn_kind: Source",
        "brn_kind: unknown",
        "brn_state: archived",
        "brn_state: CURRENT",
        "brn_kind: source extra",
        "brn_kind: source#ambiguous",
        "brn_kind: 'source'#ambiguous",
        "brn_state: 'history",
        "brn_kind: !!str source",
        "brn_kind: [source]",
        "brn_state: >\n  history",
        "brn_kind:source",
        "brn_kind : source",
        "'brn_kind': source",
        "\"brn_state\": history",
        " brn_kind: source",
        "\tbrn_state: history",
        "? brn_kind: source",
        "- brn_state: history",
        "brn_kind",
        "brn_state = history",
        "brn_kind: knowledge\nbrn_kind: knowledge",
        "brn_state: current\nbrn_state: 'history'",
        "brn_kind: source\n  extra scalar text",
        "brn_state: 'history'\n  # retained comment\n \t\n  extra scalar text",
    ] {
        let text = format!("---\n{metadata}\n---\nBody λ\n");
        assert!(
            matches!(classify(&text), Err(Error::Invalid(_))),
            "{metadata}"
        );
    }
    for text in [
        "---\nbrn_kind: source\n",
        "---\nbrn_state: history\n# No closing delimiter\n",
        "--- \nbrn_kind: source\n---\n",
        "---\nbrn_kind: source\n--- # ambiguous\n",
    ] {
        assert!(classify(text).is_err(), "{text:?}");
    }
}

#[test]
fn unrelated_blocks_and_nested_fields_are_opaque_after_each_managed_scalar() {
    for metadata in [
        "summary: |\n  brn_kind: literal text\n  brn_state: literal text\n  ---\n  ...",
        "nested:\n  brn_kind: not a root field\n  brn_state: not a root field",
        "'custom': >-\n  brn_kind: source\n\n  brn_state: history",
        "custom: {brn_kind: opaque, brn_state: nested}",
    ] {
        let text = format!(
            "---\nbrn_kind: source\n  # comment\n \t\n{metadata}\nbrn_state: history\n---\nBody\n"
        );
        assert_eq!(
            classify(&text).unwrap(),
            NoteClassification {
                source: true,
                history: true
            },
            "{metadata}"
        );
        let default = format!("---\n{metadata}\n---\nBody\n");
        assert_eq!(
            classify(&default).unwrap(),
            NoteClassification::default(),
            "{metadata}"
        );
    }
    // Each parser selects only its own documented fields; the other field's
    // validity is separately observable rather than a new legacy edit constraint.
    let malformed_identity = "---\nbrn_id: historic malformed value\nbrn_kind: source\n---\nBody\n";
    assert_eq!(
        classify(malformed_identity).unwrap(),
        NoteClassification {
            source: true,
            history: false
        }
    );
    assert!(note_identity::read(malformed_identity).is_err());
    let malformed_class = format!("---\nbrn_id: {ID}\nbrn_state: unknown\n---\nBody\n");
    assert!(classify(&malformed_class).is_err());
    assert_eq!(
        note_identity::read(&malformed_class).unwrap(),
        Some(Uuid::parse_str(ID).unwrap())
    );
    note_identity::protect(
        &malformed_class,
        &malformed_class.replace("Body", "Reviewed body"),
    )
    .unwrap();
}

#[test]
fn root_flow_layouts_are_refused_regardless_of_managed_key_position() {
    for metadata in [
        "{title: hi, brn_kind: source}",
        "{brn_state: history, title: hi}",
        "  {title: hi, 'brn_state': history}",
        "[knowledge, source]",
        "{title: unrelated}",
    ] {
        assert!(
            classify(&format!("---\n{metadata}\n---\nBody\n")).is_err(),
            "{metadata}"
        );
    }
}

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

#[test]
fn class_changes_remain_exact_reviewable_edits_with_replay_and_historical_undo() {
    let fixture = tempfile::tempdir_in("/tmp").unwrap();
    let root = fixture.path().canonicalize().unwrap();
    let data = root.join("data");
    let vault = root.join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    let original = format!(
        "\u{feff}---\r\nbrn_id: {ID}\r\nbrn_kind: knowledge\r\nbrn_state: current\r\ncustom: 日本語\r\n---\r\nBody 日本語 λ\r\n"
    );
    let proposed = original.replace("brn_kind: knowledge", "brn_kind: source");
    std::fs::write(vault.join("note.md"), original.as_bytes()).unwrap();
    let (mut store, _) = WorkStore::open(&data).unwrap();
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    let draft = ProposalDraft {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: vault.to_str().unwrap().into(),
            identity: parent.clone(),
        }),
        title: "Review classification λ".into(),
        changes: vec![NoteChange::Replace {
            path: "note.md".into(),
            parent,
            before: fingerprint(&original, 2),
            before_text: original.clone(),
            text: proposed.clone(),
        }],
        sources: vec![],
        action_changes: Vec::new(),
    };
    let created = store.create_proposal(&draft).unwrap();
    let start = proposed.find("日本語 λ").unwrap();
    let current = store
        .add_proposal_comment(&CommentRequest {
            expected: created.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Keep original wording 🦀\r\n".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start,
                    end: start + "日本語 λ".len(),
                    quote: "日本語 λ".into(),
                }),
            },
        })
        .unwrap();
    let edited = proposed.replace("brn_state: current", "brn_state: history");
    let edit = ProposalEdit {
        expected: current.stamp(),
        title: current.draft.title.clone(),
        texts: vec![Some(edited.clone())],
        action_data: Vec::new(),
    };
    brn_store::work::proposal_rewrite::validate_result(&current, &edit).unwrap();
    let updated = store.edit_proposal(&edit).unwrap();
    assert_eq!(updated.draft.changes[0].text(), Some(edited.as_str()));
    assert_eq!(updated.version, current.version + 1);
    assert_eq!(updated.comments[0].text, current.comments[0].text);
    assert_eq!(
        classify(&edited).unwrap(),
        NoteClassification {
            source: true,
            history: true
        }
    );
    assert_eq!(
        note_identity::read(&edited).unwrap(),
        Some(Uuid::parse_str(ID).unwrap())
    );
    assert_eq!(
        std::fs::read(vault.join("note.md")).unwrap(),
        original.as_bytes()
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: updated.stamp(),
    };
    let approved = store.begin_proposal_apply(&approval).unwrap();
    assert_eq!(
        approved.approved.draft.changes[0].text(),
        Some(edited.as_str())
    );
    let installed_proof = fingerprint(&edited, 100);
    let original_proof = fingerprint(&original, 2);
    store
        .record_proposal_prepared(
            approval.operation_id,
            std::slice::from_ref(&installed_proof),
        )
        .unwrap();
    store
        .finish_proposal_apply(
            approval.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(installed_proof.clone()),
                staging: Some(original_proof.clone()),
            }]),
        )
        .unwrap();
    assert!(
        store
            .proposal(draft.id)
            .unwrap()
            .unwrap()
            .comments
            .is_empty()
    );
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    let preview = store.preview_proposal_undo(&undo).unwrap();
    assert_eq!(preview.draft.changes[0].text(), Some(original.as_str()));
    assert_eq!(
        classify(preview.draft.changes[0].text().unwrap()).unwrap(),
        NoteClassification::default()
    );
    let inverse = store.begin_proposal_undo(&undo).unwrap();
    assert_eq!(inverse.approved.draft, preview.draft);
    store
        .record_proposal_prepared(undo.operation_id, std::slice::from_ref(&original_proof))
        .unwrap();
    store
        .finish_proposal_apply(
            undo.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(original_proof),
                staging: Some(installed_proof),
            }]),
        )
        .unwrap();
    let settled = store.proposal_apply(undo.operation_id).unwrap().unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(&data).unwrap();
    assert_eq!(store.begin_proposal_undo(&undo).unwrap(), settled);
    let replay = store.create_proposal(&draft).unwrap();
    assert_eq!(replay.draft.changes[0].text(), Some(edited.as_str()));
    assert!(replay.comments.is_empty());
    assert_eq!(
        std::fs::read(vault.join("note.md")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn historic_malformed_class_bytes_do_not_gain_a_store_replay_or_edit_constraint() {
    let fixture = tempfile::tempdir_in("/tmp").unwrap();
    let (mut store, _) = WorkStore::open(&fixture.path().canonicalize().unwrap()).unwrap();
    let text = format!("---\nbrn_id: {ID}\nbrn_kind: old unsupported kind\n---\nBody λ\n");
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    let draft = ProposalDraft {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        }),
        title: "Legacy class bytes".into(),
        changes: vec![NoteChange::Create {
            path: "legacy.md".into(),
            parent,
            text: text.clone(),
        }],
        sources: vec![],
        action_changes: Vec::new(),
    };
    assert!(classify(&text).is_err());
    let current = store.create_proposal(&draft).unwrap();
    let changed = text.replace("Body λ", "Reviewed body 🦀");
    let updated = store
        .edit_proposal(&ProposalEdit {
            expected: current.stamp(),
            title: current.draft.title.clone(),
            texts: vec![Some(changed.clone())],
            action_data: Vec::new(),
        })
        .unwrap();
    assert_eq!(updated.draft.changes[0].text(), Some(changed.as_str()));
    drop(store);
    let (mut store, _) = WorkStore::open(&fixture.path().canonicalize().unwrap()).unwrap();
    assert_eq!(store.create_proposal(&draft).unwrap(), updated);
}

#[test]
fn history_transition_changes_only_managed_state_and_preserves_exact_other_bytes() {
    for newline in ["\n", "\r\n"] {
        for scalar in ["current", "'current'", "\"current\""] {
            let before = format!(
                "\u{feff}---{newline}brn_id: {ID}{newline}brn_kind: knowledge{newline}brn_state:\t {scalar}\t# keep 日本語{newline}custom: |{newline}  brn_state: opaque{newline}brn_provenance: []{newline}...{newline}Exact body 🦀{newline}"
            );
            let expected = before.replacen(scalar, &scalar.replace("current", "history"), 1);
            let history = brn_store::note_metadata::to_history(&before).unwrap();
            assert_eq!(history.as_bytes(), expected.as_bytes());
            assert_eq!(
                classify(&history).unwrap(),
                NoteClassification {
                    source: false,
                    history: true
                }
            );
            assert_eq!(
                note_identity::read(&history).unwrap(),
                Some(Uuid::parse_str(ID).unwrap())
            );
            assert!(brn_store::note_metadata::to_history(&history).is_err());
        }
        let before =
            format!("\u{feff}---{newline}custom: keep{newline}---{newline}Exact body 🦀{newline}");
        assert_eq!(
            brn_store::note_metadata::to_history(&before).unwrap(),
            before.replacen(
                &format!("---{newline}"),
                &format!("---{newline}brn_state: history{newline}"),
                1
            )
        );
        let bare = format!("\u{feff}Exact body 🦀{newline}");
        assert_eq!(
            brn_store::note_metadata::to_history(&bare).unwrap(),
            format!(
                "\u{feff}---{newline}brn_state: history{newline}---{newline}Exact body 🦀{newline}"
            )
        );
    }
    assert_eq!(
        brn_store::note_metadata::to_history("").unwrap(),
        "---\nbrn_state: history\n---\n"
    );
}

#[test]
fn history_transition_refuses_source_history_malformed_headers_and_size_overflow() {
    for before in [
        "---\nbrn_kind: source\n---\nBody\n",
        "---\nbrn_state: history\n---\nBody\n",
        "---\nbrn_state: current\nbrn_state: current\n---\nBody\n",
        "---\nbrn_state: unsupported\n---\nBody\n",
        "---\nbrn_state: 'current\n---\nBody\n",
        "---\ncustom: keep\n",
        "--- \ncustom: keep\n---\nBody\n",
        "---\n{custom: keep}\n---\nBody\n",
    ] {
        assert!(
            brn_store::note_metadata::to_history(before).is_err(),
            "{before:?}"
        );
    }
    assert!(brn_store::note_metadata::to_history(&"x".repeat(1024 * 1024)).is_err());
    assert!(brn_store::note_metadata::to_history(&"x".repeat(1024 * 1024 + 1)).is_err());
}
