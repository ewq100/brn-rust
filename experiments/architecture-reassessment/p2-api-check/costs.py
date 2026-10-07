"""Enabled normal graph; lock-only optional/target packages excluded."""
import json, os, subprocess, tomllib
from pathlib import Path
P=Path(__file__).parent; ROOT=P.parents[2]
env=dict(os.environ,PATH='/opt/homebrew/opt/rustup/bin:'+os.environ['PATH'],CARGO_HOME='/private/tmp/brn-p1-cargo',CARGO_TARGET_DIR='/private/tmp/brn-p2-api-target')
def cargo(*args):return subprocess.check_output(['cargo',*args],env=env,text=True)
def graph(manifest,features=()):
 text=cargo('tree','--manifest-path',str(manifest),'--locked','--offline','--target','aarch64-apple-darwin','-e','normal','--prefix','none','--format','{p}',*features)
 return {(row.split()[0],row.split()[1][1:]) for row in text.splitlines() if len(row.split())>1 and row.split()[1].startswith('v') and '(' not in row.split()[0]}
def locked(path):return {(p['name'],p['version']) for p in tomllib.loads(path.read_text())['package'] if p.get('source','').startswith('registry+')}
base=graph(P.parent/'p1-office-mime'/'Cargo.toml'); leaf=graph(P/'Cargo.toml'); facade=graph(P/'Cargo.toml',('--features','facade')); root=locked(ROOT/'Cargo.lock')
# tree includes the local executable; retain registry entries only.
registry=locked(P/'Cargo.lock');leaf &= registry;facade &=registry;base &= locked(P.parent/'p1-office-mime'/'Cargo.lock')
cache=Path('/private/tmp/brn-p1-cargo/registry/cache/index.crates.io-1949cf8c6b5b557f')
def row(g):return {'enabled_normal_registry_packages':len(g),'exact_pairs':sorted(g),'new_exact_pairs_vs_p1':sorted(g-base),'new_exact_pairs_vs_root_lock':sorted(g-root),'archive_bytes':sum((cache/f'{n}-{v}.crate').stat().st_size for n,v in g),'incremental_archive_bytes_vs_p1':sum((cache/f'{n}-{v}.crate').stat().st_size for n,v in g-base),'incremental_archive_bytes_vs_root_lock':sum((cache/f'{n}-{v}.crate').stat().st_size for n,v in g-root)}
result={'scope':'aarch64-apple-darwin enabled normal registry graph; root comparison is root lock, not root enabled graph','p1':row(base),'structured_leaf':row(leaf),'facade':row(facade),'probe_rust_lines':len((P/'src/main.rs').read_text().splitlines()),'p1_manual_package_title_relationship_text_lines':227,'p1_manual_scope':'main.rs lines17-243 inclusive; DOCX/PPTX shared package+occurrences+recursive text mapping, excludes office() and all MIME/helper/render/supervision','unmeasured':['full BRN link delta','production adapter and guard LOC','production lifetime maintenance','signing/notarization/install/update','production rendering scope','aggregate app RSS']}
result['binary_scope']=json.loads((P/'results/build-measurements.json').read_text())
(P/'results/costs.json').write_text(json.dumps(result,indent=2)+'\n')
(P/'results/tree-features.txt').write_text(cargo('tree','--manifest-path',str(P/'Cargo.toml'),'--locked','--offline','--target','aarch64-apple-darwin','-e','features'))
(P/'results/metadata.json').write_text(cargo('metadata','--manifest-path',str(P/'Cargo.toml'),'--locked','--offline','--format-version','1'))
