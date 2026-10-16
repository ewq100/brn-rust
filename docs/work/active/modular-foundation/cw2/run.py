#!/usr/bin/env python3
"""Serial offline CW2 command receipt; no credentials or provider payloads."""
import hashlib, json, os, pathlib, subprocess, sys, time
root = pathlib.Path(__file__).resolve().parents[5]
name, *command = sys.argv[1:]
if not command:
    raise SystemExit('run.py NAME COMMAND...')
folder = pathlib.Path(__file__).resolve().parent
receipt = {'command': command, 'cwd': str(root), 'head': subprocess.check_output(['git','rev-parse','HEAD'], cwd=root, text=True).strip(), 'crates_tree': subprocess.check_output(['git','rev-parse','HEAD:crates'], cwd=root,text=True).strip(), 'diff_sha256': hashlib.sha256(subprocess.check_output(['git','diff','HEAD','--','crates'],cwd=root)).hexdigest(), 'source_files': {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for base in ['crates/brn-threads-core','crates/brn-threads-app','crates/brn-ai','crates/brn','crates/brn-desktop'] for p in sorted((root/base).rglob('*.rs'))}, 'target': '/private/tmp/brn-cw2-20261016-target', 'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=receipt['target'], CARGO_INCREMENTAL='0')
start = time.monotonic()
with (folder/(name+'.log')).open('w') as log:
    result = subprocess.run(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
receipt.update(exit_code=result.returncode,elapsed_seconds=time.monotonic()-start,log_sha256=hashlib.sha256((folder/(name+'.log')).read_bytes()).hexdigest())
(folder/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({k:v for k,v in receipt.items() if k != 'source_files'},indent=2))
raise SystemExit(result.returncode)
