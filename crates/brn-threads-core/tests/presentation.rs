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

#[test]
fn workspace_location_is_lazy_bounded_and_survives_reopen() {
    let root = TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let directory = root.path().join("data");
    let mut store = Store::open(&directory).unwrap();
    assert_eq!(store.presentation_location().unwrap(), None);
    assert!(
        !tables(&directory)
            .iter()
            .any(|name| name == "workspace_location")
    );
    assert!(store.save_presentation_location("").is_err());
    assert!(store.save_presentation_location(&"x".repeat(4097)).is_err());
    store
        .save_presentation_location(r#"{"kind":"page","page":"library"}"#)
        .unwrap();
    drop(store);
    let store = Store::open(&directory).unwrap();
    assert_eq!(
        store.presentation_location().unwrap().as_deref(),
        Some(r#"{"kind":"page","page":"library"}"#)
    );
}

#[test]
fn model_favorites_are_lazy_bounded_ordered_and_survive_reopen() {
    let root = TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let directory = root.path().join("data");
    let mut store = Store::open(&directory).unwrap();
    assert!(store.presentation_model_favorites().unwrap().is_empty());
    assert!(
        !tables(&directory)
            .iter()
            .any(|name| name == "model_favorites")
    );
    let pair = |p: &str, m: &str| (p.to_owned(), m.to_owned());
    assert!(
        store
            .save_presentation_model_favorites(&[pair("", "m")])
            .is_err()
    );
    assert!(
        store
            .save_presentation_model_favorites(&vec![pair("chatgpt", "m"); 65])
            .is_err()
    );
    let favorites = vec![pair("copilot", "gpt-5"), pair("chatgpt", "gpt-6.1-sol")];
    store.save_presentation_model_favorites(&favorites).unwrap();
    drop(store);
    let mut store = Store::open(&directory).unwrap();
    assert_eq!(store.presentation_model_favorites().unwrap(), favorites);
    store.save_presentation_model_favorites(&[]).unwrap();
    assert!(store.presentation_model_favorites().unwrap().is_empty());
}
