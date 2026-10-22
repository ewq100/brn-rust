import datetime,hashlib,json,os,pathlib,shutil,signal,subprocess,time
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust')
root=pathlib.Path('/tmp/brn-mac13-preparation')
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
main='3827d6a13953533c7322fd7157d975a32dee5cb2'
paths=['crates','agent','Cargo.toml','Cargo.lock','rust-toolchain.toml']
assert not subprocess.check_output(['git','diff',main,'--',*paths],cwd=repo)
assert not subprocess.check_output(['git','status','--porcelain','--',*paths],cwd=repo)
assert shutil.disk_usage(repo).free>=2*1024**3
script=root/'build.sh'
script.write_text('''#!/bin/zsh
set -eu
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-mac13-build
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
# Shared target last compiled privacy source in a worker checkout. Force the
# changed local app and CLI crates, retaining dependency and helper artifacts.
cargo clean -p brn -p brn-threads-app --target-dir "$CARGO_TARGET_DIR"
cargo build -p brn --locked --offline --jobs 1
''')
start=time.monotonic();p=None;reason=None;signals=[];cleanup_errors=[];exitcode=None
try:
 with (root/'build.log').open('wb') as log:
  p=subprocess.Popen(['zsh',str(script)],cwd=repo,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-start>900:reason='900s build deadline'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
   if reason:break
   time.sleep(3)
except BaseException as exc:
 reason='supervisor exception '+type(exc).__name__
finally:
 def present():
  try:os.killpg(p.pid,0);return True
  except ProcessLookupError:return False
 if p:
  try:
   for sig,label in [(signal.SIGTERM,'TERM'),(signal.SIGKILL,'KILL')]:
    p.poll()
    if not present():break
    try:os.killpg(p.pid,sig);signals.append(label)
    except ProcessLookupError:pass
    end=time.monotonic()+10
    while time.monotonic()<end:
     p.poll()
     if not present():break
     time.sleep(.1)
   p.wait(timeout=10);exitcode=p.returncode
   absent=not present()
  except BaseException as exc:
   cleanup_errors.append(type(exc).__name__);absent=False
 else:absent=True
 unchanged=head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip() and not subprocess.check_output(['git','diff',main,'--',*paths],cwd=repo)
 receipt={'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'head':head,'production_source':main,'runtime_paths':paths,'source_unchanged':unchanged,'seconds':time.monotonic()-start,'command':['cargo','build','-p','brn','--locked','--offline','--jobs','1'],'scoped_rebuild':['brn','brn-threads-app'],'exit_code':exitcode,'stop_reason':reason,'signals':signals,'group_absent':absent,'leader_reaped':p is not None and p.returncode is not None,'cleanup_errors':cleanup_errors,'qualified_build':exitcode==0 and reason is None and absent and unchanged and not cleanup_errors}
 if receipt['qualified_build']:
  artifact=pathlib.Path('/Users/evokessler/.codex/artifacts/brn-mac13-3827d6a1');artifact.mkdir(exist_ok=False)
  expected={'brn-xberg-helper':'4f46c81f3e649b4f86bd4a66c109fc86889e3fefac83b8777d9956662e74a36b','brn-xberg-helper-brn':'24662cd03614003e42687d7ec070c3e219a0ccbf65d38c2bf1160457fbce9ccc'}
  receipt['artifacts']={}
  for name in ['brn',*expected]:
   src=repo/'target/debug'/name;data=src.read_bytes();sha=hashlib.sha256(data).hexdigest()
   if name in expected:assert sha==expected[name]
   dst=artifact/name;shutil.copy2(src,dst);assert hashlib.sha256(dst.read_bytes()).hexdigest()==sha
   receipt['artifacts'][name]={'path':str(dst),'sha256':sha,'bytes':len(data)}
 (root/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps(receipt),flush=True)
raise SystemExit(0 if receipt['qualified_build'] else 1)
