use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
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
            input: owner.path().join("request.json"),
            _owner: owner,
        }
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
            .args(["--json", "--data-dir"])
            .arg(&self.data)
            .output()
            .unwrap();
        let envelope = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("CLI omitted typed envelope: {output:?}"));
        (output.status.code().unwrap(), envelope)
    }
    fn input(&self, value: &Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn repair(&self, value: &Value) -> (i32, Value) {
        self.input(value);
        self.run(&[
            "proposals",
            "repair",
            "--file",
            self.input.to_str().unwrap(),
        ])
    }
    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(&self.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.data.with_file_name("data.credentials").exists());
    }
}
fn ok(result: (i32, Value), command: &str) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["schema_version"], 1);
    assert_eq!(result.1["command"], command);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

#[test]
fn malformed_repair_syntax_and_typed_inputs_refuse_before_workspace_startup() {
    let operation = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["proposals", "repair-preview"],
        vec!["proposals", "repair-preview", "not-a-uuid"],
        vec!["proposals", "repair-preview", &nil],
        vec!["proposals", "repair-preview", &operation, "extra"],
        vec![
            "proposals",
            "repair-preview",
            &operation,
            "--direction",
            "finish",
        ],
        vec!["proposals", "repair"],
        vec!["proposals", "repair", "extra", "--file", "missing.json"],
        vec![
            "proposals",
            "repair",
            "--file",
            "one.json",
            "--file",
            "two.json",
        ],
        vec!["proposals", "repair", "--operation", &operation],
    ] {
        let f = Fixture::new();
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{}", result.1);
        f.assert_unopened();
    }
    let request = json!({"id":Uuid::new_v4(),"operation_id":operation,"expected":vec![0;32],"direction":"finish"});
    let changed = |field: &str, value: Value| {
        let mut changed = request.clone();
        changed[field] = value;
        changed
    };
    for (value, code, category) in [
        (json!({}), 2, "USAGE"),
        (changed("unexpected", json!(true)), 2, "USAGE"),
        (changed("direction", json!("retry")), 2, "USAGE"),
        (changed("expected", json!(vec![0; 31])), 2, "USAGE"),
        (changed("expected", json!("not-a-hash")), 2, "USAGE"),
        (changed("id", json!("not-a-uuid")), 2, "USAGE"),
        (changed("id", json!(Uuid::nil())), 1, "AI_TOOL_REJECTED"),
        (
            changed("operation_id", json!(Uuid::nil())),
            1,
            "AI_TOOL_REJECTED",
        ),
        (changed("id", json!(operation)), 1, "AI_TOOL_REJECTED"),
    ] {
        let f = Fixture::new();
        let result = f.repair(&value);
        assert_eq!(result.0, code, "{}", result.1);
        assert_eq!(result.1["command"], "proposals.repair");
        assert_eq!(result.1["error"]["code"], category);
        f.assert_unopened();
    }
    for bytes in [&b"{"[..], &b"\xff"[..], &b""[..]] {
        let f = Fixture::new();
        fs::write(&f.input, bytes).unwrap();
        assert_eq!(
            f.run(&["proposals", "repair", "--file", f.input.to_str().unwrap()])
                .0,
            2
        );
        f.assert_unopened();
    }
    let f = Fixture::new();
    fs::File::create(&f.input)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        f.run(&["proposals", "repair", "--file", f.input.to_str().unwrap()])
            .0,
        2
    );
    f.assert_unopened();
}

#[test]
fn nonregular_repair_input_refuses_without_waiting_for_a_fifo_writer() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: NUL-terminated path in this fixture's exclusively owned directory.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["proposals", "repair", "--file"])
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
    assert!(!blocked, "repair input waited for a FIFO writer");
    assert_eq!(output.status.code(), Some(2));
    f.assert_unopened();
}

#[cfg(target_os = "macos")]
mod files {
    use super::*;
    use brn_store::{
        files::FileFingerprint,
        work::proposal_apply::{ApplyJournal, ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        WorkStore,
    };
    use std::{collections::BTreeMap, os::unix::fs::MetadataExt};

    const OLD: &str = "\u{feff}Original 日本語\r\n";
    const TRASH: &str = "\u{feff}Retained 🦀\r\n";
    const CREATED: &str = "\u{feff}Created λ\r\n";
    const REPLACED: &str = "\u{feff}Replacement 日本語\r\n";

    fn observed(f: &Fixture, path: &str) -> FileFingerprint {
        let output = ok(
            f.run(&["edit", "open", path, "--vault", f.vault.to_str().unwrap()]),
            "edit.open",
        );
        serde_json::from_value(output["observed"].clone()).unwrap()
    }
    fn snapshot(f: &Fixture) -> BTreeMap<String, (Vec<u8>, u64)> {
        fs::read_dir(&f.vault)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    (
                        fs::read(entry.path()).unwrap(),
                        entry.metadata().unwrap().ino(),
                    ),
                )
            })
            .collect()
    }
    fn review(f: &Fixture, journal: &ApplyJournal) -> Value {
        ok(
            f.run(&["proposals", "show", &journal.approved.draft.id.to_string()]),
            "proposals.show",
        )
    }
    fn preview(f: &Fixture, journal: &ApplyJournal) -> Value {
        ok(
            f.run(&[
                "proposals",
                "repair-preview",
                &journal.request.operation_id.to_string(),
            ]),
            "proposals.repair-preview",
        )
    }
    fn repair_request(journal: &ApplyJournal, preview: &Value, direction: &str) -> Value {
        json!({"id":Uuid::new_v4(),"operation_id":journal.request.operation_id,"expected":preview["expected"],"direction":direction})
    }
    fn read(f: &Fixture, journal: &ApplyJournal) -> ApplyJournal {
        let (store, _) = WorkStore::open(&f.data).unwrap();
        store
            .proposal_apply(journal.request.operation_id)
            .unwrap()
            .unwrap()
    }

    // Build a synthetic interrupted namespace directly from public Store DTOs
    // and real files. No private recovery envelope or production failpoint.
    fn prepared(mask: u64) -> (Fixture, ApplyJournal) {
        let f = Fixture::new();
        fs::write(f.vault.join("old.md"), OLD).unwrap();
        fs::write(f.vault.join("trash.md"), TRASH).unwrap();
        fs::write(f.vault.join("source.md"), "Synthetic evidence\r\n").unwrap();
        let old = observed(&f, "old.md");
        let trash = observed(&f, "trash.md");
        let source = observed(&f, "source.md");
        fs::write(f.vault.join("prepared-create.md"), CREATED).unwrap();
        fs::write(f.vault.join("prepared-replace.md"), REPLACED).unwrap();
        let new = observed(&f, "prepared-create.md");
        let replacement = observed(&f, "prepared-replace.md");
        let id = Uuid::new_v4();
        f.input(&json!({"id":id,"group_id":Uuid::new_v4(),"session_id":null,"title":"Repair full exact proposal",
            "changes":[{"kind":"create","path":"new.md","text":CREATED},
                {"kind":"replace","path":"old.md","expected":old,"text":REPLACED},
                {"kind":"trash","path":"trash.md","expected":trash}],
            "sources":[{"path":"source.md","fingerprint":source}]}));
        ok(
            f.run(&["proposals", "create", "--file", f.input.to_str().unwrap()]),
            "proposals.create",
        );
        f.input(&json!({"expected":{"id":id,"version":1},"comment":{"id":Uuid::new_v4(),"text":"Temporary 🦀\r\n","target":{"kind":"proposal"}}}));
        ok(
            f.run(&["proposals", "comment", "--file", f.input.to_str().unwrap()]),
            "proposals.comment",
        );
        let (mut store, _) = WorkStore::open(&f.data).unwrap();
        let journal = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: store.proposal(id).unwrap().unwrap().stamp(),
            })
            .unwrap();
        fs::rename(
            f.vault.join("prepared-create.md"),
            f.vault.join(&journal.members[0].staging),
        )
        .unwrap();
        fs::rename(
            f.vault.join("prepared-replace.md"),
            f.vault.join(&journal.members[1].staging),
        )
        .unwrap();
        let journal = store
            .record_proposal_prepared(
                journal.request.operation_id,
                &[new.clone(), replacement.clone(), trash.clone()],
            )
            .unwrap();
        if mask & 1 != 0 {
            fs::rename(
                f.vault.join(&journal.members[0].staging),
                f.vault.join("new.md"),
            )
            .unwrap();
        }
        if mask & 2 != 0 {
            let temporary = f.input.with_file_name("synthetic-swap");
            fs::rename(f.vault.join("old.md"), &temporary).unwrap();
            fs::rename(
                f.vault.join(&journal.members[1].staging),
                f.vault.join("old.md"),
            )
            .unwrap();
            fs::rename(&temporary, f.vault.join(&journal.members[1].staging)).unwrap();
        }
        if mask & 4 != 0 {
            fs::rename(
                f.vault.join("trash.md"),
                f.vault.join(&journal.members[2].staging),
            )
            .unwrap();
        }
        let proofs = [
            ApplyMemberProof {
                destination: (mask & 1 != 0).then_some(new.clone()),
                staging: (mask & 1 == 0).then_some(new),
            },
            ApplyMemberProof {
                destination: Some(if mask & 2 == 0 {
                    old.clone()
                } else {
                    replacement.clone()
                }),
                staging: Some(if mask & 2 == 0 { replacement } else { old }),
            },
            ApplyMemberProof {
                destination: (mask & 4 == 0).then_some(trash.clone()),
                staging: (mask & 4 != 0).then_some(trash),
            },
        ];
        store
            .finish_proposal_apply(
                journal.request.operation_id,
                ApplyOutcome::Uncertain,
                Some(&proofs),
            )
            .unwrap();
        let journal = store
            .proposal_apply(journal.request.operation_id)
            .unwrap()
            .unwrap();
        drop(store);
        (f, journal)
    }

    #[test]
    fn full_preview_keeps_exact_approved_payload_phases_review_and_files_unchanged() {
        let (f, journal) = prepared(2);
        let before = snapshot(&f);
        let review_before = review(&f, &journal);
        let preview = preview(&f, &journal);
        assert_eq!(
            preview["operation_id"],
            journal.request.operation_id.to_string()
        );
        assert_eq!(
            preview["approved"],
            serde_json::to_value(&journal.approved.draft).unwrap()
        );
        assert_eq!(preview["phases"], json!(["before", "applied", "before"]));
        assert_eq!(preview["expected"].as_array().unwrap().len(), 32);
        assert_eq!(preview["approved"]["changes"][0]["text"], CREATED);
        assert_eq!(preview["approved"]["changes"][1]["before_text"], OLD);
        assert_eq!(preview["approved"]["changes"][2]["before_text"], TRASH);
        assert_eq!(snapshot(&f), before);
        assert_eq!(review(&f, &journal), review_before);
        assert_eq!(read(&f, &journal), journal);
    }

    #[test]
    fn explicit_finish_restore_and_uuid_replay_preserve_exact_files_and_later_work() {
        for (mask, direction, outcome) in [(2, "finish", "applied"), (5, "restore", "not_applied")]
        {
            let (f, journal) = prepared(mask);
            let request = repair_request(&journal, &preview(&f, &journal), direction);
            let receipt = ok(f.repair(&request), "proposals.repair");
            assert_eq!(receipt["id"], request["id"]);
            assert_eq!(
                receipt["operation_id"],
                journal.request.operation_id.to_string()
            );
            assert_eq!(receipt["direction"], direction);
            assert_eq!(receipt["outcome"], outcome);
            let live = review(&f, &journal);
            assert_eq!(
                live["comments"].as_array().unwrap().is_empty(),
                direction == "finish"
            );
            if direction == "finish" {
                assert_eq!(
                    fs::read(f.vault.join("new.md")).unwrap(),
                    CREATED.as_bytes()
                );
                assert_eq!(
                    fs::read(f.vault.join("old.md")).unwrap(),
                    REPLACED.as_bytes()
                );
                assert!(!f.vault.join("trash.md").exists());
                assert_eq!(
                    fs::metadata(f.vault.join("old.md")).unwrap().ino(),
                    journal.prepared.as_ref().unwrap()[1].inode
                );
            } else {
                assert!(!f.vault.join("new.md").exists());
                assert_eq!(fs::read(f.vault.join("old.md")).unwrap(), OLD.as_bytes());
                assert_eq!(
                    fs::read(f.vault.join("trash.md")).unwrap(),
                    TRASH.as_bytes()
                );
                assert_eq!(
                    fs::metadata(f.vault.join("old.md")).unwrap().ino(),
                    match &journal.approved.draft.changes[1] {
                        brn_store::work::proposals::NoteChange::Replace { before, .. } =>
                            before.inode,
                        _ => unreachable!(),
                    }
                );
            }
            fs::write(f.vault.join("old.md"), "Later owner text 日本語\r\n").unwrap();
            let before = snapshot(&f);
            assert_eq!(ok(f.repair(&request), "proposals.repair"), receipt);
            assert_eq!(snapshot(&f), before);
            for (field, value) in [
                (
                    "direction",
                    json!(if direction == "finish" {
                        "restore"
                    } else {
                        "finish"
                    }),
                ),
                ("operation_id", json!(Uuid::new_v4())),
                ("expected", json!(vec![0; 32])),
            ] {
                let mut conflict = request.clone();
                conflict[field] = value;
                let refused = f.repair(&conflict);
                assert_eq!(refused.0, 1);
                assert_eq!(refused.1["error"]["code"], "OPERATION_CONFLICT");
                assert_eq!(snapshot(&f), before);
            }
        }
    }

    #[test]
    fn stale_capture_and_unknown_occupants_refuse_without_admission_or_overwrite() {
        let (f, journal) = prepared(2);
        let request = repair_request(&journal, &preview(&f, &journal), "finish");
        fs::rename(
            f.vault.join(&journal.members[0].staging),
            f.vault.join("new.md"),
        )
        .unwrap();
        let before = snapshot(&f);
        let refused = f.repair(&request);
        assert_eq!(refused.0, 1);
        assert_eq!(refused.1["error"]["code"], "CONTEXT_STALE");
        assert_eq!(snapshot(&f), before);
        assert!(read(&f, &journal).repair.is_none());

        let (f, journal) = prepared(2);
        let request = repair_request(&journal, &preview(&f, &journal), "restore");
        fs::write(
            f.vault.join("new.md"),
            "Unknown occupied destination 🦀\r\n",
        )
        .unwrap();
        let before = snapshot(&f);
        for refused in [
            f.run(&[
                "proposals",
                "repair-preview",
                &journal.request.operation_id.to_string(),
            ]),
            f.repair(&request),
        ] {
            assert_eq!(refused.0, 1);
            assert_eq!(refused.1["error"]["code"], "CONTEXT_STALE");
        }
        assert_eq!(snapshot(&f), before);
        assert!(read(&f, &journal).repair.is_none());
    }
}
