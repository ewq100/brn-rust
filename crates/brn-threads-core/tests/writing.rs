use brn_threads_core::*;
fn fixture() -> (
    tempfile::TempDir,
    Store,
    WritingSnapshot,
    OperationId,
    String,
) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(dir.path().join("data")).unwrap();
    let fixture = store.allocate_operation("fixture").unwrap();
    let proposal = store.allocate_operation("proposal").unwrap();
    let replacement = store.allocate_operation("replacement").unwrap();
    let note = fixture.creation_id(0);
    let thread = fixture.creation_id(1);
    let run = fixture.creation_id(2);
    let request = ChangeRequest {
        reason: "Synthetic fixture".into(),
        writes: vec![
            Put {
                id: note.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working("Draft", "Original")),
            },
            Put {
                id: thread.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    title: "Write".into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            },
            Put {
                id: run.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Run(Run {
                    thread: thread.clone(),
                    provider: "Codex".into(),
                    model: "fixture".into(),
                    guide_identity: "fixture".into(),
                    budget: 4,
                    effort: "medium".into(),
                    loaded_skills: vec![],
                    fence: 1,
                    state: RunState::Working,
                    progress: "".into(),
                    operations: vec![proposal.clone(), replacement],
                    sources: vec![],
                }),
            },
            Put {
                id: fixture.creation_id(3),
                expected_version: None,
                archived: false,
                data: RecordData::Settings(WorkspaceSettings {
                    provider: "Codex".into(),
                    model: "fixture".into(),
                    effort: "medium".into(),
                    auth_file: None,
                    credentials_dir: None,
                    maintenance: true,
                    review_first: false,
                    delegation_chosen: true,
                    backup_directory: None,
                }),
            },
        ],
        inputs: vec![],
    };
    let owner = HostAuthority::owner("fixture");
    store.prepare_owner(&fixture, &request, &owner).unwrap();
    assert!(matches!(
        store.apply(&fixture, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(store.record(&fixture.creation_id(3)).unwrap().unwrap().data, RecordData::Settings(settings) if settings.delegation_chosen && settings.maintenance && !settings.review_first)
    );
    let op = store.allocate_operation("snapshot").unwrap();
    let snapshot = WritingSnapshot {
        request_message: op.creation_id(0),
        thread: thread.clone(),
        note: store.record(&note).unwrap().unwrap(),
        comments: vec![],
        messages: vec![],
    };
    let request = ChangeRequest {
        reason: "Snapshot".into(),
        writes: vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread,
                role: WRITING_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
        inputs: vec![],
    };
    store.prepare_owner(&op, &request, &owner).unwrap();
    store.apply(&op, &owner).unwrap();
    (dir, store, snapshot, proposal, run)
}
#[test]
fn maintenance_and_content_instruction_cannot_bypass_durable_review_and_replay() {
    let (_dir, mut store, snapshot, proposal, _run) = fixture();
    let request = snapshot.change_request("Proposed entire note").unwrap();
    store.prepare(&proposal, &request).unwrap();
    let instruction = HostAuthority::instruction(
        "owner instruction",
        "message",
        proposal.clone(),
        [snapshot.note.id.clone()],
        [Capability::EditContent],
    );
    for authority in [HostAuthority::maintenance("Continue"), instruction] {
        assert!(matches!(
            store.apply(&proposal, &authority).unwrap(),
            ApplyOutcome::NeedsReview { .. }
        ));
    }
    let owner = HostAuthority::owner("Exact candidate reviewed");
    let receipt = store.apply(&proposal, &owner).unwrap();
    assert!(matches!(receipt, ApplyOutcome::Applied(_)));
    assert_eq!(
        store
            .apply(&proposal, &HostAuthority::maintenance("Retry"))
            .unwrap(),
        receipt
    );
}
#[test]
fn runtime_and_owner_replacement_cannot_remove_or_downgrade_writing_marker() {
    let (_dir, mut store, snapshot, proposal, run) = fixture();
    let request = snapshot.change_request("Proposal").unwrap();
    store.prepare(&proposal, &request).unwrap();
    let replacement = store.allocate_operation("replacement").unwrap();
    let runtime = HostAuthority::runtime("host", &run).for_run(&run, 1);
    let mut stripped = request.clone();
    stripped
        .inputs
        .retain(|input| input.record != snapshot.request_message);
    assert!(
        store
            .refresh_candidate(&proposal, &replacement, &stripped, &runtime)
            .is_err()
    );
    assert!(
        store
            .replace_candidate(
                &proposal,
                &replacement,
                &stripped,
                &HostAuthority::owner("review edit")
            )
            .is_err()
    );
    let mut downgraded = request.clone();
    downgraded
        .inputs
        .iter_mut()
        .find(|input| input.record == snapshot.request_message)
        .unwrap()
        .expectation = InputExpectation::Historical;
    assert!(
        store
            .refresh_candidate(&proposal, &replacement, &downgraded, &runtime)
            .is_err()
    );
    store
        .refresh_candidate(&proposal, &replacement, &request, &runtime)
        .unwrap();
    assert!(matches!(
        store
            .apply(&replacement, &HostAuthority::maintenance("Continue"))
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
}
#[test]
fn owner_review_preserves_dirty_guards_and_exact_versions() {
    let (_dir, mut store, snapshot, proposal, _run) = fixture();
    let request = snapshot.change_request("Proposal").unwrap();
    store.prepare(&proposal, &request).unwrap();
    let edit = store
        .begin_edit(&snapshot.note.id, snapshot.note.version)
        .unwrap();
    assert!(matches!(
        store
            .apply(&proposal, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Deferred { .. }
    ));
    store.discard(&edit.id, edit.generation).unwrap();
    let owner_op = store.allocate_operation("later owner edit").unwrap();
    let mut changed = request.clone();
    changed.inputs.clear();
    if let RecordData::Note(note) = &mut changed.writes[0].data {
        note.markdown = "Later owner draft".into();
    }
    store
        .prepare_owner(&owner_op, &changed, &HostAuthority::owner("owner"))
        .unwrap();
    store
        .apply(&owner_op, &HostAuthority::owner("owner"))
        .unwrap();
    assert!(matches!(
        store
            .apply(&proposal, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Stale { .. }
    ));
}

#[test]
fn historical_request_role_keeps_gate_when_current_message_is_modified() {
    let (_dir, mut store, snapshot, proposal, _run) = fixture();
    store
        .prepare(
            &proposal,
            &snapshot.change_request("Proposed note").unwrap(),
        )
        .unwrap();
    let record = store.record(&snapshot.request_message).unwrap().unwrap();
    let modified = store
        .allocate_operation("modified request message")
        .unwrap();
    store
        .prepare_owner(
            &modified,
            &ChangeRequest {
                reason: "Owner changed request metadata".into(),
                writes: vec![Put {
                    id: record.id,
                    expected_version: Some(record.version),
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: snapshot.thread.clone(),
                        role: "owner".into(),
                        text: "Changed text".into(),
                    }),
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    store
        .apply(&modified, &HostAuthority::owner("owner"))
        .unwrap();
    assert!(matches!(
        store
            .apply(&proposal, &HostAuthority::maintenance("Continue"))
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(matches!(
        store
            .apply(&proposal, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Stale { .. }
    ));
}
