use brn_retrieval::note_index::{IndexedNote, NoteHit, NoteIndex, fuse_hits};
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

fn index_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, NoteIndex) {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    for (path, text) in notes {
        index.upsert_note(&note(path, text), text).unwrap();
    }
    (dir, index)
}

fn paths(hits: &[NoteHit]) -> Vec<&str> {
    hits.iter().map(|h| h.path.as_str()).collect()
}

#[test]
fn matches_whole_words_not_substrings() {
    let (_dir, index) = index_with(&[
        ("token.md", "red blue"),
        ("substring.md", "reddish blueprint"),
    ]);
    assert_eq!(paths(&index.keyword("red", 10).unwrap()), vec!["token.md"]);
}

#[test]
fn ignores_case_and_diacritics() {
    let (_dir, index) = index_with(&[("cafe.md", "Café résumé")]);
    assert_eq!(
        paths(&index.keyword("CAFE resume", 10).unwrap()),
        vec!["cafe.md"]
    );
}

#[test]
fn passages_matching_more_words_rank_first() {
    let (_dir, index) = index_with(&[("one.md", "alpha"), ("both.md", "alpha beta")]);
    assert_eq!(
        paths(&index.keyword("alpha beta", 10).unwrap()),
        vec!["both.md", "one.md"]
    );
}

#[test]
fn query_syntax_is_treated_as_words() {
    let (_dir, index) = index_with(&[("a.md", "alpha"), ("b.md", "beta")]);
    let hits = index.keyword("alpha\" OR NOT (beta* NEAR", 10).unwrap();
    assert_eq!(paths(&hits), vec!["a.md", "b.md"]);
}

#[test]
fn hits_carry_exact_provenance() {
    let text = "intro\n\nThe launch is in October.";
    let (_dir, index) = index_with(&[("plan.md", text)]);
    let hit = &index.keyword("october", 10).unwrap()[0];
    assert_eq!(&text[hit.start_byte..hit.end_byte], hit.quote);
    assert_eq!(
        hit.note_sha256,
        <[u8; 32]>::from(Sha256::digest(text.as_bytes()))
    );
    assert!(hit.score > 0.0);
}

#[test]
fn reindexed_and_removed_text_is_not_found() {
    let (_dir, mut index) = index_with(&[("a.md", "old words")]);
    index
        .upsert_note(&note("a.md", "new words"), "new words")
        .unwrap();
    assert!(index.keyword("old", 10).unwrap().is_empty());
    assert_eq!(paths(&index.keyword("new", 10).unwrap()), vec!["a.md"]);
    index.remove_note("a.md").unwrap();
    assert!(index.keyword("new", 10).unwrap().is_empty());
}

#[test]
fn rejects_bad_queries_and_limits() {
    let (_dir, index) = index_with(&[("a.md", "alpha")]);
    for (query, limit) in [("", 10), ("   ", 10), ("alpha", 0), ("alpha", 51)] {
        assert!(
            matches!(
                index.keyword(query, limit),
                Err(brn_retrieval::Error::Invalid(_))
            ),
            "{query:?} {limit}"
        );
    }
    assert!(matches!(
        index.keyword(&"a".repeat(513), 10),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert!(matches!(
        index.keyword("!!! ???", 10),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert_eq!(index.keyword("alpha", 50).unwrap().len(), 1);
}

#[test]
fn fusion_rewards_agreement_and_breaks_ties_by_passage() {
    let hit = |id: i64| NoteHit {
        passage_id: id,
        path: format!("{id}.md"),
        note_sha256: [0; 32],
        start_byte: 0,
        end_byte: 1,
        quote: "x".into(),
        score: 0.0,
    };
    let keyword = [hit(1), hit(2), hit(3)];
    let semantic = [hit(3), hit(4)];
    let fused = fuse_hits(&[&keyword, &semantic], 10);
    let ids: Vec<i64> = fused.iter().map(|h| h.passage_id).collect();
    assert_eq!(ids, vec![3, 1, 2, 4]);
    assert!((fused[0].score - (1.0 / 63.0 + 1.0 / 61.0)).abs() < 1e-6);
    assert_eq!(fuse_hits(&[&keyword, &semantic], 2).len(), 2);
}
