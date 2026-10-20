//! Create an explicit offline workspace with two independent generic Review groups.
//!
//! The fixture seeds two Open Actions through the owner service, then prepares
//! one immutable candidate that changes both Actions. It does not apply the
//! candidate or contact a provider.
use brn_threads_app::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf};

const FIXTURE_ID: &str = "native-review-groups-v1";
const SOURCES: [(&str, &str); 2] = [
    ("review-groups-source-venue", "Call the venue"),
    ("review-groups-source-room", "Book the room"),
];
const CANDIDATES: [&str; 2] = [
    "Call the venue and confirm the available dates",
    "Book the room after confirming capacity",
];

struct SeededSource {
    operation_id: String,
    record: Record,
}

fn usage() -> &'static str {
    "Usage: native_review_groups_fixture --output /absolute/new-root"
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn owner_seed(app: &mut Workspace, key: &str, description: &str) -> AppResult<SeededSource> {
    let operation = app.store.allocate_operation(key)?;
    let outcome = app.owner_change(
        key,
        &ChangeRequest {
            reason: "Synthetic native Review groups fixture source".into(),
            writes: vec![Put {
                id: operation.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    kept_outside_project: false,
                    chat: None,
                    description: description.into(),
                    state: ActionState::Open,
                    internal: false,
                    evidence: None,
                    due: None,
                }),
            }],
            inputs: vec![],
        },
    )?;
    let ApplyOutcome::Applied(receipt) = outcome else {
        return Err(AppError::Invalid(format!(
            "owner seed {key} did not apply: {outcome:?}"
        )));
    };
    let operation_id = receipt.operation.as_str().to_owned();
    let record = receipt
        .writes
        .into_iter()
        .map(|written| written.after)
        .find(|record| matches!(record.data, RecordData::Action(_)))
        .ok_or_else(|| AppError::Invalid(format!("owner seed {key} wrote no Action")))?;
    Ok(SeededSource {
        operation_id,
        record,
    })
}

fn candidate_put(record: &Record, description: &str) -> AppResult<Put> {
    let RecordData::Action(mut action) = record.data.clone() else {
        return Err(AppError::Invalid(format!(
            "fixture source {} is not an Action",
            record.id
        )));
    };
    action.description = description.into();
    Ok(Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived: false,
        data: RecordData::Action(action),
    })
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--output" {
        return Err(usage().into());
    }
    let root = PathBuf::from(&args[1]);
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("--output must name a new absolute root directory".into());
    }
    let parent = root.parent().ok_or("--output has no parent directory")?;
    if fs::canonicalize(parent)? != parent {
        return Err("--output parent must exist and use its canonical path".into());
    }

    // Atomically reserve the exact root; this refuses an existing file or directory.
    fs::create_dir(&root)?;
    let mut app = Workspace::open_without_default_credentials(root.join("workspace"))?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;
    let offline_settings = app.settings()?;
    if offline_settings.provider != "offline-fixture"
        || offline_settings.auth_file.is_some()
        || offline_settings.credentials_dir.is_some()
    {
        return Err("fixture workspace settings are not explicitly offline".into());
    }

    let initial = SOURCES
        .iter()
        .map(|(key, description)| owner_seed(&mut app, key, description))
        .collect::<AppResult<Vec<_>>>()?;
    let expected_request = ChangeRequest {
        reason: "Synthetic two-root native Review groups fixture candidate".into(),
        writes: initial
            .iter()
            .zip(CANDIDATES)
            .map(|(seed, description)| candidate_put(&seed.record, description))
            .collect::<AppResult<Vec<_>>>()?,
        inputs: vec![],
    };
    let candidate_host_key = "native-review-groups-candidate";
    let operation = app.store.allocate_operation(candidate_host_key)?;
    let prepared = app.store.prepare(&operation, &expected_request)?;

    let mut canonical_records_unchanged = true;
    for seed in &initial {
        let record = &seed.record;
        canonical_records_unchanged &= app.store.record(&record.id)?.as_ref() == Some(record);
    }
    if prepared.request != expected_request
        || !canonical_records_unchanged
        || app.store.receipt(&operation)?.is_some()
    {
        return Err("prepared candidate is not the exact unchanged two-group fixture".into());
    }

    // The desktop's Home Review changes section and thread sidebar are both
    // populated from Workspace::candidates(). Reopen to prove normal discovery
    // does not depend on in-memory state or a Review-attention/run association.
    drop(app);
    let app = Workspace::open_without_default_credentials(root.join("workspace"))?;
    let persisted = app
        .candidates()?
        .into_iter()
        .find(|candidate| candidate.operation == operation)
        .ok_or("prepared candidate is not discoverable after workspace reopen")?;
    let groups = app.review_groups(operation.as_str())?;
    let canonical_records_unchanged = initial
        .iter()
        .map(|seed| app.store.record(&seed.record.id))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .zip(&initial)
        .all(|(record, seed)| record.as_ref() == Some(&seed.record));
    if persisted != prepared
        || persisted.request != expected_request
        || groups.groups.len() != 2
        || groups
            .groups
            .iter()
            .map(|group| group.id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            != initial.iter().map(|seed| seed.record.id.as_str()).collect()
        || !groups.ungrouped_writes.is_empty()
        || !canonical_records_unchanged
        || app.store.receipt(&operation)?.is_some()
    {
        return Err("reopened candidate is not the exact unchanged two-group fixture".into());
    }

    let expected_values = SOURCES
        .iter()
        .zip(&initial)
        .zip(CANDIDATES)
        .enumerate()
        .map(|(index, ((source, seed), candidate))| {
            let (source_key, original) = source;
            let record = &seed.record;
            json!({
                "source_key": source_key,
                "source_operation_id": seed.operation_id,
                "source_id": record.id,
                "initial_record": record,
                "initial": {
                    "description": original,
                    "state": "Open",
                    "version": record.version,
                    "sha256": sha256(original.as_bytes())
                },
                "candidate": {
                    "description": candidate,
                    "state": "Open",
                    "sha256": sha256(candidate.as_bytes())
                },
                "candidate_write": prepared.request.writes[index]
            })
        })
        .collect::<Vec<_>>();
    let prepared_request_hash = sha256(&serde_json::to_vec(&prepared.request)?);
    let manifest = json!({
        "format": "BRN native Review groups fixture v1",
        "fixture_id": FIXTURE_ID,
        "qualification_scope": "offline generic Review Include/Omit with two independent Open Action roots; candidate reopened and discoverable through Workspace::candidates",
        "desktop_route": "Home > Review changes, or the Threads sidebar candidate row; open the candidate to enter Review",
        "provider": "offline-fixture",
        "workspace": "workspace",
        "sources": expected_values,
        "candidate": {
            "host_request_key": candidate_host_key,
            "operation_id": operation.as_str(),
            "request": prepared.request.clone(),
            "request_sha256": prepared_request_hash,
            "groups": groups.groups.iter().map(|group| json!({
                "id": group.id,
                "title": group.title,
                "kind": group.kind,
                "writes": group.writes,
                "required_groups": group.required_groups
            })).collect::<Vec<_>>(),
            "is_prepared_and_reopened": true,
            "is_applied": false,
            "canonical_source_records_unchanged": true,
            "apply_receipt_present": false
        }
    });
    let mut manifest_file = fs::File::create(root.join("manifest.json"))?;
    manifest_file.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    manifest_file.write_all(b"\n")?;
    println!("Fixture workspace: {}", root.join("workspace").display());
    println!("Fixture manifest: {}", root.join("manifest.json").display());
    Ok(())
}
