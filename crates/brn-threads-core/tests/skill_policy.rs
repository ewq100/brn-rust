use brn_threads_core::*;
fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn checked(store: &mut Store, data: RecordData) -> Record {
    let op = store
        .allocate_operation(&uuid::Uuid::new_v4().to_string())
        .unwrap();
    let id = op.creation_id(0);
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "synthetic owner fixture".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data,
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("fixture"),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &HostAuthority::owner("fixture")).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    store.record(&id).unwrap().unwrap()
}
fn settings(store: &Store) -> WorkspaceSettings {
    store
        .records()
        .unwrap()
        .into_iter()
        .find_map(|r| match r.data {
            RecordData::Settings(s) => Some(s),
            _ => None,
        })
        .unwrap()
}
fn initial(store: &mut Store) {
    checked(store, RecordData::Settings(serde_json::from_value(serde_json::json!({"provider":"Codex","model":"gpt-6.1-sol","effort":"high","auth_file":null,"credentials_dir":null,"maintenance":true,"review_first":false,"delegation_chosen":true})).unwrap()));
}
fn binding(project: Option<String>) -> SkillBinding {
    SkillBinding {
        id: "installed:review".into(),
        digest: "a".repeat(64),
        project,
        enabled: true,
        selected: false,
    }
}
#[test]
fn scoped_policy_replay_fences_and_preserves_fresh_configuration_and_backup() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    let chat = checked(
        &mut store,
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "chat".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let run = checked(
        &mut store,
        RecordData::Run(Run {
            instruction: None,
            queue_claim: None,
            thread: chat.id,
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            guide_identity: "test".into(),
            budget: 8,
            effort: "high".into(),
            loaded_skills: vec![],
            loaded_skill_details: vec![],
            fence: 1,
            state: RunState::Working,
            progress: "partial".into(),
            operations: vec![],
            sources: vec![],
        }),
    );
    let stale = settings(&store);
    let op = store.allocate_operation("enable").unwrap();
    let change = SkillPolicyChange::Set(binding(None));
    let receipt = store
        .change_skill_policy(&op, &change, &HostAuthority::owner("owner"))
        .unwrap();
    assert!(matches!(receipt, ApplyOutcome::Applied(_)));
    let RecordData::Run(ended) = store.record(&run.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(ended.fence, 2);
    assert_eq!(ended.state, RunState::Interrupted);
    assert_eq!(ended.progress, "partial");
    let configure = store
        .allocate_operation("fresh model from stale full settings")
        .unwrap();
    let mut requested = stale;
    requested.model = "other".into();
    store
        .configure_workspace(&configure, &requested, &HostAuthority::owner("owner"))
        .unwrap();
    assert_eq!(settings(&store).skills, vec![binding(None)]);
    assert_eq!(
        store
            .change_skill_policy(&op, &change, &HostAuthority::owner("retry"))
            .unwrap(),
        receipt
    );
    assert_eq!(settings(&store).model, "other");
    assert!(matches!(
        store.change_skill_policy(
            &op,
            &SkillPolicyChange::Clear {
                id: "installed:review".into(),
                project: None
            },
            &HostAuthority::owner("retry")
        ),
        Err(Error::ImmutableRequest)
    ));
    // Configure replay must preserve the originally committed policy too.
    assert!(matches!(
        store
            .configure_workspace(&configure, &requested, &HostAuthority::owner("retry"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let denied = store.allocate_operation("model policy").unwrap();
    assert!(
        store
            .change_skill_policy(&denied, &change, &HostAuthority::maintenance("model"))
            .is_err()
    );
    let backup = root.path().join("copy.sqlite3");
    store.backup(&backup).unwrap();
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(settings(&restored), settings(&store));
    assert_eq!(
        restored.record(&run.id).unwrap(),
        store.record(&run.id).unwrap()
    );
}
#[test]
fn raw_owner_replacement_cannot_bypass_binding_bounds_or_project_checks() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    let current = store
        .records()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.data, RecordData::Settings(_)))
        .unwrap();
    for bad in [
        vec![binding(None), binding(None)],
        vec![SkillBinding {
            digest: "bad".into(),
            ..binding(None)
        }],
        vec![SkillBinding {
            enabled: false,
            selected: true,
            ..binding(None)
        }],
        vec![binding(Some("missing-project".into()))],
        vec![SkillBinding {
            project: Some("   ".into()),
            enabled: false,
            ..binding(None)
        }],
        (0..129)
            .map(|i| SkillBinding {
                id: format!("installed:review-{i}"),
                ..binding(None)
            })
            .collect(),
        vec![
            SkillBinding {
                selected: true,
                ..binding(None)
            },
            SkillBinding {
                id: "installed:second".into(),
                selected: true,
                ..binding(None)
            },
        ],
    ] {
        let mut next = settings(&store);
        next.skills = bad;
        let op = store
            .allocate_operation(&uuid::Uuid::new_v4().to_string())
            .unwrap();
        let prepared = store.prepare_owner(
            &op,
            &ChangeRequest {
                reason: "invalid raw owner replacement".into(),
                writes: vec![Put {
                    id: current.id.clone(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Settings(next),
                }],
                inputs: vec![],
            },
            &HostAuthority::owner("owner"),
        );
        assert!(
            prepared.is_err()
                || !matches!(
                    store.apply(&op, &HostAuthority::owner("owner")),
                    Ok(ApplyOutcome::Applied(_))
                )
        );
        assert_eq!(store.record(&current.id).unwrap().unwrap(), current);
    }
}
#[test]
fn project_selection_is_exact_and_clear_remains_available_after_archive() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    initial(&mut store);
    let project = checked(
        &mut store,
        RecordData::Project(Project {
            title: "private".into(),
            instructions: String::new(),
            use_brn_knowledge: false,
            chats: vec![],
            documents: vec![],
            trash: None,
        }),
    );
    let b = binding(Some(project.id.clone()));
    let op = store.allocate_operation("project enable").unwrap();
    store
        .change_skill_policy(
            &op,
            &SkillPolicyChange::Set(b.clone()),
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    let select = SkillPolicyChange::Select {
        id: Some(b.id.clone()),
        digest: Some(b.digest.clone()),
        project: b.project.clone(),
    };
    let op = store.allocate_operation("project select").unwrap();
    store
        .change_skill_policy(&op, &select, &HostAuthority::owner("owner"))
        .unwrap();
    assert!(settings(&store).skills[0].selected);
    let write = Put {
        id: project.id.clone(),
        expected_version: Some(project.version),
        archived: true,
        data: project.data,
    };
    let op = store.allocate_operation("archive project fixture").unwrap();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "archive".into(),
                writes: vec![write.clone()],
                inputs: vec![],
            },
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    store.apply(&op, &HostAuthority::owner("owner")).unwrap();
    // An unrelated Configure must preserve an unchanged unavailable selection.
    let mut configured = settings(&store);
    assert!(configured.skills[0].selected);
    configured.model = "fresh configured model".into();
    let configure = store
        .allocate_operation("configure inactive selected policy")
        .unwrap();
    store
        .configure_workspace(&configure, &configured, &HostAuthority::owner("owner"))
        .unwrap();
    assert_eq!(settings(&store).skills, configured.skills);
    assert!(matches!(
        store
            .configure_workspace(&configure, &configured, &HostAuthority::owner("retry"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let reject_raw = |store: &mut Store, invalid: SkillBinding| {
        let current = store
            .records()
            .unwrap()
            .into_iter()
            .find(|r| matches!(r.data, RecordData::Settings(_)))
            .unwrap();
        let mut requested = settings(store);
        requested.skills = vec![invalid];
        let raw = store
            .allocate_operation(&uuid::Uuid::new_v4().to_string())
            .unwrap();
        store
            .prepare_owner(
                &raw,
                &ChangeRequest {
                    reason: "inactive policy cannot acquire new identity or selection".into(),
                    writes: vec![Put {
                        id: current.id.clone(),
                        expected_version: Some(current.version),
                        archived: false,
                        data: RecordData::Settings(requested),
                    }],
                    inputs: vec![],
                },
                &HostAuthority::owner("owner"),
            )
            .unwrap();
        assert!(store.apply(&raw, &HostAuthority::owner("owner")).is_err());
        assert_eq!(store.record(&current.id).unwrap().unwrap(), current);
    };
    for invalid in [
        SkillBinding {
            digest: "b".repeat(64),
            ..b.clone()
        },
        SkillBinding {
            id: "installed:another".into(),
            ..b.clone()
        },
        SkillBinding {
            project: Some("missing-project".into()),
            ..b.clone()
        },
    ] {
        reject_raw(&mut store, invalid);
    }
    let deselect = SkillPolicyChange::Select {
        id: None,
        digest: None,
        project: b.project.clone(),
    };
    let deselect_op = store
        .allocate_operation("deselect inactive project")
        .unwrap();
    let deselected = store
        .change_skill_policy(&deselect_op, &deselect, &HostAuthority::owner("owner"))
        .expect("deselecting retained inactive policy is monotone");
    assert!(matches!(deselected, ApplyOutcome::Applied(_)));
    assert_eq!(
        store
            .change_skill_policy(&deselect_op, &deselect, &HostAuthority::owner("retry"))
            .unwrap(),
        deselected
    );
    assert_eq!(
        settings(&store).skills,
        vec![SkillBinding {
            selected: false,
            ..b.clone()
        }]
    );
    reject_raw(
        &mut store,
        SkillBinding {
            selected: true,
            ..b.clone()
        },
    );
    let current = store
        .records()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.data, RecordData::Settings(_)))
        .unwrap();
    let enable = store.allocate_operation("inactive enable").unwrap();
    assert!(
        store
            .change_skill_policy(
                &enable,
                &SkillPolicyChange::Set(b.clone()),
                &HostAuthority::owner("owner")
            )
            .is_err()
    );
    assert_eq!(store.record(&current.id).unwrap().unwrap(), current);
    let op = store.allocate_operation("inactive select").unwrap();
    assert!(
        store
            .change_skill_policy(&op, &select, &HostAuthority::owner("owner"))
            .is_err()
    );
    let disable = store.allocate_operation("disable inactive").unwrap();
    assert!(matches!(
        store
            .change_skill_policy(
                &disable,
                &SkillPolicyChange::Set(SkillBinding {
                    enabled: false,
                    selected: false,
                    ..b.clone()
                }),
                &HostAuthority::owner("owner")
            )
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(!settings(&store).skills[0].enabled);
    assert!(!settings(&store).skills[0].selected);
    let op = store.allocate_operation("clear inactive").unwrap();
    store
        .change_skill_policy(
            &op,
            &SkillPolicyChange::Clear {
                id: b.id,
                project: b.project,
            },
            &HostAuthority::owner("owner"),
        )
        .unwrap();
    assert!(settings(&store).skills.is_empty());
    // Positive control: archived project data remains retained.
    assert!(store.record(&write.id).unwrap().unwrap().archived);
}
