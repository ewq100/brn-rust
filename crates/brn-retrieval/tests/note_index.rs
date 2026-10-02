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
fn damaged_or_foreign_index_is_rebuilt_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    std::fs::write(&path, vec![0x42u8; 8192]).unwrap();
    let (index, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
    drop(index);

    let other = dir.path().join("other.sqlite");
    let raw = rusqlite::Connection::open(&other).unwrap();
    raw.execute_batch("CREATE TABLE x (y INTEGER); PRAGMA user_version = 99;")
        .unwrap();
    drop(raw);
    let (index, created) = NoteIndex::open(&other).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
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
