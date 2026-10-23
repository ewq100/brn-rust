#![cfg(any(target_os = "macos", target_os = "linux"))]
//! Public Workspace integration: abandoned candidate + canonical recovery +
//! missing local configuration + fresh runtime authority + exact review/Undo.
use brn_ai::{
    AiErrorKind, AiTerminal,
    threads::{TraceLevel, run_prepared_scripted, run_prepared_scripted_then_abandon},
};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/work/active/agent-capabilities/int-d/corpus.json"
    ))
    .unwrap()
}
fn start(chat: &str, prompt: &str) -> ThreadRunStart {
    ThreadRunStart {
        thread: chat.into(),
        prompt: prompt.into(),
        request_message: None,
        queue_claim: None,
        owner_request: None,
        sources: vec![],
        max_tool_rounds: Some(8),
        trace: TraceLevel::Summary,
        offered_copilot_models: None,
    }
}
fn done(text: &str) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}
fn tools(calls: Vec<MockStreamEvent>) -> Vec<MockStreamEvent> {
    let mut out = calls;
    out.push(MockStreamEvent::final_response_with_total_tokens(1));
    out
}
fn selected_chat_input(request: &rig::completion::CompletionRequest, body: &str) {
    assert!(
        request
            .chat_history
            .iter()
            .any(|message| matches!(message.as_ref(),
        rig::message::Message::User {content} if content.iter().any(|item|
            matches!(item, rig::message::UserContent::Text(text) if text.text.contains(body)))))
    );
    assert!(
        request
            .chat_history
            .iter()
            .all(|message| !matches!(message.as_ref(),
        rig::message::Message::System {content} if content.contains("INT_D_SELECTED_BODY")))
    );
}
#[tokio::test]
async fn abandoned_selected_skill_candidate_survives_restore_without_package_and_exact_owner_review_undo()
 {
    let c = corpus();
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    app.choose_delegation(DelegationChoice::ReviewFirst)
        .unwrap();
    let mut settings = app.settings().unwrap();
    settings.provider = "Chatgpt".into();
    settings.model = "gpt-6.1-sol".into();
    settings.effort = "medium".into();
    app.configure(settings).unwrap();
    let project = app
        .project_command(
            "int-d-project",
            ProjectCommand::Create {
                title: "INT-D recovery".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    let chat = app.new_thread("INT-D pending revision").unwrap().id;
    app.project_command(
        "int-d-chat",
        ProjectCommand::MoveChat {
            chat: chat.clone(),
            project: Some(project.clone()),
        },
    )
    .unwrap();
    let initial = app
        .new_note(
            "INT-D protected draft",
            c["original_note"].as_str().unwrap(),
        )
        .unwrap();
    app.project_command(
        "int-d-note",
        ProjectCommand::LinkDocument {
            project: project.clone(),
            note: initial.id.clone(),
        },
    )
    .unwrap();
    let RecordData::Note(mut protected) = initial.data else {
        panic!()
    };
    protected.protected = true;
    app.owner_change(
        "int-d-protect",
        &ChangeRequest {
            reason: "Owner protects exact synthetic work".into(),
            inputs: vec![],
            writes: vec![Put {
                id: initial.id.clone(),
                expected_version: Some(initial.version),
                archived: false,
                data: RecordData::Note(protected),
            }],
        },
    )
    .unwrap();
    let original = app.store.record(&initial.id).unwrap().unwrap();
    let positive = app
        .new_note(
            "intdrecoverypositive",
            "Canonical general work survives, independent of instruction packages",
        )
        .unwrap();
    let source = root.path().join("package");
    std::fs::create_dir_all(source.join("references")).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: Review synthetic delivery evidence.\n---\n{}\n",
            c["selected_skill_name"].as_str().unwrap(),
            c["skill_body"].as_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        source.join("references/check.md"),
        c["reference_body"].as_str().unwrap(),
    )
    .unwrap();
    let installed = app.install_skill(&source).unwrap();
    app.set_skill_enabled(&installed.id, &installed.digest, Some(&project), true)
        .unwrap();
    app.select_skill(Some(&installed.id), Some(&project))
        .unwrap();
    let prepared = app
        .prepare_thread_run(start(&chat, c["objective"].as_str().unwrap()))
        .unwrap();
    let RecordData::Note(mut replacement) = original.data.clone() else {
        panic!()
    };
    replacement.markdown = format!(
        "{}Prepared exact revision; owner has not applied it.\n",
        c["expected_note_prefix"].as_str().unwrap()
    );
    let model = MockCompletionModel::from_stream_turns([
        tools(vec![
            MockStreamEvent::tool_call(
                "read",
                "read_records",
                json!({"ids":[original.id],"query":null}),
            ),
            MockStreamEvent::tool_call(
                "reference",
                "read_skill",
                json!({"id":installed.id,"path":"references/check.md","start":0}),
            ),
        ]),
        tools(vec![MockStreamEvent::tool_call(
            "progress",
            "retain_progress",
            json!({"summary":"Read current protected draft; not applied","objective":c["objective"],"remaining_work":[c["remaining_work"]],"open_questions":[],"operations":[]}),
        )]),
        tools(vec![MockStreamEvent::tool_call(
            "prepare",
            "prepare_changes",
            json!({"writing":null,"request_key":"int-d-crash-review","request":{"reason":"Selected synthetic instructions do not grant protected access","writes":[{"id":original.id,"expected_version":original.version,"archived":false,"data":{"Note":replacement}}],"inputs":[]}}),
        )]),
        done("This final model answer cannot prove the candidate applied."),
    ]);
    let abandoned = run_prepared_scripted_then_abandon(prepared.prepared, model.clone())
        .await
        .unwrap();
    assert_eq!(model.request_count(), 4);
    selected_chat_input(&model.requests()[0], c["skill_body"].as_str().unwrap());
    let RecordData::Run(initial_run) = app.store.record(&abandoned).unwrap().unwrap().data else {
        panic!()
    };
    assert!(
        initial_run
            .loaded_skill_details
            .iter()
            .any(|d| d.id == installed.id
                && d.package_digest == installed.digest
                && d.file == "SKILL.md")
    );
    let candidates = app.store.candidates().unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = candidates[0].clone();
    assert_eq!(app.store.record(&original.id).unwrap().unwrap(), original);
    assert!(app.store.receipt(&candidate.operation).unwrap().is_none());
    assert!(
        matches!(app.store.record(&abandoned).unwrap().unwrap().data,RecordData::Run(r) if r.state==RunState::Working)
    );
    let backup = root.path().join("backup.sqlite3");
    app.store.backup(&backup).unwrap();
    let policy = app.settings().unwrap().skills;
    drop(app);
    let restored_dir = root.path().join("restored");
    drop(Store::restore(&backup, &restored_dir).unwrap());
    let mut restored = Workspace::open_existing_no_upgrade(&restored_dir).unwrap();
    assert_eq!(restored.settings().unwrap().skills, policy);
    assert_eq!(
        restored.store.prepared(&candidate.operation).unwrap(),
        candidate
    );
    assert_eq!(
        restored.recover_orphaned_runs().unwrap(),
        vec![abandoned.clone()]
    );
    assert!(restored.recover_orphaned_runs().unwrap().is_empty());
    assert!(
        matches!(restored.store.record(&abandoned).unwrap().unwrap().data,RecordData::Run(r) if r.state==RunState::Interrupted && r.operations.contains(&candidate.operation))
    );
    let retained = restored
        .scoped_skills(Some(&project))
        .unwrap()
        .into_iter()
        .find(|s| s.id == installed.id)
        .unwrap();
    assert!(retained.enabled && retained.selected && !retained.available);
    let refusal =
        match restored.prepare_thread_run(start(&chat, c["continue_prompt"].as_str().unwrap())) {
            Ok(_) => panic!("Restored missing exact selection cannot start"),
            Err(e) => e,
        };
    let RunPreparationError::Preparation(error) = refusal else {
        panic!("expected runtime selected-package preflight")
    };
    assert_eq!(error.kind, AiErrorKind::Other);
    assert_eq!(error.detail.as_ref().unwrap().origin, "skill_catalogue");
    assert_eq!(
        error.detail.as_ref().unwrap().code.as_deref(),
        Some("skill_unavailable")
    );
    assert_eq!(
        restored.store.prepared(&candidate.operation).unwrap(),
        candidate
    );
    assert_eq!(
        restored.store.record(&original.id).unwrap().unwrap(),
        original
    );
    let reinstalled = restored.install_skill(&source).unwrap();
    assert_eq!(
        (&reinstalled.id, &reinstalled.digest),
        (&installed.id, &installed.digest)
    );
    let resume = restored
        .prepare_thread_run(start(&chat, c["continue_prompt"].as_str().unwrap()))
        .unwrap();
    let model = MockCompletionModel::from_stream_turns([
        tools(vec![MockStreamEvent::tool_call(
            "attempt",
            "apply_changes",
            json!({"operation":candidate.operation}),
        )]),
        done("Exact owner review still required; no protected grant inherited."),
    ]);
    let result = run_prepared_scripted(resume.prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert_eq!(model.request_count(), 2);
    selected_chat_input(&model.requests()[0], c["skill_body"].as_str().unwrap());
    let RecordData::Run(resumed_run) = restored.store.record(&result.run_id).unwrap().unwrap().data
    else {
        panic!()
    };
    assert!(
        resumed_run
            .loaded_skill_details
            .iter()
            .any(|d| d.id == installed.id
                && d.package_digest == installed.digest
                && d.file == "SKILL.md")
    );
    assert!(
        serde_json::to_string(&model.requests()[1])
            .unwrap()
            .contains("NeedsReview")
    );
    assert!(
        !model.requests()[0]
            .tools
            .iter()
            .any(|t| t.name.starts_with("mcp_"))
    );
    assert_eq!(
        restored.store.record(&original.id).unwrap().unwrap(),
        original
    );
    let ApplyOutcome::Applied(receipt) = restored
        .review_apply(candidate.operation.as_str())
        .unwrap()
        .outcome
    else {
        panic!("exact owner review must apply")
    };
    assert_eq!(receipt.operation, candidate.operation);
    assert_eq!(receipt.request_hash, candidate.request_hash);
    let applied = restored.store.record(&original.id).unwrap().unwrap();
    assert_ne!(applied.data, original.data);
    assert_eq!(
        restored
            .review_apply(candidate.operation.as_str())
            .unwrap()
            .outcome,
        ApplyOutcome::Applied(receipt)
    );
    assert_eq!(
        restored.store.record(&original.id).unwrap().unwrap(),
        applied
    );
    let (undo, reversed) = restored
        .undo_with_key("int-d-exact-undo", candidate.operation.as_str())
        .unwrap();
    assert!(matches!(reversed, ApplyOutcome::Applied(_)));
    assert_eq!(
        restored.store.record(&original.id).unwrap().unwrap().data,
        original.data
    );
    assert_eq!(
        restored
            .undo_with_key("int-d-exact-undo", candidate.operation.as_str())
            .unwrap(),
        (undo, reversed)
    );
    assert_eq!(
        restored.store.record(&positive.id).unwrap().unwrap(),
        positive
    );
    assert!(!restored.search("intdrecoverypositive").unwrap().is_empty());
    let export = root.path().join("export");
    let manifest = restored.export_now(&export).unwrap();
    assert!(manifest.files.iter().any(|f| f.record == positive.id));
    for file in manifest.files {
        assert!(
            !std::fs::read_to_string(export.join(file.path))
                .unwrap()
                .contains("INT_D_SELECTED_BODY")
        );
    }
}
