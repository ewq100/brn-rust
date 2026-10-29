"""One offline Mac setup attempt under the same budget later used by launch.py."""
import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from deadline_guard import Guard

HERE = Path(__file__).resolve().parent
SOURCE = '12251dfc78bc6ff9aa0eaf6809448886b0aa794c'
JOB = 'brn-terra-sol-pilot-002'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def create(path, value):
    with open(path, 'x', encoding='utf-8') as f:
        os.fchmod(f.fileno(),0o600)
        f.write(json.dumps(value,indent=2)+'\n'); f.flush(); os.fsync(f.fileno())
    fd=os.open(path.parent,os.O_RDONLY|getattr(os,'O_DIRECTORY',0))
    try: os.fsync(fd)
    finally: os.close(fd)

def main():
    p=argparse.ArgumentParser()
    for name in ('repository','source-dir','claim-directory','cargo-bin','auth-file','output'):
        p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--manifest-sha256',required=True)
    p.add_argument('--authorization-reference',required=True)
    a=p.parse_args()
    if sys.platform != 'darwin': p.error('Mac setup only')
    if sha(HERE/'package-manifest.json') != a.manifest_sha256: p.error('Wrong package')
    for name,h in json.loads((HERE/'package-manifest.json').read_text())['files'].items():
        if Path(name).is_absolute() or '..' in Path(name).parts or sha(HERE/name)!=h:
            p.error('Package integrity failure')
    for name in ('repository','source_dir','claim_directory','cargo_bin','auth_file','output'):
        if not getattr(a,name).is_absolute(): p.error('Absolute host-local bindings required')
    if not a.claim_directory.is_dir() or a.claim_directory.is_symlink(): p.error('Existing durable ledger required')
    if a.source_dir.exists() or a.output.exists(): p.error('Fresh source/output required')
    start=datetime.now(timezone.utc)
    setup={'job':JOB,'runner':'brn-mac-relay-01','source_commit':SOURCE,
           'started_at':start.isoformat(),'deadline_at':(start+timedelta(seconds=3600)).isoformat(),
           'plan_sha256':sha(HERE/'pilot-plan.json'),'package_manifest_sha256':a.manifest_sha256,
           'authorization_reference':a.authorization_reference}
    receipt=a.claim_directory/(JOB+'.setup.json')
    create(receipt,setup)
    g=Guard(a.claim_directory/(JOB+'.setup-supervisor.jsonl'),
            3600-(datetime.now(timezone.utc)-start).total_seconds(),setup)
    g.install(); status=1
    try:
        if datetime.now(timezone.utc) >= datetime(2026,10,30,5,tzinfo=timezone.utc):
            raise ValueError('Release start expiry reached')
        for name in ('rustup','rustc','cargo'):
            if not os.access(a.cargo_bin/name,os.X_OK): raise ValueError('Installed tool missing')
        env=dict(os.environ)
        for key in list(env):
            if key.startswith('BRN_'): del env[key]
        env.update(PATH=str(a.cargo_bin)+os.pathsep+env.get('PATH',''),RUSTUP_TOOLCHAIN='1.98.1',
                   CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0')
        def run(cmd,**kw):
            return subprocess.run([str(x) for x in cmd],env=env,check=True,**kw)
        for name in ('rustc','cargo'):
            found=run([a.cargo_bin/'rustup','which','--toolchain','1.98.1',name],capture_output=True,text=True).stdout.strip()
            if not Path(found).is_absolute() or not os.access(found,os.X_OK): raise ValueError('Pinned executable unavailable')
            version=run([found,'--version'],capture_output=True,text=True).stdout
            if not version.startswith(name+' 1.98.1 ('): raise ValueError('Wrong installed toolchain')
            proxy=run([a.cargo_bin/name,'+1.98.1','--version'],capture_output=True,text=True).stdout
            if proxy!=version: raise ValueError('Installed proxy mismatch')
        run(['git','-C',a.repository,'worktree','add','--detach',a.source_dir,SOURCE])
        if run(['git','-C',a.source_dir,'rev-parse','HEAD'],capture_output=True,text=True).stdout.strip()!=SOURCE:
            raise ValueError('Wrong source')
        # All build products stay in this new worktree's ignored target directory.
        target=a.source_dir/'target'; target.mkdir(exist_ok=True)
        env['CARGO_TARGET_DIR']=str(target)
        helpers=target/'pilot-document-helpers'
        run(['bash','scripts/build-document-helpers.sh','--offline','--jobs','1','--target-root',helpers],cwd=a.source_dir)
        run([a.cargo_bin/'cargo','+1.98.1','build','--locked','--offline','-p','brn'],cwd=a.source_dir)
        run([a.cargo_bin/'cargo','+1.98.1','build','--locked','--offline','-p','brn-threads-app','--example','wiki_eval_seed'],cwd=a.source_dir)
        import shutil
        for name in ('brn-xberg-helper','brn-xberg-helper-brn'):
            shutil.copy2(helpers/'artifacts'/name,target/'debug'/name)
        run([sys.executable,'-B',HERE/'test_deadline.py','-v'])
        bindings={'BRN_REPO':str(a.source_dir),'BRN_BINARY':str(target/'debug/brn'),
                  'BRN_SEEDER':str(target/'debug/examples/wiki_eval_seed'),
                  'BRN_AUTH_FILE':str(a.auth_file),'BRN_FRESH_OUTPUT':str(a.output)}
        binary_hashes={k:sha(bindings[k]) for k in ('BRN_BINARY','BRN_SEEDER')}
        binary_hashes.update({name:sha(target/'debug'/name) for name in ('brn-xberg-helper','brn-xberg-helper-brn')})
        create(a.claim_directory/(JOB+'.bindings.json'),bindings)
        create(a.claim_directory/(JOB+'.readiness.json'),{'offline_build_and_supervisor_passed':True,
            'binary_sha256':binary_hashes,'setup_receipt_sha256':sha(receipt),'setup':setup,'model_calls':0})
        status=0
    except Exception as error:
        g.record('blocked',error_class=type(error).__name__)
    finally:
        status=g.finish(status)
    return status

if __name__=='__main__': sys.exit(main())
