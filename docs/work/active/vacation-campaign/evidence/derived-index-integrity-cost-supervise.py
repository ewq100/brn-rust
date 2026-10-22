import datetime,hashlib,json,os,pathlib,shutil,signal,subprocess,time

root=pathlib.Path('/tmp/brn-derived-index-integrity-qualification')
repo=pathlib.Path('/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust')
source=repo/'crates/brn-retrieval/src/search_index.rs'
original=root/'original-source.rs'
expected='61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9'
original_hash='94efb75c085c933130196aa2681aa47624674f6a2a3074522c1280992044cc70'
assert hashlib.sha256(original.read_bytes()).hexdigest()==original_hash


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
restored=False;group_resolved=True;receipt_written=False;receipt_error=None
shutdown={'signals':[],'errors':[],'group_probes':0,'last_group_status':'not_started',
          'term_group_absent':None,'kill_group_absent':None,'leader_reaped':False,
          'resolved':True}
source_restore_status='not_attempted'
source_hash_after=None
try:
 with (root/'cost-run.log').open('wb') as log:
  p=subprocess.Popen(['bash',str(root/'cost-run.sh')],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-started>1800:reason='30-minute experiment deadline'
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
   reason='owned process group unresolved; source restoration skipped'

 try:
  current=hashlib.sha256(source.read_bytes()).hexdigest()
  source_hash_after=current
  if not group_resolved:
   source_restore_status='skipped_owned_group_unresolved'
  elif current==expected:
   source_restore_status='restore_attempted'
   try:
    source.write_bytes(original.read_bytes())
   except BaseException as error:
    record_error(shutdown,'restore_source',error)
    source_restore_status='restore_error'
    if reason is None:reason='hash-checked source restoration failed'
   try:
    source_hash_after=hashlib.sha256(source.read_bytes()).hexdigest()
    restored=source_hash_after==original_hash
    if restored:source_restore_status='restored_exactly'
    elif reason is None:reason='source restoration hash mismatch; preserve current bytes'
   except BaseException as error:
    record_error(shutdown,'verify_restored_source',error)
    source_restore_status='verification_error'
    restored=False
    if reason is None:reason='source restoration could not be verified'
  elif current==original_hash:
   restored=True
   source_restore_status='already_original_exactly'
  else:
   source_restore_status='preserved_unexpected_hash'
   if reason is None:reason='source changed unexpectedly; preserve without overwrite'
 except BaseException as error:
  record_error(shutdown,'inspect_source_for_restoration',error)
  source_restore_status='inspection_error'
  restored=False
  if reason is None:reason='source hash inspection failed'

 if shutdown['errors']:
  if reason is None:reason='owned process group cleanup had errors'
 try:
  disk_free=shutil.disk_usage(repo).free
 except BaseException as error:
  record_error(shutdown,'read_disk_free',error)
  disk_free=None
 receipt={
  'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'exit_code':code,'stop_reason':reason,'exception_type':exception,
  'elapsed_seconds':time.monotonic()-started,'disk_free_bytes':disk_free,
  'owned_process_group':shutdown,'source_restored_exactly':restored,
  'source_restore_status':source_restore_status,'source_hash_after':source_hash_after,
  'original_source_sha256':original_hash,'temporary_source_sha256':expected,
  'qualified':bool(code==0 and reason is None and restored and group_resolved and not shutdown['errors']),
  'scope':'cost-only healthy-index scan, no recovery candidate or production adoption'}
 try:
  (root/'cost-completion.json').write_text(json.dumps(receipt,indent=2)+'\n')
  receipt_written=True
 except BaseException as error:
  receipt_error=type(error).__name__
  receipt['qualified']=False
  receipt['evidence_persistence_failed']=True
 try:
  print(json.dumps({'receipt':receipt,'receipt_written':receipt_written,'receipt_error':receipt_error}),flush=True)
 except BaseException:
  pass

raise SystemExit(code if code is not None and restored and group_resolved and reason is None and not shutdown['errors'] and receipt_written else 1)
