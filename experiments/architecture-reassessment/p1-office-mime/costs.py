from pathlib import Path
import json,tomllib,hashlib,re,subprocess,os,time,resource,shutil,gzip,tarfile
R=Path(__file__).resolve().parent;E=R/'evidence';repo=R.parents[2];cargo='/opt/homebrew/opt/rustup/bin/cargo';rustc='/opt/homebrew/opt/rustup/bin/rustc';tmp=Path('/private/tmp/brn-p1-cost');tmp.mkdir(exist_ok=True)
cmd=[cargo,'build','--manifest-path',str(R/'Cargo.toml'),'--offline','--locked','--release'];env={**os.environ,'PATH':'/opt/homebrew/opt/rustup/bin:'+os.environ['PATH'],'CARGO_HOME':'/private/tmp/brn-p1-cargo','CARGO_TARGET_DIR':'/private/tmp/brn-p1-clean-target'}
shutil.rmtree('/private/tmp/brn-p1-clean-target',ignore_errors=True)
measure=[]
for name in ['clean target, source/dependency cache present','warm no-change build']:
 start=time.monotonic()
 with open(E/('build-'+('clean' if not measure else 'warm')+'.log'),'w') as log:
  p=subprocess.Popen(cmd,env=env,stdout=log,stderr=subprocess.STDOUT);_,status,ru=os.wait4(p.pid,0);p.returncode=os.waitstatus_to_exitcode(status)
 assert p.returncode==0
 measure.append({'scope':name,'wall_seconds':round(time.monotonic()-start,3),'child_peak_rss_bytes':ru.ru_maxrss,'exit':p.returncode})
binary=Path(env['CARGO_TARGET_DIR'])/'release'/'brn-p1-office-mime'; stripped=tmp/'brn-p1-office-mime';shutil.copyfile(binary,stripped);subprocess.run(['/usr/bin/strip','-x',str(stripped)],check=True)
empty=tmp/'empty.rs';empty.write_text('fn main() {}\n');subprocess.run([rustc,'--edition=2024','-O',str(empty),'-o',str(tmp/'empty-helper')],check=True);subprocess.run(['/usr/bin/strip','-x',str(tmp/'empty-helper')],check=True)
with tarfile.open(tmp/'helper.tar.gz','w:gz') as t:t.add(stripped,arcname='brn-p1-office-mime')
lock=tomllib.loads((R/'Cargo.lock').read_text());root=tomllib.loads((repo/'Cargo.lock').read_text());rootpairs={(p['name'],p['version']) for p in root['package']};rootnames={p['name'] for p in root['package']}
selected={tuple(x) for x in re.findall(r'^([A-Za-z0-9_-]+) v([^\s]+)',(E/'tree-packages.txt').read_text(),re.M)};selected.discard(('brn-p1-office-mime','0.0.0'))
extra=sorted(selected-rootpairs);meta=json.loads((E/'metadata.json').read_text());packages={(p['name'],p['version']):p for p in meta['packages']}; cache=Path('/private/tmp/brn-p1-cargo/registry/cache/index.crates.io-1949cf8c6b5b557f')
versions={p['name']:{'version':p['version'],'checksum':p.get('checksum')} for p in lock['package'] if p['name'].startswith('betteroffice-') or p['name']=='mail-parser'}
loc={p.name:len(p.read_text().splitlines()) for p in [R/'src/main.rs',R/'runner.py',R/'render.py',R/'oracle.py',R/'fixtures.py']}
report={'host':'Apple Silicon aarch64-apple-darwin','toolchain':subprocess.check_output([rustc,'--version'],text=True).strip(),'builds':measure,'locked_registry_packages':len(lock['package'])-1,'enabled_normal_registry_packages_arm64':len(selected),'incremental_exact_name_version_pairs_vs_root_lock':extra,'new_package_names_vs_root_lock':[n for n,v in extra if n not in rootnames],'enabled_source_archives_bytes':sum((cache/f'{n}-{v}.crate').stat().st_size for n,v in selected),'incremental_source_archives_bytes':sum((cache/f'{n}-{v}.crate').stat().st_size for n,v in extra),'versions':versions,'licenses':{f'{n}@{v}':packages[(n,v)]['license'] for n,v in selected},'source_lines':loc,'helper_release_bytes':binary.stat().st_size,'helper_stripped_bytes':stripped.stat().st_size,'helper_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'stripped_empty_helper_bytes':(tmp/'empty-helper').stat().st_size,'stripped_helper_increment_bytes':stripped.stat().st_size-(tmp/'empty-helper').stat().st_size,'full_helper_tar_gz_bytes':(tmp/'helper.tar.gz').stat().st_size,'install_and_full_replacement_update_scope':'isolated helper binary only; no Python/models/runtime payload needed by native helper','unmeasured':['full BRN application bundle/link delta','signed/notarized distributable','clean-machine offline installation/upgrade/rollback','delta-update transfer size','lifetime maintenance cost','rendering service/package deployment']}
(E/'costs.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['licenses','versions']},indent=2))
