"""Opt-in native offline witnesses; never configure or call a model."""
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace as NS
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(ROOT / 'benchmarks/brn_agent'))
import evaluate_wiki_v2 as v2
from evidence import grading_packet


@unittest.skipUnless(os.environ.get('BRN_AGENTBENCH_TEST_BINARY'), 'explicit native binary required')
class NativeEvidenceTests(unittest.TestCase):
    def test_backup_and_undo_probe_leave_original_unchanged(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve() / 'experiment'
            v2.create_root(root)
            arm = {'name': 'A', 'binary': os.environ['BRN_AGENTBENCH_TEST_BINARY'],
                   'model_dir': None, 'provider': None, 'model': None, 'effort': None, 'binary_sha256': 'offline-test'}
            args = NS(helper=None, stock_helper=None, brn_helper=None, timeout=30, measure_bin=None)
            case = {'id': 'N', 'checks': [{'type': 'undo'}]}
            sample = v2.Sample(args, root, case, 0, {'arm': 'A', 'repetition': 1, 'label': None}, arm, root, {}, {'trace': True})
            sample.folder.mkdir(parents=True)
            sample.cli = v2.make_cli(arm, arm['binary'], sample.folder / 'data', root, args)
            sample.cli.call('note-create', title='Synthetic witness', text='Keep these exact bytes: café.')
            before = sample.cli.call('records')
            history = sample.cli.call('history')
            receipts = [r for r in history if any('Note' in w['after']['data'] for w in r['writes'])]
            operations = [r['operation'] for r in receipts]
            sample.checkpoint(0)
            checkpoint = sample.folder / 'checkpoints/0'
            frozen = v2.sha256_file(checkpoint / 'workspace.sqlite3')
            outcome = sample.run_undo(case, [{'operations': operations}], receipts, before)
            self.assertEqual(outcome[None]['status'], 'Applied')
            self.assertTrue(outcome[None]['affected_restored'])
            self.assertTrue(outcome[None]['unrelated_preserved'])
            self.assertEqual(sample.cli.call('records'), before)
            self.assertEqual(v2.sha256_file(checkpoint / 'workspace.sqlite3'), frozen)
            packet = grading_packet(checkpoint, wiki_only=True, wiki_records=[
                {'id': r['id'], 'version': r['version']} for r in before if 'Note' in r['data']])
            self.assertTrue(any('Note' in r['data'] for r in packet['records']))
            self.assertFalse(any('Settings' in r['data'] or 'Run' in r['data'] for r in packet['records']))
            with self.assertRaises(FileExistsError):
                sample.checkpoint(0)
            self.assertEqual(json.loads((sample.folder / 'result.json').read_text())['checkpoints'][0]['backup_sha256'], frozen)

    def test_controlled_owner_edit_stale_refusal_and_undo_use_native_guards(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve() / 'experiment'
            v2.create_root(root)
            arm = {'name': 'A', 'binary': os.environ['BRN_AGENTBENCH_TEST_BINARY'],
                   'model_dir': None, 'provider': None, 'model': None, 'effort': None, 'binary_sha256': 'offline-test'}
            args = NS(helper=None, stock_helper=None, brn_helper=None, timeout=30, measure_bin=None)
            case = {'id': 'N', 'checks': []}
            sample = v2.Sample(args, root, case, 0, {'arm': 'A', 'repetition': 1, 'label': None}, arm, root, {}, {'trace': True})
            sample.folder.mkdir(parents=True)
            sample.cli = v2.make_cli(arm, arm['binary'], sample.folder / 'data', root, args)
            note = sample.cli.call('note-create', title='Synthetic original', text='Original café.')
            sample.seed = {'notes': {'note': note['id']}, 'versions': {'note': note['version']}}
            records = []
            sample.result['steps'] = records
            threads = {}
            for number, event in enumerate([
                {'kind': 'owner-edit', 'key': 'note', 'text': 'Owner revision.'},
                {'kind': 'owner-edit', 'key': 'note', 'base': 'seed', 'text': 'Must not overwrite.'},
                {'kind': 'owner-undo', 'from_step': 1, 'operation_index': 0},
                {'kind': 'owner-discard', 'from_step': 2},
                {'kind': 'owner-undo', 'from_step': 1, 'operation_index': 0},
                {'kind': 'owner-thread', 'title': 'Fresh synthetic conversation', 'as': 'followup'},
            ], 1):
                record = {'operations': []}
                records.append(record)
                result = sample.run_owner_event(number, event, threads, record)
                if number in (2, 3):
                    self.assertFalse(result['ok'])
                    self.assertEqual(sample.cli.call('read', id=note['id'])['data']['Note']['markdown'], 'Owner revision.')
                else:
                    self.assertTrue(result['ok'], result)
            self.assertEqual(sample.cli.call('read', id=note['id'])['data']['Note']['markdown'], 'Original café.')
            self.assertIn('followup', threads)
            self.assertFalse(list((sample.folder / 'owner-events').glob('*.token')))
            self.assertNotIn('token', json.dumps([r['host_events'] for r in records]))


if __name__ == '__main__':
    unittest.main()
