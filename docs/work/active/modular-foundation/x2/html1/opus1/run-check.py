import pathlib,subprocess,sys,json,time,hashlib,datetime,os
R=pathlib.Path('/private/tmp/brn-x2-html-r1');root=pathlib.Path('/Users/evokessler/.t3/worktrees/brn-rust/t3-ad771af6')
label=sys.argv[1];args=sys.argv[2:];assert not (R/'logs'/f'{label}.receipt.json').exists()
files=['Cargo.lock','crates/brn-intake/Cargo.toml']+[str(p.relative_to(root)) for p in sorted((root/'crates/brn-intake/src').rglob('*.rs'))]
def hashes():return {f:hashlib.sha256((root/f).read_bytes()).hexdigest() for f in files}
env=dict(os.environ,CARGO_TARGET_DIR=str(R/'target'),CARGO_NET_OFFLINE='true',TMPDIR=str(R/'tmp'),BRN_INTAKE_HELPER=str(R/'target/debug/brn-intake-helper'))
record={'label':label,'args':args,'cwd':str(root),'env':{k:env[k] for k in ['CARGO_TARGET_DIR','TMPDIR','CARGO_NET_OFFLINE','BRN_INTAKE_HELPER']},'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_sha256':hashes()}
(R/'logs'/f'{label}.start.json').write_text(json.dumps(record,indent=2)+'\n');start=time.monotonic()
with (R/'logs'/f'{label}.log').open('wb') as log:
 p=subprocess.Popen(args,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT);record['pid']=p.pid;record['exit']=p.wait()
record['seconds']=time.monotonic()-start;record['source_unchanged']=record['source_sha256']==hashes();record['log_sha256']=hashlib.sha256((R/'logs'/f'{label}.log').read_bytes()).hexdigest()
(R/'logs'/f'{label}.receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:record[k] for k in ['label','exit','seconds','source_unchanged']}));print((R/'logs'/f'{label}.log').read_text()[-6500:]);sys.exit(record['exit'])
