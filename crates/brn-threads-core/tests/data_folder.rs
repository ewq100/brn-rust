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

#[cfg(unix)]
#[test]
fn skills_sidecar_directory_is_narrowly_admitted_and_non_directories_refused() {
    let root = temp();
    let folder = root.path().join("data");
    drop(Store::open(&folder).unwrap());
    let sidecar = folder.join("skills");
    std::fs::create_dir(&sidecar).unwrap();
    std::fs::write(
        sidecar.join("owner-config"),
        "untouched local configuration",
    )
    .unwrap();
    drop(Store::open(&folder).expect("literal real skills directory must survive reopen"));
    drop(Store::open_existing_without_upgrade(&folder).unwrap());
    assert_eq!(
        std::fs::read_to_string(sidecar.join("owner-config")).unwrap(),
        "untouched local configuration"
    );
    assert!(!sidecar.join(".brn-skill-packages").exists());
    std::fs::remove_dir_all(&sidecar).unwrap();
    let database = folder.join(brn_threads_core::DATABASE_FILENAME);
    for kind in ["file", "symlink", "unrelated"] {
        let before = std::fs::read(&database).unwrap();
        let external = root.path().join(format!("external-{kind}"));
        std::fs::create_dir(&external).unwrap();
        std::fs::write(external.join("canary"), "outside owner bytes").unwrap();
        let path = if kind == "unrelated" {
            folder.join("foreign-folder")
        } else {
            sidecar.clone()
        };
        match kind {
            "file" => std::fs::write(&path, "owner file").unwrap(),
            "symlink" => std::os::unix::fs::symlink(&external, &path).unwrap(),
            _ => std::fs::create_dir(&path).unwrap(),
        }
        assert!(Store::open(&folder).is_err(), "{kind}");
        assert!(
            Store::open_existing_without_upgrade(&folder).is_err(),
            "{kind}"
        );
        assert_eq!(std::fs::read(&database).unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(external.join("canary")).unwrap(),
            "outside owner bytes"
        );
        if kind == "unrelated" {
            std::fs::remove_dir(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
    }
}
