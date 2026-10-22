from pathlib import Path
import datetime,hashlib,json,os,runpy,shutil,subprocess,time
repo=Path('/Users/evokessler/.codex/worktrees/export-durability/brn-rust');root=Path('/tmp/brn-tls-resolution');head='003edd71518300f545fbde2d71bf850470f7ff0e'
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()==head
assert not subprocess.check_output(['git','status','--porcelain'],cwd=repo,timeout=10)
assert shutil.disk_usage(repo).free>=3*1024**3
lockhash=hashlib.sha256((repo/'Cargo.lock').read_bytes()).hexdigest();assert lockhash=='2dcc9ad99f05129f077ebc063b0f65fe69bda993ef2bd4824da4d9a6be2b189f'
approved=Path('/tmp/brn-mac13-preparation/prepare.py');assert hashlib.sha256(approved.read_bytes()).hexdigest()=='cd272ff3734a2a46c2a83ff944a9d2f15d768e6bef45de03df49378009854813'
cleanup=runpy.run_path(str(approved))['stop_and_reap']
stages=[('workspace-tests',['cargo','test','--workspace','--no-fail-fast','--locked','--offline','--jobs','1']),('capability-tests',['cargo','test','-p','brn-ai','--features','capability-spike','--all-targets','--locked','--offline','--jobs','1']),('workspace-lint',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--jobs','1','--','-D','warnings']),('cli-build',['cargo','build','-p','brn','--bin','brn','--locked','--offline','--jobs','1']),('ordinary-native-ui',['cargo','build','-p','brn-desktop','--bin','brn-desktop','--features','native-ui','--locked','--offline','--jobs','1']),('native-retrieval-tests',['cargo','test','-p','brn-retrieval','--features','native','--lib','--test','model_download','--locked','--offline','--jobs','1']),('combined-native-build',['cargo','build','-p','brn-desktop','--bin','brn-desktop','--features','native-ui,native-retrieval','--locked','--offline','--jobs','1'])]
worktmp=Path('/private/tmp/brn-tls-qualification-20261022T1006');worktmp.mkdir(mode=0o700,exist_ok=False)
env={k:v for k,v in os.environ.items() if k in ['PATH','HOME','TMPDIR','LANG','LC_ALL','USER','LOGNAME','SHELL','SYSTEMROOT']}
receipt={'candidate':head,'lock_sha256':lockhash,'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'temporary_root':str(worktmp),'stages':[],'scope':'Existing offline synthetic suites and consumer compilation only; no account/model/GUI/TLS exploit proof'}
start=time.monotonic()
for n,(name,argv) in enumerate(stages,1):
 script=root/f'{n:02}-{name}.sh';script.write_text('#!/bin/zsh\nset -eu\n. /tmp/brn-launcher-focus-qualification/environment.sh\nexport TMPDIR='+str(worktmp)+'\nexport ORT_SKIP_DOWNLOAD=1 ORT_OFFLINE=1\n'+' '.join(argv)+'\n')
 logpath=root/f'{n:02}-{name}.log';p=None;reason=None;signals=[];errors=[];s=time.monotonic()
 try:
  with logpath.open('wb') as log:
   p=subprocess.Popen(['zsh',str(script)],cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
   while p.poll() is None:
    if time.monotonic()-start>=1770:reason='1800s total active deadline with30s cleanup reserve'
    elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
    elif logpath.stat().st_size>8*1024**2:reason='8MiB sampled log threshold'
    if reason:break
    time.sleep(2)
 except BaseException as exc:reason='supervisor exception '+type(exc).__name__
 finally:
  clean=p is not None and cleanup(p,signals,errors)
  row={'name':name,'argv':argv,'script':str(script),'log':str(logpath),'exit_code':None if p is None else p.returncode,'seconds':time.monotonic()-s,'stop_reason':reason,'signals':signals,'cleanup_errors':errors,'group_absent':clean,'leader_reaped':p is not None and p.returncode is not None}
  receipt['stages'].append(row);receipt.update(updated_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),seconds=time.monotonic()-start)
  (root/'qualification.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(row),flush=True)
 if row['exit_code']!=0 or reason or not clean or errors:break
assert head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()
assert not subprocess.check_output(['git','status','--porcelain'],cwd=repo,timeout=10)
assert lockhash==hashlib.sha256((repo/'Cargo.lock').read_bytes()).hexdigest()
receipt.update(source_unchanged=True,completed_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),all_selected_passed=len(receipt['stages'])==len(stages) and all(r['exit_code']==0 and r['stop_reason'] is None and r['group_absent'] and not r['cleanup_errors'] for r in receipt['stages']))
(root/'qualification.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'all_selected_passed':receipt['all_selected_passed'],'seconds':receipt['seconds']}),flush=True)
raise SystemExit(0 if receipt['all_selected_passed'] else 1)
