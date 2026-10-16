//! Process checks of contract 2 discovery: the envelope shape, per-command
//! help, global options, pre-open validation and missing workspaces.
mod support;
use serde_json::{json, Value};
use support::*;

fn contract_2(envelope: &Value) {
    assert_eq!(envelope["contract"], 2, "{envelope}");
    let object = envelope.as_object().unwrap();
    assert!(object.contains_key("page"), "{envelope}");
    assert!(object.contains_key("stop"), "{envelope}");
    assert_eq!(object.len(), 15, "{envelope}");
}

#[test]
fn every_envelope_has_contract_2_page_and_stop() {
    let root = Root::new();
    let data = root.join("data");
    let created = call(
        &root,
        &data,
        &["note-create", "--title", "T", "--text", "x"],
    );
    assert_eq!(created["status"], "ok");
    contract_2(&created);
    assert_eq!(created["page"], Value::Null);
    assert_eq!(created["stop"], Value::Null);
    let usage = run_argv(&root, &["--json", "notes"], None).envelope();
    assert_error(&usage, "usage", "missing_data_dir");
    contract_2(&usage);
    let rejected = call(&root, &data, &["read", "--id", "missing"]);
    assert_error(&rejected, "rejected", "not_found");
    contract_2(&rejected);
    let help = run_argv(&root, &["--json", "--help"], None);
    assert_eq!(help.code, 0);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    assert_eq!(description["contract"], json!(2));
}
