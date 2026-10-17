//! Cross-slice I01 over real checked state, production HTTP model path and SDK.
//! Scripted semantics are invariant evidence, not live model quality evidence.
use super::*;
use crate::mcp::http_tests::TestServer;
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
use std::sync::atomic::AtomicUsize;

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/work/active/agent-capabilities/int-d/corpus.json"
    ))
    .unwrap()
}
fn next(request: &ThreadRunRequest, prompt: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: request.data_dir.clone(),
        thread: request.thread.clone(),
        prompt: prompt.into(),
        selection: request.selection.clone(),
        effort: request.effort,
        max_tool_rounds: 8,
        auth_file: request.auth_file.clone(),
        credentials_dir: None,
        sources: vec![],
        search: None,
        trace: TraceLevel::Summary,
    }
}
fn save(store: &mut Store, writes: Vec<Put>) {
    let op = store.allocate_operation(&unique_host_key()).unwrap();
    let owner = HostAuthority::owner("INT-D synthetic fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "INT-D checked fixture".into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
fn history(store: &mut Store, chat: &str, c: &Value) -> Vec<String> {
    let mut ids = vec![];
    for batch in (0..c["history_messages"].as_u64().unwrap())
        .collect::<Vec<_>>()
        .chunks(32)
    {
        let allocation = store.allocate_operation(&unique_host_key()).unwrap();
        let writes = batch
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let id = allocation.creation_id(i);
                ids.push(id.clone());
                let text = match n {
                    0 => c["early_requirement"].as_str().unwrap().to_owned(),
                    40 => c["later_proposal"].as_str().unwrap().to_owned(),
                    _ => format!(
                        "Discussion {n}: {}",
                        c["history_filler"].as_str().unwrap().repeat(4)
                    ),
                };
                Put {
                    id,
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: chat.into(),
                        role: if *n != 40 && n % 2 == 0 {
                            "owner"
                        } else {
                            "assistant"
                        }
                        .into(),
                        text,
                    }),
                }
            })
            .collect();
        let owner = HostAuthority::owner("INT-D canonical discussion");
        store
            .prepare_owner(
                &allocation,
                &ChangeRequest {
                    reason: "INT-D canonical history".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        assert!(matches!(
            store.apply(&allocation, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }
    ids
}
fn semantic(c: &Value, id: &str) -> Value {
    json!({
        "objective":{"text":"Preserve Monday only / ainult esmaspäeval","sources":[id],"operations":[]},
        "requirements":[{"text":c["early_requirement"],"sources":[id],"operations":[]}],
        "questions":[],"outcomes":[],
        "proposals":[{"text":c["derived_injection"],"sources":[id],"operations":[]}]
    })
}
fn turn(output: Vec<Value>) -> MockHttpResponse {
    let mut events: Vec<Value> = output.iter().enumerate().map(|(n,item)|json!({"type":"response.output_item.done","output_index":n,"sequence_number":n+1,"item":item})).collect();
    events.push(json!({"type":"response.completed","sequence_number":output.len()+1,"response":{
        "id":"int-d-response","object":"response","created_at":1,"status":"completed","model":"gpt-6.1-sol",
        "output":output,"tools":[],"usage":{"input_tokens":100,"output_tokens":20,"total_tokens":120,"input_tokens_details":{"cached_tokens":10}}
    }}));
    MockHttpResponse::success_typed(cache_tests::sse(&events), "text/event-stream")
}
fn text(text: &str) -> MockHttpResponse {
    turn(vec![
        json!({"type":"message","id":"int-d-answer","status":"completed","role":"assistant","content":[{"type":"output_text","annotations":[],"text":text}]}),
    ])
}
fn call(id: &str, name: &str, args: Value) -> Value {
    json!({"type":"function_call","id":format!("fc_{id}"),"call_id":id,"name":name,"arguments":args.to_string(),"status":"completed"})
}
fn summary_response(c: &Value, id: &str) -> MockHttpResponse {
    text(&semantic(c, id).to_string())
}
fn bodies(http: &SequencedHttpClient) -> Vec<Value> {
    http.requests().iter().map(cache_tests::body).collect()
}
fn chat_bodies(http: &SequencedHttpClient) -> Vec<Value> {
    bodies(http)
        .into_iter()
        .filter(|b| b.get("prompt_cache_key").is_some())
        .collect()
}
fn output(body: &Value, id: &str) -> Value {
    let row = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"] == "function_call_output" && i["call_id"] == id)
        .unwrap();
    serde_json::from_str(row["output"].as_str().unwrap()).unwrap()
}
fn prefix(bodies: &[Value]) {
    for pair in bodies.windows(2) {
        assert_eq!(pair[0]["instructions"], pair[1]["instructions"]);
        assert_eq!(pair[0]["tools"], pair[1]["tools"]);
        let a = pair[0]["input"].as_array().unwrap();
        let b = pair[1]["input"].as_array().unwrap();
        assert_eq!(&b[..a.len()], a);
    }
}
// Parse the maintained serializer's actual input span, then compare the RAW
// captured bytes. Value equality alone could conceal a wire prefix rewrite.
fn raw_input(request: &rig::test_utils::CapturedHttpRequest) -> &[u8] {
    let marker = b"\"input\":";
    let start = request
        .body
        .windows(marker.len())
        .position(|p| p == marker)
        .unwrap()
        + marker.len();
    let tail = &request.body[start..];
    let mut values = serde_json::Deserializer::from_slice(tail).into_iter::<Value>();
    assert!(values.next().unwrap().unwrap().is_array());
    &tail[..values.byte_offset()]
}
fn raw_prefix(http: &SequencedHttpClient) {
    let requests = http.requests();
    let chat: Vec<_> = requests
        .iter()
        .filter(|r| cache_tests::body(r).get("prompt_cache_key").is_some())
        .collect();
    for pair in chat.windows(2) {
        let before = raw_input(pair[0]);
        let after = raw_input(pair[1]);
        assert_eq!(before.last(), Some(&b']'));
        assert!(after.starts_with(&before[..before.len() - 1]));
        assert!(matches!(after[before.len() - 1], b',' | b']'));
    }
}
fn cache(data: &std::path::Path) -> PathBuf {
    std::fs::read_dir(data.join("derived"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("thread-summary-")
                && p.extension().is_some_and(|s| s == "json")
        })
        .unwrap()
}
#[derive(Clone)]
struct AfterResponse {
    at: usize,
    seen: Arc<AtomicUsize>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl rig::http_client::HttpMiddleware for AfterResponse {
    fn after_response<'a>(
        &'a self,
        _: &'a rig::http_client::Method,
        _: &'a rig::http_client::Uri,
        _: rig::http_client::StatusCode,
        _: &'a rig::http_client::HeaderMap,
    ) -> rig_core::wasm_compat::WasmBoxedFuture<'a, rig::http_client::Result<()>> {
        Box::pin(async move {
            if self.seen.fetch_add(1, Ordering::SeqCst) == self.at {
                self.entered.notify_one();
                self.release.notified().await;
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn long_project_summary_selected_skill_optional_sdk_stop_cold_continue_exact_review_replay_undo()
 {
    for external in [false, true] {
        let c = corpus();
        let server = TestServer::start().await;
        let (root, mut request, note) = tests::fixture();
        request.auth_file = Some(cache_tests::codex_file(&root));
        request.trace = TraceLevel::Summary;
        request.max_tool_rounds = 8;
        request.prompt = c["objective"].as_str().unwrap().into();
        cache_tests::configure(&request, "Chatgpt");
        let mut store = Store::open(&request.data_dir).unwrap();
        let project = project_tests::project_command(
            &mut store,
            ProjectCommand::Create {
                title: "INT-D Aurora".into(),
            },
        )
        .writes[0]
            .after
            .id
            .clone();
        project_tests::project_command(
            &mut store,
            ProjectCommand::MoveChat {
                chat: request.thread.clone(),
                project: Some(project.clone()),
            },
        );
        project_tests::project_command(
            &mut store,
            ProjectCommand::LinkDocument {
                project: project.clone(),
                note: note.clone(),
            },
        );
        let before = store.record(&note).unwrap().unwrap();
        let RecordData::Note(mut n) = before.data else {
            panic!()
        };
        n.protected = true;
        n.markdown = c["original_note"].as_str().unwrap().into();
        save(
            &mut store,
            vec![Put {
                id: note.clone(),
                expected_version: Some(before.version),
                archived: false,
                data: RecordData::Note(n),
            }],
        );
        let original = store.record(&note).unwrap().unwrap();
        let ids = history(&mut store, &request.thread, &c);
        assert!(ids.len() > 60);
        let foreign_project = project_tests::project_command(
            &mut store,
            ProjectCommand::Create {
                title: "Other Project".into(),
            },
        )
        .writes[0]
            .after
            .id
            .clone();
        let foreign = project_tests::project_command(
            &mut store,
            ProjectCommand::CreateDocument {
                project: foreign_project,
                title: "Other scope".into(),
                markdown: c["foreign_canary"].as_str().unwrap().into(),
            },
        )
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after
        .id;
        drop(store);
        let source = root.path().join("int-d-package");
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
        let info =
            crate::skill_packages::install(&Store::open(&request.data_dir).unwrap(), &source)
                .unwrap();
        skills_tests::policy(
            &request,
            skills_tests::enabled(&info, Some(project.clone()), true),
        );
        let policy = server
            .policy(vec![McpAccess::Project(project.clone())])
            .await;
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation(&unique_host_key()).unwrap();
        store
            .change_mcp_policy(
                &op,
                Some(&policy),
                &HostAuthority::owner("INT-D exact public SDK approval"),
            )
            .unwrap();
        drop(store);
        let warm = SequencedHttpClient::new([
            summary_response(&c, &ids[0]),
            text("Summary coverage retained; no revision applied."),
        ]);
        let warm_result = run_prepared_over(
            prepare_run(next(&request, &request.prompt)).unwrap(),
            CancellationToken::new(),
            Arc::new(|_| {}),
            Some(DynHttpClient::new(warm.clone())),
        )
        .await
        .unwrap();
        assert!(matches!(warm_result.answer.terminal, AiTerminal::Completed));
        assert_eq!(warm.requests().len(), 2);
        assert_eq!(
            warm_result
                .trace
                .as_ref()
                .unwrap()
                .summary
                .as_ref()
                .unwrap()
                .outcome,
            "updated"
        );
        assert!(!bodies(&warm)[0].to_string().contains("brn-chat-"));
        assert!(bodies(&warm)[0].get("prompt_cache_key").is_none());
        assert!(
            !chat_bodies(&warm)[0]["instructions"]
                .as_str()
                .unwrap()
                .contains("INT_D_SELECTED_BODY")
        );
        assert!(
            chat_bodies(&warm)[0]
                .to_string()
                .contains("UNTRUSTED DERIVED CONTEXT")
        );
        let cache_before = std::fs::read(cache(&request.data_dir)).unwrap();
        let mut prepared = prepare_run(next(&request, &request.prompt)).unwrap();
        assert!(!prepared.summary_chunks.is_empty());
        let public = json!({"query":c["reviewed_public_query"]});
        if external {
            prepared = prepared
                .with_external_input("public_read", &public)
                .unwrap();
        }
        let scope = prepared.scope.clone();
        let old_run = scope.run.clone();
        let requests_before = server.state.requests.load(Ordering::SeqCst);
        let mut reads = vec![
            call(
                "early",
                "read_message",
                json!({"id":ids[0],"version":1,"start":0,"max_bytes":8192}),
            ),
            call(
                "notes",
                "read_records",
                json!({"ids":[note,foreign],"query":null}),
            ),
            call(
                "reference",
                "read_skill",
                json!({"id":info.id,"path":"references/check.md","start":0}),
            ),
        ];
        if external {
            reads.push(call(
                "public",
                &crate::mcp::model_name(&server.endpoint, &policy.tools[0].metadata_digest),
                public,
            ));
        }
        let mut revised = match original.data.clone() {
            RecordData::Note(n) => n,
            _ => panic!(),
        };
        revised.markdown = format!(
            "{}Exact source: Message {} version1 [BRN-I01-EARLY-001].\n",
            c["expected_note_prefix"].as_str().unwrap(),
            ids[0]
        );
        let change = ChangeRequest {
            reason: "Monday retained from exact early source, pending owner review".into(),
            writes: vec![Put {
                id: note.clone(),
                expected_version: Some(original.version),
                archived: false,
                data: RecordData::Note(revised.clone()),
            }],
            inputs: vec![RevisionInput {
                record: ids[0].clone(),
                version: 1,
                expectation: InputExpectation::Current,
            }],
        };
        let mut replies = vec![];
        if external {
            replies.push(summary_response(&c, &ids[0]));
        }
        replies.extend([
            turn(reads),
            turn(vec![call("plan","retain_progress",json!({"summary":"Read exact early source; not applied","objective":c["objective"],"remaining_work":[c["remaining_work"]],"open_questions":["Owner exact review is still required"],"operations":[]}))]),
            turn(vec![call("prepare","prepare_changes",json!({"writing":null,"request_key":"int-d-revision","request":change}))]),
            text("MUST_NOT_BECOME_A_COMPLETED_REPLY_AFTER_STOP"),
        ]);
        let gate = AfterResponse {
            at: replies.len() - 1,
            seen: Arc::new(AtomicUsize::new(0)),
            entered: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
        };
        let http = SequencedHttpClient::new(replies);
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let worker = run_prepared_over(
            prepared,
            cancel,
            Arc::new(|_| {}),
            Some(DynHttpClient::new(http.clone()).with_middleware(gate.clone())),
        );
        let supervise = async {
            tokio::time::timeout(std::time::Duration::from_secs(5), gate.entered.notified())
                .await
                .unwrap();
            let mut store = Store::open(&request.data_dir).unwrap();
            let candidates = store.candidates().unwrap();
            assert_eq!(candidates.len(), 1, "actual prepare must precede Stop");
            assert_eq!(store.record(&note).unwrap().unwrap(), original);
            let op = store.allocate_operation(&unique_host_key()).unwrap();
            store
                .stop_and_post(
                    &op,
                    &old_run,
                    &request.thread,
                    "Owner stopped the prepared revision; preserve exact review",
                    &HostAuthority::owner("INT-D Stop"),
                )
                .unwrap();
            stop.cancel();
            gate.release.notify_one();
        };
        let (stopped, ()) = tokio::join!(worker, supervise);
        let stopped = stopped.unwrap();
        assert!(matches!(stopped.answer.terminal, AiTerminal::Interrupted));
        assert!(stopped.terminal_run.is_none());
        let chat = chat_bodies(&http);
        assert_eq!(chat.len(), 4);
        assert!(
            chat[0]["input"]
                .to_string()
                .contains(c["skill_body"].as_str().unwrap())
        );
        assert!(
            !chat[0]["instructions"]
                .to_string()
                .contains("INT_D_SELECTED_BODY")
        );
        prefix(&chat);
        raw_prefix(&http);
        let early = output(&chat[1], "early");
        assert_eq!(early["id"], ids[0]);
        assert_eq!(early["version"], 1);
        assert_eq!(early["text"], c["early_requirement"]);
        assert_eq!(early["complete"], true);
        let rows = output(&chat[1], "notes");
        assert!(
            !rows
                .to_string()
                .contains(c["foreign_canary"].as_str().unwrap())
        );
        assert_eq!(rows["records"].as_array().unwrap().len(), 1);
        let reference = output(&chat[1], "reference");
        assert_eq!(reference["text"], c["reference_body"]);
        assert_eq!(reference["complete"], true);
        assert_eq!(reference["identity"]["id"], info.id);
        assert_eq!(reference["identity"]["package_digest"], info.digest);
        assert_eq!(reference["identity"]["file"], "references/check.md");
        if external {
            assert!(
                output(&chat[1], "public")["text"]
                    .as_str()
                    .unwrap()
                    .contains("fixture value 17")
            );
            assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
        } else {
            assert!(!chat[0]["tools"].to_string().contains("mcp_"));
            let warm_requests = warm.requests();
            let cold_requests = http.requests();
            let marker = b"History coverage:";
            let cut = warm_requests[1]
                .body
                .windows(marker.len())
                .position(|p| p == marker)
                .unwrap();
            assert!(
                cold_requests[0]
                    .body
                    .starts_with(&warm_requests[1].body[..cut]),
                "unchanged raw prefix between coarse history cuts"
            );
            assert_eq!(
                server.state.requests.load(Ordering::SeqCst),
                requests_before
            );
            assert_eq!(
                std::fs::read(cache(&request.data_dir)).unwrap(),
                cache_before
            );
        }
        let operation = output(&chat[3], "prepare")["operation"]
            .as_str()
            .unwrap()
            .to_owned();
        let store = Store::open(&request.data_dir).unwrap();
        let info_before = store.operation_info(&operation).unwrap();
        assert_eq!(info_before.state, OperationState::Prepared);
        assert!(info_before.receipt.is_none());
        let candidate = info_before.prepared.unwrap();
        assert_eq!(
            candidate.request.writes[0].data,
            RecordData::Note(revised.clone())
        );
        let RecordData::Run(saved) = store.record(&old_run).unwrap().unwrap().data else {
            panic!()
        };
        assert_eq!(saved.state, RunState::Interrupted);
        assert!(
            saved
                .progress
                .contains(c["remaining_work"].as_str().unwrap())
        );
        assert!(saved.operations.contains(&candidate.operation));
        assert!(
            saved.loaded_skill_details.iter().any(|d| d.id == info.id
                && d.package_digest == info.digest
                && d.file == "SKILL.md")
        );
        assert!(saved.loaded_skill_details.iter().any(|d| d.id == info.id
            && d.package_digest == info.digest
            && d.file == "references/check.md"));
        drop(store);
        drop(scope); // actual cold Scope/Store reopen, no retained authority handle
        let fresh = prepare_run(next(&request, c["continue_prompt"].as_str().unwrap())).unwrap();
        assert!(fresh.scope.external.lock().unwrap().is_none());
        assert!(fresh.request.owner_request.is_none());
        assert!(
            !fresh
                .scope
                .reads
                .lock()
                .unwrap()
                .knows(&note, original.version)
        );
        let mut resume = vec![];
        if external {
            resume.push(summary_response(&c, &ids[0]));
        }
        resume.extend([
            turn(vec![call(
                "attempt",
                "apply_changes",
                json!({"operation":operation}),
            )]),
            text("Prepared Monday revision still requires exact owner review."),
        ]);
        let resumed_http = SequencedHttpClient::new(resume);
        let count = server.state.requests.load(Ordering::SeqCst);
        let resumed = run_prepared_over(
            fresh,
            CancellationToken::new(),
            Arc::new(|_| {}),
            Some(DynHttpClient::new(resumed_http.clone())),
        )
        .await
        .unwrap();
        assert!(matches!(resumed.answer.terminal, AiTerminal::Completed));
        let resume_chat = chat_bodies(&resumed_http);
        assert_eq!(resume_chat.len(), 2);
        assert!(
            resume_chat[0]["input"]
                .to_string()
                .contains(c["skill_body"].as_str().unwrap())
        );
        assert!(
            !resume_chat[0]["instructions"]
                .to_string()
                .contains("INT_D_SELECTED_BODY")
        );
        prefix(&resume_chat);
        raw_prefix(&resumed_http);
        assert!(
            resume_chat[0]
                .to_string()
                .contains("UNTRUSTED CONTINUATION")
        );
        assert!(resume_chat[0].to_string().contains(&operation));
        assert!(resume_chat[0].to_string().contains("Prepared"));
        assert_eq!(output(&resume_chat[1], "attempt")["outcome"], "NeedsReview");
        assert!(!resume_chat[0]["tools"].to_string().contains("mcp_"));
        assert_eq!(server.state.requests.load(Ordering::SeqCst), count);
        assert_eq!(
            resumed
                .trace
                .as_ref()
                .unwrap()
                .summary
                .as_ref()
                .unwrap()
                .outcome,
            if external { "updated" } else { "cached" }
        );
        println!(
            "INT_D_SUMMARY_ACCOUNTING {}",
            json!({"mcp":external,
            "warm":{"summary":warm_result.trace.as_ref().unwrap().summary,"captured_summary_requests":bodies(&warm).len()-chat_bodies(&warm).len()},
            "cold":{"summary":stopped.trace.as_ref().unwrap().summary,"captured_summary_requests":bodies(&http).len()-chat_bodies(&http).len()},
            "continue":{"summary":resumed.trace.as_ref().unwrap().summary,"captured_summary_requests":bodies(&resumed_http).len()-resume_chat.len()}})
        );
        let mut store = Store::open(&request.data_dir).unwrap();
        let RecordData::Run(resumed_run) = store.record(&resumed.run_id).unwrap().unwrap().data
        else {
            panic!()
        };
        assert!(
            resumed_run
                .loaded_skill_details
                .iter()
                .any(|d| d.id == info.id
                    && d.package_digest == info.digest
                    && d.file == "SKILL.md")
        );
        assert_eq!(store.record(&note).unwrap().unwrap(), original);
        assert_eq!(store.prepared(&candidate.operation).unwrap(), candidate);
        let owner = HostAuthority::owner("Exact INT-D candidate reviewed by owner");
        let ApplyOutcome::Applied(receipt) = store.apply(&candidate.operation, &owner).unwrap()
        else {
            panic!()
        };
        assert_eq!(receipt.operation, candidate.operation);
        assert_eq!(receipt.request_hash, candidate.request_hash);
        let applied = store.record(&note).unwrap().unwrap();
        assert_eq!(applied.data, RecordData::Note(revised));
        assert_eq!(
            store.apply(&candidate.operation, &owner).unwrap(),
            ApplyOutcome::Applied(receipt.clone())
        );
        assert_eq!(store.record(&note).unwrap().unwrap(), applied);
        let undo = store.allocate_operation(&unique_host_key()).unwrap();
        let reversed = store.undo(&undo, &receipt.operation, &owner).unwrap();
        assert!(matches!(reversed, ApplyOutcome::Applied(_)));
        assert_eq!(store.record(&note).unwrap().unwrap().data, original.data);
        assert_eq!(
            store.undo(&undo, &receipt.operation, &owner).unwrap(),
            reversed
        );
        for id in &ids {
            assert_eq!(store.record(id).unwrap().unwrap().version, 1);
        }
        assert!(
            !serde_json::to_string(&store.records().unwrap())
                .unwrap()
                .contains("MUST_NOT_BECOME_A_COMPLETED_REPLY_AFTER_STOP")
        );
        server.stop().await;
    }
}

#[tokio::test]
async fn offline_prepare_signature_and_existing_activity_do_not_identify_historical_cause() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    use std::error::Error as _;

    let mut facts = vec![];
    for mode in ["absent", "null", "object", "stopped"] {
        let (_root, mut request, note) = tests::fixture();
        request.trace = TraceLevel::Full;
        let mut store = Store::open(&request.data_dir).unwrap();
        let prior = store.record(&note).unwrap().unwrap();
        let RecordData::Note(mut data) = prior.data else {
            panic!()
        };
        data.protected = true;
        data.markdown = "INT_D_DIAGNOSTIC_NOTE_BODY_ONLY".into();
        save(
            &mut store,
            vec![Put {
                id: note.clone(),
                expected_version: Some(prior.version),
                archived: false,
                data: RecordData::Note(data),
            }],
        );
        let original = store.record(&note).unwrap().unwrap();
        let mut c = corpus();
        c["history_messages"] = json!(2);
        let messages = history(&mut store, &request.thread, &c);
        let note_receipts = store
            .receipts()
            .unwrap()
            .into_iter()
            .filter(|r| r.writes.iter().any(|w| w.after.id == note))
            .collect::<Vec<_>>();
        drop(store);
        let (scope, input) = begin(&request).unwrap();
        assert!(scope.writing_snapshot.is_none() && scope.owner_request.is_none());
        assert!(scope.external.lock().unwrap().is_none() && scope.helper.is_none());
        let RecordData::Note(mut revised) = original.data.clone() else {
            panic!()
        };
        revised.markdown = "INT_D_DIAGNOSTIC_PROPOSED_BODY_ONLY".into();
        let change = ChangeRequest {
            reason: "New synthetic diagnostic request; not recovered live arguments".into(),
            writes: vec![Put {
                id: note.clone(),
                expected_version: Some(original.version),
                archived: false,
                data: RecordData::Note(revised),
            }],
            inputs: vec![
                RevisionInput {
                    record: note.clone(),
                    version: original.version,
                    expectation: InputExpectation::Current,
                },
                RevisionInput {
                    record: messages[0].clone(),
                    version: 1,
                    expectation: InputExpectation::Current,
                },
                RevisionInput {
                    record: messages[1].clone(),
                    version: 1,
                    expectation: InputExpectation::Current,
                },
            ],
        };
        let mut args = json!({"request_key":"offline-diagnostic","request":change});
        if mode == "null" {
            args["writing"] = Value::Null;
        }
        if mode == "object" {
            args["writing"] = json!({"quotations":[],"requirements_reported":[]});
        }
        assert!(parse_valid("prepare_changes", &args));
        let typed = Arc::new(Mutex::new(Vec::new()));
        if mode == "object" {
            let error = prepare(&scope, serde_json::from_value(args.clone()).unwrap()).unwrap_err();
            assert_eq!(error.kind, AiErrorKind::ToolRejected);
            typed.lock().unwrap().push(error.kind);
        }
        let mut turns = vec![
            vec![
                MockStreamEvent::tool_call(
                    "read",
                    "read_records",
                    json!({"ids":[note],"query":null}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call("proposal", "prepare_changes", args.clone()),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ];
        if mode == "object" {
            let mut long = args.clone();
            long["request_key"] = json!("offline-diagnostic-long");
            long["request"]["reason"] = json!("Public synthetic long metadata. ".repeat(100));
            assert!(parse_valid("prepare_changes", &long));
            turns.push(vec![
                MockStreamEvent::tool_call("long", "prepare_changes", long),
                MockStreamEvent::final_response_with_total_tokens(1),
            ]);
        }
        turns.push(vec![
            MockStreamEvent::text("Diagnostic ended; nothing applied."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ]);
        let model = MockCompletionModel::from_stream_turns(turns);
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let stop_scope = scope.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let did_stop = stopped.clone();
        let direct_args = args.clone();
        let direct_typed = typed.clone();
        let stop_at_dispatch = mode == "stopped";
        let emit = Arc::new(move |event| {
            if let AiEvent::Activity(text) = event {
                assert!(text.len() <= 2200);
                assert!(!text.contains("INT_D_DIAGNOSTIC_NOTE_BODY_ONLY"));
                assert!(!text.contains("INT_D_DIAGNOSTIC_PROPOSED_BODY_ONLY"));
                let mut rows = captured.lock().unwrap();
                assert!(rows.len() < 16);
                rows.push(text.clone());
                drop(rows);
                if stop_at_dispatch && text.starts_with("> prepare_changes ") {
                    assert!(!did_stop.swap(true, Ordering::SeqCst));
                    let mut store = stop_scope.store.lock().unwrap();
                    let op = store
                        .allocate_operation("offline-diagnostic-checked-stop")
                        .unwrap();
                    assert!(matches!(
                        store
                            .stop_and_post(
                                &op,
                                &stop_scope.run,
                                &stop_scope.chat,
                                "Synthetic checked Stop before prepare handler",
                                &HostAuthority::owner("Offline diagnostic Stop")
                            )
                            .unwrap(),
                        ApplyOutcome::Applied(_)
                    ));
                    drop(store);
                    let error = prepare(
                        &stop_scope,
                        serde_json::from_value(direct_args.clone()).unwrap(),
                    )
                    .unwrap_err();
                    assert_eq!(error.kind, AiErrorKind::ToolRejected);
                    direct_typed.lock().unwrap().push(error.kind);
                }
            }
        });
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            CancellationToken::new(),
            emit,
        )
        .await;
        assert_eq!(
            model.request_count(),
            if mode == "object" || mode == "stopped" {
                2
            } else {
                3
            }
        );
        let trace = scope
            .trace
            .as_ref()
            .unwrap()
            .finish("offline diagnostic", false, 4);
        assert_eq!(trace.dropped_calls, 0);
        if mode == "object" {
            assert_eq!(trace.stopped.as_deref(), Some("invalid_batch"));
            assert!(matches!(
                answer.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::InvalidToolUse,
                    ..
                })
            ));
        } else {
            assert_ne!(trace.stopped.as_deref(), Some("invalid_batch"));
        }
        let calls = trace
            .calls
            .iter()
            .filter(|c| c.tool == "prepare_changes")
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), if mode == "object" { 0 } else { 1 });
        let denied = mode == "object" || mode == "stopped";
        for call in &calls {
            assert_eq!(call.args, json!({"writes":{"Note":1},"inputs":3}));
            assert_eq!(call.outcome, if denied { "error" } else { "success" });
            if denied {
                assert_eq!(call.bytes, 15);
                assert!(call.result.is_null());
            }
        }
        if denied {
            assert_eq!(*typed.lock().unwrap(), vec![AiErrorKind::ToolRejected]);
            // The maintained native adapter retains a typed source but deliberately
            // gives the model only generic feedback. This is not a second driver.
            let mut cx = ToolContext::new().with_scope(scope.clone());
            let native = rig::tool::ErasedTool::execute(
                &PrepareChanges::for_scope(&scope),
                args.to_string(),
                &mut cx,
            )
            .await;
            assert_eq!(native.output().as_text(), Some("the tool failed"));
            let error = native.error().unwrap();
            assert_eq!(error.kind(), rig::tool::ToolErrorKind::Other);
            assert_eq!(
                error
                    .source()
                    .unwrap()
                    .downcast_ref::<AiError>()
                    .unwrap()
                    .kind,
                AiErrorKind::ToolRejected
            );
            if mode == "object" {
                assert!(
                    !serde_json::to_string(&model.requests())
                        .unwrap()
                        .contains("the tool failed"),
                    "the invalid batch never dispatches the handler or its generic feedback"
                );
            }
        }
        let activity = events.lock().unwrap();
        let dispatch = activity
            .iter()
            .filter_map(|s| s.strip_prefix("> prepare_changes "))
            .collect::<Vec<_>>();
        assert_eq!(dispatch.len(), calls.len());
        let (classification, shown) = if mode == "object" {
            assert!(
                dispatch.is_empty(),
                "unsupported writing must fail before Activity/handler dispatch"
            );
            ("not_dispatched", Value::Null)
        } else {
            let shown: Value = serde_json::from_str(dispatch[0]).unwrap();
            let classification = match shown.get("writing") {
                None => "absent",
                Some(Value::Null) => "null",
                _ => panic!("only supported ordinary arguments dispatched"),
            };
            assert_eq!(
                classification,
                if mode == "stopped" { "absent" } else { mode }
            );
            assert_eq!(shown["request"]["inputs"], args["request"]["inputs"]);
            assert_eq!(
                shown["request"]["writes"][0]["data"],
                "[record content omitted]"
            );
            (classification, shown)
        };
        // Long ordinary-object Activity was measured at a65; the new contract
        // rejects the first such batch, so no short/long prepare frame is emitted.
        let long_id_visibility: Option<[bool; 3]> = None;
        drop(activity);
        let persisted = finish(&scope, &answer).unwrap();
        assert_eq!(persisted, mode != "stopped");
        let store = scope.store.lock().unwrap();
        assert_eq!(store.record(&note).unwrap().unwrap(), original);
        let current_note_receipts = store
            .receipts()
            .unwrap()
            .into_iter()
            .filter(|r| r.writes.iter().any(|w| w.after.id == note))
            .collect::<Vec<_>>();
        assert_eq!(current_note_receipts, note_receipts);
        let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
            panic!()
        };
        let candidates = store.candidates().unwrap();
        if denied {
            assert!(run.operations.is_empty() && candidates.is_empty());
        } else {
            assert!(matches!(answer.terminal, AiTerminal::Completed));
            assert_eq!(run.operations.len(), 1);
            assert_eq!(candidates.len(), 1);
            let prepared = store.prepared(&run.operations[0]).unwrap();
            assert_eq!(prepared.request.writes, change.writes);
            assert!(store.receipt(&prepared.operation).unwrap().is_none());
        }
        if mode == "stopped" {
            assert!(stopped.load(Ordering::SeqCst));
            assert_eq!(run.state, RunState::Interrupted);
            assert_eq!(run.fence, 2);
        }
        facts.push(json!({"mode":mode,"new_fixture_not_original_args":true,
            "mock_model_requests":model.request_count(),"prepare_calls":calls.len(),
            "prepare_outcomes":calls.iter().map(|c|&c.outcome).collect::<Vec<_>>(),
            "error_bytes":calls.iter().filter(|c|c.outcome=="error").map(|c|c.bytes).collect::<Vec<_>>(),
            "short_activity_writing":classification,"short_activity_inputs":shown["request"]["inputs"],
            "long_activity_writing":if mode=="object" {"not dispatched: invalid batch"} else {"not exercised"},
            "long_activity_input_id_visible":long_id_visibility,
            "typed_direct_error":if denied {Some("ToolRejected")} else {None},
            "registered_operations":run.operations.len(),"candidates":candidates.len(),
            "note_and_its_receipts_unchanged":true,"checked_stop":stopped.load(Ordering::SeqCst)}));
    }
    println!(
        "INT_D_OFFLINE_PREPARE_DIAGNOSTIC {}",
        json!({"cases":facts,"historical_cause":"UNKNOWN","live_starts":0})
    );
}

// Only the identically cfg-gated parent hooks call these passive observers.
// The callback owns any retention; neither canonical state nor model output changes.
const CAPTURE_PREFIX: &str = "INT_D_TEST_CAPTURE_V1 ";
const CAPTURE_BYTES: usize = 2048;
const CAPTURE_ARGS_BYTES: usize = 1024 * 1024;

fn capture_id(value: Option<&Value>) -> Option<&str> {
    let text = value?.as_str()?;
    let (base, index) = text.rsplit_once(':')?;
    if text.len() > 64
        || index.is_empty()
        || index.len() > 20
        || !index.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let uuid = base.len() == 36
        && base.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        });
    (uuid || (base == "new" && index.parse::<u8>().is_ok_and(|i| i < 64))).then_some(text)
}
fn capture_rows(value: Option<&Value>, writes: bool) -> Value {
    let Some(rows) = value.and_then(Value::as_array) else {
        return json!({"total":null,"shown":[],"omitted":null});
    };
    let limit = if writes { 2 } else { 4 };
    let shown = rows
        .iter()
        .take(limit)
        .filter_map(|row| {
            let id = capture_id(row.get(if writes { "id" } else { "record" }))?;
            if writes {
                let version = row.get("expected_version")?;
                if !version.is_null() && version.as_u64().is_none() {
                    return None;
                }
                let kind = row
                    .get("data")
                    .and_then(Value::as_object)
                    .filter(|o| o.len() == 1)
                    .and_then(|o| o.keys().next())
                    .map(String::as_str);
                let kind = match kind {
                    Some("Note") => "Note",
                    Some("Thread") => "Thread",
                    Some("Action") => "Action",
                    Some("Source") => "Source",
                    Some("Link") => "Link",
                    Some("Comment") => "Comment",
                    _ => "UNKNOWN",
                };
                Some(json!({"id":id,"expected_version":version,"kind":kind}))
            } else {
                let version = row.get("version")?.as_u64()?;
                let expectation = match row.get("expectation") {
                    None => "Current",
                    Some(Value::String(s)) if s == "Current" => "Current",
                    Some(Value::String(s)) if s == "Historical" => "Historical",
                    Some(Value::String(s)) if s == "ChatContext" => "ChatContext",
                    _ => "UNKNOWN",
                };
                Some(json!({"record":id,"version":version,"expectation":expectation}))
            }
        })
        .collect::<Vec<_>>();
    json!({"total":rows.len(),"omitted":rows.len()-shown.len(),"shown":shown})
}
fn capture_arguments(id: u64, round: u16, raw: Option<&str>) -> Value {
    use sha2::{Digest, Sha256};
    let parsed = raw
        .filter(|s| s.len() <= CAPTURE_ARGS_BYTES)
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .filter(Value::is_object);
    let args = parsed.as_ref();
    let writing = match args {
        None => "UNKNOWN",
        Some(a) => match a.get("writing") {
            None => "ABSENT",
            Some(Value::Null) => "NULL",
            Some(Value::Object(_)) => "OBJECT",
            _ => "OTHER",
        },
    };
    let key = args
        .and_then(|a| a.get("request_key"))
        .and_then(Value::as_str);
    let key = key.map(|s| {
        json!({"bytes":s.len(),"sha256":(s.len()<=128)
        .then(||format!("{:x}",Sha256::digest(s.as_bytes())))})
    });
    json!({"event":"dispatch","dispatch":id,"round":round,
        "arguments":if args.is_some(){"PARSED"}else if raw.is_some_and(|s|s.len()>CAPTURE_ARGS_BYTES){"UNAVAILABLE_SIZE"}else{"UNAVAILABLE_PARSE"},
        "writing":writing,"request_key":key,
        "writes":capture_rows(args.and_then(|a|a.get("request")).and_then(|a|a.get("writes")),true),
        "inputs":capture_rows(args.and_then(|a|a.get("request")).and_then(|a|a.get("inputs")),false)})
}
fn capture_emit(emit: &Arc<dyn Fn(AiEvent) + Send + Sync>, mut value: Value) {
    let mut text = format!("{CAPTURE_PREFIX}{value}");
    if text.len() > CAPTURE_BYTES {
        value = json!({"event":value.get("event"),"dispatch":value.get("dispatch"),"round":value.get("round"),
            "arguments":"OMITTED_CAP","writing":"UNKNOWN","request_key":null,
            "writes":{"total":value["writes"]["total"],"shown":[],"omitted":value["writes"]["total"]},
            "inputs":{"total":value["inputs"]["total"],"shown":[],"omitted":value["inputs"]["total"]}});
        text = format!("{CAPTURE_PREFIX}{value}");
    }
    (emit)(AiEvent::Activity(text));
}
pub(super) fn capture_dispatch(
    emit: &Arc<dyn Fn(AiEvent) + Send + Sync>,
    event: &DispatchEvent<'_>,
    round: u16,
) {
    capture_emit(
        emit,
        capture_arguments(event.id.as_u64(), round, event.tool_args()),
    );
}
fn capture_result(result: Option<&rig::tool::ToolResult>) -> Value {
    use std::error::Error as _;
    let error = result.and_then(|r| r.error().or_else(|| r.refusal()));
    let source = error
        .and_then(|e| e.source())
        .and_then(|e| e.downcast_ref::<AiError>());
    json!({"status":result.map(|r|r.status_name()).unwrap_or("UNKNOWN"),
        "native_kind":error.map(|e|e.kind().as_str()).unwrap_or(if result.is_some(){"NONE"}else{"UNKNOWN"}),
        "source_kind":source.map(|e|json!(e.kind)).unwrap_or(json!(if result.is_none() || error.is_some(){"UNKNOWN"}else{"NONE"}))})
}
pub(super) fn capture_outcome(
    emit: &Arc<dyn Fn(AiEvent) + Send + Sync>,
    event: &OutcomeEvent<'_>,
    round: u16,
) {
    let mut value = capture_result(event.tool_result());
    value["event"] = json!("outcome");
    value["dispatch"] = json!(event.id.as_u64());
    value["round"] = json!(round);
    capture_emit(emit, value);
}

#[tokio::test]
async fn capture_matches_real_prepare_outcomes_and_stops_before_same_batch_apply() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let mut facts = vec![];
    for mode in ["absent", "null", "object-long", "apply-first"] {
        let (_root, mut request, note) = tests::fixture();
        request.trace = TraceLevel::Full;
        let mut store = Store::open(&request.data_dir).unwrap();
        let old = store.record(&note).unwrap().unwrap();
        let RecordData::Note(mut data) = old.data else {
            panic!()
        };
        data.protected = true;
        data.markdown = "CAPTURE_ORIGINAL_BODY_CANARY".into();
        save(
            &mut store,
            vec![Put {
                id: note.clone(),
                expected_version: Some(old.version),
                archived: false,
                data: RecordData::Note(data),
            }],
        );
        let original = store.record(&note).unwrap().unwrap();
        let mut c = corpus();
        c["history_messages"] = json!(2);
        let ids = history(&mut store, &request.thread, &c);
        let before_receipts = store
            .receipts()
            .unwrap()
            .into_iter()
            .filter(|r| r.writes.iter().any(|w| w.after.id == note))
            .collect::<Vec<_>>();
        drop(store);
        let (scope, input) = begin(&request).unwrap();
        tests::read_all(&scope);
        assert!(scope.owner_request.is_none() && scope.writing_snapshot.is_none());
        let RecordData::Note(mut revised) = original.data.clone() else {
            panic!()
        };
        revised.markdown = "CAPTURE_PROPOSED_BODY_CANARY".repeat(500);
        let inputs = vec![
            RevisionInput {
                record: note.clone(),
                version: original.version,
                expectation: InputExpectation::Current,
            },
            RevisionInput {
                record: ids[0].clone(),
                version: 1,
                expectation: InputExpectation::Current,
            },
            RevisionInput {
                record: ids[1].clone(),
                version: 1,
                expectation: InputExpectation::Current,
            },
        ];
        let request_key = "CAPTURE_REQUEST_KEY_CANARY";
        let mut args = json!({"request_key":request_key,"request":ChangeRequest {
            reason:"CAPTURE_REASON_CANARY".repeat(500),
            writes:vec![Put {id:note.clone(),expected_version:Some(original.version),archived:false,data:RecordData::Note(revised)}],
            inputs:inputs.clone(),
        }});
        if mode == "null" {
            args["writing"] = Value::Null;
        }
        if mode == "object-long" {
            args["writing"] = json!({"quotations":[{"id":note,"version":original.version,"start":0,"end":1,"quote":"CAPTURE_QUOTE_CANARY".repeat(500)}],
                "requirements_reported":[{"claim":"CAPTURE_CLAIM_CANARY","quotations":[0]}]});
        }
        assert!(args.to_string().len() > 2048 && parse_valid("prepare_changes", &args));
        let op = scope
            .store
            .lock()
            .unwrap()
            .allocate_operation(&format!("{}:proposal:{request_key}", scope.run))
            .unwrap();
        if mode == "object-long" {
            let error = prepare(&scope, serde_json::from_value(args.clone()).unwrap()).unwrap_err();
            assert_eq!(error.kind, AiErrorKind::ToolRejected);
        }
        if mode == "apply-first" {
            prepare(&scope, serde_json::from_value(args.clone()).unwrap()).unwrap();
        }
        let calls = if mode == "apply-first" {
            vec![MockStreamEvent::tool_call(
                "apply",
                "apply_changes",
                json!({"operation":op.as_str()}),
            )]
        } else {
            vec![
                MockStreamEvent::tool_call("prepare", "prepare_changes", args.clone()),
                MockStreamEvent::tool_call(
                    "apply",
                    "apply_changes",
                    json!({"operation":op.as_str()}),
                ),
            ]
        };
        let mut turn = calls;
        turn.push(MockStreamEvent::final_response_with_total_tokens(1));
        let model = MockCompletionModel::from_stream_turns([
            turn,
            vec![
                MockStreamEvent::text("must not request another turn"),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ]);
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_out = captured.clone();
        let did_stop = Arc::new(AtomicBool::new(false));
        let stopped = did_stop.clone();
        let owned_scope = scope.clone();
        let emit = Arc::new(move |event| {
            let AiEvent::Activity(text) = event else {
                return;
            };
            let first_prepare_outcome = if let Some(raw) = text.strip_prefix(CAPTURE_PREFIX) {
                assert!(text.len() <= CAPTURE_BYTES);
                for secret in [
                    "CAPTURE_ORIGINAL_BODY_CANARY",
                    "CAPTURE_PROPOSED_BODY_CANARY",
                    "CAPTURE_QUOTE_CANARY",
                    "CAPTURE_CLAIM_CANARY",
                    "CAPTURE_REASON_CANARY",
                    "CAPTURE_REQUEST_KEY_CANARY",
                ] {
                    assert!(!text.contains(secret));
                }
                let row: Value = serde_json::from_str(raw).unwrap();
                let outcome = row["event"] == "outcome";
                let mut rows = captured_out.lock().unwrap();
                assert!(rows.len() < 4);
                rows.push(row);
                outcome
            } else {
                false
            };
            // The reservation/host action is this invocation's test caller, never
            // a decision made by tool text. The pre-apply guard also covers an
            // apply-first batch, with no preceding prepare outcome.
            if (first_prepare_outcome || text.starts_with("> apply_changes "))
                && !stopped.swap(true, Ordering::SeqCst)
            {
                let mut store = owned_scope.store.lock().unwrap();
                let operation = store
                    .allocate_operation("capture-stop-first-outcome-or-before-apply")
                    .unwrap();
                assert!(matches!(
                    store
                        .stop_and_post(
                            &operation,
                            &owned_scope.run,
                            &owned_scope.chat,
                            "Synthetic capture stopped; no Apply authorized",
                            &HostAuthority::owner("Capture offline Stop guard")
                        )
                        .unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                drop(store);
                stop.cancel();
            }
        });
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            cancel,
            emit,
        )
        .await;
        assert_eq!(did_stop.load(Ordering::SeqCst), mode != "object-long");
        if mode == "object-long" {
            assert!(matches!(
                answer.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::InvalidToolUse,
                    ..
                })
            ));
        }
        assert_eq!(model.request_count(), 1);
        assert_eq!(finish(&scope, &answer).unwrap(), mode == "object-long");
        let rows = captured.lock().unwrap();
        if mode == "apply-first" || mode == "object-long" {
            assert!(rows.is_empty());
        } else {
            assert_eq!(rows.len(), 2);
            let (dispatch, outcome) = (&rows[0], &rows[1]);
            assert_eq!(dispatch["event"], "dispatch");
            assert_eq!(outcome["event"], "outcome");
            assert_eq!(dispatch["dispatch"], outcome["dispatch"]);
            assert_eq!(dispatch["round"], outcome["round"]);
            assert!(dispatch["dispatch"].is_u64());
            assert_eq!(dispatch["round"], 1);
            assert_eq!(dispatch["arguments"], "PARSED");
            assert_eq!(
                dispatch["writing"],
                match mode {
                    "absent" => "ABSENT",
                    "null" => "NULL",
                    _ => "OBJECT",
                }
            );
            assert_eq!(dispatch["inputs"]["total"], 3);
            assert_eq!(dispatch["inputs"]["omitted"], 0);
            assert_eq!(dispatch["inputs"]["shown"],json!(inputs.iter().map(|i|json!({"record":i.record,"version":i.version,"expectation":"Current"})).collect::<Vec<_>>()));
            assert_eq!(
                dispatch["writes"]["shown"],
                json!([{"id":note,"expected_version":original.version,"kind":"Note"}])
            );
            assert_eq!(dispatch["request_key"]["bytes"], request_key.len());
            use sha2::Digest;
            assert_eq!(
                dispatch["request_key"]["sha256"],
                format!("{:x}", sha2::Sha256::digest(request_key.as_bytes()))
            );
            assert_eq!(
                outcome["status"],
                if mode == "object-long" {
                    "error"
                } else {
                    "success"
                }
            );
            assert_eq!(
                outcome["native_kind"],
                if mode == "object-long" {
                    "other"
                } else {
                    "NONE"
                }
            );
            assert_eq!(
                outcome["source_kind"],
                if mode == "object-long" {
                    "tool_rejected"
                } else {
                    "NONE"
                }
            );
        }
        let trace = scope
            .trace
            .as_ref()
            .unwrap()
            .finish("capture offline", false, 4);
        let serialized = serde_json::to_string(&trace).unwrap();
        assert!(!serialized.contains(CAPTURE_PREFIX) && !serialized.contains("source_kind"));
        assert!(!trace.calls.iter().any(|c| c.tool == "apply_changes"
            && matches!(
                c.outcome.as_str(),
                "Applied" | "NeedsReview" | "Deferred" | "Stale"
            )));
        let store = scope.store.lock().unwrap();
        assert_eq!(store.record(&note).unwrap().unwrap(), original);
        assert_eq!(
            store
                .receipts()
                .unwrap()
                .into_iter()
                .filter(|r| r.writes.iter().any(|w| w.after.id == note))
                .collect::<Vec<_>>(),
            before_receipts
        );
        assert!(store.receipt(&op).unwrap().is_none());
        let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
            panic!()
        };
        assert_eq!(
            run.state,
            if mode == "object-long" {
                RunState::Failed
            } else {
                RunState::Interrupted
            }
        );
        assert_eq!(run.fence, 2);
        assert!(
            !serde_json::to_string(&store.records().unwrap())
                .unwrap()
                .contains(CAPTURE_PREFIX)
        );
        if mode == "object-long" {
            assert!(run.operations.is_empty() && store.candidates().unwrap().is_empty());
        } else {
            assert_eq!(run.operations, vec![op.clone()]);
            assert_eq!(store.prepared(&op).unwrap().operation, op);
        }
        facts.push(json!({"mode":mode,"mock_requests":model.request_count(),"capture":*rows,"saved_state":run.state,"saved_fence":run.fence,"checked_stop":did_stop.load(Ordering::SeqCst),"note_and_note_receipts_unchanged":true}));
    }
    println!(
        "INT_D_TEST_CAPTURE_FACTS {}",
        json!({"cases":facts,"historical_cause":"UNKNOWN","live_starts":0})
    );
}

#[test]
fn capture_projection_refuses_unavailable_or_unsafe_metadata_without_raw_fallback() {
    assert_eq!(
        capture_result(None),
        json!({"status":"UNKNOWN","native_kind":"UNKNOWN","source_kind":"UNKNOWN"})
    );
    let foreign = rig::tool::ToolResult::failed(rig::tool::ToolExecutionError::from_error(
        std::io::Error::other("CAPTURE_PRIVATE_ERROR_CANARY"),
    ));
    let storage = rig::tool::ToolResult::failed(rig::tool::ToolExecutionError::from_error(
        AiError::new(AiErrorKind::Storage),
    ));
    assert_eq!(
        capture_result(Some(&foreign)),
        json!({"status":"error","native_kind":"other","source_kind":"UNKNOWN"})
    );
    assert_eq!(
        capture_result(Some(&storage)),
        json!({"status":"error","native_kind":"other","source_kind":"storage"})
    );
    assert!(
        !capture_result(Some(&foreign))
            .to_string()
            .contains("CAPTURE_PRIVATE_ERROR_CANARY")
    );

    let rows = Arc::new(Mutex::new(Vec::new()));
    let out = rows.clone();
    let emit: Arc<dyn Fn(AiEvent) + Send + Sync> = Arc::new(move |e| {
        if let AiEvent::Activity(s) = e {
            assert!(s.len() <= CAPTURE_BYTES);
            out.lock().unwrap().push(s);
        }
    });
    for raw in [None, Some("{broken"), Some("null")] {
        capture_emit(&emit, capture_arguments(1, 1, raw));
    }
    let huge = "X".repeat(CAPTURE_ARGS_BYTES + 1);
    capture_emit(&emit, capture_arguments(2, 2, Some(&huge)));
    let id = "01234567-89ab-cdef-0123-456789abcdef:0";
    let bad = "https://never-resolve.invalid/CAPTURE_UNSAFE_ID_CANARY";
    let args = json!({"request_key":"CAPTURE_KEY_CANARY".repeat(100),"writing":"CAPTURE_OTHER_CANARY","request":{
        "reason":"CAPTURE_REASON_CANARY","writes":[{"id":bad,"expected_version":null,"data":{"CAPTURE_KIND_CANARY":{}}}],
        "inputs":(0..12).map(|_|json!({"record":id,"version":u64::MAX,"expectation":"Historical"})).collect::<Vec<_>>()}});
    capture_emit(&emit, capture_arguments(3, 3, Some(&args.to_string())));
    let rows = rows.lock().unwrap();
    assert_eq!(rows.len(), 5);
    let values = rows
        .iter()
        .map(|s| serde_json::from_str::<Value>(s.strip_prefix(CAPTURE_PREFIX).unwrap()).unwrap())
        .collect::<Vec<_>>();
    for v in &values[..4] {
        assert_eq!(v["writing"], "UNKNOWN");
        assert!(v["inputs"]["total"].is_null());
    }
    assert_eq!(values[3]["arguments"], "UNAVAILABLE_SIZE");
    assert_eq!(values[4]["writing"], "OTHER");
    assert!(values[4]["request_key"]["sha256"].is_null());
    assert_eq!(values[4]["writes"]["total"], 1);
    assert_eq!(values[4]["writes"]["omitted"], 1);
    assert_eq!(values[4]["inputs"]["total"], 12);
    assert_eq!(values[4]["inputs"]["shown"].as_array().unwrap().len(), 4);
    assert_eq!(values[4]["inputs"]["omitted"], 8);
    for raw in rows.iter() {
        for secret in [
            bad,
            "CAPTURE_KEY_CANARY",
            "CAPTURE_REASON_CANARY",
            "CAPTURE_OTHER_CANARY",
            "CAPTURE_KIND_CANARY",
        ] {
            assert!(!raw.contains(secret));
        }
    }
}

// Manual qualification is deliberately absent from normal/default execution.
#[cfg(target_os = "macos")]
mod capture_runner {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        os::unix::fs::OpenOptionsExt,
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    const AUTH_REFERENCE: &str = "/Users/evokessler/.codex/auth.json";
    const ROOT: &str = env!("CARGO_MANIFEST_DIR");
    const ACTIVATION_FORMAT: &str = "BRN-INT-D-CAPTURE-ONE-START-20261017-V1";
    type Checked<T> = std::result::Result<T, &'static str>;

    fn digest(bytes: &[u8]) -> String {
        use sha2::Digest;
        format!("{:x}", sha2::Sha256::digest(bytes))
    }
    fn regular(path: &Path, max: u64) -> Checked<File> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| "file unavailable")?;
        let meta = file.metadata().map_err(|_| "file metadata unavailable")?;
        if !meta.is_file() || meta.len() > max {
            return Err("file type or size refused");
        }
        Ok(file)
    }
    fn bytes(path: &Path, max: u64) -> Checked<Vec<u8>> {
        let mut out = Vec::new();
        regular(path, max)?
            .take(max + 1)
            .read_to_end(&mut out)
            .map_err(|_| "read failed")?;
        if out.len() as u64 > max {
            return Err("read grew beyond cap");
        }
        Ok(out)
    }
    fn hash_file(path: &Path, max: u64) -> Checked<String> {
        use sha2::Digest;
        let mut file = regular(path, max)?.take(max + 1);
        let mut hash = sha2::Sha256::new();
        let mut buf = [0; 16384];
        let mut total = 0;
        loop {
            let n = file.read(&mut buf).map_err(|_| "hash read failed")?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > max {
                return Err("hash grew beyond cap");
            }
            hash.update(&buf[..n]);
        }
        Ok(format!("{:x}", hash.finalize()))
    }
    fn owned(path: &Path, prefix: &str, exists: bool) -> Checked<()> {
        if !path.is_absolute()
            || path.parent() != Some(Path::new("/private/tmp"))
            || !path.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
                s.starts_with(prefix)
                    && s.len() <= 150
                    && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
        {
            return Err("explicit owned path required");
        }
        if exists {
            if path.canonicalize().map_err(|_| "root unavailable")? != path
                || !fs::symlink_metadata(path)
                    .map_err(|_| "root unavailable")?
                    .is_dir()
            {
                return Err("real root required");
            }
        } else if fs::symlink_metadata(path).is_ok() {
            return Err("output occupied");
        }
        Ok(())
    }
    fn optional_hash(path: &Path) -> Checked<Option<String>> {
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("sidecar unavailable"),
            Ok(_) => hash_file(path, 32 * 1024 * 1024).map(Some),
        }
    }
    fn db_stamp(data: &Path) -> Checked<Value> {
        Ok(
            json!({"main":hash_file(&data.join("threads.sqlite3"),32*1024*1024)?,
            "wal":optional_hash(&data.join("threads.sqlite3-wal"))?,"shm":optional_hash(&data.join("threads.sqlite3-shm"))?}),
        )
    }
    // Bounded canonical facts, never record bodies. No Auth, prepare or Run here.
    fn fixture_facts(store: &Store, chat: &str, target: &str) -> Checked<Value> {
        if store
            .workspace_purpose()
            .map_err(|_| "purpose unreadable")?
            != brn_threads_core::WorkspacePurpose::Test
        {
            return Err("Test purpose required");
        }
        let records = store.records().map_err(|_| "records unreadable")?;
        if records.iter().any(|r| matches!(r.data, RecordData::Run(_)))
            || !store
                .candidates()
                .map_err(|_| "candidates unreadable")?
                .is_empty()
        {
            return Err("fresh unused fixture required");
        }
        let c = records
            .iter()
            .find(|r| r.id == chat && !r.archived)
            .ok_or("chat missing")?;
        let RecordData::Thread(t) = &c.data else {
            return Err("chat type");
        };
        if t.state != ThreadState::Open {
            return Err("chat inactive");
        }
        let p = store
            .project_context_record(chat)
            .map_err(|_| "project unreadable")?
            .ok_or("project required")?;
        if p.archived {
            return Err("project inactive");
        }
        let s = records
            .iter()
            .find(|r| !r.archived && matches!(r.data, RecordData::Settings(_)))
            .ok_or("settings missing")?;
        let RecordData::Settings(settings) = &s.data else {
            return Err("settings type");
        };
        if !matches!(settings.provider.as_str(), "Codex" | "Chatgpt")
            || settings.model != "gpt-6.1-sol"
            || settings.effort != "medium"
            || settings.credentials_dir.is_some()
            || settings.helper.is_some()
            || settings.mcp.is_some()
            || settings.skills.iter().any(|b| b.enabled || b.selected)
            || settings
                .auth_file
                .as_deref()
                .is_some_and(|s| s != AUTH_REFERENCE)
        {
            return Err("configuration outside control route");
        }
        let n = store
            .record(target)
            .map_err(|_| "target unreadable")?
            .ok_or("target missing")?;
        if n.archived
            || !matches!(&n.data,RecordData::Note(n) if n.protected)
            || !store
                .record_visible_in_chat(target, Some(chat))
                .map_err(|_| "target visibility unreadable")?
        {
            return Err("protected visible target required");
        }
        let receipts = store.receipts().map_err(|_| "history unreadable")?;
        let mut messages = chat_messages(&receipts, &records, chat);
        messages.sort_by_key(|r| &r.id);
        if messages.len() != 68
            || !messages.iter().any(|r|matches!(&r.data,RecordData::Message(m) if m.text==corpus()["early_requirement"]))
            || !messages.iter().any(|r|matches!(&r.data,RecordData::Message(m) if m.text==corpus()["later_proposal"]))
        {
            return Err("fixed canonical history required");
        }
        Ok(
            json!({"purpose":"Test","project":{"id":p.id,"version":p.version},"chat":{"id":c.id,"version":c.version,"privacy":t.private_context,"epoch":t.context_epoch},"settings":{"id":s.id,"version":s.version,"policy_sha256":digest(&serde_json::to_vec(&serde_json::to_value(settings).map_err(|_| "settings encoding")?).map_err(|_| "settings encoding")?),"auth_reference":settings.auth_file},"target":{"id":n.id,"version":n.version,"protected":true},"inputs":messages.iter().map(|r|json!({"id":r.id,"version":r.version})).collect::<Vec<_>>()}),
        )
    }
    #[derive(Clone, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Preflight {
        format: String,
        material_pin: String,
        source_inputs: BTreeMap<String, String>,
        observer_sha256: String,
        runner_sha256: String,
        executable: PathBuf,
        executable_sha256: String,
        dep_info_sha256: String,
        corpus_sha256: String,
        prompt_sha256: String,
        rubric_sha256: String,
        fixture: PathBuf,
        manifest_sha256: String,
        output: PathBuf,
        database: Value,
        context: Value,
        provider: String,
        model: String,
        effort: String,
        auth_reference: String,
        rounds: u16,
        deadline_seconds: u64,
        maximum_starts: u8,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Activation {
        format: String,
        task: String,
        principal_activated: bool,
        preflight_sha256: String,
        expires_unix: u64,
        maximum_starts: u8,
    }
    struct Ready {
        preflight: Preflight,
        prompt: String,
    }
    fn compiled_input_set() -> Checked<BTreeMap<String, String>> {
        let raw = include_bytes!(
            "../../../docs/work/active/agent-capabilities/int-d/diagnosis/capture/material-hashes.json"
        );
        let mut inputs: BTreeMap<String, String> =
            serde_json::from_slice(raw).map_err(|_| "compiled input set")?;
        inputs.insert(
            "docs/work/active/agent-capabilities/int-d/diagnosis/capture/material-hashes.json"
                .into(),
            digest(raw),
        );
        Ok(inputs)
    }
    fn source_checks(p: &Preflight, source: &Path) -> Checked<()> {
        let prior = compiled_input_set()?;
        if p.source_inputs.keys().ne(prior.keys())
            || p.material_pin.len() != 40
            || !p.material_pin.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("source input set or pin mismatch");
        }
        if p.observer_sha256 != digest(include_bytes!("threads.rs"))
            || p.runner_sha256 != digest(include_bytes!("threads_int_d_tests.rs"))
        {
            return Err("observer or runner differs from compiled bytes");
        }
        for (path, expected) in &p.source_inputs {
            if hash_file(&source.join(path), 64 * 1024 * 1024)? != *expected {
                return Err("working source mismatch");
            }
        }
        if p.source_inputs.get("crates/brn-ai/src/threads.rs") != Some(&p.observer_sha256)
            || p.source_inputs
                .get("crates/brn-ai/src/threads_int_d_tests.rs")
                != Some(&p.runner_sha256)
        {
            return Err("source map observer mismatch");
        }
        // Exact commit bytes, not an asserted SHA; no network/fetch and no stderr dump.
        let archive = std::process::Command::new("git")
            .current_dir(source)
            .args(["cat-file", "-t", &p.material_pin])
            .output()
            .map_err(|_| "source pin unavailable")?;
        if !archive.status.success() || archive.stdout != b"commit\n" {
            return Err("source pin unavailable");
        }
        let paths = std::process::Command::new("git")
            .current_dir(source)
            .args(["ls-tree", "-r", "--name-only", &p.material_pin, "--"])
            .args(p.source_inputs.keys())
            .output()
            .map_err(|_| "pinned paths unavailable")?;
        if !paths.status.success()
            || String::from_utf8(paths.stdout)
                .map_err(|_| "pinned paths invalid")?
                .lines()
                .ne(p.source_inputs.keys().map(String::as_str))
        {
            return Err("pin omits compiled input");
        }
        // git diff uses only the exact already checked compiled input paths.
        let status = std::process::Command::new("git")
            .current_dir(source)
            .args(["diff", "--quiet", &p.material_pin, "--"])
            .args(p.source_inputs.keys())
            .status()
            .map_err(|_| "source comparison unavailable")?;
        if !status.success() {
            return Err("source pin differs");
        }
        Ok(())
    }
    fn gate(activation: &Path, preflight: &Path) -> Checked<Ready> {
        gate_at(activation, preflight, &Path::new(ROOT).join("../.."))
    }
    fn gate_at(activation: &Path, preflight: &Path, source: &Path) -> Checked<Ready> {
        let folder = preflight.parent().ok_or("preflight root missing")?;
        owned(folder, "brn-int-d-capture-preflight-20261017-", true)?;
        if preflight.file_name() != Some(std::ffi::OsStr::new("preflight.json"))
            || activation != folder.join("activation.json")
        {
            return Err("explicit paired activation/preflight required");
        }
        let a: Activation = serde_json::from_slice(&bytes(activation, 8192)?)
            .map_err(|_| "activation malformed")?;
        let raw = bytes(preflight, 128 * 1024)?;
        if a.format != ACTIVATION_FORMAT
            || a.task != "int-d-capture-runner-20261017"
            || !a.principal_activated
            || a.maximum_starts != 1
            || a.preflight_sha256 != digest(&raw)
            || a.expires_unix
                <= SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| "clock unavailable")?
                    .as_secs()
        {
            return Err("activation absent stale or wrong window");
        }
        let p: Preflight = serde_json::from_slice(&raw).map_err(|_| "preflight malformed")?;
        if p.format != "BRN-INT-D-CAPTURE-PREFLIGHT-V1"
            || p.provider != "Codex/ChatGPT"
            || p.model != "gpt-6.1-sol"
            || p.effort != "medium"
            || p.auth_reference != AUTH_REFERENCE
            || p.rounds != 8
            || p.deadline_seconds != 120
            || p.maximum_starts != 1
        {
            return Err("route or budget mismatch");
        }
        owned(&p.fixture, "brn-int-d-qualification-20261017-", true)?;
        owned(&p.output, "brn-int-d-capture-output-20261017-", false)?;
        source_checks(&p, source)?;
        let exe = std::env::current_exe().map_err(|_| "executable unavailable")?;
        if exe != p.executable
            || hash_file(&exe, 1024 * 1024 * 1024)? != p.executable_sha256
            || hash_file(&exe.with_extension("d"), 8 * 1024 * 1024)? != p.dep_info_sha256
        {
            return Err("artifact mismatch");
        }
        if p.corpus_sha256
            != digest(include_bytes!(
                "../../../docs/work/active/agent-capabilities/int-d/corpus.json"
            ))
            || hash_file(&preflight.with_file_name("prompt.txt"), 128 * 1024)? != p.prompt_sha256
            || hash_file(&preflight.with_file_name("rubric.json"), 16 * 1024)? != p.rubric_sha256
            || hash_file(&p.fixture.join("manifest.json"), 64 * 1024)? != p.manifest_sha256
        {
            return Err("corpus prompt rubric or manifest mismatch");
        }
        let manifest: Value =
            serde_json::from_slice(&bytes(&p.fixture.join("manifest.json"), 64 * 1024)?)
                .map_err(|_| "manifest invalid")?;
        let control = manifest["cases"]
            .as_array()
            .and_then(|v| v.iter().find(|v| v["case"] == "control"))
            .ok_or("control missing")?;
        let data = p.fixture.join("control");
        if control["selected"] != false
            || control["data_dir"].as_str() != data.to_str()
            || data.canonicalize().map_err(|_| "data unavailable")? != data
        {
            return Err("only actual control allowed");
        }
        // Main/WAL/SHM captured BEFORE Store opening, including explicit absence.
        if db_stamp(&data)? != p.database {
            return Err("database or sidecar changed");
        }
        let store =
            Store::open_existing_without_upgrade(&data).map_err(|_| "fixture cannot reopen")?;
        let facts = fixture_facts(
            &store,
            control["chat"].as_str().ok_or("chat missing")?,
            control["note"].as_str().ok_or("target missing")?,
        )?;
        if facts != p.context
            || facts["settings"]["auth_reference"] != AUTH_REFERENCE
            || facts["target"]["version"] != control["note_version"]
        {
            return Err("canonical fixture or auth reference mismatch");
        }
        let prompt = String::from_utf8(bytes(&preflight.with_file_name("prompt.txt"), 128 * 1024)?)
            .map_err(|_| "prompt UTF8")?;
        if prompt.is_empty() {
            return Err("prompt empty");
        }
        Ok(Ready {
            preflight: p,
            prompt,
        })
    }
    // Kept separate so offline tests count only a fake Auth/execute closure.
    fn reserve_then<T>(
        ready: Ready,
        execute: impl FnOnce(Ready, File) -> Checked<T>,
    ) -> Checked<T> {
        fs::create_dir(&ready.preflight.output).map_err(|_| "output occupied or unavailable")?;
        let mut reservation = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(ready.preflight.output.join("one-start-reservation.json"))
            .map_err(|_| "start already reserved")?;
        reservation
            .write_all(b"{\"reserved_starts\":1,\"maximum\":1,\"consumed_even_on_failure\":true}\n")
            .and_then(|_| reservation.sync_all())
            .map_err(|_| "reservation not durable")?;
        File::open(&ready.preflight.output)
            .and_then(|d| d.sync_all())
            .map_err(|_| "directory not durable")?;
        execute(ready, reservation)
    }

    #[derive(Default)]
    struct Observation {
        frames: Vec<Value>,
        pending: Option<(u64, u64)>,
        stopped: bool,
        failure: Option<&'static str>,
    }
    fn context(scope: &Scope) -> Checked<Value> {
        let s = scope.store.lock().map_err(|_| "context lock failure")?;
        let r = s
            .record(&scope.run)
            .map_err(|_| "run unreadable")?
            .ok_or("run missing")?;
        let RecordData::Run(run) = r.data else {
            return Err("run type");
        };
        let c = s
            .record(&scope.chat)
            .map_err(|_| "chat unreadable")?
            .ok_or("chat missing")?;
        let RecordData::Thread(chat) = c.data else {
            return Err("chat type");
        };
        let p = s
            .project_context_record(&scope.chat)
            .map_err(|_| "project unreadable")?;
        Ok(
            json!({"run":scope.run,"state":run.state,"fence":run.fence,"settings":scope.settings,"chat":{"id":scope.chat,"privacy":chat.private_context,"epoch":chat.context_epoch},"project":p.map(|r|json!({"id":r.id,"version":r.version}))}),
        )
    }
    fn stop(scope: &Scope, cancel: &CancellationToken) -> Checked<()> {
        let result = (|| {
            let mut store = scope.store.lock().map_err(|_| "Stop lock failure")?;
            if live(scope, &store).is_err() {
                return Ok(());
            }
            let op = store
                .allocate_operation("int-d-manual-diagnostic-stop-once")
                .map_err(|_| "Stop allocation failure")?;
            match store
                .stop_and_post(
                    &op,
                    &scope.run,
                    &scope.chat,
                    "Synthetic diagnostic stopped; no Apply authorized",
                    &HostAuthority::owner("INT-D diagnostic checked Stop"),
                )
                .map_err(|_| "Stop refused")?
            {
                ApplyOutcome::Applied(_) => Ok(()),
                _ => Err("Stop not applied"),
            }
        })();
        cancel.cancel();
        result
    }
    fn permitted(name: &str) -> bool {
        matches!(
            name,
            "read_records"
                | "read_message"
                | "read_note"
                | "read_source"
                | "read_skill"
                | "read_history"
                | "search_notes"
                | "note_connections"
                | "retain_progress"
                | "prepare_changes"
        )
    }
    fn observer(
        scope: Arc<Scope>,
        cancel: CancellationToken,
        state: Arc<Mutex<Observation>>,
        file: Option<Arc<Mutex<File>>>,
    ) -> Arc<dyn Fn(AiEvent) + Send + Sync> {
        Arc::new(move |event| {
            let AiEvent::Activity(text) = event else {
                return;
            };
            let mut should_stop = false;
            let observed = (|| -> Checked<()> {
                if let Some(raw) = text.strip_prefix(CAPTURE_PREFIX) {
                    if text.len() > CAPTURE_BYTES {
                        return Err("capture frame overflow");
                    }
                    let row: Value =
                        serde_json::from_str(raw).map_err(|_| "capture JSON invalid")?;
                    let key = (
                        row["dispatch"].as_u64().ok_or("dispatch missing")?,
                        row["round"].as_u64().ok_or("round missing")?,
                    );
                    let mut state = state.lock().map_err(|_| "observer lock failure")?;
                    if state.stopped || state.frames.len() >= 2 {
                        return Err("capture event overflow");
                    }
                    if row["event"] == "dispatch" && state.pending.is_none() {
                        state.pending = Some(key);
                    } else if row["event"] == "outcome" && state.pending == Some(key) {
                        should_stop = true;
                        state.pending = None;
                    } else {
                        return Err("capture correlation missing");
                    }
                    // Snapshot taken BEFORE the outcome's host Stop, never log body.
                    let record = json!({"capture":row,"context":context(&scope)?});
                    let mut encoded =
                        serde_json::to_vec(&record).map_err(|_| "capture encoding failure")?;
                    if encoded.len() > 4096 {
                        return Err("context frame overflow");
                    }
                    encoded.push(b'\n');
                    if let Some(file) = &file {
                        let mut f = file.lock().map_err(|_| "file lock failure")?;
                        f.write_all(&encoded)
                            .and_then(|_| f.sync_all())
                            .map_err(|_| "capture filesystem failure")?;
                    }
                    state.frames.push(record);
                } else if let Some(dispatch) = text.strip_prefix("> ") {
                    let name = dispatch
                        .split_once(' ')
                        .map(|p| p.0)
                        .ok_or("unrecognized dispatch Activity")?;
                    if !permitted(name) {
                        should_stop = true;
                    }
                }
                // All other Activity and every Text/reasoning byte is dropped.
                Ok(())
            })();
            if observed.is_err() {
                should_stop = true;
            }
            if should_stop {
                let stopped = stop(&scope, &cancel);
                if let Ok(mut state) = state.lock() {
                    state.stopped = true;
                    state.failure = state.failure.or(observed.err()).or_else(|| stopped.err());
                }
            }
        })
    }

    fn prepared_matches(prepared: &PreparedThreadRun, p: &Preflight) -> Checked<()> {
        let scope = &prepared.scope;
        let store = scope
            .store
            .lock()
            .map_err(|_| "prepared store lock failure")?;
        live(scope, &store).map_err(|_| "prepared invocation stale")?;
        if scope
            .settings
            .as_ref()
            .map(|(id, v)| json!({"id":id,"version":v}))
            != Some(
                json!({"id":p.context["settings"]["id"],"version":p.context["settings"]["version"]}),
            )
            || scope
                .project_context
                .as_ref()
                .map(|(id, v)| json!({"id":id,"version":v}))
                != Some(p.context["project"].clone())
            || scope.helper.is_some()
            || scope
                .external
                .lock()
                .map_err(|_| "external lock failure")?
                .is_some()
            || scope.owner_request.is_some()
            || scope.writing_snapshot.is_some()
        {
            return Err("prepared policy or scope rebound");
        }
        let records = store
            .records()
            .map_err(|_| "prepared records unavailable")?;
        let chat = store
            .record(&scope.chat)
            .map_err(|_| "prepared chat unavailable")?
            .ok_or("prepared chat missing")?;
        let RecordData::Thread(t) = chat.data else {
            return Err("prepared chat type");
        };
        if json!(t.context_epoch) != p.context["chat"]["epoch"]
            || json!(t.private_context) != p.context["chat"]["privacy"]
        {
            return Err("prepared privacy or context changed");
        }
        let target = store
            .record(
                p.context["target"]["id"]
                    .as_str()
                    .ok_or("target ID missing")?,
            )
            .map_err(|_| "prepared target unavailable")?
            .ok_or("prepared target missing")?;
        if json!(target.version) != p.context["target"]["version"]
            || target.archived
            || !matches!(target.data,RecordData::Note(n) if n.protected)
        {
            return Err("prepared target changed");
        }
        let history = store
            .receipts()
            .map_err(|_| "prepared history unavailable")?;
        let mut inputs = chat_messages(&history, &records, &scope.chat)
            .into_iter()
            .filter(|r| r.id != scope.instruction)
            .map(|r| json!({"id":r.id,"version":r.version}))
            .collect::<Vec<_>>();
        inputs.sort_by_key(|r| r["id"].as_str().unwrap_or_default().to_owned());
        if json!(inputs) != p.context["inputs"] {
            return Err("prepared historical inputs changed");
        }
        Ok(())
    }

    #[tokio::test]
    #[ignore = "ONE future principal activation and pinned preflight required; NO permission from --ignored"]
    async fn manual_one_start_prepare_diagnostic() {
        // Missing/malformed paths fail before any Store, prepare_run or Auth.
        let activation = std::env::var_os("BRN_INT_D_CAPTURE_ACTIVATION")
            .map(PathBuf::from)
            .expect("explicit future activation required");
        let preflight = std::env::var_os("BRN_INT_D_CAPTURE_PREFLIGHT")
            .map(PathBuf::from)
            .expect("explicit prospective preflight required");
        let ready =
            gate(&activation, &preflight).expect("activation/preflight refused before Auth");
        let (ready, _reservation) = reserve_then(ready, |r, f| Ok((r, f)))
            .expect("one start reservation refused before Auth");
        let p = &ready.preflight;
        let frames = Arc::new(Mutex::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p.output.join("safe-events.jsonl"))
                .unwrap_or_else(|_| panic!("fresh event output required")),
        ));
        // Normal checked route, one durable prepare; no credential reader here.
        let prepared = prepare_run(ThreadRunRequest {
            request_message: None,
            queue_claim: None,
            owner_request: None,
            data_dir: p.fixture.join("control"),
            thread: p.context["chat"]["id"].as_str().unwrap().into(),
            prompt: ready.prompt,
            selection: Selection {
                provider: Provider::Chatgpt,
                model: p.model.clone(),
            },
            effort: ReasoningEffort::Medium,
            max_tool_rounds: 8,
            auth_file: Some(AUTH_REFERENCE.into()),
            credentials_dir: None,
            sources: vec![],
            search: None,
            trace: TraceLevel::Summary,
        })
        .unwrap_or_else(|_| panic!("reserved preparation refused; never retry"));
        if prepared_matches(&prepared, p).is_err() {
            let _ = stop(&prepared.scope, &CancellationToken::new());
            drop(prepared);
            panic!("reserved prepared snapshot refused before Auth; no retry");
        }
        let scope = prepared.scope.clone();
        let run = scope.run.clone();
        let data = p.fixture.join("control");
        let state = Arc::new(Mutex::new(Observation::default()));
        let cancel = CancellationToken::new();
        let emit = observer(
            scope.clone(),
            cancel.clone(),
            state.clone(),
            Some(frames.clone()),
        );
        let mut worker = tokio::spawn(run_prepared(prepared, cancel.clone(), emit));
        let (terminal, usage, summary_usage, model_turns, resends) =
            match tokio::time::timeout(Duration::from_secs(120), &mut worker).await {
                Ok(Ok(Ok(result))) => {
                    let kind = match result.answer.terminal {
                        AiTerminal::Completed => "completed",
                        AiTerminal::Interrupted => "interrupted",

                        AiTerminal::Failed(_) => "failed",
                    };
                    let (usage, summary, turns) = result
                        .trace
                        .map(|t| (t.usage, t.summary, Some(t.model_turns)))
                        .unwrap_or((None, None, None));
                    (
                        kind,
                        usage,
                        summary,
                        turns,
                        Some(result.provider_resends.len()),
                    )
                }
                Ok(_) => {
                    let _ = stop(&scope, &cancel);
                    ("worker_failure", None, None, None, None)
                }
                Err(_) => {
                    let _ = stop(&scope, &cancel);
                    if tokio::time::timeout(Duration::from_secs(5), &mut worker)
                        .await
                        .is_err()
                    {
                        worker.abort();
                        let _ = worker.await;
                    }
                    ("host_timeout", None, None, None, None)
                }
            };
        if let Err(failure) = stop(&scope, &cancel)
            && let Ok(mut s) = state.lock()
        {
            s.failure = s.failure.or(Some(failure));
        } // End/no prepare is a gap, never a second start.
        let final_context = context(&scope).expect("final facts unavailable; preserve reservation");
        let effects = {
            let store = scope
                .store
                .lock()
                .unwrap_or_else(|_| panic!("diagnostic lock unavailable"));
            let target = store
                .record(p.context["target"]["id"].as_str().unwrap())
                .unwrap_or_else(|_| panic!("final target unavailable"))
                .unwrap();
            let RecordData::Run(r) = store
                .record(&run)
                .unwrap_or_else(|_| panic!("final Run unavailable"))
                .unwrap()
                .data
            else {
                panic!("saved Run type")
            };
            let operations = r
                .operations
                .iter()
                .take(8)
                .map(|id| {
                    let info = store
                        .operation_info(id.as_str())
                        .unwrap_or_else(|_| panic!("final operation unavailable"));
                    let state = match info.state {
                        brn_threads_core::OperationState::Applied => "Applied",
                        brn_threads_core::OperationState::Prepared => "Prepared",
                        brn_threads_core::OperationState::Retired => "Retired",
                        brn_threads_core::OperationState::Allocated => "Allocated",
                    };
                    json!({"id":id,"state":state,"receipt":info.receipt.is_some()})
                })
                .collect::<Vec<_>>();
            json!({"target_version":target.version,"target_version_unchanged":json!(target.version)==p.context["target"]["version"],"operations":operations,"operations_total":r.operations.len(),"omitted":r.operations.len().saturating_sub(8)})
        };
        let observed = state
            .lock()
            .unwrap_or_else(|_| panic!("diagnostic lock unavailable"));
        let result = json!({"format":"INT-D-DIAGNOSTIC-RESULT-V1","run":run,"terminal":terminal,"artifact":{"material_pin":p.material_pin,"executable_sha256":p.executable_sha256,"dep_info_sha256":p.dep_info_sha256,"observer_sha256":p.observer_sha256,"runner_sha256":p.runner_sha256,"corpus_sha256":p.corpus_sha256,"prompt_sha256":p.prompt_sha256},"capture_frames":observed.frames.len(),"guard_failure":observed.failure,"stopped":observed.stopped,"context":final_context,"canonical_effects":effects,"parent_trace_usage":usage,"auth_http_count":"UNKNOWN","account_identity":"UNKNOWN","summary_overhead":summary_usage,"parent_model_turns":model_turns,"provider_resends":resends,"historical_cause":"UNKNOWN","semantic_benefit":"not assessed"});
        drop(observed);
        drop(scope);
        drop(frames);
        drop(state);
        let store = Store::open_existing_without_upgrade(&data)
            .unwrap_or_else(|_| panic!("post-run inspection unavailable"));
        assert!(
            !crate::run_lease::run_is_live(
                &store
                    .derived_directory()
                    .unwrap_or_else(|_| panic!("derived lease path unavailable")),
                &run
            ),
            "lease still held; no retry"
        );
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(p.output.join("result.json"))
            .unwrap_or_else(|_| panic!("result must be fresh"));
        out.write_all(
            &serde_json::to_vec_pretty(&result)
                .unwrap_or_else(|_| panic!("result encoding unavailable")),
        )
        .and_then(|_| out.sync_all())
        .unwrap_or_else(|_| panic!("result unavailable; preserve reservation"));
    }

    #[test]
    fn activation_gate_checks_real_files_context_and_reserves_before_fake_auth_once() {
        let files = tempfile::Builder::new()
            .prefix("brn-int-d-capture-preflight-20261017-guard-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let pair = tempfile::Builder::new()
            .prefix("brn-int-d-qualification-20261017-guard-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let source = tempfile::tempdir_in("/private/tmp").unwrap();
        let sources = compiled_input_set().unwrap();
        let actual = Path::new(ROOT).join("../..");
        let mut hashes = BTreeMap::new();
        for path in sources.keys() {
            let to = source.path().join(path);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(actual.join(path), &to).unwrap();
            hashes.insert(path.clone(), hash_file(&to, 64 * 1024 * 1024).unwrap());
        }
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Offline fixture",
                "-c",
                "user.email=fixture@invalid",
                "commit",
                "--quiet",
                "-m",
                "Synthetic compiled input snapshot",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .current_dir(source.path())
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let pin = String::from_utf8(
            std::process::Command::new("git")
                .current_dir(source.path())
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let control = pair.path().join("control");
        let mut store = Store::create(&control, brn_threads_core::WorkspacePurpose::Test).unwrap();
        let (_base, request, _) = tests::fixture();
        let src = Store::open(&request.data_dir).unwrap();
        let mut settings = src
            .records()
            .unwrap()
            .into_iter()
            .find_map(|r| {
                if let RecordData::Settings(s) = r.data {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap();
        settings.auth_file = Some(AUTH_REFERENCE.into());
        let op = store.allocate_operation("runner-guard-settings").unwrap();
        let request = ChangeRequest {
            reason: "Offline initial settings".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Settings(settings),
            }],
            inputs: vec![],
        };
        store
            .prepare_owner(
                &op,
                &request,
                &HostAuthority::owner("Offline settings fixture"),
            )
            .unwrap();
        assert!(matches!(
            store
                .apply(&op, &HostAuthority::owner("Offline settings fixture"))
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        fn command(
            store: &mut Store,
            key: &str,
            command: brn_threads_core::ProjectCommand,
        ) -> Receipt {
            let binding = digest(&serde_json::to_vec(&command).unwrap());
            let op = store.allocate_bound_operation(key, &binding).unwrap();
            store
                .project_command(
                    &op,
                    &command,
                    &HostAuthority::owner("Offline synthetic guard"),
                )
                .unwrap()
        }
        let project = command(
            &mut store,
            "runner-guard-project",
            brn_threads_core::ProjectCommand::Create {
                title: "Public synthetic Aurora".into(),
            },
        )
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id;
        let chat = command(
            &mut store,
            "runner-guard-chat",
            brn_threads_core::ProjectCommand::CreateChat {
                project: project.clone(),
                title: "Public discussion".into(),
            },
        )
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
        let note = command(
            &mut store,
            "runner-guard-note",
            brn_threads_core::ProjectCommand::CreateDocument {
                project,
                title: "Public draft".into(),
                markdown: corpus()["original_note"].as_str().unwrap().into(),
            },
        )
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after;
        let RecordData::Note(mut n) = note.data else {
            panic!()
        };
        n.protected = true;
        save(
            &mut store,
            vec![Put {
                id: note.id.clone(),
                expected_version: Some(note.version),
                archived: false,
                data: RecordData::Note(n),
            }],
        );
        history(&mut store, &chat, &corpus());
        let context = fixture_facts(&store, &chat, &note.id).unwrap();
        let version = context["target"]["version"].as_u64().unwrap();
        drop(store);
        let manifest = json!({"cases":[{"case":"control","selected":false,"data_dir":control,"chat":chat,"note":note.id,"note_version":version}]});
        fs::write(pair.path().join("manifest.json"), manifest.to_string()).unwrap();
        fs::write(
            files.path().join("prompt.txt"),
            "Fixed public synthetic request\n",
        )
        .unwrap();
        fs::write(
            files.path().join("rubric.json"),
            "{\"capture\":\"PASS/FAIL/UNKNOWN\"}",
        )
        .unwrap();
        let output = PathBuf::from(format!(
            "/private/tmp/brn-int-d-capture-output-20261017-guard-{}",
            uuid_like_key()
        ));
        let exe = std::env::current_exe().unwrap();
        let mut p = Preflight {
            format: "BRN-INT-D-CAPTURE-PREFLIGHT-V1".into(),
            material_pin: pin,
            source_inputs: hashes,
            observer_sha256: digest(include_bytes!("threads.rs")),
            runner_sha256: digest(include_bytes!("threads_int_d_tests.rs")),
            executable: exe.clone(),
            executable_sha256: hash_file(&exe, 1024 * 1024 * 1024).unwrap(),
            dep_info_sha256: hash_file(&exe.with_extension("d"), 8 * 1024 * 1024).unwrap(),
            corpus_sha256: digest(include_bytes!(
                "../../../docs/work/active/agent-capabilities/int-d/corpus.json"
            )),
            prompt_sha256: hash_file(&files.path().join("prompt.txt"), 128 * 1024).unwrap(),
            rubric_sha256: hash_file(&files.path().join("rubric.json"), 16 * 1024).unwrap(),
            fixture: pair.path().into(),
            manifest_sha256: hash_file(&pair.path().join("manifest.json"), 64 * 1024).unwrap(),
            output: output.clone(),
            database: db_stamp(&control).unwrap(),
            context,
            provider: "Codex/ChatGPT".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_reference: AUTH_REFERENCE.into(),
            rounds: 8,
            deadline_seconds: 120,
            maximum_starts: 1,
        };
        let pf = files.path().join("preflight.json");
        let activation = files.path().join("activation.json");
        let write_gate = |p: &Preflight| {
            let b = serde_json::to_vec(p).unwrap();
            fs::write(&pf, &b).unwrap();
            let a = Activation {
                format: ACTIVATION_FORMAT.into(),
                task: "int-d-capture-runner-20261017".into(),
                principal_activated: true,
                preflight_sha256: digest(&b),
                expires_unix: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
                maximum_starts: 1,
            };
            fs::write(&activation, serde_json::to_vec(&a).unwrap()).unwrap();
        };
        let calls = AtomicUsize::new(0);
        let attempt = || -> Checked<()> {
            let ready = gate_at(&activation, &pf, source.path())?;
            reserve_then(ready, |ready, _file| {
                assert!(
                    ready
                        .preflight
                        .output
                        .join("one-start-reservation.json")
                        .is_file()
                );
                calls.fetch_add(1, Ordering::SeqCst);
                Err("fake Auth refused; consumed")
            })
        };
        assert!(attempt().is_err()); // both files missing
        write_gate(&p);
        fs::write(&activation, "[]").unwrap();
        assert!(attempt().is_err());
        write_gate(&p);
        let mut a: Value = serde_json::from_slice(&fs::read(&activation).unwrap()).unwrap();
        a["format"] = json!("original-two-start-window");
        fs::write(&activation, a.to_string()).unwrap();
        assert!(attempt().is_err());
        for field in [
            "task",
            "principal_activated",
            "preflight_sha256",
            "expires_unix",
            "maximum_starts",
        ] {
            write_gate(&p);
            let mut a: Value = serde_json::from_slice(&fs::read(&activation).unwrap()).unwrap();
            a[field] = match field {
                "principal_activated" => json!(false),
                "expires_unix" => json!(0),
                "maximum_starts" => json!(2),
                _ => json!("wrong"),
            };
            fs::write(&activation, a.to_string()).unwrap();
            assert!(attempt().is_err(), "{field}");
        }
        for field in [
            "material_pin",
            "source_inputs",
            "observer_sha256",
            "runner_sha256",
            "executable_sha256",
            "dep_info_sha256",
            "corpus_sha256",
            "prompt_sha256",
            "rubric_sha256",
            "manifest_sha256",
            "provider",
            "model",
            "effort",
            "auth_reference",
            "rounds",
            "deadline_seconds",
            "maximum_starts",
            "database",
            "context",
        ] {
            let mut bad = serde_json::to_value(&p).unwrap();
            if field == "source_inputs" {
                bad[field]["Cargo.lock"] = json!("wrong");
            } else if matches!(field, "rounds" | "deadline_seconds" | "maximum_starts") {
                bad[field] = json!(2);
            } else if matches!(field, "database" | "context") {
                bad[field] = json!({"changed":true});
            } else {
                bad[field] = json!("wrong");
            }
            let bad: Preflight = serde_json::from_value(bad).unwrap();
            write_gate(&bad);
            assert!(attempt().is_err(), "{field}");
        }
        for sidecar in ["threads.sqlite3-wal", "threads.sqlite3-shm"] {
            write_gate(&p);
            let path = control.join(sidecar);
            let old = fs::read(&path).ok();
            fs::write(&path, b"changed sidecar").unwrap();
            assert!(attempt().is_err());
            if let Some(old) = old {
                fs::write(&path, old).unwrap();
            } else {
                fs::remove_file(&path).unwrap();
            }
        }
        write_gate(&p);
        let db = control.join("threads.sqlite3");
        let original = fs::read(&db).unwrap();
        let mut changed = original.clone();
        changed[100] ^= 1;
        fs::write(&db, &changed).unwrap();
        assert!(attempt().is_err());
        fs::write(&db, original).unwrap();
        // Canonical purpose/type/version mismatch are checked independently of file hashes.
        let mut altered = Store::open_existing_without_upgrade(&control).unwrap();
        altered
            .set_workspace_purpose(brn_threads_core::WorkspacePurpose::Owner)
            .unwrap();
        drop(altered);
        let mut bad = p.clone();
        bad.database = db_stamp(&control).unwrap();
        write_gate(&bad);
        assert!(attempt().is_err());
        let mut altered = Store::open_existing_without_upgrade(&control).unwrap();
        altered
            .set_workspace_purpose(brn_threads_core::WorkspacePurpose::Test)
            .unwrap();
        drop(altered);
        p.database = db_stamp(&control).unwrap();
        for key in ["target", "chat", "project", "settings"] {
            let mut bad = p.clone();
            bad.context[key]["version"] = json!(9999);
            write_gate(&bad);
            assert!(attempt().is_err());
        }
        write_gate(&p);
        fs::create_dir(&output).unwrap();
        assert!(attempt().is_err());
        fs::remove_dir(&output).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        // First complete fake gate is actual files/Store/Git validation, not Auth.
        // It may create sidecars while reading; pre-call stamp was already checked.
        let ready = gate_at(&activation, &pf, source.path()).unwrap();
        assert!(
            reserve_then(ready, |ready, _| -> Checked<()> {
                assert!(
                    ready
                        .preflight
                        .output
                        .join("one-start-reservation.json")
                        .is_file()
                );
                calls.fetch_add(1, Ordering::SeqCst);
                let p = &ready.preflight;
                let prepared = prepare_run(ThreadRunRequest {
                    request_message: None,
                    queue_claim: None,
                    owner_request: None,
                    data_dir: p.fixture.join("control"),
                    thread: p.context["chat"]["id"].as_str().unwrap().into(),
                    prompt: ready.prompt,
                    selection: Selection {
                        provider: Provider::Chatgpt,
                        model: p.model.clone(),
                    },
                    effort: ReasoningEffort::Medium,
                    max_tool_rounds: 8,
                    auth_file: Some(AUTH_REFERENCE.into()),
                    credentials_dir: None,
                    sources: vec![],
                    search: None,
                    trace: TraceLevel::Summary,
                })
                .unwrap();
                prepared_matches(&prepared, p).unwrap();
                let mut changed = p.clone();
                changed.context["chat"]["epoch"] = json!(9999);
                assert!(prepared_matches(&prepared, &changed).is_err());
                let mut changed = p.clone();
                changed.context["settings"]["version"] = json!(9999);
                assert!(prepared_matches(&prepared, &changed).is_err());
                stop(&prepared.scope, &CancellationToken::new()).unwrap();
                drop(prepared);
                Err("fake Auth refusal")
            })
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(attempt().is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Direct duplicate reservation also cannot reach the fake closure.
        assert!(
            reserve_then(
                Ready {
                    preflight: p,
                    prompt: "public".into()
                },
                |_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        fs::remove_dir_all(output).unwrap();
    }
    fn uuid_like_key() -> String {
        unique_host_key()
            .bytes()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(char::from)
            .collect()
    }

    #[tokio::test]
    async fn runner_observer_allows_reported_progress_but_fences_all_proposal_effects() {
        use rig::test_utils::{MockCompletionModel, MockStreamEvent};
        let mut names = tool_names();
        names.extend(search_tool_names());
        for name in names {
            assert!(
                permitted(name)
                    || matches!(
                        name,
                        "apply_changes" | "withdraw_proposal" | "refresh_candidate"
                    ),
                "runtime catalogue changed: {name}"
            );
            assert!(tool_activity(name).is_some());
        }
        for mode in [
            "prepare-success",
            "prepare-failure",
            "apply_changes",
            "withdraw_proposal",
            "refresh_candidate",
            "filesystem-failure",
            "missing-correlation",
            "event-overflow",
        ] {
            let (_root, request, note) = tests::fixture();
            let mut store = Store::open(&request.data_dir).unwrap();
            let old = store.record(&note).unwrap().unwrap();
            let RecordData::Note(mut n) = old.data else {
                panic!()
            };
            n.protected = true;
            save(
                &mut store,
                vec![Put {
                    id: note.clone(),
                    expected_version: Some(old.version),
                    archived: false,
                    data: RecordData::Note(n),
                }],
            );
            let before = store.record(&note).unwrap().unwrap();
            drop(store);
            let (scope, input) = begin(&request).unwrap();
            tests::read_all(&scope);
            let RecordData::Note(mut revised) = before.data.clone() else {
                panic!()
            };
            revised.markdown = "RUNNER_PROPOSED_BODY_CANARY".into();
            let change = ChangeRequest {
                reason: "RUNNER_REASON_CANARY".into(),
                writes: vec![Put {
                    id: note.clone(),
                    expected_version: Some(before.version),
                    archived: false,
                    data: RecordData::Note(revised),
                }],
                inputs: vec![RevisionInput {
                    record: note.clone(),
                    version: before.version,
                    expectation: InputExpectation::Current,
                }],
            };
            let mut args = json!({"request_key":"runner-proposal","request":change});
            let op = scope
                .store
                .lock()
                .unwrap()
                .allocate_operation(&format!("{}:proposal:runner-proposal", scope.run))
                .unwrap();
            if mode == "prepare-failure" {
                args["request"]["inputs"][0]["record"] = json!(op.creation_id(99));
                assert!(parse_valid("prepare_changes", &args));
            }
            if matches!(
                mode,
                "apply_changes" | "withdraw_proposal" | "refresh_candidate"
            ) {
                prepare(&scope, serde_json::from_value(args.clone()).unwrap()).unwrap();
            }
            let effects_before = scope.store.lock().unwrap().receipts().unwrap();
            let effect = match mode {
                "apply_changes" | "withdraw_proposal" => {
                    MockStreamEvent::tool_call("effect", mode, json!({"operation":op.as_str()}))
                }
                "refresh_candidate" => MockStreamEvent::tool_call(
                    "effect",
                    mode,
                    json!({"operation":op.as_str(),"request_key":"replacement","request":change}),
                ),
                _ => MockStreamEvent::tool_call("prepare", "prepare_changes", args),
            };
            let model = MockCompletionModel::from_stream_turns([vec![
                MockStreamEvent::tool_call(
                    "progress",
                    "retain_progress",
                    json!({"summary":"RUNNER_REPORTED_PLAN_CANARY"}),
                ),
                effect,
                MockStreamEvent::tool_call(
                    "apply",
                    "apply_changes",
                    json!({"operation":op.as_str()}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ]]);
            let state = Arc::new(Mutex::new(Observation::default()));
            let cancel = CancellationToken::new();
            let file = if mode == "filesystem-failure" {
                Some(Arc::new(Mutex::new(File::open("/dev/null").unwrap())))
            } else {
                None
            };
            let emit = observer(scope.clone(), cancel.clone(), state.clone(), file);
            if mode == "missing-correlation" {
                emit(AiEvent::Activity(format!(
                    "{CAPTURE_PREFIX}{}",
                    json!({"event":"outcome","dispatch":99,"round":1,"status":"error","native_kind":"other","source_kind":"UNKNOWN"})
                )));
            }
            if mode == "event-overflow" {
                let frame = format!(
                    "{CAPTURE_PREFIX}{}",
                    json!({"event":"dispatch","dispatch":99,"round":1})
                );
                emit(AiEvent::Activity(frame.clone()));
                emit(AiEvent::Activity(frame));
            }
            let answer = run_model(
                model.clone(),
                scope.clone(),
                &input,
                ReasoningEffort::Medium,
                4,
                true,
                cancel.clone(),
                emit,
            )
            .await;
            assert!(cancel.is_cancelled());
            assert!(!finish(&scope, &answer).unwrap());
            let state = state.lock().unwrap();
            assert!(state.stopped);
            if mode.starts_with("prepare-") {
                assert_eq!(state.frames.len(), 2);
                assert_eq!(state.frames[0]["context"]["state"], "Working");
                assert_eq!(state.frames[1]["context"]["state"], "Working");
                assert_eq!(
                    state.frames[0]["capture"]["dispatch"],
                    state.frames[1]["capture"]["dispatch"]
                );
                assert_eq!(
                    state.frames[1]["capture"]["status"],
                    if mode == "prepare-success" {
                        "success"
                    } else {
                        "error"
                    }
                );
                assert_eq!(
                    state.frames[1]["capture"]["source_kind"],
                    if mode == "prepare-success" {
                        "NONE"
                    } else {
                        "tool_rejected"
                    }
                );
            }
            let store = scope.store.lock().unwrap();
            assert_eq!(store.record(&note).unwrap().unwrap(), before);
            assert!(store.receipt(&op).unwrap().is_none());
            if matches!(
                mode,
                "apply_changes" | "withdraw_proposal" | "refresh_candidate"
            ) {
                assert_eq!(store.prepared(&op).unwrap().operation, op);
                assert!(
                    store.operation_info(op.as_str()).unwrap().state
                        == brn_threads_core::OperationState::Prepared
                );
            }
            let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
                panic!()
            };
            assert_eq!(run.state, RunState::Interrupted);
            if !matches!(mode, "missing-correlation" | "event-overflow") {
                assert!(run.progress.contains("RUNNER_REPORTED_PLAN_CANARY"));
            }
            assert_eq!(
                store
                    .receipts()
                    .unwrap()
                    .iter()
                    .filter(|r| r.writes.iter().any(|w| w.after.id == note))
                    .count(),
                effects_before
                    .iter()
                    .filter(|r| r.writes.iter().any(|w| w.after.id == note))
                    .count()
            );
            for frame in &state.frames {
                assert!(
                    !frame.to_string().contains("RUNNER_PROPOSED_BODY_CANARY")
                        && !frame.to_string().contains("RUNNER_REASON_CANARY")
                );
            }
            assert!(model.request_count() <= 1);
            let run_id = scope.run.clone();
            let derived = store.derived_directory().unwrap();
            drop(store);
            drop(state);
            drop(scope);
            assert!(!crate::run_lease::run_is_live(&derived, &run_id));
        }
    }

    #[test]
    fn offline_python_context_interoperates_with_exact_rust_fixture_facts() {
        // Actual feature/typed-field order, not an assumed Python canonical form.
        let (_root, request, _) = tests::fixture();
        let store = Store::open_existing_without_upgrade(&request.data_dir).unwrap();
        let settings = store
            .records()
            .unwrap()
            .into_iter()
            .find_map(|r| {
                if let RecordData::Settings(s) = r.data {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap();
        let encoded = serde_json::to_value(settings).unwrap();
        assert_eq!(
            encoded
                .as_object()
                .unwrap()
                .keys()
                .take(8)
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "provider",
                "model",
                "effort",
                "auth_file",
                "credentials_dir",
                "maintenance",
                "review_first",
                "delegation_chosen"
            ]
        );
        assert_eq!(
            serde_json::from_str::<Value>(r#"{"z":1,"a":2}"#)
                .unwrap()
                .to_string(),
            r#"{"z":1,"a":2}"#
        );
        drop(store);
        let Some(copy) = std::env::var_os("BRN_INT_D_OFFLINE_CONTEXT_COPY").map(PathBuf::from)
        else {
            assert!(std::env::var_os("BRN_INT_D_OFFLINE_EXPECTED_CONTEXT").is_none());
            assert!(std::env::var_os("BRN_INT_D_OFFLINE_CONTEXT_OUTPUT").is_none());
            println!(
                "INT_D_B1_INTEROP: preserve_order qualified; external copied-fixture branch not requested"
            );
            return;
        };
        owned(&copy, "brn-int-d-runner-interop-20261017-", true).unwrap();
        let expected_path = PathBuf::from(
            std::env::var_os("BRN_INT_D_OFFLINE_EXPECTED_CONTEXT")
                .expect("explicit expected metadata required"),
        );
        let output = PathBuf::from(
            std::env::var_os("BRN_INT_D_OFFLINE_CONTEXT_OUTPUT")
                .expect("explicit fresh metadata output required"),
        );
        assert_eq!(expected_path, copy.join("expected-context.json"));
        assert_eq!(output, copy.join("actual-context.json"));
        let expected: Value =
            serde_json::from_slice(&bytes(&expected_path, 32 * 1024).unwrap()).unwrap();
        let data = copy.join("control");
        assert_eq!(data.canonicalize().unwrap(), data);
        let store = Store::open_existing_without_upgrade(&data).unwrap();
        let actual = fixture_facts(
            &store,
            expected["chat"]["id"].as_str().unwrap(),
            expected["target"]["id"].as_str().unwrap(),
        )
        .unwrap();
        drop(store);
        let raw = serde_json::to_vec_pretty(&actual).unwrap();
        assert!(raw.len() <= 32 * 1024);
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&output)
            .unwrap();
        out.write_all(&raw).and_then(|_| out.sync_all()).unwrap();
        let mut except_policy = expected.clone();
        except_policy["settings"]["policy_sha256"] = actual["settings"]["policy_sha256"].clone();
        assert_eq!(actual, except_policy, "difference outside B1 policy digest");
        println!(
            "INT_D_B1_INTEROP {}",
            json!({"whole_context_equal":actual==expected,"rust_policy_sha256":actual["settings"]["policy_sha256"],"expected_policy_sha256":expected["settings"]["policy_sha256"],"only_possible_difference":"settings.policy_sha256","auth_or_runner_invocations":0})
        );
        assert_eq!(
            actual, expected,
            "B1 Python/Rust whole-context interoperability"
        );
    }
}
