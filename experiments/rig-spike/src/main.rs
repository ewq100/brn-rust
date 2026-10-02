use anyhow::{Context, Result, bail};
use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, PromptResponse, StreamingResult};
use rig::providers::{chatgpt, copilot};
use rig::streaming::{Item, StreamEvent};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["graph"] => graph(),
        ["login", provider, creds] => login(provider, Path::new(creds)).await,
        ["models", provider, creds] => models(provider, Path::new(creds)).await,
        ["chat", provider, creds, model, prompt] => {
            chat(provider, Path::new(creds), model, prompt, None).await
        }
        ["cancel", provider, creds, model, prompt, ms] => {
            chat(provider, Path::new(creds), model, prompt, Some(ms.parse()?)).await
        }
        _ => bail!("see README.md for usage"),
    }
}

fn graph() -> Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE t USING fts5(body, tokenize='unicode61 remove_diacritics 2');
         INSERT INTO t VALUES ('red blue'), ('reddish blueprint');",
    )?;
    let hits: i64 = conn.query_row("SELECT count(*) FROM t WHERE t MATCH '\"red\"'", [], |r| {
        r.get(0)
    })?;
    println!("sqlite {} fts5 hits for red: {hits}", rusqlite::version());
    let embed: fn(fastembed::TextInitOptions) -> _ = fastembed::TextEmbedding::try_new;
    std::hint::black_box(embed);
    let app: fn() -> _ = gpui_kit::application;
    std::hint::black_box(app);
    Ok(())
}

fn prepare_creds(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    if std::fs::symlink_metadata(dir).is_err() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)?;
    }
    let meta = std::fs::symlink_metadata(dir)?;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let me = unsafe { libc::geteuid() };
    if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o7777 != 0o700 {
        bail!("credentials folder must be a real folder owned by you with mode 700");
    }
    let canonical_dir = dir.canonicalize()?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize()?;
    let repo_root = manifest_dir
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .or_else(|| manifest_dir.parent().and_then(Path::parent))
        .context("cannot determine repository root")?;
    if canonical_dir.starts_with(repo_root) {
        bail!("credentials folder must be outside the repository (including symlinked paths)");
    }
    Ok(())
}

fn chatgpt_auth(creds: &Path, allow_login: bool) -> chatgpt::auth::Authenticator {
    chatgpt::auth::Authenticator::new(
        chatgpt::auth::AuthSource::OAuth,
        Some(creds.join("chatgpt.json")),
        chatgpt::auth::DeviceCodeHandler::default(),
        allow_login,
    )
}

fn copilot_auth(creds: &Path, allow_login: bool) -> copilot::auth::Authenticator {
    copilot::auth::Authenticator::new(
        copilot::auth::AuthSource::OAuth,
        Some(creds.join("github-token")),
        Some(creds.join("copilot.json")),
        copilot::auth::DeviceCodeHandler::default(),
        allow_login,
    )
}

async fn login(provider: &str, creds: &Path) -> Result<()> {
    prepare_creds(creds)?;
    match provider {
        "chatgpt" => {
            let auth = chatgpt_auth(creds, true);
            let context = auth
                .auth_context(&rig::rig_reqwest::shared())
                .await
                .context("ChatGPT login failed")?;
            println!(
                "ChatGPT account id present: {}",
                context.account_id.is_some()
            );
            println!(
                "ChatGPT email (id_token claim): {:?}",
                chatgpt_email(creds)?
            );
        }
        "copilot" => {
            copilot::Copilot::new("")
                .authenticate(&copilot_auth(creds, true))
                .await
                .context("Copilot login failed")?;
            println!("GitHub login: {}", github_login(creds).await?);
        }
        _ => bail!("provider must be chatgpt or copilot"),
    }
    for entry in std::fs::read_dir(creds)? {
        use std::os::unix::fs::PermissionsExt;
        let entry = entry?;
        let mode = entry.metadata()?.permissions().mode() & 0o777;
        println!(
            "cache file {} mode {mode:o}",
            entry.file_name().to_string_lossy()
        );
    }
    Ok(())
}

/// The `email` claim of the cached ChatGPT `id_token` (a JWT), if present.
fn chatgpt_email(creds: &Path) -> Result<Option<String>> {
    use base64::Engine;
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(creds.join("chatgpt.json"))?)?;
    let Some(token) = record.get("id_token").and_then(|t| t.as_str()) else {
        return Ok(None);
    };
    let Some(payload) = token.split('.').nth(1) else {
        return Ok(None);
    };
    let claims: serde_json::Value =
        serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?)?;
    Ok(claims
        .get("email")
        .and_then(|e| e.as_str())
        .map(str::to_owned))
}

async fn github_login(creds: &Path) -> Result<String> {
    #[derive(Deserialize)]
    struct User {
        login: String,
    }
    let token = std::fs::read_to_string(creds.join("github-token"))?;
    let user: User = reqwest::Client::new()
        .get("https://api.github.com/user")
        .bearer_auth(token.trim())
        .header("User-Agent", "brn-rig-spike")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(user.login)
}

async fn models(provider: &str, creds: &Path) -> Result<()> {
    prepare_creds(creds)?;
    match provider {
        "chatgpt" => {
            let client = chatgpt::new("")
                .authenticate(&chatgpt_auth(creds, false))
                .await?;
            println!("{:#?}", client.list_models().await?);
        }
        "copilot" => {
            let client = copilot::Copilot::new("")
                .authenticate(&copilot_auth(creds, false))
                .await?;
            println!("{:#?}", client.list_models().await?);
        }
        _ => bail!("provider must be chatgpt or copilot"),
    }
    Ok(())
}

#[derive(Deserialize)]
struct AddArgs {
    x: i64,
    y: i64,
}

#[derive(Debug, thiserror::Error)]
#[error("add failed")]
struct AddError;

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = AddError;
    type Args = AddArgs;
    type Output = i64;

    fn description(&self) -> String {
        "Add x and y together".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer", "description": "First integer" },
                "y": { "type": "integer", "description": "Second integer" }
            },
            "required": ["x", "y"]
        })
    }

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        eprintln!("[tool add called with x={} y={}]", args.x, args.y);
        args.x.checked_add(args.y).ok_or(AddError)
    }
}

async fn print_stream(
    stream: &mut StreamingResult,
    text_bytes: &mut usize,
) -> Result<PromptResponse> {
    let mut final_response = None;
    while let Some(item) = stream.next().await {
        match item? {
            MultiTurnStreamItem::StreamAssistantItem(Item::Event(StreamEvent::Text {
                text,
                ..
            })) => {
                print!("{text}");
                std::io::stdout().flush()?;
                *text_bytes += text.len();
            }
            MultiTurnStreamItem::ToolCall { tool_call } => {
                eprintln!("[tool call: {}]", tool_call.function.name);
            }
            MultiTurnStreamItem::FinalResponse(response) => {
                final_response = Some(response);
            }
            MultiTurnStreamItem::ModelTurnRetried { turn } => {
                print!("\n[model turn {turn} rejected; retry requested]\n");
                std::io::stdout().flush()?;
            }
            _ => {}
        }
    }
    final_response.context("stream ended without a final response")
}

macro_rules! run_agent {
    ($model:expr, $prompt:expr, $cancel_after:expr) => {{
        let agent = rig::AgentBuilder::new($model)
            .preamble("You are a calculator. Always use the add tool to add numbers.")
            .tool(Adder)
            .default_max_turns(2)
            .build();
        let mut stream = agent.prompt($prompt).stream();
        let mut text_bytes = 0;
        match $cancel_after {
            None => {
                let result = print_stream(&mut stream, &mut text_bytes).await?;
                println!("\n[final] {}", result.output());
                println!("[usage] {:?}", result.usage());
            }
            Some(ms) => {
                let printed = tokio::time::timeout(
                    Duration::from_millis(ms),
                    print_stream(&mut stream, &mut text_bytes),
                )
                .await;
                match printed {
                    Err(_) => {
                        drop(stream);
                        println!("\n[cancelled after {ms} ms; stream dropped]");
                        println!(
                            "[partial text received: {}]",
                            if text_bytes > 0 { "yes" } else { "no" }
                        );
                    }
                    Ok(result) => {
                        let result = result?;
                        println!("\n[final] {}", result.output());
                        println!("[usage] {:?}", result.usage());
                        println!("\n[completed before cancel]");
                    }
                }
            }
        }
    }};
}

async fn chat(
    provider: &str,
    creds: &Path,
    model: &str,
    prompt: &str,
    cancel_after: Option<u64>,
) -> Result<()> {
    prepare_creds(creds)?;
    match provider {
        "chatgpt" => {
            let client = chatgpt::new("")
                .authenticate(&chatgpt_auth(creds, false))
                .await
                .context("reconnect needed: run login chatgpt")?;
            run_agent!(client.completion(model), prompt, cancel_after);
        }
        "copilot" => {
            let client = copilot::Copilot::new("")
                .authenticate(&copilot_auth(creds, false))
                .await
                .context("reconnect needed: run login copilot")?;
            run_agent!(client.completion(model), prompt, cancel_after);
        }
        _ => bail!("provider must be chatgpt or copilot"),
    }
    Ok(())
}
