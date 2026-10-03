#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    proposals::*,
};
use std::{fs, path::PathBuf};
use uuid::Uuid;

struct Fixture {
    _dir: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    app: App,
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let vault = dir.path().join("vault");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(
        vault.join("note.md"),
        "\u{feff}---\r\ncustom: λ\r\n---\r\nBefore 🦀\r\n",
    )
    .unwrap();
    let app = App::open(
        &data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    Fixture {
        _dir: dir,
        data,
        vault,
        app,
    }
}
fn draft(f: &mut Fixture) -> DraftRequest {
    let before = f.app.open_editor("note.md").unwrap().record.baseline;
    DraftRequest {
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: None,
        title: "Small review".into(),
        changes: vec![
            DraftNoteChange::Replace {
                path: "note.md".into(),
                expected: before.clone(),
                text: "After λ\r\n".into(),
            },
            DraftNoteChange::Create {
                path: "new.md".into(),
                text: "\u{feff}New 🦀\r\n".into(),
            },
        ],
        sources: vec![SourceVersion {
            path: "note.md".into(),
            fingerprint: before,
        }],
    }
}

#[test]
fn review_binds_exact_versions_and_never_writes_knowledge() {
    let mut f = fixture();
    let input = draft(&mut f);
    let original = fs::read(f.vault.join("note.md")).unwrap();
    let created = f.app.create_proposal(&input).unwrap();
    let NoteChange::Replace {
        before_text,
        before,
        ..
    } = &created.draft.changes[0]
    else {
        panic!()
    };
    assert_eq!(before_text.as_bytes(), original);
    assert_eq!(Some(before), input.sources.first().map(|s| &s.fingerprint));
    let comment = ReviewComment {
        id: Uuid::new_v4(),
        text: "Keep the Unicode".into(),
        target: CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 6,
            end: 8,
            quote: "λ".into(),
        }),
    };
    let commented = f
        .app
        .add_proposal_comment(&CommentRequest {
            expected: created.stamp(),
            comment,
        })
        .unwrap();
    let late = ProposalEdit {
        expected: created.stamp(),
        title: created.draft.title.clone(),
        texts: vec![Some("Late output".into()), Some("new".into())],
    };
    assert_eq!(
        f.app.rewrite_proposal(&late).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let edited = f
        .app
        .edit_proposal(&ProposalEdit {
            expected: commented.stamp(),
            title: "Edited review".into(),
            texts: vec![
                Some("After new λ\r\n".into()),
                Some("\u{feff}New 🦀\r\n".into()),
            ],
        })
        .unwrap();
    assert!(matches!(
        edited.comments[0].target,
        CommentTarget::Unresolved(_)
    ));
    assert_eq!(
        f.app.proposals(input.group_id).unwrap(),
        vec![edited.clone()]
    );
    assert_eq!(fs::read(f.vault.join("note.md")).unwrap(), original);
    assert!(!f.vault.join("new.md").exists());
    drop(f.app);
    let mut app = App::open(
        &f.data,
        AppConfig {
            vault_root: None,
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    assert_eq!(app.proposal(input.id).unwrap(), edited);
    let rejected = app.reject_proposal(edited.stamp()).unwrap();
    assert_eq!(rejected.state, ProposalState::Rejected);
    assert_eq!(rejected.comments, edited.comments);
    assert!(
        app.rewrite_proposal(&ProposalEdit {
            expected: rejected.stamp(),
            title: "Late".into(),
            texts: vec![Some("x".into()), Some("y".into())]
        })
        .is_err()
    );
    assert_eq!(fs::read(f.vault.join("note.md")).unwrap(), original);
}

#[test]
fn creation_replay_precedes_disk_checks_and_never_overwrites_edited_work() {
    let mut f = fixture();
    let input = draft(&mut f);
    let created = f.app.create_proposal(&input).unwrap();
    let edited = f
        .app
        .edit_proposal(&ProposalEdit {
            expected: created.stamp(),
            title: "Manual changes".into(),
            texts: vec![Some("My work".into()), Some("My other work".into())],
        })
        .unwrap();
    fs::write(f.vault.join("note.md"), "External later bytes").unwrap();
    fs::write(f.vault.join("new.md"), "New occupant").unwrap();
    assert_eq!(f.app.create_proposal(&input).unwrap(), edited);
    let mut changed = input.clone();
    changed.title = "Different creation".into();
    assert_eq!(
        f.app.create_proposal(&changed).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    let mut fresh = input;
    fresh.id = Uuid::new_v4();
    assert_eq!(
        f.app.create_proposal(&fresh).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    assert_eq!(f.app.proposals(None).unwrap(), vec![edited]);
    assert_eq!(
        fs::read_to_string(f.vault.join("note.md")).unwrap(),
        "External later bytes"
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("new.md")).unwrap(),
        "New occupant"
    );
}

#[test]
fn source_identity_creation_occupant_and_aliases_are_not_accepted_as_fresh_context() {
    let mut f = fixture();
    let input = draft(&mut f);
    // Replace the source with byte-identical content at a different inode.
    let bytes = fs::read(f.vault.join("note.md")).unwrap();
    fs::write(f.vault.join("other.md"), &bytes).unwrap();
    fs::rename(f.vault.join("other.md"), f.vault.join("note.md")).unwrap();
    assert_eq!(
        f.app.create_proposal(&input).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let source_only = DraftRequest {
        id: Uuid::new_v4(),
        changes: vec![DraftNoteChange::Create {
            path: "unused.md".into(),
            text: "New".into(),
        }],
        ..input.clone()
    };
    assert_eq!(
        f.app.create_proposal(&source_only).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let occupied = DraftRequest {
        id: Uuid::new_v4(),
        sources: vec![],
        changes: vec![DraftNoteChange::Create {
            path: "note.md".into(),
            text: "Would overwrite".into(),
        }],
        ..input.clone()
    };
    assert_eq!(
        f.app.create_proposal(&occupied).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let aliased = DraftRequest {
        id: Uuid::new_v4(),
        sources: vec![],
        changes: vec![
            DraftNoteChange::Create {
                path: "é.md".into(),
                text: "a".into(),
            },
            DraftNoteChange::Create {
                path: "e\u{301}.md".into(),
                text: "b".into(),
            },
        ],
        ..input
    };
    assert!(f.app.create_proposal(&aliased).is_err());
    assert!(f.app.proposals(None).unwrap().is_empty());
    assert_eq!(fs::read(f.vault.join("note.md")).unwrap(), bytes);
    assert!(!f.vault.join("unused.md").exists());
}
