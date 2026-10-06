use super::*;

#[test]
fn pinned_legacy_mirror_keeps_historical_bytes_and_empty_history_omission() {
    let bytes = include_bytes!("fixtures/inbox-original-mirror-v1.json");
    assert_eq!(
        digest(bytes),
        [
            85, 76, 157, 250, 30, 209, 87, 240, 20, 48, 110, 161, 207, 62, 152, 23, 160, 27, 137,
            35, 155, 175, 4, 96, 93, 159, 109, 190, 170, 217, 170, 180
        ]
    );
    let operation = decode(bytes).unwrap();
    assert!(matches!(operation, OriginalOperationFile::LegacyRemove(_)));
    assert_eq!(encode(&operation).unwrap(), bytes);
    assert!(!String::from_utf8_lossy(bytes).contains("original_operations"));
}

use crate::{
    app::{App, AppConfig},
    inbox::{CaptureInboxRequest, InboxKind},
    inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
    proposal_apply::ApprovalRequest,
};
use std::{fs, os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    files: InboxFiles,
    record: current::InboxOriginalRemovalRecord,
}
impl Fixture {
    fn new() -> Self {
        Self::with_text("\u{feff}Complete õ 日本語\r\n\u{0001}".into())
    }
    fn with_text(text: String) -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        let mut app = App::open(
            &data,
            AppConfig {
                vault_root: Some(vault),
                credentials_dir: Some(owner.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        let item = app
            .capture_inbox(&CaptureInboxRequest {
                id: Uuid::new_v4(),
                kind: InboxKind::Email,
                title: "Synthetic original".into(),
                original_name: None,
                text,
            })
            .unwrap();
        let batch = app
            .process_inbox(&ProcessInboxRequest {
                id: Uuid::new_v4(),
                items: vec![item.clone()],
            })
            .unwrap();
        app.advance_inbox_processing(batch.request.id, &AtomicBool::new(false))
            .unwrap();
        let draft = app
            .prepare_inbox_source(&InboxSourceRequest {
                candidate: InboxCandidateRequest {
                    batch_id: batch.request.id,
                    index: 0,
                },
                proposal_id: Uuid::new_v4(),
                note_id: Uuid::new_v4(),
                path: "source.md".into(),
                title: "Source".into(),
            })
            .unwrap();
        let proposal = app.create_proposal(&draft).unwrap();
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        })
        .unwrap();
        let preview = app.preview_inbox_removal(item.capture.id).unwrap();
        assert!(preview.evidence.blockers.is_empty());
        let evidence: current::InboxQualifiedRemovalEvidence =
            serde_json::from_value(serde_json::to_value(preview.evidence).unwrap()).unwrap();
        assert_eq!(evidence.digest().unwrap(), preview.digest);
        let bound: InboxRoot =
            serde_json::from_str(&app.store.setting("inbox.root").unwrap().unwrap()).unwrap();
        let files = InboxFiles::open(&data, Some(&bound), false)
            .unwrap()
            .unwrap();
        let record = current::InboxOriginalRemovalRecord {
            request: current::RemoveInboxOriginalRequest {
                operation_id: Uuid::new_v4(),
                item_id: item.capture.id,
                preview_digest: preview.digest,
                previous_restore: None,
                confirmation: current::InboxRemovalConfirmation {
                    version: 1,
                    exact_copy_removal_intended: true,
                },
            },
            evidence,
            namespace: files.original_namespace(),
            prepared_at_ms: item.received_at_ms + 1,
            removed_at_ms: None,
        };
        record.validate().unwrap();
        drop(app);
        Self {
            _owner: owner,
            data,
            files,
            record,
        }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.data.join("inbox").join(name)
    }
    fn operation(&self) -> OriginalOperationFile {
        OriginalOperationFile::Remove(Box::new(self.record.clone()))
    }
    fn remove(&mut self) {
        let operation = self.operation();
        self.files.publish_operation(&operation).unwrap();
        assert!(!self.files.operation_effect_observed(&operation).unwrap());
        self.files.move_original(&operation).unwrap();
        assert!(self.files.operation_effect_observed(&operation).unwrap());
        self.record.removed_at_ms = Some(self.record.prepared_at_ms + 1);
        self.files.publish_operation(&self.operation()).unwrap();
    }
    fn restore(&self, legacy_version: bool) -> OriginalOperationFile {
        let request = legacy::RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: self.record.request.operation_id,
            removal_digest: self.record.digest().unwrap(),
        };
        let original = self.record.evidence.item.clone();
        let namespace = self.record.namespace.clone();
        let prepared_at_ms = self.record.removed_at_ms.unwrap() + 1;
        if legacy_version {
            OriginalOperationFile::LegacyRestore(Box::new(legacy::InboxOriginalRestoreRecord {
                request,
                original,
                namespace,
                prepared_at_ms,
                restored_at_ms: None,
            }))
        } else {
            OriginalOperationFile::Restore(Box::new(current::InboxOriginalRestoreRecord {
                request,
                original,
                namespace,
                prepared_at_ms,
                restored_at_ms: None,
            }))
        }
    }
}
fn file_identity(path: &Path) -> (u64, u64) {
    let metadata = fs::metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

#[test]
fn exact_operation_mirrors_preserve_equal_inode_and_exclusive_move_identity() {
    for legacy_restore in [false, true] {
        let mut f = Fixture::new();
        let item = f.record.evidence.item.clone();
        let original_path = f.path(&original(item.capture.id));
        let before = fs::read(&original_path).unwrap();
        let inode = file_identity(&original_path);
        let operation = f.operation();
        f.files.publish_operation(&operation).unwrap();
        let mirror = f.path(&operation.name());
        let bytes = fs::read(&mirror).unwrap();
        let mirror_inode = file_identity(&mirror);
        f.files.publish_operation(&operation).unwrap();
        assert_eq!(fs::read(&mirror).unwrap(), bytes);
        assert_eq!(file_identity(&mirror), mirror_inode);
        assert!(matches!(
            f.files.operation(&operation.name()).unwrap(),
            OriginalOperationFile::Remove(_)
        ));
        f.remove();
        assert!(!original_path.exists());
        let retained_path = f.path(&retained(f.record.request.operation_id));
        assert_eq!(file_identity(&retained_path), inode);
        assert_eq!(fs::read(&retained_path).unwrap(), before);
        f.files
            .retained_copy(&item, f.record.request.operation_id, &f.record.namespace)
            .unwrap();
        let restore = f.restore(legacy_restore);
        f.files.publish_operation(&restore).unwrap();
        let restore_mirror = f.path(&restore.name());
        let restore_bytes = fs::read(&restore_mirror).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&restore_bytes).unwrap()["format"],
            if legacy_restore { 1 } else { 2 }
        );
        let restore_inode = file_identity(&restore_mirror);
        f.files.publish_operation(&restore).unwrap();
        assert_eq!(file_identity(&restore_mirror), restore_inode);
        assert_eq!(fs::read(&restore_mirror).unwrap(), restore_bytes);
        assert!(!f.files.operation_effect_observed(&restore).unwrap());
        f.files.move_original(&restore).unwrap();
        assert!(f.files.operation_effect_observed(&restore).unwrap());
        assert_eq!(file_identity(&original_path), inode);
        assert_eq!(fs::read(&original_path).unwrap(), before);
        assert!(!retained_path.exists());
    }
}

#[test]
fn operation_decode_refuses_versions_hashes_semantics_and_noncanonical_legacy_defaults() {
    let bytes = include_bytes!("fixtures/inbox-original-mirror-v1.json");
    for field in [
        "format",
        "sha256",
        "kind",
        "empty_history",
        "confirmation",
        "unknown",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        match field {
            "format" => value["format"] = 9.into(),
            "sha256" => value["sha256"][0] = 0.into(),
            "kind" => value["operation"]["kind"] = "legacy_remove".into(),
            "empty_history" => {
                value["operation"]["record"]["evidence"]["snapshot"]["original_operations"] =
                    serde_json::json!([])
            }
            "confirmation" => {
                value["operation"]["record"]["request"]["attestation"]["exact_copy_removal_intended"] =
                    false.into()
            }
            _ => value["unexpected"] = true.into(),
        }
        assert!(
            decode(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{field}"
        );
    }
    let f = Fixture::new();
    let mut bytes = encode(&f.operation()).unwrap();
    assert!(decode(&bytes).is_ok());
    bytes.push(b' ');
    assert!(decode(&bytes).is_err());
    let mut changed = f.record.clone();
    changed.request.confirmation.exact_copy_removal_intended = false;
    // Correct canonical syntax and a recomputed body hash cannot admit an invalid request.
    let bytes = envelope(OperationRef::Remove(&changed), 2, MAX_NEW_MIRROR_BYTES).unwrap();
    assert!(decode(&bytes).is_err());
    changed = f.record.clone();
    changed
        .evidence
        .source
        .as_mut()
        .unwrap()
        .saved
        .text
        .push_str("changed body");
    let bytes = envelope(OperationRef::Remove(&changed), 2, MAX_NEW_MIRROR_BYTES).unwrap();
    assert!(decode(&bytes).is_err());
    let legacy = decode(include_bytes!("fixtures/inbox-original-mirror-v1.json")).unwrap();
    let OriginalOperationFile::LegacyRemove(mut changed) = legacy else {
        unreachable!()
    };
    changed.request.attestation.exact_copy_removal_intended = false;
    let bytes = envelope(LegacyRef::Remove(&changed), 1, MAX_ORIGINAL_OPERATION_BYTES).unwrap();
    assert!(decode(&bytes).is_err());
}

#[test]
fn occupied_or_substituted_operation_artifacts_are_refused_without_overwrite() {
    let f = Fixture::new();
    let operation = f.operation();
    let name = operation.name();
    let path = f.path(&name);
    f.files.publish_operation(&operation).unwrap();
    let initial = fs::read(&path).unwrap();
    let initial_inode = file_identity(&path);
    let mut fork = f.operation();
    let OriginalOperationFile::Remove(record) = &mut fork else {
        unreachable!()
    };
    record.prepared_at_ms += 1;
    assert_eq!(
        f.files.publish_operation(&fork).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert_eq!(fs::read(&path).unwrap(), initial);
    assert_eq!(file_identity(&path), initial_inode);
    let wrong_name = f.path(&format!(".brn-inbox-removal-{}.intent", Uuid::new_v4()));
    fs::copy(&path, &wrong_name).unwrap();
    fs::set_permissions(&wrong_name, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        f.files
            .operation(wrong_name.file_name().unwrap().to_str().unwrap())
            .is_err()
    );
    let held = f.path("held-original-operation");
    fs::rename(&path, &held).unwrap();
    std::os::unix::fs::symlink(&held, &path).unwrap();
    assert!(f.files.operation(&name).is_err());
    assert!(f.files.publish_operation(&operation).is_err());
    fs::remove_file(&path).unwrap();
    fs::hard_link(&held, &path).unwrap();
    assert!(f.files.operation(&name).is_err());
    fs::remove_file(&path).unwrap();
    fs::rename(&held, &path).unwrap();
    // A byte-identical inode replacement during the stable read is still refused.
    let replacement = f.path("replacement-original-operation");
    fs::write(&replacement, &initial).unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
    let target = path.clone();
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || fs::rename(replacement, target).unwrap()))
    });
    assert!(f.files.operation(&name).is_err());
    assert_eq!(fs::read(&path).unwrap(), initial);
}

#[test]
fn original_namespace_exact_endpoints_and_intent_fence_effects() {
    let mut f = Fixture::new();
    let operation = f.operation();
    assert!(f.files.move_original(&operation).is_err());
    let mut foreign = f.operation();
    let OriginalOperationFile::Remove(record) = &mut foreign else {
        unreachable!()
    };
    record.namespace.data_inode += 1;
    assert!(f.files.publish_operation(&foreign).is_err());
    assert!(!f.path(&foreign.name()).exists());
    f.files.publish_operation(&operation).unwrap();
    let retained_path = f.path(&retained(operation.id()));
    fs::write(&retained_path, b"occupied synthetic endpoint").unwrap();
    fs::set_permissions(&retained_path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(f.files.move_original(&operation).is_err());
    assert!(f.files.operation_effect_observed(&operation).is_err());
    assert_eq!(
        fs::read(&retained_path).unwrap(),
        b"occupied synthetic endpoint"
    );
    fs::remove_file(&retained_path).unwrap();
    f.remove();
    let restore = f.restore(false);
    f.files.publish_operation(&restore).unwrap();
    let original_path = f.path(&original(operation.item().capture.id));
    fs::write(&original_path, b"recreated endpoint").unwrap();
    assert!(f.files.original_occupied(operation.item()).unwrap());
    assert!(f.files.move_original(&restore).is_err());
    assert_eq!(fs::read(&original_path).unwrap(), b"recreated endpoint");
    assert_eq!(
        fs::read(&retained_path).unwrap(),
        match &f.record.evidence.original {
            current::InboxQualifiedOriginal::Available { text } => text.as_bytes(),
        }
    );
    fs::remove_file(&original_path).unwrap();
    let mut namespace = f.record.namespace.clone();
    namespace.data_device += 1;
    assert!(
        f.files
            .retained_copy(operation.item(), operation.id(), &namespace)
            .is_err()
    );
}

#[test]
fn operation_reader_refuses_oversize_and_changed_namespace_before_effects() {
    let f = Fixture::new();
    let operation = f.operation();
    f.files.publish_operation(&operation).unwrap();
    let path = f.path(&operation.name());
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len((MAX_ORIGINAL_OPERATION_BYTES + 1) as u64)
        .unwrap();
    assert!(f.files.operation(&operation.name()).is_err());
    assert!(f.files.move_original(&operation).is_err());
    assert!(f.path(&original(operation.item().capture.id)).exists());
    fs::remove_file(&path).unwrap();
    let root = f.data.join("inbox");
    let retired = f.data.join("retired-inbox");
    fs::rename(&root, &retired).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(f.files.publish_operation(&operation).is_err());
    assert!(retired.join(original(operation.item().capture.id)).exists());
}

#[test]
fn operation_conversions_move_boxed_records_without_cloning() {
    let f = Fixture::new();
    let operation = f.operation();
    let OriginalOperationFile::Remove(record) = &operation else {
        unreachable!()
    };
    let address = &**record as *const current::InboxOriginalRemovalRecord;
    let stored: current::InboxOriginalOperation = operation.into();
    let current::InboxOriginalOperation::Remove(record) = &stored else {
        unreachable!()
    };
    assert_eq!(&**record as *const _, address);
    let operation: OriginalOperationFile = stored.into();
    let OriginalOperationFile::Remove(record) = &operation else {
        unreachable!()
    };
    assert_eq!(&**record as *const _, address);
    let legacy = decode(include_bytes!("fixtures/inbox-original-mirror-v1.json")).unwrap();
    let OriginalOperationFile::LegacyRemove(record) = &legacy else {
        unreachable!()
    };
    let address = &**record as *const legacy::InboxOriginalRemovalRecord;
    let stored: current::InboxOriginalOperation = legacy.into();
    let current::InboxOriginalOperation::LegacyRemove(record) = &stored else {
        unreachable!()
    };
    assert_eq!(&**record as *const _, address);
    let legacy: OriginalOperationFile = stored.into();
    let OriginalOperationFile::LegacyRemove(record) = &legacy else {
        unreachable!()
    };
    assert_eq!(&**record as *const _, address);
}

#[test]
fn near_limit_escaped_evidence_roundtrips_and_format2_overflow_refuses() {
    let f = Fixture::with_text("\u{0001}".repeat(crate::MAX_NOTE_BYTES - 16 * 1024));
    let operation = f.operation();
    let bytes = encode(&operation).unwrap();
    assert!(bytes.len() > 17 * crate::MAX_NOTE_BYTES);
    assert!(bytes.len() < MAX_NEW_MIRROR_BYTES);
    assert_eq!(encode(&decode(&bytes).unwrap()).unwrap(), bytes);
    f.files.publish_operation(&operation).unwrap();
    assert_eq!(
        encode(&f.files.operation(&operation.name()).unwrap()).unwrap(),
        bytes
    );
    let mut oversized = bytes;
    oversized.resize(MAX_NEW_MIRROR_BYTES + 1, b' ');
    assert!(decode(&oversized).is_err());
    let path = f.path(&operation.name());
    fs::write(&path, &oversized).unwrap();
    assert!(f.files.operation(&operation.name()).is_err());
    assert!(f.path(&original(operation.item().capture.id)).exists());
}

#[test]
fn fault_checkpoints_distinguish_untouched_intent_from_already_observed_move() {
    for (step, observed) in [
        ("original_operation_before_rename", false),
        ("original_operation_renamed", true),
        ("original_operation_synced", true),
    ] {
        let f = Fixture::new();
        let operation = f.operation();
        f.files.publish_operation(&operation).unwrap();
        FAULT.with(|fault| fault.set(Some((step, false))));
        let result = f.files.move_original(&operation);
        FAULT.with(|fault| fault.set(None));
        assert_eq!(result.unwrap_err().kind, ErrorKind::InboxUncertain);
        assert_eq!(
            f.files.operation_effect_observed(&operation).unwrap(),
            observed
        );
        assert_eq!(
            f.path(&original(operation.item().capture.id)).exists(),
            !observed
        );
        assert_eq!(f.path(&retained(operation.id())).exists(), observed);
    }
}
