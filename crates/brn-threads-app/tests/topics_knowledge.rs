use brn_threads_app::*;

fn workspace() -> (tempfile::TempDir, std::path::PathBuf, Workspace) {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap().join("data");
    let workspace = Workspace::open(&data).unwrap();
    (root, data, workspace)
}

fn write(
    ws: &mut Workspace,
    key: &str,
    puts: Vec<(Option<&Record>, RecordData, bool)>,
) -> Vec<Record> {
    write_with(ws, key, puts, vec![])
}

fn write_with(
    ws: &mut Workspace,
    key: &str,
    puts: Vec<(Option<&Record>, RecordData, bool)>,
    inputs: Vec<RevisionInput>,
) -> Vec<Record> {
    let op = ws.store.allocate_operation(key).unwrap();
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
    let request = ChangeRequest {
        reason: key.into(),
        writes,
        inputs,
    };
    let owner = HostAuthority::owner("fixture");
    ws.store.prepare_owner(&op, &request, &owner).unwrap();
    let ApplyOutcome::Applied(receipt) = ws.store.apply(&op, &owner).unwrap() else {
        panic!("fixture write did not apply")
    };
    receipt.writes.into_iter().map(|w| w.after).collect()
}

fn note(title: &str, markdown: &str) -> RecordData {
    RecordData::Note(Note::working(title, markdown))
}

fn link(from: &str, to: &str, relation: &str) -> RecordData {
    RecordData::Link(Link {
        from: from.into(),
        to: to.into(),
        relation: relation.into(),
    })
}

fn input(record: &Record) -> RevisionInput {
    serde_json::from_value(serde_json::json!({"record":record.id,"version":record.version}))
        .unwrap()
}

#[test]
fn topics_note_lists_linked_topic_pages_in_mention_order() {
    let (_root, data, mut ws) = workspace();
    let settings = ws.settings().unwrap();
    assert!(
        !serde_json::to_string(&settings)
            .unwrap()
            .contains("topics_note")
    );
    assert_eq!(ws.topics().unwrap(), None);

    let (topics, outcome) = ws.create_topics_note().unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied(_)));
    assert_eq!(ws.topics().unwrap().unwrap().entries, vec![]);
    let pages = write(
        &mut ws,
        "pages",
        vec![
            (
                None,
                note("Logistics", "# Logistics\n\nLoading and access.\n"),
                false,
            ),
            (None, note("Harbor project", "Delivery planning.\n"), false),
            (None, note("Subject", "A subject page.\n"), false),
        ],
    );
    let current = ws.store.record(&topics.id).unwrap().unwrap();
    let edited = write(
        &mut ws,
        "topics-text",
        vec![(
            Some(&current),
            note("Topics", "Start with Harbor project, then Logistics.\n"),
            false,
        )],
    )
    .remove(0);
    write(
        &mut ws,
        "links",
        vec![
            (
                None,
                link(&topics.id, &pages[0].id, relations::RELATED),
                false,
            ),
            (
                None,
                link(&topics.id, &pages[1].id, relations::RELATED),
                false,
            ),
            (
                None,
                link(&pages[1].id, &pages[2].id, relations::RELATED),
                false,
            ),
        ],
    );
    let listed = ws.topics().unwrap().unwrap();
    assert_eq!(listed.note, topics.id);
    let titles: Vec<&str> = listed.entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Harbor project", "Logistics"]);
    assert_eq!(listed.entries[0].linked_notes, 1);
    assert_eq!(listed.entries[1].description, "Loading and access.");

    // Undo of the Topics note's content keeps the pointer.
    let history = ws.history().unwrap();
    let edit = history
        .iter()
        .find(|r| {
            r.writes
                .iter()
                .any(|w| w.after.id == topics.id && w.after.version == edited.version)
        })
        .unwrap()
        .operation
        .clone();
    ws.undo(edit.as_str()).unwrap();
    let RecordData::Note(restored) = ws.store.record(&topics.id).unwrap().unwrap().data else {
        panic!()
    };
    assert!(
        restored
            .markdown
            .starts_with("Topic pages for this workspace")
    );
    assert_eq!(ws.topics().unwrap().unwrap().note, topics.id);

    // Replay of the same choice is identical; reopen keeps the pointer.
    let op = ws.store.allocate_operation("choose-again").unwrap();
    let owner = HostAuthority::owner("owner");
    let first = ws
        .store
        .choose_topics_note(&op, &topics.id, &owner)
        .unwrap();
    assert_eq!(
        ws.store
            .choose_topics_note(&op, &topics.id, &owner)
            .unwrap(),
        first
    );
    assert!(
        ws.store
            .choose_topics_note(&op, &pages[0].id, &owner)
            .is_err()
    );
    drop(ws);
    let mut ws = Workspace::open(&data).unwrap();
    assert_eq!(ws.topics().unwrap().unwrap().note, topics.id);

    // Archiving the Topics note leaves an honest empty state.
    let current = ws.store.record(&topics.id).unwrap().unwrap();
    write(
        &mut ws,
        "archive",
        vec![(Some(&current), current.data.clone(), true)],
    );
    assert_eq!(ws.topics().unwrap(), None);
}

#[test]
fn topics_note_must_be_a_current_general_note() {
    let (_root, _data, mut ws) = workspace();
    let chat = ws.new_thread("Private").unwrap().id;
    let private = write(
        &mut ws,
        "private",
        vec![(
            None,
            RecordData::Note(Note {
                chat: Some(chat.clone()),
                ..Note::working("Hidden index", "x")
            }),
            false,
        )],
    )
    .remove(0);
    assert!(ws.choose_topics_note(&private.id).is_err());
    assert!(ws.choose_topics_note("missing").is_err());
    assert!(ws.choose_topics_note(&chat).is_err());
    assert_eq!(ws.settings().unwrap().topics_note, None);
}

#[test]
fn knowledge_check_reports_deterministic_findings_without_chat_material() {
    let (_root, _data, mut ws) = workspace();
    let chat = ws.new_thread("Private").unwrap().id;
    let source = write(
        &mut ws,
        "source",
        vec![(
            None,
            RecordData::Source(Source {
                chat: None,
                locator: "synthetic://scan".into(),
                label: "Scan".into(),
                observed_at: "2026-10-13T00:00:00Z".into(),
                external_version: None,
                outcome: "converted".into(),
                gaps: vec!["page 3 unreadable".into()],
                report: None,
            }),
            false,
        )],
    )
    .remove(0);
    let notes = write(
        &mut ws,
        "notes",
        vec![
            (None, note("Harbor plan", "Delivery on Monday.\n"), false),
            (None, note("Harbor plans", "Delivery on Tuesday.\n"), false),
            (None, note("Copy one", "Same body text.\n"), false),
            (None, note("Copy two", "Same  body text.\n"), false),
            (None, note("Lonely", "Nobody links here.\n"), false),
            (None, note("New policy", "Replaces the old one.\n"), false),
            (None, note("Old policy", "Still current.\n"), false),
            (None, note("Retired", "Archived soon.\n"), false),
            (
                None,
                RecordData::Note(Note {
                    chat: Some(chat.clone()),
                    ..Note::working("Harbor plan", "zzsecret private plan.\n")
                }),
                false,
            ),
        ],
    );
    let id = |title: &str, private: bool| {
        notes
            .iter()
            .find(|r| matches!(&r.data, RecordData::Note(n) if n.title == title && n.chat.is_some() == private))
            .unwrap()
            .clone()
    };
    let (plan, plans, retired, secret) = (
        id("Harbor plan", false),
        id("Harbor plans", false),
        id("Retired", false),
        id("Harbor plan", true),
    );
    write(
        &mut ws,
        "links",
        vec![
            (
                None,
                link(&plan.id, &plans.id, relations::CONTRADICTS),
                false,
            ),
            (
                None,
                link(
                    &id("Copy one", false).id,
                    &id("Copy two", false).id,
                    "relates-to",
                ),
                false,
            ),
            (
                None,
                link(&plan.id, &id("Copy one", false).id, relations::RELATED),
                false,
            ),
            (
                None,
                link(
                    &id("New policy", false).id,
                    &id("Old policy", false).id,
                    relations::SUPERSEDES,
                ),
                false,
            ),
            (None, link(&plan.id, &retired.id, relations::RELATED), false),
            (None, link(&secret.id, &plan.id, relations::SOURCE), false),
        ],
    );
    let mut imported = Note::working("Scan notes", "Partial scan.\n");
    imported.import = Some(ImportCoverage {
        source: source.id.clone(),
        full_note: false,
        gaps: vec!["page 3 unreadable".into()],
        report: None,
    });
    let derived = write_with(
        &mut ws,
        "derived",
        vec![(None, RecordData::Note(imported), false)],
        vec![input(&plan)],
    )
    .remove(0);
    write(
        &mut ws,
        "link-derived",
        vec![(None, link(&derived.id, &plan.id, relations::SOURCE), false)],
    );
    // The input changes after the derived note was written.
    let plan_now = ws.store.record(&plan.id).unwrap().unwrap();
    write(
        &mut ws,
        "edit-plan",
        vec![(
            Some(&plan_now),
            note("Harbor plan", "Delivery on Wednesday.\n"),
            false,
        )],
    );
    write(
        &mut ws,
        "retire",
        vec![(Some(&retired), retired.data.clone(), true)],
    );

    let check = ws.knowledge_check().unwrap();
    let count = |kind| check.counts.get(&kind).copied().unwrap_or(0);
    assert_eq!(count(FindingKind::BrokenLink), 1);
    assert_eq!(count(FindingKind::StaleInput), 1);
    assert_eq!(count(FindingKind::CoverageGap), 2);
    assert_eq!(count(FindingKind::Orphan), 1);
    assert_eq!(count(FindingKind::DuplicateTitle), 1);
    assert_eq!(count(FindingKind::DuplicateContent), 1);
    assert_eq!(count(FindingKind::Contradiction), 1);
    assert_eq!(count(FindingKind::Supersession), 1);
    assert_eq!(count(FindingKind::RelationVariant), 1);
    let serialized = serde_json::to_string(&check).unwrap();
    assert!(!serialized.contains(&secret.id) && !serialized.contains("zzsecret"));
    assert!(!serialized.contains(&chat));
    let duplicates = check
        .findings
        .iter()
        .find(|f| f.kind == FindingKind::DuplicateTitle)
        .unwrap();
    assert_eq!(duplicates.records.len(), 2);

    let review = ws.seed_knowledge_review(&check).unwrap();
    let messages = ws.messages(&review).unwrap();
    let RecordData::Message(message) = &messages[0].data else {
        panic!()
    };
    assert!(message.text.contains("maintain-notes") && message.text.contains("Contradiction"));
    assert!(!message.text.contains(&secret.id) && !message.text.contains("zzsecret"));
    assert!(ws.messages(&chat).unwrap().is_empty());
}

#[test]
fn unlisted_topic_pages_are_reported_only_with_a_topics_note() {
    let (_root, _data, mut ws) = workspace();
    let puts = (0..6)
        .map(|i| (None, note(&format!("Leaf {i}"), "Leaf.\n"), false))
        .collect();
    let leaves = write(&mut ws, "leaves", puts);
    let hub = write(
        &mut ws,
        "hub",
        vec![(None, note("Area", "Area page.\n"), false)],
    )
    .remove(0);
    let links = leaves
        .iter()
        .map(|l| (None, link(&hub.id, &l.id, relations::RELATED), false))
        .collect();
    write(&mut ws, "hub-links", links);
    assert!(
        !ws.knowledge_check()
            .unwrap()
            .counts
            .contains_key(&FindingKind::UnlistedTopic)
    );
    let (topics, _) = ws.create_topics_note().unwrap();
    let check = ws.knowledge_check().unwrap();
    assert_eq!(check.counts.get(&FindingKind::UnlistedTopic), Some(&1));
    write(
        &mut ws,
        "list",
        vec![(None, link(&topics.id, &hub.id, relations::RELATED), false)],
    );
    assert!(
        !ws.knowledge_check()
            .unwrap()
            .counts
            .contains_key(&FindingKind::UnlistedTopic)
    );
}

#[test]
fn host_owned_import_relations_are_not_relation_variants() {
    let (_root, _data, mut ws) = workspace();
    let puts = (0..3)
        .map(|i| (None, note(&format!("Imported {i}"), "Imported.\n"), false))
        .collect();
    let n = write(&mut ws, "imports", puts);
    let links = vec![
        (None, link(&n[0].id, &n[2].id, "full-import:aaaa"), false),
        (None, link(&n[1].id, &n[2].id, "full-import:bbbb"), false),
    ];
    write(&mut ws, "import-links", links);
    assert!(
        !ws.knowledge_check()
            .unwrap()
            .counts
            .contains_key(&FindingKind::RelationVariant)
    );
    write(
        &mut ws,
        "variants",
        vec![
            (None, link(&n[0].id, &n[1].id, relations::RELATED), false),
            (None, link(&n[1].id, &n[0].id, "relates-to"), false),
        ],
    );
    assert_eq!(
        ws.knowledge_check()
            .unwrap()
            .counts
            .get(&FindingKind::RelationVariant),
        Some(&1)
    );
}

#[test]
fn retired_notes_links_and_same_group_inputs_are_not_reported() {
    let (_root, _data, mut ws) = workspace();
    let n = write(
        &mut ws,
        "notes",
        vec![
            (None, note("Old list", "Old.\n"), false),
            (None, note("Page", "Page.\n"), false),
            (None, note("Fees 2026", "New fees.\n"), false),
            (None, note("Hub", "Hub.\n"), false),
        ],
    );
    let (old, page, fees, hub) = (&n[0], &n[1], &n[2], &n[3]);
    write(
        &mut ws,
        "links",
        vec![
            (None, link(&old.id, &page.id, relations::RELATED), false),
            (None, link(&fees.id, &old.id, relations::SUPERSEDES), false),
            (None, link(&hub.id, &old.id, relations::RELATED), false),
        ],
    );
    write(
        &mut ws,
        "retire",
        vec![(Some(old), note("Old list", "Old.\n"), true)],
    );
    // Only the hub's link still points at the retired note.
    let check = ws.knowledge_check().unwrap();
    assert_eq!(check.counts.get(&FindingKind::BrokenLink), Some(&1));
    let broken: Vec<_> = check
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::BrokenLink)
        .collect();
    assert!(broken[0].detail.contains("“Hub”"), "{}", broken[0].detail);

    // One group rewrites the page and derives the hub from it.
    write_with(
        &mut ws,
        "together",
        vec![
            (Some(page), note("Page", "Page v2.\n"), false),
            (Some(hub), note("Hub", "Hub from page.\n"), false),
        ],
        vec![input(page)],
    );
    assert!(
        !ws.knowledge_check()
            .unwrap()
            .counts
            .contains_key(&FindingKind::StaleInput)
    );
    let page = ws.store.record(&page.id).unwrap().unwrap();
    write(
        &mut ws,
        "later",
        vec![(Some(&page), note("Page", "Page v3.\n"), false)],
    );
    assert_eq!(
        ws.knowledge_check()
            .unwrap()
            .counts
            .get(&FindingKind::StaleInput),
        Some(&1)
    );
}

#[test]
fn archived_chat_material_and_chat_links_are_not_reported_as_broken() {
    let (_root, _data, mut ws) = workspace();
    let chat = ws.new_thread("Private").unwrap().id;
    let notes = write(
        &mut ws,
        "notes",
        vec![
            (None, note("General page", "Public.\n"), false),
            (
                None,
                RecordData::Note(Note {
                    chat: Some(chat.clone()),
                    ..Note::working("Discarded draft", "Private.\n")
                }),
                false,
            ),
        ],
    );
    write(
        &mut ws,
        "links",
        vec![
            (
                None,
                link(&notes[1].id, &notes[0].id, relations::SOURCE),
                false,
            ),
            (None, link(&notes[0].id, &chat, "discussed-in"), false),
        ],
    );
    write(
        &mut ws,
        "archive",
        vec![(Some(&notes[1]), notes[1].data.clone(), true)],
    );
    ws.resolve_thread(&chat).unwrap();
    let check = ws.knowledge_check().unwrap();
    assert!(
        !check.counts.contains_key(&FindingKind::BrokenLink),
        "{check:?}"
    );
    let text = serde_json::to_string(&check).unwrap();
    assert!(!text.contains(&notes[1].id) && !text.contains(&chat));
}
