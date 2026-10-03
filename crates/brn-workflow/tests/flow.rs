use brn_store::Approval;
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
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            Approval::Approved,
        )
        .unwrap();
    let b = w
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            Approval::Approved,
        )
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
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            Approval::Approved,
        )
        .unwrap();
    assert_eq!(a.source_id, c.source_id);
    assert_ne!(a.version_id, c.version_id);
    assert!(
        w.search("Aurora", SearchProfile::Keyword)
            .unwrap_err()
            .message
            .contains("stale")
    );
    assert!(w.validate_evidence(&result.evidence[0]).is_err());
    w.build_index(&cancel, |_| {}).unwrap();
    w.search("Wednesday", SearchProfile::Keyword).unwrap();
    w.set_approval(
        &cancel,
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
    w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &file,
        Approval::Approved,
    )
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
#[test]
fn intact_but_wrong_generation_cannot_supply_search_results() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &file,
        Approval::Approved,
    )
    .unwrap();
    w.build_index(&AtomicBool::new(false), |_| {}).unwrap();
    let old: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("active-index.json")).unwrap()).unwrap();
    fs::write(&file, "Aurora launches on Thursday.").unwrap();
    w.import_file(
        &AtomicBool::new(false),
        Uuid::new_v4(),
        &file,
        Approval::Approved,
    )
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
            .message
            .contains("snapshot")
    );
}
#[test]
fn import_and_approval_respect_cancel_flag_before_mutation() {
    let dir = tempdir().unwrap();
    let file = fixture(dir.path());
    let mut w = Workspace::open(dir.path(), Config::default()).unwrap();
    let import_error = w
        .import_file(
            &AtomicBool::new(true),
            Uuid::new_v4(),
            &file,
            Approval::Approved,
        )
        .unwrap_err();
    assert_eq!(import_error.kind, brn_workflow::ErrorKind::Cancelled);
    assert!(w.sources().unwrap().is_empty(), "nothing may be persisted");

    let seeded = w
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            Approval::Approved,
        )
        .unwrap();
    let approval_error = w
        .set_approval(
            &AtomicBool::new(true),
            Uuid::new_v4(),
            seeded.source_id,
            seeded.version_id,
            Approval::Withdrawn,
        )
        .unwrap_err();
    assert_eq!(approval_error.kind, brn_workflow::ErrorKind::Cancelled);
    let sources = w.sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].approval, Approval::Approved, "state unchanged");
}
