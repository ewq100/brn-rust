"""One finite search cycle; reuse v2 execution, reservations and v3 deadlines.

This is an experiment entrypoint, not a provider client or scheduler. Bindings
and the corpus are prepared offline. Every phase is consumed before dispatch;
there is no resume/refund path and no candidate revision two in this allocation.
"""
import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'scripts'))
sys.path.insert(0, str(REPO / 'benchmarks/brn_agent'))
sys.path.insert(0, str(REPO / 'benchmarks/brn_agent/pilots/terra_sol_v3'))
import evaluate_wiki_v2 as v2
from deadline_guard import Guard
from limits import Reservations
from evidence import identity_matches

PHASES = {'baseline-first': (18, 1), 'candidate': (36, None),
          'baseline-second': (18, 2), 'confirmation': (48, None)}


class CycleStop(BaseException):
    """Escape per-sample recovery before another checkpoint/provider launch."""


def audit_result(raw, current):
    if raw.get('status') not in (('running', 'complete') if current else ('complete',)) or raw.get('safety'):
        raise CycleStop('Retained failure or safety stop')
    for step in raw.get('steps', []):
        if step.get('kind') != 'ask' or step.get('terminal') is None:
            continue
        trace = step.get('trace') or {}
        budget = trace.get('model_call_budget') or {}
        if step['terminal'] != 'Completed' or not identity_matches(trace, 'gpt-6.1-sol', 'medium'):
            raise CycleStop('Failed event or unverified model identity')
        if budget.get('limit') != 6 or budget.get('refused') or not 1 <= budget.get('started', 0) <= 6:
            raise CycleStop('Unknown or exhausted event accounting')


def audit_results(root, current):
    folders = list((Path(root) / 'runs').glob('*/*'))
    if not current and not folders:
        raise CycleStop('Earlier phase has no retained samples')
    for folder in folders:
        path = folder / 'result.json'
        if not path.is_file():
            raise CycleStop('Sample outcome uncertain; no automatic continuation')
        audit_result(json.loads(path.read_text()), current)


def audit_phases(owned, bindings, current=None):
    ledger = owned / 'BRN-phase-reservations.jsonl'
    rows = [json.loads(line) for line in ledger.read_text().splitlines()] if ledger.exists() else []
    for row in rows[1:]:
        name = row['identity']
        argv = bindings['phases'][name]['argv']
        root = Path(argv[argv.index('--root') + 1])
        receipt = owned / (name + '-deadline.jsonl')
        is_current = name == current
        if not receipt.is_file():
            raise CycleStop('Earlier phase has no terminal receipt; outcome uncertain')
        events = [json.loads(line) for line in receipt.read_text().splitlines()]
        terminals = [event for event in events if event.get('event') == 'terminal']
        if terminals and terminals[-1].get('exit_code') != 0:
            raise CycleStop('Earlier deadline/phase failed')
        if not terminals and not is_current:
            raise CycleStop('Earlier phase has no terminal receipt; outcome uncertain')
        audit_results(root, is_current)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bindings', type=Path, required=True)
    parser.add_argument('--phase', choices=PHASES, required=True)
    args = parser.parse_args()
    bindings = json.loads(args.bindings.read_text())
    owned = Path(bindings['owned_root'])
    if not owned.is_absolute() or not owned.is_dir():
        parser.error('Use a pre-created task-owned absolute root')
    phase = bindings['phases'][args.phase]
    # Include binary/helper/evaluator/corpus/truth/settings and this driver in
    # offline-generated hash locks. Credential paths are never files in a lock.
    locked_files = {**bindings['fixed_files'], **phase['files']}
    for index, flag in enumerate(phase['argv']):
        if flag == '--arm':
            binary = v2.parse_arm(phase['argv'][index + 1])['binary']
            if binary not in locked_files:
                raise ValueError('Every dispatched arm binary needs a frozen hash')
    for path, expected in locked_files.items():
        if not Path(path).is_absolute() or digest(path) != expected:
            raise ValueError('Frozen input mismatch; do not dispatch')
    calls, repetition = PHASES[args.phase]
    deadline_path = owned / 'live-deadline.json'
    ledger = owned / 'BRN-phase-reservations.jsonl'
    now = datetime.now(timezone.utc)
    if not deadline_path.exists():
        if args.phase != 'baseline-first' or ledger.exists():
            raise ValueError('Missing original deadline; no reset allowed')
        with deadline_path.open('x') as stream:
            json.dump({'started': now.isoformat(),
                       'deadline': (now + timedelta(seconds=5400)).isoformat()}, stream)
    deadline = json.loads(deadline_path.read_text())
    started = datetime.fromisoformat(deadline['started'])
    end = datetime.fromisoformat(deadline['deadline'])
    if end != started + timedelta(seconds=5400) or not started <= now < end:
        raise ValueError('Original live deadline expired or invalid')
    audit_phases(owned, bindings)
    previous = [json.loads(line)['identity'] for line in ledger.read_text().splitlines()[1:]] if ledger.exists() else []
    if previous != list(PHASES)[:list(PHASES).index(args.phase)]:
        raise CycleStop('Phase order differs from the frozen baseline/candidate plan')
    Reservations(ledger, 120).reserve(args.phase, calls)
    guard = Guard(owned / (args.phase + '-deadline.jsonl'), (end - now).total_seconds(), args.phase)
    active = [None]

    def before_launch():
        audit_phases(owned, bindings, args.phase)
        if active[0] is not None:
            # run_step observes ask's result before its subsequent read/checkpoint
            # but publishes only after returning; inspect that in-memory event.
            audit_result(active[0], True)

    guard.before_launch = before_launch
    guard.install()
    original_step = v2.Sample.run_step

    def tracked_step(sample, *positional):
        active[0] = sample.result
        try:
            return original_step(sample, *positional)
        except CycleStop:
            sample.persist()
            raise
        finally:
            active[0] = None

    v2.Sample.run_step = tracked_step
    # Reuse the same canonical corpus, including opaque IDs, in each fresh root.
    # Never seed another corpus under a purported pair.
    original_template = v2.ensure_template

    def frozen_template(options, root, case, scale, log):
        destination = root / 'templates' / f'{case["id"]}-{scale}'
        if not destination.exists():
            v2.copy_tree(Path(bindings['corpus_root']) / 'templates' / destination.name, destination)
        return original_template(options, root, case, scale, log)

    v2.ensure_template = frozen_template
    original_plan = v2.plan_samples

    def selected_plan(*positional, **keywords):
        plan = original_plan(*positional, **keywords)
        return [row for row in plan if repetition is None or row['repetition'] == repetition]

    v2.plan_samples = selected_plan
    status = 1
    try:
        status = v2.main(phase['argv'] + ['--live', '--model-call-limit', '6',
                         '--max-total-model-calls', str(calls), '--timeout', '90',
                         '--provider', 'Codex', '--model', 'gpt-6.1-sol', '--effort', 'medium'])
        audit_phases(owned, bindings, args.phase)
    except (CycleStop, Exception) as error:
        guard.record('stopped', error_class=type(error).__name__)
        status = 1
    finally:
        status = guard.finish(status)
    return status


if __name__ == '__main__':
    raise SystemExit(main())
