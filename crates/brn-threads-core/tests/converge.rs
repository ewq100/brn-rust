//! Owner decision B4: converging chat and project commands report
//! `Unchanged` for a no-op and record nothing; the existing functions keep
//! appending receipts.
use brn_threads_core::*;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn store() -> (TempDir, Store) {
    let dir = TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let mut store = Store::open(dir.path().join("data")).unwrap();
    chat(&mut store, "bootstrap", ChatCommand::Bootstrap).unwrap();
    (dir, store)
}
fn owner() -> HostAuthority {
    HostAuthority::owner("owner chat action")
}
fn op(store: &mut Store, key: &str, command: &impl serde::Serialize) -> OperationId {
    let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(command).unwrap()));
    store.allocate_bound_operation(key, &binding).unwrap()
}
fn chat(store: &mut Store, key: &str, command: ChatCommand) -> Result<ChatCommandOutcome> {
    let op = op(store, key, &command);
    store.chat_command(&op, &command, &owner())
}
fn chat_converging(
    store: &mut Store,
    key: &str,
    command: ChatCommand,
) -> Result<Converged<ChatCommandOutcome>> {
    let op = op(store, key, &command);
    store.chat_command_converging(&op, &command, &owner())
}
fn project(store: &mut Store, key: &str, command: ProjectCommand) -> Result<Receipt> {
    let op = op(store, key, &command);
    store.project_command(&op, &command, &owner())
}
fn project_converging(
    store: &mut Store,
    key: &str,
    command: ProjectCommand,
) -> Result<Converged<Receipt>> {
    let op = op(store, key, &command);
    store.project_command_converging(&op, &command, &owner())
}
fn created(store: &mut Store, key: &str, data: RecordData) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let request = ChangeRequest {
        reason: "synthetic".into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data,
        }],
        inputs: vec![],
    };
    store
        .prepare_owner(&op, &request, &HostAuthority::owner("owner"))
        .unwrap();
    match store.apply(&op, &HostAuthority::owner("owner")).unwrap() {
        ApplyOutcome::Applied(receipt) => receipt.writes[0].after.clone(),
        outcome => panic!("{outcome:?}"),
    }
}
fn thread(store: &mut Store, key: &str) -> String {
    created(
        store,
        key,
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: key.into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    )
    .id
}
fn receipts(store: &Store) -> usize {
    store.receipts().unwrap().len()
}
fn version(store: &Store, id: &str) -> u64 {
    store.record(id).unwrap().unwrap().version
}
fn unchanged<T>(outcome: Converged<T>) -> Unchanged {
    match outcome {
        Converged::Unchanged(unchanged) => unchanged,
        Converged::Changed(_) => panic!("expected Unchanged"),
    }
}

#[test]
fn every_converging_variant_is_unchanged_on_a_no_op() {
    let (_dir, mut store) = store();
    let chat_id = thread(&mut store, "chat");
    // Unarchive of an open chat.
    let before = (receipts(&store), version(&store, &chat_id));
    let u = unchanged(
        chat_converging(
            &mut store,
            "u1",
            ChatCommand::Unarchive {
                thread: chat_id.clone(),
            },
        )
        .unwrap(),
    );
    assert_eq!((u.record.as_str(), u.version), (chat_id.as_str(), before.1));
    assert_eq!((receipts(&store), version(&store, &chat_id)), before);
    // Archive twice: a change, then a no-op.
    let first = chat_converging(
        &mut store,
        "a1",
        ChatCommand::Archive {
            thread: chat_id.clone(),
        },
    )
    .unwrap();
    assert!(matches!(first, Converged::Changed(_)));
    let before = (receipts(&store), version(&store, &chat_id));
    let u = unchanged(
        chat_converging(
            &mut store,
            "a2",
            ChatCommand::Archive {
                thread: chat_id.clone(),
            },
        )
        .unwrap(),
    );
    assert_eq!(u.record, chat_id);
    assert_eq!((receipts(&store), version(&store, &chat_id)), before);
    // Projects: link twice, unlink twice, move a chat into its project twice.
    let receipt = project(
        &mut store,
        "p",
        ProjectCommand::Create { title: "P".into() },
    )
    .unwrap();
    let project_id = receipt.writes[0].after.id.clone();
    let note = created(
        &mut store,
        "note",
        RecordData::Note(Note::working("Doc", "text")),
    )
    .id;
    let link = || ProjectCommand::LinkDocument {
        project: project_id.clone(),
        note: note.clone(),
    };
    assert!(matches!(
        project_converging(&mut store, "l1", link()).unwrap(),
        Converged::Changed(_)
    ));
    let before = (receipts(&store), version(&store, &project_id));
    let u = unchanged(project_converging(&mut store, "l2", link()).unwrap());
    assert_eq!(
        (u.record.as_str(), u.version),
        (project_id.as_str(), before.1)
    );
    assert_eq!((receipts(&store), version(&store, &project_id)), before);
    let unlink = || ProjectCommand::UnlinkDocument {
        project: project_id.clone(),
        note: note.clone(),
    };
    assert!(matches!(
        project_converging(&mut store, "x1", unlink()).unwrap(),
        Converged::Changed(_)
    ));
    let before = receipts(&store);
    assert_eq!(
        unchanged(project_converging(&mut store, "x2", unlink()).unwrap()).record,
        project_id
    );
    assert_eq!(receipts(&store), before);
    let other = thread(&mut store, "other");
    let move_in = || ProjectCommand::MoveChat {
        chat: other.clone(),
        project: Some(project_id.clone()),
    };
    assert!(matches!(
        project_converging(&mut store, "m1", move_in()).unwrap(),
        Converged::Changed(_)
    ));
    let before = (receipts(&store), version(&store, &other));
    let u = unchanged(project_converging(&mut store, "m2", move_in()).unwrap());
    assert_eq!((u.record.as_str(), u.version), (other.as_str(), before.1));
    assert_eq!((receipts(&store), version(&store, &other)), before);
    // The allocated operation stays allocated.
    let allocated = store.operation_by_host_key("m2").unwrap().unwrap();
    assert_eq!(
        store.operation_info(allocated.as_str()).unwrap().state,
        OperationState::Allocated
    );
}

#[test]
fn the_existing_functions_still_append_on_a_no_op() {
    let (_dir, mut store) = store();
    let chat_id = thread(&mut store, "chat");
    chat(
        &mut store,
        "a1",
        ChatCommand::Archive {
            thread: chat_id.clone(),
        },
    )
    .unwrap();
    let before = (receipts(&store), version(&store, &chat_id));
    chat(
        &mut store,
        "a2",
        ChatCommand::Archive {
            thread: chat_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(receipts(&store), before.0 + 1);
    assert_eq!(version(&store, &chat_id), before.1 + 1);
    let receipt = project(
        &mut store,
        "p",
        ProjectCommand::Create { title: "P".into() },
    )
    .unwrap();
    let project_id = receipt.writes[0].after.id.clone();
    let note = created(
        &mut store,
        "note",
        RecordData::Note(Note::working("Doc", "t")),
    )
    .id;
    let link = || ProjectCommand::LinkDocument {
        project: project_id.clone(),
        note: note.clone(),
    };
    project(&mut store, "l1", link()).unwrap();
    let before = receipts(&store);
    project(&mut store, "l2", link()).unwrap();
    assert_eq!(receipts(&store), before + 1);
}

#[test]
fn a_working_run_makes_archive_a_change_with_a_fence() {
    let (_dir, mut store) = store();
    let chat_id = thread(&mut store, "chat");
    chat(
        &mut store,
        "a1",
        ChatCommand::Archive {
            thread: chat_id.clone(),
        },
    )
    .unwrap();
    let run = created(
        &mut store,
        "run",
        RecordData::Run(Run {
            loaded_skill_details: vec![],
            instruction: None,
            queue_claim: None,
            thread: chat_id.clone(),
            provider: "synthetic".into(),
            model: "synthetic".into(),
            guide_identity: "v1".into(),
            budget: 3,
            effort: "low".into(),
            loaded_skills: vec![],
            fence: 1,
            state: RunState::Working,
            progress: String::new(),
            operations: vec![],
            sources: vec![],
        }),
    );
    let outcome = chat_converging(
        &mut store,
        "a2",
        ChatCommand::Archive {
            thread: chat_id.clone(),
        },
    )
    .unwrap();
    let Converged::Changed(outcome) = outcome else {
        panic!("a Working run must be fenced");
    };
    assert!(outcome.receipt.writes.iter().any(|w| w.after.id == run.id));
    let RecordData::Run(stopped) = store.record(&run.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!((stopped.state, stopped.fence), (RunState::Interrupted, 2));
}

#[test]
fn a_binding_mismatch_is_immutable_request() {
    let (_dir, mut store) = store();
    let chat_id = thread(&mut store, "chat");
    let archive = ChatCommand::Archive {
        thread: chat_id.clone(),
    };
    let op = op(&mut store, "k1", &archive);
    let unarchive = ChatCommand::Unarchive { thread: chat_id };
    assert!(matches!(
        store.chat_command_converging(&op, &unarchive, &owner()),
        Err(Error::ImmutableRequest)
    ));
}
