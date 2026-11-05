//! The appearance preference is host presentation state outside the
//! versioned records. Opening a workspace never mutates the database for it;
//! the first save creates its table. Synthetic folders only.
use brn_threads_core::Store;
use tempfile::TempDir;

fn tables(directory: &std::path::Path) -> Vec<String> {
    let conn = rusqlite::Connection::open_with_flags(
        directory.join("threads.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn opening_never_creates_presentation_state_and_the_first_save_persists_it() {
    let root = TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let directory = root.path().join("data");
    drop(Store::open(&directory).unwrap());
    let before = tables(&directory);
    assert!(!before.iter().any(|name| name == "presentation"));

    // Reopening and reading leaves the database's table set unchanged.
    let mut store = Store::open(&directory).unwrap();
    assert_eq!(store.presentation_theme().unwrap(), None);
    drop(store);
    assert_eq!(tables(&directory), before);

    store = Store::open(&directory).unwrap();
    store.save_presentation_theme("dark").unwrap();
    assert!(store.save_presentation_theme("Not A Theme").is_err());
    drop(store);
    let store = Store::open(&directory).unwrap();
    assert_eq!(store.presentation_theme().unwrap().as_deref(), Some("dark"));
}
