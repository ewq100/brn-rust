use super::*;
use brn_workflow::{app::AppConfig, app_worker::AppWorker};
use std::{collections::VecDeque, fs, os::unix::fs::PermissionsExt, time::Duration};

fn start_worker(owner: &tempfile::TempDir) -> AppWorker {
    let worker = AppWorker::start(
        owner.path().join("data"),
        AppConfig {
            vault_root: Some(owner.path().join("vault")),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    worker
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    worker.submit(command.0, command.1).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == command.0 {
            return event;
        }
    }
}
fn settle(worker: &AppWorker, state: &mut AiState, commands: Vec<(Uuid, AppCommand)>) {
    let mut queue = VecDeque::from(commands);
    while let Some(command) = queue.pop_front() {
        let (id, event) = reply(worker, command);
        queue.extend(state.apply(id, event));
    }
}
fn open(worker: &AppWorker) -> AiState {
    let mut state = AiState {
        ready: true,
        vault_bound: true,
        ..Default::default()
    };
    let command = state.open_inbox().unwrap();
    settle(worker, &mut state, vec![command]);
    let item = state.inbox_queue.page.as_ref().unwrap().entries[0]
        .item
        .capture
        .id;
    let command = state.select_inbox(item).unwrap();
    settle(worker, &mut state, vec![command]);
    state
}
fn inspect(worker: &AppWorker, state: &mut AiState) {
    let commands = state.inspect_inbox_copy();
    assert!(!commands.is_empty());
    settle(worker, state, commands);
    assert!(!state.inbox_copy_loading());
}
fn fixture() -> (tempfile::TempDir, AppWorker, AiState) {
    let (owner, _) = crate::ai::inbox_analysis_state_tests::source_fixture();
    let worker = start_worker(&owner);
    let mut state = open(&worker);
    inspect(&worker, &mut state);
    (owner, worker, state)
}

#[test]
fn actual_worker_confirmation_restart_restore_and_new_removal_bind_the_causal_parent() {
    let (owner, mut worker, mut state) = fixture();
    let item = state.inbox_queue.selected.as_ref().unwrap().item.clone();
    let original = owner
        .path()
        .join("data/inbox")
        .join(format!("{}.txt", item.capture.id));
    let exact = fs::read(&original).unwrap();
    let source = fs::read(owner.path().join("vault/source.md")).unwrap();
    assert!(state.begin_draft(None));
    state.draft.as_mut().unwrap().title = "Retained unrelated review õ".into();
    assert!(
        state.can_remove_inbox_copy(),
        "unfinished review is independent"
    );
    let capture = state.capture_inbox_copy_removal().unwrap();
    assert!(!state.inbox_copy_pending());
    assert_eq!(
        fs::read(&original).unwrap(),
        exact,
        "opening/cancelling admits no effect"
    );
    let operation = capture.request.id();
    let command = state.confirm_inbox_copy(&capture).unwrap();
    assert_eq!(command.0, operation);
    assert!(state.application_busy());
    assert!(state.confirm_inbox_copy(&capture).is_none());
    settle(&worker, &mut state, vec![command]);
    assert!(!state.inbox_copy_pending());
    assert!(!original.exists());
    assert!(
        state
            .inbox_copy
            .receipt
            .as_ref()
            .unwrap()
            .summary()
            .unwrap()
            .settled_at_ms
            .is_some()
    );
    assert_eq!(
        state.draft.as_ref().unwrap().title,
        "Retained unrelated review õ"
    );
    assert_eq!(
        fs::read(owner.path().join("vault/source.md")).unwrap(),
        source
    );
    worker.shutdown().unwrap();

    let mut worker = start_worker(&owner);
    let mut state = open(&worker);
    assert!(
        matches!(state.inbox_queue.selected.as_ref().unwrap().original,
        InboxOriginal::RemovedRetained { operation_id } if operation_id == operation)
    );
    inspect(&worker, &mut state);
    assert!(state.can_restore_inbox_copy());
    let restore = state.capture_inbox_copy_restore().unwrap();
    let command = state.confirm_inbox_copy(&restore).unwrap();
    settle(&worker, &mut state, vec![command]);
    assert_eq!(fs::read(&original).unwrap(), exact);
    let parent = state
        .inbox_copy
        .receipt
        .as_ref()
        .unwrap()
        .summary()
        .unwrap();
    inspect(&worker, &mut state);
    let next = state.capture_inbox_copy_removal().unwrap();
    let InboxCopyRequest::Remove(request) = next.request() else {
        panic!("remove");
    };
    assert_eq!(
        request.previous_restore,
        Some(InboxOriginalParent {
            operation_id: parent.operation_id,
            record_sha256: parent.record_sha256,
        })
    );
    let command = state.confirm_inbox_copy(&next).unwrap();
    settle(&worker, &mut state, vec![command]);
    assert!(!original.exists());
    assert_eq!(
        state
            .inbox_copy
            .receipt
            .as_ref()
            .unwrap()
            .summary()
            .unwrap()
            .operation_id,
        next.request.id()
    );
    inspect(&worker, &mut state);
    let history = state.inbox_copy.history.as_ref().unwrap();
    let older = history[0].operation_id;
    let latest = history.last().unwrap().operation_id;
    let old_command = state.inspect_inbox_copy_operation(older).unwrap();
    let (old_id, old_event) = reply(&worker, old_command);
    let latest_command = state.inspect_inbox_copy_operation(latest).unwrap();
    let (latest_id, latest_event) = reply(&worker, latest_command);
    state.apply(latest_id, latest_event);
    state.apply(old_id, old_event);
    assert_eq!(
        state
            .inbox_copy
            .operation
            .as_ref()
            .unwrap()
            .summary()
            .unwrap()
            .operation_id,
        latest,
        "late historical read cannot replace a newer inspected operation"
    );
    assert!(!state.pending.contains_key(&old_id));
    assert_eq!(
        fs::read(owner.path().join("vault/source.md")).unwrap(),
        source
    );
    assert_eq!(
        fs::read(owner.path().join("vault/untouched.md")).unwrap(),
        "\u{feff}Untouched õ\r\n".as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn stale_confirmation_and_late_reads_cannot_reopen_refresh_or_closed_selection() {
    let (_owner, mut worker, mut state) = fixture();
    let capture = state.capture_inbox_copy_removal().unwrap();
    let first = state.inspect_inbox_copy();
    assert!(
        state.confirm_inbox_copy(&capture).is_none(),
        "refresh invalidates modal"
    );
    let late: Vec<_> = first
        .into_iter()
        .map(|command| reply(&worker, command))
        .collect();
    state.close_inbox();
    for (id, event) in late {
        assert!(state.apply(id, event).is_empty());
    }
    assert!(state.inbox_copy.preview.is_none());
    assert!(state.inbox_copy.history.is_none());
    assert!(!state.can_remove_inbox_copy());
    assert!(state.capture_inbox_copy_removal().is_none());
    let command = state.open_inbox().unwrap();
    settle(&worker, &mut state, vec![command]);
    let command = state.select_inbox(capture.item().capture.id).unwrap();
    settle(&worker, &mut state, vec![command]);
    inspect(&worker, &mut state);
    assert!(state.can_remove_inbox_copy());
    assert!(
        state.confirm_inbox_copy(&capture).is_none(),
        "later same item is another view"
    );
    worker.shutdown().unwrap();
}

#[test]
fn forged_preview_and_wrong_mutation_reply_leave_exact_request_unacknowledged() {
    let (_owner, mut worker, mut state) = fixture();
    let commands = state.inspect_inbox_copy();
    for command in commands {
        let (id, event) = reply(&worker, command);
        match event {
            AppEvent::InboxRemovalPreview(mut preview) => {
                let good = preview.clone();
                preview.digest[0] ^= 1;
                assert!(
                    state
                        .apply(id, AppEvent::InboxRemovalPreview(preview))
                        .is_empty()
                );
                assert!(state.pending.contains_key(&id));
                assert!(!state.can_remove_inbox_copy());
                state.apply(id, AppEvent::InboxRemovalPreview(good));
            }
            other => {
                assert!(state.apply(id, other).is_empty());
            }
        }
    }
    let capture = state.capture_inbox_copy_removal().unwrap();
    let command = state.confirm_inbox_copy(&capture).unwrap();
    let (id, event) = reply(&worker, command);
    let AppEvent::InboxOriginalRemoved(record) = event else {
        panic!("receipt");
    };
    let mut forged = record.clone();
    forged.request.operation_id = Uuid::new_v4();
    assert!(
        state
            .apply(id, AppEvent::InboxOriginalRemoved(forged))
            .is_empty()
    );
    assert!(state.inbox_copy_pending());
    let followups = state.apply(id, AppEvent::InboxOriginalRemoved(record));
    settle(&worker, &mut state, followups);
    inspect(&worker, &mut state);
    let operation = state.inbox_copy.history.as_ref().unwrap()[0].operation_id;
    let command = state.inspect_inbox_copy_operation(operation).unwrap();
    let (id, event) = reply(&worker, command);
    let AppEvent::InboxOriginalRemoval { record, .. } = event else {
        panic!("lookup");
    };
    assert!(
        state
            .apply(
                id,
                AppEvent::InboxOriginalRestore {
                    operation_id: operation,
                    record: record.clone()
                }
            )
            .is_empty()
    );
    assert!(
        state.pending.contains_key(&id),
        "wrong event kind is not acknowledgement"
    );
    state.apply(
        id,
        AppEvent::InboxOriginalRemoval {
            operation_id: operation,
            record,
        },
    );
    worker.shutdown().unwrap();
}

#[test]
fn actual_stale_source_refusal_and_lost_ack_retry_keep_original_request_uuid() {
    let (owner, mut worker, mut state) = fixture();
    let capture = state.capture_inbox_copy_removal().unwrap();
    let item = capture.item().capture.id;
    let original = owner.path().join("data/inbox").join(format!("{item}.txt"));
    let exact = fs::read(&original).unwrap();
    let source_path = owner.path().join("vault/source.md");
    let saved = fs::read(&source_path).unwrap();
    fs::write(&source_path, "Synthetic changed Source").unwrap();
    let command = state.confirm_inbox_copy(&capture).unwrap();
    settle(&worker, &mut state, vec![command]);
    assert!(state.inbox_copy.error.is_some());
    assert!(state.inbox_copy.receipt.is_none());
    assert_eq!(fs::read(&original).unwrap(), exact);
    let retry = state.retry_inbox_copy().unwrap();
    assert_eq!(retry.0, capture.request.id());
    assert!(matches!(&retry.1, AppCommand::RemoveInboxOriginal(r)
        if InboxCopyRequest::Remove(r.clone()) == capture.request));
    settle(&worker, &mut state, vec![retry]);
    assert_eq!(fs::read(&original).unwrap(), exact);
    // Restore only this synthetic fixture's original Source inode/content.
    fs::write(&source_path, saved).unwrap();
    let retry = state.retry_inbox_copy().unwrap();
    let (id, event) = reply(&worker, retry);
    let AppEvent::InboxOriginalRemoved(receipt) = event else {
        panic!("remove");
    };
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "Synthetic lost acknowledgement",
        )),
    );
    let retry = state.retry_inbox_copy().unwrap();
    assert_eq!(retry.0, id);
    let (replayed, event) = reply(&worker, retry);
    let AppEvent::InboxOriginalRemoved(replay) = &event else {
        panic!("replay");
    };
    assert_eq!(
        serde_json::to_value(&receipt).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    let followups = state.apply(replayed, event);
    settle(&worker, &mut state, followups);
    assert!(!original.exists());
    worker.shutdown().unwrap();
}

#[test]
fn recorded_unsettled_intent_reopens_read_only_and_retries_exactly_after_restart() {
    let (owner, mut worker, mut state) = fixture();
    let capture = state.capture_inbox_copy_removal().unwrap();
    let item = capture.item().capture.id;
    let operation = capture.request.id();
    let directory = owner.path().join("data/inbox");
    let original = directory.join(format!("{item}.txt"));
    let exact = fs::read(&original).unwrap();
    let collision = directory.join(format!(".brn-inbox-removed-{operation}.original"));
    fs::write(&collision, "Owned synthetic occupied endpoint").unwrap();
    fs::set_permissions(&collision, fs::Permissions::from_mode(0o600)).unwrap();
    let command = state.confirm_inbox_copy(&capture).unwrap();
    settle(&worker, &mut state, vec![command]);
    assert!(state.inbox_copy.error.is_some());
    assert_eq!(fs::read(&original).unwrap(), exact);
    worker.shutdown().unwrap();
    fs::remove_file(&collision).unwrap(); // Only the exclusive synthetic collision.
    let mut worker = start_worker(&owner);
    let mut state = open(&worker);
    inspect(&worker, &mut state);
    assert_eq!(
        fs::read(&original).unwrap(),
        exact,
        "startup never performs the intent"
    );
    assert!(
        !state.can_remove_inbox_copy(),
        "do not mint another operation"
    );
    assert!(state.can_retry_inbox_copy());
    let retry = state.retry_inbox_copy().unwrap();
    assert_eq!(retry.0, operation);
    assert!(matches!(&retry.1, AppCommand::RemoveInboxOriginal(r)
        if InboxCopyRequest::Remove(r.clone()) == capture.request));
    settle(&worker, &mut state, vec![retry]);
    assert!(!original.exists());
    assert_eq!(
        state
            .inbox_copy
            .receipt
            .as_ref()
            .unwrap()
            .summary()
            .unwrap()
            .operation_id,
        operation
    );
    worker.shutdown().unwrap();
}
