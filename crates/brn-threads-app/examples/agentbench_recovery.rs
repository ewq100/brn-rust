//! Retained offline interruption journey using the existing scripted native seam.
use brn_ai::threads::{
    ThreadRunRequest, TraceLevel, prepare_run, run_prepared_scripted,
    run_prepared_scripted_then_abandon,
};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
type ProbeResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn hash(path: &Path) -> ProbeResult<String> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}
fn save(path: &Path, value: &impl serde::Serialize) -> ProbeResult<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn checkpoint(root: &Path, workspace: &Workspace, step: usize) -> ProbeResult<Value> {
    let folder = root.join("checkpoints").join(step.to_string());
    std::fs::create_dir_all(&folder)?;
    let backup = folder.join("workspace.sqlite3");
    workspace.backup(&backup)?;
    save(&folder.join("records.json"), &workspace.records()?)?;
    save(&folder.join("receipts.json"), &workspace.history()?)?;
    save(&folder.join("candidates.json"), &workspace.candidates()?)?;
    Ok(
        json!({"step":step,"backup_sha256":hash(&backup)?,"records_sha256":hash(&folder.join("records.json"))?,
        "receipts_sha256":hash(&folder.join("receipts.json"))?,"candidates_sha256":hash(&folder.join("candidates.json"))?}),
    )
}
fn request(data: &Path, chat: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.into(),
        thread: chat.into(),
        prompt: "Propose replacing Pending with Confirmed in the synthetic note. Do not apply it."
            .into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 8,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: None,
        trace: TraceLevel::Full,
        requested_skills: Vec::new(),
        web_search: None,
    }
}
fn proposal(note: &Record) -> ProbeResult<MockCompletionModel> {
    let RecordData::Note(content) = &note.data else {
        return Err("expected note".into());
    };
    let start = content
        .markdown
        .find("Pending")
        .ok_or("expected pending passage")?;
    Ok(MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "proposal",
                "prepare_note_edit",
                json!({"id":note.id,"version":note.version,
            "start":start,"end":start+7,"expected_text":"Pending","replacement_text":"Confirmed",
            "request_key":"controlled-edit","reason":"Controlled recovery diagnostic"}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("Prepared for review, not applied."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]))
}
async fn run(root: &Path) -> ProbeResult<Value> {
    let data = root.join("data");
    let mut workspace = Workspace::open_without_default_credentials(&data)?;
    workspace.choose_delegation(DelegationChoice::ReviewFirst)?;
    let note = workspace.new_note("Recovery witness", "Pending. Owner approval required.")?;
    let chat = workspace.new_thread("Recovery diagnostic")?;
    let mut checkpoints = vec![checkpoint(root, &workspace, 0)?];
    let prepared = prepare_run(request(&data, &chat.id))?;
    let run = run_prepared_scripted_then_abandon(prepared, proposal(&note)?).await?;
    let candidates = workspace.candidates()?;
    if candidates.len() != 1 {
        return Err("expected one retained candidate".into());
    }
    let immutable = candidates[0].clone();
    let old = immutable.operation.as_str().to_owned();
    checkpoints.push(checkpoint(root, &workspace, 1)?);
    let edit = workspace.store.begin_edit(&note.id, note.version)?;
    let owner =
        "Pending. Owner approval required. Later owner qualification: scope is provisional.";
    workspace.store.update_buffer(&edit.id, 1, owner)?;
    if !matches!(workspace.store.save(&edit.id, 1)?, SaveOutcome::Saved(_)) {
        return Err("owner edit did not save".into());
    }
    checkpoints.push(checkpoint(root, &workspace, 2)?);
    drop(workspace);
    let mut workspace = Workspace::open_without_default_credentials(&data)?;
    if workspace.recover_orphaned_runs()? != vec![run.clone()]
        || !workspace.recover_orphaned_runs()?.is_empty()
    {
        return Err("recovery must occur exactly once".into());
    }
    if !matches!(workspace.store.record(&run)?.ok_or("lost run")?.data,RecordData::Run(r) if r.state==RunState::Interrupted)
    {
        return Err("abandoned run not marked Interrupted".into());
    }
    if workspace.store.prepared(&immutable.operation)? != immutable {
        return Err("recovery changed candidate".into());
    }
    checkpoints.push(checkpoint(root, &workspace, 3)?);
    let stale = workspace.review_apply(&old)?.outcome;
    if !matches!(stale, ApplyOutcome::Stale { .. }) {
        return Err("old candidate was not stale".into());
    }
    workspace.review_dismiss(&old)?;
    checkpoints.push(checkpoint(root, &workspace, 4)?);
    let current = workspace.store.record(&note.id)?.ok_or("lost owner note")?;
    let retry =
        run_prepared_scripted(prepare_run(request(&data, &chat.id))?, proposal(&current)?).await?;
    if !matches!(retry.answer.terminal, AiTerminal::Completed) {
        return Err("retry did not complete".into());
    }
    save(&root.join("retry-trace.json"), &retry.trace)?;
    let candidates = workspace.candidates()?;
    if candidates.len() != 1 || candidates[0].operation.as_str() == old {
        return Err("retry candidate identity was reused".into());
    }
    let fresh = candidates[0].operation.as_str().to_owned();
    checkpoints.push(checkpoint(root, &workspace, 5)?);
    if !matches!(
        workspace.review_apply(&fresh)?.outcome,
        ApplyOutcome::Applied(_)
    ) {
        return Err("fresh review did not apply".into());
    }
    checkpoints.push(checkpoint(root, &workspace, 6)?);
    let continuing = workspace.records()?;
    let backup = root.join("checkpoints/6/workspace.sqlite3");
    let frozen = hash(&backup)?;
    drop(Store::restore(&backup, root.join("undo-copy"))?);
    let mut copy = Workspace::open_without_default_credentials(root.join("undo-copy"))?;
    if !matches!(copy.undo(&fresh)?, ApplyOutcome::Applied(_)) {
        return Err("Undo copy did not restore".into());
    }
    if !matches!(copy.store.record(&note.id)?.ok_or("lost undo note")?.data,RecordData::Note(n) if n.markdown==owner)
    {
        return Err("Undo lost the later owner qualification".into());
    }
    save(&root.join("undo-copy-records.json"), &copy.records()?)?;
    save(&root.join("undo-copy-receipts.json"), &copy.history()?)?;
    if workspace.records()? != continuing || hash(&backup)? != frozen {
        return Err("Undo probe changed continuing evidence".into());
    }
    Ok(
        json!({"format":1,"status":"complete","kind":"controlled-recovery","qualified":false,"candidate_calls":0,
        "provider":"controlled-fixture","abandoned_run":run,"retry_run":retry.run_id,"retained_candidate":old,
        "replacement_candidate":fresh,"stale_outcome":format!("{stale:?}"),"checkpoints":checkpoints,
        "binary_sha256":hash(&std::env::current_exe()?)?,"cost":null,
        "checks":["orphan interrupted once","candidate immutable","later owner edit preserved","stale apply refused",
                  "fresh retry identity","explicit review applied","Undo on copy preserved owner qualification","continuing evidence unchanged"]}),
    )
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("expected fresh absolute output directory");
        std::process::exit(1);
    };
    let root = PathBuf::from(path);
    if !root.is_absolute() || std::fs::create_dir(&root).is_err() {
        eprintln!("fresh absolute output directory required");
        std::process::exit(1);
    }
    let result_path = root.join("recovery-result.json");
    if let Err(error) = save(&result_path, &json!({"status":"running","qualified":false})) {
        eprintln!("{error}");
        std::process::exit(1);
    }
    match run(&root).await {
        Ok(result) => {
            if let Err(error) = save(&result_path, &result) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            println!("{}", result);
        }
        Err(error) => {
            let _ = save(
                &result_path,
                &json!({"status":"failed","qualified":false,"error":error.to_string()}),
            );
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
