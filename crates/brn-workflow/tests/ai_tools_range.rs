use brn_ai::{
    AiErrorKind, ConflictKnowledge, NoteRangeRequest, READ_NOTE_BYTES, ReadScope, ReadTools,
};
use brn_workflow::{ai_tools::AiTools, library::Library};
use sha2::{Digest, Sha256};

fn request(
    path: &str,
    scope: ReadScope,
    text: &str,
    start_byte: usize,
    end_byte: usize,
) -> NoteRangeRequest {
    NoteRangeRequest {
        path: path.into(),
        scope,
        expected_sha256: Sha256::digest(text.as_bytes()).into(),
        start_byte,
        end_byte,
    }
}

#[test]
fn near_one_mib_tail_is_exact_hash_bound_scoped_and_read_only() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let head = "\u{feff}---\r\nbrn_id: abcdefab-cdef-4abc-8def-abcdefabcdef\r\nbrn_kind: source\r\n---\r\n";
    let tail = "\r\nLõplik 日本語 🦀 evidence\r\n";
    let text = format!(
        "{head}{}{tail}",
        "x".repeat(1_048_576 - head.len() - tail.len())
    );
    let file = vault.path().join("source.md");
    std::fs::write(&file, &text).unwrap();
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    let prefix = tools
        .read_note_scoped("source.md", ReadScope::Source)
        .unwrap();
    assert!(prefix.truncated);
    assert_eq!(prefix.text, text[..READ_NOTE_BYTES]);
    let metadata = tools
        .read_note_range(&request("source.md", ReadScope::Source, &text, 0, 0))
        .unwrap();
    assert_eq!(metadata.total_bytes, 1_048_576);
    assert_eq!(metadata.text, "");
    let start = metadata.total_bytes - tail.len();
    let range_request = request(
        "source.md",
        ReadScope::Source,
        &text,
        start,
        metadata.total_bytes,
    );
    let range = tools.read_note_range(&range_request).unwrap();
    assert_eq!(range.text, tail);
    assert_eq!(range.start_byte, start);
    assert_eq!(range.end_byte, text.len());
    assert_eq!(range.facts, prefix.facts);
    assert_eq!(range.facts.conflicts, ConflictKnowledge::Unknown);
    assert!(range.facts.source && !range.facts.history);
    assert_eq!(std::fs::read(&file).unwrap(), text.as_bytes());
    assert_eq!(
        tools
            .read_note_range(&NoteRangeRequest {
                scope: ReadScope::Current,
                ..range_request.clone()
            })
            .unwrap_err()
            .kind,
        AiErrorKind::ToolRejected
    );

    // The tail is unchanged; its full-file proof must still refuse a head edit.
    let changed = text.replacen('x', "y", 1);
    std::fs::write(&file, &changed).unwrap();
    assert_eq!(
        tools.read_note_range(&range_request).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(std::fs::read(&file).unwrap(), changed.as_bytes());
    std::fs::remove_file(&file).unwrap();
    assert_eq!(
        tools.read_note_range(&range_request).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert!(!file.exists());
}

#[test]
fn ranges_keep_utf8_byte_coordinates_and_refuse_invalid_intervals() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let text = "\u{feff}õ日本語🦀\r\nexact";
    let file = vault.path().join("a.md");
    std::fs::write(&file, text).unwrap();
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    for start in 0..=text.len() {
        for end in start..=text.len() {
            let result =
                tools.read_note_range(&request("a.md", ReadScope::Current, text, start, end));
            if let Some(exact) = text.get(start..end) {
                let result = result.unwrap();
                assert_eq!(result.text, exact);
                assert_eq!(
                    (result.start_byte, result.end_byte, result.total_bytes),
                    (start, end, text.len())
                );
            } else {
                assert_eq!(result.unwrap_err().kind, AiErrorKind::ToolRejected);
            }
        }
    }
    for (start, end) in [
        (3, 2),
        (0, text.len() + 1),
        (text.len() + 1, text.len() + 1),
        (0, READ_NOTE_BYTES + 1),
        (usize::MAX, 0),
    ] {
        assert_eq!(
            tools
                .read_note_range(&request("a.md", ReadScope::Current, text, start, end))
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
    assert_eq!(std::fs::read(&file).unwrap(), text.as_bytes());
}

#[test]
fn ranged_scopes_and_file_admission_match_existing_evidence_rules() {
    let vault = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(vault.path().join("archive")).unwrap();
    let notes = [
        ("current.md", "current", false, false),
        (
            "source.md",
            "---\nbrn_kind: source\n---\nsource",
            true,
            false,
        ),
        (
            "old.md",
            "---\nbrn_state: history\n---\nhistory",
            false,
            true,
        ),
        (
            "archive/source.md",
            "---\nbrn_kind: source\n---\nboth",
            true,
            true,
        ),
    ];
    for (path, text, _, _) in notes {
        std::fs::write(vault.path().join(path), text).unwrap();
    }
    let index = data.path().join("index.sqlite");
    let mut library = Library::open(vault.path(), &index, None).unwrap();
    library.refresh().unwrap();
    let tools = AiTools::open(vault.path(), &index, None).unwrap();
    for (path, text, source, history) in notes {
        for scope in [
            ReadScope::Current,
            ReadScope::Source,
            ReadScope::History,
            ReadScope::All,
        ] {
            let result = tools.read_note_range(&request(path, scope, text, 0, text.len()));
            let included = match scope {
                ReadScope::Current => !source && !history,
                ReadScope::Source => source,
                ReadScope::History => history,
                ReadScope::All => true,
            };
            if included {
                let result = result.unwrap();
                assert_eq!(result.text, text);
                assert_eq!(
                    (result.facts.source, result.facts.history),
                    (source, history)
                );
                assert_eq!(result.facts.conflicts, ConflictKnowledge::Unknown);
            } else {
                assert_eq!(result.unwrap_err().kind, AiErrorKind::ToolRejected);
            }
        }
        assert_eq!(
            std::fs::read(vault.path().join(path)).unwrap(),
            text.as_bytes()
        );
    }
    for path in ["../current.md", "/current.md", ".hidden.md", "a\\b.md"] {
        assert_eq!(
            tools
                .read_note_range(&request(path, ReadScope::All, "current", 0, 0))
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
    std::fs::write(
        vault.path().join("bad.md"),
        "---\nbrn_kind: unsupported\n---\nraw",
    )
    .unwrap();
    let bad = std::fs::read_to_string(vault.path().join("bad.md")).unwrap();
    assert_eq!(
        tools
            .read_note_range(&request("bad.md", ReadScope::All, &bad, 0, 0))
            .unwrap_err()
            .kind,
        AiErrorKind::ToolRejected
    );
    std::os::unix::fs::symlink(
        vault.path().join("current.md"),
        vault.path().join("link.md"),
    )
    .unwrap();
    std::fs::write(vault.path().join("invalid.md"), [255]).unwrap();
    std::fs::write(vault.path().join("huge.md"), vec![b'x'; 1_048_577]).unwrap();
    for path in ["link.md", "invalid.md", "huge.md"] {
        assert!(
            tools
                .read_note_range(&request(path, ReadScope::All, "", 0, 0))
                .is_err()
        );
    }
    assert_eq!(
        std::fs::read(vault.path().join("bad.md")).unwrap(),
        bad.as_bytes()
    );
    assert_eq!(
        std::fs::read(vault.path().join("invalid.md")).unwrap(),
        [255]
    );
    assert_eq!(
        std::fs::metadata(vault.path().join("huge.md"))
            .unwrap()
            .len(),
        1_048_577
    );
}
