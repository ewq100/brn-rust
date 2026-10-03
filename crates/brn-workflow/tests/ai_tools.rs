use brn_ai::{AiErrorKind, ReadTools};
use brn_workflow::{
    ai_tools::AiTools,
    library::{Embedder, Library, SearchMode, SharedEmbedder},
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn checksum(root: &Path) -> Vec<(std::path::PathBuf, [u8; 32])> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;
    fn visit(root: &Path, path: &Path, hashes: &mut Vec<(std::path::PathBuf, [u8; 32])>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let metadata = path.symlink_metadata().unwrap();
            if metadata.is_dir() {
                visit(root, &path, hashes);
            } else {
                let bytes = if metadata.file_type().is_symlink() {
                    std::fs::read_link(&path)
                        .unwrap()
                        .as_os_str()
                        .as_bytes()
                        .to_vec()
                } else {
                    std::fs::read(&path).unwrap()
                };
                hashes.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    Sha256::digest(bytes).into(),
                ));
            }
        }
    }
    let mut hashes = Vec::new();
    visit(root, root, &mut hashes);
    hashes.sort_by(|a, b| a.0.cmp(&b.0));
    hashes
}

#[test]
fn all_tools_reject_unsafe_paths_and_stale_index_text_without_writing_vault() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    for path in ["a.md", "archive/old.md", ".hidden/n.md"] {
        write(vault.path(), path, "apple secret");
    }
    let before = checksum(vault.path());
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    fn send_sync<T: Send + Sync>(_: &T) {}
    send_sync(&tools);
    for path in [
        "../a.md",
        "/a.md",
        "archive/old.md",
        ".hidden/n.md",
        "a\\b.md",
    ] {
        assert_eq!(
            tools.read_note(path).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        assert_eq!(
            tools.list_notes(None, Some(path)).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
    }
    for folder in ["../", "/tmp", "archive", ".hidden", "work/../bad", "work/"] {
        assert_eq!(
            tools.list_notes(Some(folder), None).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
    }
    assert_eq!(
        tools.search_notes("", 10).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(
        tools.search_notes("apple", 11).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(tools.search_notes("apple", 10).unwrap().hits.len(), 1);
    assert_eq!(checksum(vault.path()), before);
    write(vault.path(), "a.md", "fresh apple changed");
    let after_external_edit = checksum(vault.path());
    assert_eq!(tools.read_note("a.md").unwrap().text, "fresh apple changed");
    assert_eq!(
        tools.search_notes("apple", 10).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        tools.list_notes(None, None).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("a.md")).unwrap(),
        "fresh apple changed"
    );
    assert_eq!(checksum(vault.path()), after_external_edit);
    std::fs::remove_file(vault.path().join("a.md")).unwrap();
    let after_external_delete = checksum(vault.path());
    assert_eq!(
        tools.search_notes("apple", 10).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("archive/old.md")).unwrap(),
        "apple secret"
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join(".hidden/n.md")).unwrap(),
        "apple secret"
    );
    assert_eq!(checksum(vault.path()), after_external_delete);
}

#[test]
fn cap_preserves_exact_utf8_bom_crlf_prefix_and_folder_keyset_pages() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let text = format!("\u{feff}{}\r\nz", "é".repeat(25_000));
    write(vault.path(), "big.md", &text);
    let split_crlf = format!("\u{feff}{}\r\nz", "x".repeat(49_996));
    write(vault.path(), "split.md", &split_crlf);
    for i in 0..=200 {
        write(vault.path(), &format!("work/{i:03}.md"), "# Title\r\nexact");
    }
    write(vault.path(), "workshop/other.md", "# Outside");
    let before = checksum(vault.path());
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    let note = tools.read_note("big.md").unwrap();
    assert!(note.truncated);
    assert_eq!(note.text.len(), 49_999);
    assert_eq!(note.text, text[..49_999]);
    let split = tools.read_note("split.md").unwrap();
    assert!(split.truncated);
    assert_eq!(split.text.len(), 50_000);
    assert_eq!(split.text, split_crlf[..50_000]);
    assert!(split.text.ends_with('\r'));
    let first = tools.list_notes(Some("work"), None).unwrap();
    assert_eq!(first.notes.len(), 200);
    assert_eq!(first.next_cursor.as_deref(), Some("work/199.md"));
    let last = tools
        .list_notes(Some("work"), first.next_cursor.as_deref())
        .unwrap();
    assert_eq!(last.notes[0].path, "work/200.md");
    assert_eq!(last.next_cursor, None);
    assert_eq!(
        tools.read_note("work/000.md").unwrap().text,
        "# Title\r\nexact"
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("big.md")).unwrap(),
        text
    );
    assert_eq!(checksum(vault.path()), before);
}

struct Fake(Arc<AtomicUsize>);
impl Embedder for Fake {
    fn identity(&self) -> &str {
        "one-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(texts.iter().map(|_| vec![1.0, 0.5]).collect())
    }
}

#[test]
fn writer_and_concurrent_tools_share_one_model_and_search_policy() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    write(vault.path(), "a.md", "apple");
    let before = checksum(vault.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let shared = SharedEmbedder::new(Box::new(Fake(calls.clone())));
    let index = data.path().join("index.sqlite");
    let mut library = Library::open_shared(vault.path(), &index, Some(shared.clone())).unwrap();
    library.refresh().unwrap();
    library.embed_pending(2).unwrap();
    let tools = Arc::new(AiTools::open(vault.path(), &index, Some(shared)).unwrap());
    let expected = library.search("apple", SearchMode::Hybrid, 10).unwrap();
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let tools = tools.clone();
            std::thread::spawn(move || tools.search_notes("apple", 10).unwrap())
        })
        .collect();
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(!result.keyword_only);
        assert_eq!(result.hits[0].quote, expected.hits[0].quote);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(
        !library
            .search("apple", SearchMode::Keyword, 10)
            .unwrap()
            .keyword_only
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        4,
        "keyword search never embeds"
    );
    assert!(
        !library
            .search("apple", SearchMode::Semantic, 10)
            .unwrap()
            .keyword_only
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        5,
        "semantic search embeds exactly once"
    );
    let connection = rusqlite::Connection::open(&index).unwrap();
    connection
        .execute(
            "UPDATE meta SET value='other-model|2' WHERE key='embedding_model'",
            [],
        )
        .unwrap();
    assert!(matches!(
        library.search("apple", SearchMode::Hybrid, 10),
        Err(brn_workflow::library::LibraryError::Index(
            brn_retrieval::Error::ModelMismatch
        ))
    ));
    assert_eq!(
        tools.search_notes("apple", 10).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("a.md")).unwrap(),
        "apple"
    );
    assert_eq!(checksum(vault.path()), before);
}
#[test]
fn every_search_candidate_is_visibility_hash_range_and_exact_quote_checked() {
    use brn_retrieval::note_index::{IndexedNote, NoteIndex};
    use sha2::{Digest, Sha256};
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    write(vault.path(), "a.md", "é apple\r\n");
    write(vault.path(), "archive/old.md", "apple invisible");
    let before = checksum(vault.path());
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    assert!(tools.search_notes("apple", 10).unwrap().keyword_only);
    // Synthetic index tampering must never allow its text to reach tool output.
    let conn = rusqlite::Connection::open(&index).unwrap();
    for sql in [
        "UPDATE passages SET start_byte=-1",
        "UPDATE passages SET start_byte=1",
        "UPDATE passages SET start_byte=0, end_byte=999",
        "UPDATE passages SET start_byte=0, end_byte=10, text='apple fake'",
    ] {
        conn.execute(sql, []).unwrap();
        assert_eq!(
            tools.search_notes("apple", 10).unwrap_err().kind,
            AiErrorKind::IndexStale
        );
    }
    drop(conn);
    drop(tools);
    drop(library);
    let (mut writer, _) = NoteIndex::open(&index).unwrap();
    writer.remove_note("a.md").unwrap();
    let text = "apple invisible";
    writer
        .upsert_note(
            &IndexedNote {
                path: "archive/old.md".into(),
                title: "hidden".into(),
                size: text.len() as u64,
                modified_ns: 0,
                sha256: Sha256::digest(text.as_bytes()).into(),
            },
            text,
        )
        .unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    assert_eq!(
        tools.search_notes("apple", 10).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(
        tools.list_notes(None, None).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("a.md")).unwrap(),
        "é apple\r\n"
    );
    assert_eq!(
        std::fs::read_to_string(vault.path().join("archive/old.md")).unwrap(),
        text
    );
    assert_eq!(checksum(vault.path()), before);
}

#[test]
fn symlinks_oversize_and_non_utf8_files_never_become_note_text() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "external.md", "outside apple");
    std::os::unix::fs::symlink(
        outside.path().join("external.md"),
        vault.path().join("link.md"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), vault.path().join("alias")).unwrap();
    std::fs::write(vault.path().join("invalid.md"), b"\xff").unwrap();
    std::fs::write(vault.path().join("huge.md"), vec![b'a'; 1_048_577]).unwrap();
    let before = checksum(vault.path());
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    for path in ["link.md", "alias/external.md", "invalid.md", "huge.md"] {
        assert_eq!(
            tools.read_note(path).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
    }
    assert!(tools.list_notes(None, None).unwrap().notes.is_empty());
    assert!(tools.search_notes("apple", 10).unwrap().hits.is_empty());
    assert_eq!(
        std::fs::read_to_string(outside.path().join("external.md")).unwrap(),
        "outside apple"
    );
    assert_eq!(
        std::fs::read(vault.path().join("invalid.md")).unwrap(),
        b"\xff"
    );
    assert_eq!(
        std::fs::read(vault.path().join("huge.md")).unwrap(),
        vec![b'a'; 1_048_577]
    );
    assert_eq!(checksum(vault.path()), before);
}
