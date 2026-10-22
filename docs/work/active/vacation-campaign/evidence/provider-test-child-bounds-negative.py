#!/usr/bin/env python3
"""One owned synthetic hang negative; exact source restoration before verdict."""
import hashlib,json,os,re,signal,subprocess,sys,time,shutil
from pathlib import Path
from datetime import datetime,timezone
w=Path('/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust');q=Path('/tmp/brn-provider-test-child-bounds-qualification');source=w/'crates/brn-ai/src/provider_stderr_tests.rs';head='a6b6d9902c8797a696cb96e1d9e848c564425b9e'
def git(*a):return subprocess.check_output(['git',*a],cwd=w,text=True).strip()
assert git('rev-parse','HEAD')==head and not git('status','--porcelain')
assert shutil.disk_usage(w).free>=2*1024**3
original=source.read_bytes();(q/'negative-source-original.rs').write_bytes(original)
s=original.decode()
def replace_once(old,new):
 global s
 assert s.count(old)==1,old;s=s.replace(old,new)
replace_once('std::time::Duration::from_secs(30);','std::time::Duration::from_secs(1);')
replace_once('async fn provider_stderr_child() {','async fn provider_stderr_child() {\n    std::thread::sleep(std::time::Duration::from_secs(10));')
replace_once('            let _ = child.wait();','            let pid = child.id();\n            let status = child.wait();\n            eprintln!("BRN_NEGATIVE_STDERR_CHILD_REAP pid={pid} status={status:?}");')
cmd=['cargo','test','-p','brn-ai','--lib','provider_formats_tests::provider_stderr_tests::rejected_tool_arguments_and_partial_text_never_reach_process_stderr','--locked','--offline','--jobs','1','--','--exact','--nocapture']
receipt={'observed_at':datetime.now(timezone.utc).isoformat(),'candidate':head,'command':cmd,'control':'Test-only child sleeps10s before scripted fixtures; same parent watchdog uses1s diagnostic constant, Drop emits owned PID and actual wait status. No live provider action.','qualified_source_sha256':hashlib.sha256(original).hexdigest(),'timeout_seconds':180,'process_exit_code':None,'outer_timeout':False}
start=time.monotonic();p=None
try:
 source.write_text(s)
 (q/'negative.patch').write_text(subprocess.check_output(['git','diff','--','crates/brn-ai/src/provider_stderr_tests.rs'],cwd=w,text=True))
 with (q/'negative.log').open('wb') as f:
  p=subprocess.Popen(cmd,cwd=w,stdin=subprocess.DEVNULL,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
  try:receipt['process_exit_code']=p.wait(timeout=180)
  except subprocess.TimeoutExpired:
   receipt['outer_timeout']=True;os.killpg(p.pid,signal.SIGKILL);receipt['process_exit_code']=p.wait(timeout=10)
finally:
 try:
  if p is not None and p.poll() is None:
   try:os.killpg(p.pid,signal.SIGKILL)
   except ProcessLookupError:pass
   p.wait(timeout=10)
 finally:
  source.write_bytes(original)
  receipt['elapsed_seconds']=round(time.monotonic()-start,3);receipt['source_restored_sha256']=hashlib.sha256(source.read_bytes()).hexdigest();receipt['clean_head_restored']=git('rev-parse','HEAD')==head and not git('status','--porcelain')
  (q/'negative-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
log=(q/'negative.log').read_text(errors='replace');m=re.search(r'BRN_NEGATIVE_STDERR_CHILD_REAP pid=(\d+) status=Ok\(ExitStatus\(unix_wait_status\(9\)\)\)',log)
receipt['owned_child_reaped_sigkill']=bool(m);receipt['owned_pid']=int(m.group(1)) if m else None;receipt['expected_timeout_assertion']='synthetic provider stderr child timed out' in log;receipt['expected_test_failure']='0 passed; 1 failed;' in log;receipt['verdict']='PASS expected watchdog failure and owned child reaping; exact source restored' if receipt['process_exit_code']==101 and not receipt['outer_timeout'] and m and receipt['expected_timeout_assertion'] and receipt['expected_test_failure'] and receipt['clean_head_restored'] else 'FAIL qualification incomplete; original failure retained'
(q/'negative-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2));sys.exit(0 if receipt['verdict'].startswith('PASS') else 1)
