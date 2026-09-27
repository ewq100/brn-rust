use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[derive(Default)]
struct ProbeState {
    text: String,
    status: Option<String>,
    local_cancel: bool,
    tool_calls: usize,
}

impl ProbeState {
    fn ingest(&mut self, message: &Value) -> Result<Option<Value>, String> {
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
        match self.status.as_deref() {
            Some("completed") => Ok("completed"),
            Some("interrupted") => Ok("interrupted"),
            Some(other) => Err(format!("turn ended with {other}")),
            None if self.local_cancel => {
                Err("locally cancelled; server outcome unconfirmed".into())
            }
            None => Err("stream ended without turn/completed".into()),
        }
    }
}

struct Client {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Result<Value, String>>,
    next_id: u64,
}

impl Client {
    fn start() -> Result<Self, String> {
        let binary = std::env::var("CODEX_BIN").unwrap_or_else(|_| "codex".into());
        let mut child = Command::new(binary)
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("start app-server: {e}"))?;
        let stdin = child.stdin.take().ok_or("missing app-server stdin")?;
        let stdout = child.stdout.take().ok_or("missing app-server stdout")?;
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
            stdin,
            rx,
            next_id: 1,
        })
    }

    fn send(&mut self, value: &Value) -> Result<(), String> {
        writeln!(self.stdin, "{value}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())
    }

    fn recv(&self) -> Result<Value, String> {
        self.rx
            .recv_timeout(Duration::from_secs(45))
            .map_err(|e| format!("app-server response timeout or EOF: {e}"))?
    }

    fn handle(&mut self, message: &Value, state: &mut ProbeState) -> Result<(), String> {
        if let Some(reply) = state.ingest(message)? {
            self.send(&reply)?;
        }
        Ok(())
    }

    fn request(
        &mut self,
        method: &str,
        params: Value,
        state: &mut ProbeState,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"id":id,"method":method,"params":params}))?;
        loop {
            let message = self.recv()?;
            if message["id"] == id {
                if let Some(error) = message.get("error") {
                    return Err(format!(
                        "{method} failed: {}",
                        error["message"].as_str().unwrap_or("unknown error")
                    ));
                }
                return message
                    .get("result")
                    .cloned()
                    .ok_or_else(|| format!("{method} missing result"));
            }
            self.handle(&message, state)?;
        }
    }

    fn wait_turn(&mut self, state: &mut ProbeState) -> Result<(), String> {
        while state.status.is_none() {
            let message = self.recv()?;
            self.handle(&message, state)?;
        }
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_turn(
    client: &mut Client,
    thread_id: &str,
    prompt: &str,
) -> Result<(String, ProbeState), String> {
    let mut state = ProbeState::default();
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
    Ok((turn_id, state))
}

fn live() -> Result<(), String> {
    let mut client = Client::start()?;
    let mut state = ProbeState::default();
    client.request("initialize", json!({
        "clientInfo":{"name":"brn_rust_trial","title":"BRN Rust provider trial","version":"0.1.0"},
        "capabilities":{"experimentalApi":true}
    }), &mut state)?;
    client.send(&json!({"method":"initialized","params":{}}))?;
    let account = client.request("account/read", json!({"refreshToken":false}), &mut state)?;
    if account["account"]["type"] != "chatgpt" {
        return Err(
            "ChatGPT managed subscription login is unavailable; no model request sent".into(),
        );
    }
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

fn main() {
    if std::env::args().nth(1).as_deref() != Some("live") {
        eprintln!("usage: brn-app-server-trial live (run cargo test for credential-free checks)");
        std::process::exit(2);
    }
    if let Err(error) = live() {
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
}
