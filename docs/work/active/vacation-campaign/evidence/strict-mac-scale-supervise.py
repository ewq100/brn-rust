import subprocess,os,signal,time,shutil,json,datetime,pathlib
root=pathlib.Path('/tmp/brn-strict-mac-scale-qualification')
started=time.monotonic(); floor=1024**3; reason=None; code=None
with (root/'run.log').open('wb') as log:
 p=subprocess.Popen(['bash',str(root/'run.sh')],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 while p.poll() is None:
  if time.monotonic()-started>1800: reason='30-minute qualification deadline'
  elif shutil.disk_usage('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust').free<floor: reason='1GiB disk floor'
  if reason:
   os.killpg(p.pid,signal.SIGTERM)
   try:p.wait(timeout=10)
   except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
   break
  time.sleep(5)
 code=p.returncode
receipt={'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit_code':code,'stop_reason':reason,'elapsed_seconds':time.monotonic()-started,'disk_free_bytes':shutil.disk_usage('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust').free}
(root/'completion.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));raise SystemExit(code if code is not None else 1)
