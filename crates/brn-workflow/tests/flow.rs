use brn_store::{Approval, OperationStatus};
use brn_workflow::{Config, SearchProfile, Workspace};
use std::{fs, path::Path, sync::atomic::AtomicBool};
use tempfile::tempdir;
use uuid::Uuid;
fn fixture(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("launch.md");
    fs::write(
        &p,
        "The synthetic Aurora mission launches on Tuesday. Its crew includes Mira and Niko.\r\n",
    )
    .unwrap();
    p
}
#[test]
fn import_index_search_reopen_change_and_withdrawal() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let original = fs::read(&file).unwrap();
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    let a = w
        .import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    let b = w
        .import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    assert_eq!(a.version_id, b.version_id);
    assert!(!b.changed);
    let cancel = AtomicBool::new(false);
    w.build_index(&cancel, |_| {}).unwrap();
    let result = w.search("Aurora launch", SearchProfile::Keyword).unwrap();
    assert!(!result.evidence.is_empty());
    assert_eq!(result.evidence[0].version_id, a.version_id.to_string());
    assert!(w.search("Aurora", SearchProfile::Semantic).is_err());
    drop(w);
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    w.search("Aurora", SearchProfile::Keyword).unwrap();
    assert_eq!(fs::read(&file).unwrap(), original);
    fs::write(&file, "Aurora now launches on Wednesday.").unwrap();
    let c = w
        .import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    assert_eq!(a.source_id, c.source_id);
    assert_ne!(a.version_id, c.version_id);
    assert!(
        w.search("Aurora", SearchProfile::Keyword)
            .unwrap_err()
            .contains("stale")
    );
    assert!(w.validate_evidence(&result.evidence[0]).is_err());
    w.build_index(&cancel, |_| {}).unwrap();
    w.search("Wednesday", SearchProfile::Keyword).unwrap();
    w.set_approval(
        Uuid::new_v4(),
        c.source_id,
        c.version_id,
        Approval::Withdrawn,
    )
    .unwrap();
    assert!(w.search("Aurora", SearchProfile::Keyword).is_err());
}
#[test]
fn cancelled_build_preserves_active_pointer_and_corrupt_derived_pointer_allows_history() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let old = fs::read(dir.path().join("active-index.json")).unwrap();
    assert!(w.build_index(&AtomicBool::new(true), |_| {}).is_err());
    assert_eq!(old, fs::read(dir.path().join("active-index.json")).unwrap());
    drop(w);
    fs::write(dir.path().join("active-index.json"), b"broken").unwrap();
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    assert!(w.sessions().unwrap().is_empty());
    assert!(w.search("Aurora", SearchProfile::Keyword).is_err());
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    w.search("Aurora", SearchProfile::Keyword).unwrap();
}
fn fake(dir: &Path, mode: &str) -> Config {
    use std::os::unix::fs::PermissionsExt;
    let exe = dir.join(format!("fake-{mode}"));
    fs::write(&exe,r#"#!/usr/bin/env python3
import json,sys,os,sqlite3
if '--version' in sys.argv:
 print('codex-cli 0.155.0-alpha.16.4');sys.exit(0)
mode=os.path.basename(sys.argv[0]).split('-')[-1]
def send(x):print(json.dumps(x),flush=True)
for line in sys.stdin:
 q=json.loads(line);m=q.get('method');rid=q.get('id')
 if m=='initialize':send({'id':rid,'result':{'userAgent':'codex/0.155.0-alpha.16.4','codexHome':os.environ.get('CODEX_HOME','/tmp/fake'),'platformFamily':'unix','platformOs':'macos'}})
 elif m=='account/read':send({'id':rid,'result':{'account':{'type':'chatgpt'},'workspaceRouting':{'chatgptAccountId':'synthetic-account'}}})
 elif m in ['thread/start','thread/resume']:
  tid=q.get('params',{}).get('threadId','thr_synthetic');send({'id':rid,'result':{'thread':{'id':tid,'sessionId':tid}}})
 elif m=='turn/start':
  tid=q['params']['threadId'];db=sqlite3.connect('file:../brn.sqlite3?mode=ro',uri=True)
  assert db.execute('SELECT count(*) FROM sessions WHERE thread_id=?',(tid,)).fetchone()[0]==1
  assert db.execute("SELECT count(*) FROM chat_turns t JOIN operations o ON t.operation_id=o.id WHERE o.status='pending'").fetchone()[0]==1
  assert 'SOURCE [1]' in q['params']['input'][0]['text']
  with open('../submitted.log','a') as f:f.write('turn\n')
  send({'id':rid,'result':{'turn':{'id':'turn_synthetic'}}})
  if mode=='fail':sys.exit(0)
  send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_synthetic','delta':'Aurora launches Tuesday [1].'}})
  if mode=='cancel':continue
  send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_synthetic','status':'completed'}}})
 elif m=='turn/interrupt':
  send({'id':rid,'result':{}});send({'method':'turn/completed','params':{'threadId':q['params']['threadId'],'turn':{'id':'turn_synthetic','status':'interrupted'}}})
"#).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    Config {
        codex: Some(exe),
        codex_home: Some(dir.join("synthetic-home")),
        model_dir: None,
    }
}
#[test]
fn grounded_question_commits_before_submission_reopens_and_never_duplicates() {
    let dir = tempdir().unwrap();
    let config = fake(dir.path(), "success");
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), config.clone()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let op = Uuid::new_v4();
    let mut streamed = String::new();
    let t = w
        .ask(
            op,
            None,
            "When does Aurora launch?",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |s| streamed.push_str(s),
        )
        .unwrap();
    assert_eq!(t.status, OperationStatus::Completed);
    assert_eq!(Some(streamed), t.answer);
    assert!(t.evidence_json.contains("Tuesday"));
    let session = t.session_id;
    drop(w);
    let mut w = Workspace::open(dir.path(), config).unwrap();
    assert_eq!(w.history(session).unwrap().len(), 1);
    let duplicate = w
        .ask(
            op,
            Some(session),
            "When does Aurora launch?",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| panic!("replayed"),
        )
        .unwrap();
    assert_eq!(duplicate, t);
    assert_eq!(
        fs::read_to_string(dir.path().join("submitted.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(
        w.ask(
            op,
            Some(session),
            "Changed question",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {}
        )
        .is_err()
    );
    w.ask(
        Uuid::new_v4(),
        Some(session),
        "Who is in the Aurora crew?",
        SearchProfile::Keyword,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert_eq!(w.history(session).unwrap().len(), 2);
}
#[test]
fn uncertain_turn_survives_restart_without_resubmission() {
    let dir = tempdir().unwrap();
    let config = fake(dir.path(), "fail");
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), config.clone()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let op = Uuid::new_v4();
    assert!(
        w.ask(
            op,
            None,
            "Aurora?",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {}
        )
        .is_err()
    );
    let session = w.sessions().unwrap()[0].id;
    drop(w);
    let mut w = Workspace::open(dir.path(), config).unwrap();
    let t = w
        .ask(
            op,
            Some(session),
            "Aurora?",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(t.status, OperationStatus::Interrupted);
    assert_eq!(
        fs::read_to_string(dir.path().join("submitted.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}
#[test]
fn cancelled_or_stale_context_does_not_connect_provider() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    let source = w
        .import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    assert!(
        w.ask_guarded(
            Uuid::new_v4(),
            None,
            "Aurora",
            SearchProfile::Keyword,
            &AtomicBool::new(true),
            || panic!("provider entered"),
            |_| {}
        )
        .is_err()
    );
    w.set_approval(
        Uuid::new_v4(),
        source.source_id,
        source.version_id,
        Approval::Withdrawn,
    )
    .unwrap();
    assert!(
        w.ask_guarded(
            Uuid::new_v4(),
            None,
            "Aurora",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            || panic!("provider entered"),
            |_| {}
        )
        .is_err()
    );
    assert!(w.sessions().unwrap().is_empty());
}
#[test]
fn intact_but_wrong_generation_cannot_supply_search_results() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let old: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("active-index.json")).unwrap()).unwrap();
    fs::write(&file, "Aurora launches on Thursday.").unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    drop(w);
    let mut active: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("active-index.json")).unwrap()).unwrap();
    active["directory"] = old["directory"].clone();
    fs::write(
        dir.path().join("active-index.json"),
        serde_json::to_vec(&active).unwrap(),
    )
    .unwrap();
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    assert!(
        w.search("Aurora", SearchProfile::Keyword)
            .unwrap_err()
            .contains("snapshot")
    );
}
#[test]
fn changed_provider_store_never_replaces_saved_thread() {
    let dir = tempdir().unwrap();
    let config = fake(dir.path(), "success");
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), config.clone()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let t = w
        .ask(
            Uuid::new_v4(),
            None,
            "Aurora",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    drop(w);
    let mut changed = config;
    changed.codex_home = Some(dir.path().join("other-home"));
    let mut w = Workspace::open(dir.path(), changed).unwrap();
    assert!(
        w.ask(
            Uuid::new_v4(),
            Some(t.session_id),
            "Aurora",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| {}
        )
        .unwrap_err()
        .contains("association")
    );
    assert_eq!(w.sessions().unwrap().len(), 1);
    assert_eq!(w.history(t.session_id).unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(dir.path().join("submitted.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn cancellation_persists_interrupted_status_and_never_replays() {
    let dir = tempdir().unwrap();
    let config = fake(dir.path(), "cancel");
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), config.clone()).unwrap();
    w.import_file(Uuid::new_v4(), &file, Approval::Approved)
        .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let op = Uuid::new_v4();
    let cancel = AtomicBool::new(false);
    let turn = w
        .ask(op, None, "Aurora", SearchProfile::Keyword, &cancel, |_| {
            cancel.store(true, std::sync::atomic::Ordering::Release)
        })
        .unwrap();
    assert_eq!(turn.status, OperationStatus::Interrupted);
    drop(w);
    let mut w = Workspace::open(dir.path(), config).unwrap();
    let again = w
        .ask(
            op,
            Some(turn.session_id),
            "Aurora",
            SearchProfile::Keyword,
            &AtomicBool::new(false),
            |_| panic!("replayed"),
        )
        .unwrap();
    assert_eq!(again, turn);
    assert_eq!(
        fs::read_to_string(dir.path().join("submitted.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}
