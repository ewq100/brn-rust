#![cfg(target_os = "macos")]

use brn_store::Approval;
use brn_workflow::{Config, Workspace, notes::*};
use std::{fs, path::Path};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn ambiguous_copy_and_relink_candidates_are_refused_even_without_other_registry_paths() {
    for name in ["❤️.md", "👩‍💻.md"] {
        let mut f = Fixture::new();
        let failure =
            f.w.save_note_copy(f.request(), Path::new(name))
                .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Unsupported);
        assert!(failure.recovery_available);
        assert!(!f.vault.path().join(name).exists());
        fs::write(f.vault.path().join(name), "candidate").unwrap();
        let current = f.w.note(f.opened.id).unwrap();
        let failure =
            f.w.save_note_copy(
                NoteSubmission {
                    expected: current.stamp,
                    generation: current.stamp.generation + 1,
                    ..f.request()
                },
                Path::new(name),
            )
            .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Unsupported);
        assert!(failure.recovery_available);
        let current = f.w.note(f.opened.id).unwrap();
        let failure =
            f.w.relink_note(
                Uuid::new_v4(),
                current.id,
                current.stamp,
                Path::new(name),
                true,
            )
            .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Unsupported);
        assert_eq!(
            f.w.note(current.id).unwrap().relative_path,
            Path::new("plan.md")
        );
    }
}

#[test]
fn unqualified_registered_names_only_veto_same_or_unresolved_parent_destinations() {
    let mut f = Fixture::new();
    fs::write(f.vault.path().join("❤️.md"), "emoji").unwrap();
    let emoji =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("❤️.md"))
            .unwrap();
    fs::rename(
        f.vault.path().join("plan.md"),
        f.vault.path().join("moved.md"),
    )
    .unwrap();
    let relinked =
        f.w.relink_note(
            Uuid::new_v4(),
            f.opened.id,
            f.opened.stamp,
            Path::new("moved.md"),
            true,
        )
        .unwrap();
    assert_eq!(relinked.id, f.opened.id);
    for missing in [false, true] {
        if missing {
            fs::remove_file(f.vault.path().join("❤️.md")).unwrap();
        }
        let current = f.w.note(relinked.id).unwrap();
        let failure =
            f.w.save_note_copy(
                NoteSubmission {
                    expected: current.stamp,
                    generation: current.stamp.generation + 1,
                    ..f.request()
                },
                Path::new("rescue.md"),
            )
            .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Conflict);
        assert!(failure.recovery_available);
        let directory = if missing { "missing-rescue" } else { "rescue" };
        fs::create_dir(f.vault.path().join(directory)).unwrap();
        let current = f.w.note(relinked.id).unwrap();
        let destination = Path::new(directory).join("copy.md");
        let copy =
            f.w.save_note_copy(
                NoteSubmission {
                    expected: current.stamp,
                    generation: current.stamp.generation + 1,
                    ..f.request()
                },
                &destination,
            )
            .unwrap();
        assert_eq!(copy.source_note_id, f.opened.id);
        assert_ne!(copy.note_id, f.opened.id);
        assert_eq!(
            f.w.note(copy.note_id).unwrap().saved.as_deref(),
            Some("recover me")
        );
    }
    assert_eq!(
        f.w.note(emoji.id).unwrap().availability,
        NoteAvailability::Missing
    );
}

#[test]
fn emoji_notes_can_be_copied_or_relinked_into_a_distinct_parent() {
    for name in ["❤️.md", "👩‍💻.md"] {
        let mut f = Fixture::new();
        fs::write(f.vault.path().join(name), "emoji").unwrap();
        let emoji =
            f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new(name))
                .unwrap();
        fs::create_dir(f.vault.path().join("elsewhere")).unwrap();
        let copy =
            f.w.save_note_copy(
                NoteSubmission {
                    operation_id: Uuid::new_v4(),
                    note_id: emoji.id,
                    expected: emoji.stamp,
                    generation: 1,
                    text: "emoji recovery".into(),
                },
                Path::new("elsewhere/copy.md"),
            )
            .unwrap();
        assert_eq!(copy.source_note_id, emoji.id);
        assert_eq!(
            f.w.note(copy.note_id).unwrap().saved.as_deref(),
            Some("emoji recovery")
        );
        fs::remove_file(f.vault.path().join(name)).unwrap();
        fs::rename(
            f.vault.path().join("plan.md"),
            f.vault.path().join("elsewhere/moved.md"),
        )
        .unwrap();
        let moved =
            f.w.relink_note(
                Uuid::new_v4(),
                f.opened.id,
                f.opened.stamp,
                Path::new("elsewhere/moved.md"),
                true,
            )
            .unwrap();
        assert_eq!(moved.id, f.opened.id);
        assert_eq!(moved.relative_path, Path::new("elsewhere/moved.md"));
    }
}

#[test]
fn relink_refuses_another_registered_notes_moved_baseline_or_observed_inode() {
    for replaced in [false, true] {
        let mut f = Fixture::new();
        fs::write(f.vault.path().join("other.md"), "other").unwrap();
        let other =
            f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("other.md"))
                .unwrap();
        if replaced {
            fs::write(f.vault.path().join("replacement"), "replacement").unwrap();
            fs::rename(
                f.vault.path().join("replacement"),
                f.vault.path().join("other.md"),
            )
            .unwrap();
            f.w.note(other.id).unwrap();
        }
        fs::rename(
            f.vault.path().join("other.md"),
            f.vault.path().join("moved.md"),
        )
        .unwrap();
        let failure =
            f.w.relink_note(
                Uuid::new_v4(),
                f.opened.id,
                f.opened.stamp,
                Path::new("moved.md"),
                true,
            )
            .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Conflict);
        assert_eq!(failure.note_id, Some(other.id));
        assert!(failure.message.contains(&other.id.to_string()));
        assert!(failure.message.contains("other.md"));
        assert_eq!(
            f.w.note(f.opened.id).unwrap().relative_path,
            Path::new("plan.md")
        );
        assert_eq!(
            f.w.note(other.id).unwrap().availability,
            NoteAvailability::Missing
        );
        assert_eq!(
            fs::read(f.vault.path().join("moved.md")).unwrap(),
            if replaced {
                b"replacement".as_slice()
            } else {
                b"other".as_slice()
            }
        );
    }
}

#[test]
fn relink_refuses_another_registered_notes_fresh_replacement_inode() {
    let mut f = Fixture::new();
    fs::write(f.vault.path().join("other.md"), "other").unwrap();
    let other =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("other.md"))
            .unwrap();
    fs::write(f.vault.path().join("replacement"), "fresh replacement").unwrap();
    fs::rename(
        f.vault.path().join("replacement"),
        f.vault.path().join("other.md"),
    )
    .unwrap();
    let failure =
        f.w.relink_note(
            Uuid::new_v4(),
            f.opened.id,
            f.opened.stamp,
            Path::new("other.md"),
            true,
        )
        .unwrap_err();
    assert_eq!(failure.code, NoteErrorCode::Conflict);
    assert_eq!(failure.note_id, Some(other.id));
    assert!(failure.message.contains(&other.id.to_string()));
}

struct Fixture {
    _data: TempDir,
    vault: TempDir,
    w: Workspace,
    opened: NoteView,
}

impl Fixture {
    fn new() -> Self {
        let data = tempfile::tempdir_in(".").unwrap();
        let vault = tempfile::tempdir_in(".").unwrap();
        fs::write(vault.path().join("plan.md"), b"base").unwrap();
        let mut w = Workspace::open(data.path(), Config::default()).unwrap();
        let opened = w
            .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
            .unwrap();
        Self {
            _data: data,
            vault,
            w,
            opened,
        }
    }

    fn request(&self) -> NoteSubmission {
        NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: self.opened.id,
            expected: self.opened.stamp,
            generation: 1,
            text: "recover me".into(),
        }
    }
}

#[test]
fn collision_retains_submission_and_requires_confirmed_discard() {
    let mut f = Fixture::new();
    fs::write(f.vault.path().join("other.md"), b"occupied").unwrap();
    let request = f.request();
    let failure =
        f.w.save_note_copy(request.clone(), Path::new("other.md"))
            .unwrap_err();
    assert_eq!(failure.code, NoteErrorCode::Conflict);
    assert!(failure.recovery_available);
    assert_eq!(
        fs::read(f.vault.path().join("other.md")).unwrap(),
        b"occupied"
    );
    let dirty = f.w.note(f.opened.id).unwrap();
    assert_eq!(dirty.buffer, "recover me");
    assert_eq!(
        f.w.reload_note(Uuid::new_v4(), dirty.id, dirty.stamp, false)
            .unwrap_err()
            .code,
        NoteErrorCode::Conflict
    );
    let op = Uuid::new_v4();
    let clean = f.w.reload_note(op, dirty.id, dirty.stamp, true).unwrap();
    assert_eq!(clean.buffer, "base");
    assert_eq!(clean.stamp.generation, dirty.stamp.generation);
    assert_ne!(clean.stamp.file_state, dirty.stamp.file_state);
    assert_eq!(
        f.w.save_note_buffer(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: dirty.id,
            expected: dirty.stamp,
            generation: dirty.stamp.generation + 1,
            text: "delayed pre-discard edit".into()
        })
        .unwrap_err()
        .code,
        NoteErrorCode::StateChanged
    );
    assert_eq!(
        f.w.reload_note(op, dirty.id, dirty.stamp, true)
            .unwrap()
            .stamp,
        clean.stamp
    );
    assert_eq!(
        f.w.save_note_copy(request, Path::new("other.md"))
            .unwrap_err(),
        failure
    );
}

#[test]
fn comparison_is_three_way_and_tokens_change_without_rebasing() {
    let mut f = Fixture::new();
    f.w.save_note_buffer(f.request()).unwrap();
    fs::write(f.vault.path().join("plan.md"), b"external").unwrap();
    let first = f.w.compare_note(f.opened.id).unwrap();
    assert_eq!(first.baseline, "base");
    assert_eq!(first.working, "recover me");
    assert_eq!(first.observed.as_deref(), Some("external"));
    assert_eq!(first.stamp.generation, 1);
    assert_eq!(first.stamp.file_state, f.opened.stamp.file_state);
    assert_eq!(
        f.w.compare_note(f.opened.id).unwrap().observed_file_state,
        first.observed_file_state
    );
    fs::write(f.vault.path().join("plan.md"), b"later").unwrap();
    assert_ne!(
        f.w.compare_note(f.opened.id).unwrap().observed_file_state,
        first.observed_file_state
    );
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    let missing = f.w.compare_note(f.opened.id).unwrap();
    assert_eq!(missing.availability, NoteAvailability::Missing);
    assert!(missing.observed.is_none());
    assert!(missing.observed_file_state.is_none());
}

#[test]
fn copy_rescues_missing_original_and_replays_independent_identity() {
    let mut f = Fixture::new();
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    let request = f.request();
    let receipt =
        f.w.save_note_copy(request.clone(), Path::new("rescue.md"))
            .unwrap();
    assert_eq!(receipt.source_note_id, f.opened.id);
    assert_ne!(receipt.note_id, f.opened.id);
    assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
    assert_eq!(
        f.w.note(receipt.note_id).unwrap().search_approval,
        Approval::Draft
    );
    assert_eq!(
        fs::read(f.vault.path().join("rescue.md")).unwrap(),
        b"recover me"
    );
    assert!(!f.vault.path().join("plan.md").exists());
    fs::write(f.vault.path().join("rescue.md"), b"later external").unwrap();
    assert_eq!(
        f.w.save_note_copy(request.clone(), Path::new("rescue.md"))
            .unwrap(),
        receipt
    );
    assert_eq!(
        f.w.reconcile_note_save(request.operation_id).unwrap(),
        receipt
    );
    assert_eq!(f.w.note_recoveries().unwrap().len(), 2);
    assert_eq!(
        f.w.save_note_copy(request, Path::new("different.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn registered_missing_and_aliased_names_are_vetoed_on_actual_volume() {
    let mut f = Fixture::new();
    fs::write(f.vault.path().join("café.md"), b"unicode").unwrap();
    let unicode =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("café.md"))
            .unwrap();
    let case_insensitive = f.vault.path().join("Plan.md").exists();
    let normalization_insensitive = f.vault.path().join("cafe\u{301}.md").exists();
    println!(
        "volume name equivalence: case_insensitive={case_insensitive}, normalization_insensitive={normalization_insensitive}"
    );
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    fs::remove_file(f.vault.path().join("café.md")).unwrap();
    for spelling in ["plan.md", "Plan.md", "café.md", "cafe\u{301}.md"] {
        let current = f.w.note(f.opened.id).unwrap();
        let failure =
            f.w.save_note_copy(
                NoteSubmission {
                    expected: current.stamp,
                    generation: current.stamp.generation + 1,
                    ..f.request()
                },
                Path::new(spelling),
            )
            .unwrap_err();
        assert_eq!(failure.code, NoteErrorCode::Conflict, "{spelling}");
        assert!(failure.recovery_available);
        assert!(!f.vault.path().join(spelling).exists());
    }
    assert_eq!(
        f.w.note(unicode.id).unwrap().availability,
        NoteAvailability::Missing
    );
}

#[test]
fn full_case_fold_and_parent_aliases_conservatively_veto_missing_names() {
    let mut f = Fixture::new();
    fs::create_dir(f.vault.path().join("Folder")).unwrap();
    fs::write(f.vault.path().join("Folder/Straße.md"), b"other").unwrap();
    let other =
        f.w.open_note(
            Uuid::new_v4(),
            f.vault.path(),
            Path::new("Folder/Straße.md"),
        )
        .unwrap();
    fs::remove_file(f.vault.path().join("Folder/Straße.md")).unwrap();
    for path in ["Folder/STRASSE.md", "Folder/Straße.md"] {
        let stamp = f.w.note(f.opened.id).unwrap().stamp;
        assert_eq!(
            f.w.save_note_copy(
                NoteSubmission {
                    expected: stamp,
                    generation: stamp.generation + 1,
                    ..f.request()
                },
                Path::new(path)
            )
            .unwrap_err()
            .code,
            NoteErrorCode::Conflict
        );
    }
    assert_eq!(
        f.w.note(other.id).unwrap().availability,
        NoteAvailability::Missing
    );
}

#[test]
fn moves_and_replacements_require_explicit_relink_not_reload() {
    let mut f = Fixture::new();
    let saved_request = f.request();
    let saved_receipt = f.w.save_note(saved_request.clone()).unwrap();
    // A second save retires/prunes the first intent; its compact binding must
    // still replay against the original destination after explicit relinking.
    f.w.save_note(NoteSubmission {
        operation_id: Uuid::new_v4(),
        expected: saved_receipt.stamp,
        generation: 2,
        text: "recover me again".into(),
        ..saved_request.clone()
    })
    .unwrap();
    let before = f.w.note(f.opened.id).unwrap();
    fs::rename(
        f.vault.path().join("plan.md"),
        f.vault.path().join("moved.md"),
    )
    .unwrap();
    let refused =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("moved.md"))
            .unwrap_err();
    assert_eq!(refused.code, NoteErrorCode::Conflict);
    assert!(refused.message.contains("relink"));
    assert_eq!(
        f.w.relink_note(
            Uuid::new_v4(),
            before.id,
            before.stamp,
            Path::new("moved.md"),
            false
        )
        .unwrap_err()
        .code,
        NoteErrorCode::Conflict
    );
    let op = Uuid::new_v4();
    let relinked =
        f.w.relink_note(op, before.id, before.stamp, Path::new("moved.md"), true)
            .unwrap();
    assert_eq!(relinked.id, before.id);
    assert_eq!(relinked.relative_path, Path::new("moved.md"));
    assert_eq!(
        f.w.relink_note(op, before.id, before.stamp, Path::new("moved.md"), true)
            .unwrap()
            .stamp,
        relinked.stamp
    );
    assert_eq!(f.w.save_note(saved_request).unwrap().note_id, before.id);
    fs::write(f.vault.path().join("replacement.md"), b"replacement").unwrap();
    fs::rename(
        f.vault.path().join("replacement.md"),
        f.vault.path().join("moved.md"),
    )
    .unwrap();
    assert_eq!(
        f.w.reload_note(Uuid::new_v4(), before.id, relinked.stamp, true)
            .unwrap_err()
            .code,
        NoteErrorCode::Conflict
    );
    let accepted =
        f.w.relink_note(
            Uuid::new_v4(),
            before.id,
            relinked.stamp,
            Path::new("moved.md"),
            true,
        )
        .unwrap();
    assert_eq!(accepted.saved.as_deref(), Some("replacement"));
    assert_eq!(accepted.buffer, "recover me again");
}

#[test]
fn stale_reload_and_relink_never_discard_newer_work_or_claim_reserved_identity() {
    let mut f = Fixture::new();
    f.w.save_note_buffer(f.request()).unwrap();
    assert_eq!(
        f.w.reload_note(Uuid::new_v4(), f.opened.id, f.opened.stamp, true)
            .unwrap_err()
            .code,
        NoteErrorCode::StateChanged
    );
    fs::write(f.vault.path().join("other.md"), b"other").unwrap();
    let other =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("other.md"))
            .unwrap();
    let stamp = f.w.note(f.opened.id).unwrap().stamp;
    assert_eq!(
        f.w.relink_note(
            Uuid::new_v4(),
            f.opened.id,
            stamp,
            Path::new("other.md"),
            true
        )
        .unwrap_err()
        .code,
        NoteErrorCode::Conflict
    );
    assert_eq!(f.w.note(other.id).unwrap().buffer, "other");
}

#[test]
fn recovery_copy_does_not_require_supported_current_original_bytes() {
    for bytes in [vec![0xff], vec![b'x'; 1024 * 1024 + 1]] {
        let mut f = Fixture::new();
        fs::write(f.vault.path().join("plan.md"), bytes).unwrap();
        let receipt =
            f.w.save_note_copy(f.request(), Path::new("rescue.md"))
                .unwrap();
        assert_eq!(receipt.source_note_id, f.opened.id);
        assert_eq!(
            fs::read(f.vault.path().join("rescue.md")).unwrap(),
            b"recover me"
        );
    }
}

#[test]
fn missing_parent_and_parent_spelling_aliases_do_not_evade_reservations() {
    let mut f = Fixture::new();
    fs::create_dir(f.vault.path().join("Folder")).unwrap();
    fs::write(f.vault.path().join("Folder/saved.md"), "registered").unwrap();
    f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("Folder/saved.md"))
        .unwrap();
    fs::remove_file(f.vault.path().join("Folder/saved.md")).unwrap();
    let stamp = f.w.note(f.opened.id).unwrap().stamp;
    // The actual fixture volume is case-insensitive; this uses its resolved
    // directory identity as well as a conservatively folded component key.
    if f.vault.path().join("folder").is_dir() {
        assert_eq!(
            f.w.save_note_copy(
                NoteSubmission {
                    expected: stamp,
                    ..f.request()
                },
                Path::new("folder/SAVED.md")
            )
            .unwrap_err()
            .code,
            NoteErrorCode::Conflict
        );
    }

    fs::rename(f.vault.path().join("Folder"), f.vault.path().join("Moved")).unwrap();
    let stamp = f.w.note(f.opened.id).unwrap().stamp;
    assert_eq!(
        f.w.save_note_copy(
            NoteSubmission {
                expected: stamp,
                generation: stamp.generation + 1,
                ..f.request()
            },
            Path::new("Moved/SAVED.md")
        )
        .unwrap_err()
        .code,
        NoteErrorCode::Conflict
    );
}

#[test]
fn reoccupied_missing_registry_path_cannot_be_enrolled_through_alternate_spelling() {
    let mut f = Fixture::new();
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    fs::write(f.vault.path().join("plan.md"), b"new identity").unwrap();
    assert_eq!(
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("Plan.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::Conflict
    );
    assert_eq!(f.w.note_recoveries().unwrap().len(), 1);
    assert_eq!(f.w.note(f.opened.id).unwrap().buffer, "base");
}

#[test]
fn conservative_full_fold_keys_never_assign_identity_to_existing_distinct_files() {
    let mut f = Fixture::new();
    fs::write(f.vault.path().join("Straße.md"), b"first").unwrap();
    let first =
        f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("Straße.md"))
            .unwrap();
    let equivalent = f.vault.path().join("STRASSE.md").exists();
    println!("volume full-fold expansion equivalence: Straße/STRASSE={equivalent}");
    if equivalent {
        assert_eq!(
            f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("STRASSE.md"))
                .unwrap_err()
                .code,
            NoteErrorCode::Conflict
        );
    } else {
        fs::write(f.vault.path().join("STRASSE.md"), b"second").unwrap();
        let second =
            f.w.open_note(Uuid::new_v4(), f.vault.path(), Path::new("STRASSE.md"))
                .unwrap();
        assert_ne!(first.id, second.id);
    }
}
