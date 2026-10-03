use brn_store::Approval;
use brn_workflow::{Config, SearchProfile, Workspace};
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode, sync::atomic::AtomicBool};
use uuid::Uuid;
const HELP: &str = "BRN legacy local workflow\nUsage: brn-flow COMMAND --data-dir ABSOLUTE_EXISTING_DIRECTORY [OPTIONS]\nCommands: import --file PATH [--approve yes|no]; sources; approve --source UUID --version UUID --state approved|draft|withdrawn; build [--model-dir DIR]; search --query TEXT [--profile keyword|semantic|hybrid]; sessions; history --session UUID\nLegacy ask is retired; use brn with a separate simple workspace for new chat. Native semantic/hybrid requires native-retrieval and a verified local model directory. Imported originals are never modified. --approve yes permits search, not publication. Data directories must already exist. Output is JSON. No executable or credential configuration is accepted.";
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("run --help for usage")?;
    if command == "--help" && args.next().is_none() {
        println!("{HELP}");
        return Ok(());
    }
    let allowed: &[&str] = match command.as_str() {
        "import" => &["file", "approve", "operation"],
        "sources" | "sessions" | "build" => &[],
        "approve" => &["source", "version", "state", "operation"],
        "search" => &["query", "profile"],
        "ask" => &["query", "profile", "session", "operation"],
        "history" => &["session"],
        _ => return Err("unknown command; run --help".into()),
    };
    let mut values = BTreeMap::new();
    while let Some(key) = args.next() {
        let name = key.strip_prefix("--").ok_or("options must start with --")?;
        if !allowed.contains(&name) && !["data-dir", "model-dir"].contains(&name) {
            return Err(format!("unknown option: {key}"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?;
        if values.insert(name.to_string(), value).is_some() {
            return Err(format!("repeated option: {key}"));
        }
    }
    let get = |key: &str| {
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("--{key} is required"))
    };
    let id = |key: &str| -> Result<Uuid, String> {
        Uuid::parse_str(&get(key)?).map_err(|_| format!("invalid --{key} UUID"))
    };
    let op = || -> Result<Uuid, String> {
        values
            .get("operation")
            .map(|v| Uuid::parse_str(v).map_err(|_| "invalid operation UUID".into()))
            .unwrap_or_else(|| Ok(Uuid::new_v4()))
    };
    let profile = match values
        .get("profile")
        .map(String::as_str)
        .unwrap_or("keyword")
    {
        "keyword" => SearchProfile::Keyword,
        "semantic" => SearchProfile::Semantic,
        "hybrid" => SearchProfile::Hybrid,
        _ => return Err("invalid search profile".into()),
    };
    let config = Config {
        model_dir: values.get("model-dir").map(PathBuf::from),
    };
    for path in [&config.model_dir].into_iter().flatten() {
        if !path.is_absolute() {
            return Err("model paths must be absolute".into());
        }
    }
    if command == "ask" {
        return Err(
            "LEGACY_AI_RETIRED: legacy AI retired; use brn with a separate simple workspace".into(),
        );
    }
    let mut workspace = Workspace::open(&PathBuf::from(get("data-dir")?), config)?;
    let cancel = AtomicBool::new(false);
    let value = match command.as_str() {
        "import" => {
            let approval = match values.get("approve").map(String::as_str).unwrap_or("no") {
                "yes" => Approval::Approved,
                "no" => Approval::Draft,
                _ => return Err("--approve must be yes or no".into()),
            };
            serde_json::to_value(workspace.import_file(
                &cancel,
                op()?,
                &PathBuf::from(get("file")?),
                approval,
            )?)
            .map_err(|e| e.to_string())?
        }
        "sources" => {
            let (sources, states) = workspace.source_projection()?;
            serde_json::json!({"sources":sources,"source_states":states})
        }
        "approve" => {
            let state = match get("state")?.as_str() {
                "approved" => Approval::Approved,
                "draft" => Approval::Draft,
                "withdrawn" => Approval::Withdrawn,
                _ => return Err("invalid approval state".into()),
            };
            workspace.set_approval(&cancel, op()?, id("source")?, id("version")?, state)?;
            serde_json::json!({"updated":true})
        }
        "build" => {
            serde_json::json!({"generation":workspace.build_index(&cancel,|p|eprintln!("{p}"))?})
        }
        "search" => serde_json::to_value(workspace.search(&get("query")?, profile)?)
            .map_err(|e| e.to_string())?,
        "sessions" => serde_json::to_value(workspace.sessions()?).map_err(|e| e.to_string())?,
        "history" => {
            serde_json::to_value(workspace.history(id("session")?)?).map_err(|e| e.to_string())?
        }
        _ => unreachable!(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
