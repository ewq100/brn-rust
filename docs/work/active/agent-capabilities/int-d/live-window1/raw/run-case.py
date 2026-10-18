import pathlib,json,hashlib,tempfile,subprocess,os,time,sys,signal
n=int(sys.argv[1]);assert 2<=n<=8
base=pathlib.Path('/private/tmp/brn-int-d-qualification-20261017-window1-f1a5624f');ev=pathlib.Path('/private/tmp/brn-int-d-live-window1-f1a5624f-evidence');p=json.loads((base/'preflight.json').read_text());w=p['workflows'][n-1];sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(base/'preflight.json')=='0148ad45fa1ed0cb38d7bd52274546bf13b5f06ddb64633ca713f95a5617547d';assert sha(pathlib.Path(w['facts']['data_dir'])/'threads.sqlite3')==w['initial_database_sha256'];assert sha(pathlib.Path(w['prompt']))==w['prompt_sha256']
corpus=json.loads(pathlib.Path('docs/work/active/agent-capabilities/int-d/corpus.json').read_text());assert sha(pathlib.Path('docs/work/active/agent-capabilities/int-d/corpus.json'))==p['corpus_sha256']
if w['continue_command']:assert pathlib.Path(w['continue_command'][w['continue_command'].index('--prompt-file')+1]).read_text()==corpus['continue_prompt']+'\n'
for row in p['compiled_artifacts'].values():assert sha(pathlib.Path(row['path']))==row['sha256'] and sha(pathlib.Path(row['dep_info']))==row['dep_info_sha256']
ledger=json.loads((base/'observations/ledger.json').read_text());assert ledger['finished_cases']==list(range(1,n))
for r in ledger['starts']:
 assert r.get('exit') in [0,130] and r.get('exposed_metrics',{}).get('terminal') in ['Completed','Interrupted']
 assert not r.get('candidate_inspection') or r['candidate_inspection']['exit']==0
assert not (ev/f'case{n}-controller.json').exists()
cmd=['python3','docs/work/active/agent-capabilities/int-d/observe-workflow.py','--preflight',str(base/'preflight.json'),'--activation',str(base/'activation.json'),'--ordinal',str(n)];tmp=tempfile.mkdtemp(prefix=f'brn-int-d-live-case{n}-',dir='/private/tmp');env=dict(os.environ,TMPDIR=tmp);began=time.monotonic()
with (ev/f'case{n}-observer.stdout').open('xb') as out,(ev/f'case{n}-observer.stderr').open('xb') as err:
 child=subprocess.Popen(cmd,env=env,stdout=out,stderr=err,start_new_session=True);r={'ordinal':n,'command':cmd,'observer_pid':child.pid,'observer_pgid':os.getpgid(child.pid),'tmpdir':tmp,'initial_database_sha256':w['initial_database_sha256'],'started':time.time()};(ev/f'case{n}-controller.json').write_text(json.dumps(r,indent=2)+'\n')
 try:r['exit']=child.wait(timeout=300)
 except subprocess.TimeoutExpired:
  os.killpg(child.pid,signal.SIGINT);r['controller_timeout']=True
  try:r['exit']=child.wait(timeout=10)
  except subprocess.TimeoutExpired:
   os.killpg(child.pid,signal.SIGTERM)
   try:r['exit']=child.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);r['exit']=child.wait(timeout=5)
 r['elapsed_seconds']=time.monotonic()-began;(ev/f'case{n}-controller.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps(r,indent=2))
