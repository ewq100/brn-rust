use brn_retrieval::note_index::{Embedder, IndexedNote, NoteIndex, NoteIndexReader, NoteSearch};
use sha2::{Digest, Sha256};

struct Fake;
impl Embedder for Fake {
    fn identity(&self) -> &str {
        "model-a"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![1.0, 0.5]).collect())
    }
}

#[test]
fn reader_never_creates_repairs_or_brands_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    assert!(NoteIndexReader::open(&path).is_err());
    assert!(!path.exists());
    std::fs::write(&path, b"foreign sentinel").unwrap();
    assert!(NoteIndexReader::open(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign sentinel");
    std::fs::remove_file(&path).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("CREATE TABLE foreign_data(x)", []).unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    assert!(NoteIndexReader::open(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn writer_brands_new_main_file_before_readers_attach_or_process_can_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (writer, _) = NoteIndex::open(&path).unwrap();
    let reader = NoteIndexReader::open(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        i32::from_be_bytes(bytes[68..72].try_into().unwrap()),
        0x4252_4e49
    );
    // Another writer must recognize the BRNI file without relaxing foreign-file checks.
    let (_, rebuilt) = NoteIndex::open(&path).unwrap();
    assert!(!rebuilt);
    drop(reader);
    drop(writer);
}

#[test]
fn reader_sees_writer_commits_and_refuses_same_dimension_different_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut writer, _) = NoteIndex::open(&path).unwrap();
    let reader = NoteIndexReader::open(&path).unwrap();
    let text = "apple";
    writer
        .upsert_note(
            &IndexedNote {
                path: "a.md".into(),
                title: "A".into(),
                size: text.len() as u64,
                modified_ns: 1,
                sha256: Sha256::digest(text.as_bytes()).into(),
            },
            text,
        )
        .unwrap();
    writer.embed_pending(&mut Fake, 5).unwrap();
    assert_eq!(reader.notes().unwrap(), writer.notes().unwrap());
    assert_eq!(
        reader.keyword("apple", 10).unwrap(),
        writer.keyword("apple", 10).unwrap()
    );
    assert_eq!(
        reader
            .semantic_for_model(&[1.0, 0.5], "model-a", 10)
            .unwrap()
            .len(),
        1
    );
    assert!(reader.semantic_for_model(&[1.0], "model-a", 10).is_err());
    assert!(matches!(
        reader.semantic_for_model(&[1.0, 0.5], "model-b", 10),
        Err(brn_retrieval::Error::ModelMismatch)
    ));
    writer.use_embedding_model("model-b", 2).unwrap();
    assert!(
        reader
            .semantic_for_model(&[1.0, 0.5], "model-a", 10)
            .is_err()
    );
}
