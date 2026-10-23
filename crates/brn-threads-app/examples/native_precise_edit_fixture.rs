//! Offline runtime-to-native-review fixture; never dispatches a live provider.
use brn_ai::{
    AiTerminal,
    threads::{TraceLevel, run_prepared_scripted},
};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::{fs, path::Path};

const ORIGINAL: &str = "# Harbors\n\nÕun stays beside the pier.\nThe cobalt boats cross the bay at dawn.\n\n尾灯 remain unchanged.\n";
const EXPECTED: &str = "# Harbors\n\nÕun stays beside the pier.\nThe teal boats cross the bay at dawn.\n\n尾灯 remain unchanged.\n";
const SENTINEL: &str = "Unrelated synthetic record: amber lantern remains unchanged.\n";

fn call(id: &str, tool: &str, args: Value) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::tool_call(id, tool, args),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn finish() -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text("Offline fixture finished; review the exact candidate."),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

async fn fixture(root: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if !root.is_absolute()
        || root.file_name().is_none()
        || fs::canonicalize(root.parent().ok_or("missing parent")?)? != root.parent().unwrap()
    {
        return Err("output must be a new absolute root under an existing canonical parent".into());
    }
    fs::create_dir(root)?;
    let data = root.join("workspace");
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let mut settings = app.settings()?;
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    // Selection metadata only: the consumed invocation always uses Rig's mock.
    settings.provider = "Codex".into();
    settings.model = "gpt-6-luna".into();
    settings.effort = "low".into();
    app.configure(settings.clone())?;
    let note = app.new_note("Precise edit — synthetic harbor", ORIGINAL)?;
    let sentinel = app.new_note("Precise edit sentinel", SENTINEL)?;
    let chat = app.new_thread("Offline precise-edit qualification")?;
    assert_eq!(ORIGINAL.find("cobalt boats"), Some(43));
    let edit = |start: Value, end: Value, key: &str| {
        json!({
            "id":note.id,"version":note.version,"start":start,"end":end,
            "expected_text":"cobalt boats","replacement_text":"teal boats",
            "request_key":key,"reason":"Replace only the requested synthetic passage"
        })
    };
    let start = || ThreadRunStart {
        thread: chat.id.clone(),
        prompt: "Prepare only cobalt boats → teal boats; preserve all other bytes. Do not apply."
            .into(),
        request_message: None,
        queue_claim: None,
        owner_request: None,
        sources: vec![],
        max_tool_rounds: Some(4),
        trace: TraceLevel::Full,
        offered_copilot_models: None,
    };
    let read = || call("read", "read_records", json!({"ids":[note.id]}));
    // Replay observed wrong locations, not unknown historical text arguments.
    let mut invalid = vec![];
    for (index, offset) in [48, 49, 47, 44, 39].into_iter().enumerate() {
        invalid.push(MockStreamEvent::tool_call(
            format!("bad-{index}"),
            "prepare_note_edit",
            edit(json!(offset), json!(offset + 12), &format!("bad-{index}")),
        ));
    }
    invalid.push(MockStreamEvent::final_response_with_total_tokens(1));
    let rejected = run_prepared_scripted(
        app.prepare_thread_run(start())?.prepared,
        MockCompletionModel::from_stream_turns([read(), invalid, finish()]),
    )
    .await?;
    assert!(matches!(rejected.answer.terminal, AiTerminal::Completed));
    let trace = rejected.trace.ok_or("missing refusal trace")?;
    assert_eq!(
        trace
            .calls
            .iter()
            .filter(|c| c.tool == "prepare_note_edit" && c.outcome == "Invalid")
            .count(),
        5
    );
    assert!(app.candidates()?.is_empty());
    assert_eq!(app.store.record(&note.id)?.as_ref(), Some(&note));
    assert_eq!(app.store.record(&sentinel.id)?.as_ref(), Some(&sentinel));
    fs::write(
        root.join("refused-locations.json"),
        serde_json::to_vec_pretty(&trace)?,
    )?;

    let quoted = edit(Value::Null, Value::Null, "exact-quote");
    let prepared = run_prepared_scripted(
        app.prepare_thread_run(start())?.prepared,
        MockCompletionModel::from_stream_turns([
            read(),
            call("quote", "prepare_note_edit", quoted.clone()),
            call("repeat", "prepare_note_edit", quoted),
            finish(),
        ]),
    )
    .await?;
    assert!(matches!(prepared.answer.terminal, AiTerminal::Completed));
    let trace = prepared.trace.ok_or("missing quote trace")?;
    assert_eq!(
        trace
            .calls
            .iter()
            .filter(|c| c.tool == "prepare_note_edit" && c.outcome == "success")
            .count(),
        2
    );
    let candidates = app.candidates()?;
    assert_eq!(
        candidates.len(),
        1,
        "identical preparation must reuse one candidate"
    );
    let candidate = &candidates[0];
    assert_eq!(candidate.request.writes.len(), 1);
    let write = &candidate.request.writes[0];
    assert_eq!(write.id, note.id);
    assert_eq!(write.expected_version, Some(note.version));
    let RecordData::Note(mut expected_note) = note.data.clone() else {
        return Err("not a Note".into());
    };
    expected_note.markdown = EXPECTED.into();
    assert_eq!(write.data, RecordData::Note(expected_note));
    assert_eq!(app.store.record(&note.id)?.as_ref(), Some(&note));
    assert_eq!(app.store.record(&sentinel.id)?.as_ref(), Some(&sentinel));
    assert!(app.store.receipt(&candidate.operation)?.is_none());
    settings.provider = "offline-fixture".into();
    app.configure(settings)?;
    fs::write(
        root.join("quote-trace.json"),
        serde_json::to_vec_pretty(&trace)?,
    )?;
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&json!({
            "fixture":"native-precise-edit-v1","synthetic_model":true,"live_account":false,
            "note":note,"sentinel":sentinel,"candidate":candidate,"thread":chat.id,
            "original":ORIGINAL,"expected":EXPECTED,"run":prepared.run_id,
            "historical_offsets_replayed":[[48,60],[49,61],[47,59],[44,56],[39,51]],
            "historical_raw_expected_text_unknown":true
        }))?,
    )?;
    drop(app);
    let reopened = Workspace::open_without_default_credentials(data)?;
    assert_eq!(reopened.candidates()?.len(), 1);
    assert_eq!(reopened.store.record(&note.id)?.as_ref(), Some(&note));
    assert_eq!(
        reopened.store.record(&sentinel.id)?.as_ref(),
        Some(&sentinel)
    );
    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--output" {
        return Err("Usage: native_precise_edit_fixture --output /absolute/new-root".into());
    }
    fixture(Path::new(&args[1])).await?;
    println!("PASS offline precise-edit fixture; native review remains unobserved");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn refuses_wrong_locations_then_retains_one_exact_unapplied_candidate() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let fixture = root.path().join("precise");
        super::fixture(&fixture).await.unwrap();
        let before = std::fs::read(fixture.join("manifest.json")).unwrap();
        assert!(
            super::fixture(&fixture).await.is_err(),
            "existing output is never reused"
        );
        assert_eq!(
            std::fs::read(fixture.join("manifest.json")).unwrap(),
            before
        );
    }
}
