from pathlib import Path
import subprocess,os,time,json,hashlib,runpy,shutil,datetime
repo=Path('/Users/evokessler/.codex/worktrees/export-durability/brn-rust');root=Path('/tmp/brn-tls-resolution')
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()=='003edd71518300f545fbde2d71bf850470f7ff0e'
assert not subprocess.check_output(['git','status','--porcelain'],cwd=repo,timeout=10)
assert shutil.disk_usage(repo).free>=2*1024**3
(root/'Cargo.lock.before-rustls').write_bytes((repo/'Cargo.lock').read_bytes())
helper=Path('/tmp/brn-mac13-preparation/prepare.py');assert hashlib.sha256(helper.read_bytes()).hexdigest()=='cd272ff3734a2a46c2a83ff944a9d2f15d768e6bef45de03df49378009854813';cleanup=runpy.run_path(str(helper))['stop_and_reap']
argv=['/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo','update','-p','rustls@0.23.45','--precise','0.23.46']
env={k:v for k,v in os.environ.items() if k in ['PATH','HOME','TMPDIR','LANG','LC_ALL','USER','LOGNAME','SHELL','SSL_CERT_FILE','SSL_CERT_DIR','HTTP_PROXY','HTTPS_PROXY','NO_PROXY','http_proxy','https_proxy','no_proxy']};env.update(CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0',CARGO_HTTP_TIMEOUT='20')
start=time.monotonic();p=None;reason=None;signals=[];errors=[];logpath=root/'update-rustls.log'
try:
 with logpath.open('wb') as log:
  p=subprocess.Popen(argv,cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-start>=120:reason='120s active deadline plusbounded22s cleanup'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB diskfloor'
   elif logpath.stat().st_size>1024**2:reason='1MiB sampledlogthreshold'
   if reason:break
   time.sleep(.25)
except BaseException as exc:reason='supervisorexception '+type(exc).__name__
finally:
 clean=p is not None and cleanup(p,signals,errors)
 r={'argv':argv,'exit_code':None if p is None else p.returncode,'seconds':time.monotonic()-start,'stop_reason':reason,'signals':signals,'cleanup_errors':errors,'group_absent':clean,'leader_reaped':p is not None and p.returncode is not None,'completed_at':datetime.datetime.now(datetime.timezone.utc).isoformat()};(root/'rustls-update.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r),flush=True)
assert r['exit_code']==0 and reason is None and clean and not errors
(root/'Cargo.lock.rustls-resolver-original').write_bytes((repo/'Cargo.lock').read_bytes())
