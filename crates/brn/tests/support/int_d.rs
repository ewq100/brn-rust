//! Test-only exact candidate inspection for explicit synthetic INT-D stores.
//! No Apply/Undo/driver, no owner data discovery or model-prose substitution.
use brn_threads_app::*;
use serde_json::{json, Value};
use std::path::Path;

fn owned_root(path: &Path, prefix: &str, exists: bool) -> std::result::Result<(), String> {
    if !path.is_absolute()
        || path.parent() != Some(Path::new("/private/tmp"))
        || !path
            .file_name()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.starts_with(prefix))
    {
        return Err("explicit task-owned synthetic root required".into());
    }
    if exists {
        if path.canonicalize().map_err(|_| "missing fixture")? != path
            || !std::fs::symlink_metadata(path)
                .map_err(|_| "missing fixture")?
                .is_dir()
        {
            return Err("real existing fixture directory required".into());
        }
    } else if std::fs::symlink_metadata(path).is_ok() {
        return Err("output must be fresh; no overwrite/adoption".into());
    }
    Ok(())
}

fn export(
    fixture: &Path,
    arm: &str,
    operation: &str,
    output: &Path,
    identity: &Value,
) -> std::result::Result<Value, String> {
    owned_root(fixture, "brn-int-d-qualification-20261017-", true)?;
    owned_root(output, "brn-int-d-candidate-export-20261017-", false)?;
    if !matches!(arm, "control" | "candidate") {
        return Err("unknown corpus arm".into());
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(fixture.join("manifest.json")).map_err(|_| "missing corpus manifest")?,
    )
    .map_err(|_| "invalid corpus manifest")?;
    if manifest["corpus_version"] != "BRN-INT-D-20261017-1" {
        return Err("wrong corpus".into());
    }
    let case = manifest["cases"]
        .as_array()
        .ok_or("missing cases")?
        .iter()
        .find(|c| c["case"] == arm)
        .ok_or("missing requested case")?;
    let data = fixture.join(arm);
    if case["data_dir"].as_str() != data.to_str()
        || data.canonicalize().map_err(|_| "missing data")? != data
    {
        return Err("fixture/data identity mismatch".into());
    }
    let app = Workspace::open_existing_no_upgrade(&data).map_err(|_| "fixture cannot reopen")?;
    if app
        .store
        .workspace_purpose()
        .map_err(|_| "missing purpose")?
        != WorkspacePurpose::Test
    {
        return Err("only synthetic test-purpose workspace is eligible".into());
    }
    let info = app
        .store
        .operation_info(operation)
        .map_err(|_| "missing operation")?;
    if info.state != OperationState::Prepared || info.receipt.is_some() {
        return Err("operation is not an active unapplied candidate".into());
    }
    let prepared = app
        .store
        .prepared(&info.id)
        .map_err(|_| "missing prepared content")?;
    let target = case["note"].as_str().ok_or("missing target")?;
    if prepared.request.writes.len() != 1
        || prepared.request.writes[0].id != target
        || !matches!(prepared.request.writes[0].data, RecordData::Note(_))
        || prepared.request.writes[0].expected_version != case["note_version"].as_u64()
    {
        return Err("candidate is not the exact fixture Note/version".into());
    }
    let before = app
        .store
        .record(target)
        .map_err(|_| "cannot read target")?
        .ok_or("missing target")?;
    if Some(before.version) != prepared.request.writes[0].expected_version {
        return Err("current target differs from the prepared version".into());
    }
    let allowed = case["history_messages"]
        .as_array()
        .ok_or("missing history identities")?;
    let mut inputs = vec![];
    for input in &prepared.request.inputs {
        if input.record != target
            && !allowed
                .iter()
                .any(|id| id.as_str() == Some(input.record.as_str()))
        {
            return Err("input is outside the explicit corpus".into());
        }
        inputs.push(
            app.store
                .revision(&input.record, input.version)
                .map_err(|_| "input read failed")?
                .ok_or("missing exact input version")?,
        );
    }
    let value = json!({"kind":"INT-D synthetic exact candidate inspection; NOT owner approval",
        "source_artifact_identity":identity,"exporter_executable":std::env::current_exe().map_err(|_| "missing executable")?,
        "fixture":fixture,"case":case,"operation":prepared.operation,"request_hash":prepared.request_hash,
        "prepared":prepared,"target_current":before,"exact_inputs":inputs,"automatic_apply":false});
    let bytes = serde_json::to_vec_pretty(&value).map_err(|_| "encoding failed")?;
    if bytes.len() > 1024 * 1024 {
        return Err("synthetic inspection exceeds 1MiB; no truncation".into());
    }
    std::fs::create_dir(output).map_err(|_| "output must be fresh")?;
    std::fs::write(output.join("candidate.json"), bytes)
        .map_err(|_| "cannot publish inspection")?;
    Ok(value)
}

#[test]
fn exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content() {
    if let Some(fixture) = std::env::var_os("BRN_INT_D_INSPECT_ROOT") {
        let output =
            std::env::var_os("BRN_INT_D_INSPECT_OUTPUT").expect("explicit fresh output required");
        let arm = std::env::var("BRN_INT_D_INSPECT_ARM").expect("explicit arm required");
        let operation =
            std::env::var("BRN_INT_D_INSPECT_OPERATION").expect("explicit operation required");
        let identity = std::env::var("BRN_INT_D_INSPECT_IDENTITY")
            .expect("pinned source/artifact identity required");
        let identity: Value = serde_json::from_str(&identity).unwrap();
        assert!(identity["source_pin"]
            .as_str()
            .is_some_and(|s| s.len() == 40));
        export(
            Path::new(&fixture),
            &arm,
            &operation,
            Path::new(&output),
            &identity,
        )
        .unwrap();
        return;
    }
    let fixture = tempfile::Builder::new()
        .prefix("brn-int-d-qualification-20261017-export-test-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let data = fixture.path().join("candidate");
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let note = app
        .new_note("Synthetic exporter control", "Original public text")
        .unwrap();
    let message = app.new_thread("Synthetic public export fixture").unwrap();
    let op = app
        .store
        .allocate_operation("int-d-export-known-candidate")
        .unwrap();
    let mut revised = match note.data.clone() {
        RecordData::Note(n) => n,
        _ => panic!(),
    };
    revised.markdown =
        "Exact Monday / esmaspäeval. Õun.\nNo acceptance inferred from export.".into();
    let prepared = app
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Known synthetic exact inspection".into(),
                writes: vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(note.version),
                    archived: false,
                    data: RecordData::Note(revised.clone()),
                }],
                inputs: vec![RevisionInput {
                    record: note.id.clone(),
                    version: note.version,
                    expectation: InputExpectation::Current,
                }],
            },
            &HostAuthority::owner("Synthetic exporter fixture"),
        )
        .unwrap();
    let manifest = json!({"corpus_version":"BRN-INT-D-20261017-1","cases":[{"case":"candidate","data_dir":data,"note":note.id,"note_version":note.version,"history_messages":[],"chat":message.id}]});
    std::fs::write(
        fixture.path().join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let output_guard = tempfile::Builder::new()
        .prefix("brn-int-d-candidate-export-20261017-test-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let output = output_guard.path().to_owned();
    output_guard.close().unwrap();
    let identity =
        json!({"qualification":"deterministic known synthetic fixture; not a live artifact pin"});
    assert!(export(
        fixture.path(),
        "candidate",
        "missing-operation",
        &output,
        &identity
    )
    .is_err());
    assert!(!output.exists());
    assert!(export(fixture.path(), "control", op.as_str(), &output, &identity).is_err());
    assert!(!output.exists());
    let result = export(fixture.path(), "candidate", op.as_str(), &output, &identity).unwrap();
    assert_eq!(result["prepared"], serde_json::to_value(&prepared).unwrap());
    assert_eq!(result["request_hash"], prepared.request_hash);
    assert_eq!(
        result["prepared"]["request"]["writes"][0]["data"]["Note"]["markdown"],
        revised.markdown
    );
    assert_eq!(
        result["exact_inputs"][0],
        serde_json::to_value(&note).unwrap()
    );
    let published = std::fs::read(output.join("candidate.json")).unwrap();
    assert!(export(fixture.path(), "candidate", op.as_str(), &output, &identity).is_err());
    assert_eq!(
        std::fs::read(output.join("candidate.json")).unwrap(),
        published
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    assert_eq!(
        app.store.operation_info(op.as_str()).unwrap().state,
        OperationState::Prepared
    );
    assert!(app
        .store
        .operation_info(op.as_str())
        .unwrap()
        .receipt
        .is_none());
    // A different well-formed operation must not export a different target.
    let other = app
        .new_note("Other synthetic Note", "Not the reviewed target")
        .unwrap();
    let wrong = app
        .store
        .allocate_operation("int-d-export-wrong-target")
        .unwrap();
    app.store
        .prepare_owner(
            &wrong,
            &ChangeRequest {
                reason: "Wrong-target negative".into(),
                writes: vec![Put {
                    id: other.id,
                    expected_version: Some(other.version),
                    archived: false,
                    data: other.data,
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("Synthetic negative"),
        )
        .unwrap();
    std::fs::remove_dir_all(&output).unwrap();
    assert!(export(
        fixture.path(),
        "candidate",
        wrong.as_str(),
        &output,
        &identity
    )
    .is_err());
    assert!(!output.exists());
}

fn version_fixture() -> (
    tempfile::TempDir,
    Workspace,
    Record,
    Prepared,
    std::path::PathBuf,
) {
    let fixture = tempfile::Builder::new()
        .prefix("brn-int-d-qualification-20261017-version-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let data = fixture.path().join("candidate");
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let note = app
        .new_note("Synthetic version fixture", "Original version one")
        .unwrap();
    let op = app
        .store
        .allocate_operation("int-d-version-export")
        .unwrap();
    let mut revised = match note.data.clone() {
        RecordData::Note(n) => n,
        _ => panic!(),
    };
    revised.markdown = "Prepared Monday version".into();
    let prepared = app
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic exact-version candidate".into(),
                writes: vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(note.version),
                    archived: false,
                    data: RecordData::Note(revised),
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("Synthetic candidate control"),
        )
        .unwrap();
    let manifest = json!({"corpus_version":"BRN-INT-D-20261017-1","cases":[{"case":"candidate","data_dir":data,"note":note.id,"note_version":note.version,"history_messages":[]}]});
    std::fs::write(
        fixture.path().join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let guard = tempfile::Builder::new()
        .prefix("brn-int-d-candidate-export-20261017-version-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let output = guard.path().to_owned();
    guard.close().unwrap();
    (fixture, app, note, prepared, output)
}

#[test]
fn stale_current_target_is_refused_before_export_output_creation() {
    let (fixture, mut app, note, prepared, output) = version_fixture();
    let identity = json!({"qualification":"synthetic stale-version witness, no provider"});
    let positive = export(
        fixture.path(),
        "candidate",
        prepared.operation.as_str(),
        &output,
        &identity,
    )
    .unwrap();
    assert_eq!(positive["target_current"]["version"], note.version);
    std::fs::remove_dir_all(&output).unwrap();
    let mut edited = match note.data.clone() {
        RecordData::Note(n) => n,
        _ => panic!(),
    };
    edited.markdown = "Owner changed current version".into();
    app.owner_change(
        "int-d-owner-edit-after-prepare",
        &ChangeRequest {
            reason: "Actual checked owner edit".into(),
            writes: vec![Put {
                id: note.id.clone(),
                expected_version: Some(note.version),
                archived: false,
                data: RecordData::Note(edited),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let current = app.store.record(&note.id).unwrap().unwrap();
    assert_eq!(current.version, note.version + 1);
    assert_eq!(
        app.store
            .operation_info(prepared.operation.as_str())
            .unwrap()
            .state,
        OperationState::Prepared
    );
    let result = export(
        fixture.path(),
        "candidate",
        prepared.operation.as_str(),
        &output,
        &identity,
    );
    assert!(
        result.is_err(),
        "stale current target v{} vs prepared {:?} must refuse; got {result:?}",
        current.version,
        prepared.request.writes[0].expected_version
    );
    assert!(!output.exists());
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), current);
    assert_eq!(app.store.prepared(&prepared.operation).unwrap(), prepared);
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join("owner-sentinel"), b"preserve occupied output").unwrap();
    assert!(export(
        fixture.path(),
        "candidate",
        prepared.operation.as_str(),
        &output,
        &identity
    )
    .is_err());
    assert_eq!(
        std::fs::read(output.join("owner-sentinel")).unwrap(),
        b"preserve occupied output"
    );
    std::fs::remove_dir_all(&output).unwrap();
}

#[test]
fn applied_and_retired_candidates_are_refused_without_export_or_source_changes() {
    for applied in [true, false] {
        let (fixture, mut app, note, prepared, output) = version_fixture();
        if applied {
            assert!(matches!(
                app.store
                    .apply(
                        &prepared.operation,
                        &HostAuthority::owner("Synthetic actual owner Apply")
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
        } else {
            app.review_dismiss(prepared.operation.as_str()).unwrap();
        }
        let info = app
            .store
            .operation_info(prepared.operation.as_str())
            .unwrap();
        assert_eq!(
            info.state,
            if applied {
                OperationState::Applied
            } else {
                OperationState::Retired
            }
        );
        let before = app.store.record(&note.id).unwrap().unwrap();
        assert!(export(
            fixture.path(),
            "candidate",
            prepared.operation.as_str(),
            &output,
            &json!({"qualification":"terminal candidate negative"})
        )
        .is_err());
        assert!(!output.exists());
        assert_eq!(app.store.record(&note.id).unwrap().unwrap(), before);
        assert_eq!(
            app.store
                .operation_info(prepared.operation.as_str())
                .unwrap()
                .state,
            info.state
        );
    }
}

// Offline-only reader of CLOSED fixture copies. Never opens the original
// qualification root and never calls Auth, configure, prepare or a workflow.
pub(super) fn metadata_copy_context(
    copied: &Path,
    expected_path: &Path,
    output: &Path,
) -> std::result::Result<Value, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    owned_root(copied, "brn-int-d-metadata-copy-20261017-", true)?;
    owned_root(output, "brn-int-d-metadata-result-20261017-", false)?;
    if expected_path.parent() != Some(Path::new("/private/tmp"))
        || !expected_path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("brn-int-d-metadata-expected-20261017-"))
        || expected_path
            .canonicalize()
            .map_err(|_| "missing expected context")?
            != expected_path
        || !std::fs::symlink_metadata(expected_path)
            .map_err(|_| "missing expected context")?
            .is_file()
        || std::fs::metadata(expected_path)
            .map_err(|_| "missing expected context")?
            .len()
            > 65536
    {
        return Err("bounded explicit real expected-context file required".into());
    }
    fn real_tree(path: &Path, count: &mut usize) -> std::result::Result<(), String> {
        *count += 1;
        if *count > 128 {
            return Err("copy tree exceeds bound".into());
        }
        let kind = std::fs::symlink_metadata(path).map_err(|_| "copy metadata unavailable")?;
        if kind.is_dir() && !kind.file_type().is_symlink() {
            for entry in std::fs::read_dir(path).map_err(|_| "copy directory unavailable")? {
                real_tree(&entry.map_err(|_| "copy entry unavailable")?.path(), count)?;
            }
        } else if !kind.is_file() || kind.file_type().is_symlink() {
            return Err("copy must contain only real directories and regular files".into());
        }
        Ok(())
    }
    real_tree(copied, &mut 0)?;
    let expected: Value =
        serde_json::from_slice(&std::fs::read(expected_path).map_err(|_| "expected read failed")?)
            .map_err(|_| "invalid expected JSON")?;
    if expected
        .as_object()
        .is_none_or(|o| o.len() != 2 || !o.contains_key("facts") || !o.contains_key("context"))
    {
        return Err("only facts/context metadata is supported".into());
    }
    let f = &expected["facts"];
    let arm = f["case"].as_str().ok_or("missing case")?;
    if !matches!(arm, "control" | "candidate") || f["selected"] != (arm == "candidate") {
        return Err("wrong corpus arm".into());
    }
    let original = Path::new(f["data_dir"].as_str().ok_or("missing original identity")?);
    if original.file_name().and_then(|n| n.to_str()) != Some(arm)
        || original.parent().and_then(Path::parent) != Some(Path::new("/private/tmp"))
        || !original
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("brn-int-d-qualification-20261017-"))
    {
        return Err("explicit original case identity required, never opened".into());
    }
    let app =
        Workspace::open_existing_no_upgrade(copied.join("data")).map_err(|_| "copy cannot open")?;
    if app
        .store
        .workspace_purpose()
        .map_err(|_| "purpose unavailable")?
        != WorkspacePurpose::Test
        || !app
            .candidates()
            .map_err(|_| "candidate inspection failed")?
            .is_empty()
    {
        return Err("fresh Test-purpose copy required".into());
    }
    let records = app.records().map_err(|_| "records unavailable")?;
    if records.len() > 256 || records.iter().any(|r| matches!(r.data, RecordData::Run(_))) {
        return Err("bounded no-Run copy required".into());
    }
    let record = |id: &str| {
        records
            .iter()
            .find(|r| r.id == id && !r.archived)
            .ok_or("context record unavailable")
    };
    let chat = record(f["chat"].as_str().ok_or("missing chat")?)?;
    let RecordData::Thread(t) = &chat.data else {
        return Err("wrong chat type".into());
    };
    let project = app
        .store
        .project_context_record(&chat.id)
        .map_err(|_| "project context unavailable")?
        .ok_or("project required")?;
    if project.archived || project.id != f["project"] || t.state != ThreadState::Open {
        return Err("active same-Project chat required".into());
    }
    let target = record(f["note"].as_str().ok_or("missing target")?)?;
    if target.version != 2
        || f["note_version"] != 2
        || !matches!(&target.data, RecordData::Note(n) if n.protected)
        || !app
            .store
            .record_visible_in_chat(&target.id, Some(&chat.id))
            .map_err(|_| "target context unavailable")?
    {
        return Err("protected visible target v2 required".into());
    }
    let settings_record = records
        .iter()
        .find(|r| !r.archived && matches!(r.data, RecordData::Settings(_)))
        .ok_or("settings unavailable")?;
    let RecordData::Settings(s) = &settings_record.data else {
        unreachable!()
    };
    if s.provider != "Codex"
        || s.model != "gpt-6.1-sol"
        || s.effort != "medium"
        || s.auth_file.as_deref() != Some("/Users/evokessler/.codex/auth.json")
        || s.credentials_dir.is_some()
        || s.helper.is_some()
        || s.mcp.is_some()
    {
        return Err("exact reference-only route required".into());
    }
    let wanted = if arm == "candidate" {
        vec![SkillBinding {
            id: f["skill_id"].as_str().ok_or("missing package")?.into(),
            digest: f["skill_digest"].as_str().ok_or("missing digest")?.into(),
            project: Some(project.id.clone()),
            enabled: true,
            selected: true,
        }]
    } else {
        vec![]
    };
    if s.skills != wanted {
        return Err("wrong scoped package policy".into());
    }
    let package = app
        .inspect_skill(
            f["skill_id"]
                .as_str()
                .ok_or("missing package")?
                .strip_prefix("installed:")
                .ok_or("wrong installed identity")?,
        )
        .map_err(|_| "installed package unavailable")?;
    if package.info.id != f["skill_id"] || package.info.digest != f["skill_digest"] {
        return Err("package bytes differ".into());
    }
    let c: Value = serde_json::from_str(super::CORPUS).map_err(|_| "corpus unavailable")?;
    let ids = f["history_messages"].as_array().ok_or("missing history")?;
    if ids.len() != 68 {
        return Err("68 inputs required".into());
    }
    let mut inputs = vec![];
    for (index, id) in ids.iter().enumerate() {
        let r = record(id.as_str().ok_or("invalid input ID")?)?;
        let RecordData::Message(m) = &r.data else {
            return Err("wrong input type".into());
        };
        if r.version != 1
            || m.thread != chat.id
            || (index == 0 && (m.text != c["early_requirement"] || r.id != f["early_message"]))
            || (index == 40 && (m.text != c["later_proposal"] || r.id != f["proposal_message"]))
        {
            return Err("canonical input differs".into());
        }
        inputs.push(json!({"id":r.id,"version":r.version}));
    }
    inputs.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    if inputs.windows(2).any(|w| w[0] == w[1]) {
        return Err("duplicate input".into());
    }
    // Mac-only test helper, no new dependency: SHA256 the ACTUAL Rust serialized
    // Settings bytes using the platform utility, never re-order keys in Python.
    let bytes = serde_json::to_vec(&serde_json::to_value(s).map_err(|_| "settings value failed")?)
        .map_err(|_| "settings encoding failed")?;
    let mut hash = Command::new("/usr/bin/shasum")
        .args(["-a", "256"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| "hash utility unavailable")?;
    hash.stdin
        .take()
        .ok_or("hash input unavailable")?
        .write_all(&bytes)
        .map_err(|_| "hash write failed")?;
    let hash = hash.wait_with_output().map_err(|_| "hash process failed")?;
    if !hash.status.success() {
        return Err("hash process refused".into());
    }
    let digest = String::from_utf8(hash.stdout).map_err(|_| "invalid hash output")?;
    let digest = digest.split_whitespace().next().ok_or("missing hash")?;
    let actual = json!({"purpose":"Test","project":{"id":project.id,"version":project.version},
        "chat":{"id":chat.id,"version":chat.version,"privacy":t.private_context,"epoch":t.context_epoch},
        "settings":{"id":settings_record.id,"version":settings_record.version,"policy_sha256":digest,"auth_reference":s.auth_file},
        "target":{"id":target.id,"version":target.version,"protected":true},"inputs":inputs,
        "package":{"id":package.info.id,"digest":package.info.digest,"selected":arm=="candidate","bindings":wanted}});
    if actual != expected["context"] {
        return Err("whole Python/Rust canonical context mismatch".into());
    }
    let value = json!({"context":actual,"copy_root":copied,"original_data_opened":false});
    std::fs::create_dir(output).map_err(|_| "fresh output required")?;
    std::fs::write(
        output.join("context.json"),
        serde_json::to_vec_pretty(&value).map_err(|_| "metadata encoding failed")?,
    )
    .map_err(|_| "metadata output failed")?;
    Ok(value)
}
