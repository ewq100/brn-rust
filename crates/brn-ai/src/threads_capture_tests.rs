//! Capture bindings through ordinary runtime services; no provider execution.
use super::*;
fn capture_fixture() -> (tempfile::TempDir, ThreadRunRequest, CaptureSnapshot) {
    let (root, mut request, _) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let owner = HostAuthority::owner("synthetic posted conversation");
    let op = store.allocate_operation("capture-fixture-message").unwrap();
    let message = Put {
        id: op.creation_id(0),
        expected_version: None,
        archived: false,
        data: RecordData::Message(Message {
            thread: request.thread.clone(),
            role: "owner".into(),
            text: "We selected blue. A follow-up may be useful, but nobody has agreed to own it."
                .into(),
        }),
    };
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Fixture".into(),
                inputs: vec![],
                writes: vec![message],
            },
            &owner,
        )
        .unwrap();
    store.apply(&op, &owner).unwrap();
    let marker = store.allocate_operation("capture-fixture-request").unwrap();
    let snapshot = CaptureSnapshot {
        version: 1,
        request_message: marker.creation_id(0),
        thread: request.thread.clone(),
        private_context: false,
        project: None,
        messages: vec![store.record(&op.creation_id(0)).unwrap().unwrap()],
        context: vec![RevisionInput {
            record: request.thread.clone(),
            version: 1,
            expectation: InputExpectation::ChatContext,
        }],
    };
    store
        .prepare_owner(
            &marker,
            &ChangeRequest {
                reason: "Explicit Capture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: snapshot.request_message.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: request.thread.clone(),
                        role: CAPTURE_REQUEST_ROLE.into(),
                        text: snapshot.message_text().unwrap(),
                    }),
                }],
            },
            &owner,
        )
        .unwrap();
    store.apply(&marker, &owner).unwrap();
    request.owner_request = Some(OwnerRequest::CaptureOutcomes {
        request_message: snapshot.request_message.clone(),
    });
    (root, request, snapshot)
}
#[test]
fn capture_preparation_is_review_only_and_fresh_ordinary_refresh_cannot_strip_it() {
    let (_root, mut request, snapshot) = capture_fixture();
    let (scope, input) = begin(&request).unwrap();
    assert!(input.contains("Capture outcomes request"));
    assert_eq!(scope.instruction, snapshot.request_message);
    let prepared = prepare(
        &scope,
        PrepareArgs {
            writing: None,
            request_key: "outcome".into(),
            request: ChangeRequest {
                reason: "Useful decision for owner review".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: "new:0".into(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "Design",
                        "Blue was discussed; approval requires owner review.",
                    )),
                }],
            },
        },
    )
    .unwrap();
    let op = prepared["operation"].as_str().unwrap().to_owned();
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: op.clone()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    drop(scope);
    request.owner_request = None;
    let (ordinary, _) = begin(&request).unwrap();
    assert_eq!(
        apply(
            &ordinary,
            ApplyArgs {
                operation: op.clone()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    let inspected = read_candidate(
        &ordinary,
        CandidateArgs {
            operation: op.clone(),
        },
    )
    .unwrap();
    let full: ChangeRequest = serde_json::from_value(inspected["request"].clone()).unwrap();
    assert!(
        snapshot
            .current_inputs()
            .iter()
            .all(|i| full.inputs.contains(i))
    );
    let mut stripped = full.clone();
    stripped
        .inputs
        .retain(|i| i.record != snapshot.request_message);
    assert!(
        refresh_candidate(
            &ordinary,
            RefreshArgs {
                operation: op.clone(),
                request_key: "strip".into(),
                request: stripped
            }
        )
        .is_err()
    );
    let replacement = refresh_candidate(
        &ordinary,
        RefreshArgs {
            operation: op,
            request_key: "keep".into(),
            request: full,
        },
    )
    .unwrap();
    assert_eq!(
        apply(
            &ordinary,
            ApplyArgs {
                operation: replacement["operation"].as_str().unwrap().into()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    let store = ordinary.store.lock().unwrap();
    assert_eq!(store.candidates().unwrap().len(), 1);
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Note(_)))
            .count(),
        1,
        "only fixture Note is accepted"
    );
}
#[test]
fn capture_request_is_bound_to_chat_and_is_not_an_ordinary_message_read() {
    let (_root, request, snapshot) = capture_fixture();
    let (scope, _) = begin(&request).unwrap();
    let records = read_records(
        &scope,
        ReadArgs {
            ids: vec![snapshot.request_message.clone()],
            query: None,
        },
    )
    .unwrap();
    assert!(records["records"].as_array().unwrap().is_empty());
    assert!(
        capture::load(
            &scope.store.lock().unwrap(),
            &snapshot.request_message,
            "different-chat"
        )
        .is_err()
    );
    assert!(scope.helper.is_none());
}

#[test]
fn capture_precise_edit_keeps_review_gate_in_a_later_ordinary_run() {
    let (_root, mut request, snapshot) = capture_fixture();
    let (scope, _) = begin(&request).unwrap();
    tests::read_all(&scope);
    let note = scope
        .store
        .lock()
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.data, RecordData::Note(_)))
        .unwrap();
    let prepared = note_edit::prepare_edit(
        &scope,
        note_edit::EditArgs {
            id: note.id.clone(),
            version: note.version,
            start: None,
            end: None,
            expected_text: "Monday".into(),
            replacement_text: "Tuesday".into(),
            request_key: "capture-precise".into(),
            reason: "Propose exact outcome update".into(),
        },
    )
    .unwrap();
    let operation = prepared["operation"].as_str().unwrap().to_owned();
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: operation.clone()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    drop(scope);
    request.owner_request = None;
    let (later, _) = begin(&request).unwrap();
    assert_eq!(
        apply(
            &later,
            ApplyArgs {
                operation: operation.clone()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview",
        "Capture precise edits must never become maintenance writes after Continue/reopen"
    );
    let mut store = later.store.lock().unwrap();
    let op = store.operation(&operation).unwrap();
    assert!(
        snapshot.current_inputs().iter().all(|i| store
            .prepared(&op)
            .unwrap()
            .request
            .inputs
            .contains(i))
    );
    assert!(matches!(
        store
            .apply(&op, &HostAuthority::owner("review exact edit"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(store.record(&note.id).unwrap().unwrap().data,RecordData::Note(n) if n.markdown=="Tuesday")
    );
    let undo = store.allocate_operation("undo-capture-precise").unwrap();
    assert!(matches!(
        store
            .undo(&undo, &op, &HostAuthority::owner("undo"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(store.record(&note.id).unwrap().unwrap().data,RecordData::Note(n) if n.markdown=="Monday")
    );
}

#[test]
fn capture_cannot_apply_an_older_unmarked_candidate_but_ordinary_delegation_still_can() {
    let (_root, mut request, _snapshot) = capture_fixture();
    let capture = request.owner_request.take();
    let (ordinary, _) = begin(&request).unwrap();
    let prepared = prepare(
        &ordinary,
        PrepareArgs {
            writing: None,
            request_key: "before-capture".into(),
            request: ChangeRequest {
                reason: "Ordinary pending working note".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: "new:0".into(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "Ordinary pending",
                        "Unmarked ordinary proposal",
                    )),
                }],
            },
        },
    )
    .unwrap();
    let operation = prepared["operation"].as_str().unwrap().to_owned();
    drop(ordinary);
    request.owner_request = capture;
    let (capturing, _) = begin(&request).unwrap();
    assert_eq!(
        apply(
            &capturing,
            ApplyArgs {
                operation: operation.clone()
            }
        )
        .unwrap()["outcome"],
        "NeedsReview"
    );
    {
        let store = capturing.store.lock().unwrap();
        let op = store.operation(&operation).unwrap();
        assert!(store.receipt(&op).unwrap().is_none());
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    }
    drop(capturing);
    request.owner_request = None;
    let (ordinary, _) = begin(&request).unwrap();
    assert_eq!(
        apply(&ordinary, ApplyArgs { operation }).unwrap()["outcome"],
        "Applied"
    );
}
