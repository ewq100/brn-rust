//! Disposable native UI fixture. Run only on a fresh /private/tmp/brn-draft-native-* workspace.
use brn_store::{OperationStatus, Store};
use std::path::PathBuf;
use uuid::Uuid;

fn id(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("expected disposable data directory")?,
    );
    if std::env::var("BRN_SEED_DISPOSABLE").as_deref() != Ok("1")
        || !path
            .to_string_lossy()
            .starts_with("/private/tmp/brn-draft-native-")
        || path.join("brn.sqlite3").exists()
    {
        return Err("fixture requires BRN_SEED_DISPOSABLE=1 and a fresh /private/tmp/brn-draft-native-* directory".into());
    }
    let (mut store, _) = Store::open(&path)?;
    let draft = store.create_draft(
        id("00000000-0000-4000-8000-000000000101"),
        "Candidate target",
        "Root checkpoint\n",
    )?;
    let session = store.create_session(
        id("00000000-0000-4000-8000-000000000102"),
        "codex",
        "synthetic-disposable",
        None,
        Some("fixture-thread"),
        br#"{"fixture":true}"#,
    )?;
    let turn = id("00000000-0000-4000-8000-000000000103");
    store.prepare_turn(turn, session, "What does the fixture say?", "keyword", "[]")?;
    store.complete_turn(
        turn,
        OperationStatus::Completed,
        "Candidate fixture answer\n",
        None,
    )?;
    println!(
        "disposable draft={} root={} session={} completed_turn={}",
        draft.id, draft.stamp.base_revision, session, turn
    );
    Ok(())
}
