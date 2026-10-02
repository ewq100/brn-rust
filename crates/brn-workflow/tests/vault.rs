use brn_workflow::vault::{VaultPath, VaultPathError};

#[test]
fn accepts_note_paths() {
    for raw in [
        "a.md",
        "notes/plan.md",
        "deep/er/x.MD",
        "archive.md",
        "archived/x.md",
        "sub/archive/x.md",
        "unicode/é note.md",
    ] {
        assert_eq!(VaultPath::parse(raw).unwrap().as_str(), raw, "{raw}");
    }
}

#[test]
fn rejects_non_note_paths() {
    use VaultPathError::*;
    for (raw, expected) in [
        ("", Empty),
        ("/abs.md", Absolute),
        ("a.txt", NotMarkdown),
        ("noext", NotMarkdown),
        ("notes/", InvalidComponent),
        ("a//b.md", InvalidComponent),
        ("./a.md", InvalidComponent),
        ("a\\b.md", InvalidComponent),
        ("a\0.md", InvalidComponent),
        ("../a.md", Traversal),
        ("x/../a.md", Traversal),
        (".hidden/a.md", Hidden),
        ("x/.a.md", Hidden),
        (".md", Hidden),
        ("archive/a.md", Archived),
        ("Archive/sub/a.md", Archived),
    ] {
        assert_eq!(VaultPath::parse(raw), Err(expected), "{raw:?}");
    }
}

#[test]
fn builds_filesystem_paths() {
    let path = VaultPath::parse("notes/plan.md").unwrap();
    assert_eq!(
        path.to_fs_path(std::path::Path::new("/vault")),
        std::path::PathBuf::from("/vault/notes/plan.md")
    );
    assert_eq!(path.to_string(), "notes/plan.md");
}

use brn_workflow::vault::{ReadError, SkipReason, read_note, scan};
use sha2::{Digest, Sha256};
use std::os::unix::fs::symlink;

const LIMIT: usize = 1024 * 1024;

fn write(root: &std::path::Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn scan_lists_notes_and_skips_excluded_files() {
    let vault = tempfile::tempdir().unwrap();
    let root = vault.path();
    write(root, "a.md", b"alpha");
    write(root, "sub/b.md", b"beta");
    write(root, "sub/c.txt", b"not a note");
    write(root, ".obsidian/d.md", b"hidden");
    write(root, "sub/.e.md", b"hidden");
    write(root, "archive/f.md", b"archived");
    write(root, "sub/archive/g.md", b"nested archive is fine");
    write(root, "big.md", &vec![b'x'; LIMIT + 1]);
    write(root, "exact.md", &vec![b'x'; LIMIT]);
    symlink(root.join("a.md"), root.join("link.md")).unwrap();
    symlink(root.join("sub"), root.join("linkdir")).unwrap();

    let found = scan(root).unwrap();
    let paths: Vec<&str> = found.notes.iter().map(|n| n.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["a.md", "exact.md", "sub/archive/g.md", "sub/b.md"]
    );
    assert_eq!(found.notes[0].size, 5);
    assert!(found.notes[0].modified_ns > 0);
    assert_eq!(found.skipped.len(), 1);
    assert_eq!(found.skipped[0].path, "big.md");
    assert_eq!(found.skipped[0].reason, SkipReason::TooLarge);
}

#[test]
fn read_returns_exact_text_and_hash() {
    let vault = tempfile::tempdir().unwrap();
    let bytes = "\u{feff}# Plan é\r\nline\r\n".as_bytes();
    write(vault.path(), "notes/plan.md", bytes);
    let path = VaultPath::parse("notes/plan.md").unwrap();
    let note = read_note(vault.path(), &path).unwrap();
    assert_eq!(note.text.as_bytes(), bytes);
    assert_eq!(note.sha256, <[u8; 32]>::from(Sha256::digest(bytes)));
}

#[test]
fn read_rejects_unsafe_or_unusable_files() {
    let vault = tempfile::tempdir().unwrap();
    let root = vault.path();
    write(root, "real/a.md", b"a");
    write(root, "big.md", &vec![b'x'; LIMIT + 1]);
    write(root, "binary.md", &[0xff, 0xfe, 0x00]);
    std::fs::create_dir(root.join("dir.md")).unwrap();
    symlink(root.join("real/a.md"), root.join("link.md")).unwrap();
    symlink(root.join("real"), root.join("linked")).unwrap();
    let read = |raw: &str| read_note(root, &VaultPath::parse(raw).unwrap());
    assert!(matches!(read("absent.md"), Err(ReadError::Missing)));
    assert!(matches!(read("link.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("linked/a.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("dir.md"), Err(ReadError::NotAFile)));
    assert!(matches!(read("big.md"), Err(ReadError::TooLarge)));
    assert!(matches!(read("binary.md"), Err(ReadError::NotUtf8)));
    assert_eq!(read("real/a.md").unwrap().text, "a");
}
