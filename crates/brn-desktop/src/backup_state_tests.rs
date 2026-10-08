use super::*;

fn status(path: &str, error: Option<&str>) -> BackupStatus {
    BackupStatus {
        latest_path: path.into(),
        completed_at_ms: None,
        retention_warning: Some("Prior copy could not be pruned".into()),
        last_error: error.map(str::to_owned),
    }
}

#[test]
fn ready_queues_one_correlated_backup_status_read_without_starting_provider_work() {
    let mut state = AiState::default();
    assert!(state.request_backup(true).is_none());
    let commands = state.apply(
        Uuid::nil(),
        AppEvent::Ready {
            vault_bound: false,
            model_installed: false,
        },
    );
    let queries: Vec<_> = commands
        .iter()
        .filter(|(_, command)| matches!(command, AppCommand::BackupStatus))
        .collect();
    assert_eq!(queries.len(), 1);
    let id = queries[0].0;
    assert!(!id.is_nil());
    assert!(matches!(
        state.pending.get(&id),
        Some(Pending::BackupStatus)
    ));
    assert!(state.request_backup(false).is_none());
    assert!(state.request_backup(true).is_none());
    state.apply(id, AppEvent::BackupStatus(status("startup.sqlite", None)));
    assert!(!state.backup_pending());
    assert_eq!(state.backup_status.unwrap().completed_at_ms, None);
}

#[test]
fn automatic_backup_failure_preserves_pending_editor_and_proposal_buffers() {
    let exact = "\u{feff}Unacknowledged 日本語 õ\r\n";
    let mut state = editing("saved");
    state.begin_draft(None);
    state.draft.as_mut().unwrap().title = "Retained proposal title".into();
    state.draft.as_mut().unwrap().text = exact.into();
    state
        .editor
        .as_mut()
        .unwrap()
        .edit(exact.into(), Instant::now())
        .unwrap();
    let (editor_id, _) = state.recover_editor().unwrap();
    let (backup_id, command) = state.request_backup(true).unwrap();
    assert!(matches!(command, AppCommand::CheckpointBackup));
    state.notice = "Retained unrelated failure".into();
    state.review_error = Some("Retained review error".into());
    state.note_error = Some("Retained note error".into());
    let failure = status("last-usable.sqlite", Some("disk full"));
    assert!(
        state
            .apply(Uuid::nil(), AppEvent::BackupStatus(failure.clone()))
            .is_empty()
    );
    assert_eq!(state.backup_status.as_ref(), Some(&failure));
    assert!(state.pending.contains_key(&editor_id));
    assert!(state.pending.contains_key(&backup_id));
    assert!(state.editor.as_ref().unwrap().pending());
    assert_eq!(
        state.editor.as_ref().unwrap().text.as_bytes(),
        exact.as_bytes()
    );
    assert_eq!(
        state.draft.as_ref().unwrap().text.as_bytes(),
        exact.as_bytes()
    );
    assert_eq!(
        state.draft.as_ref().unwrap().title,
        "Retained proposal title"
    );
    assert_eq!(state.notice, "Retained unrelated failure");
    assert_eq!(state.review_error.as_deref(), Some("Retained review error"));
    assert_eq!(state.note_error.as_deref(), Some("Retained note error"));
    // A typed backup acknowledgement with another operation's UUID cannot settle it.
    state.apply(
        editor_id,
        AppEvent::BackupStatus(status("foreign.sqlite", None)),
    );
    assert_eq!(state.backup_status.as_ref(), Some(&failure));
    assert!(state.pending.contains_key(&editor_id));
    assert!(state.pending.contains_key(&backup_id));
    state.apply(backup_id, AppEvent::BackupStatus(failure));
    assert!(!state.backup_pending());
    assert!(state.pending.contains_key(&editor_id));
}

#[test]
fn manual_backup_failures_keep_last_usable_status_and_require_matching_acknowledgement() {
    let mut state = ready();
    let previous = status("last-usable.sqlite", Some("previous attempt failed"));
    state.apply(Uuid::nil(), AppEvent::BackupStatus(previous.clone()));
    let (id, command) = state.request_backup(false).unwrap();
    assert!(matches!(command, AppCommand::BackupStatus));
    state.apply(
        Uuid::new_v4(),
        AppEvent::BackupStatus(status("foreign.sqlite", None)),
    );
    assert!(state.backup_pending());
    assert_eq!(state.backup_status.as_ref(), Some(&previous));
    state.notice = "Retained operation result".into();
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("lane closed")),
    );
    assert!(!state.backup_pending());
    assert_eq!(state.backup_status.as_ref(), Some(&previous));
    assert_eq!(state.backup_request_error.as_deref(), Some("lane closed"));
    assert_eq!(state.notice, "Retained operation result");
    let (next, _) = state.request_backup(true).unwrap();
    assert!(state.backup_request_error.is_none());
    // An old response must not settle this newly requested checkpoint.
    state.apply(id, AppEvent::BackupStatus(status("late.sqlite", None)));
    assert!(state.pending.contains_key(&next));
    assert_eq!(state.backup_status.as_ref(), Some(&previous));
    let mut latest = status("new-copy.sqlite", None);
    latest.completed_at_ms = Some(1234);
    state.apply(next, AppEvent::BackupStatus(latest.clone()));
    assert!(!state.backup_pending());
    assert_eq!(state.backup_status.as_ref(), Some(&latest));
}

#[test]
fn automatic_backup_updates_leave_review_recovery_and_later_typing_owned_by_review() {
    let (record, _) = crate::review::predecessor_tests::fixture();
    let mut state = ready();
    state.review = Some(crate::review::ProposalReview::new(record));
    let original = state.review.as_ref().unwrap().text(0).unwrap().to_owned();
    let earlier = format!("{original}\r\nEarlier owner edit õ");
    let later = format!("{original}\r\nLater owner edit 日本語");
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, earlier, Instant::now())
        .unwrap();
    let (edit, _) = state.recover_review().unwrap();
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, later.clone(), Instant::now())
        .unwrap();
    state.review.as_mut().unwrap().error = Some("Retained review warning".into());
    state.apply(
        Uuid::nil(),
        AppEvent::BackupStatus(status("last-usable.sqlite", Some("backup refused"))),
    );
    let review = state.review.as_ref().unwrap();
    assert_eq!(review.text(0), Some(later.as_str()));
    assert_eq!(review.error.as_deref(), Some("Retained review warning"));
    assert!(review.pending());
    assert!(state.pending.contains_key(&edit));
}
