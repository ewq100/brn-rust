//! Thin host CLI over the same checked Threads service as the desktop.
mod contract;
mod input;
mod inspect;
mod spec;
mod stop;
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
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
const HELP: &str = "BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND [--option VALUE]\n\nCommands: init, records, notes, threads, needs-you, history, candidates, settings,\n  projects [--trash yes]; project-create --title TITLE [--key KEY]\n  project-chat --project ID --title TITLE; project-document --project ID --title TITLE --text MARKDOWN\n  project-settings --project ID --base VERSION --title TITLE --instructions TEXT --knowledge yes|no\n  chat-move --thread ID [--project ID|none]; project-link|project-unlink --project ID --note ID\n  project-delete-preview --project ID (save JSON for review)\n  project-delete --preview-file FILE --actions JSON [--key KEY]\n  project-restore --project ID --base VERSION\n  Project commands accept --key KEY for durable retry. Actions JSON maps each unfinished Action ID to Keep or Cancel.\n  Search/connections accept --project ID; intake --intent project --project ID imports privately.\n  note-create --title TITLE --text MARKDOWN\n  thread-start --title TITLE\n  read --id RECORD\n  search --query TEXT [--limit 1..50] [--cursor NEXT] [--thread CHAT]\n  connections --note ID [--hops 1|2] [--sources yes] [--thread CHAT]\n  topics; topics-create; topics-choose --note ID\n  knowledge-check [--seed-chat yes]\n  model-install --destination ABSOLUTE_FRESH_FOLDER (native builds; downloads the pinned model)\n  Semantic search uses the model folder named by BRN_EMBEDDING_MODEL_DIR.\n  begin-edit --note ID --base VERSION\n  update-buffer --session ID --generation N --text MARKDOWN\n  save|discard --session ID --generation N; recovery\n  apply|dismiss --operation ID; undo --operation ID; archive|unarchive --thread ID\n  queue --thread ID --text TEXT; keep-in-brn --note ID --base VERSION\n  ask --thread ID --prompt TEXT [--budget 1..32] [--edit-note ID --base VERSION | --record-decision yes] [--trace summary|full]\n  intake --source ABSOLUTE_FILE --intent useful|full|chat|project [--title TITLE] [--thread ID] [--project ID] [--trace summary|full]\n  configure --provider Codex|Copilot --model ID --effort low|medium|high\n    [--helper off | --helper-provider Codex|Copilot --helper-model ID --helper-effort low|medium|high\n     [--helper-auth-file FILE|--helper-credentials-dir DIR]] (read-only research helper, off by default.\n     Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower)\n  delegation --choice maintain|review-first\n  pause; resume; export|backup --destination ABSOLUTE_FRESH_PATH\n  restore --source ABSOLUTE_BACKUP\n  upgrade --backup-directory ABSOLUTE_EXTERNAL_FOLDER\n\nChoose delegation before resuming maintenance. Manual work remains available while pending.\nFresh explicit data only. Originals remain external; no external messages are sent.\nReview apply and Undo are concrete owner actions. Continue uses ask on the same Thread.";
/// Printed after `HELP` by `--help`.
const AGENT_HELP: &str = "
Agents and scripts: brn --json --data-dir DIR COMMAND ... prints one JSON envelope on stdout.
  In --json mode: exit 0 ok, 1 internal, 2 usage, 3 stale, 4 deferred, 5 needs review, 6 rejected,
  7 failed (including busy and timeout), 130 interrupted (SIGINT), 143 terminated (SIGTERM).
  Without --json, failures exit 1 and outcomes such as Stale exit 0.
  --timeout SECONDS after --data-dir bounds the call; it never hides a commit (envelope stop).
  brn --json --help prints the machine contract (commands, options, outcomes, retry classes, error codes).
  Bounded inspection: history|candidates|records --limit N [--cursor C], revisions --note ID,
  revision --note ID --version V, runs [--thread ID] [--state STATE], read --id ID --limit BYTES;
  pass result.cursor back as --cursor for the next page. operation --id OP | --key K --command NAME
  shows whether an operation (or a keyed command's key) applied, without using the key.
  Text options --title, --text, --prompt, --instructions, --actions also accept --NAME-file ABSOLUTE_FILE or --NAME-file - (stdin).
  --key KEY makes note-create, thread-start, undo, queue, archive|resolve, unarchive, keep-in-brn,
  intake --intent full|chat|project and project commands replay on retry. resolve is an alias of archive.
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
            ))
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
                })
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
    if iter.next().map(|(_, token)| token).as_deref() != Some("--data-dir") {
        let message = "Supply --data-dir ABSOLUTE_DIRECTORY; run --help";
        return Err(refuse(None, usage(Code::MissingDataDir, message)));
    }
    let Some((_, data)) = iter.next() else {
        return Err(refuse(
            None,
            usage(Code::MissingDataDir, "Missing data directory"),
        ));
    };
    let data = PathBuf::from(data);
    if !data.is_absolute() {
        let message = "Data directory must be absolute";
        return Err(refuse(None, usage(Code::InvalidDataDir, message)));
    }
    let mut timeout = None;
    let mut next = iter.next();
    while next.as_ref().is_some_and(|(_, token)| token == "--timeout") {
        if timeout.is_some() {
            let error = usage(Code::RepeatedOption, "Repeated option").with_option("timeout");
            return Err(refuse(None, error));
        }
        let Some((_, value)) = iter.next() else {
            let error = usage(Code::MissingValue, "Missing option value").with_option("timeout");
            return Err(refuse(None, error));
        };
        timeout = Some(timeout_seconds(&value).map_err(|error| refuse(None, error))?);
        next = iter.next();
    }
    let Some((_, command)) = next else {
        return Err(refuse(None, usage(Code::MissingCommand, "Missing command")));
    };
    let Some(spec) = spec::spec(&command) else {
        let mut error = usage(Code::UnknownCommand, "Unknown command; run --help");
        if let Some(command) = contract::echo(&command) {
            error = error.with("command", command);
        }
        return Err(refuse(None, error));
    };
    let refuse = |error: CliError| refuse(Some(spec.name), error);
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    while let Some((index, token)) = iter.next() {
        let Some(name) = token.strip_prefix("--") else {
            let error = usage(Code::MalformedOption, "Options need --name VALUE");
            return Err(refuse(error.with("position", index + 1)));
        };
        let Some((_, value)) = iter.next() else {
            let error = usage(Code::MissingValue, "Missing option value");
            return Err(refuse(error.with_option(name)));
        };
        if options.insert(name.into(), value).is_some() {
            let error = usage(Code::RepeatedOption, "Repeated option");
            return Err(refuse(error.with_option(name)));
        }
    }
    if let Some(unknown) = options
        .keys()
        .find(|key| !spec.options.contains(&key.as_str()))
    {
        let error = usage(Code::UnknownOption, "Unknown option for command")
            .with_option(unknown)
            .with("allowed", spec.options);
        return Err(refuse(error));
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
    let given = |name: &str| {
        options.contains_key(name) || pending_inputs.iter().any(|(pending, _)| *pending == name)
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
    if command == "delegation" {
        delegation_choice(&options["choice"]).map_err(refuse)?;
    }
    Ok(Some(Invocation {
        data,
        command,
        args: options,
        machine,
        key,
        pending_inputs,
        timeout,
    }))
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
    /// The caller's key for the shared keyed functions, or a fresh one.
    fn owner_key(&self) -> String {
        match &self.key {
            Some(_) => self.host_key("cli"),
            None => uuid::Uuid::new_v4().to_string(),
        }
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
            return Err(CliError::new(Code::InvalidRequest, message).with("option", "project"));
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
        return Err(CliError::new(Code::InvalidRequest, message).with("option", "thread"));
    }
    Ok(Some(chat.clone()))
}
fn encode(value: impl serde::Serialize) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}
fn settings_output(settings: WorkspaceSettings) -> Result<Value> {
    let chosen = settings.delegation_chosen;
    let mut output = encode(settings)?;
    // Storage omits false for old request identity; CLI status is explicit.
    output["delegation_chosen"] = json!(chosen);
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
async fn ask(
    workspace: &mut Workspace,
    inv: &Invocation,
    thread: String,
    prompt: String,
    sources: Vec<brn_ai::threads::TransientSource>,
) -> Result<Value> {
    let settings = workspace.settings()?;
    let budget = inv
        .args
        .get("budget")
        .map(|n| n.parse::<u16>())
        .transpose()
        .map_err(|_| CliError::invalid_value("budget", "Invalid budget"))?
        .unwrap_or(brn_ai::threads::default_tool_rounds(!sources.is_empty()));
    // Without a handle the search tools are not offered; the reason is reported.
    let (search, search_status) = match workspace.search_handle() {
        Ok(handle) => (
            Some(Arc::new(handle) as Arc<dyn brn_ai::threads::RunNoteSearch>),
            "available".to_owned(),
        ),
        Err(error) => (None, format!("unavailable: {error}")),
    };
    stop::check()?;
    let cancel = CancellationToken::new();
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
    let result =
        brn_ai::threads::run(
            brn_ai::threads::ThreadRunRequest {
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
                        _ => return Err(CliError::conflicting(
                            &["edit-note", "record-decision"],
                            "Choose either --edit-note ID --base VERSION or --record-decision yes",
                        )),
                    }
                } else {
                    None
                },
                data_dir: inv.data.clone(),
                thread,
                prompt,
                selection: Selection {
                    provider: provider(&settings.provider)?,
                    model: settings.model,
                },
                effort: effort(&settings.effort)?,
                max_tool_rounds: budget,
                auth_file: settings.auth_file.map(PathBuf::from),
                credentials_dir: settings.credentials_dir.map(PathBuf::from),
                sources,
                search,
                trace: trace_level(inv)?,
            },
            cancel,
            Arc::new(|_| {}),
        )
        .await;
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
fn converter(kind: DocumentKind) -> Result<Converter> {
    let executable = std::env::current_exe()?;
    let helper = std::env::var_os("BRN_INTAKE_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| executable.with_file_name("brn-intake-helper"));
    let tools = PdfTools::discover();
    Ok(Converter {
        helper_path: helper,
        pdf_tools: tools,
        temp_root: std::env::temp_dir()
            .canonicalize()?
            .join("brn-threads-intake"),
        limits: brn_threads_intake::import_limits(kind),
    })
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
        CliError::new(Code::InvalidRequest, message)
    }
}
async fn execute(inv: &Invocation, notes: &mut Annotations) -> Result<Value> {
    if CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::interrupted());
    }
    if inv.command == "restore" {
        let store = Store::restore(inv.arg("source")?, &inv.data)?;
        return Ok(json!({"restored":store.directory()}));
    }
    if inv.command == "model-install" {
        return install_model(inv);
    }
    if inv.command == "upgrade" {
        Store::upgrade(&inv.data, Path::new(inv.arg("backup-directory")?))?;
        return Ok(json!({"upgraded":true,"data":inv.data}));
    }
    let mut workspace = Workspace::open(&inv.data)?;
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
                            ))
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
                    preview: serde_json::from_slice(
                        &std::fs::read(inv.arg("preview-file")?).map_err(|e| {
                            CliError::invalid_input("preview-file", "unreadable", e.to_string())
                        })?,
                    )
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
            encode(workspace.project_command(&key, command)?)?
        }
        "records" if inspect::page_mode(inv) => inspect::records(&workspace, inv, notes)?,
        "records" => encode(workspace.records()?)?,
        "notes" => encode(workspace.notes()?)?,
        "threads" => encode(workspace.threads()?)?,
        "needs-you" => encode(workspace.needs_you()?)?,
        "history" if inspect::page_mode(inv) => inspect::history(&workspace, inv, notes)?,
        "history" => encode(workspace.history()?)?,
        "candidates" if inspect::page_mode(inv) => inspect::candidates(&workspace, inv, notes)?,
        "candidates" => encode(workspace.candidates()?)?,
        "revisions" => inspect::revisions(&workspace, inv, notes)?,
        "revision" => inspect::revision(&workspace, inv, notes)?,
        "runs" => inspect::runs(&workspace, inv, notes)?,
        "operation" => inspect::operation(&workspace, inv)?,
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
            let (note, outcome) = workspace.create_topics_note()?;
            json!({"note":note,"pointer":applied(outcome)})
        }
        "topics-choose" => applied(workspace.choose_topics_note(inv.arg("note")?)?),
        "knowledge-check" => {
            let check = workspace.knowledge_check()?;
            let chat = match inv.args.get("seed-chat").map(String::as_str) {
                Some("yes") => Some(workspace.seed_knowledge_review(&check)?),
                None => None,
                Some(_) => {
                    return Err(CliError::invalid_value(
                        "seed-chat",
                        "--seed-chat takes yes",
                    ))
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
            notes.page = Some(
                json!({"limit": limit, "shown": shown, "cursor": page.cursor, "total": page.total}),
            );
            let mut output = encode(page)?;
            output["shown"] = json!(shown);
            output
        }
        "begin-edit" => encode(
            workspace
                .store
                .begin_edit(inv.arg("note")?, inv.number("base")?)
                .map_err(|e| contract::begin_edit_error(&workspace.store, &inv.args["note"], e))?,
        )?,
        "update-buffer" => match workspace.store.update_buffer(
            inv.arg("session")?,
            inv.number("generation")?,
            inv.arg("text")?,
        )? {
            BufferOutcome::Updated(session) => json!({"outcome":"Updated","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "save" => match workspace
            .store
            .save(inv.arg("session")?, inv.number("generation")?)?
        {
            SaveOutcome::Saved(receipt) => json!({"outcome":"Saved","receipt":receipt}),
            SaveOutcome::Stale(session) => json!({"outcome":"Stale","session":session}),
            SaveOutcome::GenerationMismatch(session) => {
                json!({"outcome":"GenerationMismatch","session":session})
            }
            SaveOutcome::Closed(session) => json!({"outcome":"Closed","session":session}),
            SaveOutcome::Deferred { guarded } => json!({"outcome":"Deferred","guarded":guarded}),
        },
        "discard" => match workspace
            .store
            .discard(inv.arg("session")?, inv.number("generation")?)?
        {
            BufferOutcome::Updated(session) => json!({"outcome":"Discarded","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "recovery" => encode(workspace.store.recovery_buffers()?)?,
        "apply" => applied(
            workspace
                .review_apply(inv.arg("operation")?)
                .map_err(|e| review_error(&workspace, inv, "applied", e))?,
        ),
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
        "resolve" | "archive" => {
            encode(workspace.archive_chat(&inv.host_key("cli-archive"), inv.arg("thread")?)?)?
        }
        "unarchive" => {
            encode(workspace.unarchive_chat(&inv.host_key("cli-unarchive"), inv.arg("thread")?)?)?
        }
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
            let source = Path::new(inv.arg("source")?);
            if !source.is_absolute() {
                return Err(CliError::invalid_value(
                    "source",
                    "Source path must be absolute",
                ));
            }
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
                    ))
                }
            };
            let intent = match inv.arg("intent")? {
                "useful" => ImportIntent::UsefulInformation,
                "full" | "chat" | "project" => ImportIntent::FullNote,
                _ => {
                    return Err(CliError::invalid_value(
                        "intent",
                        "Choose useful, full, or chat with --thread",
                    ))
                }
            };
            let settings = workspace.settings()?;
            let conversion_job = brn_ai::conversion::ConversionJob {
                converter: converter(kind)?,
                kind,
                intent,
                bytes,
                source_reference: inv.arg("source")?.into(),
                inventory: None,
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
            let result = brn_ai::conversion::convert(
                conversion_job,
                cancel,
                Arc::new(AtomicBool::new(CANCEL.load(Ordering::SeqCst))),
                Arc::new(|progress| contract::emit_err(&format!("{}\n", progress.label()))),
            )
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
                let value = applied(workspace.attach_full(
                    &inv.host_key("cli-attach"),
                    scope,
                    title,
                    &converted,
                )?);
                contract::fill_conflicts(&workspace, inv, &value, notes);
                return Ok(value);
            }
            if intent == ImportIntent::FullNote {
                applied(workspace.import_full(&inv.host_key("cli-import"), title, &converted)?)
            } else {
                let thread = if let Some(id) = inv.args.get("thread") {
                    id.clone()
                } else {
                    workspace
                        .new_thread("Useful information intake")
                        .map_err(|e| e.to_string())?
                        .id
                };
                let report = brn_threads_app::import_report(&converted);
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
                return ask(&mut workspace,inv,thread,"Retain useful information from the supplied source. Reconcile current notes and suggested Actions, preserve source references and meaningful selected figures using read_source's host asset links, retain detected omissions, leave raw text external, and ask for review of protected decisions.".into(),vec![source]).await;
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
