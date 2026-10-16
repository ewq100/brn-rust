#!/usr/bin/env python3
"""Sample individual timing in the three slowest baseline test binaries; no build."""
import pathlib,re,subprocess,time,json,os
out=pathlib.Path(__file__).resolve().parent
os.environ.update(TMPDIR='/private/tmp/brn-c0-20261016-fixtures',BRN_NATIVE_MODEL_DIR='',BRN_EMBEDDING_MODEL_DIR='',BRN_INTAKE_HELPER='/private/tmp/brn-c0-20261016-target/debug/brn-intake-helper',BRN_PDFTOTEXT='/Users/evokessler/.local/Cellar/brn-poppler-tools/26.10.0/bin/pdftotext',BRN_PDFIMAGES='/Users/evokessler/.local/Cellar/brn-poppler-tools/26.10.0/bin/pdfimages')
log=(out/'06-core.log').read_text()
rows=[]
for suite in ['agent_bounds','edit_tokens','agent_journeys']:
    binary=re.search(r'Running tests/'+suite+r'\.rs \(([^)]+)\)',log)[1]
    listed=subprocess.check_output([binary,'--list'],text=True)
    names=[line.removesuffix(': test') for line in listed.splitlines() if line.endswith(': test')]
    for name in names:
        command=[binary,'--exact',name,'--nocapture']
        start=time.monotonic();p=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        rows.append(dict(suite=suite,test=name,command=command,exit=p.returncode,seconds=round(time.monotonic()-start,4),output=p.stdout))
        (out/'slow-tests.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps(sorted([{k:v for k,v in r.items() if k!='output'} for r in rows],key=lambda x:x['seconds'],reverse=True)[:8],indent=2))
