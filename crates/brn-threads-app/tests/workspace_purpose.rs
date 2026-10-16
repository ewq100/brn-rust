//! Round 3: the app seams the CLI opens and creates workspaces through.
use brn_threads_app::*;

fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

#[test]
fn a_created_test_workspace_has_no_credential_source_and_refuses_a_second_creation() {
    let root = root();
    let data = root.path().join("test");
    let workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    assert_eq!(
        workspace.store.workspace_purpose().unwrap(),
        WorkspacePurpose::Test
    );
    let settings = workspace.settings().unwrap();
    assert_eq!(settings.auth_file, None);
    assert_eq!(settings.credentials_dir, None);
    drop(workspace);
    let again = Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test)
        .err()
        .expect("second creation");
    assert_eq!(
        again.to_string(),
        "invalid request: A test workspace needs a new data folder"
    );
    // Reopening keeps the purpose.
    let reopened = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(
        reopened.store.workspace_purpose().unwrap(),
        WorkspacePurpose::Test
    );
}

#[test]
fn the_no_upgrade_open_creates_owner_workspaces_without_credentials_and_reopens_them() {
    let root = root();
    let data = root.path().join("fresh");
    let workspace = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(
        workspace.store.workspace_purpose().unwrap(),
        WorkspacePurpose::Owner
    );
    let settings = workspace.settings().unwrap();
    assert_eq!((settings.auth_file, settings.credentials_dir), (None, None));
    let note = {
        let mut workspace = workspace;
        workspace.new_note("Synthetic", "kept").unwrap()
    };
    let reopened = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(reopened.store.record(&note.id).unwrap().unwrap(), note);
}
