#[path = "../examples/wiki_eval_seed/seed.rs"]
#[allow(dead_code)]
mod seed;

use brn_threads_app::{RecordData, Workspace};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};

const PLANT: &str = "The reefer plug audit happens every second Thursday.";

fn fixture() -> String {
    json!({
        "version": 2,
        "cases": [{
            "id": "X1",
            "seeds": {
                "threads": [{"key": "private", "title": "Coordinator hiring"}],
                "notes": [
                    {"key": "target", "title": "Project Kestrel", "text": "Kestrel moves the cold store."},
                    {"key": "roster", "title": "Roster", "text": "Who works where."},
                    {"key": "secret", "title": "Offer letter", "text": "Salary details.", "chat": "private"},
                    {"key": "fees", "title": "Berth fees", "text": "Fees are 10.", "edits": ["Fees are 12.", "Fees are 14."]},
                    {"key": "policy", "title": "Policy", "text": "Imported policy.", "import": {"source": "synthetic://policy"}},
                    {"key": "handbook", "title": "Handbook", "text": "Start here.",
                     "import": {"source": "synthetic://handbook"},
                     "long": {"sections": 60, "target_bytes": 150000, "plant_section": 55,
                              "plant_heading": "Reefer plug monitoring", "plant_text": PLANT, "seed": 7}},
                    {"key": "dup-a", "title": "Coop opening hours", "text": "9 to 5."},
                    {"key": "dup-b", "title": "Coop opening hours", "text": "10 to 4."}
                ],
                "distractors": [{
                    "key": "bird", "count": 6, "title": "Project {bird}",
                    "text": "Retrofit of the {site} at the {quay} quay.",
                    "values": {"bird": ["Heron", "Egret", "Tern", "Plover", "Avocet", "Knot"], "site": ["dry store", "office"], "quay": ["south", "east"]}
                }],
                "links": [
                    {"from": "roster", "to": "target", "relation": "related"},
                    {"from": "fees", "to": "policy", "relation": "supersedes"},
                    {"from": "bird-0", "to": "case", "relation": "source"}
                ],
                "topics": {"pages": ["roster", "bird-1"]}
            }
        }]
    })
    .to_string()
}

fn dir() -> (tempfile::TempDir, PathBuf) {
    let root =
        tempfile::TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let data = root.path().join("data");
    (root, data)
}

/// Titles and bodies by key, for comparing seeds whose IDs may differ.
fn contents(workspace: &Workspace, report: &seed::Report) -> BTreeMap<String, (String, String)> {
    report
        .notes
        .iter()
        .map(|(key, id)| {
            let record = workspace.store.record(id).unwrap().unwrap();
            let RecordData::Note(note) = record.data else {
                panic!("{key} is not a note")
            };
            (key.clone(), (note.title, note.markdown))
        })
        .collect()
}

fn note(workspace: &Workspace, id: &str) -> (u64, brn_threads_app::Note) {
    let record = workspace.store.record(id).unwrap().unwrap();
    let RecordData::Note(note) = record.data else {
        panic!("not a note")
    };
    (record.version, note)
}

#[test]
fn seeding_is_deterministic_in_content_and_filler_pairs_are_linked() {
    let (_a, one) = dir();
    let (_b, two) = dir();
    let first = seed::seed_fixture(&one, &fixture(), "X1", 25, 9).unwrap();
    let second = seed::seed_fixture(&two, &fixture(), "X1", 25, 9).unwrap();
    let (_c, other_dir) = dir();
    let other = seed::seed_fixture(&other_dir, &fixture(), "X1", 25, 10).unwrap();
    let (w1, w2) = (
        Workspace::open(&one).unwrap(),
        Workspace::open(&two).unwrap(),
    );
    assert_eq!(contents(&w1, &first), contents(&w2, &second));
    assert_eq!(first.ranges.len(), 1);
    assert_eq!(
        first.ranges["handbook"].start,
        second.ranges["handbook"].start
    );
    assert_eq!(first.versions, second.versions);
    let w3 = Workspace::open(&other_dir).unwrap();
    let all = |w: &Workspace| {
        let mut texts: Vec<String> = w
            .records()
            .unwrap()
            .into_iter()
            .filter_map(|r| match r.data {
                RecordData::Note(n) => Some(n.markdown),
                _ => None,
            })
            .collect();
        texts.sort();
        texts
    };
    assert_eq!(all(&w1), all(&w2));
    assert_ne!(all(&w1), all(&w3));
    assert_eq!(other.filler, 25);

    // 6 distractors + 8 fixture notes + 25 filler + Topics.
    let records = w1.records().unwrap();
    let notes: Vec<_> = records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Note(n) => Some((r.id.as_str(), n)),
            _ => None,
        })
        .collect();
    assert_eq!(notes.len(), 6 + 8 + 25 + 1);
    let fixture_ids: std::collections::BTreeSet<&String> = first.notes.values().collect();
    let filler: Vec<&str> = notes
        .iter()
        .filter(|(id, n)| !fixture_ids.contains(&id.to_string()) && n.title.contains(" log "))
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(filler.len(), 25);
    let linked = |from: &str, to: &str| {
        records.iter().any(|r| {
            matches!(&r.data, RecordData::Link(l)
                if l.from == from && l.to == to && l.relation == "related")
        })
    };
    // Titles end in their creation number; pairs are (1,2), (3,4) ... and 25 joins 24.
    let mut ordered = filler.clone();
    ordered.sort_by_key(|id| {
        let title = note(&w1, id).1.title;
        title.split(" log ").nth(1).unwrap()[..5].to_owned()
    });
    for pair in ordered.chunks(2).filter(|p| p.len() == 2) {
        assert!(linked(pair[0], pair[1]) && linked(pair[1], pair[0]));
    }
    assert!(linked(ordered[24], ordered[23]));
}

#[test]
fn explicit_notes_links_chat_edits_imports_and_topics() {
    let (_root, data) = dir();
    let report = seed::seed_fixture(&data, &fixture(), "X1", 0, 1).unwrap();
    let workspace = Workspace::open(&data).unwrap();
    let id = |key: &str| report.notes[key].clone();

    let records = workspace.records().unwrap();
    let has_link = |from: &str, to: &str, relation: &str| {
        records.iter().any(|r| {
            matches!(&r.data, RecordData::Link(l)
                if l.from == from && l.to == to && l.relation == relation)
        })
    };
    assert!(has_link(&id("roster"), &id("target"), "related"));
    assert!(has_link(&id("fees"), &id("policy"), "supersedes"));
    assert!(has_link(&id("bird-0"), &report.threads["case"], "source"));

    let (_, secret) = note(&workspace, &id("secret"));
    assert_eq!(
        secret.chat.as_deref(),
        Some(report.threads["private"].as_str())
    );
    assert!(note(&workspace, &id("target")).1.chat.is_none());

    let (version, fees) = note(&workspace, &id("fees"));
    assert_eq!((version, report.versions["fees"]), (3, 3));
    assert_eq!(fees.markdown, "# Berth fees\n\nFees are 14.\n");
    assert_eq!(report.versions["target"], 1);

    let (_, policy) = note(&workspace, &id("policy"));
    assert!(policy.protected && policy.import.is_some_and(|i| i.full_note));

    let handbook = note(&workspace, &id("handbook")).1;
    assert!(handbook.protected);
    let range = &report.ranges["handbook"];
    assert!(range.start > 131_072, "plant starts at {}", range.start);
    assert!(handbook.markdown.len() >= 140_000 && handbook.markdown.len() < 170_000);
    let bytes = &handbook.markdown.as_bytes()[range.start..range.end];
    assert!(String::from_utf8_lossy(bytes).contains(PLANT));

    assert_eq!(report.notes.len(), 8 + 6);
    assert_eq!(report.threads.len(), 2);
    let topics = workspace.topics().unwrap().expect("topics pointer set");
    assert_eq!(Some(&topics.note), report.topics.as_ref());
    let pages: Vec<&str> = topics.entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(pages, ["Roster", "Project Egret"]);
}

#[test]
fn distractors_cycle_values_and_have_unique_titles() {
    let (_root, data) = dir();
    let report = seed::seed_fixture(&data, &fixture(), "X1", 0, 1).unwrap();
    let workspace = Workspace::open(&data).unwrap();
    let content = contents(&workspace, &report);
    assert_eq!(
        content["bird-4"],
        (
            "Project Avocet".into(),
            "# Project Avocet\n\nRetrofit of the dry store at the south quay.\n".into()
        )
    );
}

#[test]
fn invalid_fixtures_fail_with_clear_messages() {
    let with = |seeds: serde_json::Value| {
        json!({"version": 2, "cases": [{"id": "X1", "seeds": seeds}]}).to_string()
    };
    let err =
        |seeds: serde_json::Value| seed::validate(&with(seeds), "X1").unwrap_err().to_string();
    let note = |key: &str, title: &str| json!({"key": key, "title": title});

    assert!(
        err(json!({"notes": [note("a", "A"), note("a", "B")]})).contains("duplicate seed key a")
    );
    assert!(err(json!({"notes": [note("case", "A")]})).contains("duplicate seed key case"));
    assert!(
        err(json!({"notes": [note("a", "A")], "links": [{"from": "a", "to": "zz", "relation": "related"}]}))
            .contains("unknown key zz")
    );
    assert!(
        err(json!({"distractors": [{"key": "d", "count": 2, "title": "T {x}", "text": "t"}]}))
            .contains("placeholder {x} has no values")
    );
    assert!(
        err(json!({"distractors": [{"key": "d", "count": 3, "title": "Same {x}", "text": "t", "values": {"x": ["1"]}}]}))
            .contains("duplicate distractor title")
    );
    assert!(
        err(json!({"notes": [note("a", "Title")],
                   "distractors": [{"key": "d", "count": 1, "title": "Title", "text": "t"}]}))
        .contains("duplicate distractor title")
    );
    assert!(
        err(json!({"notes": [{"key": "a", "title": "A", "chat": "nobody"}]}))
            .contains("unknown chat thread")
    );
    assert!(
        err(json!({"notes": [note("a", "A")], "topics": {"pages": ["b"]}}))
            .contains("unknown note b")
    );
    assert!(
        err(json!({"notes": [{"key": "a", "title": "A", "import": {"source": "s"}, "edits": ["x"]}]}))
            .contains("cannot have edits")
    );
    assert!(
        seed::validate(&with(json!({})), "missing")
            .unwrap_err()
            .to_string()
            .contains("not found")
    );
    // Duplicate explicit titles are allowed on purpose.
    assert_eq!(
        seed::validate(
            &with(json!({"notes": [note("a", "A"), note("b", "A")]})),
            "X1"
        )
        .unwrap(),
        2
    );
}

#[test]
fn seeding_refuses_a_data_dir_that_is_not_fresh() {
    let (_root, data) = dir();
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("keep"), "x").unwrap();
    let error = seed::seed_fixture(&data, &fixture(), "X1", 0, 1).unwrap_err();
    assert!(error.to_string().contains("must be fresh"));
}
