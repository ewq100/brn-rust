from pathlib import Path
import subprocess,os,time,json,hashlib,runpy,shutil,datetime,tomllib,copy
repo=Path('/Users/evokessler/.codex/worktrees/export-durability/brn-rust'); root=Path('/tmp/brn-tls-resolution'); base='3827d6a13953533c7322fd7157d975a32dee5cb2'
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True,timeout=10).strip()==base
assert not subprocess.check_output(['git','status','--porcelain'],cwd=repo,timeout=10)
old=(repo/'Cargo.lock').read_bytes();(root/'Cargo.lock.before').write_bytes(old)
approved=Path('/tmp/brn-mac13-preparation/prepare.py');assert hashlib.sha256(approved.read_bytes()).hexdigest()=='cd272ff3734a2a46c2a83ff944a9d2f15d768e6bef45de03df49378009854813'
cleanup=runpy.run_path(str(approved))['stop_and_reap']
pairs=[('rustls-webpki','0.103.15','0.103.16'),('webpki-roots','1.0.9','1.0.10'),('webpki-root-certs','1.0.9','1.0.10')]
env={k:v for k,v in os.environ.items() if k in ['PATH','HOME','TMPDIR','LANG','LC_ALL','USER','LOGNAME','SHELL','SYSTEMROOT','SSL_CERT_FILE','SSL_CERT_DIR','HTTP_PROXY','HTTPS_PROXY','NO_PROXY','http_proxy','https_proxy','no_proxy']};env.update(CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0',CARGO_HTTP_TIMEOUT='20')
receipt={'base':base,'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'commands':[],'scope':'Three exact TLS transitive patches only; no full audit, tool install, runtime/account/native action'}
for i,(name,before,after) in enumerate(pairs,1):
 assert shutil.disk_usage(repo).free>=2*1024**3
 argv=['/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo','update','-p',name+'@'+before,'--precise',after]
 logpath=root/f'update-{i}.log';start=time.monotonic();p=None;reason=None;signals=[];errors=[]
 try:
  with logpath.open('wb') as log:
   p=subprocess.Popen(argv,cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
   while p.poll() is None:
    if time.monotonic()-start>=120:reason='120s active deadline plus bounded22s cleanup'
    elif shutil.disk_usage(repo).free<1024**3:reason='1GiB disk floor'
    elif logpath.stat().st_size>1024**2:reason='1MiB sampled log threshold'
    if reason:break
    time.sleep(.25)
 except BaseException as exc:reason='supervisor exception '+type(exc).__name__
 finally:
  clean=p is not None and cleanup(p,signals,errors)
  row={'argv':argv,'exit_code':None if p is None else p.returncode,'seconds':time.monotonic()-start,'stop_reason':reason,'signals':signals,'cleanup_errors':errors,'group_absent':clean,'leader_reaped':p is not None and p.returncode is not None,'log':str(logpath)}
  receipt['commands'].append(row);(root/'resolution.json').write_text(json.dumps(receipt,indent=2)+'\n')
 assert row['exit_code']==0 and reason is None and clean and not errors,row
new=(repo/'Cargo.lock').read_bytes();(root/'Cargo.lock.after').write_bytes(new)
expected=copy.deepcopy(tomllib.loads(old.decode()));actual=tomllib.loads(new.decode());meta=json.loads(Path('/tmp/brn-tls-advisories/release-metadata.json').read_text());checksums={x['package']:x['checksum'] for x in meta['versions']}
for package in expected['package']:
 for name,before,after in pairs:
  if package['name']==name and package['version']==before:package.update(version=after,checksum=checksums[name])
  package['dependencies']=[after if dep==before else dep for dep in package.get('dependencies',[]) for before,after in [(f'{name} {before}',f'{name} {after}')]] if 'dependencies' in package else package.get('dependencies')
 # Do not add absent dependency fields while normalizing version-qualified edges.
 if package.get('dependencies') is None:package.pop('dependencies',None)
assert expected==actual,'Unexpected additional lock resolution change; preserve and reassess'
status=subprocess.check_output(['git','status','--porcelain'],cwd=repo,text=True,timeout=10);assert status.strip()=='M Cargo.lock',status
receipt.update(completed_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),only_three_expected_package_updates=True,package_count_before=len(expected['package']),package_count_after=len(actual['package']),lock_sha256_before=hashlib.sha256(old).hexdigest(),lock_sha256_after=hashlib.sha256(new).hexdigest(),status=status)
(root/'resolution.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt),flush=True)
