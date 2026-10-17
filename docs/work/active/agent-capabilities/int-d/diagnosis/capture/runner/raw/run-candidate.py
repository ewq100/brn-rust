import subprocess,sys,pathlib,os,json,time,hashlib,tempfile,re,shutil
name=sys.argv[1];cmd=sys.argv[2:];root=pathlib.Path.cwd();ev=pathlib.Path('/private/tmp/brn-int-d-capture-runner-20261017-evidence');tmp=tempfile.mkdtemp(prefix='brn-int-d-'+name+'-',dir='/private/tmp')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def sources():
 tracked=subprocess.check_output(['git','ls-files','-z','crates','agent','Cargo.lock','Cargo.toml','rust-toolchain.toml']).decode().split('\0')
 paths=sorted(set([root/p for p in tracked if p]+list((root/'crates').rglob('*.rs'))+list((root/'docs/work/active/agent-capabilities/int-d').glob('corpus*.json'))+[root/'docs/work/active/agent-capabilities/int-d/diagnosis/capture/material-hashes.json']))
 return {str(p.relative_to(root)):sha(p) for p in sorted(paths) if p.is_file()}
pre=sources();diff=subprocess.check_output(['git','diff','--binary','HEAD']);head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();env=dict(os.environ,TMPDIR=tmp,CARGO_TARGET_DIR='/private/tmp/brn-int-d-20261017-target',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='1');fixture_env={'BRN_NATIVE_MODEL_DIR':'','BRN_EMBEDDING_MODEL_DIR':''}
if any('native' in arg for arg in cmd):
 fixture_env.update(json.loads((ev/'ort-cached-no-download-preflight.json').read_text())['env'])
helper=pathlib.Path(env['CARGO_TARGET_DIR'])/'debug/brn-intake-helper'
if helper.exists():fixture_env['BRN_INTAKE_HELPER']=str(helper)
for key,tool in [('BRN_PDFTOTEXT','pdftotext'),('BRN_PDFIMAGES','pdfimages')]:
 found=shutil.which(tool)
 if found:fixture_env[key]=found
env.update(fixture_env);start=time.time()
with open(ev/(name+'.log'),'w') as f:p=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
text=(ev/(name+'.log')).read_text();post=sources();counts=[list(map(int,m)) for m in re.findall(r'test result: [^\n]*?(\d+) passed; (\d+) failed; (\d+) ignored;',text)]
r={'source_root':str(root),'head':head,'diff_sha256':hashlib.sha256(diff).hexdigest(),'source_hashes':pre,'source_unchanged':pre==post,'command':cmd,'tmpdir':tmp,'explicit_fixture_env':fixture_env,'target':env['CARGO_TARGET_DIR'],'exit':p.returncode,'seconds':time.time()-start,'test_summaries':counts,'log_sha256':sha(ev/(name+'.log'))};(ev/(name+'.json')).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:v for k,v in r.items() if k!='source_hashes'},indent=2));print(text[-5000:]);subprocess.run([sys.executable,str(ev/'capture-artifacts.py'),name],check=True)
