use super::*;
use crate::subscription_search::{Provenance, SearchError, SearchResult, Source};
use rig::test_utils::{MockCompletionModel, MockStreamEvent};

/// Records each query; the second query fails so errors stay in the run.
#[derive(Default)]
struct FakeWeb {
    queries: Mutex<Vec<String>>,
    cancel_run: Mutex<Option<CancellationToken>>,
}
impl RunWebSearch for FakeWeb {
    fn search(&self, query: String, _cancel: CancellationToken) -> WebSearchFuture {
        self.queries.lock().unwrap().push(query.clone());
        if let Some(cancel) = self.cancel_run.lock().unwrap().take() {
            cancel.cancel();
        }
        Box::pin(async move {
            if query == "fails" {
                return Err(SearchError::Timeout);
            }
            Ok(SearchResult {
                answer: format!("Answer for {query}"),
                sources: vec![
                    Source {
                        url: "https://example.org/a".into(),
                        title: Some("Example A".into()),
                    },
                    Source {
                        url: format!("https://example.org/{query}"),
                        title: None,
                    },
                ],
                provenance: Provenance {
                    provider: Provider::Chatgpt,
                    model: "synthetic".into(),
                },
            })
        })
    }
}

fn web_call(id: &str, query: &str) -> MockStreamEvent {
    MockStreamEvent::tool_call(id, "web_search", json!({"query": query}))
}
fn done(text: &str) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

async fn run(
    request: &ThreadRunRequest,
    turns: Vec<Vec<MockStreamEvent>>,
    cancel: CancellationToken,
) -> (AiAnswer, Vec<AiEvent>, Arc<Scope>) {
    let (scope, input) = begin(request).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let answer = run_model(
        MockCompletionModel::from_stream_turns(turns),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        8,
        true,
        cancel,
        Arc::new(move |e| captured.lock().unwrap().push(e)),
    )
    .await;
    let events = events.lock().unwrap().clone();
    (answer, events, scope)
}

fn stored_run(scope: &Scope) -> Run {
    let RecordData::Run(run) = scope
        .store
        .lock()
        .unwrap()
        .record(&scope.run)
        .unwrap()
        .unwrap()
        .data
    else {
        panic!()
    };
    run
}

#[tokio::test]
async fn allowed_web_search_reports_activity_records_sources_and_is_limited() {
    let (_root, mut request, _) = tests::fixture();
    let fake = Arc::new(FakeWeb::default());
    request.web_search = Some(fake.clone());
    let (answer, events, scope) = run(
        &request,
        vec![
            vec![
                web_call("a", "rust release"),
                web_call("b", "fails"),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                web_call("c", "third"),
                web_call("d", "fourth"),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            done("Cited."),
        ],
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(answer.terminal, AiTerminal::Completed),
        "{answer:?}"
    );
    assert_eq!(
        *fake.queries.lock().unwrap(),
        ["rust release", "fails", "third"],
        "the fourth search is refused before dispatch"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AiEvent::ToolStarted { name, .. } if name == "web_search"))
    );
    assert_eq!(tool_activity("web_search"), Some(WEB_SEARCH_ACTIVITY));
    let sources = stored_run(&scope).web_sources;
    assert_eq!(
        sources.iter().map(|s| s.url.as_str()).collect::<Vec<_>>(),
        [
            "https://example.org/a",
            "https://example.org/rust release",
            "https://example.org/third"
        ],
        "sources are deduplicated by URL in first-use order"
    );
    assert_eq!(sources[0].title.as_deref(), Some("Example A"));
}

#[tokio::test]
async fn without_allowed_web_access_the_tool_is_never_offered() {
    let (_root, request, _) = tests::fixture();
    assert!(request.web_search.is_none());
    let (answer, _, scope) = run(
        &request,
        vec![vec![
            web_call("a", "rust release"),
            MockStreamEvent::final_response_with_total_tokens(1),
        ]],
        CancellationToken::new(),
    )
    .await;
    assert!(!matches!(answer.terminal, AiTerminal::Completed));
    assert!(stored_run(&scope).web_sources.is_empty());
}

#[tokio::test]
async fn a_stopped_run_discards_the_search_result() {
    let (_root, mut request, _) = tests::fixture();
    let fake = Arc::new(FakeWeb::default());
    let cancel = CancellationToken::new();
    *fake.cancel_run.lock().unwrap() = Some(cancel.clone());
    request.web_search = Some(fake.clone());
    let (answer, _, scope) = run(
        &request,
        vec![
            vec![
                web_call("a", "rust release"),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            done("Never."),
        ],
        cancel,
    )
    .await;
    assert!(
        !matches!(answer.terminal, AiTerminal::Completed),
        "{answer:?}"
    );
    assert_eq!(fake.queries.lock().unwrap().len(), 1);
    assert!(stored_run(&scope).web_sources.is_empty());
}

#[test]
fn web_arguments_are_strict() {
    assert!(web::validate(&json!({"query": "rust"})));
    assert!(!web::validate(&json!({"query": "  "})));
    assert!(!web::validate(&json!({"query": "x".repeat(1025)})));
    assert!(!web::validate(&json!({"query": "rust", "chat": "other"})));
}
