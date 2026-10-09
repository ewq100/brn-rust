//! Checked Action reads and exact identified completion through the shared workflow.
use super::{
    error::CliError, expect_positionals, positional_uuid, scan, sub_word, usage, CliFailure,
    Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    action_completion::CompleteActionRequest,
    actions::{ActionCursor, ActionListRequest, ActionState},
    app_worker::{AppCommand, AppEvent},
    dashboard::{DashboardFilter, DashboardRequest},
};
use std::{fmt::Write as _, path::PathBuf};
use uuid::Uuid;

pub enum ActionsCommand {
    Show(Uuid),
    List(ActionListRequest),
    Complete(PathBuf),
    Dashboard(DashboardRequest),
}

impl ActionsCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Show(_) => "actions.show",
            Self::List(_) => "actions.list",
            Self::Complete(_) => "actions.complete",
            Self::Dashboard(_) => "actions.dashboard",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "actions", "show|list|complete|dashboard")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "show" => ("actions.show", &[]),
        "complete" => ("actions.complete", &[("file", true)]),
        "dashboard" => (
            "actions.dashboard",
            &[
                ("as-of", true),
                ("filter", true),
                ("limit", true),
                ("before-created-at-ms", true),
                ("before-id", true),
            ],
        ),
        "list" => (
            "actions.list",
            &[
                ("state", true),
                ("limit", true),
                ("before-created-at-ms", true),
                ("before-id", true),
            ],
        ),
        _ => {
            return Err(usage(
                "unknown actions subcommand (expected show|list|complete|dashboard)",
            ))
        }
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

fn state(raw: &str) -> Result<Option<ActionState>, CliError> {
    match raw {
        "open" => Ok(Some(ActionState::Open)),
        "waiting" => Ok(Some(ActionState::Waiting)),
        "blocked" => Ok(Some(ActionState::Blocked)),
        "completed" => Ok(Some(ActionState::Completed)),
        "all" => Ok(None),
        _ => Err(usage("--state must be open|waiting|blocked|completed|all")),
    }
}

fn timestamp(raw: &str) -> Result<u64, CliError> {
    raw.parse::<u64>()
        .map_err(|_| usage("invalid --before-created-at-ms integer"))
}

fn dashboard_filter(raw: &str) -> Result<DashboardFilter, CliError> {
    match raw {
        "active" => Ok(DashboardFilter::Active),
        "open" => Ok(DashboardFilter::Open),
        "waiting" => Ok(DashboardFilter::Waiting),
        "blocked" => Ok(DashboardFilter::Blocked),
        "completed" => Ok(DashboardFilter::Completed),
        "overdue" => Ok(DashboardFilter::Overdue),
        "follow-up" => Ok(DashboardFilter::FollowUp),
        "all" => Ok(DashboardFilter::All),
        _ => Err(usage(
            "--filter must be active|open|waiting|blocked|completed|overdue|follow-up|all",
        )),
    }
}
fn page_input(scanned: &Scanned) -> Result<(usize, Option<ActionCursor>), CliError> {
    let created_at_ms = scanned
        .value("before-created-at-ms")
        .map(timestamp)
        .transpose()?;
    let id = scanned.uuid("before-id")?;
    let before = match (created_at_ms, id) {
        (Some(created_at_ms), Some(id)) => Some(ActionCursor { created_at_ms, id }),
        (None, None) => None,
        _ => {
            return Err(usage(
                "--before-created-at-ms and --before-id must be supplied together",
            ))
        }
    };
    let limit = scanned
        .value("limit")
        .map(|raw| {
            raw.parse::<usize>()
                .map_err(|_| usage("invalid --limit integer"))
        })
        .transpose()?
        .unwrap_or(25);
    Ok((limit, before))
}

pub(super) fn validate(command: &ActionsCommand) -> Result<(), CliError> {
    match command {
        ActionsCommand::Show(id) if id.is_nil() => Err(usage("Action UUID must not be nil")),
        ActionsCommand::Show(_) => Ok(()),
        ActionsCommand::Complete(path) => {
            if path.as_os_str().is_empty() {
                Err(usage("missing --file"))
            } else {
                Ok(())
            }
        }
        ActionsCommand::List(request) => {
            request.validate().map_err(|error| usage(error.to_string()))
        }
        ActionsCommand::Dashboard(request) => {
            request.validate().map_err(|error| usage(error.to_string()))
        }
    }
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<ActionsCommand, CliError> {
    let command = match name {
        "actions.show" => {
            expect_positionals(scanned, 1)?;
            ActionsCommand::Show(positional_uuid(scanned, 0, "UUID")?)
        }
        "actions.complete" => {
            expect_positionals(scanned, 0)?;
            ActionsCommand::Complete(
                scanned
                    .value("file")
                    .map(PathBuf::from)
                    .ok_or_else(|| usage("missing --file"))?,
            )
        }
        "actions.list" => {
            expect_positionals(scanned, 0)?;
            let (limit, before) = page_input(scanned)?;
            ActionsCommand::List(ActionListRequest {
                state: scanned.value("state").map(state).transpose()?.flatten(),
                limit,
                before,
            })
        }
        "actions.dashboard" => {
            expect_positionals(scanned, 0)?;
            let (limit, before) = page_input(scanned)?;
            ActionsCommand::Dashboard(DashboardRequest {
                as_of: scanned.value("as-of").map(str::to_owned),
                filter: scanned
                    .value("filter")
                    .map(dashboard_filter)
                    .transpose()?
                    .unwrap_or_default(),
                limit,
                before,
            })
        }
        _ => unreachable!("scanned Actions command"),
    };
    validate(&command)?;
    Ok(command)
}

pub(super) fn prepare(command: &ActionsCommand) -> Result<AppCommand, CliFailure> {
    validate(command)?;
    Ok(match command {
        ActionsCommand::Show(id) => AppCommand::Action(*id),
        ActionsCommand::List(request) => AppCommand::Actions(request.clone()),
        ActionsCommand::Dashboard(request) => AppCommand::ActionDashboard(request.clone()),
        ActionsCommand::Complete(path) => {
            let request: CompleteActionRequest = super::proposals::input(path)?;
            request
                .validate()
                .map_err(|error| usage(error.to_string()))?;
            AppCommand::CompleteAction(request)
        }
    })
}

pub(super) fn record_text(record: &impl serde::Serialize) -> String {
    let json = serde_json::to_string_pretty(record).expect("Action DTO serializes");
    // serde_json escapes C0, but leaves DEL/C1 controls raw. Preserve JSON's
    // formatting newlines and quote the remaining user-supplied controls too.
    let mut text = String::with_capacity(json.len());
    for ch in json.chars() {
        if ch.is_control() && ch >= '\u{007f}' {
            write!(&mut text, "\\u{:04x}", ch as u32).expect("write to String");
        } else {
            text.push(ch);
        }
    }
    text
}

pub(super) fn output(command: &AppCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let (data, text) = match (command, event) {
        (AppCommand::Action(id), AppEvent::Action(record)) if record.origin.id == *id => {
            let text = format!("{}\n", record_text(&record));
            (serde_json::json!(*record), text)
        }
        (AppCommand::Actions(request), AppEvent::Actions(page))
            if page.entries.len() <= request.limit
                && page
                    .entries
                    .iter()
                    .all(|record| request.state.is_none_or(|state| record.data.state == state)) =>
        {
            let mut text = String::new();
            for (index, record) in page.entries.iter().enumerate() {
                if index > 0 {
                    text.push('\n');
                }
                text.push_str(&record_text(record));
                text.push('\n');
            }
            if page.entries.is_empty() {
                text.push_str("No Actions in this page.\n");
            }
            if let Some(cursor) = page.next_before {
                text.push_str("Next older page cursor: ");
                text.push_str(&serde_json::to_string(&cursor).expect("cursor serializes"));
                text.push('\n');
            }
            (serde_json::json!(*page), text)
        }
        (AppCommand::CompleteAction(request), AppEvent::ActionCompleted(completion))
            if completion.request == *request && completion.validate().is_ok() =>
        {
            let text = format!("{}\n", record_text(&completion));
            (serde_json::json!(*completion), text)
        }
        (AppCommand::ActionDashboard(request), AppEvent::ActionDashboard(page))
            if request.validate_page(&page).is_ok() =>
        {
            let text = format!("{}\n", record_text(&page));
            (serde_json::json!(*page), text)
        }
        _ => {
            return Err(CliError::Workflow(
                "unexpected application reply for Action command".into(),
            )
            .into());
        }
    };
    Ok(Output { text, data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation, Outcome};

    fn parse(args: &[&str]) -> Result<Outcome, super::super::ParseFailure> {
        super::super::parse(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn actions_parser_applies_state_defaults_and_exact_paired_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let base = [
            "actions",
            "list",
            "--data-dir",
            dir.path().to_str().unwrap(),
        ];
        let Outcome::Run(invocation) =
            parse(&base).unwrap_or_else(|failure| panic!("parse failed: {}", failure.error.code()))
        else {
            panic!("run")
        };
        let Command::Actions(ActionsCommand::List(request)) = invocation.command else {
            panic!("Actions list")
        };
        assert_eq!(request, ActionListRequest::default());

        for (raw, expected) in [
            ("open", Some(ActionState::Open)),
            ("waiting", Some(ActionState::Waiting)),
            ("blocked", Some(ActionState::Blocked)),
            ("completed", Some(ActionState::Completed)),
            ("all", None),
        ] {
            let args = [
                "actions",
                "list",
                "--state",
                raw,
                "--limit",
                "200",
                "--before-created-at-ms",
                "42",
                "--before-id",
                "00000000-0000-0000-0000-000000000001",
                "--data-dir",
                dir.path().to_str().unwrap(),
            ];
            let Outcome::Run(invocation) = parse(&args)
                .unwrap_or_else(|failure| panic!("parse failed: {}", failure.error.code()))
            else {
                panic!("run")
            };
            let Command::Actions(ActionsCommand::List(request)) = invocation.command else {
                panic!("Actions list")
            };
            assert_eq!(request.state, expected);
            assert_eq!(request.limit, 200);
            assert_eq!(
                request.before,
                Some(ActionCursor {
                    created_at_ms: 42,
                    id: Uuid::from_u128(1),
                })
            );
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn malformed_actions_refuse_during_parse_without_operational_storage() {
        let dir = tempfile::tempdir().unwrap();
        let id = "00000000-0000-0000-0000-000000000001";
        let nil = Uuid::nil().to_string();
        for args in [
            vec!["actions"],
            vec!["actions", "wat"],
            vec!["actions", "show"],
            vec!["actions", "complete"],
            vec!["actions", "complete", "--file", ""],
            vec!["actions", "complete", "--file", "one", "--file", "two"],
            vec!["actions", "complete", "--file", "one", "extra"],
            vec!["actions", "show", &nil],
            vec!["actions", "show", id, "extra"],
            vec!["actions", "list", "extra"],
            vec!["actions", "list", "--state", "unknown"],
            vec!["actions", "list", "--limit", "0"],
            vec!["actions", "list", "--limit", "201"],
            vec!["actions", "list", "--limit", "-1"],
            vec!["actions", "list", "--limit", "1", "--limit", "2"],
            vec!["actions", "list", "--before-created-at-ms", "1"],
            vec!["actions", "list", "--before-id", id],
            vec![
                "actions",
                "list",
                "--before-created-at-ms",
                "-1",
                "--before-id",
                id,
            ],
            vec![
                "actions",
                "list",
                "--before-created-at-ms",
                "9223372036854775808",
                "--before-id",
                id,
            ],
            vec![
                "actions",
                "list",
                "--before-created-at-ms",
                "1",
                "--before-id",
                &nil,
            ],
        ] {
            let mut owned = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            owned.extend([
                "--data-dir".into(),
                dir.path().to_string_lossy().into_owned(),
            ]);
            let failure = match super::super::parse(&owned) {
                Err(failure) => failure,
                Ok(_) => panic!("malformed input was accepted: {owned:?}"),
            };
            assert_eq!(failure.error.code(), "USAGE", "{owned:?}");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn constructed_invalid_actions_refuse_before_workspace_and_credentials() {
        let owner = tempfile::tempdir().unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("brn.sqlite3"), b"synthetic legacy marker").unwrap();
        let credentials = owner.path().join("credentials");
        let commands = [
            ActionsCommand::Show(Uuid::nil()),
            ActionsCommand::List(ActionListRequest {
                state: None,
                limit: 0,
                before: None,
            }),
            ActionsCommand::List(ActionListRequest {
                state: None,
                limit: 1,
                before: Some(ActionCursor {
                    created_at_ms: u64::MAX,
                    id: Uuid::new_v4(),
                }),
            }),
            ActionsCommand::List(ActionListRequest {
                state: None,
                limit: 1,
                before: Some(ActionCursor {
                    created_at_ms: 0,
                    id: Uuid::nil(),
                }),
            }),
        ];
        for command in commands {
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::Actions(command),
            };
            let failure = super::super::execute(&invocation)
                .err()
                .expect("invalid input refuses");
            assert_eq!(failure.error.code(), "USAGE");
            assert_eq!(
                std::fs::read(data.join("brn.sqlite3")).unwrap(),
                b"synthetic legacy marker"
            );
            assert!(!credentials.exists());
        }
    }
    #[test]
    fn dashboard_parser_defaults_active_and_prepares_all_explicit_filters_without_authority() {
        let dir = tempfile::tempdir().unwrap();
        let base = [
            "actions",
            "dashboard",
            "--data-dir",
            dir.path().to_str().unwrap(),
        ];
        let Outcome::Run(invocation) =
            parse(&base).unwrap_or_else(|failure| panic!("{}", failure.error.code()))
        else {
            panic!("run")
        };
        let Command::Actions(ActionsCommand::Dashboard(request)) = invocation.command else {
            panic!("Dashboard")
        };
        assert_eq!(request, DashboardRequest::default());
        for (raw, expected) in [
            ("active", DashboardFilter::Active),
            ("open", DashboardFilter::Open),
            ("waiting", DashboardFilter::Waiting),
            ("blocked", DashboardFilter::Blocked),
            ("completed", DashboardFilter::Completed),
            ("overdue", DashboardFilter::Overdue),
            ("follow-up", DashboardFilter::FollowUp),
            ("all", DashboardFilter::All),
        ] {
            let args = [
                "actions",
                "dashboard",
                "--filter",
                raw,
                "--as-of",
                "2028-02-29",
                "--limit",
                "200",
                "--before-created-at-ms",
                "42",
                "--before-id",
                "00000000-0000-0000-0000-000000000001",
                "--data-dir",
                dir.path().to_str().unwrap(),
            ];
            let Outcome::Run(invocation) =
                parse(&args).unwrap_or_else(|failure| panic!("{}", failure.error.code()))
            else {
                panic!("run")
            };
            let Command::Actions(ActionsCommand::Dashboard(request)) = invocation.command else {
                panic!("Dashboard")
            };
            assert_eq!(request.filter, expected);
            assert_eq!(request.as_of.as_deref(), Some("2028-02-29"));
            assert_eq!(request.limit, 200);
            let AppCommand::ActionDashboard(prepared) =
                prepare(&ActionsCommand::Dashboard(request.clone())).unwrap()
            else {
                panic!("prepared")
            };
            assert_eq!(prepared, request);
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn dashboard_output_binds_prepared_date_filter_cursor_and_checked_reply() {
        use brn_workflow::dashboard::{DashboardCounts, DashboardEntry, DashboardPage};
        let request = DashboardRequest {
            as_of: Some("2028-02-29".into()),
            ..Default::default()
        };
        let page = DashboardPage {
            as_of: "2028-02-29".into(),
            counts: DashboardCounts {
                waiting: 1,
                ..Default::default()
            },
            entries: vec![DashboardEntry {
                action: *completion_request().before,
                overdue: false,
                follow_up: false,
                dependency_blocked: false,
                dependencies: vec![],
            }],
            next_before: None,
        };
        let accepted = output(
            &AppCommand::ActionDashboard(request.clone()),
            AppEvent::ActionDashboard(Box::new(page.clone())),
        )
        .unwrap();
        assert_eq!(accepted.data, serde_json::to_value(&page).unwrap());
        assert!(!accepted.text.contains('\u{001b}'));
        assert!(!accepted.text.contains('\u{0085}'));
        let mut wrong = page.clone();
        wrong.as_of = "2028-03-01".into();
        assert!(output(
            &AppCommand::ActionDashboard(request.clone()),
            AppEvent::ActionDashboard(Box::new(wrong))
        )
        .is_err());
        let mut wrong = page.clone();
        wrong.counts.waiting = 0;
        assert!(output(
            &AppCommand::ActionDashboard(request.clone()),
            AppEvent::ActionDashboard(Box::new(wrong))
        )
        .is_err());
        let other = DashboardRequest {
            filter: DashboardFilter::Open,
            ..request.clone()
        };
        assert!(output(
            &AppCommand::ActionDashboard(other),
            AppEvent::ActionDashboard(Box::new(page.clone()))
        )
        .is_err());
        assert!(output(
            &AppCommand::ActionDashboard(request),
            AppEvent::Action(Box::new(page.entries[0].action.clone()))
        )
        .is_err());
    }

    fn completion_request() -> CompleteActionRequest {
        use brn_store::work::{
            actions::{ActionData, ActionOrigin, ActionRecord},
            proposals::ProposalStamp,
        };
        let data = ActionData {
            title: "Exact 日本語\r\n\u{001b}\u{0085}".into(),
            description: "Õun\t\u{0000}\u{007f}".into(),
            state: ActionState::Waiting,
            owner: None,
            related_person: None,
            related_project: None,
            sources: vec![],
            thread: None,
            due_on: None,
            follow_up_on: None,
            dependencies: vec![],
            parent: None,
            follows_up: None,
            priority: None,
        };
        let request = CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(ActionRecord {
                origin: ActionOrigin {
                    id: Uuid::new_v4(),
                    proposal: ProposalStamp {
                        id: Uuid::new_v4(),
                        version: 1,
                    },
                    data: data.clone(),
                    created_at_ms: 7,
                },
                version: 1,
                data,
                updated_at_ms: 7,
                waiting_since_ms: Some(7),
                completed_at_ms: None,
            }),
        };
        request.validate().unwrap();
        request
    }

    #[test]
    fn complete_prepares_exact_strict_file_offline_and_receipt_quotes_terminal_controls() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("complete.json");
        let request = completion_request();
        std::fs::write(&file, serde_json::to_vec(&request).unwrap()).unwrap();
        let Outcome::Run(invocation) = parse(&[
            "actions",
            "complete",
            "--file",
            file.to_str().unwrap(),
            "--data-dir",
            dir.path().to_str().unwrap(),
        ])
        .unwrap_or_else(|f| panic!("{}", f.error.code())) else {
            panic!("run")
        };
        let Command::Actions(command) = invocation.command else {
            panic!("action command")
        };
        assert_eq!(command.name(), "actions.complete");
        let prepared = prepare(&command).unwrap();
        let AppCommand::CompleteAction(captured) = &prepared else {
            panic!("complete")
        };
        assert_eq!(*captured, request);
        // Changed file bytes after admission cannot alter receipt correlation.
        std::fs::write(&file, b"changed after preparation").unwrap();
        let mut after = *request.before.clone();
        after.version += 1;
        after.data.state = ActionState::Completed;
        after.updated_at_ms = 9;
        after.waiting_since_ms = None;
        after.completed_at_ms = Some(9);
        let receipt = brn_workflow::action_completion::ActionCompletion {
            request: request.clone(),
            after,
        };
        receipt.validate().unwrap();
        let result = output(
            &prepared,
            AppEvent::ActionCompleted(Box::new(receipt.clone())),
        )
        .unwrap();
        assert_eq!(result.data, serde_json::json!(receipt));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&result.text).unwrap(),
            result.data
        );
        assert!(result.text.chars().all(|ch| !ch.is_control() || ch == '\n'));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        let mut foreign = receipt.clone();
        foreign.request.operation_id = Uuid::new_v4();
        assert!(output(&prepared, AppEvent::ActionCompleted(Box::new(foreign))).is_err());
        let mut fork = receipt;
        fork.after.data.title = "changed after".into();
        assert!(output(&prepared, AppEvent::ActionCompleted(Box::new(fork))).is_err());
        assert!(output(&prepared, AppEvent::Action(Box::new(*request.before))).is_err());
    }

    #[test]
    fn complete_invalid_files_refuse_before_worker_workspace_and_credentials() {
        let owner = tempfile::tempdir().unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("preserved.marker"), b"synthetic marker").unwrap();
        let file = owner.path().join("request.json");
        let credentials = owner.path().join("credentials");
        for case in [
            "unknown",
            "nested",
            "nil",
            "completed",
            "partial",
            "directory",
        ] {
            let mut value = serde_json::to_value(completion_request()).unwrap();
            match case {
                "unknown" => value["extra"] = serde_json::json!(true),
                "nested" => value["before"]["extra"] = serde_json::json!(true),
                "nil" => value["operation_id"] = serde_json::json!(Uuid::nil()),
                "completed" => value["before"]["data"]["state"] = serde_json::json!("completed"),
                "partial" => {
                    value = serde_json::json!({"operation_id":Uuid::new_v4(),"before":{"id":Uuid::new_v4(),"version":1}})
                }
                _ => {}
            }
            std::fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::Actions(ActionsCommand::Complete(if case == "directory" {
                    data.clone()
                } else {
                    file.clone()
                })),
            };
            let failure = super::super::execute(&invocation)
                .err()
                .expect("pre-admission refusal");
            assert_eq!(failure.error.code(), "USAGE", "{case}");
            assert_eq!(
                std::fs::read(data.join("preserved.marker")).unwrap(),
                b"synthetic marker"
            );
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 1);
            assert!(!credentials.exists());
        }
    }
}
