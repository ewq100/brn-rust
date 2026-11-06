"""Additional read-only oracles for issue256; native scoring stays authoritative."""
import json
import hashlib
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
import evaluate_wiki_v2 as v2

MUTATIONS = {'prepare_changes', 'prepare_note_edit', 'apply_changes',
             'refresh_candidate', 'withdraw_proposal', 'retain_progress'}


def read_only(folder):
    """Bookkeeping may change; knowledge, proposals and mutation attempts may not."""
    folder = Path(folder)
    raw = json.loads((folder / 'result.json').read_text())
    calls = [v2.traced_calls(step) for step in raw.get('steps', [])]
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
    row['complete_for_pair'] = (raw.get('status') == 'complete'
        and all(step.get('terminal') == 'Completed' for step in raw.get('steps', []))
        and row['read_only']['passed'] is True
        and all(not (step.get('trace') or {}).get('model_call_budget', {}).get('refused')
                for step in raw.get('steps', [])))
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
    selected = set()
    for field in ('targets', 'useful', 'traps', 'private'):
        for key in case.get(field, []):
            if isinstance(key, str) and key in seed['notes']:
                selected.add(seed['notes'][key])
    calls = []
    for step in raw.get('steps', []):
        for call in v2.traced_calls(step) or []:
            calls.append(call)
            args = call.get('args') or {}
            selected.update(args.get('ids') or [])
            if isinstance(args.get('id'), str):
                selected.add(args['id'])
            selected.update((call.get('result') or {}).get('ids') or [])
    packet = {'question': case['steps'][0]['prompt'], 'expected_rubric': case.get('rubric'),
              'expected_checks': case['checks'], 'answer': raw['steps'][0].get('answer'),
              'execution_terminal': raw['steps'][0].get('terminal'),
              'gold_records': [row for row in gold['records'] if row['id'] in selected],
              'actual_trace': calls, 'gold_access_is_not_candidate_read_proof': True,
              'native_result_sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest(),
              'canonical_records_sha256': gold['records_sha256']}
    packet = anonymize_local_paths(packet, root)
    digest = hashlib.sha256(json.dumps(packet, sort_keys=True, separators=(',', ':'),
                                     ensure_ascii=False).encode()).hexdigest()
    return {'packet_sha256': digest, 'packet': packet}
