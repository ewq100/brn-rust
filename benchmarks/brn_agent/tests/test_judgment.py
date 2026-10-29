import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import calibration
import judgment


class JudgmentTests(unittest.TestCase):
    def test_new_pack_separates_candidate_and_gold_and_swaps_every_pair(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'pack'
            judgment.prepare(root)
            candidate = json.loads((root / 'helper-candidate.json').read_text())
            self.assertEqual([c['id'] for c in candidate], [f'H{i:02}' for i in range(1, 13)])
            self.assertNotIn('SYNTHETIC-PRIVATE-LARCH-29', json.dumps(candidate))
            self.assertTrue(all(set(c) == {'id', 'prompt', 'notes'} for c in candidate))
            manifest = json.loads((root / 'manifest.json').read_text())
            self.assertFalse(manifest['original_pack'])
            for split, count in [('development', 24), ('held-out', 8)]:
                packets = json.loads((root / f'{split}-packets.json').read_text())
                key = json.loads((root / f'{split}-private-key.json').read_text())
                self.assertEqual(len(packets), count)
                self.assertTrue(all(p['defect_rubric'] == judgment.load('defect-rubric') for p in packets))
                self.assertTrue(all('expected' not in p and 'defects' not in p for p in packets))
                # Gold echo proves scoring arithmetic only, never qualification.
                replies = [dict(id=p['id'], preference=key[p['id']]['expected'],
                                defects=key[p['id']]['defects'], evidence='controlled gold echo') for p in packets]
                result = calibration.assess(key, replies)
                self.assertEqual(result['preference_accuracy'], 1)
                self.assertEqual(result['position_consistency'], 1)
                self.assertFalse(result['qualified'])
                for reply in replies:
                    reply['preference'] = 'first'
                    reply['defects'] = [[], []]
                biased = calibration.assess(key, replies)
                self.assertLess(biased['preference_accuracy'], .5)
                self.assertGreater(biased['defects']['false_negative'], 0)

    def rows(self, mode):
        return [dict(id=c['id'], mode=mode, scores={g: 4 for g in c['criteria']},
                     evidence=['controlled witness, not model qualification'], breach=False,
                     false_acceptance=False, false_rejection=False)
                for c in judgment.load('helpers') if (int(c['id'][1:]) <= 6) == (mode == 'natural')]

    def test_unobserved_handoff_missing_slots_and_safety(self):
        rows = self.rows('natural')
        result = judgment.assess(rows, 'natural')
        self.assertEqual(result['overall'], 100)
        self.assertFalse(result['qualified'])
        self.assertIsNone(judgment.assess(rows[:-1], 'natural')['overall'])
        rows[1]['scores']['handoff'] = None
        self.assertIsNone(judgment.assess(rows, 'natural')['overall'])
        rows[1]['breach'] = True
        self.assertEqual(judgment.assess(rows, 'natural')['status'], 'disqualified')
        # Controlled reports cannot manufacture a natural routing score.
        controlled = judgment.assess(self.rows('controlled'), 'controlled')
        self.assertIsNone(controlled['groups']['routing'])
        self.assertIsNone(controlled['overall'])
        with self.assertRaises(ValueError):
            judgment.assess(self.rows('controlled'), 'natural')
        with self.assertRaises(ValueError):
            judgment.assess(rows + [copy.deepcopy(rows[0])], 'natural')
        rows[0]['scores']['routing'] = float('nan')
        with self.assertRaises(ValueError):
            judgment.assess(rows, 'natural')

    def test_pair_coverage_and_independent_screening_worlds(self):
        development = judgment.load('calibration-development')
        held = judgment.load('calibration-held-out')
        self.assertEqual(len({p['topic'] for p in development}), 12)
        self.assertTrue({p['id'] for p in development}.isdisjoint(p['id'] for p in held))
        self.assertEqual({p['expected'] for p in development}, {'first', 'tie', 'both_unacceptable'})
        self.assertEqual({p['expected'] for p in held}, {'first', 'second', 'tie', 'both_unacceptable'})

    def test_rubric_rejects_undefined_gold_labels(self):
        pairs = judgment.load('calibration-development')
        with self.assertRaises(ValueError):
            calibration.packets(pairs, [])
        pairs[0]['defects'][1] = ['unpublished_label']
        with self.assertRaises(ValueError):
            calibration.packets(pairs, judgment.load('defect-rubric'))
