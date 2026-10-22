import pathlib,subprocess,json,hashlib,time,sys,datetime,shutil
root=pathlib.Path('/tmp/brn-sqlite-cold-start-baseline')
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/export-durability/brn-rust')
head='4c786a962ed52998648a0ae73232c61a0b41a5e9'
source=repo/'crates/brn-threads-core/tests/sqlite_startup.rs'
test_name='simultaneous_cold_store_opens_initialize_independent_workspaces'
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert hashlib.sha256(source.read_bytes()).hexdigest()=='73d0eae7229c66eaaed50dcd7340f0a75480e1b3c6e246add0c09474a3391023'
assert shutil.disk_usage(repo).free>3*1024**3
started=time.monotonic();rows=[];binary=None;compile_code=None;error=None
try:
 build=subprocess.run(['cargo','test','-p','brn-threads-core','--test','sqlite_startup','--no-run','--message-format=json','--locked','--offline','--jobs','1'],stdout=subprocess.PIPE)
 compile_code=build.returncode
 if compile_code!=0:raise RuntimeError('witness compilation failed')
 if len(build.stdout)>8*1024*1024:raise RuntimeError('compiler JSON exceeds accepted bound')
 (root/'compile.jsonl').write_bytes(build.stdout)
 artifacts=[json.loads(line) for line in build.stdout.splitlines() if line.strip()]
 matches=[x for x in artifacts if x.get('reason')=='compiler-artifact' and x.get('target',{}).get('name')=='sqlite_startup' and x.get('executable') and pathlib.Path(x['target']['src_path'])==source]
 assert len(matches)==1
 binary=pathlib.Path(matches[0]['executable'])
 assert binary.resolve().is_relative_to(pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target').resolve())
 binary_hash=hashlib.sha256(binary.read_bytes()).hexdigest()
 print('Built one test-only cold-start witness; at most20 fresh process launches, stop on first nonzero or invalid result.',flush=True)
 for attempt in range(1,21):
  if time.monotonic()-started>150:raise RuntimeError('cold campaign deadline before launch')
  if shutil.disk_usage(repo).free<1024**3:raise RuntimeError('cold campaign disk floor')
  child=subprocess.run([str(binary),'--exact',test_name,'--test-threads=1'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=child.stdout
  accepted_size=len(log)<=64*1024
  passed=accepted_size and child.returncode==0 and b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in log
  rows.append({'attempt':attempt,'returncode':child.returncode,'one_test_pass':passed,'log_sha256':hashlib.sha256(log).hexdigest(),'log_size_bytes':len(log),'accepted_size':accepted_size,'log_persisted':False})
  print(json.dumps(rows[-1]),flush=True)
  (root/('cold-%02d.log'%attempt)).write_bytes(log)
  rows[-1]['log_persisted']=True
  if not accepted_size:raise RuntimeError('child log exceeds accepted bound')
  if not passed:break
except BaseException as exc:
 error={'type':type(exc).__name__,'message':str(exc)}
finally:
 receipt={'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'candidate':head,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'binary':str(binary) if binary else None,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest() if binary else None,'compile_exit':compile_code,'attempts':rows,'error':error,'qualified':False,'sigsegv_observed':bool(rows and rows[-1]['returncode']==-11),'all_20_cold_passed':len(rows)==20 and all(x['one_test_pass'] for x in rows),'elapsed_seconds':time.monotonic()-started,'scope':'one finite cold-process diagnosis; a native fault requires matching stack classification, and passes do not refute a timing-sensitive bug'}
 (root/'cold-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps(receipt),flush=True)
if error:sys.exit(1)
if receipt['sigsegv_observed']:sys.exit(101)
sys.exit(0 if receipt['all_20_cold_passed'] else 1)
