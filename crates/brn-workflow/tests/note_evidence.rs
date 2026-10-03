#![cfg(target_os = "macos")]

use brn_store::Approval;
use brn_workflow::{Config, ErrorKind, Workspace};
use brn_workflow::{SearchProfile, SourceCurrentState, notes::*};
use std::fs;
use std::{path::Path, sync::atomic::AtomicBool};
use uuid::Uuid;

struct Fixture {
    data: tempfile::TempDir,
    _vault: tempfile::TempDir,
    path: std::path::PathBuf,
    id: Uuid,
    w: Workspace,
}
impl Fixture {
    fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let path = vault.path().join("plan.md");
        fs::write(&path, "Aurora oldterm launches Tuesday.\r\n").unwrap();
        let mut w = Workspace::open(data.path(), Config::default()).unwrap();
        let note = w
            .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
            .unwrap();
        assert_eq!(note.search_approval, Approval::Draft);
        w.approve_note_snapshot(Uuid::new_v4(), note.id, note.current_file_state.unwrap())
            .unwrap();
        w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
        Self {
            data,
            _vault: vault,
            path,
            id: note.id,
            w,
        }
    }
    fn hit(&mut self) -> brn_retrieval::Evidence {
        self.w
            .search("oldterm", SearchProfile::Keyword)
            .unwrap()
            .evidence
            .remove(0)
    }
    fn request(&mut self, text: &str, increment: bool) -> NoteSubmission {
        let view = self.w.note(self.id).unwrap();
        NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: self.id,
            expected: view.stamp,
            generation: view.stamp.generation + u64::from(increment),
            text: text.into(),
        }
    }
}

#[test]
fn approval_uses_saved_bytes_noop_preserves_permission_and_changed_save_requires_new_snapshot() {
    let mut f = Fixture::new();
    let old = f.hit();
    let request = f.request("Aurora oldterm launches Tuesday.\r\n", false);
    assert_eq!(
        f.w.save_note(request).unwrap().filesystem_outcome,
        FileOutcome::NotApplied
    );
    f.w.validate_evidence(&old).unwrap();
    f.hit();
    let request = f.request("Aurora newterm launches Wednesday.", true);
    f.w.save_note_buffer(request.clone()).unwrap();
    let view = f.w.note(f.id).unwrap();
    let snapshot =
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, view.current_file_state.unwrap())
            .unwrap();
    assert_eq!(snapshot.version_id.to_string(), old.version_id);
    assert!(
        !f.w.sources().unwrap()[0]
            .bytes
            .windows(7)
            .any(|b| b == b"newterm")
    );
    let request = f.request("Aurora newterm launches Wednesday.", false);
    f.w.save_note(request).unwrap();
    assert_eq!(
        f.w.validate_evidence(&old).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        f.w.source_states().unwrap()[0].current_state,
        SourceCurrentState::Changed
    );
    assert!(f.w.source_projection().unwrap().0.is_empty());
    assert_eq!(
        f.w.set_approval(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            snapshot.source_id,
            snapshot.version_id,
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
    let view = f.w.note(f.id).unwrap();
    let approved =
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, view.current_file_state.unwrap())
            .unwrap();
    assert_ne!(approved.version_id, snapshot.version_id);
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    assert!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap()
            .evidence
            .is_empty()
    );
    assert!(
        !f.w.search("newterm", SearchProfile::Keyword)
            .unwrap()
            .evidence
            .is_empty()
    );
}

#[test]
fn observed_external_change_withdraws_permission_durably_and_reapproval_is_explicit() {
    let mut f = Fixture::new();
    let hit = f.hit();
    fs::write(&f.path, "newterm external").unwrap();
    let changed = f.w.note(f.id).unwrap();
    assert_eq!(changed.search_approval, Approval::Draft);
    assert_eq!(
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, changed.current_file_state.unwrap())
            .unwrap_err()
            .code,
        NoteErrorCode::StateChanged
    );
    assert_eq!(
        f.w.source_states().unwrap()[0].current_state,
        SourceCurrentState::Changed
    );
    let reloaded =
        f.w.reload_note(Uuid::new_v4(), f.id, changed.stamp, true)
            .unwrap();
    let approved =
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, reloaded.current_file_state.unwrap())
            .unwrap();
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(
        f.w.validate_evidence(&hit).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        f.w.set_approval(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            approved.source_id,
            Uuid::parse_str(&hit.version_id).unwrap(),
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
    assert!(
        !f.w.search("newterm", SearchProfile::Keyword)
            .unwrap()
            .evidence
            .is_empty()
    );
}

#[test]
fn missing_and_unavailable_notes_have_no_current_snapshot_bytes() {
    let mut f = Fixture::new();
    let hit = f.hit();
    fs::remove_file(&f.path).unwrap();
    let (docs, states) = f.w.source_projection().unwrap();
    assert!(docs.is_empty());
    assert_eq!(states[0].current_state, SourceCurrentState::Missing);
    assert_eq!(f.w.sources().unwrap_err().kind, ErrorKind::EvidenceStale);
    assert_eq!(
        f.w.validate_evidence(&hit).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    fs::write(&f.path, [0xff]).unwrap();
    assert_eq!(
        f.w.source_states().unwrap()[0].current_state,
        SourceCurrentState::Unavailable
    );
    assert!(f.w.source_projection().unwrap().0.is_empty());
}

#[test]
fn permission_does_not_revive_when_bytes_revert_and_snapshot_replay_does_not_reapprove() {
    let mut f = Fixture::new();
    let view = f.w.note(f.id).unwrap();
    let op = Uuid::new_v4();
    let receipt =
        f.w.approve_note_snapshot(op, f.id, view.current_file_state.unwrap())
            .unwrap();
    fs::write(&f.path, "changed").unwrap();
    f.w.note(f.id).unwrap();
    fs::write(&f.path, "Aurora oldterm launches Tuesday.\r\n").unwrap();
    assert_eq!(f.w.note(f.id).unwrap().search_approval, Approval::Draft);
    assert_eq!(
        f.w.approve_note_snapshot(op, f.id, view.current_file_state.unwrap())
            .unwrap(),
        receipt
    );
    assert_eq!(f.w.note(f.id).unwrap().search_approval, Approval::Draft);
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    let now = f.w.note(f.id).unwrap();
    f.w.approve_note_snapshot(Uuid::new_v4(), f.id, now.current_file_state.unwrap())
        .unwrap();
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    f.hit();
}

#[test]
fn managed_path_import_is_an_explicit_snapshot_and_cannot_bypass_reconciliation() {
    let mut f = Fixture::new();
    let first = f.w.sources().unwrap()[0].clone();
    let imported =
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &f.path,
            Approval::Approved,
        )
        .unwrap();
    assert_eq!(imported.source_id, first.source_id);
    assert_eq!(imported.version_id, first.version_id);
    assert!(!imported.changed);
    fs::write(&f.path, "newterm").unwrap();
    assert_eq!(
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &f.path,
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
    let view = f.w.note(f.id).unwrap();
    f.w.reload_note(Uuid::new_v4(), f.id, view.stamp, true)
        .unwrap();
    let imported =
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &f.path,
            Approval::Approved,
        )
        .unwrap();
    assert_eq!(imported.source_id, first.source_id);
    assert_ne!(imported.version_id, first.version_id);
    assert!(imported.changed);
}

#[test]
fn shadowed_origin_cannot_be_reimported_independently_after_relink() {
    let mut f = Fixture::new();
    // Enroll an exact-path legacy import in a second note.
    let legacy_path = f._vault.path().join("legacy.md");
    fs::write(&legacy_path, "legacy oldterm").unwrap();
    let imported =
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &legacy_path,
            Approval::Approved,
        )
        .unwrap();
    let note =
        f.w.open_note(Uuid::new_v4(), f._vault.path(), Path::new("legacy.md"))
            .unwrap();
    fs::rename(&legacy_path, f._vault.path().join("moved.md")).unwrap();
    f.w.relink_note(
        Uuid::new_v4(),
        note.id,
        note.stamp,
        Path::new("moved.md"),
        true,
    )
    .unwrap();
    fs::write(&legacy_path, "unrelated new bytes at old origin").unwrap();
    assert_eq!(
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &legacy_path,
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        f.w.source_states()
            .unwrap()
            .into_iter()
            .find(|s| s.source_id == imported.source_id)
            .unwrap()
            .current_state,
        SourceCurrentState::Shadowed
    );
}

#[test]
fn managed_missing_import_fails_as_stale_instead_of_uncategorized_io() {
    let mut f = Fixture::new();
    fs::remove_file(&f.path).unwrap();
    assert_eq!(
        f.w.import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &f.path,
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
}

#[test]
fn relinked_identical_bytes_get_a_new_snapshot_identity_not_a_reassigned_version() {
    let mut f = Fixture::new();
    let old = f.hit();
    let view = f.w.note(f.id).unwrap();
    fs::write(f._vault.path().join("replacement.md"), view.saved.unwrap()).unwrap();
    let relinked =
        f.w.relink_note(
            Uuid::new_v4(),
            f.id,
            view.stamp,
            Path::new("replacement.md"),
            true,
        )
        .unwrap();
    let approved =
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, relinked.current_file_state.unwrap())
            .unwrap();
    assert_ne!(approved.version_id.to_string(), old.version_id);
    assert_eq!(
        f.w.validate_evidence(&old).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
}

#[test]
fn unavailable_excluded_note_does_not_block_an_unrelated_legacy_index() {
    let mut f = Fixture::new();
    let snapshot = f.w.sources().unwrap()[0].clone();
    f.w.set_approval(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        snapshot.source_id,
        snapshot.version_id,
        Approval::Withdrawn,
    )
    .unwrap();
    let path = f.data.path().join("unrelated.txt");
    fs::write(&path, "unrelated uniqueterm").unwrap();
    f.w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &path,
        Approval::Approved,
    )
    .unwrap();
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    drop(f.w);
    fs::rename(f._vault.path(), f.data.path().join("unavailable-vault")).unwrap();
    f.w = Workspace::open(f.data.path(), Config::default()).unwrap();
    let (docs, states) = f.w.source_projection().unwrap();
    assert_eq!(docs.len(), 1);
    assert!(
        states
            .iter()
            .any(|s| s.current_state == SourceCurrentState::Unavailable)
    );
    assert!(
        !f.w.search("uniqueterm", SearchProfile::Keyword)
            .unwrap()
            .evidence
            .is_empty()
    );
    assert!(f.w.history(Uuid::new_v4()).unwrap().is_empty());
}

#[test]
fn owned_elsewhere_notes_are_explicit_and_have_no_current_bytes() {
    let mut f = Fixture::new();
    drop(f.w);
    let other_data = tempfile::tempdir().unwrap();
    let mut owner = Workspace::open(other_data.path(), Config::default()).unwrap();
    owner
        .open_note(Uuid::new_v4(), f._vault.path(), Path::new("plan.md"))
        .unwrap();
    f.w = Workspace::open(f.data.path(), Config::default()).unwrap();
    let (docs, states) = f.w.source_projection().unwrap();
    assert!(docs.is_empty());
    assert_eq!(states[0].current_state, SourceCurrentState::OwnedElsewhere);
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    assert_eq!(f.w.note(f.id).unwrap().saved, None);
}

#[test]
fn unresolved_original_save_suspends_eligibility_even_with_matching_disk_bytes() {
    let mut f = Fixture::new();
    let hit = f.hit();
    drop(f.w);
    let (mut store, _) = brn_store::Store::open(f.data.path()).unwrap();
    let record = store.note_record(f.id).unwrap();
    let recovery = store.note_recovery(f.id).unwrap().unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: f.id,
        expected: record.stamp,
        generation: record.stamp.generation,
        text: recovery.working,
    };
    store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            brn_store::notes::NoteWriteKind::Replace,
            &brn_store::notes::DestinationPrecondition::Existing {
                fingerprint: record.baseline,
                baseline_text: recovery.baseline,
            },
        )
        .unwrap();
    drop(store);
    f.w = Workspace::open(f.data.path(), Config::default()).unwrap();
    assert_eq!(
        f.w.source_states().unwrap()[0].current_state,
        SourceCurrentState::Uncertain
    );
    assert_eq!(
        f.w.validate_evidence(&hit).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    let view = f.w.note(f.id).unwrap();
    assert!(
        f.w.approve_note_snapshot(Uuid::new_v4(), f.id, view.current_file_state.unwrap())
            .is_err()
    );
    assert_eq!(
        f.w.reconcile_note_save(request.operation_id)
            .unwrap()
            .filesystem_outcome,
        FileOutcome::NotApplied
    );
    f.w.validate_evidence(&hit).unwrap();
    f.hit();
}

#[test]
fn changes_during_index_build_prevent_publication_of_an_old_corpus() {
    let mut f = Fixture::new();
    let prior = fs::read(f.data.path().join("active-index.json")).unwrap();
    let failure =
        f.w.build_index(&AtomicBool::new(false), |_| {
            fs::write(&f.path, "newterm changed while indexing").unwrap();
        })
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::IndexStale);
    assert_eq!(
        fs::read(f.data.path().join("active-index.json")).unwrap(),
        prior
    );
    assert_eq!(
        f.w.search("oldterm", SearchProfile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
}

#[test]
fn enrolled_import_cannot_remain_current_after_disk_changes() {
    let data = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    let path = vault.path().join("plan.md");
    std::fs::write(&path, b"oldterm").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let cancel = AtomicBool::new(false);
    let imported = w
        .import_file(&cancel, Uuid::new_v4(), &path, Approval::Approved)
        .unwrap();
    w.build_index(&cancel, |_| {}).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let approved = w
        .approve_note_snapshot(
            Uuid::new_v4(),
            opened.id,
            opened.current_file_state.unwrap(),
        )
        .unwrap();
    w.build_index(&cancel, |_| {}).unwrap();
    let prior = w
        .search("oldterm", brn_retrieval::Profile::Keyword)
        .unwrap();
    assert!(!prior.evidence.is_empty());
    let old_hit = prior.evidence[0].clone();
    std::fs::write(&path, b"new current content").unwrap();
    assert_eq!(
        w.validate_evidence(&old_hit).unwrap_err().kind,
        ErrorKind::EvidenceStale
    );
    assert_eq!(
        w.search("oldterm", brn_retrieval::Profile::Keyword)
            .unwrap_err()
            .kind,
        ErrorKind::IndexStale
    );
    assert_eq!(
        w.set_approval(
            &cancel,
            Uuid::new_v4(),
            imported.source_id,
            imported.version_id,
            Approval::Approved
        )
        .unwrap_err()
        .kind,
        ErrorKind::EvidenceStale
    );
    assert_ne!(approved.source_id, imported.source_id);
}
