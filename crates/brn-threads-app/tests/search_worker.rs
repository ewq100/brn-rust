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

fn link(from: &str, to: &str, relation: &str) -> RecordData {
    RecordData::Link(Link {
        from: from.into(),
        to: to.into(),
        relation: relation.into(),
    })
}

#[test]
fn pages_have_exact_counts_evidence_quota_and_context_scoped_link_counts() {
    let (_root, _data, mut ws) = workspace();
    let chat_a = ws.new_thread("A").unwrap().id;
    let chat_b = ws.new_thread("B").unwrap().id;
    let source_record = write(&mut ws, "source", vec![(None, source(None), false)]).remove(0);
    let mut puts: Vec<(Option<&Record>, RecordData, bool)> = (0..20)
        .map(|i| {
            (
                None,
                note(
                    &format!("Summary {i}"),
                    &format!("Plover plover plover summary {i}.\n"),
                    None,
                ),
                false,
            )
        })
        .collect();
    for i in 0..2 {
        let mut imported = Note::working(format!("Field report {i}"), "One plover seen.\n");
        imported.import = Some(ImportCoverage {
            source: source_record.id.clone(),
            full_note: false,
            gaps: vec![],
            report: None,
        });
        puts.push((None, RecordData::Note(imported), false));
    }
    puts.push((None, note("Survey sheet", "A plover line.\n", None), false));
    for i in 0..5 {
        puts.push((
            None,
            note(
                &format!("Private {i}"),
                "Plover nest secret.\n",
                Some(&chat_a),
            ),
            false,
        ));
    }
    let notes = write(&mut ws, "notes", puts);
    let id = |title: &str| {
        notes
            .iter()
            .find(|r| matches!(&r.data, RecordData::Note(n) if n.title == title))
            .unwrap()
            .id
            .clone()
    };
    let (summary, survey) = (id("Summary 0"), id("Survey sheet"));
    write(
        &mut ws,
        "links",
        vec![
            (None, link(&summary, &survey, relations::SOURCE), false),
            (
                None,
                link(&id("Private 0"), &survey, relations::RELATED),
                false,
            ),
        ],
    );
    let (handle, _worker) = start(&ws);

    let first = handle
        .request_page("plover", None, 12, None)
        .wait()
        .unwrap();
    assert_eq!(
        (first.total, first.reachable, first.matches.len()),
        (23, 23, 12)
    );
    assert_eq!(first.next.as_deref(), Some("12"));
    let evidence: Vec<&str> = first
        .matches
        .iter()
        .filter(|m| m.evidence)
        .map(|m| m.title.as_str())
        .collect();
    assert_eq!(evidence.len(), 3, "all evidence reaches the first page");
    assert!(first.matches.iter().filter(|m| m.imported).count() == 2);
    let sheet = first.matches.iter().find(|m| m.note == survey).unwrap();
    // The chat-only link is invisible from general search.
    assert_eq!(sheet.inbound_links, 1);
    assert!(sheet.snippet.text.contains("plover"));
    let second = handle
        .request_page("plover", None, 12, first.next.as_deref())
        .wait()
        .unwrap();
    assert_eq!(
        (second.offset, second.matches.len(), second.next),
        (12, 11, None)
    );
    let mut all: Vec<_> = first
        .matches
        .iter()
        .chain(&second.matches)
        .map(|m| m.note.clone())
        .collect();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 23);
    assert!(
        first
            .matches
            .iter()
            .chain(&second.matches)
            .all(|m| !m.title.starts_with("Private") && !m.snippet.text.contains("secret"))
    );

    let in_a = handle
        .request_page("plover", Some(&chat_a), 50, None)
        .wait()
        .unwrap();
    assert_eq!((in_a.total, in_a.matches.len()), (28, 28));
    let sheet = in_a.matches.iter().find(|m| m.note == survey).unwrap();
    assert_eq!(sheet.inbound_links, 2);
    let in_b = handle
        .request_page("secret", Some(&chat_b), 50, None)
        .wait()
        .unwrap();
    assert_eq!((in_b.total, in_b.matches.len()), (0, 0));
    let general = handle
        .request_page("secret", None, 50, None)
        .wait()
        .unwrap();
    assert_eq!(general.total, 0);

    assert!(
        handle
            .request_page("plover", None, 12, Some("5"))
            .wait()
            .is_err()
    );
    assert!(
        handle
            .request_page("plover", None, 12, Some("x"))
            .wait()
            .is_err()
    );
}

#[test]
fn evidence_quota_keeps_rank_order_and_every_result_once() {
    use brn_threads_app::search::arrange;
    let evidence: Vec<bool> = (0..30).map(|i| i >= 20).collect();
    let order = arrange(&evidence, 12);
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(sorted, (0..30).collect::<Vec<_>>());
    let first = &order[..12];
    assert_eq!(first.iter().filter(|&&i| evidence[i]).count(), 4);
    assert!(first.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(&first[..8], &[0, 1, 2, 3, 4, 5, 6, 7]);
    // Small pages are not rebalanced.
    assert_eq!(arrange(&evidence[..3], 3), [0, 1, 2]);
    assert_eq!(arrange(&[false; 5], 12), [0, 1, 2, 3, 4]);
}

#[test]
fn a_removed_derived_directory_is_recreated_while_running() {
    let (_root, data, mut ws) = workspace();
    ws.new_note("Ibis", "Ibis count.\n").unwrap();
    let (handle, _worker) = start(&ws);
    assert_eq!(found(&handle, "ibis", None), ["Ibis"]);
    std::fs::remove_dir_all(data.join("derived")).unwrap();
    ws.new_note("Stork", "Stork count.\n").unwrap();
    let mut titles = found(&handle, "count", None);
    titles.sort();
    assert_eq!(titles, ["Ibis", "Stork"]);
    assert!(index_path(&ws.store).unwrap().exists());
}

#[test]
fn every_offered_cursor_is_accepted_up_to_the_reachable_cap() {
    let (_root, _data, mut ws) = workspace();
    let puts = (0..130)
        .map(|i| {
            let text = match i % 3 {
                0 => format!("Shipment {i} delayed.\n"),
                1 => format!("Container shipments {i}.\n"),
                _ => format!("Delay of container {i}.\n"),
            };
            (None, note(&format!("Log {i:03}"), &text, None), false)
        })
        .collect();
    write(&mut ws, "many", puts);
    let (handle, _worker) = start(&ws);
    let mut cursor = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let page = handle
            .request_page("delay shipment container", None, 12, cursor.as_deref())
            .wait()
            .unwrap();
        assert_eq!(page.total, 130);
        assert!(page.reachable <= brn_threads_app::search::REACHABLE);
        for m in &page.matches {
            assert!(seen.insert(m.note.clone()), "duplicate across pages");
        }
        match page.next {
            Some(next) => cursor = Some(next),
            None => {
                assert_eq!(seen.len(), page.reachable);
                break;
            }
        }
    }
}

#[test]
fn connections_are_bounded_ordered_and_visibility_checked() {
    use brn_threads_app::search::{NEIGHBOURS, NodeKind};
    let (_root, _data, mut ws) = workspace();
    let chat_a = ws.new_thread("A").unwrap().id;
    let src = write(&mut ws, "source", vec![(None, source(None), false)]).remove(0);
    let mut puts: Vec<(Option<&Record>, RecordData, bool)> = vec![
        (None, note("Hub", "Hub page.\n", None), false),
        (None, note("Report", "Evidence.\n", None), false),
        (None, note("Old claim", "Claim.\n", None), false),
        (None, note("Far", "Two hops away.\n", None), false),
        (None, note("Secret", "Chat only.\n", Some(&chat_a)), false),
    ];
    for i in 0..10 {
        puts.push((
            None,
            note(&format!("Related {i}"), "Related.\n", None),
            false,
        ));
    }
    let notes = write(&mut ws, "notes", puts);
    let id = |title: &str| {
        notes
            .iter()
            .find(|r| matches!(&r.data, RecordData::Note(n) if n.title == title))
            .unwrap()
            .id
            .clone()
    };
    let hub = id("Hub");
    let mut links = vec![
        (None, link(&hub, &id("Report"), relations::SOURCE), false),
        (None, link(&hub, &id("Report"), relations::SOURCE), false),
        (
            None,
            link(&id("Old claim"), &hub, relations::CONTRADICTS),
            false,
        ),
        (None, link(&id("Secret"), &hub, relations::RELATED), false),
        (None, link(&hub, &src.id, relations::SOURCE), false),
        (None, link(&id("Report"), &id("Far"), "explains"), false),
        (None, link(&hub, &chat_a, "discussed-in"), false),
    ];
    for i in 0..10 {
        links.push((
            None,
            link(&hub, &id(&format!("Related {i}")), relations::RELATED),
            false,
        ));
    }
    let written = write(&mut ws, "links", links);
    // A changed duplicate reports its current version.
    write(
        &mut ws,
        "touch",
        vec![(
            Some(&written[1]),
            link(&hub, &id("Report"), relations::SOURCE),
            false,
        )],
    );
    let (handle, _worker) = start(&ws);

    let general = handle.connections(&hub, None, 1, false).unwrap();
    assert_eq!(general.centre.title, "Hub");
    let hop = &general.hops[0];
    assert_eq!((hop.available, hop.shown.len()), (12, NEIGHBOURS));
    let titles: Vec<&str> = hop.shown.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(&titles[..3], ["Report", "Old claim", "Related 0"]);
    assert!(hop.shown[0].relations[0].outgoing);
    // Duplicate Links with one relation stay one relation listing both.
    assert_eq!(hop.shown[0].relations.len(), 1);
    let refs: Vec<(String, u64)> = hop.shown[0].relations[0]
        .links
        .iter()
        .map(|l| (l.id.clone(), l.version))
        .collect();
    assert_eq!(
        refs,
        vec![(written[0].id.clone(), 1), (written[1].id.clone(), 2)]
    );
    assert!(!hop.shown[1].relations[0].outgoing);
    assert!(titles.iter().all(|t| *t != "Secret" && *t != "A"));

    let with_sources = handle.connections(&hub, None, 1, true).unwrap();
    assert_eq!(with_sources.hops[0].available, 13);
    assert_eq!(with_sources.hops[0].shown[1].kind, NodeKind::Source);
    let in_chat = handle.connections(&hub, Some(&chat_a), 1, false).unwrap();
    assert_eq!(in_chat.hops[0].available, 13);

    let two = handle.connections(&hub, None, 2, false).unwrap();
    let far = &two.hops[1].shown[0];
    assert_eq!(
        (far.title.as_str(), far.via.as_deref()),
        ("Far", Some(id("Report").as_str()))
    );
    assert_eq!(far.relations[0].relation, "explains");
    assert_eq!(two.hops[1].available, 1);

    assert!(handle.connections(&id("Secret"), None, 1, false).is_err());
    assert!(
        handle
            .connections(&id("Secret"), Some(&chat_a), 1, false)
            .is_ok()
    );
    assert!(handle.connections(&hub, None, 3, false).is_err());
}
