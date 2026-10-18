import sys,json,pathlib,re,hashlib,time
base=pathlib.Path('/private/tmp/brn-int-d-offline-prepare-diagnostic-20261017-evidence')
name=sys.argv[1];receipt=base/(name+'.json');r=json.loads(receipt.read_text());log=(base/(name+'.log')).read_text();rows=[]
for raw in re.findall(r'^\s*Running .*? \((/private/tmp/[^)]+)\)',log,re.M):
 p=pathlib.Path(raw)
 if p.exists():
  dep=p.with_suffix('.d'); row={'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'mtime':p.stat().st_mtime,'receipt_mtime':receipt.stat().st_mtime,'not_modified_after_receipt':p.stat().st_mtime<=receipt.stat().st_mtime,'dep_info':str(dep),'dep_info_sha256':hashlib.sha256(dep.read_bytes()).hexdigest() if dep.exists() else None}
  rows.append(row)
# Cargo's integration-test binary dependency is not printed as Running.
# Associate a debug executable by its hard-linked hashed Cargo artifact.
for exe in ['brn','brn-desktop','brn-intake-helper']:
 p=pathlib.Path(r['target'])/'debug'/exe
 if p.exists():
  matches=[d for d in (p.parent/'deps').glob(exe.replace('-','_')+'-*') if d.is_file() and d.suffix=='' and d.stat().st_size==p.stat().st_size and hashlib.sha256(d.read_bytes()).digest()==hashlib.sha256(p.read_bytes()).digest()]
  for artifact in matches:
   dep=artifact.with_suffix('.d')
   rows.append({'path':str(p),'hashed_artifact':str(artifact),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'mtime':p.stat().st_mtime,'receipt_mtime':receipt.stat().st_mtime,'not_modified_after_receipt':p.stat().st_mtime<=receipt.stat().st_mtime,'dep_info':str(dep),'dep_info_sha256':hashlib.sha256(dep.read_bytes()).hexdigest() if dep.exists() else None,'association':'Exact executable bytehash matches hashed Cargo artifact; not itself a Running test target'})
(base/(name+'-artifacts.json')).write_text(json.dumps({'captured_at':time.time(),'receipt_sha256':hashlib.sha256(receipt.read_bytes()).hexdigest(),'source_fingerprints_in':str(receipt),'artifacts':rows,'qualification':'Only not_modified_after_receipt=true associates the captured file to that run; later artifacts are not relabelled.'},indent=2)+'\n');print(name,len(rows),[r['not_modified_after_receipt'] for r in rows])
