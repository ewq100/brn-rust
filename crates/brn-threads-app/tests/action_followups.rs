//! Deterministic service fixtures, not evidence of live meeting interpretation.
use brn_threads_app::*;

fn fixture() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let app = Workspace::create_without_default_credentials(
        root.path().join("data"),
        WorkspacePurpose::Test,
    )
    .unwrap();
    (root, app)
}
fn proposed() -> Action {
    Action {
        kept_outside_project: false,
        chat: None,
        description: "Ask who can confirm supplier approval".into(),
        state: ActionState::Suggested,
        internal: false,
        evidence: None,
        due: None,
        owner: None,
        waiting_on: Some("Supplier approval".into()),
        follow_up: None,
    }
}
fn applied(outcome: ApplyOutcome) -> Receipt {
    let ApplyOutcome::Applied(receipt) = outcome else {
        panic!("{outcome:?}")
    };
    receipt
}
fn edit(record: &Record, data: RecordData) -> Put {
    Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived: record.archived,
        data,
    }
}
fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "Meeting follow-up fixture".into(),
        inputs: vec![],
        writes,
    }
}

#[test]
fn ambiguous_followup_review_replay_reopen_and_undo_preserve_unknowns_without_duplicate_task() {
    let (root, mut app) = fixture();
    let meeting = app
        .new_note(
            "Meeting evidence",
            "Someone could ask the supplier after approval. No owner or date agreed.",
        )
        .unwrap();
    let operation = app
        .store
        .allocate_operation("one source one authored proposal")
        .unwrap();
    let action_id = operation.creation_id(0);
    let mut change = request(vec![Put {
        id: action_id.clone(),
        expected_version: None,
        archived: false,
        data: RecordData::Action(proposed()),
    }]);
    change.inputs.push(RevisionInput {
        record: meeting.id.clone(),
        version: meeting.version,
        expectation: InputExpectation::Current,
    });
    app.store.prepare(&operation, &change).unwrap();
    app.store.prepare(&operation, &change).unwrap();
    assert!(app.store.record(&action_id).unwrap().is_none());
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    let receipt = applied(app.review_apply(operation.as_str()).unwrap().outcome);
    assert_eq!(
        applied(app.review_apply(operation.as_str()).unwrap().outcome),
        receipt
    );
    let current = app.store.record(&action_id).unwrap().unwrap();
    assert_eq!(current.data, RecordData::Action(proposed()));
    assert_eq!(
        app.records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Action(_)))
            .count(),
        1
    );
    assert_eq!(
        app.store.prepared(&operation).unwrap().request.inputs,
        change.inputs
    );
    assert_eq!(app.store.record(&meeting.id).unwrap().unwrap(), meeting);
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(app.store.record(&action_id).unwrap().unwrap(), current);
    applied(app.undo(receipt.operation.as_str()).unwrap());
    assert!(
        app.store
            .record(&action_id)
            .unwrap()
            .is_none_or(|r| r.archived)
    );
    assert_eq!(app.store.record(&meeting.id).unwrap().unwrap(), meeting);
}

#[test]
fn changed_meeting_evidence_stales_followup_without_touching_existing_task() {
    let (_root, mut app) = fixture();
    let meeting = app
        .new_note(
            "Meeting evidence",
            "Mari follows up on 2028-03-02; delivery remains Friday.",
        )
        .unwrap();
    let create = app.store.allocate_operation("existing task").unwrap();
    let mut task = proposed();
    task.state = ActionState::Open;
    task.owner = Some("Mari".into());
    task.follow_up = Some("2028-03-02".into());
    task.due = Some("Friday".into());
    let record = applied(
        app.owner_change(
            "existing task",
            &request(vec![Put {
                id: create.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Action(task.clone()),
            }]),
        )
        .unwrap(),
    )
    .writes[0]
        .after
        .clone();
    let update = app
        .store
        .allocate_operation("updated same commitment")
        .unwrap();
    task.waiting_on = Some("Written confirmation".into());
    let mut change = request(vec![edit(&record, RecordData::Action(task))]);
    change.inputs.push(RevisionInput {
        record: meeting.id.clone(),
        version: meeting.version,
        expectation: InputExpectation::Current,
    });
    app.store.prepare(&update, &change).unwrap();
    applied(
        app.owner_change(
            "correct meeting evidence",
            &request(vec![edit(
                &meeting,
                RecordData::Note(Note::working(
                    "Meeting evidence",
                    "Owner and follow-up date are disputed.",
                )),
            )]),
        )
        .unwrap(),
    );
    assert!(matches!(
        app.review_apply(update.as_str()).unwrap().outcome,
        ApplyOutcome::Stale { .. }
    ));
    assert_eq!(app.store.record(&record.id).unwrap().unwrap(), record);
    assert_eq!(
        app.records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Action(_)))
            .count(),
        1
    );
}

#[test]
fn exact_review_clears_followup_fields_and_undo_restores_them_without_completing_task() {
    let (root, mut app) = fixture();
    let operation = app.store.allocate_operation("known details").unwrap();
    let mut task = proposed();
    task.state = ActionState::Waiting;
    task.owner = Some("Märta".into());
    task.follow_up = Some("2028-02-29".into());
    task.due = Some("2028-03-05".into());
    let record = applied(
        app.owner_change(
            "known details",
            &request(vec![Put {
                id: operation.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Action(task.clone()),
            }]),
        )
        .unwrap(),
    )
    .writes[0]
        .after
        .clone();
    let update = app
        .store
        .allocate_operation("uncertain new evidence")
        .unwrap();
    task.owner = None;
    task.waiting_on = None;
    task.follow_up = None;
    let summary = action_review_text(&task);
    assert!(summary.contains("Owner: Unspecified\nWaiting on: Unspecified"));
    assert!(summary.contains("Due: 2028-03-05\nFollow up: Unspecified"));
    app.store
        .prepare(
            &update,
            &request(vec![edit(&record, RecordData::Action(task.clone()))]),
        )
        .unwrap();
    let receipt = applied(app.review_apply(update.as_str()).unwrap().outcome);
    assert_eq!(
        app.store.record(&record.id).unwrap().unwrap().data,
        RecordData::Action(task)
    );
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    applied(app.undo(receipt.operation.as_str()).unwrap());
    assert_eq!(
        app.store.record(&record.id).unwrap().unwrap().data,
        record.data
    );
}

#[test]
fn concurrent_owner_edit_fences_a_prepared_followup_after_reopen() {
    let (root, mut app) = fixture();
    let create = app.store.allocate_operation("race task").unwrap();
    let mut task = proposed();
    task.owner = Some("Mari".into());
    let record = applied(
        app.owner_change(
            "race task",
            &request(vec![Put {
                id: create.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Action(task.clone()),
            }]),
        )
        .unwrap(),
    )
    .writes[0]
        .after
        .clone();
    task.follow_up = Some("2028-03-09".into());
    let pending = app.store.allocate_operation("delayed followup").unwrap();
    app.store
        .prepare(
            &pending,
            &request(vec![edit(&record, RecordData::Action(task))]),
        )
        .unwrap();
    let RecordData::Action(mut owner_task) = record.data.clone() else {
        panic!()
    };
    owner_task.owner = Some("Leena".into());
    let owner_record = applied(
        app.owner_change(
            "owner reassigned",
            &request(vec![edit(&record, RecordData::Action(owner_task))]),
        )
        .unwrap(),
    )
    .writes[0]
        .after
        .clone();
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert!(matches!(
        app.review_apply(pending.as_str()).unwrap().outcome,
        ApplyOutcome::Stale { .. }
    ));
    assert_eq!(app.store.record(&record.id).unwrap().unwrap(), owner_record);
}
