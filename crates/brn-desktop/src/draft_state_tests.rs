use super::*;
use crate::draft::DraftKind;
use brn_workflow::{
    ErrorKind, MAX_NOTE_BYTES, WorkTurnStatus,
    app::App,
    proposals::{NoteChange, ProposalEdit, ProposalSource},
};

const FULL: &str = "\u{feff}---\r\ncustom: λ\r\n---\r\n正文 日本語 õ🦀\nlast\r";
const LATER: &str = "\u{feff}Later typed Ελληνικά\r\nλ\r\n";

fn files(fixture: &Fixture) -> Vec<(String, Vec<u8>, u64)> {
    let mut entries = fs::read_dir(fixture.vault())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
                entry.metadata().unwrap().ino(),
            )
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn edit(state: &mut AiState, title: &str, path: &str, text: &str, kind: DraftKind) {
    state
        .draft
        .as_mut()
        .unwrap()
        .edit(title.into(), path.into(), text.into(), kind);
}

fn load_source(worker: &AppWorker, state: &mut AiState) -> ProposalSource {
    let command = state.draft_source().unwrap();
    let (id, event) = reply(worker, command);
    assert!(matches!(event, AppEvent::ProposalSource(_)));
    assert!(state.apply(id, event).is_empty());
    state
        .draft
        .as_ref()
        .unwrap()
        .source
        .as_ref()
        .unwrap()
        .clone()
}

#[test]
fn creation_correlates_immutable_record_and_keeps_later_typing_without_vault_effects() {
    let fixture = Fixture::new();
    fs::write(fixture.vault().join("existing.md"), FULL).unwrap();
    let mut worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_draft(None));
    edit(
        &mut state,
        "\u{feff}Full title 日本語\r\n",
        "new.md",
        FULL,
        DraftKind::Create,
    );
    let before = files(&fixture);
    let command = state.create_draft().unwrap();
    let outer = command.0;
    let AppCommand::CreateProposal(submitted) = &command.1 else {
        panic!("typed creation");
    };
    let submitted = submitted.clone();
    assert_eq!(
        submitted.changes[0],
        DraftNoteChange::Create {
            path: "new.md".into(),
            text: FULL.into()
        }
    );
    assert_critical(&state);
    assert!(!state.discard_draft());
    assert!(!state.separate_draft());
    assert!(!state.begin_draft(None));
    let (id, AppEvent::Proposal(created)) = reply(&worker, command) else {
        panic!("created record");
    };
    assert_eq!(id, outer);
    assert!(
        state
            .apply(Uuid::new_v4(), AppEvent::Proposal(created.clone()))
            .is_empty()
    );
    assert!(state.apply(outer, AppEvent::Proposals(vec![])).is_empty());
    let mut wrong_records = vec![];
    let mut wrong = created.clone();
    wrong.draft.id = Uuid::new_v4();
    wrong_records.push(wrong);
    let mut wrong = created.clone();
    wrong.draft.group_id = Some(Uuid::new_v4());
    wrong_records.push(wrong);
    let mut wrong = created.clone();
    wrong.draft.session_id = Some(Uuid::new_v4());
    wrong_records.push(wrong);
    let mut wrong = created.clone();
    if let NoteChange::Create { path, .. } = &mut wrong.draft.changes[0] {
        *path = "another.md".into();
    }
    wrong_records.push(wrong);
    let mut wrong = created.clone();
    wrong.version = 0;
    wrong_records.push(wrong);
    for wrong in wrong_records {
        assert!(state.apply(outer, AppEvent::Proposal(wrong)).is_empty());
        assert!(state.pending.contains_key(&outer));
        assert!(state.draft.as_ref().unwrap().pending);
        assert_critical(&state);
    }
    edit(
        &mut state,
        "Later title λ\r\n",
        "new.md",
        LATER,
        DraftKind::Create,
    );
    let newer_generation = state.draft.as_ref().unwrap().generation;
    let followups = state.apply(outer, AppEvent::Proposal(created.clone()));
    drain_reads(&worker, &mut state, followups);
    assert!(!state.application_busy());
    let form = state.draft.as_ref().unwrap();
    assert!(!form.pending);
    assert_eq!(form.text.as_bytes(), LATER.as_bytes());
    assert_eq!(form.title, "Later title λ\r\n");
    assert_eq!(form.submitted.as_ref().unwrap().request, submitted);
    let (acknowledged_generation, result) = form.result.as_ref().unwrap();
    assert!(*acknowledged_generation < newer_generation);
    assert_eq!(result, &created);
    assert_eq!(result.draft.changes[0].text(), Some(FULL));
    assert!(!form.can_leave());
    assert!(!state.review_can_leave());
    assert!(!state.begin_draft(None));
    assert_eq!(files(&fixture), before);
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert_eq!(record(&worker, submitted.id), created);
    assert_eq!(files(&fixture), before);
    worker.shutdown().unwrap();
}

#[test]
fn replay_acknowledges_actual_current_edited_review_instead_of_original_version_one() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_draft(None));
    edit(&mut state, "Initial", "new.md", FULL, DraftKind::Create);
    let command = state.create_draft().unwrap();
    let outer = command.0;
    let AppCommand::CreateProposal(initial_request) = &command.1 else {
        panic!("creation");
    };
    let initial_request = initial_request.clone();
    let (_, AppEvent::Proposal(initial)) = reply(&worker, command) else {
        panic!("initial record");
    };
    assert_eq!(initial.version, 1);
    let (_, AppEvent::Proposal(edited)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::EditProposal(ProposalEdit {
                expected: initial.stamp(),
                title: "Later stored full review 日本語\r\n".into(),
                texts: vec![Some(LATER.into())],
            }),
        ),
    ) else {
        panic!("later persisted review");
    };
    assert_eq!(edited.version, 2);
    let (_, AppEvent::Proposal(replayed)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(initial_request.clone()),
        ),
    ) else {
        panic!("creation replay");
    };
    assert_eq!(replayed, edited);
    let followups = state.apply(outer, AppEvent::Proposal(replayed.clone()));
    drain_reads(&worker, &mut state, followups);
    let form = state.draft.as_ref().unwrap();
    assert_eq!(form.result.as_ref().unwrap().1, edited);
    assert_eq!(form.text, FULL); // Retained input is not the returned authoritative review.
    assert!(form.can_leave());
    assert_eq!(state.last_draft_request.as_ref(), Some(&initial_request));
    assert!(files(&fixture).is_empty());
    assert!(state.discard_draft());
    open(&worker, &mut state, edited.draft.id);
    assert_eq!(state.review.as_ref().unwrap().text(0), Some(LATER));
    worker.shutdown().unwrap();
}

#[test]
fn source_path_type_generation_and_same_path_reload_cannot_replace_newer_capture_or_body() {
    let fixture = Fixture::new();
    fs::write(fixture.vault().join("a.md"), FULL).unwrap();
    fs::write(fixture.vault().join("b.md"), "Other before λ\r\n").unwrap();
    let mut worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_draft(None));
    edit(&mut state, "Seed", "a.md", LATER, DraftKind::Replace);
    let old = state.draft_source().unwrap();
    let outer = old.0;
    let (_, AppEvent::ProposalSource(capture_a)) = reply(&worker, old) else {
        panic!("source A");
    };
    assert!(
        state
            .apply(Uuid::new_v4(), AppEvent::ProposalSource(capture_a.clone()))
            .is_empty()
    );
    assert!(state.apply(outer, AppEvent::Proposals(vec![])).is_empty());
    let mut wrong = capture_a.clone();
    wrong.source.path = "b.md".into();
    assert!(
        state
            .apply(outer, AppEvent::ProposalSource(wrong))
            .is_empty()
    );
    assert!(state.pending.contains_key(&outer));
    assert!(state.draft.as_ref().unwrap().source.is_none());
    assert!(state.create_draft().is_none()); // A current source lookup is unacknowledged.

    let older_binding = state.draft.as_ref().unwrap().binding_generation;
    edit(&mut state, "Seed", "b.md", LATER, DraftKind::Trash);
    assert!(state.draft.as_ref().unwrap().binding_generation > older_binding);
    let current_b = load_source(&worker, &mut state);
    state.apply(outer, AppEvent::ProposalSource(capture_a));
    state.apply(
        outer,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old lookup failure")),
    );
    assert_eq!(
        state.draft.as_ref().unwrap().source.as_ref(),
        Some(&current_b)
    );
    assert_eq!(state.draft.as_ref().unwrap().text, LATER);

    let older = state.draft_source().unwrap();
    let (older_id, older_reply) = reply(&worker, older);
    fs::write(fixture.vault().join("b.md"), "New source bytes 日本語\r\n").unwrap();
    let newer = state.draft_source().unwrap();
    let (newer_id, newer_reply) = reply(&worker, newer);
    state.apply(newer_id, newer_reply);
    let newest = state
        .draft
        .as_ref()
        .unwrap()
        .source
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(newest.text, "New source bytes 日本語\r\n");
    state.apply(older_id, older_reply);
    state.apply(
        older_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "superseded same-path failure",
        )),
    );
    let form = state.draft.as_ref().unwrap();
    assert_eq!(form.source.as_ref(), Some(&newest));
    assert!(form.source_error.is_none());
    assert!(form.source_operation.is_none());
    assert_eq!(form.text, LATER);
    assert!(!state.pending.contains_key(&older_id));
    let (_, AppEvent::Proposals(records)) =
        reply(&worker, (Uuid::new_v4(), AppCommand::Proposals(None)))
    else {
        panic!("draft list");
    };
    assert!(records.is_empty());
    worker.shutdown().unwrap();
}

#[test]
fn stale_creation_retains_exact_proposal_request_and_payload_change_needs_explicit_separation() {
    for kind in [DraftKind::Create, DraftKind::Replace, DraftKind::Trash] {
        let fixture = Fixture::new();
        if kind != DraftKind::Create {
            fs::write(fixture.vault().join("target.md"), FULL).unwrap();
        }
        let mut worker = fixture.worker();
        let mut state = ready();
        assert!(state.begin_draft(None));
        edit(
            &mut state,
            "Initial full draft",
            "target.md",
            FULL,
            DraftKind::Create,
        );
        edit(&mut state, "Initial full draft", "target.md", FULL, kind);
        if kind != DraftKind::Create {
            load_source(&worker, &mut state);
        }
        if kind == DraftKind::Trash {
            // Switching modes retains the full body rather than silently omitting it
            // from the typed Trash request. Clearing it is a separate explicit edit.
            assert_eq!(state.draft.as_ref().unwrap().text, FULL);
            assert!(state.create_draft().is_none());
            assert!(state.draft.as_ref().unwrap().submitted.is_none());
            assert_eq!(state.draft.as_ref().unwrap().text, FULL);
            edit(&mut state, "Initial full draft", "target.md", "", kind);
        }
        fs::write(fixture.vault().join("target.md"), LATER).unwrap();
        let unchanged = files(&fixture);
        let command = state.create_draft().unwrap();
        let AppCommand::CreateProposal(original) = &command.1 else {
            panic!("typed initial request");
        };
        let original = original.clone();
        let (id, AppEvent::Failed(error)) = reply(&worker, command) else {
            panic!("stale destination/source refusal");
        };
        assert_eq!(error.kind, ErrorKind::ContextStale);
        let followups = state.apply(id, AppEvent::Failed(error));
        drain_reads(&worker, &mut state, followups);
        assert_eq!(state.last_draft_request.as_ref(), Some(&original));
        assert_eq!(
            state
                .draft
                .as_ref()
                .unwrap()
                .submitted
                .as_ref()
                .unwrap()
                .request,
            original
        );
        assert!(state.draft.as_ref().unwrap().error.is_some());
        assert_eq!(files(&fixture), unchanged);
        assert!(state.proposals.is_empty());

        // Transport correlation can be fresh; the durable proposal UUID/payload stays exact.
        let retry = state.create_draft().unwrap();
        assert!(matches!(&retry.1, AppCommand::CreateProposal(request) if request == &original));
        let (id, AppEvent::Failed(error)) = reply(&worker, retry) else {
            panic!("same identified request still stale");
        };
        let followups = state.apply(id, AppEvent::Failed(error));
        drain_reads(&worker, &mut state, followups);
        let changed_text = if kind == DraftKind::Trash {
            ""
        } else {
            "Changed full draft λ\r\n"
        };
        edit(
            &mut state,
            "Explicitly changed payload 日本語",
            "target.md",
            changed_text,
            kind,
        );
        assert!(state.create_draft().is_none());
        assert_eq!(state.last_draft_request.as_ref(), Some(&original));
        assert_eq!(
            state
                .draft
                .as_ref()
                .unwrap()
                .submitted
                .as_ref()
                .unwrap()
                .request
                .id,
            original.id
        );
        assert!(state.separate_draft());
        assert_ne!(state.draft.as_ref().unwrap().id, original.id);
        assert_eq!(state.draft.as_ref().unwrap().text, changed_text);
        if kind == DraftKind::Create {
            edit(
                &mut state,
                "Explicitly changed payload 日本語",
                "separate.md",
                changed_text,
                kind,
            );
        } else {
            load_source(&worker, &mut state);
        }
        let created = state.create_draft().unwrap();
        let (id, AppEvent::Proposal(result)) = reply(&worker, created) else {
            panic!("separate fresh proposal");
        };
        let followups = state.apply(id, AppEvent::Proposal(result));
        drain_reads(&worker, &mut state, followups);
        assert_eq!(state.proposals.len(), 1);
        assert!(state.draft.as_ref().unwrap().can_leave());
        assert_eq!(files(&fixture), unchanged);
        worker.shutdown().unwrap();
    }
}

#[test]
fn acknowledged_full_ai_answer_prefills_exact_session_and_failed_provisional_or_oversized_cannot() {
    let fixture = Fixture::new();
    let (completed, failed, oversized) = {
        let mut app = App::open(
            &fixture.0.path().join("data"),
            AppConfig {
                vault_root: Some(fixture.vault()),
                credentials_dir: Some(fixture.0.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        let mut seed = |status: WorkTurnStatus, answer: &str| {
            let id = Uuid::new_v4();
            app.work_store_mut()
                .begin_turn_with_effort(
                    id,
                    None,
                    "Write a full synthetic answer 日本語",
                    "chatgpt",
                    "gpt-5.5",
                    Some("high"),
                )
                .unwrap();
            app.work_store_mut()
                .finish_turn(
                    id,
                    status,
                    answer,
                    (status == WorkTurnStatus::Failed).then_some("other"),
                )
                .unwrap()
        };
        (
            seed(WorkTurnStatus::Completed, FULL),
            seed(WorkTurnStatus::Failed, "Failed partial λ\r\n"),
            seed(WorkTurnStatus::Completed, &"x".repeat(MAX_NOTE_BYTES + 1)),
        )
    };
    let mut worker = fixture.worker();
    let mut state = ready();
    for turn in [&failed, &oversized] {
        let command = state.navigate(Some(turn.conversation_id)).unwrap();
        let (id, event) = reply(&worker, command);
        state.apply(id, event);
        assert!(state.turns.iter().any(|stored| stored.id == turn.id));
        assert!(!state.begin_draft(Some(turn.id)));
        assert!(state.draft.is_none());
    }
    let command = state.navigate(Some(completed.conversation_id)).unwrap();
    let (id, event) = reply(&worker, command);
    state.apply(id, event);
    let provisional = state
        .ask("Unsubmitted synthetic provisional request".into())
        .unwrap();
    state.apply(
        provisional.id,
        AppEvent::Chat(ChatEvent::Text {
            id: provisional.id,
            generation: provisional.generation,
            text: "Raw provisional partial λ".into(),
        }),
    );
    assert!(!state.begin_draft(Some(provisional.id)));
    assert!(!state.begin_draft(Some(completed.id)));
    assert!(state.draft.is_none());
    state.active = None; // No provider work was submitted; end this presentation fixture.
    assert!(!state.begin_draft(Some(Uuid::new_v4())));
    assert!(state.begin_draft(Some(completed.id)));
    let form = state.draft.as_ref().unwrap();
    assert_eq!(form.text.as_bytes(), FULL.as_bytes());
    assert_eq!(form.session_id, Some(completed.conversation_id));
    assert_eq!(form.kind, DraftKind::Create);
    edit(
        &mut state,
        "Review full acknowledged answer",
        "answer.md",
        FULL,
        DraftKind::Create,
    );
    let command = state.create_draft().unwrap();
    let (id, AppEvent::Proposal(created)) = reply(&worker, command) else {
        panic!("answer review proposal");
    };
    assert_eq!(created.draft.session_id, Some(completed.conversation_id));
    assert_eq!(created.draft.changes[0].text(), Some(FULL));
    let followups = state.apply(id, AppEvent::Proposal(created.clone()));
    drain_reads(&worker, &mut state, followups);
    assert!(files(&fixture).is_empty());
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert_eq!(record(&worker, created.draft.id), created);
    assert!(files(&fixture).is_empty());
    worker.shutdown().unwrap();
}

#[test]
fn admitted_full_creation_drains_before_restart_and_replay_keeps_one_operational_record() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_draft(None));
    edit(
        &mut state,
        "Drain this complete draft",
        "drained.md",
        FULL,
        DraftKind::Create,
    );
    let (id, command) = state.create_draft().unwrap();
    let AppCommand::CreateProposal(request) = &command else {
        panic!("typed creation");
    };
    let request = request.clone();
    worker.submit(id, command).unwrap();
    worker.shutdown().unwrap();
    let (actual, AppEvent::Proposal(created)) = worker.try_event().unwrap() else {
        panic!("creation drains and returns its record");
    };
    assert_eq!(actual, id);
    assert!(worker.try_event().is_none());
    let followups = state.apply(id, AppEvent::Proposal(created.clone()));
    assert!(!state.application_busy());
    assert!(state.draft.as_ref().unwrap().can_leave());
    let mut worker = fixture.worker();
    drain_reads(&worker, &mut state, followups);
    let (_, AppEvent::Proposal(replayed)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::CreateProposal(request)),
    ) else {
        panic!("historical creation replay");
    };
    assert_eq!(replayed, created);
    assert_eq!(state.proposals, vec![created]);
    assert!(files(&fixture).is_empty());
    worker.shutdown().unwrap();
}
