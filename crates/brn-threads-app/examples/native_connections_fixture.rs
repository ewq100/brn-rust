//! Prepare an offline MAC-14 Connections fixture through checked owner services.
//! Does not unlink, undo, invoke a provider, or claim native interaction success.
use brn_threads_app::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const PRIVATE_TEXT: &str =
    "Private synthetic connection — copper-orchid-731.\nRetain in Project A.\n";
const CENTRE_TEXT: &str = "General synthetic center for Project B.\n";
const SENTINEL_TEXT: &str = "Unrelated synthetic sentinel — leave unchanged.\n";

fn project(app: &mut Workspace, key: &str, title: &str) -> AppResult<Record> {
    app.project_command(
        key,
        ProjectCommand::Create {
            title: title.into(),
        },
    )?
    .writes
    .into_iter()
    .find(|write| matches!(write.after.data, RecordData::Project(_)))
    .map(|write| write.after)
    .ok_or_else(|| AppError::Invalid("Project creation returned no Project".into()))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--output" {
        return Err("Usage: native_connections_fixture --output /absolute/new-root".into());
    }
    let root = PathBuf::from(&args[1]);
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("--output must name a new absolute root directory".into());
    }
    let parent = root.parent().ok_or("--output needs an existing parent")?;
    if fs::canonicalize(parent)? != parent {
        return Err("--output parent must exist and use its canonical path".into());
    }
    // Reserve the requested path atomically: never modify an existing fixture.
    fs::create_dir(&root)?;
    let data = root.join("workspace");
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;

    let a = project(
        &mut app,
        "mac14-project-a",
        "Project A — retained private source",
    )?;
    let b = project(
        &mut app,
        "mac14-project-b",
        "Project B — shared connections",
    )?;
    let private_receipt = app.project_command(
        "mac14-private-note",
        ProjectCommand::CreateDocument {
            project: a.id.clone(),
            title: "Private neighbor copper-orchid-731".into(),
            markdown: PRIVATE_TEXT.into(),
        },
    )?;
    let private = private_receipt
        .writes
        .iter()
        .find(|write| matches!(write.after.data, RecordData::Note(_)))
        .map(|write| write.after.clone())
        .ok_or("private document creation returned no Note")?;
    let centre = app.new_note("General center for Connections", CENTRE_TEXT)?;
    let sentinel = app.new_note("Unrelated sentinel", SENTINEL_TEXT)?;
    let mut link_receipts = Vec::new();
    for (key, note) in [
        ("mac14-link-centre-b", &centre),
        ("mac14-link-private-b", &private),
    ] {
        link_receipts.push(app.project_command(
            key,
            ProjectCommand::LinkDocument {
                project: b.id.clone(),
                note: note.id.clone(),
            },
        )?);
    }
    let key = "mac14-related-edge";
    let operation = app.store.allocate_operation(key)?;
    let relation = app.owner_change(
        key,
        &ChangeRequest {
            reason: "Synthetic MAC14 general-center/private-neighbor relation".into(),
            writes: vec![Put {
                id: operation.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: centre.id.clone(),
                    to: private.id.clone(),
                    relation: relations::RELATED.into(),
                }),
            }],
            inputs: vec![],
        },
    )?;
    let ApplyOutcome::Applied(relation_receipt) = relation else {
        return Err("checked relation creation did not apply".into());
    };
    drop(app);
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let connections = app
        .search_handle()?
        .connections(&centre.id, Some(&b.id), 1, false)?;
    if !app.store.record_visible_in_chat(&private.id, Some(&a.id))?
        || !app.store.record_visible_in_chat(&private.id, Some(&b.id))?
        || app.store.record_visible_in_chat(&private.id, None)?
        || !connections.hops[0]
            .shown
            .iter()
            .any(|node| node.id == private.id)
    {
        return Err("private neighbor visibility or scoped Connections precondition failed".into());
    }
    for record in [&private, &centre, &sentinel] {
        if app.store.record(&record.id)?.as_ref() != Some(record) {
            return Err("fixture Note changed during preparation".into());
        }
    }
    let records = app.records()?;
    let history = app.history()?;
    let digest = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
    let manifest = json!({
        "format": "BRN native Connections fixture v1",
        "workspace": "workspace",
        "provider": "offline-fixture",
        "project_a": a.id,
        "project_b": b.id,
        "centre": centre,
        "private_neighbor": private,
        "sentinel": sentinel,
        "expected_text_sha256": {
            "centre": digest(CENTRE_TEXT), "private_neighbor": digest(PRIVATE_TEXT),
            "sentinel": digest(SENTINEL_TEXT)
        },
        "private_creation_receipt": private_receipt,
        "project_link_receipts": link_receipts,
        "relation_receipt": relation_receipt,
        "initial_connections_b": {
            "centre": connections.centre.id,
            "neighbor_ids": connections.hops[0].shown.iter().map(|node| &node.id).collect::<Vec<_>>()
        },
        "expected_initial_visibility": {"project_a": true, "project_b": true, "general": false},
        "desktop_route": "Projects > Project B > General center > Connections > Open beside > Projects; Unlink private neighbor from B; Changes > exact unlink receipt > Undo",
        "expected_unlink": "B visibility false; neighbor absent from successful settled Connections; center/scope/panel retained; all three Notes unchanged",
        "expected_undo": "B visibility true; neighbor restored with same center/scope; all three Notes unchanged",
        "native_observation": "not performed by fixture generator"
    });
    for (name, value) in [
        ("manifest.json", manifest),
        ("initial-records.json", serde_json::to_value(records)?),
        ("initial-history.json", serde_json::to_value(history)?),
    ] {
        fs::write(
            root.join(name),
            format!("{}\n", serde_json::to_string_pretty(&value)?),
        )?;
    }
    println!("Fixture workspace: {}", data.display());
    println!("Fixture manifest: {}", root.join("manifest.json").display());
    Ok(())
}
