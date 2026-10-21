//! Thin host CLI over the same checked Threads service as the desktop.
mod contract;
mod help;
mod input;
mod inspect;
mod spec;
mod stop;
use brn_ai::{AiTerminal, Provider, ReasoningEffort};
use brn_threads_app::*;
use brn_threads_intake::*;
use contract::{Annotations, CliError, Code};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio_util::sync::CancellationToken;
static CANCEL: AtomicBool = AtomicBool::new(false);
extern "C" fn on_sigint(_: libc::c_int) {
    CANCEL.store(true, Ordering::SeqCst);
}
const HELP: &str = "BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND [--option VALUE]\n\nCommands: init [--purpose test], records, notes, threads, needs-you, history, candidates, settings,\n  projects [--trash yes]; project-create --title TITLE [--key KEY]\n  project-chat --project ID --title TITLE; project-document --project ID --title TITLE --text MARKDOWN\n  project-settings --project ID --base VERSION --title TITLE --instructions TEXT --knowledge yes|no\n  chat-move --thread ID [--project ID|none]; project-link|project-unlink --project ID --note ID\n  project-delete-preview --project ID (save JSON for review)\n  project-delete --preview-file FILE --actions JSON [--key KEY]\n  project-restore --project ID --base VERSION\n  Project commands accept --key KEY for durable retry. Actions JSON maps each unfinished Action ID to Keep or Cancel.\n  Search/connections accept --project ID; intake --intent project --project ID imports privately.\n  intake --intent useful reuses the chat that already received the same source (in --project ID when given),\n  unless work there is still running; then it starts a new chat (reused_chat false).\n  skills; skill-inspect --name NAME; skill-install --source ABSOLUTE_FOLDER
  skill-remove --name NAME --digest SHA256 (owner-only local package configuration)
  skills [--project ID]; skill-enable|skill-disable --id ID --digest SHA256 [--project ID]
  skill-select --id ID|none [--project ID]; skill-clear --id ID [--project ID]
  Inventory/inspection is workspace-wide owner configuration; --project selects policy only.
  Scoped policy/selection is canonical and backed up; exact installed bytes remain local. Selected instructions load at run preparation; all others are lazy.
  mcp-status [--thread ID]; mcp-discover --endpoint http://127.0.0.1:PORT/mcp
  mcp-configure --policy JSON|null; mcp-disable (also fences/disconnects active use)
  ask ... --mcp-tool EXACT_APPROVED_NAME --mcp-input JSON (one transient reviewed call)
  Anonymous local HTTP only; remote/authenticated servers unsupported. Enablement is not an external-data grant.
  Installed skills are excluded from current BRN backups/exports; preserve originals for reinstall.
  note-create --title TITLE --text MARKDOWN\n  thread-start --title TITLE\n  read --id RECORD\n  search --query TEXT [--limit 1..50] [--cursor NEXT] [--thread CHAT]\n  connections --note ID [--hops 1|2] [--sources yes] [--thread CHAT]\n  topics; topics-create; topics-choose --note ID\n  knowledge-check [--seed-chat yes]\n  model-install --destination ABSOLUTE_FRESH_FOLDER (native builds; downloads the pinned model)\n  Semantic search uses the model folder named by BRN_EMBEDDING_MODEL_DIR.\n  begin-edit --note ID --base VERSION [--token-out ABSOLUTE_NEW_FILE] (prints the session token once)\n  update-buffer --session ID --generation N --text MARKDOWN --token-file ABSOLUTE_FILE|-\n  save|discard --session ID --generation N --token-file ABSOLUTE_FILE|-; recovery\n  reclaim-edit --session ID [--token-out ABSOLUTE_NEW_FILE] (test workspaces only)\n  apply|dismiss --operation ID; undo --operation ID; archive|unarchive --thread ID\n  queue --thread ID --text TEXT; keep-in-brn --note ID --base VERSION\n  ask --thread ID --prompt TEXT [--budget 1..32] [--edit-note ID --base VERSION | --record-decision yes] [--trace summary|full]\n  intake --source ABSOLUTE_FILE --intent useful|full|chat|project [--title TITLE] [--thread ID] [--project ID] [--trace summary|full]\n  configure --provider Codex|Copilot --model ID --effort low|medium|high\n    [--helper off | --helper-provider Codex|Copilot --helper-model ID --helper-effort low|medium|high\n     [--helper-auth-file FILE|--helper-credentials-dir DIR]] (read-only research helper, off by default.\n     Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower)\n  document-conversion --converter xberg|xberg-brn --ai off|on\n  delegation --choice maintain|review-first\n  pause; resume; export|backup --destination ABSOLUTE_FRESH_PATH\n  restore --source ABSOLUTE_BACKUP\n  upgrade --backup-directory ABSOLUTE_EXTERNAL_FOLDER\n\nChoose delegation before resuming maintenance. Manual work remains available while pending.\nFresh explicit data only. Originals remain external; no external messages are sent.\nReview apply and Undo are concrete owner actions. Continue uses ask on the same Thread.";
/// Printed after `HELP` by `--help`.
const AGENT_HELP: &str = "
Agents and scripts: brn --json --data-dir DIR COMMAND ... prints one JSON envelope on stdout.
  In --json mode: exit 0 ok, 1 internal, 2 usage, 3 stale, 4 deferred, 5 needs review, 6 rejected,
  7 failed (including busy and timeout), 130 interrupted (SIGINT), 143 terminated (SIGTERM).
  Without --json, failures exit 1 and outcomes such as Stale exit 0.
  --timeout SECONDS after --data-dir bounds the call; it never hides a commit (envelope stop).
  --actor NAME (or BRN_ACTOR) names the caller on the receipts this call commits; it is not identity.
  Every envelope reports authority {workspace_purpose, cli_authority, enforced: false, actor}.
  brn --json --help prints the machine contract (commands, options, outcomes, retry classes, error codes);
  brn [--json] COMMAND --help prints one command's help. --data-dir and --timeout may follow the command.
  Bounded inspection: history|candidates|records|notes|threads|needs-you|recovery --limit N [--cursor C],
  projects [--trash yes] --limit N, messages --thread ID, revisions --note ID,
  revision --note ID --version V, runs [--thread ID] [--state STATE], read --id ID --limit BYTES;
  pass result.cursor back as --cursor for the next page. operation --id OP | --key K --command NAME
  shows whether an operation (or a keyed command's key) applied, without using the key.
  Text options --title, --text, --prompt, --instructions, --actions also accept --NAME-file ABSOLUTE_FILE or --NAME-file - (stdin).
  --key KEY makes note-create, thread-start, undo, queue, archive|resolve, unarchive, keep-in-brn,
  intake --intent full|chat|project, topics-create, knowledge-check --seed-chat yes and project commands
  replay on retry. resolve is an alias of archive. A keyed full/chat/project intake replays when bytes,
  path, scope, title and captured converter/AI/bundle match a committed import, without converting or sending the document again;
  if that import found the bytes already imported, the retry converts again and records only an observation.
  Edit writes need the session token: begin-edit --token-out FILE, then --token-file FILE (never argv).
  Stale save (exit 3): keep result.session.markdown; discard your session (--token-file OLD); begin-edit --base
  <conflict[0].version> --token-out NEW; reapply; update-buffer --generation 1 --text-file merged.md --token-file NEW;
  save --generation 1 --token-file NEW.
  upgrade_required (exit 6): the workspace has an older schema and nothing changed; the CLI never upgrades
  implicitly. On the owner's instruction: brn --data-dir DIR upgrade --backup-directory ABSOLUTE_DIR (backs up first).
  intake --intent chat --thread <PROJECT_ID> attaches the document to that project: an owner decision.
  Commands marked owner_decision record an owner decision. Running the CLI does not make you the owner:
  run them only on the owner's explicit instruction for that exact operation.";
type Result<T> = std::result::Result<T, CliError>;
struct Invocation {
    data: PathBuf,
    command: String,
    args: BTreeMap<String, String>,
    /// A leading `--json` selects the machine contract.
    machine: bool,
    /// The caller's `--key`, already checked against the key pattern.
    key: Option<String>,
    /// `(NAME, SOURCE)` for each `--NAME-file`, read by `input::resolve`.
    pending_inputs: Vec<(&'static str, String)>,
    /// Global `--timeout SECONDS`, already checked to be 1..86400.
    timeout: Option<u64>,
    /// `--token-file ABSOLUTE_PATH|-`, read by `input::resolve` into `token`.
    token_source: Option<String>,
    /// The edit session token; never stored in `args`.
    token: Option<EditToken>,
    /// Global `--actor NAME` (else `BRN_ACTOR`), already checked.
    actor: Option<String>,
}
/// A refusal found while parsing argv, before anything is opened or read.
struct Refused {
    machine: bool,
    command: Option<String>,
    key: Option<String>,
    error: CliError,
}
/// Keeps the saved helper unless the owner turns it off or names a complete new
/// selection; a helper on the main provider reuses the main credentials unless
/// the owner names other ones.
fn helper_settings(
    inv: &Invocation,
    settings: &WorkspaceSettings,
) -> Result<Option<brn_threads_app::HelperSettings>> {
    let named = [
        "helper-provider",
        "helper-model",
        "helper-effort",
        "helper-auth-file",
        "helper-credentials-dir",
    ]
    .iter()
    .any(|key| inv.args.contains_key(*key));
    match inv.args.get("helper").map(String::as_str) {
        Some("off") if !named => return Ok(None),
        Some(_) => {
            return Err(CliError::conflicting(
                &["helper", "helper-provider", "helper-model", "helper-effort"],
                "Use --helper off alone, or name the helper options",
            ));
        }
        None if !named => return Ok(settings.helper.clone()),
        None => {}
    }
    let chosen = provider(inv.arg("helper-provider")?)?;
    let name = match chosen {
        Provider::Chatgpt => "Codex",
        Provider::Copilot => "Copilot",
    };
    let (mut auth_file, mut credentials_dir) = (
        inv.args.get("helper-auth-file").cloned(),
        inv.args.get("helper-credentials-dir").cloned(),
    );
    if auth_file.is_none() && credentials_dir.is_none() && settings.provider == name {
        auth_file = settings.auth_file.clone();
        credentials_dir = settings.credentials_dir.clone();
    }
    Ok(Some(brn_threads_app::HelperSettings {
        provider: name.into(),
        model: inv.arg("helper-model")?.into(),
        effort: effort(inv.arg("helper-effort")?)?.as_str().into(),
        auth_file,
        credentials_dir,
    }))
}
/// Reads argv against the command table. Every grammar, requirement and
/// key check runs here, before the workspace opens or any input is read.
fn parse(
    args: impl IntoIterator<Item = OsString>,
) -> std::result::Result<Option<Invocation>, Refused> {
    let args: Vec<OsString> = args.into_iter().collect();
    if args.len() == 1 && args[0] == "--help" {
        contract::emit_line(&format!("{HELP}\n{AGENT_HELP}"));
        return Ok(None);
    }
    if args.len() == 2 && args[0] == "--json" && args[1] == "--help" {
        contract::emit_line(&spec::description().to_string());
        return Ok(None);
    }
    let machine = args.first().is_some_and(|arg| arg == "--json");
    let mut tokens = Vec::with_capacity(args.len());
    for (index, arg) in args.into_iter().enumerate() {
        match arg.into_string() {
            Ok(token) => tokens.push(token),
            Err(_) => {
                return Err(Refused {
                    machine,
                    command: None,
                    key: None,
                    error: CliError::new(Code::MalformedOption, "Arguments must be valid UTF-8")
                        .with("position", index + 1),
                });
            }
        }
    }
    let refuse = |command: Option<&str>, error: CliError| Refused {
        machine,
        command: command.map(str::to_owned),
        key: None,
        error,
    };
    let start = usize::from(machine);
    let mut iter = tokens.into_iter().enumerate().skip(start);
    let usage = |code, message: &str| CliError::new(code, message);
    let supply = "Supply --data-dir ABSOLUTE_DIRECTORY; run --help";
    // Global options: --data-dir, --timeout and --actor in any option-name position.
    let mut data: Option<PathBuf> = None;
    let mut timeout = None;
    let mut actor: Option<String> = None;
    let set_actor =
        |actor: &mut Option<String>, value: Option<String>| -> std::result::Result<(), CliError> {
            if actor.is_some() {
                return Err(usage(Code::RepeatedOption, "Repeated option").with_option("actor"));
            }
            let value = value.ok_or_else(|| {
                usage(Code::MissingValue, "Missing option value").with_option("actor")
            })?;
            if !actor_is_valid(&value) {
                return Err(CliError::invalid_value("actor", ACTOR_EXPECTED));
            }
            *actor = Some(value);
            Ok(())
        };
    let set_data = |data: &mut Option<PathBuf>,
                    value: Option<String>|
     -> std::result::Result<(), CliError> {
        if data.is_some() {
            return Err(usage(Code::RepeatedOption, "Repeated option").with_option("data-dir"));
        }
        let value = value.ok_or_else(|| usage(Code::MissingDataDir, "Missing data directory"))?;
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(usage(
                Code::InvalidDataDir,
                "Data directory must be absolute",
            ));
        }
        *data = Some(path);
        Ok(())
    };
    let set_timeout =
        |timeout: &mut Option<u64>, value: Option<String>| -> std::result::Result<(), CliError> {
            if timeout.is_some() {
                return Err(usage(Code::RepeatedOption, "Repeated option").with_option("timeout"));
            }
            let value = value.ok_or_else(|| {
                usage(Code::MissingValue, "Missing option value").with_option("timeout")
            })?;
            *timeout = Some(timeout_seconds(&value)?);
            Ok(())
        };
    let command = loop {
        match iter.next() {
            Some((_, token)) if token == "--data-dir" => {
                let value = iter.next().map(|(_, v)| v);
                set_data(&mut data, value).map_err(|e| refuse(None, e))?;
            }
            Some((_, token)) if token == "--timeout" => {
                let value = iter.next().map(|(_, v)| v);
                set_timeout(&mut timeout, value).map_err(|e| refuse(None, e))?;
            }
            Some((_, token)) if token == "--actor" => {
                let value = iter.next().map(|(_, v)| v);
                set_actor(&mut actor, value).map_err(|e| refuse(None, e))?;
            }
            // Before the command, any other option needs --data-dir first.
            Some((_, token)) if token.starts_with("--") && data.is_none() => {
                return Err(refuse(None, usage(Code::MissingDataDir, supply)));
            }
            Some((_, token)) => break Some(token),
            None => break None,
        }
    };
    let Some(command) = command else {
        let error = match data {
            None => usage(Code::MissingDataDir, supply),
            Some(_) => usage(Code::MissingCommand, "Missing command"),
        };
        return Err(refuse(None, error));
    };
    let rest: Vec<(usize, String)> = iter.collect();
    let Some(spec) = spec::spec(&command) else {
        // An unknown command without any --data-dir is still the missing data dir.
        let named = rest.iter().any(|(_, t)| t == "--data-dir" || t == "--help");
        if data.is_none() && !named {
            return Err(refuse(None, usage(Code::MissingDataDir, supply)));
        }
        let mut error = usage(Code::UnknownCommand, "Unknown command; run --help");
        if let Some(command) = contract::echo(&command) {
            error = error.with("command", command);
        }
        return Err(refuse(None, error));
    };
    let refuse = |error: CliError| refuse(Some(spec.name), error);
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let mut help = false;
    let mut iter = rest.into_iter();
    while let Some((index, token)) = iter.next() {
        let Some(name) = token.strip_prefix("--") else {
            let error = usage(Code::MalformedOption, "Options need --name VALUE");
            return Err(refuse(error.with("position", index + 1)));
        };
        // `--help` in an option-name position takes no value.
        if name == "help" {
            help = true;
            continue;
        }
        let value = iter.next().map(|(_, value)| value);
        match name {
            "data-dir" => set_data(&mut data, value).map_err(refuse)?,
            "timeout" => set_timeout(&mut timeout, value).map_err(refuse)?,
            "actor" => set_actor(&mut actor, value).map_err(refuse)?,
            _ => {
                let Some(value) = value else {
                    let error = usage(Code::MissingValue, "Missing option value");
                    return Err(refuse(error.with_option(name)));
                };
                if options.insert(name.into(), value).is_some() {
                    let error = usage(Code::RepeatedOption, "Repeated option");
                    return Err(refuse(error.with_option(name)));
                }
            }
        }
    }
    if help {
        if machine {
            contract::emit_line(&help::json(spec).to_string());
        } else {
            contract::emit_line(&help::text(spec));
        }
        return Ok(None);
    }
    let Some(data) = data else {
        // As before round 2: no --data-dir at all reports no command.
        return Err(Refused {
            machine,
            command: None,
            key: None,
            error: usage(Code::MissingDataDir, supply),
        });
    };
    if let Some(unknown) = options
        .keys()
        .find(|key| !spec.options.contains(&key.as_str()))
    {
        let error = usage(Code::UnknownOption, "Unknown option for command")
            .with_option(unknown)
            .with("allowed", spec.options);
        return Err(refuse(error));
    }
    if actor.is_none() {
        if let Some(value) = std::env::var_os("BRN_ACTOR") {
            match value.into_string() {
                Ok(value) if actor_is_valid(&value) => actor = Some(value),
                _ => {
                    let error =
                        CliError::invalid_value("actor", ACTOR_EXPECTED).with("source", "env");
                    return Err(refuse(error));
                }
            }
        }
    }
    if let Some(actor) = &actor {
        contract::set_actor(actor);
    }
    let key = options
        .get("key")
        .filter(|key| spec::key_is_valid(key))
        .cloned();
    let refuse = |error: CliError| Refused {
        key: key.clone(),
        ..refuse(error)
    };
    for name in spec::TEXT_OPTIONS {
        let file = format!("{name}-file");
        if options.contains_key(name) && options.contains_key(&file) {
            let message = format!("Choose either --{name} or --{file}");
            return Err(refuse(CliError::conflicting(&[name, &file], message)));
        }
    }
    if let Some(variant) = spec::key_refused(spec, &options) {
        let message = format!(
            "--key is not accepted with --{} {}",
            variant.option, variant.equals
        );
        return Err(refuse(CliError::conflicting(
            &["key", variant.option],
            message,
        )));
    }
    if let Some((option, equals)) = spec.key_requires {
        if options.contains_key("key") && options.get(option).map(String::as_str) != Some(equals) {
            let message = format!("--key needs --{option} {equals}");
            return Err(refuse(CliError::conflicting(&["key", option], message)));
        }
    }
    if options.contains_key("key") && key.is_none() {
        let message = "--key must match [A-Za-z0-9._:-]{1,128}";
        return Err(refuse(CliError::invalid_value("key", message)));
    }
    let mut pending_inputs = Vec::new();
    let mut stdin: Option<String> = None;
    for name in spec::TEXT_OPTIONS {
        let file = format!("{name}-file");
        let Some(source) = options.remove(&file) else {
            continue;
        };
        if source != "-" && !Path::new(&source).is_absolute() {
            let message = format!("--{file} takes an absolute path or -");
            return Err(refuse(
                CliError::invalid_value(&file, message).with("expected", "absolute path or -"),
            ));
        }
        if source == "-" {
            if let Some(first) = &stdin {
                let message = format!("--{file} -: stdin is already read by --{first}");
                return Err(refuse(CliError::invalid_input(
                    &file,
                    "stdin_reused",
                    message,
                )));
            }
            stdin = Some(file.clone());
        }
        pending_inputs.push((name, source));
    }
    // The edit token is secret input: a 0600 file or a pipe, never argv.
    let token_source = options.remove("token-file");
    if let Some(source) = &token_source {
        if source != "-" && !Path::new(source).is_absolute() {
            let message = "--token-file takes an absolute path or -";
            return Err(refuse(
                CliError::invalid_value("token-file", message)
                    .with("expected", "absolute path or -"),
            ));
        }
        if source == "-" {
            if let Some(first) = &stdin {
                let message = format!("--token-file -: stdin is already read by --{first}");
                return Err(refuse(CliError::invalid_input(
                    "token-file",
                    "stdin_reused",
                    message,
                )));
            }
        }
    }
    let given = |name: &str| {
        options.contains_key(name)
            || pending_inputs.iter().any(|(pending, _)| *pending == name)
            || (name == "token-file" && token_source.is_some())
    };
    let conditional = spec
        .required_when
        .iter()
        .filter(|rule| {
            options
                .get(rule.option)
                .is_some_and(|value| value == rule.equals)
        })
        .flat_map(|rule| rule.required);
    if let Some(missing) = spec
        .required
        .iter()
        .chain(conditional)
        .find(|name| !given(name))
    {
        return Err(refuse(CliError::missing_option(missing)));
    }
    if let Some((_, needs)) = spec
        .needs
        .iter()
        .find(|(option, needs)| options.contains_key(*option) && !given(needs))
    {
        return Err(refuse(CliError::missing_option(needs)));
    }
    spec::check_values(spec.name, &options).map_err(refuse)?;
    if spec.name == "operation" {
        inspect::operation_form(&options).map_err(refuse)?;
    }
    if matches!(spec.name, "ask" | "intake") {
        if let Some(trace) = options.get("trace") {
            if !matches!(trace.as_str(), "summary" | "full") {
                let message = "--trace takes summary or full";
                return Err(refuse(CliError::invalid_value("trace", message)));
            }
        }
    }
    if spec.name == "ask" {
        match (options.get("mcp-tool"), options.get("mcp-input")) {
            (None, None) => {}
            (Some(tool), Some(input)) => {
                if tool.is_empty()
                    || tool.len() > 64
                    || !tool
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                {
                    return Err(refuse(CliError::invalid_value(
                        "mcp-tool",
                        "Use the exact owner-approved remote tool name",
                    )));
                }
                if input.len() > brn_ai::mcp::MAX_ARGUMENT_BYTES
                    || !serde_json::from_str::<Value>(input).is_ok_and(|v| v.is_object())
                {
                    return Err(refuse(CliError::invalid_value(
                        "mcp-input",
                        "MCP input must be one bounded JSON object, at most 8192 UTF-8 bytes",
                    )));
                }
            }
            _ => {
                return Err(refuse(CliError::missing_option(
                    if options.contains_key("mcp-tool") {
                        "mcp-input"
                    } else {
                        "mcp-tool"
                    },
                )));
            }
        }
    }
    if spec.name == "mcp-discover" {
        brn_ai::mcp::Endpoint::parse(&options["endpoint"]).map_err(|_|refuse(CliError::invalid_value("endpoint", "Use anonymous literal-loopback HTTP with an explicit nonzero port and path; no credentials, query, fragment, DNS or remote endpoint")))?;
    }
    if spec.name == "mcp-configure"
        && (options["policy"].len() > 32 * 1024
            || serde_json::from_str::<Option<McpPolicy>>(&options["policy"]).is_err())
    {
        return Err(refuse(CliError::invalid_value(
            "policy",
            "Policy must be bounded MCP policy JSON or null to clear; no inputs or secrets",
        )));
    }
    if command == "delegation" {
        delegation_choice(&options["choice"]).map_err(refuse)?;
    }
    if let Some(out) = options.get("token-out") {
        token_out_free(Path::new(out)).map_err(refuse)?;
    }
    Ok(Some(Invocation {
        data,
        command,
        args: options,
        machine,
        key,
        pending_inputs,
        timeout,
        token_source,
        token: None,
        actor,
    }))
}
const ACTOR_EXPECTED: &str = "--actor takes a name matching [A-Za-z0-9._:-]{1,64}";
/// `^[A-Za-z0-9._:-]{1,64}$`, the shared service's actor rule.
fn actor_is_valid(actor: &str) -> bool {
    (1..=64).contains(&actor.len())
        && actor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}
/// `--token-out`: a new file whose parent folder exists, checked before opening.
fn token_out_free(path: &Path) -> Result<()> {
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(CliError::new(
            Code::DestinationExists,
            "--token-out already exists; choose a new path",
        )
        .with("option", "token-out"));
    }
    if path.parent().is_some_and(|parent| !parent.is_dir()) {
        return Err(CliError::new(
            Code::InvalidRequest,
            "--token-out needs an existing parent folder",
        )
        .with("option", "token-out")
        .with("reason", "parent_missing"));
    }
    Ok(())
}
/// `--timeout SECONDS`: an integer 1..86400.
fn timeout_seconds(value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|seconds| {
            (1..=86400).contains(seconds) && value.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(|| {
            CliError::invalid_value("timeout", "--timeout takes an integer 1..86400 (seconds)")
        })
}
impl Invocation {
    fn arg(&self, key: &str) -> Result<&str> {
        self.args
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| CliError::missing_option(key))
    }
    fn number(&self, key: &str) -> Result<u64> {
        self.arg(key)?
            .parse()
            .map_err(|_| CliError::invalid_value(key, format!("--{key} must be an integer")))
    }
    /// The host request key: `cli-key:{command}:{KEY}` under `--key`, else
    /// today's random key with `prefix`.
    fn host_key(&self, prefix: &str) -> String {
        match &self.key {
            Some(key) => spec::cli_key(&self.command, key),
            None => format!("{prefix}:{}", uuid::Uuid::new_v4()),
        }
    }
    /// The edit token read from `--token-file`.
    fn token(&self) -> Result<&EditToken> {
        self.token
            .as_ref()
            .ok_or_else(|| CliError::missing_option("token-file"))
    }
    /// The caller's key for the shared keyed functions, or a fresh one.
    fn owner_key(&self) -> String {
        match &self.key {
            Some(_) => self.host_key("cli"),
            None => uuid::Uuid::new_v4().to_string(),
        }
    }
}
/// The `--token-out` file, created empty and owner-only before the service
/// call: `create_new` refuses an existing path or symlink.
fn token_file(inv: &Invocation) -> Result<Option<(PathBuf, std::fs::File)>> {
    let Some(path) = inv.args.get("token-out").map(PathBuf::from) else {
        return Ok(None);
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(&path) {
        Ok(file) => Ok(Some((path, file))),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(CliError::new(
            Code::DestinationExists,
            "--token-out already exists; choose a new path",
        )
        .with("option", "token-out")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(CliError::new(
            Code::InvalidRequest,
            "--token-out needs an existing parent folder",
        )
        .with("option", "token-out")
        .with("reason", "parent_missing")),
        Err(error) => Err(error.into()),
    }
}
fn discard_token_file(out: &Option<(PathBuf, std::fs::File)>) {
    if let Some((path, _)) = out {
        let _ = std::fs::remove_file(path);
    }
}
/// `begin-edit` and `reclaim-edit`: the session with its holder, and the
/// token once, in `result.token` or in the `--token-out` file. If the file
/// cannot be written, `begin-edit` closes its new session (`closed`).
fn issued_output(
    workspace: &mut Workspace,
    issued: IssuedEdit,
    out: Option<(PathBuf, std::fs::File)>,
    close_on_failure: bool,
) -> Result<Value> {
    use std::io::Write;
    let IssuedEdit { session, token } = issued;
    let mut output = encode(&session)?;
    let Some((path, mut file)) = out else {
        output["token"] = json!(token.expose());
        output["token_file"] = Value::Null;
        return Ok(output);
    };
    let written = file
        .write_all(format!("{}\n", token.expose()).as_bytes())
        .and_then(|()| file.sync_all());
    if let Err(error) = written {
        let _ = std::fs::remove_file(&path);
        let closed = close_on_failure
            && matches!(
                workspace
                    .store
                    .discard_caller_edit(&session.id, &token, session.generation),
                Ok(BufferOutcome::Updated(_))
            );
        let message = format!("Could not write --token-out: {error}");
        return Err(CliError::new(Code::IoError, message).with("closed", closed));
    }
    output["token"] = Value::Null;
    output["token_file"] = json!(path);
    Ok(output)
}
/// True when `data` does not exist or is an empty folder (no workspace
/// marker). A symlinked folder or ancestor is never missing: opening it
/// refuses it as `data_folder_unusable`, before anything is created.
fn workspace_missing(data: &Path) -> bool {
    if data
        .ancestors()
        .any(|path| std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()))
    {
        return false;
    }
    match std::fs::read_dir(data) {
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
        Ok(mut entries) => entries.next().is_none(),
    }
}
/// The explicit owner command that downloads the pinned embedding model.
#[cfg(feature = "native-retrieval")]
fn install_model(inv: &Invocation) -> Result<Value> {
    let target = PathBuf::from(inv.arg("destination")?);
    let bytes = install_embedding_model(&target, &CANCEL, |done, total| {
        contract::emit_err(&format!("\rmodel {done}/{total} bytes"));
    })?;
    contract::emit_err("\n");
    Ok(json!({"installed":target,"downloaded_bytes":bytes,"use_with":EMBEDDING_MODEL_ENV}))
}
#[cfg(not(feature = "native-retrieval"))]
fn install_model(_: &Invocation) -> Result<Value> {
    let message = "This build has no native retrieval; build with --features native-retrieval";
    Err(CliError::new(Code::Unavailable, message))
}
/// `--trace summary|full` records the run's tool use (structure only; `full`
/// adds query text and is for synthetic evaluation data).
fn trace_level(inv: &Invocation) -> Result<brn_ai::threads::TraceLevel> {
    match inv.args.get("trace").map(String::as_str) {
        None => Ok(brn_ai::threads::TraceLevel::Off),
        Some("summary") => Ok(brn_ai::threads::TraceLevel::Summary),
        Some("full") => Ok(brn_ai::threads::TraceLevel::Full),
        Some(_) => Err(CliError::invalid_value(
            "trace",
            "--trace takes summary or full",
        )),
    }
}
/// The owner's chosen chat context (`--thread`), which must name a chat.
fn chat_option(workspace: &Workspace, inv: &Invocation) -> Result<Option<String>> {
    if let Some(project) = inv.args.get("project") {
        if inv.args.contains_key("thread") {
            let message = "Choose either --project or --thread";
            return Err(CliError::conflicting(&["project", "thread"], message));
        }
        if !workspace
            .store
            .record(project)?
            .is_some_and(|r| !r.archived && matches!(r.data, RecordData::Project(_)))
        {
            let message = "--project must name an active project";
            return Err(CliError::invalid_request(message).with("option", "project"));
        }
        return Ok(Some(project.clone()));
    }
    let Some(chat) = inv.args.get("thread") else {
        return Ok(None);
    };
    let is_chat = workspace
        .store
        .record(chat)?
        .is_some_and(|r| matches!(r.data, RecordData::Thread(_)));
    if !is_chat {
        let message = "--thread must name a chat";
        return Err(CliError::invalid_request(message).with("option", "thread"));
    }
    Ok(Some(chat.clone()))
}
fn encode(value: impl serde::Serialize) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}
fn settings_output(settings: WorkspaceSettings) -> Result<Value> {
    let chosen = settings.delegation_chosen;
    let documents = settings.document_conversion.clone();
    let mut output = encode(settings)?;
    // Storage omits false for old request identity; CLI status is explicit.
    output["delegation_chosen"] = json!(chosen);
    output["document_conversion"] = json!(documents);
    Ok(output)
}
fn delegation_choice(value: &str) -> Result<DelegationChoice> {
    match value {
        "maintain" => Ok(DelegationChoice::MaintainOrdinaryNotes),
        "review-first" => Ok(DelegationChoice::ReviewFirst),
        _ => Err(CliError::invalid_value(
            "choice",
            "Delegation choice must be maintain or review-first",
        )),
    }
}
/// A converging command's receipt, or `{outcome: "Unchanged", record, version}`
/// when the requested state already held and nothing was recorded.
fn converged(outcome: Converged<Receipt>) -> Result<Value> {
    match outcome {
        Converged::Changed(receipt) => encode(receipt),
        Converged::Unchanged(unchanged) => {
            Ok(json!({"outcome":"Unchanged","record":unchanged.record,"version":unchanged.version}))
        }
    }
}
fn applied(outcome: ApplyOutcome) -> Value {
    match outcome {
        ApplyOutcome::Applied(receipt) => json!({"outcome":"Applied","receipt":receipt}),
        ApplyOutcome::Deferred { guarded } => json!({"outcome":"Deferred","guarded":guarded}),
        ApplyOutcome::Stale { records } => json!({"outcome":"Stale","records":records}),
        ApplyOutcome::NeedsReview { denied } => json!({"outcome":"NeedsReview","denied":denied}),
        ApplyOutcome::Superseded { run } => json!({"outcome":"Superseded","run":run}),
    }
}
fn effort(text: &str) -> Result<ReasoningEffort> {
    match text {
        "low" => Ok(ReasoningEffort::Low),
        "medium" => Ok(ReasoningEffort::Medium),
        "high" => Ok(ReasoningEffort::High),
        _ => Err(CliError::invalid_value(
            "effort",
            "Effort must be low, medium or high",
        )),
    }
}
fn provider(text: &str) -> Result<Provider> {
    match text.to_lowercase().as_str() {
        "codex" | "chatgpt" => Ok(Provider::Chatgpt),
        "copilot" => Ok(Provider::Copilot),
        _ => Err(CliError::invalid_value(
            "provider",
            "Provider must be Codex or Copilot",
        )),
    }
}
fn preparation_cli_error(error: RunPreparationError) -> CliError {
    match error {
        RunPreparationError::Preparation(error)
        | RunPreparationError::Selection(RunSelectionError::Model(error)) => {
            contract::run_error(error)
        }
        RunPreparationError::Selection(RunSelectionError::Provider) => {
            CliError::invalid_value("provider", "Provider must be Codex or Copilot")
        }
        RunPreparationError::Selection(RunSelectionError::Effort) => {
            CliError::invalid_value("effort", "Effort must be low, medium or high")
        }
        other => CliError::from(AppError::from(other)),
    }
}
async fn ask(
    workspace: &mut Workspace,
    inv: &Invocation,
    thread: String,
    prompt: String,
    sources: Vec<brn_ai::threads::TransientSource>,
) -> Result<Value> {
    let settings = workspace.settings()?;
    // Shared with useful intake, whose arm checks first; this also guards it.
    contract::require_credentials(inv, &settings)?;
    let budget = inv
        .args
        .get("budget")
        .map(|n| n.parse::<u16>())
        .transpose()
        .map_err(|_| CliError::invalid_value("budget", "Invalid budget"))?
        .unwrap_or(brn_ai::threads::default_tool_rounds(!sources.is_empty()));
    stop::check()?;
    let cancel = CancellationToken::new();
    let prepared = workspace
        .prepare_thread_run(brn_threads_app::ThreadRunStart {
            request_message: None,
            queue_claim: None,
            owner_request: if inv.command == "ask" {
                match (inv.args.get("edit-note"), inv.args.get("record-decision")) {
                    (Some(id), None) => Some(brn_ai::threads::OwnerRequest::EditNote {
                        id: id.clone(),
                        version: inv.number("base")?,
                    }),
                    (None, Some(value)) if value == "yes" && !inv.args.contains_key("base") => {
                        Some(brn_ai::threads::OwnerRequest::RecordDecision)
                    }
                    (None, None) if !inv.args.contains_key("base") => None,
                    _ => {
                        return Err(CliError::conflicting(
                            &["edit-note", "record-decision"],
                            "Choose either --edit-note ID --base VERSION or --record-decision yes",
                        ));
                    }
                }
            } else {
                None
            },
            thread,
            prompt,
            sources,
            max_tool_rounds: Some(budget),
            trace: trace_level(inv)?,
            offered_copilot_models: None,
        })
        .map_err(preparation_cli_error)?;
    let prepared = if let Some(tool) = inv.args.get("mcp-tool") {
        let input: Value = serde_json::from_str(inv.arg("mcp-input")?)
            .map_err(|_| CliError::invalid_value("mcp-input", "MCP input must be a JSON object"))?;
        prepared
            .with_external_input(tool, &input)
            .map_err(contract::run_error)?
    } else {
        prepared
    };
    let search_status = prepared.search_status;
    let observed = cancel.clone();
    let watcher = tokio::spawn(async move {
        loop {
            if CANCEL.load(Ordering::SeqCst) {
                observed.cancel();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
    let result = brn_ai::threads::run_prepared(prepared.prepared, cancel, Arc::new(|_| {})).await;
    watcher.abort();
    let result = result.map_err(contract::run_error)?;
    let failure = match &result.answer.terminal {
        AiTerminal::Failed(error) => Some(encode(error)?),
        _ => None,
    };
    let terminal = match result.answer.terminal {
        AiTerminal::Completed => "Completed".into(),
        AiTerminal::Interrupted => "Interrupted".into(),
        AiTerminal::Failed(error) => format!("Failed:{:?}", error.kind),
    };
    let mut output = json!({"run":result.run_id,"terminal":terminal,"answer":result.answer.text,"search":search_status,"notices":result.notices});
    if let Some(failure) = failure {
        output["failure"] = failure;
    }
    if !result.provider_resends.is_empty() {
        output["provider_resends"] = encode(&result.provider_resends)?;
    }
    if let Some(trace) = result.trace {
        output["trace"] = encode(trace)?;
    }
    Ok(output)
}
/// The created Record (today's output), with the receipt kept for the envelope.
fn created(notes: &mut Annotations, receipt: Receipt) -> Result<Value> {
    notes.operation = Some(receipt.operation.as_str().into());
    notes.changed = contract::changed(&encode(&receipt.writes)?);
    let record = receipt.writes.into_iter().next().map(|w| w.after);
    encode(record.ok_or_else(|| AppError::Invalid("Empty receipt".into()))?)
}
/// `review_apply` and `review_dismiss` return `AppError::Invalid` only after
/// the decision committed when their attention cleanup fails: retry the same command.
fn review_error(workspace: &Workspace, inv: &Invocation, done: &str, error: AppError) -> CliError {
    let AppError::Invalid(message) = error else {
        return error.into();
    };
    let committed = done == "dismissed"
        || inv.args.get("operation").is_some_and(|id| {
            workspace
                .store
                .operation(id)
                .and_then(|op| workspace.store.receipt(&op))
                .is_ok_and(|receipt| receipt.is_some())
        });
    if committed {
        CliError::new(Code::Internal, message).with(done, true)
    } else {
        CliError::invalid_request(message)
    }
}
async fn execute(inv: &Invocation, notes: &mut Annotations) -> Result<Value> {
    execute_document_transport(
        inv,
        notes,
        #[cfg(all(test, feature = "scripted-document-tests"))]
        None,
    )
    .await
}
async fn execute_document_transport(
    inv: &Invocation,
    notes: &mut Annotations,
    #[cfg(all(test, feature = "scripted-document-tests"))] document_transport: Option<
        brn_ai::document_refine::DocumentHttpClient,
    >,
) -> Result<Value> {
    if CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::interrupted());
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    if matches!(
        inv.command.as_str(),
        "skills"
            | "skill-inspect"
            | "skill-install"
            | "skill-remove"
            | "skill-enable"
            | "skill-disable"
            | "skill-select"
            | "skill-clear"
    ) {
        return Err(contract::skills_unavailable());
    }
    if inv.command == "restore" {
        let store = Store::restore(inv.arg("source")?, &inv.data)?;
        // Every restored copy is an owner workspace; read back from the copy.
        let purpose = store.workspace_purpose()?;
        return Ok(json!({"restored":store.directory(),"workspace_purpose":purpose.as_str()}));
    }
    if inv.command == "model-install" {
        return install_model(inv);
    }
    if inv.command == "upgrade" {
        Store::upgrade(&inv.data, Path::new(inv.arg("backup-directory")?))?;
        return Ok(json!({"upgraded":true,"data":inv.data}));
    }
    // Machine mode: a read never creates a workspace at a mistyped path.
    if inv.machine && spec::retry_for(inv) == spec::Class::Read && workspace_missing(&inv.data) {
        return Err(CliError::new(
            Code::MissingWorkspace,
            "No BRN workspace at --data-dir; check the path, or run init deliberately",
        ));
    }
    // A workspace the CLI creates has no credential source until `configure`.
    // The CLI never upgrades: an older workspace is refused unchanged
    // (upgrade_required); only `upgrade` upgrades, backup-first.
    let mut workspace = if inv.command == "init" && inv.args.contains_key("purpose") {
        Workspace::create_without_default_credentials(&inv.data, WorkspacePurpose::Test)?
    } else {
        Workspace::open_existing_no_upgrade(&inv.data)?
    };
    workspace.store.set_actor(inv.actor.as_deref())?;
    contract::set_purpose(workspace.store.workspace_purpose()?);
    stop::check()?;
    // A named but unusable model folder is an error, never a silent fallback.
    if matches!(inv.command.as_str(), "search" | "ask" | "intake") {
        if let Some(Err(error)) = workspace.embedding_model_from_env() {
            let message = format!("Semantic search is unavailable: {error}");
            return Err(CliError::new(Code::Unavailable, message));
        }
    }
    let result = match inv.command.as_str() {
        "init" => {
            json!({"ready":true,"data":inv.data,"settings":settings_output(workspace.settings()?)?})
        }
        "projects" if inspect::page_mode(inv) => inspect::projects(&workspace, inv, notes)?,
        "projects" => {
            encode(workspace.projects(inv.args.get("trash").is_some_and(|v| v == "yes"))?)?
        }
        "project-delete-preview" => encode(workspace.project_delete_preview(inv.arg("project")?)?)?,
        "project-create" | "project-chat" | "project-document" | "project-settings"
        | "chat-move" | "project-link" | "project-unlink" | "project-delete"
        | "project-restore" => {
            let command = match inv.command.as_str() {
                "project-create" => ProjectCommand::Create {
                    title: inv.arg("title")?.into(),
                },
                "project-chat" => ProjectCommand::CreateChat {
                    project: inv.arg("project")?.into(),
                    title: inv.arg("title")?.into(),
                },
                "project-document" => ProjectCommand::CreateDocument {
                    project: inv.arg("project")?.into(),
                    title: inv.arg("title")?.into(),
                    markdown: inv.arg("text")?.into(),
                },
                "project-settings" => ProjectCommand::Configure {
                    project: inv.arg("project")?.into(),
                    version: inv.number("base")?,
                    title: inv.arg("title")?.into(),
                    instructions: inv.arg("instructions")?.into(),
                    use_brn_knowledge: match inv.arg("knowledge")? {
                        "yes" => true,
                        "no" => false,
                        _ => {
                            return Err(CliError::invalid_value(
                                "knowledge",
                                "--knowledge takes yes|no",
                            ));
                        }
                    },
                },
                "chat-move" => ProjectCommand::MoveChat {
                    chat: inv.arg("thread")?.into(),
                    project: inv
                        .args
                        .get("project")
                        .filter(|p| p.as_str() != "none")
                        .cloned(),
                },
                "project-link" => ProjectCommand::LinkDocument {
                    project: inv.arg("project")?.into(),
                    note: inv.arg("note")?.into(),
                },
                "project-unlink" => ProjectCommand::UnlinkDocument {
                    project: inv.arg("project")?.into(),
                    note: inv.arg("note")?.into(),
                },
                "project-delete" => ProjectCommand::Delete {
                    preview: serde_json::from_slice(&input::read_preview_file(
                        inv.arg("preview-file")?.into(),
                        &CANCEL,
                    )?)
                    .map_err(|e| {
                        CliError::invalid_input("preview-file", "invalid_json", e.to_string())
                    })?,
                    actions: serde_json::from_str(inv.arg("actions")?).map_err(|e| {
                        let expected = "JSON object mapping Action IDs to Keep or Cancel";
                        CliError::invalid_value("actions", e.to_string()).with("expected", expected)
                    })?,
                },
                "project-restore" => ProjectCommand::Restore {
                    project: inv.arg("project")?.into(),
                    version: inv.number("base")?,
                },
                _ => unreachable!(),
            };
            let key = inv
                .args
                .get("key")
                .cloned()
                .unwrap_or_else(|| format!("cli:{}", uuid::Uuid::new_v4()));
            match command {
                ProjectCommand::MoveChat { .. }
                | ProjectCommand::LinkDocument { .. }
                | ProjectCommand::UnlinkDocument { .. } => {
                    converged(workspace.project_command_converged(&key, command)?)?
                }
                command => encode(workspace.project_command(&key, command)?)?,
            }
        }
        "records" if inspect::page_mode(inv) => inspect::records(&workspace, inv, notes)?,
        "records" => encode(workspace.records()?)?,
        "notes" if inspect::page_mode(inv) => inspect::notes(&workspace, inv, notes)?,
        "notes" => encode(workspace.notes()?)?,
        "threads" if inspect::page_mode(inv) => inspect::threads(&workspace, inv, notes)?,
        "threads" => encode(workspace.threads()?)?,
        "needs-you" if inspect::page_mode(inv) => inspect::needs_you(&workspace, inv, notes)?,
        "needs-you" => encode(workspace.needs_you()?)?,
        "history" if inspect::page_mode(inv) => inspect::history(&workspace, inv, notes)?,
        "history" => encode(workspace.history()?)?,
        "candidates" if inspect::page_mode(inv) => inspect::candidates(&workspace, inv, notes)?,
        "candidates" => encode(workspace.candidates()?)?,
        "revisions" => inspect::revisions(&workspace, inv, notes)?,
        "revision" => inspect::revision(&workspace, inv, notes)?,
        "runs" => inspect::runs(&workspace, inv, notes)?,
        "messages" => inspect::messages(&workspace, inv, notes)?,
        "operation" => inspect::operation(&workspace, inv)?,
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skills" => {
            let policy = workspace
                .scoped_skills(inv.args.get("project").map(String::as_str))
                .map_err(contract::skill_error)?;
            let inventory = workspace.skills().map_err(|error| {
                let error = contract::skill_error(error);
                if policy.iter().any(|s| !s.available) {
                    error.with("skill_policy", json!(&policy))
                } else {
                    error
                }
            })?;
            json!({"skills":inventory,"policy":policy,"canonical_policy_in_backup":true,"installed_configuration_in_backup":false,"installed_configuration_in_export":false,"retention_notice":SKILL_RETENTION_NOTICE})
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-inspect" => {
            let name = inv.arg("name")?;
            let policy = workspace
                .scoped_skills(inv.args.get("project").map(String::as_str))
                .map_err(contract::skill_error)?
                .into_iter()
                .filter(|s| s.id == name || s.id == format!("installed:{name}"))
                .collect::<Vec<_>>();
            match workspace.inspect_skill(name) {
                Ok(package) => {
                    let mut value = serde_json::to_value(package)?;
                    value["policy"] = encode(policy)?;
                    value
                }
                Err(AppError::Invalid(_)) if policy.iter().any(|s| !s.available) => {
                    json!({"info":null,"files":null,"policy":policy,"unavailable":true})
                }
                Err(error) => return Err(contract::skill_error(error)),
            }
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-install" => serde_json::to_value(
            workspace
                .install_skill(Path::new(inv.arg("source")?))
                .map_err(contract::skill_error)?,
        )?,
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-remove" => {
            json!({"removed":workspace.remove_skill(inv.arg("name")?,inv.arg("digest")?).map_err(contract::skill_error)?,"retention_notice":SKILL_RETENTION_NOTICE})
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-enable" | "skill-disable" => applied(
            workspace
                .set_skill_enabled(
                    inv.arg("id")?,
                    inv.arg("digest")?,
                    inv.args.get("project").map(String::as_str),
                    inv.command == "skill-enable",
                )
                .map_err(contract::skill_error)?,
        ),
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-select" => applied(
            workspace
                .select_skill(
                    (inv.arg("id")? != "none").then_some(inv.arg("id")?),
                    inv.args.get("project").map(String::as_str),
                )
                .map_err(contract::skill_error)?,
        ),
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        "skill-clear" => applied(
            workspace
                .clear_skill_policy(inv.arg("id")?, inv.args.get("project").map(String::as_str))
                .map_err(contract::skill_error)?,
        ),
        "settings" => settings_output(workspace.settings()?)?,
        "delegation" => {
            applied(workspace.choose_delegation(delegation_choice(inv.arg("choice")?)?)?)
        }
        "read" if inv.args.contains_key("limit") => inspect::read(&workspace, inv, notes)?,
        "read" => encode(workspace.store.record(inv.arg("id")?)?)?,
        "note-create" => created(
            notes,
            workspace.new_note_with_key(&inv.owner_key(), inv.arg("title")?, inv.arg("text")?)?,
        )?,
        "thread-start" => created(
            notes,
            workspace.new_thread_with_key(&inv.owner_key(), inv.arg("title")?)?,
        )?,
        "topics" => encode(workspace.topics()?)?,
        "topics-create" => {
            let (note, outcome) = match &inv.key {
                Some(_) => workspace.create_topics_note_with_key(&inv.host_key("cli-topics"))?,
                None => workspace.create_topics_note()?,
            };
            json!({"note":note,"pointer":applied(outcome)})
        }
        "topics-choose" => applied(workspace.choose_topics_note(inv.arg("note")?)?),
        "knowledge-check" => {
            let check = workspace.knowledge_check()?;
            let chat = match inv.args.get("seed-chat").map(String::as_str) {
                Some("yes") if inv.key.is_some() => {
                    let (chat, message) = workspace
                        .seed_knowledge_review_with_key(&inv.host_key("cli-knowledge"), &check)?;
                    notes.operation = Some(message.as_str().into());
                    Some(chat)
                }
                Some("yes") => Some(workspace.seed_knowledge_review(&check)?),
                None => None,
                Some(_) => {
                    return Err(CliError::invalid_value(
                        "seed-chat",
                        "--seed-chat takes yes",
                    ));
                }
            };
            let mut output = encode(check)?;
            output["review_chat"] = json!(chat);
            output
        }
        "connections" => {
            let hops = match inv.args.get("hops") {
                Some(_) => u8::try_from(inv.number("hops")?)
                    .map_err(|e| CliError::invalid_value("hops", e.to_string()))?,
                None => 1,
            };
            let sources = match inv.args.get("sources").map(String::as_str) {
                Some("yes") => true,
                None => false,
                Some(_) => return Err(CliError::invalid_value("sources", "--sources takes yes")),
            };
            let chat = chat_option(&workspace, inv)?;
            let handle = workspace.search_handle()?;
            stop::check()?;
            let found = handle.connections(inv.arg("note")?, chat.as_deref(), hops, sources)?;
            let node = |n: &brn_threads_app::search::Neighbour| {
                json!({"id":n.id,"version":n.version,"title":n.title,
                    "kind":format!("{:?}", n.kind).to_lowercase(),
                    "relations":n.relations.iter().map(|r| format!("{}:{}", if r.outgoing {"out"} else {"in"}, r.relation)).collect::<Vec<_>>(),
                    "links":n.relations.iter().flat_map(|r| r.links.iter().map(move |l| json!({"id":l.id,"version":l.version,"relation":format!("{}:{}", if r.outgoing {"out"} else {"in"}, r.relation)}))).collect::<Vec<_>>(),
                    "via":n.via,"inbound_links":n.inbound_links})
            };
            json!({"centre":node(&found.centre),"hops":found.hops.iter().enumerate().map(|(i, h)| json!({
                "hop":i + 1,"available":h.available,"shown":h.shown.len(),
                "neighbours":h.shown.iter().map(node).collect::<Vec<_>>()})).collect::<Vec<_>>()})
        }
        "search" => {
            let limit = match inv.args.get("limit") {
                Some(_) => usize::try_from(inv.number("limit")?)
                    .map_err(|e| CliError::invalid_value("limit", e.to_string()))?,
                None => 12,
            };
            let chat = chat_option(&workspace, inv)?;
            let page = workspace.search_page(
                inv.arg("query")?,
                chat.as_deref(),
                limit,
                inv.args.get("cursor").map(String::as_str),
            )?;
            stop::check()?;
            let shown = page.hits.len();
            // The envelope page covers both lanes, which share one offset:
            // null means neither lane has more.
            let cursor = page.cursor.clone().or_else(|| page.message_cursor.clone());
            let total = page.total + page.message_total;
            let both = shown + page.messages.len();
            notes.page =
                Some(json!({"limit": limit, "shown": both, "cursor": cursor, "total": total}));
            let mut output = encode(page)?;
            output["shown"] = json!(shown);
            output
        }
        "begin-edit" => {
            let out = token_file(inv)?;
            let issued = workspace
                .store
                .begin_caller_edit(inv.arg("note")?, inv.number("base")?)
                .map_err(|error| {
                    discard_token_file(&out);
                    contract::begin_edit_error(error)
                })?;
            issued_output(&mut workspace, issued, out, true)?
        }
        "reclaim-edit" => {
            let out = token_file(inv)?;
            let issued = workspace
                .store
                .reclaim_caller_edit(inv.arg("session")?)
                .map_err(|error| {
                    discard_token_file(&out);
                    CliError::from(error)
                })?;
            issued_output(&mut workspace, issued, out, false)?
        }
        "update-buffer" => match workspace.store.update_caller_buffer(
            inv.arg("session")?,
            inv.token()?,
            inv.number("generation")?,
            inv.arg("text")?,
        )? {
            BufferOutcome::Updated(session) => json!({"outcome":"Updated","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "save" => match workspace.store.save_caller_edit(
            inv.arg("session")?,
            inv.token()?,
            inv.number("generation")?,
        )? {
            SaveOutcome::Saved(receipt) => json!({"outcome":"Saved","receipt":receipt}),
            SaveOutcome::Stale(session) => json!({"outcome":"Stale","session":session}),
            SaveOutcome::GenerationMismatch(session) => {
                json!({"outcome":"GenerationMismatch","session":session})
            }
            SaveOutcome::Closed(session) => json!({"outcome":"Closed","session":session}),
            SaveOutcome::Deferred { guarded } => json!({"outcome":"Deferred","guarded":guarded}),
        },
        "discard" => match workspace.store.discard_caller_edit(
            inv.arg("session")?,
            inv.token()?,
            inv.number("generation")?,
        )? {
            BufferOutcome::Updated(session) => json!({"outcome":"Discarded","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "recovery" if inspect::page_mode(inv) => inspect::recovery(&workspace, inv, notes)?,
        "recovery" => encode(workspace.store.recovery_buffers()?)?,
        "apply" => {
            let result = workspace
                .review_apply(inv.arg("operation")?)
                .map_err(|e| review_error(&workspace, inv, "applied", e))?;
            let mut value = applied(result.outcome);
            if let Some(error) = result.attention_error {
                value["attention_error"] = json!(error);
            }
            value
        }
        "dismiss" => {
            workspace
                .review_dismiss(inv.arg("operation")?)
                .map_err(|e| review_error(&workspace, inv, "dismissed", e))?;
            json!({"outcome":"Dismissed","operation":inv.arg("operation")?})
        }
        "undo" => {
            let (operation, outcome) =
                workspace.undo_with_key(&inv.owner_key(), inv.arg("operation")?)?;
            notes.operation = Some(operation.as_str().into());
            if let ApplyOutcome::Applied(receipt) = &outcome {
                notes.changed = contract::changed(&encode(&receipt.writes)?);
            }
            applied(outcome)
        }
        "resolve" | "archive" => converged(
            workspace.archive_chat_converged(&inv.host_key("cli-archive"), inv.arg("thread")?)?,
        )?,
        "unarchive" => converged(
            workspace
                .unarchive_chat_converged(&inv.host_key("cli-unarchive"), inv.arg("thread")?)?,
        )?,
        "keep-in-brn" => applied(workspace.keep_in_brn(
            &inv.host_key("cli-keep"),
            inv.arg("note")?,
            inv.number("base")?,
        )?),
        "queue" => encode(workspace.queue_message(
            &inv.host_key("cli-queue"),
            inv.arg("thread")?,
            inv.arg("text")?,
        )?)?,
        "pause" => applied(workspace.pause(true)?),
        "resume" => applied(workspace.pause(false)?),
        "mcp-status" => encode(workspace.mcp_status(inv.args.get("thread").map(String::as_str))?)?,
        "mcp-discover" => encode(Workspace::discover_mcp(inv.arg("endpoint")?).await?)?,
        "mcp-configure" => {
            let policy: Option<McpPolicy> =
                serde_json::from_str(inv.arg("policy")?).map_err(|_| {
                    CliError::invalid_value("policy", "MCP policy must be bounded JSON or null")
                })?;
            applied(workspace.configure_mcp(policy)?)
        }
        "mcp-disable" => match workspace.disable_mcp()? {
            Some(outcome) => applied(outcome),
            None => json!({"outcome":"Unchanged"}),
        },
        "configure" => {
            let mut settings = workspace.settings()?;
            let chosen = provider(inv.arg("provider")?)?;
            settings.provider = match chosen {
                Provider::Chatgpt => "Codex",
                Provider::Copilot => "Copilot",
            }
            .into();
            settings.model = inv.arg("model")?.into();
            settings.effort = effort(inv.arg("effort")?)?.as_str().into();
            if let Some(auth) = inv.args.get("auth-file") {
                settings.auth_file = Some(auth.clone());
                settings.credentials_dir = None;
            }
            if let Some(dir) = inv.args.get("credentials-dir") {
                settings.credentials_dir = Some(dir.clone());
                settings.auth_file = None;
            }
            settings.helper = helper_settings(inv, &settings)?;
            applied(workspace.configure(settings)?)
        }
        "document-conversion" => {
            let converter = match inv.arg("converter")? {
                "xberg" => DocumentConverter::Xberg,
                "xberg-brn" => DocumentConverter::XbergBrn,
                _ => {
                    return Err(CliError::invalid_value(
                        "converter",
                        "Choose xberg or xberg-brn",
                    ));
                }
            };
            let ai = match inv.arg("ai")? {
                "off" => false,
                "on" => true,
                _ => return Err(CliError::invalid_value("ai", "Choose off or on")),
            };
            applied(
                workspace
                    .choose_document_conversion(DocumentConversionSettings { converter, ai })?,
            )
        }
        "ask" => {
            ask(
                &mut workspace,
                inv,
                inv.arg("thread")?.into(),
                inv.arg("prompt")?.into(),
                vec![],
            )
            .await?
        }
        "intake" => {
            let settings = workspace.settings()?;
            let captured = settings.document_conversion.clone();
            let source = Path::new(inv.arg("source")?);
            if !source.is_absolute() {
                return Err(CliError::invalid_value(
                    "source",
                    "Source path must be absolute",
                ));
            }
            // Useful intake for a project stays in that project; checked before reading.
            let useful_project =
                if inv.arg("intent")? == "useful" && inv.args.contains_key("project") {
                    chat_option(&workspace, inv)?
                } else {
                    None
                };
            let bytes =
                brn_threads_intake::read_input_file(source, brn_intake::MAX_INPUT_BYTES, &CANCEL)
                    .map_err(contract::source_read_error)?;
            let kind = match source
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                .as_str()
            {
                "md" | "markdown" => DocumentKind::Markdown,
                "txt" => DocumentKind::Text,
                "eml" => DocumentKind::Eml,
                "pdf" => DocumentKind::Pdf,
                "docx" => DocumentKind::Docx,
                "html" | "htm" => DocumentKind::Html,
                "xlsx" => DocumentKind::Xlsx,
                "pptx" => DocumentKind::Pptx,
                _ => {
                    return Err(CliError::invalid_value(
                        "source",
                        "Supported sources: Markdown, text, EML, PDF, DOCX, HTML, XLSX, PPTX",
                    ));
                }
            };
            let intent = match inv.arg("intent")? {
                "useful" => ImportIntent::UsefulInformation,
                "full" | "chat" | "project" => ImportIntent::FullNote,
                _ => {
                    return Err(CliError::invalid_value(
                        "intent",
                        "Choose useful, full, or chat with --thread",
                    ));
                }
            };
            // A keyed full/chat/project intake whose committed import matches
            // replays here, before conversion: nothing is converted or sent again.
            if let (Some(key), ImportIntent::FullNote) = (&inv.key, intent) {
                let scope = match inv.arg("intent")? {
                    "chat" => Some(inv.arg("thread")?),
                    "project" => Some(inv.arg("project")?),
                    _ => None,
                };
                let title = inv
                    .args
                    .get("title")
                    .map_or("Imported note", String::as_str);
                let replayed = workspace.full_import_replay_with_conversion(
                    &spec::cli_key(&inv.command, key),
                    title,
                    inv.arg("source")?,
                    &bytes,
                    scope,
                    &captured,
                )?;
                if let Some(outcome) = replayed {
                    let value = applied(outcome);
                    contract::fill_conflicts(&workspace, inv, &value, notes);
                    return Ok(value);
                }
            }
            // The useful-information agent has its separate credential requirement.
            if intent == ImportIntent::UsefulInformation || captured.ai {
                contract::require_credentials(inv, &settings)?;
            }
            let helpers = DocumentHelpers::discover()
                .map_err(brn_ai::conversion::ConversionFailure::Intake)?;
            let conversion_job = brn_ai::document_refine::DocumentConversionJob {
                converter: Converter {
                    temp_root: std::env::temp_dir()
                        .canonicalize()?
                        .join("brn-threads-intake"),
                    limits: import_limits(kind),
                },
                helpers,
                settings: captured.clone(),
                kind,
                intent,
                bytes,
                source_reference: inv.arg("source")?.into(),
                auth_file: settings.auth_file.map(PathBuf::from),
                credentials_dir: settings.credentials_dir.map(PathBuf::from),
            };
            let cancel = CancellationToken::new();
            let observed = cancel.clone();
            let watcher = tokio::spawn(async move {
                loop {
                    if CANCEL.load(Ordering::SeqCst) {
                        observed.cancel();
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
            });
            let flag = Arc::new(AtomicBool::new(CANCEL.load(Ordering::SeqCst)));
            let emit: Arc<dyn Fn(brn_ai::conversion::ConversionProgress) + Send + Sync> =
                Arc::new(|progress| contract::emit_err(&format!("{}\n", progress.label())));
            #[cfg(all(test, feature = "scripted-document-tests"))]
            let result = match document_transport {
                Some(http) => {
                    brn_ai::document_refine::convert_document_job_with_http(
                        conversion_job,
                        cancel,
                        flag,
                        emit,
                        http,
                    )
                    .await
                }
                None => {
                    brn_ai::document_refine::convert_document_job(
                        conversion_job,
                        cancel,
                        flag,
                        emit,
                    )
                    .await
                }
            };
            #[cfg(not(all(test, feature = "scripted-document-tests")))]
            let result =
                brn_ai::document_refine::convert_document_job(conversion_job, cancel, flag, emit)
                    .await;
            watcher.abort();
            let converted = result?;
            if CANCEL.load(Ordering::SeqCst) {
                return Err(CliError::new(
                    Code::Interrupted,
                    "Import stopped before saving converted content",
                ));
            }
            let title = inv
                .args
                .get("title")
                .map(String::as_str)
                .unwrap_or("Imported note");
            if matches!(inv.arg("intent")?, "chat" | "project") {
                let scope = if inv.arg("intent")? == "project" {
                    inv.arg("project")?
                } else {
                    inv.arg("thread")?
                };
                let value = applied(workspace.import_full_scoped_with_conversion(
                    &inv.host_key("cli-attach"),
                    title,
                    &converted,
                    Some(scope),
                    &captured,
                )?);
                contract::fill_conflicts(&workspace, inv, &value, notes);
                return Ok(value);
            }
            if intent == ImportIntent::FullNote {
                applied(workspace.import_full_scoped_with_conversion(
                    &inv.host_key("cli-import"),
                    title,
                    &converted,
                    None,
                    &captured,
                )?)
            } else {
                let report = brn_threads_app::import_report_with_conversion(&converted, &captured)?;
                // A retry or re-supply reuses the chat holding the earlier outcome.
                // While a Run there is still live, it gets a new chat instead:
                // `reused_chat` false and the new `thread`. `reused_chat` is null
                // when the owner named the chat.
                let (thread, reused) = if let Some(id) = inv.args.get("thread") {
                    (id.clone(), None)
                } else {
                    let (chat, reused) = workspace.source_intake_chat(
                        useful_project.as_deref(),
                        &converted.source_reference,
                        "Useful information intake",
                    )?;
                    (chat.id, Some(reused))
                };
                let source = brn_ai::threads::TransientSource {
                    fingerprint: Some(brn_intake::hex(&converted.source_sha256)),
                    report: Some(report),
                    reference: converted.source_reference,
                    text: converted.markdown,
                    assets: converted
                        .assets
                        .into_iter()
                        .map(|asset| brn_ai::threads::TransientAsset {
                            name: asset.name,
                            media_type: asset.media_type,
                            sha256: asset.sha256,
                            width: asset.width,
                            height: asset.height,
                            bytes: asset.bytes,
                        })
                        .collect(),
                };
                let mut output = ask(
                    &mut workspace,
                    inv,
                    thread.clone(),
                    brn_threads_app::SOURCE_INTAKE_INSTRUCTION.into(),
                    vec![source],
                )
                .await?;
                output["thread"] = json!(thread);
                output["reused_chat"] = json!(reused);
                return Ok(output);
            }
        }
        "export" => encode(
            workspace
                .export_now(Path::new(inv.arg("destination")?))
                .map_err(|e| contract::destination_error(Path::new(&inv.args["destination"]), e))?,
        )?,
        "backup" => {
            workspace
                .backup(Path::new(inv.arg("destination")?))
                .map_err(|e| contract::destination_error(Path::new(&inv.args["destination"]), e))?;
            json!({"backup":inv.arg("destination")?})
        }
        _ => return Err(CliError::new(Code::Internal, "Unknown command")),
    };
    contract::fill_conflicts(&workspace, inv, &result, notes);
    Ok(result)
}
#[tokio::main]
async fn main() -> ExitCode {
    // SAFETY: this signal handler only stores a lock-free atomic flag.
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
    }
    match parse(std::env::args_os().skip(1)) {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(mut inv)) => {
            if inv.machine {
                stop::install_sigterm();
            }
            if inv.machine || inv.timeout.is_some() {
                stop::start(stop::Watch {
                    machine: inv.machine,
                    deadline: inv.timeout,
                    command: inv.command.clone(),
                    key: inv.key.clone(),
                    retry: spec::retry_for(&inv),
                });
            }
            let mut notes = Annotations::default();
            let outcome = match input::resolve(&mut inv, &CANCEL) {
                Ok(()) => execute(&inv, &mut notes).await,
                Err(error) => Err(error),
            };
            finish(&inv, outcome, notes)
        }
        Err(refused) if refused.machine => {
            let envelope = contract::Envelope {
                command: refused.command.as_deref(),
                status: refused.error.code.status(),
                outcome: None,
                operation: None,
                key: refused.key.as_deref(),
                retry: spec::Class::Refuse,
                changed: vec![],
                conflict: None,
                result: Value::Null,
                error: Some(refused.error),
                page: None,
                stop: None,
            };
            emit_envelope(envelope)
        }
        Err(refused) => {
            if stop::claim_output() {
                contract::emit_err(&format!("{}\n", refused.error.default_message()));
            }
            ExitCode::FAILURE
        }
    }
}
/// Prints the result: today's output by default, one envelope with `--json`.
fn finish(inv: &Invocation, outcome: Result<Value>, notes: Annotations) -> ExitCode {
    let outcome = match outcome {
        // In machine mode an unknown record is an error, not a null result.
        Ok(Value::Null) if inv.machine && inv.command == "read" => {
            Err(CliError::new(Code::NotFound, "Record not found"))
        }
        Err(error) => Err(stop::restamp(error, inv.timeout).bind_key(inv.key.as_deref())),
        other => other,
    };
    if !inv.machine {
        if !stop::claim_output() {
            park();
        }
        return match outcome {
            Ok(result) => {
                contract::emit_line(&result.to_string());
                ExitCode::SUCCESS
            }
            Err(error) => {
                contract::emit_err(&format!("{}\n", error.default_message()));
                ExitCode::FAILURE
            }
        };
    }
    let stop = notes.stop.unwrap_or_else(stop::reason);
    let retry = spec::retry_for(inv);
    let envelope = match outcome {
        Ok(result) => {
            let classified = contract::classify(inv, &result, &notes, stop);
            contract::Envelope {
                command: Some(&inv.command),
                status: classified.status,
                outcome: classified.outcome,
                operation: classified.operation,
                key: inv.key.as_deref(),
                retry,
                changed: classified.changed,
                conflict: notes.conflict,
                result,
                error: None,
                page: notes.page,
                stop,
            }
        }
        Err(error) => contract::Envelope {
            command: Some(&inv.command),
            status: error.code.status(),
            outcome: None,
            operation: None,
            key: inv.key.as_deref(),
            retry,
            changed: vec![],
            conflict: None,
            result: Value::Null,
            error: Some(error),
            page: None,
            stop,
        },
    };
    emit_envelope(envelope)
}
/// The watchdog claimed the output and is ending the process; wait for it.
fn park() -> ! {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
fn emit_envelope(envelope: contract::Envelope) -> ExitCode {
    if !stop::claim_output() {
        park();
    }
    let value = envelope.to_value();
    contract::emit_line(&value.to_string());
    ExitCode::from(envelope.status.exit())
}

#[cfg(test)]
mod cw2_run_tests {
    use super::*;
    #[test]
    fn shared_selection_retains_cli_invalid_provider_and_effort_contract() {
        for option in ["provider", "effort"] {
            let mut settings = WorkspaceSettings {
                document_conversion: Default::default(),
                mcp: None,
                skills: vec![],
                provider: "Codex".into(),
                model: "synthetic".into(),
                effort: "medium".into(),
                auth_file: None,
                credentials_dir: None,
                maintenance: false,
                review_first: true,
                delegation_chosen: false,
                backup_directory: None,
                theme: None,
                topics_note: None,
                helper: None,
            };
            let expected = if option == "provider" {
                settings.provider = "unsupported".into();
                provider(&settings.provider).unwrap_err()
            } else {
                settings.effort = "unsupported".into();
                effort(&settings.effort).unwrap_err()
            };
            let error = resolve_run_selection(&settings, None).unwrap_err();
            let actual = preparation_cli_error(RunPreparationError::Selection(error));
            assert_eq!(actual.code.as_str(), expected.code.as_str());
            assert_eq!(actual.message, expected.message);
            assert_eq!(actual.details, expected.details);
        }
    }
}

// This module is an in-process CLI-route witness, never a subprocess/live proof.
#[cfg(all(test, feature = "scripted-document-tests", target_os = "macos"))]
mod document_route_tests {
    use super::*;
    use brn_ai::document_refine::{
        DocumentHttpClient, DocumentMockHttpResponse, DocumentSequencedHttpClient,
    };
    use std::os::unix::fs::PermissionsExt;

    fn completed(markdown: &str, hash: [u8; 32]) -> DocumentSequencedHttpClient {
        let text=json!({"schema":1,"source_sha256":hash,"markdown":markdown,"uncertainty":"Synthetic check"}).to_string();
        let response = json!({"type":"response.completed","sequence_number":1,"response":{"id":"synthetic-response","object":"response","created_at":1,"status":"completed","model":"gpt-6-luna","output":[{"type":"message","id":"synthetic-message","status":"completed","role":"assistant","content":[{"type":"output_text","annotations":[],"text":text}]}],"tools":[],"usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}}});
        DocumentSequencedHttpClient::new([DocumentMockHttpResponse::success_typed(
            format!("data: {response}\n\ndata: [DONE]\n\n"),
            "text/event-stream",
        )])
    }
    fn invocation(data: &Path, source: &Path, key: &str) -> Invocation {
        parse(
            [
                "--json",
                "--data-dir",
                data.to_str().unwrap(),
                "intake",
                "--source",
                source.to_str().unwrap(),
                "--intent",
                "full",
                "--key",
                key,
            ]
            .into_iter()
            .map(OsString::from),
        )
        .unwrap_or_else(|_| panic!("valid authored command"))
        .unwrap()
    }
    #[tokio::test]
    async fn documents_cli_route_ai_on_both_engines_saves_snapshot_and_mismatch_never_dispatches() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("data");
        let mut workspace =
            Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
        let auth = root.path().join("synthetic-auth.json");
        std::fs::write(&auth,r#"{"auth_mode":"chatgpt","tokens":{"access_token":"synthetic-not-a-credential","account_id":"synthetic"}}"#).unwrap();
        std::fs::set_permissions(&auth, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut settings = workspace.settings().unwrap();
        settings.auth_file = Some(auth.to_str().unwrap().into());
        workspace.configure(settings).unwrap();
        let source = root.path().join("source.txt");
        let bytes = b"Original engine input\r\n";
        std::fs::write(&source, bytes).unwrap();
        for (converter, key) in [
            (DocumentConverter::Xberg, "stock"),
            (DocumentConverter::XbergBrn, "brn"),
        ] {
            workspace
                .choose_document_conversion(DocumentConversionSettings {
                    converter,
                    ai: true,
                })
                .unwrap();
            let helpers = DocumentHelpers::discover().unwrap();
            let expected = Converter {
                temp_root: root.path().join("selected-output-control"),
                limits: import_limits(DocumentKind::Text),
            }
            .convert_document(
                ConversionRequest {
                    kind: DocumentKind::Text,
                    intent: ImportIntent::FullNote,
                    bytes,
                    source_reference: source.to_str().unwrap(),
                },
                &helpers,
                if converter == DocumentConverter::Xberg {
                    "xberg"
                } else {
                    "xberg_brn"
                },
                &AtomicBool::new(false),
            )
            .unwrap();
            let http = completed("# Synthetic refined output\n", brn_intake::digest(bytes));
            let inv = invocation(&data, &source, key);
            execute_document_transport(
                &inv,
                &mut Annotations::default(),
                Some(DocumentHttpClient::new(http.clone())),
            )
            .await
            .unwrap();
            assert_eq!(http.requests().len(), 1);
            // Capture only body, never headers or credential bytes.
            let body: Value = serde_json::from_slice(&http.requests()[0].body).unwrap();
            assert_eq!(body["model"], "gpt-6-luna");
            assert_eq!(body["reasoning"], json!({"effort":"high"}));
            let prompt = body["input"][0]["content"][0]["text"].as_str().unwrap();
            assert!(prompt.contains(&serde_json::to_string(&expected.markdown).unwrap()));
            if let Some(path) = std::env::var_os("BRN_TASK3_CAPTURES") {
                let path = PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(
                    path.join(format!("on-{key}.json")),
                    serde_json::to_vec_pretty(&json!({"selected":expected,"request_body":body}))
                        .unwrap(),
                )
                .unwrap();
            }
            let records = workspace.records().unwrap();
            let note = records
                .iter()
                .filter_map(|r| match &r.data {
                    RecordData::Note(n)
                        if n.import
                            .as_ref()
                            .and_then(|i| i.report.as_ref())
                            .and_then(|r| r.conversion.as_ref())
                            .is_some_and(|p| p.settings.converter == converter) =>
                    {
                        Some(n)
                    }
                    _ => None,
                })
                .next()
                .unwrap();
            assert!(note.protected);
            assert_eq!(note.markdown, "# Synthetic refined output\n");
            assert_eq!(
                note.import
                    .as_ref()
                    .unwrap()
                    .report
                    .as_ref()
                    .unwrap()
                    .conversion
                    .as_ref()
                    .unwrap()
                    .settings,
                DocumentConversionSettings {
                    converter,
                    ai: true
                }
            );
            let other = if converter == DocumentConverter::Xberg {
                DocumentConverter::XbergBrn
            } else {
                DocumentConverter::Xberg
            };
            workspace
                .choose_document_conversion(DocumentConversionSettings {
                    converter: other,
                    ai: true,
                })
                .unwrap();
            let refused = DocumentSequencedHttpClient::new([]);
            let error = execute_document_transport(
                &inv,
                &mut Annotations::default(),
                Some(DocumentHttpClient::new(refused.clone())),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code.as_str(), "key_conflict");
            assert!(refused.requests().is_empty());
        }
    }
}
