#!/usr/bin/env python3
"""Exact-attempt BRN CI summary and conservative saved-log comparison (read only)."""
import argparse
import difflib
import json
from pathlib import Path
import re
import subprocess
import sys

APPLICABLE = (
    'Core and CLI (ubuntu-24.04)',
    'Core and CLI (macos-15)',
    'Native UI build and state (macos-15)',
    'Native retrieval and combined build (macos-15)',
)
DOCS = 'Documentation and tooling'


def gh_json(path):
    return json.loads(subprocess.check_output(['gh', 'api', path]))


def summarize(run, jobs, expected_commit):
    if run['head_sha'] != expected_commit:
        raise ValueError('Requested commit does not match this run attempt')
    lines = [f"BRN CI run {run['id']} attempt {run['run_attempt']} / {run['head_sha']}",
             f"Event: {run['event']}; overall: {run.get('conclusion') or run['status']}",
             '| Job | Scope | Result |', '| --- | --- | --- |']
    seen = set()
    ready = True
    for job in jobs:
        name = job['name']
        seen.add(name)
        applicable = name in APPLICABLE or name == DOCS
        result = job.get('conclusion') or job['status']
        scope = 'applicable PR gate' if applicable else ('informational platform probe' if
            name.endswith(('(windows-2025)', '(ubuntu-24.04)')) else 'unclassified — review')
        lines.append(f'| {name} | {scope} | {result} |')
        if (applicable or scope.startswith('unclassified')) and result != 'success':
            ready = False
    for name in APPLICABLE:
        if name not in seen:
            lines.append(f'| {name} | applicable PR gate | missing |')
            ready = False
    # The docs/tooling job starts with this change; old evidence stays distinguishable.
    if DOCS not in seen:
        lines.append('| Documentation and tooling | introduced after this historical run | not present |')
    lines.append(f'Applicable observed gates: {"passed" if ready else "pending/failed"}. Overall run result above is preserved.')
    return '\n'.join(lines), ready


def normalize_log(text):
    # Preserve compiler/assertion/backtrace lines and their order. Remove only
    # known presentation noise, never grep errors or sort diagnostic facts.
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', text)
    text = re.sub(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ', '', text, flags=re.M)
    text = re.sub(r'\bpid=\d+\b', 'pid=<PID>', text)
    text = re.sub(r'^(\s*\d+:\s+)0x[0-9a-fA-F]{8,16}(\s+-\s+)', r'\1<ADDRESS>\2', text, flags=re.M)
    return text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='action', required=True)
    run = commands.add_parser('run')
    run.add_argument('--repo', default='ewq100/brn-rust')
    run.add_argument('--run', type=int, required=True)
    run.add_argument('--attempt', type=int, required=True)
    run.add_argument('--commit', required=True)
    compare = commands.add_parser('compare-logs')
    compare.add_argument('before', type=Path)
    compare.add_argument('after', type=Path)
    opts = parser.parse_args()
    if opts.action == 'compare-logs':
        before, after = [normalize_log(path.read_text()).splitlines(keepends=True) for path in (opts.before, opts.after)]
        if before == after:
            print('Saved logs match after only documented presentation-noise normalization.')
            return 0
        sys.stdout.writelines(difflib.unified_diff(before, after, fromfile=str(opts.before), tofile=str(opts.after)))
        return 1
    if not re.fullmatch(r'[0-9a-f]{40}', opts.commit) or opts.run < 1 or opts.attempt < 1:
        parser.error('Use a full commit SHA and positive run/attempt IDs')
    base = f'repos/{opts.repo}/actions/runs/{opts.run}/attempts/{opts.attempt}'
    attempt = gh_json(base)
    jobs, page = [], 1
    while True:
        batch = gh_json(f'{base}/jobs?per_page=100&page={page}')['jobs']
        jobs.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    report, _ready = summarize(attempt, jobs, opts.commit)
    print(report)
    # Informational failures remain red/failed in both report and terminal exit.
    return 0 if attempt.get('conclusion') == 'success' else 1


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        print(f'CI summary unavailable: {error}', file=sys.stderr)
        sys.exit(2)
