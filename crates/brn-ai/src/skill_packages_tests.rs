use super::*;
use std::{
    fs as disk,
    os::unix::fs::{PermissionsExt, symlink},
    sync::{Arc, Barrier},
};
fn fixture() -> (tempfile::TempDir, Store, std::path::PathBuf) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let store = Store::create(
        root.path().join("data"),
        brn_threads_core::WorkspacePurpose::Test,
    )
    .unwrap();
    let source = root.path().join("source");
    disk::create_dir(&source).unwrap();
    manifest(&source, "document-review", "Evidence canary");
    (root, store, source)
}
fn manifest(source: &Path, name: &str, body: &str) {
    disk::write(
        source.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: Kontrolli allikaid / review evidence.\n---\n{body}\n"
        ),
    )
    .unwrap();
}
#[test]
fn strict_manifest_subset_byte_bounds_and_builtins() {
    let multiline = [
        "---\nname: x\ndescription: first\u{2028}second\n---\nbody",
        "---\nname: x\ndescription: first\u{2029}second\n---\nbody",
    ];
    let wrongly_accepted = multiline
        .into_iter()
        .filter(|text| skill_metadata(text).is_ok())
        .collect::<Vec<_>>();
    assert!(
        wrongly_accepted.is_empty(),
        "accepted multiline descriptions: {wrongly_accepted:?}"
    );
    let unicode = "---\nname: x\ndescription: Õiged allikad / 正確な根拠\n---\n# Multiline\nFirst\u{2028}Second\u{2029}Third\n";
    assert_eq!(
        skill_metadata(unicode).unwrap().body,
        "# Multiline\nFirst\u{2028}Second\u{2029}Third\n"
    );
    for text in [
        "---\nname: x\ndescription: control\r\r\n---\nbody",
        "name: x\ndescription: absent delimiters",
        "---\nname: Upper\ndescription: Invalid name\n---\nbody",
        "---\nname: x\nname: y\ndescription: Duplicate\n---\nbody",
        "---\nname: x\ndescription: *anchor\n---\nbody",
        "---\nname: x\ndescription: |\n  nested\n---\nbody",
        "---\nname: x\ndescription: \"quoted\"\n---\nbody",
        "---\nname: x\ndescription: valid\nrequirements: shell\n---\nbody",
        "---\nname: x\ndescription: bad\tvalue\n---\nbody",
        "---\nname: x\ndescription: \n---\nbody",
        "---\nname: x\ndescription: valid\n---\n ",
    ] {
        assert!(skill_metadata(text).is_err(), "{text}");
    }
    let text = format!("---\nname: x\ndescription: {}\n---\nbody", "õ".repeat(512));
    assert!(skill_metadata(&text).is_ok());
    assert!(skill_metadata(&text.replace(&"õ".repeat(512), &"õ".repeat(513))).is_err());
    let mut text = "---\nname: x\ndescription: Exact byte boundary\n---\n".to_owned();
    text.push_str(&"x".repeat(FILE_BYTES - text.len()));
    assert!(skill_metadata(&text).is_ok());
    text.push('x');
    assert!(skill_metadata(&text).is_err());
    for skill in CATALOG {
        assert_eq!(skill_metadata(skill.text).unwrap().name, skill.id);
        assert_eq!(
            crate::threads_guides::read_skill(skill.id),
            Some(skill.text)
        );
    }
}
#[test]
fn real_copy_reopen_digest_conflict_and_removal_do_not_enable_runtime() {
    let (root, store, source) = fixture();
    disk::create_dir(source.join("references")).unwrap();
    disk::write(source.join("references/check.md"), "Evidence õ\n").unwrap();
    manifest(
        &source,
        "document-review",
        "See [check](references/check.md)",
    );
    let before = store.records().unwrap();
    let receipts = store.receipts().unwrap();
    let info = install(&store, &source).unwrap();
    assert_eq!(info.origin, "owner_installed");
    assert_eq!(info.scope, "workspace_installation");
    assert!(!info.enabled);
    assert!(!info.included_in_backup);
    assert!(!info.included_in_export);
    assert_eq!(info.file_count, 2);
    assert_eq!(install(&store, &source).unwrap(), info);
    let original = inspect(&store, "document-review").unwrap().files;
    manifest(&source, "document-review", "Changed after installation");
    assert_eq!(
        install(&store, &source).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(inspect(&store, "document-review").unwrap().files, original);
    assert!(source.join("SKILL.md").exists());
    assert_eq!(store.records().unwrap(), before);
    assert_eq!(store.receipts().unwrap(), receipts);
    drop(store);
    let store = Store::open(root.path().join("data")).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    assert!(crate::threads_guides::read_skill("document-review").is_none());
    assert!(crate::threads_guides::read_skill(&info.id).is_none());
    assert!(remove(&store, "document-review", &"0".repeat(64)).is_err());
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    assert!(remove(&store, "intake", &info.digest).is_err());
    assert!(remove(&store, "document-review", &info.digest).unwrap());
    assert!(!remove(&store, "document-review", &info.digest).unwrap());
    assert_eq!(list(&store).unwrap().len(), 4);
    let other = Store::create(
        root.path().join("other"),
        brn_threads_core::WorkspacePurpose::Test,
    )
    .unwrap();
    assert!(inspect(&other, "document-review").is_err());
}
#[test]
fn unsupported_references_scripts_collisions_and_encoding_fail_before_install() {
    let (_root, store, source) = fixture();
    for name in [
        "intake",
        "maintain-notes",
        "resolve-conflict",
        "prepare-reply",
    ] {
        manifest(&source, name, "Must not replace built-in");
        assert!(install(&store, &source).is_err());
    }
    for link in [
        "../outside.md",
        "/absolute.md",
        "https://example.test/dependency",
        "references/missing.md",
        "./SKILL.md",
    ] {
        manifest(&source, "document-review", &format!("[bad]({link})"));
        assert!(install(&store, &source).is_err(), "{link}");
    }
    manifest(&source, "document-review", "Instructions");
    for name in ["run.sh", "requirements.txt", "tools.json"] {
        disk::write(source.join(name), "never execute").unwrap();
        assert!(install(&store, &source).is_err());
        disk::remove_file(source.join(name)).unwrap();
    }
    disk::set_permissions(source.join("SKILL.md"), disk::Permissions::from_mode(0o700)).unwrap();
    assert!(install(&store, &source).is_err());
    disk::set_permissions(source.join("SKILL.md"), disk::Permissions::from_mode(0o600)).unwrap();
    disk::write(source.join("SKILL.md"), [0xff, 0xfe]).unwrap();
    assert!(install(&store, &source).is_err());
    assert_eq!(list(&store).unwrap().len(), 4);
}
#[test]
fn symlink_ancestors_leaf_fifo_hardlink_and_foreign_catalog_are_refused() {
    let (root, store, source) = fixture();
    let owner = root.path().join("owner.txt");
    disk::write(&owner, "outside owner canary").unwrap();
    let linked = root.path().join("linked");
    symlink(&source, &linked).unwrap();
    assert!(install(&store, &linked).is_err());
    disk::create_dir(source.join("references")).unwrap();
    let reference = source.join("references/secret.md");
    symlink(&owner, &reference).unwrap();
    assert!(install(&store, &source).is_err());
    disk::remove_file(&reference).unwrap();
    disk::hard_link(&owner, &reference).unwrap();
    assert!(install(&store, &source).is_err());
    disk::remove_file(&reference).unwrap();
    use std::os::unix::ffi::OsStrExt;
    let fifo = std::ffi::CString::new(reference.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(install(&store, &source).is_err());
    disk::remove_file(&reference).unwrap();
    let outside = root.path().join("foreign");
    disk::create_dir(&outside).unwrap();
    symlink(&outside, store.directory().join("skills")).unwrap();
    assert!(install(&store, &source).is_err());
    assert_eq!(disk::read_dir(&outside).unwrap().count(), 0);
    disk::remove_file(store.directory().join("skills")).unwrap();
    disk::create_dir(store.directory().join("skills")).unwrap();
    disk::write(store.directory().join("skills/owner-file"), "preserve").unwrap();
    assert!(install(&store, &source).is_err());
    assert_eq!(
        disk::read_to_string(store.directory().join("skills/owner-file")).unwrap(),
        "preserve"
    );
    assert!(
        !store
            .directory()
            .join("skills/.brn-skill-packages")
            .exists()
    );
    assert_eq!(disk::read_to_string(owner).unwrap(), "outside owner canary");
}
#[test]
fn encoded_large_payload_file_count_total_depth_and_artifact_tamper_are_bounded() {
    let (_root, store, source) = fixture();
    disk::create_dir(source.join("references")).unwrap();
    for n in 0..15 {
        disk::write(
            source.join(format!("references/{n}.txt")),
            "\\\"\nõ".repeat(4000),
        )
        .unwrap();
    }
    // Fifteen medium reference files exceed the 256KiB aggregate, not just a
    // helper arithmetic limit; actual source-folder installation refuses it.
    assert!(install(&store, &source).is_err());
    let remainder = TOTAL_BYTES - disk::read(source.join("SKILL.md")).unwrap().len();
    for n in 0..15 {
        let length = remainder / 15 + usize::from(n < remainder % 15);
        let text = "\\\"\n".repeat(length.div_ceil(3));
        disk::write(source.join(format!("references/{n}.txt")), &text[..length]).unwrap();
    }
    let info = install(&store, &source).unwrap();
    assert_eq!(info.file_count, 16);
    assert_eq!(info.total_bytes, TOTAL_BYTES);
    let boundary = source.join("references/0.txt");
    let mut text = disk::read_to_string(&boundary).unwrap();
    text.push('x');
    disk::write(&boundary, &text).unwrap();
    assert!(install(&store, &source).is_err());
    text.pop();
    disk::write(&boundary, text).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    disk::write(source.join("references/extra.txt"), "").unwrap();
    assert!(install(&store, &source).is_err());
    disk::remove_file(source.join("references/extra.txt")).unwrap();
    disk::create_dir(source.join("references/nested")).unwrap();
    disk::create_dir(source.join("references/nested/deeper")).unwrap();
    assert!(install(&store, &source).is_err());
    let path = store.directory().join("skills/document-review.json");
    let mut artifact: serde_json::Value =
        serde_json::from_slice(&disk::read(&path).unwrap()).unwrap();
    artifact["files"]["SKILL.md"] = serde_json::json!("tampered");
    let bytes = serde_json::to_vec(&artifact).unwrap();
    disk::write(&path, &bytes).unwrap();
    assert!(inspect(&store, "document-review").is_err());
    assert!(remove(&store, "document-review", &info.digest).is_err());
    assert_eq!(disk::read(&path).unwrap(), bytes);
}
#[test]
fn atomic_exclusive_publish_and_management_lock_preserve_existing_bytes() {
    let (_root, store, source) = fixture();
    let info = install(&store, &source).unwrap();
    let catalog = Catalog::open(&store, false).unwrap().unwrap();
    let name = "document-review.json";
    let before = bounded(
        catalog.dir.file(name, libc::O_RDONLY).unwrap(),
        ARTIFACT_BYTES,
    )
    .unwrap();
    let mut stage = catalog
        .dir
        .file(".stage-test", libc::O_CREAT | libc::O_EXCL | libc::O_WRONLY)
        .unwrap();
    stage.write_all(b"other owner bytes").unwrap();
    assert!(catalog.dir.rename_exclusive(".stage-test", name).is_err());
    assert_eq!(
        bounded(
            catalog.dir.file(name, libc::O_RDONLY).unwrap(),
            ARTIFACT_BYTES
        )
        .unwrap(),
        before
    );
    assert_eq!(
        bounded(
            catalog.dir.file(".stage-test", libc::O_RDONLY).unwrap(),
            128
        )
        .unwrap(),
        "other owner bytes"
    );
    assert_eq!(
        remove(&store, "document-review", &info.digest)
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(catalog);
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
}
#[test]
fn competing_real_installs_have_one_identity_no_overwrite_and_owner_retry() {
    let (root, store, source) = fixture();
    let initial = install(&store, &source).unwrap();
    remove(&store, "document-review", &initial.digest).unwrap();
    let second = root.path().join("second");
    disk::create_dir(&second).unwrap();
    manifest(&second, "document-review", "Other bytes");
    let barrier = Arc::new(Barrier::new(2));
    let data = store.directory().to_owned();
    drop(store);
    let children = [source.clone(), second]
        .into_iter()
        .map(|source| {
            let barrier = barrier.clone();
            let data = data.clone();
            std::thread::spawn(move || {
                let store = Store::open(data).unwrap();
                barrier.wait();
                install(&store, &source)
            })
        })
        .collect::<Vec<_>>();
    let results = children
        .into_iter()
        .map(|child| child.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    let store = Store::open(&data).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, winner);
    assert_eq!(list(&store).unwrap().len(), 5);
}
#[test]
fn installed_catalog_limit_and_nonregular_artifacts_are_fail_closed() {
    let (_root, store, source) = fixture();
    for n in 0..PACKAGES {
        manifest(&source, &format!("review-{n}"), "bounded catalogue");
        install(&store, &source).unwrap();
    }
    manifest(&source, "one-more", "too many");
    assert!(install(&store, &source).is_err());
    assert_eq!(list(&store).unwrap().len(), PACKAGES + 4);
    let path = store.directory().join("skills/review-0.json");
    disk::remove_file(&path).unwrap();
    symlink(source.join("SKILL.md"), &path).unwrap();
    assert!(list(&store).is_err());
    assert!(inspect(&store, "review-0").is_err());
    assert!(source.join("SKILL.md").exists());
}

#[test]
fn descriptor_anchor_does_not_follow_replaced_source_ancestor() {
    let (root, store, source) = fixture();
    let held = Dir::absolute(&source).unwrap();
    let original = root.path().join("original");
    disk::rename(&source, &original).unwrap();
    let outside = root.path().join("outside");
    disk::create_dir(&outside).unwrap();
    manifest(&outside, "outside-skill", "outside instruction canary");
    symlink(&outside, &source).unwrap();
    let mut files = BTreeMap::new();
    source_files(&held, "", &mut files, &mut 0).unwrap();
    assert!(files["SKILL.md"].contains("Evidence canary"));
    assert!(!files["SKILL.md"].contains("outside instruction canary"));
    assert!(install(&store, &source).is_err());
    assert_eq!(list(&store).unwrap().len(), 4);
    assert!(outside.join("SKILL.md").exists());
}

#[test]
fn well_formed_body_digest_and_file_name_binding_refuse_without_deletion() {
    let (_root, store, source) = fixture();
    let info = install(&store, &source).unwrap();
    let path = store.directory().join("skills/document-review.json");
    let original = disk::read(&path).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    assert_eq!(list(&store).unwrap().len(), 5);
    for change_name in [false, true] {
        let mut artifact: Artifact = serde_json::from_slice(&original).unwrap();
        let text = artifact.files.get_mut("SKILL.md").unwrap();
        if change_name {
            *text = text.replace("name: document-review", "name: different-review");
            // Self-consistent new bytes isolate the filename/name binding guard.
            artifact.digest = digest(&artifact.files);
        } else {
            text.push_str("\nA valid changed instruction body.\n");
            // Leave the stored digest untouched; syntax alone must not refuse.
            assert_eq!(artifact.digest, info.digest);
        }
        assert!(validate(artifact.files.clone(), false).is_ok());
        let changed = serde_json::to_vec(&artifact).unwrap();
        disk::write(&path, &changed).unwrap();
        assert!(
            inspect(&store, "document-review").is_err(),
            "name change: {change_name}"
        );
        assert!(list(&store).is_err());
        assert!(remove(&store, "document-review", &info.digest).is_err());
        assert_eq!(disk::read(&path).unwrap(), changed);
    }
    disk::write(&path, &original).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    assert!(remove(&store, "document-review", &info.digest).unwrap());
}

#[test]
fn remove_rechecks_real_replacement_and_rolls_back_without_deleting_it() {
    let (_root, store, source) = fixture();
    let info = install(&store, &source).unwrap();
    let path = store.directory().join("skills/document-review.json");
    let original = disk::read(&path).unwrap();
    assert_eq!(inspect(&store, "document-review").unwrap().info, info);
    let mut artifact: Artifact = serde_json::from_slice(&original).unwrap();
    artifact
        .files
        .get_mut("SKILL.md")
        .unwrap()
        .push_str("\nValid replacement body.\n");
    assert!(validate(artifact.files.clone(), false).is_ok());
    assert_eq!(artifact.digest, info.digest);
    let replacement = serde_json::to_vec(&artifact).unwrap();
    let bytes = replacement.clone();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            BEFORE_REMOVE_RENAME.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
    let _reset = Reset;
    BEFORE_REMOVE_RENAME.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(move |dir, name| {
            observed.set(observed.get() + 1);
            let mut next = dir
                .file(
                    ".replacement-test",
                    libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
                )
                .unwrap();
            next.write_all(&bytes).unwrap();
            next.sync_all().unwrap();
            dir.unlink(name).unwrap();
            dir.rename_exclusive(".replacement-test", name).unwrap();
        }));
    });
    assert!(remove(&store, "document-review", &info.digest).is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(disk::read(&path).unwrap(), replacement);
    assert!(inspect(&store, "document-review").is_err());
    assert!(
        disk::read_dir(store.directory().join("skills"))
            .unwrap()
            .all(|entry| {
                let name = entry.unwrap().file_name();
                !name.to_string_lossy().starts_with(".removed-") && name != ".replacement-test"
            })
    );
    // Actual normal removal succeeds once the valid original is restored.
    disk::write(&path, original).unwrap();
    assert!(remove(&store, "document-review", &info.digest).unwrap());
    assert!(!remove(&store, "document-review", &info.digest).unwrap());
    assert_eq!(calls.get(), 1);
}
