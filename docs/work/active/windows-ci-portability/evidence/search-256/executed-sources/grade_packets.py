"""Operational launcher only; fixed judge, packet builder and rubric are frozen."""
from pathlib import Path
from datetime import datetime,timezone
import sys,json,hashlib,subprocess,random
repo=Path('/Users/evokessler/repos/brn-agent-e-256')
sys.path[:0]=[str(repo/'benchmarks/brn_agent'),str(repo/'benchmarks/brn_agent/pilots/terra_sol_v3')]
from limits import Reservations
from deadline_guard import Guard
root=Path('/Users/evokessler/Documents/Codex/brn-agent-e-256')
block=sys.argv[1];source=root/'grading'/block
packets=sorted(p for p in source.glob('*.json') if p.name!='private-key.json')
random.Random(256).shuffle(packets)
output=root/'grading'/(block+'-receipts');output.mkdir(exist_ok=False)
end=datetime.fromisoformat(json.loads((root/'live-deadline.json').read_text())['deadline'])
remaining=(end-datetime.now(timezone.utc)).total_seconds()
if remaining<60:raise RuntimeError('Original cycle deadline exhausted; no grading dispatch')
bindings=json.loads((root/'bindings.json').read_text())
checks=[root/'binaries/agentbench-judge',root/'grader-instructions.txt']
for path in checks:
 if hashlib.sha256(path.read_bytes()).hexdigest()!=bindings['fixed_files'][str(path)]:raise RuntimeError('Frozen grader changed')
guard=Guard(output/'deadline.jsonl',min(600,remaining),block);guard.install();status=1
try:
 for packet in packets:
  payload=json.loads(packet.read_text());identity=payload['packet_sha256']
  actual=hashlib.sha256(json.dumps(payload['packet'],sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
  if identity!=actual:raise RuntimeError('Packet hash mismatch')
  Reservations(root/'grader-dispatch.jsonl',24).reserve(block+'/'+identity,1)
  with (output/(identity+'.json')).open('x') as stream,(output/(identity+'.stderr')).open('x') as error:
   p=subprocess.run([str(root/'binaries/agentbench-judge'),'--live','/Users/evokessler/.codex/auth.json',str(packet),str(root/'grader-instructions.txt')],stdout=stream,stderr=error,timeout=50)
  receipt=json.loads((output/(identity+'.json')).read_text())
  if p.returncode or receipt.get('status')!='completed':raise RuntimeError('Grader failure; retained, no automatic retry')
  budget=receipt.get('model_call_budget') or {}
  if budget.get('limit')!=1 or budget.get('started')!=1 or budget.get('refused'):raise RuntimeError('Grader accounting unknown')
  text=receipt['result']['text'].strip()
  if text.startswith('```'):text='\n'.join(text.splitlines()[1:-1])
  grade=json.loads(text)
  if grade.get('packet_sha256')!=identity:raise RuntimeError('Grade belongs to another packet')
  if set(grade.get('scores',{}))!={'correctness','completeness','citations','abstention','visibility'}:raise RuntimeError('Missing grade dimensions')
  if any(type(v)is not int or not 0<=v<=4 for v in grade['scores'].values()):raise RuntimeError('Invalid grade scale')
  (output/(identity+'.grade.json')).write_text(json.dumps(grade,indent=2)+'\n')
  print(json.dumps({'packet':identity,'scores':grade['scores'],'complete':grade.get('complete'),'defects':grade.get('material_defects')}),flush=True)
 status=0
finally:
 status=guard.finish(status)
sys.exit(status)
