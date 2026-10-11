#[path = "../examples/synthetic_baseline/support.rs"]
mod support;

#[test]
fn fixture_refuses_existing_relative_and_invalid_size_destinations_before_writes() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let occupied = root.path().join("occupied");
    std::fs::create_dir(&occupied).unwrap();
    std::fs::write(occupied.join("owner.txt"), b"untouched").unwrap();
    assert!(support::create(&occupied, 20, "test").is_err());
    assert_eq!(
        std::fs::read(occupied.join("owner.txt")).unwrap(),
        b"untouched"
    );
    assert_eq!(std::fs::read_dir(&occupied).unwrap().count(), 1);
    let empty = root.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert!(support::create(&empty, 20, "test").is_err());
    assert_eq!(std::fs::read_dir(empty).unwrap().count(), 0);
    assert!(support::create(std::path::Path::new("relative-fixture"), 20, "test").is_err());
    let invalid = root.path().join("invalid");
    assert!(support::create(&invalid, 1001, "test").is_err());
    assert!(!invalid.exists());
    assert!(support::measure(&occupied).is_err());
    assert_eq!(std::fs::read_dir(occupied).unwrap().count(), 1);
}

#[test]
fn bounded_corpus_exercises_images_revisions_threads_receipts_and_reopened_search() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let first = root.path().join("first");
    let a = support::create(&first, 20, "test-source").unwrap();
    assert_eq!((a.short_notes, a.medium_notes, a.long_notes), (15, 4, 1));
    assert_eq!(
        (
            a.managed_images,
            a.sources,
            a.revised_notes,
            a.additional_note_revisions
        ),
        (4, 4, 5, 15)
    );
    assert_eq!((a.threads, a.messages, a.actions), (2, 24, 2));
    assert!(a.database_bytes > a.current_markdown_bytes as u64);
    let report = support::measure(&first).unwrap();
    assert!(report["headless_ms"]["search_cold_index"]["median_ms"].is_number());
    let b = support::create(&root.path().join("second"), 20, "test-source").unwrap();
    assert_eq!(a.semantic_content_sha256, b.semantic_content_sha256);
    assert_eq!((a.records, a.receipts), (b.records, b.receipts));
    assert_eq!(a.current_markdown_bytes, b.current_markdown_bytes);
    assert_ne!(a.selected_note, b.selected_note);
}

#[cfg(unix)]
#[test]
fn fixture_refuses_symlink_parent_before_creating_a_destination() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(root.path(), &link).unwrap();
    assert!(support::create(&link.join("new-fixture"), 20, "test").is_err());
    assert!(!root.path().join("new-fixture").exists());
}
