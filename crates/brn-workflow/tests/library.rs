use brn_workflow::library::{
    Embedder, EmbeddingProgress, KnowledgeScope, Library, RefreshReport, SearchMode, Unreadable,
};
use sha2::{Digest, Sha256};
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

fn refreshed_titles(cases: &[(&str, String, &str)]) -> (Setup, Library) {
    let s = Setup::new();
    for (path, text, _) in cases {
        write(s.vault.path(), path, text.as_bytes());
    }
    let mut library = s.open(None);
    library.refresh().unwrap();
    let notes = library.notes_scoped(KnowledgeScope::All).unwrap();
    for (path, text, expected) in cases {
        let note = notes
            .iter()
            .find(|note| note.path == *path)
            .unwrap_or_else(|| panic!("{path} remains indexed"));
        assert_eq!(note.title, *expected, "{path}");
        let saved = std::fs::read(s.vault.path().join(path)).unwrap();
        assert_eq!(saved, text.as_bytes(), "{path} bytes are unchanged");
        assert_eq!(
            note.sha256,
            <[u8; 32]>::from(Sha256::digest(&saved)),
            "{path}"
        );
    }
    (s, library)
}

#[test]
fn title_ignores_markdown_code_and_html_pseudo_headings() {
    let cases = [
        (
            "backtick.md",
            "```md\n# Fake backtick\n```\n# Real backtick\n".to_owned(),
            "Real backtick",
        ),
        (
            "tilde.md",
            "~~~\r\n# Fake tilde\r\n~~~\r\n# Real tilde\r\n".to_owned(),
            "Real tilde",
        ),
        (
            "unclosed-fence.md",
            "intro\n```\n# Fake unclosed\n".to_owned(),
            "unclosed-fence",
        ),
        (
            "html.md",
            "<div>\n# Fake html\n</div>\n\n<h1>Html heading</h1>\n\n# Real html\n".to_owned(),
            "Real html",
        ),
        (
            "indented-code.md",
            "    # Fake indented\n\n# Real indented\n".to_owned(),
            "Real indented",
        ),
        ("fallback.md", "no heading here\n".to_owned(), "fallback"),
    ];
    refreshed_titles(&cases);
}

#[test]
fn title_keeps_literal_atx_line_policy() {
    let cases = [
        (
            "inline.md",
            "# *Emph* [link](x.md) \\# `code` <b>x</b> ##  \n".to_owned(),
            "*Emph* [link](x.md) \\# `code` <b>x</b> ##",
        ),
        (
            "unicode.md",
            "# Pealkiri ÕÄÖÜ 🦀\nbody".to_owned(),
            "Pealkiri ÕÄÖÜ 🦀",
        ),
        (
            "rejected-forms.md",
            "Setext\n======\n #  Indented\n#\tTab\n#NoSpace\n## Second\n> # Quote\n# \n#   Real literal  \n"
                .to_owned(),
            "Real literal",
        ),
        (
            "setext-only.md",
            "Setext only\n===========\n".to_owned(),
            "setext-only",
        ),
    ];
    refreshed_titles(&cases);
}

#[test]
fn title_skips_supported_headers_and_preserves_compatible_layouts() {
    let cases = [
        (
            "dots.md",
            "---\n# yaml comment\ntitle: x\n...\n# Dots Title\n".to_owned(),
            "Dots Title",
        ),
        (
            "bom-crlf-dots.md",
            "\u{feff}---\r\n# yaml comment\r\n...\r\n```\r\n# Fake\r\n```\r\n# BOM Dots\r\n"
                .to_owned(),
            "BOM Dots",
        ),
        (
            "unmanaged-unclosed.md",
            "---\ntitle: x\n# Unclosed Body Title\n".to_owned(),
            "Unclosed Body Title",
        ),
        (
            "source.md",
            "---\nbrn_kind: source\n---\n```\n# Fake source\n```\n# Source Title\n".to_owned(),
            "Source Title",
        ),
    ];
    let (_setup, library) = refreshed_titles(&cases);
    let current: Vec<_> = library
        .notes()
        .unwrap()
        .into_iter()
        .map(|note| note.path)
        .collect();
    assert!(!current.contains(&"source.md".to_owned()));
    assert!(current.contains(&"dots.md".to_owned()));
    assert_eq!(
        library.notes_scoped(KnowledgeScope::Source).unwrap()[0].path,
        "source.md"
    );
}

#[test]
fn malformed_metadata_titles_do_not_change_refresh_eligibility() {
    let s = Setup::new();
    let cases = [
        (
            "malformed-managed.md",
            "---\n# yaml comment\nbrn_id:bad\n---\n```\n# Fake\n```\n# Malformed Title\n",
        ),
        ("malformed-opening.md", "--- \n# Opening Title\n---\n"),
    ];
    for (path, text) in cases {
        write(s.vault.path(), path, text.as_bytes());
    }
    let mut library = s.open(None);
    let report = library.refresh().unwrap();
    assert_eq!(report.added, 2, "refresh keeps indexing both notes");
    assert_eq!(
        report.unreadable,
        cases
            .iter()
            .map(|(path, _)| Unreadable {
                path: (*path).into(),
                reason: "invalid managed metadata"
            })
            .collect::<Vec<_>>()
    );
    assert!(
        library
            .notes_scoped(KnowledgeScope::All)
            .unwrap()
            .is_empty()
    );
    for (path, text) in cases {
        assert_eq!(
            std::fs::read(s.vault.path().join(path)).unwrap(),
            text.as_bytes()
        );
    }
    assert_eq!(
        library.refresh().unwrap().unchanged,
        2,
        "stable derived titles keep the unchanged fast path"
    );
}

#[test]
fn markdown_parser_failure_falls_back_without_aborting_refresh() {
    // Pinned markdown 1.0.0 panics on this valid setext/thematic-break order.
    let cases = [
        (
            "parser-failure.md",
            "Meeting notes\n-------------\n---\nAgenda\n------\n# Title\n".to_owned(),
            "parser-failure",
        ),
        (
            "failure-after-title.md",
            "# Kept Title\nMeeting notes\n-------------\n---\nAgenda\n------\n".to_owned(),
            "Kept Title",
        ),
        ("ordinary.md", "# Ordinary\n".to_owned(), "Ordinary"),
    ];
    let (s, mut library) = refreshed_titles(&cases);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 3,
            ..Default::default()
        }
    );
    drop(s);
}

#[test]
fn title_parsing_stays_bounded_for_inline_heavy_notes() {
    let line = "**b** [x".repeat(1_200);
    let text = format!("{}# Late Title\n", format!("{line}\n").repeat(49));
    assert!(text.len() > 400_000);
    let started = std::time::Instant::now();
    refreshed_titles(&[("inline-heavy.md", text, "Late Title")]);
    // Default inline parsing took about two minutes here in a debug build.
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn refresh_replaces_titles_derived_by_an_older_policy() {
    let s = Setup::new();
    let text = "```\n# Fake\n```\n# Real\n";
    write(s.vault.path(), "stale.md", text.as_bytes());
    write(s.vault.path(), "kept.md", b"# Kept\n");
    let mut library = s.open(None);
    library.refresh().unwrap();
    drop(library);
    let index_path = s.data.path().join("index.sqlite");
    let (mut index, _) = brn_retrieval::note_index::NoteIndex::open(&index_path).unwrap();
    let mut stale = index.note("stale.md").unwrap().unwrap();
    stale.title = "Fake".into();
    index.upsert_note(&stale, text).unwrap();
    drop(index);
    let mut library = s.open(None);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            updated: 1,
            unchanged: 1,
            ..Default::default()
        }
    );
    let titles: Vec<_> = library
        .notes()
        .unwrap()
        .into_iter()
        .map(|note| (note.path, note.title))
        .collect();
    assert_eq!(
        titles,
        [
            ("kept.md".into(), "Kept".into()),
            ("stale.md".into(), "Real".into())
        ]
    );
}

#[test]
fn title_counts_fifty_physical_saved_lines_including_headers() {
    let filler = |count: usize, newline: &str| {
        (1..=count)
            .map(|n| format!("line {n}{newline}"))
            .collect::<String>()
    };
    let header = |lines: usize| {
        format!(
            "---\n# yaml comment\n{}---\n",
            (3..lines).map(|n| format!("k{n}: v\n")).collect::<String>()
        )
    };
    let cases = [
        (
            "line-50.md",
            format!("{}# Line Fifty\n", filler(49, "\n")),
            "Line Fifty",
        ),
        (
            "line-51.md",
            format!("{}# Line Fifty-One\n", filler(50, "\n")),
            "line-51",
        ),
        (
            "crlf-bom-50.md",
            format!("\u{feff}{}# CRLF Fifty\r\n", filler(49, "\r\n")),
            "CRLF Fifty",
        ),
        (
            "header-50.md",
            format!("{}{}# Header Fifty\n", header(4), filler(45, "\n")),
            "Header Fifty",
        ),
        (
            "header-51.md",
            format!("{}{}# Header Fifty-One\n", header(4), filler(46, "\n")),
            "header-51",
        ),
        (
            "long-header.md",
            format!("{}# After Long Header\n", header(51)),
            "long-header",
        ),
        (
            "fence-crosses-window.md",
            format!("```\n{}# Fake\n```\n# After\n", filler(48, "\n")),
            "fence-crosses-window",
        ),
    ];
    assert_eq!(header(4).lines().count(), 4);
    assert_eq!(header(51).lines().count(), 51);
    refreshed_titles(&cases);
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
fn client_search_evidence_survives_disposable_passage_reallocation() {
    let s = Setup::new();
    write(s.vault.path(), "a.md", b"temporary unrelated passage");
    let mut library = s.open(None);
    library.refresh().unwrap();
    let original = b"\xef\xbb\xbf# Current knowledge\r\nneedle \xce\xbb\r\n";
    write(s.vault.path(), "knowledge.md", original);
    library.refresh().unwrap();
    std::fs::remove_file(s.vault.path().join("a.md")).unwrap();
    library.refresh().unwrap();
    let before = library.search("needle", SearchMode::Keyword, 5).unwrap();
    assert_eq!(before.hits.len(), 1);
    drop(library);
    std::fs::remove_file(s.data.path().join("index.sqlite")).unwrap();
    let mut rebuilt = s.open(None);
    rebuilt.refresh().unwrap();
    let after = rebuilt.search("needle", SearchMode::Keyword, 5).unwrap();
    assert_eq!(
        after, before,
        "derived row allocation is not client identity"
    );
    assert_eq!(
        std::fs::read(s.vault.path().join("knowledge.md")).unwrap(),
        original
    );
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
