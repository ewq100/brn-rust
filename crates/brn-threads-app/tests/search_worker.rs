use brn_threads_app::search::{SearchHandle, SearchWorker, index_path};
use brn_threads_app::*;
use std::path::PathBuf;

fn workspace() -> (tempfile::TempDir, PathBuf, Workspace) {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap().join("data");
    let workspace = Workspace::open(&data).unwrap();
    (root, data, workspace)
}

fn write(
    workspace: &mut Workspace,
    key: &str,
    puts: Vec<(Option<&Record>, RecordData, bool)>,
) -> Vec<Record> {
    let op = workspace.store.allocate_operation(key).unwrap();
    let writes = puts
        .into_iter()
        .enumerate()
        .map(|(i, (prior, data, archived))| Put {
            id: prior.map_or_else(|| op.creation_id(i), |r| r.id.clone()),
            expected_version: prior.map(|r| r.version),
            archived,
            data,
        })
        .collect();
    let ApplyOutcome::Applied(receipt) = workspace
        .owner_change(
            key,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
        )
        .unwrap()
    else {
        panic!("synthetic write did not apply")
    };
    receipt.writes.into_iter().map(|w| w.after).collect()
}

fn note(title: &str, markdown: &str, chat: Option<&str>) -> RecordData {
    RecordData::Note(Note {
        chat: chat.map(Into::into),
        ..Note::working(title, markdown)
    })
}

fn source(chat: Option<&str>) -> RecordData {
    RecordData::Source(Source {
        chat: chat.map(Into::into),
        locator: "synthetic://report".into(),
        label: "Report".into(),
        observed_at: "2026-10-13T00:00:00Z".into(),
        external_version: None,
        outcome: "converted".into(),
        gaps: vec![],
        report: None,
    })
}

fn found(handle: &SearchHandle, query: &str, chat: Option<&str>) -> Vec<String> {
    handle
        .search(query, chat, 20)
        .unwrap()
        .matches
        .into_iter()
        .map(|m| m.title)
        .collect()
}

fn start(workspace: &Workspace) -> (SearchHandle, SearchWorker) {
    SearchHandle::spawn(&workspace.store).unwrap()
}

#[test]
fn follows_canonical_writes_incrementally_with_exact_quotes() {
    let (_root, _data, mut ws) = workspace();
    let (handle, worker) = start(&ws);
    let first = ws
        .new_note("Pumps", "# Garden\n\nThe pumps are running nightly.\n")
        .unwrap();
    assert_eq!(found(&handle, "pump runs", None), ["Pumps"]);
    let best = &handle.search("nightly", None, 5).unwrap().matches[0];
    let RecordData::Note(saved) = &first.data else {
        unreachable!()
    };
    assert_eq!(&saved.markdown[best.start..best.end], best.quote);
    assert_eq!((best.heading_path.as_str(), best.version), ("Garden", 1));

    let RecordData::Note(mut edited) = first.data.clone() else {
        unreachable!()
    };
    edited.markdown = "# Garden\n\nThe valves are closed.\n".into();
    write(
        &mut ws,
        "edit",
        vec![(Some(&first), RecordData::Note(edited.clone()), false)],
    );
    assert!(found(&handle, "nightly running", None).is_empty());
    assert_eq!(found(&handle, "valve", None), ["Pumps"]);
    let current = ws.store.record(&first.id).unwrap().unwrap();
    write(
        &mut ws,
        "archive",
        vec![(Some(&current), RecordData::Note(edited), true)],
    );
    assert!(found(&handle, "valves", None).is_empty());
    drop(handle);
    worker.join().unwrap();
}

#[test]
fn chat_only_notes_and_chat_sources_stay_in_their_chat() {
    let (_root, _data, mut ws) = workspace();
    let chat_a = ws.new_thread("A").unwrap().id;
    let chat_b = ws.new_thread("B").unwrap().id;
    let source_record = write(
        &mut ws,
        "source",
        vec![(None, source(Some(&chat_a)), false)],
    )
    .remove(0);
    let mut imported = Note::working("Imported report", "Kestrel survey totals.\n");
    imported.import = Some(ImportCoverage {
        source: source_record.id.clone(),
        full_note: false,
        gaps: vec![],
        report: None,
    });
    write(
        &mut ws,
        "notes",
        vec![
            (
                None,
                note("Private kestrel", "Kestrel nest location.\n", Some(&chat_a)),
                false,
            ),
            (
                None,
                note("Public kestrel", "Kestrel facts.\n", None),
                false,
            ),
            (None, RecordData::Note(imported), false),
        ],
    );
    let (handle, _worker) = start(&ws);
    let sorted = |chat: Option<&str>| {
        let mut titles = found(&handle, "kestrel", chat);
        titles.sort();
        titles
    };
    assert_eq!(sorted(None), ["Public kestrel"]);
    assert_eq!(sorted(Some(&chat_b)), ["Public kestrel"]);
    assert_eq!(
        sorted(Some(&chat_a)),
        ["Imported report", "Private kestrel", "Public kestrel"]
    );
    // Promoting the source changes only the source record; the importing note
    // must follow it into general search.
    write(
        &mut ws,
        "promote",
        vec![(Some(&source_record), source(None), false)],
    );
    assert_eq!(sorted(None), ["Imported report", "Public kestrel"]);
    assert_eq!(sorted(Some(&chat_b)), ["Imported report", "Public kestrel"]);
}

#[test]
fn restarts_reuse_or_recreate_the_derived_index() {
    let (_root, _data, mut ws) = workspace();
    ws.new_note("Heron", "Heron sightings.\n").unwrap();
    let (handle, worker) = start(&ws);
    assert_eq!(found(&handle, "heron", None), ["Heron"]);
    drop(handle);
    worker.join().unwrap();
    let path = index_path(&ws.store).unwrap();
    assert!(path.exists());
    ws.new_note("Egret", "Egret sightings.\n").unwrap();
    let (handle, worker) = start(&ws);
    assert_eq!(found(&handle, "egret", None), ["Egret"]);
    drop(handle);
    worker.join().unwrap();
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    let (handle, _worker) = start(&ws);
    let mut titles = found(&handle, "sightings", None);
    titles.sort();
    assert_eq!(titles, ["Egret", "Heron"]);
    // The workspace still opens with its derived directory present.
    drop(ws);
    Workspace::open(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn concurrent_handles_and_a_second_process_view_agree() {
    let (_root, data, mut ws) = workspace();
    for i in 0..40 {
        ws.new_note(&format!("Note {i}"), &format!("Shared plover entry {i}.\n"))
            .unwrap();
    }
    let (handle, _worker) = start(&ws);
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let handle = handle.clone();
            std::thread::spawn(move || handle.search("plover", None, 50).unwrap().matches.len())
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), 40);
    }
    // A second workspace view (as the CLI would open) shares the index file.
    let other = Workspace::open(&data).unwrap();
    let (other_handle, _other_worker) = start(&other);
    ws.new_note("Late", "Late plover entry.\n").unwrap();
    assert_eq!(
        other_handle
            .search("plover", None, 50)
            .unwrap()
            .matches
            .len(),
        41
    );
    assert_eq!(handle.search("plover", None, 50).unwrap().matches.len(), 41);
}

#[test]
fn failures_are_reported_not_answered_as_empty() {
    let (_root, _data, ws) = workspace();
    let (handle, _worker) = start(&ws);
    assert!(handle.search("anything", None, 0).is_err());
    assert!(handle.search("   ", None, 5).is_err());
    let path = index_path(&ws.store).unwrap();
    std::fs::write(&path, b"someone else's file").unwrap();
    assert!(handle.search("anything", None, 5).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"someone else's file");
}

#[test]
fn dropped_tickets_do_not_block_later_requests() {
    let (_root, _data, ws) = workspace();
    let (handle, worker) = start(&ws);
    drop(handle.request("abandoned", None, 5));
    let ticket = handle.request("kept", None, 5);
    assert!(ticket.wait().unwrap().matches.is_empty());
    drop(handle);
    worker.join().unwrap();
}
