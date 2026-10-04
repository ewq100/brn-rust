use brn_workflow::{
    MAX_NOTE_BYTES,
    vault::{
        EvidencePath, ReadError, SkipReason, VaultPath, VaultPathError, read_evidence, read_note,
        scan, scan_evidence,
    },
};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::symlink, path::Path};

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn explicit_evidence_path_opens_archive_without_expanding_current_paths() {
    let vault = tempfile::tempdir_in("/tmp").unwrap();
    let root = vault.path().canonicalize().unwrap();
    let bytes = "\u{feff}---\r\ncustom: 日本語 🦀\r\n---\r\n# Source λ\r\nexact source wording\r\n"
        .as_bytes();
    for path in [
        "Archive/source.md",
        "Archive/deep/資料.MD",
        "work/current.md",
    ] {
        write(&root, path, bytes);
        let evidence = EvidencePath::parse(path).unwrap();
        assert_eq!(evidence.as_str(), path);
        assert_eq!(evidence.to_fs_path(&root), root.join(path));
        assert_eq!(evidence.to_string(), path);
        let note = read_evidence(&root, &evidence).unwrap();
        assert_eq!(note.text.as_bytes(), bytes);
        assert_eq!(note.sha256, <[u8; 32]>::from(Sha256::digest(bytes)));
        assert_eq!(std::fs::read(root.join(path)).unwrap(), bytes);
        if path
            .split('/')
            .next()
            .unwrap()
            .eq_ignore_ascii_case("archive")
        {
            assert_eq!(VaultPath::parse(path), Err(VaultPathError::Archived));
        } else {
            assert_eq!(
                note,
                read_note(&root, &VaultPath::parse(path).unwrap()).unwrap()
            );
        }
    }
    assert_eq!(
        VaultPath::validate_folder("archive"),
        Err(VaultPathError::Archived)
    );
    assert_eq!(
        VaultPath::validate_folder("Archive/deep"),
        Err(VaultPathError::Archived)
    );
    let current = scan(&root).unwrap();
    assert_eq!(
        current
            .notes
            .iter()
            .map(|note| note.path.as_str())
            .collect::<Vec<_>>(),
        ["work/current.md"]
    );
    let evidence = scan_evidence(&root).unwrap();
    assert_eq!(
        evidence
            .notes
            .iter()
            .map(|note| note.path.as_str())
            .collect::<Vec<_>>(),
        [
            "Archive/deep/資料.MD",
            "Archive/source.md",
            "work/current.md"
        ]
    );
    for note in &evidence.notes {
        assert_eq!(note.size, bytes.len() as u64);
        assert!(note.modified_ns > 0);
    }
}

#[test]
fn evidence_paths_preserve_containment_and_visible_markdown_rules() {
    use VaultPathError::*;
    for (raw, expected) in [
        ("", Empty),
        ("/archive/a.md", Absolute),
        ("archive/a.txt", NotMarkdown),
        ("archive/", InvalidComponent),
        ("archive//a.md", InvalidComponent),
        ("archive/./a.md", InvalidComponent),
        ("archive\\a.md", InvalidComponent),
        ("archive/a\0.md", InvalidComponent),
        ("../a.md", Traversal),
        ("archive/../a.md", Traversal),
        (".hidden/a.md", Hidden),
        ("archive/.private/a.md", Hidden),
        ("archive/.md", Hidden),
    ] {
        assert_eq!(EvidencePath::parse(raw), Err(expected), "{raw:?}");
    }
    for path in [
        "a.md",
        "archive.md",
        "sub/archive/a.md",
        "archive/資料\r\n🧭.MD",
    ] {
        assert_eq!(EvidencePath::parse(path).unwrap().as_str(), path);
    }
}

#[test]
fn evidence_reads_and_scan_preserve_file_limits_and_refuse_unsafe_targets() {
    // Keep the exclusively-owned socket fixture below Unix socket path limits,
    // independently of a longer verification TMPDIR; canonical roots stay outside Git.
    let vault = tempfile::tempdir_in("/tmp").unwrap();
    let outside = tempfile::tempdir_in("/tmp").unwrap();
    let root = vault.path().canonicalize().unwrap();
    write(&root, "archive/real/note.md", b"exact");
    write(&root, "archive/big.md", &vec![b'x'; MAX_NOTE_BYTES + 1]);
    write(&root, "archive/limit.md", &vec![b'x'; MAX_NOTE_BYTES]);
    write(&root, "archive/binary.md", &[0xff, 0xfe, 0]);
    write(&root, "archive/bad\\name.md", b"unsupported path name");
    write(&root, "archive/.hidden.md", b"hidden");
    write(&root, ".internal/invisible.md", b"hidden");
    write(outside.path(), "outside.md", b"outside fixture");
    std::fs::create_dir(root.join("archive/directory.md")).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.join("archive/socket.md")).unwrap();
    symlink(
        root.join("archive/real/note.md"),
        root.join("archive/link.md"),
    )
    .unwrap();
    symlink(outside.path(), root.join("archive/alias")).unwrap();
    let read = |path| read_evidence(&root, &EvidencePath::parse(path).unwrap());
    assert!(matches!(
        read("archive/missing.md"),
        Err(ReadError::Missing)
    ));
    assert!(matches!(
        read("archive/directory.md"),
        Err(ReadError::NotAFile)
    ));
    assert!(matches!(
        read("archive/socket.md"),
        Err(ReadError::NotAFile)
    ));
    assert!(matches!(read("archive/link.md"), Err(ReadError::NotAFile)));
    assert!(matches!(
        read("archive/alias/outside.md"),
        Err(ReadError::NotAFile)
    ));
    assert!(matches!(read("archive/big.md"), Err(ReadError::TooLarge)));
    assert!(matches!(read("archive/binary.md"), Err(ReadError::NotUtf8)));
    assert_eq!(read("archive/limit.md").unwrap().text.len(), MAX_NOTE_BYTES);

    let found = scan_evidence(&root).unwrap();
    // Scanning lists supported-sized files; byte encoding is checked by reading.
    assert_eq!(
        found
            .notes
            .iter()
            .map(|note| note.path.as_str())
            .collect::<Vec<_>>(),
        [
            "archive/binary.md",
            "archive/limit.md",
            "archive/real/note.md"
        ]
    );
    assert_eq!(found.skipped.len(), 2);
    assert_eq!(found.skipped[0].path, "archive/bad\\name.md");
    assert_eq!(found.skipped[0].reason, SkipReason::InvalidName);
    assert_eq!(found.skipped[1].path, "archive/big.md");
    assert_eq!(found.skipped[1].reason, SkipReason::TooLarge);
    assert!(scan(&root).unwrap().notes.is_empty());
    assert_eq!(
        std::fs::read(outside.path().join("outside.md")).unwrap(),
        b"outside fixture"
    );
    assert_eq!(
        std::fs::read(root.join("archive/real/note.md")).unwrap(),
        b"exact"
    );
}
