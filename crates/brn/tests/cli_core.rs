//! Subprocess tests for the Phase C1 ingestion/retrieval commands: `import`,
//! `documents set-search-approval`, `index build`, `search`. Workspaces are
//! disposable temp dirs and are seeded through the CLI itself.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;
use uuid::Uuid;

#[cfg(target_os = "macos")]
#[test]
fn documents_surfaces_label_shadowed_changed_and_missing_without_old_current_bytes() {
    use brn_workflow::{Config, SearchApproval, Workspace};
    use std::sync::atomic::AtomicBool;
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let file = write(vault.path(), "plan.md", "oldterm");
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let imported = w
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            SearchApproval::Approved,
        )
        .unwrap();
    let note = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let managed = w
        .approve_note_snapshot(Uuid::new_v4(), note.id, note.current_file_state.unwrap())
        .unwrap();
    drop(w);
    let (c, envelope) = run_json(data.path(), &["documents", "list"]);
    let docs = data_ok(c, &envelope, "managed list")["documents"]
        .as_array()
        .unwrap();
    let shadowed = docs
        .iter()
        .find(|d| d["source_id"] == imported.source_id.to_string())
        .unwrap();
    assert_eq!(shadowed["current_state"], "Shadowed");
    assert_eq!(shadowed["note_id"], note.id.to_string());
    assert!(shadowed["message"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(shadowed.get("sha256_hex").is_none());
    assert!(shadowed.get("content").is_none());
    let human = brn(&[
        "documents",
        "list",
        "--data-dir",
        data.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&human), 0);
    let human = text(&human);
    assert!(
        human.starts_with(&format!(
            "{} {} approved plan.md\n",
            managed.source_id, managed.version_id
        )),
        "{human}"
    );
    assert!(human.contains(&format!("{} Shadowed plan.md: ", imported.source_id)));
    assert!(!human.contains("oldterm"));
    let (c, envelope) = run_json(
        data.path(),
        &["documents", "show", &managed.source_id.to_string()],
    );
    let shown = data_ok(c, &envelope, "managed show");
    assert_eq!(shown["content"], "oldterm");
    assert_eq!(shown["current_state"], "Current");
    for source in [imported.source_id, managed.source_id] {
        if source == managed.source_id {
            fs::write(&file, "newterm").unwrap();
        }
        let (c, envelope) = run_json(data.path(), &["documents", "show", &source.to_string()]);
        assert_eq!(c, 1);
        assert_eq!(envelope["error"]["code"], "EVIDENCE_STALE");
        assert!(envelope.get("data").is_none());
    }
    let (_, envelope) = run_json(data.path(), &["documents", "list"]);
    let docs = envelope["data"]["documents"].as_array().unwrap();
    let changed = docs
        .iter()
        .find(|d| d["source_id"] == managed.source_id.to_string())
        .unwrap();
    assert_eq!(changed["current_state"], "Changed");
    assert!(changed["message"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(changed.get("version_id").is_none());
    let human = brn(&[
        "documents",
        "list",
        "--data-dir",
        data.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&human), 0);
    let human = text(&human);
    assert!(human.contains(&format!("{} Changed plan.md: ", managed.source_id)));
    assert!(!human.contains("oldterm"));
    assert!(!human.contains(&managed.version_id.to_string()));
    fs::remove_file(&file).unwrap();
    let (_, envelope) = run_json(data.path(), &["documents", "list"]);
    assert!(envelope["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["current_state"] == "Missing"));
    let human = brn(&[
        "documents",
        "list",
        "--data-dir",
        data.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&human), 0);
    let human = text(&human);
    assert!(human.contains(&format!("{} Missing plan.md: ", managed.source_id)));
    assert!(!human.contains("oldterm"));
    let (c, envelope) = run_json(
        data.path(),
        &["documents", "show", &managed.source_id.to_string()],
    );
    assert_eq!(c, 1);
    assert_eq!(envelope["error"]["code"], "EVIDENCE_STALE");
    assert!(envelope.get("data").is_none());
    let (c, envelope) = run_json(
        data.path(),
        &[
            "documents",
            "set-search-approval",
            &imported.source_id.to_string(),
            "--version-id",
            &imported.version_id.to_string(),
            "--state",
            "approved",
        ],
    );
    assert_eq!(c, 1);
    assert_eq!(envelope["error"]["code"], "EVIDENCE_STALE");
}

fn brn(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("brn binary runs")
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exit code")
}

fn text(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8")
}

/// Assert stdout holds exactly one JSON value and return it.
fn one_json(out: &Output) -> Value {
    serde_json::from_str(&text(out)).expect("exactly one JSON object on stdout")
}

/// Run brn with `--data-dir dir --json`; returns (exit code, envelope).
fn run_json(dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut all: Vec<&str> = args.to_vec();
    let dir_str = dir.to_str().unwrap();
    all.extend_from_slice(&["--data-dir", dir_str, "--json"]);
    let out = brn(&all);
    let envelope = one_json(&out);
    (code(&out), envelope)
}

/// Assert a success envelope and return its `data` object.
fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    &envelope["data"]
}

fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let file = dir.join(name);
    fs::write(&file, contents).unwrap();
    file
}

/// Seed one approved source and build the index; returns (source, version).
fn seed_built(root: &Path, name: &str, contents: &str) -> (String, String) {
    let file = write(root, name, contents);
    let path = file.to_str().unwrap();
    let (c, env) = run_json(root, &["import", path, "--approve-for-search"]);
    let data = data_ok(c, &env, "seed import");
    let source = data["source_id"].as_str().unwrap().to_string();
    let version = data["version_id"].as_str().unwrap().to_string();
    let (c, env) = run_json(root, &["index", "build"]);
    data_ok(c, &env, "seed build");
    (source, version)
}

#[test]
fn full_flow_import_approve_build_search() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = write(
        root,
        "note.md",
        "# Kanban\n\nThe zephyr engine hums quietly.\n",
    );
    let path = file.to_str().unwrap();

    // Default import is draft: approval is never implicit.
    let (c, env) = run_json(root, &["import", path]);
    let data = data_ok(c, &env, "import");
    assert_eq!(data["approval"], "draft");
    assert_eq!(data["changed"], true);
    assert!(data["operation_id"].as_str().is_some());
    let source = data["source_id"].as_str().unwrap().to_string();
    let version = data["version_id"].as_str().unwrap().to_string();

    let (c, env) = run_json(root, &["documents", "list"]);
    let data = data_ok(c, &env, "documents list");
    assert_eq!(data["documents"][0]["approval"], "draft");

    // No implicit index build: search before build fails INDEX_MISSING.
    let (c, env) = run_json(root, &["search", "zephyr"]);
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "INDEX_MISSING");
    assert!(env.get("data").is_none(), "no data on failure");

    let (c, env) = run_json(
        root,
        &[
            "documents",
            "set-search-approval",
            &source,
            "--version-id",
            &version,
            "--state",
            "approved",
        ],
    );
    let data = data_ok(c, &env, "set-search-approval");
    assert_eq!(data["state"], "approved");
    assert_eq!(data["source_id"], source);
    assert_eq!(data["version_id"], version);

    let (c, env) = run_json(root, &["index", "build"]);
    let data = data_ok(c, &env, "index build");
    assert!(!data["generation"].as_str().unwrap().is_empty());

    let (c, env) = run_json(root, &["search", "zephyr", "--profile", "keyword"]);
    let data = data_ok(c, &env, "search");
    assert_eq!(data["query"], "zephyr");
    assert_eq!(data["profile"], "keyword");
    let evidence = data["evidence"].as_array().unwrap();
    assert!(!evidence.is_empty());
    let first = &evidence[0];
    assert_eq!(first["source_id"], source);
    assert_eq!(first["version_id"], version);
    assert!(first["quote"].as_str().unwrap().contains("zephyr"));
    assert!(
        first["start_byte"].as_u64().unwrap() < first["end_byte"].as_u64().unwrap(),
        "{first}"
    );
    let hash = first["source_hash"].as_str().unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
}

#[test]
fn semantic_and_hybrid_are_unavailable_without_fallback() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed_built(root, "note.md", "synthetic content for retrieval\n");
    for profile in ["semantic", "hybrid"] {
        let (c, env) = run_json(root, &["search", "content", "--profile", profile]);
        assert_eq!(c, 1, "{profile}: {env}");
        assert_eq!(env["ok"], false, "{profile}");
        assert_eq!(env["error"]["code"], "PROFILE_UNAVAILABLE", "{profile}");
        assert!(
            env.get("data").is_none(),
            "no data at all on the failure envelope (no keyword fallback) for {profile}"
        );
    }
}

#[test]
fn stale_index_after_reimport_requires_rebuild() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = write(root, "doc.md", "original alpha content\n");
    let path = file.to_str().unwrap();
    let (c, env) = run_json(root, &["import", path, "--approve-for-search"]);
    data_ok(c, &env, "import");
    let (c, env) = run_json(root, &["index", "build"]);
    data_ok(c, &env, "build");
    let (c, env) = run_json(root, &["search", "alpha"]);
    data_ok(c, &env, "search before change");

    fs::write(&file, "revised alpha content\n").unwrap();
    let (c, env) = run_json(root, &["import", path, "--approve-for-search"]);
    data_ok(c, &env, "reimport");
    let (c, env) = run_json(root, &["search", "alpha"]);
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "INDEX_STALE");

    let (c, env) = run_json(root, &["index", "build"]);
    data_ok(c, &env, "rebuild");
    let (c, env) = run_json(root, &["search", "alpha"]);
    data_ok(c, &env, "search after rebuild");
}

#[test]
fn operation_id_idempotency_and_conflict() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = write(root, "same.md", "identical bytes\n");
    let path = file.to_str().unwrap();
    let op = Uuid::new_v4().to_string();

    // Establish the source, then import the unchanged file under an explicit
    // operation: recorded result is changed=false.
    let (c, first) = run_json(root, &["import", path]);
    let data = data_ok(c, &first, "establishing import");
    assert_eq!(data["changed"], true);
    let source = data["source_id"].as_str().unwrap().to_string();
    let version = data["version_id"].as_str().unwrap().to_string();

    let (c, second) = run_json(root, &["import", path, "--operation", &op]);
    let data = data_ok(c, &second, "explicit-operation import");
    assert_eq!(data["changed"], false);
    assert_eq!(data["source_id"], source);
    assert_eq!(data["version_id"], version);

    // Same operation + same file replays the recorded result unchanged.
    let (c, replay) = run_json(root, &["import", path, "--operation", &op]);
    let data = data_ok(c, &replay, "replayed import");
    assert_eq!(data["changed"], false);
    assert_eq!(data["source_id"], source);
    assert_eq!(data["version_id"], version);

    let other = write(root, "other.md", "different bytes\n");
    let (c, env) = run_json(
        root,
        &["import", other.to_str().unwrap(), "--operation", &op],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "OPERATION_CONFLICT");
}

#[test]
fn approval_binds_the_current_version_only() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = write(root, "doc.md", "first revision\n");
    let path = file.to_str().unwrap();
    let (c, env) = run_json(root, &["import", path]);
    let data = data_ok(c, &env, "first import");
    let source = data["source_id"].as_str().unwrap().to_string();
    let old_version = data["version_id"].as_str().unwrap().to_string();

    fs::write(&file, "second revision\n").unwrap();
    let (c, env) = run_json(root, &["import", path]);
    data_ok(c, &env, "second import");

    // The previous version is superseded: approval must not bind to it.
    let (c, env) = run_json(
        root,
        &[
            "documents",
            "set-search-approval",
            &source,
            "--version-id",
            &old_version,
            "--state",
            "approved",
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "NOT_FOUND");

    let missing = Uuid::new_v4().to_string();
    let (c, env) = run_json(
        root,
        &[
            "documents",
            "set-search-approval",
            &missing,
            "--version-id",
            &missing,
            "--state",
            "approved",
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "NOT_FOUND");
}

#[test]
fn withdrawn_source_fails_search() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (source, version) = seed_built(root, "note.md", "approvable body text\n");
    let (c, env) = run_json(root, &["search", "body"]);
    data_ok(c, &env, "search while approved");

    let (c, env) = run_json(
        root,
        &[
            "documents",
            "set-search-approval",
            &source,
            "--version-id",
            &version,
            "--state",
            "withdrawn",
        ],
    );
    data_ok(c, &env, "withdraw");
    let (c, env) = run_json(root, &["search", "body"]);
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    // Observed behavior: withdrawing approval stales the index, so the search
    // refuses with INDEX_STALE until a rebuild.
    assert_eq!(env["error"]["code"], "INDEX_STALE");
}

#[test]
fn index_build_rejects_extra_positionals_as_usage() {
    let out = brn(&["index", "build", "junk", "--json"]);
    assert_eq!(code(&out), 2, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "USAGE");
    assert_eq!(envelope["command"], "index.build");
}

#[test]
fn import_survives_spaces_unicode_and_crlf() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let nested = root.join("my notes");
    fs::create_dir(&nested).unwrap();
    let file = nested.join("ünïcode 名前.md");
    let contents = "héllo\r\nwörld — ünïcode ✓\r\n";
    fs::write(&file, contents).unwrap();
    let (c, env) = run_json(root, &["import", file.to_str().unwrap()]);
    let data = data_ok(c, &env, "import");
    let source = data["source_id"].as_str().unwrap().to_string();

    let (c, env) = run_json(root, &["documents", "show", &source]);
    let data = data_ok(c, &env, "documents show");
    assert_eq!(data["content"], contents);
    assert_eq!(data["title"], "ünïcode 名前.md");
}

#[test]
fn search_human_output_is_stdout_only() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (source, _) = seed_built(root, "note.md", "the zephyr engine hums\n");
    let out = brn(&["search", "zephyr", "--data-dir", root.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let stdout = text(&out);
    assert!(stdout.starts_with("keyword zephyr\n"), "{stdout}");
    assert!(stdout.contains(&source), "{stdout}");
    assert!(stdout.contains("\n  the zephyr"), "{stdout}");
    assert!(
        serde_json::from_str::<Value>(&stdout).is_err(),
        "human output is not JSON"
    );
}

#[test]
fn import_extension_rules() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let txt = write(root, "plain.txt", "plain text body\n");
    let (c, env) = run_json(root, &["import", txt.to_str().unwrap()]);
    data_ok(c, &env, "import .txt");

    let rs = write(root, "code.rs", "fn main() {}\n");
    let (c, env) = run_json(root, &["import", rs.to_str().unwrap()]);
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");
}
