use super::*;
use brn_workflow::{
    app::{App, AppConfig},
    proposal_rewrite::{RewriteSpec, RewriteStatus},
    proposals::{CommentTarget, DraftNoteChange, DraftRequest, ProposalEdit},
};

fn fixture() -> (tempfile::TempDir, App, ProposalRecord) {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    std::fs::create_dir(base.path().join("data")).unwrap();
    std::fs::create_dir(base.path().join("vault")).unwrap();
    let mut app = App::open(
        &base.path().join("data"),
        AppConfig {
            vault_root: Some(base.path().join("vault")),
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let record = app
        .create_proposal(&DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
        inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Full review 🧭".into(),
            changes: vec![DraftNoteChange::Create {
                path: "proposed.md".into(),
                text: "\u{feff}正文 λ\r\n".into(),
            }],
            sources: vec![],
        })
        .unwrap();
    (base, app, record)
}

fn job(request: &RewriteRequest, status: RewriteStatus) -> RewriteJob {
    RewriteJob {
        spec: RewriteSpec {
            id: request.id,
            expected: request.expected,
            provider: match request.selection.provider {
                Provider::Chatgpt => "chatgpt",
                Provider::Copilot => "copilot",
            }.into(),
            model: request.selection.model.clone(),
            effort: request.effort.as_str().into(),
        },
        capture_sha256: [1; 32],
        status,
        result_stamp: None,
        outcome_sha256: None,
        error_code: None,
        started_at_ms: 1,
        finished_at_ms: None,
    }
}

#[test]
fn loading_and_full_edit_correlate_without_losing_later_typing_or_admitting_comments() {
    let (base, mut app, record) = fixture();
    let mut state = ready();
    let (old, _) = state.open_review(record.draft.id).unwrap();
    let (current, _) = state.open_review(record.draft.id).unwrap();
    state.apply(old, AppEvent::Proposal(record.clone()));
    assert!(state.review.is_none());
    state.apply(current, AppEvent::Proposal(record.clone()));
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, "first λ\r\n".into(), Instant::now())
        .unwrap();
    assert!(!state.review_can_leave());
    assert!(!state.review_can_mutate());
    assert!(state.start_rewrite().is_none());
    assert!(state.reject_review().is_none());
    let comment = ReviewComment {
        id: Uuid::new_v4(),
        text: "temporary".into(),
        target: CommentTarget::Proposal,
    };
    assert!(state.review_comment(comment.clone(), false).is_none());
    let (id, AppCommand::EditProposal(edit)) = state.recover_review().unwrap() else {
        panic!("edit");
    };
    let first = app.edit_proposal(&edit).unwrap();
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, "latest 🧭\r\n".into(), Instant::now())
        .unwrap();
    state.apply(Uuid::new_v4(), AppEvent::Proposal(first.clone()));
    assert!(state.review.as_ref().unwrap().pending());
    state.apply(id, AppEvent::Proposal(first));
    assert_eq!(
        state.review.as_ref().unwrap().text(0),
        Some("latest 🧭\r\n")
    );
    assert!(!state.review_can_leave());
    let (id, AppCommand::EditProposal(edit)) = state.recover_review().unwrap() else {
        panic!("latest edit");
    };
    assert_eq!(edit.expected.version, record.version + 1);
    let latest = app.edit_proposal(&edit).unwrap();
    state.apply(id, AppEvent::Proposal(latest));
    let (id, AppCommand::AddProposalComment(request)) =
        state.review_comment(comment.clone(), false).unwrap()
    else {
        panic!("comment");
    };
    assert!(!state.review_editable());
    assert!(!state.review_can_leave());
    let commented = app.add_proposal_comment(&request).unwrap();
    state.apply(id, AppEvent::Proposal(commented));
    assert!(state.review_can_leave() && state.review_can_mutate());
    assert_eq!(
        state.review.as_ref().unwrap().record.comments,
        vec![comment]
    );
    assert!(!base.path().join("vault/proposed.md").exists());
}

#[test]
fn rewrite_freezes_choice_retries_early_stop_and_retains_local_conflict() {
    let (base, mut app, record) = fixture();
    let mut state = ready();
    state.review = Some(crate::review::ProposalReview::new(record.clone()));
    let (id, AppCommand::StartProposalRewrite(request)) = state.start_rewrite().unwrap() else {
        panic!("Rewrite");
    };
    assert_eq!(request.expected, record.stamp());
    assert_eq!(request.effort, ReasoningEffort::High);
    assert!(!state.can_ask());
    state.effort = Some(ReasoningEffort::Low);
    assert_eq!(
        state.rewrite.as_ref().unwrap().request.effort,
        ReasoningEffort::High
    );
    assert_eq!(state.stop_rewrite(), Some(id));
    assert!(
        state
            .stop_controls()
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::CancelTurn(actual) if *actual == id))
    );
    state.apply(
        id,
        AppEvent::Rewrite(RewriteEvent::ToolStarted {
            id,
            generation: request.generation + 1,
            name: "wrong".into(),
        }),
    );
    assert!(state.rewrite.as_ref().unwrap().tool.is_none());
    let commands = state.apply(
        id,
        AppEvent::Rewrite(RewriteEvent::Started {
            id,
            generation: request.generation,
            job: job(&request, RewriteStatus::Running),
        }),
    );
    assert!(
        commands
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::CancelTurn(actual) if *actual == id))
    );
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, "later local 🧭\r\n".into(), Instant::now())
        .unwrap();
    let rewritten = app
        .rewrite_proposal(&ProposalEdit {
            action_data: Vec::new(),
            expected: record.stamp(),
            title: record.draft.title.clone(),
            texts: vec![Some("AI suggestion λ\r\n".into())],
        })
        .unwrap();
    let mut completed = job(&request, RewriteStatus::Completed);
    completed.result_stamp = Some(rewritten.stamp());
    let commands = state.apply(
        id,
        AppEvent::Rewrite(RewriteEvent::Finished {
            id,
            generation: request.generation,
            job: completed,
        }),
    );
    let query = commands
        .iter()
        .find_map(|(id, command)| {
            matches!(command, AppCommand::Proposal(proposal) if *proposal == record.draft.id)
                .then_some(*id)
        })
        .unwrap();
    state.apply(query, AppEvent::Proposal(rewritten.clone()));
    let review = state.review.as_ref().unwrap();
    assert_eq!(review.text(0), Some("later local 🧭\r\n"));
    assert_eq!(review.observed, Some(rewritten.clone()));
    assert!(!state.review_can_leave() && !state.review_can_mutate());
    assert!(state.review.as_mut().unwrap().discard_local());
    assert_eq!(
        state.review.as_ref().unwrap().text(0),
        Some("AI suggestion λ\r\n")
    );
    assert!(state.review_can_mutate());
    assert!(state.rewrite.is_none());
    assert!(!base.path().join("vault/proposed.md").exists());
}

#[test]
fn rewrite_finishing_after_navigation_cannot_replace_another_review() {
    let (_base, mut app, record) = fixture();
    let mut state = ready();
    state.review = Some(crate::review::ProposalReview::new(record.clone()));
    let (id, AppCommand::StartProposalRewrite(request)) = state.start_rewrite().unwrap() else {
        panic!("Rewrite");
    };
    let other = app
        .create_proposal(&DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
        inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "another".into(),
            changes: vec![DraftNoteChange::Create {
                path: "other.md".into(),
                text: "other".into(),
            }],
            sources: vec![],
        })
        .unwrap();
    let (query, _) = state.open_review(other.draft.id).unwrap();
    state.apply(query, AppEvent::Proposal(other.clone()));
    let commands = state.apply(
        id,
        AppEvent::Rewrite(RewriteEvent::Finished {
            id,
            generation: request.generation,
            job: job(&request, RewriteStatus::Interrupted),
        }),
    );
    assert!(
        commands
            .iter()
            .all(|(_, command)| !matches!(command, AppCommand::Proposal(_)))
    );
    assert_eq!(state.review.as_ref().unwrap().record, other);
    assert!(state.rewrite.is_none());
}

#[test]
fn mutation_failure_preserves_review_and_storage_failure_blocks_fresh_rewrite() {
    let (_base, _app, record) = fixture();
    let mut state = ready();
    state.review = Some(crate::review::ProposalReview::new(record.clone()));
    let (id, _) = state.reject_review().unwrap();
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "synthetic storage refusal",
        )),
    );
    assert_eq!(state.review.as_ref().unwrap().record, record);
    assert!(!state.review_can_mutate());
    assert!(state.review.as_mut().unwrap().retry());
    let (id, AppCommand::StartProposalRewrite(request)) = state.start_rewrite().unwrap() else {
        panic!("Rewrite");
    };
    state.apply(
        id,
        AppEvent::Rewrite(RewriteEvent::PersistenceFailed {
            id,
            generation: request.generation,
            error: brn_workflow::WorkflowError::msg("synthetic finalization failure"),
        }),
    );
    assert!(state.rewrite.is_none());
    assert!(state.rewrite_storage_failed);
    assert!(state.start_rewrite().is_none());
    assert_eq!(state.review.as_ref().unwrap().record, record);
    assert!(state.notice.contains("not acknowledged"));
}

#[test]
fn failed_old_refresh_cannot_poison_the_newer_review() {
    let (_base, mut app, record) = fixture();
    let mut state = ready();
    state.review = Some(crate::review::ProposalReview::new(record));
    let (old, _) = state.refresh_review().unwrap();
    let other = app
        .create_proposal(&DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
        inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "other review".into(),
            changes: vec![DraftNoteChange::Create {
                path: "other.md".into(),
                text: "other bytes".into(),
            }],
            sources: vec![],
        })
        .unwrap();
    let (current, _) = state.open_review(other.draft.id).unwrap();
    state.apply(current, AppEvent::Proposal(other.clone()));
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old refresh refused")),
    );
    assert_eq!(state.review.as_ref().unwrap().record, other);
    assert!(state.review.as_ref().unwrap().error.is_none());
    assert!(state.review_can_mutate());
}


#[test]
fn action_rewrite_full_refresh_preserves_later_invalid_raw_fields_and_immutable_capture() {
    let record = crate::review::action_tests::fixture();
    let mut state = ready(); state.review = Some(crate::review::ProposalReview::new(record.clone()));
    crate::ai::session_state_tests::acknowledge_legacy_review(&mut state);
    state.vault_bound = false; assert!(state.start_rewrite().is_none()); state.vault_bound = true;
    let (id,AppCommand::StartProposalRewrite(request)) = state.start_rewrite().unwrap() else { panic!("Action Rewrite") };
    assert_eq!(request.expected,record.stamp());
    let mut edit = ProposalEdit { expected:record.stamp(),title:record.draft.title.clone(),texts:record.draft.changes.iter().map(|c|c.text().map(str::to_owned)).collect(),action_data:record.draft.action_changes.iter().map(|c|c.data().clone()).collect() };
    edit.action_data[1].description = "Full AI Action suggestion õ\r\n".into();
    let rewritten = crate::review::action_tests::reply(&record,&edit);
    let mut fields = state.review.as_ref().unwrap().action_fields()[0].clone(); fields.values[3] = "late-incomplete-uuid".into();
    state.review.as_mut().unwrap().edit_action_fields(0,fields.clone(),Instant::now()).unwrap();
    let mut completed = job(&request,RewriteStatus::Completed); completed.result_stamp = Some(rewritten.stamp());
    let commands = state.apply(id,AppEvent::Rewrite(RewriteEvent::Finished { id,generation:request.generation,job:completed }));
    let query = commands.iter().find_map(|(id,command)|matches!(command,AppCommand::Proposal(proposal) if *proposal==record.draft.id).then_some(*id)).unwrap();
    state.apply(query,AppEvent::Proposal(rewritten.clone()));
    let review = state.review.as_ref().unwrap();
    assert_eq!(review.action_fields()[0],fields); assert_eq!(review.observed,Some(rewritten.clone()));
    assert!(!review.can_leave() && !review.can_mutate());
    let copy:serde_json::Value = serde_json::from_str(&review.copy_local().unwrap()).unwrap();
    assert_eq!(copy["local_action_fields"][0]["values"][3],"late-incomplete-uuid");
    assert!(state.review.as_mut().unwrap().discard_local());
    assert_eq!(state.review.as_ref().unwrap().record,rewritten);
    assert_eq!(state.review.as_ref().unwrap().action_data()[1].description,edit.action_data[1].description);
    assert!(state.can_rewrite());
}


#[test]
fn actual_canonical_rewrite_job_keys_are_accepted_but_each_misbound_reply_stays_pending() {
    for provider in [Provider::Chatgpt,Provider::Copilot] {
        let mut state = ready(); state.selection.as_mut().unwrap().provider = provider;
        state.review = Some(crate::review::ProposalReview::new(crate::review::action_tests::fixture()));
        crate::ai::session_state_tests::acknowledge_legacy_review(&mut state);
        let (id,AppCommand::StartProposalRewrite(request)) = state.start_rewrite().unwrap() else { panic!("Rewrite") };
        let actual = job(&request,RewriteStatus::Running);
        for case in 0..6 {
            let mut wrong = actual.clone();
            match case {
                0 => wrong.spec.id = Uuid::new_v4(),
                1 => wrong.spec.expected.id = Uuid::new_v4(),
                2 => wrong.spec.expected.version += 1,
                3 => wrong.spec.model.push_str("-other"),
                4 => wrong.spec.effort = "low".into(),
                _ => wrong.spec.provider = provider_name(provider).into(),
            }
            assert!(state.apply(id,AppEvent::Rewrite(RewriteEvent::Started { id,generation:request.generation,job:wrong })).is_empty());
            assert!(state.rewrite.as_ref().unwrap().job.is_none());
        }
        state.apply(id,AppEvent::Rewrite(RewriteEvent::Started { id,generation:request.generation,job:actual.clone() }));
        assert_eq!(state.rewrite.as_ref().unwrap().job,Some(actual));
        assert_eq!(state.stop_rewrite(),Some(id));
        let commands = state.apply(id,AppEvent::Rewrite(RewriteEvent::Finished { id,generation:request.generation,job:job(&request,RewriteStatus::Interrupted) }));
        assert!(state.rewrite.is_none());
        assert!(commands.iter().any(|(_,command)|matches!(command,AppCommand::Proposal(proposal) if *proposal==request.expected.id)));
    }
}
