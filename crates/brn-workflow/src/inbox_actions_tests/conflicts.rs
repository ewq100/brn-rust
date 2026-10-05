//! Tentative conflicts through owned callbacks, complete lookup and exact closure.
use super::*;
use crate::findings::{
    CloseFindingRequest, FindingEvidenceOutcome, FindingState, NoteConflictPage,
    NoteConflictRequest,
};
use crate::library::KnowledgeScope;
use brn_ai::{ConflictArgs, ConflictQuote};

fn quote(_saved: &ProposalSource, wording: &str) -> ConflictQuote {
    ConflictQuote {
        quote: wording.into(),
        occurrence: None,
    }
}
fn conflict(source: &SourceFixture, other: &ProposalSource) -> ConflictArgs {
    ConflictArgs {
        title: "Unresolved opposing evidence õ".into(),
        summary: "The saved sources disagree; neither has been selected as authoritative.".into(),
        source_quote: quote(&source.source, "Blue õ 🦀"),
        other_path: other.source.path.clone(),
        other_quote: quote(other, "Known context."),
    }
}
fn lookup(
    worker: &AppWorker,
    path: &str,
    scope: KnowledgeScope,
    limit: usize,
    cursor: Option<crate::findings::NoteConflictCursor>,
) -> AppEvent {
    reply(
        worker,
        AppCommand::NoteConflicts(Box::new(NoteConflictRequest {
            path: path.into(),
            scope,
            limit,
            cursor,
        })),
    )
}
fn page(worker: &AppWorker, path: &str, scope: KnowledgeScope) -> NoteConflictPage {
    let AppEvent::NoteConflicts(page) = lookup(worker, path, scope, 100, None) else {
        panic!("conflict page")
    };
    *page
}

#[test]
fn inbox_conflict_keeps_exact_opposing_proofs_without_knowledge_writes_and_survives_restart() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (other_id, other) = link_tests::target(&w, "project.md", false);
    let input = conflict(&source, &other);
    *script.lock().unwrap() = vec![Step::Conflict(input.clone())];
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert!(results(&turn)[0].get("ok").is_some(), "{}", turn.answer);
    let all = analysis(&w, r.id);
    assert!(all.proposals.is_empty());
    assert!(all.needs_semantic_review);
    assert_eq!(all.findings.len(), 1);
    let record = &all.findings[0];
    assert_eq!(record.state, FindingState::Open);
    assert_eq!(record.draft.evidence[0].source, source.source.source);
    assert_eq!(record.draft.evidence[0].note_id, Some(source.note_id));
    assert_eq!(record.draft.evidence[1].source, other.source);
    assert_eq!(record.draft.evidence[1].note_id, Some(other_id));
    assert_eq!(
        record.draft.evidence[0].quote.as_ref().unwrap().quote,
        input.source_quote.quote
    );
    assert_eq!(
        record.draft.evidence[1].quote.as_ref().unwrap().quote,
        input.other_quote.quote
    );
    let current = page(&w, "project.md", KnowledgeScope::Current);
    assert_eq!(current.open_count, 1);
    assert_eq!(current.entries[0].record, *record);
    assert!(
        current.entries[0]
            .evidence
            .iter()
            .all(|e| e.outcome == FindingEvidenceOutcome::Unchanged)
    );
    assert!(matches!(
        lookup(&w, "source.md", KnowledgeScope::Current, 10, None),
        AppEvent::Failed(_)
    ));
    assert_eq!(
        page(&w, "source.md", KnowledgeScope::Source).entries[0].record,
        *record
    );
    assert_eq!(
        std::fs::read_to_string(f.base.path().join("vault/project.md")).unwrap(),
        other.text
    );
    assert_eq!(
        std::fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text
    );
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("data/index.sqlite")).unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(analysis(&w, r.id).findings, all.findings);
    assert_eq!(
        page(&w, "project.md", KnowledgeScope::Current).entries[0].record,
        *record
    );
    let close = CloseFindingRequest {
        expected: record.stamp(),
        state: FindingState::Dismissed,
    };
    let AppEvent::Finding(closed) = reply(&w, AppCommand::CloseFinding(close.clone())) else {
        panic!("closure")
    };
    assert_eq!(closed.draft, record.draft);
    assert_eq!(closed.version, record.version + 1);
    assert!(
        page(&w, "project.md", KnowledgeScope::Current)
            .entries
            .is_empty()
    );
    let AppEvent::Finding(replayed) = reply(&w, AppCommand::CloseFinding(close)) else {
        panic!("closure replay")
    };
    assert_eq!(replayed, closed);
    assert_eq!(analysis(&w, r.id).findings, vec![*closed]);
    assert_eq!(
        std::fs::read_to_string(f.base.path().join("vault/project.md")).unwrap(),
        other.text
    );
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_conflict_fresh_capture_refuses_stale_ambiguous_history_and_wrong_body_without_insert() {
    for mode in 0..10 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let path = "project.md";
        let (_, mut other) = link_tests::target(&w, path, mode == 3);
        if mode == 9 {
            other.text = other.text.replace(
                "brn_state: current\r\n",
                "brn_state: current\r\nbrn_provenance: not-json\r\n",
            );
            std::fs::write(f.base.path().join("vault/project.md"), &other.text).unwrap();
        }
        let mut input = conflict(&source, &other);
        match mode {
            0 => input.source_quote.quote = "Green õ 🦀".into(),
            1 => std::fs::write(
                f.base.path().join("vault/source.md"),
                source.source.text.clone() + "Changed",
            )
            .unwrap(),
            2 => std::fs::write(f.base.path().join("vault/alias.md"), &other.text).unwrap(),
            3 | 9 => {}
            8 => {
                std::fs::create_dir(f.base.path().join("vault/archive")).unwrap();
                std::fs::rename(
                    f.base.path().join("vault/project.md"),
                    f.base.path().join("vault/archive/project.md"),
                )
                .unwrap();
                input.other_path = "archive/project.md".into();
            }
            4 => {
                input.other_path = "source.md".into();
                input.other_quote = input.source_quote.clone();
            }
            5 => {
                input.other_quote = quote(&other, &other.text[..3]);
            }
            6 => input.other_quote.occurrence = Some(0),
            7 => std::fs::remove_file(f.base.path().join("vault/project.md")).unwrap(),
            _ => unreachable!(),
        }
        *script.lock().unwrap() = vec![Step::Conflict(input)];
        let r = semantic(&source);
        match analyze(&w, &r) {
            Ok(turn) => assert!(
                results(&turn)[0].get("error").is_some(),
                "mode {mode}: {}",
                turn.answer
            ),
            Err(_) => assert_eq!(mode, 1, "only changed selected Source may refuse admission"),
        }
        // Admission-refused jobs need not exist; the general queue must still
        // contain no partial record from any rejected input.
        let AppEvent::Findings(findings) = reply(&w, AppCommand::Findings(Default::default()))
        else {
            panic!("queue")
        };
        assert!(findings.entries.is_empty(), "mode {mode}");
        no_actions(&w);
        retained_original(&w, &source);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn inbox_conflict_original_callback_replay_preserves_later_closure_after_source_loss() {
    let f = Fixture::new();
    let input = Arc::new(Mutex::new(None::<ConflictArgs>));
    let script = input.clone();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let args = script.lock().unwrap().clone().unwrap();
        let created = created.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                created
                    .send(proposals.report_conflict(args.clone()).unwrap())
                    .unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let same = tool_reply(proposals.report_conflict(args.clone()));
                let mut changed = args.clone();
                changed.summary.push('!');
                let changed_summary = tool_reply(proposals.report_conflict(changed));
                let mut changed_occurrence = args;
                changed_occurrence.other_quote.occurrence = Some(1);
                vec![
                    same,
                    changed_summary,
                    tool_reply(proposals.report_conflict(changed_occurrence)),
                ]
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&out).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut w = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (_, other) = link_tests::target(&w, "project.md", false);
    let args = conflict(&source, &other);
    *input.lock().unwrap() = Some(args.clone());
    let r = semantic(&source);
    w.submit(r.id, AppCommand::AnalyzeInboxActions(Box::new(r.clone())))
        .unwrap();
    let record: crate::findings::FindingRecord =
        serde_json::from_value(ready.recv_timeout(Duration::from_secs(10)).unwrap()).unwrap();
    let AppEvent::Finding(closed) = reply(
        &w,
        AppCommand::CloseFinding(CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Resolved,
        }),
    ) else {
        panic!("close")
    };
    std::fs::remove_file(f.base.path().join("vault/source.md")).unwrap();
    std::fs::remove_file(f.base.path().join("vault/project.md")).unwrap();
    release.send(()).unwrap();
    let turn = finish(&w, &r).unwrap();
    let out = results(&turn);
    assert_eq!(out[0]["ok"], json!(closed));
    assert!(out[1].get("error").is_some());
    assert!(out[2].get("error").is_some());
    assert_eq!(analysis(&w, r.id).findings, vec![(*closed).clone()]);
    w.shutdown().unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(analysis(&w, r.id).findings, vec![*closed]);
    assert_eq!(json!(analyze(&w, &r).unwrap()), json!(turn));
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn conflict_lookup_keeps_changed_unavailable_and_ambiguous_sides_distinct_from_retained_quotes() {
    for mode in 0..3 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let (_, other) = link_tests::target(&w, "project.md", false);
        *script.lock().unwrap() = vec![Step::Conflict(conflict(&source, &other))];
        let r = semantic(&source);
        analyze(&w, &r).unwrap();
        let retained = analysis(&w, r.id).findings.remove(0);
        match mode {
            0 => std::fs::write(
                f.base.path().join("vault/project.md"),
                other.text.clone() + "Changed\r\n",
            )
            .unwrap(),
            1 => std::fs::remove_file(f.base.path().join("vault/project.md")).unwrap(),
            2 => std::fs::write(f.base.path().join("vault/alias.md"), &other.text).unwrap(),
            _ => unreachable!(),
        }
        let p = page(&w, "source.md", KnowledgeScope::Source);
        assert_eq!(p.entries[0].record, retained);
        assert_eq!(
            p.entries[0].evidence[0].outcome,
            FindingEvidenceOutcome::Unchanged
        );
        assert_eq!(
            p.entries[0].evidence[1].outcome,
            if mode == 0 {
                FindingEvidenceOutcome::Changed
            } else {
                FindingEvidenceOutcome::Unavailable
            }
        );
        if mode != 0 {
            assert!(matches!(
                lookup(&w, "project.md", KnowledgeScope::Current, 10, None),
                AppEvent::Failed(_)
            ));
        }
        no_actions(&w);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn ordinary_ask_conflict_reads_page_complete_records_and_keep_closed_anchor_and_query_binding() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let (shown, first_page) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let answer: crate::simple_worker_tests::AnswerHook = Arc::new(move |_, _, tools, _, _| {
        let shown = shown.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                let first = tools
                    .read_conflicts("project.md", ReadScope::Current, 1, None)
                    .unwrap();
                assert_eq!(first["entries"].as_array().unwrap().len(), 1);
                assert_eq!(first["open_count"], 3);
                let cursor = first["next_cursor"].as_str().unwrap().to_string();
                shown.send(first.clone()).unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                assert!(
                    tools
                        .read_conflicts("source.md", ReadScope::Source, 10, Some(&cursor))
                        .is_err()
                );
                assert!(
                    tools
                        .read_conflicts("project.md", ReadScope::All, 10, Some(&cursor))
                        .is_err()
                );
                assert!(
                    tools
                        .read_conflicts("project.md", ReadScope::Current, 0, None)
                        .is_err()
                );
                assert!(
                    tools
                        .read_conflicts("project.md", ReadScope::Current, 10, Some("not JSON"))
                        .is_err()
                );
                let mut invented: Value = serde_json::from_str(&cursor).unwrap();
                invented["before"] = json!(Uuid::new_v4());
                assert!(
                    tools
                        .read_conflicts(
                            "project.md",
                            ReadScope::Current,
                            10,
                            Some(&invented.to_string())
                        )
                        .is_err()
                );
                let rest = tools
                    .read_conflicts("project.md", ReadScope::Current, 100, Some(&cursor))
                    .unwrap();
                assert_eq!(rest["entries"].as_array().unwrap().len(), 2);
                assert_eq!(rest["open_count"], 2);
                assert!(rest["next_cursor"].is_null());
                json!({"first":first,"rest":rest})
            })
            .await
            .unwrap();
            AiAnswer {
                text: out.to_string(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let analysis_hook = scripted(script.clone());
    let combined: ProposalAnswerHook =
        Arc::new(move |ask, history, tools, proposals, cancel, emit| {
            if proposals.knowledge_enabled() {
                analysis_hook(ask, history, tools, proposals, cancel, emit)
            } else {
                drop(proposals);
                answer(ask, history, tools, cancel, emit)
            }
        });
    let mut w = f.start(Hooks {
        proposal_answer: Some(combined),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (_, other) = link_tests::target(&w, "project.md", false);
    *script.lock().unwrap() = (0..3)
        .map(|n| {
            let mut input = conflict(&source, &other);
            input.title.push_str(&format!(" {n}"));
            Step::Conflict(input)
        })
        .collect();
    let r = semantic(&source);
    analyze(&w, &r).unwrap();
    let captured = analysis(&w, r.id).findings;
    let ask = f.request();
    w.submit(ask.id, AppCommand::Ask(ask.clone())).unwrap();
    let first = first_page.recv_timeout(Duration::from_secs(10)).unwrap();
    let record: crate::findings::FindingRecord =
        serde_json::from_value(first["entries"][0]["record"].clone()).unwrap();
    let AppEvent::Finding(_) = reply(
        &w,
        AppCommand::CloseFinding(CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Dismissed,
        }),
    ) else {
        panic!("close page anchor")
    };
    release.send(()).unwrap();
    let answered = terminal(&w, ask.id);
    let out: Value = serde_json::from_str(&answered.answer).unwrap();
    let entries = out["rest"]["entries"].as_array().unwrap();
    for entry in entries {
        let retained: crate::findings::FindingRecord =
            serde_json::from_value(entry["record"].clone()).unwrap();
        assert!(captured.contains(&retained));
        assert_eq!(retained.draft.evidence.len(), 2);
    }
    let cursor: crate::findings::NoteConflictCursor =
        serde_json::from_str(first["next_cursor"].as_str().unwrap()).unwrap();
    std::fs::write(
        f.base.path().join("vault/project.md"),
        other.text.clone() + "Changed",
    )
    .unwrap();
    assert!(matches!(
        lookup(&w, "project.md", KnowledgeScope::Current, 100, Some(cursor)),
        AppEvent::Failed(_)
    ));
    assert_eq!(
        page(&w, "project.md", KnowledgeScope::Current).open_count,
        2
    );
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn conflict_remains_durable_when_owned_analysis_finishes_failed_or_interrupted() {
    for interrupted in [false, true] {
        let f = Fixture::new();
        let input = Arc::new(Mutex::new(None::<ConflictArgs>));
        let script = input.clone();
        let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
            let args = script.lock().unwrap().clone().unwrap();
            Box::pin(async move {
                tokio::task::spawn_blocking(move || proposals.report_conflict(args).unwrap())
                    .await
                    .unwrap();
                AiAnswer {
                    text: "Partial analysis: the conflict remains unresolved".into(),
                    terminal: if interrupted {
                        AiTerminal::Interrupted
                    } else {
                        AiTerminal::Failed(brn_ai::AiError::new(brn_ai::AiErrorKind::Other))
                    },
                }
            })
        });
        let mut w = f.start(Hooks {
            proposal_answer: Some(hook),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let (_, other) = link_tests::target(&w, "project.md", false);
        *input.lock().unwrap() = Some(conflict(&source, &other));
        let r = semantic(&source);
        let turn = analyze(&w, &r).unwrap();
        assert_eq!(
            turn.status,
            if interrupted {
                WorkTurnStatus::Interrupted
            } else {
                WorkTurnStatus::Failed
            }
        );
        let retained = analysis(&w, r.id).findings;
        assert_eq!(retained.len(), 1);
        w.shutdown().unwrap();
        let mut w = f.start(Hooks::default());
        assert_eq!(analysis(&w, r.id).findings, retained);
        assert_eq!(
            page(&w, "project.md", KnowledgeScope::Current).open_count,
            1
        );
        no_actions(&w);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn mixed_inbox_consequence_cap_counts_conflicts_and_preserves_replay_at_capacity() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (_, other) = link_tests::target(&w, "project.md", false);
    let original = conflict(&source, &other);
    let mut steps = vec![Step::Conflict(original.clone())];
    steps.extend((1..20).map(|n| {
        let mut input = conflict(&source, &other);
        input.title.push_str(&format!(" {n}"));
        Step::Conflict(input)
    }));
    steps.push(Step::Action(args()));
    steps.push(Step::Knowledge(knowledge(&source)));
    let mut beyond = conflict(&source, &other);
    beyond.title.push_str(" beyond cap");
    steps.push(Step::Conflict(beyond));
    steps.push(Step::Conflict(original));
    *script.lock().unwrap() = steps;
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    let out = results(&turn);
    assert!(out[..20].iter().all(|v| v.get("ok").is_some()));
    assert!(out[20..23].iter().all(|v| v.get("error").is_some()));
    assert_eq!(out[23], out[0]);
    let all = analysis(&w, r.id);
    assert_eq!(all.findings.len(), 20);
    assert!(all.proposals.is_empty());
    assert!(!f.base.path().join("vault/color.md").exists());
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn conflict_lookup_refuses_oversized_complete_pages_and_recovers_with_smaller_unclipped_pages() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let wording = "\u{1}".repeat(16 * 1024);
    let source = capture_source(&w, &wording);
    let (_, mut other) = link_tests::target(&w, "project.md", false);
    other.text.push_str(&wording);
    std::fs::write(f.base.path().join("vault/project.md"), &other.text).unwrap();
    let mut inputs = Vec::new();
    for n in 0..4 {
        inputs.push(ConflictArgs {
            title: format!("Whole retained conflicting quotations {n}"),
            summary: wording.clone(),
            source_quote: quote(&source.source, &wording),
            other_path: "project.md".into(),
            other_quote: quote(&other, &wording),
        });
    }
    *script.lock().unwrap() = inputs.iter().cloned().map(Step::Conflict).collect();
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert!(
        results(&turn).iter().all(|v| v.get("ok").is_some()),
        "{}",
        turn.answer
    );
    assert!(matches!(
        lookup(&w, "project.md", KnowledgeScope::Current, 4, None),
        AppEvent::Failed(_)
    ));
    let mut cursor = None;
    let mut seen = Vec::new();
    loop {
        let AppEvent::NoteConflicts(page) =
            lookup(&w, "project.md", KnowledgeScope::Current, 1, cursor)
        else {
            panic!("smaller complete page")
        };
        assert_eq!(page.open_count, 4);
        assert_eq!(page.entries.len(), 1);
        let record = &page.entries[0].record;
        assert_eq!(record.draft.summary, wording);
        assert!(
            record
                .draft
                .evidence
                .iter()
                .all(|e| e.quote.as_ref().unwrap().quote == wording)
        );
        seen.push(record.draft.request.id);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 4);
    no_actions(&w);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn conflict_lookup_refuses_malformed_managed_provenance_instead_of_claiming_no_conflicts() {
    let f = Fixture::new();
    let mut w = f.start(Hooks::default());
    let (_, other) = link_tests::target(&w, "project.md", false);
    let malformed = other.text.replace(
        "brn_state: current\r\n",
        "brn_state: current\r\nbrn_provenance: not-json\r\n",
    );
    std::fs::write(f.base.path().join("vault/project.md"), malformed).unwrap();
    assert!(
        matches!(
            lookup(&w, "project.md", KnowledgeScope::Current, 10, None),
            AppEvent::Failed(_)
        ),
        "malformed provenance must not appear as a successful empty lookup"
    );
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn conflict_selected_occurrences_have_rust_ranges_and_exact_intent_ids_within_the_owned_analysis() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\nBlue õ 🦀\r\n");
    let (_, mut other) = link_tests::target(&w, "project.md", false);
    other.text.push_str("Known context.\r\n");
    std::fs::write(f.base.path().join("vault/project.md"), &other.text).unwrap();
    let ambiguous = conflict(&source, &other);
    let mut first = ambiguous.clone();
    first.source_quote.occurrence = Some(2);
    first.other_quote.occurrence = Some(1);
    let mut second = first.clone();
    second.other_quote.occurrence = Some(2);
    *script.lock().unwrap() = vec![ambiguous, first.clone(), first.clone(), second.clone()]
        .into_iter()
        .map(Step::Conflict)
        .collect();
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    let out = results(&turn);
    assert_eq!(out[0]["error"], json!(AiErrorKind::QuoteAmbiguous));
    assert_eq!(out[1], out[2]);
    assert_ne!(
        out[1]["ok"]["draft"]["request"]["id"],
        out[3]["ok"]["draft"]["request"]["id"]
    );
    let all = analysis(&w, r.id);
    assert_eq!(all.findings.len(), 2);
    for record in &all.findings {
        let source_quote = record.draft.evidence[0].quote.as_ref().unwrap();
        assert_eq!(
            source_quote.start_byte,
            source.source.text.rfind("Blue õ 🦀").unwrap()
        );
        let other_quote = record.draft.evidence[1].quote.as_ref().unwrap();
        assert_eq!(
            other.text.get(other_quote.start_byte..other_quote.end_byte),
            Some("Known context.")
        );
    }
    *script.lock().unwrap() = vec![Step::Conflict(first)];
    let later = semantic(&source);
    analyze(&w, &later).unwrap();
    let new_record = analysis(&w, later.id).findings.remove(0);
    assert!(
        all.findings
            .iter()
            .all(|old| old.draft.request.id != new_record.draft.request.id)
    );
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}
