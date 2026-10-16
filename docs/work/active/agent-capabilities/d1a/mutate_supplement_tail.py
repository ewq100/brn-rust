#!/usr/bin/env python3
"""Continue auto tail only, with the original immutable supplement deadline."""
import pathlib,json,time,subprocess,os,signal,hashlib
E=pathlib.Path(__file__).resolve().parent;LP=E/'mutation-supplement-ledger.json';L=json.loads(LP.read_text());S=pathlib.Path(L['source']);T=pathlib.Path(L['target']);D=L['deadline_epoch'];START=L['segment_started_epoch'];W=S.parent
L.setdefault('controller_stops',[]).append({'finished_epoch':L.pop('finished_epoch',None),'reason':'first list lacked explicit -p brn-ai; stdout empty, stderr No mutants to filter; parser error retained; no auto verdicts'})
L['auto_list_correction']='qualify brn-ai package explicitly; split stdout/stderr; continuation only, no manual restart/deadline reset'
env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true',TMPDIR=L['tmpdir'])
def save():
 L['segment_elapsed_seconds']=time.time()-START;L['total_original_charged_seconds']=L['prior_used_seconds']+L['segment_elapsed_seconds'];LP.write_text(json.dumps(L,indent=2)+'\n')
def run(label,cmd,seconds):
 if time.time()>=D-12:return {'label':label,'status':'Unexecuted','reason':'original deadline'}
 log=E/('supp-'+label+'.log');err=E/('supp-'+label+'.stderr');r={'label':label,'command':cmd,'source':str(S),'target':str(T),'started_elapsed':time.time()-START}
 with log.open('w') as out,err.open('w') as errors:
  p=subprocess.Popen(cmd,cwd=S,env=env,stdout=out,stderr=errors,start_new_session=True);r['pid']=p.pid;L['active']=r;save()
  try:r['exit_code']=p.wait(timeout=max(1,min(seconds,D-time.time()-12)));r['status']='Success' if r['exit_code']==0 else 'Failure'
  except subprocess.TimeoutExpired:
   r['status']='Timeout';os.killpg(p.pid,signal.SIGINT)
   try:r['exit_code']=p.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);r['exit_code']=p.wait()
 r.update(ended_elapsed=time.time()-START,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),stderr_sha256=hashlib.sha256(err.read_bytes()).hexdigest());L.pop('active',None);L['records'].append(r);save();return r
assert all(hashlib.sha256((S/n).read_bytes()).hexdigest()==h for n,h in L['source_sha256'].items())
command=L['records'][-1]['command'][:-2]
assert command[-2:]!=['--list','--json']
command[2:2]=['-p','brn-ai']
try:
 listed=run('new-guards-list-qualified',command+['--list','--json'],30)
 if listed['status']=='Success':
  raw=(E/'supp-new-guards-list-qualified.log').read_text();selected=json.loads(raw) if raw.strip() else [];L['auto_selected_count']=len(selected);save();print('selected',len(selected),'new algorithm variants',flush=True)
  if selected:
   L['auto_result']=run('new-guards-qualified',command,max(1,D-time.time()-15))
   outcomes=list((W/'supplement-new-guards').rglob('outcomes.json'))
   if outcomes:(E/'supp-new-guards-outcomes.json').write_bytes(outcomes[0].read_bytes())
finally:
 L['changed_source_files']=[n for n,h in L['source_sha256'].items() if hashlib.sha256((S/n).read_bytes()).hexdigest()!=h];L['finished_epoch']=time.time();save();print(json.dumps({'elapsed':L['segment_elapsed_seconds'],'total_original_charged':L['total_original_charged_seconds'],'changed_source_files':L['changed_source_files']}),flush=True)
