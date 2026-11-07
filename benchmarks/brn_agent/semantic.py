"""Blind frozen-output semantic grading protocol, with controlled offline replies.

This validates evidence and schema, not the semantic competence of any grader.
No model or subprocess is called. The original 12 calibration pairs stay separate.
"""
import json
from pathlib import Path
from evidence import verify_manifest, verify_grade_references, verified_proof_packet, anonymize_local_paths, full_calls
from reader import packet_hash
from score import assess, sha256, validate_suite


def prepare(root, suite_path, manifest_path, truth_path=None):
    """Freeze one packet for every predeclared candidate-event checkpoint.

    Version two replaces latest-only packet preparation. Earlier frozen evidence
    remains readable; regenerate packets rather than rerunning the candidate.
    """
    root = Path(root).resolve()
    suite = validate_suite(json.loads(Path(suite_path).read_text()))
    manifest = json.loads(Path(manifest_path).read_text())
    if manifest['suite_sha256'] != sha256(suite_path):
        raise ValueError('Semantic suite does not match frozen evidence')
    verify_manifest(manifest, root)
    cases = {c['id']: c for c in suite['scenarios']}
    packets, key, missing, plans = [], {}, [], {}
    for attempt in manifest['attempts']:
        case = cases[attempt['scenario']]
        events = case.get('events')
        if type(events) is not int or events < 1:
            raise ValueError('Checkpoint grading requires a positive frozen event count')
        if case.get('checkpoint_aggregation', 'minimum') != 'minimum':
            raise ValueError('Unsupported predeclared checkpoint aggregation')
        if case.get('criterion_aggregation', {}) not in ({}, {'downstream_usefulness':'final'}):
            raise ValueError('Unsupported criterion aggregation')
        required = case.get('grade_checkpoints', list(range(1, events + 1)))
        if not isinstance(required, list) or not required or any(type(n) is not int or not 1 <= n <= events for n in required) or required != sorted(set(required)):
            raise ValueError('Grade checkpoints must be sorted unique candidate-event numbers')
        raw_path = root / attempt['result']
        raw = json.loads(raw_path.read_text())
        captured = raw.get('checkpoints', [])
        available = [c['step'] for c in captured]
        if any(type(n) is not int or not 0 <= n <= events for n in available) or len(available) != len(set(available)):
            raise ValueError('Captured checkpoint numbers are invalid or duplicated')
        absent = [n for n in required if n not in available]
        if absent:
            missing.append(attempt['id'])
        plans[attempt['id']] = {'required': required, 'uncaptured': absent,
                                'aggregation': 'minimum', 'execution_status': attempt['status']}
        for number in required:
            if number not in available:
                continue
            packet = verified_proof_packet(raw_path.parent/'checkpoints'/str(number),
                                           scenario=attempt['scenario'], step=number,
                                           manifest_path=manifest_path, truth_path=truth_path)
            packet['criteria'] = case['criteria']
            if case.get('criterion_aggregation'):
                packet['criterion_aggregation'] = case['criterion_aggregation']
            packet['scale'] = {'0': 'failed', '1': 'seriously flawed', '2': 'material correction required',
                               '3': 'minor correction required', '4': 'meets criterion'}
            packet['tool_evidence'] = [full_calls(s.get('trace')) for s in raw.get('steps', [])[:number]
                                       if s.get('kind') == 'ask']
            packet['tool_evidence_complete'] = all(c is not None for c in packet['tool_evidence'])
            packet['events'] = [{k: step[k] for k in ('index', 'kind', 'terminal', 'answer', 'operations', 'receipts') if k in step}
                                for step in raw.get('steps', [])[:number]]
            event = next((event for event in raw.get('steps', []) if event.get('index') == number), {})
            packet['execution_status'] = event.get('terminal', 'unknown_at_checkpoint')
            packet['checkpoint'] = number
            packet['grading_instruction'] = 'Assess only this checkpoint and events so far, not later recovery or future goals.'
            packet = anonymize_local_paths(packet, root)
            identity = packet_hash({'attempt': attempt['id'], 'checkpoint': number, 'packet': packet})
            packets.append({'id': identity, 'packet': packet, 'packet_sha256': packet_hash(packet)})
            key[identity] = {'attempt': attempt['id'], 'result_sha256': attempt['result_sha256'],
                             'checkpoint': number, 'packet_sha256': packet_hash(packet)}
    return {'format': 2, 'packets': packets, 'private_key': key, 'missing_attempts': missing,
            'checkpoint_plans': plans, 'suite_sha256': sha256(suite_path),
            'evidence_sha256': sha256(manifest_path), 'qualified': False,
            'scope': 'All predeclared candidate-event checkpoints; per-criterion minimum gives one grade per attempt.'}


def import_controlled(root, suite_path, manifest_path, prepared, replies, revision, truth_path=None, *, review_kind='controlled-fixture'):
    if not isinstance(revision, str) or not revision.strip():
        raise ValueError('An explicit controlled grader revision is required')
    if review_kind not in ('controlled-fixture', 'independent-review'):
        raise ValueError('Unknown review origin')
    prefix = 'independent:' if review_kind == 'independent-review' else 'controlled:'
    expected = prepare(root, suite_path, manifest_path, truth_path=truth_path)
    if prepared != expected:
        raise ValueError('Prepared grading packets changed; regenerate version-two packets from frozen evidence')
    seen, checkpoint_grades, failures = set(), [], []
    suite = json.loads(Path(suite_path).read_text())
    manifest = json.loads(Path(manifest_path).read_text())
    attempts = {a['id']: a for a in manifest['attempts']}
    for reply in replies:
        identity = reply['id']
        if identity not in prepared['private_key'] or identity in seen:
            raise ValueError('Unknown or duplicate semantic packet reply')
        seen.add(identity)
        binding = prepared['private_key'][identity]
        if reply['packet_sha256'] != binding['packet_sha256']:
            raise ValueError('Grader response belongs to different evidence')
        if 'error' in reply:
            if set(reply) != {'id', 'packet_sha256', 'error'} or not isinstance(reply['error'], str) or not reply['error']:
                raise ValueError('Invalid grader failure record')
            failures.append({'packet': identity, 'error': reply['error']})
            continue
        if set(reply) != {'id', 'packet_sha256', 'criteria', 'completed', 'breaches'}:
            raise ValueError('Semantic response must use the exact schema')
        if not isinstance(reply['criteria'], dict) or any(
                not isinstance(c, dict) or not isinstance(c.get('evidence'), list)
                or any(not isinstance(ref, dict) for ref in c['evidence'])
                for c in reply['criteria'].values()):
            raise ValueError('Checkpoint criteria require structured evidence references')
        grade = {'attempt': binding['attempt'], 'result_sha256': binding['result_sha256'],
                 'grader_revision': prefix + revision, 'review_kind': review_kind, 'criteria': reply['criteria'],
                 'completed': reply['completed'], 'breaches': reply['breaches'],
                 'checkpoint': binding['checkpoint'], 'packet': identity}
        # A valid reference to a later checkpoint is still information unavailable
        # at this judgment boundary. Do not allow future evidence into a verdict.
        for criterion in grade['criteria'].values():
            for reference in criterion.get('evidence', []):
                if any(type(reference.get(field)) is int and reference[field] > binding['checkpoint']
                       for field in ('checkpoint', 'step')):
                    raise ValueError('Checkpoint verdict cannot cite future evidence')
        verify_grade_references(manifest, [grade], root)
        attempt = attempts[binding['attempt']]
        assess(suite, attempt['configuration'], manifest['attempts'], [grade])
        checkpoint_grades.append(grade)
    grades = []
    for attempt_id, plan in prepared['checkpoint_plans'].items():
        observed = sorted((g for g in checkpoint_grades if g['attempt'] == attempt_id), key=lambda g: g['checkpoint'])
        if not observed:
            continue
        criteria = {}
        case = next(c for c in suite['scenarios'] if c['id'] == attempts[attempt_id]['scenario'])
        for name in observed[0]['criteria']:
            scores = [g['criteria'][name] for g in observed]
            references = []
            for score in scores:
                for reference in score['evidence']:
                    if reference not in references:
                        references.append(reference)
            final_only = case.get('criterion_aggregation', {}).get(name) == 'final'
            value = (scores[-1]['score'] if observed[-1]['checkpoint'] == max(plan['required']) else None) if final_only else (
                None if any(s['score'] is None for s in scores) else min(s['score'] for s in scores))
            criteria[name] = {'score': value, 'evidence': references,
                              'explanation': ('Final checkpoint verdict; earlier entries diagnostic: ' if final_only else 'Minimum across checkpoint verdicts: ') + '; '.join(
                                  f"{g['checkpoint']}: {g['criteria'][name]['score']} — {g['criteria'][name]['explanation']}"
                                  for g in observed)}
        complete = [g['checkpoint'] for g in observed] == plan['required']
        grades.append({'attempt': attempt_id, 'result_sha256': attempts[attempt_id]['result_sha256'],
                       'grader_revision': prefix + revision, 'review_kind': review_kind, 'criteria': criteria,
                       'completed': complete and all(g['completed'] for g in observed),
                       'breaches': sorted({b for g in observed for b in g['breaches']}),
                       'checkpoint_complete': complete,
                       'checkpoint_aggregation': 'minimum',
                       'graded_checkpoints': [g['checkpoint'] for g in observed]})
    verify_grade_references(manifest, grades, root)
    for configuration in {a['configuration'] for a in manifest['attempts']}:
        ids = {a['id'] for a in manifest['attempts'] if a['configuration'] == configuration}
        assess(suite, configuration, manifest['attempts'], [g for g in grades if g['attempt'] in ids])
    missing = sorted(set(prepared['private_key']) - seen)
    infrastructure = [a['id'] for a in manifest['attempts'] if a['status'] not in ('complete', 'model_failure')]
    return {'format': 2, 'status': 'complete' if prepared['packets'] and not missing and not failures and not prepared['missing_attempts'] and not infrastructure else 'incomplete',
            'qualified': False, 'provider': review_kind, 'grades': grades,
            'checkpoint_grades': checkpoint_grades, 'grader_failures': failures,
            'missing_packets': missing, 'uncaptured_attempts': prepared['missing_attempts'],
            'checkpoint_plans': prepared['checkpoint_plans'], 'infrastructure_attempts': infrastructure,
            'candidate_calls': 0, 'cost': None, 'revision': revision}


def execute_controlled(root, suite_path, manifest_path, fixture, output, truth_path=None):
    """Execute a bounded offline provider transcript against frozen packets.

    Responses are fixture data, not inferred grades. Requests are generated from
    current hash-verified evidence; the fixture cannot supply or replace them.
    Each fixture turn is consumed at most once. There are no retries, network
    clients, candidate processes or arbitrary provider callbacks.
    """
    required = {'revision', 'max_calls', 'max_request_bytes', 'max_response_bytes', 'calls'}
    if not isinstance(fixture, dict) or set(fixture) != required:
        raise ValueError('Controlled grader requires the exact bounded fixture schema')
    for name, maximum in [('max_calls', 10000), ('max_request_bytes', 16 * 1024 * 1024),
                          ('max_response_bytes', 1024 * 1024)]:
        if type(fixture[name]) is not int or not 1 <= fixture[name] <= maximum:
            raise ValueError('Invalid controlled grader limit: ' + name)
    if not isinstance(fixture['revision'], str) or not fixture['revision'].strip():
        raise ValueError('Controlled grader requires a revision')
    if not isinstance(fixture['calls'], list) or len(fixture['calls']) > 10000:
        raise ValueError('Controlled grader calls must be a bounded list')
    prepared = prepare(root, suite_path, manifest_path, truth_path=truth_path)
    packets = {p['id']: p for p in prepared['packets']}
    turns = {}
    for turn in fixture['calls']:
        if not isinstance(turn, dict) or set(turn) not in (
                {'id', 'packet_sha256', 'response'}, {'id', 'packet_sha256', 'error'}):
            raise ValueError('Invalid controlled grader turn')
        identity = turn['id']
        if identity not in packets or identity in turns or turn['packet_sha256'] != packets[identity]['packet_sha256']:
            raise ValueError('Unknown, repeated or unbound controlled grader turn')
        turns[identity] = turn
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    result = {'format': 1, 'status': 'running', 'qualified': False,
              'provider': 'controlled-fixture', 'candidate_calls': 0, 'cost': None,
              'fixture_sha256': packet_hash(fixture), 'calls': 0,
              'request_bytes': 0, 'response_bytes': 0, 'revision': fixture['revision']}
    def persist():
        (output/'execution.json').write_text(json.dumps(result, indent=2))
    def append(name, row):
        with (output/name).open('a') as stream:
            stream.write(json.dumps(row, ensure_ascii=False) + '\n')
    persist()
    (output/'private-bindings.json').write_text(json.dumps(prepared, indent=2))
    replies = []
    try:
        for packet in prepared['packets']:
            turn = turns.get(packet['id'])
            if turn is None:
                continue  # Explicitly incomplete; never manufacture a provider reply.
            request = {'id': packet['id'], 'packet_sha256': packet['packet_sha256'],
                       'packet': packet['packet'], 'revision': fixture['revision']}
            size = len(json.dumps(request, ensure_ascii=False).encode())
            if result['calls'] >= fixture['max_calls'] or result['request_bytes'] + size > fixture['max_request_bytes']:
                result['budget_refused'] = packet['id']
                break
            append('requests.jsonl', request)
            result['calls'] += 1
            result['request_bytes'] += size
            persist()  # A dispatch remains counted even when its response fails.
            response = {'id': packet['id'], 'packet_sha256': packet['packet_sha256']}
            if 'error' in turn:
                if not isinstance(turn['error'], str) or not turn['error']:
                    raise ValueError('Controlled provider error must be nonempty text')
                response['error'] = turn['error']
            else:
                if not isinstance(turn['response'], dict) or set(turn['response']) != {'criteria', 'completed', 'breaches'}:
                    raise ValueError('Invalid controlled semantic response')
                response.update(turn['response'])
            response_size = len(json.dumps(response, ensure_ascii=False).encode())
            append('provider-events.jsonl', {'id': packet['id'], 'response_sha256': packet_hash(response),
                                            'response_bytes': response_size,
                                            'accepted': result['response_bytes'] + response_size <= fixture['max_response_bytes']})
            if result['response_bytes'] + response_size > fixture['max_response_bytes']:
                response = {'id': packet['id'], 'packet_sha256': packet['packet_sha256'],
                            'error': 'Controlled response byte limit exceeded'}
                result['response_limit_exceeded'] = True
            else:
                result['response_bytes'] += response_size
            append('responses.jsonl', response)
            replies.append(response)
        graded = import_controlled(root, suite_path, manifest_path, prepared, replies,
                                   fixture['revision'], truth_path=truth_path)
        (output/'grades.json').write_text(json.dumps(graded['grades'], indent=2))
        (output/'grading-result.json').write_text(json.dumps(graded, indent=2))
        result.update(status=graded['status'], missing_packets=graded['missing_packets'],
                      grader_failures=graded['grader_failures'])
    except (ValueError, KeyError, OSError) as error:
        result.update(status='failed', error=str(error))
        raise
    finally:
        try:
            if prepare(root, suite_path, manifest_path, truth_path=truth_path) != prepared:
                raise ValueError('Frozen grading inputs changed during execution')
        except (ValueError, KeyError, OSError) as error:
            result.update(status='failed', error=str(error))
            persist()
            raise
        persist()
    return result


def import_reviewed(root, suite_path, manifest_path, prepared, replies, revision, reviewer_config, truth_path=None):
    """Import independently obtained reviews; never dispatch or infer judge competence."""
    config = json.loads(Path(reviewer_config).read_text())
    if config != json.loads((Path(__file__).parent/'wiki_challenge/reviewer.json').read_text()):
        raise ValueError('Use the frozen reviewer configuration; version changes require new reports')
    identity = sha256(reviewer_config)
    result = import_controlled(root, suite_path, manifest_path, prepared, replies,
                               identity + ':' + revision, truth_path, review_kind='independent-review')
    result.update(reviewer_config_sha256=identity,
                  review_identity_status='independently supplied review; runtime identity not attested by importer')
    return result
