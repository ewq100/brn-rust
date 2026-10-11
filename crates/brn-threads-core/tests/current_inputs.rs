use brn_threads_core::*;

fn owner() -> HostAuthority {
    HostAuthority::owner("synthetic owner")
}

fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "Synthetic change".into(),
        writes,
        inputs: vec![],
    }
}

fn put(id: String, version: Option<u64>, text: &str) -> Put {
    Put {
        id,
        expected_version: version,
        archived: false,
        data: RecordData::Note(Note::working("Delivery", text)),
    }
}

fn apply(store: &mut Store, op: &OperationId, request: &ChangeRequest) -> Receipt {
    store.prepare_owner(op, request, &owner()).unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(op, &owner()).unwrap() else {
        panic!("fixture must apply")
    };
    receipt
}

#[test]
fn changed_current_input_on_another_connection_stales_the_whole_group() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let directory = root.path().join("fresh");
    let mut store = Store::open(&directory).unwrap();
    let fixture = store.allocate_operation("input Monday").unwrap();
    let input = fixture.creation_id(0);
    apply(
        &mut store,
        &fixture,
        &request(vec![put(input.clone(), None, "Monday")]),
    );

    let summary = store.allocate_operation("prepared Monday summary").unwrap();
    let mut proposed = request(vec![
        put(summary.creation_id(0), None, "Delivery Monday"),
        put(summary.creation_id(1), None, "Related Monday work"),
    ]);
    proposed.inputs.push(RevisionInput {
        expectation: InputExpectation::Current,
        record: input.clone(),
        version: 1,
    });
    store.prepare(&summary, &proposed).unwrap();

    let mut other = Store::open(&directory).unwrap();
    let friday = other.allocate_operation("owner Friday").unwrap();
    apply(
        &mut other,
        &friday,
        &request(vec![put(input.clone(), Some(1), "Friday")]),
    );

    assert_eq!(
        store.apply(&summary, &owner()).unwrap(),
        ApplyOutcome::Stale {
            records: vec![input]
        }
    );
    assert!(store.receipt(&summary).unwrap().is_none());
    for index in 0..2 {
        assert!(store.record(&summary.creation_id(index)).unwrap().is_none());
        assert!(
            store
                .revision(&summary.creation_id(index), 1)
                .unwrap()
                .is_none()
        );
    }
}

fn fixture() -> (tempfile::TempDir, Store, String) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    let op = store.allocate_operation("Monday input").unwrap();
    let input = op.creation_id(0);
    apply(
        &mut store,
        &op,
        &request(vec![put(input.clone(), None, "Monday")]),
    );
    (root, store, input)
}

fn derived(
    store: &mut Store,
    key: &str,
    input: &str,
    version: u64,
    expectation: InputExpectation,
) -> (OperationId, ChangeRequest) {
    let op = store.allocate_operation(key).unwrap();
    let mut req = request(vec![put(op.creation_id(0), None, "Derived Monday")]);
    req.inputs.push(RevisionInput {
        record: input.into(),
        version,
        expectation,
    });
    (op, req)
}

#[test]
fn input_changed_before_prepare_remains_stale_and_cannot_be_owner_approved_away() {
    let (_root, mut store, input) = fixture();
    let friday = store.allocate_operation("Friday before prepare").unwrap();
    apply(
        &mut store,
        &friday,
        &request(vec![put(input.clone(), Some(1), "Friday")]),
    );
    let (op, req) = derived(
        &mut store,
        "obsolete read",
        &input,
        1,
        InputExpectation::Current,
    );
    store.prepare(&op, &req).unwrap();
    for authority in [owner(), HostAuthority::maintenance("agent")] {
        assert_eq!(
            store.apply(&op, &authority).unwrap(),
            ApplyOutcome::Stale {
                records: vec![input.clone()]
            }
        );
        assert!(store.receipt(&op).unwrap().is_none());
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    }
    assert!(store.revision(&input, 1).unwrap().is_some());
}

#[test]
fn inactive_current_inputs_stale_even_at_exact_version_but_historical_citations_apply() {
    for archived in [true, false] {
        let (_root, mut store, input) = fixture();
        let inactive = store.allocate_operation("input retires").unwrap();
        let mut write = put(input.clone(), Some(1), "Monday");
        if archived {
            write.archived = true;
            apply(&mut store, &inactive, &request(vec![write]));
        } else {
            let replacement = inactive.creation_id(0);
            let RecordData::Note(note) = &mut write.data else {
                panic!()
            };
            note.superseded_by = Some(replacement.clone());
            apply(
                &mut store,
                &inactive,
                &request(vec![write, put(replacement, None, "Friday")]),
            );
        }
        let (op, req) = derived(
            &mut store,
            "inactive current",
            &input,
            2,
            InputExpectation::Current,
        );
        store.prepare(&op, &req).unwrap();
        assert_eq!(
            store.apply(&op, &owner()).unwrap(),
            ApplyOutcome::Stale {
                records: vec![input.clone()]
            }
        );
        assert!(store.receipt(&op).unwrap().is_none());
        let (history, req) = derived(
            &mut store,
            "intentional Monday history",
            &input,
            1,
            InputExpectation::Historical,
        );
        let receipt = apply(&mut store, &history, &req);
        assert_eq!(receipt.writes.len(), 1);
        assert_eq!(
            store.prepared(&history).unwrap().request.inputs[0].expectation,
            InputExpectation::Historical
        );
    }
}

#[test]
fn missing_current_input_is_stale_and_missing_historical_revision_is_invalid() {
    let (_root, mut store, _input) = fixture();
    for expectation in [InputExpectation::Current, InputExpectation::Historical] {
        let (op, req) = derived(
            &mut store,
            &format!("missing {expectation:?}"),
            "absent",
            1,
            expectation,
        );
        store.prepare(&op, &req).unwrap();
        match expectation {
            InputExpectation::Current => assert_eq!(
                store.apply(&op, &owner()).unwrap(),
                ApplyOutcome::Stale {
                    records: vec!["absent".into()]
                }
            ),
            InputExpectation::Historical => {
                assert!(matches!(store.apply(&op, &owner()), Err(Error::Invalid(_))))
            }
        }
        assert!(store.receipt(&op).unwrap().is_none());
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    }
}

#[test]
fn old_request_bytes_hash_and_applied_replay_survive_reopen_and_input_changes() {
    use sha2::{Digest, Sha256};
    let (root, mut store, input) = fixture();
    let (op, req) = derived(
        &mut store,
        "legacy serialized request",
        &input,
        1,
        InputExpectation::Current,
    );
    let old_body = serde_json::to_string(&req).unwrap();
    assert!(!old_body.contains("expectation"));
    let old_request: ChangeRequest = serde_json::from_str(&old_body).unwrap();
    assert_eq!(old_request.inputs[0].expectation, InputExpectation::Current);
    assert_eq!(serde_json::to_string(&old_request).unwrap(), old_body);
    let prepared = store.prepare(&op, &old_request).unwrap();
    assert_eq!(
        prepared.request_hash,
        format!("{:x}", Sha256::digest(old_body.as_bytes()))
    );
    drop(store);
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    assert_eq!(store.prepare(&op, &old_request).unwrap(), prepared);
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner()).unwrap() else {
        panic!()
    };
    let friday = store.allocate_operation("Friday after apply").unwrap();
    let changed = apply(
        &mut store,
        &friday,
        &request(vec![put(input, Some(1), "Friday")]),
    );
    assert_eq!(changed.needs_refresh, vec![op.creation_id(0)]);
    drop(store);
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    assert_eq!(store.prepare(&op, &old_request).unwrap(), prepared);
    assert_eq!(
        store.apply(&op, &owner()).unwrap(),
        ApplyOutcome::Applied(receipt)
    );
    assert_eq!(
        store.record(&op.creation_id(0)).unwrap().unwrap().version,
        1
    );
    let mut historical = old_request;
    historical.inputs[0].expectation = InputExpectation::Historical;
    assert!(matches!(
        store.prepare(&op, &historical),
        Err(Error::ImmutableRequest)
    ));
}

#[test]
fn self_input_checks_prestate_and_undo_does_not_reapply_current_read_requirement() {
    let (_root, mut store, input) = fixture();
    let op = store.allocate_operation("self derived edit").unwrap();
    let mut req = request(vec![put(input.clone(), Some(1), "Monday clarified")]);
    req.inputs.push(RevisionInput {
        record: input.clone(),
        version: 1,
        expectation: InputExpectation::Current,
    });
    let original = apply(&mut store, &op, &req);
    assert!(original.needs_refresh.is_empty());
    assert_eq!(
        store.apply(&op, &owner()).unwrap(),
        ApplyOutcome::Applied(original)
    );
    let undo = store.allocate_operation("Undo self derived edit").unwrap();
    let ApplyOutcome::Applied(receipt) = store.undo(&undo, &op, &owner()).unwrap() else {
        panic!()
    };
    assert_eq!(receipt.writes[0].after.version, 3);
    assert!(
        matches!(&receipt.writes[0].after.data, RecordData::Note(note) if note.markdown == "Monday")
    );
    assert!(store.prepared(&undo).unwrap().request.inputs.is_empty());
    assert_eq!(
        store.undo(&undo, &op, &owner()).unwrap(),
        ApplyOutcome::Applied(receipt)
    );
}

#[test]
fn owner_replacement_preserves_current_inputs_and_explicit_historical_replacement_is_reviewable() {
    let (_root, mut store, input) = fixture();
    let (op, req) = derived(
        &mut store,
        "original stale candidate",
        &input,
        1,
        InputExpectation::Current,
    );
    store.prepare(&op, &req).unwrap();
    let friday = store.allocate_operation("Friday before review").unwrap();
    apply(
        &mut store,
        &friday,
        &request(vec![put(input.clone(), Some(1), "Friday")]),
    );
    let replacement = store.allocate_operation("edited review candidate").unwrap();
    let mut replaced = req.clone();
    replaced.writes[0].id = replacement.creation_id(0);
    replaced.writes[0].data = RecordData::Note(Note::working("Review", "Monday reworded"));
    store
        .replace_candidate(&op, &replacement, &replaced, &owner())
        .unwrap();
    assert_eq!(
        store.apply(&replacement, &owner()).unwrap(),
        ApplyOutcome::Stale {
            records: vec![input.clone()]
        }
    );
    let history = store
        .allocate_operation("explicit historical review candidate")
        .unwrap();
    replaced.writes[0].id = history.creation_id(0);
    replaced.inputs[0].expectation = InputExpectation::Historical;
    let prepared = store
        .replace_candidate(&replacement, &history, &replaced, &owner())
        .unwrap();
    assert_eq!(prepared.request.inputs, replaced.inputs);
    assert!(matches!(
        store.apply(&history, &owner()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn mixed_inputs_preserve_history_and_require_every_current_read_without_duplicate_stale_ids() {
    let (_root, mut store, input) = fixture();
    let fresh_op = store.allocate_operation("unrelated fresh input").unwrap();
    let fresh = fresh_op.creation_id(0);
    apply(
        &mut store,
        &fresh_op,
        &request(vec![put(fresh.clone(), None, "Unchanged")]),
    );
    let friday = store.allocate_operation("input Friday").unwrap();
    apply(
        &mut store,
        &friday,
        &request(vec![put(input.clone(), Some(1), "Friday")]),
    );
    let op = store
        .allocate_operation("mixed stale target and input group")
        .unwrap();
    let mut req = request(vec![
        put(input.clone(), Some(1), "Obsolete target edit"),
        put(
            op.creation_id(0),
            None,
            "Derived Monday and unchanged facts",
        ),
    ]);
    req.inputs = vec![
        RevisionInput {
            record: input.clone(),
            version: 1,
            expectation: InputExpectation::Historical,
        },
        RevisionInput {
            record: fresh.clone(),
            version: 1,
            expectation: InputExpectation::Current,
        },
        RevisionInput {
            record: input.clone(),
            version: 1,
            expectation: InputExpectation::Current,
        },
    ];
    store.prepare(&op, &req).unwrap();
    assert_eq!(
        store.apply(&op, &owner()).unwrap(),
        ApplyOutcome::Stale {
            records: vec![input.clone()]
        }
    );
    assert!(store.receipt(&op).unwrap().is_none());
    assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    assert_eq!(store.record(&input).unwrap().unwrap().version, 2);
    assert_eq!(store.record(&fresh).unwrap().unwrap().version, 1);
}

#[test]
fn undo_restores_older_provenance_without_turning_it_into_a_new_current_read() {
    let (_root, mut store, input_a) = fixture();
    let b = store.allocate_operation("input B").unwrap();
    let input_b = b.creation_id(0);
    apply(
        &mut store,
        &b,
        &request(vec![put(input_b.clone(), None, "Source B")]),
    );
    let (original, req) = derived(
        &mut store,
        "summary from A",
        &input_a,
        1,
        InputExpectation::Current,
    );
    apply(&mut store, &original, &req);
    let summary = original.creation_id(0);
    let rewritten = store.allocate_operation("summary now from B").unwrap();
    let mut req = request(vec![put(summary.clone(), Some(1), "Derived B")]);
    req.inputs.push(RevisionInput {
        record: input_b,
        version: 1,
        expectation: InputExpectation::Current,
    });
    apply(&mut store, &rewritten, &req);
    let changed_a = store.allocate_operation("A changes before Undo").unwrap();
    assert!(
        apply(
            &mut store,
            &changed_a,
            &request(vec![put(input_a.clone(), Some(1), "Friday")])
        )
        .needs_refresh
        .is_empty()
    );
    let undo = store
        .allocate_operation("Undo restores historical A footing")
        .unwrap();
    assert!(matches!(
        store.undo(&undo, &rewritten, &owner()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(store.prepared(&undo).unwrap().request.inputs.is_empty());
    let newer_a = store.allocate_operation("A changes after Undo").unwrap();
    let receipt = apply(
        &mut store,
        &newer_a,
        &request(vec![put(input_a, Some(2), "Saturday")]),
    );
    assert_eq!(receipt.needs_refresh, vec![summary]);
}
