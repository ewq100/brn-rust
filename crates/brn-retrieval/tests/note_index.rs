use brn_retrieval::note_index::{IndexedNote, NoteIndex};
use sha2::{Digest, Sha256};

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn passages(index_path: &std::path::Path, path: &str) -> Vec<(i64, i64, String)> {
    let conn = rusqlite::Connection::open(index_path).unwrap();
    let mut statement = conn
        .prepare(
            "SELECT start_byte, end_byte, text FROM passages WHERE path = ?1 ORDER BY start_byte",
        )
        .unwrap();
    statement
        .query_map([path], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn new_index_is_created_then_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
    drop(index);
    let (index, created) = NoteIndex::open(&path).unwrap();
    assert!(!created);
    assert_eq!(index.notes().unwrap(), vec![note("a.md", "alpha")]);
}

#[test]
fn damaged_or_outdated_index_is_rebuilt_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
    drop(index);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4096..8192].fill(0x42);
    std::fs::write(&path, bytes).unwrap();
    let (index, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
    drop(index);

    for sql in ["PRAGMA user_version = 99", "DROP TRIGGER passages_delete"] {
        let (mut index, _) = NoteIndex::open(&path).unwrap();
        index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
        drop(index);
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(sql).unwrap();
        drop(raw);
        let (index, created) = NoteIndex::open(&path).unwrap();
        assert!(created, "{sql}");
        assert!(index.notes().unwrap().is_empty(), "{sql}");
    }

    let empty = dir.path().join("empty.sqlite");
    std::fs::write(&empty, []).unwrap();
    let (index, created) = NoteIndex::open(&empty).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
}

#[test]
fn damaged_foreign_database_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    for application in [0, 0x4252_4e32] {
        let path = dir.path().join(format!("{application}.sqlite"));
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "CREATE TABLE x (y INTEGER CHECK(y > 0));
             PRAGMA ignore_check_constraints = ON;
             INSERT INTO x VALUES (-1);",
        )
        .unwrap();
        raw.pragma_update(None, "application_id", application)
            .unwrap();
        raw.pragma_update(None, "ignore_check_constraints", "OFF")
            .unwrap();
        let check: String = raw
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .unwrap();
        assert_ne!(check, "ok");
        drop(raw);
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(
            NoteIndex::open(&path),
            Err(brn_retrieval::Error::Invalid(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn non_sqlite_files_are_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    for bytes in [
        vec![0x42; 8192],
        b"SQLite format 3\0".to_vec(),
        vec![0x42; 99],
    ] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            NoteIndex::open(&path),
            Err(brn_retrieval::Error::Invalid(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn other_databases_are_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    for application in [0, 0x4252_4e32] {
        let path = dir.path().join(format!("{application}.sqlite"));
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("CREATE TABLE x (y INTEGER); INSERT INTO x VALUES (42);")
            .unwrap();
        raw.pragma_update(None, "application_id", application)
            .unwrap();
        drop(raw);

        assert!(matches!(
            NoteIndex::open(&path),
            Err(brn_retrieval::Error::Invalid(_))
        ));
        let raw = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            raw.query_row("SELECT y FROM x", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert_eq!(
            raw.query_row("PRAGMA application_id", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            application
        );
    }
}

#[test]
fn upsert_splits_into_passages_and_replaces_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    let long = "word ".repeat(500);
    index.upsert_note(&note("a.md", &long), &long).unwrap();
    let parts = passages(&path, "a.md");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].0, 0);
    assert_eq!(parts[0].1, parts[1].0);
    assert_eq!(parts[1].1 as usize, long.len());
    let joined: String = parts.iter().map(|p| p.2.as_str()).collect();
    assert_eq!(joined, long);

    index
        .upsert_note(&note("a.md", "short é\r\n"), "short é\r\n")
        .unwrap();
    assert_eq!(
        passages(&path, "a.md"),
        vec![(0, 10, "short é\r\n".to_string())]
    );
    assert_eq!(index.notes().unwrap().len(), 1);
}

#[test]
fn upsert_rejects_text_that_does_not_match_size_or_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    assert!(matches!(
        index.upsert_note(&note("a.md", "alpha"), "alphx"),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert!(index.notes().unwrap().is_empty());
}

#[test]
fn metadata_update_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
    index.update_metadata("a.md", 5, 99).unwrap();
    assert_eq!(index.note("a.md").unwrap().unwrap().modified_ns, 99);
    index.remove_note("a.md").unwrap();
    assert_eq!(index.note("a.md").unwrap(), None);
    assert!(passages(&path, "a.md").is_empty());
}
