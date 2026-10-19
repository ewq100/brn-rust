//! A test-only loopback stand-in for the Copilot chat endpoint (std only).
//! The CLI reaches it through a synthetic cached `copilot.json` whose
//! `endpoints.api` names the stub; every child also gets proxy variables
//! that point at a closed port, so nothing else is reachable. No production
//! code knows about the stub.
use super::Root;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// One scripted reply, used for the next request.
pub enum Reply {
    /// A complete SSE stream of chat-completion chunks, then `[DONE]`.
    Sse(Vec<Value>),
    /// Headers and one chunk, then the stream stays open until the stub drops.
    Hold,
    /// An HTTP error status with a small JSON body.
    Status(u16),
}

/// A final text answer.
pub fn answer(text: &str) -> Reply {
    Reply::Sse(vec![
        chunk(json!({"role": "assistant", "content": text}), None),
        finish("stop"),
    ])
}
/// One tool call; the model continues after the tool result.
pub fn tool(id: &str, name: &str, args: Value) -> Reply {
    let call = json!({"index": 0, "id": id, "type": "function",
        "function": {"name": name, "arguments": args.to_string()}});
    Reply::Sse(vec![
        chunk(json!({"role": "assistant", "tool_calls": [call]}), None),
        finish("tool_calls"),
    ])
}
fn chunk(delta: Value, reason: Option<&str>) -> Value {
    json!({"id": "stub", "object": "chat.completion.chunk", "created": 0, "model": "synthetic",
        "choices": [{"index": 0, "delta": delta, "finish_reason": reason}]})
}
fn finish(reason: &str) -> Value {
    let mut value = chunk(json!({}), Some(reason));
    value["usage"] = json!({"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2});
    value
}

/// The environment every stub-backed child gets: proxies on a closed port.
pub fn proxy_env() -> Vec<(String, String)> {
    let closed = "http://127.0.0.1:9";
    let mut env = vec![];
    for name in ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"] {
        env.push((name.to_owned(), closed.to_owned()));
        env.push((name.to_lowercase(), closed.to_owned()));
    }
    for name in ["NO_PROXY", "no_proxy"] {
        env.push((name.to_owned(), "127.0.0.1,localhost".to_owned()));
    }
    env
}

pub struct Stub {
    pub port: u16,
    seen: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
}
impl Stub {
    /// Serves `script` in order; a request beyond it gets HTTP 500.
    pub fn start(script: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let seen = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let script = Arc::new(Mutex::new(script.into_iter()));
        let (seen_for, stop_for) = (seen.clone(), stop.clone());
        std::thread::spawn(move || {
            while !stop_for.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (seen, stop, script) =
                            (seen_for.clone(), stop_for.clone(), script.clone());
                        std::thread::spawn(move || serve(stream, &seen, &stop, &script));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
        Self { port, seen, stop }
    }
    /// The JSON bodies received so far.
    pub fn requests(&self) -> Vec<Value> {
        self.seen.lock().unwrap().clone()
    }
    /// Waits (up to 30 s) until `count` requests arrived.
    pub fn wait_for(&self, count: usize) {
        let started = Instant::now();
        while self.requests().len() < count {
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "the stub saw {} of {count} requests",
                self.requests().len()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    /// A 0700 credentials folder whose 0600 `copilot.json` holds a synthetic,
    /// unexpired session token routed to this stub.
    pub fn credentials(&self, root: &Root) -> PathBuf {
        self.credentials_in(&root.path)
    }
    /// As [`Self::credentials`], but place the synthetic cache under a caller-owned
    /// durable fixture root instead of the short-lived test Root.
    pub fn credentials_in(&self, root: &std::path::Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let dir = root.join(format!("copilot-{}", self.port));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let record = json!({"token": "SYNTHETIC", "expires_at": 4102444800i64,
            "endpoints": {"api": format!("http://127.0.0.1:{}", self.port)}});
        let file = dir.join("copilot.json");
        std::fs::write(&file, record.to_string()).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        dir
    }
}
impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn serve(
    stream: TcpStream,
    seen: &Mutex<Vec<Value>>,
    stop: &AtomicBool,
    script: &Mutex<std::vec::IntoIter<Reply>>,
) {
    stream.set_nonblocking(false).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }
    seen.lock()
        .unwrap()
        .push(serde_json::from_slice(&body).unwrap_or(Value::Null));
    let reply = script.lock().unwrap().next().unwrap_or(Reply::Status(500));
    let mut out = stream;
    let sse = |out: &mut TcpStream, chunks: &[Value]| {
        let _ = out.write_all(
            b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n",
        );
        for chunk in chunks {
            let _ = out.write_all(format!("data: {chunk}\n\n").as_bytes());
        }
        let _ = out.flush();
    };
    match reply {
        Reply::Sse(chunks) => {
            sse(&mut out, &chunks);
            let _ = out.write_all(b"data: [DONE]\n\n");
        }
        Reply::Hold => {
            sse(
                &mut out,
                &[chunk(
                    json!({"role": "assistant", "content": "Working"}),
                    None,
                )],
            );
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        Reply::Status(code) => {
            let body = br#"{"error":"synthetic stub failure"}"#;
            let _ = out.write_all(
                format!(
                    "HTTP/1.1 {code} Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = out.write_all(body);
        }
    }
    let _ = out.flush();
}
