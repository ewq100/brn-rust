#!/usr/bin/env python3
"""Serial, pinned, offline D1A receipts; synthetic data, no live providers."""
import hashlib, json, os, pathlib, subprocess, sys, time

folder = pathlib.Path(__file__).resolve().parent
root = folder.parents[4]
name, *command = sys.argv[1:]
if not command:
    raise SystemExit('run.py NAME COMMAND...')
cwd = pathlib.Path(os.environ.get('D1A_SOURCE', root))
tmp = os.environ.get('D1A_TMPDIR') or pathlib.Path('/private/tmp/brn-d1a-tmpdir-path').read_text().strip()
receipt = {
    'command': command, 'cwd': str(cwd),
    'accepted_base': 'b617d7842c1e668292b4f23c7b321ec195a76a1d',
    'head': subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),
    'diff_sha256': hashlib.sha256(subprocess.check_output(['git','diff','HEAD'],cwd=root)).hexdigest(),
    'owned_sources': {str(p.relative_to(cwd)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted([*(cwd/'crates/brn-ai/src').glob('*.rs'), cwd/'crates/brn-threads-core/src/projects.rs', cwd/'crates/brn-threads-core/tests/message_search_scope.rs'])},
    'target': os.environ.get('D1A_TARGET','/private/tmp/brn-d1a-20261016-target'), 'tmpdir': tmp,
    'source_git_head': subprocess.check_output(['git','rev-parse','HEAD'],cwd=cwd,text=True).strip(),
    'source_diff_sha256': hashlib.sha256(subprocess.check_output(['git','diff','HEAD'],cwd=cwd)).hexdigest(),
    'request_inputs': {str(p.relative_to(cwd)): hashlib.sha256(p.read_bytes()).hexdigest() for p in [cwd/'Cargo.lock',cwd/'rust-toolchain.toml',cwd/'crates/brn-ai/Cargo.toml',cwd/'crates/brn-ai/src/threads_model_limits.json']},
    'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
}
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=receipt['target'], CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true', TMPDIR=tmp)
start = time.monotonic()
with (folder/(name+'.log')).open('w') as log:
    result = subprocess.run(command,cwd=cwd,env=env,stdout=log,stderr=subprocess.STDOUT)
receipt.update(exit_code=result.returncode, elapsed_seconds=time.monotonic()-start, log_sha256=hashlib.sha256((folder/(name+'.log')).read_bytes()).hexdigest())
(folder/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({k:v for k,v in receipt.items() if k != 'owned_sources'},indent=2))
raise SystemExit(result.returncode)
