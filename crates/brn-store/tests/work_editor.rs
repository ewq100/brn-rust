use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, PreparedFile},
    work::{EditRequest, EditorRecord, SaveOutcome, SaveRequest},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn fixture(text: &str) -> (tempfile::TempDir, WorkStore, EditorRecord) {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let editor = store
        .open_editor("notes/note.md", &fingerprint(text, 2), text)
        .unwrap();
    (dir, store, editor)
}

fn edit(editor: &EditorRecord, generation: u64, text: &str) -> EditRequest {
    EditRequest {
        path: editor.path.clone(),
        expected: editor.stamp,
        generation,
        text: text.into(),
    }
}

fn request(editor: &EditorRecord, generation: u64, text: &str, copy: Option<&str>) -> SaveRequest {
    SaveRequest {
        operation_id: Uuid::new_v4(),
        edit: edit(editor, generation, text),
        destination: copy.map(str::to_owned),
    }
}

fn stage(request: &SaveRequest) -> PathBuf {
    Path::new(request.destination.as_deref().unwrap_or(&request.edit.path))
        .with_file_name(format!(".brn-{}.stage", request.operation_id))
}

fn prepare(store: &mut WorkStore, request: &SaveRequest, inode: u64) -> FileFingerprint {
    let installed = fingerprint(&request.edit.text, inode);
    store
        .prepare_editor_save(
            request.operation_id,
            &PreparedFile {
                relative: stage(request),
                fingerprint: installed.clone(),
            },
        )
        .unwrap();
    installed
}

#[test]
fn exact_bytes_and_existing_recovery_survive_restart() {
    let baseline = "\u{feff}---\r\ncustom: 🦀\r\n---\r\n";
    let (_dir, mut store, editor) = fixture(baseline);
    let working = "\u{feff}---\r\ncustom: 🦀\r\n---\r\nsmall correction\r\n";
    let recovered = store.recover_editor(&edit(&editor, 1, working)).unwrap();
    assert_eq!(recovered.baseline_text.as_bytes(), baseline.as_bytes());
    assert_eq!(recovered.text.as_bytes(), working.as_bytes());
    assert_eq!(
        store
            .open_editor(&editor.path, &fingerprint("external", 99), "external")
            .unwrap(),
        recovered
    );
    let dir = store.data_dir().to_owned();
    drop(store);
    let (store, _) = WorkStore::open(&dir).unwrap();
    assert_eq!(store.editor(&editor.path).unwrap(), Some(recovered));
    assert_eq!(store.editors().unwrap().len(), 1);
}

#[test]
fn generations_allow_lagged_acknowledgements_but_refuse_regression_and_conflicting_equal_text() {
    let (_dir, mut store, editor) = fixture("base");
    store.recover_editor(&edit(&editor, 2, "two")).unwrap();
    // The original acknowledged stamp can still submit a newer generation.
    let three = store.recover_editor(&edit(&editor, 3, "three")).unwrap();
    assert_eq!(
        store.recover_editor(&edit(&editor, 3, "three")).unwrap(),
        three
    );
    for invalid in [edit(&editor, 2, "two"), edit(&editor, 3, "different")] {
        assert!(matches!(
            store.recover_editor(&invalid),
            Err(Error::StateChanged(_))
        ));
    }
    let mut future = edit(&three, 4, "four");
    future.expected.generation = 10;
    assert!(store.recover_editor(&future).is_err());
    let mut foreign = edit(&three, 4, "four");
    foreign.expected.baseline = Uuid::new_v4();
    assert!(store.recover_editor(&foreign).is_err());
    assert_eq!(store.editor(&editor.path).unwrap(), Some(three));
}

#[test]
fn input_limits_and_fingerprint_are_checked_without_normalizing_empty_text() {
    let (_dir, mut store, editor) = fixture("");
    assert_eq!(editor.baseline_text, "");
    let maximum = "x".repeat(brn_store::MAX_NOTE_BYTES);
    store.recover_editor(&edit(&editor, 1, &maximum)).unwrap();
    assert!(
        store
            .recover_editor(&edit(&editor, 2, &(maximum + "x")))
            .is_err()
    );
    assert!(
        store
            .open_editor("bad.md", &fingerprint("a", 4), "b")
            .is_err()
    );
    for invalid in [
        "",
        "/note.md",
        "../note.md",
        "nested/../note.md",
        "nested//note.md",
        "./note.md",
        "note.txt",
        "a\\note.md",
    ] {
        assert!(
            store.open_editor(invalid, &fingerprint("", 4), "").is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn save_intent_atomically_protects_input_and_binds_replay_before_current_state() {
    let (_dir, mut store, editor) = fixture("old");
    let request = request(&editor, 1, "submitted", None);
    let intent = store.begin_editor_save(&request, &stage(&request)).unwrap();
    assert_eq!(
        store.editor(&editor.path).unwrap().unwrap().text,
        "submitted"
    );
    assert_eq!(intent.baseline_text, "old");
    store
        .recover_editor(&edit(&editor, 2, "later typing"))
        .unwrap();
    assert_eq!(
        store.begin_editor_save(&request, &stage(&request)).unwrap(),
        intent
    );
    let mut conflicting = request.clone();
    conflicting.edit.text = "different".into();
    assert!(matches!(
        store.begin_editor_save(&conflicting, &stage(&request)),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.begin_editor_save(&request, Path::new("other.stage")),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(store.editor_saves().unwrap(), vec![intent]);
}

#[test]
fn invalid_save_rolls_back_buffer_and_creates_no_intent() {
    let (_dir, mut store, editor) = fixture("old");
    let mut request = request(&editor, 1, "new", None);
    request.edit.expected.baseline = Uuid::new_v4();
    assert!(store.begin_editor_save(&request, &stage(&request)).is_err());
    assert_eq!(store.editor(&editor.path).unwrap(), Some(editor));
    assert!(store.editor_save(request.operation_id).unwrap().is_none());
    request.edit.expected.baseline = Uuid::new_v4();
    assert!(
        store
            .begin_editor_save(&request, Path::new("notes/unsafe.md"))
            .is_err()
    );
}

#[test]
fn applied_original_updates_baseline_keeps_later_typing_and_replays_after_restart() {
    let (dir, mut store, editor) = fixture("old\r\n");
    let request = request(&editor, 1, "new\r\n", None);
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let installed = prepare(&mut store, &request, 3);
    store
        .recover_editor(&edit(&editor, 2, "typing after Save\r\n"))
        .unwrap();
    let receipt = store
        .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    let updated = store.editor(&editor.path).unwrap().unwrap();
    assert_eq!(updated.baseline, installed);
    assert_eq!(updated.baseline_text, request.edit.text);
    assert_eq!(updated.text, "typing after Save\r\n");
    assert_eq!(updated.stamp.generation, 2);
    assert_eq!(receipt.stamp.generation, 1);
    assert_eq!(receipt.stamp.baseline, updated.stamp.baseline);
    assert_ne!(updated.stamp.baseline, editor.stamp.baseline);
    assert_eq!(
        store.editor_previous(&editor.path).unwrap(),
        Some(("old\r\n".into(), "new\r\n".into()))
    );
    // An old baseline token cannot edit after the apply acknowledgement.
    assert!(store.recover_editor(&edit(&editor, 3, "stale")).is_err());
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let replay = store.begin_editor_save(&request, &stage(&request)).unwrap();
    assert_eq!(replay.receipt, Some(receipt.clone()));
    assert_eq!(
        store
            .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
            .unwrap(),
        receipt
    );
    assert!(matches!(
        store.finish_editor_save(
            request.operation_id,
            SaveOutcome::Applied,
            Some(&fingerprint(&request.edit.text, 999))
        ),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(store.editor(&editor.path).unwrap(), Some(updated));
}

#[test]
fn applied_save_requires_exact_prepared_identity() {
    let (_dir, mut store, editor) = fixture("old");
    let request = request(&editor, 1, "new", None);
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let candidate = fingerprint("new", 3);
    assert!(
        store
            .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&candidate))
            .is_err()
    );
    assert!(
        store
            .prepare_editor_save(
                request.operation_id,
                &PreparedFile {
                    relative: stage(&request),
                    fingerprint: fingerprint("wrong", 3)
                }
            )
            .is_err()
    );
    let installed = prepare(&mut store, &request, 3);
    assert!(
        store
            .finish_editor_save(
                request.operation_id,
                SaveOutcome::Applied,
                Some(&fingerprint("new", 999))
            )
            .is_err()
    );
    store
        .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
}

#[test]
fn uncertain_or_pending_original_blocks_new_original_but_allows_copy_and_reconciliation() {
    let (_dir, mut store, editor) = fixture("old");
    let original = request(&editor, 1, "new", None);
    store
        .begin_editor_save(&original, &stage(&original))
        .unwrap();
    let installed = prepare(&mut store, &original, 3);
    let later = request(&editor, 2, "later", None);
    assert!(store.begin_editor_save(&later, &stage(&later)).is_err());
    store
        .finish_editor_save(original.operation_id, SaveOutcome::Uncertain, None)
        .unwrap();
    assert!(store.begin_editor_save(&later, &stage(&later)).is_err());
    let copy = request(&editor, 2, "later", Some("rescue.md"));
    store.begin_editor_save(&copy, &stage(&copy)).unwrap();
    let before = store.editor(&editor.path).unwrap().unwrap();
    let copy_installed = prepare(&mut store, &copy, 4);
    store
        .finish_editor_save(
            copy.operation_id,
            SaveOutcome::Applied,
            Some(&copy_installed),
        )
        .unwrap();
    assert_eq!(store.editor(&editor.path).unwrap(), Some(before));
    assert_eq!(store.editor_previous(&editor.path).unwrap(), None);
    store
        .finish_editor_save(
            original.operation_id,
            SaveOutcome::Applied,
            Some(&installed),
        )
        .unwrap();
    let current = store.editor(&editor.path).unwrap().unwrap();
    assert_eq!(current.text, "later");
    assert_eq!(current.stamp.generation, 2);
    let next = request(&current, 3, "final", None);
    store.begin_editor_save(&next, &stage(&next)).unwrap();
}

#[test]
fn not_applied_keeps_baseline_and_does_not_replace_previous_applied_pair() {
    let (_dir, mut store, editor) = fixture("old");
    let applied = request(&editor, 1, "new", None);
    store.begin_editor_save(&applied, &stage(&applied)).unwrap();
    let installed = prepare(&mut store, &applied, 3);
    store
        .finish_editor_save(applied.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    let current = store.editor(&editor.path).unwrap().unwrap();
    let noop = request(&current, 1, "new", None);
    store.begin_editor_save(&noop, &stage(&noop)).unwrap();
    let receipt = store
        .finish_editor_save(noop.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    assert_eq!(receipt.stamp, current.stamp);
    assert_eq!(store.editor(&editor.path).unwrap(), Some(current.clone()));
    assert_eq!(
        store.editor_previous(&editor.path).unwrap(),
        Some(("old".into(), "new".into()))
    );
    let failed = request(&current, 2, "failed submitted text", None);
    store.begin_editor_save(&failed, &stage(&failed)).unwrap();
    store
        .finish_editor_save(failed.operation_id, SaveOutcome::Uncertain, None)
        .unwrap();
    store
        .finish_editor_save(failed.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    let retained = store.editor(&editor.path).unwrap().unwrap();
    assert_eq!(retained.baseline_text, "new");
    assert_eq!(retained.text, "failed submitted text");
    assert_eq!(retained.stamp.baseline, current.stamp.baseline);
    assert!(matches!(
        store.finish_editor_save(failed.operation_id, SaveOutcome::Applied, Some(&installed)),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn failed_receipt_transaction_never_exposes_a_changed_baseline() {
    let (dir, mut store, editor) = fixture("old");
    let request = request(&editor, 1, "new", None);
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let installed = prepare(&mut store, &request, 3);
    let before = store.editor(&editor.path).unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_pair BEFORE INSERT ON editor_previous BEGIN SELECT RAISE(FAIL,'synthetic persistence failure'); END;").unwrap();
    assert!(
        store
            .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
            .is_err()
    );
    assert_eq!(store.editor(&editor.path).unwrap(), before);
    assert!(
        store
            .editor_save(request.operation_id)
            .unwrap()
            .unwrap()
            .receipt
            .is_none()
    );
    conn.execute_batch("DROP TRIGGER fail_pair").unwrap();
    store
        .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
}

#[test]
fn hash_checked_records_fail_visibly_when_payloads_are_modified() {
    let (dir, mut store, editor) = fixture("old");
    let request = request(&editor, 1, "new", None);
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute(
        "UPDATE editor_saves SET intent_json=?1 WHERE operation_id=?2",
        rusqlite::params![b"{}".as_slice(), request.operation_id.to_string()],
    )
    .unwrap();
    assert!(matches!(
        store.editor_save(request.operation_id),
        Err(Error::Invalid(_))
    ));
    conn.execute(
        "UPDATE editors SET record_json=?1 WHERE path=?2",
        rusqlite::params![b"{}".as_slice(), editor.path],
    )
    .unwrap();
    assert!(matches!(store.editor(&editor.path), Err(Error::Invalid(_))));
}

#[test]
fn v2_upgrade_preserves_existing_unsaved_edits_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .put_unsaved_edit(
            "before.md",
            fingerprint("base", 1).sha256,
            "\u{feff}unsaved\r\n",
        )
        .unwrap();
    store.set_setting("synthetic", "kept").unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute_batch("DROP TABLE findings; ALTER TABLE messages DROP COLUMN started_at_ms; ALTER TABLE messages DROP COLUMN finished_at_ms; ALTER TABLE conversations DROP COLUMN last_activity_at_ms; ALTER TABLE messages DROP COLUMN effort; DROP TABLE proposal_rewrites; DROP TABLE proposal_applies; DROP TABLE proposals; DROP TABLE editor_completed; DROP TABLE editor_previous; DROP TABLE editor_saves; DROP TABLE editors; PRAGMA user_version=2;").unwrap();
    drop(conn);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.unsaved_edit("before.md").unwrap().unwrap().text,
        "\u{feff}unsaved\r\n"
    );
    assert_eq!(store.setting("synthetic").unwrap().as_deref(), Some("kept"));
    assert!(store.editors().unwrap().is_empty());
    store
        .open_editor("after.md", &fingerprint("", 2), "")
        .unwrap();
}

#[test]
fn prior_unsaved_text_is_promoted_only_when_its_baseline_matches() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let baseline = fingerprint("saved\r\n", 1);
    store
        .put_unsaved_edit("match.md", baseline.sha256, "\u{feff}recovered\r\n")
        .unwrap();
    store
        .put_unsaved_edit("conflict.md", baseline.sha256, "protected old edit")
        .unwrap();
    let promoted = store
        .open_editor("match.md", &baseline, "saved\r\n")
        .unwrap();
    assert_eq!(promoted.baseline_text, "saved\r\n");
    assert_eq!(promoted.text, "\u{feff}recovered\r\n");
    assert_eq!(promoted.stamp.generation, 1);
    assert!(store.unsaved_edit("match.md").unwrap().is_none());
    assert!(matches!(
        store.open_editor(
            "conflict.md",
            &fingerprint("external edit", 2),
            "external edit"
        ),
        Err(Error::StateChanged(_))
    ));
    assert!(store.editor("conflict.md").unwrap().is_none());
    assert_eq!(
        store.unsaved_edit("conflict.md").unwrap().unwrap().text,
        "protected old edit"
    );
}

#[test]
fn explicit_reload_uses_exact_stamp_and_discard_without_resetting_generation() {
    let (_dir, mut store, editor) = fixture("saved");
    let dirty = store
        .recover_editor(&edit(&editor, 4, "unfinished"))
        .unwrap();
    let observed = fingerprint("external\r\n", 9);
    assert!(matches!(
        store.reload_editor(&editor.path, editor.stamp, &observed, "external\r\n", true),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.reload_editor(&editor.path, dirty.stamp, &observed, "external\r\n", false),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(store.editor(&editor.path).unwrap(), Some(dirty.clone()));
    let reloaded = store
        .reload_editor(&editor.path, dirty.stamp, &observed, "external\r\n", true)
        .unwrap();
    assert_eq!(reloaded.text, "external\r\n");
    assert_eq!(reloaded.baseline_text, reloaded.text);
    assert_eq!(reloaded.baseline, observed);
    assert_eq!(reloaded.stamp.generation, 4);
    assert_ne!(reloaded.stamp.baseline, dirty.stamp.baseline);
    assert!(
        store
            .recover_editor(&edit(&dirty, 5, "stale after reload"))
            .is_err()
    );
    assert_eq!(store.editor_previous(&editor.path).unwrap(), None);
    // A clean reload also replaces the token even for unchanged exact bytes.
    let clean = store
        .reload_editor(
            &editor.path,
            reloaded.stamp,
            &observed,
            "external\r\n",
            false,
        )
        .unwrap();
    assert_ne!(clean.stamp.baseline, reloaded.stamp.baseline);
}

#[test]
fn reload_cannot_hide_an_unresolved_original_save() {
    let (_dir, mut store, editor) = fixture("saved");
    let request = request(&editor, 1, "submitted", None);
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let current = store.editor(&editor.path).unwrap().unwrap();
    assert!(matches!(
        store.reload_editor(
            &editor.path,
            current.stamp,
            &fingerprint("external", 9),
            "external",
            true
        ),
        Err(Error::SaveUncertain(_))
    ));
    store
        .finish_editor_save(request.operation_id, SaveOutcome::Uncertain, None)
        .unwrap();
    assert!(matches!(
        store.reload_editor(
            &editor.path,
            current.stamp,
            &fingerprint("external", 9),
            "external",
            true
        ),
        Err(Error::SaveUncertain(_))
    ));
    assert_eq!(store.editor(&editor.path).unwrap(), Some(current));
}

#[test]
fn compact_noop_replays_before_state_checks_without_reopening_a_save() {
    let (dir, mut store, editor) = fixture("same\r\n");
    let noop = request(&editor, 0, "same\r\n", None);
    store.begin_editor_save(&noop, &stage(&noop)).unwrap();
    store.mark_editor_save_noop(noop.operation_id).unwrap();
    let receipt = store
        .finish_editor_save(noop.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    store.compact_editor_save(noop.operation_id).unwrap();
    assert!(store.editor_save(noop.operation_id).unwrap().is_none());
    assert_eq!(
        store.editor_save_replay(&noop, &stage(&noop)).unwrap(),
        Some((receipt.clone(), true))
    );
    assert_eq!(
        store.editor_save_receipt(noop.operation_id).unwrap(),
        Some(receipt.clone())
    );
    assert!(matches!(
        store.begin_editor_save(&noop, &stage(&noop)),
        Err(Error::StateChanged(_))
    ));
    let mut conflicting = noop.clone();
    conflicting.edit.text = "other".into();
    assert!(matches!(
        store.editor_save_replay(&conflicting, &stage(&noop)),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.begin_editor_save(&conflicting, &stage(&noop)),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.editor_save_replay(&noop, Path::new("other.stage")),
        Err(Error::OperationConflict(_))
    ));
    // Replacing the editor baseline cannot invalidate an exact settled replay.
    store
        .reload_editor(
            &editor.path,
            editor.stamp,
            &fingerprint("external", 9),
            "external",
            false,
        )
        .unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store.compact_editor_save(noop.operation_id).unwrap();
    assert_eq!(
        store.editor_save_replay(&noop, &stage(&noop)).unwrap(),
        Some((receipt, true))
    );
    assert_eq!(store.editor_previous(&editor.path).unwrap(), None);
}

#[test]
fn compaction_retains_latest_applied_original_and_one_recovery_pair_per_path() {
    let (_dir, mut store, editor) = fixture("base");
    let first = request(&editor, 1, "first", None);
    store.begin_editor_save(&first, &stage(&first)).unwrap();
    let installed = prepare(&mut store, &first, 3);
    let first_receipt = store
        .finish_editor_save(first.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    store.compact_editor_save(first.operation_id).unwrap();
    assert!(store.editor_save(first.operation_id).unwrap().is_some());
    assert!(
        store
            .editor_save_replay(&first, &stage(&first))
            .unwrap()
            .is_none()
    );
    let current = store.editor(&editor.path).unwrap().unwrap();
    let second = request(&current, 2, "second", None);
    store.begin_editor_save(&second, &stage(&second)).unwrap();
    let installed = prepare(&mut store, &second, 4);
    store
        .finish_editor_save(second.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    store.compact_editor_save(first.operation_id).unwrap();
    store.compact_editor_save(second.operation_id).unwrap();
    assert!(store.editor_save(first.operation_id).unwrap().is_none());
    assert_eq!(
        store.editor_save_replay(&first, &stage(&first)).unwrap(),
        Some((first_receipt, false))
    );
    assert_eq!(store.editor_saves().unwrap().len(), 1);
    assert_eq!(
        store.editor_saves().unwrap()[0].request.operation_id,
        second.operation_id
    );
    assert_eq!(
        store.editor_previous(&editor.path).unwrap(),
        Some(("first".into(), "second".into()))
    );
    // A separate path's latest Applied original also retains its full journal.
    let other = store
        .open_editor("other.md", &fingerprint("", 20), "")
        .unwrap();
    let other_request = request(&other, 1, "other", None);
    store
        .begin_editor_save(&other_request, &stage(&other_request))
        .unwrap();
    let installed = prepare(&mut store, &other_request, 21);
    store
        .finish_editor_save(
            other_request.operation_id,
            SaveOutcome::Applied,
            Some(&installed),
        )
        .unwrap();
    store
        .compact_editor_save(other_request.operation_id)
        .unwrap();
    assert_eq!(store.editor_saves().unwrap().len(), 2);
}

#[test]
fn pending_and_uncertain_original_keep_full_reconciliation_payloads() {
    let (_dir, mut store, editor) = fixture("base");
    let original = request(&editor, 1, "submitted", None);
    let intent = store
        .begin_editor_save(&original, &stage(&original))
        .unwrap();
    store.compact_editor_save(original.operation_id).unwrap();
    assert_eq!(
        store.editor_save(original.operation_id).unwrap(),
        Some(intent)
    );
    let installed = prepare(&mut store, &original, 3);
    store
        .finish_editor_save(original.operation_id, SaveOutcome::Uncertain, None)
        .unwrap();
    let uncertain = store.editor_save(original.operation_id).unwrap();
    store.compact_editor_save(original.operation_id).unwrap();
    assert_eq!(store.editor_save(original.operation_id).unwrap(), uncertain);
    assert!(
        store
            .editor_save_replay(&original, &stage(&original))
            .unwrap()
            .is_none()
    );
    store
        .finish_editor_save(
            original.operation_id,
            SaveOutcome::Applied,
            Some(&installed),
        )
        .unwrap();
    store.compact_editor_save(original.operation_id).unwrap();
    assert!(store.editor_save(original.operation_id).unwrap().is_some());
}

#[test]
fn settled_copy_and_refused_original_compact_without_copying_text_into_history() {
    let (dir, mut store, editor) = fixture("base");
    let copy = request(
        &editor,
        1,
        &"x".repeat(brn_store::MAX_NOTE_BYTES),
        Some("copy.md"),
    );
    store.begin_editor_save(&copy, &stage(&copy)).unwrap();
    let installed = prepare(&mut store, &copy, 3);
    let receipt = store
        .finish_editor_save(copy.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    store.compact_editor_save(copy.operation_id).unwrap();
    assert_eq!(
        store.editor_save_replay(&copy, &stage(&copy)).unwrap(),
        Some((receipt, false))
    );
    assert!(store.editor_save(copy.operation_id).unwrap().is_none());
    assert_eq!(store.editor_previous(&editor.path).unwrap(), None);
    let current = store.editor(&editor.path).unwrap().unwrap();
    let refusal = request(&current, 2, "refused text", None);
    store.begin_editor_save(&refusal, &stage(&refusal)).unwrap();
    let receipt = store
        .finish_editor_save(refusal.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    store.compact_editor_save(refusal.operation_id).unwrap();
    assert_eq!(
        store
            .editor_save_replay(&refusal, &stage(&refusal))
            .unwrap(),
        Some((receipt, false))
    );
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let lengths: (i64, i64) = conn.query_row(
        "SELECT length(request_sha256),length(receipt_json) FROM editor_completed WHERE operation_id=?1",
        [copy.operation_id.to_string()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(lengths.0, 32);
    assert!(lengths.1 < 600);
    assert_eq!(store.editor_saves().unwrap(), vec![]);
}

#[test]
fn compact_ledger_hashes_and_noop_metadata_are_checked_on_replay() {
    let (dir, mut store, editor) = fixture("base");
    let noop = request(&editor, 0, "base", None);
    store.begin_editor_save(&noop, &stage(&noop)).unwrap();
    store.mark_editor_save_noop(noop.operation_id).unwrap();
    store
        .finish_editor_save(noop.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    store.compact_editor_save(noop.operation_id).unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute(
        "UPDATE editor_completed SET no_op=0 WHERE operation_id=?1",
        [noop.operation_id.to_string()],
    )
    .unwrap();
    assert!(matches!(
        store.editor_save_replay(&noop, &stage(&noop)),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.editor_save_receipt(noop.operation_id),
        Err(Error::Invalid(_))
    ));
    conn.execute(
        "UPDATE editor_completed SET no_op=1,receipt_json=?1 WHERE operation_id=?2",
        rusqlite::params![b"{}".as_slice(), noop.operation_id.to_string()],
    )
    .unwrap();
    assert!(matches!(
        store.editor_save_replay(&noop, &stage(&noop)),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.compact_editor_save(noop.operation_id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn compaction_is_atomic_if_journal_retirement_fails() {
    let (dir, mut store, editor) = fixture("base");
    let refused = request(&editor, 1, "submitted", None);
    store.begin_editor_save(&refused, &stage(&refused)).unwrap();
    store
        .finish_editor_save(refused.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    let original = store.editor_save(refused.operation_id).unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_retire BEFORE DELETE ON editor_saves BEGIN SELECT RAISE(FAIL,'synthetic retirement failure'); END;").unwrap();
    assert!(store.compact_editor_save(refused.operation_id).is_err());
    assert_eq!(store.editor_save(refused.operation_id).unwrap(), original);
    assert!(
        store
            .editor_save_replay(&refused, &stage(&refused))
            .unwrap()
            .is_none()
    );
    conn.execute_batch("DROP TRIGGER fail_retire").unwrap();
    store.compact_editor_save(refused.operation_id).unwrap();
    assert!(
        store
            .editor_save_replay(&refused, &stage(&refused))
            .unwrap()
            .is_some()
    );
}

#[test]
fn destination_parent_binding_is_durable_immutable_and_preparation_only() {
    use brn_store::files::VaultIdentity;
    let (dir, mut store, editor) = fixture("base");
    let request = request(&editor, 1, "submitted", Some("copy.md"));
    store.begin_editor_save(&request, &stage(&request)).unwrap();
    let parent = VaultIdentity {
        device: 1,
        inode: 100,
    };
    store
        .bind_editor_save_parent(request.operation_id, &parent)
        .unwrap();
    store
        .bind_editor_save_parent(request.operation_id, &parent)
        .unwrap();
    assert!(matches!(
        store.bind_editor_save_parent(
            request.operation_id,
            &VaultIdentity {
                device: 1,
                inode: 101
            }
        ),
        Err(Error::OperationConflict(_))
    ));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store
            .editor_save(request.operation_id)
            .unwrap()
            .unwrap()
            .parent,
        Some(parent.clone())
    );
    let installed = prepare(&mut store, &request, 3);
    assert!(matches!(
        store.bind_editor_save_parent(request.operation_id, &parent),
        Err(Error::StateChanged(_))
    ));
    store
        .finish_editor_save(request.operation_id, SaveOutcome::Applied, Some(&installed))
        .unwrap();
    assert!(matches!(
        store.bind_editor_save_parent(request.operation_id, &parent),
        Err(Error::StateChanged(_))
    ));
}

#[test]
fn refused_equal_text_never_becomes_a_successful_noop_on_compacted_replay() {
    let (dir, mut store, editor) = fixture("same");
    let refused = request(&editor, 0, "same", None);
    store.begin_editor_save(&refused, &stage(&refused)).unwrap();
    let receipt = store
        .finish_editor_save(refused.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    assert!(
        !store
            .editor_save(refused.operation_id)
            .unwrap()
            .unwrap()
            .no_op
    );
    store.compact_editor_save(refused.operation_id).unwrap();
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store
            .editor_save_replay(&refused, &stage(&refused))
            .unwrap(),
        Some((receipt, false))
    );
}

#[test]
fn marked_noop_survives_restart_and_compacts_only_its_recorded_success() {
    let (dir, mut store, editor) = fixture("same\r\n");
    let noop = request(&editor, 0, "same\r\n", None);
    store.begin_editor_save(&noop, &stage(&noop)).unwrap();
    store.mark_editor_save_noop(noop.operation_id).unwrap();
    store.mark_editor_save_noop(noop.operation_id).unwrap();
    assert!(
        store
            .prepare_editor_save(
                noop.operation_id,
                &PreparedFile {
                    relative: stage(&noop),
                    fingerprint: fingerprint("same\r\n", 3)
                }
            )
            .is_err()
    );
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let intent = store.editor_save(noop.operation_id).unwrap().unwrap();
    assert!(intent.no_op);
    assert!(intent.prepared.is_none());
    let receipt = store
        .finish_editor_save(noop.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    assert!(matches!(
        store.mark_editor_save_noop(noop.operation_id),
        Err(Error::StateChanged(_))
    ));
    store.compact_editor_save(noop.operation_id).unwrap();
    assert_eq!(
        store.editor_save_replay(&noop, &stage(&noop)).unwrap(),
        Some((receipt, true))
    );
}

#[test]
fn noop_mark_refuses_changed_text_copy_and_prepared_original() {
    let (_dir, mut store, editor) = fixture("base");
    let changed = request(&editor, 1, "changed", None);
    store.begin_editor_save(&changed, &stage(&changed)).unwrap();
    assert!(matches!(
        store.mark_editor_save_noop(changed.operation_id),
        Err(Error::StateChanged(_))
    ));
    store
        .finish_editor_save(changed.operation_id, SaveOutcome::NotApplied, None)
        .unwrap();
    let current = store.editor(&editor.path).unwrap().unwrap();
    let copy = request(&current, 2, "base", Some("copy.md"));
    store.begin_editor_save(&copy, &stage(&copy)).unwrap();
    assert!(matches!(
        store.mark_editor_save_noop(copy.operation_id),
        Err(Error::StateChanged(_))
    ));
    let original = request(&current, 3, "base", None);
    store
        .begin_editor_save(&original, &stage(&original))
        .unwrap();
    prepare(&mut store, &original, 3);
    assert!(matches!(
        store.mark_editor_save_noop(original.operation_id),
        Err(Error::StateChanged(_))
    ));
    assert!(
        !store
            .editor_save(original.operation_id)
            .unwrap()
            .unwrap()
            .no_op
    );
}
