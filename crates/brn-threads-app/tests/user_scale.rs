use brn_threads_app::*;
#[test]
fn five_thousand_current_notes_search_current_versions_and_exclude_archive() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut workspace = Workspace::open(root.path().join("data")).unwrap();
    let started = std::time::Instant::now();
    let mut anchor = String::new();
    for i in 0..5000 {
        let record = workspace
            .new_note(
                &format!("Project {i}"),
                &format!("Synthetic project record {i}. Search anchor harbor_unique_{i}."),
            )
            .unwrap();
        if i == 2500 {
            anchor = record.id;
        }
    }
    let hits = workspace.search("harbor_unique_2500").unwrap();
    assert!(
        hits.iter()
            .any(|hit| hit.note == anchor && hit.version == 1)
    );
    let session = workspace.store.begin_edit(&anchor, 1).unwrap();
    workspace
        .store
        .update_buffer(&session.id, 1, "Current handoff berth_shifted_anchor")
        .unwrap();
    workspace.store.save(&session.id, 1).unwrap();
    assert!(
        !workspace
            .search("harbor_unique_2500")
            .unwrap()
            .iter()
            .any(|hit| hit.note == anchor)
    );
    assert!(
        workspace
            .search("berth_shifted_anchor")
            .unwrap()
            .iter()
            .any(|hit| hit.note == anchor && hit.version == 2)
    );
    let record = workspace.store.record(&anchor).unwrap().unwrap();
    workspace
        .owner_change(
            "archive-anchor",
            &ChangeRequest {
                reason: "Owner archived synthetic anchor".into(),
                writes: vec![Put {
                    id: anchor.clone(),
                    expected_version: Some(record.version),
                    archived: true,
                    data: record.data,
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    assert!(
        !workspace
            .search("berth_shifted_anchor")
            .unwrap()
            .iter()
            .any(|hit| hit.note == anchor)
    );
    eprintln!(
        "5,000-note insertion, indexing, Save refresh and archive exclusion: {:?}",
        started.elapsed()
    );
}
