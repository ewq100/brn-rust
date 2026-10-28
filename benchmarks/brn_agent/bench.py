#!/usr/bin/env python3
"""BRN-AgentBench offline preparation and frozen-output reports.

Native execution remains scripts/evaluate_wiki_v2.py. No model calls here.
"""
import argparse
import json
from pathlib import Path
import proof
from evidence import collect, verify_manifest, verify_grade_references
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
    report = sub.add_parser('report', help='grade frozen evidence; never rerun a candidate')
    report.add_argument('--suite', type=Path, required=True)
    report.add_argument('--evidence', type=Path, required=True)
    report.add_argument('--grades', type=Path, required=True)
    report.add_argument('--configuration', required=True)
    report.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == 'prepare':
            proof.write(args.output)
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
