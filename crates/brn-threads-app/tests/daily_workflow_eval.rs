//! Daily-workflow journey (docs/work/active/daily-workflow-reliability): a
//! project, a document, an email, a protected-decision contradiction, a drafted
//! reply, a restart and a re-supply. A scripted model drives the real runtime
//! tools over fresh synthetic data; no provider is contacted. It asserts what
//! the host offers, enforces and persists, not model quality.
use brn_ai::threads::{PreparedThreadRun, ThreadRunRequest, prepare_run, run_prepared_scripted};
use brn_ai::{Provider, ReasoningEffort, Selection};
use brn_threads_app::search::SearchHandle;
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const EMAIL: &str = "From: Mari Example <mari@example.invalid>\nSubject: Crane delivery\n\nThe berth survey finished early: measured depth is 1.4 m.\nCan we move crane delivery to 15 November?\n";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    workspace: Workspace,
    handle: SearchHandle,
    project: String,
    chat: String,
    decision: String,
    survey: String,
}

fn created(receipt: &Receipt, kind: &str) -> String {
    receipt
        .writes
        .iter()
        .find(|w| {
            w.before.is_none()
                && serde_json::to_value(&w.after.data)
                    .unwrap()
                    .get(kind)
                    .is_some()
        })
        .map(|w| w.after.id.clone())
        .expect(kind)
}

fn world() -> World {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace = Workspace::open(&data).unwrap();
    let settings = workspace.settings().unwrap();
    workspace
        .configure(WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            ..settings
        })
        .unwrap();
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let receipt = workspace
        .project_command(
            "eval-project",
            ProjectCommand::Create {
                title: "Harbor crane".into(),
            },
        )
        .unwrap();
    let project = created(&receipt, "Project");
    let version = workspace.store.record(&project).unwrap().unwrap().version;
    workspace
        .project_command(
            "eval-project-settings",
            ProjectCommand::Configure {
                project: project.clone(),
                version,
                title: "Harbor crane".into(),
                instructions:
                    "Cite the source for every date. Never promise delivery dates; ask Eva first."
                        .into(),
                use_brn_knowledge: true,
            },
        )
        .unwrap();
    let chat = created(
        &workspace
            .project_command(
                "eval-chat",
                ProjectCommand::CreateChat {
                    project: project.clone(),
                    title: "Crane delivery".into(),
                },
            )
            .unwrap(),
        "Thread",
    );
    workspace
        .project_command(
            "eval-doc",
            ProjectCommand::CreateDocument {
                project: project.clone(),
                title: "Harbor crane plan".into(),
                markdown: "# Harbor crane plan\n\nThe Nordcrane unit needs a dredged berth of at least 1.2 m.\nCrane delivery and assembly take 5 working days.\n".into(),
            },
        )
        .unwrap();
    let survey = workspace
        .new_note(
            "Berth survey",
            "# Berth survey\n\nThe crane berth survey is scheduled for October.\n",
        )
        .unwrap()
        .id;
    let mut decision = Note::working(
        "Crane delivery decision",
        "# Crane delivery decision\n\nDecision: crane delivery is 22 November.\n",
    );
    decision.protected = true;
    let op = workspace.store.allocate_operation("eval-decision").unwrap();
    let owner = HostAuthority::owner("synthetic protected decision");
    workspace
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Owner recorded a protected decision".into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(decision),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let receipt = applied(workspace.store.apply(&op, &owner).unwrap());
    let decision = receipt.writes[0].after.id.clone();
    let handle = workspace.search_handle().unwrap();
    handle.prepare().wait().unwrap();
    World {
        _root: root,
        data,
        workspace,
        handle,
        project,
        chat,
        decision,
        survey,
    }
}

fn applied(outcome: ApplyOutcome) -> Receipt {
    match outcome {
        ApplyOutcome::Applied(receipt) => receipt,
        other => panic!("not applied: {other:?}"),
    }
}

fn email() -> brn_ai::threads::TransientSource {
    brn_ai::threads::TransientSource {
        reference: "synthetic://mari-crane-email".into(),
        text: EMAIL.into(),
        fingerprint: Some("mari-crane-email-v1".into()),
        report: Some(ImportReport {
            status: ImportStatus::Unchecked,
            issues: vec![],
        }),
        assets: vec![],
    }
}

fn request(
    w: &World,
    chat: &str,
    prompt: &str,
    sources: Vec<brn_ai::threads::TransientSource>,
) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: w.data.clone(),
        thread: chat.to_owned(),
        prompt: prompt.into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources,
        search: Some(Arc::new(w.handle.clone())),
        trace: brn_ai::threads::TraceLevel::Summary,
    }
}

fn call(id: &str, name: &str, args: Value) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::tool_call(id, name, args),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn finish(text: &str) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn operation(data: &Path, prepared: &PreparedThreadRun, key: &str) -> String {
    Store::open(data)
        .unwrap()
        .allocate_operation(&format!("{}:proposal:{key}", prepared.run_id()))
        .unwrap()
        .as_str()
        .to_owned()
}

/// Each model request's newest message: the prompt, then each tool result.
fn seen(model: &MockCompletionModel) -> Vec<String> {
    model
        .requests()
        .iter()
        .map(|request| format!("{:?}", request.chat_history.last()))
        .collect()
}

fn first_prompt(model: &MockCompletionModel) -> String {
    format!("{:?}", model.requests()[0].chat_history)
}

/// Outcome of the `apply_changes` tool call shown in a model request.
fn apply_outcome(seen: &[String], index: usize) -> &'static str {
    let shown = seen.get(index).map(String::as_str).unwrap_or_default();
    let after: String = shown
        .split("outcome")
        .nth(1)
        .unwrap_or_default()
        .chars()
        .take(40)
        .collect();
    ["Applied", "NeedsReview", "Deferred", "Stale", "Superseded"]
        .into_iter()
        .find(|outcome| after.contains(outcome))
        .unwrap_or("missing")
}

fn sources(workspace: &Workspace) -> Vec<Record> {
    workspace
        .records()
        .unwrap()
        .into_iter()
        .filter(|r| matches!(r.data, RecordData::Source(_)))
        .collect()
}

fn note_text(workspace: &Workspace, id: &str) -> String {
    match workspace.store.record(id).unwrap().unwrap().data {
        RecordData::Note(note) => note.markdown,
        _ => panic!("not a note"),
    }
}

fn action(workspace: &Workspace) -> Record {
    workspace
        .records()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.data, RecordData::Action(_)))
        .expect("Action")
}

fn action_state(record: &Record) -> ActionState {
    match &record.data {
        RecordData::Action(action) => action.state.clone(),
        _ => panic!("not an Action"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn daily_workflow_with_contradiction_reply_restart_and_resupply() {
    let mut w = world();
    // Steps 2-4: the email arrives in the project chat. The scripted model
    // searches, reads the source, tries to change the protected decision and
    // records the new information in the ordinary survey note plus an Action.
    let intake = "Retain useful information from Mari's email for this project.";
    let prepared = prepare_run(request(&w, &w.chat, intake, vec![email()])).unwrap();
    let protected_op = operation(&w.data, &prepared, "decision");
    let survey_op = operation(&w.data, &prepared, "survey");
    let survey_update = "# Berth survey\n\nThe crane berth survey is scheduled for October.\n\nMari reported the survey finished early with a measured depth of 1.4 m (source: Mari's crane email).\n";
    let model = MockCompletionModel::from_stream_turns([
        call(
            "s",
            "search_notes",
            json!({"query":"crane delivery berth","limit":8,"mode":"hybrid","cursor":null}),
        ),
        call(
            "r",
            "read_source",
            json!({"reference":"synthetic://mari-crane-email"}),
        ),
        call(
            "p1",
            "prepare_changes",
            json!({"request_key":"decision","request":{"reason":"Email asks for 15 November","writes":[{"id":w.decision,"expected_version":1,"archived":false,"data":{"Note":{"title":"Crane delivery decision","markdown":"# Crane delivery decision\n\nDecision: crane delivery is 15 November.\n","protected":true,"confirmed":false,"superseded_by":null,"import":null}}}],"inputs":[{"record":w.decision,"version":1}]}}),
        ),
        call("a1", "apply_changes", json!({"operation":protected_op})),
        call(
            "p2",
            "prepare_changes",
            json!({"request_key":"survey","request":{"reason":"Record the measured berth depth from Mari's email","writes":[
                {"id":w.survey,"expected_version":1,"archived":false,"data":{"Note":{"title":"Berth survey","markdown":survey_update,"protected":false,"confirmed":false,"superseded_by":null,"import":null}}},
                {"id":"new:0","expected_version":null,"archived":false,"data":{"Action":{"description":"Answer Mari about the requested 15 November crane delivery","state":"Suggested","internal":false,"evidence":null,"due":null}}}
            ],"inputs":[{"record":w.survey,"version":1}]}}),
        ),
        call("a2", "apply_changes", json!({"operation":survey_op})),
        finish(
            "Mari asks for 15 November, but the protected decision says 22 November. Keep 22 November or accept 15 November?",
        ),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    let seen1 = seen(&model);
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Completed
    ));
    // The project's instructions and both scopes of knowledge reach the run.
    assert!(first_prompt(&model).contains("Never promise delivery dates"));
    for title in [
        "Harbor crane plan",
        "Crane delivery decision",
        "Berth survey",
    ] {
        assert!(seen1[1].contains(title), "search result lacks {title}");
    }
    // The protected decision is never changed by the model, and a
    // private-context run cannot write general notes on its own.
    assert_eq!(apply_outcome(&seen1, 4), "NeedsReview");
    assert_eq!(apply_outcome(&seen1, 6), "NeedsReview");
    assert!(note_text(&w.workspace, &w.decision).contains("22 November"));
    assert_eq!(w.workspace.candidates().unwrap().len(), 2);

    // The owner reviews and applies the ordinary survey update and Action.
    assert!(matches!(
        w.workspace.review_apply(&survey_op).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(note_text(&w.workspace, &w.survey).contains("1.4 m"));
    let suggested = action(&w.workspace);
    assert_eq!(action_state(&suggested), ActionState::Suggested);

    // Step 5: the owner resolves; the model drafts a reply and tries to close the Action.
    let answer = "Keep 22 November and draft a reply to Mari explaining it.";
    let prepared = prepare_run(request(&w, &w.chat, answer, vec![])).unwrap();
    let done_op = operation(&w.data, &prepared, "done");
    let reply = "Draft reply to Mari (not sent):\n\nHi Mari, thanks for the early survey (1.4 m measured). We are keeping crane delivery on 22 November as decided; 15 November is not possible. Eva";
    let model = MockCompletionModel::from_stream_turns([
        call(
            "s",
            "search_notes",
            json!({"query":"crane delivery decision","limit":8,"mode":"hybrid","cursor":null}),
        ),
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"done","request":{"reason":"Reply drafted","writes":[{"id":suggested.id,"expected_version":suggested.version,"archived":false,"data":{"Action":{"description":"Answer Mari about the requested 15 November crane delivery","state":"Done","internal":false,"evidence":"Reply drafted in chat","due":null}}}],"inputs":[]}}),
        ),
        call("a", "apply_changes", json!({"operation":done_op})),
        finish(reply),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Completed
    ));
    // A drafted reply is not a delivered one: the human Action stays open.
    assert_eq!(apply_outcome(&seen(&model), 3), "NeedsReview");
    assert_eq!(
        action_state(&w.workspace.store.record(&suggested.id).unwrap().unwrap()),
        ActionState::Suggested
    );
    assert!(
        w.workspace
            .needs_you()
            .unwrap()
            .iter()
            .any(|chat| chat.id == w.chat)
    );

    // Step 6: reopen BRN from the same directory and find the result.
    let data = w.data.clone();
    drop(w.handle);
    drop(w.workspace);
    let mut reopened = Workspace::open(&data).unwrap();
    let messages: Vec<_> = reopened
        .messages(&w.chat)
        .unwrap()
        .into_iter()
        .filter_map(|m| match m.data {
            RecordData::Message(m) => Some((m.role, m.text)),
            _ => None,
        })
        .collect();
    let roles: Vec<_> = messages.iter().map(|(role, _)| role.as_str()).collect();
    assert_eq!(roles, ["owner", "assistant", "owner", "assistant"]);
    assert_eq!(messages[3].1, reply);
    assert!(reopened.threads().unwrap().iter().any(|t| t.id == w.chat));

    // Step 7: the same email again, in the same chat.
    let handle = reopened.search_handle().unwrap();
    handle.prepare().wait().unwrap();
    let w2 = World {
        _root: w._root,
        data: data.clone(),
        workspace: reopened,
        handle,
        project: w.project,
        chat: w.chat.clone(),
        decision: w.decision,
        survey: w.survey,
    };
    let before = sources(&w2.workspace);
    assert_eq!(before.len(), 1);
    let prepared = prepare_run(request(&w2, &w2.chat, intake, vec![email()])).unwrap();
    let model = MockCompletionModel::from_stream_turns([finish("Already recorded.")]);
    run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    // The run sees the earlier observation and its result; no second Source.
    let prompt = first_prompt(&model);
    assert!(prompt.contains("mari-crane-email-v1"));
    assert!(prompt.contains("measured depth of 1.4 m"));
    let after = sources(&w2.workspace);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].id, before[0].id);
    assert_eq!(after[0].version, before[0].version + 1);
}
