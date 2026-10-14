//! Thin host CLI over the same checked Threads service as the desktop.
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::*;
use brn_threads_intake::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
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
const HELP: &str = "BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND [--option VALUE]\n\nCommands: init, records, notes, threads, needs-you, history, candidates, settings,\n  projects [--trash yes]; project-create --title TITLE [--key KEY]\n  project-chat --project ID --title TITLE; project-document --project ID --title TITLE --text MARKDOWN\n  project-settings --project ID --base VERSION --title TITLE --instructions TEXT --knowledge yes|no\n  chat-move --thread ID [--project ID|none]; project-link|project-unlink --project ID --note ID\n  project-delete-preview --project ID (save JSON for review)\n  project-delete --preview-file FILE --actions JSON [--key KEY]\n  project-restore --project ID --base VERSION\n  Project commands accept --key KEY for durable retry. Actions JSON maps each unfinished Action ID to Keep or Cancel.\n  Search/connections accept --project ID; intake --intent project --project ID imports privately.\n  note-create --title TITLE --text MARKDOWN\n  thread-start --title TITLE\n  read --id RECORD\n  search --query TEXT [--limit 1..50] [--cursor NEXT] [--thread CHAT]\n  connections --note ID [--hops 1|2] [--sources yes] [--thread CHAT]\n  topics; topics-create; topics-choose --note ID\n  knowledge-check [--seed-chat yes]\n  model-install --destination ABSOLUTE_FRESH_FOLDER (native builds; downloads the pinned model)\n  Semantic search uses the model folder named by BRN_EMBEDDING_MODEL_DIR.\n  begin-edit --note ID --base VERSION\n  update-buffer --session ID --generation N --text MARKDOWN\n  save|discard --session ID --generation N; recovery\n  apply|dismiss --operation ID; undo --operation ID; archive|unarchive --thread ID\n  queue --thread ID --text TEXT; keep-in-brn --note ID --base VERSION\n  ask --thread ID --prompt TEXT [--budget 1..32] [--edit-note ID --base VERSION | --record-decision yes] [--trace summary|full]\n  intake --source ABSOLUTE_FILE --intent useful|full|chat|project [--title TITLE] [--thread ID] [--project ID] [--trace summary|full]\n  configure --provider Codex|Copilot --model ID --effort low|medium|high\n  delegation --choice maintain|review-first\n  pause; resume; export|backup --destination ABSOLUTE_FRESH_PATH\n  restore --source ABSOLUTE_BACKUP\n  upgrade --backup-directory ABSOLUTE_EXTERNAL_FOLDER\n\nChoose delegation before resuming maintenance. Manual work remains available while pending.\nFresh explicit data only. Originals remain external; no external messages are sent.\nReview apply and Undo are concrete owner actions. Continue uses ask on the same Thread.";
type Result<T> = std::result::Result<T, String>;
struct Invocation {
    data: PathBuf,
    command: String,
    args: BTreeMap<String, String>,
}
fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Invocation>> {
    let args: Vec<_> = args.into_iter().collect();
    if args == ["--help"] {
        println!("{HELP}");
        return Ok(None);
    }
    let mut iter = args.into_iter();
    if iter.next().as_deref() != Some("--data-dir") {
        return Err("Supply --data-dir ABSOLUTE_DIRECTORY; run --help".into());
    }
    let data = PathBuf::from(iter.next().ok_or("Missing data directory")?);
    if !data.is_absolute() {
        return Err("Data directory must be absolute".into());
    }
    let command = iter.next().ok_or("Missing command")?;
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    while let Some(key) = iter.next() {
        let Some(key) = key.strip_prefix("--") else {
            return Err("Options need --name VALUE".into());
        };
        let value = iter.next().ok_or("Missing option value")?;
        if options.insert(key.into(), value).is_some() {
            return Err("Repeated option".into());
        }
    }
    let allowed: &[&str] = match command.as_str() {
        "init" | "records" | "notes" | "threads" | "needs-you" | "history" | "candidates"
        | "settings" | "pause" | "resume" | "recovery" | "topics" | "topics-create" => &[],
        "projects" => &["trash"],
        "project-create" => &["title", "key"],
        "project-chat" => &["project", "title", "key"],
        "project-document" => &["project", "title", "text", "key"],
        "project-settings" => &[
            "project",
            "base",
            "title",
            "instructions",
            "knowledge",
            "key",
        ],
        "chat-move" => &["thread", "project", "key"],
        "project-link" | "project-unlink" => &["project", "note", "key"],
        "project-delete-preview" => &["project"],
        "project-delete" => &["preview-file", "actions", "key"],
        "project-restore" => &["project", "base", "key"],
        "topics-choose" => &["note"],
        "connections" => &["note", "hops", "sources", "thread", "project"],
        "knowledge-check" => &["seed-chat"],
        "model-install" => &["destination"],
        "note-create" => &["title", "text"],
        "thread-start" => &["title"],
        "delegation" => &["choice"],
        "read" => &["id"],
        "search" => &["query", "limit", "cursor", "thread", "project"],
        "begin-edit" => &["note", "base"],
        "update-buffer" => &["session", "generation", "text"],
        "save" | "discard" => &["session", "generation"],
        "apply" | "undo" | "dismiss" => &["operation"],
        "resolve" | "archive" | "unarchive" => &["thread"],
        "keep-in-brn" => &["note", "base"],
        "queue" => &["thread", "text"],
        "ask" => &[
            "thread",
            "prompt",
            "budget",
            "edit-note",
            "base",
            "record-decision",
            "trace",
        ],
        "intake" => &["source", "intent", "title", "thread", "project", "trace"],
        "configure" => &[
            "provider",
            "model",
            "effort",
            "auth-file",
            "credentials-dir",
        ],
        "export" | "backup" => &["destination"],
        "restore" => &["source"],
        "upgrade" => &["backup-directory"],
        _ => return Err("Unknown command; run --help".into()),
    };
    if options.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("Unknown option for command".into());
    }
    if command == "delegation" {
        delegation_choice(options.get("choice").ok_or("Missing --choice")?)?;
    }
    Ok(Some(Invocation {
        data,
        command,
        args: options,
    }))
}
impl Invocation {
    fn arg(&self, key: &str) -> Result<&str> {
        self.args
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("Missing --{key}"))
    }
    fn number(&self, key: &str) -> Result<u64> {
        self.arg(key)?
            .parse()
            .map_err(|_| format!("--{key} must be an integer"))
    }
}
/// The explicit owner command that downloads the pinned embedding model.
#[cfg(feature = "native-retrieval")]
fn install_model(inv: &Invocation) -> Result<Value> {
    let target = PathBuf::from(inv.arg("destination")?);
    let bytes = install_embedding_model(&target, &CANCEL, |done, total| {
        eprint!("\rmodel {done}/{total} bytes");
    })
    .map_err(|e| e.to_string())?;
    eprintln!();
    Ok(json!({"installed":target,"downloaded_bytes":bytes,"use_with":EMBEDDING_MODEL_ENV}))
}
#[cfg(not(feature = "native-retrieval"))]
fn install_model(_: &Invocation) -> Result<Value> {
    Err("This build has no native retrieval; build with --features native-retrieval".into())
}
/// `--trace summary|full` records the run's tool use (structure only; `full`
/// adds query text and is for synthetic evaluation data).
fn trace_level(inv: &Invocation) -> Result<brn_ai::threads::TraceLevel> {
    match inv.args.get("trace").map(String::as_str) {
        None => Ok(brn_ai::threads::TraceLevel::Off),
        Some("summary") => Ok(brn_ai::threads::TraceLevel::Summary),
        Some("full") => Ok(brn_ai::threads::TraceLevel::Full),
        Some(_) => Err("--trace takes summary or full".into()),
    }
}
/// The owner's chosen chat context (`--thread`), which must name a chat.
fn chat_option(workspace: &Workspace, inv: &Invocation) -> Result<Option<String>> {
    if let Some(project) = inv.args.get("project") {
        if inv.args.contains_key("thread") {
            return Err("Choose either --project or --thread".into());
        }
        if !workspace
            .store
            .record(project)
            .map_err(|e| e.to_string())?
            .is_some_and(|r| !r.archived && matches!(r.data, RecordData::Project(_)))
        {
            return Err("--project must name an active project".into());
        }
        return Ok(Some(project.clone()));
    }
    let Some(chat) = inv.args.get("thread") else {
        return Ok(None);
    };
    let is_chat = workspace
        .store
        .record(chat)
        .map_err(|e| e.to_string())?
        .is_some_and(|r| matches!(r.data, RecordData::Thread(_)));
    if !is_chat {
        return Err("--thread must name a chat".into());
    }
    Ok(Some(chat.clone()))
}
fn encode(value: impl serde::Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| e.to_string())
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
        _ => Err("Delegation choice must be maintain or review-first".into()),
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
        _ => Err("Effort must be low, medium or high".into()),
    }
}
fn provider(text: &str) -> Result<Provider> {
    match text.to_lowercase().as_str() {
        "codex" | "chatgpt" => Ok(Provider::Chatgpt),
        "copilot" => Ok(Provider::Copilot),
        _ => Err("Provider must be Codex or Copilot".into()),
    }
}
async fn ask(
    workspace: &mut Workspace,
    inv: &Invocation,
    thread: String,
    prompt: String,
    sources: Vec<brn_ai::threads::TransientSource>,
) -> Result<Value> {
    let settings = workspace.settings().map_err(|e| e.to_string())?;
    let budget = inv
        .args
        .get("budget")
        .map(|n| n.parse::<u16>())
        .transpose()
        .map_err(|_| "Invalid budget")?
        .unwrap_or(brn_ai::threads::default_tool_rounds(!sources.is_empty()));
    // Without a handle the search tools are not offered; the reason is reported.
    let (search, search_status) = match workspace.search_handle() {
        Ok(handle) => (
            Some(Arc::new(handle) as Arc<dyn brn_ai::threads::RunNoteSearch>),
            "available".to_owned(),
        ),
        Err(error) => (None, format!("unavailable: {error}")),
    };
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
                        _ => return Err(
                            "Choose either --edit-note ID --base VERSION or --record-decision yes"
                                .into(),
                        ),
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
    let result = result.map_err(|e| format!("Provider run failed: {:?}", e.kind))?;
    let terminal = match result.answer.terminal {
        AiTerminal::Completed => "Completed".into(),
        AiTerminal::Interrupted => "Interrupted".into(),
        AiTerminal::Failed(error) => format!("Failed:{:?}", error.kind),
    };
    let mut output = json!({"run":result.run_id,"terminal":terminal,"answer":result.answer.text,"search":search_status});
    if let Some(trace) = result.trace {
        output["trace"] = encode(trace)?;
    }
    Ok(output)
}
fn converter(kind: DocumentKind) -> Result<Converter> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = std::env::var_os("BRN_INTAKE_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| executable.with_file_name("brn-intake-helper"));
    let tools = PdfTools::discover();
    Ok(Converter {
        helper_path: helper,
        pdf_tools: tools,
        temp_root: std::env::temp_dir()
            .canonicalize()
            .map_err(|e| e.to_string())?
            .join("brn-threads-intake"),
        limits: brn_threads_intake::import_limits(kind),
    })
}
async fn execute(inv: &Invocation) -> Result<Value> {
    if CANCEL.load(Ordering::SeqCst) {
        return Err("Interrupted before operation".into());
    }
    if inv.command == "restore" {
        let store = Store::restore(inv.arg("source")?, &inv.data).map_err(|e| e.to_string())?;
        return Ok(json!({"restored":store.directory()}));
    }
    if inv.command == "model-install" {
        return install_model(inv);
    }
    if inv.command == "upgrade" {
        Store::upgrade(&inv.data, Path::new(inv.arg("backup-directory")?))
            .map_err(|e| e.to_string())?;
        return Ok(json!({"upgraded":true,"data":inv.data}));
    }
    let mut workspace = Workspace::open(&inv.data).map_err(|e| e.to_string())?;
    // A named but unusable model folder is an error, never a silent fallback.
    if matches!(inv.command.as_str(), "search" | "ask" | "intake") {
        if let Some(Err(error)) = workspace.embedding_model_from_env() {
            return Err(format!("Semantic search is unavailable: {error}"));
        }
    }
    let result = match inv.command.as_str() {
        "init" => {
            json!({"ready":true,"data":inv.data,"settings":settings_output(workspace.settings().map_err(|e|e.to_string())?)?})
        }
        "projects" => encode(
            workspace
                .projects(inv.args.get("trash").is_some_and(|v| v == "yes"))
                .map_err(|e| e.to_string())?,
        )?,
        "project-delete-preview" => encode(
            workspace
                .project_delete_preview(inv.arg("project")?)
                .map_err(|e| e.to_string())?,
        )?,
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
                        _ => return Err("--knowledge takes yes|no".into()),
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
                        &std::fs::read(inv.arg("preview-file")?).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?,
                    actions: serde_json::from_str(inv.arg("actions")?)
                        .map_err(|e| e.to_string())?,
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
            encode(
                workspace
                    .project_command(&key, command)
                    .map_err(|e| e.to_string())?,
            )?
        }
        "records" => encode(workspace.records().map_err(|e| e.to_string())?)?,
        "notes" => encode(workspace.notes().map_err(|e| e.to_string())?)?,
        "threads" => encode(workspace.threads().map_err(|e| e.to_string())?)?,
        "needs-you" => encode(workspace.needs_you().map_err(|e| e.to_string())?)?,
        "history" => encode(workspace.history().map_err(|e| e.to_string())?)?,
        "candidates" => encode(workspace.candidates().map_err(|e| e.to_string())?)?,
        "settings" => settings_output(workspace.settings().map_err(|e| e.to_string())?)?,
        "delegation" => applied(
            workspace
                .choose_delegation(delegation_choice(inv.arg("choice")?)?)
                .map_err(|e| e.to_string())?,
        ),
        "read" => encode(
            workspace
                .store
                .record(inv.arg("id")?)
                .map_err(|e| e.to_string())?,
        )?,
        "note-create" => encode(
            workspace
                .new_note(inv.arg("title")?, inv.arg("text")?)
                .map_err(|e| e.to_string())?,
        )?,
        "thread-start" => encode(
            workspace
                .new_thread(inv.arg("title")?)
                .map_err(|e| e.to_string())?,
        )?,
        "topics" => encode(workspace.topics().map_err(|e| e.to_string())?)?,
        "topics-create" => {
            let (note, outcome) = workspace.create_topics_note().map_err(|e| e.to_string())?;
            json!({"note":note,"pointer":applied(outcome)})
        }
        "topics-choose" => applied(
            workspace
                .choose_topics_note(inv.arg("note")?)
                .map_err(|e| e.to_string())?,
        ),
        "knowledge-check" => {
            let check = workspace.knowledge_check().map_err(|e| e.to_string())?;
            let chat = match inv.args.get("seed-chat").map(String::as_str) {
                Some("yes") => Some(
                    workspace
                        .seed_knowledge_review(&check)
                        .map_err(|e| e.to_string())?,
                ),
                None => None,
                Some(_) => return Err("--seed-chat takes yes".into()),
            };
            let mut output = encode(check)?;
            output["review_chat"] = json!(chat);
            output
        }
        "connections" => {
            let hops = match inv.args.get("hops") {
                Some(_) => u8::try_from(inv.number("hops")?).map_err(|e| e.to_string())?,
                None => 1,
            };
            let sources = match inv.args.get("sources").map(String::as_str) {
                Some("yes") => true,
                None => false,
                Some(_) => return Err("--sources takes yes".into()),
            };
            let chat = chat_option(&workspace, inv)?;
            let handle = workspace.search_handle().map_err(|e| e.to_string())?;
            let found = handle
                .connections(inv.arg("note")?, chat.as_deref(), hops, sources)
                .map_err(|e| e.to_string())?;
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
                Some(_) => usize::try_from(inv.number("limit")?).map_err(|e| e.to_string())?,
                None => 12,
            };
            let chat = chat_option(&workspace, inv)?;
            let page = workspace
                .search_page(
                    inv.arg("query")?,
                    chat.as_deref(),
                    limit,
                    inv.args.get("cursor").map(String::as_str),
                )
                .map_err(|e| e.to_string())?;
            let shown = page.hits.len();
            let mut output = encode(page)?;
            output["shown"] = json!(shown);
            output
        }
        "begin-edit" => encode(
            workspace
                .store
                .begin_edit(inv.arg("note")?, inv.number("base")?)
                .map_err(|e| e.to_string())?,
        )?,
        "update-buffer" => match workspace
            .store
            .update_buffer(
                inv.arg("session")?,
                inv.number("generation")?,
                inv.arg("text")?,
            )
            .map_err(|e| e.to_string())?
        {
            BufferOutcome::Updated(session) => json!({"outcome":"Updated","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "save" => match workspace
            .store
            .save(inv.arg("session")?, inv.number("generation")?)
            .map_err(|e| e.to_string())?
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
            .discard(inv.arg("session")?, inv.number("generation")?)
            .map_err(|e| e.to_string())?
        {
            BufferOutcome::Updated(session) => json!({"outcome":"Discarded","session":session}),
            BufferOutcome::Ignored(session) => json!({"outcome":"Ignored","session":session}),
        },
        "recovery" => encode(
            workspace
                .store
                .recovery_buffers()
                .map_err(|e| e.to_string())?,
        )?,
        "apply" => applied(
            workspace
                .review_apply(inv.arg("operation")?)
                .map_err(|e| e.to_string())?,
        ),
        "dismiss" => {
            workspace
                .review_dismiss(inv.arg("operation")?)
                .map_err(|e| e.to_string())?;
            json!({"outcome":"Dismissed","operation":inv.arg("operation")?})
        }
        "undo" => applied(
            workspace
                .undo(inv.arg("operation")?)
                .map_err(|e| e.to_string())?,
        ),
        "resolve" | "archive" => encode(
            workspace
                .archive_chat(
                    &format!("cli-archive:{}", uuid::Uuid::new_v4()),
                    inv.arg("thread")?,
                )
                .map_err(|e| e.to_string())?,
        )?,
        "unarchive" => encode(
            workspace
                .unarchive_chat(
                    &format!("cli-unarchive:{}", uuid::Uuid::new_v4()),
                    inv.arg("thread")?,
                )
                .map_err(|e| e.to_string())?,
        )?,
        "keep-in-brn" => applied(
            workspace
                .keep_in_brn(
                    &format!("cli-keep:{}", uuid::Uuid::new_v4()),
                    inv.arg("note")?,
                    inv.number("base")?,
                )
                .map_err(|e| e.to_string())?,
        ),
        "queue" => encode(
            workspace
                .queue_message(
                    &format!("cli-queue:{}", uuid::Uuid::new_v4()),
                    inv.arg("thread")?,
                    inv.arg("text")?,
                )
                .map_err(|e| e.to_string())?,
        )?,
        "pause" => applied(workspace.pause(true).map_err(|e| e.to_string())?),
        "resume" => applied(workspace.pause(false).map_err(|e| e.to_string())?),
        "configure" => {
            let mut settings = workspace.settings().map_err(|e| e.to_string())?;
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
            applied(workspace.configure(settings).map_err(|e| e.to_string())?)
        }
        "ask" => {
            return ask(
                &mut workspace,
                inv,
                inv.arg("thread")?.into(),
                inv.arg("prompt")?.into(),
                vec![],
            )
            .await;
        }
        "intake" => {
            let source = Path::new(inv.arg("source")?);
            if !source.is_absolute() {
                return Err("Source path must be absolute".into());
            }
            let bytes =
                brn_threads_intake::read_input_file(source, brn_intake::MAX_INPUT_BYTES, &CANCEL)
                    .map_err(|e| e.to_string())?;
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
                    return Err(
                        "Supported sources: Markdown, text, EML, PDF, DOCX, HTML, XLSX, PPTX"
                            .into(),
                    )
                }
            };
            let intent = match inv.arg("intent")? {
                "useful" => ImportIntent::UsefulInformation,
                "full" | "chat" | "project" => ImportIntent::FullNote,
                _ => return Err("Choose useful, full, or chat with --thread".into()),
            };
            let settings = workspace.settings().map_err(|e| e.to_string())?;
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
                Arc::new(|progress| eprintln!("{}", progress.label())),
            )
            .await;
            watcher.abort();
            let converted = result.map_err(|e| e.to_string())?;
            if CANCEL.load(Ordering::SeqCst) {
                return Err("Import stopped before saving converted content".into());
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
                return Ok(applied(
                    workspace
                        .attach_full(
                            &format!("cli-attach:{}", uuid::Uuid::new_v4()),
                            scope,
                            title,
                            &converted,
                        )
                        .map_err(|e| e.to_string())?,
                ));
            }
            if intent == ImportIntent::FullNote {
                applied(
                    workspace
                        .import_full(
                            &format!("cli-import:{}", uuid::Uuid::new_v4()),
                            title,
                            &converted,
                        )
                        .map_err(|e| e.to_string())?,
                )
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
                .map_err(|e| e.to_string())?,
        )?,
        "backup" => {
            workspace
                .backup(Path::new(inv.arg("destination")?))
                .map_err(|e| e.to_string())?;
            json!({"backup":inv.arg("destination")?})
        }
        _ => return Err("Unknown command".into()),
    };
    Ok(result)
}
#[tokio::main]
async fn main() -> ExitCode {
    // SAFETY: this signal handler only stores a lock-free atomic flag.
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
    }
    match parse(std::env::args().skip(1)) {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(inv)) => match execute(&inv).await {
            Ok(result) => {
                println!("{result}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
