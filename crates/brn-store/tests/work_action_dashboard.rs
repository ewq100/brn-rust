use brn_store::{
    WorkStore,
    work::{
        action_completion::CompleteActionRequest,
        actions::dashboard::{ActionDashboardFilter as Filter, ActionDashboardRequest as Request},
        actions::{ActionCursor, ActionData, ActionRecord, ActionState},
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
fn request(filter: Filter) -> Request {
    Request {
        as_of: "2028-02-29".into(),
        filter,
        limit: 25,
        before: None,
    }
}
fn approved(
    store: &mut WorkStore,
    state: ActionState,
    due: Option<&str>,
    follow: Option<&str>,
    dependencies: Vec<Uuid>,
) -> ActionRecord {
    let id = Uuid::new_v4();
    let data = ActionData {
        title: "Approved õ 日本語\r\n".into(),
        description: "Exact details\t\r\n".into(),
        state: if state == ActionState::Completed {
            ActionState::Open
        } else {
            state
        },
        owner: Some("Zoë".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: due.map(str::to_owned),
        follow_up_on: follow.map(str::to_owned),
        dependencies,
        parent: None,
        follows_up: None,
        priority: None,
    };
    let review = store
        .create_proposal(&ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Dashboard fixture review".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create { id, data }],
        })
        .unwrap();
    let op = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: op,
            expected: review.stamp(),
        })
        .unwrap();
    store.record_proposal_prepared(op, &[]).unwrap();
    store
        .finish_proposal_apply(op, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    let before = store.action(id).unwrap().unwrap();
    if state == ActionState::Completed {
        store
            .complete_action_with(
                &CompleteActionRequest {
                    operation_id: Uuid::new_v4(),
                    before: Box::new(before),
                },
                0,
                |_| Ok(()),
            )
            .unwrap()
            .after
    } else {
        before
    }
}

#[test]
fn global_counts_and_date_filters_include_old_work_beyond_first_page_and_survive_restart() {
    let (dir, mut store) = fixture();
    let specifications = [
        (ActionState::Open, Some("2028-02-28"), Some("2028-03-01")),
        (ActionState::Waiting, Some("2028-02-29"), Some("2028-02-29")),
        (ActionState::Blocked, Some("2028-03-01"), Some("2028-02-28")),
        (
            ActionState::Completed,
            Some("2028-02-28"),
            Some("2028-02-28"),
        ),
        (ActionState::Open, None, None),
        (ActionState::Waiting, Some("2028-02-28"), Some("2028-02-29")),
        (ActionState::Blocked, Some("2028-02-28"), Some("2028-03-01")),
        (
            ActionState::Completed,
            Some("2028-02-29"),
            Some("2028-02-29"),
        ),
    ];
    let records: Vec<_> = specifications
        .into_iter()
        .map(|(s, d, f)| approved(&mut store, s, d, f, vec![]))
        .collect();
    for _ in 0..35 {
        approved(&mut store, ActionState::Open, None, None, vec![]);
    }
    let first = store.action_dashboard(&request(Filter::Active)).unwrap();
    first.validate(&request(Filter::Active)).unwrap();
    assert_eq!(
        (
            first.counts.open,
            first.counts.waiting,
            first.counts.blocked,
            first.counts.completed,
            first.counts.overdue,
            first.counts.follow_up
        ),
        (37, 2, 2, 2, 3, 3)
    );
    assert_eq!(first.entries.len(), 25);
    assert!(first.entries.iter().all(|e| e.action.data.due_on.is_none()));
    assert!(first.next_before.is_some());
    for (filter, n) in [
        (Filter::Active, 41),
        (Filter::Open, 37),
        (Filter::Waiting, 2),
        (Filter::Blocked, 2),
        (Filter::Completed, 2),
        (Filter::Overdue, 3),
        (Filter::FollowUp, 3),
        (Filter::All, 43),
    ] {
        let mut input = request(filter);
        input.limit = 200;
        let page = store.action_dashboard(&input).unwrap();
        page.validate(&input).unwrap();
        assert_eq!(page.counts, first.counts);
        assert_eq!(page.entries.len(), n);
        assert!(page.next_before.is_none());
        if filter == Filter::Overdue {
            let mut ids: Vec<_> = page.entries.iter().map(|e| e.action.origin.id).collect();
            ids.sort();
            let mut expected = vec![
                records[0].origin.id,
                records[5].origin.id,
                records[6].origin.id,
            ];
            expected.sort();
            assert_eq!(ids, expected);
        }
    }
    let mut later = request(Filter::Active);
    later.before = first.next_before;
    let second = store.action_dashboard(&later).unwrap();
    assert_eq!(second.counts, first.counts);
    assert_eq!(second.entries.len(), 16);
    assert!(second.entries.iter().all(|e| {
        !first
            .entries
            .iter()
            .any(|old| old.action.origin.id == e.action.origin.id)
    }));
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.action_dashboard(&request(Filter::Active)).unwrap(),
        first
    );
}

#[test]
fn dependencies_are_same_snapshot_exact_order_and_missing_never_means_completed() {
    let (_dir, mut store) = fixture();
    let done = approved(&mut store, ActionState::Completed, None, None, vec![]);
    let waiting = approved(&mut store, ActionState::Waiting, None, None, vec![]);
    let missing = Uuid::new_v4();
    let dependent = approved(
        &mut store,
        ActionState::Open,
        Some("2028-02-28"),
        Some("2028-02-29"),
        vec![waiting.origin.id, missing, done.origin.id],
    );
    let page = store.action_dashboard(&request(Filter::Open)).unwrap();
    page.validate(&request(Filter::Open)).unwrap();
    let [entry] = page.entries.as_slice() else {
        panic!("one Open action")
    };
    assert_eq!(entry.action, dependent);
    assert!(entry.overdue && entry.follow_up && entry.dependency_blocked);
    assert_eq!(
        entry
            .dependencies
            .iter()
            .map(|d| (d.id, d.state))
            .collect::<Vec<_>>(),
        vec![
            (waiting.origin.id, Some(ActionState::Waiting)),
            (missing, None),
            (done.origin.id, Some(ActionState::Completed))
        ]
    );
    store
        .complete_action_with(
            &CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: Box::new(waiting),
            },
            0,
            |_| Ok(()),
        )
        .unwrap();
    let refreshed = store.action_dashboard(&request(Filter::Open)).unwrap();
    assert_eq!(
        refreshed.entries[0].dependencies[0].state,
        Some(ActionState::Completed)
    );
    assert!(refreshed.entries[0].dependency_blocked);
    assert_eq!(store.action(dependent.origin.id).unwrap(), Some(dependent));
}

#[test]
fn explicit_completion_removes_date_signals_without_reopening_or_changing_other_fields() {
    let (_dir, mut store) = fixture();
    let before = approved(
        &mut store,
        ActionState::Waiting,
        Some("2028-02-28"),
        Some("2028-02-29"),
        vec![],
    );
    let input = request(Filter::Active);
    let initial = store.action_dashboard(&input).unwrap();
    assert_eq!(
        (
            initial.counts.waiting,
            initial.counts.overdue,
            initial.counts.follow_up
        ),
        (1, 1, 1)
    );
    let completion = store
        .complete_action_with(
            &CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: Box::new(before),
            },
            0,
            |_| Ok(()),
        )
        .unwrap();
    let after = store.action_dashboard(&input).unwrap();
    assert!(after.entries.is_empty());
    assert_eq!(
        (
            after.counts.waiting,
            after.counts.completed,
            after.counts.overdue,
            after.counts.follow_up
        ),
        (0, 1, 0, 0)
    );
    let completed = store.action_dashboard(&request(Filter::Completed)).unwrap();
    assert_eq!(completed.entries[0].action, completion.after);
    assert!(!completed.entries[0].overdue && !completed.entries[0].follow_up);
}

#[test]
fn canonical_date_and_page_validation_refuse_before_any_query() {
    let (_dir, store) = fixture();
    for date in [
        "",
        "2028-2-29",
        "0000-01-01",
        "2027-02-29",
        "2028-02-30",
        "2028-13-01",
        "10000-01-01",
        "2028-02-29\n",
    ] {
        let mut input = request(Filter::Active);
        input.as_of = date.into();
        assert!(input.validate().is_err());
        assert!(store.action_dashboard(&input).is_err());
    }
    for date in ["0001-01-01", "9999-12-31", "2000-02-29"] {
        let mut input = request(Filter::All);
        input.as_of = date.into();
        assert!(store.action_dashboard(&input).unwrap().entries.is_empty());
    }
    for limit in [0, 201, usize::MAX] {
        let mut input = request(Filter::All);
        input.limit = limit;
        assert!(input.validate().is_err());
    }
    let mut input = request(Filter::All);
    input.before = Some(ActionCursor {
        created_at_ms: 0,
        id: Uuid::nil(),
    });
    assert!(input.validate().is_err());
}

// Deliberate SQL fixtures qualify tied ordering and corruption detection, not
// application production writes. Real approval/restart is exercised above.
fn rewrite_fixture(dir: &std::path::Path, record: &ActionRecord) {
    record.validate().unwrap();
    let bytes = serde_json::to_vec(record).unwrap();
    let origin = serde_json::to_vec(&record.origin).unwrap();
    Connection::open(dir.join("brn.sqlite")).unwrap().execute("UPDATE actions SET created_at_ms=?2,creation_sha256=?3,record_json=?4,record_sha256=?5 WHERE id=?1",params![record.origin.id.to_string(),record.origin.created_at_ms as i64,Sha256::digest(origin).as_slice(),&bytes,Sha256::digest(&bytes).as_slice()]).unwrap();
}

#[test]
fn tied_creation_cursor_is_exclusive_stable_and_counts_ignore_cursor() {
    let (dir, mut store) = fixture();
    let mut records = vec![];
    for _ in 0..4 {
        let mut record = approved(&mut store, ActionState::Open, None, None, vec![]);
        record.origin.created_at_ms = 42;
        record.updated_at_ms = 42;
        rewrite_fixture(dir.path(), &record);
        records.push(record);
    }
    records.sort_by_key(|r| std::cmp::Reverse(r.origin.id));
    let mut input = request(Filter::All);
    input.limit = 1;
    for record in records {
        let page = store.action_dashboard(&input).unwrap();
        page.validate(&input).unwrap();
        assert_eq!(page.counts.open, 4);
        assert_eq!(page.entries[0].action, record);
        input.before = page.next_before;
    }
    assert!(input.before.is_none());
}

#[test]
fn corrupt_record_outside_visible_page_refuses_the_whole_dashboard() {
    let (dir, mut store) = fixture();
    let old = approved(&mut store, ActionState::Open, None, None, vec![]);
    approved(&mut store, ActionState::Open, None, None, vec![]);
    Connection::open(dir.path().join("brn.sqlite"))
        .unwrap()
        .execute(
            "UPDATE actions SET record_sha256=zeroblob(32) WHERE id=?1",
            [old.origin.id.to_string()],
        )
        .unwrap();
    let mut input = request(Filter::All);
    input.limit = 1;
    assert!(store.action_dashboard(&input).is_err());
}

#[test]
fn reply_validation_rejects_wrong_date_page_binding_and_dependency_facts() {
    let (_dir, mut store) = fixture();
    approved(
        &mut store,
        ActionState::Open,
        Some("2028-02-28"),
        None,
        vec![],
    );
    let input = request(Filter::Active);
    let page = store.action_dashboard(&input).unwrap();
    page.validate(&input).unwrap();
    let mut wrong = page.clone();
    wrong.as_of = "2028-03-01".into();
    assert!(wrong.validate(&input).is_err());
    let mut wrong = page.clone();
    wrong.entries[0].overdue = false;
    assert!(wrong.validate(&input).is_err());
    let mut wrong = page.clone();
    wrong.counts.open = 0;
    assert!(wrong.validate(&input).is_err());
    let mut wrong = page.clone();
    wrong.entries[0].dependency_blocked = true;
    assert!(wrong.validate(&input).is_err());
    let mut wrong = page.clone();
    wrong.next_before = Some(ActionCursor {
        created_at_ms: 0,
        id: Uuid::new_v4(),
    });
    assert!(wrong.validate(&input).is_err());
}
