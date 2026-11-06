"""Role/deadline admission around the maintained native scheduler; no new scheduler.

Each operation JSON is frozen before execution. Shared D/E and per-role escrow
already reserve the allowance; native reservations debit the chosen role again
as an operational projection, never a new allowance. No refunds or replacements.
"""
import argparse
import datetime
import json
from pathlib import Path
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
sys.path[:0] = [str(REPO/'benchmarks/brn_agent'), str(REPO/'scripts'),
                str(REPO/'benchmarks/brn_agent/pilots/terra_sol_v3')]
from deadline_guard import Guard
from limits import Reservations
from evidence import identity_matches
import evaluate_wiki_v2 as native

CAPS = {'phase1':600, 'phase2-development':264, 'phase2-broad':432,
        'phase2-fresh':144, 'grader':120}

class CycleStop(BaseException):
    """Escapes the runner's per-sample failure handling before another launch."""


def audit_result(raw):
    if raw.get('status') not in ('running', 'complete') or raw.get('safety'):
        raise CycleStop('Retained failure/safety stop')
    for event in raw.get('steps', []):
        if event.get('kind') not in ('ask', 'intake') or event.get('terminal') is None:
            continue
        if not native.evaluated(event['terminal']):
            raise CycleStop('Retained provider failure')
        if not identity_matches(event.get('trace'), raw['model'], raw['effort']):
            raise CycleStop('Runtime model/effort identity unavailable or mismatched')
        budget = (event.get('trace') or {}).get('model_call_budget')
        if not budget or budget.get('started', 0) > budget.get('limit', 0):
            raise CycleStop('Runtime accounting unavailable')


def audit(root):
    for path in Path(root).glob('runs/*/*/result.json'):
        raw = json.loads(path.read_text())
        audit_result(raw)


def active_audit():
    """Observe the terminal set before run_step's follow-up read/persist."""
    active = []
    original = native.Sample.run_step
    def checked(sample, *args, **kwargs):
        active.append(sample)
        try:
            return original(sample, *args, **kwargs)
        except CycleStop:
            sample.persist()
            raise
        finally:
            active.pop()
    native.Sample.run_step = checked
    def check():
        for sample in active:
            audit_result(sample.result)
    return check


def install_role(role_path, role, operation, expected_calls):
    """Debit the protected role before local native reservation, conservatively."""
    original = native.LIMITS.Reservations
    class ScopedReservations(original):
        def reserve(self, identity, calls):
            if calls != expected_calls:
                raise CycleStop('Native cap differs from frozen operation')
            Reservations(role_path, CAPS[role]).reserve(operation+'/'+identity, calls)
            return super().reserve(identity, calls)
    native.LIMITS.Reservations = ScopedReservations


def audit_operations(roles, current=None):
    for marker in roles.glob('operation-*.json'):
        operation = json.loads(marker.read_text())
        audit(operation['root'])
        receipt = Path(operation['root']+'.deadline.jsonl')
        if receipt.exists():
            rows = [json.loads(line) for line in receipt.read_text().splitlines()]
            terminals = [r for r in rows if r.get('event') == 'terminal']
            if terminals and terminals[-1].get('exit_code') != 0:
                raise CycleStop('Earlier operation failed; no automatic cycle continuation')
            if not terminals and operation['root'] != current:
                raise CycleStop('Earlier operation outcome uncertain')
        elif operation['root'] != current:
            raise CycleStop('Earlier operation never recorded a terminal outcome')


def phase_deadline(roles, phase, now):
    if phase not in ('phase1', 'phase2'):
        raise CycleStop('Explicit frozen phase required')
    path = roles/(phase+'-clock.json')
    try:
        with path.open('x') as stream:
            json.dump({'started_at':now.isoformat(), 'seconds':3600 if phase == 'phase1' else 5400}, stream)
    except FileExistsError:
        pass
    clock = json.loads(path.read_text())
    expected = 3600 if phase == 'phase1' else 5400
    if clock.get('seconds') != expected:
        raise CycleStop('Phase clock differs from original limit')
    return datetime.datetime.fromisoformat(clock['started_at'])+datetime.timedelta(seconds=expected)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--operation', type=Path, required=True)
    parser.add_argument('--account-file', type=Path, required=True)
    parser.add_argument('--cycle', type=Path, required=True)
    parser.add_argument('--roles', type=Path, required=True)
    args = parser.parse_args(argv)
    op = json.loads(args.operation.read_text())
    role = op['role']
    if role not in CAPS or not args.account_file.is_absolute():
        raise ValueError('Explicit frozen role and account path required')
    now = datetime.datetime.now(datetime.timezone.utc)
    deadline = datetime.datetime.fromisoformat(json.loads(args.cycle.read_text())['whole_cycle_deadline'])
    root = Path(op['root'])
    roles = args.roles.resolve()
    roles.mkdir(parents=True, exist_ok=True)
    phase = 'phase1' if role == 'phase1' else 'phase2' if role.startswith('phase2-') else op.get('phase')
    deadline = min(deadline, phase_deadline(roles, phase, now))
    remaining = (deadline-now).total_seconds()
    if remaining < op['minimum_seconds']+30:
        raise CycleStop('Insufficient original cycle/phase time; operation not admitted')
    audit_operations(roles)
    # Exclusive operation identity also prevents restarting a partially dispatched
    # reader/grader whose runner did not yet get as far as its own reservation.
    marker = roles/('operation-'+op['id']+'.json')
    with marker.open('x') as stream:
        json.dump({'root':str(root), 'role':role}, stream)
    role_path = roles/(role+'-dispatch.jsonl')
    guard = Guard(str(root)+'.deadline.jsonl', min(op['seconds'], remaining),
                  {'operation':op['id'], 'role':role, 'original_deadline':deadline.isoformat()})
    check_active = active_audit()
    def before_launch():
        check_active()
        audit_operations(roles, str(root))
    guard.before_launch = before_launch
    guard.install()
    status = 1
    try:
        if op['kind'] == 'candidate':
            if op['model_call_limit'] != 6 or op['max_calls'] > CAPS[role]:
                raise CycleStop('Unexpected candidate HTTP cap')
            install_role(role_path, role, op['id'], 6)
            command = ['--live', '--fixture', op['fixture'], '--truth', op['truth'],
                '--root', str(root), '--seeder', op['seeder'], '--cases', op['cases'],
                '--repetitions', str(op['repetitions']), '--provider', 'Codex', '--effort', 'medium',
                '--auth-file', str(args.account_file), '--model-call-limit', '6',
                '--max-total-model-calls', str(op['max_calls']), '--timeout', '60']
            for arm in op['arms']:
                command += ['--arm', arm]
            for key, flag in [('stock_helper','--stock-helper'), ('brn_helper','--brn-helper')]:
                command += [flag, op[key]]
            status = native.main(command)
            audit(root)
        elif op['kind'] == 'reader':
            install_role(role_path, role, op['id'], 12)
            import reader
            result = reader.measured(Path(op['checkpoint']), op['question'], op['records'], 'wiki-only',
                Path(op['binary']), root, auth_file=args.account_file, live=True)
            if result.get('status') != 'complete' or not result.get('measured_answer'):
                raise CycleStop('Reader identity/retrieval/completion unverified')
            status = 0
        elif op['kind'] == 'grader':
            Reservations(role_path, CAPS[role]).reserve(op['id'], 1)
            root.mkdir(parents=True, exist_ok=False)
            with (root/'receipt.json').open('x') as output, (root/'stderr.txt').open('x') as error:
                status = subprocess.run([op['binary'], '--live', str(args.account_file),
                    op['packet'], op['instructions']], stdout=output, stderr=error,
                    timeout=50, check=False).returncode
        else:
            raise CycleStop('Unknown operation kind')
    except (CycleStop, Exception) as error:
        status = 1
        guard.record('stopped', error_class=type(error).__name__)
    finally:
        status = guard.finish(status)
    return status

if __name__ == '__main__':
    sys.exit(main())
