use brn_workflow::library::{
    Embedder, EmbeddingProgress, Library, RefreshReport, SearchMode, Unreadable,
};
use std::{
    path::Path,
    time::{Duration, SystemTime},
};

#[test]
fn shared_embedder_caches_metadata_and_reports_poisoning_without_downgrade() {
    use brn_workflow::library::SharedEmbedder;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct PanicModel(Arc<AtomicUsize>);
    impl Embedder for PanicModel {
        fn identity(&self) -> &str {
            self.0.fetch_add(1, Ordering::SeqCst);
            "synthetic-poison"
        }
        fn dimension(&self) -> usize {
            self.0.fetch_add(1, Ordering::SeqCst);
            2
        }
        fn embed(&mut self, _: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
            panic!("synthetic model panic")
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut shared = SharedEmbedder::new(Box::new(PanicModel(calls.clone())));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let mut worker = shared.clone();
    assert!(
        std::thread::spawn(move || worker.embed(&["apple"]))
            .join()
            .is_err()
    );
    assert_eq!(shared.identity(), "synthetic-poison");
    assert_eq!(shared.dimension(), 2);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "metadata never locks or re-enters the model"
    );
    assert!(matches!(
        shared.embed(&["apple"]),
        Err(brn_retrieval::Error::EmbedderPoisoned)
    ));
}

/// Three topics: fruit, vehicles, everything else.
struct TopicEmbedder;

impl Embedder for TopicEmbedder {
    fn identity(&self) -> &str {
        "test-topics-v1"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
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

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn bump_mtime(root: &Path, relative: &str) {
    let file = std::fs::File::options()
        .write(true)
        .open(root.join(relative))
        .unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(60))
        .unwrap();
}

struct Setup {
    vault: tempfile::TempDir,
    data: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        Self {
            vault: tempfile::tempdir().unwrap(),
            data: tempfile::tempdir().unwrap(),
        }
    }
    fn open(&self, embedder: Option<Box<dyn Embedder + Send>>) -> Library {
        Library::open(
            self.vault.path(),
            &self.data.path().join("index.sqlite"),
            embedder,
        )
        .unwrap()
    }
}

#[test]
fn refresh_tracks_added_changed_unchanged_and_removed_notes() {
    let s = Setup::new();
    let root = s.vault.path();
    write(root, "a.md", b"# Alpha plan\nfirst");
    write(root, "sub/b.md", b"second");
    write(root, "archive/old.md", b"archived");
    write(root, ".hidden/h.md", b"hidden");
    let mut library = s.open(None);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 3,
            ..Default::default()
        }
    );
    let notes = library.notes().unwrap();
    assert_eq!(
        notes
            .iter()
            .map(|n| (n.path.as_str(), n.title.as_str()))
            .collect::<Vec<_>>(),
        vec![("a.md", "Alpha plan"), ("sub/b.md", "b")]
    );
    assert_eq!(
        library
            .notes_scoped(brn_workflow::library::KnowledgeScope::History)
            .unwrap()[0]
            .path,
        "archive/old.md"
    );

    write(root, "a.md", b"# Alpha plan\nfirst, edited");
    bump_mtime(root, "a.md");
    std::fs::remove_file(root.join("sub/b.md")).unwrap();
    write(root, "c.md", b"third");
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 1,
            updated: 1,
            removed: 1,
            unchanged: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 3,
            ..Default::default()
        }
    );
}

#[test]
fn title_skips_frontmatter_and_handles_crlf() {
    let s = Setup::new();
    let cases = [
        (
            "frontmatter.md",
            "---\r\ntitle: x\r\nsummary: |\r\n  # not a heading\r\n---\r\n# Real Title\r\nbody",
            "Real Title",
        ),
        ("plain.md", "#NoSpace\nbody", "plain"),
        (
            "bom.md",
            "\u{feff}---\r\n# Not the title\r\n---\r\n# BOM Title\r\nbody",
            "BOM Title",
        ),
        (
            "unclosed.md",
            "---\n# Unclosed Title\nbody",
            "Unclosed Title",
        ),
        ("empty.md", "# \n# Nonempty Title\nbody", "Nonempty Title"),
    ];
    for (path, text, _) in cases {
        write(s.vault.path(), path, text.as_bytes());
    }
    let mut library = s.open(None);
    library.refresh().unwrap();
    let notes = library.notes().unwrap();
    for (path, _, expected) in cases {
        let note = notes.iter().find(|note| note.path == path).unwrap();
        assert_eq!(note.title, expected, "{path}");
    }
}

#[test]
fn touched_but_identical_note_is_not_reindexed() {
    let s = Setup::new();
    write(s.vault.path(), "a.md", b"same");
    let mut library = s.open(None);
    library.refresh().unwrap();
    bump_mtime(s.vault.path(), "a.md");
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 1,
            ..Default::default()
        }
    );
}

#[test]
fn unreadable_notes_are_listed_and_dropped_from_the_index() {
    let s = Setup::new();
    let root = s.vault.path();
    write(root, "bad.md", b"fine at first");
    write(root, "big.md", &vec![b'x'; 1024 * 1024 + 1]);
    let mut library = s.open(None);
    library.refresh().unwrap();
    write(root, "bad.md", &[0xff, 0xfe]);
    bump_mtime(root, "bad.md");
    let report = library.refresh().unwrap();
    assert_eq!(report.removed, 1);
    assert_eq!(
        report.unreadable,
        vec![
            Unreadable {
                path: "bad.md".into(),
                reason: "not valid UTF-8"
            },
            Unreadable {
                path: "big.md".into(),
                reason: "larger than 1 MiB"
            },
        ]
    );
    assert!(library.notes().unwrap().is_empty());
}

#[test]
fn search_without_a_model_is_keyword_only_and_says_so() {
    let s = Setup::new();
    write(s.vault.path(), "fruit.md", b"Apples and bananas.");
    let mut library = s.open(None);
    library.refresh().unwrap();
    assert_eq!(library.embed_pending(10).unwrap(), None);
    let keyword = library.search("apples", SearchMode::Keyword, 10).unwrap();
    assert!(keyword.keyword_only);
    assert_eq!(keyword.hits[0].path, "fruit.md");
    let semantic = library.search("apples", SearchMode::Semantic, 10).unwrap();
    assert!(semantic.keyword_only);
    assert_eq!(semantic.hits[0].path, "fruit.md");
    let hybrid = library.search("apples", SearchMode::Hybrid, 10).unwrap();
    assert!(hybrid.keyword_only);
    assert_eq!(hybrid.hits[0].path, "fruit.md");
}

#[test]
fn semantic_and_hybrid_search_use_embedded_passages() {
    let s = Setup::new();
    write(s.vault.path(), "fruit.md", b"Apples and bananas.");
    write(s.vault.path(), "travel.md", b"Trains and cars.");
    let mut library = s.open(Some(Box::new(TopicEmbedder)));
    library.refresh().unwrap();
    assert!(
        library
            .search("fruit", SearchMode::Semantic, 5)
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(
        library.embed_pending(10).unwrap(),
        Some(EmbeddingProgress {
            embedded: 2,
            total: 2
        })
    );
    let semantic = library.search("fruit", SearchMode::Semantic, 1).unwrap();
    assert!(!semantic.keyword_only);
    assert_eq!(semantic.hits[0].path, "fruit.md");
    let hybrid = library.search("trains", SearchMode::Hybrid, 2).unwrap();
    assert_eq!(hybrid.hits[0].path, "travel.md");
    let keyword = library.search("apples", SearchMode::Keyword, 10).unwrap();
    assert!(!keyword.keyword_only);
}

#[test]
fn deleted_index_is_rebuilt_from_the_vault() {
    let s = Setup::new();
    write(s.vault.path(), "a.md", b"alpha");
    let index = s.data.path().join("index.sqlite");
    {
        let mut library = s.open(None);
        library.refresh().unwrap();
    }
    std::fs::remove_file(&index).unwrap();
    let mut library = s.open(None);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library
            .search("alpha", SearchMode::Keyword, 5)
            .unwrap()
            .hits
            .len(),
        1
    );
}
