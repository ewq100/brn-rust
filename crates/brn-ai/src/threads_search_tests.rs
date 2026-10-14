use super::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};

/// Records the context each call received; the model never chooses it.
#[derive(Default)]
struct FakeSearch {
    calls: Mutex<Vec<String>>,
}

impl RunNoteSearch for FakeSearch {
    fn search(
        &self,
        chat: &str,
        query: &str,
        limit: usize,
        cursor: Option<&str>,
        hybrid: bool,
    ) -> std::result::Result<NoteSearchPage, NoteSearchError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("search {chat} {query} {limit} {cursor:?} {hybrid}"));
        Ok(NoteSearchPage {
            total: 1,
            shown: 1,
            cursor: None,
            semantic: "none".into(),
            results: vec![NoteSearchResult {
                id: "n1".into(),
                version: 1,
                title: "Harbor".into(),
                heading_path: "Plan".into(),
                snippet: "harbor plan".into(),
                start: 0,
                matches: 1,
                protected: false,
                imported: false,
                evidence: false,
                inbound_links: 0,
            }],
        })
    }
    fn connections(
        &self,
        chat: &str,
        note: &str,
        hops: u8,
        sources: bool,
    ) -> std::result::Result<NoteNeighbourhood, NoteSearchError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("connections {chat} {note} {hops} {sources}"));
        if note == "hidden" {
            return Err(NoteSearchError::Rejected);
        }
        if note == "down" {
            return Err(NoteSearchError::Unavailable("search worker stopped".into()));
        }
        Ok(NoteNeighbourhood {
            centre: ConnectionNode {
                id: note.into(),
                version: 1,
                title: "Harbor".into(),
                kind: "note".into(),
                relations: vec![],
                links: vec![],
                via: None,
                inbound_links: 0,
            },
            hops: vec![],
        })
    }
}

fn search_call(id: &str, args: Value) -> MockStreamEvent {
    MockStreamEvent::tool_call(id, "search_notes", args)
}

async fn run_turns(
    request: &ThreadRunRequest,
    turns: Vec<Vec<MockStreamEvent>>,
) -> (AiAnswer, Vec<AiEvent>) {
    let (scope, input) = begin(request).unwrap();
    let model = MockCompletionModel::from_stream_turns(turns);
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let answer = run_model(
        model,
        scope,
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(move |e| captured.lock().unwrap().push(e)),
    )
    .await;
    let events = events.lock().unwrap().clone();
    (answer, events)
}

#[tokio::test]
async fn search_tools_use_the_run_chat_and_report_activity() {
    let (_root, mut request, _) = tests::fixture();
    let fake = Arc::new(FakeSearch::default());
    request.search = Some(fake.clone());
    let (answer, events) = run_turns(
        &request,
        vec![
            vec![
                search_call(
                    "s",
                    json!({"query":"harbor plan","limit":5,"mode":"hybrid","cursor":null}),
                ),
                MockStreamEvent::tool_call(
                    "c",
                    "note_connections",
                    json!({"id":"n1","hops":2,"sources":true}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::text("Found it."),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ],
    )
    .await;
    assert!(
        matches!(answer.terminal, AiTerminal::Completed),
        "{answer:?}"
    );
    let chat = &request.thread;
    assert_eq!(
        *fake.calls.lock().unwrap(),
        [
            format!("search {chat} harbor plan 5 None true"),
            format!("connections {chat} n1 2 true"),
        ]
    );
    for name in search_tool_names() {
        assert!(tool_activity(name).is_some());
        assert!(schema(name).is_object());
        assert!(events.iter().any(
            |e| matches!(e, AiEvent::Activity(text) if text == &format!("< {name} · success"))
        ));
    }
    assert!(!tool_names().contains(&"search_notes"));
}

#[tokio::test]
async fn a_model_argument_cannot_choose_the_search_context() {
    let (_root, mut request, _) = tests::fixture();
    let fake = Arc::new(FakeSearch::default());
    request.search = Some(fake.clone());
    let (answer, _) = run_turns(
        &request,
        vec![vec![
            search_call(
                "s",
                json!({"query":"harbor","limit":5,"mode":"keyword","cursor":null,"chat":"other"}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ]],
    )
    .await;
    assert!(
        matches!(&answer.terminal, AiTerminal::Failed(e) if e.kind == AiErrorKind::InvalidToolUse)
    );
    assert!(fake.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn without_host_search_the_tools_are_not_offered() {
    let (_root, request, _) = tests::fixture();
    assert!(request.search.is_none());
    let (answer, _) = run_turns(
        &request,
        vec![vec![
            search_call(
                "s",
                json!({"query":"harbor","limit":5,"mode":"keyword","cursor":null}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ]],
    )
    .await;
    assert!(!matches!(answer.terminal, AiTerminal::Completed));
}

#[tokio::test]
async fn summary_traces_hash_queries_and_off_records_nothing() {
    let (_root, mut request, _) = tests::fixture();
    request.search = Some(Arc::new(FakeSearch::default()));
    request.trace = TraceLevel::Summary;
    let (scope, input) = begin(&request).unwrap();
    let query = json!({"query":"secret harbor words","limit":5,"mode":"keyword","cursor":null});
    let model = MockCompletionModel::from_stream_turns([
        vec![
            search_call("a", query.clone()),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            search_call("b", query),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("Done."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let answer = run_model(
        model,
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    let trace = finish_trace(&scope, &answer).unwrap();
    assert_eq!(trace.level, TraceLevel::Summary);
    assert_eq!(trace.guide, crate::threads_guides::BUNDLE_VERSION);
    assert_eq!((trace.rounds, trace.calls.len()), (2, 2));
    let text = serde_json::to_string(&trace).unwrap();
    assert!(!text.contains("secret harbor"), "{text}");
    assert_eq!(trace.calls[0].args["words"], 3);
    assert_eq!(
        trace.calls[0].args["query_hash"],
        trace.calls[1].args["query_hash"]
    );
    assert_eq!(trace.calls[0].result["ids"][0], "n1");

    let (_root, mut request, _) = tests::fixture();
    request.trace = TraceLevel::Off;
    let (scope, _) = begin(&request).unwrap();
    assert!(finish_trace(&scope, &answer).is_none());
}

#[test]
fn search_arguments_are_strict_and_bounded() {
    let ok = json!({"query":"harbor","limit":20,"mode":"hybrid","cursor":"12"});
    assert!(parse_valid("search_notes", &ok));
    for bad in [
        json!({"query":"harbor","limit":21,"mode":"hybrid","cursor":null}),
        json!({"query":"harbor","limit":0,"mode":"hybrid","cursor":null}),
        json!({"query":"harbor","limit":5,"mode":"dense","cursor":null}),
        json!({"query":"harbor","limit":5,"mode":"hybrid"}),
        json!({"query":"!!!","limit":5,"mode":"hybrid","cursor":null}),
        json!({"query":"x".repeat(513),"limit":5,"mode":"hybrid","cursor":null}),
        json!({"query":"harbor","limit":5,"mode":"hybrid","cursor":""}),
    ] {
        assert!(!parse_valid("search_notes", &bad), "{bad}");
    }
    assert!(parse_valid(
        "note_connections",
        &json!({"id":"n1","hops":1,"sources":false})
    ));
    for bad in [
        json!({"id":"n1","hops":3,"sources":false}),
        json!({"id":"","hops":1,"sources":false}),
        json!({"id":"n1","hops":1}),
        json!({"id":"n1","hops":1,"sources":false,"chat":"x"}),
    ] {
        assert!(!parse_valid("note_connections", &bad), "{bad}");
    }
}

#[tokio::test]
async fn hidden_notes_are_refused_like_missing_ones_and_outages_are_not() {
    let (_root, mut request, _) = tests::fixture();
    request.search = Some(Arc::new(FakeSearch::default()));
    let (scope, _) = begin(&request).unwrap();
    let args: note_search::ConnectionsArgs =
        serde_json::from_value(json!({"id":"hidden","hops":1,"sources":false})).unwrap();
    let error = note_search::note_connections(scope.clone(), args)
        .await
        .unwrap_err();
    assert_eq!(error.kind, AiErrorKind::ToolRejected);
    let args: note_search::ConnectionsArgs =
        serde_json::from_value(json!({"id":"down","hops":1,"sources":false})).unwrap();
    let error = note_search::note_connections(scope, args)
        .await
        .unwrap_err();
    assert_eq!(error.kind, AiErrorKind::Storage);
}

#[test]
fn with_search_a_listing_names_notes_compactly_and_ids_read_them_whole() {
    let (_root, mut request, note) = tests::fixture();
    let (plain, _) = begin(&request).unwrap();
    let full = read_records(
        &plain,
        ReadArgs {
            ids: vec![],
            query: None,
        },
    )
    .unwrap();
    let row = |v: &Value| {
        v["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == note.as_str())
            .cloned()
            .unwrap()
    };
    assert!(row(&full)["data"]["Note"]["markdown"].is_string());

    request.search = Some(Arc::new(FakeSearch::default()));
    let (scope, _) = begin(&request).unwrap();
    let listed = row(&read_records(
        &scope,
        ReadArgs {
            ids: vec![],
            query: None,
        },
    )
    .unwrap());
    assert_eq!(listed["kind"], "Note");
    assert!(listed["title"].is_string() && listed["bytes"].as_u64().unwrap() > 0);
    assert!(listed.get("data").is_none(), "{listed}");
    let exact = read_records(
        &scope,
        ReadArgs {
            ids: vec![note.clone()],
            query: None,
        },
    )
    .unwrap();
    assert!(row(&exact)["data"]["Note"]["markdown"].is_string());
}

#[test]
fn a_broken_store_rule_is_reported_as_invalid_with_its_reason() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    tests::read_all(&scope);
    let write = json!({"id":note,"expected_version":1,"archived":false,
        "data":{"Note":{"title":"Harbor","markdown":"x","protected":false,"confirmed":false,
            "superseded_by":null,"import":null}}});
    let args: PrepareArgs = serde_json::from_value(json!({"request_key":"dup","request":{
        "reason":"Two writes to one note","writes":[write.clone(), write],"inputs":[]}}))
    .unwrap();
    let result = prepare(&scope, args).unwrap();
    assert_eq!(result["outcome"], "Invalid");
    assert_eq!(result["reason"], "duplicate write target");
}

#[test]
fn source_runs_start_with_intake_playbooks_and_more_rounds() {
    assert_eq!(
        (default_tool_rounds(false), default_tool_rounds(true)),
        (8, 12)
    );
    let with = crate::threads_guides::preamble(crate::threads_guides::preloaded(true));
    let without = crate::threads_guides::preamble(crate::threads_guides::preloaded(false));
    for id in ["intake", "maintain-notes"] {
        let text = crate::threads_guides::read_skill(id).unwrap();
        assert!(with.contains(text) && !without.contains(text));
    }
    assert!(with.contains("- resolve-conflict:") && without.contains("- intake:"));
}
