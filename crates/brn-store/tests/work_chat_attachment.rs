use brn_store::{Error, WorkStore, work::WorkTurnStatus};
use std::sync::{Arc, Barrier};
use uuid::Uuid;

#[test]
fn attachment_retains_the_exact_owner_lock_after_the_owner_drops() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (owner, _) = WorkStore::open(dir.path()).unwrap();
    let mut first = owner.chat_connection().unwrap();
    let second = owner.chat_connection().unwrap();
    drop(owner);
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::WorkspaceBusy(_))
    ));
    let id = Uuid::new_v4();
    first
        .begin_turn(id, None, "attached", "chatgpt", "gpt-5.5")
        .unwrap();
    first
        .finish_turn(id, WorkTurnStatus::Completed, "durable", None)
        .unwrap();
    drop(first);
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::WorkspaceBusy(_))
    ));
    assert_eq!(second.turn(id).unwrap().unwrap().answer, "durable");
    drop(second);
    assert!(WorkStore::open(dir.path()).is_ok());
}

#[test]
fn attached_finalization_and_owner_recovery_writes_are_serialized_without_snapshot_upgrade() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (mut owner, _) = WorkStore::open(dir.path()).unwrap();
    let mut chat = owner.chat_connection().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let ready = barrier.clone();
    let thread = std::thread::spawn(move || {
        for i in 0..50 {
            let id = Uuid::new_v4();
            ready.wait();
            chat.begin_turn(id, None, &format!("q{i}"), "chatgpt", "gpt-5.5")
                .unwrap();
            ready.wait();
            chat.finish_turn(id, WorkTurnStatus::Interrupted, "partial", None)
                .unwrap();
        }
    });
    for i in 0..50 {
        barrier.wait();
        owner
            .put_unsaved_edit("a.md", [1; 32], &format!("before{i}"))
            .unwrap();
        barrier.wait();
        owner
            .put_unsaved_edit("a.md", [2; 32], &format!("after{i}"))
            .unwrap();
    }
    thread.join().unwrap();
    assert_eq!(owner.conversations().unwrap().len(), 50);
    assert_eq!(owner.unsaved_edit("a.md").unwrap().unwrap().text, "after49");
}
