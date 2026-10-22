import pathlib,subprocess,json,hashlib,time,sys,datetime,shutil
root=pathlib.Path('/tmp/brn-writing-setup-parallel-baseline')
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust')
head='ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b'
source=repo/'crates/brn-threads-app/tests/writing_setup.rs'
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert hashlib.sha256(source.read_bytes()).hexdigest()=='241ef3a5e87d8e0e1d02487aba5e43759664428196a0f2546a91937ee036f2b8'
assert shutil.disk_usage(repo).free>3*1024**3
started=time.monotonic();rows=[];binary=None;binary_verified=False;error=None
try:
 binary=pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target/debug/deps/writing_setup-0a70c929436e2f9d')
 assert hashlib.sha256(binary.read_bytes()).hexdigest()=='cd20fe93330b7b00eb3e9a226421f451e46620c53a308c4106eecafb7e7e2ca4'
 assert '79F53E61-A47D-382F-B602-4D3046124A83' in subprocess.check_output(['/usr/bin/dwarfdump','--uuid',str(binary)],text=True)
 binary_verified=True
 print('Original exact app binary,22 test workers; at most20 fresh process launches, stop on first nonzero or invalid result; diagnostic only.',flush=True)
 for attempt in range(1,21):
  if time.monotonic()-started>150:raise RuntimeError('cold campaign deadline before launch')
  if shutil.disk_usage(repo).free<1024**3:raise RuntimeError('cold campaign disk floor')
  child=subprocess.run([str(binary),'--test-threads=22'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=child.stdout
  accepted_size=len(log)<=64*1024
  passed=accepted_size and child.returncode==0 and b'test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in log
  rows.append({'attempt':attempt,'returncode':child.returncode,'expected_suite_pass':passed,'log_sha256':hashlib.sha256(log).hexdigest(),'log_size_bytes':len(log),'accepted_size':accepted_size,'log_persisted':False})
  print(json.dumps(rows[-1]),flush=True)
  (root/('cold-%02d.log'%attempt)).write_bytes(log)
  rows[-1]['log_persisted']=True
  if not accepted_size:raise RuntimeError('child log exceeds accepted bound')
  if not passed:break
except BaseException as exc:
 error={'type':type(exc).__name__,'message':str(exc)}
finally:
 receipt={'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'candidate':head,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'binary':str(binary) if binary else None,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest() if binary else None,'binary_verified':binary_verified,'test_harness_workers':22,'expected_test_count':22,'attempts':rows,'error':error,'qualified':False,'sigsegv_observed':bool(rows and rows[-1]['returncode']==-11),'all_20_cold_passed':len(rows)==20 and all(x['expected_suite_pass'] for x in rows),'elapsed_seconds':time.monotonic()-started,'scope':'one finite original-app cold-process diagnosis with22testworkers; no rebuild/fix/qualification, nativefaultrequiresmatchingstack, passesdonotrefutetiming-sensitivebug'}
 (root/'cold-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps(receipt),flush=True)
if error:sys.exit(1)
if receipt['sigsegv_observed']:sys.exit(101)
sys.exit(0 if receipt['all_20_cold_passed'] else 1)
