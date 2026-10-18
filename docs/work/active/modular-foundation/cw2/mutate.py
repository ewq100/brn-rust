#!/usr/bin/env python3
"""One serial, offline, synthetic targeted campaign with a shared 1800s cap."""
import hashlib, json, os, pathlib, signal, subprocess, time
root=pathlib.Path(__file__).resolve().parents[5]
evidence=pathlib.Path(__file__).resolve().parent
pin=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
if subprocess.check_output(['git','diff','HEAD','--','crates'],cwd=root):
    raise SystemExit('Refuse mutation with dirty crate source')
work=pathlib.Path('/private/tmp/brn-cw2-mutation-'+pin[:8]);work.mkdir(exist_ok=False)
diff=work/'source.diff';diff.write_bytes(subprocess.check_output(['git','diff','3e09b979dbe5204098d871469d0ed280186db20d..'+pin,'--','crates'],cwd=root))
common=['cargo','mutants','--in-diff',str(diff),'--jobs','1','--timeout','12','--build-timeout','90','--cargo-arg=--locked','--cargo-arg=--offline','--colors','never','--no-times','--leak-dirs']
phases=[
 ('core',540,['-p','brn-threads-core','--file','crates/brn-threads-core/src/store.rs','--file','crates/brn-threads-core/src/context.rs','--file','crates/brn-threads-core/src/writing.rs','--test-package','brn-threads-core','--test-package','brn-threads-app','--cargo-arg=--test=cw2_visibility','--cargo-arg=--test=cw2_review','--cargo-arg=--test=writing','--cargo-arg=--test=writing_setup','--cargo-arg=--test=current_inputs']),
 ('ai',640,['-p','brn-ai','--file','crates/brn-ai/src/threads.rs','--file','crates/brn-ai/src/threads_reads.rs','--file','crates/brn-ai/src/threads_search.rs','--file','crates/brn-ai/src/threads_research.rs','--re','attach_writing_evidence|writing_read_allowed|ReadLedger::covers|verify_writing_quotation|search_notes|note_connections|writing_request|prepare ','--features','scripted-runs','--cargo-arg=--lib']),
 ('app',580,['-p','brn-threads-app','--file','crates/brn-threads-app/src/writing.rs','--file','crates/brn-threads-app/src/review.rs','--file','crates/brn-threads-app/src/run.rs','--test-package','brn-threads-app','--cargo-arg=--test=cw2_review','--cargo-arg=--test=writing_context','--cargo-arg=--test=shared_run','--cargo-arg=--test=writing','--cargo-arg=--test=writing_setup'])]
env=os.environ.copy();env.update(CARGO_TARGET_DIR='/private/tmp/brn-cw2-20261016-target',CARGO_INCREMENTAL='0')
start=time.monotonic();deadline=start+1800
receipt={'pin':pin,'crates_tree':subprocess.check_output(['git','rev-parse','HEAD:crates'],cwd=root,text=True).strip(),'diff_sha256':hashlib.sha256(diff.read_bytes()).hexdigest(),'target':env['CARGO_TARGET_DIR'],'work':str(work),'cap_seconds':1800,'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'phases':[]}
def save():
 receipt['elapsed_seconds']=time.monotonic()-start
 (evidence/'mutation-run.json').write_text(json.dumps(receipt,indent=2)+'\n')
for name,budget,args in phases:
 if time.monotonic()>=deadline-20:break
 command=common+args+['--output',str(work/name)]
 # Listing preserves exact selected IDs without a build or test.
 listing=subprocess.run(command+['--list','--json'],cwd=root,env=env,capture_output=True,text=True)
 (work/(name+'-selected.json')).write_text(listing.stdout)
 (work/(name+'-selected.stderr')).write_text(listing.stderr)
 if listing.returncode:raise SystemExit(listing.stderr)
 phase={'name':name,'command':command,'selected':len(json.loads(listing.stdout)),'log':str(work/(name+'.log')),'started_elapsed':time.monotonic()-start}
 receipt['phases'].append(phase);save()
 with open(phase['log'],'w') as log:
  child=subprocess.Popen(command,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  try:phase['exit_code']=child.wait(timeout=min(budget,deadline-time.monotonic()-20));phase['interrupted']=False
  except subprocess.TimeoutExpired:
   phase['interrupted']=True
   os.killpg(child.pid,signal.SIGINT)
   try:phase['exit_code']=child.wait(timeout=10)
   except subprocess.TimeoutExpired:
    os.killpg(child.pid,signal.SIGKILL);phase['exit_code']=child.wait()
 # Reap any process-group descendant before the next serial phase.
 try:os.killpg(child.pid,signal.SIGKILL)
 except ProcessLookupError:pass
 phase['ended_elapsed']=time.monotonic()-start
 phase['log_sha256']=hashlib.sha256(pathlib.Path(phase['log']).read_bytes()).hexdigest()
 save()
 print(json.dumps(phase),flush=True)
save()
print(json.dumps({'elapsed_seconds':receipt['elapsed_seconds'],'work':str(work)}),flush=True)
