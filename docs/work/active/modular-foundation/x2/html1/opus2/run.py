from pathlib import Path
import subprocess,sys,os,json,hashlib,time,datetime,shutil
R=Path('/private/tmp/brn-x2-html-r2');side,label,*args=sys.argv[1:];assert side in ['before','candidate'];root=R/'before-src' if side=='before' else Path('/Users/evokessler/.t3/worktrees/brn-rust/t3-ad771af6');target=R/'target-before' if side=='before' else Path('/private/tmp/brn-x2-html-r1/target');tmp=R/('tmp-'+side);log=R/'logs'/(label+'.log');receipt=R/'logs'/(label+'.json');assert not log.exists() and not receipt.exists()
sha=lambda b:hashlib.sha256(b).hexdigest()
files=['Cargo.lock','Cargo.toml','rust-toolchain.toml','crates/brn-intake/Cargo.toml']+[str(p.relative_to(root)) for p in sorted((root/'crates/brn-intake').rglob('*.rs'))]
def hashes():return {f:sha((root/f).read_bytes()) for f in files}
env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_NET_OFFLINE='true',TMPDIR=str(tmp));record={'label':label,'side':side,'args':args,'cwd':str(root),'target':str(target),'tmpdir':str(tmp),'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_sha256':hashes()};(R/'logs'/(label+'.start.json')).write_text(json.dumps(record,indent=2)+'\n');started=time.monotonic()
with log.open('wb') as out:
 p=subprocess.Popen(args,cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT);record['pid']=p.pid;record['exit']=p.wait()
record.update(seconds=time.monotonic()-started,source_unchanged=record['source_sha256']==hashes(),raw_log_sha256=sha(log.read_bytes()));receipt.write_text(json.dumps(record,indent=2)+'\n');assert record['source_unchanged'];print(json.dumps({k:record[k] for k in ['label','exit','seconds','source_unchanged']}));print(log.read_text()[-5000:]);sys.exit(record['exit'])
