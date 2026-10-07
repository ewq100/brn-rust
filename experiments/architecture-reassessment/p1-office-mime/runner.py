"""Host harness. Runs the parser under an actual deny-default macOS sandbox."""
from pathlib import Path
import os, sys, json, tempfile, shutil, subprocess, time, resource, signal, socket, hashlib, platform, threading
ROOT=Path(__file__).resolve().parent
BIN=Path('/private/tmp/brn-p1-target/release/brn-p1-office-mime')
E=ROOT/'evidence'; E.mkdir(exist_ok=True)
def limits():
 resource.setrlimit(resource.RLIMIT_CPU,(3,3)); resource.setrlimit(resource.RLIMIT_FSIZE,(32*1024*1024,32*1024*1024)); resource.setrlimit(resource.RLIMIT_NOFILE,(64,64))
def context():
 tmp=Path(tempfile.mkdtemp(prefix='brn-p1-helper-',dir='/private/tmp')); (tmp/'out').mkdir()
 profile=f'''(version 1)
(deny default)
(allow process-fork)
(allow sysctl-read)
(allow file-map-executable (literal "{BIN}") (subpath "/System/Library") (subpath "/usr/lib"))
(allow file-read* (subpath "/System/Library") (subpath "/System/Volumes/Preboot") (subpath "/private/preboot") (subpath "/usr/lib") (subpath "/private/var/db/dyld") (literal "{BIN}") (subpath "{tmp}"))
(allow file-write* (subpath "{tmp}/out"))
(allow file-read* file-write* (literal "/dev/null"))
'''
 (tmp/'profile.sb').write_text(profile); return tmp,profile

def invoke(tmp,args):
 start=time.monotonic(); log=open(tmp/'stdout','wb'); err=open(tmp/'stderr','wb')
 p=subprocess.Popen([str(BIN),'--sandbox',str(tmp/'profile.sb'),*map(str,args)],cwd=tmp,env={},stdout=log,stderr=err,start_new_session=True,preexec_fn=limits)
 expired=threading.Event()
 def timeout():
  expired.set()
  try: os.killpg(p.pid,signal.SIGKILL)
  except ProcessLookupError: pass
 timer=threading.Timer(5,timeout);timer.start()
 _,status,ru=os.wait4(p.pid,0); timer.cancel();p.returncode=os.waitstatus_to_exitcode(status); log.close();err.close()
 return {'exit':p.returncode,'wall_timeout':expired.is_set(),'wall_ms':round((time.monotonic()-start)*1000,3),'peak_rss_bytes':ru.ru_maxrss,'stderr':(tmp/'stderr').read_text()},(tmp/'stdout').read_text()
results=[]
for name in ['harbor.docx','harbor.pptx','single.eml','plural.eml','hostile-path.docx','hostile-dtd.docx','hostile-inflate.docx','hostile-members.docx']:
 for attempt in range(3 if not name.startswith('hostile') else 1):
  tmp,profile=context(); shutil.copyfile(ROOT/'fixtures'/name,tmp/'input')
  # Extension is host controlled; member or MIME filenames never select a host path.
  ext=Path(name).suffix; (tmp/'input').rename(tmp/f'input{ext}')
  measurement,output=invoke(tmp,[tmp/f'input{ext}',tmp/'out']); measurement.update(fixture=name,attempt=attempt,cache_scope='first process (OS caches uncontrolled)' if attempt==0 else 'warm process')
  results.append(measurement)
  if measurement['exit']==0 and attempt==0:
   dest=E/name.replace('.','-'); dest.mkdir(exist_ok=True)
   for p in (tmp/'out').iterdir(): shutil.copyfile(p,dest/p.name)
   (dest/'stdout.json').write_text(output)
  shutil.rmtree(tmp)
(E/"launch-measurements.json").write_text(json.dumps(results,indent=2))
# Restriction checks use synthetic canaries outside the input tree, not actual vault/secrets.
canary=Path(tempfile.mkdtemp(prefix='brn-p1-forbidden-',dir='/private/tmp')); (canary/'vault.md').write_text('synthetic vault canary'); (canary/'credential').write_text('synthetic credential canary')
tmp,profile=context(); listener=socket.socket(); listener.bind(('127.0.0.1',49731));listener.listen()
restriction,output=invoke(tmp,['--restriction-check',canary/'vault.md',canary/'credential',ROOT/'Cargo.toml']);listener.close()
(E/'restriction.stdout.txt').write_text(output);(E/'sandbox.sb').write_text(profile.replace(str(tmp),'<per-run-input-root>').replace(str(BIN),'<pinned-helper>'))
restriction['assertions']={'vault_read_denied':'vault.md Err' in output and 'Operation not permitted' in output,'credential_read_denied':'credential Err' in output,'repository_read_denied':'Cargo.toml Err' in output,'network_denied':'network Err' in output and 'Operation not permitted' in output}
shutil.rmtree(canary);shutil.rmtree(tmp)
# Same helper executable makes child and grandchild; group cancellation targets all three.
tmp,profile=context();log=open(tmp/'tree.log','wb');p=subprocess.Popen([str(BIN),'--sandbox',str(tmp/'profile.sb'),'--cancel-tree'],cwd=tmp,env={},stdout=log,stderr=subprocess.STDOUT,start_new_session=True,preexec_fn=limits)
time.sleep(.25); start=time.monotonic();os.killpg(p.pid,signal.SIGTERM) if p.poll() is None else None; p.wait(timeout=2); log.close(); text=(tmp/'tree.log').read_text();pids=sorted(set(int(line.split('=')[1]) for line in text.splitlines() if line.startswith(('pid=','child='))))
states={pid:subprocess.run(['/bin/ps','-p',str(pid),'-o','stat='],capture_output=True,text=True).stdout.strip() for pid in pids}
cancel={'parent_exit':p.returncode,'pids':pids,'states_after':states,'wall_ms':round((time.monotonic()-start)*1000,3),'assertion':len(pids)==3 and all(not state or state.startswith('Z') for state in states.values()),'note':'empty or zombie is terminated; orphan reaping is OS responsibility'}
(E/'cancel.stdout.txt').write_text(text);shutil.rmtree(tmp)
# A stalled inherited process tree proves the wall watchdog, not only manual cancellation.
tmp,profile=context(); wall,output=invoke(tmp,['--cancel-tree']);(E/'wall-timeout.stdout.txt').write_text(output);shutil.rmtree(tmp)
assert wall['wall_timeout'] and wall['exit']==-9
# Invalid activation must stop before any input read/converter call.
tmp,profile=context();(tmp/'profile.sb').write_text('(invalid profile)');fail,output=invoke(tmp,[ROOT/'fixtures'/'harbor.docx',tmp/'out']);assert fail['exit']==70 and not list((tmp/'out').iterdir());shutil.rmtree(tmp)
report={'host':platform.platform(),'architecture':platform.machine(),'environment':{},'limits':{'input_bytes':8*1024*1024,'inflated_bytes':16*1024*1024,'members':512,'xml_depth':128,'xml_events_per_part':200000,'cpu_seconds':3,'wall_seconds':5,'output_bytes_per_file':32*1024*1024,'file_descriptors':64,'hard_rss_limit':'not established on macOS; byte/event caps + measured RSS'},'runs':results,'restrictions':restriction,'cancel':cancel,'wall_timeout_witness':wall,'activation_failure_witness':fail}
(E/'measurements.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
assert all(x['exit']==0 for x in results), 'helper execution failed; inspect evidence'
assert all(restriction['assertions'].values()),'restriction witness failed'
assert cancel['assertion'],'process-tree cancellation failed'
