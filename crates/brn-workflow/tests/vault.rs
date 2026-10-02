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
