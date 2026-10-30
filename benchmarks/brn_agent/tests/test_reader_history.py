import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace as NS
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
import evaluate_wiki_v2 as v2
from evidence import collect, verified_proof_packet
from reader import run, packet_hash
from external import digest, native_fixture, stage_history


def history_fixture():
    cases = [{'question_id':'unit_abs', 'question_type':'unit', 'question':'What is the current target?',
              'question_date':'2026-02-03','answer':'GOLD_ONLY_CANARY','answer_session_ids':['new'],
              'haystack_session_ids':['old','new'],'haystack_dates':['2026-01-01','2026-02-01'],
              'haystack_sessions':[[{'role':'user','content':'Target April 1.','has_answer':False}],
                                   [{'role':'user','content':'Target revised to May 1.','has_answer':True}]]}]
    admission = {'dataset':'xiaowu0162/longmemeval-cleaned','revision':'synthetic-only',
                 'license_text_sha256':'unit-license','provenance':'newly synthetic unit witness',
                 'reviewed_by':'test author','access_date':'2026-10-28','allowed_use':'approved',
                 'variant':'s','raw_sha256':digest(cases)}
    return cases, admission


class NativeMappingTests(unittest.TestCase):
    def test_native_fixture_preserves_order_and_hides_gold(self):
        cases, admission = history_fixture()
        fixture, gold, locks = native_fixture(cases, admission)
        case = fixture['cases'][0]
        self.assertEqual([s['kind'] for s in case['steps']], ['intake','intake','ask'])
        self.assertIn('2026-01-01', case['steps'][0]['text'])
        self.assertIn('2026-02-01', case['steps'][1]['text'])
        for hidden in ('GOLD_ONLY_CANARY','has_answer','answer_session_ids','unit_abs'):
            self.assertNotIn(hidden, json.dumps(fixture))
        self.assertEqual(v2.validate_case(case), [])
        self.assertEqual(gold[0]['answer'], 'GOLD_ONLY_CANARY')
        self.assertEqual(locks[0]['native_fixture_sha256'], digest(fixture))

    @unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_TEST_BINARY'), 'explicit native binary required')
    def test_native_full_intake_is_sequential_and_gold_free(self):
        cases, admission = history_fixture()
        fixture, _, _ = native_fixture(cases, admission)
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'stage'
            result = stage_history(cases, admission, fixture['cases'][0]['id'],
                                   os.environ['BRN_AGENTBENCH_TEST_BINARY'], output)
            self.assertEqual(result['status'], 'complete')
            self.assertFalse(result['candidate_executed'])
            first = (output / 'checkpoints/1/records.json').read_text()
            second = (output / 'checkpoints/2/records.json').read_text()
            self.assertIn('April 1', first)
            self.assertNotIn('May 1', first)
            self.assertIn('May 1', second)
            self.assertNotIn('GOLD_ONLY_CANARY', second)
            self.assertNotIn('has_answer', second)
            self.assertEqual(len(result['events']), 2)
            lock = json.loads((output/'grader-only/dataset-lock.json').read_text())[0]
            self.assertEqual(lock['revision'], 'synthetic-only')
            self.assertEqual(lock['admission']['reviewed_by'], 'test author')
            self.assertNotIn('GOLD_ONLY_CANARY', json.dumps(result))
            with self.assertRaises(FileExistsError):
                stage_history(cases, admission, fixture['cases'][0]['id'],
                              os.environ['BRN_AGENTBENCH_TEST_BINARY'], output)


@unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_TEST_BINARY') and os.environ.get('BRN_AGENTBENCH_READER_BINARY'),
                     'explicit native CLI and controlled reader binaries required')
class ReaderExecutionTests(unittest.TestCase):
    def test_wiki_only_reader_uses_clone_and_never_sees_raw_source_canary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'experiment'
            v2.create_root(root)
            binary = os.environ['BRN_AGENTBENCH_TEST_BINARY']
            arm = {'name':'A','binary':binary,'model_dir':None,'provider':None,'model':None,'effort':None,'binary_sha256':'unit'}
            sample = v2.Sample(NS(helper=None,stock_helper=None,brn_helper=None,timeout=30,measure_bin=None), root,
                              {'id':'UNIT','checks':[]},0,{'arm':'A','repetition':1,'label':None},arm,root,{}, {'trace':True})
            sample.folder.mkdir(parents=True)
            sample.cli = v2.make_cli(arm,binary,sample.folder/'data',root,sample.args)
            sample.cli.call('note-create',title='Raw source',text='RAW_SOURCE_READER_CANARY')
            sample.cli.call('note-create',title='Wiki summary',text='Current target is May 1.')
            before = sample.cli.call('records')
            wiki = next(r for r in before if r['data'].get('Note',{}).get('title')=='Wiki summary')
            sample.checkpoint(0)
            (root/'manifest.json').write_text('{"fixture_sha256":"unit-fixture"}')
            suite = root/'suite.json'
            suite.write_text('{"fixture_sha256":"unit-fixture"}')
            (root/'agentbench-evidence.json').write_text(json.dumps(collect(root,suite)))
            checkpoint = sample.folder/'checkpoints/0'
            reads = [{'id':wiki['id'],'version':wiki['version']}]
            packet = verified_proof_packet(checkpoint,wiki_records=reads)
            fixture = {'mode':'wiki-only','question':'What is the current target?','records':reads,
                       'controlled_answer':'The saved wiki says May 1.','packet_sha256':packet_hash(packet)}
            result = run(checkpoint,fixture,binary,os.environ['BRN_AGENTBENCH_READER_BINARY'],Path(tmp)/'probe')
            self.assertEqual(result['status'],'complete')
            self.assertFalse(result['qualified'])
            self.assertNotIn('RAW_SOURCE_READER_CANARY', json.dumps(result['observed']['model_visible_history']))
            self.assertIn('May 1', json.dumps(result['observed']['model_visible_history']))
            self.assertEqual(sample.cli.call('records'), before)
            source = next(r for r in before if r['data'].get('Note',{}).get('title')=='Raw source')
            normal_reads = reads + [{'id':source['id'],'version':source['version']}]
            normal_packet = verified_proof_packet(checkpoint)
            normal_fixture = dict(fixture,mode='normal-product',records=normal_reads,packet_sha256=packet_hash(normal_packet))
            normal = run(checkpoint,normal_fixture,binary,os.environ['BRN_AGENTBENCH_READER_BINARY'],Path(tmp)/'normal')
            self.assertIn('RAW_SOURCE_READER_CANARY', json.dumps(normal['observed']['model_visible_history']))
            self.assertEqual(sample.cli.call('records'), before)
            bad = dict(fixture,packet_sha256='wrong')
            with self.assertRaises(ValueError):
                run(checkpoint,bad,binary,os.environ['BRN_AGENTBENCH_READER_BINARY'],Path(tmp)/'bad')
            self.assertFalse((Path(tmp)/'bad').exists())

@unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_RECOVERY_BINARY'), 'explicit controlled recovery binary required')
class RecoveryExecutionTests(unittest.TestCase):
    def test_retained_recovery_receipts_and_undo_copy(self):
        import subprocess
        from score import sha256
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)/'recovery'
            command = [os.environ['BRN_AGENTBENCH_RECOVERY_BINARY'],str(output)]
            subprocess.run(command,check=True,capture_output=True,text=True,timeout=30)
            result = json.loads((output/'recovery-result.json').read_text())
            self.assertEqual(result['status'],'complete')
            self.assertFalse(result['qualified'])
            self.assertNotEqual(result['abandoned_run'],result['retry_run'])
            self.assertNotEqual(result['retained_candidate'],result['replacement_candidate'])
            self.assertEqual(len(result['checkpoints']),7)
            for checkpoint in result['checkpoints']:
                folder = output/'checkpoints'/str(checkpoint['step'])
                for name,field in [('workspace.sqlite3','backup_sha256'),('records.json','records_sha256'),
                                   ('receipts.json','receipts_sha256'),('candidates.json','candidates_sha256')]:
                    self.assertEqual(sha256(folder/name),checkpoint[field])
            self.assertEqual((output/'checkpoints/1/candidates.json').read_bytes(),
                             (output/'checkpoints/3/candidates.json').read_bytes())
            current = json.loads((output/'checkpoints/6/records.json').read_text())
            undone = json.loads((output/'undo-copy-records.json').read_text())
            def note(records):
                return next(r['data']['Note']['markdown'] for r in records
                            if r['data'].get('Note',{}).get('title')=='Recovery witness')
            self.assertIn('Confirmed',note(current))
            self.assertIn('Pending',note(undone))
            self.assertIn('scope is provisional',note(current))
            self.assertIn('scope is provisional',note(undone))
            original = (output/'recovery-result.json').read_bytes()
            refused = subprocess.run(command,capture_output=True,text=True,timeout=30)
            self.assertNotEqual(refused.returncode,0)
            self.assertEqual((output/'recovery-result.json').read_bytes(),original)
