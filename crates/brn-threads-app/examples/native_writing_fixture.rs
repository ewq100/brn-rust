//! Create an explicit offline workspace for native v1 writing review.
//!
//! This prepares one exact captured request and an unapplied candidate. It
//! does not run a provider, apply the candidate, or exercise v2 evidence review.
use brn_threads_app::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf};

const ORIGINAL: &str = "tere tere õun\n\nThis is a synthetic writing fixture.\n";
const UNRELATED: &str = "UNRELATED synthetic note — unchanged.\n";
const REVISED: &str = "tere tere õun\n\nThis synthetic writing fixture is clearer.\n";

fn usage() -> &'static str {
    "Usage: native_writing_fixture --output /absolute/new-root"
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

    // Reserve the caller's exact root before opening or creating workspace data.
    // create_dir is atomic and refuses existing directories and files.
    fs::create_dir(&root)?;
    let mut app = Workspace::open_without_default_credentials(root.join("workspace"))?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;

    let note = app.new_note("Native selected-feedback fixture", ORIGINAL)?;
    let unrelated = app.new_note("Unrelated fixture note", UNRELATED)?;
    let context = app.ensure_writing_context("native-writing-fixture", &note.id, None)?;
    let thread_id = context
        .conversation
        .ok_or("writing context did not create a conversation")?;

    let first = app.add_writing_comment(
        "native-writing-comment-fruit",
        &WritingCommentInput {
            note: note.id.clone(),
            version: note.version,
            thread: thread_id.clone(),
            range: (10, 14),
            quote: "õun".into(),
            body: "Keep the fruit word.".into(),
        },
    )?;
    let second = app.add_writing_comment(
        "native-writing-comment-opening",
        &WritingCommentInput {
            note: note.id.clone(),
            version: note.version,
            thread: thread_id.clone(),
            range: (0, 4),
            quote: "tere".into(),
            body: "Make the explanation clearer.".into(),
        },
    )?;
    let comment_ids = vec![second.id.clone(), first.id.clone()];
    let snapshot = app.snapshot_writing_request(
        "native-writing-request",
        &note.id,
        note.version,
        &thread_id,
        &comment_ids,
    )?;
    let expected_request = snapshot.change_request(REVISED)?;
    let candidate_operation = app.store.allocate_operation("native-writing-candidate")?;
    let prepared = app.store.prepare(&candidate_operation, &expected_request)?;

    // Confirm through public read APIs that this is the exact v1 native-review
    // input, all captured comments remain pending, and no write was applied.
    let reopened_request =
        app.writing_request_in_context(&snapshot.request_message, &note.id, None)?;
    if reopened_request != snapshot
        || snapshot.note != note
        || snapshot.comments.len() != 2
        || prepared.request != expected_request
    {
        return Err("prepared candidate or captured writing request changed".into());
    }
    let expected_feedback = [
        (&second, (0, 4), "tere", "Make the explanation clearer."),
        (&first, (10, 14), "õun", "Keep the fruit word."),
    ];
    for (record, range, quote, body) in expected_feedback {
        let Some(RecordData::Comment(value)) = snapshot
            .comments
            .iter()
            .find(|captured| captured.id == record.id)
            .map(|captured| &captured.data)
        else {
            return Err("captured request is missing an expected comment".into());
        };
        if value.note != note.id
            || value.base_version != note.version
            || value.range != Some(range)
            || value.quote != quote
            || value.body != body
        {
            return Err("captured request comment differs from its exact source feedback".into());
        }
    }
    let candidate_note = prepared
        .request
        .writes
        .iter()
        .find_map(|write| match &write.data {
            RecordData::Note(value) if write.id == note.id => Some(value),
            _ => None,
        })
        .ok_or("prepared candidate does not contain the target Note")?;
    if candidate_note.markdown != REVISED
        || app.store.record(&note.id)?.as_ref() != Some(&note)
        || app.store.record(&unrelated.id)?.as_ref() != Some(&unrelated)
        || app.store.receipt(&candidate_operation)?.is_some()
    {
        return Err("candidate validation found an unexpected write or receipt".into());
    }
    let comments = app.writing_comments(&note.id)?;
    if comments.len() != 2
        || comments.iter().any(|comment| comment.addressed)
        || comments
            .iter()
            .map(|comment| comment.record.id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            != comment_ids.iter().map(String::as_str).collect()
    {
        return Err("captured comments are not both present and pending".into());
    }

    let manifest = json!({
        "format": "BRN native writing fixture v1",
        "qualification_scope": "v1 captured-request selector, Apply and Undo; no v2 evidence review",
        "provider": "offline-fixture",
        "workspace": "workspace",
        "source_constants": {
            "original": ORIGINAL,
            "original_sha256": sha256(ORIGINAL.as_bytes()),
            "unrelated": UNRELATED,
            "unrelated_sha256": sha256(UNRELATED.as_bytes()),
            "revised": REVISED,
            "revised_sha256": sha256(REVISED.as_bytes())
        },
        "target_note": {
            "id": note.id,
            "version": note.version,
            "title": "Native selected-feedback fixture",
            "original_bytes": ORIGINAL.as_bytes().len(),
            "original_sha256": sha256(ORIGINAL.as_bytes()),
            "revised_bytes": REVISED.as_bytes().len(),
            "revised_sha256": sha256(REVISED.as_bytes())
        },
        "unrelated_note": {
            "id": unrelated.id,
            "title": "Unrelated fixture note",
            "bytes": UNRELATED.as_bytes().len(),
            "sha256": sha256(UNRELATED.as_bytes())
        },
        "conversation_id": thread_id,
        "comments": [
            {"id": second.id, "quote": "tere", "range_utf8_bytes": [0, 4], "body": "Make the explanation clearer."},
            {"id": first.id, "quote": "õun", "range_utf8_bytes": [10, 14], "body": "Keep the fruit word."}
        ],
        "captured_request": {
            "message_id": snapshot.request_message,
            "comment_ids": comment_ids,
            "snapshot": snapshot
        },
        "candidate": {
            "operation_id": candidate_operation.as_str(),
            "is_prepared": true,
            "is_applied": false,
            "request_matches_snapshot_change_request": true,
            "initial_comments_pending": [second.id, first.id],
            "expected_after_apply_selected_first_only_addressed": [second.id],
            "expected_after_undo_both_pending": [second.id, first.id]
        }
    });
    let mut manifest_file = fs::File::create(root.join("manifest.json"))?;
    manifest_file.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    manifest_file.write_all(b"\n")?;
    println!("Fixture manifest: {}", root.join("manifest.json").display());
    Ok(())
}
