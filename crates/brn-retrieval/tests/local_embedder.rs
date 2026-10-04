#![cfg(feature = "native")]
use brn_retrieval::{native::LocalEmbedder, note_index::Embedder};

#[test]
fn missing_model_folder_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        LocalEmbedder::open(dir.path()),
        Err(brn_retrieval::Error::Unavailable(_))
    ));
}

/// Needs the five model files in `BRN_NATIVE_MODEL_DIR`; skipped (not passed) otherwise.
#[test]
fn local_model_embeds_by_meaning() {
    let Some(dir) = std::env::var_os("BRN_NATIVE_MODEL_DIR") else {
        eprintln!("skipped: set BRN_NATIVE_MODEL_DIR to an explicitly acquired local model folder");
        return;
    };
    let mut embedder = LocalEmbedder::open(std::path::Path::new(&dir)).unwrap();
    assert_eq!(embedder.dimension(), 384);
    assert!(
        embedder
            .identity()
            .starts_with("fastembed-7.1.0/all-MiniLM-L6-v2/mean/384/")
            || embedder.identity().starts_with(
                "fastembed-7.1.0/paraphrase-multilingual-MiniLM-L12-v2/mean/384/max128/static/"
            )
    );
    let v = embedder
        .embed(&[
            "The release ships in October.",
            "Launch date is next month.",
            "A café menu.",
        ])
        .unwrap();
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    assert!(dot(&v[0], &v[1]) > dot(&v[0], &v[2]));
}

#[test]
#[ignore = "requires explicitly authorized pinned model assets; not an offline fixture"]
fn multilingual_model_matches_english_estonian_paraphrases_and_inflections() {
    let dir = std::env::var_os("BRN_NATIVE_MODEL_DIR")
        .expect("set BRN_NATIVE_MODEL_DIR to the authorized pinned multilingual model folder");
    let mut embedder = LocalEmbedder::open(std::path::Path::new(&dir)).unwrap();
    assert!(embedder.identity().starts_with(
        "fastembed-7.1.0/paraphrase-multilingual-MiniLM-L12-v2/mean/384/max128/static/"
    ));
    let notes = [
        "Anna ootab lõplikku värvispetsifikatsiooni. Enne selle saabumist on töö blokeeritud.",
        "The new software release will ship in October after testing is complete.",
        "Koosolek toimub kolmapäeval kell kümme Tallinna kontoris.",
        "The café menu includes coffee, soup and fresh bread.",
    ];
    let queries = [
        ("Which missing paint specification is blocking the work?", 0),
        ("Kellelt ootame värvi lõplikke nõudeid?", 0),
        ("Millal uus tarkvaraversioon välja antakse?", 1),
        ("When is the software launch scheduled?", 1),
        ("Where and when is Wednesday's meeting?", 2),
        ("Kohviku menüüs olevad toidud", 3),
    ];
    let mut texts = notes.to_vec();
    texts.extend(queries.iter().map(|(query, _)| *query));
    let vectors = embedder.embed(&texts).unwrap();
    assert_eq!(vectors.len(), texts.len());
    for vector in &vectors {
        assert_eq!(vector.len(), 384);
        assert!(vector.iter().all(|value| value.is_finite()));
        let norm = vector.iter().map(|value| value * value).sum::<f32>();
        assert!((norm - 1.0).abs() < 0.002);
    }
    for (i, (query, expected)) in queries.iter().enumerate() {
        let query_vector = &vectors[notes.len() + i];
        let best = (0..notes.len())
            .max_by(|left, right| {
                let score = |note: usize| {
                    query_vector
                        .iter()
                        .zip(&vectors[note])
                        .map(|(a, b)| a * b)
                        .sum::<f32>()
                };
                score(*left).total_cmp(&score(*right))
            })
            .unwrap();
        assert_eq!(best, *expected, "incorrect bilingual result for {query}");
    }
}
