"""Deterministic checks over frozen records and explicitly adjudicated claims.

This module does NOT extract meaning from prose. An independent reviewer (or an
explicit controlled provider fixture) supplies canonical claim annotations with
exact record/version/quote evidence. Exact quotes prove provenance, not entailment;
semantic correctness of that annotation still needs independent qualification.
The output remains unqualified even when every deterministic invariant passes.
"""
import collections


def assess(gold, records, annotations, *, baseline=None, source_bindings=None, receipts=(),
           claimed_operations=(), adjudication_kind='controlled-fixture'):
    if adjudication_kind not in ('controlled-fixture', 'independent-review'):
        raise ValueError('Claim annotations must declare their independent or controlled origin')
    indexed={r['id']:r for r in records}
    if len(indexed)!=len(records): raise ValueError('Duplicate record identity')
    before={r['id']:r for r in (baseline or [])}
    expected={f['id']:f for f in gold['facts']}
    required=set(gold['required_fact_ids'])
    if not required <= set(expected): raise ValueError('Required truth is not available at this checkpoint')
    source_bindings=source_bindings or {}
    links=collections.Counter((r['data']['Link']['from'],r['data']['Link']['to'],r['data']['Link']['relation'])
                              for r in records if 'Link' in r['data'])
    supported,stale,unsupported,missing_links=set(),[],[],[]
    seen=set()
    for claim in annotations:
        if set(claim)!={'fact_id','value','status','record','version','quote'}:
            raise ValueError('Claim annotation must use the exact evidence schema')
        key=(claim['fact_id'],claim['record'])
        if key in seen: raise ValueError('Duplicate adjudicated claim in a record')
        seen.add(key)
        record=indexed.get(claim['record'])
        if record is None or record['version']!=claim['version'] or type(claim['version']) is not int:
            raise ValueError('Claim record/version does not exist in the frozen checkpoint')
        text=record['data'].get('Note',{}).get('markdown')
        if not isinstance(text,str) or not isinstance(claim['quote'],str) or not claim['quote'].strip() or claim['quote'] not in text:
            raise ValueError('Claim evidence must be an exact nonempty quotation from a saved note')
        if gold['task_contract'].get('exclude_raw_sources_from_artifacts') and claim['record'] in source_bindings.values():
            unsupported.append(claim['fact_id'])
            continue
        truth=expected.get(claim['fact_id'])
        correct=truth is not None and type(claim['value']) is type(truth['value']) and claim['value']==truth['value'] and claim['status']==truth['status']
        if not correct:
            unsupported.append(claim['fact_id'])
            historical=gold.get('superseded_facts',[])
            if claim['status'] in ('current','confirmed') and any(f['id']==claim['fact_id'] and f['value']==claim['value'] for f in historical):
                stale.append(claim['fact_id'])
            continue
        supported.add(claim['fact_id'])
        if gold['task_contract'].get('source_links_required'):
            sources=[source_bindings[k] for k in truth['source_keys'] if k in source_bindings and source_bindings[k] in indexed and not indexed[source_bindings[k]].get('archived',False)]
            # A source note itself need not link to itself; derived/saved wiki
            # claims must link to at least one actually bound supporting source.
            if not sources or not any(s==record['id'] or any(a==record['id'] and b==s and relation=='source' for a,b,relation in links) for s in sources):
                missing_links.append(claim['fact_id'])
    changed_notes=[r['id'] for r in records if 'Note' in r['data'] and
                   (r['id'] not in before or r['data']['Note'].get('markdown')!=before[r['id']]['data'].get('Note',{}).get('markdown'))]
    contract=gold['task_contract']
    saved_requirement_met=(True if not contract.get('require_saved') or contract.get('allow_no_change') else
                           None if baseline is None else bool(changed_notes))
    unrelated=source_bindings.get('unrelated')
    unrelated_preserved=(True if not contract.get('preserve_unrelated') else
                         None if unrelated is None or unrelated not in before else indexed.get(unrelated)==before[unrelated])
    operations={r['operation'] for r in receipts if isinstance(r,dict) and r.get('operation')}
    false_completion=sorted(set(claimed_operations)-operations)
    missing=sorted(required-supported)
    duplicate_links=sum(max(0,n-1) for n in links.values())
    unknown=[]
    if saved_requirement_met is None: unknown.append('saved-state baseline missing')
    if unrelated_preserved is None: unknown.append('unrelated record baseline/binding missing')
    passed=not(missing or unsupported or missing_links or false_completion or duplicate_links or unknown) and saved_requirement_met is True and unrelated_preserved is True
    return {'kind':'adjudicated-claim-invariants','adjudication_kind':adjudication_kind,
            'qualified':False,'passed':passed,
            'required_facts':len(required),'retained_facts':len(required & supported),
            'required_fact_retention':len(required & supported)/len(required) if required else None,
            'missing_facts':missing,'unsupported_material_claims':unsupported,'stale_current_claims':stale,
            'missing_source_links':missing_links,'duplicate_relationships':duplicate_links,
            'false_completion_operations':false_completion,'saved_requirement_met':saved_requirement_met,'incomplete_evidence':unknown,
            'unrelated_preserved':unrelated_preserved,
            'limitation':'Deterministic checks validate adjudicated claim values and references, not the semantic entailment of annotations or prose. Independent qualification remains required.'}
