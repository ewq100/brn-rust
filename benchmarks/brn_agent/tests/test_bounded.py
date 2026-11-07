import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT/'benchmarks/brn_agent'))
sys.path.insert(0, str(ROOT/'scripts'))
import calibration
import check_status
import evidence
import evaluate_wiki_v2 as v2
import freeze
import proof
import reader
import reporting
import wiki_challenge
from score import assess, sha256


class BoundedTests(unittest.TestCase):
    def test_zero_inapplicable_report_only_unknown_and_failed_checks(self):
        for checks in ([], [dict(passed=True, applicable=False)], [dict(passed=True, report_only=True)]):
            self.assertEqual(check_status.summarize(checks)['display'], 'not evaluated')
        checks = [dict(index=0, passed=True, applicable=True, report_only=False)]
        self.assertIsNone(v2.case_passed(checks, excluded={0}))
        self.assertIsNone(v2.case_passed([]))
        self.assertEqual(check_status.summarize([dict(passed=None)])['status'], 'unverified')
        self.assertEqual(check_status.summarize([dict(passed=None),dict(passed=False)])['status'], 'failed')
        result = dict(case='UNIT', scale=0, arm='A', slot='A-r1', repetition=1, attempt=1,
                      status='complete', safety=[], metrics={}, checks=[], planned=1)
        report = v2.build_report([result], [dict(id='UNIT', repetitions=1)])
        self.assertIsNone(report['cases'][0]['case_passed'])
        self.assertIn('not evaluated', v2.format_report(report))

    def test_six_distinct_histories_native_schema_and_baseline_preserved(self):
        before = proof.build()
        fixture, suite, gold = wiki_challenge.build()
        self.assertEqual(v2.validate_fixture(fixture), [])
        self.assertEqual([c['id'] for c in fixture['cases']], ['W01','W02','W03','W04','W05','W06'])
        self.assertEqual(len({c['world'] for c in suite['scenarios']}), 6)
        self.assertEqual(before, proof.build())
        screening,screen_suite,screen_gold=wiki_challenge.build(screening=True)
        self.assertEqual(v2.validate_fixture(screening),[])
        self.assertEqual([c['id'] for c in screening['cases']],['S01','S02'])
        self.assertFalse({c['world'] for c in suite['scenarios']} & {c['world'] for c in screen_suite['scenarios']})
        self.assertEqual(screen_suite['qualification'],'visible-screening')
        self.assertEqual(len(suite['scenarios'][0]['criteria']), 6)
        self.assertNotIn('reader_questions', json.dumps(fixture))
        self.assertNotIn('2026-04-27', json.dumps(gold['checkpoints']['W06']['1']))
        old = gold['checkpoints']['W04']['1']
        new = gold['checkpoints']['W04']['3']
        self.assertIn('2027-06-12', str(old['facts']))
        self.assertIn('2027-06-19', str(new['facts']))
        self.assertNotIn('40', str(new['facts']))
        self.assertEqual(len(new['superseded_facts']), 1)
        duplicate = gold['checkpoints']['W03']
        self.assertEqual(duplicate['1']['facts'], duplicate['3']['facts'])
        self.assertEqual(len(duplicate['4']['facts']), len(duplicate['3']['facts'])+1)
        self.assertTrue(duplicate['3']['task_contract']['allow_no_change'])
        self.assertIn('mg/L', str(gold['checkpoints']['W05']['2']['facts']))
        self.assertIn('unknown', json.dumps(gold['reader_questions']['W06']).lower())
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'challenge'
            wiki_challenge.write(path)
            self.assertEqual(sha256(path/'fixture.json'), suite['fixture_sha256'])
            self.assertEqual(sha256(path/'gold.json'), suite['gold_sha256'])

    def test_unmeasured_usefulness_cannot_become_a_score(self):
        _, suite, _ = wiki_challenge.build()
        attempts = [dict(id=c['id'], result_sha256=c['id'], scenario=c['id'],
                         configuration='A', repetition=1, status='complete') for c in suite['scenarios']]
        grades = [dict(attempt=c['id'], result_sha256=c['id'], grader_revision='independent-test',
                       completed=True, breaches=[], criteria={n:dict(score=4, evidence=[{'step':1}],
                       explanation='Synthetic arithmetic witness') for n in c['criteria']}) for c in suite['scenarios']]
        report = assess(suite,'A',attempts,grades)
        self.assertIsNone(report['overall_quality'])
        self.assertIsNone(report['challenge_quality'])
        self.assertIsNone(report['reviewed_quality']['dimensions']['downstream_usefulness'])
        self.assertEqual(report['reviewed_quality']['dimensions']['correctness'],100)
        self.assertFalse(report['ranked'])
        self.assertEqual(len(report['missing']),6)
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp)/'controlled.json'
            p.write_text(json.dumps(dict(kind='controlled-fixture',status='complete',measured_answer=True)))
            with self.assertRaisesRegex(ValueError,'real wiki-only reader'):
                reader.verify_measurement(dict(path=p.name,sha256=sha256(p)),Path(temp),'checkpoint')
        # Only the adapter may set this after verifying actual bound reader receipts.
        for grade in grades:
            grade['reader_evidence_verified'] = True
        report = assess(suite,'A',attempts,grades)
        self.assertEqual(report['challenge_quality'],100)
        self.assertIsNone(report['overall_quality'])
        self.assertFalse(report['ranked'])
        report['evaluation_identity'] = 'synthetic-arithmetic-witness'
        other = dict(report,configuration='B')
        pair = reporting.compare(suite,[report,other],bootstrap_samples=100)['comparisons'][0]
        self.assertEqual(pair['overall_delta'],0)
        other['scoring_revision'] = 'old'
        with self.assertRaisesRegex(ValueError,'scoring revisions'):
            reporting.compare(suite,[report,other])

    def test_sources_alone_do_not_satisfy_articles_but_valid_no_change_does(self):
        import scenario_checks
        gold=wiki_challenge.build()[2]['checkpoints']['W03']['3']
        text='\n'.join(f['assertion'] for f in gold['facts'])
        raw=dict(id='raw',version=1,data={'Note':{'markdown':text}})
        other=dict(id='unrelated',version=1,data={'Note':{'markdown':gold['unrelated_fact']['assertion']}})
        def claims(identity):
            return [dict(fact_id=f['id'],value=f['value'],status=f['status'],record=identity,version=1,quote=f['assertion']) for f in gold['facts']]
        result=scenario_checks.assess(gold,[raw,other],claims('raw'),baseline=[raw,other],source_bindings={'brief':'raw','unrelated':'unrelated'})
        self.assertFalse(result['passed'])
        self.assertEqual(result['retained_facts'],0)
        wiki=dict(id='wiki',version=1,data={'Note':{'markdown':text}})
        link=dict(id='link',version=1,data={'Link':{'from':'wiki','to':'raw','relation':'source'}})
        rows=[raw,other,wiki,link]
        result=scenario_checks.assess(gold,rows,claims('wiki'),baseline=rows,source_bindings={'brief':'raw','unrelated':'unrelated'})
        self.assertTrue(result['passed'])
        self.assertTrue(result['saved_requirement_met'])

    def test_freeze_detects_changed_evaluator_candidate_and_retrieval(self):
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp)/'source'; p.write_text('frozen')
            spec=dict(evaluation_files={'rubric':str(p)}, candidate_files={'app':str(p)},
                labels=dict(app_source_revision='test',benchmark_revision='test',build_source_binding='unverified'),
                settings=dict(candidate={},reader={},grader={},retrieval={}),isolation='visible-screening')
            receipt=freeze.freeze(spec)
            self.assertFalse(receipt['isolation_attested'])
            self.assertEqual(freeze.verify(spec,receipt)['status'],'verified')
            p.write_text('changed')
            with self.assertRaises(ValueError): freeze.verify(spec,receipt)
            p.write_text('frozen')
            changed=copy.deepcopy(spec);changed['settings']['retrieval']['top_k']=10
            with self.assertRaises(ValueError): freeze.verify(changed,receipt)
            changed=copy.deepcopy(spec);changed['evaluation_files']['auth.json']=str(p)
            with self.assertRaises(ValueError):freeze.freeze(changed)

    def test_retained_pilot_and_observed_calibration_are_bound_and_not_qualified(self):
        folder=ROOT/'benchmarks/brn_agent/tests/fixtures/bounded-evidence'
        packets=json.loads((folder/'pilot003-packets.json').read_text())
        review=json.loads((folder/'pilot003-review.json').read_text())
        self.assertEqual(sha256(folder/'pilot003-packets.json'),review['packet_file_sha256'])
        self.assertEqual(len(packets),16)
        self.assertEqual({p['id'] for p in packets},{p['id'] for p in review['packets']})
        for packet in packets:
            self.assertEqual(reader.packet_hash(packet['packet']),packet['packet_sha256'])
            row=next(r for r in review['packets'] if r['id']==packet['id'])
            for ref in row['evidence']:
                if 'record' in ref:
                    self.assertTrue(any(r['id']==ref['record'] and r['version']==ref['version'] for r in packet['packet']['records']))
        labels=json.loads((ROOT/'benchmarks/brn_agent/wiki_challenge/calibration-labels.json').read_text())
        rubric=json.loads((ROOT/'benchmarks/brn_agent/wiki_challenge/defect-rubric.json').read_text())
        public,key=calibration.packets(labels,rubric)
        judgments=json.loads((folder/'calibration-judgments.json').read_text())
        result=calibration.assess(key,judgments)
        self.assertEqual(result['observed_judgments'],34)
        self.assertEqual(result['defects'],dict(true_positive=30,false_positive=0,false_negative=0))
        self.assertFalse(result['qualified'])

    def test_macos_alias_does_not_reveal_arm_in_anonymous_packet(self):
        packet={'locator':'/var/folders/example/runs/SECRET-r1/file.md'}
        cleaned=evidence.anonymize_local_paths(packet,Path('/private/var/folders/example'))
        self.assertNotIn('SECRET',str(cleaned))


class NativeReaderPreparationTests(unittest.TestCase):
    @unittest.skipUnless(__import__('os').environ.get('BRN_AGENTBENCH_TEST_BINARY'), 'explicit native CLI required')
    def test_real_retrieval_and_wiki_scope_without_any_model(self):
        import os
        from types import SimpleNamespace as NS
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp).resolve()/'experiment';v2.create_root(root)
            binary=Path(os.environ['BRN_AGENTBENCH_TEST_BINARY'])
            args=NS(helper=None,stock_helper=None,brn_helper=None,timeout=30)
            cli=v2.make_cli({'model_dir':None},binary,root/'source/data',root,args)
            note=cli.call('note-create',title='Maintained program wiki',text='Moss program has 30 public places. Ramp closed April 27; use south lift.')
            cli.call('note-create',title='Raw uncompiled source',text='EXCLUDED_CANARY_FENNEC budget EUR 9981.')
            backup=root/'workspace.sqlite3';cli.call('backup',destination=backup)
            selected=[dict(id=note['id'],version=note['version'])]
            visible={}
            for mode in ('wiki-only','normal-product'):
                clone=v2.make_cli({'model_dir':None},binary,root/mode/'data',root,args)
                thread,before,probe=reader.prepare_native_reader(clone,backup,mode,selected,'EXCLUDED_CANARY_FENNEC','Read only the wiki.')
                visible[mode]=probe['hits']
                self.assertTrue(thread)
                self.assertTrue(before)
                self.assertEqual(cli.call('read',id=note['id']),note)
                self.assertFalse(clone.call('records') == [])
            self.assertEqual(visible['wiki-only'],[])
            self.assertTrue(visible['normal-product'])


if __name__ == '__main__':
    unittest.main()
