//! The SELECT-only inspection pages, with data written only through `Store`.
use brn_threads_core::*;
use tempfile::TempDir;

fn store() -> (TempDir, Store) {
    let dir = TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let store = Store::open(dir.path().join("data")).unwrap();
    (dir, store)
}
fn owner() -> HostAuthority {
    HostAuthority::owner("synthetic owner")
}
fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "synthetic change".into(),
        writes,
        inputs: vec![],
    }
}
fn created(op: &OperationId, index: usize, data: RecordData) -> Put {
    Put {
        id: op.creation_id(index),
        expected_version: None,
        archived: false,
        data,
    }
}
/// Prepares and applies the writes built for the new operation.
fn commit(store: &mut Store, key: &str, writes: impl Fn(&OperationId) -> Vec<Put>) -> Receipt {
    let op = store.allocate_operation(key).unwrap();
    store
        .prepare_owner(&op, &request(writes(&op)), &owner())
        .unwrap();
    match store.apply(&op, &owner()).unwrap() {
        ApplyOutcome::Applied(receipt) => receipt,
        outcome => panic!("{outcome:?}"),
    }
}
fn new_note(store: &mut Store, key: &str) -> Record {
    commit(store, key, |op| {
        vec![created(op, 0, RecordData::Note(Note::working(key, "text")))]
    })
    .writes[0]
        .after
        .clone()
}
fn edit_note(store: &mut Store, key: &str, note: &Record) -> Record {
    let RecordData::Note(mut data) = note.data.clone() else {
        panic!()
    };
    data.markdown.push_str(" more");
    commit(store, key, |_| {
        vec![Put {
            id: note.id.clone(),
            expected_version: Some(note.version),
            archived: false,
            data: RecordData::Note(data.clone()),
        }]
    })
    .writes[0]
        .after
        .clone()
}
fn thread(title: &str) -> RecordData {
    RecordData::Thread(Thread {
        private_context: false,
        context_epoch: 0,
        title: title.into(),
        state: ThreadState::Open,
        attention: vec![],
    })
}
fn run(thread: &str, state: RunState) -> RecordData {
    RecordData::Run(Run {
        instruction: None,
        queue_claim: None,
        thread: thread.into(),
        provider: "synthetic".into(),
        model: "synthetic".into(),
        guide_identity: "v1".into(),
        budget: 3,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence: 1,
        state,
        progress: "starting".into(),
        operations: vec![],
        sources: vec![],
    })
}

#[test]
fn receipt_pages_never_repeat_and_see_later_receipts() {
    let (_dir, mut store) = store();
    for i in 0..5 {
        new_note(&mut store, &format!("n{i}"));
    }
    let (first, more) = store.receipt_page(0, 2, None).unwrap();
    assert_eq!(first.len(), 2);
    assert!(more);
    // A receipt committed between pages is seen later, and nothing repeats.
    new_note(&mut store, "late");
    let mut seen: Vec<String> = first
        .iter()
        .map(|(_, r)| r.operation.as_str().to_owned())
        .collect();
    let mut after = first.last().unwrap().0;
    loop {
        let (page, more) = store.receipt_page(after, 2, None).unwrap();
        seen.extend(page.iter().map(|(_, r)| r.operation.as_str().to_owned()));
        if !more {
            break;
        }
        after = page.last().unwrap().0;
    }
    let all: Vec<String> = store
        .receipts()
        .unwrap()
        .into_iter()
        .map(|r| r.operation.as_str().to_owned())
        .collect();
    assert_eq!(seen, all);
    assert_eq!(store.receipt_count(None).unwrap(), 6);
}

#[test]
fn receipt_page_filters_by_record_and_refuses_unknown_records() {
    let (_dir, mut store) = store();
    let a = new_note(&mut store, "a");
    new_note(&mut store, "b");
    let a2 = edit_note(&mut store, "a-edit", &a);
    let (page, more) = store.receipt_page(0, 10, Some(&a.id)).unwrap();
    assert!(!more);
    assert_eq!(page.len(), 2);
    assert!(
        page.iter()
            .all(|(_, r)| r.writes.iter().any(|w| w.after.id == a.id))
    );
    assert_eq!(a2.version, 2);
    assert_eq!(store.receipt_count(Some(&a.id)).unwrap(), 2);
    assert!(matches!(
        store.receipt_page(0, 10, Some("no-such-record")),
        Err(Error::Missing(_))
    ));
    assert!(matches!(
        store.receipt_count(Some("no-such-record")),
        Err(Error::Missing(_))
    ));
}

#[test]
fn candidate_page_lists_only_unapplied_unretired_candidates() {
    let (_dir, mut store) = store();
    // One active candidate per note, so each candidate edits its own note.
    let edit = |store: &mut Store, key: &str| {
        let note = new_note(store, &format!("{key}-base"));
        let op = store.allocate_operation(key).unwrap();
        let RecordData::Note(mut data) = note.data.clone() else {
            panic!()
        };
        data.markdown = key.into();
        store
            .prepare(
                &op,
                &request(vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(data),
                }]),
            )
            .unwrap();
        op
    };
    let kept = edit(&mut store, "kept");
    let retired = edit(&mut store, "retired");
    store.retire(&retired).unwrap();
    let second = edit(&mut store, "second");
    let (page, more) = store.candidate_page(0, 1).unwrap();
    assert!(more);
    assert_eq!(page[0].prepared.operation, kept);
    let (rest, more) = store.candidate_page(page[0].rowid, 10).unwrap();
    assert!(!more);
    assert_eq!(
        rest.iter()
            .map(|c| &c.prepared.operation)
            .collect::<Vec<_>>(),
        vec![&second]
    );
    store.apply(&kept, &owner()).unwrap();
    let (page, _) = store.candidate_page(0, 10).unwrap();
    assert!(page.iter().all(|c| c.prepared.operation != kept));
}

#[test]
fn record_page_filters_by_kind_in_id_order_with_a_cursor() {
    let (_dir, mut store) = store();
    let a = new_note(&mut store, "a");
    let b = new_note(&mut store, "b");
    commit(&mut store, "chat", |op| {
        vec![created(op, 0, thread("chat"))]
    });
    let mut notes = [a.id.clone(), b.id.clone()];
    notes.sort();
    let (page, more) = store.record_page(Some("Note"), "", 1).unwrap();
    assert!(more);
    assert_eq!(page[0].id, notes[0]);
    let (page, more) = store.record_page(Some("Note"), &notes[0], 10).unwrap();
    assert!(!more);
    assert_eq!(
        page.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        vec![notes[1].clone()]
    );
    let (all, _) = store.record_page(None, "", 100).unwrap();
    assert_eq!(all.len(), store.records().unwrap().len());
    assert!(matches!(
        store.record_page(Some("Bogus"), "", 10),
        Err(Error::Invalid(_))
    ));
    // A kind is matched as a bound JSON path, never formatted into SQL.
    assert!(matches!(
        store.record_page(Some("Note') OR 1=1 --"), "", 10),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn run_page_filters_by_thread_and_state() {
    let (_dir, mut store) = store();
    let receipt = commit(&mut store, "runs", |op| {
        vec![
            created(op, 0, thread("one")),
            created(op, 1, thread("two")),
            created(op, 2, run(&op.creation_id(0), RunState::Working)),
            created(op, 3, run(&op.creation_id(1), RunState::Completed)),
        ]
    });
    let one = receipt.writes[0].after.id.clone();
    let working = receipt.writes[2].after.id.clone();
    let (all, more) = store.run_page(None, None, "", 10).unwrap();
    assert_eq!(all.len(), 2);
    assert!(!more);
    let (page, _) = store.run_page(Some(&one), None, "", 10).unwrap();
    assert_eq!(
        page.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        vec![working.clone()]
    );
    let (page, _) = store
        .run_page(None, Some(RunState::Working), "", 10)
        .unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].id, working);
    let (page, _) = store
        .run_page(None, Some(RunState::Failed), "", 10)
        .unwrap();
    assert!(page.is_empty());
    let (first, more) = store.run_page(None, None, "", 1).unwrap();
    assert!(more);
    let (rest, more) = store.run_page(None, None, &first[0].id, 1).unwrap();
    assert!(!more);
    assert_ne!(rest[0].id, first[0].id);
}

#[test]
fn operation_lookup_reports_all_four_states_and_never_allocates() {
    let (_dir, mut store) = store();
    assert_eq!(store.operation_by_host_key("never").unwrap(), None);
    assert_eq!(store.operation_by_host_key("never").unwrap(), None);
    let allocated = store.allocate_operation("allocated").unwrap();
    assert_eq!(
        store.operation_by_host_key("allocated").unwrap(),
        Some(allocated.clone())
    );
    let info = store.operation_info(allocated.as_str()).unwrap();
    assert_eq!(info.state, OperationState::Allocated);
    assert!(info.prepared.is_none() && info.receipt.is_none());
    new_note(&mut store, "applied");
    let applied = store.operation_by_host_key("applied").unwrap().unwrap();
    let info = store.operation_info(applied.as_str()).unwrap();
    assert_eq!(info.state, OperationState::Applied);
    assert!(info.receipt.is_some() && info.prepared.is_some());
    let edit = |store: &mut Store, key: &str| {
        let note = new_note(store, &format!("{key}-base"));
        let op = store.allocate_operation(key).unwrap();
        let RecordData::Note(data) = note.data.clone() else {
            panic!()
        };
        store
            .prepare(
                &op,
                &request(vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(Note {
                        markdown: key.into(),
                        ..data
                    }),
                }]),
            )
            .unwrap();
        op
    };
    let prepared = edit(&mut store, "prepared");
    assert_eq!(
        store.operation_info(prepared.as_str()).unwrap().state,
        OperationState::Prepared
    );
    let retired = edit(&mut store, "retired");
    store.retire(&retired).unwrap();
    assert_eq!(
        store.operation_info(retired.as_str()).unwrap().state,
        OperationState::Retired
    );
    assert!(matches!(
        store.operation_info("no-such-operation"),
        Err(Error::Missing(_))
    ));
}

#[test]
fn page_limits_outside_one_to_a_hundred_are_refused() {
    let (_dir, store) = store();
    for limit in [0, 101] {
        assert!(matches!(
            store.receipt_page(0, limit, None),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.candidate_page(0, limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.record_page(None, "", limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.run_page(None, None, "", limit),
            Err(Error::Invalid(_))
        ));
    }
    assert!(store.receipt_page(0, 100, None).is_ok());
    assert!(store.receipt_page(0, 1, None).is_ok());
}
