#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-writing-setup-serial-diagnostic-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
python3 - <<'CHECK'
import subprocess,pathlib,hashlib,shutil
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
p=pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target/debug/deps/writing_setup-0a70c929436e2f9d')
assert hashlib.sha256(p.read_bytes()).hexdigest()=='cd20fe93330b7b00eb3e9a226421f451e46620c53a308c4106eecafb7e7e2ca4'
assert '79F53E61-A47D-382F-B602-4D3046124A83' in subprocess.check_output(['/usr/bin/dwarfdump','--uuid',str(p)],text=True)
assert shutil.disk_usage('.').free>3*1024**3
CHECK
/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target/debug/deps/writing_setup-0a70c929436e2f9d --test-threads=1
