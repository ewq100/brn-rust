#!/usr/bin/env python3
"""Changed-pin guard checks within the ORIGINAL single shared 1800s cap."""
import hashlib,json,os,pathlib,signal,subprocess,time
root=pathlib.Path(__file__).resolve().parents[5]; evidence=pathlib.Path(__file__).resolve().parent
original=json.loads((evidence/'mutation-run.json').read_text()); used=original['elapsed_seconds']
if any('ended_elapsed' not in p for p in original['phases']):raise SystemExit('Original controller still running')
if (evidence/'mutation-supplement.json').exists():raise SystemExit('Supplement already attempted; never reset budget')
if subprocess.check_output(['git','diff','HEAD','--','crates'],cwd=root):raise SystemExit('Dirty source')
pin=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
start=time.monotonic();deadline=start+1800-used
work=pathlib.Path('/private/tmp/brn-cw2-mutation-supplement-'+pin[:8]);work.mkdir(exist_ok=False)
diff=work/'source.diff';diff.write_bytes(subprocess.check_output(['git','diff','3e09b979..'+pin,'--','crates'],cwd=root))
env=os.environ.copy();env.update(CARGO_TARGET_DIR=original['target'],CARGO_INCREMENTAL='0')
r={'pin':pin,'crates_tree':subprocess.check_output(['git','rev-parse','HEAD:crates'],cwd=root,text=True).strip(),'original_pin':original['pin'],'original_elapsed_seconds':used,'cap_seconds':1800,'target':env['CARGO_TARGET_DIR'],'work':str(work),'diff_sha256':hashlib.sha256(diff.read_bytes()).hexdigest(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'phases':[]}
base=['cargo','mutants','--in-diff',str(diff),'--jobs','1','--timeout','12','--build-timeout','90','--cargo-arg=--locked','--cargo-arg=--offline','--colors','never','--no-times','--leak-dirs','--copy-vcs=true']
phases=[
 ('core-critical',480,['-p','brn-threads-core','--file','crates/brn-threads-core/src/store.rs','--re','replace_candidate|apply_on','--test-package','brn-threads-core','--test-package','brn-threads-app','--cargo-arg=--package=brn-threads-app','--cargo-arg=--test=cw2_visibility','--cargo-arg=--test=cw2_review','--cargo-arg=--test=writing','--cargo-arg=--test=writing_setup','--cargo-arg=--test=current_inputs']),
 ('ai-critical',450,['-p','brn-ai','--file','crates/brn-ai/src/threads.rs','--file','crates/brn-ai/src/threads_search.rs','--file','crates/brn-ai/src/threads_reads.rs','--file','crates/brn-ai/src/threads_research.rs','--re','writing_read_allowed|attach_writing_evidence|ReadLedger::covers|verify_writing_quotation|search_notes|note_connections|writing_request|prepare ','--features','scripted-runs','--cargo-arg=--lib']),
 ('app-changed',270,['-p','brn-threads-app','--file','crates/brn-threads-app/src/writing.rs','--file','crates/brn-threads-app/src/review.rs','--file','crates/brn-threads-app/src/run.rs','--re','writing_context_review|active_writing_context_from|select_writing_conversation|writing_request_in_context_from|writing_candidate_review_from|resolve_run_selection','--test-package','brn-threads-app','--cargo-arg=--test=cw2_review','--cargo-arg=--test=writing_context','--cargo-arg=--test=shared_run','--cargo-arg=--test=writing','--cargo-arg=--test=writing_setup'])]
def save():
 r['supplement_elapsed_seconds']=time.monotonic()-start;r['total_shared_seconds']=used+r['supplement_elapsed_seconds']
 (evidence/'mutation-supplement.json').write_text(json.dumps(r,indent=2)+'\n')
save()
try:
 for name,budget,args in phases:
  if time.monotonic()>=deadline-20:break
  command=base+args+['--output',str(work/name)]
  phase={'name':name,'command':command,'started_total_elapsed':used+time.monotonic()-start,'log':str(work/(name+'.log'))};r['phases'].append(phase);save()
  try:
   listing=subprocess.run(command+['--list','--json'],cwd=root,env=env,capture_output=True,text=True,timeout=max(1,deadline-time.monotonic()-20))
   (work/(name+'-selected.json')).write_text(listing.stdout);(work/(name+'-selected.stderr')).write_text(listing.stderr)
   phase['listing_exit']=listing.returncode
   if listing.returncode:continue
   phase['selected']=len(json.loads(listing.stdout));save()
   with open(phase['log'],'w') as log:
    child=subprocess.Popen(command,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    try:phase['exit_code']=child.wait(timeout=max(1,min(budget,deadline-time.monotonic()-20)));phase['interrupted']=False
    except subprocess.TimeoutExpired:
     phase['interrupted']=True;os.killpg(child.pid,signal.SIGINT)
     try:phase['exit_code']=child.wait(timeout=10)
     except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);phase['exit_code']=child.wait()
   try:os.killpg(child.pid,signal.SIGKILL)
   except ProcessLookupError:pass
   phase['log_sha256']=hashlib.sha256(pathlib.Path(phase['log']).read_bytes()).hexdigest()
  finally:phase['ended_total_elapsed']=used+time.monotonic()-start;save();print(json.dumps(phase),flush=True)
finally:save();print(json.dumps({'total_shared_seconds':r['total_shared_seconds'],'cap':1800}),flush=True)
