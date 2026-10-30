import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from evidence import collect
from score import assess, sha256
import proof
import semantic


class TemporalGradingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.folder = self.root/'runs/J01-0/A-r1'
        self.folder.mkdir(parents=True)
        self.suite = self.root/'suite.json'
        self.suite.write_text(json.dumps(proof.build()[1]))
        (self.root/'manifest.json').write_text(json.dumps({'fixture_sha256': proof.build()[1]['fixture_sha256']}))
        self.raw = {'case': 'J01', 'arm': 'A', 'status': 'complete', 'repetition': 1, 'attempt': 1,
                    'steps': [], 'checkpoints': []}
        for n in range(13):
            cp = self.folder/'checkpoints'/str(n)
            cp.mkdir(parents=True)
            (cp/'workspace.sqlite3').write_bytes(f'unit backup {n}'.encode())
            text = 'Corrected final state.' if n == 12 else 'Earlier saved state.'
            (cp/'records.json').write_text(json.dumps([
                {'id': 'note', 'version': n + 1, 'data': {'Note': {'markdown': text}}}]))
            self.raw['checkpoints'].append({'step': n, 'records_sha256': sha256(cp/'records.json'),
                                            'backup_sha256': sha256(cp/'workspace.sqlite3')})
            if n:
                self.raw['steps'].append({'index': n, 'kind': 'ask', 'terminal': 'Completed',
                                          'answer': 'FUTURE_12_ONLY' if n == 12 else f'Event {n}',
                                          'operations': []})
        self.manifest = self.root/'evidence.json'
        self.refresh()

    def refresh(self):
        (self.folder/'result.json').write_text(json.dumps(self.raw))
        self.manifest.write_text(json.dumps(collect(self.root, self.suite)))
        self.prepared = semantic.prepare(self.root, self.suite, self.manifest)

    def replies(self, score=4):
        return [{'id': packet['id'], 'packet_sha256': packet['packet_sha256'],
                 'completed': True, 'breaches': [],
                 'criteria': {name: {'score': score, 'explanation': 'Controlled checkpoint verdict',
                                    'evidence': [{'checkpoint': packet['packet']['checkpoint'],
                                                  'record': 'note', 'version': packet['packet']['checkpoint'] + 1}]}
                              for name in packet['packet']['criteria']}}
                for packet in self.prepared['packets']]

    def grade(self, replies):
        return semantic.import_controlled(self.root, self.suite, self.manifest,
                                          self.prepared, replies, 'temporal-fixture')

    def test_intermediate_defect_cannot_be_hidden_by_final_good_state(self):
        self.assertEqual(len(self.prepared['packets']), 12)
        self.assertEqual([p['packet']['checkpoint'] for p in self.prepared['packets']], list(range(1, 13)))
        self.assertNotIn('FUTURE_12_ONLY', json.dumps(self.prepared['packets'][0]))
        replies = self.replies()
        criterion = next(iter(replies[2]['criteria']))
        replies[2]['criteria'][criterion]['score'] = 0
        replies[2]['criteria'][criterion]['explanation'] = 'Lost qualifier at intermediate checkpoint; later repair cannot erase it.'
        result = self.grade(replies)
        self.assertEqual(result['status'], 'complete')
        self.assertEqual(len(result['grades']), 1)
        self.assertEqual(result['grades'][0]['criteria'][criterion]['score'], 0)
        self.assertEqual(result['grades'][0]['graded_checkpoints'], list(range(1, 13)))
        self.assertTrue(result['grades'][0]['checkpoint_complete'])
        self.assertEqual(len(result['checkpoint_grades']), 12)
        alternative = self.grade(self.replies())
        self.assertTrue(all(c['score'] == 4 for c in alternative['grades'][0]['criteria'].values()))
        self.assertFalse(alternative['qualified'])

    def test_missing_verdict_or_capture_stays_incomplete_and_known_breach_survives(self):
        replies = self.replies()
        replies[0]['breaches'] = ['cross_scope_disclosure']
        result = self.grade(replies[:-1])
        self.assertEqual(result['status'], 'incomplete')
        self.assertFalse(result['grades'][0]['checkpoint_complete'])
        manifest = json.loads(self.manifest.read_text())
        report = assess(json.loads(self.suite.read_text()), 'A', manifest['attempts'], result['grades'])
        self.assertEqual(report['status'], 'disqualified')
        self.assertIsNone(report['scenarios']['J01'])
        self.assertEqual(len(report['safety_breaches']), 1)
        self.raw['checkpoints'] = [cp for cp in self.raw['checkpoints'] if cp['step'] != 5]
        self.refresh()
        self.assertEqual(self.prepared['checkpoint_plans'][manifest['attempts'][0]['id']]['uncaptured'], [5])
        result = self.grade(self.replies())
        self.assertEqual(result['status'], 'incomplete')
        self.assertFalse(result['grades'][0]['checkpoint_complete'])

    def test_future_evidence_and_infrastructure_success_are_rejected(self):
        replies = self.replies()
        criterion = next(iter(replies[0]['criteria']))
        replies[0]['criteria'][criterion]['evidence'] = [{'checkpoint': 12, 'record': 'note', 'version': 13}]
        with self.assertRaisesRegex(ValueError, 'future evidence'):
            self.grade(replies)
        self.raw['status'] = 'setup_failure'
        self.refresh()
        result = self.grade(self.replies())
        self.assertEqual(result['status'], 'incomplete')
        self.assertEqual(len(result['infrastructure_attempts']), 1)
        # No final attempt failure label leaks back into an earlier checkpoint packet.
        self.assertEqual(self.prepared['packets'][0]['packet']['execution_status'], 'Completed')
