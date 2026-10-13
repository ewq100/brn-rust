//! The dense lane (Task 5) with deterministic fake models: no download.
use brn_retrieval::embedding::Embedder;
use brn_threads_app::search::SearchHandle;
use brn_threads_app::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const GROUPS: [&[&str]; 4] = [
    &["car", "automobile", "vehicle", "fleet"],
    &["ship", "vessel", "boat"],
    &["money", "budget", "funds"],
    &["plant", "garden", "flower"],
];

/// Words of one synonym group share a dimension, so paraphrases match.
struct Topics {
    seen: Arc<Mutex<Vec<String>>>,
}

impl Embedder for Topics {
    fn identity(&self) -> &str {
        "fake-topics/1"
    }
    fn dimension(&self) -> usize {
        GROUPS.len() + 1
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.seen
            .lock()
            .unwrap()
            .extend(texts.iter().map(|t| t.to_string()));
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0; GROUPS.len() + 1];
                v[GROUPS.len()] = 0.01;
                for word in text.split(|c: char| !c.is_alphanumeric()) {
                    let word = word.to_lowercase();
                    if let Some(g) = GROUPS.iter().position(|g| g.contains(&word.as_str())) {
                        v[g] += 1.0;
                    }
                }
                v
            })
            .collect())
    }
    fn prefixes(&self) -> (&'static str, &'static str) {
        ("query: ", "passage: ")
    }
}

struct Broken;

impl Embedder for Broken {
    fn identity(&self) -> &str {
        "broken/1"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, _: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Err(brn_retrieval::Error::Invalid("synthetic model failure"))
    }
}

fn workspace() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap().join("data");
    let workspace = Workspace::open(&data).unwrap();
    (root, workspace)
}

/// Waits until idle-time embedding covers every passage.
fn embedded(handle: &SearchHandle) {
    handle.prepare().wait().unwrap();
    let started = Instant::now();
    while handle
        .embedding_progress()
        .is_none_or(|(with, all)| with < all)
    {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "embedding stalled"
        );
        std::thread::sleep(Duration::from_millis(5));
        // A cheap request lets the worker refresh its counts.
        handle.prepare().wait().unwrap();
    }
}

#[test]
fn dense_lane_finds_paraphrases_with_prefixes_and_reports_coverage() {
    let (_root, mut ws) = workspace();
    let chat = ws.new_thread("Private").unwrap().id;
    let fleet = ws
        .new_note(
            "Logistics",
            "# Fleet\n\nThe automobile fleet grows every year.\n",
        )
        .unwrap();
    ws.new_note("Harbor", "The vessel docks at noon.\n")
        .unwrap();
    let op = ws.store.allocate_operation("private").unwrap();
    let owner = HostAuthority::owner("fixture");
    let request = ChangeRequest {
        reason: "private".into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note {
                chat: Some(chat.clone()),
                ..Note::working("Secret garage", "A hidden vehicle.\n")
            }),
        }],
        inputs: vec![],
    };
    ws.store.prepare_owner(&op, &request, &owner).unwrap();
    ws.store.apply(&op, &owner).unwrap();

    let seen = Arc::new(Mutex::new(Vec::new()));
    let (handle, _worker) =
        SearchHandle::spawn_with(&ws.store, Some(Box::new(Topics { seen: seen.clone() }))).unwrap();
    embedded(&handle);
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|t| t.starts_with("passage: "))
    );

    // "car" appears in no note: only the dense lane can find the fleet note.
    let hybrid = handle.request_page("car", None, 10, None).wait().unwrap();
    assert_eq!(hybrid.semantic, "complete");
    assert_eq!(hybrid.matches[0].note, fleet.id);
    assert_eq!(hybrid.matches[0].heading_path, "Fleet");
    assert_eq!(hybrid.total, hybrid.matches.len());
    assert!(hybrid.matches.iter().all(|m| m.title != "Secret garage"));
    assert!(seen.lock().unwrap().iter().any(|t| t == "query: car"));
    let keyword = handle
        .request_keyword_page("car", None, 10, None)
        .wait()
        .unwrap();
    assert!(keyword.matches.is_empty());
    assert!(keyword.semantic.starts_with("off"));
    let in_chat = handle
        .request_page("car", Some(&chat), 10, None)
        .wait()
        .unwrap();
    assert!(in_chat.matches.iter().any(|m| m.title == "Secret garage"));

    // An edit re-chunks the note; its new passage is embedded again.
    let session = ws.store.begin_edit(&fleet.id, 1).unwrap();
    ws.store
        .update_buffer(&session.id, 1, "Our budget for funds is tight.\n")
        .unwrap();
    ws.store.save(&session.id, 1).unwrap();
    embedded(&handle);
    let money = handle.request_page("money", None, 10, None).wait().unwrap();
    assert_eq!(
        (money.matches[0].note.as_str(), money.matches[0].version),
        (fleet.id.as_str(), 2)
    );
    let car = handle.request_page("car", None, 10, None).wait().unwrap();
    assert!(car.matches.iter().all(|m| m.note != fleet.id));
    // Unrelated queries find nothing: the floor keeps abstention honest.
    let nothing = handle.request_page("plant", None, 10, None).wait().unwrap();
    assert_eq!((nothing.total, nothing.matches.len()), (0, 0));
}

#[test]
fn a_failing_model_is_reported_and_keyword_search_still_answers() {
    let (_root, ws) = workspace();
    let mut ws = ws;
    ws.new_note("Harbor", "The vessel docks at noon.\n")
        .unwrap();
    let (handle, _worker) = SearchHandle::spawn_with(&ws.store, Some(Box::new(Broken))).unwrap();
    handle.prepare().wait().unwrap();
    // Let the idle worker attempt (and fail) a batch.
    let started = Instant::now();
    loop {
        let page = handle.request_page("vessel", None, 5, None).wait().unwrap();
        assert_eq!(page.matches.len(), 1);
        if page.semantic.starts_with("failed") {
            assert!(
                page.semantic.contains("synthetic model failure"),
                "{}",
                page.semantic
            );
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{}",
            page.semantic
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn without_a_model_semantic_coverage_is_none() {
    let (_root, mut ws) = workspace();
    ws.new_note("Harbor", "The vessel docks.\n").unwrap();
    let page = ws.search_page("vessel", None, 5, None).unwrap();
    assert_eq!(page.semantic, "none");
    assert!(ws.search_handle().unwrap().embedding_progress().is_none());
}

#[test]
fn the_embedding_model_must_be_chosen_before_search_starts() {
    let (_root, mut ws) = workspace();
    ws.search("anything").unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    assert!(ws.use_embedder(Box::new(Topics { seen })).is_err());
}

#[test]
fn coverage_is_partial_right_after_a_sync_and_private_passages_cannot_crowd_out() {
    let (_root, mut ws) = workspace();
    let chat = ws.new_thread("Private").unwrap().id;
    let op = ws.store.allocate_operation("private-many").unwrap();
    let owner = HostAuthority::owner("fixture");
    let writes = (0..300)
        .map(|i| Put {
            id: op.creation_id(i),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note {
                chat: Some(chat.clone()),
                ..Note::working(format!("Garage {i}"), "automobile automobile vehicle\n")
            }),
        })
        .collect();
    let request = ChangeRequest {
        reason: "private".into(),
        writes,
        inputs: vec![],
    };
    ws.store.prepare_owner(&op, &request, &owner).unwrap();
    ws.store.apply(&op, &owner).unwrap();
    let public = ws.new_note("Depot", "One car is parked here.\n").unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (handle, _worker) =
        SearchHandle::spawn_with(&ws.store, Some(Box::new(Topics { seen }))).unwrap();
    embedded(&handle);
    // Every private passage is at least as close to the query as the public
    // one, yet the public note is still found in general search.
    let page = handle
        .request_page("automobile", None, 10, None)
        .wait()
        .unwrap();
    assert_eq!(page.matches.len(), 1);
    assert_eq!(page.matches[0].note, public.id);
    assert_eq!(page.total, 1);

    // A note written just now has no vector: its first answer says so.
    ws.new_note("Fresh", "A new vessel arrives.\n").unwrap();
    let page = handle
        .request_page("vessel", None, 10, None)
        .wait()
        .unwrap();
    assert!(page.semantic.starts_with("partial"), "{}", page.semantic);
    assert_eq!(page.matches.len(), 1, "the keyword lanes still answer");
}
