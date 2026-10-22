//! Prepare an offline writing-context attention fixture for native menu checks.
//! The blocker is synthetic checked state, not evidence of a provider failure.
use brn_threads_app::*;
use serde_json::json;
use std::{fs, path::PathBuf};

const TITLE: &str = "Synthetic writing context with a very long title that must remain fully named in the actionable Needs you menu item";
const REASON: &str = "Synthetic blocker: inspect this exact saved writing context before continuing; the full reason must remain accessible even when the visible menu text is truncated.";
const BODY: &str =
    "Synthetic writing context — café, jõgi, 日本語.\nThis saved text must remain unchanged.\n";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--output" {
        return Err("Usage: native_attention_fixture --output /absolute/new-root".into());
    }
    let root = PathBuf::from(&args[1]);
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("--output must name a new absolute root directory".into());
    }
    let parent = root.parent().ok_or("--output needs an existing parent")?;
    if fs::canonicalize(parent)? != parent {
        return Err("--output parent must exist and use its canonical path".into());
    }
    fs::create_dir(&root)?;
    let data = root.join("workspace");
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;
    let note = app.new_note(TITLE, BODY)?;
    let sentinel = app.new_note(
        "Unrelated attention sentinel",
        "Leave this exact sentinel unchanged.\n",
    )?;
    let context = app.ensure_writing_context("native-attention-writing-context", &note.id, None)?;
    let thread_id = context.conversation.ok_or("missing writing conversation")?;
    let current = app
        .store
        .record(&thread_id)?
        .ok_or("missing writing Thread")?;
    let RecordData::Thread(mut thread) = current.data.clone() else {
        return Err("writing conversation is not a Thread".into());
    };
    thread.title = TITLE.into();
    thread.attention = vec![Attention {
        kind: AttentionKind::Blocker,
        reason: REASON.into(),
        record: Some(note.id.clone()),
    }];
    let outcome = app.owner_change(
        "native-attention-synthetic-blocker",
        &ChangeRequest {
            reason: "Synthetic native menu naming fixture; no provider Run occurred".into(),
            inputs: vec![],
            writes: vec![Put {
                id: thread_id.clone(),
                expected_version: Some(current.version),
                archived: false,
                data: RecordData::Thread(thread),
            }],
        },
    )?;
    let ApplyOutcome::Applied(receipt) = outcome else {
        return Err("synthetic checked attention did not apply".into());
    };
    drop(app);
    let app = Workspace::open_without_default_credentials(&data)?;
    let needs = app.needs_you()?;
    if needs.len() != 1 || needs[0].id != thread_id {
        return Err("expected writing Thread is not the sole Needs you item after reopen".into());
    }
    for record in [&note, &sentinel] {
        if app.store.record(&record.id)?.as_ref() != Some(record) {
            return Err("saved Note changed while preparing attention".into());
        }
    }
    let records = app.records()?;
    if records
        .iter()
        .any(|record| matches!(record.data, RecordData::Run(_)))
    {
        return Err("offline attention fixture unexpectedly contains a Run".into());
    }
    let manifest = json!({
        "format": "BRN native attention fixture v1",
        "workspace": "workspace", "provider": "offline-fixture",
        "target_note": note, "sentinel": sentinel, "thread_id": thread_id,
        "title": TITLE, "reason": REASON,
        "expected_accessible_menu_item_name": format!("{TITLE} · {REASON}"),
        "attention_receipt": receipt,
        "expected_route": "Needs you badge > actionable outer MenuItem by full title/reason > exact writing Note and Thread",
        "expected_unchanged": "saved Note bytes/version, sentinel, candidate inventory; no provider Run",
        "native_observation": "not performed by fixture generator"
    });
    for (name, value) in [
        ("manifest.json", manifest),
        ("initial-records.json", serde_json::to_value(records)?),
        (
            "initial-history.json",
            serde_json::to_value(app.history()?)?,
        ),
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
