use brn_retrieval::{Error, note_index::*};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};
use uuid::Uuid;

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn add(index: &mut NoteIndex, path: &str, text: &str, metadata: &NoteMetadata) -> EdgeEndpoint {
    let note = IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    };
    index
        .upsert_note_with_metadata(&note, text, metadata)
        .unwrap();
    EdgeEndpoint {
        path: path.into(),
        note_id: metadata.note_id.unwrap(),
        sha256: note.sha256,
    }
}
fn metadata() -> NoteMetadata {
    NoteMetadata {
        note_id: Some(Uuid::new_v4()),
        ..NoteMetadata::default()
    }
}
fn edge(
    source: &EdgeEndpoint,
    target: &EdgeEndpoint,
    origin: EdgeOrigin,
    text: &str,
    start: usize,
    end: usize,
) -> NoteEdge {
    NoteEdge {
        source: source.clone(),
        target: target.clone(),
        origin,
        evidence: vec![EdgeEvidence {
            endpoint: if origin == EdgeOrigin::ExplicitLink {
                EvidenceEndpoint::Source
            } else {
                EvidenceEndpoint::Target
            },
            start_byte: start,
            end_byte: end,
            quote: text[start..end].into(),
        }],
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    index: NoteIndex,
    source: EdgeEndpoint,
    target: EdgeEndpoint,
    source_text: String,
    target_text: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = fixture();
        let path = dir.path().join("index.sqlite");
        let (mut index, _) = NoteIndex::open(&path).unwrap();
        let source_text = format!(
            "\u{feff}{}õ λ\r\n[link](b.md)\r\n{}",
            "a".repeat(1590),
            "s".repeat(1700)
        );
        let target_text = format!("\u{feff}Exact target 日本語 λ\r\n{}", "t".repeat(1800));
        let source = add(&mut index, "a.md", &source_text, &metadata());
        let target = add(&mut index, "b.md", &target_text, &metadata());
        Self {
            _dir: dir,
            path,
            index,
            source,
            target,
            source_text,
            target_text,
        }
    }
    fn edges(&self) -> Vec<NoteEdge> {
        vec![
            edge(
                &self.source,
                &self.target,
                EdgeOrigin::ExplicitLink,
                &self.source_text,
                1593,
                self.source_text.len(),
            ),
            edge(
                &self.source,
                &self.target,
                EdgeOrigin::InferredProvenance,
                &self.target_text,
                0,
                self.target_text.len(),
            ),
        ]
    }
}

#[test]
fn exact_cross_passage_proofs_origins_reader_parity_and_safe_pagination() {
    let mut f = Fixture::new();
    let expected = f.edges();
    f.index
        .replace_edges(&[expected[1].clone(), expected[0].clone()])
        .unwrap();
    let reader = NoteIndexReader::open(&f.path).unwrap();
    let page = f.index.edges(KnowledgeScope::Current, 0, 200).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.edges, expected);
    assert_eq!(reader.edges(KnowledgeScope::Current, 0, 200).unwrap(), page);
    let second = reader.edges(KnowledgeScope::Current, 1, 1).unwrap();
    assert_eq!(second.edges, vec![expected[1].clone()]);
    assert_eq!(second.total, 2);
    let far = reader.edges(KnowledgeScope::All, usize::MAX, 1).unwrap();
    assert_eq!(far.offset, usize::MAX);
    assert_eq!(far.total, 2);
    assert!(far.edges.is_empty());
    for limit in [0, 201, usize::MAX] {
        assert!(matches!(
            f.index.edges(KnowledgeScope::All, 0, limit),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            reader.edges(KnowledgeScope::All, 0, limit),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn invalid_or_failed_replacement_preserves_the_complete_prior_cache() {
    let mut f = Fixture::new();
    let original = f.edges();
    f.index.replace_edges(&original).unwrap();
    let baseline = f.index.edges(KnowledgeScope::All, 0, 200).unwrap();
    let mut invalids = Vec::new();
    let mut invalid = original[0].clone();
    invalid.source.note_id = Uuid::nil();
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.target.note_id = invalid.source.note_id;
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.target.path = invalid.source.path.clone();
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.source.path.clear();
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.target.sha256[0] ^= 1;
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.target.note_id = Uuid::new_v4();
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence.clear();
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence[0].endpoint = EvidenceEndpoint::Target;
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence[0].quote.push('λ');
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence[0].start_byte += 1;
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence[0].end_byte = usize::MAX;
    invalids.push(invalid);
    let mut invalid = original[0].clone();
    invalid.evidence.push(invalid.evidence[0].clone());
    invalids.push(invalid);
    for invalid in invalids {
        assert!(f.index.replace_edges(&[invalid]).is_err());
        assert_eq!(
            f.index.edges(KnowledgeScope::All, 0, 200).unwrap(),
            baseline
        );
    }
    assert!(
        f.index
            .replace_edges(&[original[0].clone(), original[0].clone()])
            .is_err()
    );
    assert_eq!(
        f.index.edges(KnowledgeScope::All, 0, 200).unwrap(),
        baseline
    );
    let raw = Connection::open(&f.path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_edges BEFORE INSERT ON edges BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(f.index.replace_edges(&[original[0].clone()]).is_err());
    assert_eq!(
        f.index.edges(KnowledgeScope::All, 0, 200).unwrap(),
        baseline
    );
    raw.execute_batch("DROP TRIGGER fail_edges;").unwrap();
    f.index.replace_edges(&[]).unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
}

#[test]
fn per_edge_proof_and_aggregate_quote_limits_have_exact_boundaries() {
    let mut f = Fixture::new();
    let text = "a".repeat(1024 * 1024);
    f.source = add(
        &mut f.index,
        "a.md",
        &text,
        &NoteMetadata {
            note_id: Some(f.source.note_id),
            ..NoteMetadata::default()
        },
    );
    let mut candidate = edge(&f.source, &f.target, EdgeOrigin::ExplicitLink, &text, 0, 1);
    candidate.evidence = (0..8192)
        .map(|start| EdgeEvidence {
            endpoint: EvidenceEndpoint::Source,
            start_byte: start,
            end_byte: start + 512,
            quote: text[start..start + 512].into(),
        })
        .collect();
    f.index.replace_edges(&[candidate.clone()]).unwrap();
    let baseline = f.index.edges(KnowledgeScope::All, 0, 1).unwrap();
    let mut too_many = candidate.clone();
    too_many.evidence.push(EdgeEvidence {
        endpoint: EvidenceEndpoint::Source,
        start_byte: 8192,
        end_byte: 8193,
        quote: "a".into(),
    });
    assert!(f.index.replace_edges(&[too_many]).is_err());
    let mut too_long = candidate;
    for proof in &mut too_long.evidence {
        proof.end_byte += 1;
        proof.quote.push('a');
    }
    assert!(f.index.replace_edges(&[too_long]).is_err());
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 1).unwrap(), baseline);
}

#[test]
fn both_endpoint_scopes_filter_before_limit_and_total() {
    let dir = fixture();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    let mut edges = Vec::new();
    let mut current = None;
    let mut source = None;
    for (label, is_source, history, count) in [
        ("a-source", true, false, 30),
        ("b-history", false, true, 30),
        ("z-current", false, false, 3),
    ] {
        for i in 0..count {
            let m = NoteMetadata {
                source: is_source,
                history,
                ..metadata()
            };
            let a = add(&mut index, &format!("{label}-{i}-a.md"), "exact λ", &m);
            let b = add(
                &mut index,
                &format!("{label}-{i}-b.md"),
                "target",
                &NoteMetadata {
                    note_id: Some(Uuid::new_v4()),
                    ..m
                },
            );
            if i == 0 && is_source {
                source = Some(b.clone());
            }
            if i == 0 && !is_source && !history {
                current = Some(a.clone());
            }
            edges.push(edge(
                &a,
                &b,
                EdgeOrigin::ExplicitLink,
                "exact λ",
                0,
                "exact λ".len(),
            ));
        }
    }
    edges.push(edge(
        &current.unwrap(),
        &source.unwrap(),
        EdgeOrigin::ExplicitLink,
        "exact λ",
        0,
        "exact λ".len(),
    ));
    index.replace_edges(&edges).unwrap();
    let reader = NoteIndexReader::open(&path).unwrap();
    for (scope, total, prefix) in [
        (KnowledgeScope::Current, 3, "z-current"),
        (KnowledgeScope::Source, 30, "a-source"),
        (KnowledgeScope::History, 30, "b-history"),
        (KnowledgeScope::All, 64, "a-source"),
    ] {
        let page = index.edges(scope, 0, 1).unwrap();
        assert_eq!(page.total, total);
        assert_eq!(page.edges.len(), 1);
        assert!(page.edges[0].source.path.starts_with(prefix));
        assert_eq!(reader.edges(scope, 0, 1).unwrap(), page);
    }
}

#[test]
fn changed_identity_eligibility_size_alias_and_removal_invalidate_touching_edges() {
    let mut f = Fixture::new();
    let edges = f.edges();
    f.index.replace_edges(&edges).unwrap();
    let unchanged = f.index.note_metadata("a.md").unwrap().unwrap();
    f.index.update_note_metadata("a.md", &unchanged).unwrap();
    f.index
        .update_metadata("a.md", f.source_text.len() as u64, 999)
        .unwrap();
    assert_eq!(
        f.index.edges(KnowledgeScope::All, 0, 200).unwrap().edges,
        edges
    );
    f.index
        .update_note_metadata(
            "a.md",
            &NoteMetadata {
                source: true,
                ..unchanged.clone()
            },
        )
        .unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    f.index.update_note_metadata("a.md", &unchanged).unwrap();
    f.index.replace_edges(&edges).unwrap();
    let alias = add(
        &mut f.index,
        "alias.md",
        "alias",
        &NoteMetadata {
            note_id: Some(f.target.note_id),
            issue: Some("classification is unresolved".into()),
            ..NoteMetadata::default()
        },
    );
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    assert!(f.index.replace_edges(&edges).is_err());
    f.index.remove_note(&alias.path).unwrap();
    f.index.replace_edges(&edges).unwrap();
    f.index
        .update_metadata("a.md", f.source_text.len() as u64 + 1, 999)
        .unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    f.index
        .update_metadata("a.md", f.source_text.len() as u64, 999)
        .unwrap();
    f.index.replace_edges(&edges).unwrap();
    f.index.remove_note("b.md").unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
}

#[test]
fn absent_note_metadata_update_remains_a_noop_for_existing_edges() {
    let mut f = Fixture::new();
    let edges = f.edges();
    f.index.replace_edges(&edges).unwrap();
    let metadata = f.index.note_metadata("a.md").unwrap().unwrap();
    f.index
        .update_note_metadata("absent.md", &metadata)
        .unwrap();
    assert!(f.index.note_metadata("absent.md").unwrap().is_none());
    assert_eq!(
        f.index.edges(KnowledgeScope::All, 0, 200).unwrap().edges,
        edges
    );
}

#[test]
fn metadata_invalidation_and_failed_update_share_one_transaction() {
    let mut f = Fixture::new();
    let edges = f.edges();
    f.index.replace_edges(&edges).unwrap();
    let previous = f.index.note_metadata("a.md").unwrap().unwrap();
    let mut changed = previous.clone();
    changed.note_id = Some(Uuid::new_v4());
    let raw = Connection::open(&f.path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_metadata BEFORE UPDATE ON notes BEGIN SELECT RAISE(ABORT,'synthetic metadata failure'); END;").unwrap();
    assert!(f.index.update_note_metadata("a.md", &changed).is_err());
    assert_eq!(
        f.index.note_metadata("a.md").unwrap(),
        Some(previous.clone())
    );
    assert_eq!(
        f.index.edges(KnowledgeScope::All, 0, 200).unwrap().edges,
        edges
    );
    raw.execute_batch("DROP TRIGGER fail_metadata;").unwrap();
    f.index.update_note_metadata("a.md", &changed).unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    assert!(f.index.replace_edges(&edges).is_err());
    f.index.update_note_metadata("a.md", &previous).unwrap();
    f.index.replace_edges(&edges).unwrap();
    let mut ineligible = previous;
    ineligible.issue = Some("unresolved identity".into());
    f.index.update_note_metadata("a.md", &ineligible).unwrap();
    assert_eq!(f.index.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    assert!(f.index.replace_edges(&edges).is_err());
}

struct Fake;
impl Embedder for Fake {
    fn identity(&self) -> &str {
        "edge-test-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![1.0, 0.0]).collect())
    }
}
fn vectors(path: &Path) -> Vec<(i64, String, Vec<u8>)> {
    let raw = Connection::open(path).unwrap();
    raw.prepare("SELECT p.id,p.text,e.vector FROM passages p JOIN embeddings e ON e.passage_id=p.id ORDER BY p.id").unwrap().query_map([],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}
#[test]
fn healthy_v2_upgrade_preserves_exact_passages_vectors_and_reader_refuses_until_upgrade() {
    let mut f = Fixture::new();
    f.index.embed_pending(&mut Fake, 100).unwrap();
    let before = vectors(&f.path);
    drop(f.index);
    let raw = Connection::open(&f.path).unwrap();
    raw.execute_batch("DROP TABLE edges; PRAGMA user_version=2;")
        .unwrap();
    drop(raw);
    let bytes = std::fs::read(&f.path).unwrap();
    assert!(matches!(
        NoteIndexReader::open(&f.path),
        Err(Error::Corrupt(_))
    ));
    assert_eq!(std::fs::read(&f.path).unwrap(), bytes);
    let (writer, rebuilt) = NoteIndex::open(&f.path).unwrap();
    assert!(!rebuilt);
    assert_eq!(vectors(&f.path), before);
    assert_eq!(writer.embedding_progress().unwrap().embedded, before.len());
    assert_eq!(writer.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    let reader = NoteIndexReader::open(&f.path).unwrap();
    assert!(
        !reader
            .semantic_for_model(&[1.0, 0.0], "edge-test-model", 5)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        Connection::open(&f.path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn malformed_edge_rows_schema_and_full_passage_proofs_refuse_reader_then_rebuild_writer() {
    for mutation in [
        "UPDATE edges SET payload='{}'",
        "UPDATE edges SET origin='untrusted' WHERE origin='explicit_link'",
        "UPDATE edges SET source_path='missing.md'",
        "UPDATE notes SET metadata_issue='bad' WHERE path='a.md'",
        "UPDATE passages SET text='changed' WHERE path='a.md' AND start_byte=0",
        "UPDATE passages SET start_byte=1 WHERE path='a.md' AND start_byte=0",
        "ALTER TABLE edges RENAME COLUMN payload TO unexpected",
        "DROP TABLE edges",
        "DROP TABLE edges; CREATE TABLE edges(source_path TEXT NOT NULL,target_path TEXT NOT NULL,origin TEXT NOT NULL,payload TEXT NOT NULL,PRIMARY KEY(source_path,target_path,origin))",
    ] {
        let mut f = Fixture::new();
        f.index.replace_edges(&f.edges()).unwrap();
        drop(f.index);
        let raw = Connection::open(&f.path).unwrap();
        raw.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        raw.execute_batch(mutation).unwrap();
        raw.execute_batch("PRAGMA ignore_check_constraints=OFF;")
            .unwrap();
        drop(raw);
        let bytes = std::fs::read(&f.path).unwrap();
        assert!(
            matches!(NoteIndexReader::open(&f.path), Err(Error::Corrupt(_))),
            "{mutation}"
        );
        assert_eq!(std::fs::read(&f.path).unwrap(), bytes, "{mutation}");
        let (writer, rebuilt) = NoteIndex::open(&f.path).unwrap();
        assert!(rebuilt, "{mutation}");
        assert!(writer.all_notes().unwrap().is_empty());
        assert_eq!(writer.edges(KnowledgeScope::All, 0, 200).unwrap().total, 0);
    }
}

#[test]
fn retained_reader_rechecks_current_payload_and_pagination_is_one_snapshot() {
    let mut f = Fixture::new();
    let edges = f.edges();
    f.index.replace_edges(&edges).unwrap();
    let reader = NoteIndexReader::open(&f.path).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = barrier.clone();
    let mut writer = f.index;
    let join = std::thread::spawn(move || {
        writer_barrier.wait();
        for _ in 0..100 {
            writer.replace_edges(&[]).unwrap();
            writer.replace_edges(&edges).unwrap();
        }
        writer
    });
    barrier.wait();
    let mut samples = 0;
    for _ in 0..500 {
        let page = reader.edges(KnowledgeScope::All, 0, 200).unwrap();
        assert_eq!(page.total, page.edges.len());
        assert!(page.total == 0 || page.total == 2);
        samples += 1;
        if join.is_finished() {
            break;
        }
    }
    let _writer = join.join().unwrap();
    assert!(samples > 0);
    Connection::open(&f.path)
        .unwrap()
        .execute("UPDATE edges SET payload=?1", params!["{\"unknown\":true}"])
        .unwrap();
    assert!(matches!(
        reader.edges(KnowledgeScope::All, 0, 200),
        Err(Error::Corrupt(_))
    ));
}
