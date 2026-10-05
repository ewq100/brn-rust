//! Explicit input preparation and protocol mapping; originals are owned by AppWorker.
use super::{
    error::CliError, expect_positionals, positional_uuid, scan, sub_word, usage, CliFailure,
    Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox::{CaptureInboxRequest, InboxKind, InboxListRequest},
};
use std::{fmt::Write as _, path::PathBuf};
use uuid::Uuid;
pub enum InboxCommand {
    Add {
        id: Uuid,
        kind: InboxKind,
        title: String,
        original_name: Option<String>,
        input: PathBuf,
    },
    Show(Uuid),
    List(InboxListRequest),
}
impl InboxCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Add { .. } => "inbox.add",
            Self::Show(_) => "inbox.show",
            Self::List(_) => "inbox.list",
        }
    }
}
pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "inbox", "add|show|list")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "add" => (
            "inbox.add",
            &[
                ("id", true),
                ("kind", true),
                ("title", true),
                ("original-name", true),
                ("file", true),
            ],
        ),
        "show" => ("inbox.show", &[]),
        "list" => ("inbox.list", &[("limit", true), ("after", true)]),
        _ => return Err(usage("unknown Inbox subcommand (expected add|show|list)")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}
fn metadata(command: &InboxCommand) -> Result<(), CliError> {
    match command {
        InboxCommand::Add {
            id,
            kind,
            title,
            original_name,
            ..
        } => CaptureInboxRequest {
            id: *id,
            kind: *kind,
            title: title.clone(),
            original_name: original_name.clone(),
            text: String::new(),
        }
        .validate()
        .map_err(|e| usage(e.message)),
        InboxCommand::Show(id) if id.is_nil() => Err(usage("Inbox UUID must not be nil")),
        InboxCommand::List(r) => r.validate().map_err(|e| usage(e.to_string())),
        _ => Ok(()),
    }
}
pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<InboxCommand, CliError> {
    let command = match name {
        "inbox.add" => {
            expect_positionals(s, 0)?;
            let kind = match s.value("kind").unwrap_or("text") {
                "text" => InboxKind::Text,
                "markdown" => InboxKind::Markdown,
                "email" => InboxKind::Email,
                "teams" => InboxKind::Teams,
                _ => return Err(usage("--kind must be text|markdown|email|teams")),
            };
            InboxCommand::Add {
                id: s.uuid("id")?.ok_or_else(|| usage("missing --id"))?,
                kind,
                title: s
                    .value("title")
                    .ok_or_else(|| usage("missing --title"))?
                    .into(),
                original_name: s.value("original-name").map(str::to_owned),
                input: PathBuf::from(s.value("file").ok_or_else(|| usage("missing --file"))?),
            }
        }
        "inbox.show" => {
            expect_positionals(s, 1)?;
            InboxCommand::Show(positional_uuid(s, 0, "UUID")?)
        }
        "inbox.list" => {
            expect_positionals(s, 0)?;
            InboxCommand::List(InboxListRequest {
                limit: s
                    .value("limit")
                    .map(|v| {
                        v.parse::<usize>()
                            .map_err(|_| usage("invalid --limit integer"))
                    })
                    .transpose()?
                    .unwrap_or(25),
                after: s.uuid("after")?,
            })
        }
        _ => unreachable!("scanned Inbox command"),
    };
    metadata(&command)?;
    Ok(command)
}
pub(super) fn prepare(command: &InboxCommand) -> Result<AppCommand, CliFailure> {
    metadata(command)?;
    Ok(match command {
        InboxCommand::Add {
            id,
            kind,
            title,
            original_name,
            input,
        } => {
            let request = CaptureInboxRequest {
                id: *id,
                kind: *kind,
                title: title.clone(),
                original_name: original_name.clone(),
                text: super::input::read_text_file(input, "Inbox original")?,
            };
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            AppCommand::CaptureInbox(request)
        }
        InboxCommand::Show(id) => AppCommand::InboxItem(*id),
        InboxCommand::List(r) => AppCommand::InboxItems(r.clone()),
    })
}
pub(super) fn output(command: &AppCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let data = match (command, event) {
        (AppCommand::CaptureInbox(r), AppEvent::InboxCaptured(item))
            if r.validate_receipt(&item).is_ok() =>
        {
            serde_json::json!(*item)
        }
        (AppCommand::InboxItem(id), AppEvent::InboxItem(item)) if item.item.capture.id == *id => {
            serde_json::json!(*item)
        }
        (AppCommand::InboxItems(r), AppEvent::InboxItems(page))
            if page.entries.len() <= r.limit =>
        {
            serde_json::json!(*page)
        }
        _ => {
            return Err(
                CliError::Workflow("unexpected application reply for Inbox command".into()).into(),
            )
        }
    };
    let pretty = serde_json::to_string_pretty(&data).expect("Inbox DTO serializes");
    let mut text = String::with_capacity(pretty.len() + 1);
    for ch in pretty.chars() {
        if ch.is_control() && ch >= '\u{007f}' {
            write!(&mut text, "\\u{:04x}", ch as u32).expect("write to String");
        } else {
            text.push(ch);
        }
    }
    text.push('\n');
    Ok(Output { text, data })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation, Outcome};
    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| (*t).into()).collect()
    }
    #[test]
    fn bounded_parser_rejects_invalid_intake_metadata_and_paging() {
        let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data_path = data.path().to_str().unwrap();
        let id = Uuid::new_v4().to_string();
        for tokens in [
            vec![
                "inbox", "add", "--id", &id, "--title", "Exact õ", "--file", "copy.txt",
            ],
            vec!["inbox", "show", &id],
            vec!["inbox", "list", "--after", &id, "--limit", "100"],
        ] {
            let mut a = args(&tokens);
            a.extend(args(&["--data-dir", data_path]));
            assert!(matches!(crate::cli::parse(&a), Ok(Outcome::Run(_))));
        }
        for tokens in [
            vec![
                "inbox", "add", "--id", &id, "--title", "x", "--kind", "pdf", "--file", "x",
            ],
            vec!["inbox", "add", "--id", &id, "--title", " \t", "--file", "x"],
            vec!["inbox", "list", "--limit", "101"],
            vec![
                "inbox",
                "list",
                "--after",
                "00000000-0000-0000-0000-000000000000",
            ],
            vec!["inbox", "show", "../outside"],
        ] {
            let mut a = args(&tokens);
            a.extend(args(&["--data-dir", data_path]));
            assert!(crate::cli::parse(&a).is_err());
        }
    }
    #[test]
    fn invalid_direct_requests_and_input_files_leave_application_state_unopened() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("credentials");
        let invalid = owner.path().join("invalid.txt");
        std::fs::write(&invalid, [0xff]).unwrap();
        let oversized = owner.path().join("oversized.txt");
        std::fs::write(&oversized, vec![b'x'; brn_workflow::MAX_NOTE_BYTES + 1]).unwrap();
        let make = |input| InboxCommand::Add {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Explicit".into(),
            original_name: None,
            input,
        };
        for command in [
            InboxCommand::Show(Uuid::nil()),
            InboxCommand::List(InboxListRequest {
                limit: 0,
                after: None,
            }),
            make(invalid),
            make(oversized),
            make(owner.path().to_owned()),
            make(owner.path().join("absent")),
        ] {
            let i = Invocation {
                json: true,
                data_dir: data.clone(),
                vault: None,
                credentials_dir: Some(credentials.clone()),
                model_dir: None,
                command: Command::Inbox(command),
            };
            assert!(crate::cli::execute(&i).is_err());
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn actual_cli_owner_copy_show_replay_and_paged_inventory_preserve_exact_input() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("credentials");
        let input = owner.path().join("copy.txt");
        let exact = "\u{feff}quoted õ\r\n\u{009b}sentences 日本語";
        std::fs::write(&input, exact).unwrap();
        let id = Uuid::new_v4();
        let invocation = |command| Invocation {
            json: true,
            data_dir: data.clone(),
            vault: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(command),
        };
        let add = || InboxCommand::Add {
            id,
            kind: InboxKind::Email,
            title: "Email õ".into(),
            original_name: Some("../../label.eml".into()),
            input: input.clone(),
        };
        let first = crate::cli::execute(&invocation(add())).unwrap();
        assert_eq!(
            crate::cli::execute(&invocation(add())).unwrap().data,
            first.data
        );
        let shown = crate::cli::execute(&invocation(InboxCommand::Show(id))).unwrap();
        assert_eq!(shown.data["item"], first.data);
        assert_eq!(shown.data["original"]["text"], exact);
        assert!(!shown.text.contains('\u{009b}'));
        assert!(shown.text.contains("\\u009b"));
        let listed =
            crate::cli::execute(&invocation(InboxCommand::List(InboxListRequest::default())))
                .unwrap();
        assert_eq!(listed.data["total_count"], 1);
        assert_eq!(listed.data["entries"][0]["availability"], "available");
        assert_eq!(listed.data["issues"], serde_json::json!([]));
        assert_eq!(std::fs::read(input).unwrap(), exact.as_bytes());
        assert_eq!(std::fs::read_dir(credentials).unwrap().count(), 0);
        assert!(!data.join("index.sqlite").exists());
    }
}
