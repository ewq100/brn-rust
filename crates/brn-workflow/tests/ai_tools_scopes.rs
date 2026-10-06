use brn_ai::{AiErrorKind, ConflictKnowledge, NoteFacts, ReadScope, ReadTools};
use brn_workflow::{ai_tools::AiTools, library::Library};
use sha2::{Digest, Sha256};
use std::path::Path;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Library, AiTools) {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let vault = tempfile::tempdir_in(&parent).unwrap();
    let data = tempfile::tempdir_in(&parent).unwrap();
    write(
        vault.path(),
        "current.md",
        "---\nbrn_id: ABCDEFAB-CDEF-4ABC-8DEF-ABCDEFABCDEF\n---\nneedle current",
    );
    write(
        vault.path(),
        "source.md",
        "\u{feff}---\r\nbrn_kind: source\r\n---\r\nneedle algne allikas\r\n",
    );
    write(
        vault.path(),
        "old.md",
        "---\nbrn_state: history\n---\nneedle previous knowledge",
    );
    write(
        vault.path(),
        "archive/original.md",
        "---\nbrn_kind: source\n---\nneedle archived original",
    );
    write(
        vault.path(),
        "bad.md",
        "---\nbrn_kind: unsupported\n---\nneedle malformed",
    );
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    (vault, data, library, tools)
}

#[test]
fn scoped_ai_search_list_read_preserve_saved_originals_and_current_defaults() {
    let (vault, _data, _library, tools) = fixture();
    let originals: Vec<_> = [
        "current.md",
        "source.md",
        "old.md",
        "archive/original.md",
        "bad.md",
    ]
    .into_iter()
    .map(|path| (path, std::fs::read(vault.path().join(path)).unwrap()))
    .collect();
    let scopes = [
        (ReadScope::Current, vec!["current.md"]),
        (ReadScope::Source, vec!["archive/original.md", "source.md"]),
        (ReadScope::History, vec!["archive/original.md", "old.md"]),
        (
            ReadScope::All,
            vec!["archive/original.md", "current.md", "old.md", "source.md"],
        ),
    ];
    for (scope, expected) in scopes {
        let page = tools.list_notes_scoped(None, None, scope).unwrap();
        assert_eq!(
            page.notes
                .iter()
                .map(|note| note.path.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        let hits = tools.search_notes_scoped("needle", 10, scope).unwrap();
        assert!(hits.keyword_only);
        let mut paths = hits
            .hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths, expected);
        for path in expected {
            let note = tools.read_note_scoped(path, scope).unwrap();
            let saved = std::fs::read_to_string(vault.path().join(path)).unwrap();
            assert_eq!(note.text, saved);
            assert!(!note.truncated);
            let hit = hits.hits.iter().find(|hit| hit.path == path).unwrap();
            assert_eq!(
                saved.get(hit.start_byte..hit.end_byte),
                Some(hit.quote.as_str())
            );
            let facts = NoteFacts {
                note_id: (path == "current.md")
                    .then(|| "abcdefab-cdef-4abc-8def-abcdefabcdef".into()),
                sha256: Sha256::digest(saved.as_bytes()).into(),
                source: matches!(path, "source.md" | "archive/original.md"),
                history: matches!(path, "old.md" | "archive/original.md"),
                conflicts: ConflictKnowledge::Unknown,
            };
            assert_eq!(note.facts, facts);
            assert_eq!(hit.facts, facts);
            assert_eq!(
                page.notes
                    .iter()
                    .find(|note| note.path == path)
                    .unwrap()
                    .facts,
                facts
            );
        }
        for path in ["../source.md", ".hidden.md", "/source.md", "bad.md"] {
            assert_eq!(
                tools.read_note_scoped(path, scope).unwrap_err().kind,
                AiErrorKind::ToolRejected
            );
        }
    }
    assert_eq!(
        tools.list_notes(None, None).unwrap().notes[0].path,
        "current.md"
    );
    assert_eq!(
        tools.search_notes("needle", 10).unwrap().hits[0].path,
        "current.md"
    );
    assert_eq!(
        tools.read_note("source.md").unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(
        tools.read_note("archive/original.md").unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    let archived = tools
        .list_notes_scoped(Some("archive"), None, ReadScope::Source)
        .unwrap();
    assert_eq!(archived.notes.len(), 1);
    assert_eq!(archived.notes[0].path, "archive/original.md");
    for (path, original) in originals {
        assert_eq!(std::fs::read(vault.path().join(path)).unwrap(), original);
    }
}

#[test]
fn retained_scoped_tools_reject_stale_class_hash_and_preserve_utf8_cap() {
    let (vault, _data, mut library, tools) = fixture();
    write(
        vault.path(),
        "source.md",
        "---\nbrn_kind: knowledge\nbrn_state: history\n---\nneedle newer",
    );
    assert_eq!(
        tools
            .read_note_scoped("source.md", ReadScope::Source)
            .unwrap_err()
            .kind,
        AiErrorKind::ToolRejected
    );
    assert_eq!(
        tools
            .search_notes_scoped("needle", 10, ReadScope::Source)
            .unwrap_err()
            .kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        tools
            .list_notes_scoped(None, None, ReadScope::Source)
            .unwrap_err()
            .kind,
        AiErrorKind::IndexStale
    );
    library.refresh().unwrap();
    assert_eq!(
        tools
            .search_notes_scoped("needle", 10, ReadScope::Source)
            .unwrap()
            .hits
            .len(),
        1
    );
    assert_eq!(
        tools
            .read_note_scoped("source.md", ReadScope::History)
            .unwrap()
            .text,
        "---\nbrn_kind: knowledge\nbrn_state: history\n---\nneedle newer"
    );
    let prefix = "\u{feff}---\r\nbrn_kind: source\r\n---\r\n";
    let long = format!("{prefix}{}🦀", "x".repeat(49_999 - prefix.len()));
    write(vault.path(), "source.md", &long);
    let note = tools
        .read_note_scoped("source.md", ReadScope::Source)
        .unwrap();
    assert!(note.truncated);
    assert_eq!(note.text, &long[..49_999]);
    assert_eq!(
        note.facts.sha256,
        <[u8; 32]>::from(Sha256::digest(long.as_bytes()))
    );
    assert_ne!(
        note.facts.sha256,
        <[u8; 32]>::from(Sha256::digest(note.text.as_bytes()))
    );
    assert!(note.facts.source);
    assert!(!note.facts.history);
    assert_eq!(note.facts.note_id, None);
    assert_eq!(note.facts.conflicts, ConflictKnowledge::Unknown);
    assert_eq!(
        std::fs::read_to_string(vault.path().join("source.md")).unwrap(),
        long
    );
    for scope in [
        ReadScope::Current,
        ReadScope::Source,
        ReadScope::History,
        ReadScope::All,
    ] {
        assert_eq!(
            tools
                .search_notes_scoped("needle", 0, scope)
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
        assert_eq!(
            tools
                .list_notes_scoped(Some(".."), None, scope)
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
}

#[test]
fn ambiguous_managed_metadata_is_refused_instead_of_inventing_note_facts() {
    let (vault, _data, mut library, tools) = fixture();
    write(
        vault.path(),
        "bad.md",
        "---\nbrn_id: 00000000-0000-0000-0000-000000000001\nbrn_id: 00000000-0000-0000-0000-000000000002\n---\nneedle ambiguous",
    );
    library.refresh().unwrap();
    for scope in [
        ReadScope::Current,
        ReadScope::Source,
        ReadScope::History,
        ReadScope::All,
    ] {
        assert_eq!(
            tools.read_note_scoped("bad.md", scope).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        assert!(
            !tools
                .list_notes_scoped(None, None, scope)
                .unwrap()
                .notes
                .iter()
                .any(|note| note.path == "bad.md")
        );
        assert!(
            !tools
                .search_notes_scoped("needle", 10, scope)
                .unwrap()
                .hits
                .iter()
                .any(|hit| hit.path == "bad.md")
        );
    }
}
