import datetime,hashlib,json,os,pathlib,shutil,signal,subprocess,time

root=pathlib.Path('/tmp/brn-recovery-copy-remaining-qualification')
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust')
candidate='ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b'


def record_error(state,operation,error):
 state['errors'].append({'operation':operation,'error_type':type(error).__name__})


def group_present(proc,state):
 state['group_probes']+=1
 try:
  os.killpg(proc.pid,0)
  state['last_group_status']='present'
  return True
 except ProcessLookupError:
  state['last_group_status']='absent'
  return False
 except BaseException as error:
  record_error(state,'probe_process_group',error)
  state['last_group_status']='unknown'
  return None


def signal_group(proc,sig,label,state):
 attempt={'signal':label,'outcome':'sent'}
 state['signals'].append(attempt)
 try:
  os.killpg(proc.pid,sig)
 except ProcessLookupError:
  attempt['outcome']='already_absent'
  state['last_group_status']='absent'
 except BaseException as error:
  attempt['outcome']='error'
  attempt['error_type']=type(error).__name__
  record_error(state,'signal_process_group_'+label,error)


def wait_group_absent(proc,timeout,state,label):
 deadline=time.monotonic()+timeout
 while True:
  status=group_present(proc,state)
  if status is False:
   state[label+'_group_absent']=True
   return True
  if time.monotonic()>=deadline:
   state[label+'_group_absent']=False
   return False
  try:
   time.sleep(0.1)
  except BaseException as error:
   record_error(state,label+'_group_wait',error)
   state[label+'_group_absent']=False
   return False


started=time.monotonic();reason=None;code=None;p=None;exception=None
checkout_unchanged=False;group_resolved=True;receipt_written=False;receipt_error=None
shutdown={'signals':[],'errors':[],'group_probes':0,'last_group_status':'not_started',
          'term_group_absent':None,'kill_group_absent':None,'leader_reaped':False,
          'resolved':True}
try:
 with (root/'run.log').open('wb') as log:
  p=subprocess.Popen(['bash',str(root/'run.sh')],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-started>1800:reason='30-minute qualification deadline'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
   if reason:break
   time.sleep(5)
except BaseException as error:
 exception=type(error).__name__
 reason='supervisor exception'
finally:
 if p is not None:
  try:
   status=group_present(p,shutdown)
   if status is not False:
    signal_group(p,signal.SIGTERM,'TERM',shutdown)
    term_absent=wait_group_absent(p,10,shutdown,'term')
   else:
    term_absent=True
    shutdown['term_group_absent']=True
   if not term_absent:
    signal_group(p,signal.SIGKILL,'KILL',shutdown)
    wait_group_absent(p,10,shutdown,'kill')
  except BaseException as error:
   record_error(shutdown,'term_kill_sequence',error)
   try:
    signal_group(p,signal.SIGKILL,'KILL',shutdown)
    wait_group_absent(p,10,shutdown,'kill_after_error')
   except BaseException as kill_error:
    record_error(shutdown,'kill_after_error',kill_error)
  try:
   p.wait(timeout=10)
   shutdown['leader_reaped']=True
  except BaseException as error:
   record_error(shutdown,'reap_leader',error)
   try:
    shutdown['leader_reaped']=p.poll() is not None
   except BaseException as poll_error:
    record_error(shutdown,'check_leader',poll_error)
  try:
   final_group_status=group_present(p,shutdown)
   group_resolved=(final_group_status is False and shutdown['leader_reaped'])
  except BaseException as error:
   record_error(shutdown,'final_group_check',error)
   group_resolved=False
  try:
   code=p.returncode
  except BaseException as error:
   record_error(shutdown,'read_leader_status',error)
  shutdown['resolved']=group_resolved
  if not group_resolved and reason is None:
   reason='owned process group unresolved'

 try:
  actual_head=subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip()
  dirty=subprocess.check_output(['git','-C',str(repo),'status','--porcelain'],text=True).strip()
  checkout_unchanged=actual_head==candidate and not dirty
  if not checkout_unchanged and reason is None:reason='diagnostic checkout changed unexpectedly'
 except BaseException as error:
  record_error(shutdown,'verify_checkout',error)
  if reason is None:reason='cannot verify unchanged checkout'

 if shutdown['errors']:
  if reason is None:reason='owned process group cleanup had errors'
 try:
  disk_free=shutil.disk_usage(repo).free
 except BaseException as error:
  record_error(shutdown,'read_disk_free',error)
  disk_free=None
 log_validation={'stages_passed':[],'test_summaries':[],'required_tests':{},'actual_desktop_compiled':False,'error_type':None}
 try:
  import re
  log_path=root/'run.log'
  if log_path.stat().st_size>8*1024*1024:raise ValueError('qualification log exceeds 8MiB accepted read bound')
  text=log_path.read_text(errors='replace')
  expected=['desktop','app-clippy','desktop-clippy','native-build','fmt','diff']
  log_validation['stages_passed']=re.findall(r'\[gate:([^:]+):pass\]',text)
  log_validation['test_summaries']=re.findall(r'test result: (?:ok|FAILED)\. [^\n]+',text)
  required=['stale_private_note_copy_stays_out_of_general_knowledge','stale_recovery_from_private_basis_is_refused_in_general_scope']
  log_validation['required_tests']={name: bool(re.search(r'test [^\n]*'+re.escape(name)+r' \.\.\. ok',text)) for name in required}
  compiled=lambda package: bool(re.search(r'Compiling '+re.escape(package)+r' v[^\n]+architecture-status-route/brn-rust/',text))
  log_validation['actual_desktop_compiled']=compiled('brn-desktop')
  log_valid=(log_validation['stages_passed']==expected and all(log_validation['required_tests'].values()) and log_validation['actual_desktop_compiled'] and len(log_validation['test_summaries'])>=1 and all('test result: ok.' in x and not x.startswith('test result: ok. 0 passed;') for x in log_validation['test_summaries']))
 except BaseException as error:
  log_validation['error_type']=type(error).__name__
  log_valid=False
 receipt={
  'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'exit_code':code,'stop_reason':reason,'exception_type':exception,
  'elapsed_seconds':time.monotonic()-started,'disk_free_bytes':disk_free,
  'owned_process_group':shutdown,'checkout_unchanged':checkout_unchanged,
  'candidate':candidate,'log_validation':log_validation,
  'qualified':False,
  'remaining_gates_passed':bool(code==0 and reason is None and checkout_unchanged and group_resolved and not shutdown['errors'] and log_valid),
  'scope':'unreached privacy desktop/lint/native-build gates only; overall qualification remains false while original SQLite parallel-init crash is unresolved'}
 try:
  (root/'completion.json').write_text(json.dumps(receipt,indent=2)+'\n')
  receipt_written=True
 except BaseException as error:
  receipt_error=type(error).__name__
  receipt['qualified']=False
  receipt['evidence_persistence_failed']=True
 try:
  print(json.dumps({'receipt':receipt,'receipt_written':receipt_written,'receipt_error':receipt_error}),flush=True)
 except BaseException:
  pass

raise SystemExit(code if code is not None and checkout_unchanged and group_resolved and reason is None and not shutdown['errors'] and receipt_written and log_valid else 1)
