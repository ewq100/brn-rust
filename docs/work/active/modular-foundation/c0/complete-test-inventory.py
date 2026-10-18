#!/usr/bin/env python3
"""Combine executed local feature lanes with complete exact-baseline hosted core evidence."""
import collections, json, pathlib, re
out = pathlib.Path(__file__).resolve().parent
ansi = re.compile(r'(?:\x1b|\^\[)\[[0-9;]*m')
log = out / 'hosted-core-macos-baseline.log'
steps = {
    'Qualify maintained intake helper on macOS': ('helper', 'cargo test -p brn-intake --features helper --all-targets --locked --offline'),
    'Test the default workspace': ('default', 'cargo test --workspace --locked --offline'),
    'Test provider capability fixtures offline': ('capability-spike', 'cargo test -p brn-ai --features capability-spike --all-targets --locked --offline'),
}
rows = []
headers = collections.defaultdict(list)
results = collections.defaultdict(list)
crate_by_step = {}
for raw in log.read_text().splitlines():
    _, step, body = raw.split('\t', 2)
    if step not in steps:
        continue
    text = ansi.sub('', body.split('Z ', 1)[-1]).strip()
    if text.startswith('Running '):
        if 'unittests' in text and 'examples/' not in text:
            match = re.search(r'/deps/([a-z_]+)-[0-9a-f]+', text)
            assert match, text
            crate_by_step[step] = match[1].replace('_', '-').removesuffix('-helper')
        headers[step].append((crate_by_step[step], text.split(' (')[0]))
    elif text.startswith('Doc-tests '):
        headers[step].append((text.split('Doc-tests ')[1].replace('_', '-'), 'doc-tests'))
    match = re.search(r'test result: .*?(\d+) passed; (\d+) failed; (\d+) ignored;.*finished in ([0-9.]+)s', text)
    if match:
        results[step].append(dict(passed=int(match[1]), failed=int(match[2]), ignored=int(match[3]), seconds=float(match[4])))
for step, (feature, command) in steps.items():
    # stderr's next "Running" header can precede stdout's previous summary.
    # Pair complete ordered header and result lists, not the most recent header.
    assert len(headers[step]) == len(results[step]), (step, len(headers[step]), len(results[step]))
    for (crate, suite), counts in zip(headers[step], results[step]):
        rows.append(dict(source='hosted', log=log.name, job=116997504186, feature=feature, command=command, crate=crate, suite=suite, **counts))
docs = [row for row in rows if row['suite'] == 'doc-tests']
assert len(docs) == 6 and all(row['passed'] == row['failed'] == row['ignored'] == 0 for row in docs)
local = json.loads((out / 'test-suites.json').read_text())
for row in local:
    if row['log'] not in ['02-helper.log', '06-core.log', '07-core.log']:
        rows.append(dict(source='local', **row))
(out / 'complete-test-inventory.json').write_text(json.dumps(dict(baseline='8f768943964a77711b26d5235df146f8efe1dc6c', hosted_run=38908335265, hosted_job=116997504186, suites=rows), indent=2) + '\n')
groups = collections.defaultdict(lambda: [0, 0, 0, 0])
for row in rows:
    key = (row['source'], row['command'], row['crate'])
    for i, keyname in enumerate(['passed', 'failed', 'ignored']):
        groups[key][i] += row[keyname]
    groups[key][3] += row['suite'] == 'doc-tests'
lines = ['# Complete executed test inventory', '',
    'Exact-baseline macOS [job 116997504186](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504186) supplies the complete default/helper/capability run. [Retained log](hosted-core-macos-baseline.log) contains only workflow test/build/lint/retirement/help/fixture steps, preserving each line’s job, step and UTC timestamp. Setup/post-job/account configuration is excluded. Source head SHA was verified by the run metadata in `hosted-core-metadata.json`. Local feature lanes remain local, with no claim of local Ubuntu execution.', '',
    'The original local workspace run stopped at the failed PDF qualification: 1,012 passed / 1 failed / 6 ignored. Those counts remain accurate partial evidence. The complete hosted default run has 1,013 passed / 0 failed / 6 ignored. All six library doc-test targets actually ran with zero tests; CLI and desktop are binary-only (no doc-test target). No zero-test target is counted as semantic proof.', '',
    '| Source / exact command / crate | Passed | Failed | Ignored | Doc targets executed |',
    '|---|---:|---:|---:|---:|']
for (source, command, crate), counts in sorted(groups.items()):
    lines.append(f'| {source} / `{command}` / {crate} | ' + ' | '.join(map(str, counts)) + ' |')
lines += ['', 'Every executed binary and doc target, feature command and result is retained in [machine-readable inventory](complete-test-inventory.json). stderr headers and stdout summaries can interleave: the parser pairs the complete ordered header/result lists per workflow step, asserts equal lengths, and verifies all six doc results are zero. Native-only local commands use the prior recorded logs without redundant broad builds. Hosted UI widget failure remains a separate required-lane failure; this inventory does not claim four green lanes.', '']
(out / 'complete-test-inventory.md').write_text('\n'.join(lines))
print('Executed suites/targets:', len(rows), '; hosted default:', {k: sum(r[k] for r in rows if r.get('feature') == 'default') for k in ['passed','failed','ignored']})
