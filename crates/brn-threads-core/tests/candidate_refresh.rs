use brn_threads_core::*;

#[test]
fn runtime_refresh_checks_family_fence_all_versions_and_preserves_apply_authority() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("fresh")).unwrap();
    let fixture = store.allocate_operation("fixture").unwrap();
    let original = store.allocate_operation("original").unwrap();
    let replacement = store.allocate_operation("replacement").unwrap();
    let foreign = store.allocate_operation("foreign").unwrap();
    let thread = fixture.creation_id(0);
    let note = fixture.creation_id(1);
    let run = fixture.creation_id(2);
    let mut protected = Note::working("Protected decision", "Original accepted text");
    protected.protected = true;
    let owner = HostAuthority::owner("test owner");
    let setup = ChangeRequest {
        reason: "Synthetic runtime setup".into(),
        writes: vec![
            Put {
                id: thread.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    private_context: false,
                    context_epoch: 0,
                    title: "Work".into(),
                    state: ThreadState::Open,
                    attention: vec![
                        Attention {
                            kind: AttentionKind::Conflict,
                            reason: "Stale candidate".into(),
                            record: Some(original.as_str().into()),
                        },
                        Attention {
                            kind: AttentionKind::Question,
                            reason: "Other question".into(),
                            record: None,
                        },
                    ],
                }),
            },
            Put {
                id: note.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(protected.clone()),
            },
            Put {
                id: run.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Run(Run {
                    instruction: None,
                    queue_claim: None,
                    thread: thread.clone(),
                    provider: "Codex".into(),
                    model: "gpt-6.1-sol".into(),
                    guide_identity: "test".into(),
                    budget: 4,
                    effort: "medium".into(),
                    loaded_skills: vec![],
                    fence: 1,
                    state: RunState::Working,
                    progress: String::new(),
                    operations: vec![original.clone(), replacement.clone()],
                    sources: vec![],
                }),
            },
        ],
        inputs: vec![],
    };
    store.prepare_owner(&fixture, &setup, &owner).unwrap();
    store.apply(&fixture, &owner).unwrap();
    let mut proposed = protected.clone();
    proposed.markdown = "Old proposal".into();
    let mut request = ChangeRequest {
        reason: "Proposal".into(),
        writes: vec![Put {
            id: note.clone(),
            expected_version: Some(1),
            archived: false,
            data: RecordData::Note(proposed.clone()),
        }],
        inputs: vec![RevisionInput {
            expectation: InputExpectation::Current,
            record: note.clone(),
            version: 1,
        }],
    };
    store.prepare(&original, &request).unwrap();
    let changed = store.allocate_operation("owner-change").unwrap();
    protected.markdown = "Current owner text".into();
    store
        .prepare_owner(
            &changed,
            &ChangeRequest {
                reason: "Owner changed current".into(),
                writes: vec![Put {
                    id: note.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(protected),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    store.apply(&changed, &owner).unwrap();
    let runtime = HostAuthority::runtime("host", &run).for_run(&run, 1);
    assert!(
        store
            .refresh_candidate(
                &original,
                &replacement,
                &request,
                &HostAuthority::runtime("host", &run)
            )
            .is_err()
    );
    assert!(
        store
            .refresh_candidate(
                &original,
                &replacement,
                &request,
                &HostAuthority::runtime("host", &run).for_run(&run, 2)
            )
            .is_err()
    );
    assert!(
        store
            .refresh_candidate(&foreign, &replacement, &request, &runtime)
            .is_err()
    );
    assert!(
        store
            .refresh_candidate(&original, &replacement, &request, &runtime)
            .is_err()
    );
    request.writes[0].expected_version = Some(2);
    assert!(
        store
            .refresh_candidate(&original, &replacement, &request, &runtime)
            .is_err()
    );
    assert!(
        store
            .candidates()
            .unwrap()
            .iter()
            .any(|candidate| candidate.operation == original)
    );
    request.inputs[0].version = 2;
    proposed.markdown = "Fresh proposed text".into();
    request.writes[0].data = RecordData::Note(proposed);
    let prepared = store
        .refresh_candidate(&original, &replacement, &request, &runtime)
        .unwrap();
    assert_eq!(
        store
            .refresh_candidate(&original, &replacement, &request, &runtime)
            .unwrap(),
        prepared
    );
    assert!(
        !store
            .candidates()
            .unwrap()
            .iter()
            .any(|candidate| candidate.operation == original)
    );
    assert!(
        matches!(store.record(&note).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Current owner text")
    );
    let RecordData::Thread(current_thread) = store.record(&thread).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(
        current_thread.attention[0].record.as_deref(),
        Some(replacement.as_str())
    );
    assert_eq!(current_thread.attention[1].reason, "Other question");
    assert!(matches!(
        store.apply(&replacement, &runtime).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(matches!(
        store
            .apply(&replacement, &HostAuthority::maintenance("agent"))
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(matches!(
        store.apply(&replacement, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(
        store
            .refresh_candidate(&original, &replacement, &request, &runtime)
            .unwrap(),
        prepared
    );
    request.reason = "Changed immutable retry".into();
    assert!(matches!(
        store.refresh_candidate(&original, &replacement, &request, &runtime),
        Err(Error::ImmutableRequest)
    ));
}
