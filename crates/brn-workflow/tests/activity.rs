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
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
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
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
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

fn action_data(title: &str) -> brn_workflow::actions::ActionData {
    brn_workflow::actions::ActionData {
        title: title.into(),
        description: "PRIVATE ACTION DESCRIPTION must stay outside Activity pages".into(),
        state: brn_workflow::actions::ActionState::Open,
        owner: None,
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: None,
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    }
}
#[test]
fn activity_action_inventory_is_historical_ordered_body_free_through_edit_undo_complete_and_restart()
 {
    use brn_workflow::{
        action_completion::CompleteActionRequest, proposal_apply::UndoRequest,
        proposals::ActionChange,
    };
    let f = Fixture::new();
    let mut app = f.app();
    let ids = [Uuid::new_v4(), Uuid::new_v4()];
    let titles = ["Created õ\r\n日本語\u{1b}[31m", "Second λ"];
    let review = app
        .create_proposal(&DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Mixed historical inventory".into(),
            changes: vec![DraftNoteChange::Create {
                path: "retained.md".into(),
                text: "Exact retained note õ\r\n".into(),
            }],
            sources: vec![],
            action_changes: ids
                .iter()
                .zip(titles)
                .map(|(id, title)| ActionChange::Create {
                    id: *id,
                    data: action_data(title),
                })
                .collect(),
        })
        .unwrap();
    let created_request = approval(&review);
    let created_receipt = app.approve_proposal(&created_request).unwrap();
    let created_page = app.activity(&ActivityRequest::default()).unwrap();
    let created_json = serde_json::to_value(&created_page).unwrap();
    assert_eq!(
        created_json["entries"][0]["action_changes"],
        serde_json::json!([
            {"kind":"created", "action_id":ids[0], "title":titles[0]}, {"kind":"created", "action_id":ids[1], "title":titles[1]}
        ])
    );
    assert_eq!(created_page.entries[0].changes.len(), 1);
    let before = app.action(ids[0]).unwrap();
    let mut changed = before.data.clone();
    changed.title = "Approved edited õ\nλ".into();
    let edit = app
        .create_proposal(&DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Replace named Action".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Replace {
                before: Box::new(before.clone()),
                data: changed.clone(),
            }],
        })
        .unwrap();
    let edited_request = approval(&edit);
    let edited_receipt = app.approve_proposal(&edited_request).unwrap();
    let edited_page = app.activity(&ActivityRequest::default()).unwrap();
    let edited_entry = edited_page
        .entries
        .iter()
        .find(|entry| entry.operation_id == edited_request.operation_id)
        .unwrap();
    assert_eq!(
        serde_json::to_value(edited_entry).unwrap()["action_changes"],
        serde_json::json!([{"kind":"replaced", "action_id":ids[0], "title":changed.title}])
    );
    assert_eq!(
        edited_page
            .entries
            .iter()
            .find(|entry| entry.operation_id == created_request.operation_id)
            .unwrap(),
        &created_page.entries[0]
    );
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: edited_request.operation_id,
        trash_member: None,
    };
    app.preview_proposal_undo(&undo).unwrap();
    let undo_receipt = app.undo_proposal(&undo).unwrap();
    assert_eq!(undo_receipt.outcome, ApplyOutcome::Applied);
    let history = app.activity(&ActivityRequest::default()).unwrap();
    let undo_entry = history
        .entries
        .iter()
        .find(|entry| entry.operation_id == undo.operation_id)
        .unwrap();
    assert_eq!(
        undo_entry.undo.as_ref().unwrap().operation_id,
        edited_request.operation_id
    );
    assert_eq!(
        serde_json::to_value(undo_entry).unwrap()["action_changes"],
        serde_json::json!([{"kind":"replaced", "action_id":ids[0], "title":titles[0]}])
    );
    let completion = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(app.action(ids[0]).unwrap()),
        sent_source: None,
    };
    let completed = app.complete_action(&completion).unwrap();
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), history);
    assert_eq!(
        app.approve_proposal(&created_request).unwrap(),
        created_receipt
    );
    assert_eq!(
        app.approve_proposal(&edited_request).unwrap(),
        edited_receipt
    );
    assert_eq!(app.undo_proposal(&undo).unwrap(), undo_receipt);
    assert_eq!(app.action(ids[0]).unwrap(), completed.after);
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), history);
    let encoded = serde_json::to_string(&history).unwrap();
    for body in [
        "PRIVATE ACTION DESCRIPTION",
        "description",
        "origin",
        "fingerprint",
        "Exact retained note",
        "before_text",
    ] {
        assert!(!encoded.contains(body), "Activity must omit {body}");
    }
    let mut cursor = None;
    let mut operations = vec![];
    loop {
        let page = app
            .activity(&ActivityRequest {
                limit: 1,
                before: cursor,
            })
            .unwrap();
        assert_eq!(page.entries.len(), 1);
        operations.push(page.entries[0].operation_id);
        cursor = page.next_before;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        operations,
        history
            .entries
            .iter()
            .map(|entry| entry.operation_id)
            .collect::<Vec<_>>()
    );
    drop(app);
    let app = f.app();
    assert_eq!(app.activity(&ActivityRequest::default()).unwrap(), history);
    assert_eq!(app.action(ids[0]).unwrap(), completed.after);
    assert_eq!(
        fs::read(f.vault.join("retained.md")).unwrap(),
        b"Exact retained note \xc3\xb5\r\n"
    );
}
