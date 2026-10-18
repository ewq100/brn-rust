#!/usr/bin/env python3
"""Five fixed source mutations; the original remaining D1A budget only."""
import pathlib,json,time,subprocess,os,signal,hashlib,difflib,re,tempfile
E=pathlib.Path(__file__).resolve().parent
B=json.loads((E/'mutation-final-budget.json').read_text())
START=B['segment_started_epoch']; D=B['deadline_epoch']; PIN=B['pin']
S=pathlib.Path('/private/tmp/brn-d1a-20261016-target/mutation-202bf2d7/source')
T=S.parent/'artifacts'; LP=E/'mutation-final-ledger.json'
L=dict(B,source=str(S),target=str(T),records=[],mutants=[],stop_requested=False)
active=None

def save():
    L['segment_elapsed_seconds']=time.time()-START
    L['total_original_charged_seconds']=B['prior_total_charged_seconds']+L['segment_elapsed_seconds']
    L['remaining_original_seconds']=1800-L['total_original_charged_seconds']
    LP.write_text(json.dumps(L,indent=2)+'\n')
def stop(sig,frame):
    L['stop_requested']=True
    if active is not None:
        try: os.killpg(active.pid,signal.SIGINT)
        except ProcessLookupError: pass
    save()
signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true')
def run(label,cmd,limit=90):
    global active
    if L['stop_requested'] or time.time()>D-30:return {'label':label,'status':'Unexecuted'}
    log=E/('final-'+label+'.log');r={'label':label,'command':cmd,'started_elapsed':time.time()-START,'source':str(S),'target':str(T)}
    with log.open('w') as out:
        active=subprocess.Popen(cmd,cwd=S,env=env,stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
        r['pid']=active.pid;L['active']=r;save()
        try:
            r['exit_code']=active.wait(timeout=max(1,min(limit,D-time.time()-30)))
            r['status']='Success' if r['exit_code']==0 else 'Failure'
        except subprocess.TimeoutExpired:
            r['status']='Timeout';os.killpg(active.pid,signal.SIGINT)
            try:r['exit_code']=active.wait(timeout=5)
            except subprocess.TimeoutExpired:os.killpg(active.pid,signal.SIGKILL);r['exit_code']=active.wait()
        active=None
    r.update(ended_elapsed=time.time()-START,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
    L.pop('active',None);L['records'].append(r);save();return r

def artifact():
    f=T/'debug/deps/brn_ai-28a197b27edef53b'
    return [{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [f,pathlib.Path(str(f)+'.d')] if p.exists()]

# Same fixed copied root/target as prior campaign, never the candidate target.
# Retain genuine isolated .git, no remotes/history/credentials copied.
try:
    archive=subprocess.check_output(['git','archive',PIN],cwd=E.parents[4]);L['archive_sha256']=hashlib.sha256(archive).hexdigest()
    subprocess.run(['tar','-x','-C',str(S)],input=archive,check=True)
    paths=subprocess.check_output(['git','ls-tree','-r','--name-only',PIN,'--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml','agent'],cwd=E.parents[4],text=True).splitlines()
    L['source_sha256']={n:hashlib.sha256((S/n).read_bytes()).hexdigest() for n in paths}
    # Verify every archived material blob against the pinned Git object.
    for n in paths:
        expected=subprocess.check_output(['git','show',PIN+':'+n],cwd=E.parents[4])
        assert hashlib.sha256(expected).hexdigest()==L['source_sha256'][n],n
    tmp=tempfile.mkdtemp(prefix='brn-d1a-final-',dir='/private/tmp');env['TMPDIR']=tmp;L['tmpdir']=tmp
    save();print('setup verified',len(paths),'pinned material files',flush=True)
    assert run('git-add',['git','add','.'],20)['status']=='Success'
    assert run('git-commit',['git','-c','user.name=BRN synthetic mutation fixture','-c','user.email=fixture@example.invalid','commit','--quiet','-m','Synthetic final D1A '+PIN],20)['status']=='Success'
    base=['cargo','test','--locked','--offline','-p','brn-ai','--features','scripted-runs','--lib']
    baseline=run('baseline',base,120)
    assert baseline['status']=='Success','baseline must pass'
    assert '393 passed; 0 failed; 1 ignored' in (E/'final-baseline.log').read_text()
    L['baseline_artifacts']=artifact();save();print('baseline PASS393/0/1',flush=True)
    specs=[
        ('post-apply-full-output','crates/brn-ai/src/threads.rs','fn applied_output(', 'if context_budget::result_fits(&full) {','if true {'),
        ('ack-refresh-omitted-count','crates/brn-ai/src/threads.rs','fn change_acknowledgement(', '"records_omitted":records.len()-refresh_count','"records_omitted":records.len()'),
        ('source-offset','crates/brn-ai/src/threads_sources.rs','pub(super) fn source_metadata(', '&text[start..], |prefix| {','&text, |prefix| {'),
        ('section-end','crates/brn-ai/src/threads.rs','fn read_note(', 'let limit = note.section_end.unwrap_or(note.total_bytes);','let limit = note.total_bytes;'),
        ('utf8-plus-one-recheck','crates/brn-ai/src/threads_context_budget.rs','pub(super) fn fit_prefix(', 'boundaries.len() - 1','boundaries.len() + 1'),
    ]
    for label,name,anchor,old,new in specs:
        m={'id':'final:'+label,'source_pin':PIN,'file':name,'outcome':'NoVerdict'};L['mutants'].append(m)
        if time.time()>D-55 or L['stop_requested']:m['reason']='original remainder reserved for safe cleanup';save();continue
        path=S/name;original=path.read_bytes();text=original.decode();start=text.index(anchor);at=text.index(old,start);modified=text[:at]+text[at:].replace(old,new,1)
        diff=''.join(difflib.unified_diff(text.splitlines(True),modified.splitlines(True),fromfile=name,tofile=label));(E/('final-'+label+'.diff')).write_text(diff);m['diff_sha256']=hashlib.sha256(diff.encode()).hexdigest();m['mutated_source_sha256']=hashlib.sha256(modified.encode()).hexdigest()
        try:
            path.write_text(modified);save();print('mutant',label,flush=True)
            build=run(label+'-build',base+['--no-run'],75);m['build']=build
            if build['status']!='Success':m['outcome']='Unviable' if build['status']=='Failure' else build['status'];continue
            m['artifacts']=artifact();test=run(label+'-test',base,30);m['test']=test
            raw=(E/('final-'+label+'-test.log')).read_text() if test.get('log_sha256') else ''
            m['failing_tests']=[line for line in raw.splitlines() if line.startswith('test ') and line.endswith(' FAILED')]
            m['outcome']='Caught' if test['status']=='Failure' and m['failing_tests'] else ('Missed' if test['status']=='Success' else ('NoVerdict' if test['status']=='Unexecuted' else test['status']))
        finally:
            path.write_bytes(original);m['source_restored']=hashlib.sha256(path.read_bytes()).hexdigest()==L['source_sha256'][name];save();print(label,m['outcome'],flush=True)
finally:
    L['changed_source_files']=[n for n,h in L.get('source_sha256',{}).items() if hashlib.sha256((S/n).read_bytes()).hexdigest()!=h]
    L['finished_epoch']=time.time();save();print(json.dumps({'counts':{x:sum(m['outcome']==x for m in L['mutants']) for x in set(m['outcome'] for m in L['mutants'])},'charged':L['total_original_charged_seconds'],'remaining':L['remaining_original_seconds'],'changed':L['changed_source_files']}),flush=True)
