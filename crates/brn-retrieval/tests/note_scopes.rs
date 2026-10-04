use brn_retrieval::{
    Error,
    note_index::{
        Embedder, IndexedNote, KnowledgeScope, NoteHit, NoteIndex, NoteIndexReader, NoteMetadata,
        NoteSearch,
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{cell::Cell, collections::BTreeMap, path::Path};
use uuid::Uuid;

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 9,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

struct Fake;
impl Embedder for Fake {
    fn identity(&self) -> &str {
        "scoped-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| {
                if text.contains("current") {
                    vec![0.6, 0.8]
                } else {
                    vec![1.0, 0.0]
                }
            })
            .collect())
    }
}

fn exact(
    hits: &[NoteHit],
    texts: &BTreeMap<String, String>,
    scope: KnowledgeScope,
    metadata: &BTreeMap<String, NoteMetadata>,
) {
    for hit in hits {
        let text = &texts[&hit.path];
        let meta = &metadata[&hit.path];
        assert!(meta.issue.is_none() && scope.includes(meta.source, meta.history));
        assert_eq!(hit.note_sha256, note(&hit.path, text).sha256);
        assert_eq!(&text[hit.start_byte..hit.end_byte], hit.quote);
    }
}

#[test]
fn scopes_filter_before_limits_with_exact_quotes_reader_parity_and_duplicate_id_observations() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    let id = Uuid::new_v4();
    let mut texts = BTreeMap::new();
    let mut metadata = BTreeMap::new();
    for (label, source, history, count) in [
        ("source", true, false, 30),
        ("history", false, true, 30),
        ("source-history", true, true, 1),
        ("current", false, false, 3),
    ] {
        for i in 0..count {
            let relative = format!("{label}-{i}.md");
            let text = format!("\u{feff}Apple õ\r\n正文 日本語 {label}-{i}\r\n");
            let meta = NoteMetadata {
                note_id: Some(id),
                source,
                history,
                issue: None,
            };
            index
                .upsert_note_with_metadata(&note(&relative, &text), &text, &meta)
                .unwrap();
            texts.insert(relative.clone(), text);
            metadata.insert(relative, meta);
        }
    }
    let issue = NoteMetadata {
        issue: Some("uninterpretable caller metadata".into()),
        ..NoteMetadata::default()
    };
    index
        .upsert_note_with_metadata(&note("issue.md", "apple"), "apple", &issue)
        .unwrap();
    metadata.insert("issue.md".into(), issue.clone());
    texts.insert("issue.md".into(), "apple".into());
    index.embed_pending(&mut Fake, 100).unwrap();
    let reader = NoteIndexReader::open(&path).unwrap();
    for (scope, count, name) in [
        (KnowledgeScope::Current, 3, "current"),
        (KnowledgeScope::Source, 31, "source"),
        (KnowledgeScope::History, 31, "history"),
        (KnowledgeScope::All, 64, "all"),
    ] {
        assert_eq!(serde_json::to_value(scope).unwrap(), name);
        assert_eq!(
            serde_json::from_value::<KnowledgeScope>(serde_json::json!(name)).unwrap(),
            scope
        );
        let listed = index.notes_scoped(scope).unwrap();
        assert_eq!(listed.len(), count);
        assert_eq!(reader.notes_scoped(scope).unwrap(), listed);
        let keyword = index.keyword_scoped("apple", 3, scope).unwrap();
        assert_eq!(keyword.len(), 3);
        assert_eq!(reader.keyword_scoped("apple", 3, scope).unwrap(), keyword);
        for search in [&index as &dyn NoteSearch, &reader as &dyn NoteSearch] {
            assert_eq!(search.keyword_scoped("apple", 3, scope).unwrap(), keyword);
        }
        exact(&keyword, &texts, scope, &metadata);
        let semantic = index
            .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 3, scope)
            .unwrap();
        assert_eq!(semantic.len(), 3);
        assert_eq!(
            reader
                .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 3, scope)
                .unwrap(),
            semantic
        );
        for search in [&index as &dyn NoteSearch, &reader as &dyn NoteSearch] {
            assert_eq!(
                search
                    .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 3, scope)
                    .unwrap(),
                semantic
            );
        }
        exact(&semantic, &texts, scope, &metadata);
    }
    assert_eq!(index.notes().unwrap().len(), 3);
    assert_eq!(reader.notes().unwrap(), index.notes().unwrap());
    assert_eq!(index.all_notes().unwrap().len(), 65);
    for (path, metadata) in metadata {
        assert_eq!(index.note_metadata(&path).unwrap(), Some(metadata.clone()));
        assert_eq!(reader.note_metadata(&path).unwrap(), Some(metadata));
    }
    assert_eq!(reader.note_metadata("absent.md").unwrap(), None);
    assert_eq!(
        index.keyword("apple", 3).unwrap(),
        index
            .keyword_scoped("apple", 3, KnowledgeScope::Current)
            .unwrap()
    );
    assert_eq!(
        index.semantic(&[1.0, 0.0], 3).unwrap(),
        index
            .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 3, KnowledgeScope::Current)
            .unwrap()
    );
}

fn snapshots(path: &Path) -> Vec<(i64, String, Vec<u8>)> {
    let conn = Connection::open(path).unwrap();
    let mut statement = conn.prepare("SELECT p.id, p.text, e.vector FROM passages p JOIN embeddings e ON e.passage_id=p.id ORDER BY p.id").unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn metadata_only_updates_preserve_passages_vectors_and_reject_nil_ids_atomically() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("index.sqlite");
    let (mut writer, _) = NoteIndex::open(&path).unwrap();
    let text = "\u{feff}apple current õ\r\n日本語";
    writer.upsert_note(&note("a.md", text), text).unwrap();
    writer.embed_pending(&mut Fake, 20).unwrap();
    let snapshot = snapshots(&path);
    let progress = writer.embedding_progress().unwrap();
    let id = Uuid::new_v4();
    let classified = NoteMetadata {
        note_id: Some(id),
        source: true,
        history: true,
        issue: None,
    };
    let reader = NoteIndexReader::open(&path).unwrap();
    writer.update_note_metadata("a.md", &classified).unwrap();
    assert_eq!(
        reader.note_metadata("a.md").unwrap(),
        Some(classified.clone())
    );
    assert!(reader.keyword("apple", 5).unwrap().is_empty());
    assert_eq!(
        reader
            .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 5, KnowledgeScope::Source)
            .unwrap()
            .len(),
        1
    );
    writer
        .update_note_metadata(
            "a.md",
            &NoteMetadata {
                issue: Some("".into()),
                ..classified.clone()
            },
        )
        .unwrap();
    assert!(writer.notes_scoped(KnowledgeScope::All).unwrap().is_empty());
    assert!(
        writer
            .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 5, KnowledgeScope::All)
            .unwrap()
            .is_empty()
    );
    assert_eq!(writer.all_notes().unwrap().len(), 1);
    let valid = NoteMetadata {
        note_id: Some(id),
        ..NoteMetadata::default()
    };
    writer.update_note_metadata("a.md", &valid).unwrap();
    writer
        .update_metadata("a.md", text.len() as u64, 99)
        .unwrap();
    assert_eq!(reader.keyword("apple", 5).unwrap().len(), 1);
    assert_eq!(
        reader
            .semantic_for_model(&[1.0, 0.0], "scoped-model", 5)
            .unwrap()
            .len(),
        1
    );
    let invalid = NoteMetadata {
        note_id: Some(Uuid::nil()),
        ..NoteMetadata::default()
    };
    assert!(matches!(
        writer.update_note_metadata("a.md", &invalid),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        writer.upsert_note_with_metadata(&note("a.md", "changed"), "changed", &invalid),
        Err(Error::Invalid(_))
    ));
    assert_eq!(writer.note_metadata("a.md").unwrap(), Some(valid));
    assert_eq!(snapshots(&path), snapshot);
    assert_eq!(writer.embedding_progress().unwrap(), progress);
    drop(reader);
    drop(writer);
    let (writer, rebuilt) = NoteIndex::open(&path).unwrap();
    assert!(!rebuilt);
    assert_eq!(snapshots(&path), snapshot);
    assert_eq!(
        writer.note_metadata("a.md").unwrap().unwrap().note_id,
        Some(id)
    );
}

#[test]
fn schema_one_and_deleted_index_rebuild_without_adopting_stale_metadata() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("index.sqlite");
    let (mut writer, _) = NoteIndex::open(&path).unwrap();
    writer
        .upsert_note(&note("old.md", "apple"), "apple")
        .unwrap();
    drop(writer);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TABLE edges; ALTER TABLE notes DROP COLUMN note_id; ALTER TABLE notes DROP COLUMN metadata_issue; PRAGMA user_version=1").unwrap();
    drop(raw);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        NoteIndexReader::open(&path),
        Err(Error::Corrupt(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let (mut writer, rebuilt) = NoteIndex::open(&path).unwrap();
    assert!(rebuilt);
    assert!(writer.all_notes().unwrap().is_empty());
    let metadata = NoteMetadata {
        source: true,
        history: true,
        ..NoteMetadata::default()
    };
    writer
        .upsert_note_with_metadata(&note("archive/a.md", "apple"), "apple", &metadata)
        .unwrap();
    assert_eq!(
        writer.notes_scoped(KnowledgeScope::History).unwrap().len(),
        1
    );
    assert_eq!(
        writer.note_metadata("archive/a.md").unwrap(),
        Some(metadata)
    );
    drop(writer);
    std::fs::remove_file(&path).unwrap();
    let (writer, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    assert!(writer.all_notes().unwrap().is_empty());
    assert_eq!(
        Connection::open(&path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn malformed_metadata_rows_and_schema_refuse_readers_then_rebuild_only_branded_index() {
    for sql in [
        "UPDATE notes SET note_id='not-a-uuid'",
        "UPDATE notes SET note_id='00000000-0000-0000-0000-000000000000'",
        "PRAGMA ignore_check_constraints=ON; UPDATE notes SET source=2; PRAGMA ignore_check_constraints=OFF",
        "PRAGMA ignore_check_constraints=ON; UPDATE notes SET history=-1; PRAGMA ignore_check_constraints=OFF",
        "UPDATE notes SET metadata_issue=x'ff'",
        "ALTER TABLE notes RENAME COLUMN source TO unexpected",
    ] {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("index.sqlite");
        let (mut writer, _) = NoteIndex::open(&path).unwrap();
        writer.upsert_note(&note("a.md", "apple"), "apple").unwrap();
        drop(writer);
        let raw = Connection::open(&path).unwrap();
        raw.execute_batch(sql).unwrap();
        drop(raw);
        let before = std::fs::read(&path).unwrap();
        assert!(NoteIndexReader::open(&path).is_err(), "{sql}");
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let (writer, rebuilt) = NoteIndex::open(&path).unwrap();
        assert!(rebuilt, "{sql}");
        assert!(writer.all_notes().unwrap().is_empty());
    }
}

#[test]
fn semantic_scope_excludes_vectors_before_loading_and_preserves_model_guards() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("index.sqlite");
    let (mut writer, _) = NoteIndex::open(&path).unwrap();
    writer
        .upsert_note(&note("current.md", "apple current"), "apple current")
        .unwrap();
    writer
        .upsert_note_with_metadata(
            &note("source.md", "apple source"),
            "apple source",
            &NoteMetadata {
                source: true,
                ..NoteMetadata::default()
            },
        )
        .unwrap();
    writer.embed_pending(&mut Fake, 5).unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute("UPDATE embeddings SET vector=?1 WHERE passage_id IN (SELECT id FROM passages WHERE path='source.md')", params![vec![0_u8]]).unwrap();
    let reader = NoteIndexReader::open(&path).unwrap();
    assert_eq!(
        reader
            .semantic_for_model(&[1.0, 0.0], "scoped-model", 5)
            .unwrap()[0]
            .path,
        "current.md"
    );
    assert!(matches!(
        reader.semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 5, KnowledgeScope::Source),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        reader.semantic_for_model_scoped(&[1.0], "scoped-model", 5, KnowledgeScope::Current),
        Err(Error::ModelMismatch)
    ));
    assert!(matches!(
        reader.semantic_for_model_scoped(&[1.0, 0.0], "wrong-model", 5, KnowledgeScope::All),
        Err(Error::ModelMismatch)
    ));
    writer
        .update_note_metadata(
            "source.md",
            &NoteMetadata {
                source: true,
                issue: Some("invalid classification".into()),
                ..NoteMetadata::default()
            },
        )
        .unwrap();
    assert_eq!(
        reader
            .semantic_for_model_scoped(&[1.0, 0.0], "scoped-model", 5, KnowledgeScope::All)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn old_search_implementations_accept_current_but_never_fallback_for_other_scopes() {
    struct Old {
        keyword: Cell<usize>,
        semantic: Cell<usize>,
    }
    impl NoteSearch for Old {
        fn keyword(&self, _: &str, _: usize) -> brn_retrieval::Result<Vec<NoteHit>> {
            self.keyword.set(self.keyword.get() + 1);
            Ok(Vec::new())
        }
        fn semantic_for_model(
            &self,
            _: &[f32],
            _: &str,
            _: usize,
        ) -> brn_retrieval::Result<Vec<NoteHit>> {
            self.semantic.set(self.semantic.get() + 1);
            Ok(Vec::new())
        }
    }
    let old = Old {
        keyword: Cell::new(0),
        semantic: Cell::new(0),
    };
    assert!(
        old.keyword_scoped("apple", 3, KnowledgeScope::Current)
            .unwrap()
            .is_empty()
    );
    assert!(
        old.semantic_for_model_scoped(&[1.0], "model", 3, KnowledgeScope::Current)
            .unwrap()
            .is_empty()
    );
    for scope in [
        KnowledgeScope::Source,
        KnowledgeScope::History,
        KnowledgeScope::All,
    ] {
        assert!(matches!(
            old.keyword_scoped("apple", 3, scope),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            old.semantic_for_model_scoped(&[1.0], "model", 3, scope),
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(old.keyword.get(), 1);
    assert_eq!(old.semantic.get(), 1);
}
