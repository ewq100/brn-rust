//! Actual Text Source approval and whole completion, including retained FS recovery.
use super::*;
use crate::{
    action_completion::{PrepareSentCompletionRequest, SentCompletionPreview},
    inbox::{CaptureInboxRequest, InboxKind, InboxOriginal},
    inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
    proposals::DraftNoteChange,
};
use std::sync::atomic::AtomicBool;

const ACTUAL: &str = "\u{feff}Actual sent version: Tuesday, not Monday. õ 日本語 🦀\r\nPlease confirm the revised access time.\r\n";

fn ready(f: &Fixture) -> (SentCompletionPreview, crate::inbox::InboxItem) {
    let thread = Uuid::new_v4();
    fs::write(
        f.base.join("vault/thread.md"),
        format!("---\r\nbrn_id: {thread}\r\n---\r\n# Retained explicit thread\r\n"),
    )
    .unwrap();
    let ordinary = f.prepare_with_thread(Some(thread));
    let mut app = f.app();
    let editor = app.open_editor("a.md").unwrap();
    let input = proposal_input(&editor);
    let reply = app
        .create_proposal(&DraftRequest {
            title: "Reviewed planned reply, not sent evidence".into(),
            changes: vec![DraftNoteChange::Create {
                path: "planned-reply.md".into(),
                text: "# Original planned reply\r\nMonday.\r\n".into(),
            }],
            action_changes: vec![],
            ..input
        })
        .unwrap();
    app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: reply.stamp(),
    })
    .unwrap();
    assert_eq!(
        app.action(ordinary.before.origin.id).unwrap(),
        *ordinary.before
    );
    let item = app
        .capture_inbox(&CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Actual externally sent reply".into(),
            original_name: None,
            text: ACTUAL.into(),
        })
        .unwrap();
    let batch = Uuid::new_v4();
    app.process_inbox(&ProcessInboxRequest {
        limits: None,
        id: batch,
        items: vec![item.clone()],
    })
    .unwrap();
    app.advance_inbox_processing(batch, &AtomicBool::new(false))
        .unwrap();
    let draft = app
        .prepare_inbox_source(&InboxSourceRequest {
            candidate: InboxCandidateRequest {
                batch_id: batch,
                index: 0,
            },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "sent-source.md".into(),
            title: "Actual sent reply".into(),
        })
        .unwrap();
    let source = app.create_proposal(&draft).unwrap();
    app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: source.stamp(),
    })
    .unwrap();
    assert_eq!(
        app.action(ordinary.before.origin.id).unwrap(),
        *ordinary.before,
        "neither draft nor Source approval completes"
    );
    assert_eq!(source.draft.session_id, None);
    let request = PrepareSentCompletionRequest {
        before: ordinary.before,
        source_path: "sent-source.md".into(),
    };
    let preview = app.prepare_sent_action_completion(&request).unwrap();
    preview.validate_for(&request).unwrap();
    assert!(preview.source.text.contains(ACTUAL));
    assert!(!preview.source.text.contains("Monday.\r\n"));
    assert_eq!(
        app.action(preview.request.before.origin.id).unwrap(),
        *preview.request.before
    );
    fs::write(
        f.base.join("request.json"),
        serde_json::to_vec(&preview.request).unwrap(),
    )
    .unwrap();
    (preview, item)
}

#[test]
fn actual_sent_source_completes_once_and_replays_after_restart_and_source_edit_without_inference() {
    let f = Fixture::new();
    let (preview, item) = ready(&f);
    let source_before = fs::read(f.base.join("vault/sent-source.md")).unwrap();
    let mut app = f.app();
    let completion = app.complete_action(&preview.request).unwrap();
    completion.validate().unwrap();
    let mut data = preview.request.before.data.clone();
    data.state = ActionState::Completed;
    data.sources
        .push(preview.request.sent_source.as_ref().unwrap().note_id);
    assert_eq!(completion.after.data, data);
    assert_eq!(completion.after.origin, preview.request.before.origin);
    assert_eq!(app.complete_action(&preview.request).unwrap(), completion);
    assert_eq!(
        fs::read(f.base.join("vault/sent-source.md")).unwrap(),
        source_before
    );
    assert!(
        matches!(app.inbox.original(&item), InboxOriginal::Available { text } if text == ACTUAL)
    );
    drop(app);
    fs::write(
        f.base.join("vault/sent-source.md"),
        b"Owner later edit; preserve it.\r\n",
    )
    .unwrap();
    let mut app = f.app();
    assert_eq!(app.complete_action(&preview.request).unwrap(), completion);
    assert_eq!(
        fs::read(f.base.join("vault/sent-source.md")).unwrap(),
        b"Owner later edit; preserve it.\r\n"
    );
    let mut changed = preview.request.clone();
    changed
        .sent_source
        .as_mut()
        .unwrap()
        .source
        .fingerprint
        .sha256[0] ^= 1;
    assert_eq!(
        app.complete_action(&changed).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert!(app.store.conversations().unwrap().is_empty());
}

#[test]
fn fresh_sent_confirmation_refuses_changed_ambiguous_missing_source_without_completion_or_refresh()
{
    for condition in ["changed", "ambiguous", "missing"] {
        let f = Fixture::new();
        let (preview, _) = ready(&f);
        let source_path = f.base.join("vault/sent-source.md");
        let original = fs::read(&source_path).unwrap();
        match condition {
            "changed" => {
                let modified = fs::metadata(&source_path).unwrap().modified().unwrap();
                let changed = String::from_utf8(original.clone())
                    .unwrap()
                    .replacen("Tuesday", "Friday!", 1);
                assert_eq!(changed.len(), original.len());
                fs::write(&source_path, changed).unwrap();
                fs::File::options()
                    .write(true)
                    .open(&source_path)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_modified(modified))
                    .unwrap();
            }
            "ambiguous" => {
                fs::create_dir(f.base.join("vault/archive")).unwrap();
                fs::write(f.base.join("vault/archive/duplicate.md"), &original).unwrap();
            }
            "missing" => fs::remove_file(&source_path).unwrap(),
            _ => unreachable!(),
        }
        let mut app = f.app();
        assert!(
            app.complete_action(&preview.request).is_err(),
            "{condition}"
        );
        assert_eq!(
            app.action(preview.request.before.origin.id).unwrap(),
            *preview.request.before
        );
        assert_eq!(
            app.store.action_completion_for(&preview.request).unwrap(),
            None
        );
        assert!(!f.receipt_path(preview.request.operation_id).exists());
        assert!(
            app.prepare_sent_action_completion(&PrepareSentCompletionRequest {
                before: preview.request.before.clone(),
                source_path: preview.source.source.path.clone()
            })
            .is_err()
        );
        assert_eq!(app.store.proposals(None).unwrap().len(), 3);
    }
}

#[test]
fn sent_publication_failure_recovers_historical_binding_without_overwriting_later_source() {
    let f = Fixture::new();
    let (preview, _) = ready(&f);
    let mut app = f.app();
    FAILURE.with(|phase| phase.set(Some("published")));
    let failure = app.complete_action(&preview.request).unwrap_err();
    FAILURE.with(|phase| phase.set(None));
    assert_eq!(failure.kind, ErrorKind::SaveUncertain);
    assert!(app.current_evidence_blocked().unwrap());
    assert_eq!(
        app.store.action(preview.request.before.origin.id).unwrap(),
        Some(*preview.request.before.clone())
    );
    let receipt_bytes = fs::read(f.receipt_path(preview.request.operation_id)).unwrap();
    fs::write(
        f.base.join("vault/sent-source.md"),
        b"Later owner Source wording; never restore over it.\r\n",
    )
    .unwrap();
    let completion = app.complete_action(&preview.request).unwrap();
    assert_eq!(completion.request, preview.request);
    assert_eq!(completion.after.data.state, ActionState::Completed);
    assert!(!app.current_evidence_blocked().unwrap());
    assert_eq!(
        fs::read(f.receipt_path(preview.request.operation_id)).unwrap(),
        receipt_bytes
    );
    assert_eq!(
        fs::read(f.base.join("vault/sent-source.md")).unwrap(),
        b"Later owner Source wording; never restore over it.\r\n"
    );
}

#[test]
fn sent_crash_recovery_imports_source_journals_before_completion_without_resurrecting_source() {
    for phase in ["published", "settled"] {
        for database in ["current", "older", "fresh"] {
            let f = Fixture::new();
            let (preview, _) = ready(&f);
            fs::copy(f.base.join("data/brn.sqlite"), f.base.join("before.sqlite")).unwrap();
            let lock = crate::SUBPROCESS_FIXTURES.lock().unwrap();
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "action_completion::tests::crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("BRN_COMPLETE_TEST_BASE", &f.base)
                .env("BRN_COMPLETE_TEST_PHASE", phase)
                .output()
                .unwrap();
            drop(lock);
            assert_eq!(
                result.status.code(),
                Some(73),
                "{phase}/{database}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            fs::remove_file(f.base.join("vault/sent-source.md")).unwrap();
            if database != "current" {
                let data = f.base.join("data");
                for name in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
                    let path = data.join(name);
                    if path.exists() {
                        fs::remove_file(path).unwrap();
                    }
                }
                if database == "older" {
                    fs::copy(f.base.join("before.sqlite"), data.join("brn.sqlite")).unwrap();
                } else {
                    fs::remove_dir_all(data.join("backups")).unwrap();
                }
            }
            let mut app = f.app();
            let completion = app
                .store
                .action_completion_for(&preview.request)
                .unwrap()
                .unwrap();
            completion.validate().unwrap();
            assert_eq!(completion.request, preview.request);
            assert_eq!(app.complete_action(&preview.request).unwrap(), completion);
            assert_eq!(
                app.action(preview.request.before.origin.id).unwrap(),
                completion.after
            );
            assert!(
                !f.base.join("vault/sent-source.md").exists(),
                "historical proof cannot resurrect a removed Source"
            );
            assert!(
                app.store
                    .proposal(
                        preview
                            .request
                            .sent_source
                            .as_ref()
                            .unwrap()
                            .source_approval
                            .expected
                            .id
                    )
                    .unwrap()
                    .is_some()
            );
            assert!(app.store.conversations().unwrap().is_empty());
        }
    }
}

#[test]
fn sent_preview_rejects_missing_and_non_text_provenance_even_with_matching_full_hashes() {
    let f = Fixture::new();
    let (preview, _) = ready(&f);
    let request = PrepareSentCompletionRequest {
        before: preview.request.before.clone(),
        source_path: preview.source.source.path.clone(),
    };
    let line = preview
        .source
        .text
        .split_inclusive('\n')
        .find(|line| line.starts_with("brn_inbox_source: "))
        .unwrap()
        .to_owned();
    for kind in [None, Some("email")] {
        let mut changed = preview.clone();
        let replacement = if let Some(kind) = kind {
            let mut provenance: serde_json::Value =
                serde_json::from_str(line.strip_prefix("brn_inbox_source: ").unwrap().trim_end())
                    .unwrap();
            provenance["kind"] = serde_json::json!(kind);
            format!(
                "brn_inbox_source: {}\n",
                serde_json::to_string(&provenance).unwrap()
            )
        } else {
            String::new()
        };
        changed.source.text = changed.source.text.replacen(&line, &replacement, 1);
        changed.source.source.fingerprint.len = changed.source.text.len() as u64;
        changed.source.source.fingerprint.sha256 =
            brn_intake::digest(changed.source.text.as_bytes());
        changed.request.sent_source.as_mut().unwrap().source = changed.source.source.clone();
        // The witness is not an invalid JSON/hash case. Existing provenance
        // parsing accepts both absence and a syntactically valid Email profile.
        let provenance =
            brn_store::work::inbox_source::read_provenance(&changed.source.text).unwrap();
        assert_eq!(provenance.is_some(), kind.is_some());
        assert!(changed.validate_for(&request).is_err(), "kind={kind:?}");
    }
}

#[test]
fn sent_preview_refuses_changed_before_path_source_bytes_identity_and_missing_binding() {
    let f = Fixture::new();
    let (preview, _) = ready(&f);
    let request = PrepareSentCompletionRequest {
        before: preview.request.before.clone(),
        source_path: preview.source.source.path.clone(),
    };
    let mut corruptions = vec![];
    let mut changed = preview.clone();
    changed.request.before.data.title.push('!');
    corruptions.push(changed);
    let mut changed = preview.clone();
    changed.source.text.push('!');
    corruptions.push(changed);
    let mut changed = preview.clone();
    changed.request.sent_source = None;
    corruptions.push(changed);
    let mut changed = preview.clone();
    changed.request.sent_source.as_mut().unwrap().note_id = Uuid::new_v4();
    corruptions.push(changed);
    let mut changed = preview.clone();
    changed.title.clear();
    corruptions.push(changed);
    for changed in corruptions {
        assert!(changed.validate_for(&request).is_err());
    }
    let mut wrong_path = request.clone();
    wrong_path.source_path = "other-source.md".into();
    assert!(preview.validate_for(&wrong_path).is_err());
    let mut no_thread = request.clone();
    no_thread.before.data.thread = None;
    assert!(no_thread.validate().is_err());
    let mut app = f.app();
    app.completion_uncertain = true;
    assert_eq!(
        app.prepare_sent_action_completion(&request)
            .unwrap_err()
            .kind,
        ErrorKind::SaveUncertain
    );
}
