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
