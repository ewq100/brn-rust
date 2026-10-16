//! Guard for owner decision B2 (one test per process, because it sets
//! CODEX_HOME in-process): a workspace that already exists keeps its stored
//! credential source when the CLI opens it.
mod support;
use support::*;

#[test]
fn existing_workspace_keeps_its_stored_auth_file() {
    let root = Root::new();
    let home = root.join("home");
    std::env::set_var("CODEX_HOME", &home);
    let data = root.join("data");
    drop(brn_threads_app::Workspace::open(&data).unwrap());
    let settings = ok(&root, &data, &["settings"]);
    assert_eq!(
        settings["auth_file"],
        home.join("auth.json").to_str().unwrap(),
        "{settings}"
    );
}
