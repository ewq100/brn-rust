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
        loaded_skill_details: vec![],
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

/// Every page of `page(after, 2)` concatenated, checking pages never repeat.
fn all_pages<T: Clone>(
    mut page: impl FnMut(&str) -> (Vec<T>, bool),
    id: impl Fn(&T) -> String,
) -> Vec<T> {
    let mut seen: Vec<T> = vec![];
    let mut after = String::new();
    loop {
        let (rows, more) = page(&after);
        assert!(rows.len() <= 2);
        seen.extend(rows.iter().cloned());
        if !more {
            break;
        }
        after = id(rows.last().unwrap());
    }
    let ids: Vec<String> = seen.iter().map(&id).collect();
    let mut unique = ids.clone();
    unique.dedup();
    assert_eq!(ids, unique, "a page repeated a row");
    seen
}
fn ids(records: &[Record]) -> Vec<String> {
    records.iter().map(|r| r.id.clone()).collect()
}
fn put(record: &Record, archived: bool, data: RecordData) -> Put {
    Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived,
        data,
    }
}
fn project(title: &str) -> RecordData {
    RecordData::Project(Project {
        title: title.into(),
        instructions: String::new(),
        use_brn_knowledge: false,
        chats: vec![],
        documents: vec![],
        trash: None,
    })
}
fn message(thread: &str, role: &str, text: &str) -> RecordData {
    RecordData::Message(Message {
        thread: thread.into(),
        role: role.into(),
        text: text.into(),
    })
}

#[test]
fn note_thread_needs_you_and_project_pages_match_the_app_filters() {
    let (_dir, mut store) = store();
    let open = commit(&mut store, "chat", |op| {
        vec![created(op, 0, thread("open"))]
    })
    .writes[0]
        .after
        .id
        .clone();
    let receipt = commit(&mut store, "fixture", |op| {
        let mut attention = thread("attention");
        if let RecordData::Thread(t) = &mut attention {
            t.attention.push(Attention {
                kind: AttentionKind::Question,
                reason: "synthetic".into(),
                record: None,
            });
        }
        let mut superseded = Note::working("superseded", "old");
        superseded.superseded_by = Some(op.creation_id(2));
        let mut private = Note::working("private", "chat only");
        private.chat = Some(open.clone());
        vec![
            created(op, 0, thread("second")),
            created(op, 1, attention.clone()),
            created(op, 2, RecordData::Note(Note::working("kept", "a"))),
            created(op, 3, RecordData::Note(Note::working("kept-2", "b"))),
            created(op, 4, RecordData::Note(Note::working("kept-3", "c"))),
            created(op, 5, RecordData::Note(superseded)),
            created(op, 6, RecordData::Note(private)),
            created(op, 7, RecordData::Note(Note::working("archived", "d"))),
            created(op, 8, project("p1")),
            created(op, 9, project("p2")),
            created(op, 10, project("trashed")),
            created(op, 11, thread("archived")),
            created(op, 12, attention),
        ]
    });
    let after: Vec<Record> = receipt.writes.iter().map(|w| w.after.clone()).collect();
    commit(&mut store, "archive", |_| {
        vec![
            put(&after[7], true, after[7].data.clone()),
            put(&after[10], true, after[10].data.clone()),
            put(&after[11], true, after[11].data.clone()),
            put(&after[12], true, after[12].data.clone()),
        ]
    });
    let records = store.records().unwrap();
    let expect = |keep: &dyn Fn(&Record) -> bool| -> Vec<String> {
        records
            .iter()
            .filter(|r| keep(r))
            .map(|r| r.id.clone())
            .collect()
    };
    let notes = all_pages(|a| store.note_page(a, 2).unwrap(), |r| r.id.clone());
    assert_eq!(
        ids(&notes),
        expect(&|r| !r.archived
            && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none() && n.chat.is_none()))
    );
    assert_eq!(notes.len(), 3);
    let threads = all_pages(|a| store.thread_page(a, 2).unwrap(), |r| r.id.clone());
    assert_eq!(
        ids(&threads),
        expect(&|r| !r.archived && matches!(r.data, RecordData::Thread(_)))
    );
    assert_eq!(threads.len(), 3);
    // An archived chat with attention stays in needs-you.
    let needs = all_pages(|a| store.needs_you_page(a, 2).unwrap(), |r| r.id.clone());
    assert_eq!(
        ids(&needs),
        expect(&|r| matches!(&r.data, RecordData::Thread(t) if !t.attention.is_empty()))
    );
    assert_eq!(needs.len(), 2);
    assert!(needs.iter().any(|r| r.archived));
    for include_trash in [false, true] {
        let projects = all_pages(
            |a| store.project_page(include_trash, a, 2).unwrap(),
            |r| r.id.clone(),
        );
        assert_eq!(
            ids(&projects),
            expect(&|r| (include_trash || !r.archived) && matches!(r.data, RecordData::Project(_)))
        );
        assert_eq!(projects.len(), if include_trash { 3 } else { 2 });
    }
    // Pages return the full records, exactly as stored.
    assert_eq!(
        notes,
        records
            .iter()
            .filter(|r| ids(&notes).contains(&r.id))
            .cloned()
            .collect::<Vec<_>>()
    );
}

#[test]
fn recovery_page_lists_open_sessions_in_id_order() {
    let (_dir, mut store) = store();
    let mut open = vec![];
    for i in 0..4 {
        let note = new_note(&mut store, &format!("s{i}"));
        open.push(store.begin_edit(&note.id, 1).unwrap());
    }
    let closed = open.pop().unwrap();
    assert!(matches!(
        store.discard(&closed.id, 0).unwrap(),
        BufferOutcome::Updated(_)
    ));
    let sessions = all_pages(|a| store.recovery_page(a, 2).unwrap(), |s| s.id.clone());
    assert_eq!(sessions, store.recovery_buffers().unwrap());
    assert_eq!(sessions.len(), 3);
    assert!(sessions.iter().all(|s| s.id != closed.id));
}

#[test]
fn message_page_keeps_creation_order_per_chat() {
    let (_dir, mut store) = store();
    let chats = commit(&mut store, "chats", |op| {
        vec![created(op, 0, thread("one")), created(op, 1, thread("two"))]
    });
    let (one, two) = (
        chats.writes[0].after.id.clone(),
        chats.writes[1].after.id.clone(),
    );
    let mut written = vec![];
    for (i, (chat, role)) in [
        (&one, "owner"),
        (&two, "owner"),
        (&one, "assistant-partial"),
        (&one, "assistant"),
        (&one, "owner"),
        (&one, "owner"),
    ]
    .iter()
    .enumerate()
    {
        let receipt = commit(&mut store, &format!("m{i}"), |op| {
            vec![created(op, 0, message(chat, role, &format!("text {i}")))]
        });
        written.push(receipt.writes[0].after.clone());
    }
    // Editing an earlier message keeps its position; archiving drops one.
    commit(&mut store, "edit", |_| {
        vec![put(&written[0], false, message(&one, "owner", "edited"))]
    });
    commit(&mut store, "drop", |_| {
        vec![put(&written[5], true, written[5].data.clone())]
    });
    let mut seen = vec![];
    let mut after = 0;
    loop {
        let (rows, more) = store.message_page(&one, after, 2).unwrap();
        assert!(rows.len() <= 2);
        assert!(rows.iter().all(|(pos, _)| *pos > after));
        seen.extend(rows.iter().map(|(_, r)| r.clone()));
        if !more {
            break;
        }
        after = rows.last().unwrap().0;
    }
    let roles: Vec<String> = seen
        .iter()
        .map(|r| match &r.data {
            RecordData::Message(m) => m.role.clone(),
            _ => panic!(),
        })
        .collect();
    assert_eq!(roles, ["owner", "assistant-partial", "assistant", "owner"]);
    assert_eq!(seen[0].id, written[0].id);
    assert_eq!(seen[0].version, 2);
    let (other, more) = store.message_page(&two, 0, 100).unwrap();
    assert!(!more);
    assert_eq!(
        ids(&other.into_iter().map(|(_, r)| r).collect::<Vec<_>>()),
        [written[1].id.clone()]
    );
    assert!(
        store
            .message_page("no-such-chat", 0, 10)
            .unwrap()
            .0
            .is_empty()
    );
}

#[test]
fn list_page_limits_outside_one_to_a_hundred_are_refused() {
    let (_dir, store) = store();
    for limit in [0, 101] {
        assert!(matches!(store.note_page("", limit), Err(Error::Invalid(_))));
        assert!(matches!(
            store.thread_page("", limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.needs_you_page("", limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.project_page(true, "", limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.recovery_page("", limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.message_page("c", 0, limit),
            Err(Error::Invalid(_))
        ));
    }
    assert!(store.note_page("", 100).unwrap().0.is_empty());
}
