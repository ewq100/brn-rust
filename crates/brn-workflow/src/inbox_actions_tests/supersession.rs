//! Exact paired consequences through the real owned worker callback.
use super::link_tests::{relationships, target};
use super::*;
use crate::{library::KnowledgeScope, proposals::NoteChange};

fn current_paths(w: &AppWorker, scope: KnowledgeScope) -> Vec<String> {
    let AppEvent::Notes(page) = reply(
        w,
        AppCommand::ScopedNotes {
            scope,
            folder: None,
            cursor: None,
        },
    ) else {
        panic!("scoped list")
    };
    page.notes.into_iter().map(|n| n.path).collect()
}

#[test]
fn inbox_supersession_exact_pair_preserves_history_and_rebuilds_then_undoes() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (previous_id, previous) = target(&w, "previous.md", false);
    let mut k = knowledge(&source);
    k.supersedes = Some("previous.md".into());
    *script.lock().unwrap() = vec![Step::Knowledge(k.clone())];
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert!(results(&turn)[0].get("ok").is_some(), "{}", turn.answer);
    let record = analysis(&w, r.id).proposals.remove(0);
    assert_eq!(record.draft.changes.len(), 2);
    assert_eq!(
        record.draft.sources,
        vec![source.source.source.clone(), previous.source.clone()]
    );
    let binding = record.draft.inbox_knowledge.as_ref().unwrap();
    assert_eq!(binding.supersedes.as_ref().unwrap().note_id, previous_id);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        previous.text.as_bytes()
    );
    assert!(!f.base.path().join("vault/color.md").exists());
    let history = brn_store::note_metadata::to_history(&previous.text).unwrap();
    assert!(
        matches!(&record.draft.changes[1], NoteChange::Replace {before, before_text, text, ..}
        if before == &previous.source.fingerprint && before_text == &previous.text && text == &history)
    );
    let text = record.draft.changes[0]
        .text()
        .unwrap()
        .replace("The team chose", "Reviewed: the team chose");
    let AppEvent::Proposal(edited) = reply(
        &w,
        AppCommand::EditProposal(ProposalEdit {
            expected: record.stamp(),
            title: "Reviewed supersession".into(),
            texts: vec![Some(text), Some(history.clone())],
            action_data: vec![],
        }),
    ) else {
        panic!("current edit")
    };
    assert!(matches!(
        reply(
            &w,
            AppCommand::EditProposal(ProposalEdit {
                expected: edited.stamp(),
                title: edited.draft.title.clone(),
                texts: vec![
                    Some(edited.draft.changes[0].text().unwrap().into()),
                    Some(history.clone() + "changed")
                ],
                action_data: vec![],
            })
        ),
        AppEvent::Failed(_)
    ));
    let applied = approved(&w, &edited);
    assert_eq!(applied.outcome, ApplyOutcome::Applied);
    assert!(current_paths(&w, KnowledgeScope::Current).contains(&k.path));
    assert!(!current_paths(&w, KnowledgeScope::Current).contains(&"previous.md".into()));
    assert!(current_paths(&w, KnowledgeScope::History).contains(&"previous.md".into()));
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        history.as_bytes()
    );
    let edges = relationships(&w, KnowledgeScope::All).edges;
    assert!(
        edges
            .iter()
            .any(|e| e.source.note_id.to_string() == k.note_id && e.target.note_id == previous_id)
    );
    retained_original(&w, &source);
    w.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("data/index.sqlite")).unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(relationships(&w, KnowledgeScope::All).edges, edges);
    let undo = crate::proposal_apply::UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: applied.operation_id,
        trash_member: None,
    };
    let AppEvent::ProposalApplied(undone) =
        reply_at(&w, undo.operation_id, AppCommand::UndoProposal(undo))
    else {
        panic!("undo")
    };
    assert_eq!(undone.outcome, ApplyOutcome::Applied);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        previous.text.as_bytes()
    );
    assert!(!f.base.path().join("vault/color.md").exists());
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_supersession_refuses_history_source_duplicate_and_identity_occupation() {
    for mode in 0..6 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let (_, previous) = target(&w, "previous.md", mode == 0);
        let mut k = knowledge(&source);
        k.supersedes = Some(
            if mode == 1 {
                "source.md"
            } else {
                "previous.md"
            }
            .into(),
        );
        if mode == 2 {
            k.source_paths.push("previous.md".into());
        }
        if mode == 3 {
            std::fs::write(f.base.path().join("vault/alias.md"), &previous.text).unwrap();
        }
        if mode == 4 {
            k.note_id = brn_store::note_identity::read(&previous.text)
                .unwrap()
                .unwrap()
                .to_string();
        }
        if mode == 5 {
            let old_id = brn_store::note_identity::read(&previous.text)
                .unwrap()
                .unwrap();
            k.text
                .push_str(&format!("\n[Context](brn://note/{old_id})\n```text"));
        }
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let r = semantic(&source);
        let turn = analyze(&w, &r).unwrap();
        assert!(
            results(&turn)[0].get("error").is_some(),
            "mode {mode}: {}",
            turn.answer
        );
        assert!(analysis(&w, r.id).proposals.is_empty());
        assert_eq!(
            std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
            previous.text.as_bytes()
        );
        retained_original(&w, &source);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn inbox_supersession_approval_refuses_stale_or_ambiguous_predecessor() {
    for alias in [false, true] {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let (_, previous) = target(&w, "previous.md", false);
        let mut k = knowledge(&source);
        k.supersedes = Some("previous.md".into());
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let r = semantic(&source);
        analyze(&w, &r).unwrap();
        let record = analysis(&w, r.id).proposals.remove(0);
        let path = f.base.path().join(if alias {
            "vault/alias.md"
        } else {
            "vault/previous.md"
        });
        let changed = if alias {
            previous.text.clone()
        } else {
            previous.text.clone() + "Later owner edit\r\n"
        };
        std::fs::write(&path, &changed).unwrap();
        let a = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(matches!(
            reply_at(&w, a.operation_id, AppCommand::ApproveProposal(a)),
            AppEvent::Failed(_)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
        assert!(!f.base.path().join("vault/color.md").exists());
        retained_original(&w, &source);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn inbox_supersession_approval_refuses_dirty_predecessor_and_hidden_history_link() {
    for hidden in [false, true] {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        target(&w, "previous.md", false);
        let mut k = knowledge(&source);
        k.supersedes = Some("previous.md".into());
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let r = semantic(&source);
        analyze(&w, &r).unwrap();
        let mut record = analysis(&w, r.id).proposals.remove(0);
        if hidden {
            let current = record.draft.changes[0].text().unwrap();
            let footer = current.find("\n\nPrevious version:").unwrap();
            let old_id = record
                .draft
                .inbox_knowledge
                .as_ref()
                .unwrap()
                .supersedes
                .as_ref()
                .unwrap()
                .note_id;
            let text = format!(
                "{}\n[Context](brn://note/{old_id})\n```text{}",
                &current[..footer],
                &current[footer..]
            );
            let AppEvent::Proposal(edited) = reply(
                &w,
                AppCommand::EditProposal(ProposalEdit {
                    expected: record.stamp(),
                    title: record.draft.title.clone(),
                    texts: vec![
                        Some(text),
                        Some(record.draft.changes[1].text().unwrap().into()),
                    ],
                    action_data: vec![],
                }),
            ) else {
                panic!("edit hidden relation")
            };
            record = edited;
        } else {
            let AppEvent::Editor(view) = reply(&w, AppCommand::OpenEditor("previous.md".into()))
            else {
                panic!("open editor")
            };
            assert!(matches!(
                reply(
                    &w,
                    AppCommand::RecoverEditor(crate::editor::EditRequest {
                        path: "previous.md".into(),
                        expected: view.record.stamp,
                        generation: 1,
                        text: view.record.text + "Unfinished owner change\r\n",
                    })
                ),
                AppEvent::EditorRecovered(_)
            ));
        }
        let a = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(matches!(
            reply_at(&w, a.operation_id, AppCommand::ApproveProposal(a)),
            AppEvent::Failed(_)
        ));
        assert!(!f.base.path().join("vault/color.md").exists());
        retained_original(&w, &source);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}
