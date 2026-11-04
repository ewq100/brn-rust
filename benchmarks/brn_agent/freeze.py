"""Immutable input receipts for the existing runner. No execution or scheduling."""
import json
from pathlib import Path
from reporting import canonical_hash, provenance


def freeze(spec):
    required = {'evaluation_files', 'candidate_files', 'labels', 'settings', 'isolation'}
    if not isinstance(spec, dict) or set(spec) != required:
        raise ValueError('Freeze requires evaluation/candidate files, labels, settings and isolation')
    if not {'app_source_revision', 'benchmark_revision', 'build_source_binding'} <= set(spec['labels']):
        raise ValueError('Record app source, benchmark revision and explicit source/build binding status')
    if not {'candidate', 'reader', 'grader', 'retrieval'} <= set(spec['settings']):
        raise ValueError('Record candidate, reader, grader and retrieval settings separately')
    if spec['isolation'] not in ('visible-screening', 'separate-evaluation-owner'):
        raise ValueError('Declare actual isolation; a gitignored directory is visible screening')
    all_paths = {**spec['evaluation_files'], **spec['candidate_files']}
    if set(spec['evaluation_files']) & set(spec['candidate_files']):
        raise ValueError('Input names must be unique across evaluator and candidate')
    for name, path in all_paths.items():
        if any(word in (name+' '+str(path)).lower() for word in ('credential', 'secret', 'auth.json', 'token')):
            raise ValueError('Never include credentials or secrets in experiment receipts')
        if not Path(path).is_absolute():
            raise ValueError('Freeze explicit absolute input paths')
    package = Path(__file__).resolve().parent
    repo = package.parents[1]
    automatic = sorted(package.glob('*.py')) + sorted((package/'wiki_challenge').glob('*.json'))
    automatic += [repo/'scripts/evaluate_wiki_v2.py', repo/'scripts/evaluate-wiki-guidance.py']
    files = {**spec['evaluation_files'], **{f'benchmark:{p.relative_to(repo)}':p for p in automatic}}
    evaluation = provenance(files)
    candidate = provenance(spec['candidate_files'], spec['labels'])
    fixed = dict(evaluation=evaluation, reader=spec['settings']['reader'],
                 grader=spec['settings']['grader'], retrieval=spec['settings']['retrieval'],
                 isolation=spec['isolation'])
    receipt = dict(format=1, kind='frozen-experiment-inputs', evaluation_identity=canonical_hash(fixed),
                   evaluation=evaluation, candidate=candidate, settings=spec['settings'], isolation=spec['isolation'],
                   isolation_attested=False, qualification='unverified',
                   limitations=['File hashes do not attest source-to-binary identity or inaccessible evaluation ownership.',
                                'Keep the evaluator, rubric, reviewer and acceptance inputs fixed between comparisons.'])
    return dict(receipt, identity=canonical_hash(receipt))


def verify(spec, receipt):
    if freeze(spec) != receipt:
        raise ValueError('Frozen experiment inputs changed; preserve the old run and create a new experiment')
    return {'status':'verified', 'identity':receipt['identity'], 'qualification':'unverified'}
