use super::*;

fn replace(scope: &Scope, note: &str, version: u64, markdown: &str) -> Value {
    prepare(
        scope,
        PrepareArgs {
            request_key: format!("replace-{version}-{}", markdown.len()),
            request: ChangeRequest {
                reason: "Replace the whole note".into(),
                writes: vec![Put {
                    id: note.into(),
                    expected_version: Some(version),
                    archived: false,
                    data: RecordData::Note(Note::working("Decision", markdown)),
                }],
                inputs: vec![],
            },
        },
    )
    .unwrap()
}

fn save_long_note(request: &ThreadRunRequest, note: &str) -> String {
    let markdown = format!(
        "# Main\n\n{}\n## Tail\n\nThe final fact is 47.\n",
        "Ordinary saved material.\n".repeat(7000)
    );
    let mut writer = Store::open(&request.data_dir).unwrap();
    let session = writer.begin_edit(note, 1).unwrap();
    writer.update_buffer(&session.id, 1, &markdown).unwrap();
    assert!(matches!(
        writer.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    markdown
}

#[test]
fn a_replacement_of_a_note_that_was_not_read_is_invalid() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let result = replace(&scope, &note, 1, "A rewrite from a guess");
    assert_eq!(result["outcome"], "Invalid", "{result}");
    assert!(
        result["reason"]
            .as_str()
            .unwrap()
            .contains("whole Markdown was not read")
    );
    assert!(result.get("operation").is_none());
}

#[test]
fn a_whole_exact_id_read_allows_the_replacement() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let read = read_records(
        &scope,
        ReadArgs {
            ids: vec![note.clone()],
            query: None,
        },
    )
    .unwrap();
    assert_ne!(read["records"][0]["truncated"], true);
    let result = replace(&scope, &note, 1, "A rewrite after reading");
    assert!(result.get("operation").is_some(), "{result}");
}

#[test]
fn a_truncated_read_does_not_but_reading_through_to_the_end_does() {
    let (_root, request, note) = tests::fixture();
    let markdown = save_long_note(&request, &note);
    let (scope, _) = begin(&request).unwrap();
    let listed = read_records(
        &scope,
        ReadArgs {
            ids: vec![note.clone()],
            query: None,
        },
    )
    .unwrap();
    assert_eq!(listed["records"][0]["truncated"], true);
    let result = replace(&scope, &note, 2, "A prefix-based rewrite");
    assert_eq!(result["outcome"], "Invalid", "{result}");
    let mut start = 0;
    loop {
        let read = read_note(
            &scope,
            ReadNoteArgs {
                id: note.clone(),
                version: 2,
                start,
                max_bytes: MAX_NOTE_READ_BYTES,
                section: false,
            },
        )
        .unwrap();
        assert_eq!(read["outcome"], "Read");
        match read["note"]["next_start"].as_u64() {
            Some(next) => {
                let early = replace(&scope, &note, 2, "Still too early");
                assert_eq!(early["outcome"], "Invalid", "{early}");
                start = next as usize;
            }
            None => break,
        }
    }
    assert!(start > 0 && start < markdown.len());
    let result = replace(&scope, &note, 2, "A rewrite after reading to the end");
    assert!(result.get("operation").is_some(), "{result}");
}

#[test]
fn a_read_of_an_older_version_does_not_cover_a_newer_one() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    tests::read_all(&scope);
    let mut writer = Store::open(&request.data_dir).unwrap();
    let session = writer.begin_edit(&note, 1).unwrap();
    writer
        .update_buffer(&session.id, 1, "Changed by the owner meanwhile")
        .unwrap();
    writer.save(&session.id, 1).unwrap();
    drop(writer);
    let result = replace(&scope, &note, 2, "A rewrite built on version 1");
    assert_eq!(result["outcome"], "Invalid", "{result}");
}

#[test]
fn a_write_that_keeps_the_markdown_needs_no_read() {
    let (_root, request, note) = tests::fixture();
    let current = {
        let store = Store::open(&request.data_dir).unwrap();
        match store.record(&note).unwrap().unwrap().data {
            RecordData::Note(n) => n.markdown,
            _ => unreachable!(),
        }
    };
    let (scope, _) = begin(&request).unwrap();
    let result = replace(&scope, &note, 1, &current);
    assert!(result.get("operation").is_some(), "{result}");
}

#[test]
fn creating_a_note_needs_no_read() {
    let (_root, request, _) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let result = prepare(
        &scope,
        PrepareArgs {
            request_key: "new".into(),
            request: ChangeRequest {
                reason: "Create a note".into(),
                writes: vec![Put {
                    id: "new:0".into(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("New", "Fresh text")),
                }],
                inputs: vec![],
            },
        },
    )
    .unwrap();
    assert!(result.get("operation").is_some(), "{result}");
}

#[test]
fn an_applied_replacement_counts_as_read_for_the_next_one() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    tests::read_all(&scope);
    let first = replace(&scope, &note, 1, "First rewrite");
    let applied = apply(
        &scope,
        ApplyArgs {
            operation: first["operation"].as_str().unwrap().into(),
        },
    )
    .unwrap();
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    let second = replace(&scope, &note, 2, "Second rewrite");
    assert!(second.get("operation").is_some(), "{second}");
}

#[test]
fn an_owner_edit_grant_still_needs_the_note_read() {
    let (_root, mut request, note) = tests::fixture();
    request.owner_request = Some(OwnerRequest::EditNote {
        id: note.clone(),
        version: 1,
    });
    let (scope, _) = begin(&request).unwrap();
    let result = replace(&scope, &note, 1, "Owner-directed rewrite");
    assert_eq!(result["outcome"], "Invalid", "{result}");
    tests::read_all(&scope);
    let result = replace(&scope, &note, 1, "Owner-directed rewrite");
    assert!(result.get("operation").is_some(), "{result}");
}
