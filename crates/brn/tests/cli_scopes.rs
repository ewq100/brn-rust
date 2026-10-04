//! Scoped reads through real CLI processes with saved synthetic Markdown only.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let vault = data.path().parent().unwrap().join("vault");
        fs::create_dir(&vault).unwrap();
        Self { data, vault }
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.vault.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn process(&self, args: &[&str], machine: bool) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command
            .args(args)
            .arg("--data-dir")
            .arg(self.data.path())
            .arg("--vault")
            .arg(&self.vault);
        if machine {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        let value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{args:?}: {output:?}"));
        (output.status.code().unwrap(), value)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, envelope) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {envelope}");
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }

    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(self.data.path()).unwrap().count(), 0);
        assert!(!self.data.path().with_file_name("data.credentials").exists());
    }
}

fn paths(data: &Value, key: &str) -> Vec<String> {
    let mut paths: Vec<_> = data[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["path"].as_str().unwrap().to_owned())
        .collect();
    paths.sort();
    paths
}

fn notes() -> Vec<(&'static str, String)> {
    vec![
        ("current.md", "\u{feff}# Current õ\r\nBeacon current wording\r\n".into()),
        (
            "sources/current.md",
            "---\nbrn_kind: source\n---\nBeacon original current source".into(),
        ),
        (
            "sources/old.md",
            "---\nbrn_kind: source\nbrn_state: history\n---\nBeacon superseded original".into(),
        ),
        (
            "history/old.md",
            "---\nbrn_state: history\n---\nBeacon historical knowledge".into(),
        ),
        ("archive/legacy.md", "Beacon archived unmanaged wording".into()),
        (
            "archive/source.md",
            "\u{feff}---\r\nbrn_kind: source\r\nbrn_state: current\r\n---\r\nBeacon archived original õ".into(),
        ),
    ]
}

#[test]
fn default_queries_only_return_current_knowledge_and_explicit_scopes_return_saved_evidence() {
    let fixture = Fixture::new();
    let notes = notes();
    for (path, text) in &notes {
        fixture.write(path, text);
    }
    let cases = [
        ("current", vec!["current.md"]),
        (
            "source",
            vec!["archive/source.md", "sources/current.md", "sources/old.md"],
        ),
        (
            "history",
            vec![
                "archive/legacy.md",
                "archive/source.md",
                "history/old.md",
                "sources/old.md",
            ],
        ),
        (
            "all",
            vec![
                "archive/legacy.md",
                "archive/source.md",
                "current.md",
                "history/old.md",
                "sources/current.md",
                "sources/old.md",
            ],
        ),
    ];
    for (scope, expected) in cases {
        let page = fixture.ok(&["notes", "list", "--scope", scope]);
        assert_eq!(page["scope"], scope);
        assert_eq!(paths(&page, "notes"), expected);
        assert!(page["next_cursor"].is_null());
        let search = fixture.ok(&["search", "Beacon", "--scope", scope, "--limit", "50"]);
        assert_eq!(search["scope"], scope);
        assert_eq!(search["query"], "Beacon");
        assert_eq!(search["keyword_only"], true);
        assert_eq!(paths(&search, "hits"), expected);
        for hit in search["hits"].as_array().unwrap() {
            let text = &notes
                .iter()
                .find(|(path, _)| *path == hit["path"].as_str().unwrap())
                .unwrap()
                .1;
            let start = hit["start_byte"].as_u64().unwrap() as usize;
            let end = hit["end_byte"].as_u64().unwrap() as usize;
            assert_eq!(hit["quote"], &text[start..end]);
        }
    }
    for args in [vec!["notes", "list"], vec!["search", "Beacon"]] {
        let result = fixture.ok(&args);
        assert_eq!(result["scope"], "current");
        assert_eq!(
            paths(&result, if args[0] == "notes" { "notes" } else { "hits" }),
            ["current.md"]
        );
    }
    for (path, text) in notes {
        assert_eq!(fs::read(fixture.vault.join(path)).unwrap(), text.as_bytes());
    }
    assert_eq!(fixture.ok(&["proposals", "list"]), json!([]));
    assert_eq!(fixture.ok(&["edit", "list"]), json!([]));
}

#[test]
fn scoped_show_preserves_complete_bytes_and_reports_the_requested_scope() {
    let fixture = Fixture::new();
    for (path, text) in notes() {
        fixture.write(path, &text);
        let scope = if path == "current.md" {
            "current"
        } else if path.contains("source") {
            "source"
        } else {
            "history"
        };
        for requested in [scope, "all"] {
            let note = fixture.ok(&["notes", "show", path, "--scope", requested]);
            assert_eq!(
                note,
                json!({"path": path, "text": text, "scope": requested})
            );
            let plain = fixture.process(&["notes", "show", path, "--scope", requested], false);
            assert!(plain.status.success(), "{plain:?}");
            assert_eq!(plain.stdout, text.as_bytes());
        }
        if path != "current.md" {
            let (code, envelope) = fixture.run(&["notes", "show", path]);
            assert_ne!(code, 0, "{envelope}");
        }
        assert_eq!(fs::read(fixture.vault.join(path)).unwrap(), text.as_bytes());
    }
}

#[test]
fn scoped_folder_and_cursor_filters_are_contained_and_exclusive() {
    let fixture = Fixture::new();
    for (path, text) in notes() {
        fixture.write(path, &text);
    }
    let page = fixture.ok(&["notes", "list", "--scope=source", "--folder", "sources"]);
    assert_eq!(
        paths(&page, "notes"),
        ["sources/current.md", "sources/old.md"]
    );
    let page = fixture.ok(&[
        "notes",
        "list",
        "--scope",
        "source",
        "--folder",
        "sources",
        "--cursor",
        "sources/current.md",
    ]);
    assert_eq!(paths(&page, "notes"), ["sources/old.md"]);
    assert_eq!(
        fixture.ok(&["notes", "list", "--folder", "sources"])["notes"],
        json!([])
    );
    let page = fixture.ok(&[
        "notes",
        "list",
        "--scope",
        "history",
        "--folder",
        "archive",
        "--cursor",
        "archive/legacy.md",
    ]);
    assert_eq!(paths(&page, "notes"), ["archive/source.md"]);
}

#[test]
fn scope_filtering_precedes_page_and_search_limits() {
    let fixture = Fixture::new();
    fixture.write("current.md", "Beacon current knowledge");
    for number in 0..201 {
        fixture.write(
            &format!("sources/{number:03}.md"),
            "---\nbrn_kind: source\n---\nBeacon Beacon Beacon original evidence",
        );
    }
    let current = fixture.ok(&["notes", "list"]);
    assert_eq!(paths(&current, "notes"), ["current.md"]);
    assert!(current["next_cursor"].is_null());
    assert_eq!(
        paths(&fixture.ok(&["search", "Beacon", "--limit", "1"]), "hits"),
        ["current.md"]
    );
    let first = fixture.ok(&["notes", "list", "--scope", "source"]);
    assert_eq!(first["notes"].as_array().unwrap().len(), 200);
    assert_eq!(first["notes"][0]["path"], "sources/000.md");
    assert_eq!(first["next_cursor"], "sources/199.md");
    let second = fixture.ok(&[
        "notes",
        "list",
        "--scope",
        "source",
        "--cursor",
        first["next_cursor"].as_str().unwrap(),
    ]);
    assert_eq!(paths(&second, "notes"), ["sources/200.md"]);
    assert!(second["next_cursor"].is_null());
}

#[test]
fn malformed_scope_and_containment_arguments_refuse_before_storage_or_auth_startup() {
    let cases: &[&[&str]] = &[
        &["notes", "list", "--scope", "Current"],
        &["notes", "list", "--scope", "sources"],
        &["notes", "show", "current.md", "--scope", "ALL"],
        &["search", "Beacon", "--scope", "historical"],
        &["notes", "list", "--scope="],
        &["notes", "show", "current.md", "--scope"],
        &["search", "Beacon", "--scope"],
        &["notes", "list", "--scope", "all", "--scope=all"],
        &[
            "notes",
            "show",
            "current.md",
            "--scope=all",
            "--scope",
            "current",
        ],
        &[
            "search", "Beacon", "--scope", "source", "--scope", "history",
        ],
        &["notes", "list", "--scopes", "all"],
        &["notes", "show", "--scope", "all"],
        &["search", "--scope", "all"],
        &["notes", "show", "current.md", "extra", "--scope", "all"],
        &["notes", "list", "--folder", "archive"],
        &[
            "notes",
            "list",
            "--folder",
            "archive/subdir",
            "--scope",
            "current",
        ],
        &["notes", "list", "--cursor", "archive/old.md"],
        &["notes", "show", "archive/old.md"],
        &["notes", "list", "--scope", "all", "--folder", "../escape"],
        &[
            "notes",
            "list",
            "--scope",
            "history",
            "--folder",
            "archive/../escape",
        ],
        &["notes", "list", "--scope", "source", "--folder", ".hidden"],
        &["notes", "list", "--scope", "all", "--folder", "/absolute"],
        &[
            "notes",
            "list",
            "--scope",
            "all",
            "--folder",
            "archive//old",
        ],
        &[
            "notes",
            "list",
            "--scope",
            "all",
            "--cursor",
            "archive/../escape.md",
        ],
        &[
            "notes",
            "list",
            "--scope",
            "all",
            "--cursor",
            "archive/.hidden.md",
        ],
        &[
            "notes",
            "list",
            "--scope",
            "all",
            "--cursor",
            "archive/old.txt",
        ],
        &["notes", "show", "../escape.md", "--scope", "all"],
        &["notes", "show", "/absolute.md", "--scope", "source"],
        &["notes", "show", "archive/.hidden.md", "--scope", "history"],
        &["notes", "show", "archive\\old.md", "--scope", "all"],
        &["notes", "show", "archive/old.txt", "--scope", "all"],
        &["search", "Beacon", "--scope", "all", "--limit", "0"],
        &["search", "Beacon", "--scope", "all", "--limit", "51"],
    ];
    for args in cases {
        let fixture = Fixture::new();
        let (code, envelope) = fixture.run(args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        fixture.assert_unopened();
    }
}

#[test]
fn saved_classification_changes_with_retained_size_and_mtime_are_observed_after_restart() {
    let fixture = Fixture::new();
    let path = fixture.vault.join("mutable.md");
    let current = "---\nbrn_kind: knowledge\n---\nBeacon exact saved wording õ";
    let source = "---\nbrn_kind: source   \n---\nBeacon exact saved wording õ";
    assert_eq!(current.len(), source.len());
    fixture.write("mutable.md", current);
    assert_eq!(
        paths(&fixture.ok(&["search", "Beacon"]), "hits"),
        ["mutable.md"]
    );
    let before = fs::metadata(&path).unwrap();
    fs::write(&path, source).unwrap();
    fs::File::open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(before.modified().unwrap()))
        .unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(fixture.ok(&["notes", "list"])["notes"], json!([]));
    assert_eq!(fixture.ok(&["search", "Beacon"])["hits"], json!([]));
    assert_eq!(
        paths(
            &fixture.ok(&["search", "Beacon", "--scope", "source"]),
            "hits"
        ),
        ["mutable.md"]
    );
    assert_eq!(
        fixture.ok(&["notes", "show", "mutable.md", "--scope", "source"])["text"],
        source
    );
    assert_eq!(fs::read(path).unwrap(), source.as_bytes());
}
