#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    activity::{ActivityChangeKind, ActivityRequest},
    app::{App, AppConfig},
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, path::PathBuf};
use uuid::Uuid;

struct Fixture {
    _dir: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let vault = dir.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _dir: dir,
            data,
            vault,
        }
    }
    fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap()
    }
}
fn create(app: &mut App, path: &str) -> brn_workflow::proposals::ProposalRecord {
    app.create_proposal(&DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: format!("Create {path}"),
        changes: vec![DraftNoteChange::Create {
            path: path.into(),
            text: "Private full body λ\r\n".into(),
        }],
        sources: vec![],
    })
    .unwrap()
}
fn approval(record: &brn_workflow::proposals::ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    }
}

#[test]
fn settled_activity_is_body_free_and_survives_later_bytes_and_database_restore() {
    let f = Fixture::new();
    fs::write(f.vault.join("replace.md"), "Original body").unwrap();
    fs::write(f.vault.join("trash.md"), "Retained body").unwrap();
    let mut app = f.app();
    let before = app.open_editor("replace.md").unwrap().record.baseline;
    let trash = app.open_editor("trash.md").unwrap().record.baseline;
    let group = Uuid::new_v4();
    let session = app
        .work_store_mut()
        .begin_turn(
            Uuid::new_v4(),
            None,
            "Synthetic session",
            "chatgpt",
            "test-model",
        )
        .unwrap()
        .conversation_id;
    let draft = app
        .create_proposal(&DraftRequest {
            id: Uuid::new_v4(),
            group_id: Some(group),
            session_id: Some(session),
            title: "Approved Unicode λ🦀".into(),
            changes: vec![
                DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "Private created body".into(),
                },
                DraftNoteChange::Replace {
                    path: "replace.md".into(),
                    expected: before,
                    text: "Private replacement body".into(),
                },
                DraftNoteChange::Trash {
                    path: "trash.md".into(),
                    expected: trash,
                },
            ],
            sources: vec![],
        })
        .unwrap();
    assert!(
        app.activity(&ActivityRequest::default())
            .unwrap()
            .entries
            .is_empty()
    );
    let request = approval(&draft);
    assert_eq!(
        app.approve_proposal(&request).unwrap().outcome,
        ApplyOutcome::Applied
    );
    let page = app.activity(&ActivityRequest::default()).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert!(page.next_before.is_none());
    let entry = &page.entries[0];
    assert_eq!(entry.operation_id, request.operation_id);
    assert_eq!(entry.proposal_id, draft.draft.id);
    assert_eq!(entry.group_id, Some(group));
    assert_eq!(entry.session_id, Some(session));
    assert_eq!(entry.title, draft.draft.title);
    assert_eq!(
        entry.summary,
        "Created 1 note; replaced 1 note; moved 1 note to Trash."
    );
    assert!(entry.approved_at_utc.as_ref().unwrap().ends_with('Z'));
    let journal = app
        .work_store()
        .proposal_apply(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(entry.approved_at_ms, journal.started_at_ms);
    assert_eq!(
        entry
            .changes
            .iter()
            .map(|c| (c.kind, c.path.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (ActivityChangeKind::Created, "new.md"),
            (ActivityChangeKind::Replaced, "replace.md"),
            (ActivityChangeKind::Trashed, "trash.md")
        ]
    );
    let encoded = serde_json::to_string(&page).unwrap();
    for body in [
        "Private",
        "Original body",
        "Retained body",
        "before_text",
        "fingerprint",
        "comments",
        "staging",
    ] {
        assert!(!encoded.contains(body), "activity must omit {body}");
    }
    drop(app);
    fs::write(f.vault.join("replace.md"), "Later owner bytes").unwrap();
    // Synthetic recovery only: remove this fixture's DB/backups; retained
    // ordinary completion restores history without repeating old file effects.
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let app = f.app();
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), page);
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Later owner bytes"
    );
    drop(app);
    fs::rename(&f.vault, f.vault.with_file_name("unavailable-vault")).unwrap();
    let app = App::open(
        &f.data,
        AppConfig {
            vault_root: None,
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    assert_eq!(
        app.activity(&ActivityRequest::default()).unwrap(),
        page,
        "operational history does not require current vault access"
    );
}

#[test]
fn only_applied_history_is_visible_even_when_current_evidence_is_fenced() {
    let f = Fixture::new();
    let mut app = f.app();
    let applied = create(&mut app, "approved.md");
    let approval = approval(&applied);
    app.approve_proposal(&approval).unwrap();
    let expected = app.activity(&ActivityRequest::default()).unwrap();
    let rejected = create(&mut app, "rejected.md");
    app.reject_proposal(rejected.stamp()).unwrap();
    let draft = create(&mut app, "draft.md");
    let refused = create(&mut app, "refused.md");
    let refused_request = crate::approval(&refused);
    app.work_store_mut()
        .begin_proposal_apply(&refused_request)
        .unwrap();
    app.work_store_mut()
        .finish_proposal_apply(
            refused_request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&[brn_workflow::proposal_apply::ApplyMemberProof {
                destination: None,
                staging: None,
            }]),
        )
        .unwrap();
    let pending = crate::approval(&draft);
    app.work_store_mut().begin_proposal_apply(&pending).unwrap();
    assert_eq!(
        app.note("approved.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), expected);
    app.work_store_mut()
        .finish_proposal_apply(pending.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), expected);
    drop(app);
    let app = f.app();
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), expected);
    assert_eq!(
        app.note("approved.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
}

#[test]
fn pages_have_exact_exclusive_cursors_without_repeating_entries() {
    let f = Fixture::new();
    let mut app = f.app();
    let mut operations = vec![];
    for i in 0..5 {
        let record = create(&mut app, &format!("{i}.md"));
        let request = approval(&record);
        app.approve_proposal(&request).unwrap();
        let journal = app
            .work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        operations.push((journal.started_at_ms, request.operation_id));
    }
    operations.sort_unstable_by(|a, b| b.cmp(a));
    let mut cursor = None;
    let mut found = vec![];
    loop {
        let page = app
            .activity(&ActivityRequest {
                limit: 2,
                before: cursor,
            })
            .unwrap();
        assert!(page.entries.len() <= 2);
        if page.next_before.is_some() {
            assert_eq!(
                page.next_before,
                page.entries.last().map(|e| e.operation_id)
            );
        }
        found.extend(page.entries.iter().map(|e| e.operation_id));
        cursor = page.next_before;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(found, operations.iter().map(|e| e.1).collect::<Vec<_>>());
    let oldest = *found.last().unwrap();
    assert!(
        app.activity(&ActivityRequest {
            limit: 1,
            before: Some(oldest)
        })
        .unwrap()
        .entries
        .is_empty()
    );
    assert_eq!(
        app.activity(&ActivityRequest {
            limit: 1,
            before: Some(Uuid::new_v4())
        })
        .unwrap_err()
        .kind,
        ErrorKind::ContextStale
    );
    for request in [
        ActivityRequest {
            limit: 0,
            before: None,
        },
        ActivityRequest {
            limit: 101,
            before: None,
        },
        ActivityRequest {
            limit: 1,
            before: Some(Uuid::nil()),
        },
    ] {
        assert_eq!(
            app.activity(&request).unwrap_err().kind,
            ErrorKind::ToolRejected
        );
    }
    assert_eq!(app.work_store().proposal_applies().unwrap().len(), 5);

    // Import valid synthetic terminal snapshots with one shared approval time
    // into another fresh operational folder. UUID ordering must disambiguate
    // ties without losing or repeating members across exclusive pages.
    let mut journals = app.work_store().proposal_applies().unwrap();
    let shared_time = journals.iter().map(|j| j.started_at_ms).max().unwrap();
    let tied_data = f.data.with_file_name("tied-data");
    fs::create_dir(&tied_data).unwrap();
    let (mut store, _) = brn_store::WorkStore::open(&tied_data).unwrap();
    for journal in &mut journals {
        journal.started_at_ms = shared_time;
        store.restore_proposal_apply(journal).unwrap();
    }
    drop(store);
    let tied = App::open(
        &tied_data,
        AppConfig {
            vault_root: Some(f.vault.clone()),
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    let mut ids = journals
        .iter()
        .map(|j| j.request.operation_id)
        .collect::<Vec<_>>();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    let first = tied
        .activity(&ActivityRequest {
            limit: 2,
            before: None,
        })
        .unwrap();
    let second = tied
        .activity(&ActivityRequest {
            limit: 2,
            before: first.next_before,
        })
        .unwrap();
    let third = tied
        .activity(&ActivityRequest {
            limit: 2,
            before: second.next_before,
        })
        .unwrap();
    assert_eq!(
        [first.entries, second.entries, third.entries]
            .concat()
            .iter()
            .map(|entry| entry.operation_id)
            .collect::<Vec<_>>(),
        ids
    );
    assert!(third.next_before.is_none());
}
