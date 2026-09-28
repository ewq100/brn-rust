#![cfg(unix)]
use brn_provider::{Client, Config, ProviderError, TurnStatus};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn fake(mode: &str) -> (Config, PathBuf) {
    let dir =
        std::env::temp_dir().join(format!("brn-provider-test-{}-{}", std::process::id(), mode));
    let _ = fs::create_dir_all(&dir);
    let exe = dir.join(format!("codex-fake-{mode}"));
    fs::write(&exe, r#"#!/usr/bin/env python3
import json, os, sys, time, subprocess
mode=os.path.basename(sys.argv[0]).split('-')[-1]
turn_count=0
pending_interrupt=None
def send(v):
 print(json.dumps(v),flush=True)
for line in sys.stdin:
 try: req=json.loads(line)
 except: break
 method=req.get('method')
 rid=req.get('id')
 if method=='initialize':
  if mode=='descendant': subprocess.Popen(['sleep','30'],stdout=sys.stdout,stderr=subprocess.DEVNULL);sys.exit(0)
  if mode=='malformed': print('{broken',flush=True); continue
  if mode=='exit': sys.exit(0)
  if mode=='hang': time.sleep(5); continue
  send({'id':rid,'result':{'userAgent':'codex/'+('0.0.0' if mode=='wrongversion' else '0.155.0-alpha.16.4'),'codexHome':os.environ['CODEX_HOME'],'platformFamily':'unix','platformOs':'macos'}})
 elif method=='account/read':
  if mode=='auth': send({'id':rid,'error':{'code':401,'message':'secret-token-abc'}})
  else: send({'id':rid,'result':{'account':{'type':'chatgpt','email':'private@example.org'},'workspaceRouting':{'chatgptAccountId':'acct_fake'}}})
 elif method=='thread/start':
  send({'id':rid,'result':{'thread':{'id':'thr_test','sessionId':'thr_test'}}})
 elif method=='thread/resume':
  tid='wrong' if mode=='mismatch' else req['params']['threadId']
  send({'id':rid,'result':{'thread':{'id':tid,'sessionId':tid}}})
 elif method=='turn/start':
  turn_count+=1
  if mode=='start_hang': time.sleep(5); continue
  tid=req['params']['threadId']
  assert req['params']['approvalPolicy']=='never'
  assert req['params']['sandboxPolicy']['type']=='readOnly'
  assert req['params']['sandboxPolicy']['networkAccess']==False
  if mode=='tool':
   send({'id':777,'method':'item/tool/call','params':{'tool':'unsafe'}})
   denial=json.loads(sys.stdin.readline())
   assert denial['id']==777 and 'error' in denial
  if mode=='early': send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_test','delta':'Hi'}})
  send({'id':rid,'result':{'turn':{'id':'turn_test'}}})
  if pending_interrupt is not None: send({'id':pending_interrupt,'result':{}});pending_interrupt=None
  if mode in ['cancel','cancel_late_ack'] or (mode=='late_ack_active' and turn_count==1): continue
  if mode!='early': send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_test','delta':'Hi'}})
  send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_test','status':'completed','usage':{'inputTokens':3}}}})
 elif method=='turn/interrupt':
  terminal={'method':'turn/completed','params':{'threadId':req['params']['threadId'],'turn':{'id':req['params']['turnId'],'status':'interrupted'}}}
  if mode=='late_ack_active': send(terminal);pending_interrupt=rid
  elif mode=='cancel_late_ack': send(terminal);send({'id':rid,'result':{}})
  else: send({'id':rid,'result':{}});send(terminal)
"#).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    let mut cfg = Config::new(exe, dir.clone());
    cfg.request_timeout = Duration::from_secs(3);
    cfg.turn_timeout = Duration::from_secs(2);
    cfg.shutdown_timeout = Duration::from_millis(100);
    cfg.codex_home = Some(dir.join("codex-home"));
    (cfg, dir)
}
#[test]
fn stream_and_early_event() {
    for mode in ["normal", "early"] {
        let (cfg, _dir) = fake(mode);
        let mut client = Client::connect(cfg).unwrap();
        let thread = client.thread_start().unwrap();
        assert_eq!(thread.id, "thr_test");
        let mut deltas = Vec::new();
        let result = client
            .turn(
                &thread,
                "Grounded passage",
                &AtomicBool::new(false),
                |_| Ok(()),
                |d| deltas.push(d.to_owned()),
            )
            .unwrap();
        assert_eq!(result.status, TurnStatus::Completed);
        assert_eq!(result.text, "Hi");
        assert_eq!(deltas, ["Hi"]);
        assert_eq!(result.usage.unwrap()["inputTokens"], 3);
    }
}
#[test]
fn cancellation_waits_for_server_status() {
    let (cfg, _) = fake("cancel");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let cancel = AtomicBool::new(false);
    let result = client
        .turn(
            &thread,
            "Stop",
            &cancel,
            |_| {
                cancel.store(true, Ordering::Release);
                Ok(())
            },
            |_| {},
        )
        .unwrap();
    assert_eq!(result.status, TurnStatus::Interrupted);
}
#[test]
fn pre_cancelled_turn_is_not_submitted() {
    let (cfg, _) = fake("normal");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let cancel = AtomicBool::new(true);
    assert_eq!(
        client
            .turn(&thread, "No send", &cancel, |_| Ok(()), |_| {})
            .unwrap_err(),
        ProviderError::Cancelled
    );
    let result = client
        .turn(
            &thread,
            "Then send",
            &AtomicBool::new(false),
            |_| Ok(()),
            |_| {},
        )
        .unwrap();
    assert_eq!(result.status, TurnStatus::Completed);
}
#[test]
fn terminal_before_interrupt_ack_keeps_transport_correlated() {
    let (cfg, _) = fake("cancel_late_ack");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let cancel = AtomicBool::new(false);
    let result = client
        .turn(
            &thread,
            "Stop",
            &cancel,
            |_| {
                cancel.store(true, Ordering::Release);
                Ok(())
            },
            |_| {},
        )
        .unwrap();
    assert_eq!(result.status, TurnStatus::Interrupted);
    assert_eq!(client.thread_resume("thr_test").unwrap().id, "thr_test");
}
#[test]
fn retired_interrupt_ack_during_next_active_turn_is_ignored() {
    let (cfg, _) = fake("late_ack_active");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let cancel = AtomicBool::new(false);
    let first = client
        .turn(
            &thread,
            "Stop",
            &cancel,
            |_| {
                cancel.store(true, Ordering::Release);
                Ok(())
            },
            |_| {},
        )
        .unwrap();
    assert_eq!(first.status, TurnStatus::Interrupted);
    let second = client
        .turn(
            &thread,
            "Answer",
            &AtomicBool::new(false),
            |_| Ok(()),
            |_| {},
        )
        .unwrap();
    assert_eq!(second.status, TurnStatus::Completed);
    assert_eq!(second.text, "Hi");
}
#[test]
fn unsolicited_tool_request_is_denied() {
    let (cfg, _) = fake("tool");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let result = client
        .turn(
            &thread,
            "Answer from context",
            &AtomicBool::new(false),
            |_| Ok(()),
            |_| {},
        )
        .unwrap();
    assert_eq!(result.status, TurnStatus::Completed);
}
#[test]
fn failed_started_persistence_aborts_submission() {
    let (cfg, _) = fake("normal");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let err = client
        .turn(
            &thread,
            "Draft",
            &AtomicBool::new(false),
            |_| Err("store failed".into()),
            |_| {},
        )
        .unwrap_err();
    assert_eq!(err, ProviderError::UncertainTurn);
    assert_eq!(
        client
            .turn(
                &thread,
                "Again",
                &AtomicBool::new(false),
                |_| Ok(()),
                |_| {}
            )
            .unwrap_err(),
        ProviderError::UncertainTurn
    );
}
#[test]
fn resume_mismatch_fails_closed() {
    let (cfg, _) = fake("mismatch");
    let mut client = Client::connect(cfg).unwrap();
    assert_eq!(
        client.thread_resume("thr_test").unwrap_err(),
        ProviderError::Protocol
    );
}
#[test]
fn auth_is_redacted() {
    let (cfg, _) = fake("auth");
    let err = Client::connect(cfg).err().unwrap();
    assert_eq!(err, ProviderError::Authentication);
    assert!(!err.to_string().contains("secret-token-abc"));
}
#[test]
fn incompatible_server_version_fails_before_auth_or_turn() {
    let (cfg, _) = fake("wrongversion");
    assert_eq!(Client::connect(cfg).err().unwrap(), ProviderError::Protocol);
}
#[test]
fn malformed_missing_exit_and_timeout() {
    for (mode, expected) in [
        ("malformed", ProviderError::MalformedOutput),
        ("exit", ProviderError::UnexpectedExit),
        ("hang", ProviderError::Timeout),
    ] {
        let (cfg, _) = fake(mode);
        assert_eq!(Client::connect(cfg).err().unwrap(), expected);
    }
    let (mut cfg, _) = fake("normal");
    cfg.executable = PathBuf::from("/missing/codex");
    assert_eq!(
        Client::connect(cfg).err().unwrap(),
        ProviderError::MissingExecutable
    );
}
#[test]
fn cancel_during_start_is_prompt_and_uncertain() {
    let (cfg, _) = fake("start_hang");
    let mut client = Client::connect(cfg).unwrap();
    let thread = client.thread_start().unwrap();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        signal.store(true, Ordering::Release);
    });
    let started = std::time::Instant::now();
    assert_eq!(
        client
            .turn(&thread, "Question", &cancel, |_| Ok(()), |_| {})
            .unwrap_err(),
        ProviderError::UncertainTurn
    );
    assert!(started.elapsed() < Duration::from_millis(750));
    timer.join().unwrap();
}
#[test]
fn drop_kills_descendant_holding_stdout() {
    let (cfg, _) = fake("descendant");
    let started = std::time::Instant::now();
    assert_eq!(Client::connect(cfg).err().unwrap(), ProviderError::Timeout);
    assert!(started.elapsed() < Duration::from_secs(5));
}
