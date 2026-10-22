from pathlib import Path
import argparse,datetime,hashlib,json,os,runpy,shutil,subprocess,time
p=argparse.ArgumentParser();p.add_argument('--head',required=True);p.add_argument('--stage',choices=['baseline','focused','desktop','lint','ordinary-build'],required=True);p.add_argument('--test',default='');a=p.parse_args()
repo=Path('/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust');root=Path('/tmp/brn-connections-project-qualification')
def git(*args):return subprocess.check_output(['git',*args],cwd=repo,text=True,timeout=10).strip()
assert git('rev-parse','HEAD')==a.head and not git('status','--porcelain')
assert shutil.disk_usage(repo).free>=2*1024**3
cleanup_source=Path('/tmp/brn-mac13-preparation/prepare.py');assert hashlib.sha256(cleanup_source.read_bytes()).hexdigest()=='cd272ff3734a2a46c2a83ff944a9d2f15d768e6bef45de03df49378009854813'
cleanup=runpy.run_path(str(cleanup_source))['stop_and_reap']
out=root/("baseline-observed" if a.stage=="baseline" else a.stage);out.mkdir(mode=0o700,exist_ok=False);tmp=out/'data';tmp.mkdir(mode=0o700)
features=['--features','native-ui,native-test-support','--locked','--offline','--jobs','1']
commands={'baseline':['cargo','test','-p','brn-desktop',*features,a.test,'--','--nocapture'],'focused':['cargo','test','-p','brn-desktop',*features,a.test,'--','--nocapture'],'desktop':['cargo','test','-p','brn-desktop',*features],'lint':['cargo','clippy','-p','brn-desktop',*features,'--all-targets','--','-D','warnings'],'ordinary-build':['cargo','build','-p','brn-desktop','--bin','brn-desktop','--features','native-ui','--locked','--offline','--jobs','1']}
if a.stage in ['baseline','focused']:assert a.test and all(c.isalnum() or c=='_' for c in a.test)
env={k:v for k,v in os.environ.items() if k in ['PATH','HOME','LANG','LC_ALL','USER','LOGNAME','SHELL']}
tool=Path('/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin');target=Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target')
env.update(PATH=str(tool/'bin')+':'+env['PATH'],DYLD_LIBRARY_PATH=str(tool/'lib'),CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='1',CARGO_NET_OFFLINE='true',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',BRN_XBERG_HELPER=str(target/'debug/brn-xberg-helper'),BRN_XBERG_HELPER_BRN=str(target/'debug/brn-xberg-helper-brn'),TMPDIR=str(tmp))
argv=commands[a.stage];logpath=out/'output.log';start=time.monotonic();reason=None;signals=[];errors=[];child=None;absent=False
record={'head':a.head,'tree':git('rev-parse','HEAD^{tree}'),'stage':a.stage,'argv':argv,'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'temporary_root':str(tmp),'target':str(target),'scope':'Offline synthetic toolkit/compilation only; no native observation/account/model/library acquisition.'}
try:
 with logpath.open('wb') as f:
  child=subprocess.Popen(argv,cwd=repo,env=env,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
  while child.poll() is None:
   if time.monotonic()-start>=870:reason='900s deadline with30s cleanup reserve'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB live disk floor'
   elif logpath.stat().st_size>8*1024**2:reason='8MiB sampled log limit'
   if reason:break
   time.sleep(2)
except BaseException as exc:reason='supervisor exception '+type(exc).__name__
finally:
 if child is not None:absent=cleanup(child,signals,errors)
 record.update(completed_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),seconds=time.monotonic()-start,exit_code=None if child is None else child.returncode,stop_reason=reason,signals=signals,cleanup_errors=errors,group_absent=absent,leader_reaped=child is not None and child.returncode is not None,source_unchanged=git('rev-parse','HEAD')==a.head and not git('status','--porcelain'),log_sha256=hashlib.sha256(logpath.read_bytes()).hexdigest())
 (out/'result.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record),flush=True)
raise SystemExit(0 if record['exit_code']==0 and reason is None and absent and not errors and record['source_unchanged'] else 1)
