use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            data,
            vault,
            input: owner.path().join("input.json"),
            _owner: owner,
        }
    }
    fn input(&self, value: &Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        run(&self.data, args)
    }
    fn write(&self, sub: &str) -> (i32, Value) {
        self.run(&[
            "proposals",
            sub,
            "--file",
            self.input.to_str().unwrap(),
            "--vault",
            self.vault.to_str().unwrap(),
        ])
    }
}
fn run(data: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--json", "--data-dir"])
        .arg(data)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{out:?}")),
    )
}
#[cfg(target_os = "macos")]
fn ok(result: (i32, Value)) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

#[cfg(target_os = "macos")]
#[test]
fn review_across_processes_preserves_comments_newer_work_and_vault_bytes() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let group = Uuid::new_v4();
    let text = "\u{feff}Draft λ\r\n";
    let input = json!({"id":id, "group_id":group, "session_id":null,
        "title":"Review", "changes":[{"kind":"create","path":"new.md","text":text}], "sources":[]});
    f.input(&input);
    let created = ok(f.write("create"));
    assert_eq!(created["version"], 1);
    assert_eq!(created["draft"]["changes"][0]["text"], text);
    assert!(!f.vault.join("new.md").exists());
    let comment_id = Uuid::new_v4();
    f.input(
        &json!({"expected":{"id":id,"version":1},"comment":{"id":comment_id,"text":"Keep λ",
        "target":{"kind":"text","anchor":{"change_index":0,"start":9,"end":11,"quote":"λ"}}}}),
    );
    let commented = ok(f.write("comment"));
    assert_eq!(commented["version"], 2);
    f.input(&json!({"expected":{"id":id,"version":1},"title":"Late Rewrite","texts":["late"]}));
    let late = f.write("rewrite-result");
    assert_eq!(late.0, 1);
    assert_eq!(late.1["error"]["code"], "CONTEXT_STALE");
    f.input(&json!({"expected":{"id":id,"version":2},"title":"My review","texts":["\u{feff}Draft new λ\r\n"]}));
    let edited = ok(f.write("edit"));
    assert_eq!(edited["version"], 3);
    assert_eq!(edited["comments"][0]["target"]["kind"], "unresolved");
    let list = ok(f.run(&["proposals", "list", "--group", &group.to_string()]));
    assert_eq!(list, json!([edited.clone()]));
    f.input(&input);
    assert_eq!(ok(f.write("create")), edited);
    let rejected = ok(f.run(&[
        "proposals",
        "reject",
        &id.to_string(),
        "--review-version",
        "3",
    ]));
    assert_eq!(rejected["version"], 4);
    assert_eq!(rejected["state"], "rejected");
    assert_eq!(rejected["comments"], edited["comments"]);
    assert_eq!(ok(f.run(&["proposals", "show", &id.to_string()])), rejected);
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
}

#[test]
fn invalid_typed_inputs_and_incomplete_approval_fail_before_workspace_open() {
    for input in [
        json!({"id":"not a uuid"}),
        json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,
        "title":"Review","changes":[{"kind":"create","path":"../escape.md","text":"x"}],"sources":[]}),
        json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,
        "title":"Review","changes":[{"kind":"create","path":"new.md","text":"x","extra":"no"}],"sources":[]}),
    ] {
        let f = Fixture::new();
        f.input(&input);
        assert_ne!(f.write("create").0, 0);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
    let f = Fixture::new();
    assert_eq!(
        f.run(&["proposals", "approve", &Uuid::new_v4().to_string()])
            .0,
        2
    );
    assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
}

#[test]
fn invalid_predecessor_requests_refuse_before_workspace_open() {
    let expected = json!({"id": Uuid::new_v4(), "version": 1});
    for input in [
        json!({"expected": expected, "predecessor_path":"../outside.md"}),
        json!({"expected": expected, "predecessor_path":".hidden.md"}),
        json!({"expected": expected, "predecessor_path":"archive/old.md"}),
        json!({"expected":{"id":Uuid::nil(),"version":1},"predecessor_path":"old.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":0},"predecessor_path":"old.md"}),
        json!({"expected": expected, "predecessor_path":"old.md", "refresh_evidence":true}),
    ] {
        let f = Fixture::new();
        f.input(&input);
        let result = f.write("attach-predecessor");
        assert_ne!(result.0, 0, "{}", result.1);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn predecessor_cli_roundtrip_preserves_review_replays_creation_and_applies_revised_pair() {
    use brn_store::{
        note_identity, note_provenance,
        work::{
            inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
            inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
            inbox_processing::InboxConversionFormat,
            inbox_source::InboxSourceBinding,
        },
        WorkStore,
    };
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let predecessor_id = Uuid::new_v4();
    let note_id = Uuid::new_v4();
    let analysis_id = Uuid::new_v4();
    let body = "Blue õ was selected.\r\n";
    // Seed a clearly synthetic saved Source/capture; every reviewed operation
    // and effect below uses a new ordinary CLI process, with no provider work.
    let source_binding = InboxSourceBinding {
        extraction: None,
        visual: None,
        batch_id: Uuid::new_v4(),
        index: 0,
        original: InboxItem {
            capture: InboxCapture {
                id: Uuid::new_v4(),
                kind: InboxKind::Markdown,
                title: "Synthetic CLI predecessor evidence".into(),
                original_name: Some("synthetic.md".into()),
                copy: InboxCopy {
                    directory: f._owner.path().join("synthetic-copy"),
                    directory_device: 1,
                    directory_inode: 2,
                    file_device: 1,
                    file_inode: 3,
                    byte_len: body.len() as u64,
                    sha256: brn_intake::digest(body.as_bytes()),
                },
            },
            received_at_ms: 1,
        },
        format: InboxConversionFormat::VerbatimMarkdownV1,
        byte_len: body.len() as u64,
        sha256: brn_intake::digest(body.as_bytes()),
        note_id: source_id,
    };
    let source_text = source_binding.markdown(body).unwrap();
    let predecessor_text =
        note_identity::assign("Approved prior choice\r\n", predecessor_id).unwrap();
    fs::write(f.vault.join("source.md"), &source_text).unwrap();
    fs::write(f.vault.join("previous.md"), &predecessor_text).unwrap();
    let proof = ok(f.run(&[
        "proposals",
        "source",
        "source.md",
        "--vault",
        f.vault.to_str().unwrap(),
    ]));
    let (mut store, _) = WorkStore::open(&f.data).unwrap();
    let job = store
        .reserve_inbox_action(
            &InboxActionCapture {
                intake: None,
                visual_asset: None,
                purpose: InboxAnalysisPurpose::KnowledgeAndActions,
                id: analysis_id,
                conversation: None,
                source: Some(serde_json::from_value(proof["source"].clone()).unwrap()),
                source_text: source_text.clone(),
                provider: "chatgpt".into(),
                model: "gpt-6-luna".into(),
                effort: "medium".into(),
            },
            "Synthetic retained interpretation; no inference",
        )
        .unwrap();
    drop(store);
    let quote = "Blue õ";
    let start = source_text.find(quote).unwrap();
    let citation = note_provenance::VaultCitation {
        note_id: source_id,
        sha256: brn_intake::digest(source_text.as_bytes()),
        start_byte: start,
        end_byte: start + quote.len(),
        quote: quote.into(),
    };
    let text = note_provenance::write(
        &note_identity::assign("Owner-reviewed Blue õ choice\r\n", note_id).unwrap(),
        std::slice::from_ref(&citation),
    )
    .unwrap();
    let proposal_id = Uuid::new_v4();
    let creation = json!({"id":proposal_id,"group_id":analysis_id,"session_id":null,
        "title":"Synthetic supplemental review","changes":[{"kind":"create","path":"choice.md","text":text}],"sources":[proof["source"].clone()],
        "inbox_knowledge":{"analysis_id":job.capture.id,"note_id":note_id,"source":proof["source"].clone(),"citations":[citation]}});
    f.input(&creation);
    let original = ok(f.write("create"));
    f.input(&json!({"expected":{"id":proposal_id,"version":1},"comment":{"id":Uuid::new_v4(),"text":"Preserve owner wording","target":{"kind":"proposal"}}}));
    let commented = ok(f.write("comment"));
    f.input(&json!({"expected":{"id":proposal_id,"version":2},"predecessor_path":"previous.md"}));
    let attached = ok(f.write("attach-predecessor"));
    assert_eq!(attached["version"], 3);
    assert_eq!(attached["comments"], commented["comments"]);
    assert_eq!(attached["draft"]["changes"].as_array().unwrap().len(), 2);
    assert!(attached["draft"]["changes"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with(&text));
    assert_eq!(
        fs::read(f.vault.join("previous.md")).unwrap(),
        predecessor_text.as_bytes()
    );
    assert!(!f.vault.join("choice.md").exists());
    f.input(&creation);
    assert_eq!(ok(f.write("create")), attached);
    let stale = f.run(&[
        "proposals",
        "approve",
        &proposal_id.to_string(),
        "--review-version",
        original["version"].to_string().as_str(),
        "--operation",
        &Uuid::new_v4().to_string(),
    ]);
    assert_eq!(stale.1["error"]["code"], "CONTEXT_STALE");
    let operation = Uuid::new_v4();
    let args = [
        "proposals",
        "approve",
        &proposal_id.to_string(),
        "--review-version",
        "3",
        "--operation",
        &operation.to_string(),
    ];
    let applied = ok(f.run(&args));
    assert_eq!(applied["outcome"], "applied");
    assert_eq!(ok(f.run(&args)), applied);
    assert_eq!(
        fs::read(f.vault.join("previous.md")).unwrap(),
        brn_store::note_metadata::to_history(&predecessor_text)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        source_text.as_bytes()
    );
}

#[test]
fn asset_paths_and_noncanonical_payloads_refuse_before_workspace_open() {
    for path in [
        "../escape.bin",
        ".hidden.bin",
        "note.md",
        "archive/old.bin",
        "a//b.bin",
    ] {
        let f = Fixture::new();
        let result = f.run(&["proposals", "asset", path]);
        assert_eq!(result.0, 2, "{}", result.1);
        assert_eq!(result.1["command"], "proposals.asset");
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
    }
    for bytes in [
        json!([0, 255]),
        json!("AP8BgA"),
        json!("AP8BgA==="),
        json!("AP8B gA=="),
        json!("AP8BgB=="),
    ] {
        let f = Fixture::new();
        f.input(
            &json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,"title":"Assets",
            "changes":[{"kind":"create_asset","path":"asset.bin","bytes":bytes}],"sources":[]}),
        );
        assert_ne!(f.write("create").0, 0);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
    let f = Fixture::new();
    let file = fs::File::create(&f.input).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    assert_ne!(f.write("create").0, 0);
    assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
fn asset(f: &Fixture, path: &str) -> Value {
    let proof = ok(f.run(&[
        "proposals",
        "asset",
        path,
        "--vault",
        f.vault.to_str().unwrap(),
    ]));
    assert_eq!(proof["path"], path);
    assert_eq!(proof.as_object().unwrap().len(), 2);
    assert!(proof.get("bytes").is_none() && proof.get("text").is_none());
    assert!(proof["fingerprint"]["device"].as_u64().unwrap() > 0);
    assert!(proof["fingerprint"]["inode"].as_u64().unwrap() > 0);
    proof["fingerprint"].clone()
}

#[cfg(target_os = "macos")]
#[test]
fn asset_create_fresh_proof_replace_trash_and_exact_undo_use_the_real_worker() {
    let f = Fixture::new();
    fs::create_dir(f.vault.join("assets")).unwrap();
    let path = "assets/exact.bin";
    let id = Uuid::new_v4();
    f.input(
        &json!({"id":id,"group_id":null,"session_id":null,"title":"Mixed assets",
        "changes":[{"kind":"create_asset","path":path,"bytes":"AP8BgA=="},
            {"kind":"create","path":"note.md","text":"Original note λ\r\n"}],"sources":[]}),
    );
    let created = ok(f.write("create"));
    assert_eq!(created["draft"]["changes"][0]["bytes"], "AP8BgA==");
    assert!(!f.vault.join(path).exists() && !f.vault.join("note.md").exists());
    f.input(&json!({"expected":{"id":id,"version":1},"title":"Forged text edit","texts":["opaque bytes","note"]}));
    assert_ne!(f.write("edit").0, 0);
    f.input(&json!({"expected":{"id":id,"version":1},"title":"Owner title","texts":[null,"Owner note λ\r\n"]}));
    let edited = ok(f.write("edit"));
    assert_eq!(edited["version"], 2);
    assert_eq!(
        edited["draft"]["changes"][0],
        created["draft"]["changes"][0]
    );
    let create = Uuid::new_v4();
    assert_eq!(ok(approve(&f, id, 2, create))["outcome"], "applied");
    assert_eq!(fs::read(f.vault.join(path)).unwrap(), [0, 255, 1, 128]);
    assert_eq!(
        fs::read(f.vault.join("note.md")).unwrap(),
        "Owner note λ\r\n".as_bytes()
    );
    let original = asset(&f, path);
    assert_eq!(original["len"], 4);

    let inverse = Uuid::new_v4();
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo",
            &create.to_string(),
            "--operation",
            &inverse.to_string()
        ]))["outcome"],
        "applied"
    );
    assert!(!f.vault.join(path).exists() && !f.vault.join("note.md").exists());
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo",
            &inverse.to_string(),
            "--operation",
            &Uuid::new_v4().to_string()
        ]))["outcome"],
        "applied"
    );
    assert_eq!(asset(&f, path), original);
    assert_eq!(fs::read(f.vault.join(path)).unwrap(), [0, 255, 1, 128]);

    let replacement = Uuid::new_v4();
    f.input(&json!({"id":replacement,"group_id":null,"session_id":null,"title":"Replace asset",
        "changes":[{"kind":"replace_asset","path":path,"expected":original,"bytes":"/gB/"}],"sources":[]}));
    let captured = ok(f.write("create"));
    assert_eq!(captured["draft"]["changes"][0]["before_bytes"], "AP8BgA==");
    let replace = Uuid::new_v4();
    assert_eq!(
        ok(approve(&f, replacement, 1, replace))["outcome"],
        "applied"
    );
    assert_eq!(fs::read(f.vault.join(path)).unwrap(), [254, 0, 127]);
    let replaced = asset(&f, path);
    assert_ne!(replaced, original);
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo",
            &replace.to_string(),
            "--operation",
            &Uuid::new_v4().to_string()
        ]))["outcome"],
        "applied"
    );
    assert_eq!(asset(&f, path), original);

    let trash = Uuid::new_v4();
    f.input(
        &json!({"id":trash,"group_id":null,"session_id":null,"title":"Trash asset",
        "changes":[{"kind":"trash_asset","path":path,"expected":original}],"sources":[]}),
    );
    let captured = ok(f.write("create"));
    assert_eq!(captured["draft"]["changes"][0]["before_bytes"], "AP8BgA==");
    let moved = Uuid::new_v4();
    assert_eq!(ok(approve(&f, trash, 1, moved))["outcome"], "applied");
    assert!(!f.vault.join(path).exists());
    let restore = Uuid::new_v4();
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "restore-trash",
            &moved.to_string(),
            "--member",
            "0",
            "--operation",
            &restore.to_string()
        ]))["outcome"],
        "applied"
    );
    assert_eq!(asset(&f, path), original);
    assert_eq!(fs::read(f.vault.join(path)).unwrap(), [0, 255, 1, 128]);
    let restored = asset(&f, path);
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "restore-trash",
            &moved.to_string(),
            "--member",
            "0",
            "--operation",
            &restore.to_string()
        ]))["outcome"],
        "applied"
    );
    assert_eq!(asset(&f, path), restored);
}

#[cfg(target_os = "macos")]
fn observe(f: &Fixture, path: &str) -> Value {
    let result = ok(f.run(&["edit", "open", path, "--vault", f.vault.to_str().unwrap()]));
    assert!(result["observed"].is_object());
    result["observed"].clone()
}

#[cfg(target_os = "macos")]
fn approve(f: &Fixture, id: Uuid, version: u64, operation: Uuid) -> (i32, Value) {
    f.run(&[
        "proposals",
        "approve",
        &id.to_string(),
        "--review-version",
        &version.to_string(),
        "--operation",
        &operation.to_string(),
    ])
}

#[cfg(target_os = "macos")]
#[test]
fn exact_approved_member_set_replays_and_reconciles_across_processes_without_more_writes() {
    let f = Fixture::new();
    let old = "\u{feff}Old 日本語\r\n";
    let trash = "\u{feff}Trash 🦀\r\n";
    let new = "\u{feff}New λ\r\n";
    let replacement = "\u{feff}Replacement 日本語\r\n";
    fs::write(f.vault.join("old.md"), old).unwrap();
    fs::write(f.vault.join("trash.md"), trash).unwrap();
    fs::write(f.vault.join("source.md"), "source 🦀\r\n").unwrap();
    let before = observe(&f, "old.md");
    let before_trash = observe(&f, "trash.md");
    let source = observe(&f, "source.md");
    let id = Uuid::new_v4();
    let operation = Uuid::new_v4();
    f.input(
        &json!({"id":id,"group_id":null,"session_id":null,"title":"Exact bytes",
        "changes":[{"kind":"create","path":"new.md","text":new},
            {"kind":"replace","path":"old.md","expected":before,"text":replacement},
            {"kind":"trash","path":"trash.md","expected":before_trash}],
        "sources":[{"path":"source.md","fingerprint":source}]}),
    );
    let created = ok(f.write("create"));
    assert_eq!(created["version"], 1);
    f.input(&json!({"expected":{"id":id,"version":1},"comment":{
        "id":Uuid::new_v4(),"text":"Temporary review","target":{"kind":"proposal"}}}));
    let commented = ok(f.write("comment"));
    assert_eq!(commented["version"], 2);
    let receipt = ok(approve(&f, id, 2, operation));
    assert_eq!(receipt["operation_id"], operation.to_string());
    assert_eq!(receipt["proposal_id"], id.to_string());
    assert_eq!(receipt["approved_version"], 2);
    assert_eq!(receipt["stamp"]["version"], 4);
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(fs::read(f.vault.join("new.md")).unwrap(), new.as_bytes());
    assert_eq!(
        fs::read(f.vault.join("old.md")).unwrap(),
        replacement.as_bytes()
    );
    assert!(!f.vault.join("trash.md").exists());
    let live = ok(f.run(&["proposals", "show", &id.to_string()]));
    assert_eq!(live["state"], "applied");
    assert_eq!(live["comments"], json!([]));
    let journals = ok(f.run(&["proposals", "applies"]));
    assert_eq!(journals.as_array().unwrap().len(), 1);
    assert_eq!(journals[0]["receipt"], receipt);
    assert_eq!(journals[0]["approved"]["comments"], json!([]));
    // A later external edit demonstrates that neither exact replay nor explicit
    // reconciliation installs the approved text a second time.
    fs::write(f.vault.join("new.md"), "Later external text\r\n").unwrap();
    assert_eq!(ok(approve(&f, id, 2, operation)), receipt);
    assert_eq!(
        ok(f.run(&["proposals", "reconcile", &operation.to_string()])),
        receipt
    );
    assert_eq!(
        fs::read(f.vault.join("new.md")).unwrap(),
        b"Later external text\r\n"
    );
    assert_eq!(ok(f.run(&["proposals", "applies"])), journals);
    let conflict = approve(&f, id, 4, operation);
    assert_eq!(conflict.0, 1);
    assert_eq!(conflict.1["error"]["code"], "OPERATION_CONFLICT");
}

#[cfg(target_os = "macos")]
#[test]
fn changed_evidence_refuses_preflight_and_preserves_comments_and_external_bytes() {
    let f = Fixture::new();
    fs::write(f.vault.join("source.md"), "Original source\r\n").unwrap();
    let source = observe(&f, "source.md");
    let id = Uuid::new_v4();
    let operation = Uuid::new_v4();
    f.input(
        &json!({"id":id,"group_id":null,"session_id":null,"title":"Guard source",
        "changes":[{"kind":"create","path":"first.md","text":"Proposed 日本語\r\n"}],
        "sources":[{"path":"source.md","fingerprint":source}]}),
    );
    ok(f.write("create"));
    f.input(&json!({"expected":{"id":id,"version":1},"comment":{
        "id":Uuid::new_v4(),"text":"Retain refusal review","target":{"kind":"proposal"}}}));
    let commented = ok(f.write("comment"));
    let stale = approve(&f, id, 1, Uuid::new_v4());
    assert_eq!(stale.0, 1);
    assert_eq!(stale.1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(ok(f.run(&["proposals", "applies"])), json!([]));
    fs::write(f.vault.join("source.md"), "Changed source 🦀\r\n").unwrap();
    let failure = approve(&f, id, 2, operation);
    assert_eq!(failure.0, 1);
    assert_eq!(failure.1["error"]["code"], "CONTEXT_STALE");
    assert!(!f.vault.join("first.md").exists());
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        "Changed source 🦀\r\n".as_bytes()
    );
    let journals = ok(f.run(&["proposals", "applies"]));
    assert_eq!(journals, json!([]));
    let current = ok(f.run(&["proposals", "show", &id.to_string()]));
    assert_eq!(current["state"], "draft");
    assert_eq!(current, commented);
    let repeated = approve(&f, id, 2, operation);
    assert_eq!(repeated.0, 1);
    assert_eq!(repeated.1["error"]["code"], "CONTEXT_STALE");
    let absent = f.run(&["proposals", "reconcile", &operation.to_string()]);
    assert_eq!(absent.0, 1);
    assert_eq!(absent.1["error"]["code"], "NOT_FOUND");
    assert!(!f.vault.join("first.md").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn captured_group_stops_on_conflict_and_never_admits_omitted_or_later_members() {
    let f = Fixture::new();
    let group = Uuid::new_v4();
    let ids: Vec<_> = (0..4).map(|_| Uuid::new_v4()).collect();
    let operations: Vec<_> = (0..3).map(|_| Uuid::new_v4()).collect();
    let paths = ["first.md", "blocked.md", "third.md", "new-arrival.md"];
    for (id, path) in ids.iter().zip(paths).take(3) {
        f.input(&json!({"id":id,"group_id":group,"session_id":null,"title":"Captured member",
            "changes":[{"kind":"create","path":path,"text":format!("Approved {path} 日本語\r\n")}],"sources":[]}));
        ok(f.write("create"));
    }
    fs::write(f.vault.join("blocked.md"), "External occupant 🦀\r\n").unwrap();
    let approvals: Vec<_> = ids
        .iter()
        .zip(&operations)
        .map(|(id, operation)| json!({"operation_id":operation,"expected":{"id":id,"version":1}}))
        .collect();
    let captured = json!({"group_id":group,"approvals":approvals});
    f.input(
        &json!({"id":ids[3],"group_id":group,"session_id":null,"title":"Later arrival",
        "changes":[{"kind":"create","path":paths[3],"text":"New arrival"}],"sources":[]}),
    );
    ok(f.write("create"));
    f.input(&captured);
    let result = ok(f.write("approve-group"));
    assert_eq!(result["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(result["receipts"][0]["outcome"], "applied");
    assert_eq!(result["stopped"]["operation_id"], operations[1].to_string());
    assert_eq!(
        fs::read(f.vault.join("first.md")).unwrap(),
        "Approved first.md 日本語\r\n".as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("blocked.md")).unwrap(),
        "External occupant 🦀\r\n".as_bytes()
    );
    for index in [2, 3] {
        assert!(!f.vault.join(paths[index]).exists());
        let record = ok(f.run(&["proposals", "show", &ids[index].to_string()]));
        assert_eq!(record["version"], 1);
        assert_eq!(record["state"], "draft");
    }
    assert_eq!(
        ok(f.run(&["proposals", "applies"]))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let replay = ok(f.write("approve-group"));
    assert_eq!(replay["receipts"], result["receipts"]);
    assert_eq!(replay["stopped"]["operation_id"], operations[1].to_string());
    assert!(!f.vault.join("third.md").exists());
    assert!(!f.vault.join("new-arrival.md").exists());
}

#[test]
fn malformed_approval_inputs_are_usage_errors_before_workspace_admission() {
    let id = Uuid::new_v4().to_string();
    let operation = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["proposals", "approve", &id, "--review-version", "1"],
        vec![
            "proposals",
            "approve",
            &id,
            "--review-version",
            "0",
            "--operation",
            &operation,
        ],
        vec![
            "proposals",
            "approve",
            &nil,
            "--review-version",
            "1",
            "--operation",
            &operation,
        ],
        vec![
            "proposals",
            "approve",
            &id,
            "--review-version",
            "1",
            "--operation",
            &nil,
        ],
        vec!["proposals", "reconcile", "not-a-uuid"],
        vec!["proposals", "reconcile", &nil],
        vec!["proposals", "applies", "extra"],
    ] {
        let f = Fixture::new();
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{}", result.1);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
    }
    for input in [
        json!({"group_id":Uuid::new_v4(),"approvals":[],"unexpected":true}),
        json!({"group_id":Uuid::new_v4(),"approvals":[{"operation_id":Uuid::new_v4(),
            "expected":{"id":Uuid::new_v4(),"version":1},"unexpected":true}]}),
    ] {
        let f = Fixture::new();
        f.input(&input);
        let result = f.write("approve-group");
        assert_eq!(result.0, 2, "{}", result.1);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
    }
}

#[test]
fn invalid_captured_groups_refuse_before_worker_startup() {
    let group = Uuid::new_v4();
    let first = json!({"operation_id":Uuid::new_v4(),
        "expected":{"id":Uuid::new_v4(),"version":1}});
    let second = json!({"operation_id":Uuid::new_v4(),
        "expected":{"id":Uuid::new_v4(),"version":1}});
    let mut cases = vec![
        json!({"group_id":group,"approvals":[]}),
        json!({"group_id":Uuid::nil(),"approvals":[first.clone()]}),
    ];
    for path in [
        vec!["operation_id"],
        vec!["expected", "id"],
        vec!["expected", "version"],
    ] {
        let mut invalid = first.clone();
        let mut field = &mut invalid;
        for key in &path {
            field = &mut field[*key];
        }
        *field = if path.last() == Some(&"version") {
            json!(0)
        } else {
            json!(Uuid::nil())
        };
        cases.push(json!({"group_id":group,"approvals":[invalid]}));
    }
    let mut duplicate_id = second.clone();
    duplicate_id["expected"]["id"] = first["expected"]["id"].clone();
    cases.push(json!({"group_id":group,"approvals":[first.clone(),duplicate_id]}));
    let mut duplicate_operation = second;
    duplicate_operation["operation_id"] = first["operation_id"].clone();
    cases.push(json!({"group_id":group,"approvals":[first,duplicate_operation]}));
    let excessive: Vec<_> = (0..65)
        .map(|_| {
            json!({"operation_id":Uuid::new_v4(),
        "expected":{"id":Uuid::new_v4(),"version":1}})
        })
        .collect();
    cases.push(json!({"group_id":group,"approvals":excessive}));
    for invalid in cases {
        let f = Fixture::new();
        f.input(&invalid);
        let result = f.write("approve-group");
        assert_eq!(result.0, 1, "{}", result.1);
        assert_eq!(result.1["ok"], false);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
}

#[test]
fn nonregular_proposal_input_cannot_block_before_admission() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: a valid NUL-terminated path in this test's exclusive synthetic folder.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["proposals", "create", "--file"])
        .arg(&f.input)
        .args(["--json", "--data-dir"])
        .arg(&f.data)
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
    assert!(!blocked, "preparation blocked on a nonregular input");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn failed_creation_does_not_store_a_partial_member_set_or_overwrite_an_occupant() {
    let f = Fixture::new();
    fs::write(f.vault.join("occupied.md"), "External bytes\r\n").unwrap();
    f.input(
        &json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,"title":"Two members",
        "changes":[{"kind":"create","path":"first.md","text":"First"},
                   {"kind":"create","path":"occupied.md","text":"Would overwrite"}],"sources":[]}),
    );
    let failed = f.write("create");
    assert_eq!(failed.0, 1);
    assert_eq!(failed.1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(ok(f.run(&["proposals", "list"])), json!([]));
    assert!(!f.vault.join("first.md").exists());
    assert_eq!(
        fs::read_to_string(f.vault.join("occupied.md")).unwrap(),
        "External bytes\r\n"
    );
}
