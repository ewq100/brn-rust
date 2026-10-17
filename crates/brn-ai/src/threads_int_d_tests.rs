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
        let mut store = Store::open(&request.data_dir).unwrap();
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
