#!/usr/bin/env python3
"""Read-only scope/source audit; verify patch against the pinned crate archive."""
import datetime
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import tarfile
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BASE = '7e9d0376d677ed2a46bdbcd8933f55f9e1746a02'
Z1 = 'b9bd1a14862fe2efe246790e887797c949f13c50'
def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
changes = git('diff', '--name-only', BASE).splitlines()
untracked = git('ls-files', '--others', '--exclude-standard').splitlines()
assert all(p.startswith('experiments/passage-comment-geometry/') for p in changes + untracked)
assert git('rev-parse', 'work/z1-zorite') == Z1
assert not git('ls-files', 'experiments/passage-comment-geometry/.vendor')
original_worktree = ROOT.parent / 'z1-zorite'
assert not subprocess.check_output(['git', '-C', str(original_worktree), 'status', '--porcelain'], text=True).strip()
archive = next((Path.home() / '.cargo/registry/cache').glob('*/gpui-base-0.7.3.crate'))
assert hashlib.sha256(archive.read_bytes()).hexdigest() == '2e9955b6f4b048fd50ca15fc8e225d80bb00ba9ebc8fecdddde5ad26dbef9270'
expected = json.loads((HERE / 'evidence/review-toolkit-sources.json').read_text())
patch = HERE / 'patches/gpui-base-0.7.3-range-start.patch'
with tempfile.TemporaryDirectory(prefix='patch-check-', dir=HERE / '.vendor') as directory:
    directory = Path(directory)
    with tarfile.open(archive) as bundle:
        for path in expected:
            dest = directory / path
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(bundle.extractfile('gpui-base-0.7.3/' + path).read())
    subprocess.run(['patch', '--batch', '-p1', '-i', str(patch)], cwd=directory, check=True, capture_output=True)
    for path, checksum in expected.items():
        assert hashlib.sha256((directory / path).read_bytes()).hexdigest() == checksum, path
        assert hashlib.sha256((HERE / '.vendor/gpui-base' / path).read_bytes()).hexdigest() == checksum, path
relevant = ['AGENTS.md', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
            'crates/brn-desktop', 'crates/brn-threads-core', 'crates/brn-threads-app']
current = git('rev-parse', 'origin/main')
identical = {path: git('rev-parse', f'{BASE}:{path}') == git('rev-parse', f'{current}:{path}') for path in relevant}
raw = plistlib.loads(subprocess.check_output(['/usr/sbin/ioreg', '-a', '-n', 'Root', '-d', '1']))
roots = raw if isinstance(raw, list) else [raw]
flags = []
for root in roots:
    for session in root.get('IOConsoleUsers', []):
        flags.append({key: session[key] for key in ['CGSSessionScreenIsLocked'] if key in session})
print(json.dumps({
    'checked_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'head_at_check': git('rev-parse', 'HEAD'),
    'branch': git('branch', '--show-current'),
    'baseline': BASE, 'freshly_fetched_origin_main': current,
    'production_and_guidance_objects_match_fetched_main': identical,
    'original_z1_unchanged_and_clean': Z1,
    'all_changed_and_untracked_paths_in_experiment': True,
    'tracked_vendor_files': [], 'production_manifest_or_lock_changes': [],
    'toolkit_patch_sha256': hashlib.sha256(patch.read_bytes()).hexdigest(),
    'applied_patch_reproduces_tested_sources': expected,
    'macos': subprocess.check_output(['/usr/bin/sw_vers', '-productVersion'], text=True).strip(),
    'console_session_flags': flags,
    'c_owner_response': 'no c yet',
    'c_confirmed_integration_ref': None, 'approved_production_toolkit_source': None,
    'production_integration_commit': None,
}, indent=2))
