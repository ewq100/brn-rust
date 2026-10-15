//! Desktop useful intake reuses the chat that already received the same
//! source, in the open chat's project or in general. Synthetic data; no
//! provider credentials, so every run fails offline as ReconnectNeeded.
use super::*;
use gpui_kit::TestAppContext;

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
