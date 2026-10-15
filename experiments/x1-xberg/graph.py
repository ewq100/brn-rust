#!/usr/bin/env python3
"""Record locked PDF-only graph and compare target packages to BRN's root lock."""
import collections, hashlib, json, os, pathlib, subprocess, tomllib
ROOT = pathlib.Path(__file__).resolve().parent
REPO = ROOT.parent.parent
OUT = ROOT / 'evidence'
CARGO = '/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo'
env = os.environ.copy(); env['PATH'] = str(pathlib.Path(CARGO).parent) + ':' + env['PATH']
env['CARGO_TARGET_DIR'] = '/private/tmp/brn-x1-target'
base = [CARGO, 'metadata', '--manifest-path', str(ROOT/'Cargo.toml'), '--locked', '--offline', '--features', 'candidate', '--format-version', '1']
for suffix, extra in [('', []), ('-macos', ['--filter-platform', 'aarch64-apple-darwin'])]:
    result = subprocess.check_output(base + extra, env=env)
    (OUT / f'metadata{suffix}.json').write_bytes(result)
    data = json.loads(result); ids = {n['id'] for n in data['resolve']['nodes']}
    packages = sorted([p for p in data['packages'] if p['id'] in ids], key=lambda p:(p['name'],p['version']))
    (OUT / f'licenses{suffix}.tsv').write_text('package\tversion\tlicense\tlicense_file\tsource\n' + ''.join(
        f"{p['name']}\t{p['version']}\t{p.get('license')}\t{p.get('license_file')}\t{p.get('source')}\n" for p in packages))
    if not suffix: continue
    root = collections.defaultdict(set)
    for p in tomllib.loads((REPO/'Cargo.lock').read_text())['package']: root[p['name']].add(p['version'])
    (OUT/'root-lock-comparison.tsv').write_text('package\texperiment\troot\n'+''.join(
        f"{p['name']}\t{p['version']}\t{','.join(sorted(root[p['name']])) or '-'}\n" for p in packages))
    scripts=[]
    for p in packages:
        for t in p['targets']:
            if 'custom-build' in t['kind']:
                path=pathlib.Path(t['src_path'])
                scripts.append(dict(package=p['name'],version=p['version'],path=str(path),sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    (OUT/'build-script-inventory.json').write_text(json.dumps(scripts,indent=2)+'\n')
    forbidden={'ort','ort-sys','pdfium-render','xberg-tesseract','tesseract-sys','hf-hub','reqwest','ureq','kreuzberg'}
    assert not forbidden.intersection(p['name'] for p in packages)
    (OUT/'graph-summary.json').write_text(json.dumps(dict(packages=len(packages),xberg_features=next(n['features'] for n in data['resolve']['nodes'] if n['id'].startswith('registry+') and n['id'].endswith('xberg@1.3.10')),licenses=dict(collections.Counter(p['license'] for p in packages)),forbidden_present=sorted(forbidden.intersection(p['name'] for p in packages))),indent=2)+'\n')
for label,flags in [('duplicates',['-d']),('features',['-e','features'])]:
    output=subprocess.check_output([CARGO,'tree','--manifest-path',str(ROOT/'Cargo.toml'),'--locked','--offline','--features','candidate']+flags,env=env)
    (OUT/f'{label}.txt').write_bytes(output)
