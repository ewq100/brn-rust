#!/usr/bin/env python3
"""Summarize captured C0 results, without rerunning or changing production checks."""
import json, pathlib, re, collections
out=pathlib.Path(__file__).resolve().parent
results=json.loads((out/'results.json').read_text())
suites=[]
ignored=[]
for row in results['results']:
    crate='unknown'; suite=''
    for line in (out/row['log']).read_text().splitlines():
        if 'Running unittests' in line:
            match=re.search('/deps/([a-z_]+)-[0-9a-f]+',line)
            if match: crate=match[1].replace('_','-').removesuffix('-helper') if match[1]=='brn_intake_helper' else match[1].replace('_','-')
            suite=line.strip().split(' (')[0]
        elif 'Running tests/' in line or 'Running unittests examples/' in line:
            suite=line.strip().split(' (')[0]
        elif 'Doc-tests ' in line:
            crate=line.split('Doc-tests ')[-1].replace('_','-');suite='doc-tests'
        if line.startswith('test ') and ' ... ignored' in line:ignored.append(dict(log=row['log'],crate=crate,test=line))
        match=re.search(r'test result: .*?(\d+) passed; (\d+) failed; (\d+) ignored;.*finished in ([0-9.]+)s',line)
        if match: suites.append(dict(log=row['log'],command=row['command'],crate=crate,suite=suite,passed=int(match[1]),failed=int(match[2]),ignored=int(match[3]),seconds=float(match[4])))
(out/'test-suites.json').write_text(json.dumps(suites,indent=2)+'\n')
lines=['# Local baseline results','', 'macOS local execution at unchanged production tree `8f768943`; all commands serialized. `results.json` records exact commands, environment and elapsed wall seconds including compilation. Harness suite durations exclude compilation. The runner continues to independent lanes after failures; inspect each captured exit (the initial runner continued after failure and returned zero; the checked-in runner now returns nonzero for any failed command).', '', '| Lane | Commands | Wall seconds | Failed commands |','|---|---:|---:|---:|']
for lane in ['helper','core','native-ui','native-retrieval']:
    rows=[r for r in results['results'] if r['lane']==lane]
    lines.append(f"| {lane} | {len(rows)} | {sum(r['seconds'] for r in rows):.3f} | {sum(r['exit']!=0 for r in rows)} |")
lines+=['','## Exact commands','','| Log | Exit | Seconds | Command |','|---|---:|---:|---|']
for r in results['results']:lines.append(f"| [{r['log']}]({r['log']}) | {r['exit']} | {r['seconds']} | `{r['command']}` |")
lines+=['','## Counts per crate and feature command','','Counts aggregate executed test binaries under each exact feature command; examples and doc tests count only actual tests. A zero-test example contributes zero, not proof. The workspace command stops at intake qualification, so later binaries/doc tests may not execute; these numbers are observed counts, not an exhaustive listing. No newly ignored tests were introduced.','','| Command log / crate | Passed | Failed | Ignored |','|---|---:|---:|---:|']
groups=collections.defaultdict(lambda:[0,0,0])
for r in suites:
    for i,k in enumerate(['passed','failed','ignored']):groups[(r['log'],r['crate'])][i]+=r[k]
for (log,crate),counts in sorted(groups.items()):lines.append(f"| {log} / {crate} | {' | '.join(map(str,counts))} |")
lines+=['','## Slow-suite evidence and limits','','The harness reports duration per test binary; this is **not** an individual-test timing ranking. Slow individual tests are sampled separately from the slow binaries, with process wall time and commands in `slow-tests.json`; compilation and scheduling contaminate small differences. No global fastest/slowest claim, flaky rate, or benchmark improvement is inferred.','','| Log / crate / suite | Harness seconds |','|---|---:|']
for r in sorted(suites,key=lambda x:x['seconds'],reverse=True)[:12]:lines.append(f"| {r['log']} / {r['crate']} / {r['suite']} | {r['seconds']} |")
lines+=['','## Existing ignored witnesses','']
for r in ignored:lines.append(f"- {r['log']} / {r['crate']}: `{r['test']}`")
lines+=['','## Failures and qualification boundaries','','- Default workspace exit 101: `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` at `qualification.rs:470`, `Pdf preparation failed: Unavailable`. Native `pdftoppm` missing; explicit native `pdftotext` and `pdfimages` are present. No wrapper/fake renderer and no fixture suppression. This is a local environment gap, not a production fix.', '- Exact-baseline hosted macOS native-widget failure remains a separate pre-existing failure even though the same local command passes (306 passed, 1 existing ignored, 3 filtered). One local pass cannot establish cause or a flake rate.', '- Linux core is hosted evidence only. No local Linux/Windows/native-observation/live-provider/model qualification claimed.', '- Native retrieval `model_download` cases refuse occupied/cancelled installs without fetching. Model directories explicitly empty; no model installation or evaluation performed.', '- Existing `block 0.1.6` future-compatibility warning is retained in native logs; strict lint exits passed.']
(out/'baseline-results.md').write_text('\n'.join(lines)+'\n')
