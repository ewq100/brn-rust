from pathlib import Path
import subprocess,os,time,json,hashlib,tarfile,io,difflib,signal,datetime,tempfile,sys
out=Path(__file__).parent;candidate=Path('/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc');root=out/'source';target=out/'target';start=json.loads((out/'start.json').read_text())['start_monotonic'];hard=start+600;stop=start+555
sha=lambda b:hashlib.sha256(b).hexdigest();now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat();events=[];active=None;pgids=[]
base=json.loads((candidate/'docs/work/active/modular-foundation/int-w/source-manifest.json').read_text()); originals={};variants=[];result={'start_utc':now(),'selection_frozen_before_baselines':True,'campaign_kind':'THREE manually seeded omission rechecks; not cargo-mutants','limit_seconds':600,'used_before_this_campaign':0,'copy':str(root),'target':str(target),'variants':variants,'commands':events}
def save():
 result['elapsed_seconds']=time.monotonic()-start;(out/'campaign.json').write_text(json.dumps(result,indent=2)+'\n')
def run(label,cmd,*,expected=None):
 global active
 if time.monotonic()>=stop:raise TimeoutError('reserved cleanup deadline reached before '+label)
 env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(target),CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true',GIT_CONFIG_GLOBAL='/dev/null',GIT_CONFIG_NOSYSTEM='1',TMPDIR=tempfile.mkdtemp(prefix=label+'-',dir=out),CARGO_BUILD_JOBS='8');env.pop('BRN_EMBEDDING_MODEL_DIR',None);env.pop('BRN_NATIVE_MODEL_DIR',None);env.pop('ORT_LIB_PATH',None)
 inputs={p:sha((root/p).read_bytes()) for p in base};log=out/(label+'.log');record={'label':label,'command':cmd,'cwd':str(root),'target':str(target),'start_utc':now(),'input_hashes':inputs,'TMPDIR':env['TMPDIR'],'CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'8'};before=time.monotonic()
 with log.open('wb') as f:
  active=subprocess.Popen(cmd,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT,start_new_session=True);pgids.append(active.pid);record['pid']=active.pid;record['pgid']=active.pid;save()
  try:code=active.wait(timeout=max(.01,stop-time.monotonic()))
  except subprocess.TimeoutExpired:
   os.killpg(active.pid,signal.SIGTERM)
   try:active.wait(timeout=2)
   except subprocess.TimeoutExpired:os.killpg(active.pid,signal.SIGKILL);active.wait()
   record['interrupted_for_budget']=True;code=active.returncode
 active=None;record.update(exit_code=code,seconds=time.monotonic()-before,log=str(log),log_sha256=sha(log.read_bytes()),end_utc=now());text=log.read_text();record['build_succeeded']='Finished `test` profile' in text;record['source_unchanged_during_command']=all(sha((root/p).read_bytes())==h for p,h in inputs.items());assert record['source_unchanged_during_command']
 import re
 record['counts']=[list(map(int,x)) for x in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;.*?(\d+) filtered out;',text)];record['artifacts']={}
 for f in re.findall(r'Running[^\n]*\((/[^)]+)\)',text):
  p=Path(f);d=p.with_suffix('.d');r={'sha256':sha(p.read_bytes()),'bytes':p.stat().st_size}
  if d.exists():r.update(depinfo=str(d),depinfo_sha256=sha(d.read_bytes()),depinfo_text=d.read_text())
  record['artifacts'][f]=r
 events.append(record);save();print(label,code,record['seconds'],record['counts'],flush=True)
 return record
try:
 root.mkdir();archive=subprocess.check_output(['git','archive','9a0ce46de0a0334af6a7215263a3ced422cbab8a'],cwd=candidate)
 with tarfile.open(fileobj=io.BytesIO(archive)) as tar:tar.extractall(root,filter='data')
 assert all(sha((root/p).read_bytes())==h for p,h in base.items())
 genv=os.environ.copy();genv.update(GIT_CONFIG_GLOBAL='/dev/null',GIT_CONFIG_NOSYSTEM='1')
 for cmd in [['git','init','-q','-b','synthetic-intw-guard'],['git','config','user.name','Synthetic INT-W'],['git','config','user.email','synthetic@example.invalid'],['git','add','.'],['git','commit','-q','-m','Synthetic frozen INT-W fixture']]:subprocess.run(cmd,cwd=root,env=genv,check=True)
 result['synthetic_git']={'HEAD':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True,env=genv).strip(),'remotes':subprocess.check_output(['git','remote','-v'],cwd=root,text=True,env=genv),'all_332_material_match':True}
 specs=[('M01','crates/brn-threads-app/src/export.rs','\n                && record_in_chat(r, &full_snapshot, None)','', 'app','general export leaked INTW_P1_TITLE'),('M02','crates/brn-desktop/src/threads_lists.rs','        .filter(|r| record_in_chat(r, records, None))\n','','native','a hidden reference must not contribute an inbound count'),('M03','crates/brn-desktop/src/threads_lists.rs','\n                    && record_in_chat(record, records, None)','','native','general Notes UI must not reveal a P1-private retained-source reference title')]
 for id,path,before,after,kind,message in specs:
  text=(root/path).read_text();assert text.count(before)==1,(id,text.count(before));originals[path]=text;changed=text.replace(before,after,1);patch=''.join(difflib.unified_diff(text.splitlines(True),changed.splitlines(True),fromfile='a/'+path,tofile='b/'+path));(out/(id+'.patch')).write_text(patch);variants.append({'id':id,'path':path,'original_sha256':sha(text.encode()),'variant_sha256':sha(changed.encode()),'patch_sha256':sha(patch.encode()),'kind':kind,'expected_assertion':message,'verdict':'NO_VERDICT','reason':'not executed'})
 result['frozen_selection_utc']=now();save()
 app=['cargo','+1.98.1','test','-p','brn-threads-app','--test','cw2_review','--locked','--offline','int_w_f21_private_context_never_enters_p2_or_general_delivery','--','--exact','--nocapture']
 native=['cargo','+1.98.1','test','-p','brn-desktop','--features','native-ui,native-test-support','--locked','--offline','threads_ui::writing_tests::int_w_general_notes_list_hides_p1_reference_with_scoped_retained_source','--','--exact','--nocapture']
 for label,cmd in [('baseline-app',app),('baseline-native',native)]:
  r=run(label,cmd)
  if r['exit_code']!=0:raise RuntimeError(label+' did not pass; no variants run')
 for spec,v in zip(specs,variants):
  id,path,before,after,kind,message=spec
  if time.monotonic()>=stop:break
  try:
   (root/path).write_text(originals[path].replace(before,after,1));r=run(id,app if kind=='app' else native);text=Path(r['log']).read_text();v.update(command_label=id,build_succeeded=r['build_succeeded'])
   if r['exit_code']==0:v.update(verdict='MISSED',reason='mutated fixture passed')
   elif r['build_succeeded'] and r['counts'] and r['counts'][-1][1]==1 and message in text and 'panicked at' in text and not r.get('interrupted_for_budget'):v.update(verdict='CAUGHT',reason='successful mutated compile followed by expected behavioral assertion failure',failure_excerpt=text[text.index('failures:'):])
   else:v.update(verdict='NO_VERDICT',reason='compile/interruption/unexpected assertion or fixture failure')
  finally:
   (root/path).write_text(originals[path]);v['restored_sha256']=sha((root/path).read_bytes());assert v['restored_sha256']==v['original_sha256'];save()
except BaseException as e:
 result['controller_issue']=repr(e);save();print(repr(e),flush=True)
finally:
 if active is not None:
  try:os.killpg(active.pid,signal.SIGKILL)
  except ProcessLookupError:pass
  active.wait()
 for path,text in originals.items():(root/path).write_text(text)
 restored={p:sha((root/p).read_bytes()) for p in base} if root.exists() else {};result['copy_all_332_restored']=restored==base;result['candidate_all_332_unchanged']=all(sha((candidate/p).read_bytes())==h for p,h in base.items());result['candidate_crates_tree']=subprocess.check_output(['git','rev-parse','HEAD:crates'],cwd=candidate,text=True).strip();result['own_pgids']=pgids
 # Reap every owned command group, including any compiler descendant left after its controller exited.
 for pgid in pgids:
  try:os.killpg(pgid,signal.SIGTERM)
  except ProcessLookupError:continue
  time.sleep(.1)
  try:os.killpg(pgid,signal.SIGKILL)
  except ProcessLookupError:pass
 heavy=[];owned=[]
 for line in subprocess.check_output(['ps','-axo','pid=,ppid=,pgid=,comm=,args='],text=True).splitlines():
  f=line.strip().split(None,4)
  if len(f)<4:continue
  if Path(f[3]).name in {'cargo','rustc','clippy-driver','cargo-clippy','cargo-mutants','rustdoc'}:heavy.append(line)
  if int(f[2]) in pgids:owned.append(line)
 result.update(final_heavy_processes=heavy,remaining_own_group_processes=owned,end_utc=now(),elapsed_seconds=time.monotonic()-start,budget_exhausted=time.monotonic()>hard);save();print(json.dumps({k:v for k,v in result.items() if k not in ['commands','variants']},indent=2),flush=True)
