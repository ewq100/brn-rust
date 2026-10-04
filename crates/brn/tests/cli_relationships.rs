//! Relationship observations through real CLI processes and disposable saved notes.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    credentials: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let parent = data.path().parent().unwrap();
        let vault = parent.join("vault");
        fs::create_dir(&vault).unwrap();
        Self {
            vault,
            credentials: parent.join("task.credentials"),
            data,
        }
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
            .arg(&self.vault)
            .arg("--credentials-dir")
            .arg(&self.credentials);
        if machine {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        let value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("missing typed envelope: {output:?}"));
        (output.status.code().unwrap(), value)
    }

    fn ok(&self, args: &[&str], name: &str) -> Value {
        let (code, envelope) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {envelope}");
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["command"], name);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }

    fn page(&self, args: &[&str]) -> Value {
        let mut full = vec!["relationships", "list"];
        full.extend_from_slice(args);
        self.ok(&full, "relationships.list")
    }
}

fn managed(id: Uuid, body: &str) -> String {
    format!("---\nbrn_id: {id}\n---\n{body}")
}

#[test]
fn invalid_page_arguments_refuse_before_storage_or_credentials_open() {
    let f = Fixture::new();
    for args in [
        vec!["relationships"],
        vec!["relationships", "unknown"],
        vec!["relationships", "list", "extra"],
        vec!["relationships", "list", "--unknown", "value"],
        vec!["relationships", "list", "--scope"],
        vec!["relationships", "list", "--scope", "Current"],
        vec![
            "relationships",
            "list",
            "--scope",
            "current",
            "--scope",
            "all",
        ],
        vec!["relationships", "list", "--offset"],
        vec!["relationships", "list", "--offset", "-1"],
        vec!["relationships", "list", "--offset", "bad"],
        vec!["relationships", "list", "--offset", "184467440737095516160"],
        vec!["relationships", "list", "--offset", "0", "--offset", "0"],
        vec!["relationships", "list", "--limit"],
        vec!["relationships", "list", "--limit", "0"],
        vec!["relationships", "list", "--limit", "201"],
        vec!["relationships", "list", "--limit", "50", "--limit", "50"],
    ] {
        let (code, envelope) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(f.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
        assert!(!f.credentials.exists());
    }
}

#[test]
fn default_and_all_pages_coalesce_exact_proofs_keep_origins_and_rebuild_without_writes() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let archived_id = Uuid::new_v4();
    let target = managed(target_id, "Target õ\r\n");
    f.write("target.md", &target);
    let inventory = f.ok(&["identity", "inventory"], "identity.inventory");
    let target_hash = inventory["notes"][0]["sha256"].clone();
    let target_quote = "Target õ\r\n";
    let start = target.find(target_quote).unwrap();
    let citation = json!({"note_id":target_id,"sha256":target_hash,
        "start_byte":start,"end_byte":start+target_quote.len(),"quote":target_quote});
    let source = format!(
        "\u{feff}---\r\nbrn_id: {source_id}\r\nbrn_provenance: [{}]\r\n---\r\n[stable](brn://note/{target_id})\r\n[again](target.md)\r\n[reference][t]\r\n\r\n[t]: target.md\r\n",
        serde_json::to_string(&citation).unwrap()
    );
    let archived = format!(
        "---\nbrn_id: {archived_id}\nbrn_kind: source\n---\n[history](brn://note/{target_id})\n"
    );
    f.write("current.md", &source);
    f.write("archive/source.md", &archived);

    let current = f.page(&[]);
    assert_eq!(current["scope"], "current");
    assert_eq!(current["offset"], 0);
    assert_eq!(current["total"], 2);
    assert_eq!(current["issues"], json!([]));
    assert_eq!(current["duplicates"], json!([]));
    let edges = current["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    for origin in ["explicit_link", "inferred_provenance"] {
        let edge = edges.iter().find(|edge| edge["origin"] == origin).unwrap();
        assert_eq!(edge["source"]["path"], "current.md");
        assert_eq!(edge["source"]["note_id"], source_id.to_string());
        assert_eq!(edge["target"]["path"], "target.md");
        assert_eq!(edge["target"]["note_id"], target_id.to_string());
        assert_eq!(edge["target"]["sha256"], target_hash);
        let evidence = edge["evidence"].as_array().unwrap();
        assert!(!evidence.is_empty());
        if origin == "explicit_link" {
            assert!(
                evidence.len() >= 3,
                "repeated links lost exact proofs: {edge}"
            );
        }
        for proof in evidence {
            let text = match proof["endpoint"].as_str().unwrap() {
                "source" => &source,
                "target" => &target,
                _ => panic!("unknown endpoint: {proof}"),
            };
            let start = proof["start_byte"].as_u64().unwrap() as usize;
            let end = proof["end_byte"].as_u64().unwrap() as usize;
            assert_eq!(&text[start..end], proof["quote"].as_str().unwrap());
        }
    }
    let all = f.page(&["--scope", "all"]);
    assert_eq!(all["scope"], "all");
    assert_eq!(all["total"], 3);
    assert!(all["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["source"]["path"] == "archive/source.md"));
    let current_limited = f.page(&["--limit", "1"]);
    assert_eq!(current_limited["scope"], "current");
    assert_eq!(current_limited["total"], 2);
    assert_eq!(current_limited["edges"], json!([current["edges"][0]]));
    for scope in ["source", "history"] {
        let page = f.page(&["--scope", scope]);
        assert_eq!(page["scope"], scope);
        assert_eq!(page["total"], 0);
        assert_eq!(page["edges"], json!([]));
    }
    let second = f.page(&["--scope", "all", "--offset", "1", "--limit", "1"]);
    assert_eq!(second["offset"], 1);
    assert_eq!(second["total"], 3);
    assert_eq!(second["edges"], json!([all["edges"][1]]));
    assert_eq!(
        f.page(&["--scope", "all", "--offset", "3"])["edges"],
        json!([])
    );

    fs::remove_file(f.data.path().join("index.sqlite")).unwrap();
    assert_eq!(f.page(&[]), current);
    let plain = f.process(&["relationships", "list"], false);
    assert!(plain.status.success(), "{plain:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&plain.stdout).unwrap(),
        current
    );
    assert_eq!(f.ok(&["proposals", "list"], "proposals.list"), json!([]));
    for (path, text) in [
        ("current.md", source),
        ("target.md", target),
        ("archive/source.md", archived),
    ] {
        assert_eq!(fs::read(f.vault.join(path)).unwrap(), text.as_bytes());
    }
}

#[test]
fn unchanged_source_links_observe_target_edits_uuid_moves_duplicates_and_incomplete_inventory() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let source = managed(
        source_id,
        &format!("[stable](brn://note/{target_id})\n[path](target.md)\n"),
    );
    f.write("current.md", &source);
    f.write("target.md", &managed(target_id, "Before õ\n"));
    let before = f.page(&[]);
    assert_eq!(before["total"], 1);
    let edited = managed(target_id, "Changed target õ\n");
    f.write("target.md", &edited);
    let after = f.page(&[]);
    assert_eq!(after["total"], 1);
    assert_ne!(
        after["edges"][0]["target"]["sha256"],
        before["edges"][0]["target"]["sha256"]
    );
    assert_eq!(after["edges"][0]["source"], before["edges"][0]["source"]);
    fs::rename(f.vault.join("target.md"), f.vault.join("moved.md")).unwrap();
    let moved = f.page(&[]);
    assert_eq!(moved["total"], 1);
    assert_eq!(moved["edges"][0]["target"]["path"], "moved.md");
    fs::copy(f.vault.join("moved.md"), f.vault.join("duplicate.md")).unwrap();
    let duplicate = f.page(&[]);
    assert_eq!(duplicate["edges"], json!([]));
    assert_eq!(duplicate["duplicates"][0]["note_id"], target_id.to_string());
    fs::remove_file(f.vault.join("duplicate.md")).unwrap();
    f.write("bad.md", "---\nbrn_id: invalid\n---\nIncomplete evidence\n");
    let incomplete = f.page(&[]);
    assert_eq!(incomplete["edges"], json!([]));
    assert!(incomplete["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["path"] == "bad.md"));
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        source.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("moved.md")).unwrap(),
        edited.as_bytes()
    );
    assert_eq!(f.ok(&["proposals", "list"], "proposals.list"), json!([]));
}
