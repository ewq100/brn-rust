use brn_retrieval::search_index::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

fn derived() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("derived");
    std::fs::create_dir(&path).unwrap();
    (root, path)
}

fn doc(id: &str, version: u64, title: &str, markdown: &str, visibility: Visibility) -> DocInput {
    DocInput {
        id: id.into(),
        version,
        title: title.into(),
        markdown: markdown.into(),
        visibility,
        protected: false,
        imported_from: None,
        language: "en".into(),
    }
}

fn step(index: &mut SearchIndex, changes: Changes) {
    let base = index.sync_state().unwrap();
    index.apply(&Changes { base, ..changes }).unwrap();
}

fn upsert(index: &mut SearchIndex, docs: Vec<DocInput>, watermark: i64) {
    step(
        index,
        Changes {
            upsert_docs: docs,
            watermark,
            ..Changes::default()
        },
    );
}

fn docs_for(index: &SearchIndex, query: &str, chat: Option<&str>) -> Vec<String> {
    let mut docs: Vec<String> = index
        .keyword(query, chat, 50)
        .unwrap()
        .into_iter()
        .map(|h| h.doc)
        .collect();
    docs.dedup();
    docs
}

fn fts_consistent(path: &Path) {
    let conn = Connection::open(path.join(FILE_NAME)).unwrap();
    for table in ["fts_exact", "fts_stem"] {
        conn.execute(
            &format!("INSERT INTO {table}({table}, rank) VALUES ('integrity-check', 1)"),
            [],
        )
        .unwrap();
    }
}

#[test]
fn creates_reopens_and_finds_exact_and_inflected_terms() {
    let (_root, dir) = derived();
    let (mut index, opened) = SearchIndex::open(&dir).unwrap();
    assert_eq!(opened, Opened::Created);
    assert_eq!(index.sync_state().unwrap(), None);
    let markdown = "# Garden\n\nThe gardener runs the irrigation pumps nightly.\n\n## Costs\n\nInvoice QX-7731 covers spare valves.\n";
    upsert(
        &mut index,
        vec![doc("n1", 1, "Allotment", markdown, Visibility::General)],
        7,
    );
    drop(index);
    let (index, opened) = SearchIndex::open(&dir).unwrap();
    assert_eq!(opened, Opened::Existing);
    assert_eq!(
        index.sync_state().unwrap(),
        Some(SyncState {
            workspace_identity: None,
            watermark: 7
        })
    );
    assert_eq!(docs_for(&index, "QX-7731", None), ["n1"]);
    // Only the stemmed lane matches inflected forms.
    assert_eq!(docs_for(&index, "running pump", None), ["n1"]);
    let hit = &index.keyword("valves", None, 10).unwrap()[0];
    assert_eq!(hit.heading_path, "Garden › Costs");
    assert_eq!(&markdown[hit.start_byte..hit.end_byte], hit.text);
    assert!(hit.text.contains("valves"));
    // The title is searchable through every passage's indexed text.
    assert_eq!(docs_for(&index, "allotment", None), ["n1"]);
    fts_consistent(&dir);
}

#[test]
fn title_and_content_changes_leave_no_stale_terms_in_either_lane() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    upsert(
        &mut index,
        vec![doc(
            "n1",
            1,
            "Zebra notes",
            "Walking the heron trail.\n",
            Visibility::General,
        )],
        1,
    );
    upsert(
        &mut index,
        vec![doc(
            "n1",
            2,
            "Yak notes",
            "Walking the egret trail.\n",
            Visibility::General,
        )],
        2,
    );
    for gone in ["zebra", "heron", "herons"] {
        assert!(docs_for(&index, gone, None).is_empty(), "{gone}");
    }
    assert_eq!(docs_for(&index, "yaks egret", None), ["n1"]);
    fts_consistent(&dir);
    step(
        &mut index,
        Changes {
            remove_docs: vec!["n1".into()],
            watermark: 3,
            ..Changes::default()
        },
    );
    assert!(docs_for(&index, "egret", None).is_empty());
    assert_eq!(index.doc_count().unwrap(), 0);
    fts_consistent(&dir);
}

#[test]
fn visibility_filters_contexts_and_metadata_updates_do_not_rechunk() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    let mut imported = doc(
        "private",
        1,
        "Private",
        "Shared marker kestrel.\n",
        Visibility::Chat("A".into()),
    );
    imported.imported_from = Some("source-1".into());
    upsert(
        &mut index,
        vec![
            doc(
                "general",
                1,
                "General",
                "Shared marker kestrel.\n",
                Visibility::General,
            ),
            imported.clone(),
            doc(
                "hidden",
                1,
                "Hidden",
                "Shared marker kestrel.\n",
                Visibility::Hidden,
            ),
        ],
        1,
    );
    assert_eq!(docs_for(&index, "kestrel", None), ["general"]);
    let mut in_a = docs_for(&index, "kestrel", Some("A"));
    in_a.sort();
    assert_eq!(in_a, ["general", "private"]);
    assert_eq!(docs_for(&index, "kestrel", Some("B")), ["general"]);
    assert_eq!(
        index.docs_importing(&["source-1".into()]).unwrap(),
        ["private"]
    );
    let before = index.keyword("kestrel", Some("A"), 10).unwrap();
    imported.visibility = Visibility::General;
    imported.version = 2;
    upsert(&mut index, vec![imported], 2);
    let after = index.keyword("kestrel", None, 10).unwrap();
    let passage = |hits: &[DocHit]| hits.iter().find(|h| h.doc == "private").unwrap().passage;
    assert_eq!(passage(&before), passage(&after));
    assert_eq!(
        after.iter().find(|h| h.doc == "private").unwrap().version,
        2
    );
}

#[test]
fn links_mirror_and_full_resync_replaces_everything() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    let link = LinkInput {
        id: "l1".into(),
        from: "a".into(),
        to: "b".into(),
        relation: "source".into(),
    };
    step(
        &mut index,
        Changes {
            upsert_docs: vec![doc("a", 1, "A", "alpha\n", Visibility::General)],
            upsert_links: vec![link.clone()],
            watermark: 4,
            ..Changes::default()
        },
    );
    assert_eq!(index.links_of("b").unwrap(), [link]);
    step(
        &mut index,
        Changes {
            remove_links: vec!["l1".into()],
            watermark: 5,
            ..Changes::default()
        },
    );
    assert!(index.links_of("a").unwrap().is_empty());
    step(
        &mut index,
        Changes {
            full: true,
            workspace_identity: Some("workspace-1".into()),
            upsert_docs: vec![doc("b", 1, "B", "beta\n", Visibility::General)],
            watermark: 9,
            ..Changes::default()
        },
    );
    assert_eq!(index.doc_count().unwrap(), 1);
    assert!(docs_for(&index, "alpha", None).is_empty());
    assert_eq!(
        index.sync_state().unwrap(),
        Some(SyncState {
            workspace_identity: Some("workspace-1".into()),
            watermark: 9
        })
    );
}

#[test]
fn changes_computed_from_an_older_state_are_refused_as_stale() {
    let (_root, dir) = derived();
    let (mut first, _) = SearchIndex::open(&dir).unwrap();
    let (mut second, _) = SearchIndex::open(&dir).unwrap();
    let seen = first.sync_state().unwrap();
    upsert(
        &mut second,
        vec![doc("new", 2, "New", "newer text\n", Visibility::General)],
        20,
    );
    let stale = Changes {
        base: seen,
        upsert_docs: vec![doc("new", 1, "New", "older text\n", Visibility::Hidden)],
        watermark: 10,
        ..Changes::default()
    };
    assert!(matches!(
        first.apply(&stale),
        Err(brn_retrieval::Error::Stale)
    ));
    assert!(matches!(
        first.apply(&Changes {
            full: true,
            ..stale
        }),
        Err(brn_retrieval::Error::Stale)
    ));
    assert_eq!(docs_for(&first, "newer", None), ["new"]);
    assert_eq!(first.sync_state().unwrap().unwrap().watermark, 20);
}

#[test]
fn many_matching_passages_in_one_note_do_not_crowd_out_others() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    let long: String = (0..80)
        .map(|i| {
            format!(
                "## Part {i}\n\n{}osprey record {i}.\n\n",
                "filler words here ".repeat(120)
            )
        })
        .collect();
    let mut docs = vec![doc("long", 1, "Long", &long, Visibility::General)];
    docs.extend((0..10).map(|i| {
        doc(
            &format!("short{i}"),
            1,
            "Short",
            "One osprey.\n",
            Visibility::General,
        )
    }));
    upsert(&mut index, docs, 1);
    let hits = index.keyword("osprey", None, 50).unwrap();
    assert_eq!(hits.len(), 11);
    let long_hit = hits.iter().find(|h| h.doc == "long").unwrap();
    assert!(long_hit.passages >= 40, "{}", long_hit.passages);
    assert_eq!(&long[long_hit.start_byte..long_hit.end_byte], long_hit.text);
}

#[test]
fn unknown_languages_are_refused() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    let mut estonian = doc("n", 1, "Märkmed", "tere\n", Visibility::General);
    estonian.language = "et".into();
    assert!(
        index
            .apply(&Changes {
                base: index.sync_state().unwrap(),
                upsert_docs: vec![estonian],
                ..Changes::default()
            })
            .is_err()
    );
    assert!(analyzer("en").is_some() && analyzer("et").is_none());
}

#[test]
fn outdated_damaged_or_interrupted_indexes_are_rebuilt_in_place() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    upsert(
        &mut index,
        vec![doc("n", 1, "N", "kept\n", Visibility::General)],
        3,
    );
    drop(index);
    let conn = Connection::open(dir.join(FILE_NAME)).unwrap();
    conn.execute(
        "UPDATE meta SET value = 'markdown-0' WHERE key = 'chunker_version'",
        [],
    )
    .unwrap();
    drop(conn);
    let (index, opened) = SearchIndex::open(&dir).unwrap();
    assert_eq!(opened, Opened::Rebuilt);
    assert_eq!(
        (index.doc_count().unwrap(), index.sync_state().unwrap()),
        (0, None)
    );
    drop(index);

    let conn = Connection::open(dir.join(FILE_NAME)).unwrap();
    conn.pragma_update(None, "user_version", 0).unwrap();
    drop(conn);
    assert_eq!(SearchIndex::open(&dir).unwrap().1, Opened::Rebuilt);

    // Keep our header but damage every page after it.
    let file = dir.join(FILE_NAME);
    let mut bytes = std::fs::read(&file).unwrap();
    for byte in bytes.iter_mut().skip(100) {
        *byte = 0xA5;
    }
    std::fs::write(&file, bytes).unwrap();
    let _ = std::fs::remove_file(dir.join(format!("{FILE_NAME}-wal")));
    let _ = std::fs::remove_file(dir.join(format!("{FILE_NAME}-shm")));
    assert_eq!(SearchIndex::open(&dir).unwrap().1, Opened::Rebuilt);

    std::fs::write(&file, b"").unwrap();
    assert_eq!(SearchIndex::open(&dir).unwrap().1, Opened::Rebuilt);
    fts_consistent(&dir);
}

#[test]
fn foreign_files_and_symlinks_are_refused_without_change() {
    let (_root, dir) = derived();
    let file = dir.join(FILE_NAME);
    std::fs::write(&file, b"not a database at all, but someone's text file").unwrap();
    assert!(SearchIndex::open(&dir).is_err());
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"not a database at all, but someone's text file"
    );
    std::fs::remove_file(&file).unwrap();

    let conn = Connection::open(&file).unwrap();
    conn.execute_batch("CREATE TABLE theirs(x); INSERT INTO theirs VALUES (1);")
        .unwrap();
    drop(conn);
    assert!(SearchIndex::open(&dir).is_err());
    let conn = Connection::open(&file).unwrap();
    assert_eq!(
        conn.query_row("SELECT x FROM theirs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(conn);
    std::fs::remove_file(&file).unwrap();

    let elsewhere = dir.parent().unwrap().join("elsewhere.sqlite");
    std::os::unix::fs::symlink(&elsewhere, &file).unwrap();
    assert!(SearchIndex::open(&dir).is_err());
    assert!(!elsewhere.exists());
}

#[test]
fn concurrent_writers_serialize_without_errors() {
    let (_root, dir) = derived();
    SearchIndex::open(&dir).unwrap();
    let workers: Vec<_> = (0..4)
        .map(|w| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                let (mut index, _) = SearchIndex::open(&dir).unwrap();
                for i in 0..25 {
                    let changes = Changes {
                        upsert_docs: vec![doc(
                            &format!("w{w}-{i}"),
                            1,
                            "Concurrent",
                            "parallel writers share one index\n",
                            Visibility::General,
                        )],
                        watermark: i,
                        ..Changes::default()
                    };
                    // Writers that lose a race recompute from the new state.
                    loop {
                        let base = index.sync_state().unwrap();
                        match index.apply(&Changes {
                            base,
                            ..changes.clone()
                        }) {
                            Ok(()) => break,
                            Err(brn_retrieval::Error::Stale) => continue,
                            Err(error) => panic!("{error}"),
                        }
                    }
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let (index, _) = SearchIndex::open(&dir).unwrap();
    assert_eq!(index.doc_count().unwrap(), 100);
    fts_consistent(&dir);
}

#[test]
fn counts_link_facts_and_all_word_matches_respect_visibility() {
    let (_root, dir) = derived();
    let (mut index, _) = SearchIndex::open(&dir).unwrap();
    let mut imported = doc(
        "ev",
        1,
        "Report",
        "Delayed shipments of containers.\n",
        Visibility::General,
    );
    imported.imported_from = Some("src".into());
    let mut decoys: Vec<DocInput> = (0..30)
        .map(|i| {
            doc(
                &format!("d{i:02}"),
                1,
                &format!("Decoy {i}"),
                "Shipment shipment shipment delay delay.\n",
                Visibility::General,
            )
        })
        .collect();
    decoys.push(imported);
    decoys.push(doc(
        "private",
        1,
        "Private",
        "Container delay.\n",
        Visibility::Chat("A".into()),
    ));
    decoys.push(doc(
        "gone",
        1,
        "Hidden",
        "Container delay.\n",
        Visibility::Hidden,
    ));
    step(
        &mut index,
        Changes {
            upsert_docs: decoys,
            upsert_links: vec![
                LinkInput {
                    id: "l1".into(),
                    from: "d00".into(),
                    to: "ev".into(),
                    relation: "source".into(),
                },
                LinkInput {
                    id: "l2".into(),
                    from: "private".into(),
                    to: "ev".into(),
                    relation: "source".into(),
                },
                LinkInput {
                    id: "l3".into(),
                    from: "d01".into(),
                    to: "ev".into(),
                    relation: "related".into(),
                },
            ],
            watermark: 1,
            ..Changes::default()
        },
    );
    // The all-words lane ranks the only note matching every word first.
    assert_eq!(docs_for(&index, "delay shipment container", None)[0], "ev");
    assert_eq!(index.count("delay shipment container", None).unwrap(), 31);
    assert_eq!(
        index.count("delay shipment container", Some("A")).unwrap(),
        32
    );
    assert_eq!(index.count("container", Some("B")).unwrap(), 1);
    let ids = vec![
        "ev".to_string(),
        "private".to_string(),
        "missing".to_string(),
    ];
    let general = index.facts(&ids, None, "source").unwrap();
    assert_eq!(
        general["ev"],
        DocFacts {
            protected: false,
            imported: true,
            inbound_links: 2,
            evidence_links: 1
        }
    );
    assert!(!general.contains_key("missing"));
    let in_chat = index.facts(&ids, Some("A"), "source").unwrap();
    assert_eq!(
        (in_chat["ev"].inbound_links, in_chat["ev"].evidence_links),
        (3, 2)
    );
}
