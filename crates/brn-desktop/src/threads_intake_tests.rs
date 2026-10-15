//! Desktop useful intake reuses the chat that already received the same
//! source, in the open chat's project or in general. Synthetic data; no
//! provider credentials, so every run fails offline as ReconnectNeeded.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
use std::path::Path;

const SOURCE: &str = "/synthetic/harbor/mari-email.txt";
const INTAKE_TITLE: &str = "Information from mari-email";

struct Fixture {
    _root: Option<tempfile::TempDir>,
    data: PathBuf,
    view: Entity<ThreadsUi>,
    window: gpui_kit::AnyWindowHandle,
}

fn open(cx: &mut TestAppContext, workspace: Workspace, data: PathBuf) -> Fixture {
    cx.update(gpui_kit::init);
    let view_data = data.clone();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, view_data, window, cx);
                ui.poll = None;
                ui
            })
        })
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: None,
        data,
        view,
        window,
    }
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("intake");
    let mut workspace = Workspace::open(&data).unwrap();
    let mut settings = workspace.settings().unwrap();
    settings.provider = "Codex".into();
    settings.model = "gpt-6.1-sol".into();
    settings.effort = "medium".into();
    // No credentials at all: a run fails before any credential or network access.
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
    Fixture {
        _root: Some(root),
        ..open(cx, workspace, data)
    }
}

fn email(reference: &str) -> ConversionResult {
    ConversionResult {
        intent: ImportIntent::UsefulInformation,
        markdown: "From: Mari Example\n\nThe berth survey measured 1.4 m.\n".into(),
        assets: vec![],
        source_sha256: [7; 32],
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

fn update<T>(
    cx: &mut TestAppContext,
    f: &Fixture,
    change: impl FnOnce(&mut ThreadsUi, &mut Window, &mut Context<ThreadsUi>) -> T,
) -> T {
    let result = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |ui, cx| change(ui, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

/// Supplies `reference` as useful information, waits for the offline run to
/// finish and returns the chat it ran in.
fn supply(cx: &mut TestAppContext, f: &Fixture, reference: &str) -> String {
    let chat = update(cx, f, |ui, window, cx| {
        ui.useful_intake(email(reference), "mari-email".into(), window, cx);
        assert!(ui.worker.is_some(), "{}", ui.notice);
        ui.chat_thread().unwrap().to_owned()
    });
    for _ in 0..1000 {
        if update(cx, f, |ui, window, cx| {
            ui.poll_worker(window, cx);
            ui.worker.is_none()
        }) {
            return chat;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the offline run did not finish");
}

fn saved(cx: &mut TestAppContext, f: &Fixture) -> Vec<Record> {
    cx.read(|cx| f.view.read(cx).workspace.records().unwrap())
}

fn intake_chats(records: &[Record]) -> Vec<String> {
    records
        .iter()
        .filter(|r| matches!(&r.data, RecordData::Thread(t) if t.title == INTAKE_TITLE))
        .map(|r| r.id.clone())
        .collect()
}

/// `(chat, state)` of each Run that received `SOURCE`.
fn intake_runs(records: &[Record]) -> Vec<(String, RunState)> {
    let sources: Vec<&str> = records
        .iter()
        .filter(|r| matches!(&r.data, RecordData::Source(s) if s.locator == SOURCE))
        .map(|r| r.id.as_str())
        .collect();
    records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Run(run) if run.sources.iter().any(|s| sources.contains(&s.as_str())) => {
                Some((run.thread.clone(), run.state.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The `chat` field of each Source record for `SOURCE`.
fn source_scopes(records: &[Record]) -> Vec<Option<String>> {
    records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Source(s) if s.locator == SOURCE => Some(s.chat.clone()),
            _ => None,
        })
        .collect()
}

#[gpui_kit::test]
fn supplying_the_same_source_twice_in_general_reuses_its_chat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let first = supply(cx, &f, SOURCE);
    let second = supply(cx, &f, SOURCE);
    // Before: a second "Information from mari-email" chat.
    assert_eq!(second, first);
    let records = saved(cx, &f);
    assert_eq!(intake_chats(&records), vec![first.clone()]);
    assert_eq!(
        intake_runs(&records),
        vec![
            (first.clone(), RunState::Failed),
            (first.clone(), RunState::Failed)
        ]
    );
    assert_eq!(source_scopes(&records), vec![None]);
    assert!(brn_threads_app::project_for_chat(&records, &first).is_none());
}

#[gpui_kit::test]
fn supplying_from_inside_a_project_keeps_the_source_in_a_project_chat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let general = supply(cx, &f, SOURCE);
    let (project, inside) = update(cx, &f, |ui, window, cx| {
        let project = ui
            .workspace
            .project_command(
                "intake-project",
                ProjectCommand::Create {
                    title: "Harbor crane".into(),
                },
            )
            .unwrap()
            .writes
            .into_iter()
            .find(|w| matches!(w.after.data, RecordData::Project(_)))
            .unwrap()
            .after
            .id;
        let inside = ui
            .workspace
            .project_command(
                "intake-project-chat",
                ProjectCommand::CreateChat {
                    project: project.clone(),
                    title: "Crane delivery".into(),
                },
            )
            .unwrap()
            .writes
            .into_iter()
            .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
            .unwrap()
            .after
            .id;
        ui.refresh();
        ui.open_thread(&inside, window, cx);
        (project, inside)
    });
    let scoped = supply(cx, &f, SOURCE);
    // Before: a new general chat, so the source left the project.
    assert_ne!(scoped, general);
    assert_ne!(scoped, inside, "the open chat never received this source");
    let records = saved(cx, &f);
    assert_eq!(
        brn_threads_app::project_for_chat(&records, &scoped).map(|p| p.id.clone()),
        Some(project.clone())
    );
    let mut scopes = source_scopes(&records);
    scopes.sort();
    assert_eq!(scopes, vec![None, Some(scoped.clone())]);
    // Supplying it again from inside the project reuses the project chat.
    update(cx, &f, |ui, window, cx| ui.open_thread(&inside, window, cx));
    assert_eq!(supply(cx, &f, SOURCE), scoped);
    let records = saved(cx, &f);
    assert_eq!(intake_chats(&records).len(), 2);
    assert_eq!(source_scopes(&records).len(), 2);
}

#[gpui_kit::test]
fn a_chat_whose_run_is_still_live_is_not_reused(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let first = supply(cx, &f, SOURCE);
    // Another BRN process is still running work in that chat.
    let live = brn_ai::threads::prepare_run(brn_ai::threads::ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: f.data.clone(),
        thread: first.clone(),
        prompt: "Still working in another window".into(),
        selection: brn_ai::Selection {
            provider: brn_ai::Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: brn_ai::ReasoningEffort::Medium,
        max_tool_rounds: 4,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    })
    .unwrap();
    let second = supply(cx, &f, SOURCE);
    assert_ne!(second, first);
    let records = saved(cx, &f);
    assert!(
        matches!(&records.iter().find(|r| r.id == live.run_id()).unwrap().data,
            RecordData::Run(run) if run.state == RunState::Working && run.fence == 1),
        "the live Run must not be interrupted"
    );
    drop(live);
}

const KILLED_TITLE: &str = "Information from b1-source";

fn lease_file(ui: &ThreadsUi, run: &str) -> PathBuf {
    ui.workspace
        .store
        .derived_directory()
        .unwrap()
        .join("run-leases")
        .join(format!("{}.lock", run.replace(':', "_")))
}

/// The store the owner found after `pkill -9` during a useful intake
/// (`native-b1-kill`): a Working Run whose candidate has no attention and
/// whose unlocked lease file is left behind. The desktop has no scripted
/// runtime, so the prepared candidate is stored directly. Returns the chat,
/// Run and candidate operation.
fn killed_intake_in(ui: &mut ThreadsUi, data: &Path) -> (String, String, String) {
    let chat = ui.workspace.new_thread(KILLED_TITLE).unwrap().id;
    let live = brn_ai::threads::prepare_run(brn_ai::threads::ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.clone(),
        prompt: "Capture useful information.".into(),
        selection: brn_ai::Selection {
            provider: brn_ai::Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: brn_ai::ReasoningEffort::Medium,
        max_tool_rounds: 4,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    })
    .unwrap();
    let run = live.run_id().to_owned();
    let store = &mut ui.workspace.store;
    let operation = store
        .allocate_operation(&format!("{run}:proposal:crane-arrival-source-capture-1"))
        .unwrap();
    // As the runtime's prepare tool does: the Run names the operation first.
    let saved = store.record(&run).unwrap().unwrap();
    let RecordData::Run(mut working) = saved.data else {
        panic!("Run")
    };
    working.operations.push(operation.clone());
    let recorded = ui
        .workspace
        .owner_change(
            &format!("{run}:names-operation"),
            &ChangeRequest {
                reason: "Runtime progress".into(),
                writes: vec![Put {
                    id: run.clone(),
                    expected_version: Some(saved.version),
                    archived: false,
                    data: RecordData::Run(working),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    assert!(matches!(recorded, ApplyOutcome::Applied(_)));
    ui.workspace
        .store
        .prepare(
            &operation,
            &ChangeRequest {
                reason: "Record the reported crane arrival date".into(),
                writes: vec![Put {
                    id: operation.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "Crane arrival — reported date",
                        "# Crane arrival — reported date\n\nThe source reports 22 November.\n",
                    )),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    drop(live);
    std::fs::write(lease_file(ui, &run), b"").unwrap();
    ui.refresh();
    (chat, run, operation.as_str().to_owned())
}

fn row<'a>(ui: &'a ThreadsUi, chat: &str) -> &'a ThreadRow {
    ui.thread_rows.iter().find(|row| row.id == chat).unwrap()
}

fn in_needs_you(ui: &ThreadsUi, chat: &str) -> bool {
    lists::sidebar(&ui.thread_rows, None)
        .needs
        .iter()
        .any(|row| row.id == chat)
}

fn has_candidate(ui: &ThreadsUi, operation: &str) -> bool {
    ui.candidates
        .iter()
        .any(|c| c.operation.as_str() == operation)
}

fn killed_reasons() -> Vec<String> {
    vec![
        brn_threads_app::run_attention::REVIEW_ATTENTION.to_owned(),
        brn_ai::threads::INTERRUPTED_ATTENTION.to_owned(),
    ]
}

#[gpui_kit::test]
fn a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (chat, run, operation) = update(cx, &f, |ui, _, _| {
        assert!(!ui.settings.delegation_chosen);
        let killed = killed_intake_in(ui, &f.data);
        let (chat, _, operation) = &killed;
        // What the owner saw: only the sidebar "Review · …" row, no Needs you.
        assert!(row(ui, chat).reasons.is_empty());
        assert!(!in_needs_you(ui, chat));
        assert!(has_candidate(ui, operation));
        killed
    });
    update(cx, &f, |ui, window, cx| {
        ui.show_tile(TileKind::Overview, window, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("home-needs-you").is_some());
        assert!(window.try_find(format!("home-attention-{chat}")).is_none());
    })
    .unwrap();
    update(cx, &f, |ui, _, cx| {
        ui.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx);
        assert!(
            matches!(&ui.workspace.store.record(&run).unwrap().unwrap().data,
                RecordData::Run(r) if r.state == RunState::Cancelled)
        );
        assert_eq!(row(ui, &chat).reasons, killed_reasons());
        assert!(in_needs_you(ui, &chat));
        assert!(has_candidate(ui, &operation));
        cx.notify();
    });
    // Home's Needs you now lists the chat.
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find(format!("home-attention-{chat}")).is_some());
    })
    .unwrap();
    update(cx, &f, |ui, window, cx| {
        ui.open_thread(&chat, window, cx);
        // The chat's proposal card joins attention to a listed candidate.
        let RecordData::Thread(thread) = &ui.records.iter().find(|r| r.id == chat).unwrap().data
        else {
            panic!("Thread")
        };
        assert_eq!(
            thread.attention[0].record.as_deref(),
            Some(operation.as_str())
        );
        assert!(has_candidate(
            ui,
            thread.attention[0].record.as_deref().unwrap()
        ));
    });
    // Two attention cards; the first carries the exact candidate's proposal.
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("attention-card-0").is_some());
        assert!(window.try_find("attention-card-1").is_some());
        assert!(
            window
                .within("attention-card-0")
                .try_find(format!("proposal-{operation}"))
                .is_some()
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn resume_after_a_kill_recovers_the_intake_and_keeps_its_candidate_reviewable(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (chat, run, operation, questions) = update(cx, &f, |ui, _, cx| {
        ui.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx);
        let (chat, run, operation) = killed_intake_in(ui, &f.data);
        // Restart with a saved queue: the question waits, paused.
        let questions = ui.workspace.new_thread("Questions").unwrap().id;
        ui.workspace
            .queue_message("killed-queue", &questions, "When can the crane arrive?")
            .unwrap();
        ui.workspace.recover_queue("killed-restart").unwrap();
        ui.refresh();
        // The Resume pill shows for a saved item and is enabled when paused
        // with no worker.
        let queue = ui.queue_state();
        assert!(queue.paused && (queue.pending.is_some() || !queue.queue.is_empty()));
        assert!(ui.worker.is_none());
        assert!(row(ui, &chat).reasons.is_empty());
        (chat, run, operation, questions)
    });
    update(cx, &f, |ui, window, cx| {
        ui.open_thread(&questions, window, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("queue-resume").is_some());
    })
    .unwrap();
    update(cx, &f, |ui, _, cx| ui.resume_chat_queue(cx));
    for _ in 0..1000 {
        if update(cx, &f, |ui, window, cx| {
            ui.poll_worker(window, cx);
            ui.worker.is_none()
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    update(cx, &f, |ui, _, _| {
        assert!(ui.worker.is_none(), "the offline run did not finish");
        ui.refresh();
        assert!(
            matches!(&ui.workspace.store.record(&run).unwrap().unwrap().data,
                RecordData::Run(r) if r.state == RunState::Interrupted && r.fence == 2)
        );
        assert_eq!(row(ui, &chat).reasons, killed_reasons());
        assert!(in_needs_you(ui, &chat));
        assert!(has_candidate(ui, &operation));
        assert!(!lease_file(ui, &run).exists());
        // The queued question ran once and failed offline.
        let records = ui.workspace.records().unwrap();
        let runs: Vec<_> = records
            .iter()
            .filter_map(|r| match &r.data {
                RecordData::Run(r) if r.thread == questions => Some(r.state.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(runs, vec![RunState::Failed]);
        let asked = records
            .iter()
            .filter(|r| matches!(&r.data, RecordData::Message(m) if m.thread == questions && m.role == "owner"))
            .count();
        assert_eq!(asked, 1);
    });
}

/// The CLI half runs first on shared synthetic data from the same build:
/// `brn intake --intent useful --source SOURCE` with offline credentials.
#[gpui_kit::test]
#[ignore = "needs BRN_DWR_SHARED_DATA, BRN_DWR_SHARED_SOURCE and BRN_DWR_SHARED_CHAT from a CLI useful intake"]
fn a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left(cx: &mut TestAppContext) {
    let data = PathBuf::from(std::env::var("BRN_DWR_SHARED_DATA").unwrap());
    let source = std::env::var("BRN_DWR_SHARED_SOURCE").unwrap();
    let chat = std::env::var("BRN_DWR_SHARED_CHAT").unwrap();
    let workspace = Workspace::open(&data).unwrap();
    let runs = |records: &[Record]| {
        records
            .iter()
            .filter(|r| matches!(&r.data, RecordData::Run(run) if run.thread == chat))
            .count()
    };
    let threads = workspace.threads().unwrap().len();
    let runs_before = runs(&workspace.records().unwrap());
    assert!(runs_before >= 1, "the CLI intake ran in {chat}");
    let f = open(cx, workspace, data);
    assert_eq!(supply(cx, &f, &source), chat);
    let records = saved(cx, &f);
    assert_eq!(
        records
            .iter()
            .filter(|r| !r.archived && matches!(r.data, RecordData::Thread(_)))
            .count(),
        threads,
        "no new chat"
    );
    assert_eq!(runs(&records), runs_before + 1);
}
