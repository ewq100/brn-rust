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
        eprintln!("skipped: set BRN_NATIVE_MODEL_DIR to a local all-MiniLM-L6-v2 folder");
        return;
    };
    let mut embedder = LocalEmbedder::open(std::path::Path::new(&dir)).unwrap();
    assert_eq!(embedder.dimension(), 384);
    assert!(
        embedder
            .identity()
            .starts_with("fastembed-7.1.0/all-MiniLM-L6-v2/mean/384/")
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
