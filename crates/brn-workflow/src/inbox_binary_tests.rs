use super::*;

fn request(bytes: Vec<u8>) -> CaptureBinaryInboxRequest {
    CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Opaque original õ".into(),
        original_name: Some("../supplied-name.pdf".into()),
        bytes,
    }
}
fn symbolic_item(r: &CaptureBinaryInboxRequest) -> InboxItem {
    InboxItem {
        capture: InboxCapture {
            id: r.id,
            kind: InboxKind::Binary,
            title: r.title.clone(),
            original_name: r.original_name.clone(),
            copy: brn_store::work::inbox::InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: r.bytes.len() as u64,
                sha256: Sha256::digest(&r.bytes).into(),
            },
        },
        received_at_ms: 1,
    }
}
#[test]
fn binary_request_bounds_and_complete_dto_receipts_are_explicit() {
    let r = request(vec![0, 0xff, 0x80, b'a']);
    let item = symbolic_item(&r);
    r.validate_receipt(&item).unwrap();
    let good = InboxRead {
        item: item.clone(),
        original: InboxOriginal::AvailableBinary {
            byte_len: item.capture.copy.byte_len,
            sha256: item.capture.copy.sha256,
        },
    };
    good.validate_receipt().unwrap();
    for field in 0..5 {
        let mut fork = r.clone();
        match field {
            0 => fork.id = Uuid::new_v4(),
            1 => fork.title.push('x'),
            2 => fork.original_name = None,
            3 => fork.bytes.push(0),
            _ => fork.bytes[0] ^= 1,
        }
        assert!(fork.validate_receipt(&item).is_err());
    }
    let mut wrong = good.clone();
    wrong.original = InboxOriginal::AvailableBinary {
        byte_len: 4,
        sha256: [0; 32],
    };
    assert!(wrong.validate_receipt().is_err());
    wrong.original = InboxOriginal::AvailableBinary {
        byte_len: 3,
        sha256: item.capture.copy.sha256,
    };
    assert!(wrong.validate_receipt().is_err());
    wrong.original = InboxOriginal::Available {
        text: "body".into(),
    };
    assert!(wrong.validate_receipt().is_err());
    wrong.original = InboxOriginal::RemovedRetained {
        operation_id: Uuid::new_v4(),
    };
    assert!(wrong.validate_receipt().is_err());
    wrong.original = good.original;
    wrong.item.capture.kind = InboxKind::Text;
    assert!(wrong.validate_receipt().is_err());
    for original in [
        InboxOriginal::Missing,
        InboxOriginal::Changed {
            reason: "changed".into(),
        },
        InboxOriginal::Unavailable {
            reason: "unavailable".into(),
        },
    ] {
        InboxRead {
            item: item.clone(),
            original,
        }
        .validate_receipt()
        .unwrap();
    }
    let text = CaptureInboxRequest {
        id: r.id,
        kind: InboxKind::Binary,
        title: r.title,
        original_name: r.original_name,
        text: "valid UTF-8".into(),
    };
    assert!(text.validate().unwrap_err().message.contains("Binary"));
    let mut limit = request(vec![0; MAX_INBOX_BINARY_BYTES]);
    limit.validate().unwrap();
    limit.bytes.push(0);
    assert!(limit.validate().is_err());
    limit.bytes.clear();
    limit.validate().unwrap();
    limit.id = Uuid::nil();
    assert!(limit.validate().is_err());
    limit.id = Uuid::new_v4();
    for label in ["\n".to_owned(), "x".repeat(513)] {
        limit.title = label;
        assert!(limit.validate().is_err());
    }
}

#[cfg(target_os = "macos")]
mod files {
    use super::*;
    use crate::{
        app::AppConfig,
        files::inbox::{FAULT, OBSERVE_HOOK},
    };
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::{Path, PathBuf},
    };
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        fs::create_dir(&data).unwrap();
        (base, data)
    }
    fn config(data: &Path) -> AppConfig {
        AppConfig {
            vault_root: None,
            credentials_dir: Some(data.parent().unwrap().join("credentials")),
            model_dir: None,
        }
    }
    fn proof(r: &CaptureBinaryInboxRequest) -> InboxOriginal {
        InboxOriginal::AvailableBinary {
            byte_len: r.bytes.len() as u64,
            sha256: Sha256::digest(&r.bytes).into(),
        }
    }
    fn private_write(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    #[test]
    fn binary_exact_limit_retains_nontext_bytes_and_replays_without_rewriting() {
        let (_base, data) = fixture();
        let mut app = App::open(&data, config(&data)).unwrap();
        for bytes in [
            vec![],
            vec![0, 0xff, 0x80, b'\n'],
            vec![0x80; MAX_INBOX_BINARY_BYTES],
        ] {
            let r = request(bytes);
            let item = app.capture_binary_inbox(&r).unwrap();
            let path = data.join("inbox").join(item.capture.copy_name());
            assert_eq!(fs::read(&path).unwrap(), r.bytes);
            assert!(!path.with_extension("txt").exists());
            let before = path.metadata().unwrap().ino();
            let receipt = data
                .join("inbox")
                .join(format!(".brn-inbox-{}.receipt", r.id));
            let receipt_bytes = fs::read(&receipt).unwrap();
            let receipt_inode = receipt.metadata().unwrap().ino();
            let read = app.inbox_item(r.id).unwrap();
            read.validate_receipt().unwrap();
            assert_eq!(read.original, proof(&r));
            assert_eq!(
                app.inbox
                    .files
                    .as_ref()
                    .unwrap()
                    .read_binary_bytes(&item)
                    .unwrap()
                    .as_deref(),
                Some(r.bytes.as_slice())
            );
            assert_eq!(app.capture_binary_inbox(&r).unwrap(), item);
            assert_eq!(path.metadata().unwrap().ino(), before);
            assert_eq!(fs::read(&receipt).unwrap(), receipt_bytes);
            assert_eq!(receipt.metadata().unwrap().ino(), receipt_inode);
            let mut changed = r.clone();
            changed.title.push('x');
            assert_eq!(
                app.capture_binary_inbox(&changed).unwrap_err().kind,
                ErrorKind::OperationConflict
            );
            fs::remove_file(&path).unwrap();
            assert_eq!(app.capture_binary_inbox(&r).unwrap(), item);
            assert_eq!(
                app.inbox_item(r.id).unwrap().original,
                InboxOriginal::Missing
            );
        }
        let count = app
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .total_count;
        let overflow = request(vec![0; MAX_INBOX_BINARY_BYTES + 1]);
        assert!(app.capture_binary_inbox(&overflow).is_err());
        assert_eq!(
            app.inbox_items(&InboxListRequest::default())
                .unwrap()
                .total_count,
            count
        );
        assert!(
            !data
                .join("inbox")
                .join(format!(".brn-inbox-{}.stage", overflow.id))
                .exists()
        );
    }
    #[test]
    fn binary_current_proof_refuses_identity_links_permissions_and_midread_replacement() {
        for damage in 0..6 {
            let (_base, data) = fixture();
            let mut app = App::open(&data, config(&data)).unwrap();
            let r = request(vec![0, 0x80, 0xff]);
            let item = app.capture_binary_inbox(&r).unwrap();
            let path = data.join("inbox").join(item.capture.copy_name());
            match damage {
                0 => private_write(&path, &[0, 0x80, 0xfe]),
                1 => {
                    fs::rename(&path, path.with_extension("kept")).unwrap();
                    private_write(&path, &r.bytes);
                }
                2 => fs::hard_link(&path, path.with_extension("alias")).unwrap(),
                3 => {
                    fs::rename(&path, path.with_extension("kept")).unwrap();
                    std::os::unix::fs::symlink(path.with_extension("kept"), &path).unwrap();
                }
                4 => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
                _ => {
                    let captured = path.clone();
                    let bytes = r.bytes.clone();
                    OBSERVE_HOOK.with(|hook| {
                        *hook.borrow_mut() = Some(Box::new(move || {
                            fs::rename(&captured, captured.with_extension("kept")).unwrap();
                            private_write(&captured, &bytes);
                        }))
                    });
                }
            }
            let result = app.inbox_item(r.id).unwrap();
            assert!(
                !matches!(result.original, InboxOriginal::AvailableBinary { .. }),
                "damage {damage}"
            );
            assert!(
                app.inbox
                    .files
                    .as_ref()
                    .unwrap()
                    .read_binary_bytes(&item)
                    .is_err()
            );
            assert_eq!(app.capture_binary_inbox(&r).unwrap(), item);
        }
    }
    #[test]
    fn either_suffix_and_shared_artifacts_are_exclusive_for_both_capture_types() {
        for binary in [true, false] {
            for suffix in ["txt", "bin", "stage", "receipt"] {
                let (_base, data) = fixture();
                let mut app = App::open(&data, config(&data)).unwrap();
                app.capture_binary_inbox(&request(vec![1])).unwrap();
                let r = request(vec![0x80]);
                let name = if ["stage", "receipt"].contains(&suffix) {
                    format!(".brn-inbox-{}.{suffix}", r.id)
                } else {
                    format!("{}.{suffix}", r.id)
                };
                let path = data.join("inbox").join(name);
                private_write(&path, b"unowned artifact");
                let inode = path.metadata().unwrap().ino();
                let result = if binary {
                    app.capture_binary_inbox(&r)
                } else {
                    app.capture_inbox(&CaptureInboxRequest {
                        id: r.id,
                        kind: InboxKind::Text,
                        title: r.title.clone(),
                        original_name: None,
                        text: "text".into(),
                    })
                };
                assert_eq!(result.unwrap_err().kind, ErrorKind::InboxUncertain);
                assert_eq!(fs::read(&path).unwrap(), b"unowned artifact");
                assert_eq!(path.metadata().unwrap().ino(), inode);
                assert!(app.store.inbox_item(r.id).unwrap().is_none());
            }
        }
    }
    #[test]
    fn binary_receipts_recover_healthy_older_and_fresh_sql_without_suffix_fallback() {
        for mode in 0..3 {
            let (base, data) = fixture();
            let mut app = App::open(&data, config(&data)).unwrap();
            drop(app);
            let baseline = fs::read(data.join("brn.sqlite")).unwrap();
            app = App::open(&data, config(&data)).unwrap();
            let r = request(vec![0, 0xff, 0x80, b'a']);
            let item = app.capture_binary_inbox(&r).unwrap();
            let path = data.join("inbox").join(item.capture.copy_name());
            let inode = path.metadata().unwrap().ino();
            let receipt = data
                .join("inbox")
                .join(format!(".brn-inbox-{}.receipt", r.id));
            let receipt_bytes = fs::read(&receipt).unwrap();
            drop(app);
            if mode != 0 {
                for name in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
                    let path = data.join(name);
                    if path.exists() {
                        fs::rename(&path, base.path().join(format!("retired-{name}"))).unwrap();
                    }
                }
                fs::rename(data.join("backups"), base.path().join("retired-backups")).unwrap();
                if mode == 1 {
                    fs::write(data.join("brn.sqlite"), baseline).unwrap();
                }
            }
            let mut app = App::open(&data, config(&data)).unwrap();
            assert_eq!(app.inbox_item(r.id).unwrap().original, proof(&r));
            assert_eq!(app.capture_binary_inbox(&r).unwrap(), item);
            assert_eq!(path.metadata().unwrap().ino(), inode);
            assert_eq!(fs::read(&path).unwrap(), r.bytes);
            assert_eq!(fs::read(&receipt).unwrap(), receipt_bytes);
            fs::rename(&path, path.with_extension("txt")).unwrap();
            assert_eq!(
                app.inbox_item(r.id).unwrap().original,
                InboxOriginal::Missing
            );
            drop(app);
            let app = App::open(&data, config(&data)).unwrap();
            assert_ne!(app.inbox_item(r.id).unwrap().original, proof(&r));
            assert!(!path.exists());
            assert_eq!(fs::read(path.with_extension("txt")).unwrap(), r.bytes);
        }
    }
    #[test]
    fn non_docx_binary_processing_fails_durably_and_cleanup_stays_unsupported() {
        let (_base, data) = fixture();
        let mut app = App::open(&data, config(&data)).unwrap();
        let r = request(b"looks like text".to_vec());
        let item = app.capture_binary_inbox(&r).unwrap();
        let process = brn_store::work::inbox_processing::ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![item.clone()],
        };
        app.process_inbox(&process).unwrap();
        let failed = app
            .advance_inbox_processing(process.id, &std::sync::atomic::AtomicBool::new(false))
            .unwrap();
        assert!(
            matches!(&failed.entries[0].outcome, crate::inbox_processing::InboxProcessOutcome::Failed { code } if code == "binary_unsupported")
        );
        assert_eq!(app.process_inbox(&process).unwrap(), failed);
        assert!(
            app.inbox_candidate(&crate::inbox_processing::InboxCandidateRequest {
                batch_id: process.id,
                index: 0
            })
            .is_err()
        );
        assert!(
            app.preview_inbox_removal(r.id)
                .unwrap_err()
                .message
                .contains("Binary")
        );
        assert!(app.inbox_original_operations(r.id).unwrap().is_empty());
        assert_eq!(app.inbox_item(r.id).unwrap().original, proof(&r));
        let forged = crate::inbox_removal::InboxRemovalEvidence {
            item,
            original: proof(&r),
            source: None,
            blockers: vec![
                crate::inbox_removal::InboxRemovalBlocker::OriginalUnavailable,
                crate::inbox_removal::InboxRemovalBlocker::SourceRequired,
            ],
            needs_owner_confirmation: true,
        };
        assert!(forged.digest().unwrap_err().message.contains("Binary"));
    }
    #[test]
    fn binary_publication_failures_reconcile_only_published_exact_receipts() {
        for step in [
            "original_durable",
            "mirror_durable",
            "original_installed",
            "catalog_settled",
        ] {
            let (_base, data) = fixture();
            let mut app = App::open(&data, config(&data)).unwrap();
            let r = request(vec![0, 0x80, 0xff]);
            FAULT.with(|f| f.set(Some((step, false))));
            assert_eq!(
                app.capture_binary_inbox(&r).unwrap_err().kind,
                ErrorKind::InboxUncertain
            );
            FAULT.with(|f| f.set(None));
            let stage = data
                .join("inbox")
                .join(format!(".brn-inbox-{}.stage", r.id));
            drop(app);
            let mut app = App::open(&data, config(&data)).unwrap();
            if step == "original_durable" {
                assert!(app.inbox_item(r.id).is_err());
                assert_eq!(fs::read(&stage).unwrap(), r.bytes);
                assert!(!data.join("inbox").join(format!("{}.bin", r.id)).exists());
            } else {
                let read = app.inbox_item(r.id).unwrap();
                assert_eq!(read.original, proof(&r));
                assert_eq!(app.capture_binary_inbox(&r).unwrap(), read.item);
                assert!(!stage.exists());
            }
        }
    }
    #[test]
    fn binary_crash_windows_recover_only_a_known_explicit_capture() {
        let _fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        for step in [
            "original_durable",
            "mirror_durable",
            "original_installed",
            "catalog_settled",
        ] {
            let (_base, data) = fixture();
            let r = request(vec![0, 0xff, 0x80]);
            let input = data.parent().unwrap().join("binary-input.json");
            fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "inbox::binary_tests::files::binary_crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("BRN_BINARY_SYNTHETIC_DATA", &data)
                .env("BRN_BINARY_SYNTHETIC_INPUT", &input)
                .env("BRN_BINARY_SYNTHETIC_STEP", step)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(91),
                "{step}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let app = App::open(&data, config(&data)).unwrap();
            if step == "original_durable" {
                assert!(app.inbox_item(r.id).is_err());
                assert_eq!(
                    fs::read(
                        data.join("inbox")
                            .join(format!(".brn-inbox-{}.stage", r.id))
                    )
                    .unwrap(),
                    r.bytes
                );
                assert!(!data.join("inbox").join(format!("{}.bin", r.id)).exists());
            } else {
                assert_eq!(app.inbox_item(r.id).unwrap().original, proof(&r));
                let mirror = app.inbox.files.as_ref().unwrap().mirror(r.id).unwrap();
                r.validate_receipt(&mirror).unwrap();
            }
        }
    }
    #[test]
    #[ignore = "invoked only by the binary crash parent with an exclusive synthetic fixture"]
    fn binary_crash_child() {
        let Some(data) = std::env::var_os("BRN_BINARY_SYNTHETIC_DATA") else {
            return;
        };
        let data = PathBuf::from(data);
        let input = PathBuf::from(std::env::var_os("BRN_BINARY_SYNTHETIC_INPUT").unwrap());
        assert_eq!(input.parent(), data.parent());
        assert_eq!(data.file_name().unwrap(), "data");
        let r: CaptureBinaryInboxRequest =
            serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
        let step = match std::env::var("BRN_BINARY_SYNTHETIC_STEP").unwrap().as_str() {
            "original_durable" => "original_durable",
            "mirror_durable" => "mirror_durable",
            "original_installed" => "original_installed",
            "catalog_settled" => "catalog_settled",
            _ => panic!("unknown private checkpoint"),
        };
        let mut app = App::open(&data, config(&data)).unwrap();
        FAULT.with(|f| f.set(Some((step, true))));
        let _ = app.capture_binary_inbox(&r);
        panic!("checkpoint did not terminate the child");
    }
}
