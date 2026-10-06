//! Public CLI findings retain exact saved evidence without knowledge changes.
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    credentials: PathBuf,
    input: PathBuf,
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
            input: parent.join("finding.json"),
            data,
        }
    }
    fn process(&self, args: &[&str], machine: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command
            .args(args)
            .arg("--data-dir")
            .arg(self.data.path())
            .arg("--credentials-dir")
            .arg(&self.credentials);
        if machine {
            command.arg("--json");
        }
        command.output().unwrap()
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|_| panic!("missing typed envelope: {output:?}")),
        )
    }
    fn ok(&self, args: &[&str], name: &str) -> Value {
        let (code, envelope) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {envelope}");
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["command"], name);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }
    fn capture(&self, request: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(request).unwrap()).unwrap();
        self.run(&[
            "findings",
            "capture",
            "--file",
            self.input.to_str().unwrap(),
        ])
    }
    fn unopened(&self) {
        assert_eq!(fs::read_dir(self.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.credentials.exists());
    }
}

#[test]
fn malformed_findings_refuse_before_operational_vault_or_credential_authority() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["findings"],
        vec!["findings", "unknown"],
        vec!["findings", "capture"],
        vec!["findings", "capture", "extra", "--file", "missing.json"],
        vec!["findings", "capture", "--file", "one", "--file", "two"],
        vec!["findings", "list", "extra"],
        vec!["findings", "list", "--limit", "0"],
        vec!["findings", "list", "--limit", "101"],
        vec!["findings", "list", "--limit", "-1"],
        vec!["findings", "list", "--limit", "not-an-integer"],
        vec!["findings", "list", "--limit", "1", "--limit", "2"],
        vec!["findings", "list", "--before", &nil],
        vec!["findings", "list", "--before", "invalid"],
        vec!["findings", "list", "--state", "unknown"],
        vec!["findings", "list", "--state", "open", "--state", "all"],
        vec!["findings", "show"],
        vec!["findings", "show", &nil],
        vec!["findings", "show", &id, "--version", "1"],
        vec!["findings", "inspect", &id, "extra"],
        vec!["findings", "inspect", "invalid"],
        vec!["findings", "close", &id],
        vec!["findings", "close", &id, "--state", "resolved"],
        vec![
            "findings",
            "close",
            &id,
            "--version",
            "0",
            "--state",
            "resolved",
        ],
        vec![
            "findings",
            "close",
            &id,
            "--version",
            "1",
            "--state",
            "open",
        ],
        vec!["findings", "close", &id, "--version", "1", "--state", "all"],
        vec![
            "findings",
            "close",
            &id,
            "--version",
            "1",
            "--state",
            "resolved",
            "--version",
            "2",
        ],
        vec!["findings", "list", "--unknown"],
    ] {
        let (code, envelope) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        f.unopened();
    }
    for request in [
        json!({}),
        json!({"id":Uuid::nil(),"origin":{"kind":"identity_ambiguity","note_id":Uuid::new_v4()}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"identity_ambiguity","note_id":Uuid::nil()}}),
        json!({"id":Uuid::new_v4(),"origin":null}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"identity_ambiguity","note_id":Uuid::new_v4(),"extra":true}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"identity_ambiguity","note_id":Uuid::new_v4()},"extra":true}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unknown","note_id":Uuid::new_v4()}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"../outside.md","source_sha256":vec![1;32],"destination":"missing.md","start_byte":0}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"current.md","source_sha256":vec![1;31],"destination":"missing.md","start_byte":0}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"current.md","source_sha256":vec![1;32],"destination":"","start_byte":0}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"current.md","source_sha256":vec![1;32],"destination":"missing.md","start_byte":1024*1024}}),
        json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"current.md","source_sha256":null,"destination":"missing.md","start_byte":0}}),
    ] {
        let (code, envelope) = f.capture(&request);
        assert_eq!(code, 2, "{request}: {envelope}");
        assert_eq!(envelope["command"], "findings.capture");
        f.unopened();
    }
    let duplicate = format!("{{\"id\":\"{id}\",\"id\":\"{id}\",\"origin\":{{\"kind\":\"identity_ambiguity\",\"note_id\":\"{id}\"}}}}");
    let duplicate_origin = format!("{{\"id\":\"{id}\",\"origin\":{{\"kind\":\"identity_ambiguity\",\"kind\":\"identity_ambiguity\",\"note_id\":\"{id}\"}}}}");
    let duplicate_note_id = format!("{{\"id\":\"{id}\",\"origin\":{{\"kind\":\"identity_ambiguity\",\"note_id\":\"{id}\",\"note_id\":\"{id}\"}}}}");
    for bytes in [
        b"{".as_slice(),
        b"\xff".as_slice(),
        b"null".as_slice(),
        duplicate.as_bytes(),
        duplicate_origin.as_bytes(),
        duplicate_note_id.as_bytes(),
    ] {
        fs::write(&f.input, bytes).unwrap();
        assert_eq!(
            f.run(&["findings", "capture", "--file", f.input.to_str().unwrap()])
                .0,
            2
        );
        f.unopened();
    }
    fs::File::create(&f.input)
        .unwrap()
        .set_len(64 * 1024 + 1)
        .unwrap();
    assert_eq!(
        f.run(&["findings", "capture", "--file", f.input.to_str().unwrap()])
            .0,
        2
    );
    f.unopened();
}

#[test]
fn nonregular_finding_input_refuses_without_waiting_for_a_fifo_writer() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: this is an exclusively owned synthetic, NUL-terminated fixture path.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["findings", "capture", "--file"])
        .arg(&f.input)
        .args(["--json", "--data-dir"])
        .arg(f.data.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut blocked = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            blocked = true;
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!blocked, "finding input waited for a FIFO writer");
    assert_eq!(output.status.code(), Some(2));
    f.unopened();
}

#[test]
fn empty_finding_pages_work_without_a_vault_and_do_not_create_knowledge() {
    let f = Fixture::new();
    let page = f.ok(&["findings", "list"], "findings.list");
    assert_eq!(
        page,
        json!({"entries":[],"next_before":null,"open_count":0})
    );
    let output = f.process(
        &["findings", "list", "--state", "all", "--limit", "100"],
        false,
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Open findings: 0\nNo findings in this page.\n"
    );
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn captures_restart_drift_exact_quotes_and_competing_closure_preserve_knowledge() {
    let f = Fixture::new();
    let duplicate_id = Uuid::new_v4();
    let source_id = Uuid::new_v4();
    let missing_id = Uuid::new_v4();
    let duplicate_a =
        format!("\u{feff}---\r\nbrn_id: {duplicate_id}\r\n---\r\n# Duplicate 日本語 λ\r\n");
    let duplicate_b =
        format!("---\nbrn_id: {duplicate_id}\nbrn_kind: source\n---\nArchived duplicate õ\r\n");
    let consumer = format!("\u{feff}---\r\nbrn_id: {source_id}\r\n---\r\n# Consumer 日本語\r\n[根拠 λ][source]\r\n\r\n[source]: brn://note/{missing_id} \"保持\"\r\n");
    fs::create_dir(f.vault.join("archive")).unwrap();
    for (path, text) in [
        ("duplicate.md", &duplicate_a),
        ("archive/資料.md", &duplicate_b),
        ("consumer.md", &consumer),
    ] {
        fs::write(f.vault.join(path), text).unwrap();
    }
    let links = f.ok(
        &[
            "links",
            "show",
            "consumer.md",
            "--vault",
            f.vault.to_str().unwrap(),
        ],
        "links.show",
    );
    let link = &links["links"][0];
    assert_eq!(link["outcome"], "absent");
    let identity_request =
        json!({"id":Uuid::new_v4(),"origin":{"kind":"identity_ambiguity","note_id":duplicate_id}});
    let link_request = json!({"id":Uuid::new_v4(),"origin":{"kind":"unresolved_link","path":"consumer.md","source_sha256":links["source"]["sha256"],"destination":link["destination"],"start_byte":link["evidence"][0]["start_byte"]}});
    let mut records = vec![];
    for request in [&identity_request, &link_request] {
        let (code, envelope) = f.capture(request);
        assert_eq!(code, 0, "{envelope}");
        assert_eq!(envelope["command"], "findings.capture");
        let record = envelope["data"].clone();
        assert_eq!(record["draft"]["request"], *request);
        assert_eq!(record["version"], 1);
        assert_eq!(record["state"], "open");
        records.push(record);
    }
    assert_eq!(records[0]["draft"]["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(records[1]["draft"]["evidence"].as_array().unwrap().len(), 2);
    let list = f.ok(&["findings", "list", "--limit", "1"], "findings.list");
    assert_eq!(list["open_count"], 2);
    assert_eq!(list["entries"].as_array().unwrap().len(), 1);
    let cursor = list["next_before"].as_str().unwrap();
    let older = f.ok(&["findings", "list", "--before", cursor], "findings.list");
    assert_eq!(older["entries"].as_array().unwrap().len(), 1);
    assert_eq!(older["open_count"], 2);
    assert_ne!(
        list["entries"][0]["draft"]["request"]["id"],
        older["entries"][0]["draft"]["request"]["id"]
    );
    for record in &records {
        let id = record["draft"]["request"]["id"].as_str().unwrap();
        assert_eq!(f.ok(&["findings", "show", id], "findings.show"), *record);
        let inspection = f.ok(&["findings", "inspect", id], "findings.inspect");
        assert_eq!(inspection["record"], *record);
        assert!(inspection["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|observation| observation["outcome"] == "unchanged"));
        let output = f.process(&["findings", "show", id], false);
        assert!(output.status.success(), "{output:?}");
        let human = String::from_utf8(output.stdout).unwrap();
        for field in ["title", "summary"] {
            assert!(human.contains(&serde_json::to_string(&record["draft"][field]).unwrap()));
        }
        for evidence in record["draft"]["evidence"].as_array().unwrap() {
            let source = &evidence["source"];
            let path = source["path"].as_str().unwrap();
            let saved = fs::read(f.vault.join(path)).unwrap();
            assert_eq!(source["fingerprint"]["len"], saved.len());
            let hash: String = source["fingerprint"]["sha256"]
                .as_array()
                .unwrap()
                .iter()
                .map(|byte| format!("{:02x}", byte.as_u64().unwrap()))
                .collect();
            assert!(human.contains(&hash));
            assert!(human.contains(&serde_json::to_string(path).unwrap()));
            for field in ["device", "inode", "len"] {
                assert!(human.contains(&source["fingerprint"][field].to_string()));
            }
            if let Some(quote) = evidence["quote"].as_object() {
                let start = quote["start_byte"].as_u64().unwrap() as usize;
                let end = quote["end_byte"].as_u64().unwrap() as usize;
                assert_eq!(
                    &saved[start..end],
                    quote["quote"].as_str().unwrap().as_bytes()
                );
                assert!(human.contains(&format!("Quote bytes: {start}..{end}")));
                assert!(human.contains(&serde_json::to_string(&quote["quote"]).unwrap()));
            }
        }
    }
    assert!(f
        .ok(&["proposals", "list"], "proposals.list")
        .as_array()
        .unwrap()
        .is_empty());
    let changed = format!("{consumer}Owner-authored later bytes õ\r\n");
    fs::write(f.vault.join("consumer.md"), &changed).unwrap();
    let link_id = link_request["id"].as_str().unwrap();
    let drift = f.ok(&["findings", "inspect", link_id], "findings.inspect");
    assert_eq!(drift["record"], records[1]);
    assert!(drift["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .all(|observation| observation["outcome"] == "changed"));
    fs::remove_file(f.vault.join("archive/資料.md")).unwrap();
    let identity_id = identity_request["id"].as_str().unwrap();
    let missing = f.ok(&["findings", "inspect", identity_id], "findings.inspect");
    assert_eq!(missing["record"], records[0]);
    assert!(missing["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|observation| observation["outcome"] == "unavailable"));
    let resolved = f.ok(
        &[
            "findings",
            "close",
            link_id,
            "--version",
            "1",
            "--state",
            "resolved",
        ],
        "findings.close",
    );
    assert_eq!(resolved["state"], "resolved");
    assert_eq!(resolved["version"], 2);
    assert_eq!(resolved["draft"], records[1]["draft"]);
    assert_eq!(
        f.ok(
            &[
                "findings",
                "close",
                link_id,
                "--version",
                "1",
                "--state",
                "resolved"
            ],
            "findings.close"
        ),
        resolved
    );
    let (code, stale) = f.run(&[
        "findings",
        "close",
        link_id,
        "--version",
        "1",
        "--state",
        "dismissed",
    ]);
    assert_eq!(code, 1);
    assert_eq!(stale["error"]["code"], "CONTEXT_STALE");
    let page = f.ok(&["findings", "list"], "findings.list");
    assert_eq!(page["open_count"], 1);
    assert_eq!(page["entries"].as_array().unwrap().len(), 1);
    let terminal = f.ok(
        &["findings", "list", "--state", "resolved"],
        "findings.list",
    );
    assert_eq!(terminal["entries"], json!([resolved.clone()]));
    let mut conflicting = link_request.clone();
    conflicting["origin"]["destination"] = json!("another.md");
    assert_eq!(
        f.capture(&conflicting).1["error"]["code"],
        "OPERATION_CONFLICT"
    );
    // Closure and exact creation replay are available even after current vault
    // access disappears; neither substitutes fresh proof nor corrects Markdown.
    let retained_vault = f.vault.with_extension("retained");
    fs::rename(&f.vault, &retained_vault).unwrap();
    assert_eq!(f.capture(&link_request).1["data"], resolved);
    assert_eq!(
        f.ok(&["findings", "show", link_id], "findings.show"),
        resolved
    );
    let dismissed = f.ok(
        &[
            "findings",
            "close",
            identity_id,
            "--version",
            "1",
            "--state",
            "dismissed",
        ],
        "findings.close",
    );
    assert_eq!(dismissed["state"], "dismissed");
    assert_eq!(dismissed["draft"], records[0]["draft"]);
    let offline = f.ok(&["findings", "list", "--state", "all"], "findings.list");
    assert_eq!(offline["entries"].as_array().unwrap().len(), 2);
    assert_eq!(offline["open_count"], 0);
    assert!(f
        .ok(&["proposals", "list"], "proposals.list")
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        fs::read(retained_vault.join("consumer.md")).unwrap(),
        changed.as_bytes()
    );
    assert_eq!(
        fs::read(retained_vault.join("duplicate.md")).unwrap(),
        duplicate_a.as_bytes()
    );
    assert!(!retained_vault.join("archive/資料.md").exists());
    assert!(!f.vault.exists());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[test]
fn malformed_conflict_lookups_refuse_before_authority_and_cursor_json_is_bounded() {
    let f = Fixture::new();
    for args in [
        vec!["findings", "conflicts"],
        vec!["findings", "conflicts", "note.md", "extra"],
        vec!["findings", "conflicts", "../escape.md"],
        vec!["findings", "conflicts", "archive/old.md"],
        vec!["findings", "conflicts", "note.md", "--scope", "invalid"],
        vec!["findings", "conflicts", "note.md", "--limit", "0"],
        vec!["findings", "conflicts", "note.md", "--limit", "101"],
        vec!["findings", "conflicts", "note.md", "--limit", "-1"],
        vec!["findings", "conflicts", "note.md", "--limit", "NaN"],
        vec![
            "findings",
            "conflicts",
            "note.md",
            "--limit",
            "1",
            "--limit",
            "2",
        ],
        vec!["findings", "conflicts", "note.md", "--cursor", "{}"],
        vec!["findings", "conflicts", "note.md", "--cursor", "null"],
        vec!["findings", "conflicts", "note.md", "--cursor", "{"],
        vec!["findings", "conflicts", "note.md", "--before", "1"],
    ] {
        let (code, envelope) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        f.unopened();
    }
    let cursor = json!({"vault_id":Uuid::new_v4(),"path":"note.md","note_id":Uuid::new_v4(),"scope":"current","source":{"path":"note.md","fingerprint":{"device":1,"inode":2,"len":10,"sha256":vec![0;32]}},"before":Uuid::new_v4()});
    for change in 0..10 {
        let mut bad = cursor.clone();
        match change {
            0 => bad["vault_id"] = json!(Uuid::nil()),
            1 => bad["note_id"] = json!(Uuid::nil()),
            2 => bad["before"] = json!(Uuid::nil()),
            3 => bad["path"] = json!("another.md"),
            4 => bad["scope"] = json!("all"),
            5 => bad["source"]["path"] = json!("another.md"),
            6 => bad["source"]["fingerprint"]["len"] = json!(1024 * 1024 + 1),
            7 => bad["extra"] = json!(true),
            8 => bad["source"]["unexpected"] = json!(true),
            _ => bad["source"]["fingerprint"]["unexpected"] = json!(true),
        }
        let text = bad.to_string();
        assert_eq!(
            f.run(&["findings", "conflicts", "note.md", "--cursor", &text])
                .0,
            2,
            "cursor change {change}"
        );
        f.unopened();
    }
    let oversized = format!("{}{}", cursor, " ".repeat(8192));
    assert_eq!(
        f.run(&["findings", "conflicts", "note.md", "--cursor", &oversized])
            .0,
        2
    );
    let duplicate = cursor
        .to_string()
        .replacen("\"path\":", "\"path\":\"note.md\",\"path\":", 1);
    assert_eq!(
        f.run(&["findings", "conflicts", "note.md", "--cursor", &duplicate])
            .0,
        2
    );
    f.unopened();
}

#[cfg(target_os = "macos")]
#[test]
fn empty_conflict_lookup_routes_through_worker_preserves_saved_bytes_and_scope() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let text = format!("\u{feff}---\r\nbrn_id: {id}\r\n---\r\n# Synthetic õ 日本語\r\n");
    fs::write(f.vault.join("current.md"), &text).unwrap();
    let page = f.ok(
        &[
            "findings",
            "conflicts",
            "current.md",
            "--vault",
            f.vault.to_str().unwrap(),
        ],
        "findings.conflicts",
    );
    assert_eq!(page["path"], "current.md");
    assert_eq!(page["note_id"], json!(id));
    assert_eq!(page["scope"], "current");
    assert_eq!(page["entries"], json!([]));
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["open_count"], 0);
    let human = f.process(
        &[
            "findings",
            "conflicts",
            "current.md",
            "--scope",
            "all",
            "--limit",
            "100",
        ],
        false,
    );
    assert!(human.status.success(), "{human:?}");
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("Open conflicts matching this note: 0"));
    assert!(human.contains("No conflicts in this page."));
    assert!(human.contains("SHA-256:"));
    assert!(human.contains(&id.to_string()));
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        text.as_bytes()
    );
    assert!(f
        .ok(&["proposals", "list"], "proposals.list")
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn conflict_pages_keep_full_evidence_observations_and_query_bound_cursor_after_closure() {
    use brn_store::{
        work::{
            inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
            inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
            inbox_processing::InboxConversionFormat,
            inbox_source::InboxSourceBinding,
        },
        WorkStore,
    };
    let f = Fixture::new();
    let body = "```text\nBlue õ 🦀\n```\n";
    let source_id = Uuid::new_v4();
    let current_id = Uuid::new_v4();
    let analysis = Uuid::new_v4();
    let binding = InboxSourceBinding {
        visual: None,
        batch_id: Uuid::new_v4(),
        index: 0,
        original: InboxItem {
            capture: InboxCapture {
                id: Uuid::new_v4(),
                kind: InboxKind::Email,
                title: "Synthetic disagreement".into(),
                original_name: None,
                copy: InboxCopy {
                    directory: "/synthetic/inbox".into(),
                    directory_device: 1,
                    directory_inode: 2,
                    file_device: 1,
                    file_inode: 3,
                    byte_len: 4,
                    sha256: [0; 32],
                },
            },
            received_at_ms: 1,
        },
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: body.len() as u64,
        // SHA-256 of the fixed synthetic UTF-8 body above.
        sha256: [
            13, 27, 33, 70, 109, 119, 122, 25, 193, 101, 240, 130, 48, 66, 219, 76, 13, 62, 52,
            170, 81, 202, 66, 61, 95, 124, 104, 130, 233, 196, 90, 21,
        ],
        note_id: source_id,
    };
    let source_text = binding.markdown(body).unwrap();
    let current_text = format!("---\nbrn_id: {current_id}\n---\n# Decision\nGreen 日本語\n");
    fs::write(f.vault.join("source.md"), &source_text).unwrap();
    fs::write(f.vault.join("current.md"), &current_text).unwrap();
    let proof = f.ok(
        &[
            "proposals",
            "source",
            "source.md",
            "--vault",
            f.vault.to_str().unwrap(),
        ],
        "proposals.source",
    );
    let (mut store, _) = WorkStore::open(f.data.path()).unwrap();
    store
        .reserve_inbox_action(
            &InboxActionCapture {
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: analysis,
                conversation: None,
                source: serde_json::from_value(proof["source"].clone()).unwrap(),
                source_text: source_text.clone(),
                provider: "chatgpt".into(),
                model: "gpt-6-luna".into(),
                effort: "medium".into(),
            },
            "Compare synthetic color evidence",
        )
        .unwrap();
    drop(store);
    let quote = |text: &str, wording: &str| {
        let start = text.find(wording).unwrap();
        json!({"start_byte":start,"end_byte":start+wording.len(),"quote":wording})
    };
    let summary = format!(
        "{}\r\nTentative only. \u{001b}[31m",
        "Evidence õ 日本語 🦀 ".repeat(100)
    );
    for n in 0..2 {
        let request = json!({"id":Uuid::new_v4(),"origin":{"kind":"inbox_conflict","analysis_id":analysis,"title":format!("Unresolved color {n}"),"summary":summary,"source_quote":quote(&source_text,"Blue õ 🦀"),"other_path":"current.md","other_quote":quote(&current_text,"Green 日本語")}});
        assert_eq!(f.capture(&request).0, 0);
    }
    let first = f.ok(
        &["findings", "conflicts", "current.md", "--limit", "1"],
        "findings.conflicts",
    );
    assert_eq!(first["open_count"], 2);
    assert_eq!(first["entries"].as_array().unwrap().len(), 1);
    let cursor = first["next_cursor"].to_string();
    assert_ne!(first["next_cursor"], Value::Null);
    let id = first["entries"][0]["record"]["draft"]["request"]["id"]
        .as_str()
        .unwrap();
    f.ok(
        &[
            "findings",
            "close",
            id,
            "--version",
            "1",
            "--state",
            "dismissed",
        ],
        "findings.close",
    );
    fs::remove_file(f.vault.join("source.md")).unwrap();
    let next = f.ok(
        &[
            "findings",
            "conflicts",
            "current.md",
            "--limit",
            "1",
            "--cursor",
            &cursor,
        ],
        "findings.conflicts",
    );
    assert_eq!(next["open_count"], 1);
    assert_eq!(next["next_cursor"], Value::Null);
    let inspection = &next["entries"][0];
    assert_eq!(inspection["record"]["draft"]["summary"], summary);
    assert_eq!(inspection["evidence"][0]["outcome"], "unavailable");
    assert_eq!(inspection["evidence"][1]["outcome"], "unchanged");
    let human = f.process(&["findings", "conflicts", "current.md"], false);
    assert!(human.status.success(), "{human:?}");
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains(&serde_json::to_string(&summary).unwrap()));
    assert!(!human.contains('\u{001b}'));
    assert!(human.contains("Fresh observation 0: Unavailable"));
    assert!(human.contains("Fresh observation 1: Unchanged"));
    for retained in inspection["record"]["draft"]["evidence"]
        .as_array()
        .unwrap()
    {
        assert!(human.contains(&retained["quote"]["quote"].to_string()));
        let hash = retained["source"]["fingerprint"]["sha256"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| format!("{:02x}", v.as_u64().unwrap()))
            .collect::<String>();
        assert!(human.contains(&hash));
    }
    assert_eq!(
        f.run(&[
            "findings",
            "conflicts",
            "current.md",
            "--scope",
            "all",
            "--cursor",
            &cursor
        ])
        .0,
        2
    );
    fs::write(
        f.vault.join("current.md"),
        format!("{current_text}Owner edit\n"),
    )
    .unwrap();
    let (code, stale) = f.run(&["findings", "conflicts", "current.md", "--cursor", &cursor]);
    assert_eq!(code, 1);
    assert_eq!(stale["error"]["code"], "CONTEXT_STALE");
    assert!(f
        .ok(&["proposals", "list"], "proposals.list")
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}
