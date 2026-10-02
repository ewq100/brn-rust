use brn_retrieval::note_index::{Embedder, EmbeddingProgress, IndexedNote, NoteIndex, fuse_hits};
use sha2::{Digest, Sha256};

/// Three topics: fruit, vehicles, everything else.
struct TopicEmbedder {
    calls: usize,
}

impl Embedder for TopicEmbedder {
    fn identity(&self) -> &str {
        "test-topics-v1"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.calls += 1;
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0, 0.0, 0.1];
                for word in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
                    match word {
                        "apple" | "apples" | "banana" | "bananas" | "fruit" => v[0] += 1.0,
                        "train" | "trains" | "car" | "cars" | "vehicle" => v[1] += 1.0,
                        _ => {}
                    }
                }
                v
            })
            .collect())
    }
}

struct BrokenEmbedder(Vec<f32>);

impl Embedder for BrokenEmbedder {
    fn identity(&self) -> &str {
        "broken"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| self.0.clone()).collect())
    }
}

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn index_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, NoteIndex) {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    for (path, text) in notes {
        index.upsert_note(&note(path, text), text).unwrap();
    }
    (dir, index)
}

const NOTES: [(&str, &str); 3] = [
    ("fruit.md", "Apples and bananas in the kitchen."),
    ("travel.md", "Trains and cars for the trip."),
    ("other.md", "Meeting notes."),
];

#[test]
fn embeds_pending_passages_in_batches() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 2,
            total: 3
        }
    );
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 3,
            total: 3
        }
    );
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 3,
            total: 3
        }
    );
    assert_eq!(embedder.calls, 2);
}

#[test]
fn semantic_search_finds_by_meaning() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let query = embedder.embed(&["fruit"]).unwrap().remove(0);
    let hits = index.semantic(&query, 2).unwrap();
    assert_eq!(hits[0].path, "fruit.md");
    assert_eq!(hits.len(), 2);
    assert!(index.keyword("fruit", 10).unwrap().is_empty());
}

#[test]
fn hybrid_fusion_combines_keyword_and_semantic() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let keyword = index.keyword("meeting", 50).unwrap();
    let semantic = index
        .semantic(&embedder.embed(&["fruit"]).unwrap().remove(0), 50)
        .unwrap();
    let fused = fuse_hits(&[&keyword, &semantic], 10);
    let top: Vec<&str> = fused.iter().take(2).map(|h| h.path.as_str()).collect();
    assert!(
        top.contains(&"other.md") && top.contains(&"fruit.md"),
        "{top:?}"
    );
}

#[test]
fn model_change_and_reindexing_drop_embeddings() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let text = "Apples only.";
    index.upsert_note(&note("fruit.md", text), text).unwrap();
    assert_eq!(
        index.embedding_progress().unwrap(),
        EmbeddingProgress {
            embedded: 2,
            total: 3
        }
    );
    index.use_embedding_model("test-topics-v2", 3).unwrap();
    assert_eq!(
        index.embedding_progress().unwrap(),
        EmbeddingProgress {
            embedded: 0,
            total: 3
        }
    );
}

#[test]
fn rejects_bad_vectors_and_queries() {
    let (_dir, mut index) = index_with(&NOTES);
    for bad in [
        vec![f32::NAN, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![1.0, 2.0],
    ] {
        assert!(matches!(
            index.embed_pending(&mut BrokenEmbedder(bad), 10),
            Err(brn_retrieval::Error::Invalid(_))
        ));
    }
    assert_eq!(index.embedding_progress().unwrap().embedded, 0);
    assert!(index.semantic(&[1.0, 0.0], 5).is_err());
    assert!(index.semantic(&[1.0, 0.0, 0.0], 0).is_err());
}

#[test]
fn semantic_is_empty_before_any_model() {
    let (_dir, index) = index_with(&NOTES);
    assert!(index.semantic(&[1.0, 0.0, 0.0], 5).unwrap().is_empty());
}
