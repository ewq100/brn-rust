//! An unusable data folder is refused by name, with the reason in owner
//! terms and no OS error text. Synthetic folders only.
use brn_threads_core::{DataFolderProblem, Error, Store};
use std::path::Path;
use tempfile::TempDir;

fn temp() -> TempDir {
    TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap()
}

fn assert_refused(folder: &Path, expected: DataFolderProblem) {
    let error = match Store::open(folder) {
        Ok(_) => panic!("{} opened", folder.display()),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(
        message.starts_with("BRN cannot use the data folder "),
        "{message}"
    );
    assert!(message.contains(folder.to_str().unwrap()), "{message}");
    assert!(!message.contains("os error"), "{message}");
    assert!(std::error::Error::source(&error).is_some(), "{message}");
    assert!(
        matches!(&error, Error::DataFolder { problem, .. } if *problem == expected),
        "{error:?}"
    );
}

/// The evidence case: `--data-dir /data` on the sealed macOS system volume.
#[cfg(target_os = "macos")]
#[test]
fn a_read_only_disk_is_named() {
    let folder = Path::new("/").join(format!("brn-dwr-readonly-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir(&folder).is_ok() {
        std::fs::remove_dir(&folder).unwrap();
        eprintln!("skipped: / is writable here");
        return;
    }
    assert_refused(&folder, DataFolderProblem::ReadOnly);
    assert_eq!(
        DataFolderProblem::ReadOnly.to_string(),
        "it is on a read-only disk. Choose a folder you can write to."
    );
}

#[cfg(unix)]
#[test]
fn a_folder_without_permission_is_named() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp();
    let parent = root.path().join("locked");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
    // Privileged runners can write into a mode-0500 folder.
    let probe = parent.join("probe");
    if std::fs::create_dir(&probe).is_ok() {
        std::fs::remove_dir(&probe).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        eprintln!("skipped: privileged runner");
        return;
    }
    let folder = parent.join("data");
    assert_refused(&folder, DataFolderProblem::PermissionDenied);
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!folder.exists());
}

#[test]
fn a_file_in_place_of_the_folder_is_named() {
    let root = temp();
    let file = root.path().join("notes.txt");
    std::fs::write(&file, b"synthetic").unwrap();
    assert_refused(&file, DataFolderProblem::NotAFolder);
    assert_refused(&file.join("data"), DataFolderProblem::NotAFolder);
    assert_eq!(std::fs::read(&file).unwrap(), b"synthetic");
}

#[cfg(unix)]
#[test]
fn a_symlinked_path_is_refused_by_name() {
    let root = temp();
    let real = root.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    assert_refused(&alias, DataFolderProblem::SymlinkRefused);
    assert_refused(&alias.join("data"), DataFolderProblem::SymlinkRefused);
    assert_eq!(std::fs::read_dir(&real).unwrap().count(), 0);
    #[cfg(target_os = "macos")]
    {
        let folder = Path::new("/tmp").join(format!("brn-dwr-{}", uuid::Uuid::new_v4()));
        assert_refused(&folder, DataFolderProblem::SymlinkRefused);
        assert!(!folder.exists());
    }
}
