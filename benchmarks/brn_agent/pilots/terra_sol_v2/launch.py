"""Owner-released launch only. No implicit release, account selection, or retry."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
from datetime import datetime, timezone, timedelta
from deadline_guard import Guard

HERE = Path(__file__).resolve().parent

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

class PilotStop(BaseException):
    pass


def check_completed_attempts(root):
    for path in Path(root).glob('runs/*/*/result.json'):
        raw = json.loads(path.read_text())
        if raw.get('status') == 'running':
            continue
        if raw.get('status') != 'complete' or raw.get('safety'):
            raise PilotStop('Retained failure/safety stop')
        for step in raw.get('steps', []):
            if step.get('kind') not in ('ask', 'intake'):
                continue
            if str(step.get('terminal', '')).startswith('Failed:'):
                raise PilotStop('Retained model/runtime failure')
            trace = step.get('trace') or {}
            requests = trace.get('wire_requests', [])
            if not requests or trace.get('wire_requests_dropped', 0):
                raise PilotStop('Runtime identity unverified')
            if any(r.get('model') != raw['model'] or r.get('effort') != 'medium' for r in requests):
                raise PilotStop('Runtime identity mismatch')
            if any(m is not None and m != raw['model'] for m in trace.get('returned_models', [])):
                raise PilotStop('Returned model mismatch')


def remaining_budget(setup, release, now):
    started = datetime.fromisoformat(setup['started_at'])
    deadline = datetime.fromisoformat(setup['deadline_at'])
    if started.tzinfo is None or deadline.tzinfo is None or started > now:
        raise ValueError('Invalid setup clock')
    if deadline != started + timedelta(seconds=3600):
        raise ValueError('Setup budget must be exactly one hour')
    if setup.get('job') != 'brn-terra-sol-pilot-001' or setup.get('runner') != 'brn-mac-relay-01':
        raise ValueError('Wrong setup job')
    if setup.get('plan_sha256') != release['plan_sha256']:
        raise ValueError('Wrong setup plan')
    if setup.get('package_manifest_sha256') != release['package_manifest_sha256']:
        raise ValueError('Wrong setup package')
    if setup.get('source_commit') != '12251dfc78bc6ff9aa0eaf6809448886b0aa794c':
        raise ValueError('Wrong setup source')
    remaining = (deadline-now).total_seconds()
    if remaining < 60 or remaining > 3600:
        raise ValueError('Insufficient remaining setup budget; do not dispatch')
    return remaining


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--release', type=Path, required=True)
    parser.add_argument('--bindings', type=Path, required=True)
    parser.add_argument('--setup-receipt', type=Path, required=True)
    parser.add_argument('--capability-receipt', type=Path, required=True)
    args = parser.parse_args()
    plan = json.loads((HERE/'pilot-plan.json').read_text())
    release = json.loads(args.release.read_text())
    bindings = json.loads(args.bindings.read_text())
    expected = digest(HERE/'pilot-plan.json')
    manifest = json.loads((HERE/'package-manifest.json').read_text())
    if release.get('package_manifest_sha256') != digest(HERE/'package-manifest.json'):
        parser.error('Package fingerprint does not match release')
    for name, sha in manifest['files'].items():
        if Path(name).is_absolute() or '..' in Path(name).parts or digest(HERE/name) != sha:
            parser.error('Package file identity mismatch')
    if release.get('bindings_sha256') != digest(args.bindings):
        parser.error('Bindings differ from owner-released host paths')
    if release.get('capability_receipt_sha256') != digest(args.capability_receipt):
        parser.error('Capability receipt identity mismatch')
    capability = json.loads(args.capability_receipt.read_text())
    if capability.get('runner') != 'brn-mac-relay-01' or capability.get('auth_route') != 'chatgpt_subscription':
        parser.error('Wrong capability runner or route')
    for model in ('gpt-6.1-terra','gpt-6.1-sol'):
        if 'medium' not in capability.get('models', {}).get(model, []):
            parser.error('Requested model/effort not positively observed')
    required = {'plan_sha256': expected, 'runner': 'brn-mac-relay-01', 'live_release': True,
                'offline002_terminal_and_processes_stopped': True, 'existing_codex_subscription': True,
                'models_and_medium_verified': True, 'candidate_runs': 16, 'max_model_calls': 192}
    if any(release.get(k) != v for k,v in required.items()):
        parser.error('Release does not match this plan and its admission gates')
    if datetime.now(timezone.utc) >= datetime.fromisoformat(release['start_before']):
        parser.error('Release expired')
    if not release.get('authorization_reference') or not release.get('capability_receipt_sha256'):
        parser.error('Authenticated release and capability receipt references required')
    # Claim precedes git, binary inspection and all setup; existing claim is consumed.
    claim_dir = Path(release['claim_directory'])
    if not claim_dir.is_absolute() or not claim_dir.is_dir() or claim_dir.is_symlink():
        parser.error('Existing runner-owned durable claim directory required')
    if args.setup_receipt.resolve() != (claim_dir/'brn-terra-sol-pilot-001.setup.json').resolve():
        parser.error('Setup must use the fixed durable job receipt')
    if release.get('setup_receipt_sha256') != digest(args.setup_receipt):
        parser.error('Setup receipt fingerprint mismatch')
    setup = json.loads(args.setup_receipt.read_text())
    if release.get('offline_build_and_supervisor_passed') is not True:
        parser.error('Offline build and Mac supervisor acceptance required')
    supervisor = claim_dir/'brn-terra-sol-pilot-001.setup-supervisor.jsonl'
    rows = [json.loads(line) for line in supervisor.read_text().splitlines()]
    terminal = rows[-1] if rows else {}
    if terminal.get('status') != 'completed' or terminal.get('exit_code') != 0 or terminal.get('cleanup_errors'):
        parser.error('Setup supervisor did not complete cleanly')
    seconds = remaining_budget(setup, release, datetime.now(timezone.utc))
    receipt = claim_dir/'brn-terra-sol-pilot-001.launch.jsonl'
    guard = Guard(receipt, seconds, {'job': 'brn-terra-sol-pilot-001',
                                     'plan_sha256': expected, 'release_sha256': digest(args.release)})
    guard.install()
    status = 1
    try:
        for name, value in bindings.items():
            if name not in {'BRN_REPO','BRN_BINARY','BRN_SEEDER','BRN_AUTH_FILE','BRN_FRESH_OUTPUT'}:
                raise ValueError('Unexpected binding')
            if not Path(value).is_absolute():
                raise ValueError('Bindings must be absolute host-local paths')
        if set(bindings) != {'BRN_REPO','BRN_BINARY','BRN_SEEDER','BRN_AUTH_FILE','BRN_FRESH_OUTPUT'}:
            raise ValueError('Missing host-local binding')
        repo = Path(bindings['BRN_REPO'])
        head = subprocess.check_output(['git','rev-parse','HEAD'], cwd=repo, text=True).strip()
        dirty = subprocess.check_output(['git','status','--porcelain'], cwd=repo, text=True)
        if head != plan['source_commit'] or dirty:
            raise ValueError('Source identity mismatch or dirty checkout')
        for name in ('BRN_BINARY','BRN_SEEDER'):
            if not os.access(bindings[name], os.X_OK):
                raise ValueError('Missing built executable; this launcher does not build or install')
        if Path(bindings['BRN_FRESH_OUTPUT']).exists():
            raise ValueError('Output must be fresh; no resume')
        for name, sha in plan['evidence_sha256'].items():
            if digest(HERE/'planning'/name) != sha:
                raise ValueError('Fixture/truth/suite hash mismatch')
        # Receipt hashes executable bytes without reading any credential file.
        for helper_name in ('brn-xberg-helper','brn-xberg-helper-brn'):
            helper = Path(bindings['BRN_BINARY']).with_name(helper_name)
            if not os.access(helper, os.X_OK) or digest(helper) != release.get('binary_sha256', {}).get(helper_name):
                raise ValueError('Missing or mismatched pinned document helper')
        for name in ('BRN_BINARY','BRN_SEEDER'):
            if digest(bindings[name]) != release.get('binary_sha256', {}).get(name):
                raise ValueError('Executable differs from approved release')
        for name in list(os.environ):
            if name.startswith('BRN_'):
                os.environ.pop(name)
        guard.record('admitted', source_commit=head, binary_sha256=digest(bindings['BRN_BINARY']),
                     seeder_sha256=digest(bindings['BRN_SEEDER']))
        guard.before_launch = lambda: check_completed_attempts(bindings['BRN_FRESH_OUTPUT'])
        substitutions = {**bindings, 'PLANNING': str(HERE/'planning')}
        argv = []
        for item in plan['arguments_template'][1:]:
            for key,value in substitutions.items():
                item = item.replace('${'+key+'}',value)
            argv.append(item)
        sys.argv = argv+['--live']
        os.chdir(repo)
        try:
            runpy.run_path(argv[0], run_name='__main__')
            check_completed_attempts(bindings['BRN_FRESH_OUTPUT'])
            status = 0
        except SystemExit as result:
            status = result.code if isinstance(result.code, int) else 1
            check_completed_attempts(bindings['BRN_FRESH_OUTPUT'])
    except (Exception, PilotStop) as error:
        status = 1
        # No exception text: paths and provider payloads remain in private harness evidence.
        guard.record('blocked', error_class=type(error).__name__)
    finally:
        status = guard.finish(status)
    return status

if __name__ == '__main__':
    sys.exit(main())
