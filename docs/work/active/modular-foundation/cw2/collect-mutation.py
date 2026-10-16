#!/usr/bin/env python3
"""Read-only per-ID receipts; never upgrades a survivor without a mutant rerun."""
import collections,hashlib,json,pathlib,re,subprocess
root=pathlib.Path(__file__).resolve().parents[5];evidence=pathlib.Path(__file__).resolve().parent

def classification(m,old):
 name=m['name'];line=m['span']['start']['line'];file=m['file']
 if not old:
  function=m['function']['function_name'] if m.get('function') else ''
  if file.endswith('brn-threads-core/src/store.rs') and line==1502:
   reason='Selected core/app tests do not isolate ChatContext-move Stale outcome preference before visibility refusal; later guards can mask this edge. Full AI normal regressions are not part of this mutant command. No equivalence claimed.'
  elif file.endswith('brn-threads-core/src/store.rs') and line==1525:
   reason='Core Apply subset does not independently exercise working-Note selected-conversation exception versus canonical general/private reference scope. Normal AI privacy witnesses do not retroactively kill this core mutant. Coverage gap, not equivalence.'
  elif file.endswith('brn-threads-core/src/store.rs') and line==1543:
   reason='Selected core/app fixtures revoke current access; they do not isolate an archived/invisible historical revision while current record remains visible. Current gate masks this edge in tested data. No equivalence claimed.'
  elif 'ReadLedger' in function:
   reason='Selected AI fixtures do not distinguish this exact byte-range coverage edge; see source window and actual mutation diff. Remaining range-accounting coverage gap or masked predicate, no equivalence claimed.'
  elif 'writing_read_allowed' in function:
   reason='Selected AI fixtures did not distinguish this exact current/historical/archive/working-Note predicate despite normal general/private delivery witnesses. Source window retained; remaining privacy edge coverage gap, no equivalence claimed.'
  elif 'attach_writing_evidence' in function:
   reason='Selected AI fixtures do not isolate this exact ledger/claim/quotation/retry binding from canonical preparation and other checks. Remaining evidence edge coverage gap or unproven redundancy.'
  elif 'writing_context_review' in function:
   reason='Selected app tests do not isolate this exact batch identity/token/error boundary; native notify/access/candidate witnesses were not run against this app mutant. No equivalence claimed.'
  elif 'active_writing_context' in function or 'select_writing' in function:
   reason='App fixtures combine explicit selection and request events; this exact archived/undone/scope/choice branch was not independently distinguished. No equivalence claimed.'
  elif 'writing_candidate_review' in function:
   reason='App fixtures do not isolate this exact coverage/provenance/disclosure predicate from canonical candidate validation. Remaining representation gap or unproven redundancy.'
  elif 'writing_request_in_context' in function:
   reason='App fixtures do not independently distinguish every request target/conversation/scope condition. Other guards mask this mutation in tested data; no equivalence claimed.'
  elif 'resolve_run_selection' in function:
   reason='App fixtures do not distinguish this exact accepted provider/effort/catalogue combination. CLI normal compatibility proof is not an actual rerun of this mutant.'
  else:reason='Actual survivor; exact function/source window/diff and selected tests retained. Remaining coverage gap or redundant guard needs assessment; no equivalence or kill claimed.'
  return {'classification':'coverage-gap-or-unproven-redundancy','reason':reason}
 if file.endswith('/run.rs'):
  reason='Selected app witnesses lack a positive Copilot/low/high or catalogue/source-budget combination for this branch; not equivalent.'
 elif file.endswith('/review.rs') and line>=306:
  reason='Positive exhaustive retained-text completion and mixed read/version/requirements cases were not asserted by selected app tests. No equivalence claimed; remaining review-representation coverage gap.'
 elif file.endswith('/review.rs') and line in [213,224,225,226]:
  reason='Canonical candidate preparation already validates role and immutable evidence/thread/scope. Selected fixtures do not isolate this app guard from those other checks; redundancy in tested fixtures is not proof of equivalence.'
 elif file.endswith('/review.rs'):
  reason='Selected app fixtures do not distinguish this exact read/source/privacy/wider-Note predicate (general/private and selected-vs-canonical cases incomplete at f282). No equivalence or later kill claimed.'
 elif file.endswith('/writing.rs') and line<110:
  reason='Resolver fixtures combine explicit context Messages and v2 requests; selected tests do not isolate this event/scope branch. No equivalence; archived/undone/repeated choices changed in later pin and need actual reruns.'
 elif file.endswith('/writing.rs') and line>=549:
  reason='Selected fixtures do not independently distinguish all target/conversation/canonical request visibility conditions. Other guards mask this mutation in tested data; no equivalence claimed.'
 else:reason='Selected fixtures omit this exact visibility/creation/archive branch or precise owner error behavior. No equivalence claimed; changed pin normal tests do not retroactively kill this original mutant.'
 return {'classification':'coverage-gap-or-unproven-redundancy','reason':reason}

result={'original_cap_seconds':1800,'qualification':'Partial targeted campaigns, no full mutation pass. New tests are not kills absent actual mutant recheck. All setup time counts.','campaigns':[]}
for receipt_name in ['mutation-run.json','mutation-supplement.json']:
 path=evidence/receipt_name
 if not path.exists():continue
 receipt=json.loads(path.read_text());work=pathlib.Path(receipt['work']);old=receipt_name=='mutation-run.json'
 campaign={'receipt':receipt_name,'pin':receipt['pin'],'crates_tree':receipt['crates_tree'],'phases':[]}
 for phase in receipt['phases']:
  base=work/phase['name']/'mutants.out';out=base/'outcomes.json';p={'name':phase['name'],'selected':phase.get('selected'),'command':phase['command'],'controller_interrupted':phase.get('interrupted'),'ids':[],'raw_files':{}}
  for raw in [out,base/'mutants.json',base/'debug.log',work/(phase['name']+'-selected.json'),work/'source.diff']:
   if raw.exists():p['raw_files'][str(raw)]={'sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),'bytes':raw.stat().st_size}
  if out.exists():
   data=json.loads(out.read_text());p['counts']=dict(collections.Counter(o['summary'] for o in data['outcomes']))
   for outcome in data['outcomes']:
    mutant=outcome['scenario'].get('Mutant') if isinstance(outcome['scenario'],dict) else None
    entry={'id':mutant['name'] if mutant else 'Baseline','verdict':outcome['summary']}
    for field in ['log_path','diff_path']:
     if outcome.get(field):
      raw=base/outcome[field];entry[field]={'path':str(raw),'sha256':hashlib.sha256(raw.read_bytes()).hexdigest() if raw.exists() else None}
    if mutant and outcome['summary']=='MissedMutant':
     entry.update(classification(mutant,old))
     lines=subprocess.check_output(['git','show',receipt['pin']+':'+mutant['file']],cwd=root,text=True).splitlines();at=mutant['span']['start']['line']
     entry['source_window']={'file':mutant['file'],'start_line':max(1,at-1),'lines':lines[max(0,at-2):at+1]}
    p['ids'].append(entry)
   attempted={i['id'] for i in p['ids']}
   selected=base/'mutants.json'
   if selected.exists():
    all_ids=json.loads(selected.read_text());p['unexecuted_ids']=[m['name'] for m in all_ids if m['name'] not in attempted]
  # Verify leaked copy identity after each campaign; interruptions may leave an
  # intentionally mutated file, which is reported rather than hidden.
  debug=base/'debug.log'
  if debug.exists():
   matches=re.findall(r'temp_dir="([^"]+)"',debug.read_text())
   if matches:
    copy=pathlib.Path(matches[0]);files=subprocess.check_output(['git','ls-tree','-r','--name-only',receipt['pin'],'--','crates/brn-ai','crates/brn-threads-core','crates/brn-threads-app','crates/brn','crates/brn-desktop'],cwd=root,text=True).splitlines();proof={}
    for file in files:
     if file.endswith('.rs'):
      raw=copy/file;expected=hashlib.sha256(subprocess.check_output(['git','show',receipt['pin']+':'+file],cwd=root)).hexdigest();actual=hashlib.sha256(raw.read_bytes()).hexdigest() if raw.exists() else None
      proof[file]={'expected':expected,'actual':actual,'match':actual==expected}
    p['copy_identity']={'path':str(copy),'files':len(proof),'matches':sum(v['match'] for v in proof.values()),'mismatches':{k:v for k,v in proof.items() if not v['match']}}
  campaign['phases'].append(p)
 result['campaigns'].append(campaign)
result['total_shared_seconds']=receipt.get('total_shared_seconds',receipt.get('elapsed_seconds'))
(evidence/'mutation-outcomes.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'campaigns':[(c['pin'],[(p['name'],p.get('counts'),len(p.get('unexecuted_ids',[]))) for p in c['phases']]) for c in result['campaigns']],'total_shared_seconds':result['total_shared_seconds']},indent=2))
