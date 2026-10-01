#![cfg(target_os = "macos")]

use brn_store::Approval;
use brn_store::{EvidenceCurrentness, OperationStatus};
use brn_workflow::{Config, ErrorKind, Workspace};
use brn_workflow::{ProviderOutcome, SearchProfile, SourceCurrentState, notes::*};
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
    fn new(provider: bool) -> Self {
        let data = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let path = vault.path().join("plan.md");
        fs::write(&path, "Aurora oldterm launches Tuesday.\r\n").unwrap();
        let config = if provider {
            fake(data.path())
        } else {
            Config::default()
        };
        let mut w = Workspace::open(data.path(), config).unwrap();
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
    fn calls(&self, method: &str) -> usize {
        fs::read_to_string(self.data.path().join("calls.log"))
            .unwrap_or_default()
            .lines()
            .filter(|line| *line == method)
            .count()
    }
}

#[test]
fn approval_uses_saved_bytes_noop_preserves_permission_and_changed_save_requires_new_snapshot() {
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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
    let mut f = Fixture::new(false);
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

fn fake(dir: &Path) -> Config {
    use std::os::unix::fs::PermissionsExt;
    let exe = dir.join("fake");
    fs::write(&exe,r#"#!/usr/bin/env python3
import json,sys,os,sqlite3
if '--version' in sys.argv:
 print('codex-cli 0.155.0-alpha.16.4');sys.exit(0)
def send(x):print(json.dumps(x),flush=True)
for line in sys.stdin:
 q=json.loads(line);m=q.get('method');rid=q.get('id')
 with open('../calls.log','a') as f:f.write(str(m)+'\n')
 if m=='initialize':send({'id':rid,'result':{'userAgent':'codex/0.155.0-alpha.16.4','codexHome':os.environ['CODEX_HOME'],'platformFamily':'unix','platformOs':'macos'}})
 elif m=='account/read':send({'id':rid,'result':{'account':{'type':'chatgpt'},'workspaceRouting':{'chatgptAccountId':'synthetic-account'}}})
 elif m in ['thread/start','thread/resume']:
  marker='../change-on-thread'
  if os.path.exists(marker):
   with open(marker) as f:path=f.read()
   with open(path,'w') as f:f.write('external newterm')
  tid=q.get('params',{}).get('threadId','thr_synthetic')
  send({'id':rid,'result':{'thread':{'id':tid,'sessionId':tid}}})
 elif m=='turn/start':
  tid=q['params']['threadId']
  if os.path.exists('../fail-commit'):
   db=sqlite3.connect('../brn.sqlite3')
   db.execute("CREATE TRIGGER fail_completion BEFORE UPDATE OF answer ON chat_turns BEGIN SELECT RAISE(ABORT,'synthetic completion failure'); END")
   db.commit();db.close()
  send({'id':rid,'result':{'turn':{'id':'turn_synthetic'}}})
  send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_synthetic','delta':'Aurora launches Tuesday [1].'}})
  send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_synthetic','status':'completed'}}})
"#).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    Config {
        codex: Some(exe),
        codex_home: Some(dir.join("synthetic-home")),
        model_dir: None,
    }
}

#[test]
fn provider_handoff_revalidates_after_search_and_after_thread_creation() {
    for after_thread in [false, true] {
        let mut f = Fixture::new(true);
        f.hit();
        if after_thread {
            fs::write(
                f.data.path().join("change-on-thread"),
                f.path.to_str().unwrap(),
            )
            .unwrap();
        }
        let failure =
            f.w.ask_full(
                Uuid::new_v4(),
                None,
                "oldterm",
                SearchProfile::Keyword,
                &AtomicBool::new(false),
                || {
                    if !after_thread {
                        fs::write(&f.path, "external newterm").unwrap();
                    }
                    Ok(())
                },
                |_| {},
            )
            .unwrap_err();
        assert_eq!(failure.kind, ErrorKind::EvidenceStale);
        assert_eq!(failure.recorded_status, None);
        assert_eq!(failure.provider_outcome, ProviderOutcome::Unknown);
        assert_eq!(f.calls("turn/start"), 0);
        assert_eq!(f.calls("initialize"), usize::from(after_thread));
    }
}

#[test]
fn stale_completion_is_preserved_and_replayed_without_provider_access() {
    let mut f = Fixture::new(true);
    f.hit();
    let op = Uuid::new_v4();
    let failure =
        f.w.ask_detailed(
            op,
            None,
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {
                fs::write(&f.path, "external newterm").unwrap();
            },
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::EvidenceStale);
    assert_eq!(failure.recorded_status, Some(OperationStatus::Completed));
    assert_eq!(
        failure.provider_outcome,
        ProviderOutcome::Confirmed(OperationStatus::Completed)
    );
    let turn = *failure.receipt.unwrap();
    assert_eq!(
        turn.evidence_currentness,
        EvidenceCurrentness::StaleAtCompletion
    );
    assert_eq!(turn.answer.as_deref(), Some("Aurora launches Tuesday [1]."));
    let session = turn.session_id;
    assert_eq!(f.w.history(session).unwrap(), vec![turn.clone()]);
    drop(f.w);
    f.w = Workspace::open(f.data.path(), Config::default()).unwrap();
    let replay =
        f.w.ask_detailed(
            op,
            Some(session),
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| panic!("replayed"),
        )
        .unwrap_err();
    assert_eq!(replay.kind, ErrorKind::EvidenceStale);
    assert_eq!(replay.receipt, Some(Box::new(turn)));
    assert_eq!(replay.recorded_status, Some(OperationStatus::Completed));
    assert_eq!(
        replay.provider_outcome,
        ProviderOutcome::Confirmed(OperationStatus::Completed)
    );
    assert_eq!(f.calls("turn/start"), 1);
    assert_eq!(
        f.w.ask_detailed(
            Uuid::new_v4(),
            Some(session),
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {}
        )
        .unwrap_err()
        .kind,
        ErrorKind::ContextStale
    );
    assert_eq!(f.calls("thread/resume"), 0);
}

#[test]
fn completed_receipt_becomes_stale_and_current_mode_resume_is_refused_before_authentication() {
    let mut f = Fixture::new(true);
    let op = Uuid::new_v4();
    let turn =
        f.w.ask_detailed(
            op,
            None,
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(
        turn.evidence_currentness,
        EvidenceCurrentness::CurrentAtCompletion
    );
    fs::write(&f.path, "external newterm").unwrap();
    let replay =
        f.w.ask_detailed(
            op,
            Some(turn.session_id),
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| panic!("replayed"),
        )
        .unwrap_err();
    assert_eq!(replay.kind, ErrorKind::EvidenceStale);
    assert_eq!(replay.receipt, Some(Box::new(turn.clone())));
    let failure =
        f.w.ask_detailed(
            Uuid::new_v4(),
            Some(turn.session_id),
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::ContextStale);
    assert_eq!(f.calls("initialize"), 1);
    assert_eq!(f.calls("thread/resume"), 0);
    assert_eq!(f.calls("turn/start"), 1);
}

#[test]
fn change_during_thread_resume_is_rejected_before_turn_submission() {
    let mut f = Fixture::new(true);
    let turn =
        f.w.ask_detailed(
            Uuid::new_v4(),
            None,
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    fs::write(
        f.data.path().join("change-on-thread"),
        f.path.to_str().unwrap(),
    )
    .unwrap();
    let failure =
        f.w.ask_detailed(
            Uuid::new_v4(),
            Some(turn.session_id),
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::ContextStale);
    assert_eq!(failure.recorded_status, None);
    assert_eq!(f.calls("thread/resume"), 1);
    assert_eq!(f.calls("turn/start"), 1);
}

#[test]
fn prior_managed_context_is_revalidated_at_completion_even_when_selected_hit_is_legacy() {
    let mut f = Fixture::new(true);
    let prior =
        f.w.ask_detailed(
            Uuid::new_v4(),
            None,
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let legacy = f.data.path().join("legacy.txt");
    fs::write(&legacy, "legacy uniqueterm").unwrap();
    f.w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &legacy,
        Approval::Approved,
    )
    .unwrap();
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let failure =
        f.w.ask_detailed(
            Uuid::new_v4(),
            Some(prior.session_id),
            "uniqueterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {
                fs::write(&f.path, "current changed note").unwrap();
            },
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::EvidenceStale);
    assert_eq!(failure.recorded_status, Some(OperationStatus::Completed));
    assert_eq!(
        failure.provider_outcome,
        ProviderOutcome::Confirmed(OperationStatus::Completed)
    );
    let receipt = failure.receipt.unwrap();
    assert_eq!(
        receipt.evidence_currentness,
        EvidenceCurrentness::StaleAtCompletion
    );
    assert!(!receipt.evidence_json.contains("oldterm"));
    assert_eq!(f.calls("turn/start"), 2);
}

#[test]
fn unqualified_unrelated_legacy_context_does_not_invalidate_a_session() {
    let mut f = Fixture::new(true);
    let legacy = f.data.path().join("legacy.txt");
    fs::write(&legacy, "legacy uniqueterm").unwrap();
    f.w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &legacy,
        Approval::Approved,
    )
    .unwrap();
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let prior =
        f.w.ask_detailed(
            Uuid::new_v4(),
            None,
            "uniqueterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    drop(f.w);
    let status = std::process::Command::new("python3").args(["-c","import sqlite3,sys; db=sqlite3.connect(sys.argv[1]); db.execute(\"UPDATE chat_turns SET evidence_currentness='unqualified'\"); db.commit()"]).arg(f.data.path().join("brn.sqlite3")).status().unwrap();
    assert!(status.success());
    fs::write(&f.path, "current changed note").unwrap();
    f.w = Workspace::open(f.data.path(), fake(f.data.path())).unwrap();
    assert_eq!(
        f.w.history(prior.session_id).unwrap()[0].evidence_currentness,
        EvidenceCurrentness::Unqualified
    );
    f.w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let resumed =
        f.w.ask_detailed(
            Uuid::new_v4(),
            Some(prior.session_id),
            "uniqueterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(resumed.status, OperationStatus::Completed);
    assert_eq!(f.calls("thread/resume"), 1);
    assert_eq!(f.calls("turn/start"), 2);
}

#[test]
fn completion_commit_failure_keeps_confirmed_provider_outcome_without_inventing_recorded_status() {
    let mut f = Fixture::new(true);
    fs::write(f.data.path().join("fail-commit"), "").unwrap();
    let failure =
        f.w.ask_detailed(
            Uuid::new_v4(),
            None,
            "oldterm",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::Other);
    assert!(failure.message.contains("synthetic completion failure"));
    assert_eq!(failure.recorded_status, None);
    assert_eq!(
        failure.provider_outcome,
        ProviderOutcome::Confirmed(OperationStatus::Completed)
    );
    let history = f.w.history(failure.session_id.unwrap()).unwrap();
    assert_eq!(history[0].status, OperationStatus::Running);
    assert_eq!(history[0].answer, None);
    assert_eq!(f.calls("turn/start"), 1);
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
