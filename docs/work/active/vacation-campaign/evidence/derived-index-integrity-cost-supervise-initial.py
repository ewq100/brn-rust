import subprocess,os,signal,time,shutil,json,datetime,pathlib,hashlib
root=pathlib.Path('/tmp/brn-derived-index-integrity-qualification')
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust')
source=repo/'crates/brn-retrieval/src/search_index.rs'
original=root/'original-source.rs'
expected='61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9'
original_hash='94efb75c085c933130196aa2681aa47624674f6a2a3074522c1280992044cc70'
assert hashlib.sha256(original.read_bytes()).hexdigest()==original_hash
started=time.monotonic();reason=None;code=None;p=None;exception=None;restored=False
try:
 with (root/'cost-run.log').open('wb') as log:
  p=subprocess.Popen(['bash',str(root/'cost-run.sh')],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-started>1800:reason='30-minute experiment deadline'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
   if reason:break
   time.sleep(5)
except BaseException as e:
 exception=type(e).__name__;reason='supervisor exception'
finally:
 if p is not None:
  if p.poll() is None:
   try:os.killpg(p.pid,signal.SIGTERM)
   except ProcessLookupError:pass
   try:p.wait(timeout=10)
   except subprocess.TimeoutExpired:
    os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=10)
  code=p.returncode
 current=hashlib.sha256(source.read_bytes()).hexdigest()
 if current==expected:
  source.write_bytes(original.read_bytes())
  restored=hashlib.sha256(source.read_bytes()).hexdigest()==original_hash
 elif current==original_hash:restored=True
 else:reason=(reason+'; ' if reason else '')+'source changed unexpectedly; preserve without overwrite'
 receipt={'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit_code':code,'stop_reason':reason,'exception_type':exception,'elapsed_seconds':time.monotonic()-started,'disk_free_bytes':shutil.disk_usage(repo).free,'source_restored_exactly':restored,'original_source_sha256':original_hash,'temporary_source_sha256':expected,'scope':'cost-only healthy-index scan, no recovery candidate or production adoption'}
 (root/'cost-completion.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps(receipt),flush=True)
raise SystemExit(code if code is not None and restored and reason is None else 1)
