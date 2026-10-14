use super::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};

fn figure(name: &str, media_type: &str, bytes: &[u8]) -> TransientAsset {
    let bytes = bytes.to_vec();
    let digest = Asset {
        note: String::new(),
        source: None,
        media_type: media_type.into(),
        bytes: bytes.clone(),
    }
    .content_hash();
    TransientAsset {
        name: name.into(),
        media_type: media_type.into(),
        sha256: std::array::from_fn(|index| {
            u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap()
        }),
        width: 480,
        height: 240,
        bytes,
    }
}
fn source() -> TransientSource {
    TransientSource {
        reference: "synthetic://meaningful-figures".into(),
        text: "Retain the map where relevant. ![Map](map.png)\nUnneeded illustration ![Unused](unused.jpg)\nEXCLUDED_RAW_CANARY_FIGURES_68219".into(),
        fingerprint: Some("independent-synthetic-source-fingerprint".into()),
        report: Some(ImportReport {
            status: ImportStatus::Partial,
            issues: vec![
                ImportIssue { kind: ImportIssueKind::Omission, locator: "body/chart-3".into(), detail: "Unsupported chart could not be extracted".into() },
                ImportIssue { kind: ImportIssueKind::Unverified, locator: "document".into(), detail: "No independent complete inventory".into() },
            ],
        }),
        assets: vec![
            figure("map.png", "image/png", include_bytes!("../../brn-intake/tests/fixtures/quay-map.png")),
            figure("unused.jpg", "image/jpeg", include_bytes!("../../brn-intake/tests/fixtures/quay-map.jpg")),
        ],
    }
}

#[tokio::test]
async fn useful_intake_model_tools_retain_selected_figure_omissions_and_replay_without_raw_archive()
{
    let (_root, mut request, note) = super::tests::fixture();
    request.sources.push(source());
    let (scope, input) = begin(&request).unwrap();
    super::tests::read_all(&scope);
    let metadata = read_source(
        &scope,
        SourceArgs {
            reference: request.sources[0].reference.clone(),
        },
    )
    .unwrap();
    assert_eq!(metadata["figures"][0]["link"], "brn-transient-asset://0/0");
    assert!(
        metadata["text"]
            .as_str()
            .unwrap()
            .contains("](brn-transient-asset://0/0)")
    );
    assert!(metadata["figures"][0].get("bytes").is_none());
    assert_eq!(metadata["coverage"]["issues"][0]["kind"], "Omission");
    let mut selected_note = Note::working(
        "Useful map",
        "Selected map and a source omission remain visible. ![Quay map](brn-transient-asset://0/0)",
    );
    // A tool proposal cannot silently replace host coverage with invented completeness.
    selected_note.import = Some(ImportCoverage {
        source: metadata["source_record"]["id"].as_str().unwrap().into(),
        full_note: true,
        gaps: vec![],
        report: Some(ImportReport {
            status: ImportStatus::Complete,
            issues: vec![],
        }),
    });
    let change = ChangeRequest {
        reason: "Retained selected supported knowledge and map".into(),
        writes: vec![Put {
            id: note.clone(),
            expected_version: Some(1),
            archived: false,
            data: RecordData::Note(selected_note),
        }],
        inputs: vec![],
    };
    let operation = scope
        .store
        .lock()
        .unwrap()
        .allocate_operation(&format!("{}:proposal:retain-map", scope.run))
        .unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "read",
                "read_source",
                json!({"reference":request.sources[0].reference}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "prepare",
                "prepare_changes",
                json!({"request_key":"retain-map","request":change}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "apply",
                "apply_changes",
                json!({"operation":operation.as_str()}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text(
                "Retained the useful map; the source has a detected omitted chart.",
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let answer = run_model(
        model.clone(),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(matches!(answer.terminal, AiTerminal::Completed));
    assert_eq!(model.request_count(), 4);
    let first = prepare(
        &scope,
        PrepareArgs {
            request_key: "retain-map".into(),
            request: change.clone(),
        },
    )
    .unwrap();
    let again = prepare(
        &scope,
        PrepareArgs {
            request_key: "retain-map".into(),
            request: change,
        },
    )
    .unwrap();
    assert_eq!(first, again);
    let receipt = apply(
        &scope,
        ApplyArgs {
            operation: operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(
        receipt,
        apply(
            &scope,
            ApplyArgs {
                operation: operation.as_str().into()
            }
        )
        .unwrap()
    );
    drop(scope);
    let store = Store::open(&request.data_dir).unwrap();
    let records = store.records().unwrap();
    let assets: Vec<_> = records
        .iter()
        .filter_map(|record| {
            if let RecordData::Asset(asset) = &record.data {
                Some((record, asset))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(assets.len(), 1, "unselected figure must not be retained");
    assert_eq!(assets[0].1.bytes, request.sources[0].assets[0].bytes);
    assert_eq!(assets[0].1.note, note);
    let RecordData::Note(note) = store.note(&note).unwrap().unwrap().data else {
        panic!("expected note")
    };
    assert!(
        note.markdown
            .contains(&format!("](brn-asset://{})", assets[0].0.id))
    );
    assert!(!note.markdown.contains("brn-transient-asset://"));
    let import = note.import.unwrap();
    assert!(!import.full_note);
    let report = import.report.unwrap();
    assert_eq!(report.summary(), "Detected omissions · Partial");
    assert_eq!(report.issues[1].kind, ImportIssueKind::Unverified);
    let source = records
        .iter()
        .find_map(|record| {
            if let RecordData::Source(source) = &record.data {
                Some(source)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(source.report.as_ref(), Some(&report));
    assert!(source.gaps[0].starts_with("Detected omission"));
    assert!(
        !serde_json::to_string(&records)
            .unwrap()
            .contains("EXCLUDED_RAW_CANARY_FIGURES_68219")
    );
}

#[test]
fn damaged_figure_identity_and_unknown_transient_link_refuse_without_publication() {
    let (_root, mut request, note) = super::tests::fixture();
    let mut bad = source();
    bad.assets[0].sha256[0] ^= 1;
    request.sources.push(bad);
    assert!(begin(&request).is_err());
    request.sources[0] = source();
    let (scope, _) = begin(&request).unwrap();
    let result = prepare(
        &scope,
        PrepareArgs {
            request_key: "unknown-map".into(),
            request: ChangeRequest {
                reason: "bad host reference".into(),
                writes: vec![Put {
                    id: note.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "bad",
                        "![Unknown](brn-transient-asset://0/99)",
                    )),
                }],
                inputs: vec![],
            },
        },
    );
    assert!(result.is_err());
    let store = scope.store.lock().unwrap();
    assert!(
        store
            .records()
            .unwrap()
            .iter()
            .all(|record| !matches!(record.data, RecordData::Asset(_)))
    );
    assert!(
        matches!(store.note(&note).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Monday")
    );
}

#[test]
fn changed_source_identity_refuses_old_transient_figure_before_publication() {
    let (_root, mut request, note) = super::tests::fixture();
    request.sources.push(source());
    let (scope, _) = begin(&request).unwrap();
    {
        let mut store = scope.store.lock().unwrap();
        let current = store
            .records()
            .unwrap()
            .into_iter()
            .find(|record| matches!(record.data, RecordData::Source(_)))
            .unwrap();
        let RecordData::Source(mut source) = current.data else {
            panic!()
        };
        source.external_version = Some("concurrent-new-source-content".into());
        let operation = store.allocate_operation("observed changed source").unwrap();
        let owner = HostAuthority::owner("bounded synthetic source change");
        store
            .prepare_owner(
                &operation,
                &ChangeRequest {
                    reason: "Source changed".into(),
                    writes: vec![Put {
                        id: current.id,
                        expected_version: Some(current.version),
                        archived: false,
                        data: RecordData::Source(source),
                    }],
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        assert!(matches!(
            store.apply(&operation, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }
    assert!(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "stale-map".into(),
                request: ChangeRequest {
                    reason: "Old map".into(),
                    writes: vec![Put {
                        id: note,
                        expected_version: Some(1),
                        archived: false,
                        data: RecordData::Note(Note::working(
                            "Stale map",
                            "![Map](brn-transient-asset://0/0)"
                        ))
                    }],
                    inputs: vec![]
                }
            }
        )
        .is_err()
    );
    assert!(
        scope
            .store
            .lock()
            .unwrap()
            .records()
            .unwrap()
            .iter()
            .all(|record| !matches!(record.data, RecordData::Asset(_)))
    );
}

fn change_source(store: &mut Store, key: &str) -> Record {
    let current = store
        .records()
        .unwrap()
        .into_iter()
        .find(|record| matches!(record.data, RecordData::Source(_)))
        .unwrap();
    let RecordData::Source(mut source) = current.data.clone() else {
        panic!()
    };
    source.external_version = Some(key.into());
    let operation = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic new observation");
    store
        .prepare_owner(
            &operation,
            &ChangeRequest {
                reason: "New source observation".into(),
                writes: vec![Put {
                    id: current.id.clone(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Source(source),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&operation, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    store.record(&current.id).unwrap().unwrap()
}

#[test]
fn source_changed_after_prepare_refuses_runtime_and_core_owner_apply_atomically() {
    let (_root, mut request, note) = super::tests::fixture();
    request.sources.push(source());
    let (scope, _) = begin(&request).unwrap();
    super::tests::read_all(&scope);
    let prepared: Prepared = serde_json::from_value(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "race-map".into(),
                request: ChangeRequest {
                    reason: "Selected map".into(),
                    writes: vec![Put {
                        id: note.clone(),
                        expected_version: Some(1),
                        archived: false,
                        data: RecordData::Note(Note::working(
                            "Map",
                            "![Map](brn-transient-asset://0/0)",
                        )),
                    }],
                    inputs: vec![],
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    let source_id = {
        // A separate connection represents concurrent local intake, outside the
        // runtime's Mutex. No dependent note existed when this update committed.
        let mut concurrent = Store::open(&request.data_dir).unwrap();
        change_source(&mut concurrent, "changed-between-prepare-and-apply").id
    };
    let result = apply(
        &scope,
        ApplyArgs {
            operation: prepared.operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(result["outcome"], "Stale");
    assert_eq!(result["records"], json!([source_id]));
    let mut store = scope.store.lock().unwrap();
    // Even skipping the runtime comparison cannot bypass the SQLite writer check.
    assert!(
        matches!(store.apply(&prepared.operation, &HostAuthority::owner("reviewed stale source proposal")).unwrap(), ApplyOutcome::Stale { records } if records.contains(&source_id))
    );
    assert!(store.receipt(&prepared.operation).unwrap().is_none());
    assert!(
        store
            .records()
            .unwrap()
            .iter()
            .all(|record| !matches!(record.data, RecordData::Asset(_)))
    );
    assert!(
        matches!(store.note(&note).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Monday")
    );
}

#[test]
fn committed_figure_receipt_replays_after_later_source_change() {
    let (_root, mut request, note) = super::tests::fixture();
    request.sources.push(source());
    let (scope, _) = begin(&request).unwrap();
    super::tests::read_all(&scope);
    let prepared: Prepared = serde_json::from_value(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "committed-map".into(),
                request: ChangeRequest {
                    reason: "Selected map".into(),
                    writes: vec![Put {
                        id: note,
                        expected_version: Some(1),
                        archived: false,
                        data: RecordData::Note(Note::working(
                            "Map",
                            "![Map](brn-transient-asset://0/0)",
                        )),
                    }],
                    inputs: vec![],
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    let result = apply(
        &scope,
        ApplyArgs {
            operation: prepared.operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(result["outcome"], "Applied");
    change_source(
        &mut scope.store.lock().unwrap(),
        "source-changed-after-committed-map",
    );
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into()
            }
        )
        .unwrap()["receipt"],
        result["receipt"]
    );
}

#[test]
fn model_cannot_forge_host_source_report_or_fingerprint_in_same_change_group() {
    let (_root, mut request, note) = super::tests::fixture();
    request.sources.push(source());
    let (scope, _) = begin(&request).unwrap();
    let source = scope
        .store
        .lock()
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .find(|record| matches!(record.data, RecordData::Source(_)))
        .unwrap();
    for kind in 0..6 {
        let RecordData::Source(mut forged) = source.data.clone() else {
            panic!()
        };
        match kind {
            0 => {
                forged.report = Some(ImportReport {
                    status: ImportStatus::Complete,
                    issues: vec![],
                })
            }
            1 => forged.report = None,
            2 => forged.external_version = Some("invented-complete-fingerprint".into()),
            3 => forged.gaps.clear(),
            4 => forged.outcome = "Complete".into(),
            _ => forged.locator = "synthetic://different-input".into(),
        }
        assert!(
            prepare(
                &scope,
                PrepareArgs {
                    request_key: format!("forged-source-{kind}"),
                    request: ChangeRequest {
                        reason: "Invented source completeness".into(),
                        writes: vec![
                            Put {
                                id: source.id.clone(),
                                expected_version: Some(source.version),
                                archived: false,
                                data: RecordData::Source(forged)
                            },
                            Put {
                                id: note.clone(),
                                expected_version: Some(1),
                                archived: false,
                                data: RecordData::Note(Note::working(
                                    "Forged",
                                    "![Map](brn-transient-asset://0/0)"
                                ))
                            }
                        ],
                        inputs: vec![]
                    }
                }
            )
            .is_err()
        );
    }
    assert_eq!(
        scope
            .store
            .lock()
            .unwrap()
            .record(&source.id)
            .unwrap()
            .unwrap(),
        source
    );
    let RecordData::Source(mut duplicate) = source.data.clone() else {
        panic!()
    };
    duplicate.report = None;
    duplicate.gaps.clear();
    duplicate.outcome = "Complete".into();
    assert!(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "duplicate-host-source".into(),
                request: ChangeRequest {
                    reason: "Fake parallel completeness".into(),
                    writes: vec![Put {
                        id: "new:0".into(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Source(duplicate)
                    }],
                    inputs: vec![]
                }
            }
        )
        .is_err()
    );
    let mut forged = match source.data.clone() {
        RecordData::Source(source) => source,
        _ => panic!(),
    };
    forged.label = "Useful public source label".into();
    let prepared: Prepared = serde_json::from_value(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "source-label".into(),
                request: ChangeRequest {
                    reason: "Useful source label".into(),
                    writes: vec![Put {
                        id: source.id.clone(),
                        expected_version: Some(source.version),
                        archived: false,
                        data: RecordData::Source(forged),
                    }],
                    inputs: vec![],
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into()
            }
        )
        .unwrap()["outcome"],
        "Applied"
    );
    assert!(
        matches!(scope.store.lock().unwrap().record(&source.id).unwrap().unwrap().data, RecordData::Source(updated) if updated.label == "Useful public source label" && updated.report == request.sources[0].report)
    );
}

#[test]
fn selected_complete_second_source_cannot_hide_first_source_detected_omission() {
    let (_root, mut request, note) = super::tests::fixture();
    let first = source();
    let mut second = source();
    second.reference = "synthetic://z-checked-source".into();
    second.report = Some(ImportReport {
        status: ImportStatus::Complete,
        issues: vec![],
    });
    request.sources = vec![first, second];
    let (scope, _) = begin(&request).unwrap();
    super::tests::read_all(&scope);
    let prepared: Prepared = serde_json::from_value(prepare(&scope, PrepareArgs { request_key: "multiple-maps".into(), request: ChangeRequest { reason: "Selected knowledge from both sources".into(), writes: vec![Put { id: note.clone(), expected_version: Some(1), archived: false, data: RecordData::Note(Note::working("Two sources", "![First](brn-transient-asset://0/0)\n![Second](brn-transient-asset://1/0)")) }], inputs: vec![] } }).unwrap()).unwrap();
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into()
            }
        )
        .unwrap()["outcome"],
        "Applied"
    );
    let store = scope.store.lock().unwrap();
    let RecordData::Note(note) = store.record(&note).unwrap().unwrap().data else {
        panic!()
    };
    let report = note.import.unwrap().report.unwrap();
    assert_eq!(report.summary(), "Detected omissions · Partial");
    assert!(
        report.issues[0]
            .locator
            .starts_with("synthetic://meaningful-figures")
    );
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(|record| matches!(record.data, RecordData::Asset(_)))
            .count(),
        2
    );
}

#[test]
fn action_source_input_guard_checks_used_source_without_blocking_unused_source() {
    for changed_used in [false, true] {
        let (_root, mut request, _) = super::tests::fixture();
        let mut unused = source();
        unused.reference = "synthetic://z-unused-source".into();
        request.sources = vec![source(), unused];
        let (scope, _) = begin(&request).unwrap();
        let records = scope.store.lock().unwrap().records().unwrap();
        let used = records.iter().find(|record| matches!(&record.data, RecordData::Source(source) if source.locator == request.sources[0].reference)).unwrap();
        let unused = records.iter().find(|record| matches!(&record.data, RecordData::Source(source) if source.locator == request.sources[1].reference)).unwrap();
        let prepared: Prepared = serde_json::from_value(
            prepare(
                &scope,
                PrepareArgs {
                    request_key: "source-action".into(),
                    request: ChangeRequest {
                        reason: "Suggested action based on used source".into(),
                        writes: vec![Put {
                            id: "new:0".into(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Action(Action {
                                kept_outside_project: false,
                                chat: None,
                                description: "Inspect the qualified map".into(),
                                state: ActionState::Suggested,
                                internal: false,
                                evidence: None,
                                due: None,
                            }),
                        }],
                        inputs: vec![RevisionInput {
                            expectation: InputExpectation::Current,
                            record: used.id.clone(),
                            version: used.version,
                        }],
                    },
                },
            )
            .unwrap(),
        )
        .unwrap();
        let changed = if changed_used { used } else { unused };
        let RecordData::Source(mut changed_source) = changed.data.clone() else {
            panic!()
        };
        changed_source.external_version = Some("new-source-content".into());
        {
            let mut concurrent = Store::open(&request.data_dir).unwrap();
            let update = concurrent
                .allocate_operation("concurrent-used-or-unused")
                .unwrap();
            let owner = HostAuthority::owner("synthetic observation");
            concurrent
                .prepare_owner(
                    &update,
                    &ChangeRequest {
                        reason: "Observed source changed".into(),
                        writes: vec![Put {
                            id: changed.id.clone(),
                            expected_version: Some(changed.version),
                            archived: false,
                            data: RecordData::Source(changed_source),
                        }],
                        inputs: vec![],
                    },
                    &owner,
                )
                .unwrap();
            assert!(matches!(
                concurrent.apply(&update, &owner).unwrap(),
                ApplyOutcome::Applied(_)
            ));
        }
        let outcome = apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into(),
            },
        )
        .unwrap();
        assert_eq!(
            outcome["outcome"],
            if changed_used { "Stale" } else { "Applied" }
        );
        assert_eq!(
            scope
                .store
                .lock()
                .unwrap()
                .record(&prepared.operation.creation_id(0))
                .unwrap()
                .is_some(),
            !changed_used
        );
    }
}

#[test]
fn fingerprint_only_host_source_cannot_be_rewritten_by_model_without_report() {
    let (_root, mut request, _) = super::tests::fixture();
    let mut supplied = source();
    supplied.report = None;
    request.sources.push(supplied);
    let (scope, _) = begin(&request).unwrap();
    let source = scope
        .store
        .lock()
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .find(|record| matches!(record.data, RecordData::Source(_)))
        .unwrap();
    let RecordData::Source(mut proposed) = source.data.clone() else {
        panic!()
    };
    proposed.external_version = Some("invented-fingerprint-without-report".into());
    assert!(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "forged-fingerprint-only".into(),
                request: ChangeRequest {
                    reason: "Invented fingerprint".into(),
                    writes: vec![Put {
                        id: source.id.clone(),
                        expected_version: Some(source.version),
                        archived: false,
                        data: RecordData::Source(proposed)
                    }],
                    inputs: vec![]
                }
            }
        )
        .is_err()
    );
    assert_eq!(
        scope
            .store
            .lock()
            .unwrap()
            .record(&source.id)
            .unwrap()
            .unwrap(),
        source
    );
    let RecordData::Source(duplicate) = source.data.clone() else {
        panic!()
    };
    assert!(
        prepare(
            &scope,
            PrepareArgs {
                request_key: "duplicate-fingerprint-only".into(),
                request: ChangeRequest {
                    reason: "Duplicate host evidence".into(),
                    writes: vec![Put {
                        id: "new:0".into(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Source(duplicate)
                    }],
                    inputs: vec![]
                }
            }
        )
        .is_err()
    );
}
