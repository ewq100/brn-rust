#![cfg(target_os = "macos")]
use brn_ai::{AiErrorKind, ReadTools};
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    library::{Embedder, KnowledgeScope, Library, SearchMode},
    proposal_apply::ApprovalRequest,
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
        }
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.vault.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.data.clone(), self.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready {
                vault_bound: true,
                ..
            }
        ));
        worker
    }
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(actual, id);
    event
}
fn paths(worker: &AppWorker, scope: KnowledgeScope) -> Vec<String> {
    match reply(
        worker,
        AppCommand::ScopedNotes {
            scope,
            folder: None,
            cursor: None,
        },
    ) {
        AppEvent::Notes(page) => page.notes.into_iter().map(|n| n.path).collect(),
        AppEvent::Failed(e) => panic!("list failed: {e}"),
        _ => panic!("expected notes"),
    }
}

#[test]
fn complete_worker_queries_separate_current_original_sources_and_history_without_writes() {
    use KnowledgeScope::*;
    let f = Fixture::new();
    let cases = [
        ("current.md", "# Current\r\nneedle current Täpne\r\n"),
        (
            "source.md",
            "\u{feff}---\r\nbrn_kind: source\r\n---\r\n# Original\r\nneedle original 日本語\r\n",
        ),
        (
            "previous.md",
            "---\nbrn_state: history\n---\nneedle previous\n",
        ),
        (
            "source-history.md",
            "---\nbrn_kind: source\nbrn_state: history\n---\nneedle old source\n",
        ),
        (
            "archive/forced.md",
            "---\nbrn_state: current\n---\nneedle archived\n",
        ),
        (
            "archive/source.md",
            "---\nbrn_kind: source\n---\nneedle archived original\n",
        ),
    ];
    for (path, text) in cases {
        f.write(path, text);
    }
    let mut worker = f.worker();
    for (scope, expected) in [
        (Current, vec!["current.md"]),
        (
            Source,
            vec!["archive/source.md", "source-history.md", "source.md"],
        ),
        (
            History,
            vec![
                "archive/forced.md",
                "archive/source.md",
                "previous.md",
                "source-history.md",
            ],
        ),
        (
            All,
            vec![
                "archive/forced.md",
                "archive/source.md",
                "current.md",
                "previous.md",
                "source-history.md",
                "source.md",
            ],
        ),
    ] {
        assert_eq!(paths(&worker, scope), expected);
        let AppEvent::Search(results) = reply(
            &worker,
            AppCommand::ScopedSearch {
                scope,
                query: "needle".into(),
                mode: SearchMode::Keyword,
                limit: 50,
            },
        ) else {
            panic!()
        };
        let mut hits = results
            .hits
            .iter()
            .map(|h| h.path.as_str())
            .collect::<Vec<_>>();
        hits.sort();
        hits.dedup();
        assert_eq!(hits, expected);
        assert!(results.keyword_only);
        for hit in results.hits {
            let text = fs::read_to_string(f.vault.join(&hit.path)).unwrap();
            assert_eq!(&text[hit.start_byte..hit.end_byte], hit.quote);
        }
    }
    assert!(
        matches!(reply(&worker,AppCommand::Note("source.md".into())),AppEvent::Failed(e)if e.kind==ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker,AppCommand::ScopedNote{path:"source.md".into(),scope:Source}),AppEvent::Note(note)if note.text==cases[1].1)
    );
    assert!(
        matches!(reply(&worker,AppCommand::ScopedNote{path:"archive/forced.md".into(),scope:History}),AppEvent::Note(note)if note.text==cases[4].1)
    );
    assert!(
        matches!(reply(&worker,AppCommand::ScopedNotes{scope:History,folder:Some("archive".into()),cursor:Some("archive/forced.md".into())}),AppEvent::Notes(page)if page.notes.len()==1 &&page.notes[0].path=="archive/source.md")
    );
    assert!(
        matches!(reply(&worker,AppCommand::Editors),AppEvent::Editors(records)if records.is_empty())
    );
    assert!(
        matches!(reply(&worker,AppCommand::Proposals(None)),AppEvent::Proposals(records)if records.is_empty())
    );
    worker.shutdown().unwrap();
    fs::remove_file(f.data.join("index.sqlite")).unwrap();
    let mut worker = f.worker();
    assert_eq!(
        paths(&worker, Source),
        ["archive/source.md", "source-history.md", "source.md"]
    );
    worker.shutdown().unwrap();
    for (path, text) in cases {
        assert_eq!(fs::read(f.vault.join(path)).unwrap(), text.as_bytes());
    }
}

#[test]
fn refresh_observes_retained_mtime_class_and_content_changes_while_retained_tools_refuse_stale_current()
 {
    use KnowledgeScope::*;
    let f = Fixture::new();
    let original = "---\nbrn_state: current\n---\nneedle exact\n";
    f.write("a.md", original);
    let mut app = App::open(&f.data, f.config()).unwrap();
    let tools = app.tools().unwrap();
    assert_eq!(tools.search_notes("needle", 10).unwrap().hits.len(), 1);
    let times = fs::FileTimes::new().set_modified(
        fs::metadata(f.vault.join("a.md"))
            .unwrap()
            .modified()
            .unwrap(),
    );
    let changed = original
        .replace("current", "history")
        .replace("needle", "thread");
    assert_eq!(changed.len(), original.len());
    f.write("a.md", &changed);
    fs::File::options()
        .write(true)
        .open(f.vault.join("a.md"))
        .unwrap()
        .set_times(times)
        .unwrap();
    assert_eq!(
        tools.search_notes("needle", 10).unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        tools.read_note("a.md").unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert!(
        app.search("needle", SearchMode::Keyword, 10)
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(app.notes(None, None).unwrap().notes.is_empty());
    let history = app
        .search_scoped("thread", SearchMode::Keyword, 10, History)
        .unwrap();
    assert_eq!(history.hits.len(), 1);
    assert_eq!(history.hits[0].path, "a.md");
    assert_eq!(app.note_scoped("a.md", History).unwrap().text, changed);
    assert!(tools.search_notes("thread", 10).unwrap().hits.is_empty());
    assert_eq!(fs::read(f.vault.join("a.md")).unwrap(), changed.as_bytes());
}

struct FlatEmbedder;
impl Embedder for FlatEmbedder {
    fn identity(&self) -> &str {
        "synthetic-scope-flat-v1"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![1.0, 0.0]).collect())
    }
}
#[test]
fn source_and_history_candidates_do_not_crowd_current_semantic_or_hybrid_limits() {
    use KnowledgeScope::*;
    let f = Fixture::new();
    for i in 0..70 {
        f.write(
            &format!("archive/{i:03}.md"),
            "---\nbrn_kind: source\n---\nneedle needle needle needle\n",
        );
    }
    f.write("z-current.md", "needle approved current\n");
    let mut library = Library::open(
        &f.vault,
        &f.data.join("index.sqlite"),
        Some(Box::new(FlatEmbedder)),
    )
    .unwrap();
    library.refresh().unwrap();
    let progress = library.embed_pending(200).unwrap().unwrap();
    assert_eq!(progress.embedded, progress.total);
    for mode in [
        SearchMode::Keyword,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let current = library.search("needle", mode, 1).unwrap();
        assert_eq!(current.hits.len(), 1);
        assert_eq!(current.hits[0].path, "z-current.md");
        assert!(!current.keyword_only);
        let source = library.search_scoped("needle", mode, 1, Source).unwrap();
        assert_eq!(source.hits.len(), 1);
        assert!(source.hits[0].path.starts_with("archive/"));
    }
    library.refresh().unwrap();
    assert_eq!(
        library.embed_pending(200).unwrap().unwrap().embedded,
        progress.total
    );
    assert_eq!(
        library
            .search("needle", SearchMode::Semantic, 1)
            .unwrap()
            .hits[0]
            .path,
        "z-current.md"
    );
}

#[test]
fn malformed_classifications_are_reported_and_never_defaulted_into_scoped_queries() {
    use KnowledgeScope::*;
    let f = Fixture::new();
    f.write("good.md", "needle approved\n");
    for (path, text) in [
        (
            "bad-kind.md",
            "---\nbrn_kind: mysterious\n---\nneedle bad\n",
        ),
        (
            "bad-state.md",
            "---\nbrn_state: current\nbrn_state: history\n---\nneedle bad\n",
        ),
        (
            "archive/bad.md",
            "---\nbrn_kind: source\n  continued\n---\nneedle bad\n",
        ),
    ] {
        f.write(path, text);
    }
    let mut app = App::open(&f.data, f.config()).unwrap();
    let report = app.refresh().unwrap();
    assert_eq!(report.unreadable.len(), 3);
    assert!(
        report
            .unreadable
            .iter()
            .all(|i| i.reason == "invalid managed metadata")
    );
    for scope in [Current, Source, History, All] {
        let results = app
            .search_scoped("needle", SearchMode::Keyword, 50, scope)
            .unwrap();
        assert!(results.hits.iter().all(|h| h.path == "good.md"));
    }
    assert_eq!(
        app.note_scoped("bad-kind.md", All).unwrap_err().kind,
        ErrorKind::ToolRejected
    );
    assert_eq!(
        app.evidence_note("bad-kind.md").unwrap().text,
        "---\nbrn_kind: mysterious\n---\nneedle bad\n"
    );
}

#[test]
fn unresolved_application_fences_every_scope_and_preserves_original_sources() {
    use KnowledgeScope::*;
    use brn_ai::ReadScope;
    let f = Fixture::new();
    f.write("current.md", "current approved\n");
    f.write(
        "archive/source.md",
        "---\nbrn_kind: source\n---\noriginal source\n",
    );
    let mut app = App::open(&f.data, f.config()).unwrap();
    let tools = app.tools().unwrap();
    let source = app.proposal_source("current.md").unwrap();
    let record = app
        .create_proposal(&DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "known pending".into(),
            changes: vec![DraftNoteChange::Create {
                path: "new.md".into(),
                text: "new knowledge".into(),
            }],
            sources: vec![source.source],
        })
        .unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    for scope in [Current, Source, History, All] {
        assert_eq!(
            app.notes_scoped(None, None, scope).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.search_scoped("source", SearchMode::Keyword, 10, scope)
                .unwrap_err()
                .kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.note_scoped("archive/source.md", scope)
                .unwrap_err()
                .kind,
            ErrorKind::SaveUncertain
        );
        let read_scope = match scope {
            Current => ReadScope::Current,
            Source => ReadScope::Source,
            History => ReadScope::History,
            All => ReadScope::All,
        };
        assert_eq!(
            tools
                .list_notes_scoped(None, None, read_scope)
                .unwrap_err()
                .kind,
            AiErrorKind::IndexStale
        );
        assert_eq!(
            tools
                .search_notes_scoped("source", 10, read_scope)
                .unwrap_err()
                .kind,
            AiErrorKind::IndexStale
        );
        assert_eq!(
            tools
                .read_note_scoped("current.md", read_scope)
                .unwrap_err()
                .kind,
            AiErrorKind::IndexStale
        );
    }
    assert_eq!(
        fs::read_to_string(f.vault.join("archive/source.md")).unwrap(),
        "---\nbrn_kind: source\n---\noriginal source\n"
    );
    assert!(!f.vault.join("new.md").exists());
}

#[test]
fn unreadable_archived_source_is_reported_without_blocking_current_knowledge() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.write("current.md", "needle approved current\n");
    f.write(
        "archive/source.md",
        "---\nbrn_kind: source\n---\noriginal\n",
    );
    let source = f.vault.join("archive/source.md");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();
    let blocked = fs::read(&source).unwrap_err();
    assert_eq!(blocked.kind(), std::io::ErrorKind::PermissionDenied);
    let inspected = (|| {
        let mut app = App::open(&f.data, f.config())?;
        let report = app.refresh()?;
        let results = app.search("needle", SearchMode::Keyword, 10)?;
        let rejected = app.evidence_note("archive/source.md").unwrap_err();
        Ok::<_, brn_workflow::WorkflowError>((report, results, rejected))
    })();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
    let (report, results, rejected) =
        inspected.expect("unrelated unreadable archive must not block current queries");
    assert_eq!(report.unreadable.len(), 1);
    assert_eq!(report.unreadable[0].path, "archive/source.md");
    assert_eq!(report.unreadable[0].reason, "could not read note");
    assert_eq!(results.hits.len(), 1);
    assert_eq!(results.hits[0].path, "current.md");
    assert_eq!(rejected.kind, ErrorKind::ToolRejected);
    assert_eq!(
        fs::read_to_string(source).unwrap(),
        "---\nbrn_kind: source\n---\noriginal\n"
    );
}

#[test]
fn unreadable_archive_folder_keeps_current_queries_and_removes_stale_descendants() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    f.write("current.md", "needle approved current\n");
    f.write(
        "archive/source.md",
        &format!("---\nbrn_id: {source_id}\nbrn_kind: source\n---\noriginal\n"),
    );
    let folder = f.vault.join("archive");
    let mut app = App::open(&f.data, f.config()).unwrap();
    assert_eq!(
        app.search_scoped("original", SearchMode::Keyword, 10, KnowledgeScope::Source)
            .unwrap()
            .hits
            .len(),
        1
    );
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        fs::read_dir(&folder).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let inspected = (|| {
        let report = app.refresh()?;
        let current = app.search("needle", SearchMode::Keyword, 10)?;
        let source =
            app.search_scoped("original", SearchMode::Keyword, 10, KnowledgeScope::Source)?;
        let resolution = app.resolve_note_identity(source_id)?;
        assert_eq!(app.note("current.md")?.text, "needle approved current\n");
        drop(app);
        let mut reopened = App::open(&f.data, f.config())?;
        assert_eq!(reopened.notes(None, None)?.notes.len(), 1);
        Ok::<_, brn_workflow::WorkflowError>((report, current, source, resolution))
    })();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    let (report, current, source, resolution) =
        inspected.expect("unreadable archive folder must not block known current knowledge");
    assert_eq!(report.unreadable.len(), 1);
    assert_eq!(report.unreadable[0].path, "archive");
    assert_eq!(report.unreadable[0].reason, "could not inspect folder");
    assert_eq!(report.removed, 1);
    assert_eq!(current.hits.len(), 1);
    assert!(source.hits.is_empty());
    assert_eq!(
        resolution.outcome,
        brn_workflow::knowledge::IdentityOutcome::Incomplete
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("archive/source.md")).unwrap(),
        format!("---\nbrn_id: {source_id}\nbrn_kind: source\n---\noriginal\n")
    );
}
