#!/usr/bin/env python3
"""BRN-AgentBench offline preparation and frozen-output reports.

Native execution remains scripts/evaluate_wiki_v2.py. No model calls here.
"""
import argparse
import json
import subprocess
from pathlib import Path
import proof
import freeze
import wiki_challenge
from limits import dry_plan
from evidence import native_attempt, collect, verify_manifest, verify_grade_references, verified_proof_packet, assess_checkpoint_claims
import calibration
import judgment
import external
import diagnostics
import host_checks
import reader
import reader_grading
import semantic
import reporting
import helper_execution
from score import assess, sha256, validate_suite, write_report


def load(path):
    return json.loads(Path(path).read_text())


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    prepare = sub.add_parser('prepare', help='write deterministic synthetic fixture, suite and separate gold')
    prepare.add_argument('--suite-kind', choices=['baseline', 'wiki-challenge', 'visible-screening'], default='baseline')
    prepare.add_argument('--cases', help='predeclare a focused suite subset; native fixture and truth stay intact')
    prepare.add_argument('--repetitions', type=int, choices=[1,2,3])
    prepare.add_argument('--output', type=Path, required=True)
    frozen = sub.add_parser('freeze', help='hash explicit evaluator/build/data/prompt/model/retrieval inputs')
    frozen.add_argument('--spec', type=Path, required=True)
    frozen.add_argument('--output', type=Path, required=True)
    verify_frozen = sub.add_parser('verify-freeze', help='refuse changed experiment inputs before execution')
    verify_frozen.add_argument('--spec', type=Path, required=True)
    verify_frozen.add_argument('--receipt', type=Path, required=True)
    collector = sub.add_parser('collect', help='freeze a native evaluator result index without model calls')
    collector.add_argument('--root', type=Path, required=True)
    collector.add_argument('--suite', type=Path, required=True)
    collector.add_argument('--output-name', default='agentbench-evidence.json')
    plan = sub.add_parser('plan', help='check conservative call reservations without executing anything')
    plan.add_argument('--suite', type=Path, required=True)
    plan.add_argument('--max-calls', type=int, required=True)
    plan.add_argument('--calls-per-event', type=int, required=True)
    plan.add_argument('--configurations', type=int, default=1)
    plan.add_argument('--max-spend', type=float)
    claims = sub.add_parser('check-claims', help='verify reviewed claim annotations against frozen native state')
    claims.add_argument('--checkpoint', type=Path, required=True)
    claims.add_argument('--scenario', required=True)
    claims.add_argument('--step', type=int, required=True)
    claims.add_argument('--truth', type=Path)
    claims.add_argument('--adjudication', type=Path, required=True)
    claims.add_argument('--output', type=Path, required=True)
    packet = sub.add_parser('packet', help='export a frozen anonymous grader/reader packet')
    packet.add_argument('--checkpoint', type=Path, required=True)
    packet.add_argument('--scenario', help='proof scenario for checkpoint-scoped semantic gold')
    packet.add_argument('--step', type=int, help='proof checkpoint number')
    packet.add_argument('--wiki-records', type=Path, help='explicit ID/version allowlist for wiki-only reading')
    packet.add_argument('--truth', type=Path, help='experiment-pinned versioned truth file')
    packet.add_argument('--output', type=Path, required=True)
    mini = sub.add_parser('prepare-judgment', help='prepare newly authored mini fixtures, not the original pack')
    mini.add_argument('--output', type=Path, required=True)
    helper_score = sub.add_parser('helper-score', help='aggregate reviewed helper criteria, no model calls')
    helper_score.add_argument('--rows', type=Path, required=True)
    helper_score.add_argument('--mode', choices=['natural', 'controlled'], required=True)
    helper_score.add_argument('--output', type=Path, required=True)
    pairs = sub.add_parser('calibration-prepare', help='create blind swapped packets and separate private key')
    pairs.add_argument('--pairs', type=Path, required=True)
    pairs.add_argument('--rubric', type=Path, help='public defect taxonomy and overlap rules')
    pairs.add_argument('--output', type=Path, required=True)
    screening = sub.add_parser('calibration-score', help='screen imported judgments without calling a model')
    screening.add_argument('--key', type=Path, required=True)
    screening.add_argument('--judgments', type=Path, required=True)
    screening.add_argument('--output', type=Path, required=True)
    synthetic = sub.add_parser('adapt-synthetic', help='explicitly synthetic upstream-schema adapter witnesses; no corpus admission')
    synthetic.add_argument('--adapter', choices=['memoryagentbench', 'clawarena'], required=True)
    synthetic.add_argument('--input', type=Path, required=True)
    synthetic.add_argument('--manifest', type=Path, required=True)
    synthetic.add_argument('--output', type=Path, required=True)
    adapter = sub.add_parser('adapt-longmemeval', help='transform admitted local histories, no downloads')
    adapter.add_argument('--input', type=Path, required=True)
    adapter.add_argument('--admission', type=Path, required=True)
    adapter.add_argument('--output', type=Path, required=True)
    adapter.add_argument('--native', action='store_true', help='emit native BRN intake/ask fixture')
    stage = sub.add_parser('stage-history', help='native offline intake of admitted dated histories only')
    stage.add_argument('--input', type=Path, required=True)
    stage.add_argument('--admission', type=Path, required=True)
    stage.add_argument('--case', required=True)
    stage.add_argument('--binary', type=Path, required=True)
    stage.add_argument('--output', type=Path, required=True)
    diagnostic = sub.add_parser('prepare-diagnostics', help='prepare separate protection/instruction boundary cases')
    diagnostic.add_argument('--output', type=Path, required=True)
    host = sub.add_parser('check-host', help='run fixed existing offline native contracts, never model scores')
    host.add_argument('--checks', nargs='+', choices=sorted(host_checks.CHECKS))
    host.add_argument('--output', type=Path, required=True)
    read = sub.add_parser('reader-controlled', help='run a fixed provider fixture on a restored checkpoint clone')
    read.add_argument('--checkpoint', type=Path, required=True)
    read.add_argument('--fixture', type=Path, required=True)
    read.add_argument('--binary', type=Path, required=True)
    read.add_argument('--reader-binary', type=Path, required=True)
    read.add_argument('--output', type=Path, required=True)
    measured = sub.add_parser('reader-measured', help='fixed native retrieval/model reader; requires new live allocation')
    measured.add_argument('--checkpoint', type=Path, required=True)
    measured.add_argument('--question', required=True)
    measured.add_argument('--wiki-records', type=Path, required=True)
    measured.add_argument('--mode', choices=['wiki-only', 'normal-product'], required=True)
    measured.add_argument('--binary', type=Path, required=True)
    measured.add_argument('--model-dir', type=Path)
    measured.add_argument('--auth-file', type=Path, required=True)
    measured.add_argument('--output', type=Path, required=True)
    measured.add_argument('--live', action='store_true')
    for name in ('reader-review-prepare', 'reader-review-import'):
        command = sub.add_parser(name, help='bound anonymous independent reviews of measured reader answers; no calls')
        command.add_argument('--root', type=Path, required=True)
        command.add_argument('--suite', type=Path, required=True)
        command.add_argument('--evidence', type=Path, required=True)
        command.add_argument('--truth', type=Path, required=True)
        command.add_argument('--index', type=Path, required=True)
        command.add_argument('--output', type=Path, required=True)
        if name == 'reader-review-import':
            command.add_argument('--prepared', type=Path, required=True)
            command.add_argument('--replies', type=Path, required=True)
            command.add_argument('--revision', required=True)
    helper_run = sub.add_parser('helpers-controlled', help='retained native parent/helper provider-fixture journey')
    helper_run.add_argument('--binary', type=Path, required=True)
    helper_run.add_argument('--cases', nargs='+')
    helper_run.add_argument('--witness', choices=['good', 'bad', 'alternative'], default='good')
    helper_run.add_argument('--output', type=Path, required=True)
    helper_assess = sub.add_parser('helpers-assess', help='reassess frozen controlled helper outputs')
    helper_assess.add_argument('--root', type=Path, required=True)
    helper_assess.add_argument('--output', type=Path, required=True)
    recovery = sub.add_parser('recovery-controlled', help='retain a scripted native crash/recovery/review/Undo journey')
    recovery.add_argument('--recovery-binary', type=Path, required=True)
    recovery.add_argument('--output', type=Path, required=True)
    compare = sub.add_parser('compare', help='paired configuration differences clustered by independent world')
    compare.add_argument('--suite', type=Path, required=True)
    compare.add_argument('--reports', type=Path, nargs='+', required=True)
    compare.add_argument('--output', type=Path, required=True)
    diffs = sub.add_parser('export-diffs', help='verified frozen note diffs and evidence index')
    diffs.add_argument('--root', type=Path, required=True)
    diffs.add_argument('--evidence', type=Path, required=True)
    diffs.add_argument('--output', type=Path, required=True)
    for name in ('semantic-prepare', 'semantic-controlled', 'semantic-execute', 'semantic-reviewed'):
        command = sub.add_parser(name, help='blind frozen grading protocol; no live calls')
        command.add_argument('--truth', type=Path)
        if name == 'semantic-execute':
            command.add_argument('--fixture', type=Path, required=True)
        command.add_argument('--root', type=Path, required=True)
        command.add_argument('--suite', type=Path, required=True)
        command.add_argument('--evidence', type=Path, required=True)
        command.add_argument('--output', type=Path, required=True)
        if name == 'semantic-reviewed':
            command.add_argument('--reviewer-config', type=Path, required=True)
        if name in ('semantic-controlled', 'semantic-reviewed'):
            command.add_argument('--prepared', type=Path, required=True)
            command.add_argument('--replies', type=Path, required=True)
            command.add_argument('--revision', required=True)
    report = sub.add_parser('report', help='grade frozen evidence; never rerun a candidate')
    report.add_argument('--truth', type=Path)
    report.add_argument('--freeze-spec', type=Path)
    report.add_argument('--freeze-receipt', type=Path)
    report.add_argument('--suite', type=Path, required=True)
    report.add_argument('--evidence', type=Path, required=True)
    report.add_argument('--reader-grades', type=Path)
    report.add_argument('--grades', type=Path, required=True)
    report.add_argument('--configuration', required=True)
    report.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == 'freeze':
            with args.output.open('x') as stream:
                json.dump(freeze.freeze(load(args.spec)), stream, indent=2)
        elif args.command == 'verify-freeze':
            print(json.dumps(freeze.verify(load(args.spec), load(args.receipt))))
        elif args.command == 'check-claims':
            result = assess_checkpoint_claims(args.checkpoint, args.scenario, args.step, load(args.adjudication), truth_path=args.truth)
            with args.output.open('x') as stream:
                json.dump(result, stream, indent=2)
        elif args.command == 'compare':
            result = reporting.compare(load(args.suite), [load(p) for p in args.reports])
            with args.output.open('x') as stream:
                json.dump(result, stream, indent=2)
        elif args.command == 'export-diffs':
            reporting.export_diffs(args.root, load(args.evidence), args.output)
        elif args.command == 'adapt-synthetic':
            adapt = external.memoryagentbench_native if args.adapter == 'memoryagentbench' else external.clawarena_native
            fixture, gold, lock = adapt(load(args.input), load(args.manifest))
            args.output.mkdir(parents=True, exist_ok=False)
            for name, value in [('fixture', fixture), ('gold', gold), ('dataset-lock', lock)]:
                (args.output / (name + '.json')).write_text(json.dumps(value, indent=2))
        elif args.command == 'prepare-judgment':
            judgment.prepare(args.output)
        elif args.command == 'helper-score':
            result = judgment.assess(load(args.rows), args.mode)
            with args.output.open('x') as stream:
                json.dump(result, stream, indent=2)
        elif args.command == 'prepare':
            if args.suite_kind == 'baseline':
                proof.write(args.output)
            else:
                wiki_challenge.write(args.output, screening=args.suite_kind == 'visible-screening')
            if args.cases or args.repetitions:
                suite = load(args.output/'suite.json')
                if args.cases:
                    requested = args.cases.split(',')
                    if len(set(requested)) != len(requested) or not set(requested) <= {c['id'] for c in suite['scenarios']}:
                        raise ValueError('Subset needs unique known cases; partial preparation retained')
                    suite['scenarios'] = [c for c in suite['scenarios'] if c['id'] in requested]
                    suite['revision'] += ':screen:' + ','.join(c['id'] for c in suite['scenarios'])
                if args.repetitions:
                    for case in suite['scenarios']:
                        case['repetitions'] = args.repetitions
                (args.output/'suite.json').write_text(json.dumps(suite, ensure_ascii=False, indent=2)+'\n')
        elif args.command == 'prepare-diagnostics':
            diagnostics.write(args.output)
        elif args.command == 'check-host':
            result = host_checks.run(args.output, args.checks)
            return 0 if result['status'] == 'passed' else 2
        elif args.command == 'stage-history':
            external.stage_history(load(args.input), load(args.admission), args.case, args.binary, args.output)
        elif args.command == 'helpers-controlled':
            helper_execution.run(args.binary, args.output, args.cases, args.witness)
        elif args.command == 'helpers-assess':
            with args.output.open('x') as stream:
                json.dump(helper_execution.assess(args.root), stream, indent=2)
        elif args.command.startswith('reader-review-'):
            index = load(args.index)
            if args.command == 'reader-review-prepare':
                result = reader_grading.prepare(args.root,args.suite,args.evidence,args.truth,index)
            else:
                result = reader_grading.assess(args.root,args.suite,args.evidence,args.truth,index,
                                               load(args.prepared),load(args.replies),args.revision)
            args.output.mkdir(parents=True,exist_ok=False)
            (args.output/'private-result.json').write_text(json.dumps(result,indent=2)+'\n')
            public = result['packets'] if args.command == 'reader-review-prepare' else result['grades']
            (args.output/('packets.json' if args.command == 'reader-review-prepare' else 'reader-grades.json')).write_text(json.dumps(public,indent=2)+'\n')
        elif args.command == 'reader-measured':
            reader.measured(args.checkpoint, args.question, load(args.wiki_records), args.mode, args.binary, args.output,
                            auth_file=args.auth_file, model_dir=args.model_dir, live=args.live)
        elif args.command == 'reader-controlled':
            reader.run(args.checkpoint, load(args.fixture), args.binary, args.reader_binary, args.output)
        elif args.command == 'recovery-controlled':
            if args.output.exists() or not args.output.is_absolute():
                raise ValueError('Recovery output must be a fresh absolute directory')
            process = subprocess.run([str(args.recovery_binary.resolve()), str(args.output)], capture_output=True, text=True, timeout=60)
            if process.returncode:
                raise ValueError('Controlled recovery failed; retained evidence: ' + str(args.output))
            result = load(args.output / 'recovery-result.json')
            if result.get('status') != 'complete' or result.get('provider') != 'controlled-fixture':
                raise ValueError('Recovery binary did not return completed controlled evidence')
        elif args.command == 'semantic-execute':
            result = semantic.execute_controlled(args.root, args.suite, args.evidence, load(args.fixture), args.output, truth_path=args.truth)
            return 0 if result['status'] == 'complete' else 2
        elif args.command.startswith('semantic-'):
            if args.command == 'semantic-prepare':
                result = semantic.prepare(args.root, args.suite, args.evidence, truth_path=args.truth)
            elif args.command == 'semantic-reviewed':
                result = semantic.import_reviewed(args.root, args.suite, args.evidence, load(args.prepared),
                                                  load(args.replies), args.revision, args.reviewer_config, truth_path=args.truth)
            else:
                result = semantic.import_controlled(args.root, args.suite, args.evidence,
                                                     load(args.prepared), load(args.replies), args.revision, truth_path=args.truth)
            args.output.mkdir(parents=True, exist_ok=False)
            (args.output / 'private-result.json').write_text(json.dumps(result, indent=2))
            public = result['packets'] if args.command == 'semantic-prepare' else result['grades']
            (args.output / ('packets.json' if args.command == 'semantic-prepare' else 'grades.json')).write_text(json.dumps(public, indent=2))
        elif args.command == 'packet':
            if (args.scenario is None) != (args.step is None):
                raise ValueError('Scenario and step must be supplied together')
            if args.step is not None and args.checkpoint.name != str(args.step):
                raise ValueError('Checkpoint directory does not match the selected step')
            if args.wiki_records and args.scenario:
                raise ValueError('Reader packets must not contain semantic gold')
            packet = verified_proof_packet(args.checkpoint, scenario=args.scenario, step=args.step,
                                           wiki_records=load(args.wiki_records) if args.wiki_records else None, truth_path=args.truth)
            with args.output.open('x') as stream:
                json.dump(packet, stream, indent=2)
        elif args.command == 'calibration-prepare':
            packets, key = calibration.packets(load(args.pairs), load(args.rubric) if args.rubric else None)
            args.output.mkdir(parents=True, exist_ok=False)
            (args.output / 'packets.json').write_text(json.dumps(packets, indent=2))
            (args.output / 'private-key.json').write_text(json.dumps(key, indent=2))
        elif args.command == 'calibration-score':
            result = calibration.assess(load(args.key), load(args.judgments))
            with args.output.open('x') as stream:
                json.dump(result, stream, indent=2)
        elif args.command == 'adapt-longmemeval':
            adapt = external.native_fixture if args.native else external.transform
            public, gold, lock = adapt(load(args.input), load(args.admission))
            args.output.mkdir(parents=True, exist_ok=False)
            for name, data in [('fixture' if args.native else 'candidate', public), ('gold', gold), ('dataset-lock', lock)]:
                (args.output / (name + '.json')).write_text(json.dumps(data, indent=2))
        elif args.command == 'plan':
            result = dry_plan(load(args.suite), args.max_calls, args.calls_per_event, args.configurations, args.max_spend)
            print(json.dumps(result, indent=2))
            return 0 if result['admitted'] else 2
        elif args.command == 'collect':
            if Path(args.output_name).name != args.output_name:
                raise ValueError('Output name must be a basename')
            manifest = collect(args.root, args.suite)
            with (args.root / args.output_name).open('x') as stream:
                json.dump(manifest, stream, indent=2)
        else:
            evidence = load(args.evidence)
            suite = validate_suite(load(args.suite))
            if evidence['suite_sha256'] != sha256(args.suite):
                raise ValueError('Evidence belongs to a different suite')
            if evidence['fixture_sha256'] != suite['fixture_sha256']:
                raise ValueError('Evidence belongs to a different native fixture')
            root = args.evidence.resolve().parent
            verify_manifest(evidence, root)
            grades = load(args.grades)
            verify_grade_references(evidence, grades, root)
            attempts = [native_attempt(root / a['result'], root) for a in evidence['attempts']]
            reader_artifact = None
            if suite.get('score_family') == wiki_challenge.REVISION:
                # Candidate-only grades cannot establish measured reader answer quality.
                for grade in grades:
                    grade.pop('reader_evidence_verified',None)
                    grade.pop('reader_evidence',None)
                    grade['criteria']['downstream_usefulness'] = dict(score=None,
                        explanation='No complete bound independent reader review',evidence=[{'step':1}])
                if args.reader_grades:
                    if not args.truth:
                        raise ValueError('Bound reader review requires the pinned --truth file')
                    reader_artifact = load(args.reader_grades)
                    grades = reader_grading.merge(root,args.suite,args.evidence,args.truth,reader_artifact,grades)
                    verify_grade_references(evidence,grades,root)
            result = assess(suite, args.configuration, attempts, grades)
            if reader_artifact:
                # Known reader safety findings survive incomplete/ungraded candidate slots.
                ids = {a['id'] for a in attempts if a['configuration'] == args.configuration}
                findings = [f for f in reader_artifact['safety_findings'] if f['attempt'] in ids]
                result['safety_and_authority']['reader_findings'] = findings
                for finding in findings:
                    if finding not in result['safety_breaches']:
                        result['safety_breaches'].append(finding)
                if findings:
                    result.update(status='disqualified', ranked=False)
            result['unindexed_attempts'] = evidence.get('unindexed_attempts', [])
            if result['unindexed_attempts']:
                result['ranked'] = False
                result['status'] = 'incomplete' if result['status'] != 'disqualified' else result['status']
                result['eligibility_gaps'].append('Interrupted attempts lack result metadata; cannot assign to configurations')
            result['operations']['unindexed_attempts_experiment'] = len(result['unindexed_attempts'])
            result['suite_sha256'] = sha256(args.suite)
            result['evidence_sha256'] = sha256(args.evidence)
            result['grades_sha256'] = sha256(args.grades)
            result['reader_grades_sha256'] = sha256(args.reader_grades) if args.reader_grades else None
            result['provenance'] = load(root / 'manifest.json').get('provenance', {'status': 'unavailable-legacy'})
            if bool(args.freeze_spec) != bool(args.freeze_receipt):
                raise ValueError('Supply both freeze spec and receipt')
            if args.freeze_spec:
                receipt = load(args.freeze_receipt)
                freeze.verify(load(args.freeze_spec), receipt)
                result['evaluation_identity'] = receipt['evaluation_identity']
                result['experiment_freeze_sha256'] = sha256(args.freeze_receipt)
            write_report(args.output, result)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(2, f'{error}\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
