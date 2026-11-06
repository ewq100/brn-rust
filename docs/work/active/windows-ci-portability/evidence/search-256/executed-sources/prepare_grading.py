from pathlib import Path
import json,sys
repo=Path('/Users/evokessler/repos/brn-agent-e-256')
sys.path[:0]=[str(repo/'benchmarks/brn_agent'),str(repo/'benchmarks/brn_agent/experiments')]
import evidence,score,search_256_evidence as e
r=Path('/Users/evokessler/Documents/Codex/brn-agent-e-256');block=sys.argv[1];roots=sys.argv[2:];out=r/'grading'/block;out.mkdir(parents=True,exist_ok=False);keys={}
for phase in roots:
 root=r/phase;fixture=r/('confirmation.json' if phase=='confirmation' else 'screening.json');manifest=json.loads((root/'manifest.json').read_text());cases=json.loads(fixture.read_text())['cases']
 suite={'version':1,'weights':score.WEIGHTS,'qualification':'limited-engineering-unqualified','fixture_sha256':manifest['fixture_sha256'],'scenarios':[{'id':c['id'],'category':'research','weight':1,'repetitions':2,'events':1,'criteria':['correctness','completeness','citations','abstention','visibility'],'coverage':'partial'} for c in cases]}
 (root/'suite.json').write_text(json.dumps(suite,indent=2)+'\n');index=evidence.collect(root,root/'suite.json');(root/'agentbench-evidence.json').write_text(json.dumps(index,indent=2)+'\n');evidence.verify_manifest(index,root)
 attempts=[]
 for p in sorted(root.glob('runs/*/*/result.json')):
  attempts.append(e.attempt(p,root));packet=e.answer_packet(p,root,fixture,root/'agentbench-evidence.json');identity=packet['packet_sha256'];(out/(identity+'.json')).write_text(json.dumps(packet,indent=2)+'\n');keys[identity]={'native_result':str(p),'packet_sha256':identity}
 (root/'E-attempts.json').write_text(json.dumps(attempts,indent=2)+'\n')
(out/'private-key.json').write_text(json.dumps(keys,indent=2)+'\n');print('Prepared',len(keys),'anonymous bound packets')
