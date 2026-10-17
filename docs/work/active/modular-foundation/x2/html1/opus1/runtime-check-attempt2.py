from pathlib import Path
import subprocess,json,time,hashlib,os,signal,re
R=Path('/private/tmp/brn-x2-html-r1');binary=R/'artifacts/final-brn-intake-helper'
sha=lambda b:hashlib.sha256(b).hexdigest()
records=[]
fixture=Path('/Users/evokessler/.t3/worktrees/brn-rust/t3-ad771af6/crates/brn-intake/tests/fixtures/textual.html').read_bytes()
for name,request in [('large-links',(R/'links-large-request.json').read_bytes()),('friendly',json.dumps({'limits':None,'kind':'html','bytes':list(fixture)}).encode())]:
 prefix=R/'logs'/('final-runtime-'+name);prefix.with_suffix('.request.json').write_bytes(request) if not prefix.with_suffix('.request.json').exists() else None
 source=bytes(json.loads(request)['bytes']);elapsed=None;pid=None;exitcode=0
 if name=='friendly':
  started=time.monotonic()
  with prefix.with_suffix('.stdout').open('wb') as out,prefix.with_suffix('.time').open('wb') as err:
   p=subprocess.Popen(['/usr/bin/time','-l',str(binary)],stdin=subprocess.PIPE,stdout=out,stderr=err,env={},start_new_session=True)
   pid=p.pid
   try:p.communicate(request,timeout=30)
   except BaseException:
    os.killpg(p.pid,signal.SIGKILL);p.wait();raise
  elapsed=time.monotonic()-started;exitcode=p.returncode
 result=json.loads(prefix.with_suffix('.stdout').read_bytes());assert exitcode==0,(exitcode,result)
 assert result['schema']==1 and bytes(result['original_sha256']).hex()==sha(source)
 assert result['sources'][0]['bytes']==list(source) and result['sources'][0]['text']==result['markdown']
 md=result['markdown']
 if name=='large-links':
  assert md.count('steady')==300000
  cursor=0
  for i in range(2000):
   marker=f'[l{i}](https://x.test/u{i})';cursor=md.index(marker,cursor)+len(marker)
 else:
  cursor=0
  for marker in ['Harbour report','Café','日本','[source link](https://example.test/a?b=1&c=2)','first item','second item','step one','step two','quoted words','let tide = 3;','end']:
   cursor=md.index(marker,cursor)+len(marker)
 gone=None
 if pid is not None:
  try:os.killpg(pid,0)
  except ProcessLookupError:gone=True
  else:gone=False
  assert gone
 rec={'name':name,'args':['/usr/bin/time','-l',str(binary)],'env':{},'binary_sha256':sha(binary.read_bytes()),'request_sha256':sha(request),'source_sha256':sha(source),'input_bytes':len(source),'exit':exitcode,'wall_python_seconds':elapsed,'output_markdown_bytes':len(md.encode()),'timing':prefix.with_suffix('.time').read_text(),'stdout_sha256':sha(prefix.with_suffix('.stdout').read_bytes()),'pgid':pid,'pgid_absent_after_wait':gone,'retained_large_result_validated_without_rerun':name=='large-links','assertions':'exact root bytes/hash and source text; all 300000 steady words and 2000 link labels/targets in order' if name=='large-links' else 'exact root bytes/hash and source text; ordered heading/Unicode/list/quote/code/link/end facts'}
 prefix.with_suffix('.receipt.json').write_text(json.dumps(rec,indent=2)+'\n');records.append(rec)
 print(json.dumps(rec))
