//! The plan's 10,000-note scale check. Bounds are the acceptance targets
//! when `BRN_SCALE_STRICT=1` (run with `--release` on the target Mac) and
//! lenient otherwise, so unoptimized CI builds still check the behavior.
use brn_threads_app::*;
use std::time::{Duration, Instant};

const NOTES: usize = 10_000;

fn bound(strict_ms: u64) -> Duration {
    let strict = std::env::var("BRN_SCALE_STRICT").is_ok_and(|v| v == "1");
    Duration::from_millis(if strict { strict_ms } else { strict_ms * 40 })
}

fn create_notes(workspace: &mut Workspace, notes: usize) -> Vec<String> {
    let owner = HostAuthority::owner("synthetic scale fixture");
    let mut ids = Vec::with_capacity(notes);
    for batch in 0..notes / 500 {
        let op = workspace
            .store
            .allocate_operation(&format!("scale-batch:{batch}"))
            .unwrap();
        let writes = (0..500)
            .map(|j| {
                let i = batch * 500 + j;
                Put {
                    id: op.creation_id(j),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        format!("Project {i}"),
                        format!(
                            "# Overview\n\nSynthetic project record {i} about harbor logistics.\n\n\
                             ## Anchor\n\nSearch anchor harborunique{i} and berths number {}.",
                            i % 97
                        ),
                    )),
                }
            })
            .collect();
        let request = ChangeRequest {
            reason: "Synthetic scale notes".into(),
            writes,
            inputs: vec![],
        };
        workspace
            .store
            .prepare_owner(&op, &request, &owner)
            .unwrap();
        let ApplyOutcome::Applied(receipt) = workspace.store.apply(&op, &owner).unwrap() else {
            panic!("batch not applied");
        };
        ids.extend(receipt.writes.into_iter().map(|w| w.after.id));
    }
    ids
}

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() * 95 / 100]
}

#[test]
fn ten_thousand_notes_reopen_without_rebuild_sync_one_note_and_query_quickly() {
    profile(NOTES, 0);
}

#[test]
#[ignore = "explicit repeated synthetic timing; normal CI retains the 10k correctness witness"]
fn repeated_startup_search_scale_profile() {
    for repeat in 0..2 {
        for notes in [1_000, 5_000, 10_000] {
            profile(notes, repeat);
        }
    }
}

fn profile(notes: usize, repeat: usize) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let started = Instant::now();
    let mut workspace = Workspace::open(&data).unwrap();
    let fresh_open = started.elapsed();
    let started = Instant::now();
    let ids = create_notes(&mut workspace, notes);
    let seed = started.elapsed();
    let anchor_index = notes / 4;
    let anchor = ids[anchor_index].clone();
    let original_query = format!("harborunique{anchor_index}");

    let started = Instant::now();
    workspace.search_handle().unwrap().prepare().wait().unwrap();
    let cold_build = started.elapsed();
    let hits = workspace.search(&original_query).unwrap();
    assert_eq!(hits[0].note, anchor);
    assert_eq!(hits[0].version, 1);
    assert!(hits[0].heading_path.starts_with("Overview"));
    assert!(hits[0].snippet.contains(&original_query));
    let page = workspace.search_page("harbor", None, 12, None).unwrap();
    assert_eq!((page.total, page.hits.len()), (notes, 12));
    assert_eq!(page.cursor.as_deref(), Some("12"));
    drop(workspace);

    // Reopen: the persisted index already mirrors the canonical watermark.
    let started = Instant::now();
    let mut workspace = Workspace::open(&data).unwrap();
    let workspace_reopen = started.elapsed();
    let started = Instant::now();
    workspace.search_handle().unwrap().prepare().wait().unwrap();
    let reopen = started.elapsed();
    assert!(
        reopen < bound(50) && reopen * 4 < cold_build,
        "reopen {reopen:?} vs cold build {cold_build:?}"
    );

    // One saved note: sync only that note, then find its current text.
    let session = workspace.store.begin_edit(&anchor, 1).unwrap();
    workspace
        .store
        .update_buffer(&session.id, 1, "Current handoff berthshiftedanchor")
        .unwrap();
    workspace.store.save(&session.id, 1).unwrap();
    let started = Instant::now();
    workspace.search_handle().unwrap().prepare().wait().unwrap();
    let one_note_sync = started.elapsed();
    assert!(one_note_sync < bound(50), "one-note sync {one_note_sync:?}");
    assert!(
        !workspace
            .search(&original_query)
            .unwrap()
            .iter()
            .any(|hit| hit.note == anchor),
        "saved-over text must not match"
    );
    let found = workspace.search("berthshiftedanchor").unwrap();
    assert_eq!(
        (found[0].note.as_str(), found[0].version),
        (anchor.as_str(), 2)
    );

    let mut samples = Vec::new();
    for i in (0..notes).step_by(notes / 60) {
        for query in [
            format!("harborunique{i}"),
            format!("berths number {}", i % 97),
        ] {
            let started = Instant::now();
            let page = workspace.search_page(&query, None, 12, None).unwrap();
            samples.push(started.elapsed());
            assert!(!page.hits.is_empty(), "{query}");
        }
    }
    let query_count = samples.len();
    let query_p95 = p95(samples);
    assert!(query_p95 < bound(150), "query p95 {query_p95:?}");

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
    assert!(workspace.search("berthshiftedanchor").unwrap().is_empty());
    eprintln!(
        "SCALE_PROFILE {}",
        serde_json::json!({
            "notes": notes, "repeat": repeat,
            "fresh_open_ms": fresh_open.as_secs_f64() * 1000.,
            "seed_ms": seed.as_secs_f64() * 1000.,
            "cold_build_ms": cold_build.as_secs_f64() * 1000.,
            "workspace_reopen_ms": workspace_reopen.as_secs_f64() * 1000.,
            "index_reopen_ms": reopen.as_secs_f64() * 1000.,
            "one_note_sync_ms": one_note_sync.as_secs_f64() * 1000.,
            "query_p95_ms": query_p95.as_secs_f64() * 1000.,
            "query_count": query_count,
        })
    );
}
