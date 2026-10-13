use brn_threads_core::*;
use rusqlite::Connection;

fn owner_writes(store: &mut Store, key: &str, data: Vec<(RecordData, bool)>) -> Vec<Record> {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    let writes = data
        .into_iter()
        .enumerate()
        .map(|(index, (data, archived))| Put {
            id: op.creation_id(index),
            expected_version: None,
            archived,
            data,
        })
        .collect();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!("synthetic write did not apply")
    };
    receipt.writes.into_iter().map(|w| w.after).collect()
}

fn thread(title: &str) -> RecordData {
    RecordData::Thread(Thread {
        title: title.into(),
        state: ThreadState::Open,
        attention: vec![],
    })
}

fn source(chat: Option<&str>, locator: &str) -> RecordData {
    RecordData::Source(Source {
        chat: chat.map(Into::into),
        locator: locator.into(),
        label: locator.into(),
        observed_at: "2026-10-13T00:00:00Z".into(),
        external_version: None,
        outcome: "converted".into(),
        gaps: vec![],
        report: None,
    })
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// Search eligibility must admit exactly the notes `records_for_chat` lists in
/// every context, for random mixes of chat-only notes, chat-only import
/// sources, archived and superseded notes.
#[test]
fn search_eligibility_matches_core_context_for_random_graphs() {
    for seed in 0..12 {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().canonicalize().unwrap().join("data");
        let mut store = Store::open(&data).unwrap();
        let chats: Vec<String> = owner_writes(
            &mut store,
            "chats",
            vec![(thread("A"), false), (thread("B"), false)],
        )
        .into_iter()
        .map(|r| r.id)
        .collect();
        let mut rng = Rng(seed);
        let pick_chat = |rng: &mut Rng| match rng.below(3) {
            0 => None,
            n => Some(chats[n - 1].clone()),
        };
        let sources: Vec<String> = owner_writes(
            &mut store,
            "sources",
            (0..4)
                .map(|i| {
                    (
                        source(pick_chat(&mut rng).as_deref(), &format!("synthetic://{i}")),
                        false,
                    )
                })
                .collect(),
        )
        .into_iter()
        .map(|r| r.id)
        .collect();
        let targets: Vec<String> = owner_writes(
            &mut store,
            "targets",
            vec![
                (RecordData::Note(Note::working("Target", "general")), false),
                (
                    RecordData::Note(Note {
                        chat: Some(chats[0].clone()),
                        ..Note::working("Chat target", "private")
                    }),
                    false,
                ),
            ],
        )
        .into_iter()
        .map(|r| r.id)
        .collect();
        let notes: Vec<_> = (0..30)
            .map(|i| {
                let mut note = Note::working(format!("Note {i}"), format!("Body {i}"));
                note.chat = pick_chat(&mut rng);
                if rng.below(2) == 0 {
                    note.import = Some(ImportCoverage {
                        source: sources[rng.below(sources.len())].clone(),
                        full_note: false,
                        gaps: vec![],
                        report: None,
                    });
                }
                if rng.below(5) == 0 {
                    note.superseded_by = Some(targets[rng.below(targets.len())].clone());
                }
                (RecordData::Note(note), rng.below(6) == 0)
            })
            .collect();
        let ids: Vec<String> = owner_writes(&mut store, "notes", notes)
            .into_iter()
            .chain(store.records().unwrap())
            .filter(|r| matches!(r.data, RecordData::Note(_)))
            .map(|r| r.id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut reader = Store::open_reader(&data).unwrap();
        let snapshot = reader.snapshot().unwrap();
        for chat in [None, Some(chats[0].as_str()), Some(chats[1].as_str())] {
            let expected: std::collections::BTreeSet<String> = store
                .records_for_chat(chat)
                .unwrap()
                .into_iter()
                .filter(|r| matches!(r.data, RecordData::Note(_)))
                .map(|r| r.id)
                .collect();
            assert_eq!(
                snapshot.searchable_notes(&ids, chat).unwrap(),
                expected,
                "seed {seed} chat {chat:?}"
            );
        }
    }
}

#[test]
fn snapshots_report_a_rising_watermark_and_only_changed_records() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap().join("data");
    let mut store = Store::open(&data).unwrap();
    let mut reader = Store::open_reader(&data).unwrap();
    {
        let empty = reader.snapshot().unwrap();
        assert_eq!(empty.watermark().unwrap(), 0);
        assert_eq!(empty.workspace_identity().unwrap(), None);
    }
    let first = owner_writes(
        &mut store,
        "first",
        vec![
            (RecordData::Note(Note::working("One", "one")), false),
            (RecordData::Note(Note::working("Two", "two")), false),
        ],
    );
    let (mark, identity) = {
        let snapshot = reader.snapshot().unwrap();
        let mark = snapshot.watermark().unwrap();
        assert!(mark > 0);
        assert_eq!(snapshot.changed_since(0).unwrap().len(), 2);
        assert!(snapshot.changed_since(mark).unwrap().is_empty());
        (mark, snapshot.workspace_identity().unwrap().unwrap())
    };
    let pinned = reader.snapshot().unwrap();
    assert_eq!(pinned.watermark().unwrap(), mark);
    let RecordData::Note(mut note) = first[0].data.clone() else {
        unreachable!()
    };
    note.markdown = "one, edited".into();
    let op = store.allocate_operation("edit").unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "edit".into(),
                writes: vec![Put {
                    id: first[0].id.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(note),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    store.apply(&op, &owner).unwrap();
    // A snapshot stays consistent while another connection commits.
    assert_eq!(pinned.watermark().unwrap(), mark);
    assert!(pinned.changed_since(mark).unwrap().is_empty());
    drop(pinned);
    let snapshot = reader.snapshot().unwrap();
    assert!(snapshot.watermark().unwrap() > mark);
    let changed = snapshot.changed_since(mark).unwrap();
    assert_eq!(changed.len(), 1);
    assert_eq!(
        (changed[0].id.as_str(), changed[0].version),
        (first[0].id.as_str(), 2)
    );
    assert_eq!(snapshot.workspace_identity().unwrap().unwrap(), identity);
}

#[test]
fn readers_refuse_unvalidated_or_old_workspaces_without_changing_them() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    assert!(Store::open_reader("relative").is_err());
    assert!(Store::open_reader(base.join("missing")).is_err());
    let data = base.join("legacy");
    let mut store = Store::open(&data).unwrap();
    owner_writes(
        &mut store,
        "knowledge",
        vec![(RecordData::Note(Note::working("Old", "bytes")), false)],
    );
    drop(store);
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    conn.execute_batch(
        "DROP TABLE chat_drafts; DROP INDEX one_chat_control; PRAGMA user_version=3;",
    )
    .unwrap();
    drop(conn);
    assert!(Store::open_reader(&data).is_err());
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
}
