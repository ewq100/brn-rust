"""Offline paired comparisons and inspectable, hash-bound evidence derivatives.

No model calls. Bootstrap intervals describe observed independent worlds only;
three synthetic worlds do not establish population validity or safety.
"""
import difflib
import hashlib
import html
import json
import math
from pathlib import Path
import random
from score import WEIGHTS, sha256, suite_identity, validate_suite, CHALLENGE_FAMILY


def canonical_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def provenance(files, labels=None):
    """Hash explicitly selected inputs without exporting their bytes or local paths."""
    if not isinstance(files, dict) or not files:
        raise ValueError('Explicit named provenance files are required')
    entries = {}
    for name, path in files.items():
        if not isinstance(name, str) or not name.strip() or not Path(path).is_file():
            raise ValueError('Each provenance input needs a name and an existing file')
        entries[name] = {'sha256': sha256(path), 'bytes': Path(path).stat().st_size}
    if labels is not None and (not isinstance(labels, dict) or any(
            not isinstance(k, str) or not isinstance(v, str) for k, v in labels.items())):
        raise ValueError('Provenance labels must be explicit string pairs')
    result = {'format': 1, 'files': entries, 'labels': labels or {}}
    return dict(result, identity=canonical_hash(result))


def _weighted(suite, values, multiplicity=None):
    if suite.get('score_family') == CHALLENGE_FAMILY:
        members = [(c, (multiplicity or {}).get(c.get('world'), 1)) for c in suite['scenarios']]
        members = [(c, n) for c, n in members if n]
        if not members or any(values.get(c['id']) is None for c, _ in members):
            return None
        return sum(values[c['id']] * c['weight'] * n for c, n in members) / sum(c['weight'] * n for c, n in members)
    categories = {}
    for category, weight in WEIGHTS.items():
        members = [(case, (multiplicity or {}).get(case.get('world'), 1))
                   for case in suite['scenarios'] if case['category'] == category]
        members = [(case, copies) for case, copies in members if copies]
        if not members or any(values.get(case['id']) is None for case, _ in members):
            return None
        denominator = sum(case['weight'] * copies for case, copies in members)
        categories[category] = sum(values[case['id']] * case['weight'] * copies
                                   for case, copies in members) / denominator
    return sum(categories[k] * w / 100 for k, w in WEIGHTS.items())


def _quantile(values, q):
    values = sorted(values)
    position = (len(values) - 1) * q
    lo, hi = math.floor(position), math.ceil(position)
    return values[lo] + (values[hi] - values[lo]) * (position - lo)


def compare(suite, reports, bootstrap_samples=2000, seed=0):
    """Pair complete scenario means, resampling whole worlds, never individual facts.

    Reports must come from assess() for the exact same frozen suite. This binds
    definitions, not the truth of supplied reports: the CLI must verify source
    evidence before producing reports. Serialized report hashes remain traceable.
    """
    validate_suite(suite)
    if type(bootstrap_samples) is not int or not 100 <= bootstrap_samples <= 100000 or type(seed) is not int:
        raise ValueError('Use 100–100000 bootstrap samples and an integer seed')
    if not isinstance(reports, list) or len(reports) < 2:
        raise ValueError('At least two configuration reports are required')
    cases = {c['id']: c for c in suite['scenarios']}
    worlds = sorted({c.get('world') for c in suite['scenarios'] if isinstance(c.get('world'), str) and c['world']})
    world_complete = all(isinstance(c.get('world'), str) and c['world'] for c in suite['scenarios'])
    identities = set()
    for report in reports:
        name = report.get('configuration')
        if not isinstance(name, str) or not name or name in identities:
            raise ValueError('Configuration names must be nonempty and unique')
        identities.add(name)
        if report.get('suite_definition_sha256') != suite_identity(suite) or set(report['scenarios']) != set(cases):
            raise ValueError('Reports do not describe the exact same frozen scenario suite')
        for value in report['scenarios'].values():
            if value is not None and (type(value) not in (int, float) or not math.isfinite(value) or not 0 <= value <= 100):
                raise ValueError('Scenario scores must be finite 0–100 or null')
        computed = _weighted(suite, report['scenarios'])
        actual = report.get('challenge_quality') if suite.get('score_family') == CHALLENGE_FAMILY else report.get('overall_quality')
        if (computed is None) != (actual is None) or (computed is not None and
                (type(actual) not in (int, float) or not math.isfinite(actual) or not math.isclose(computed, actual))):
            raise ValueError('Report overall does not match its frozen scenario scores')
    if len({r.get('scoring_revision', 'legacy-v1') for r in reports}) != 1:
        raise ValueError('Paired comparison requires identical scoring revisions; regrade frozen outputs')
    if suite.get('score_family') == CHALLENGE_FAMILY and (not reports[0].get('evaluation_identity') or
            len({r.get('evaluation_identity') for r in reports}) != 1):
        raise ValueError('Challenge comparison requires identical frozen evaluator identities')
    baseline = reports[0]
    revisions = baseline.get('grade_revisions')
    if (not isinstance(revisions, list) or len(revisions) != 1 or
            not isinstance(revisions[0], str) or not revisions[0].strip() or
            any(r.get('grade_revisions') != revisions for r in reports[1:])):
        raise ValueError('Paired comparison requires one identical frozen grader revision across all reports')
    comparisons = []
    for candidate in reports[1:]:
        values = {identity: candidate['scenarios'][identity] - baseline['scenarios'][identity]
                  if candidate['scenarios'][identity] is not None and baseline['scenarios'][identity] is not None else None
                  for identity in cases}
        missing = sorted(k for k, v in values.items() if v is None)
        interval, draws, rejected = None, [], 0
        status = 'incomplete_pairs' if missing else 'missing_world_identity' if not world_complete else 'insufficient_clusters' if len(worlds) < 3 else 'pilot_interval'
        if status == 'pilot_interval':
            rng = random.Random(seed)
            for _ in range(bootstrap_samples):
                multiplicity = dict.fromkeys(worlds, 0)
                for world in rng.choices(worlds, k=len(worlds)):
                    multiplicity[world] += 1
                value = _weighted(suite, values, multiplicity)
                if value is None:
                    rejected += 1
                else:
                    draws.append(value)
            # Do not silently condition on bootstrap draws retaining every category.
            if rejected:
                status = 'incomplete_cluster_category_coverage'
            else:
                interval = [_quantile(draws, .025), _quantile(draws, .975)]
        comparisons.append({'baseline': baseline['configuration'], 'candidate': candidate['configuration'],
                            'score_family': suite.get('score_family', 'baseline'),
                            'direction': 'candidate minus baseline, percentage points',
                            'scenario_deltas': values, 'overall_delta': _weighted(suite, values),
                            'missing_pairs': missing, 'clusters': len(worlds),
                            'uncertainty': {'status': status, 'method': 'paired world-cluster percentile bootstrap',
                                            'interval_95': interval, 'draws': bootstrap_samples if draws or rejected else 0,
                                            'rejected_draws': rejected, 'seed': seed},
                            'eligible_for_selection': False,
                            'baseline_status': baseline['status'], 'candidate_status': candidate['status'],
                            'safety_breaches': {'baseline': baseline['safety_breaches'], 'candidate': candidate['safety_breaches']}})
    return {'format': 1, 'suite_definition_sha256': suite_identity(suite), 'comparisons': comparisons,
            'report_sha256': {r['configuration']: canonical_hash(r) for r in reports},
            'qualified': False, 'ranked': False,
            'limitations': ['Intervals are pilot diagnostics, not model or grader qualification.',
                            'Independent-world assumptions require review; repetitions and facts are not independent clusters.',
                            'All missing pairs remain visible; no success-only comparison or automatic selection.']}


def export_diffs(root, manifest, output):
    """Export local synthetic note diffs after native evidence verification.

    Each before/after artifact preserves exact note IDs, versions and markdown,
    and identifies its source records hash. Raw databases are never exported.
    """
    from evidence import verify_manifest
    root, output = Path(root).resolve(), Path(output).resolve()
    verify_manifest(manifest, root)
    output.mkdir(parents=True, exist_ok=False)
    entries = []
    for attempt_index, attempt in enumerate(manifest['attempts']):
        result_path = root / attempt['result']
        raw = json.loads(result_path.read_text())
        previous, previous_step, previous_hash = {}, None, None
        captures = sorted(raw.get('checkpoints', []), key=lambda c: c['step'])
        if len({c['step'] for c in captures}) != len(captures):
            raise ValueError('Duplicate checkpoint steps')
        for checkpoint in captures:
            step = checkpoint['step']
            records_path = result_path.parent / 'checkpoints' / str(step) / 'records.json'
            records = json.loads(records_path.read_text())
            notes = {}
            for record in records:
                if 'Note' not in record.get('data', {}):
                    continue
                if record['id'] in notes:
                    raise ValueError('Duplicate note IDs in checkpoint')
                notes[record['id']] = record
            for identity in sorted(set(previous) | set(notes)):
                before, after = previous.get(identity), notes.get(identity)
                if before == after:
                    continue
                name = f'{attempt_index:04}-{step:04}-' + hashlib.sha256(identity.encode()).hexdigest()[:16]
                reference = {'attempt': attempt['id'], 'result_sha256': attempt['result_sha256'],
                             'record': identity, 'from_checkpoint': previous_step, 'to_checkpoint': step,
                             'from_version': before['version'] if before else None,
                             'to_version': after['version'] if after else None,
                             'from_records_sha256': previous_hash,
                             'to_records_sha256': checkpoint['records_sha256']}
                packet = dict(reference, before=before, after=after)
                artifact_path = output / (name + '.json')
                artifact_path.write_text(json.dumps(packet, indent=2, ensure_ascii=False) + '\n')
                before_text = before['data']['Note'].get('markdown', '') if before else ''
                after_text = after['data']['Note'].get('markdown', '') if after else ''
                # Line-oriented view is normalized; JSON artifacts retain exact bytes/newlines.
                diff = '\n'.join(difflib.unified_diff(before_text.splitlines(), after_text.splitlines(),
                                                    fromfile=f'checkpoint-{previous_step}@{reference["from_version"]}',
                                                    tofile=f'checkpoint-{step}@{reference["to_version"]}', lineterm=''))
                if diff:
                    diff += '\n'
                (output / (name + '.diff')).write_text(diff)
                entries.append(dict(reference, artifact=name+'.json', artifact_sha256=sha256(artifact_path),
                                    diff=name+'.diff', diff_sha256=sha256(output/(name+'.diff'))))
            previous, previous_step, previous_hash = notes, step, checkpoint['records_sha256']
    # Reverify input hashes after export: fail closed if inputs changed while read.
    verify_manifest(manifest, root)
    index = {'format': 1, 'kind': 'verified-note-diffs', 'evidence_sha256': canonical_hash(manifest),
             'entries': entries, 'qualified': False, 'scope': 'Local synthetic evidence; not a share-sanitization guarantee.'}
    (output / 'index.json').write_text(json.dumps(index, indent=2) + '\n')
    links = ''.join('<li>' + html.escape(f'{e["attempt"]} / {e["record"]}: {e["from_version"]} → {e["to_version"]}') +
                    f' — <a href="{e["diff"]}">diff</a> · <a href="{e["artifact"]}">exact evidence</a></li>' for e in entries)
    (output / 'index.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>BRN note evidence</title>'
                                     '<h1>Verified note changes</h1><p>Local synthetic evidence. Sources are bound by hashes and record versions.</p><ul>' + links + '</ul></html>')
    return index
