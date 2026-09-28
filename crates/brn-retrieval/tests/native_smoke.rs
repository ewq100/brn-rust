#![cfg(feature = "native")]
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
fn native_three_profiles_reopen_with_local_model() {
    let model = std::env::var("BRN_NATIVE_MODEL_DIR")
        .unwrap_or_else(|_| "/private/tmp/brn-retrieval-final-state-20260927/model".into());
    if !std::path::Path::new(&model).is_dir() {
        eprintln!("native model unavailable; set BRN_NATIVE_MODEL_DIR");
        return;
    }
    let dir = tempdir().unwrap();
    let path = dir.path().join("generation");
    let docs = [
        doc(
            "launch",
            "The launch window is October. The release checklist is approved.",
        ),
        doc("cafe", "A café has a naïve résumé and a compass 🧭."),
        doc("other", "The support desk has a separate schedule."),
    ];
    let mut index = Index::build(
        &path,
        &docs,
        Some(std::path::Path::new(&model)),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    for profile in [Profile::Keyword, Profile::Semantic, Profile::Hybrid] {
        let hits = index.search("launch window October", profile, 3).unwrap();
        assert!(!hits.is_empty());
        for hit in hits {
            index.verify_hit(&hit).unwrap();
        }
    }
    drop(index);
    let mut reopened = Index::open(&path).unwrap();
    assert!(
        !reopened
            .search("café résumé", Profile::Semantic, 3)
            .unwrap()
            .is_empty()
    );
    assert!(
        !reopened
            .search("launch window", Profile::Hybrid, 3)
            .unwrap()
            .is_empty()
    );
    std::fs::remove_file(path.join("model/tokenizer.json")).unwrap();
    let mut degraded = Index::open(&path).unwrap();
    assert!(
        !degraded
            .search("launch window", Profile::Keyword, 3)
            .unwrap()
            .is_empty()
    );
    assert!(
        degraded
            .search("launch window", Profile::Semantic, 3)
            .is_err()
    );
    assert!(
        degraded
            .search("launch window", Profile::Hybrid, 3)
            .is_err()
    );
    std::fs::write(path.join("model/tokenizer.json"), b"corrupt").unwrap();
    let mut corrupt = Index::open(&path).unwrap();
    assert!(
        !corrupt
            .search("launch", Profile::Keyword, 3)
            .unwrap()
            .is_empty()
    );
    assert!(corrupt.search("launch", Profile::Semantic, 3).is_err());
}
