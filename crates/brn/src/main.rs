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
const HELP:&str="BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND [--option VALUE]\n\nCommands: init, records, notes, threads, needs-you, history, candidates, settings,\n  note-create --title TITLE --text MARKDOWN\n  thread-start --title TITLE\n  read --id RECORD; search --query TEXT\n  begin-edit --note ID --base VERSION\n  update-buffer --session ID --generation N --text MARKDOWN\n  save|discard --session ID --generation N; recovery\n  apply|dismiss --operation ID; undo --operation ID; resolve --thread ID\n  ask --thread ID --prompt TEXT [--budget 1..32]\n  intake --source ABSOLUTE_FILE --intent useful|full [--title TITLE] [--thread ID]\n  configure --provider Codex|Copilot --model ID --effort low|medium|high\n  pause; resume; export|backup --destination ABSOLUTE_FRESH_PATH\n  restore --source ABSOLUTE_BACKUP\n\nFresh explicit data only. Originals remain external; no external messages are sent.\nReview apply and Undo are concrete owner actions. Continue uses ask on the same Thread.";
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
        | "settings" | "pause" | "resume" | "recovery" => &[],
        "note-create" => &["title", "text"],
        "thread-start" => &["title"],
        "read" => &["id"],
        "search" => &["query"],
        "begin-edit" => &["note", "base"],
        "update-buffer" => &["session", "generation", "text"],
        "save" | "discard" => &["session", "generation"],
        "apply" | "undo" | "dismiss" => &["operation"],
        "resolve" => &["thread"],
        "ask" => &["thread", "prompt", "budget"],
        "intake" => &["source", "intent", "title", "thread"],
        "configure" => &[
            "provider",
            "model",
            "effort",
            "auth-file",
            "credentials-dir",
        ],
        "export" | "backup" => &["destination"],
        "restore" => &["source"],
        _ => return Err("Unknown command; run --help".into()),
    };
    if options.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("Unknown option for command".into());
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
fn encode(value: impl serde::Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| e.to_string())
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
        .unwrap_or(8);
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
    let result = brn_ai::threads::run(
        brn_ai::threads::ThreadRunRequest {
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
    Ok(json!({"run":result.run_id,"terminal":terminal,"answer":result.answer.text}))
}
fn converter() -> Result<Converter> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = std::env::var_os("BRN_INTAKE_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| executable.with_file_name("brn-intake-helper"));
    let tools = match (
        std::env::var_os("BRN_PDFTOTEXT"),
        std::env::var_os("BRN_PDFIMAGES"),
    ) {
        (Some(text), Some(images)) => Some(PdfTools {
            pdftotext: text.into(),
            pdfimages: images.into(),
        }),
        _ => {
            let text = PathBuf::from("/opt/homebrew/bin/pdftotext");
            let images = PathBuf::from("/opt/homebrew/bin/pdfimages");
            if text.is_file() && images.is_file() {
                Some(PdfTools {
                    pdftotext: text,
                    pdfimages: images,
                })
            } else {
                None
            }
        }
    };
    Ok(Converter {
        helper_path: helper,
        pdf_tools: tools,
        temp_root: std::env::temp_dir()
            .canonicalize()
            .map_err(|e| e.to_string())?
            .join("brn-threads-intake"),
        limits: Default::default(),
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
    let mut workspace = Workspace::open(&inv.data).map_err(|e| e.to_string())?;
    let result = match inv.command.as_str() {
        "init" => {
            json!({"ready":true,"data":inv.data,"settings":workspace.settings().map_err(|e|e.to_string())?})
        }
        "records" => encode(workspace.records().map_err(|e| e.to_string())?)?,
        "notes" => encode(workspace.notes().map_err(|e| e.to_string())?)?,
        "threads" => encode(workspace.threads().map_err(|e| e.to_string())?)?,
        "needs-you" => encode(workspace.needs_you().map_err(|e| e.to_string())?)?,
        "history" => encode(workspace.history().map_err(|e| e.to_string())?)?,
        "candidates" => encode(workspace.candidates().map_err(|e| e.to_string())?)?,
        "settings" => encode(workspace.settings().map_err(|e| e.to_string())?)?,
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
        "search" => encode(
            workspace
                .search(inv.arg("query")?)
                .map_err(|e| e.to_string())?,
        )?,
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
        "resolve" => applied(
            workspace
                .resolve_thread(inv.arg("thread")?)
                .map_err(|e| e.to_string())?,
        ),
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
            .await
        }
        "intake" => {
            let source = Path::new(inv.arg("source")?);
            if !source.is_absolute() {
                return Err("Source path must be absolute".into());
            }
            let meta = std::fs::metadata(source).map_err(|e| e.to_string())?;
            if !meta.is_file() || meta.len() > brn_intake::MAX_INPUT_BYTES as u64 {
                return Err("Source exceeds input limit".into());
            }
            let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
            let kind = match source
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                .as_str()
            {
                "md" => DocumentKind::Markdown,
                "txt" => DocumentKind::Text,
                "eml" => DocumentKind::Eml,
                "pdf" => DocumentKind::Pdf,
                "docx" => DocumentKind::Docx,
                _ => return Err("Supported sources: Markdown, text, EML, PDF, DOCX".into()),
            };
            let intent = match inv.arg("intent")? {
                "useful" => ImportIntent::UsefulInformation,
                "full" => ImportIntent::FullNote,
                _ => return Err("Choose useful or full".into()),
            };
            let converted = converter()?
                .convert(
                    ConversionRequest {
                        kind,
                        intent,
                        bytes: &bytes,
                        source_reference: inv.arg("source")?,
                        inventory: None,
                    },
                    &CANCEL,
                )
                .map_err(|e| e.to_string())?;
            let title = inv
                .args
                .get("title")
                .map(String::as_str)
                .unwrap_or("Imported note");
            if intent == ImportIntent::FullNote {
                applied(
                    workspace
                        .import_full(
                            &format!(
                                "import:{}:{}",
                                inv.arg("source")?,
                                brn_intake::hex(&converted.source_sha256)
                            ),
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
                return ask(&mut workspace,inv,thread,"Retain useful information from the supplied source. Reconcile current notes and suggested Actions, preserve source references, leave raw text external, and ask for review of protected decisions.".into(),vec![brn_ai::threads::TransientSource{reference:converted.source_reference,text:converted.markdown}]).await;
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
