use brn_ai::{
    AiTerminal,
    threads::{TraceLevel, run_prepared_scripted},
};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};

#[test]
fn selection_errors_precede_any_run_write_and_catalogue_never_substitutes() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let mut settings = app.settings().unwrap();
    settings.provider = "Copilot".into();
    settings.model = "chosen".into();
    settings.effort = "medium".into();
    assert!(resolve_run_selection(&settings, Some(&["different".into()])).is_err());
    settings.provider = "unknown".into();
    assert!(resolve_run_selection(&settings, None).is_err());
    settings.provider = "Codex".into();
    settings.effort = "unsupported".into();
    assert!(resolve_run_selection(&settings, None).is_err());
    let chat = app.new_thread("Shared start").unwrap();
    let before = app.records().unwrap();
    let result = app.prepare_thread_run(ThreadRunStart {
        thread: chat.id,
        prompt: "Synthetic".into(),
        request_message: None,
        queue_claim: None,
        owner_request: None,
        sources: vec![],
        max_tool_rounds: Some(0),
        trace: TraceLevel::Off,
        offered_copilot_models: None,
    });
    assert!(matches!(
        result,
        Err(RunPreparationError::Preparation(brn_ai::AiError {
            kind: brn_ai::AiErrorKind::ToolRejected,
            ..
        }))
    ));
    assert_eq!(app.records().unwrap(), before);
}
#[tokio::test]
async fn shared_preparation_runs_scripted_with_the_same_durable_run_identity() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let chat = app.new_thread("Shared start").unwrap();
    let prepared = app
        .prepare_thread_run(ThreadRunStart {
            thread: chat.id,
            prompt: "Synthetic offline question".into(),
            request_message: None,
            queue_claim: None,
            owner_request: None,
            sources: vec![],
            max_tool_rounds: Some(2),
            trace: TraceLevel::Off,
            offered_copilot_models: None,
        })
        .unwrap();
    let id = prepared.prepared.run_id().to_owned();
    assert_eq!(prepared.search_status, "available");
    let result = run_prepared_scripted(
        prepared.prepared,
        MockCompletionModel::from_stream_turns(vec![vec![
            MockStreamEvent::text("Offline answer"),
            MockStreamEvent::final_response_with_total_tokens(1),
        ]]),
    )
    .await
    .unwrap();
    assert_eq!(result.run_id, id);
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert_eq!(
        app.records()
            .unwrap()
            .iter()
            .filter(|r| matches!(&r.data, RecordData::Run(_)))
            .count(),
        1
    );
}
