use super::*;
use crate::draft::DraftKind;
const SOURCE: &str =
    "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n---\r\n正文 õ🦀\r\n";
const TARGET: &str =
    "---\nbrn_kind: source\nbrn_id: 22222222-2222-4222-8222-222222222222\n---\nArchived λ\n";
fn setup() -> (Fixture, AppWorker, AiState) {
    let fixture = Fixture::new();
    fs::create_dir(fixture.vault().join("archive")).unwrap();
    fs::write(fixture.vault().join("current.md"), SOURCE).unwrap();
    fs::write(fixture.vault().join("archive/source.md"), TARGET).unwrap();
    let worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_draft(None));
    state.draft.as_mut().unwrap().edit(
        "Link õ".into(),
        "current.md".into(),
        "".into(),
        DraftKind::Replace,
    );
    let command = state.draft_source().unwrap();
    let (id, event) = reply(&worker, command);
    state.apply(id, event);
    state.edit_link_input("archive/source.md".into(), "Õ [label] \u{1f9ed}".into());
    (fixture, worker, state)
}
fn target(worker: &AppWorker, state: &mut AiState) {
    let command = state
        .inspect_link_target()
        .expect("explicit target inspection");
    let (id, event) = reply(worker, command);
    state.apply(id, event);
    assert!(state.link_preparation.target.is_some());
}
#[test]
fn real_preparation_retains_two_full_bindings_without_admission_or_vault_write() {
    let (fixture, mut worker, mut state) = setup();
    target(&worker, &mut state);
    let before_id = state.draft.as_ref().unwrap().id;
    let command = state
        .prepare_link_draft()
        .expect("explicit read-only preparation");
    let (id, event) = reply(&worker, command);
    state.apply(id, event);
    let form = state.draft.as_ref().unwrap();
    assert_ne!(form.id, before_id);
    let request = form.request().unwrap();
    assert_eq!(request.sources.len(), 2);
    assert_eq!(form.source.as_ref().unwrap().text, SOURCE);
    assert!(form.text.as_bytes().starts_with(SOURCE.as_bytes()));
    assert!(
        form.text
            .contains("brn://note/22222222-2222-4222-8222-222222222222")
    );
    assert_eq!(
        fs::read(fixture.vault().join("current.md")).unwrap(),
        SOURCE.as_bytes()
    );
    let (_, AppEvent::Proposals(records)) =
        reply(&worker, (Uuid::new_v4(), AppCommand::Proposals(None)))
    else {
        panic!("review records");
    };
    assert!(records.is_empty());
    assert_eq!(
        fs::read_dir(fixture.0.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
    let command = state.create_draft().unwrap();
    let (id, event) = reply(&worker, command);
    let followups = state.apply(id, event);
    drain_reads(&worker, &mut state, followups);
    assert_eq!(
        state
            .draft
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .1
            .draft
            .sources,
        request.sources
    );
    assert_eq!(
        fs::read(fixture.vault().join("current.md")).unwrap(),
        SOURCE.as_bytes()
    );
    let proposal = state.draft.as_ref().unwrap().id;
    assert!(state.discard_draft());
    open(&worker, &mut state, proposal);
    let exact = state.capture_approval(false).unwrap();
    let command = state.confirm_approval(&exact).unwrap();
    let (id, event) = reply(&worker, command);
    assert!(
        matches!(&event, AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let followups = state.apply(id, event);
    drain_reads(&worker, &mut state, followups);
    assert_eq!(
        fs::read(fixture.vault().join("current.md")).unwrap(),
        match &request.changes[0] {
            DraftNoteChange::Replace { text, .. } => text.as_bytes(),
            _ => panic!("replacement"),
        }
    );
    assert_eq!(
        fs::read(fixture.vault().join("archive/source.md")).unwrap(),
        TARGET.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn target_wrong_operation_path_form_and_input_generations_cannot_replace_current_input() {
    let (_fixture, mut worker, mut state) = setup();
    let old = state.inspect_link_target().unwrap();
    let old_id = old.0;
    let (_, AppEvent::NoteLinks(event)) = reply(&worker, old) else {
        panic!("target links");
    };
    state.apply(Uuid::new_v4(), AppEvent::NoteLinks(event.clone()));
    assert!(state.link_preparation.target.is_none());
    let mut wrong = event.clone();
    wrong.source.path = "wrong.md".into();
    state.apply(old_id, AppEvent::NoteLinks(wrong));
    assert!(state.pending.contains_key(&old_id));
    state.edit_link_input("archive/source.md".into(), "Later label õ".into());
    let new = state.inspect_link_target().unwrap();
    let new_id = new.0;
    let (_, new_event) = reply(&worker, new);
    state.apply(new_id, new_event);
    let retained_target = state.link_preparation.target.clone();
    state.apply(old_id, AppEvent::NoteLinks(event));
    state.apply(
        old_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old failure")),
    );
    assert_eq!(state.link_preparation.target, retained_target);
    assert!(state.link_preparation.error.is_none());
    assert_eq!(state.link_preparation.label, "Later label õ");
    let stale = state.inspect_link_target().unwrap();
    let stale_id = stale.0;
    let (_, event) = reply(&worker, stale);
    let form = state.draft.as_mut().unwrap();
    form.edit(
        "New title λ".into(),
        form.path.clone(),
        form.text.clone(),
        form.kind,
    );
    state.apply(stale_id, event);
    assert!(state.link_preparation.target.is_none());
    assert!(state.link_preparation.operation.is_none());
    assert_eq!(state.draft.as_ref().unwrap().title, "New title λ");
    target(&worker, &mut state);
    let stale = state.prepare_link_draft().unwrap();
    let stale_id = stale.0;
    let (_, event) = reply(&worker, stale);
    let old_form = state.draft.as_ref().unwrap().id;
    assert!(state.separate_draft());
    let current_form = state.draft.as_ref().unwrap().id;
    assert_ne!(current_form, old_form);
    state.apply(stale_id, event);
    assert_eq!(state.draft.as_ref().unwrap().id, current_form);
    assert!(state.draft.as_ref().unwrap().prepared_request().is_none());
    worker.shutdown().unwrap();
}

#[test]
fn late_preparation_after_body_label_or_document_change_preserves_full_input_and_errors() {
    for change in 0..3 {
        let (_fixture, mut worker, mut state) = setup();
        target(&worker, &mut state);
        let command = state.prepare_link_draft().unwrap();
        let id = command.0;
        let (_, event) = reply(&worker, command);
        let form_id = state.draft.as_ref().unwrap().id;
        if change == 0 {
            let form = state.draft.as_mut().unwrap();
            form.edit(
                form.title.clone(),
                form.path.clone(),
                "\u{feff}Later typed õ\r\n".into(),
                form.kind,
            );
        } else if change == 1 {
            state.edit_link_input("archive/source.md".into(), "Later label λ".into());
        } else {
            state.note_generation += 1;
        }
        state.link_preparation.error = Some("Current input warning".into());
        state.apply(id, event);
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg(
                "stale preparation failure",
            )),
        );
        assert_eq!(state.draft.as_ref().unwrap().id, form_id);
        assert!(state.draft.as_ref().unwrap().prepared_request().is_none());
        assert_eq!(
            state.link_preparation.error.as_deref(),
            Some("Current input warning")
        );
        if change == 0 {
            assert_eq!(
                state.draft.as_ref().unwrap().text,
                "\u{feff}Later typed õ\r\n"
            );
        }
        worker.shutdown().unwrap();
    }
}

#[test]
fn authored_body_and_invalid_label_are_refused_without_losing_text_or_bindings() {
    let (_fixture, mut worker, mut state) = setup();
    target(&worker, &mut state);
    let form = state.draft.as_mut().unwrap();
    let original_source = form.source.clone();
    form.edit(
        form.title.clone(),
        form.path.clone(),
        "Full authored replacement õ\r\n".into(),
        form.kind,
    );
    assert!(state.prepare_link_draft().is_none());
    assert_eq!(
        state.draft.as_ref().unwrap().text,
        "Full authored replacement õ\r\n"
    );
    assert_eq!(state.draft.as_ref().unwrap().source, original_source);
    assert!(
        state
            .link_preparation
            .error
            .as_ref()
            .unwrap()
            .contains("retained")
    );
    let form = state.draft.as_mut().unwrap();
    form.edit(
        form.title.clone(),
        form.path.clone(),
        SOURCE.into(),
        form.kind,
    );
    state.edit_link_input("archive/source.md".into(), "Invalid\r\nlabel".into());
    assert!(state.prepare_link_draft().is_none());
    assert_eq!(state.link_preparation.label, "Invalid\r\nlabel");
    assert_eq!(state.draft.as_ref().unwrap().text, SOURCE);
    state.edit_link_input("archive/source.md".into(), "Valid õ".into());
    assert!(state.prepare_link_draft().is_some());
    worker.shutdown().unwrap();
}

#[test]
fn moved_archived_target_follows_uuid_and_same_byte_replaced_consumer_requires_recapture() {
    for replace_consumer in [false, true] {
        let (fixture, mut worker, mut state) = setup();
        target(&worker, &mut state);
        if replace_consumer {
            fs::rename(
                fixture.vault().join("current.md"),
                fixture.vault().join("old.txt"),
            )
            .unwrap();
            fs::write(fixture.vault().join("current.md"), SOURCE).unwrap();
        } else {
            fs::rename(
                fixture.vault().join("archive/source.md"),
                fixture.vault().join("archive/moved.md"),
            )
            .unwrap();
        }
        let old_id = state.draft.as_ref().unwrap().id;
        let command = state.prepare_link_draft().unwrap();
        let id = command.0;
        let (_, event) = reply(&worker, command);
        assert!(matches!(&event, AppEvent::NoteLinkDraft(_)));
        state.apply(id, event);
        let form = state.draft.as_ref().unwrap();
        if replace_consumer {
            assert_eq!(form.id, old_id);
            assert!(form.prepared_request().is_none());
            assert_eq!(form.text, "");
            assert!(state.link_preparation.operation.is_none());
            assert!(
                state
                    .link_preparation
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("proof changed")
            );
        } else {
            assert_ne!(form.id, old_id);
            assert!(
                form.request()
                    .unwrap()
                    .sources
                    .iter()
                    .any(|source| source.path == "archive/moved.md")
            );
        }
        assert_eq!(
            fs::read(fixture.vault().join("current.md")).unwrap(),
            SOURCE.as_bytes()
        );
        worker.shutdown().unwrap();
    }
}

#[test]
fn unmanaged_duplicate_and_changed_target_refuse_without_guessing_or_admission() {
    for problem in 0..3 {
        let (fixture, mut worker, mut state) = setup();
        if problem == 0 {
            fs::write(fixture.vault().join("archive/source.md"), "Unmanaged õ\r\n").unwrap();
        }
        if problem == 1 {
            fs::write(fixture.vault().join("archive/duplicate.md"), TARGET).unwrap();
        }
        target(&worker, &mut state);
        let id = state.draft.as_ref().unwrap().id;
        if problem < 2 {
            assert!(state.link_preparation.error.is_some());
            assert!(state.prepare_link_draft().is_none());
        } else {
            fs::write(
                fixture.vault().join("archive/source.md"),
                format!("{TARGET}Changed λ\n"),
            )
            .unwrap();
            let command = state.prepare_link_draft().unwrap();
            let (op, event) = reply(&worker, command);
            assert!(matches!(&event, AppEvent::Failed(_)));
            state.apply(op, event);
            assert!(state.link_preparation.error.is_some());
            assert!(state.link_preparation.operation.is_none());
        }
        assert_eq!(state.draft.as_ref().unwrap().id, id);
        assert_eq!(
            fs::read(fixture.vault().join("current.md")).unwrap(),
            SOURCE.as_bytes()
        );
        worker.shutdown().unwrap();
    }
}
