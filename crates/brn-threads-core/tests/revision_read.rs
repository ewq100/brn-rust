use brn_threads_core::*;

#[test]
fn bounded_accepted_revision_pages_and_slices_survive_restart_and_archive() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("fresh");
    let mut store = Store::open(&data).unwrap();
    let operation = store.allocate_operation("first").unwrap();
    let id = operation.creation_id(0);
    let original = format!(
        "Õun. {}Distinctive original tail.",
        "Evidence. ".repeat(20_000)
    );
    let first = ChangeRequest {
        reason: "Accepted original".into(),
        writes: vec![Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working("History", &original)),
        }],
        inputs: vec![],
    };
    let owner = HostAuthority::owner("fixture");
    store.prepare_owner(&operation, &first, &owner).unwrap();
    store.apply(&operation, &owner).unwrap();
    for version in 1..35 {
        let operation = store
            .allocate_operation(&format!("revision-{version}"))
            .unwrap();
        let request = ChangeRequest {
            reason: format!("Accepted revision {version}"),
            writes: vec![Put {
                id: id.clone(),
                expected_version: Some(version),
                archived: version == 34,
                data: RecordData::Note(Note::working("History", format!("Current {version}"))),
            }],
            inputs: vec![],
        };
        store.prepare_owner(&operation, &request, &owner).unwrap();
        assert!(matches!(
            store.apply(&operation, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }
    let pending = store.allocate_operation("pending").unwrap();
    store
        .prepare(
            &pending,
            &ChangeRequest {
                reason: "Never accepted".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: Some(35),
                    archived: false,
                    data: RecordData::Note(Note::working("History", "Not accepted")),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    drop(store);
    let store = Store::open(data).unwrap();
    let page = store.note_revisions(&id, 0).unwrap();
    assert_eq!(page.revisions.len(), 32);
    assert_eq!(page.next_after, Some(32));
    assert_eq!(page.revisions[0].reason, "Accepted original");
    let tail = store.note_revisions(&id, page.next_after.unwrap()).unwrap();
    assert_eq!(
        tail.revisions
            .iter()
            .map(|revision| revision.version)
            .collect::<Vec<_>>(),
        vec![33, 34, 35]
    );
    assert_eq!(tail.next_after, None);
    assert!(
        tail.revisions
            .iter()
            .all(|revision| revision.operation != pending.as_str())
    );
    let first = store
        .read_note_revision(&id, 1, 0, MAX_NOTE_READ_BYTES)
        .unwrap();
    assert_eq!(first.version, 1);
    assert!(first.markdown.starts_with("Õun"));
    assert!(!first.markdown.contains("Distinctive original tail"));
    let last = store
        .read_note_revision(&id, 1, first.next_start.unwrap(), MAX_NOTE_READ_BYTES)
        .unwrap();
    assert!(last.markdown.ends_with("Distinctive original tail."));
    assert!(last.next_start.is_none());
    assert!(store.read_note_revision(&id, 1, 1, 10).is_err());
    assert!(store.read_note_revision(&id, 1, 0, 1).is_err());
    assert!(store.read_note_revision(&id, 0, 0, 10).is_err());
    assert!(store.read_note_revision(&id, 36, 0, 10).is_err());
    assert!(
        store
            .read_note_revision(&id, 1, original.len() + 1, 10)
            .is_err()
    );
    assert!(
        store
            .read_note_revision(&id, 1, 0, MAX_NOTE_READ_BYTES + 1)
            .is_err()
    );
}
