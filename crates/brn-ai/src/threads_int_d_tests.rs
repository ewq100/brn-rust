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
            if mode == "object" {
                4
            } else if mode == "stopped" {
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
        assert_ne!(trace.stopped.as_deref(), Some("invalid_batch"));
        let calls = trace
            .calls
            .iter()
            .filter(|c| c.tool == "prepare_changes")
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), if mode == "object" { 2 } else { 1 });
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
            let native =
                rig::tool::ErasedTool::execute(&PrepareChanges, args.to_string(), &mut cx).await;
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
                    serde_json::to_string(&model.requests()[2])
                        .unwrap()
                        .contains("the tool failed")
                );
            }
        }
        let activity = events.lock().unwrap();
        let dispatch = activity
            .iter()
            .filter_map(|s| s.strip_prefix("> prepare_changes "))
            .collect::<Vec<_>>();
        assert_eq!(dispatch.len(), calls.len());
        let shown: Value = serde_json::from_str(dispatch[0]).unwrap();
        let classification = match shown.get("writing") {
            None => "absent",
            Some(Value::Null) => "null",
            Some(Value::Object(_)) => "object",
            _ => panic!(),
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
        if mode == "object" {
            assert!(dispatch[1].ends_with(" … [arguments truncated]"));
            assert!(serde_json::from_str::<Value>(dispatch[1]).is_err());
            assert!(!dispatch[1].contains("\"writing\""));
            assert!(
                [&note, &messages[0], &messages[1]]
                    .iter()
                    .any(|id| !dispatch[1].contains(id.as_str()))
            );
        }
        let long_id_visibility = (mode == "object")
            .then(|| [&note, &messages[0], &messages[1]].map(|id| dispatch[1].contains(id)));
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
            "long_activity_writing":if mode=="object" {"not observable: truncated"} else {"not exercised"},
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
        assert!(did_stop.load(Ordering::SeqCst));
        assert_eq!(model.request_count(), 1);
        assert!(!finish(&scope, &answer).unwrap());
        let rows = captured.lock().unwrap();
        if mode == "apply-first" {
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
        assert_eq!(run.state, RunState::Interrupted);
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
        facts.push(json!({"mode":mode,"mock_requests":model.request_count(),"capture":*rows,"guarded_interrupted_fence":run.fence,"note_and_note_receipts_unchanged":true}));
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
