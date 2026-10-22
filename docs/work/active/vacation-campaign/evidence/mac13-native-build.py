from pathlib import Path
import datetime,hashlib,json,os,runpy,shutil,subprocess,time
repo=Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust')
root=Path('/tmp/brn-native-mac13-preparation')
source='3827d6a13953533c7322fd7157d975a32dee5cb2'
paths=['crates','agent','Cargo.toml','Cargo.lock','rust-toolchain.toml','scripts/make-macos-app.sh','tools/xberg-intake-helper','scripts/prepare-xberg-adapted.py','scripts/build-document-helpers.sh']
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()
assert not subprocess.check_output(['git','diff',source,'--',*paths],cwd=repo,timeout=10)
assert not subprocess.check_output(['git','status','--porcelain','--',*paths],cwd=repo,timeout=10)
assert shutil.disk_usage(repo).free>=2*1024**3
approved=Path('/tmp/brn-mac13-preparation/prepare.py')
assert hashlib.sha256(approved.read_bytes()).hexdigest()=='cd272ff3734a2a46c2a83ff944a9d2f15d768e6bef45de03df49378009854813'
cleanup=runpy.run_path(str(approved))['stop_and_reap']
fixture=json.loads((repo/'docs/work/active/vacation-campaign/native-fixtures/mac13-manifest.json').read_text())
cli_build=json.loads((repo/'docs/work/active/vacation-campaign/evidence/mac13-cli-build-original.json').read_text())
artifact=Path('/Users/evokessler/.codex/artifacts/brn-mac13-native-3827d6a1')
artifact.mkdir(exist_ok=False)
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def fixture_files():
 result={}
 for item in fixture['primary_roots']:
  for p in Path(item['data_dir']).rglob('*'):
   assert not p.is_symlink()
   if p.is_file():result[str(p)]=digest(p)
 return result
before_fixtures=fixture_files()
old=repo/'target/debug/brn-desktop';old_proof=None
if old.exists():
 saved=artifact/'preexisting-target-unqualified';shutil.copy2(old,saved)
 old_proof={'path':str(saved),'sha256':digest(saved),'bytes':saved.stat().st_size,'source':'unknown; diagnostic preservation only, not adopted'}
script=root/'build.sh';script.write_text('''#!/bin/zsh
set -eu
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-native-mac13-build
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
# Preserve target executable first (supervisor), then rebuild only desktop
# to bind this output to the actual current source. No dependency/helper clean.
cargo clean -p brn-desktop --target-dir "$CARGO_TARGET_DIR"
cargo build -p brn-desktop --bin brn-desktop --features native-ui --locked --offline --jobs 1
''')
start=time.monotonic();p=None;reason=None;signals=[];errors=[];receipt={'source':source,'head':head,'source_paths':paths,'preexisting_target':old_proof,'native_observed':False,'owner_accepted':False,'app_launched':False,'signing_or_release':False}
try:
 with (root/'build.log').open('wb') as log:
  p=subprocess.Popen(['zsh',str(script)],cwd=repo,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if time.monotonic()-start>=870:reason='900s total deadline with30s cleanup reserve'
   elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
   elif (root/'build.log').stat().st_size>8*1024**2:reason='8MiB sampled log threshold'
   if reason:break
   time.sleep(2)
except BaseException as exc:reason='supervisor exception '+type(exc).__name__
finally:
 clean=p is not None and cleanup(p,signals,errors)
 receipt.update(exit_code=None if p is None else p.returncode,seconds=time.monotonic()-start,stop_reason=reason,signals=signals,cleanup_errors=errors,group_absent=clean,leader_reaped=p is not None and p.returncode is not None)
 (root/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
assert receipt['exit_code']==0 and reason is None and clean and not errors
assert head==subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()
assert not subprocess.check_output(['git','diff',source,'--',*paths],cwd=repo,timeout=10)
assert 'Compiling brn-desktop v' in (root/'build.log').read_text()
artifacts={}
for name in ['brn-desktop','brn-xberg-helper','brn-xberg-helper-brn']:
 src=repo/'target/debug'/name if name=='brn-desktop' else Path(cli_build['artifacts'][name]['path'])
 sha=digest(src)
 if name!='brn-desktop':assert sha==cli_build['artifacts'][name]['sha256']
 dst=artifact/name;shutil.copy2(src,dst);dst.chmod(0o555);assert digest(dst)==sha
 artifacts[name]={'path':str(dst),'sha256':sha,'bytes':dst.stat().st_size}
bundles=[]
for item in fixture['primary_roots']:
 label='Private' if item['scope']=='private-chat' else 'General'
 app=artifact/f'BRN MAC13 {label}.app'
 argv=['bash','scripts/make-macos-app.sh','--output',str(app),'--binary',artifacts['brn-desktop']['path'],'--xberg-helper',artifacts['brn-xberg-helper']['path'],'--xberg-helper-brn',artifacts['brn-xberg-helper-brn']['path'],'--data-dir',item['data_dir']]
 package=subprocess.run(argv,cwd=repo,capture_output=True,text=True,timeout=30)
 assert package.returncode==0,(package.returncode,package.stderr)
 for name,proof in artifacts.items():assert digest(app/'Contents/Resources/bin'/name)==proof['sha256']
 launcher=app/'Contents/MacOS/BRN-Threads';assert item['data_dir'] in launcher.read_text()
 bundles.append({'scope':item['scope'],'app':str(app),'data_dir':item['data_dir'],'command':argv,'stdout':package.stdout,'launcher_sha256':digest(launcher),'plist_sha256':digest(app/'Contents/Info.plist'),'binary_and_helper_hashes':{n:v['sha256'] for n,v in artifacts.items()}})
after_fixtures=fixture_files();assert after_fixtures==before_fixtures
receipt.update(observed_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),artifacts=artifacts,bundles=bundles,fixture_files_before=before_fixtures,fixture_files_after=after_fixtures,fixture_files_unchanged=True,qualified_artifact_build=True,limits='Current ordinary native-ui artifact and unsigned explicit-data assembly only; no app launch/native/control/account/release/owner acceptance. Then-current source/availability must be reconciled before future use.')
(root/'artifact-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt),flush=True)
