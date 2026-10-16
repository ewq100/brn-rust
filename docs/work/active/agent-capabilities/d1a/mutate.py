#!/usr/bin/env python3
"""One bounded D1A campaign, exclusively in a fixed disposable Git snapshot.

The persisted wall-clock deadline includes setup, listing and baselines. Never
reset it for later rounds. Candidate artifacts/source are never mutated/reused.
"""
import hashlib, io, json, os, pathlib, signal, subprocess, tarfile, time

evidence = pathlib.Path(__file__).resolve().parent
root = evidence.parents[4]
pin = '202bf2d747bca6fab16a9fffdfa55c45d24bd7eb'
ledger_path = evidence/'mutation-ledger.json'
if ledger_path.exists():
    raise SystemExit('Existing campaign: retain its deadline and ledger; no restart/reset.')
started = time.time()
deadline = started + 1800
work = pathlib.Path('/private/tmp/brn-d1a-20261016-target/mutation-202bf2d7')
work.mkdir(exist_ok=False)
source = work/'source'
source.mkdir()
target = work/'artifacts'
tmp = work/'synthetic-tmp'
tmp.mkdir()
ledger = {'pin':pin, 'started_epoch':started, 'deadline_epoch':deadline,
          'maximum_seconds':1800, 'source':str(source), 'target':str(target),
          'method':'--in-place only in disposable archive; genuine git init, no parent Git/remote/credentials copied; one fixed source and target for baseline/mutants; default single mutant worker',
          'phases':[], 'setup':[]}
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true', TMPDIR=str(tmp))

def save():
    ledger['elapsed_seconds'] = time.time()-started
    ledger_path.write_text(json.dumps(ledger,indent=2)+'\n')

def setup(command):
    result = subprocess.run(command,cwd=source,env=env,capture_output=True,text=True,
                            timeout=max(1,deadline-time.time()-20))
    ledger['setup'].append({'command':command,'exit_code':result.returncode,
                            'stdout':result.stdout,'stderr':result.stderr})
    save()
    result.check_returncode()

save()
archive = subprocess.check_output(['git','archive',pin],cwd=root)
ledger['archive_sha256'] = hashlib.sha256(archive).hexdigest()
tarfile.open(fileobj=io.BytesIO(archive)).extractall(source,filter='data')
setup(['git','init','--quiet'])
setup(['git','add','.'])
setup(['git','-c','user.name=BRN D1A synthetic fixture','-c','user.email=fixture@example.invalid',
       'commit','--quiet','-m','Synthetic immutable D1A source '+pin])
setup(['cargo','mutants','--version'])
paths = subprocess.check_output(['git','ls-tree','-r','--name-only',pin,'--','crates','agent','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=root,text=True).splitlines()
hashes = {p:hashlib.sha256((source/p).read_bytes()).hexdigest() for p in paths}
for p in paths:
    expected = subprocess.check_output(['git','show',pin+':'+p],cwd=root)
    assert hashlib.sha256(expected).hexdigest() == hashes[p], p
ledger['source_hashes'] = hashes
diff = work/'guards.diff'
diff.write_bytes(subprocess.check_output(['git','diff','b617d784',pin,'--',
    'crates/brn-threads-core/src/projects.rs','crates/brn-ai/src/threads_context_budget.rs',
    'crates/brn-ai/src/threads_search.rs','crates/brn-ai/src/error.rs'],cwd=root))
ledger['diff_sha256'] = hashlib.sha256(diff.read_bytes()).hexdigest()
common = ['cargo','mutants','--in-place','--in-diff',str(diff),'--timeout','15',
          '--build-timeout','120','--cargo-arg=--locked','--cargo-arg=--offline',
          '--colors','never','--no-times']
phases = [
    ('core-fences',250,['-p','brn-threads-core','--file','crates/brn-threads-core/src/projects.rs',
        '--test-package','brn-threads-core','--cargo-arg=--test=message_search_scope']),
    ('ai-budget-scope',1450,['-p','brn-ai','--file','crates/brn-ai/src/threads_context_budget.rs',
        '--file','crates/brn-ai/src/threads_search.rs','--file','crates/brn-ai/src/error.rs',
        '--re','Budget::accepts|BudgetHook|read_message|is_context_limit',
        '--test-package','brn-ai','--features','scripted-runs','--cargo-arg=--lib']),
]
save()
try:
    for name, allowance, args in phases:
        if time.time() >= deadline-20:
            ledger.setdefault('unstarted_phases',[]).append(name)
            continue
        command = common+args+['--output',str(work/name)]
        phase = {'name':name,'command':command,'started_elapsed':time.time()-started}
        ledger['phases'].append(phase)
        listing = subprocess.run(command+['--list','--json'],cwd=source,env=env,
            capture_output=True,text=True,timeout=max(1,deadline-time.time()-20))
        (evidence/(name+'-selected.json')).write_text(listing.stdout)
        (evidence/(name+'-selected.stderr')).write_text(listing.stderr)
        phase['listing_exit'] = listing.returncode
        if listing.returncode:
            phase['not_executed'] = 'selection failed'
            save()
            continue
        phase['selected'] = len(json.loads(listing.stdout))
        save()
        log = evidence/(name+'-mutants.log')
        with log.open('w') as output:
            child = subprocess.Popen(command,cwd=source,env=env,stdout=output,
                                     stderr=subprocess.STDOUT,start_new_session=True)
            phase['pid'] = child.pid
            save()
            try:
                phase['exit_code'] = child.wait(timeout=max(1,min(allowance,deadline-time.time()-20)))
                phase['interrupted'] = False
            except subprocess.TimeoutExpired:
                phase['interrupted'] = True
                os.killpg(child.pid,signal.SIGINT)
                try:
                    phase['exit_code'] = child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid,signal.SIGKILL)
                    phase['exit_code'] = child.wait()
        phase['ended_elapsed'] = time.time()-started
        phase['log_sha256'] = hashlib.sha256(log.read_bytes()).hexdigest()
        outcomes = list((work/name).rglob('outcomes.json'))
        if outcomes:
            data = outcomes[0].read_bytes()
            (evidence/(name+'-outcomes.json')).write_bytes(data)
            phase['outcomes_sha256'] = hashlib.sha256(data).hexdigest()
        phase['changed_source_files'] = [p for p,h in hashes.items()
            if hashlib.sha256((source/p).read_bytes()).hexdigest() != h]
        save()
        print(json.dumps(phase),flush=True)
        if phase['changed_source_files']:
            ledger['stopped_reason'] = 'disposable source not restored; no following phase'
            break
finally:
    ledger['finished_epoch'] = time.time()
    save()
    print(json.dumps({'elapsed_seconds':ledger['elapsed_seconds'],'cap':1800}),flush=True)
