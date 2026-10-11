use brn_threads_app::*;
use sha2::{Digest, Sha256};

fn fixture() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let app = Workspace::open(root.path().join("data")).unwrap();
    (root, app)
}

fn apply(app: &mut Workspace, key: &str, writes: Vec<Put>) {
    assert!(matches!(
        app.owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic export metadata fixture".into(),
                writes,
                inputs: vec![],
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

fn scalar(header: &str, key: &str) -> String {
    let prefix = format!("{key}: ");
    let value = header
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .unwrap();
    serde_json::from_str(value).unwrap()
}

#[test]
fn metadata_quotes_arbitrary_titles_sources_and_preserves_existing_frontmatter_body() {
    let (root, mut app) = fixture();
    let title = "Töö \"quoted\" title\n---\n# absent from body: [yes] 🦀";
    let locator = "synthetic://mail/\"message\"\n---\nhttps://näide.test/?a=b";
    let label = "Source: \"mail\"\n---\nÕhtu";
    let observed = "2026-10-11\n# observed: yes";
    let external = "v\"2\"\n---";
    let body =
        "---\r\noriginal: \"leave unchanged\"\r\n---\r\n\nUnicode Õ and 🦀.\n\tIndented.  \n";
    let operation = app
        .store
        .allocate_operation("metadata with import and link")
        .unwrap();
    let note_id = operation.creation_id(0);
    let source_id = operation.creation_id(1);
    let mut note = Note::working(title, body);
    note.import = Some(ImportCoverage {
        source: source_id.clone(),
        full_note: false,
        gaps: vec![],
        report: None,
    });
    apply(
        &mut app,
        "metadata with import and link",
        vec![
            Put {
                id: note_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(note),
            },
            Put {
                id: source_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Source(Source {
                    locator: locator.into(),
                    label: label.into(),
                    observed_at: observed.into(),
                    external_version: Some(external.into()),
                    outcome: "Complete".into(),
                    gaps: vec![],
                    report: None,
                }),
            },
            Put {
                id: operation.creation_id(2),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: note_id.clone(),
                    to: source_id.clone(),
                    relation: "source".into(),
                }),
            },
        ],
    );
    drop(app);
    let app = Workspace::open(root.path().join("data")).unwrap();
    let destination = root.path().join("snapshot");
    let manifest = app.export_now(&destination).unwrap();
    let file = manifest
        .files
        .iter()
        .find(|file| file.record == note_id)
        .unwrap();
    let bytes = std::fs::read(destination.join(&file.path)).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let rest = text
        .strip_prefix("---\n")
        .expect("portable metadata wrapper");
    let (header, exported_body) = rest.split_once("\n---\n").unwrap();
    assert_eq!(scalar(header, "title"), title);
    assert_eq!(scalar(header, "locator"), locator);
    assert_eq!(scalar(header, "label"), label);
    assert!(!header.contains("observed_at:"));
    assert!(!header.contains("external_version:"));
    assert_eq!(scalar(header, "- record"), source_id);
    assert_eq!(
        header.matches("- record:").count(),
        1,
        "deduplicate import and link"
    );
    assert_eq!(exported_body.as_bytes(), body.as_bytes());
    assert_eq!(file.sha256, format!("{:x}", Sha256::digest(&bytes)));
    let persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(destination.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(persisted, serde_json::to_value(&manifest).unwrap());
    let second = app
        .export_now(&root.path().join("second-snapshot"))
        .unwrap();
    assert_eq!(serde_json::to_value(second).unwrap(), persisted);
}

#[test]
fn plain_note_empty_body_and_asset_body_have_valid_metadata_and_snapshot_hashes() {
    let (root, mut app) = fixture();
    let empty = app.new_note("---\nnull: \"\"\nÕ", "").unwrap();
    let operation = app
        .store
        .allocate_operation("asset metadata fixture")
        .unwrap();
    let note_id = operation.creation_id(0);
    let asset_id = operation.creation_id(1);
    let body = format!("Before\n![Figure](brn-asset://{asset_id})\nAfter  \n");
    apply(
        &mut app,
        "asset metadata fixture",
        vec![
            Put {
                id: note_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working("Figure title", &body)),
            },
            Put {
                id: asset_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Asset(Asset {
                    note: note_id.clone(),
                    source: None,
                    media_type: "image/png".into(),
                    bytes: vec![137, 80, 78, 71],
                }),
            },
        ],
    );
    let destination = root.path().join("snapshot");
    let manifest = app.export_now(&destination).unwrap();
    for file in &manifest.files {
        let bytes = std::fs::read(destination.join(&file.path)).unwrap();
        assert_eq!(file.sha256, format!("{:x}", Sha256::digest(&bytes)));
    }
    let read = |id: &str| {
        std::fs::read_to_string(
            destination.join(
                &manifest
                    .files
                    .iter()
                    .find(|file| file.record == id)
                    .unwrap()
                    .path,
            ),
        )
        .unwrap()
    };
    let empty_text = read(&empty.id);
    assert!(empty_text.ends_with("sources: []\n---\n"));
    let asset = manifest
        .files
        .iter()
        .find(|file| file.record == asset_id)
        .unwrap();
    let expected_body = body.replace(&format!("brn-asset://{asset_id}"), &asset.path);
    assert_eq!(
        read(&note_id).split_once("\n---\n").unwrap().1,
        expected_body
    );
    assert!(app.export_now(&destination).is_err());
    assert_eq!(
        read(&note_id).split_once("\n---\n").unwrap().1,
        expected_body
    );
}

#[test]
fn source_links_without_import_coverage_are_exported() {
    let (root, mut app) = fixture();
    let note = app
        .new_note("Working note", "Source linked externally")
        .unwrap();
    let operation = app
        .store
        .allocate_operation("linked source metadata")
        .unwrap();
    apply(
        &mut app,
        "linked source metadata",
        vec![
            Put {
                id: operation.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Source(Source {
                    locator: "synthetic://linked-only".into(),
                    label: "Reference".into(),
                    observed_at: "today".into(),
                    external_version: None,
                    outcome: "Useful information".into(),
                    gaps: vec![],
                    report: None,
                }),
            },
            Put {
                id: operation.creation_id(1),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: note.id.clone(),
                    to: operation.creation_id(0),
                    relation: "source".into(),
                }),
            },
        ],
    );
    let snapshot = root.path().join("snapshot");
    let manifest = app.export_now(&snapshot).unwrap();
    let file = manifest
        .files
        .iter()
        .find(|file| file.record == note.id)
        .unwrap();
    let text = std::fs::read_to_string(snapshot.join(&file.path)).unwrap();
    assert!(text.contains("locator: \"synthetic://linked-only\""));
    assert!(!text.contains("external_version:"));
}

#[test]
fn later_source_observation_does_not_attribute_its_version_to_preserved_note() {
    let (root, mut app) = fixture();
    let op = app
        .store
        .allocate_operation("source version fixture")
        .unwrap();
    let source_id = op.creation_id(1);
    let mut note = Note::working(
        "Original document",
        "Complete content captured from source v1.\n",
    );
    note.protected = true;
    note.import = Some(ImportCoverage {
        source: source_id.clone(),
        full_note: true,
        gaps: vec![],
        report: None,
    });
    apply(
        &mut app,
        "source version fixture",
        vec![
            Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Note(note),
            },
            Put {
                id: source_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Source(Source {
                    locator: "synthetic://document".into(),
                    label: "Original reference".into(),
                    observed_at: "Monday".into(),
                    external_version: Some("v1".into()),
                    outcome: "Complete".into(),
                    gaps: vec![],
                    report: None,
                }),
            },
        ],
    );
    let source = app.store.record(&source_id).unwrap().unwrap();
    let RecordData::Source(mut updated) = source.data else {
        panic!()
    };
    updated.observed_at = "Friday".into();
    updated.external_version = Some("v2".into());
    apply(
        &mut app,
        "later source observation",
        vec![Put {
            id: source_id,
            expected_version: Some(source.version),
            archived: false,
            data: RecordData::Source(updated),
        }],
    );
    let snapshot = root.path().join("snapshot");
    let manifest = app.export_now(&snapshot).unwrap();
    let file = manifest
        .files
        .iter()
        .find(|file| file.record == op.creation_id(0))
        .unwrap();
    let text = std::fs::read_to_string(snapshot.join(&file.path)).unwrap();
    let (header, body) = text
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap();
    assert_eq!(body, "Complete content captured from source v1.\n");
    assert_eq!(scalar(header, "locator"), "synthetic://document");
    assert!(!header.contains("version:"));
    assert!(!header.contains("observed_at:"));
    assert!(!header.contains("v2"));
    assert!(!header.contains("Friday"));
}
