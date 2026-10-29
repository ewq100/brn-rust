import copy
import collections
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'benchmarks/brn_agent'))
import proof
import scenario_checks


def record(identity,text,version=1):
    return {'id':identity,'version':version,'data':{'Note':{'markdown':text}}}


def witness(gold,split=False):
    """Hand-controlled semantic annotations; never pretend these are NLP extraction."""
    rows=[record('unrelated',gold['unrelated_fact']['assertion'])];claims=[];sources={'unrelated':'unrelated'}
    for f in gold['facts']:
        for source in f['source_keys']:
            sources[source]='source:'+source
    rows.extend(record(identity,'Source evidence') for identity in set(sources.values()) if identity!='unrelated')
    facts=[f for f in gold['facts'] if f['id'] in gold['required_fact_ids']]
    if split:
        for f in facts:
            identity='wiki:'+f['id'];text='Useful alternate structure: '+f['assertion']
            rows.append(record(identity,text))
            claims.append(dict(fact_id=f['id'],value=f['value'],status=f['status'],record=identity,version=1,quote=f['assertion']))
    else:
        rows.append(record('wiki','\n'.join(f['assertion'] for f in facts)))
        claims=[dict(fact_id=f['id'],value=f['value'],status=f['status'],record='wiki',version=1,quote=f['assertion']) for f in facts]
    linked=set()
    for i,c in enumerate(claims):
        f=next(f for f in facts if f['id']==c['fact_id'])
        edge=(c['record'],sources[f['source_keys'][0]])
        if edge in linked: continue
        linked.add(edge)
        rows.append({'id':f'link:{i}','version':1,'data':{'Link':{'from':edge[0],'to':edge[1],'relation':'source'}}})
    return rows,claims,sources


def baseline(rows):
    return [r for r in rows if r['id']=='unrelated' or r['id'].startswith('source:')]


class ScenarioBankTests(unittest.TestCase):
    def test_bank_counts_native_schema_and_versioned_hashes(self):
        spec=importlib.util.spec_from_file_location('native_eval',ROOT/'scripts/evaluate_wiki_v2.py')
        native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
        fixture,suite,gold=proof.build()
        self.assertEqual(native.validate_fixture(fixture),[])
        focused=[c for c in fixture['cases'] if c['id'].startswith('F')]
        self.assertEqual(collections.Counter(c['category'] for c in focused),{c:3 for c in proof.WEIGHTS})
        journeys=[c for c in fixture['cases'] if c['id'].startswith('J')]
        self.assertEqual([len(c['steps']) for c in journeys],[12,12,12])
        self.assertEqual(len({c['world'] for c in suite['scenarios'] if c['id'].startswith('J')}),3)
        self.assertEqual(len(gold['checkpoints']),27)
        self.assertEqual(proof.fixture_revision(suite['fixture_sha256']),proof.REVISION)
        self.assertEqual(proof.fixture_revision(proof.legacy_proof.build()[1]['fixture_sha256']),'legacy-proof-v1')
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)/'bank';proof.write(output)
            from score import sha256
            self.assertEqual(json.loads((output/'suite.json').read_text())['gold_sha256'],sha256(output/'gold.json'))
            self.assertEqual(suite['fixture_sha256'],sha256(output/'fixture.json'))

    def test_each_world_checkpoint_hides_future_and_retains_supersession(self):
        for i,(world_id,world) in enumerate(proof.WORLDS.items(),1):
            scenario=f'J{i:02}'
            first=proof.checkpoint_gold(scenario,1)
            self.assertNotIn(world['new_target'],json.dumps(first))
            self.assertNotIn(world['proposal']['value'],json.dumps(first))
            self.assertNotIn(world['deferred_fact']['value'],json.dumps(first))
            second=proof.checkpoint_gold(scenario,2)
            self.assertEqual(next(f['value'] for f in second['facts'] if f['id']=='target'),world['new_target'])
            sixth=proof.checkpoint_gold(scenario,6)
            estimate=next(f for f in sixth['facts'] if f['id']==world['proposal']['id'])
            self.assertEqual(estimate['status'],'unconfirmed')
            tenth=proof.checkpoint_gold(scenario,10)
            estimate=next(f for f in tenth['facts'] if f['id']==world['proposal']['id'])
            self.assertEqual(estimate['status'],'confirmed')
            self.assertEqual(len(tenth['known_owner_events']),10)
            self.assertTrue(any(f['status']=='superseded' for f in tenth['superseded_facts']))
            with self.assertRaises(ValueError):proof.checkpoint_gold(scenario,13)
        self.assertEqual([proof.derived_fact(w)['value'] for w in proof.WORLDS],[9600,36,22])

    def test_good_bad_and_alternative_valid_every_scenario(self):
        fixture,_,_=proof.build()
        for case in fixture['cases']:
            gold=proof.checkpoint_gold(case['id'],len(case['steps']))
            for split in (False,True):
                rows,claims,sources=witness(gold,split)
                result=scenario_checks.assess(gold,rows,claims,baseline=baseline(rows),source_bindings=sources)
                self.assertTrue(result['passed'],(case['id'],result))
                self.assertFalse(result['qualified'])
                # A perfectly titled saved document without the required meaning
                # must not pass merely because a note exists.
                bad=scenario_checks.assess(gold,[record('wiki',case['name'])],[],source_bindings=sources)
                self.assertFalse(bad['passed'])
                self.assertTrue(bad['missing_facts'])
                corrupted=copy.deepcopy(rows)
                dropped=claims[0]
                for row in corrupted:
                    if row['id']==dropped['record']:
                        row['data']['Note']['markdown']=row['data']['Note']['markdown'].replace(dropped['quote'],'')
                remaining=[c for c in claims if c is not dropped]
                loss=scenario_checks.assess(gold,corrupted,remaining,baseline=baseline(rows),source_bindings=sources)
                self.assertIn(dropped['fact_id'],loss['missing_facts'])
                # An invented canonical value still fails even when its exact
                # quote exists; source binding alone does not establish truth.
                mutated=copy.deepcopy(claims);mutated[0]['value']='invented value'
                self.assertFalse(scenario_checks.assess(gold,rows,mutated,source_bindings=sources)['passed'])

    def test_specific_mutations_fail_the_invariant_they_name(self):
        gold=proof.checkpoint_gold('J01',12)
        rows,claims,sources=witness(gold)
        # Lost exception and false current date.
        missing=[c for c in claims if c['fact_id']!='exception']
        corrupted=copy.deepcopy(rows)
        exception=next(c for c in claims if c['fact_id']=='exception')
        next(r for r in corrupted if r['id']==exception['record'])['data']['Note']['markdown']=next(r for r in corrupted if r['id']==exception['record'])['data']['Note']['markdown'].replace(exception['quote'],'')
        self.assertIn('exception',scenario_checks.assess(gold,corrupted,missing,baseline=baseline(rows),source_bindings=sources)['missing_facts'])
        lost_qualifier=copy.deepcopy(claims)
        next(c for c in lost_qualifier if c['fact_id']=='exception')['value'].pop('dry_rooms')
        self.assertIn('exception',scenario_checks.assess(gold,rows,lost_qualifier,baseline=baseline(rows),source_bindings=sources)['unsupported_material_claims'])
        stale=copy.deepcopy(claims)
        next(c for c in stale if c['fact_id']=='target')['value']='2027-02-15'
        self.assertEqual(scenario_checks.assess(gold,rows,stale,source_bindings=sources)['stale_current_claims'],['target'])
        self.assertTrue(scenario_checks.assess(gold,[r for r in rows if 'Link' not in r['data']],claims,source_bindings=sources)['missing_source_links'])
        duplicate=copy.deepcopy(next(r for r in rows if 'Link' in r['data']));duplicate['id']='duplicate-link'
        self.assertEqual(scenario_checks.assess(gold,rows+[duplicate],claims,baseline=baseline(rows),source_bindings=sources)['duplicate_relationships'],1)
        self.assertEqual(scenario_checks.assess(gold,rows,claims,source_bindings=sources,claimed_operations=['invented'])['false_completion_operations'],['invented'])
        changed=record('unrelated','Wrong new content',2);before=[record('unrelated','Keep this')]
        self.assertFalse(scenario_checks.assess(gold,[r for r in rows if r['id']!='unrelated']+[changed],claims,baseline=before,source_bindings={**sources,'unrelated':'unrelated'})['unrelated_preserved'])
        forged=copy.deepcopy(claims);forged[0]['version']=99
        with self.assertRaises(ValueError):scenario_checks.assess(gold,rows,forged,source_bindings=sources)
        forged=copy.deepcopy(claims);forged[0]['quote']='not in the saved record'
        with self.assertRaises(ValueError):scenario_checks.assess(gold,rows,forged,source_bindings=sources)

    def test_source_relations_and_required_before_state_cannot_be_invented(self):
        gold=proof.checkpoint_gold('F01',1)
        rows,claims,sources=witness(gold)
        unknown=scenario_checks.assess(gold,rows,claims,source_bindings=sources)
        self.assertFalse(unknown['passed'])
        self.assertIsNone(unknown['saved_requirement_met'])
        self.assertIsNone(unknown['unrelated_preserved'])
        unchanged=scenario_checks.assess(gold,rows,claims,baseline=rows,source_bindings=sources)
        self.assertFalse(unchanged['saved_requirement_met'])
        wrong=copy.deepcopy(rows)
        for row in wrong:
            if 'Link' in row['data']:row['data']['Link']['relation']='contradicts'
        self.assertTrue(scenario_checks.assess(gold,wrong,claims,baseline=baseline(rows),source_bindings=sources)['missing_source_links'])
        missing_source=[row for row in rows if not row['id'].startswith('source:')]
        self.assertTrue(scenario_checks.assess(gold,missing_source,claims,baseline=baseline(rows),source_bindings=sources)['missing_source_links'])

    def test_valid_paraphrase_and_correct_no_change_are_accepted(self):
        gold=proof.checkpoint_gold('F01',1)
        rows,claims,sources=witness(gold)
        paraphrases={'budget':'The approved ceiling is €12,000.',
                     'paid':'We have already paid a €2,400 deposit.',
                     'target':'Aim for 15 February 2027; there is no signed completion promise.',
                     'exception':'Waterproofing membrane is mandatory in the bathroom, whereas dry rooms require only primer.'}
        next(r for r in rows if r['id']=='wiki')['data']['Note']['markdown']='\n'.join(paraphrases.values())
        for claim in claims:claim['quote']=paraphrases[claim['fact_id']]
        self.assertTrue(scenario_checks.assess(gold,rows,claims,baseline=baseline(rows),source_bindings=sources)['passed'])
        no_change=proof.checkpoint_gold('F18',1)
        rows,claims,sources=witness(no_change)
        result=scenario_checks.assess(no_change,rows,claims,baseline=copy.deepcopy(rows),source_bindings=sources)
        self.assertTrue(result['passed'])
        self.assertTrue(result['saved_requirement_met'])


class NativeScenarioPrefixTests(unittest.TestCase):
    def test_f16_stale_edit_prefix_uses_real_host_refusal_and_preserves_owner_change(self):
        import os
        from types import SimpleNamespace as NS
        binary=os.environ.get('BRN_AGENTBENCH_TEST_BINARY')
        if not binary:self.skipTest('Explicit native binary required; no live candidate calls')
        sys.path.insert(0,str(ROOT/'scripts'))
        import evaluate_wiki_v2 as v2
        case=next(c for c in proof.build()[0]['cases'] if c['id']=='F16')
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve()/'experiment';v2.create_root(root)
            arm={'name':'A','binary':binary,'model_dir':None,'provider':None,'model':None,'effort':None,'binary_sha256':'offline-test'}
            args=NS(helper=None,stock_helper=None,brn_helper=None,timeout=30,measure_bin=None)
            sample=v2.Sample(args,root,case,0,{'arm':'A','repetition':1,'label':None},arm,root,{}, {'trace':True})
            sample.folder.mkdir(parents=True)
            sample.cli=v2.make_cli(arm,binary,sample.folder/'data',root,args)
            plan=case['seeds']['notes'][0]
            note=sample.cli.call('note-create',title=plan['title'],text=plan['text'])
            sample.seed={'notes':{'plan':note['id']},'versions':{'plan':note['version']}}
            sample.result['steps']=[]
            outcomes=[]
            for number,event in enumerate(case['steps'][:3],1):
                entry={'operations':[]};sample.result['steps'].append(entry)
                outcomes.append(sample.run_owner_event(number,event,{},entry))
            self.assertEqual([outcome['ok'] for outcome in outcomes],[True,False,True])
            current=sample.cli.call('read',id=note['id'])
            self.assertEqual(current['data']['Note']['markdown'],case['steps'][0]['text'])
            self.assertNotIn('Obsolete replacement',current['data']['Note']['markdown'])
            self.assertEqual(next(c for c in proof.build()[1]['scenarios'] if c['id']=='F16')['grade_checkpoints'],[4])
            self.assertEqual(len(sample.result['steps']),3)  # The candidate ask was not executed.
