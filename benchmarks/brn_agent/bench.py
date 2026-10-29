#!/usr/bin/env python3
"""BRN-AgentBench offline preparation and frozen-output reports.

Native execution remains scripts/evaluate_wiki_v2.py. No model calls here.
"""
import argparse
import json
import subprocess
from pathlib import Path
import proof
from limits import dry_plan
from evidence import collect, verify_manifest, verify_grade_references, verified_proof_packet
import calibration
import judgment
import external
import diagnostics
import host_checks
import reader
import semantic
from score import assess, sha256, validate_suite, write_report


def load(path):
    return json.loads(Path(path).read_text())


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    prepare = sub.add_parser('prepare', help='write deterministic synthetic fixture, suite and separate gold')
    prepare.add_argument('--output', type=Path, required=True)
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
    packet = sub.add_parser('packet', help='export a frozen anonymous grader/reader packet')
    packet.add_argument('--checkpoint', type=Path, required=True)
    packet.add_argument('--scenario', help='proof scenario for checkpoint-scoped semantic gold')
    packet.add_argument('--step', type=int, help='proof checkpoint number')
    packet.add_argument('--wiki-records', type=Path, help='explicit ID/version allowlist for wiki-only reading')
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
    recovery = sub.add_parser('recovery-controlled', help='retain a scripted native crash/recovery/review/Undo journey')
    recovery.add_argument('--recovery-binary', type=Path, required=True)
    recovery.add_argument('--output', type=Path, required=True)
    for name in ('semantic-prepare', 'semantic-controlled'):
        command = sub.add_parser(name, help='blind frozen grading protocol; no live calls')
        command.add_argument('--root', type=Path, required=True)
        command.add_argument('--suite', type=Path, required=True)
        command.add_argument('--evidence', type=Path, required=True)
        command.add_argument('--output', type=Path, required=True)
        if name == 'semantic-controlled':
            command.add_argument('--prepared', type=Path, required=True)
            command.add_argument('--replies', type=Path, required=True)
            command.add_argument('--revision', required=True)
    report = sub.add_parser('report', help='grade frozen evidence; never rerun a candidate')
    report.add_argument('--suite', type=Path, required=True)
    report.add_argument('--evidence', type=Path, required=True)
    report.add_argument('--grades', type=Path, required=True)
    report.add_argument('--configuration', required=True)
    report.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == 'prepare-judgment':
            judgment.prepare(args.output)
        elif args.command == 'helper-score':
            result = judgment.assess(load(args.rows), args.mode)
            with args.output.open('x') as stream:
                json.dump(result, stream, indent=2)
        elif args.command == 'prepare':
            proof.write(args.output)
        elif args.command == 'prepare-diagnostics':
            diagnostics.write(args.output)
        elif args.command == 'check-host':
            result = host_checks.run(args.output, args.checks)
            return 0 if result['status'] == 'passed' else 2
        elif args.command == 'stage-history':
            external.stage_history(load(args.input), load(args.admission), args.case, args.binary, args.output)
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
        elif args.command.startswith('semantic-'):
            if args.command == 'semantic-prepare':
                result = semantic.prepare(args.root, args.suite, args.evidence)
            else:
                result = semantic.import_controlled(args.root, args.suite, args.evidence,
                                                     load(args.prepared), load(args.replies), args.revision)
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
                                           wiki_records=load(args.wiki_records) if args.wiki_records else None)
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
            result = assess(suite, args.configuration, evidence['attempts'], grades)
            result['unindexed_attempts'] = evidence.get('unindexed_attempts', [])
            if result['unindexed_attempts']:
                result['ranked'] = False
                result['status'] = 'incomplete' if result['status'] != 'disqualified' else result['status']
                result['eligibility_gaps'].append('Interrupted attempts lack result metadata; cannot assign to configurations')
            result['operations']['unindexed_attempts_experiment'] = len(result['unindexed_attempts'])
            result['suite_sha256'] = sha256(args.suite)
            result['evidence_sha256'] = sha256(args.evidence)
            result['grades_sha256'] = sha256(args.grades)
            write_report(args.output, result)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(2, f'{error}\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
