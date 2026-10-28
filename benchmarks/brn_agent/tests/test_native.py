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
            packet = grading_packet(checkpoint, wiki_only=True)
            self.assertTrue(any('Note' in r['data'] for r in packet['records']))
            self.assertFalse(any('Settings' in r['data'] or 'Run' in r['data'] for r in packet['records']))
            with self.assertRaises(FileExistsError):
                sample.checkpoint(0)
            self.assertEqual(json.loads((sample.folder / 'result.json').read_text())['checkpoints'][0]['backup_sha256'], frozen)


if __name__ == '__main__':
    unittest.main()
