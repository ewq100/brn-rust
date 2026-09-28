use brn_retrieval::{Document, Index, Profile, hash};
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

fn doc(id: &str, text: &str) -> Document {
    Document {
        source_id: id.into(),
        version_id: "v1".into(),
        title: id.into(),
        text: text.into(),
        source_hash: hash(text.as_bytes()),
    }
}
#[test]
fn keyword_index_returns_exact_utf8_passage_and_reopens() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let text = format!("{} café 🌍 target phrase", "A".repeat(1580));
    let docs = [doc("one", &text), doc("two", "unrelated")];
    let mut index = Index::build(&path, &docs, None, &AtomicBool::new(false), |_| {}).unwrap();
    let hit = index
        .search("target phrase", Profile::Keyword, 3)
        .unwrap()
        .remove(0);
    assert_eq!(&text[hit.start_byte..hit.end_byte], hit.quote);
    assert!(text.is_char_boundary(hit.start_byte) && text.is_char_boundary(hit.end_byte));
    assert!(hit.end_byte - hit.start_byte <= 1600);
    assert_eq!(hit.source_hash, docs[0].source_hash);
    assert_eq!(hit.generation, index.generation());
    drop(index);
    let mut index = Index::open(&path).unwrap();
    assert_eq!(
        index.search("target phrase", Profile::Keyword, 3).unwrap()[0].source_id,
        "one"
    );
}

#[test]
fn chunks_cover_each_utf8_byte_once_without_exceeding_limit() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let text = "café 🌍 apple ".repeat(500);
    let docs = [doc("a", &text)];
    let mut index = Index::build(&path, &docs, None, &AtomicBool::new(false), |_| {}).unwrap();
    let hits = index.search("apple", Profile::Keyword, 100).unwrap();
    assert!(hits.len() > 1);
    let mut ranges: Vec<_> = hits.iter().map(|h| (h.start_byte, h.end_byte)).collect();
    ranges.sort();
    assert_eq!(ranges[0].0, 0);
    assert_eq!(ranges.last().unwrap().1, text.len());
    for pair in ranges.windows(2) {
        assert_eq!(pair[0].1, pair[1].0);
    }
    for (start, end) in ranges {
        assert!(end - start <= 1600);
        assert!(text.is_char_boundary(start) && text.is_char_boundary(end));
    }
}

#[test]
fn keyword_ranking_and_unavailable_semantic_are_explicit() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let docs = [
        doc("both", "red blue"),
        doc("one", "red"),
        doc("none", "green"),
    ];
    let mut index = Index::build(&path, &docs, None, &AtomicBool::new(false), |_| {}).unwrap();
    let hits = index.search("red blue", Profile::Keyword, 3).unwrap();
    assert_eq!(
        hits.iter()
            .map(|h| h.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["both", "one"]
    );
    assert!(index.search("red", Profile::Semantic, 3).is_err());
    assert!(index.search("red", Profile::Hybrid, 3).is_err());
    assert!(index.search("", Profile::Keyword, 3).is_err());
    assert!(index.search("red", Profile::Keyword, 0).is_err());
}

#[test]
fn incomplete_corrupt_and_existing_generation_fail_closed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let docs = [doc("a", "café")];
    let cancel = AtomicBool::new(true);
    assert!(Index::build(&path, &docs, None, &cancel, |_| {}).is_err());
    assert!(!path.exists());
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("sentinel"), b"unchanged").unwrap();
    assert!(Index::build(&path, &docs, None, &AtomicBool::new(false), |_| {}).is_err());
    assert_eq!(std::fs::read(path.join("sentinel")).unwrap(), b"unchanged");
    assert!(Index::open(&path).is_err());
    std::fs::remove_dir_all(&path).unwrap();
    let index = Index::build(&path, &docs, None, &AtomicBool::new(false), |_| {}).unwrap();
    drop(index);
    std::fs::write(path.join("chunks.json"), b"[]").unwrap();
    assert!(Index::open(&path).is_err());
}

#[test]
fn mismatched_source_hash_is_rejected_before_generation_creation() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let mut document = doc("a", "hello");
    document.source_hash = "0".repeat(64);
    assert!(Index::build(&path, &[document], None, &AtomicBool::new(false), |_| {}).is_err());
    assert!(!path.exists());
}

#[test]
fn cancellation_after_directory_creation_leaves_unpublished_generation() {
    use std::sync::atomic::Ordering;
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let cancel = AtomicBool::new(false);
    let docs = [doc("a", "a long document with apple")];
    let result = Index::build(&path, &docs, None, &cancel, |phase| {
        if phase == "chunking" {
            cancel.store(true, Ordering::Relaxed);
        }
    });
    assert!(result.is_err());
    assert!(path.exists());
    assert!(!path.join("COMPLETE").exists());
    assert!(Index::open(&path).is_err());
}

#[test]
fn malformed_manifest_and_invalid_query_are_rejected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let mut index = Index::build(
        &path,
        &[doc("a", "apple")],
        None,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert!(index.search(&"x".repeat(513), Profile::Keyword, 1).is_err());
    drop(index);
    std::fs::write(path.join("manifest.json"), b"broken").unwrap();
    assert!(Index::open(&path).is_err());
}

#[test]
fn punctuation_is_not_part_of_keyword_terms() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let mut index = Index::build(
        &path,
        &[doc("aurora", "Aurora launch notes")],
        None,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert_eq!(
        index.search("Aurora?", Profile::Keyword, 3).unwrap()[0].source_id,
        "aurora"
    );
    assert!(index.search("?", Profile::Keyword, 3).is_err());
}
