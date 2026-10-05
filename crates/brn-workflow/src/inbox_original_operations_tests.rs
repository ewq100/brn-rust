use super::*;
use crate::{
    app::AppConfig,
    files::inbox::FAULT,
    inbox::{InboxAvailability, InboxListRequest},
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
    RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: f.item,
        preview_digest: preview.digest,
        previous_restore: None,
        attestation: InboxRemovalAttestation {
            version: 1,
            copy_disposable: true,
            meaningful_content_preserved: true,
            consequences_reviewed: true,
            conflicts_acknowledged: true,
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
    again.previous_restore = Some(restore.operation_id);
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
}

#[test]
fn unapproved_or_stale_confirmation_does_not_prepare_or_move_original() {
    for mode in [
        "disposable",
        "semantic",
        "conflicts",
        "consequences",
        "effect",
        "version",
        "source",
        "duplicate",
        "original",
    ] {
        let mut f = Fixture::new();
        f.source();
        let mut request = removal_request(&mut f);
        match mode {
            "disposable" => request.attestation.copy_disposable = false,
            "semantic" => request.attestation.meaningful_content_preserved = false,
            "conflicts" => request.attestation.conflicts_acknowledged = false,
            "consequences" => request.attestation.consequences_reviewed = false,
            "effect" => request.attestation.exact_copy_removal_intended = false,
            "version" => request.attestation.version = 2,
            "source" => fs::write(f.vault.join("source.md"), "changed saved Source").unwrap(),
            "duplicate" => fs::copy(f.vault.join("source.md"), f.vault.join("duplicate.md"))
                .map(|_| ())
                .unwrap(),
            "original" => fs::write(original(&f), "changed original").unwrap(),
            _ => unreachable!(),
        }
        let bytes = fs::read(original(&f)).unwrap();
        assert!(f.app.remove_inbox_original(&request).is_err(), "{mode}");
        assert!(
            f.app
                .inbox_original_removal(request.operation_id)
                .unwrap()
                .is_none(),
            "{mode}"
        );
        assert_eq!(fs::read(original(&f)).unwrap(), bytes);
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
    assert!(
        f.app
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .issues
            .iter()
            .any(|i| i.message.contains("occupied") || i.message.contains("unqualified"))
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
            .removed_at_ms
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
        let original_item = removed.evidence.snapshot.review.original.clone();
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
        assert!(app.preview_inbox_removal(item).unwrap().evidence.blockers.iter().any(|b| matches!(b, crate::inbox_removal::InboxRemovalBlocker::SourceConversionHistoryUnavailable { .. })));
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
                    "format" => envelope["format"] = serde_json::json!(2),
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
        let f = reopen(f);
        assert!(!original(&f).exists(), "{mode}");
        assert!(
            !f.app
                .inbox_items(&InboxListRequest::default())
                .unwrap()
                .issues
                .is_empty(),
            "{mode}"
        );
        assert!(
            matches!(
                f.app.inbox_item(f.item).unwrap().original,
                InboxOriginal::Unavailable { .. }
            ),
            "{mode}"
        );
        if mode != "retained" {
            assert_eq!(fs::read(retained(&f, request.operation_id)).unwrap(), kept);
        }
        assert_eq!(
            f.app
                .inbox_original_removal(request.operation_id)
                .unwrap()
                .unwrap()
                .digest()
                .unwrap(),
            record.digest().unwrap()
        );
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
        assert_eq!(record.restored_at_ms.is_some(), moved, "{step}");
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
        assert_eq!(record.removed_at_ms.is_some(), moved, "{step}");
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
