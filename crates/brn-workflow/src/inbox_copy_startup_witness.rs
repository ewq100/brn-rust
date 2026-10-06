//! Explicit synthetic cost witness. Never selected by ordinary verification.
//! App::open includes WorkStore::open; their timings must not be added together.
use super::*;
use crate::{
    app::AppConfig,
    inbox::{CaptureInboxRequest, InboxKind},
    inbox_removal::tests::Fixture,
};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const MARKER: &str = "b2-synthetic-copy-startup-v1";
const REPEATS: usize = 3;
struct Dataset {
    owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    label: &'static str,
    history: Vec<InboxOriginalOperationSummary>,
}
fn configuration(data: &Path, vault: &Path) -> AppConfig {
    AppConfig {
        vault_root: Some(vault.to_owned()),
        credentials_dir: Some(data.parent().unwrap().join("credentials")),
        model_dir: None,
    }
}
fn milliseconds() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
fn emit(value: serde_json::Value) {
    eprintln!("B2_WITNESS {}", value);
}
fn empty(parent: &Path) -> Dataset {
    let owner = tempfile::tempdir_in(parent).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    drop(App::open(&data, configuration(&data, &vault)).unwrap());
    Dataset {
        owner,
        data,
        vault,
        label: "empty",
        history: vec![],
    }
}
fn confirmed(
    item_id: Uuid,
    preview_digest: [u8; 32],
    previous_restore: Option<InboxOriginalParent>,
) -> RemoveInboxOriginalRequest {
    RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id,
        preview_digest,
        previous_restore,
        confirmation: InboxRemovalConfirmation {
            version: 1,
            exact_copy_removal_intended: true,
        },
    }
}
// Same private exclusive publication/effect fixture used by the qualified
// escaped-size recovery test. Evidence comes from genuine conversion/approval.
fn chain(f: &mut Fixture, count: usize) {
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    assert!(preview.evidence.blockers.is_empty());
    let first = confirmed(f.item, preview.digest, None);
    let evidence = f.app.qualify_original_removal(&first).unwrap();
    let files = f.app.inbox.files.as_ref().unwrap();
    let namespace = files.original_namespace();
    let mut previous = None;
    let mut minimum = evidence.item.received_at_ms;
    for index in 0..count {
        if index % 2 == 0 {
            let mut record = InboxOriginalRemovalRecord {
                request: confirmed(f.item, preview.digest, previous.clone()),
                evidence: evidence.clone(),
                namespace: namespace.clone(),
                prepared_at_ms: now_ms(minimum),
                removed_at_ms: None,
            };
            let operation = OriginalOperationFile::Remove(Box::new(record.clone()));
            files.publish_operation(&operation).unwrap();
            files.move_original(&operation).unwrap();
            record.removed_at_ms = Some(now_ms(record.prepared_at_ms));
            minimum = record.removed_at_ms.unwrap();
            let operation = OriginalOperationFile::Remove(Box::new(record));
            files.publish_operation(&operation).unwrap();
            previous = Some(InboxOriginalParent {
                operation_id: operation.id(),
                record_sha256: InboxOriginalOperation::from(operation).digest().unwrap(),
            });
        } else {
            let parent = previous.as_ref().unwrap();
            let mut record = InboxOriginalRestoreRecord {
                request: RestoreInboxOriginalRequest {
                    operation_id: Uuid::new_v4(),
                    removal_operation_id: parent.operation_id,
                    removal_digest: parent.record_sha256,
                },
                original: evidence.item.clone(),
                namespace: namespace.clone(),
                prepared_at_ms: now_ms(minimum),
                restored_at_ms: None,
            };
            let operation = OriginalOperationFile::Restore(Box::new(record.clone()));
            files.publish_operation(&operation).unwrap();
            files.move_original(&operation).unwrap();
            record.restored_at_ms = Some(now_ms(record.prepared_at_ms));
            minimum = record.restored_at_ms.unwrap();
            let operation = OriginalOperationFile::Restore(Box::new(record));
            files.publish_operation(&operation).unwrap();
            previous = Some(InboxOriginalParent {
                operation_id: operation.id(),
                record_sha256: InboxOriginalOperation::from(operation).digest().unwrap(),
            });
        }
    }
    import_mirrors(&mut f.app.store, files, &files.names().unwrap()).unwrap();
}
fn finish(f: Fixture, label: &'static str, expected: usize) -> Dataset {
    let history = f.app.store.inbox_original_operations(&[]).unwrap().history;
    assert_eq!(history.len(), expected);
    let Fixture {
        _owner: owner,
        data,
        vault,
        app,
        ..
    } = f;
    drop(app);
    Dataset {
        owner,
        data,
        vault,
        label,
        history,
    }
}
fn small(parent: &Path) -> Dataset {
    let mut f = Fixture::new_in(parent);
    f.source();
    chain(&mut f, 256);
    finish(f, "small_256", 256)
}
fn escaped(parent: &Path) -> Dataset {
    let mut f = Fixture::new_in(parent);
    f.item = f
        .app
        .capture_inbox(&CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Escaped near-limit synthetic copy".into(),
            original_name: None,
            text: "\u{0001}".repeat(crate::MAX_NOTE_BYTES - 4096),
        })
        .unwrap()
        .capture
        .id;
    f.source();
    chain(&mut f, 7);
    let dataset = finish(f, "escaped_seven_four_removes", 7);
    let receipt_bytes: u64 = fs::read_dir(dataset.data.join("inbox"))
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.starts_with(".brn-inbox-removal-") && name.ends_with(".receipt")
            })
        })
        .map(|entry| entry.metadata().unwrap().len())
        .sum();
    // All original-operation mirrors are independently size-reported below;
    // the known four large Remove receipts must exceed the old aggregate cap.
    assert!(receipt_bytes > 64 * 1024 * 1024);
    dataset
}
fn mixed(parent: &Path) -> Dataset {
    let mut f = Fixture::new_in(parent);
    f.source();
    let mut old = extra_tests::legacy_record(&mut f);
    let files = f.app.inbox.files.as_ref().unwrap();
    let operation = OriginalOperationFile::LegacyRemove(Box::new(old.clone()));
    files.publish_operation(&operation).unwrap();
    files.move_original(&operation).unwrap();
    old.removed_at_ms = Some(now_ms(old.prepared_at_ms));
    files
        .publish_operation(&OriginalOperationFile::LegacyRemove(Box::new(old.clone())))
        .unwrap();
    import_mirrors(&mut f.app.store, files, &files.names().unwrap()).unwrap();
    let restored = f
        .app
        .restore_inbox_original(&RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: old.request.operation_id,
            removal_digest: old.digest().unwrap(),
        })
        .unwrap();
    let preview = f.app.preview_inbox_removal(f.item).unwrap();
    f.app
        .remove_inbox_original(&confirmed(
            f.item,
            preview.digest,
            Some(InboxOriginalParent {
                operation_id: restored.request.operation_id,
                record_sha256: restored.digest().unwrap(),
            }),
        ))
        .unwrap();
    finish(f, "mixed_legacy_new", 3)
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct FileProof {
    inode: u64,
    sha256: [u8; 32],
    bytes: u64,
}
fn files(path: &Path) -> BTreeMap<String, FileProof> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<String, FileProof>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let metadata = fs::symlink_metadata(entry.path()).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "synthetic fixtures contain no links"
            );
            if metadata.is_dir() {
                visit(root, &entry.path(), result);
            } else {
                assert!(metadata.is_file());
                let content = fs::read(entry.path()).unwrap();
                result.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                    FileProof {
                        inode: metadata.ino(),
                        sha256: sha256(&content),
                        bytes: content.len() as u64,
                    },
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    if path.exists() {
        visit(path, path, &mut result);
    }
    result
}
fn directory_identity(path: &Path) -> serde_json::Value {
    assert_eq!(path.canonicalize().unwrap(), path);
    let metadata = fs::symlink_metadata(path).unwrap();
    assert!(metadata.is_dir());
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    json!({"path":path,"device":metadata.dev(),"inode":metadata.ino(),"uid":metadata.uid()})
}
fn binding(dataset: &Dataset) -> serde_json::Value {
    json!({"format":1,"marker":MARKER,"label":dataset.label,
        "root":directory_identity(dataset.owner.path()),
        "data":directory_identity(&dataset.data),"vault":directory_identity(&dataset.vault)})
}
fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).into()
}
fn bytes(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                bytes(&entry.path())
            } else {
                metadata.len()
            }
        })
        .sum()
}
fn prepare_scenario(dataset: &Dataset, scenario: &str) {
    assert_eq!(
        fs::read_to_string(dataset.owner.path().join("witness.marker")).unwrap(),
        MARKER
    );
    let retained: serde_json::Value = serde_json::from_slice(
        &fs::read(dataset.owner.path().join("witness.binding.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(retained, binding(dataset));
    match scenario {
        "healthy" => {}
        "sql_only_repair" => {
            let inbox = dataset.data.join("inbox");
            if inbox.exists() {
                for entry in fs::read_dir(inbox).unwrap() {
                    let entry = entry.unwrap();
                    let name = entry.file_name().into_string().unwrap();
                    if original_operation_name(&name) {
                        fs::remove_file(entry.path()).unwrap();
                    }
                }
            }
        }
        "fresh_sql_recovery" => {
            // Artificial reset of this explicitly generated fixture only. Backups
            // must be absent to exercise ordinary-mirror recovery rather than restore.
            for name in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
                let path = dataset.data.join(name);
                if path.exists() {
                    fs::remove_file(path).unwrap();
                }
            }
            let backups = dataset.data.join("backups");
            if backups.exists() {
                for entry in fs::read_dir(&backups).unwrap() {
                    let entry = entry.unwrap();
                    let name = entry.file_name().into_string().unwrap();
                    assert!(name.starts_with("brn-") && name.ends_with(".sqlite"));
                    assert!(fs::symlink_metadata(entry.path()).unwrap().is_file());
                    fs::remove_file(entry.path()).unwrap();
                }
                fs::remove_dir(backups).unwrap();
            }
        }
        _ => panic!("fixed witness scenario"),
    }
}
fn probe(
    dataset: &Dataset,
    scenario: &str,
    repeat: usize,
    expected_inbox: &BTreeMap<String, FileProof>,
    expected_vault: &BTreeMap<String, FileProof>,
) -> (u128, u128) {
    prepare_scenario(dataset, scenario);
    let inbox = files(&dataset.data.join("inbox"));
    let before_bytes = bytes(&dataset.data);
    let began_at = milliseconds();
    let started = Instant::now();
    let opened = WorkStore::open(&dataset.data);
    let store_ns = started.elapsed().as_nanos();
    emit(
        json!({"dataset":dataset.label,"scenario":scenario,"repeat":repeat,"phase":"store_open",
        "started_at_ms":began_at,"finished_at_ms":milliseconds(),"elapsed_ns":store_ns,
        "success":opened.is_ok(),"error":opened.as_ref().err().map(ToString::to_string),"data_bytes_before":before_bytes}),
    );
    drop(opened.unwrap());
    // Restore the requested starting state again outside timing. App::open must
    // exercise fresh SQL recovery independently of the Store-only observation.
    prepare_scenario(dataset, scenario);
    let began_at = milliseconds();
    let started = Instant::now();
    let opened = App::open(&dataset.data, configuration(&dataset.data, &dataset.vault));
    let app_ns = started.elapsed().as_nanos();
    emit(
        json!({"dataset":dataset.label,"scenario":scenario,"repeat":repeat,"phase":"app_open_including_store",
        "started_at_ms":began_at,"finished_at_ms":milliseconds(),"elapsed_ns":app_ns,
        "success":opened.is_ok(),"error":opened.as_ref().err().map(ToString::to_string)}),
    );
    let app = opened.unwrap();
    let inventory = app.store.inbox_original_operations(&[]).unwrap();
    assert_eq!(inventory.history, dataset.history);
    // Existing counter is a separate post-timing probe, not total startup work.
    emit(
        json!({"dataset":dataset.label,"scenario":scenario,"repeat":repeat,"phase":"post_timing_integrity_probe",
        "inventory_probe_parsed_bodies":inventory.parsed_bodies,"history_rows":inventory.history.len(),
        "data_bytes_after":bytes(&dataset.data),"backup_bytes":bytes(&dataset.data.join("backups")),
        "backup_count":fs::read_dir(dataset.data.join("backups")).unwrap().count()}),
    );
    assert_eq!(files(&dataset.vault), *expected_vault);
    let after = files(&dataset.data.join("inbox"));
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        expected_inbox.keys().collect::<Vec<_>>()
    );
    for (name, proof) in expected_inbox {
        // All repaired mirrors must have exact original bytes. Original and
        // retained endpoints keep their inode too; no startup move/resurrection.
        let actual = after.get(name).unwrap();
        assert_eq!(
            (actual.sha256, actual.bytes),
            (proof.sha256, proof.bytes),
            "startup changed {name}"
        );
        if let Some(before) = inbox.get(name) {
            assert_eq!(actual, before, "startup replaced existing {name}");
        }
    }
    assert!(fs::read_dir(dataset.data.join("backups")).unwrap().count() <= 5);
    (store_ns, app_ns)
}
fn spread(mut samples: Vec<u128>) -> serde_json::Value {
    samples.sort_unstable();
    json!({"samples_ns":samples,"min_ns":samples[0],"median_ns":samples[samples.len()/2],"max_ns":samples[samples.len()-1]})
}
fn source_identity() -> serde_json::Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let git = |arguments: &[&str]| {
        let output = std::process::Command::new("git")
            .args(arguments)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    json!({"head":git(&["rev-parse","HEAD"]).trim(),"branch":git(&["branch","--show-current"]).trim(),
        "status":git(&["status","--short"]),"tracked_diff_sha256":sha256(git(&["diff","HEAD"]).as_bytes()),
        "compiled_witness_sha256":sha256(include_bytes!("inbox_copy_startup_witness.rs")),
        "compiled_fixture_sha256":sha256(include_bytes!("inbox_removal.rs")),
        "lock_sha256":sha256(&fs::read(root.join("Cargo.lock")).unwrap()),
        "toolchain":include_str!("../../../rust-toolchain.toml"),
        "runtime_toolchain":std::env::var("RUSTUP_TOOLCHAIN").ok(),
        "os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
        "profile":if cfg!(debug_assertions) {"debug-test"} else {"release-test"},
        "features":std::env::var("BRN_COPY_STARTUP_WITNESS_FEATURES").expect("explicit feature description required"),
        "command":std::env::var("BRN_COPY_STARTUP_WITNESS_COMMAND").expect("exact invocation required")})
}
#[test]
#[ignore = "explicit potentially slow synthetic B2 cost witness; never a regular gate"]
fn synthetic_copy_startup_cost_witness() {
    let mode = std::env::var("BRN_COPY_STARTUP_WITNESS_MODE")
        .expect("explicit fixtures-only or full mode required");
    assert!(
        matches!(mode.as_str(), "fixtures-only" | "full"),
        "unknown witness mode"
    );
    let identity = source_identity();
    emit(
        json!({"phase":"identity","mode":mode,"identity":identity,"started_at_ms":milliseconds()}),
    );
    let parent = PathBuf::from(
        std::env::var_os("BRN_COPY_STARTUP_WITNESS_PARENT")
            .expect("explicit owned synthetic parent required"),
    );
    assert!(parent.is_absolute());
    assert_eq!(parent.canonicalize().unwrap(), parent);
    let metadata = fs::symlink_metadata(&parent).unwrap();
    assert!(metadata.is_dir());
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(metadata.permissions().mode() & 0o077, 0);
    let run = tempfile::Builder::new()
        .prefix("brn-b2-witness-")
        .tempdir_in(&parent)
        .unwrap();
    let setup_start = Instant::now();
    let datasets = vec![
        empty(run.path()),
        small(run.path()),
        mixed(run.path()),
        escaped(run.path()),
    ];
    emit(
        json!({"phase":"fixture_creation","elapsed_ns":setup_start.elapsed().as_nanos(),"success":true}),
    );
    let mut manifest = Vec::new();
    for dataset in datasets {
        fs::write(dataset.owner.path().join("witness.marker"), MARKER).unwrap();
        let bound = binding(&dataset);
        fs::write(
            dataset.owner.path().join("witness.binding.json"),
            serde_json::to_vec_pretty(&bound).unwrap(),
        )
        .unwrap();
        let inbox = files(&dataset.data.join("inbox"));
        let vault = files(&dataset.vault);
        let mirror_bytes: u64 = inbox
            .iter()
            .filter(|(name, _)| original_operation_name(name))
            .map(|(_, proof)| proof.bytes)
            .sum();
        emit(
            json!({"dataset":dataset.label,"phase":"fixture_size","history_rows":dataset.history.len(),
            "mirror_encoded_bytes":mirror_bytes,"inbox_files":inbox,"data_bytes":bytes(&dataset.data),"vault_bytes":bytes(&dataset.vault)}),
        );
        if mode == "full" {
            for scenario in ["healthy", "sql_only_repair", "fresh_sql_recovery"] {
                let mut store = Vec::new();
                let mut app = Vec::new();
                for repeat in 0..REPEATS {
                    let (store_ns, app_ns) = probe(&dataset, scenario, repeat, &inbox, &vault);
                    store.push(store_ns);
                    app.push(app_ns);
                }
                emit(
                    json!({"dataset":dataset.label,"scenario":scenario,"phase":"repeat_spread",
                "store_open":spread(store),"app_open_including_store":spread(app),"cache_policy":"warm repeats; no OS cache eviction"}),
                );
            }
        } else {
            emit(
                json!({"dataset":dataset.label,"phase":"fixtures_only","startup_timings_performed":false}),
            );
        }
        let backups = fs::read_dir(dataset.data.join("backups")).unwrap().count();
        assert!((1..=5).contains(&backups));
        assert_eq!(binding(&dataset), bound);
        // Final proofs include legitimate repaired mirror inodes for the next,
        // separately built shipping CLI run; canonical bytes remain unchanged.
        manifest.push(json!({"binding":bound,"inbox_files":files(&dataset.data.join("inbox")),
            "vault_files":files(&dataset.vault),"history":dataset.history,"mirror_encoded_bytes":mirror_bytes}));
        dataset.owner.keep();
    }
    let manifest_path = run.path().join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&json!({"format":1,"marker":MARKER,
        "run":directory_identity(run.path()),"identity":identity,"mode":mode,"datasets":manifest}))
        .unwrap(),
    )
    .unwrap();
    emit(json!({"phase":"shipping_fixtures_retained","manifest":manifest_path,"success":true}));
    run.keep();
}
