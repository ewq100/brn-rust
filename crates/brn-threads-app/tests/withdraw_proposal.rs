//! Slice G: the agent withdraws its own obsolete proposal with
//! `withdraw_proposal`. Withdrawal is the owner's shared dismissal path under
//! the Run's fenced authority: it retires the candidate and clears its Review
//! item, and never applies anything. Owner changes, other chats' proposals and
//! applied operations are refused. Scripted models drive the real runtime
//! tools over synthetic data; no provider is contacted.
use brn_ai::threads::{ThreadRunRequest, TraceLevel, prepare_run, run_prepared_scripted};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::run_attention::REVIEW_ATTENTION;
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const PLAN: &str = "# Delivery\n\nSynthetic: delivery is planned for 22 November.\n";
const PROPOSED: &str = "# Delivery\n\nSynthetic: delivery moves to 15 November.\n";
const WITHDRAWN: &str = "Agent withdrew its obsolete proposal; preserve other attention requests";
const DISMISSED: &str = "Dismissed proposal; preserve other attention requests";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    workspace: Workspace,
    chat: String,
    note: String,
}

/// Offline settings (no sign-in file or credentials) and the owner's
/// delegation choice, one chat and one saved note.
fn world(choice: DelegationChoice) -> World {
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
    workspace.choose_delegation(choice).unwrap();
    let chat = workspace.new_thread("Delivery").unwrap().id;
    let note = workspace.new_note("Delivery", PLAN).unwrap().id;
    World {
        _root: root,
        data,
        workspace,
        chat,
        note,
    }
}

fn request(data: &Path, chat: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: "Keep the delivery plan current.".into(),
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
        trace: TraceLevel::Summary,
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

fn change(note: &str, version: u64, markdown: &str) -> Value {
    json!({
        "reason":"Record the requested delivery date",
        "writes":[{"id":note,"expected_version":version,"archived":false,
            "data":{"Note":{"title":"Delivery","markdown":markdown,"protected":false,
                "confirmed":false,"superseded_by":null,"import":null}}}],
        "inputs":[{"record":note,"version":version}]
    })
}

/// A Run in `chat` reads `note`, prepares `markdown` for it and applies it.
/// Returns the operation and the apply outcome the model saw.
async fn propose(w: &World, chat: &str, note: &str, markdown: &str) -> (String, String) {
    let prepared = prepare_run(request(&w.data, chat)).unwrap();
    // Allocation by host key is idempotent, so this is the Run's operation.
    let operation = Store::open(&w.data)
        .unwrap()
        .allocate_operation(&format!("{}:proposal:delivery", prepared.run_id()))
        .unwrap()
        .as_str()
        .to_owned();
    let version = w.workspace.store.record(note).unwrap().unwrap().version;
    let model = MockCompletionModel::from_stream_turns([
        call("r", "read_records", json!({"ids":[note],"query":null})),
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"delivery","request":change(note, version, markdown)}),
        ),
        call("a", "apply_changes", json!({"operation":operation})),
        finish("Proposed the new date."),
    ]);
    let result = run_prepared_scripted(prepared, model).await.unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let trace = result.trace.unwrap();
    assert_eq!(trace.calls[2].tool, "apply_changes");
    (operation, trace.calls[2].outcome.clone())
}

/// A later Run in `chat` calls `withdraw_proposal` for `operation`. Returns
/// the call's outcome label and the tool result the model saw.
async fn withdraw(w: &World, chat: &str, operation: &str) -> (String, String) {
    let prepared = prepare_run(request(&w.data, chat)).unwrap();
    let model = MockCompletionModel::from_stream_turns([
        call("w", "withdraw_proposal", json!({"operation":operation})),
        finish("Handled the obsolete proposal."),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let trace = result.trace.unwrap();
    assert_eq!(trace.calls[0].tool, "withdraw_proposal");
    let seen = format!("{:?}", model.requests()[1].chat_history.last());
    (trace.calls[0].outcome.clone(), seen)
}

fn attention(workspace: &Workspace, chat: &str) -> Vec<Attention> {
    match workspace.store.record(chat).unwrap().unwrap().data {
        RecordData::Thread(thread) => thread.attention,
        _ => panic!("Thread"),
    }
}

fn review(operation: &str) -> Attention {
    Attention {
        kind: AttentionKind::Review,
        reason: REVIEW_ATTENTION.into(),
        record: Some(operation.into()),
    }
}

fn listed(workspace: &Workspace, operation: &str) -> bool {
    workspace
        .candidates()
        .unwrap()
        .iter()
        .any(|c| c.operation.as_str() == operation)
}

/// Prepared, without a receipt and no longer listed: retired.
fn retired(workspace: &Workspace, operation: &str) -> bool {
    let id = workspace.store.operation(operation).unwrap();
    workspace.store.prepared(&id).is_ok()
        && workspace.store.receipt(&id).unwrap().is_none()
        && workspace.store.active_candidates(&[id]).unwrap().is_empty()
}

fn receipts_with(workspace: &Workspace, reason: &str) -> Vec<Receipt> {
    workspace
        .history()
        .unwrap()
        .into_iter()
        .filter(|r| r.reason == reason)
        .collect()
}

fn note(workspace: &Workspace, id: &str) -> (u64, String) {
    let record = workspace.store.record(id).unwrap().unwrap();
    match record.data {
        RecordData::Note(note) => (record.version, note.markdown),
        _ => panic!("Note"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_agent_withdraws_its_own_obsolete_proposal_after_the_owner_decides() {
    let mut w = world(DelegationChoice::ReviewFirst);
    let (chat, target) = (w.chat.clone(), w.note.clone());
    let (op, applied) = propose(&w, &chat, &target, PROPOSED).await;
    assert_eq!(applied, "NeedsReview");
    assert_eq!(attention(&w.workspace, &chat), [review(&op)]);
    // The owner decides against it, and a later Run withdraws it.
    w.workspace
        .post_message(&chat, "Keep 22 November.")
        .unwrap();
    let (outcome, seen) = withdraw(&w, &chat, &op).await;
    assert_eq!(outcome, "Withdrawn");
    assert!(seen.contains(r#""outcome": String("Withdrawn")"#), "{seen}");
    assert!(!listed(&w.workspace, &op));
    assert!(retired(&w.workspace, &op));
    assert!(
        attention(&w.workspace, &chat)
            .iter()
            .all(|a| a.record.as_deref() != Some(op.as_str()))
    );
    // One checked cleanup change, which writes only this chat's Thread.
    let cleanups = receipts_with(&w.workspace, WITHDRAWN);
    assert_eq!(cleanups.len(), 1);
    let written: Vec<_> = cleanups[0].writes.iter().map(|w| &w.after.id).collect();
    assert_eq!(written, [&chat]);
    // Nothing was applied, and the owner cannot apply it any more either.
    let id = w.workspace.store.operation(&op).unwrap();
    assert!(w.workspace.store.receipt(&id).unwrap().is_none());
    assert!(w.workspace.review_apply(&op).is_err());
    assert_eq!(note(&w.workspace, &target), (1, PLAN.into()));
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_another_chats_proposal_is_refused() {
    let mut w = world(DelegationChoice::ReviewFirst);
    let other = w.workspace.new_thread("Other delivery").unwrap().id;
    let target = w.note.clone();
    let (op, applied) = propose(&w, &other, &target, PROPOSED).await;
    assert_eq!(applied, "NeedsReview");
    let before = attention(&w.workspace, &other);
    assert_eq!(before, [review(&op)]);
    let (outcome, seen) = withdraw(&w, &w.chat.clone(), &op).await;
    assert_eq!(outcome, "error");
    assert!(!seen.contains("Withdrawn"), "{seen}");
    assert!(listed(&w.workspace, &op));
    assert_eq!(attention(&w.workspace, &other), before);
    assert!(receipts_with(&w.workspace, WITHDRAWN).is_empty());
    assert_eq!(note(&w.workspace, &target), (1, PLAN.into()));
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_an_owner_created_candidate_is_refused() {
    let mut w = world(DelegationChoice::ReviewFirst);
    let (chat, target) = (w.chat.clone(), w.note.clone());
    let owner = HostAuthority::owner("synthetic owner draft");
    let op = w.workspace.store.allocate_operation("owner-draft").unwrap();
    w.workspace
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Owner draft".into(),
                writes: vec![Put {
                    id: target.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(Note::working("Delivery", PROPOSED)),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let thread = w.workspace.store.record(&chat).unwrap().unwrap();
    let RecordData::Thread(mut data) = thread.data.clone() else {
        panic!("Thread")
    };
    data.attention.push(review(op.as_str()));
    w.workspace
        .owner_change(
            "owner-draft-review",
            &ChangeRequest {
                reason: "Owner review item".into(),
                writes: vec![Put {
                    id: chat.clone(),
                    expected_version: Some(thread.version),
                    archived: false,
                    data: RecordData::Thread(data),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    assert!(listed(&w.workspace, op.as_str()));
    let (outcome, _) = withdraw(&w, &chat, op.as_str()).await;
    assert_eq!(outcome, "error");
    assert!(listed(&w.workspace, op.as_str()));
    assert_eq!(attention(&w.workspace, &chat), [review(op.as_str())]);
    assert!(receipts_with(&w.workspace, WITHDRAWN).is_empty());
    assert_eq!(note(&w.workspace, &target), (1, PLAN.into()));
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_an_applied_proposal_is_refused() {
    let w = world(DelegationChoice::MaintainOrdinaryNotes);
    let (chat, target) = (w.chat.clone(), w.note.clone());
    let (op, applied) = propose(&w, &chat, &target, PROPOSED).await;
    assert_eq!(applied, "Applied");
    let id = w.workspace.store.operation(&op).unwrap();
    let receipt = w.workspace.store.receipt(&id).unwrap().unwrap();
    let (outcome, seen) = withdraw(&w, &chat, &op).await;
    assert_eq!(outcome, "Refused");
    assert!(seen.contains("This proposal was applied"), "{seen}");
    assert_eq!(w.workspace.store.receipt(&id).unwrap().unwrap(), receipt);
    assert!(receipts_with(&w.workspace, WITHDRAWN).is_empty());
    assert_eq!(note(&w.workspace, &target), (2, PROPOSED.into()));
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_twice_replays_without_another_receipt() {
    let w = world(DelegationChoice::ReviewFirst);
    let (chat, target) = (w.chat.clone(), w.note.clone());
    let (op, _) = propose(&w, &chat, &target, PROPOSED).await;
    let (first, _) = withdraw(&w, &chat, &op).await;
    assert_eq!(first, "Withdrawn");
    let cleanups = receipts_with(&w.workspace, WITHDRAWN);
    assert_eq!(cleanups.len(), 1);
    let (second, seen) = withdraw(&w, &chat, &op).await;
    assert_eq!(second, "Withdrawn");
    assert!(seen.contains(r#""replay": Bool(true)"#), "{seen}");
    assert_eq!(receipts_with(&w.workspace, WITHDRAWN), cleanups);
    assert!(retired(&w.workspace, &op));
    assert!(attention(&w.workspace, &chat).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn undo_after_a_withdrawal_matches_undo_after_dismiss() {
    let mut w = world(DelegationChoice::ReviewFirst);
    let (chat, target) = (w.chat.clone(), w.note.clone());
    let other = w.workspace.new_thread("Other delivery").unwrap().id;
    let second = w.workspace.new_note("Crane", PLAN).unwrap().id;
    let (withdrawn, _) = propose(&w, &chat, &target, PROPOSED).await;
    let (dismissed, _) = propose(&w, &other, &second, PROPOSED).await;
    w.workspace.review_dismiss(&dismissed).unwrap();
    let (outcome, _) = withdraw(&w, &chat, &withdrawn).await;
    assert_eq!(outcome, "Withdrawn");
    let [withdrawal] = receipts_with(&w.workspace, WITHDRAWN).try_into().unwrap();
    let [dismissal] = receipts_with(&w.workspace, DISMISSED).try_into().unwrap();
    let undo_withdrawal = w.workspace.undo(withdrawal.operation.as_str()).unwrap();
    let undo_dismissal = w.workspace.undo(dismissal.operation.as_str()).unwrap();
    assert!(matches!(undo_withdrawal, ApplyOutcome::Applied(_)));
    assert_eq!(
        std::mem::discriminant(&undo_withdrawal),
        std::mem::discriminant(&undo_dismissal)
    );
    // Each Review item comes back; each candidate stays retired, so the
    // refusal to apply it stays a refusal.
    assert_eq!(attention(&w.workspace, &chat), [review(&withdrawn)]);
    assert_eq!(attention(&w.workspace, &other), [review(&dismissed)]);
    for op in [&withdrawn, &dismissed] {
        assert!(retired(&w.workspace, op));
        assert!(w.workspace.review_apply(op).is_err());
    }
    assert_eq!(note(&w.workspace, &target), (1, PLAN.into()));
    assert_eq!(note(&w.workspace, &second), (1, PLAN.into()));
}
