#!/usr/bin/env python3
"""Only targeted rechecks in the ORIGINAL campaign's actually remaining time."""
import hashlib,json,os,pathlib,re,signal,subprocess,time
root=pathlib.Path(__file__).resolve().parents[5];evidence=pathlib.Path(__file__).resolve().parent
previous=json.loads((evidence/'mutation-supplement.json').read_text());used=previous['total_shared_seconds']
if any('ended_total_elapsed' not in p for p in previous['phases']):raise SystemExit('Previous controller active')
if (evidence/'mutation-guard-recheck.json').exists():raise SystemExit('Never restart a recheck budget')
if subprocess.check_output(['git','diff','HEAD','--','crates'],cwd=root):raise SystemExit('Dirty source')
pin=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip();start=time.monotonic();deadline=start+1800-used
work=pathlib.Path('/private/tmp/brn-cw2-mutation-recheck-'+pin[:8]);work.mkdir(exist_ok=False)
diff=work/'source.diff';diff.write_bytes(subprocess.check_output(['git','diff','3e09b979..'+pin,'--','crates'],cwd=root))
lines=(root/'crates/brn-threads-core/src/store.rs').read_text().splitlines();scope=next(i+1 for i,s in enumerate(lines) if s.strip()=='input.record != snapshot.note.id');historical=max(i+1 for i,s in enumerate(lines) if s.strip()=='.is_none_or(|r| r.archived || !crate::context::visible(r, &context_rows, chat))')
route=next(i+1 for i,s in enumerate((root/'crates/brn-threads-app/src/review.rs').read_text().splitlines()) if 'if route.snapshot.note.id != note' in s)
env=os.environ.copy();env.update(CARGO_TARGET_DIR=previous['target'],CARGO_INCREMENTAL='0')
r={'pin':pin,'crates_tree':subprocess.check_output(['git','rev-parse','HEAD:crates'],cwd=root,text=True).strip(),'original_pin':previous['original_pin'],'prior_total_shared_seconds':used,'cap_seconds':1800,'target':env['CARGO_TARGET_DIR'],'work':str(work),'diff_sha256':hashlib.sha256(diff.read_bytes()).hexdigest(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'phases':[],'qualification':'New-pin actual rechecks, never retroactive kills of original f282/144848 verdicts. No preference-only mutant added.'}
base=['cargo','mutants','--in-diff',str(diff),'--jobs','1','--timeout','12','--build-timeout','70','--cargo-arg=--locked','--cargo-arg=--offline','--colors','never','--no-times','--leak-dirs','--copy-vcs=true']
phases=[('core-scope-history-token',105,['-p','brn-threads-core','--file','crates/brn-threads-core/src/store.rs','--re',f'Store::change_token|:{scope}:|:{historical}:','--test-package','brn-threads-core','--cargo-arg=--test=cw2_visibility']),('app-request-route',105,['-p','brn-threads-app','--file','crates/brn-threads-app/src/review.rs','--re',f':{route}:','--test-package','brn-threads-app','--cargo-arg=--lib','--cargo-arg=--test=writing_context'])]
def save():
 r['recheck_elapsed_seconds']=time.monotonic()-start;r['total_shared_seconds']=used+r['recheck_elapsed_seconds']
 (evidence/'mutation-guard-recheck.json').write_text(json.dumps(r,indent=2)+'\n')
save()
try:
 for name,budget,args in phases:
  if time.monotonic()>=deadline-20:break
  command=base+args+['--output',str(work/name)];p={'name':name,'command':command,'started_total_elapsed':used+time.monotonic()-start,'log':str(work/(name+'.log'))};r['phases'].append(p);save()
  try:
   listing=subprocess.run(command+['--list','--json'],cwd=root,env=env,capture_output=True,text=True,timeout=max(1,deadline-time.monotonic()-20))
   (work/(name+'-selected.json')).write_text(listing.stdout);(work/(name+'-selected.stderr')).write_text(listing.stderr);p['listing_exit']=listing.returncode
   if listing.returncode:continue
   p['selected']=len(json.loads(listing.stdout));save()
   with open(p['log'],'w') as log:
    child=subprocess.Popen(command,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    try:p['exit_code']=child.wait(timeout=max(1,min(budget,deadline-time.monotonic()-20)));p['interrupted']=False
    except subprocess.TimeoutExpired:
     p['interrupted']=True;os.killpg(child.pid,signal.SIGINT)
     try:p['exit_code']=child.wait(timeout=10)
     except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);p['exit_code']=child.wait()
   try:os.killpg(child.pid,signal.SIGKILL)
   except ProcessLookupError:pass
   p['log_sha256']=hashlib.sha256(pathlib.Path(p['log']).read_bytes()).hexdigest()
  finally:p['ended_total_elapsed']=used+time.monotonic()-start;save();print(json.dumps(p),flush=True)
finally:save();print(json.dumps({'total_shared_seconds':r['total_shared_seconds'],'cap':1800}),flush=True)
