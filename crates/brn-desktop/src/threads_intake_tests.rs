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

fn offline_workspace(data: &Path) -> Workspace {
    let mut workspace = Workspace::open(data).unwrap();
    let mut settings = workspace.settings().unwrap();
    settings.provider = "Codex".into();
    settings.model = "gpt-6.1-sol".into();
    settings.effort = "medium".into();
    // No credentials at all: a run fails before any credential or network access.
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
    workspace
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("intake");
    let workspace = offline_workspace(&data);
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

#[cfg(unix)]
#[gpui_kit::test]
fn supplying_one_file_through_two_spellings_reuses_its_chat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    // A real file in `real/`, also reachable through the symlinked `alias/`.
    let root = f.data.parent().unwrap();
    let real = root.join("real");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, root.join("alias")).unwrap();
    std::fs::write(
        real.join("mari-email.txt"),
        "The berth survey measured 1.4 m.\n",
    )
    .unwrap();
    let alias = root.join("alias/mari-email.txt");
    let real = real.join("mari-email.txt");
    let first = supply(cx, &f, alias.to_str().unwrap());
    let second = supply(cx, &f, real.to_str().unwrap());
    // Before: a second "Information from mari-email" chat.
    assert_eq!(second, first);
    let records = saved(cx, &f);
    assert_eq!(intake_chats(&records), vec![first]);
    let locators: Vec<_> = records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Source(s) => Some(s.locator.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(locators, vec![real.to_str().unwrap()]);
}

/// `(id, text)` of each owner Message in `chat`, oldest first.
fn owner_messages(workspace: &Workspace, chat: &str) -> Vec<(String, String)> {
    workspace
        .messages(chat)
        .unwrap()
        .into_iter()
        .filter_map(|r| match r.data {
            RecordData::Message(m) if m.role == "owner" => Some((r.id, m.text)),
            _ => None,
        })
        .collect()
}

/// `(state, instruction)` of each Run in `chat`.
fn chat_runs(records: &[Record], chat: &str) -> Vec<(RunState, Option<String>)> {
    records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Run(run) if run.thread == chat => {
                Some((run.state.clone(), run.instruction.clone()))
            }
            _ => None,
        })
        .collect()
}

/// `brn intake --source <reference> --intent useful` in-process: the intake
/// chat, then a Run with the shared instruction and no posted Message, which
/// fails offline. Returns the chat.
fn cli_useful_intake(workspace: &mut Workspace, data: &Path, reference: &str) -> String {
    let (chat, _) = workspace
        .source_intake_chat(None, reference, "Useful information intake")
        .unwrap();
    let converted = email(reference);
    let request = brn_ai::threads::ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.id.clone(),
        prompt: brn_threads_app::SOURCE_INTAKE_INSTRUCTION.into(),
        selection: brn_ai::Selection {
            provider: brn_ai::Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: brn_ai::ReasoningEffort::Medium,
        max_tool_rounds: brn_ai::threads::default_tool_rounds(true),
        auth_file: None,
        credentials_dir: None,
        sources: vec![brn_ai::threads::TransientSource {
            fingerprint: Some(brn_intake::hex(&converted.source_sha256)),
            report: Some(brn_threads_app::import_report(&converted)),
            reference: converted.source_reference,
            text: converted.markdown,
            assets: vec![],
        }],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    };
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(brn_ai::threads::run(
            request,
            CancellationToken::new(),
            Arc::new(|_| {}),
        ))
        .unwrap();
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Failed(_)
    ));
    chat.id
}

#[gpui_kit::test]
fn supplying_the_same_source_twice_posts_one_instruction(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let chat = supply(cx, &f, SOURCE);
    assert_eq!(supply(cx, &f, SOURCE), chat);
    // Before: a second, identical owner instruction.
    let owner = cx.read(|cx| owner_messages(&f.view.read(cx).workspace, &chat));
    assert_eq!(owner.len(), 1, "{owner:?}");
    assert_eq!(owner[0].1, brn_threads_app::SOURCE_INTAKE_INSTRUCTION);
    assert_eq!(
        chat_runs(&saved(cx, &f), &chat),
        vec![(RunState::Failed, Some(owner[0].0.clone())); 2]
    );
}

#[gpui_kit::test]
fn a_cli_useful_intake_then_a_desktop_supply_post_one_instruction(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("intake");
    let mut workspace = offline_workspace(&data);
    let chat = cli_useful_intake(&mut workspace, &data, SOURCE);
    let f = Fixture {
        _root: Some(root),
        ..open(cx, workspace, data)
    };
    assert_eq!(supply(cx, &f, SOURCE), chat);
    // Before: the CLI's and the desktop's instructions differed, so two.
    let owner = cx.read(|cx| owner_messages(&f.view.read(cx).workspace, &chat));
    assert_eq!(owner.len(), 1, "{owner:?}");
    assert_eq!(owner[0].1, brn_threads_app::SOURCE_INTAKE_INSTRUCTION);
    assert_eq!(
        chat_runs(&saved(cx, &f), &chat),
        vec![(RunState::Failed, Some(owner[0].0.clone())); 2]
    );
}

#[gpui_kit::test]
fn owner_text_between_supplies_gets_a_fresh_instruction(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let chat = supply(cx, &f, SOURCE);
    update(cx, &f, |ui, _, _| {
        ui.workspace
            .post_message(&chat, "Focus on the berth depth")
            .unwrap();
    });
    assert_eq!(supply(cx, &f, SOURCE), chat);
    let owner = cx.read(|cx| owner_messages(&f.view.read(cx).workspace, &chat));
    let texts: Vec<_> = owner.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(
        texts,
        [
            brn_threads_app::SOURCE_INTAKE_INSTRUCTION,
            "Focus on the berth depth",
            brn_threads_app::SOURCE_INTAKE_INSTRUCTION
        ]
    );
    assert_eq!(
        chat_runs(&saved(cx, &f), &chat)
            .into_iter()
            .map(|(_, instruction)| instruction)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([Some(owner[0].0.clone()), Some(owner[2].0.clone())])
    );
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
    lease_path(&ui.workspace, run)
}

fn lease_path(workspace: &Workspace, run: &str) -> PathBuf {
    workspace
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
    let (run, operation) = killed_run_in(&mut ui.workspace, data, &chat, None);
    ui.refresh();
    (chat, run, operation)
}

/// As [`killed_intake_in`], in `chat`. With `queued`, the Run is that
/// dispatched queue item's, built as `start_run` builds it. Returns the Run
/// and candidate operation.
fn killed_run_in(
    workspace: &mut Workspace,
    data: &Path,
    chat: &str,
    queued: Option<&QueuedMessage>,
) -> (String, String) {
    let live = brn_ai::threads::prepare_run(brn_ai::threads::ThreadRunRequest {
        request_message: queued.and_then(|q| q.message.clone()),
        queue_claim: queued.and_then(|q| q.dispatch.clone()),
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: queued
            .map_or("Capture useful information.", |q| q.text.as_str())
            .into(),
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
    let store = &mut workspace.store;
    let operation = store
        .allocate_operation(&format!("{run}:proposal:crane-arrival-source-capture-1"))
        .unwrap();
    // As the runtime's prepare tool does: the Run names the operation first.
    let saved = store.record(&run).unwrap().unwrap();
    let RecordData::Run(mut working) = saved.data else {
        panic!("Run")
    };
    working.operations.push(operation.clone());
    let recorded = workspace
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
    workspace
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
    std::fs::write(lease_path(workspace, &run), b"").unwrap();
    (run, operation.as_str().to_owned())
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
                RecordData::Run(r) if r.state == RunState::Interrupted)
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
        // The run line above Continue shows the latest Run's state. Before
        // slice D it said Cancelled beside the "Run interrupted" notice.
        let latest = latest_run(&ui.records, &ui.history, &chat).unwrap();
        let RecordData::Run(latest_state) = &latest.data else {
            panic!("Run")
        };
        assert_eq!(
            (latest.id.as_str(), format!("{:?}", latest_state.state)),
            (run.as_str(), "Interrupted".to_owned())
        );
    });
    // Two attention cards; the first carries the exact candidate's proposal.
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find(format!("continue-{run}")).is_some());
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

/// A kill during a queued intake, then a restart and Resume. The killed Run is
/// the pending item's, so Resume goes through the desktop's resume-stale stop.
#[gpui_kit::test]
fn resume_after_a_killed_queued_intake_shows_one_notice(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("intake");
    let mut workspace = offline_workspace(&data);
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let chat = workspace.new_thread(KILLED_TITLE).unwrap().id;
    workspace
        .queue_message("killed-queue", &chat, "Capture useful information.")
        .unwrap();
    let queued = workspace
        .dispatch_queued("killed-dispatch")
        .unwrap()
        .unwrap();
    let (run, operation) = killed_run_in(&mut workspace, &data, &chat, Some(&queued));
    // The restart: startup pauses the saved queue.
    let f = Fixture {
        _root: Some(root),
        ..open(cx, workspace, data)
    };
    update(cx, &f, |ui, _, _| {
        let queue = ui.queue_state();
        assert!(queue.paused);
        assert_eq!(
            queue.pending.and_then(|q| q.run).as_deref(),
            Some(run.as_str())
        );
        assert!(ui.worker.is_none());
        assert!(row(ui, &chat).reasons.is_empty());
        assert!(lease_file(ui, &run).exists());
    });
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
        // Before: the resume-stale stop added only the Review and left the
        // lease file.
        assert!(!lease_file(ui, &run).exists());
        let history = ui.workspace.history().unwrap();
        let stop = history
            .iter()
            .find(|receipt| {
                receipt.writes.iter().any(|w| {
                    w.after.id == run
                        && matches!(&w.before, Some(Record { data: RecordData::Run(r), .. }) if r.state == RunState::Working)
                        && matches!(&w.after.data, RecordData::Run(r) if r.state == RunState::Interrupted)
                })
            })
            .expect("the resume-stale stop");
        assert_eq!(stop.reason, "Chat control: stop and pause");
        let stopped_with = stop
            .writes
            .iter()
            .find_map(|w| match &w.after.data {
                RecordData::Thread(t) if w.after.id == chat => Some(
                    t.attention
                        .iter()
                        .map(|a| a.reason.clone())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .expect("the stop's chat write");
        assert_eq!(stopped_with, killed_reasons());
        // The item ran once more and failed offline; its result replaced the
        // notice, so the chat names one cause.
        let mut states: Vec<_> = ui
            .workspace
            .records()
            .unwrap()
            .iter()
            .filter_map(|r| match &r.data {
                RecordData::Run(r) if r.thread == chat => Some(r.state.clone()),
                _ => None,
            })
            .collect();
        states.sort_by_key(|state| format!("{state:?}"));
        assert_eq!(states, vec![RunState::Failed, RunState::Interrupted]);
        let reasons = &row(ui, &chat).reasons;
        assert_eq!(reasons.len(), 2, "{reasons:?}");
        assert_eq!(reasons[0], brn_threads_app::run_attention::REVIEW_ATTENTION);
        assert!(reasons[1].starts_with("Run failed: "), "{reasons:?}");
        assert!(in_needs_you(ui, &chat));
        assert!(has_candidate(ui, &operation));
        let asked = ui
            .workspace
            .records()
            .unwrap()
            .iter()
            .filter(|r| matches!(&r.data, RecordData::Message(m) if m.thread == chat && m.role == "owner"))
            .count();
        assert_eq!(asked, 1);
    });
}

/// Owner decision 2 (16 October 2026), through the desktop's Archive pill:
/// archiving a chat whose Run was killed adds the Review item for its
/// proposal but no notice, and removes the free lease file.
#[gpui_kit::test]
fn archiving_a_killed_chat_adds_its_review_but_no_notice(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (chat, run, operation) = update(cx, &f, |ui, _, cx| {
        ui.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx);
        let killed = killed_intake_in(ui, &f.data);
        assert!(row(ui, &killed.0).reasons.is_empty());
        killed
    });
    update(cx, &f, |ui, _, cx| ui.resolve_thread(&chat, cx));
    update(cx, &f, |ui, _, _| {
        let saved = ui.workspace.store.record(&chat).unwrap().unwrap();
        assert!(saved.archived);
        assert!(
            matches!(&ui.workspace.store.record(&run).unwrap().unwrap().data,
                RecordData::Run(r) if r.state == RunState::Interrupted && r.fence == 2)
        );
        // Before (round 5 C): the Review and "Run interrupted. Continue
        // starts fresh.".
        assert_eq!(
            row(ui, &chat).reasons,
            vec![brn_threads_app::run_attention::REVIEW_ATTENTION.to_owned()]
        );
        assert!(has_candidate(ui, &operation));
        assert!(!lease_file(ui, &run).exists());
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
    let owner_before = owner_messages(&workspace, &chat);
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
    // The desktop reused the CLI's instruction: no owner Message was added,
    // and the new Run names the CLI's latest one.
    let owner = cx.read(|cx| owner_messages(&f.view.read(cx).workspace, &chat));
    assert_eq!(owner, owner_before);
    let latest = owner.last().unwrap();
    assert_eq!(latest.1, brn_threads_app::SOURCE_INTAKE_INSTRUCTION);
    let named = chat_runs(&records, &chat)
        .into_iter()
        .filter(|(_, instruction)| instruction.as_ref() == Some(&latest.0))
        .count();
    assert!(named >= 2, "the CLI's and the desktop's Runs name it");
}
/// Round 5 slice F: the notes search answer the desktop receives carries a
/// saved chat Message as its own kind, with its chat. The desktop renders
/// only note hits for now (UI finding F-UI1), so this checks the data.
#[gpui_kit::test]
fn the_notes_search_answer_carries_a_saved_chat_message(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let chat = update(cx, &f, |ui, _, _| {
        let chat = ui.workspace.new_thread("Replies").unwrap().id;
        ui.workspace
            .post_message(&chat, "Berth survey reply draft 1.4 m")
            .unwrap();
        chat
    });
    update(cx, &f, |ui, window, cx| {
        ui.search_changed("berth".into(), window, cx)
    });
    cx.read(|cx| {
        let ui = f.view.read(cx);
        let page = ui.results.page.clone().expect("answered");
        assert_eq!((page.total, ui.hits.len()), (0, 0));
        assert_eq!(page.message_total, 1);
        let hit = &page.messages[0];
        assert_eq!(
            (
                hit.chat.as_str(),
                hit.chat_title.as_str(),
                hit.role.as_str()
            ),
            (chat.as_str(), "Replies", "owner")
        );
        assert!(!hit.archived);
    });
    // Owner decision 3: after Archive the Message stays, labelled for the
    // UI's Archived marker (F-UI1 renders it on the combined branch).
    update(cx, &f, |ui, window, cx| {
        ui.resolve_thread(&chat, cx);
        ui.search_changed("berth survey".into(), window, cx)
    });
    cx.read(|cx| {
        let page = f.view.read(cx).results.page.clone().expect("answered");
        assert_eq!(page.message_total, 1);
        assert!(page.messages[0].archived);
    });
}
