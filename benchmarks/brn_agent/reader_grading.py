"""Reader-answer adapters for the existing frozen anonymous grading protocol.

No model calls or scheduling. A native invocation and an independent judgment
are separately bound; neither a supplied answer nor a relabelled question counts.
"""
import copy
import json
from pathlib import Path
from evidence import verify_manifest, anonymize_local_paths
from reader import packet_hash, verify_measurement
from score import sha256, CHALLENGE_FAMILY, BREACHES

CONFIG = Path(__file__).parent/'wiki_challenge/reviewer.json'


def prepare(root, suite_path, evidence_path, truth_path, index):
    root = Path(root).resolve()
    suite = json.loads(Path(suite_path).read_text())
    manifest = json.loads(Path(evidence_path).read_text())
    truth = json.loads(Path(truth_path).read_text())
    if (suite.get('score_family') != CHALLENGE_FAMILY or manifest['suite_sha256'] != sha256(suite_path)
            or suite['gold_sha256'] != sha256(truth_path) or manifest.get('gold_sha256') != sha256(truth_path)
            or truth['fixture_sha256'] != manifest['fixture_sha256']):
        raise ValueError('Reader review requires the experiment-pinned challenge suite and truth')
    verify_manifest(manifest, root)
    if not isinstance(index, list):
        raise ValueError('Reader index must be a list of attempt/question/measurement bindings')
    attempts = {a['id']: a for a in manifest['attempts']}
    cases = {c['id']: c for c in suite['scenarios']}
    public, key, seen, invocations = [], {}, set(), set()
    for row in index:
        if not isinstance(row, dict) or set(row) != {'attempt', 'question_id', 'measurement'}:
            raise ValueError('Invalid reader-index entry')
        if row['attempt'] not in attempts:
            raise ValueError('Reader index refers to an unknown candidate attempt')
        attempt = attempts[row['attempt']]
        questions = truth['reader_questions'][attempt['scenario']]
        question = next((q for q in questions if q['id'] == row['question_id']), None)
        pair = (row['attempt'], row['question_id'])
        if question is None or pair in seen:
            raise ValueError('Unknown or duplicate reader question')
        seen.add(pair)
        case = cases[attempt['scenario']]
        final = max(case['grade_checkpoints'])
        native_path = root/attempt['result']
        native = json.loads(native_path.read_text())
        capture = next((c for c in native.get('checkpoints', []) if c['step'] == final), None)
        if capture is None:
            raise ValueError('Reader requires the final frozen candidate checkpoint')
        records = json.loads((native_path.parent/'checkpoints'/str(final)/'records.json').read_text())
        receipt = verify_measurement(row['measurement'], root, capture['records_sha256'], records)
        if receipt['question'] != question['question'] or receipt['invocation_identity'] in invocations:
            raise ValueError('Reader question mismatch or reused native invocation')
        invocations.add(receipt['invocation_identity'])
        allowed = {(r['id'], r['version']) for r in receipt['article_allowlist']}
        packet = dict(kind='reader-answer-review-v1', question=question['question'],
                      expected=question['expected'], answer=receipt['answer'],
                      reader_sha256=row['measurement']['sha256'],
                      records=[r for r in records if (r['id'],r['version']) in allowed],
                      instructions=json.loads(CONFIG.read_text())['instructions'],
                      criterion='downstream_usefulness', scale={'0':'failed','1':'seriously flawed',
                          '2':'material correction required','3':'minor correction required','4':'meets criterion'})
        if {(r['id'],r['version']) for r in packet['records']} != allowed:
            raise ValueError('Reader packet omits an exact selected article version')
        packet = anonymize_local_paths(packet, root)
        digest = packet_hash(packet)
        identity = packet_hash(dict(attempt=row['attempt'], question=row['question_id'], packet=digest))
        public.append(dict(id=identity, packet_sha256=digest, packet=packet))
        key[identity] = dict(attempt=row['attempt'], question_id=row['question_id'],
                             measurement=row['measurement'], packet_sha256=digest)
    expected = {(a['id'],q['id']) for a in manifest['attempts']
                for q in truth['reader_questions'][a['scenario']]}
    return dict(format=1, packets=public, private_key=key,
                missing=[dict(attempt=a,question_id=q) for a,q in sorted(expected-seen)],
                suite_sha256=sha256(suite_path), evidence_sha256=sha256(evidence_path),
                truth_sha256=sha256(truth_path), reviewer_config_sha256=sha256(CONFIG), qualified=False)


def assess(root, suite_path, evidence_path, truth_path, index, prepared, replies, revision):
    expected = prepare(root, suite_path, evidence_path, truth_path, index)
    if prepared != expected:
        raise ValueError('Reader review packets changed; regenerate from frozen evidence')
    if not isinstance(revision, str) or not revision.strip():
        raise ValueError('Reader reviewer revision is required')
    seen, rows, failures = set(), [], []
    for reply in replies:
        identity = reply['id']
        if identity not in prepared['private_key'] or identity in seen:
            raise ValueError('Unknown or repeated reader judgment')
        seen.add(identity)
        binding = prepared['private_key'][identity]
        if reply['packet_sha256'] != binding['packet_sha256']:
            raise ValueError('Reader judgment belongs to a different answer packet')
        if set(reply) == {'id','packet_sha256','error'} and isinstance(reply['error'], str) and reply['error']:
            failures.append(dict(packet=identity,error=reply['error']))
            continue
        if (set(reply) != {'id','packet_sha256','score','explanation','evidence','breaches'}
                or type(reply['score']) is not int or not 0 <= reply['score'] <= 4
                or not isinstance(reply['explanation'],str) or not reply['explanation'].strip()
                or reply['evidence'] != [{'reader_sha256':binding['measurement']['sha256']}]
                or not isinstance(reply['breaches'],list) or not set(reply['breaches']) <= BREACHES):
            raise ValueError('Reader judgment needs score, explanation, exact reader digest evidence and safety findings')
        rows.append(dict(binding, score=reply['score'], explanation=reply['explanation'], breaches=reply['breaches']))
    manifest = json.loads(Path(evidence_path).read_text())
    truth = json.loads(Path(truth_path).read_text())
    grades = {}
    for attempt in manifest['attempts']:
        observed = [r for r in rows if r['attempt'] == attempt['id']]
        required = {q['id'] for q in truth['reader_questions'][attempt['scenario']]}
        if {r['question_id'] for r in observed} != required:
            continue
        grades[attempt['id']] = dict(
            criterion=dict(score=min(r['score'] for r in observed),
                explanation='Minimum across independently reviewed final reader answers: '+'; '.join(r['explanation'] for r in observed),
                evidence=[{'reader_sha256':r['measurement']['sha256']} for r in observed]),
            reader_evidence=[r['measurement'] for r in observed],
            breaches=sorted({b for r in observed for b in r['breaches']}),
            grader_revision='independent:'+sha256(CONFIG)+':'+revision,
            packet_binding=packet_hash(prepared))
    return dict(format=1, kind='bound-independent-reader-reviews', qualified=False,
                status='complete' if prepared['packets'] and len(rows)==len(prepared['packets']) and not prepared['missing'] and not failures else 'incomplete',
                index=index, prepared=prepared, replies=replies, revision=revision,
                grades=grades, failures=failures,
                safety_findings=[dict(attempt=r['attempt'],kind=b) for r in rows for b in r['breaches']], missing_packets=sorted(set(prepared['private_key'])-seen))


def merge(root, suite_path, evidence_path, truth_path, artifact, grades):
    """Re-derive the imported artifact before joining it to candidate quality."""
    if artifact != assess(root,suite_path,evidence_path,truth_path,artifact['index'],artifact['prepared'],artifact['replies'],artifact['revision']):
        raise ValueError('Reader review artifact changed')
    merged = copy.deepcopy(grades)
    for grade in merged:
        grade.pop('reader_evidence_verified',None)
        grade.pop('reader_evidence',None)
        grade['breaches'] = sorted(set(grade['breaches']) | {f['kind'] for f in artifact['safety_findings'] if f['attempt']==grade['attempt']})
        bound = artifact['grades'].get(grade['attempt'])
        grade['criteria']['downstream_usefulness'] = dict(score=None,explanation='No complete bound independent reader review',evidence=[{'step':1}])
        if bound is None:
            continue
        if grade['grader_revision'] != bound['grader_revision'] or grade.get('review_kind') != 'independent-review':
            raise ValueError('Candidate and reader must use the same frozen independent reviewer configuration/revision')
        grade['criteria']['downstream_usefulness'] = bound['criterion']
        grade['reader_evidence'] = bound['reader_evidence']
        grade['reader_evidence_verified'] = True
        grade['reader_review_binding'] = bound['packet_binding']
        grade['breaches'] = sorted(set(grade['breaches']) | set(bound['breaches']))
    return merged
