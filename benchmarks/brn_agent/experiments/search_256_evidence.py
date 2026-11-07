"""Additional read-only oracles for issue256; native scoring stays authoritative."""
import json
import hashlib
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))

MUTATIONS = {'prepare_changes', 'prepare_note_edit', 'apply_changes',
             'refresh_candidate', 'withdraw_proposal', 'retain_progress'}


def full_calls(trace):
    """Retain failed attempts; incomplete parent/helper evidence is unknown."""
    if (not isinstance(trace, dict) or trace.get('level') != 'full'
            or trace.get('dropped_calls') != 0 or not isinstance(trace.get('calls'), list)):
        return None
    calls = list(trace['calls'])
    if any(not isinstance(call, dict) or not isinstance(call.get('tool'), str) for call in calls):
        return None
    helpers = trace.get('helpers', [])
    if not isinstance(helpers, list):
        return None
    for helper in helpers:
        nested = full_calls(helper.get('trace') if isinstance(helper, dict) else None)
        if nested is None:
            return None
        calls.extend(nested)
    return calls


def read_only(folder):
    """Bookkeeping may change; knowledge, proposals and mutation attempts may not."""
    folder = Path(folder)
    raw = json.loads((folder / 'result.json').read_text())
    calls = [full_calls(step.get('trace')) for step in raw.get('steps', [])]
    if not calls or any(group is None for group in calls):
        return {'passed': None, 'reason': 'Missing full tool trace'}
    mutations = [call['tool'] for group in calls for call in group if call['tool'] in MUTATIONS]
    operations = [op for step in raw['steps'] for op in step.get('operations', [])]
    checkpoints = [folder / 'checkpoints' / str(i) / 'records.json' for i in (0, 1)]
    if not all(path.is_file() for path in checkpoints):
        return {'passed': None, 'reason': 'Missing before/after canonical checkpoints'}
    def knowledge(path):
        records = json.loads(path.read_text())
        return {row['id']: row for row in records
                if set(row['data']) & {'Note', 'Link', 'Action'}}
    same = knowledge(checkpoints[0]) == knowledge(checkpoints[1])
    return {'passed': not mutations and not operations and same,
            'mutation_tools': mutations, 'operations': operations, 'knowledge_unchanged': same}


def complete_usage(raw, usage):
    """Include separately reported summaries without inventing missing counts."""
    # The native extractor retains parent usage separately from compactor usage.
    # Do not turn an unreported HTTP attempt into a zero-token observation.
    complete = True
    for step in raw.get('steps', []):
        trace = step.get('trace') or {}
        if trace.get('helpers'):
            complete = False  # This frozen helper-off protocol cannot qualify their usage.
        turns = trace.get('turn_usage') or []
        summary_usage = (trace.get('summary') or {}).get('usage')
        if summary_usage:
            for key in ('input', 'output', 'cached_input'):
                if usage[key] is None or summary_usage.get(key) is None:
                    complete = False
                else:
                    usage[key] += summary_usage[key]
        observed = len(turns) + bool(summary_usage)
        if (not turns or any(not (turn.get('input') or turn.get('output')) for turn in turns)
                or observed != len(trace.get('wire_requests') or [])):
            complete = False
    if any(usage[key] is None for key in ('input', 'output', 'cached_input')):
        complete = False
    if not complete:
        for key in ('input', 'output', 'cached_input', 'uncached_input'):
            usage[key] = None
    else:
        usage['uncached_input'] = usage['input'] - usage['cached_input']
    return complete


def attempt(path, root):
    """Use maintained all-attempt usage/identity extraction, preserving unknowns."""
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from evidence import native_attempt
    row = native_attempt(path, root)
    raw = json.loads(Path(path).read_text())
    row['reported_usage_totals'] = dict(row['usage'])
    row['all_HTTP_usage_observed'] = complete_usage(raw, row['usage'])
    row['read_only'] = read_only(Path(path).parent)
    row['usage']['cost'] = None  # Subscription charges are not observed.
    steps = raw.get('steps', [])
    # This frozen projection has exactly one ask and no helper. Missing event,
    # identity, usage, safety, check or budget evidence cannot qualify a pair.
    events_complete = len(steps) == 1 and steps[0].get('kind') == 'ask'
    for step in steps:
        trace = step.get('trace') or {}
        budget = trace.get('model_call_budget') or {}
        started = budget.get('started')
        events_complete = (events_complete and step.get('terminal') == 'Completed'
            and budget.get('limit') == 6 and budget.get('refused') is False
            and type(started) is int and 1 <= started <= 6
            and started == len(trace.get('wire_requests') or [])
            and trace.get('wire_requests_dropped') == 0
            and trace.get('helpers', []) == [])
    row['complete_for_pair'] = (raw.get('status') == 'complete' and events_complete
        and raw.get('provider') == 'Codex' and raw.get('model') == 'gpt-6.1-sol'
        and raw.get('effort') == 'medium' and raw.get('helper') is None
        and row['runtime_identity_verified'] is True and row['all_HTTP_usage_observed']
        and row['automatic_checks']['status'] == 'passed' and raw.get('safety') == []
        and row['read_only']['passed'] is True)
    return row


def answer_packet(path, root, fixture_path, manifest_path):
    """Bound anonymous evaluation packet; full records are evaluator gold only.

    The maintained checkpoint verifier checks native backup/record/result hashes.
    Only relevant/observed records enter this compact grader packet. The read
    trace stays separate, so evaluator access never becomes candidate read proof.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from evidence import verified_proof_packet, anonymize_local_paths
    raw = json.loads(Path(path).read_text())
    fixture = json.loads(Path(fixture_path).read_text())
    case = next(c for c in fixture['cases'] if c['id'] == raw['case'])
    seed = json.loads((Path(root) / 'templates' / f'{raw["case"]}-1000' / 'seed.json').read_text())
    gold = verified_proof_packet(Path(path).parent / 'checkpoints/1', manifest_path=manifest_path)
    required = set()
    for field in ('targets', 'useful', 'traps', 'private'):
        for key in case.get(field, []):
            if isinstance(key, str) and key in seed['notes']:
                required.add(seed['notes'][key])
    requested = set()
    calls = []
    for step in raw.get('steps', []):
        traced = full_calls(step.get('trace'))
        if traced is None:
            raise ValueError('Incomplete grader tool evidence')
        for call in traced:
            calls.append(call)
            args = call.get('args') or {}
            requested.update(args.get('ids') or [])
            if isinstance(args.get('id'), str):
                requested.add(args['id'])
            if call.get('outcome') != 'error':
                required.update((call.get('result') or {}).get('ids') or [])
    # Semantic gold excludes Messages and archived records. Retain selected
    # records of those kinds separately, from the same verified checkpoint.
    records_bytes = (Path(path).parent / 'checkpoints/1/records.json').read_bytes()
    if hashlib.sha256(records_bytes).hexdigest() != gold['records_sha256']:
        raise ValueError('Verified grader checkpoint changed')
    records = json.loads(records_bytes)
    available = {row['id'] for row in records}
    uncovered = required - available
    if uncovered:
        raise ValueError('Selected records absent from verified checkpoint: ' + ', '.join(sorted(uncovered)))
    selected = required | requested
    gold_ids = {row['id'] for row in gold['records']}
    packet = {'question': case['steps'][0]['prompt'], 'expected_rubric': case.get('rubric'),
              'expected_checks': case['checks'], 'answer': raw['steps'][0].get('answer'),
              'execution_terminal': raw['steps'][0].get('terminal'),
              'gold_records': [row for row in gold['records'] if row['id'] in selected],
              'additional_checkpoint_records': [row for row in records
                  if row['id'] in selected and row['id'] not in gold_ids],
              'unavailable_requested_ids': sorted(requested - available),
              'actual_trace': calls, 'gold_access_is_not_candidate_read_proof': True,
              'native_result_sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest(),
              'canonical_records_sha256': gold['records_sha256']}
    packet = anonymize_local_paths(packet, root)
    digest = hashlib.sha256(json.dumps(packet, sort_keys=True, separators=(',', ':'),
                                     ensure_ascii=False).encode()).hexdigest()
    return {'packet_sha256': digest, 'packet': packet}
