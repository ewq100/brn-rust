use super::*;

fn pending(request: &ThreadRunRequest) -> (String, u64) {
    let mut store = Store::open(&request.data_dir).unwrap();
    let record = store
        .records()
        .unwrap()
        .into_iter()
        .find(|record| matches!(record.data, RecordData::Settings(_)))
        .unwrap();
    let RecordData::Settings(mut settings) = record.data else {
        panic!()
    };
    settings.delegation_chosen = false;
    settings.maintenance = false;
    let op = store
        .allocate_operation("synthetic pending bootstrap")
        .unwrap();
    let owner = HostAuthority::owner("fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic pending workspace".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: record.id.clone(),
                    expected_version: Some(record.version),
                    archived: false,
                    data: RecordData::Settings(settings),
                }],
            },
            &owner,
        )
        .unwrap();
    store.apply(&op, &owner).unwrap();
    (record.id, record.version + 1)
}

#[test]
fn invocation_input_reports_only_current_host_policy_and_keeps_settings_binding() {
    for policy in ["Pending", "Paused", "ReviewFirst", "MaintainOrdinaryNotes"] {
        let (_root, request, _) = tests::fixture();
        let (settings_id, _) = pending(&request);
        let mut store = Store::open(&request.data_dir).unwrap();
        let owner = HostAuthority::owner("fixture owner choice");
        if policy != "Pending" {
            let op = store.allocate_operation("explicit policy choice").unwrap();
            let choice = if policy == "ReviewFirst" {
                DelegationChoice::ReviewFirst
            } else {
                DelegationChoice::MaintainOrdinaryNotes
            };
            store.choose_delegation(&op, choice, &owner).unwrap();
            if policy == "Paused" {
                let op = store.allocate_operation("explicit pause").unwrap();
                store.pause_workspace(&op, true, &owner).unwrap();
            }
        }
        let version = store.record(&settings_id).unwrap().unwrap().version;
        let (scope, input) = begin(&request).unwrap();
        assert!(
            input.contains(&format!("Host maintenance policy: {policy}\n")),
            "{input}"
        );
        assert_eq!(scope.settings, Some((settings_id, version)));
        for private_field in [
            "auth_file",
            "credentials_dir",
            "delegation_chosen",
            "review_first",
        ] {
            assert!(!input.contains(private_field));
        }
        let op = store
            .allocate_operation("steer policy while invocation exists")
            .unwrap();
        store
            .choose_delegation(&op, DelegationChoice::ReviewFirst, &owner)
            .unwrap();
        assert!(live(&scope, &scope.store.lock().unwrap()).is_err());
    }
}

#[test]
fn pending_runtime_requires_review_but_scoped_owner_instruction_still_applies() {
    for direct in [false, true] {
        let (_root, mut request, note) = tests::fixture();
        pending(&request);
        if direct {
            request.owner_request = Some(OwnerRequest::EditNote {
                id: note.clone(),
                version: 1,
            });
        }
        let (scope, _) = begin(&request).unwrap();
        let prepared = prepare(
            &scope,
            PrepareArgs {
                request_key: "pending update".into(),
                request: ChangeRequest {
                    reason: "Update selected synthetic note".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: note.clone(),
                        expected_version: Some(1),
                        archived: false,
                        data: RecordData::Note(Note::working("Decision", "Tuesday")),
                    }],
                },
            },
        )
        .unwrap();
        let result = apply(
            &scope,
            ApplyArgs {
                operation: prepared["operation"].as_str().unwrap().into(),
            },
        )
        .unwrap();
        assert_eq!(
            result["outcome"],
            if direct { "Applied" } else { "NeedsReview" }
        );
        let store = scope.store.lock().unwrap();
        let RecordData::Note(current) = store.record(&note).unwrap().unwrap().data else {
            panic!()
        };
        assert_eq!(current.markdown, if direct { "Tuesday" } else { "Monday" });
    }
}
