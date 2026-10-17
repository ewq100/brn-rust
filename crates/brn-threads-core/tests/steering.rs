use brn_threads_core::*;

fn apply(store: &mut Store, key: &str, writes: Vec<Put>) -> Receipt {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic owner");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!("fixture did not apply")
    };
    receipt
}

fn fixture(state: RunState, fence: u64) -> (tempfile::TempDir, Store, String, String) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    let op = store.allocate_operation("setup").unwrap();
    let thread = op.creation_id(0);
    let run = op.creation_id(1);
    apply(
        &mut store,
        "setup",
        vec![
            Put {
                id: thread.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    private_context: false,
                    context_epoch: 0,
                    title: "Synthetic steering".into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            },
            Put {
                id: run.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Run(Run {
                    loaded_skill_details: vec![],
                    instruction: None,
                    queue_claim: None,
                    thread: thread.clone(),
                    provider: "Offline".into(),
                    model: "synthetic".into(),
                    guide_identity: "fixture".into(),
                    budget: 4,
                    effort: "medium".into(),
                    loaded_skills: vec![],
                    fence,
                    state,
                    progress: "retained progress".into(),
                    operations: vec![],
                    sources: vec![],
                }),
            },
        ],
    );
    (root, store, thread, run)
}

fn change_record(store: &mut Store, id: &str, key: &str, change: impl FnOnce(&mut Record)) {
    let mut record = store.record(id).unwrap().unwrap();
    change(&mut record);
    apply(
        store,
        key,
        vec![Put {
            id: record.id,
            expected_version: Some(record.version),
            archived: record.archived,
            data: record.data,
        }],
    );
}

#[test]
fn saves_exact_message_and_fences_stale_writes_and_completion_atomically() {
    let (_root, mut store, thread, run) = fixture(RunState::Working, 7);
    let old = store.record(&run).unwrap().unwrap();
    let stale = store.allocate_operation("old completion").unwrap();
    let RecordData::Run(mut completed) = old.data else {
        panic!()
    };
    completed.state = RunState::Completed;
    completed.fence += 1;
    store
        .prepare(
            &stale,
            &ChangeRequest {
                reason: "Delayed old completion".into(),
                writes: vec![Put {
                    id: run.clone(),
                    expected_version: Some(old.version),
                    archived: false,
                    data: RecordData::Run(completed),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    let write = store.allocate_operation("old note write").unwrap();
    store
        .prepare(
            &write,
            &ChangeRequest {
                reason: "Delayed old write".into(),
                writes: vec![Put {
                    id: write.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Old", "Must not commit")),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    let op = store.allocate_operation("stop and send").unwrap();
    let outcome = store
        .stop_and_post(
            &op,
            &run,
            &thread,
            "  New instruction\n",
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = outcome else {
        panic!()
    };
    assert_eq!(receipt.writes.len(), 2);
    assert!(
        matches!(&receipt.writes[0].after.data, RecordData::Message(m) if m.thread==thread && m.role=="owner" && m.text=="  New instruction\n")
    );
    assert!(
        matches!(store.record(&run).unwrap().unwrap().data, RecordData::Run(r) if r.state==RunState::Interrupted && r.fence==8 && r.progress=="retained progress")
    );
    for delayed in [&stale, &write] {
        assert!(matches!(
            store
                .apply(
                    delayed,
                    &HostAuthority::owner("host old run").for_run(&run, 7)
                )
                .unwrap(),
            ApplyOutcome::Superseded { .. }
        ));
        assert!(store.receipt(delayed).unwrap().is_none());
    }
}

#[test]
fn lost_response_replays_after_later_run_resolution_archive_and_reopen() {
    let (root, mut store, thread, run) = fixture(RunState::Working, 1);
    let op = store.allocate_operation("durable stop").unwrap();
    let owner = HostAuthority::owner("owner");
    let receipt = store
        .stop_and_post(&op, &run, &thread, "New instruction", &owner)
        .unwrap();
    change_record(&mut store, &thread, "resolve and archive", |r| {
        r.archived = true;
        let RecordData::Thread(t) = &mut r.data else {
            panic!()
        };
        t.state = ThreadState::Resolved;
    });
    change_record(&mut store, &run, "archive exact run", |r| {
        r.archived = true;
        let RecordData::Run(r) = &mut r.data else {
            panic!()
        };
        r.fence += 1;
    });
    drop(store);
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    assert_eq!(
        store
            .stop_and_post(&op, &run, &thread, "New instruction", &owner)
            .unwrap(),
        receipt
    );
    for (run, thread, text) in [
        (&run[..], &thread[..], "Changed"),
        ("different run", &thread[..], "New instruction"),
        (&run[..], "different thread", "New instruction"),
    ] {
        assert!(matches!(
            store.stop_and_post(&op, run, thread, text, &owner),
            Err(Error::ImmutableRequest)
        ));
    }
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Message(_)))
            .count(),
        1
    );
}

#[test]
fn terminal_race_keeps_terminal_state_and_accepts_only_one_message() {
    for state in [
        RunState::Completed,
        RunState::Cancelled,
        RunState::Failed,
        RunState::Interrupted,
    ] {
        let (_root, mut store, thread, run) = fixture(state, 9);
        let before = store.record(&run).unwrap().unwrap();
        let op = store.allocate_operation("terminal send").unwrap();
        let receipt = store
            .stop_and_post(
                &op,
                &run,
                &thread,
                "Continue differently",
                &HostAuthority::owner("owner"),
            )
            .unwrap();
        assert!(matches!(receipt, ApplyOutcome::Applied(ref r) if r.writes.len()==1));
        assert_eq!(store.record(&run).unwrap().unwrap(), before);
    }
}

#[test]
fn rejects_any_other_working_invocation_in_the_same_thread() {
    for state in [RunState::Interrupted, RunState::Completed] {
        let (_root, mut store, thread, run) = fixture(state, 1);
        let op = store.allocate_operation("other run").unwrap();
        let RecordData::Run(mut other) = store.record(&run).unwrap().unwrap().data else {
            panic!()
        };
        other.state = RunState::Working;
        apply(
            &mut store,
            "other run",
            vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Run(other),
            }],
        );
        let before = store.records().unwrap();
        let op = store
            .allocate_operation("send to obsolete invocation")
            .unwrap();
        assert!(
            store
                .stop_and_post(
                    &op,
                    &run,
                    &thread,
                    "New instruction",
                    &HostAuthority::owner("owner")
                )
                .is_err()
        );
        assert_eq!(store.records().unwrap(), before);
        assert!(store.receipt(&op).unwrap().is_none());
    }
}

#[test]
fn invalid_destinations_text_authority_and_fence_overflow_do_not_partially_write() {
    for failure in [
        "missing run",
        "wrong thread",
        "resolved",
        "archived thread",
        "archived run",
        "not run",
        "empty",
        "oversized",
        "authority",
        "overflow",
    ] {
        let (_root, mut store, thread, run) = fixture(
            RunState::Working,
            if failure == "overflow" { u64::MAX } else { 1 },
        );
        if matches!(failure, "resolved" | "archived thread") {
            change_record(&mut store, &thread, "change destination", |r| {
                if failure == "archived thread" {
                    r.archived = true;
                } else if let RecordData::Thread(t) = &mut r.data {
                    t.state = ThreadState::Resolved;
                }
            });
        }
        if failure == "archived run" {
            change_record(&mut store, &run, "archive run", |r| {
                r.archived = true;
                if let RecordData::Run(r) = &mut r.data {
                    r.fence += 1;
                }
            });
        }
        let before = store.records().unwrap();
        let op = store.allocate_operation("failed stop").unwrap();
        let oversized = "a".repeat(128 * 1024 + 1);
        let text = match failure {
            "empty" => " \n",
            "oversized" => &oversized,
            _ => "Instruction",
        };
        let authority = if failure == "authority" {
            HostAuthority::maintenance("agent")
        } else {
            HostAuthority::owner("owner")
        };
        assert!(
            store
                .stop_and_post(
                    &op,
                    match failure {
                        "missing run" => "missing",
                        "not run" => &thread,
                        _ => &run,
                    },
                    if failure == "wrong thread" {
                        "wrong"
                    } else {
                        &thread
                    },
                    text,
                    &authority
                )
                .is_err(),
            "{failure}"
        );
        assert_eq!(store.records().unwrap(), before, "{failure}");
        assert!(store.receipt(&op).unwrap().is_none(), "{failure}");
        assert!(store.prepared(&op).is_err(), "{failure}");
    }
}

#[test]
fn cannot_rebind_an_existing_unrelated_committed_receipt() {
    let (_root, mut store, thread, run) = fixture(RunState::Working, 1);
    let op = store.allocate_operation("setup").unwrap();
    let before = store.records().unwrap();
    assert!(matches!(
        store.stop_and_post(
            &op,
            &run,
            &thread,
            "Instruction",
            &HostAuthority::owner("owner")
        ),
        Err(Error::ImmutableRequest)
    ));
    assert_eq!(store.records().unwrap(), before);
}

#[test]
fn checked_apply_failure_rolls_back_preparation_message_and_fence() {
    let (_root, mut store, thread, run) = fixture(RunState::Working, 1);
    let before = store.records().unwrap();
    let op = store.allocate_operation("checked rollback").unwrap();
    assert!(
        store
            .stop_and_post(
                &op,
                &run,
                &thread,
                "Instruction",
                &HostAuthority::owner("owner").for_settings("missing settings", 1)
            )
            .is_err()
    );
    assert_eq!(store.records().unwrap(), before);
    assert!(store.prepared(&op).is_err());
    assert!(store.receipt(&op).unwrap().is_none());
    assert!(
        store
            .stop_and_post(
                &op,
                &run,
                &thread,
                "Instruction",
                &HostAuthority::owner("owner")
            )
            .is_ok()
    );
}

#[test]
fn existing_wrong_thread_membership_is_rejected_without_partial_effects() {
    let (_root, mut store, thread, run) = fixture(RunState::Working, 1);
    let op = store.allocate_operation("other thread").unwrap();
    let other = op.creation_id(0);
    apply(
        &mut store,
        "other thread",
        vec![Put {
            id: other.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Thread(Thread {
                private_context: false,
                context_epoch: 0,
                title: "Other destination".into(),
                state: ThreadState::Open,
                attention: vec![],
            }),
        }],
    );
    let before = store.records().unwrap();
    let op = store.allocate_operation("wrong membership").unwrap();
    assert!(
        store
            .stop_and_post(
                &op,
                &run,
                &other,
                "Instruction",
                &HostAuthority::owner("owner")
            )
            .is_err()
    );
    assert_eq!(store.records().unwrap(), before);
    assert!(
        store
            .stop_and_post(
                &op,
                &run,
                &thread,
                "Instruction",
                &HostAuthority::owner("owner")
            )
            .is_ok()
    );
}
