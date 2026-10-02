# Step 1: Rig spike

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Answer go/no-go: can Rig `0.43.0` log in, stream a chat with a tool call, and reuse credentials after restart for both ChatGPT and Copilot, in the same dependency graph as BRN's SQLite, embeddings and GUI crates?

**Architecture:** A standalone, throwaway binary in `experiments/rig-spike` with its own lockfile and no dependency on BRN crates. Its output is `experiments/rig-spike/FINDINGS.md` and an evidence entry, not reusable code.

**Tech Stack:** Rust `1.98.1`, `rig =0.43.0`, `rusqlite =0.40.2` (bundled), `fastembed =7.1.0`, `gpui-kit =0.6.6`, Tokio, reqwest.

**Spec:** [Simple Rig-based notes app](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md), sections 7 and 9; [roadmap](plan.md).

## Global Constraints

- Inherit the [roadmap](plan.md) constraints.
- Credentials go only in a folder outside the repository, created with mode `0700` (use `"$TMPDIR/brn-spike-creds"`). Never print tokens, never commit cache files, never paste device codes into evidence.
- Never copy raw error output, tokens, account IDs or email addresses into FINDINGS.md or evidence. Record an error category (auth, network, rate limit, model, other) and yes/no facts instead.
- Tasks 3 and 4 make live calls and need the user to enter device codes. Ask the user before starting them.
- Rig signatures below were read from the `v0.43.0` source. If one doesn't compile, adapt to the real signature and record the difference in FINDINGS.md; that is a finding, not a failure.

---

### Task 1: Dependency graph check

**Files:**
- Create: `experiments/rig-spike/Cargo.toml`
- Create: `experiments/rig-spike/src/main.rs`
- Create: `experiments/rig-spike/README.md`

**Interfaces:**
- Produces: `cargo run -- graph` prints the SQLite version and an FTS5 match count; the binary links rig, rusqlite, fastembed and gpui-kit together.

- [ ] **Step 1: Create the manifest**

```toml
[package]
name = "rig-spike"
version = "0.0.0"
edition = "2024"
rust-version = "1.98.1"
publish = false

[dependencies]
rig = { version = "=0.43.0", default-features = false, features = ["agent", "derive", "reqwest", "rustls"] }
rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled"] }
fastembed = { version = "=7.1.0", default-features = false, features = ["ort-download-binaries-rustls-tls", "hf-hub-rustls-tls"] }
gpui-kit = "=0.6.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
anyhow = "1"
thiserror = "2"
libc = "0.2"
base64 = "0.22"

[features]
cassette = ["rig/cassette"]
test-utils = ["rig/test-utils"]

[workspace]
```

- [ ] **Step 2: Create `src/main.rs` with only the graph check**

```rust
use anyhow::{Result, bail};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["graph"] => graph(),
        _ => bail!("usage: rig-spike graph"),
    }
}

fn graph() -> Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE t USING fts5(body, tokenize='unicode61 remove_diacritics 2');
         INSERT INTO t VALUES ('red blue'), ('reddish blueprint');",
    )?;
    let hits: i64 =
        conn.query_row("SELECT count(*) FROM t WHERE t MATCH '\"red\"'", [], |r| r.get(0))?;
    println!("sqlite {} fts5 hits for red: {hits}", rusqlite::version());
    let embed: fn(fastembed::TextInitOptions) -> _ = fastembed::TextEmbedding::try_new;
    std::hint::black_box(embed);
    let app: fn() -> _ = gpui_kit::application;
    std::hint::black_box(app);
    std::hint::black_box(rig::providers::chatgpt::PROVIDER_NAME);
    std::hint::black_box(rig::providers::copilot::PROVIDER_NAME);
    Ok(())
}
```

- [ ] **Step 3: Resolve and build**

Run: `cargo build --manifest-path experiments/rig-spike/Cargo.toml`
Expected: builds and creates `experiments/rig-spike/Cargo.lock`. If it fails, record the full error in FINDINGS.md (Task 5 format) and stop: this is a no-go to report to the user.

- [ ] **Step 4: Run the graph check**

Run: `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- graph`
Expected: `sqlite 3.x.y fts5 hits for red: 1` (token match, not `reddish`).

- [ ] **Step 5: Record the resolved graph**

Run:
```sh
cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -i libsqlite3-sys
cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -d --depth 0
```
Expected: exactly one `libsqlite3-sys`. Save both outputs for FINDINGS.md.

- [ ] **Step 6: Write the README and commit**

`experiments/rig-spike/README.md`:

```markdown
# Rig spike (throwaway)

Answers whether Rig 0.43.0 works for ChatGPT and Copilot subscriptions in BRN's dependency graph.
Not product code; nothing depends on it. Results: [FINDINGS.md](FINDINGS.md).

    cargo run --locked -- graph
    cargo run --locked -- login chatgpt|copilot CREDS_DIR
    cargo run --locked -- models chatgpt|copilot CREDS_DIR
    cargo run --locked -- chat chatgpt|copilot CREDS_DIR MODEL "PROMPT"
    cargo run --locked -- cancel chatgpt|copilot CREDS_DIR MODEL "PROMPT" MILLISECONDS

CREDS_DIR must be outside the repository; it is created with mode 0700.
```

```bash
git add experiments/rig-spike/Cargo.toml experiments/rig-spike/Cargo.lock experiments/rig-spike/src/main.rs experiments/rig-spike/README.md
git commit -m "test: add Rig spike dependency graph check

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Login, models and chat commands

**Files:**
- Modify: `experiments/rig-spike/src/main.rs`

**Interfaces:**
- Consumes: Task 1 manifest.
- Produces: `login`, `models`, `chat` and `cancel` subcommands as listed in the README.

- [ ] **Step 1: Replace `src/main.rs`**

```rust
use anyhow::{Context, Result, bail};
use rig::agent::stream_to_stdout;
use rig::prelude::*;
use rig::providers::{chatgpt, copilot};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

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
    let hits: i64 =
        conn.query_row("SELECT count(*) FROM t WHERE t MATCH '\"red\"'", [], |r| r.get(0))?;
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
        std::fs::DirBuilder::new().mode(0o700).recursive(true).create(dir)?;
    }
    let meta = std::fs::symlink_metadata(dir)?;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let me = unsafe { libc::geteuid() };
    if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o7777 != 0o700 {
        bail!("credentials folder must be a real folder owned by you with mode 700");
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
            println!("ChatGPT account id present: {}", context.account_id.is_some());
            println!("ChatGPT email (id_token claim): {:?}", chatgpt_email(creds)?);
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
        println!("cache file {} mode {mode:o}", entry.file_name().to_string_lossy());
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
    let claims: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?,
    )?;
    Ok(claims.get("email").and_then(|e| e.as_str()).map(str::to_owned))
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
            let client = chatgpt::new("").authenticate(&chatgpt_auth(creds, false)).await?;
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
                "x": { "type": "number", "description": "First number" },
                "y": { "type": "number", "description": "Second number" }
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
        Ok(args.x + args.y)
    }
}

macro_rules! run_agent {
    ($model:expr, $prompt:expr, $cancel_after:expr) => {{
        let agent = rig::AgentBuilder::new($model)
            .preamble("You are a calculator. Always use the add tool to add numbers.")
            .tool(Adder)
            .default_max_turns(2)
            .build();
        let mut stream = agent.prompt($prompt).stream();
        match $cancel_after {
            None => {
                let result = stream_to_stdout(&mut stream).await?;
                println!("\n[final] {}", result.output());
                println!("[usage] {:?}", result.usage());
            }
            Some(ms) => {
                let printed = tokio::time::timeout(
                    Duration::from_millis(ms),
                    stream_to_stdout(&mut stream),
                )
                .await;
                match printed {
                    Err(_) => println!("\n[cancelled after {ms} ms; stream dropped]"),
                    Ok(result) => {
                        result?;
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
```

- [ ] **Step 2: Build**

Run: `cargo build --manifest-path experiments/rig-spike/Cargo.toml --locked`
Expected: builds. Fix any signature differences against the pinned Rig source (`~/.cargo/registry/src/*/rig-core-0.43.0/`) and note each one for FINDINGS.md.

- [ ] **Step 3: Check that `chat` without credentials doesn't start a login**

Run: `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- chat chatgpt "$TMPDIR/brn-spike-empty" gpt-5.4 "hi"`
Expected: exits with an error containing `reconnect needed`, prints no device code and no verification link. Repeat with `copilot` and model `gpt-4o`.

- [ ] **Step 4: Commit**

```bash
git add experiments/rig-spike/src/main.rs
git commit -m "test: add Rig spike login and chat commands

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Live login (user present)

**Files:** none changed. Results go to Task 5.

- [ ] **Step 1: Ask the user to start live login.** Explain that two device-code logins follow and that the codes go to the terminal only.

- [ ] **Step 2: Log in to ChatGPT**

Run: `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- login chatgpt "$TMPDIR/brn-spike-creds"`
Expected: a verification link and code; after the user approves, `ChatGPT account id present: true`, `ChatGPT email (id_token claim): Some(...)` and the cache file modes. Record whether the account id and the email are available (yes/no, not their values) and whether `chatgpt.json` is `600` or wider. If no email is available, record it as a limitation: the app would then show "ChatGPT account" without a name.

- [ ] **Step 3: Log in to Copilot**

Run: `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- login copilot "$TMPDIR/brn-spike-creds"`
Expected: `GitHub login: <name>` and the modes of `github-token` and `copilot.json`.

- [ ] **Step 4: List models**

Run `models chatgpt` and `models copilot` with the same folder. Record which model IDs are available and whether `list_models` works for ChatGPT.

### Task 4: Live chat, tool call, restart and cancel (user present)

**Files:** none changed. Results go to Task 5.

- [ ] **Step 1: Streamed chat with a tool call, each provider**

Run:
```sh
cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- chat chatgpt "$TMPDIR/brn-spike-creds" gpt-5.4 "Use the add tool to add 2 and 3, then say the result."
cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- chat copilot "$TMPDIR/brn-spike-creds" gpt-4o "Use the add tool to add 2 and 3, then say the result."
```
Expected for each: `[tool add called with x=2 y=3]` on stderr, streamed text mentioning 5, `[final]` and `[usage]` lines. Each command is a new process, so a success also proves credential reuse after restart without a new login.

- [ ] **Step 2: Copilot with a newer model.** Repeat the Copilot command with a model from Task 3 step 4 that uses the Responses route (for example `gpt-5.3-codex` if listed). Record whether tool calls work there.

- [ ] **Step 3: Cancel mid-stream**

Run: `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- cancel chatgpt "$TMPDIR/brn-spike-creds" gpt-5.4 "Count slowly from 1 to 200, one number per line." 1500`
Expected: partial output, then `[cancelled after 1500 ms; stream dropped]`, and the process exits promptly. Record whether any text arrived before the cancel line. If none did, the timer fired before streaming started: rerun with 4000, then 8000. Repeat with `copilot`.

- [ ] **Step 4: Expired-session behaviour.** Delete only `copilot.json` (keep `github-token`) and rerun the Copilot chat. Expected: it succeeds by exchanging a new session token without a device login. Record the result.

### Task 5: Test-support features, findings and decision

**Files:**
- Create: `experiments/rig-spike/FINDINGS.md`
- Modify: `docs/work/active/simple-rig-notes/evidence.md`

- [ ] **Step 1: Check Rig's test support compiles in this graph**

Run:
```sh
cargo check --manifest-path experiments/rig-spike/Cargo.toml --features cassette
cargo check --manifest-path experiments/rig-spike/Cargo.toml --features test-utils
```
Expected: both pass; if `--locked` is rejected because features add packages, run without it and note that the lockfile changed. Read `rig-cassette`'s README in the registry source and note how replay is configured (needed for step 4's offline tests).

- [ ] **Step 2: Write FINDINGS.md** with these sections, filled with actual observations:

```markdown
# Rig spike findings

Date, commit, macOS version, Rust version.

## Dependency graph
- Build result, `libsqlite3-sys` versions (from `cargo tree -i`), notable duplicates.
- FTS5 check output.

## API differences from the plan
- Each signature that differed and what compiled instead.

## ChatGPT
- Login: worked / failed (error category only).
- Account identity: account id available yes/no; email claim available yes/no.
- Cache file and mode.
- Models listed; model used.
- Streamed tool call; restart reuse; cancel behaviour (did partial text arrive before cancel; which delay).

## Copilot
- Same fields, plus GitHub login lookup and the session-token refresh result (Task 4 step 4).
- Tool calls on chat-completions and on Responses-route models.

## Test support
- `cassette` and `test-utils` results; how replay is configured.

## Decision
- Go / no-go for step 4, with reasons and any required changes to the spec.
```

- [ ] **Step 3: Add the evidence entry.** Append to `docs/work/active/simple-rig-notes/evidence.md` a "Step 1: spike" section with the date, commit, the exact commands run, pass/fail per task and a link to FINDINGS.md.

- [ ] **Step 4: Check for secrets, then commit**

Run: `git diff --cached | grep -iE 'token|bearer|refresh|device' || true` after staging, and confirm nothing secret is present (mentions of the words in prose are fine; values are not).

```bash
git add experiments/rig-spike/FINDINGS.md docs/work/active/simple-rig-notes/evidence.md
git commit -m "docs: record Rig spike findings

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

- [ ] **Step 5: Report the go/no-go decision to the user** before step 4's plan is written.
