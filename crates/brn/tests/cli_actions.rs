//! Actions CLI reads preserve the checked typed records and stop at apply fences.
use brn_store::{
    work::{
        actions::{ActionData, ActionPriority, ActionState},
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
    WorkStore,
};
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
    credentials: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let credentials = data.path().parent().unwrap().join("task.credentials");
        Self { data, credentials }
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

    fn ok(&self, args: &[&str], command: &str) -> Value {
        let (code, envelope) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {envelope}");
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["command"], command);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }
}

fn data(state: ActionState) -> ActionData {
    ActionData {
        title: "  Exact \"title\" λ\r\n\u{0007}\u{001b}  ".into(),
        description:
            "\u{feff}BOM then 日本語 and Eesti õ.\r\ncontrol:\t\u{0001}\u{007f}\u{0085}\u{009b}\r\n"
                .into(),
        state,
        owner: Some("  Zoë \"quoted\"\n\u{0000}  ".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2024-02-29".into()),
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(ActionPriority::High),
    }
}

fn create_action(f: &Fixture, title: &str, state: ActionState, settle: bool) -> Uuid {
    let id = Uuid::new_v4();
    let mut store = WorkStore::open(f.data.path()).unwrap().0;
    let proposal = store
        .create_proposal(&ProposalDraft {
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: title.into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id,
                data: data(state),
            }],
        })
        .unwrap();
    let operation = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: operation,
            expected: proposal.stamp(),
        })
        .unwrap();
    store.record_proposal_prepared(operation, &[]).unwrap();
    if settle {
        store
            .finish_proposal_apply(operation, ApplyOutcome::Applied, Some(&[]))
            .unwrap();
    }
    id
}

#[test]
fn list_filters_defaults_cursor_and_show_preserve_exact_typed_record_bytes() {
    let f = Fixture::new();
    let open = create_action(&f, "open", ActionState::Open, true);
    let waiting = create_action(&f, "waiting", ActionState::Waiting, true);
    let blocked = create_action(&f, "blocked", ActionState::Blocked, true);

    let all = f.ok(&["actions", "list"], "actions.list");
    assert_eq!(all["entries"].as_array().unwrap().len(), 3);
    assert_eq!(all["next_before"], Value::Null);
    let all_ids = all["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["origin"]["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert!(all_ids.contains(&open.to_string()));
    assert!(all_ids.contains(&waiting.to_string()));
    assert!(all_ids.contains(&blocked.to_string()));

    for (state, id) in [("open", open), ("waiting", waiting), ("blocked", blocked)] {
        let page = f.ok(&["actions", "list", "--state", state], "actions.list");
        assert_eq!(page["entries"].as_array().unwrap().len(), 1);
        assert_eq!(page["entries"][0]["origin"]["id"], id.to_string());
        assert_eq!(page["entries"][0]["data"]["state"], state);
    }
    let completed = f.ok(&["actions", "list", "--state", "completed"], "actions.list");
    assert_eq!(completed["entries"], json!([]));

    let first = f.ok(&["actions", "list", "--limit", "1"], "actions.list");
    let first_cursor = first["next_before"].clone();
    assert!(!first_cursor.is_null());
    let cursor_time = first_cursor["created_at_ms"].as_u64().unwrap().to_string();
    let cursor_id = first_cursor["id"].as_str().unwrap();
    let second = f.ok(
        &[
            "actions",
            "list",
            "--limit",
            "1",
            "--before-created-at-ms",
            &cursor_time,
            "--before-id",
            cursor_id,
        ],
        "actions.list",
    );
    assert_eq!(second["entries"].as_array().unwrap().len(), 1);
    assert_ne!(
        second["entries"][0]["origin"]["id"],
        first["entries"][0]["origin"]["id"]
    );

    let record = f.ok(&["actions", "show", &open.to_string()], "actions.show");
    let (store, _) = WorkStore::open(f.data.path()).unwrap();
    assert_eq!(
        record,
        serde_json::to_value(store.action(open).unwrap().unwrap()).unwrap()
    );
    drop(store);
    let human = f.process(&["actions", "show", &open.to_string()], false);
    assert!(human.status.success(), "{human:?}");
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("\\u0007"), "control character is quoted");
    assert!(human.contains("\\u001b"), "terminal escape is quoted");
    assert!(human.contains("\\u0000"), "NUL is quoted");
    assert!(!human.contains('\u{001b}'));
    assert!(human.contains("\\u007f"), "DEL is quoted");
    assert!(human.contains("\\u0085"), "C1 next-line is quoted");
    assert!(human.contains("\\u009b"), "C1 control sequence is quoted");
    assert!(!human.contains('\u{007f}'));
    assert!(!human.contains('\u{0085}'));
    assert!(!human.contains('\u{009b}'));
    assert_eq!(serde_json::from_str::<Value>(&human).unwrap(), record);
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[test]
fn missing_action_uses_the_typed_not_found_error() {
    let f = Fixture::new();
    let (code, envelope) = f.run(&["actions", "show", &Uuid::new_v4().to_string()]);
    assert_eq!(code, 1);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
    assert_eq!(envelope["error"]["message"], "Action does not exist");
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[test]
fn malformed_actions_refuse_before_operational_storage_or_credentials() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["actions"],
        vec!["actions", "unknown"],
        vec!["actions", "show"],
        vec!["actions", "show", &nil],
        vec!["actions", "show", &id, "extra"],
        vec!["actions", "list", "extra"],
        vec!["actions", "list", "--state", "unknown"],
        vec!["actions", "list", "--limit", "0"],
        vec!["actions", "list", "--limit", "201"],
        vec!["actions", "list", "--limit", "-1"],
        vec!["actions", "list", "--limit", "nope"],
        vec!["actions", "list", "--limit", "1", "--limit", "2"],
        vec!["actions", "list", "--before-created-at-ms", "1"],
        vec!["actions", "list", "--before-id", &id],
        vec![
            "actions",
            "list",
            "--before-created-at-ms",
            "-1",
            "--before-id",
            &id,
        ],
        vec![
            "actions",
            "list",
            "--before-created-at-ms",
            "9223372036854775808",
            "--before-id",
            &id,
        ],
        vec![
            "actions",
            "list",
            "--before-created-at-ms",
            "1",
            "--before-id",
            &nil,
        ],
        vec!["actions", "list", "--unknown"],
        vec!["actions", "dashboard", "extra"],
        vec!["actions", "dashboard", "--as-of", "2028-2-29"],
        vec!["actions", "dashboard", "--as-of", "2027-02-29"],
        vec!["actions", "dashboard", "--filter", "unknown"],
        vec!["actions", "dashboard", "--limit", "0"],
        vec!["actions", "dashboard", "--limit", "201"],
        vec![
            "actions",
            "dashboard",
            "--before-created-at-ms",
            "1",
            "--before-id",
            &id,
        ],
        vec![
            "actions",
            "dashboard",
            "--as-of",
            "2028-02-29",
            "--before-id",
            &id,
        ],
        vec![
            "actions",
            "dashboard",
            "--as-of",
            "2028-02-29",
            "--before-created-at-ms",
            "1",
            "--before-id",
            &nil,
        ],
        vec!["actions", "dashboard", "--state", "open"],
    ] {
        let (code, envelope) = f.run(&args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(f.data.path()).unwrap().count(), 0);
        assert!(!f.credentials.exists());
    }
}

#[test]
fn pending_action_proposal_fences_reads_without_applying_action_or_requesting_authority() {
    let f = Fixture::new();
    let id = create_action(&f, "pending", ActionState::Open, false);
    let (code, envelope) = f.run(&["actions", "list"]);
    assert_ne!(code, 0, "pending proposal must fence the read: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "SAVE_UNCERTAIN");
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);

    let (code, envelope) = f.run(&["actions", "dashboard"]);
    assert_ne!(code, 0);
    assert_eq!(envelope["error"]["code"], "SAVE_UNCERTAIN");

    let (store, _) = WorkStore::open(f.data.path()).unwrap();
    assert_eq!(store.action(id).unwrap(), None);
    let proposal = store
        .proposals(None)
        .unwrap()
        .into_iter()
        .find(|proposal| proposal.draft.title == "pending")
        .unwrap();
    assert_eq!(
        proposal.state,
        brn_store::work::proposals::ProposalState::Applying
    );
}

#[test]
fn dashboard_cli_preserves_global_counts_filtered_paging_dates_dependencies_and_safe_text() {
    let f = Fixture::new();
    let open = create_action(&f, "open", ActionState::Open, true);
    create_action(&f, "waiting", ActionState::Waiting, true);
    create_action(&f, "blocked", ActionState::Blocked, true);
    let input = [
        "actions",
        "dashboard",
        "--as-of",
        "2028-02-29",
        "--limit",
        "1",
    ];
    let page = f.ok(&input, "actions.dashboard");
    assert_eq!(page["as_of"], "2028-02-29");
    assert_eq!(
        page["counts"],
        json!({"open":1,"waiting":1,"blocked":1,"completed":0,"overdue":3,"follow_up":0})
    );
    assert_eq!(page["entries"].as_array().unwrap().len(), 1);
    assert!(page["entries"][0]["overdue"].as_bool().unwrap());
    assert_eq!(page["entries"][0]["dependencies"], json!([]));
    let cursor = page["next_before"].clone();
    let ms = cursor["created_at_ms"].as_u64().unwrap().to_string();
    let id = cursor["id"].as_str().unwrap();
    let next = f.ok(
        &[
            "actions",
            "dashboard",
            "--as-of",
            page["as_of"].as_str().unwrap(),
            "--limit",
            "1",
            "--before-created-at-ms",
            &ms,
            "--before-id",
            id,
        ],
        "actions.dashboard",
    );
    assert_eq!(next["counts"], page["counts"]);
    assert_ne!(
        next["entries"][0]["action"]["origin"]["id"],
        page["entries"][0]["action"]["origin"]["id"]
    );
    let filtered = f.ok(
        &[
            "actions",
            "dashboard",
            "--as-of",
            "2028-02-29",
            "--filter",
            "open",
        ],
        "actions.dashboard",
    );
    assert_eq!(filtered["counts"], page["counts"]);
    assert_eq!(
        filtered["entries"][0]["action"]["origin"]["id"],
        open.to_string()
    );
    let human = f.process(&input, false);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&human).unwrap(), page);
    for control in ['\u{001b}', '\u{007f}', '\u{0085}', '\u{009b}'] {
        assert!(!human.contains(control));
    }
    let defaults = f.ok(&["actions", "dashboard"], "actions.dashboard");
    assert_eq!(defaults["as_of"].as_str().unwrap().len(), 10);
    assert_eq!(defaults["entries"].as_array().unwrap().len(), 3);
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    assert!(!f.data.path().join("index.sqlite").exists());
}
