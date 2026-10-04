//! Ownership contention: the exclusive brn.owner.lock must be preserved and
//! surface as WORKSPACE_BUSY to a second process. No lock deletion anywhere.
use brn_store::WorkStore;
use serde_json::Value;
use std::process::{Command, Output, Stdio};
mod support;

fn brn_status(dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["status", "--data-dir", dir.to_str().unwrap(), "--json"])
        .stdin(Stdio::null())
        .output()
        .expect("brn binary runs")
}

#[test]
fn busy_workspace_reports_workspace_busy_and_lock_survives() {
    let dir = support::data_dir();
    // Hold the exclusive ownership lock in-process for the whole subprocess call.
    let (workspace, _) = WorkStore::open(dir.path()).unwrap();
    let out = brn_status(dir.path());
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let envelope: Value =
        serde_json::from_str(&String::from_utf8(out.stdout).expect("UTF-8 stdout"))
            .expect("exactly one JSON object on stdout");
    assert_eq!(envelope["command"], "status");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "WORKSPACE_BUSY");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("already owned"),
        "{}",
        envelope["error"]["message"]
    );
    drop(workspace);
    // The lock was never deleted: once released, the workspace opens normally.
    let after = brn_status(dir.path());
    assert_eq!(after.status.code(), Some(0));
}
