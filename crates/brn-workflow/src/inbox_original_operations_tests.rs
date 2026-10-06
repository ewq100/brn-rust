use super::*;
use crate::{
    app::AppConfig,
    files::inbox::FAULT,
    inbox::{InboxAvailability, InboxListRequest, InboxOriginal},
    inbox_removal::tests::Fixture,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};

fn config(data: &std::path::Path, vault: &std::path::Path) -> AppConfig {
    AppConfig {
        vault_root: Some(vault.to_owned()),
        credentials_dir: Some(data.parent().unwrap().join("credentials")),
        model_dir: None,
    }
}
fn reopen(f: Fixture) -> Fixture {
    let Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    } = f;
    drop(app);
    let app = App::open(&data, config(&data, &vault)).unwrap();
    Fixture {
        _owner,
        data,
        vault,
        app,
        item,
    }
}
fn removal_request(f: &mut Fixture) -> RemoveInboxOriginalRequest {
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    assert!(
        preview.evidence.blockers.is_empty(),
        "{:?}",
        preview.evidence.blockers
    );
    confirmed_request(f.item, preview.digest)
}
fn confirmed_request(item_id: Uuid, preview_digest: [u8; 32]) -> RemoveInboxOriginalRequest {
    RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id,
        preview_digest,
        previous_restore: None,
        confirmation: InboxRemovalConfirmation {
            version: 1,
            exact_copy_removal_intended: true,
        },
    }
}
fn undo(record: &InboxOriginalRemovalRecord) -> RestoreInboxOriginalRequest {
    RestoreInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        removal_operation_id: record.request.operation_id,
        removal_digest: record.digest().unwrap(),
    }
}
fn original(f: &Fixture) -> PathBuf {
    f.data.join("inbox").join(format!("{}.txt", f.item))
}
fn retained(f: &Fixture, operation: Uuid) -> PathBuf {
    f.data
        .join("inbox")
        .join(format!(".brn-inbox-removed-{operation}.original"))
}

#[test]
fn reopened_removed_inventory_rechecks_each_retained_copy_independently() {
    let mut f = Fixture::new();
    f.source();
    let first_item = f.item;
    let first_request = removal_request(&mut f);
    f.app.remove_inbox_original(&first_request).unwrap();
    fs::rename(f.vault.join("source.md"), f.vault.join("first-source.md")).unwrap();
    f.item = f
        .app
        .capture_inbox(&crate::inbox::CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: crate::inbox::InboxKind::Text,
            title: "Second synthetic copy".into(),
            original_name: None,
            text: "Second exact body".into(),
        })
        .unwrap()
        .capture
        .id;
    f.source();
    let second_request = removal_request(&mut f);
    let second = f.app.remove_inbox_original(&second_request).unwrap();
    let mut f = reopen(f);
    for (item, operation_id) in [
        (first_item, first_request.operation_id),
        (f.item, second_request.operation_id),
    ] {
        assert_eq!(
            f.app.inbox_item(item).unwrap().original,
            InboxOriginal::RemovedRetained { operation_id }
        );
    }
    fs::write(
        retained(&f, first_request.operation_id),
        b"Changed after startup",
    )
    .unwrap();
    assert!(matches!(
        f.app.inbox_item(first_item).unwrap().original,
        InboxOriginal::Unavailable { .. }
    ));
    assert_eq!(
        f.app.inbox_item(f.item).unwrap().original,
        InboxOriginal::RemovedRetained {
            operation_id: second_request.operation_id
        }
    );
    assert!(
        f.app
            .restore_inbox_original(&undo(&second))
            .unwrap()
            .restored_at_ms
            .is_some()
    );
    assert_eq!(fs::read(original(&f)).unwrap(), b"Second exact body");
}

#[test]
fn exact_owner_removal_restoration_and_causal_replay_preserve_bytes_and_identity() {
    let mut f = Fixture::new();
    f.source();
    let path = original(&f);
    let bytes = fs::read(&path).unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    let saved = fs::read(f.vault.join("source.md")).unwrap();
    let request = removal_request(&mut f);
    let removed = f.app.remove_inbox_original(&request).unwrap();
    assert!(!path.exists());
    assert_eq!(fs::read(retained(&f, request.operation_id)).unwrap(), bytes);
    assert_eq!(
        fs::metadata(retained(&f, request.operation_id))
            .unwrap()
            .ino(),
        inode
    );
    assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), saved);
    assert!(removed.removed_at_ms.unwrap() >= removed.prepared_at_ms);
    let mut f = reopen(f);
    assert_eq!(
        f.app.inbox_item(f.item).unwrap().original,
        InboxOriginal::RemovedRetained {
            operation_id: request.operation_id
        }
    );
    assert_eq!(
        f.app
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .entries[0]
            .availability,
        InboxAvailability::RemovedRetained
    );
    let restore = undo(&removed);
    let restored = f.app.restore_inbox_original(&restore).unwrap();
    assert!(restored.restored_at_ms.unwrap() >= restored.prepared_at_ms);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(
        f.app
            .remove_inbox_original(&request)
            .unwrap()
            .digest()
            .unwrap(),
        removed.digest().unwrap()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        f.app
            .restore_inbox_original(&restore)
            .unwrap()
            .digest()
            .unwrap(),
        restored.digest().unwrap()
    );
    let mut again = removal_request(&mut f);
    again.previous_restore = Some(InboxOriginalParent {
        operation_id: restore.operation_id,
        record_sha256: restored.digest().unwrap(),
    });
    let second = f.app.remove_inbox_original(&again).unwrap();
    assert_eq!(fs::read(retained(&f, again.operation_id)).unwrap(), bytes);
    assert_eq!(
        f.app
            .inbox_original_operations(f.item)
            .unwrap()
            .iter()
            .map(|s| s.operation_id)
            .collect::<Vec<_>>(),
        vec![
            request.operation_id,
            restore.operation_id,
            second.request.operation_id
        ]
    );
    let second_restore = undo(&second);
    let restored_again = f.app.restore_inbox_original(&second_restore).unwrap();
    let mut f = reopen(f);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(
        f.app
            .remove_inbox_original(&again)
            .unwrap()
            .digest()
            .unwrap(),
        second.digest().unwrap()
    );
    assert_eq!(
        f.app
            .restore_inbox_original(&second_restore)
            .unwrap()
            .digest()
            .unwrap(),
        restored_again.digest().unwrap()
    );
    assert_eq!(
        f.app
            .remove_inbox_original(&request)
            .unwrap()
            .digest()
            .unwrap(),
        removed.digest().unwrap()
    );
    assert_eq!(
        f.app
            .restore_inbox_original(&restore)
            .unwrap()
            .digest()
            .unwrap(),
        restored.digest().unwrap()
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), saved);
}

#[test]
fn unapproved_or_stale_confirmation_does_not_prepare_or_move_original() {
    for mode in [
        "effect",
        "version",
        "source",
        "unapproved_source",
        "missing_source",
        "duplicate",
        "original",
        "missing_original",
    ] {
        let mut f = Fixture::new();
        let mut request = if mode == "unapproved_source" {
            let preview = f.app.preview_inbox_removal(f.item).unwrap();
            assert_eq!(
                preview.evidence.blockers,
                vec![crate::inbox_removal::InboxRemovalBlocker::SourceRequired]
            );
            confirmed_request(f.item, preview.digest)
        } else {
            f.source();
            removal_request(&mut f)
        };
        match mode {
            "effect" => request.confirmation.exact_copy_removal_intended = false,
            "version" => request.confirmation.version = 2,
            "source" => fs::write(f.vault.join("source.md"), "changed saved Source").unwrap(),
            "missing_source" => fs::remove_file(f.vault.join("source.md")).unwrap(),
            "duplicate" => fs::copy(f.vault.join("source.md"), f.vault.join("duplicate.md"))
                .map(|_| ())
                .unwrap(),
            "original" => fs::write(original(&f), "changed original").unwrap(),
            "missing_original" => fs::rename(original(&f), f.data.join("held-original")).unwrap(),
            "unapproved_source" => {}
            _ => unreachable!(),
        }
        let observed = if mode == "missing_original" {
            f.data.join("held-original")
        } else {
            original(&f)
        };
        let bytes = fs::read(&observed).unwrap();
        assert!(f.app.remove_inbox_original(&request).is_err(), "{mode}");
        assert!(
            f.app
                .inbox_original_removal(request.operation_id)
                .unwrap()
                .is_none(),
            "{mode}"
        );
        assert_eq!(fs::read(observed).unwrap(), bytes);
        assert_eq!(original(&f).exists(), mode != "missing_original");
        assert!(!retained(&f, request.operation_id).exists());
    }
}

#[test]
fn removal_replay_and_occupied_restore_never_adopt_or_overwrite_recreated_endpoint() {
    let mut f = Fixture::new();
    f.source();
    let request = removal_request(&mut f);
    let removed = f.app.remove_inbox_original(&request).unwrap();
    let kept = fs::read(retained(&f, request.operation_id)).unwrap();
    fs::write(original(&f), "unrelated recreated bytes").unwrap();
    fs::set_permissions(original(&f), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        f.app
            .remove_inbox_original(&request)
            .unwrap()
            .digest()
            .unwrap(),
        removed.digest().unwrap()
    );
    assert!(f.app.restore_inbox_original(&undo(&removed)).is_err());
    assert_eq!(
        fs::read(original(&f)).unwrap(),
        b"unrelated recreated bytes"
    );
    assert_eq!(fs::read(retained(&f, request.operation_id)).unwrap(), kept);
    let f = reopen(f);
    assert_eq!(
        fs::read(original(&f)).unwrap(),
        b"unrelated recreated bytes"
    );
    assert!(matches!(
        f.app.inbox_item(f.item).unwrap().original,
        InboxOriginal::Changed { .. }
    ));
    assert_eq!(
        f.app
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .entries[0]
            .availability,
        InboxAvailability::Changed
    );
}

#[test]
fn durable_intent_alone_never_removes_on_restart_and_retries_requalify() {
    let mut f = Fixture::new();
    f.source();
    let request = removal_request(&mut f);
    FAULT.with(|fault| fault.set(Some(("original_operation_intent", false))));
    assert_eq!(
        f.app.remove_inbox_original(&request).unwrap_err().kind,
        ErrorKind::InboxUncertain
    );
    FAULT.with(|fault| fault.set(None));
    let bytes = fs::read(original(&f)).unwrap();
    let mut f = reopen(f);
    assert_eq!(fs::read(original(&f)).unwrap(), bytes);
    assert!(
        f.app
            .inbox_original_removal(request.operation_id)
            .unwrap()
            .unwrap()
            .summary()
            .unwrap()
            .settled_at_ms
            .is_none()
    );
    fs::write(f.vault.join("source.md"), "later changed Source").unwrap();
    assert_eq!(
        f.app.remove_inbox_original(&request).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    assert_eq!(fs::read(original(&f)).unwrap(), bytes);
}

#[test]
fn removal_namespace_checks_reject_symlinks_hardlinks_and_rebound_root() {
    for mode in ["symlink", "hardlink", "root"] {
        let mut f = Fixture::new();
        f.source();
        let request = removal_request(&mut f);
        let path = original(&f);
        let bytes = fs::read(&path).unwrap();
        match mode {
            "symlink" => {
                fs::rename(&path, f.data.join("unrelated-original")).unwrap();
                std::os::unix::fs::symlink(f.data.join("unrelated-original"), &path).unwrap();
            }
            "hardlink" => fs::hard_link(&path, f.data.join("extra-link")).unwrap(),
            "root" => {
                fs::rename(f.data.join("inbox"), f.data.join("original-inbox")).unwrap();
                fs::create_dir(f.data.join("inbox")).unwrap();
                fs::set_permissions(f.data.join("inbox"), fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(f.app.remove_inbox_original(&request).is_err(), "{mode}");
        let observed = if mode == "root" {
            f.data
                .join("original-inbox")
                .join(path.file_name().unwrap())
        } else {
            path
        };
        assert_eq!(fs::read(observed).unwrap(), bytes);
        assert!(
            f.app
                .inbox_original_removal(request.operation_id)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn fresh_and_older_databases_preserve_certificate_before_approval_recovery_without_resurrection() {
    for older in [false, true] {
        let mut f = Fixture::new();
        let baseline;
        {
            baseline = fs::read(
                fs::read_dir(f.data.join("backups"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path(),
            )
            .unwrap();
        }
        f.source();
        let request = removal_request(&mut f);
        let removed = f.app.remove_inbox_original(&request).unwrap();
        let original_item = removed.evidence.item.clone();
        let Fixture {
            _owner,
            data,
            vault,
            app,
            item,
        } = f;
        drop(app);
        for name in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
            let path = data.join(name);
            if path.exists() {
                fs::rename(&path, _owner.path().join(format!("retired-{name}"))).unwrap();
            }
        }
        fs::rename(data.join("backups"), _owner.path().join("retired-backups")).unwrap();
        if older {
            fs::write(data.join("brn.sqlite"), baseline).unwrap();
        }
        let mut app = App::open(&data, config(&data, &vault)).unwrap();
        assert!(app.open_report().restored_from.is_none());
        assert_eq!(app.inbox_item(item).unwrap().item, original_item);
        assert_eq!(
            app.inbox_item(item).unwrap().original,
            InboxOriginal::RemovedRetained {
                operation_id: request.operation_id
            }
        );
        assert!(!data.join("inbox").join(format!("{item}.txt")).exists());
        let recovered = app
            .inbox_original_removal(request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.digest().unwrap(), removed.digest().unwrap());
        assert_eq!(
            app.remove_inbox_original(&request)
                .unwrap()
                .digest()
                .unwrap(),
            removed.digest().unwrap()
        );
        app.restore_inbox_original(&undo(&removed)).unwrap();
        assert!(matches!(
            app.inbox_item(item).unwrap().original,
            InboxOriginal::Available { .. }
        ));
        // Recovery retains the exact approved Source proof without requiring
        // reconstructed processing/analysis history as a cleanup admission gate.
        assert!(
            app.preview_inbox_removal(item)
                .unwrap()
                .evidence
                .blockers
                .is_empty()
        );
    }
}

#[test]
fn missing_operation_mirrors_rebuild_from_checked_store_without_repeating_effects() {
    let mut f = Fixture::new();
    f.source();
    let request = removal_request(&mut f);
    let record = f.app.remove_inbox_original(&request).unwrap();
    for suffix in ["intent", "receipt"] {
        fs::remove_file(f.data.join("inbox").join(format!(
            ".brn-inbox-removal-{}.{suffix}",
            request.operation_id
        )))
        .unwrap();
    }
    let f = reopen(f);
    assert!(!original(&f).exists());
    assert_eq!(
        f.app
            .inbox_original_removal(request.operation_id)
            .unwrap()
            .unwrap()
            .digest()
            .unwrap(),
        record.digest().unwrap()
    );
    for suffix in ["intent", "receipt"] {
        assert!(
            f.data
                .join("inbox")
                .join(format!(
                    ".brn-inbox-removal-{}.{suffix}",
                    request.operation_id
                ))
                .exists()
        );
    }
}

#[test]
fn damaged_operation_mirror_never_falls_through_to_old_capture_installation() {
    for mode in ["format", "extra", "timestamp", "hardlink", "retained"] {
        let mut f = Fixture::new();
        f.source();
        let request = removal_request(&mut f);
        let record = f.app.remove_inbox_original(&request).unwrap();
        let path = f.data.join("inbox").join(format!(
            ".brn-inbox-removal-{}.receipt",
            request.operation_id
        ));
        let kept = fs::read(retained(&f, request.operation_id)).unwrap();
        match mode {
            "format" | "extra" | "timestamp" => {
                let mut envelope: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                match mode {
                    "format" => envelope["format"] = serde_json::json!(9),
                    "extra" => envelope["extra"] = serde_json::json!(true),
                    "timestamp" => {
                        envelope["operation"]["record"]["removed_at_ms"] =
                            serde_json::json!(record.removed_at_ms.unwrap() + 1)
                    }
                    _ => unreachable!(),
                }
                fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
            }
            "hardlink" => fs::hard_link(&path, f.data.join("unrelated-link")).unwrap(),
            "retained" => fs::write(
                retained(&f, request.operation_id),
                b"changed retained bytes",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        if mode == "retained" {
            let f = reopen(f);
            assert!(!original(&f).exists());
            assert_eq!(
                f.app
                    .inbox_items(&InboxListRequest::default())
                    .unwrap()
                    .entries[0]
                    .availability,
                InboxAvailability::Unavailable
            );
            assert!(matches!(
                f.app.inbox_item(f.item).unwrap().original,
                InboxOriginal::Unavailable { .. }
            ));
            assert_eq!(
                f.app
                    .inbox_original_removal(request.operation_id)
                    .unwrap()
                    .unwrap()
                    .digest()
                    .unwrap(),
                record.digest().unwrap()
            );
        } else {
            let Fixture {
                _owner,
                data,
                vault,
                app,
                item,
            } = f;
            drop(app);
            assert!(App::open(&data, config(&data, &vault)).is_err(), "{mode}");
            assert!(
                !data.join("inbox").join(format!("{item}.txt")).exists(),
                "{mode}"
            );
            assert_eq!(
                fs::read(data.join("inbox").join(format!(
                    ".brn-inbox-removed-{}.original",
                    request.operation_id
                )))
                .unwrap(),
                kept,
                "{mode}"
            );
        }
    }
}

#[test]
fn process_crash_windows_certify_only_already_performed_restoration() {
    let _fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
    for step in [
        "original_operation_store_intent",
        "original_operation_intent",
        "original_operation_before_rename",
        "original_operation_renamed",
        "original_operation_synced",
        "original_operation_receipt",
        "original_operation_store_receipt",
    ] {
        let mut f = Fixture::new();
        f.source();
        let request = removal_request(&mut f);
        let removed = f.app.remove_inbox_original(&request).unwrap();
        let request = undo(&removed);
        let bytes = fs::read(retained(&f, removed.request.operation_id)).unwrap();
        let input = f.data.parent().unwrap().join("operation.json");
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        let Fixture {
            _owner,
            data,
            vault,
            app,
            item,
        } = f;
        drop(app);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inbox_original_operations::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BRN_ORIGINAL_SYNTHETIC_DATA", &data)
            .env("BRN_ORIGINAL_SYNTHETIC_VAULT", &vault)
            .env("BRN_ORIGINAL_SYNTHETIC_INPUT", &input)
            .env("BRN_ORIGINAL_SYNTHETIC_STEP", step)
            .env("BRN_ORIGINAL_SYNTHETIC_KIND", "restore")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(91),
            "{step}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut app = App::open(&data, config(&data, &vault)).unwrap();
        let record = app
            .inbox_original_restore(request.operation_id)
            .unwrap()
            .unwrap();
        let moved = matches!(
            step,
            "original_operation_renamed"
                | "original_operation_synced"
                | "original_operation_receipt"
                | "original_operation_store_receipt"
        );
        assert_eq!(
            record.summary().unwrap().settled_at_ms.is_some(),
            moved,
            "{step}"
        );
        let path = data.join("inbox").join(if moved {
            format!("{item}.txt")
        } else {
            format!(
                ".brn-inbox-removed-{}.original",
                removed.request.operation_id
            )
        });
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert!(
            app.restore_inbox_original(&request)
                .unwrap()
                .restored_at_ms
                .is_some(),
            "{step}"
        );
    }
}

#[test]
fn process_crash_windows_certify_only_already_performed_removal() {
    let _fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
    for step in [
        "original_operation_store_intent",
        "original_operation_intent",
        "original_operation_before_rename",
        "original_operation_renamed",
        "original_operation_synced",
        "original_operation_receipt",
        "original_operation_store_receipt",
    ] {
        let mut f = Fixture::new();
        f.source();
        let request = removal_request(&mut f);
        let bytes = fs::read(original(&f)).unwrap();
        let input = f.data.parent().unwrap().join("operation.json");
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        let Fixture {
            _owner,
            data,
            vault,
            app,
            item,
        } = f;
        drop(app);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inbox_original_operations::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BRN_ORIGINAL_SYNTHETIC_DATA", &data)
            .env("BRN_ORIGINAL_SYNTHETIC_VAULT", &vault)
            .env("BRN_ORIGINAL_SYNTHETIC_INPUT", &input)
            .env("BRN_ORIGINAL_SYNTHETIC_STEP", step)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(91),
            "{step}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut app = App::open(&data, config(&data, &vault)).unwrap();
        let record = app
            .inbox_original_removal(request.operation_id)
            .unwrap()
            .unwrap();
        let moved = matches!(
            step,
            "original_operation_renamed"
                | "original_operation_synced"
                | "original_operation_receipt"
                | "original_operation_store_receipt"
        );
        assert_eq!(
            record.summary().unwrap().settled_at_ms.is_some(),
            moved,
            "{step}"
        );
        let path = data.join("inbox").join(if moved {
            format!(".brn-inbox-removed-{}.original", request.operation_id)
        } else {
            format!("{item}.txt")
        });
        assert_eq!(fs::read(path).unwrap(), bytes);
        let result = app.remove_inbox_original(&request).unwrap();
        assert!(result.removed_at_ms.is_some(), "{step}");
    }
}
#[test]
#[ignore = "invoked only by the parent synthetic process-crash witness"]
fn crash_child() {
    let Some(data) = std::env::var_os("BRN_ORIGINAL_SYNTHETIC_DATA") else {
        return;
    };
    let data = PathBuf::from(data);
    let vault = PathBuf::from(std::env::var_os("BRN_ORIGINAL_SYNTHETIC_VAULT").unwrap());
    let input = PathBuf::from(std::env::var_os("BRN_ORIGINAL_SYNTHETIC_INPUT").unwrap());
    assert_eq!(data.file_name().unwrap(), "data");
    assert_eq!(data.parent(), input.parent());
    assert_eq!(data.parent(), vault.parent());
    let bytes = fs::read(input).unwrap();
    let step = match std::env::var("BRN_ORIGINAL_SYNTHETIC_STEP")
        .unwrap()
        .as_str()
    {
        "original_operation_store_intent" => "original_operation_store_intent",
        "original_operation_intent" => "original_operation_intent",
        "original_operation_before_rename" => "original_operation_before_rename",
        "original_operation_renamed" => "original_operation_renamed",
        "original_operation_synced" => "original_operation_synced",
        "original_operation_receipt" => "original_operation_receipt",
        "original_operation_store_receipt" => "original_operation_store_receipt",
        _ => panic!("unknown test checkpoint"),
    };
    let mut app = App::open(&data, config(&data, &vault)).unwrap();
    FAULT.with(|fault| fault.set(Some((step, true))));
    if std::env::var("BRN_ORIGINAL_SYNTHETIC_KIND").ok().as_deref() == Some("restore") {
        let request: RestoreInboxOriginalRequest = serde_json::from_slice(&bytes).unwrap();
        let _ = app.restore_inbox_original(&request);
    } else {
        let request: RemoveInboxOriginalRequest = serde_json::from_slice(&bytes).unwrap();
        let _ = app.remove_inbox_original(&request);
    }
    panic!("checkpoint did not terminate child");
}
