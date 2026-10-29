"""Blind frozen-output semantic grading protocol, with controlled offline replies.

This validates evidence and schema, not the semantic competence of any grader.
No model or subprocess is called. The original 12 calibration pairs stay separate.
"""
import json
from pathlib import Path
from evidence import verify_manifest, verify_grade_references, verified_proof_packet, anonymize_local_paths
from reader import packet_hash
from score import assess, sha256, validate_suite


def prepare(root, suite_path, manifest_path):
    root = Path(root).resolve()
    suite = validate_suite(json.loads(Path(suite_path).read_text()))
    manifest = json.loads(Path(manifest_path).read_text())
    if manifest['suite_sha256'] != sha256(suite_path):
        raise ValueError('Semantic suite does not match frozen evidence')
    verify_manifest(manifest, root)
    cases = {c['id']:c for c in suite['scenarios']}
    packets, key, missing = [], {}, []
    for attempt in manifest['attempts']:
        raw_path = root / attempt['result']
        raw = json.loads(raw_path.read_text())
        captured = raw.get('checkpoints', [])
        if not captured:
            missing.append(attempt['id'])
            continue
        number = max(c['step'] for c in captured)
        packet = verified_proof_packet(raw_path.parent/'checkpoints'/str(number),
                                       scenario=attempt['scenario'],step=number,manifest_path=manifest_path)
        packet['criteria'] = cases[attempt['scenario']]['criteria']
        packet['scale'] = {'0':'failed','1':'seriously flawed','2':'material correction required',
                           '3':'minor correction required','4':'meets criterion'}
        packet['events'] = [{k:step[k] for k in ('index','kind','terminal','answer','operations','receipts') if k in step}
                            for step in raw.get('steps', [])[:number]]
        packet['execution_status'] = attempt['status']
        packet = anonymize_local_paths(packet, root)
        identity = packet_hash({'packet':packet,'ordinal':len(packets)})
        packets.append({'id':identity,'packet':packet,'packet_sha256':packet_hash(packet)})
        key[identity] = {'attempt':attempt['id'],'result_sha256':attempt['result_sha256'],
                         'packet_sha256':packet_hash(packet)}
    return {'format':1,'packets':packets,'private_key':key,'missing_attempts':missing,
            'suite_sha256':sha256(suite_path),'evidence_sha256':sha256(manifest_path),
            'qualified':False,'scope':'latest captured state per attempt; earlier checkpoint packets remain separately available'}


def import_controlled(root, suite_path, manifest_path, prepared, replies, revision):
    if not isinstance(revision,str) or not revision.strip():
        raise ValueError('An explicit controlled grader revision is required')
    expected = prepare(root,suite_path,manifest_path)
    if prepared != expected:
        raise ValueError('Prepared grading packets or private bindings changed')
    seen, grades, failures = set(), [], []
    for reply in replies:
        identity = reply['id']
        if identity not in prepared['private_key'] or identity in seen:
            raise ValueError('Unknown or duplicate semantic packet reply')
        seen.add(identity)
        binding = prepared['private_key'][identity]
        if reply['packet_sha256'] != binding['packet_sha256']:
            raise ValueError('Grader response belongs to different evidence')
        if 'error' in reply:
            if set(reply) != {'id','packet_sha256','error'} or not isinstance(reply['error'],str) or not reply['error']:
                raise ValueError('Invalid grader failure record')
            failures.append({'packet':identity,'error':reply['error']})
            continue
        if set(reply) != {'id','packet_sha256','criteria','completed','breaches'}:
            raise ValueError('Semantic response must use the exact schema')
        grades.append({'attempt':binding['attempt'],'result_sha256':binding['result_sha256'],
                       'grader_revision':'controlled:'+revision,'criteria':reply['criteria'],
                       'completed':reply['completed'],'breaches':reply['breaches']})
    suite = json.loads(Path(suite_path).read_text())
    manifest = json.loads(Path(manifest_path).read_text())
    verify_grade_references(manifest,grades,root)
    for configuration in {a['configuration'] for a in manifest['attempts']}:
        ids = {a['id'] for a in manifest['attempts'] if a['configuration']==configuration}
        assess(suite,configuration,manifest['attempts'],[g for g in grades if g['attempt'] in ids])
    missing = sorted(set(prepared['private_key'])-seen)
    return {'format':1,'status':'complete' if prepared['packets'] and not missing and not failures and not prepared['missing_attempts'] else 'incomplete',
            'qualified':False,'provider':'controlled-fixture','grades':grades,'grader_failures':failures,
            'missing_packets':missing,'uncaptured_attempts':prepared['missing_attempts'],
            'candidate_calls':0,'cost':None,'revision':revision}
