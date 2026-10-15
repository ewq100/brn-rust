//! Useful intake of the same source reuses the chat that already received it,
//! within the same project scope. Synthetic data; a scripted model answers.
use brn_ai::threads::{ThreadRunRequest, TransientSource, prepare_run, run_prepared_scripted};
use brn_ai::{Provider, ReasoningEffort, Selection};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use std::path::PathBuf;

const LOCATOR: &str = "/synthetic/mari-crane-email.txt";
const TITLE: &str = "Useful information intake";

fn workspace() -> (tempfile::TempDir, PathBuf, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace = Workspace::open(&data).unwrap();
    let settings = workspace.settings().unwrap();
    workspace
        .configure(WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            ..settings
        })
        .unwrap();
    (root, data, workspace)
}

fn project(workspace: &mut Workspace, title: &str) -> String {
    workspace
        .project_command(
            &format!("project-{title}"),
            ProjectCommand::Create {
                title: title.into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id
}

fn request(data: &std::path::Path, chat: &str) -> ThreadRunRequest {
    request_for(data, chat, LOCATOR)
}

fn request_for(data: &std::path::Path, chat: &str, reference: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: "Retain useful information from the supplied source.".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources: vec![TransientSource::text(
            reference,
            "Can we move crane delivery to 15 November?",
        )],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    }
}

/// One useful intake run of the synthetic email in `chat`.
async fn supply(data: &std::path::Path, chat: &str) {
    supply_from(data, chat, LOCATOR).await
}

/// One useful intake run of `reference` in `chat`.
async fn supply_from(data: &std::path::Path, chat: &str, reference: &str) {
    let prepared = prepare_run(request_for(data, chat, reference)).unwrap();
    let model = MockCompletionModel::from_stream_turns([vec![
        MockStreamEvent::text("Recorded."),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]]);
    run_prepared_scripted(prepared, model).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_same_source_reuses_its_chat_per_scope_and_never_an_archived_or_foreign_one() {
    let (_root, data, mut w) = workspace();
    let harbor = project(&mut w, "Harbor crane");
    let depot = project(&mut w, "Depot");

    // General: the first supply creates a chat, the next one reuses it.
    let (general, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert!(!reused);
    supply(&data, &general.id).await;
    let records = w.records().unwrap().len();
    let receipts = w.history().unwrap().len();
    let (again, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (general.id.as_str(), true));
    assert_eq!(w.records().unwrap().len(), records, "reuse adds no records");
    assert_eq!(w.history().unwrap().len(), receipts);

    // A project gets its own chat for the same locator, then reuses it.
    let (harbor_chat, reused) = w.source_intake_chat(Some(&harbor), LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert_ne!(harbor_chat.id, general.id);
    assert_eq!(
        project_for_chat(&w.records().unwrap(), &harbor_chat.id).map(|p| p.id.clone()),
        Some(harbor.clone())
    );
    supply(&data, &harbor_chat.id).await;
    let (again, reused) = w.source_intake_chat(Some(&harbor), LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (harbor_chat.id.as_str(), true));
    // Its Source stays private to that chat.
    assert!(w.records().unwrap().iter().any(|r| matches!(&r.data,
        RecordData::Source(s) if s.locator == LOCATOR && s.chat.as_deref() == Some(harbor_chat.id.as_str()))));
    // The general scope still reuses the general chat.
    assert_eq!(
        w.source_intake_chat(None, LOCATOR, TITLE).unwrap().0.id,
        general.id
    );

    // Another project's chat is never reused.
    let (depot_chat, reused) = w.source_intake_chat(Some(&depot), LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert!(depot_chat.id != harbor_chat.id && depot_chat.id != general.id);

    // An archived chat is never reused.
    w.archive_chat("archive-general", &general.id).unwrap();
    let (fresh, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert_ne!(fresh.id, general.id);

    // An unknown project is refused.
    let missing = w
        .source_intake_chat(Some("missing"), LOCATOR, TITLE)
        .unwrap_err();
    assert!(missing.to_string().contains("active project"), "{missing}");
}

fn run(workspace: &Workspace, id: &str) -> Run {
    match workspace.store.record(id).unwrap().unwrap().data {
        RecordData::Run(run) => run,
        _ => panic!("Run"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chat_whose_run_is_still_live_is_not_reused_until_its_lease_is_free() {
    let (_root, data, mut w) = workspace();
    let (original, _) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    supply(&data, &original.id).await;
    // Another process is still working on this source there: its prepared
    // Run is Working and holds the lease through its own file handle.
    let live = prepare_run(request(&data, &original.id)).unwrap();
    let live_id = live.run_id().to_owned();
    let before = run(&w, &live_id);
    assert_eq!(before.state, RunState::Working);

    // Before: the original chat was reused and `begin()` fenced the live Run.
    let (other, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    supply(&data, &other.id).await;
    let after = run(&w, &live_id);
    assert_eq!(
        (after.state, after.fence),
        (RunState::Working, before.fence),
        "the live Run is not interrupted"
    );
    assert!(!reused);
    assert_ne!(other.id, original.id);

    // Once its process lets go, the orphan no longer blocks reuse, and the
    // next Run there interrupts it as Continue does.
    drop(live);
    w.archive_chat("archive-other", &other.id).unwrap();
    let (again, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (original.id.as_str(), true));
    supply(&data, &original.id).await;
    let recovered = run(&w, &live_id);
    assert_eq!(
        (recovered.state, recovered.fence),
        (RunState::Interrupted, before.fence + 1)
    );
}

// One file is one source however its path is spelled. These use real files.

/// `real/<name>` holds a file and `alias` is a symlink to `real`. Returns the
/// file's path through `alias`, then its real path.
#[cfg(unix)]
fn linked_file(root: &std::path::Path, name: &str) -> (String, String) {
    let real = root.join("real");
    std::fs::create_dir_all(&real).unwrap();
    let alias = root.join("alias");
    if !alias.exists() {
        std::os::unix::fs::symlink(&real, &alias).unwrap();
    }
    std::fs::write(real.join(name), "Crane delivery moves to 15 November.\n").unwrap();
    let path = |dir: &std::path::Path| dir.join(name).to_str().unwrap().to_owned();
    (path(&alias), path(&real))
}

fn sources(w: &Workspace) -> Vec<Record> {
    w.records()
        .unwrap()
        .into_iter()
        .filter(|r| matches!(r.data, RecordData::Source(_)))
        .collect()
}

fn locator(record: &Record) -> &str {
    match &record.data {
        RecordData::Source(s) => &s.locator,
        _ => panic!("Source"),
    }
}

/// Supplying `first`, then `second` (two spellings of the file at
/// `canonical`) reuses one chat and records one Source at its real path.
async fn one_chat_and_one_source(
    w: &mut Workspace,
    data: &std::path::Path,
    first: &str,
    second: &str,
    canonical: &str,
) {
    let (chat, reused) = w.source_intake_chat(None, first, TITLE).unwrap();
    assert!(!reused);
    supply_from(data, &chat.id, first).await;
    let (again, reused) = w.source_intake_chat(None, second, TITLE).unwrap();
    // Before: a second chat, as the stored spelling differed.
    assert_eq!((again.id.as_str(), reused), (chat.id.as_str(), true));
    supply_from(data, &again.id, second).await;
    let sources = sources(w);
    assert_eq!(sources.len(), 1, "one file is one Source");
    assert_eq!(locator(&sources[0]), canonical);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn the_same_file_through_a_symlinked_folder_reuses_one_chat_and_one_source() {
    let (root, data, mut w) = workspace();
    let (alias, real) = linked_file(root.path(), "mail.txt");
    one_chat_and_one_source(&mut w, &data, &alias, &real, &real).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn tmp_and_private_tmp_spellings_reuse_one_chat_and_one_source() {
    let (_root, data, mut w) = workspace();
    let dir = tempfile::tempdir_in("/private/tmp").unwrap();
    let real = dir.path().join("mail.txt");
    std::fs::write(&real, "Crane delivery moves to 15 November.\n").unwrap();
    let real = real.to_str().unwrap().to_owned();
    let tmp = real.replacen("/private/tmp/", "/tmp/", 1);
    assert_ne!(tmp, real);
    one_chat_and_one_source(&mut w, &data, &tmp, &real, &real).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_different_file_with_the_same_name_gets_its_own_chat_and_source() {
    let (root, data, mut w) = workspace();
    let path = |folder: &str| {
        let dir = root.path().join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mail.txt"), format!("From {folder}.\n")).unwrap();
        dir.join("mail.txt").to_str().unwrap().to_owned()
    };
    let (a, b) = (path("a"), path("b"));
    let (first, _) = w.source_intake_chat(None, &a, TITLE).unwrap();
    supply_from(&data, &first.id, &a).await;
    let (second, reused) = w.source_intake_chat(None, &b, TITLE).unwrap();
    assert!(!reused);
    assert_ne!(second.id, first.id);
    supply_from(&data, &second.id, &b).await;
    let mut locators: Vec<_> = sources(&w).iter().map(|s| locator(s).to_owned()).collect();
    locators.sort();
    assert_eq!(locators, vec![a, b]);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_source_saved_under_an_older_spelling_still_matches_and_is_recorded_canonically() {
    let (root, data, mut w) = workspace();
    let (alias, real) = linked_file(root.path(), "mail.txt");
    // An earlier intake saved its Source under the alias spelling.
    let chat = w.new_thread(TITLE).unwrap().id;
    let op = w.store.allocate_operation("older-spelling").unwrap();
    let source_id = op.creation_id(0);
    let authority = HostAuthority::owner("synthetic earlier intake");
    w.store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Earlier useful intake".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: source_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Source(Source {
                            chat: None,
                            locator: alias.clone(),
                            label: alias.clone(),
                            observed_at: "unix:1".into(),
                            external_version: None,
                            outcome: "Supplied for useful information; not archived".into(),
                            gaps: vec![],
                            report: None,
                        }),
                    },
                    Put {
                        id: op.creation_id(1),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            instruction: None,
                            queue_claim: None,
                            thread: chat.clone(),
                            provider: "Codex".into(),
                            model: "gpt-6.1-sol".into(),
                            guide_identity: "fixture".into(),
                            budget: 12,
                            effort: "medium".into(),
                            loaded_skills: vec![],
                            fence: 1,
                            state: RunState::Failed,
                            progress: String::new(),
                            operations: vec![],
                            sources: vec![source_id.clone()],
                        }),
                    },
                ],
            },
            &authority,
        )
        .unwrap();
    assert!(matches!(
        w.store.apply(&op, &authority).unwrap(),
        ApplyOutcome::Applied(_)
    ));

    let (again, reused) = w.source_intake_chat(None, &real, TITLE).unwrap();
    // Before: a new chat for the real spelling.
    assert_eq!((again.id.as_str(), reused), (chat.as_str(), true));
    supply_from(&data, &chat, &real).await;
    let sources = sources(&w);
    // Before: a second Source with the real spelling.
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id, source_id);
    assert_eq!(locator(&sources[0]), real);
    assert!(sources[0].version > 1);
    // The earlier spelling stays in history.
    let earlier = w.store.revision(&source_id, 1).unwrap().unwrap();
    assert_eq!(locator(&earlier), alias);
    let runs: Vec<_> = w
        .records()
        .unwrap()
        .into_iter()
        .filter_map(|r| match r.data {
            RecordData::Run(run) if run.thread == chat => Some(run.sources),
            _ => None,
        })
        .collect();
    assert_eq!(runs, vec![vec![source_id.clone()], vec![source_id]]);
}

fn full_note(reference: &str) -> brn_threads_intake::ConversionResult {
    brn_threads_intake::ConversionResult {
        intent: brn_threads_intake::ImportIntent::FullNote,
        markdown: "# Crane report\n\nDelivery moves to 15 November.\n".into(),
        assets: vec![],
        source_sha256: [3; 32],
        source_reference: reference.into(),
        converter: "synthetic".into(),
        sources: vec![],
        coverage: brn_threads_intake::Coverage {
            status: brn_threads_intake::CoverageStatus::Unchecked,
            checked_items: vec![],
            gaps: vec![],
        },
    }
}

#[cfg(unix)]
#[test]
fn full_import_of_one_file_through_two_spellings_observes_one_source() {
    let (root, _data, mut w) = workspace();
    let (alias, real) = linked_file(root.path(), "report.md");
    let first = w
        .import_full("first", "Crane report", &full_note(&alias))
        .unwrap();
    assert!(matches!(first, ApplyOutcome::Applied(_)));
    let ApplyOutcome::Applied(second) = w
        .import_full("second", "Crane report", &full_note(&real))
        .unwrap()
    else {
        panic!("Applied")
    };
    // Before: a second Source and a second full note.
    assert_eq!(
        second.reason,
        "Observed previously imported full note: Crane report"
    );
    let sources = sources(&w);
    assert_eq!(sources.len(), 1);
    assert_eq!(locator(&sources[0]), real);
}
