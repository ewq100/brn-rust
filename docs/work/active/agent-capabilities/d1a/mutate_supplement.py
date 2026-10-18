#!/usr/bin/env python3
"""Bounded continuation of the one D1A campaign; never resets its allowance."""
import pathlib, json, time, subprocess, os, signal, hashlib, tarfile, io, difflib
E=pathlib.Path(__file__).resolve().parent
R=E.parents[4]
B=json.loads((E/'mutation-supplement-budget.json').read_text())
PIN=B['pin']; START=B['segment_started_epoch']; DEADLINE=B['deadline_epoch']
LP=E/'mutation-supplement-ledger.json'
if LP.exists(): raise SystemExit('Existing segment: do not restart/reset; retain exact deadline/outcomes.')
W=pathlib.Path('/private/tmp/brn-d1a-20261016-target/mutation-202bf2d7')
S=W/'source'; T=W/'artifacts'; TMP=W/'synthetic-tmp'/'supplement'
TMP.mkdir(exist_ok=False)
env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true',TMPDIR=str(TMP))
L={'pin':PIN,'prior_used_seconds':B['prior_used_seconds'],'segment_started_epoch':START,'deadline_epoch':DEADLINE,'source':str(S),'target':str(T),'tmpdir':str(TMP),'records':[],'manual':[]}
def save():
 L['segment_elapsed_seconds']=time.time()-START;L['total_original_charged_seconds']=B['prior_used_seconds']+L['segment_elapsed_seconds'];LP.write_text(json.dumps(L,indent=2)+'\n')
def run(label,cmd,seconds=120):
 if time.time()>=DEADLINE-12: return {'label':label,'status':'Unexecuted','reason':'original budget deadline'}
 log=E/('supp-'+label+'.log');record={'label':label,'command':cmd,'source':str(S),'target':str(T),'started_elapsed':time.time()-START}
 with log.open('w') as out:
  p=subprocess.Popen(cmd,cwd=S,env=env,stdout=out,stderr=subprocess.STDOUT,start_new_session=True);record['pid']=p.pid;L['active']=record;save()
  try:record['exit_code']=p.wait(timeout=max(1,min(seconds,DEADLINE-time.time()-12)));record['status']='Success' if record['exit_code']==0 else 'Failure'
  except subprocess.TimeoutExpired:
   record['status']='Timeout';os.killpg(p.pid,signal.SIGINT)
   try:record['exit_code']=p.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);record['exit_code']=p.wait()
 record.update(ended_elapsed=time.time()-START,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest());L.pop('active',None);L['records'].append(record);save();return record
save()
paths=subprocess.check_output(['git','ls-tree','-r','--name-only',PIN,'--','crates','agent','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=R,text=True).splitlines()
# Same fixed disposable source and artifacts as the first campaign; never candidate/before roots.
archive=subprocess.check_output(['git','archive',PIN],cwd=R);tarfile.open(fileobj=io.BytesIO(archive)).extractall(S,filter='data')
for c in [['git','add','.'],['git','-c','user.name=BRN D1A mutation fixture','-c','user.email=fixture@example.invalid','commit','--quiet','-m','Synthetic corrected D1A '+PIN]]:
 result=run('setup-'+str(len(L['records'])),c,30)
 if result['status']!='Success':raise SystemExit('setup failed')
assert (S/'.git').is_dir()
assert subprocess.check_output(['git','remote'],cwd=S,text=True)==''
original={n:(S/n).read_bytes() for n in paths};hashes={n:hashlib.sha256(v).hexdigest() for n,v in original.items()}
for n,h in hashes.items():assert hashlib.sha256(subprocess.check_output(['git','show',PIN+':'+n],cwd=R)).hexdigest()==h,n
L['source_sha256']=hashes;L['archive_sha256']=hashlib.sha256(archive).hexdigest();save()
base=['cargo','test','--locked','--offline','-p','brn-ai','--features','scripted-runs','--lib']
baseline=run('baseline',base,120)
if baseline['status']!='Success':L['stop_reason']='actual copied baseline did not pass';save();raise SystemExit('baseline failed; no verdicts')
L['baseline_artifacts']=[{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [T/'debug/deps/brn_ai-28a197b27edef53b',T/'debug/deps/libbrn_threads_core-441e246c1aee373f.rlib',T/'debug/deps/brn_ai-28a197b27edef53b.d'] if p.exists()];save()
# Explicit seeded guards; planned labels are not cargo-mutants generated IDs.
manual=[
 ('partial-ledger-complete','crates/brn-ai/src/threads.rs','if markdown.len() == total {','if true {'),
 ('omitted-ledger-grant','crates/brn-ai/src/threads.rs','    let mut reads = scope.reads.lock().map_err(storage)?;','    let mut reads = scope.reads.lock().map_err(storage)?;\n    for row in out["omitted_records"].as_array().unwrap() { reads.complete(row["id"].as_str().unwrap(), row["version"].as_u64().unwrap()); }'),
 ('provider-local-origin-guard','crates/brn-ai/src/error.rs','"brn_request_size_estimate_exceeded" if detail.origin == "context_budget" =>','"brn_request_size_estimate_exceeded" =>'),
 ('encoded-fit-bypass','crates/brn-ai/src/threads_context_budget.rs','estimate(&rig::tool::ToolOutput::json(value.clone())) <= RESULT','true'),
 ('prior-local-origin-deletion','crates/brn-ai/src/threads_context_budget.rs','        origin: "context_budget".into(),\n',''),
 ('prior-result-equality','crates/brn-ai/src/threads_context_budget.rs','estimate(result.output()) > RESULT','estimate(result.output()) >= RESULT'),
 ('prior-helper-discussion-guard','crates/brn-ai/src/threads_search.rs','|| scope.scout.is_some()','&& scope.scout.is_some()'),
 ('prior-writing-discussion-guard','crates/brn-ai/src/threads_search.rs','|| scope.writing_snapshot.is_some()','&& scope.writing_snapshot.is_some()'),
]
try:
 for label,name,needle,replacement in manual:
  row={'id':'seed:'+label,'file':name,'source_pin':PIN};L['manual'].append(row)
  if time.time()>=DEADLINE-45:row['outcome']='Unexecuted';save();continue
  old=original[name].decode();assert needle in old,(label,needle);new=old.replace(needle,replacement,1)
  diff=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+name,tofile='b/'+name));(E/('supp-'+label+'.diff')).write_text(diff);row['diff_sha256']=hashlib.sha256(diff.encode()).hexdigest()
  (S/name).write_text(new);save()
  try:
   build=run(label+'-build',base+['--no-run'],90)
   if build['status']=='Timeout':row['outcome']='Timeout'
   elif build['status']=='Unexecuted':row['outcome']='Unexecuted'
   elif build['status']!='Success':row['outcome']='Unviable'
   else:
    tested=run(label+'-test',base,20);raw=(E/('supp-'+label+'-test.log')).read_text() if tested['status']!='Unexecuted' else ''
    row['outcome']='Missed' if tested['status']=='Success' else ('Timeout' if tested['status']=='Timeout' else ('Caught' if 'test result: FAILED.' in raw else 'Error'))
    row['failing_test_lines']=[x for x in raw.splitlines() if ' ... FAILED' in x]
  finally:(S/name).write_bytes(original[name])
  row['source_restored']=all(hashlib.sha256((S/n).read_bytes()).hexdigest()==h for n,h in hashes.items());assert row['source_restored'];save();print(json.dumps(row),flush=True)
 # Small new algorithm scope only, not the prior41 classifier/search campaign.
 diff=W/'supplement-new-guards.diff';diff.write_bytes(subprocess.check_output(['git','diff','d3f67954',PIN,'--','crates/brn-ai/src/threads.rs','crates/brn-ai/src/threads_context_budget.rs'],cwd=R));L['auto_diff_sha256']=hashlib.sha256(diff.read_bytes()).hexdigest()
 command=['cargo','mutants','--in-place','--in-diff',str(diff),'--file','crates/brn-ai/src/threads_context_budget.rs','--file','crates/brn-ai/src/threads.rs','--re','fit_prefix|record_note_prefix','--test-package','brn-ai','--features','scripted-runs','--cargo-arg=--lib','--cargo-arg=--locked','--cargo-arg=--offline','--timeout','15','--build-timeout','90','--colors','never','--no-times','--output',str(W/'supplement-new-guards')]
 listed=run('new-guards-list',command+['--list','--json'],30)
 if listed['status']=='Success':
  selected=json.loads((E/'supp-new-guards-list.log').read_text());L['auto_selected_count']=len(selected);save()
  result=run('new-guards',command,max(1,DEADLINE-time.time()-15));L['auto_result']=result
  outcomes=list((W/'supplement-new-guards').rglob('outcomes.json'))
  if outcomes:(E/'supp-new-guards-outcomes.json').write_bytes(outcomes[0].read_bytes())
finally:
 L['changed_source_files']=[n for n,h in hashes.items() if hashlib.sha256((S/n).read_bytes()).hexdigest()!=h]
 L['finished_epoch']=time.time();save();print(json.dumps({'segment_elapsed':L['segment_elapsed_seconds'],'total_original_charged':L['total_original_charged_seconds'],'changed_source_files':L['changed_source_files']}),flush=True)
