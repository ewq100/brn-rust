use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct PersistedState {
    cwd: String,
    thread_id: Option<String>,
    turn_outcome: String,
}

impl PersistedState {
    fn create(path: &Path, cwd: &str) -> Result<Self, String> {
        let state = Self {
            cwd: cwd.into(),
            thread_id: None,
            turn_outcome: "pending".into(),
        };
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock error")?
            .as_nanos();
        let temp = path.with_extension(format!("tmp-{}-{stamp}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let result = (|| {
            let mut file = options
                .open(&temp)
                .map_err(|_| "state temp creation failed")?;
            file.write_all(state.json().to_string().as_bytes())
                .map_err(|_| "state write failed")?;
            file.sync_all().map_err(|_| "state sync failed")?;
            fs::hard_link(&temp, path)
                .map_err(|_| "state path already exists or cannot be created")?;
            Ok(())
        })();
        let _ = fs::remove_file(&temp);
        result.map(|()| state)
    }

    fn json(&self) -> Value {
        json!({"version":1,"cwd":self.cwd,"thread_id":self.thread_id,"turn_outcome":self.turn_outcome})
    }

    fn with_thread(mut self, id: &str) -> Result<Self, String> {
        if !valid_thread_id(id) {
            return Err("invalid thread id".into());
        }
        self.thread_id = Some(id.into());
        Ok(self)
    }

    fn completed(mut self) -> Self {
        self.turn_outcome = "completed".into();
        self
    }
    fn pending(mut self) -> Self {
        self.turn_outcome = "pending".into();
        self
    }

    fn write(&self, path: &Path) -> Result<(), String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock error")?
            .as_nanos();
        let temp = path.with_extension(format!("tmp-{}-{stamp}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let result = (|| {
            let mut file = options
                .open(&temp)
                .map_err(|_| "state temp creation failed")?;
            file.write_all(self.json().to_string().as_bytes())
                .map_err(|_| "state write failed")?;
            file.sync_all().map_err(|_| "state sync failed")?;
            fs::rename(&temp, path).map_err(|_| "state replacement failed")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    fn load_ready(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "state missing"
            } else {
                "state unreadable"
            }
        })?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "state corrupt")?;
        let version = value["version"].as_u64().ok_or("state corrupt")?;
        if version != 1 {
            return Err("unsupported state version".into());
        }
        let cwd = value["cwd"].as_str().ok_or("state corrupt")?;
        if !Path::new(cwd).is_absolute() {
            return Err("state corrupt: cwd must be absolute".into());
        }
        let outcome = value["turn_outcome"].as_str().ok_or("state corrupt")?;
        if outcome != "completed" {
            return Err("state pending or uncertain; inspect before retrying".into());
        }
        let thread_id = value["thread_id"]
            .as_str()
            .ok_or("state has invalid thread id")?;
        if !valid_thread_id(thread_id) {
            return Err("state has invalid thread id".into());
        }
        Ok(Self {
            cwd: cwd.into(),
            thread_id: Some(thread_id.into()),
            turn_outcome: outcome.into(),
        })
    }
}

fn valid_thread_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[derive(Debug, PartialEq, Eq)]
enum MessageKind {
    ServerRequest,
    Response,
    Notification,
    OtherResponse,
}

fn classify_message(message: &Value, id: u64) -> MessageKind {
    if message.get("method").is_some() {
        if message.get("id").is_some() {
            MessageKind::ServerRequest
        } else {
            MessageKind::Notification
        }
    } else if message["id"] == id {
        MessageKind::Response
    } else {
        MessageKind::OtherResponse
    }
}

fn is_auth_error(value: &Value) -> bool {
    if value["code"].as_i64() == Some(401) || value["httpStatusCode"].as_u64() == Some(401) {
        return true;
    }
    let info = &value["codexErrorInfo"];
    if info == "unauthorized" {
        return true;
    }
    [
        "httpConnectionFailed",
        "responseStreamConnectionFailed",
        "responseStreamDisconnected",
        "responseTooManyFailedAttempts",
    ]
    .iter()
    .any(|key| info[*key]["httpStatusCode"].as_u64() == Some(401))
        || is_auth_error_in_data(&value["data"])
}

fn is_auth_error_in_data(data: &Value) -> bool {
    data["httpStatusCode"].as_u64() == Some(401) || data["codexErrorInfo"] == "unauthorized"
}

fn classify_auth_failure(_payload: &str, refresh: bool) -> String {
    if refresh {
        "managed ChatGPT refresh failed; run codex login and retry auth-refresh".into()
    } else {
        "managed ChatGPT sign-in unavailable or unsupported; run codex login".into()
    }
}

#[derive(Default)]
struct ProbeState {
    text: String,
    status: Option<String>,
    local_cancel: bool,
    tool_calls: usize,
    thread_id: Option<String>,
    turn_id: Option<String>,
    pending_events: Vec<Value>,
    auth_failed: bool,
}

impl ProbeState {
    fn ingest(&mut self, message: &Value) -> Result<Option<Value>, String> {
        let params = &message["params"];
        let tracked = matches!(
            message["method"].as_str(),
            Some("item/agentMessage/delta" | "turn/completed")
        );
        if tracked {
            if let Some(expected) = &self.thread_id
                && params["threadId"].as_str() != Some(expected)
            {
                return Ok(None);
            }
            let event_turn = params["turnId"]
                .as_str()
                .or_else(|| params["turn"]["id"].as_str());
            if let Some(expected) = &self.turn_id {
                if event_turn != Some(expected) {
                    return Ok(None);
                }
            } else if self.thread_id.is_some() {
                if self.pending_events.len() >= 10_000 {
                    return Err("too many events before turn/start response".into());
                }
                self.pending_events.push(message.clone());
                return Ok(None);
            }
        }
        match message["method"].as_str() {
            Some("item/agentMessage/delta") => {
                let delta = message["params"]["delta"]
                    .as_str()
                    .ok_or("missing text delta")?;
                self.text.push_str(delta);
                Ok(None)
            }
            Some("turn/completed") => {
                let status = message["params"]["turn"]["status"]
                    .as_str()
                    .ok_or("missing turn status")?;
                self.status = Some(status.to_owned());
                self.auth_failed = is_auth_error(&message["params"]["turn"]["error"]);
                Ok(None)
            }
            Some("item/tool/call") => {
                if message["params"]["tool"] != "fixture_lookup" {
                    return Err("unexpected tool".into());
                }
                if message["params"]["arguments"]["key"] != "alpha" {
                    return Err("unknown fixture key".into());
                }
                let id = message.get("id").ok_or("missing tool request id")?;
                self.tool_calls += 1;
                Ok(Some(
                    json!({"id":id,"result":{"success":true,"contentItems":[{"type":"inputText","text":"alpha: fixture value 17"}]}}),
                ))
            }
            _ => Ok(None),
        }
    }

    fn outcome(&self) -> Result<&str, String> {
        if self.auth_failed {
            return Err("managed ChatGPT authorization expired or revoked; run codex login, then inspect pending state before retrying".into());
        }
        match self.status.as_deref() {
            Some("completed") => Ok("completed"),
            Some("interrupted") => Ok("interrupted"),
            Some(_) => Err("turn did not complete successfully".into()),
            None if self.local_cancel => {
                Err("locally cancelled; server outcome unconfirmed".into())
            }
            None => Err("stream ended without turn/completed".into()),
        }
    }
}

type WriteRequest = (String, Sender<Result<(), String>>);

struct Client {
    child: Child,
    writer: Option<Sender<WriteRequest>>,
    rx: Receiver<Result<Value, String>>,
    next_id: u64,
}

impl Client {
    fn start() -> Result<Self, String> {
        let binary = std::env::var("CODEX_BIN").unwrap_or_else(|_| "codex".into());
        Self::start_with(&binary)
    }

    fn start_with(binary: &str) -> Result<Self, String> {
        let mut child = Command::new(binary)
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("start app-server: {e}"))?;
        let stdin = child.stdin.take().ok_or("missing app-server stdin")?;
        let stdout = child.stdout.take().ok_or("missing app-server stdout")?;
        let (write_tx, write_rx) = mpsc::channel::<WriteRequest>();
        std::thread::spawn(move || writer_loop(stdin, write_rx));
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let parsed = line.map_err(|e| e.to_string()).and_then(|s| {
                    serde_json::from_str(&s).map_err(|e| format!("invalid JSON: {e}"))
                });
                if tx.send(parsed).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            writer: Some(write_tx),
            rx,
            next_id: 1,
        })
    }

    fn send(&mut self, value: &Value) -> Result<(), String> {
        self.send_until(value, Instant::now() + Duration::from_secs(45))
    }

    fn send_until(&mut self, value: &Value, deadline: Instant) -> Result<(), String> {
        let writer = self.writer.as_ref().ok_or("app-server stdin closed")?;
        if Instant::now() >= deadline {
            return Err("app-server operation timeout".into());
        }
        let (ack_tx, ack_rx) = mpsc::channel();
        writer
            .send((value.to_string(), ack_tx))
            .map_err(|_| "app-server writer exited".to_string())?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("app-server operation timeout".into());
        }
        match ack_rx.recv_timeout(remaining) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err("app-server operation timeout".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("app-server writer exited".into()),
        }
    }

    fn recv_until(&mut self, deadline: Instant) -> Result<Value, String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("app-server operation timeout".into());
        }
        match self.rx.recv_timeout(remaining) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => Err("app-server malformed output".into()),
            Err(mpsc::RecvTimeoutError::Timeout) => Err("app-server operation timeout".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("app-server exited before response".into())
            }
        }
    }

    fn shutdown(&mut self) {
        self.writer.take();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn handle(&mut self, message: &Value, state: &mut ProbeState) -> Result<(), String> {
        self.handle_until(message, state, Instant::now() + Duration::from_secs(45))
    }

    fn handle_until(
        &mut self,
        message: &Value,
        state: &mut ProbeState,
        deadline: Instant,
    ) -> Result<(), String> {
        if message.get("id").is_some() && message["method"] != "item/tool/call" {
            return Err("unknown server request".into());
        }
        if let Some(reply) = state.ingest(message)? {
            self.send_until(&reply, deadline)?;
        }
        Ok(())
    }

    fn request(
        &mut self,
        method: &str,
        params: Value,
        state: &mut ProbeState,
    ) -> Result<Value, String> {
        self.request_until(
            method,
            params,
            state,
            Instant::now() + Duration::from_secs(45),
        )
    }

    fn request_until(
        &mut self,
        method: &str,
        params: Value,
        state: &mut ProbeState,
        deadline: Instant,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_until(&json!({"id":id,"method":method,"params":params}), deadline)?;
        loop {
            let message = self.recv_until(deadline)?;
            if classify_message(&message, id) == MessageKind::ServerRequest {
                self.handle_until(&message, state, deadline)?;
                continue;
            }
            if classify_message(&message, id) == MessageKind::Response {
                if let Some(error) = message.get("error") {
                    if is_auth_error(error) {
                        return Err("managed ChatGPT authorization expired or revoked; run codex login, then inspect pending state before retrying".into());
                    }
                    return Err(format!("{method} failed (server details redacted)"));
                }
                return message
                    .get("result")
                    .cloned()
                    .ok_or_else(|| format!("{method} missing result"));
            }
            self.handle_until(&message, state, deadline)?;
        }
    }

    fn wait_turn(&mut self, state: &mut ProbeState) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(120);
        while state.status.is_none() {
            let message = self.recv_until(deadline)?;
            self.handle_until(&message, state, deadline)?;
        }
        Ok(())
    }
}

fn writer_loop(mut stdin: ChildStdin, requests: Receiver<WriteRequest>) {
    for (line, ack) in requests {
        let result = writeln!(stdin, "{line}")
            .and_then(|_| stdin.flush())
            .map_err(|_| "app-server write failed".to_string());
        let failed = result.is_err();
        let _ = ack.send(result);
        if failed {
            break;
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn start_turn(
    client: &mut Client,
    thread_id: &str,
    prompt: &str,
) -> Result<(String, ProbeState), String> {
    let mut state = ProbeState {
        thread_id: Some(thread_id.into()),
        ..Default::default()
    };
    let result = client.request(
        "turn/start",
        json!({
            "threadId":thread_id,
            "input":[{"type":"text","text":prompt}],
            "approvalPolicy":"never",
            "sandboxPolicy":{"type":"readOnly"}
        }),
        &mut state,
    )?;
    let turn_id = result["turn"]["id"]
        .as_str()
        .ok_or("turn/start missing turn id")?
        .to_owned();
    state.turn_id = Some(turn_id.clone());
    for event in std::mem::take(&mut state.pending_events) {
        client.handle(&event, &mut state)?;
    }
    Ok((turn_id, state))
}

fn validate_resumed_thread(response: &Value, expected: &str) -> Result<(), String> {
    if response["thread"]["id"].as_str() == Some(expected) {
        Ok(())
    } else {
        Err("thread/resume returned a different thread id".into())
    }
}

fn initialize_protocol(client: &mut Client) -> Result<(), String> {
    let mut state = ProbeState::default();
    client.request("initialize", json!({
        "clientInfo":{"name":"brn_rust_trial","title":"BRN Rust provider trial","version":"0.1.0"},
        "capabilities":{"experimentalApi":true}
    }), &mut state)?;
    client.send(&json!({"method":"initialized","params":{}}))?;
    Ok(())
}

fn check_auth(client: &mut Client, refresh: bool) -> Result<(), String> {
    check_auth_until(client, refresh, Instant::now() + Duration::from_secs(45))
}

fn check_auth_until(client: &mut Client, refresh: bool, deadline: Instant) -> Result<(), String> {
    let mut state = ProbeState::default();
    let account = client
        .request_until(
            "account/read",
            json!({"refreshToken":refresh}),
            &mut state,
            deadline,
        )
        .map_err(|error| {
            if error == "account/read failed (server details redacted)" {
                classify_auth_failure(&error, refresh)
            } else {
                error
            }
        })?;
    if account["account"]["type"] != "chatgpt" {
        return Err(
            "managed ChatGPT sign-in unavailable or unsupported; run codex login before retrying"
                .into(),
        );
    }
    Ok(())
}

fn initialize(client: &mut Client) -> Result<(), String> {
    initialize_protocol(client)?;
    check_auth(client, false)
}

fn live() -> Result<(), String> {
    let mut client = Client::start()?;
    let mut state = ProbeState::default();
    initialize(&mut client)?;
    println!("account: managed ChatGPT (credentials omitted)");
    let root = std::env::temp_dir().join("brn-app-server-fixtures");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let result = client.request("thread/start", json!({
        "cwd":root,
        "ephemeral":true,
        "approvalPolicy":"never",
        "sandbox":"read-only",
        "dynamicTools":[{"type":"function","name":"fixture_lookup","description":"Read the synthetic fixture value for key alpha.","inputSchema":{"type":"object","properties":{"key":{"type":"string","enum":["alpha"]}},"required":["key"],"additionalProperties":false}}]
    }), &mut state)?;
    let thread_id = result["thread"]["id"]
        .as_str()
        .ok_or("thread/start missing id")?
        .to_owned();

    let (_, mut stream) = start_turn(
        &mut client,
        &thread_id,
        "Synthetic check. Reply exactly STREAM_OK.",
    )?;
    client.wait_turn(&mut stream)?;
    if stream.outcome()? != "completed" || !stream.text.contains("STREAM_OK") {
        return Err("streamed text check failed".into());
    }
    println!(
        "streamed text: confirmed ({} characters)",
        stream.text.len()
    );

    let (_, mut tool) = start_turn(
        &mut client,
        &thread_id,
        "Use fixture_lookup with key alpha exactly once. Then state the fixture value.",
    )?;
    client.wait_turn(&mut tool)?;
    if tool.outcome()? != "completed" || tool.tool_calls != 1 || !tool.text.contains("17") {
        return Err("read-only tool round-trip failed".into());
    }
    println!("read-only tool round-trip: confirmed (one call)");

    let (_, mut continuation) = start_turn(
        &mut client,
        &thread_id,
        "What was the fixture value from the preceding turn? Answer briefly without calling tools.",
    )?;
    client.wait_turn(&mut continuation)?;
    if continuation.outcome()? != "completed" || !continuation.text.contains("17") {
        return Err("conversation continuation failed".into());
    }
    println!("conversation continuation: confirmed");

    let (turn_id, mut interruption) = start_turn(
        &mut client,
        &thread_id,
        "Write the numbers from 1 to 1000, one per line.",
    )?;
    interruption.local_cancel = true;
    let interrupt = client.request(
        "turn/interrupt",
        json!({"threadId":thread_id,"turnId":turn_id}),
        &mut interruption,
    );
    if let Err(error) = interrupt {
        return Err(format!("server interruption request failed: {error}"));
    }
    client.wait_turn(&mut interruption)?;
    if interruption.outcome()? != "interrupted" {
        return Err("server did not confirm interrupted status".into());
    }
    println!("server-side interruption: confirmed by turn/completed");
    Ok(())
}

fn resume_live() -> Result<(), String> {
    let root = std::env::temp_dir().join("brn-app-server-fixtures");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mut first = Client::start()?;
    initialize(&mut first)?;
    let mut state = ProbeState::default();
    let created = first.request(
        "thread/start",
        json!({"cwd":root,"ephemeral":false,"approvalPolicy":"never","sandbox":"read-only"}),
        &mut state,
    )?;
    let thread_id = created["thread"]["id"]
        .as_str()
        .ok_or("thread/start missing id")?
        .to_owned();
    let (_, mut first_turn) = start_turn(
        &mut first,
        &thread_id,
        "Synthetic persistence check. Reply exactly BRN_RESUME_47.",
    )?;
    first.wait_turn(&mut first_turn)?;
    if first_turn.outcome()? != "completed" || !first_turn.text.contains("BRN_RESUME_47") {
        return Err("initial persisted turn failed".into());
    }
    drop(first);

    let mut second = Client::start()?;
    initialize(&mut second)?;
    let mut resumed_state = ProbeState::default();
    let response = second.request(
        "thread/resume",
        json!({"threadId":thread_id}),
        &mut resumed_state,
    )?;
    validate_resumed_thread(&response, &thread_id)?;
    let (_, mut next_turn) = start_turn(
        &mut second,
        &thread_id,
        "What exact marker did you output in the previous turn? Reply with only that marker.",
    )?;
    second.wait_turn(&mut next_turn)?;
    if next_turn.outcome()? != "completed" || !next_turn.text.contains("BRN_RESUME_47") {
        return Err("resumed conversation did not recall synthetic marker".into());
    }
    println!("persisted thread resume: confirmed across App Server process restart");
    Ok(())
}

fn fixture_root() -> Result<std::path::PathBuf, String> {
    let root = std::env::temp_dir().join("brn-app-server-fixtures");
    fs::create_dir_all(&root).map_err(|_| "fixture directory unavailable".to_string())?;
    Ok(root)
}

fn persistent_thread(response: &Value, expected: Option<&str>) -> Result<String, String> {
    let id = response["thread"]["id"]
        .as_str()
        .ok_or("thread response missing id")?;
    if !valid_thread_id(id) {
        return Err("invalid thread id".into());
    }
    if expected.is_some_and(|value| value != id) {
        return Err("thread/resume returned a different thread id".into());
    }
    if response["thread"]["ephemeral"] != false {
        return Err("thread is not explicitly persistent".into());
    }
    Ok(id.into())
}

const MARKER: &str = "BRN_PERSIST_4729";

fn persist_start(path: &Path) -> Result<(), String> {
    let root = fixture_root()?;
    let cwd = root.to_str().ok_or("fixture path is not UTF-8")?;
    let state = PersistedState::create(path, cwd)?;
    let mut client = Client::start()?;
    initialize(&mut client)?;
    let mut probe = ProbeState::default();
    let response = client.request(
        "thread/start",
        json!({
            "cwd":cwd,"ephemeral":false,"approvalPolicy":"never","sandbox":"read-only"
        }),
        &mut probe,
    )?;
    let id = persistent_thread(&response, None)?;
    let state = state.with_thread(&id)?;
    state.write(path)?;
    let (_, mut turn) = start_turn(
        &mut client,
        &id,
        "Synthetic persistence check. Reply exactly BRN_PERSIST_4729.",
    )?;
    client.wait_turn(&mut turn)?;
    if turn.outcome()? != "completed" || !turn.text.contains(MARKER) {
        return Err("initial persisted turn did not confirm marker; state remains pending".into());
    }
    state.completed().write(path)?;
    println!("persistent thread saved; synthetic marker confirmed");
    Ok(())
}

fn persist_resume(path: &Path) -> Result<(), String> {
    let state = PersistedState::load_ready(path)?;
    let id = state
        .thread_id
        .as_deref()
        .ok_or("invalid thread id")?
        .to_string();
    let mut client = Client::start()?;
    initialize(&mut client)?;
    let mut probe = ProbeState::default();
    let response = client.request("thread/resume", json!({"threadId":id}), &mut probe)?;
    persistent_thread(&response, Some(&id))?;
    let state = state.pending();
    state.write(path)?;
    let (_, mut turn) = start_turn(
        &mut client,
        &id,
        "What exact marker did you output in the previous turn? Reply with only that marker.",
    )?;
    client.wait_turn(&mut turn)?;
    if turn.outcome()? != "completed" || !turn.text.contains(MARKER) {
        return Err("resumed conversation did not recall marker; state remains pending".into());
    }
    state.completed().write(path)?;
    println!("persistent resume confirmed across executable invocations");
    Ok(())
}

fn auth_command(refresh: bool) -> Result<(), String> {
    let mut client = Client::start()?;
    initialize_protocol(&mut client)?;
    check_auth(&mut client, refresh)?;
    println!("managed ChatGPT auth confirmed (credentials omitted)");
    Ok(())
}

fn sidecar_check() -> Result<(), String> {
    let mut client = Client::start()?;
    initialize_protocol(&mut client)?;
    client.shutdown();
    println!("App Server lifecycle confirmed");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command] if command == "live" => live(),
        [command] if command == "resume-live" => resume_live(),
        [command, path] if command == "persist-start" => persist_start(Path::new(path)),
        [command, path] if command == "persist-resume" => persist_resume(Path::new(path)),
        [command] if command == "auth-check" => auth_command(false),
        [command] if command == "auth-refresh" => auth_command(true),
        [command] if command == "sidecar-check" => sidecar_check(),
        _ => {
            eprintln!(
                "usage: brn-app-server-trial [live|resume-live|persist-start STATE_PATH|persist-resume STATE_PATH|auth-check|auth-refresh|sidecar-check]"
            );
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("trial failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_requires_terminal_success() {
        let mut state = ProbeState::default();
        state
            .ingest(&json!({"method":"item/agentMessage/delta","params":{"delta":"hel"}}))
            .unwrap();
        state
            .ingest(&json!({"method":"item/agentMessage/delta","params":{"delta":"lo"}}))
            .unwrap();
        assert_eq!(state.text, "hello");
        assert!(state.outcome().is_err());
        state
            .ingest(&json!({"method":"turn/completed","params":{"turn":{"status":"completed"}}}))
            .unwrap();
        assert_eq!(state.outcome().unwrap(), "completed");
    }

    #[test]
    fn interrupted_status_is_distinct_from_local_drop() {
        let mut state = ProbeState {
            local_cancel: true,
            ..Default::default()
        };
        assert!(state.outcome().is_err());
        state
            .ingest(&json!({"method":"turn/completed","params":{"turn":{"status":"interrupted"}}}))
            .unwrap();
        assert_eq!(state.outcome().unwrap(), "interrupted");
    }

    #[test]
    fn read_only_fixture_tool_rejects_unknown_inputs() {
        let mut state = ProbeState::default();
        let request = json!({"id":19,"method":"item/tool/call","params":{"tool":"fixture_lookup","arguments":{"key":"alpha"}}});
        let reply = state.ingest(&request).unwrap().unwrap();
        assert_eq!(reply["id"], 19);
        assert_eq!(
            reply["result"]["contentItems"][0]["text"],
            "alpha: fixture value 17"
        );
        assert_eq!(state.tool_calls, 1);
        assert!(state.ingest(&json!({"id":20,"method":"item/tool/call","params":{"tool":"fixture_lookup","arguments":{"key":"unknown"}}})).is_err());
        assert_eq!(state.tool_calls, 1);
    }

    #[test]
    fn state_rejects_replay_and_invalid_files() {
        let path = std::env::temp_dir().join(format!("brn-state-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(
            PersistedState::load_ready(&path)
                .unwrap_err()
                .contains("missing")
        );
        let state = PersistedState::create(&path, "/tmp").unwrap();
        assert!(PersistedState::create(&path, "/tmp").is_err());
        assert!(
            PersistedState::load_ready(&path)
                .unwrap_err()
                .contains("pending")
        );
        state
            .with_thread("thread-123")
            .unwrap()
            .completed()
            .write(&path)
            .unwrap();
        assert_eq!(
            PersistedState::load_ready(&path)
                .unwrap()
                .thread_id
                .as_deref(),
            Some("thread-123")
        );
        std::fs::write(&path, "not json").unwrap();
        assert!(
            PersistedState::load_ready(&path)
                .unwrap_err()
                .contains("corrupt")
        );
        std::fs::write(
            &path,
            r#"{"version":999,"cwd":"/tmp","thread_id":"thread-123","turn_outcome":"completed"}"#,
        )
        .unwrap();
        assert!(
            PersistedState::load_ready(&path)
                .unwrap_err()
                .contains("version")
        );
        std::fs::write(
            &path,
            r#"{"version":1,"cwd":"/tmp","thread_id":"../bad","turn_outcome":"completed"}"#,
        )
        .unwrap();
        assert!(
            PersistedState::load_ready(&path)
                .unwrap_err()
                .contains("thread id")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn turn_events_are_correlated_before_and_after_start_response() {
        let mut state = ProbeState {
            thread_id: Some("thread-1".into()),
            ..Default::default()
        };
        state.ingest(&json!({"method":"item/agentMessage/delta","params":{"threadId":"other","turnId":"turn-1","delta":"wrong"}})).unwrap();
        state.ingest(&json!({"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","delta":"right"}})).unwrap();
        assert!(state.text.is_empty());
        state.turn_id = Some("turn-1".into());
        for event in std::mem::take(&mut state.pending_events) {
            state.ingest(&event).unwrap();
        }
        assert_eq!(state.text, "right");
        state.ingest(&json!({"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"other","status":"completed"}}})).unwrap();
        assert!(state.status.is_none());
        state.ingest(&json!({"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed"}}})).unwrap();
        assert_eq!(state.outcome().unwrap(), "completed");
    }

    #[test]
    fn server_request_id_is_not_client_response() {
        assert_eq!(
            classify_message(&json!({"id":1,"method":"unknown/request","params":{}}), 1),
            MessageKind::ServerRequest
        );
        assert_eq!(
            classify_message(&json!({"id":1,"result":{}}), 1),
            MessageKind::Response
        );
    }

    #[test]
    fn unauthorized_terminal_turn_has_login_recovery_without_payload() {
        let mut state = ProbeState::default();
        state.ingest(&json!({"method":"turn/completed","params":{"turn":{"id":"turn-1","status":"failed","error":{"message":"secret token=abc","codexErrorInfo":"unauthorized"}}}})).unwrap();
        let error = state.outcome().unwrap_err();
        assert!(error.contains("codex login"));
        assert!(!error.contains("secret"));
        assert!(is_auth_error(&json!({"code":401,"message":"token=abc"})));
        assert!(is_auth_error(
            &json!({"codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":401}}})
        ));
    }

    #[cfg(unix)]
    #[test]
    fn account_read_transport_failure_keeps_transport_diagnostic() {
        for (body, expected, wait) in [
            (
                "IFS= read -r line; exit 0",
                "exited",
                Duration::from_secs(1),
            ),
            (
                "exec python3 -c 'import time; time.sleep(30)'",
                "timeout",
                Duration::from_millis(150),
            ),
        ] {
            let script = fake_sidecar(body);
            let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
            let error = check_auth_until(&mut client, true, Instant::now() + wait).unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("codex login"), "{error}");
            drop(client);
            let _ = fs::remove_file(script);
        }
    }

    #[cfg(unix)]
    #[test]
    fn blocked_stdin_write_respects_deadline_and_reaps_sidecar() {
        let script = fake_sidecar("exec python3 -c 'import time; time.sleep(30)'");
        let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
        let began = Instant::now();
        let large = json!({"payload":"x".repeat(2_000_000)});
        let error = client
            .send_until(&large, began + Duration::from_millis(150))
            .unwrap_err();
        assert!(error.contains("timeout"), "{error}");
        client.shutdown();
        assert!(client.child.try_wait().unwrap().is_some());
        assert!(began.elapsed() < Duration::from_secs(4));
        drop(client);
        let _ = fs::remove_file(script);
    }

    #[cfg(unix)]
    #[test]
    fn fake_account_unavailable_unsupported_and_refresh_rejected() {
        for response in [
            r#"{"id":1,"result":{"account":null,"requiresOpenaiAuth":true}}"#,
            r#"{"id":1,"result":{"account":{"type":"apiKey"},"requiresOpenaiAuth":true}}"#,
            r#"{"id":1,"error":{"code":401,"message":"token=private"}}"#,
        ] {
            let script = fake_sidecar(&format!("IFS= read -r line; printf '%s\\n' '{response}'"));
            let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
            let error = check_auth(&mut client, true).unwrap_err();
            assert!(error.contains("codex login"));
            assert!(!error.contains("private"));
            drop(client);
            let _ = fs::remove_file(script);
        }
    }

    #[test]
    fn auth_failure_redacts_server_payload() {
        let error = classify_auth_failure("secret token=abc", true);
        assert!(error.contains("codex login"));
        assert!(!error.contains("secret"));
        assert!(!error.contains("abc"));
    }

    #[cfg(unix)]
    fn fake_sidecar(body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("brn-fake-{}-{stamp}.sh", std::process::id()));
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn fake_sidecar_missing_exit_and_stderr_are_safe() {
        assert!(Client::start_with("/definitely/missing/codex").is_err());
        let script = fake_sidecar("echo 'token=private' >&2; exit 7");
        let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
        let error = client
            .request_until(
                "initialize",
                json!({}),
                &mut ProbeState::default(),
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap_err();
        assert!(error.contains("exited") || error.contains("write failed"));
        assert!(!error.contains("private"));
        drop(client);
        let _ = fs::remove_file(script);
    }

    #[cfg(unix)]
    #[test]
    fn notification_chatter_cannot_extend_request_deadline() {
        let script = fake_sidecar(
            "IFS= read -r line; while :; do printf '{\"method\":\"thread/started\",\"params\":{}}\\n'; sleep 0.01; done",
        );
        let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
        let began = Instant::now();
        let error = client
            .request_until(
                "initialize",
                json!({}),
                &mut ProbeState::default(),
                began + Duration::from_millis(150),
            )
            .unwrap_err();
        assert!(error.contains("timeout"));
        assert!(began.elapsed() < Duration::from_secs(1));
        drop(client);
        let _ = fs::remove_file(script);
    }

    #[cfg(unix)]
    #[test]
    fn fake_sidecar_force_kills_child_that_ignores_eof() {
        let script = fake_sidecar(
            "IFS= read -r line; printf '{\"id\":1,\"result\":{}}\\n'; while :; do sleep 1; done",
        );
        let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
        client
            .request_until(
                "initialize",
                json!({}),
                &mut ProbeState::default(),
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        client.shutdown();
        assert!(client.child.try_wait().unwrap().is_some());
        drop(client);
        let _ = fs::remove_file(script);
    }

    #[cfg(unix)]
    #[test]
    fn fake_sidecar_closes_stdin_then_reaps_child() {
        let script = fake_sidecar(
            "IFS= read -r line; printf '{\"id\":1,\"result\":{}}\\n'; while IFS= read -r line; do :; done",
        );
        let mut client = Client::start_with(script.to_str().unwrap()).unwrap();
        client
            .request_until(
                "initialize",
                json!({}),
                &mut ProbeState::default(),
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        client.shutdown();
        assert!(client.child.try_wait().unwrap().is_some());
        drop(client);
        let _ = fs::remove_file(script);
    }

    #[test]
    fn resume_must_return_the_requested_thread() {
        let response = json!({"thread":{"id":"different"}});
        assert!(validate_resumed_thread(&response, "expected").is_err());
        let response = json!({"thread":{"id":"expected"}});
        assert!(validate_resumed_thread(&response, "expected").is_ok());
    }
}
