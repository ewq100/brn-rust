use brn_threads_core::*;

#[test]
fn saved_tail_fact_is_readable_after_restart_without_mixing_versions() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("fresh");
    let mut store = Store::open(&data).unwrap();
    let markdown = format!(
        "# Main\n\n{}\n## Tail evidence\n\nHarbor's final berth is 47. Õun ja öö.\n\n# Appendix\n\nLater material.\n",
        "Stored ordinary prose. Õun.\n".repeat(7000)
    );
    let operation = store.allocate_operation("long saved fixture").unwrap();
    let id = operation.creation_id(0);
    let request = ChangeRequest {
        reason: "synthetic long note".into(),
        writes: vec![Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working("Long saved note", &markdown)),
        }],
        inputs: vec![],
    };
    store
        .prepare_owner(&operation, &request, &HostAuthority::owner("fixture"))
        .unwrap();
    assert!(matches!(
        store
            .apply(&operation, &HostAuthority::owner("fixture"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    drop(store);
    let mut reopened = Store::open(&data).unwrap();
    let tail = note_headings(&markdown)
        .into_iter()
        .find(|heading| heading.title == "Tail evidence")
        .unwrap();
    assert!(tail.start > MAX_NOTE_READ_BYTES);
    let NoteReadOutcome::Read { note } = reopened
        .read_note_range(&id, 1, tail.start, 4096, true)
        .unwrap()
    else {
        panic!()
    };
    assert!(note.markdown.contains("final berth is 47"));
    assert!(!note.markdown.contains("Later material"));
    assert!(note.next_start.is_none());
    assert_eq!(note.end, note.section_end.unwrap());
    let mut cursor = 0;
    let mut reconstructed = String::new();
    loop {
        let NoteReadOutcome::Read { note } = reopened
            .read_note_range(&id, 1, cursor, MAX_NOTE_READ_BYTES, false)
            .unwrap()
        else {
            panic!()
        };
        assert!(note.markdown.len() <= MAX_NOTE_READ_BYTES);
        assert!(note.end > note.start);
        reconstructed.push_str(&note.markdown);
        match note.next_start {
            Some(next) => cursor = next,
            None => break,
        }
    }
    assert_eq!(reconstructed, markdown);
    let session = reopened.begin_edit(&id, 1).unwrap();
    reopened
        .update_buffer(&session.id, 1, "New owner wording")
        .unwrap();
    assert!(matches!(
        reopened.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    assert_eq!(
        reopened
            .read_note_range(&id, 1, cursor, 4096, false)
            .unwrap(),
        NoteReadOutcome::Stale {
            id: id.clone(),
            expected_version: 1,
            current_version: 2
        }
    );
    assert!(
        reopened
            .read_note_range(&id, 2, 0, MAX_NOTE_READ_BYTES + 1, false)
            .is_err()
    );
}

#[test]
fn headings_ignore_fenced_code_and_range_cursors_preserve_utf8() {
    assert_eq!(
        note_headings("# Real\n```md\n# Not a section\n```\n## Detail\n")
            .iter()
            .map(|heading| heading.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Real", "Detail"]
    );
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    let op = store.allocate_operation("utf8 fixture").unwrap();
    let id = op.creation_id(0);
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "UTF-8".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Unicode", "õun")),
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    store.apply(&op, &HostAuthority::owner("fixture")).unwrap();
    assert!(store.read_note_range(&id, 1, 1, 3, false).is_err());
    assert!(store.read_note_range(&id, 1, 0, 1, false).is_err());
    let NoteReadOutcome::Read { note } = store.read_note_range(&id, 1, 0, 3, false).unwrap() else {
        panic!()
    };
    assert_eq!(note.markdown, "õu");
    assert_eq!(note.next_start, Some(3));
}
