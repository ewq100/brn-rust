//! Read-only access to checked operational Actions through the shared workflow.
use super::{
    error::CliError, expect_positionals, positional_uuid, scan, sub_word, usage, CliFailure,
    Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    actions::{ActionCursor, ActionListRequest, ActionRecord, ActionState},
    app_worker::{AppCommand, AppEvent},
};
use std::fmt::Write as _;
use uuid::Uuid;

pub enum ActionsCommand {
    Show(Uuid),
    List(ActionListRequest),
}

impl ActionsCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Show(_) => "actions.show",
            Self::List(_) => "actions.list",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "actions", "show|list")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "show" => ("actions.show", &[]),
        "list" => (
            "actions.list",
            &[
                ("state", true),
                ("limit", true),
                ("before-created-at-ms", true),
                ("before-id", true),
            ],
        ),
        _ => return Err(usage("unknown actions subcommand (expected show|list)")),
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

pub(super) fn validate(command: &ActionsCommand) -> Result<(), CliError> {
    match command {
        ActionsCommand::Show(id) if id.is_nil() => Err(usage("Action UUID must not be nil")),
        ActionsCommand::Show(_) => Ok(()),
        ActionsCommand::List(request) => {
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
        "actions.list" => {
            expect_positionals(scanned, 0)?;
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
                    ));
                }
            };
            ActionsCommand::List(ActionListRequest {
                state: scanned.value("state").map(state).transpose()?.flatten(),
                limit: scanned
                    .value("limit")
                    .map(|raw| {
                        raw.parse::<usize>()
                            .map_err(|_| usage("invalid --limit integer"))
                    })
                    .transpose()?
                    .unwrap_or(25),
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
    })
}

fn record_text(record: &ActionRecord) -> String {
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

pub(super) fn output(command: &ActionsCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let (data, text) = match (command, event) {
        (ActionsCommand::Show(id), AppEvent::Action(record)) if record.origin.id == *id => {
            let text = format!("{}\n", record_text(&record));
            (serde_json::json!(*record), text)
        }
        (ActionsCommand::List(request), AppEvent::Actions(page))
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
}
