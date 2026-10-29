import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import unittest
from calibration import packets, assess


class CalibrationTests(unittest.TestCase):
    def test_order_swap_blindness_and_defect_controls(self):
        # Independent tiny unit witnesses, not the unavailable original 12 pairs.
        pairs = [{'id': 'unit-exception', 'facts': 'Valve closed except during inspection.',
                  'answers': ['Always closed.', 'Closed except during inspection.'],
                  'expected': 'second', 'defects': [['lost_exception'], []]},
                 {'id': 'unit-alternatives', 'facts': 'Cost is 10.', 'answers': ['10 total.', 'Total: 10.'],
                  'expected': 'tie', 'defects': [[], []]}]
        public, key = packets(pairs)
        self.assertEqual(len(public), 4)
        self.assertNotIn('expected', str(public))
        judgments = [{'id': p['id'], 'preference': key[p['id']]['expected'],
                      'defects': key[p['id']]['defects'], 'evidence': ['unit source']} for p in public]
        result = assess(key, judgments)
        self.assertEqual(result['preference_accuracy'], 1)
        self.assertEqual(result['position_consistency'], 1)
        self.assertFalse(result['qualified'])
        judgments[0]['preference'] = 'first'
        judgments[0]['defects'] = [[], ['invented']]
        result = assess(key, judgments)
        self.assertGreater(result['defects']['false_negative'], 0)
        self.assertGreater(result['defects']['false_positive'], 0)
        self.assertLess(result['position_consistency'], 1)
        self.assertEqual(assess(key, judgments[:1])['status'], 'incomplete')
        with self.assertRaises(ValueError):
            assess(key, judgments + judgments[:1])

    def test_malformed_defect_sets_are_refused(self):
        with self.assertRaises(ValueError):
            packets([])
        with self.assertRaises(ValueError):
            assess({}, [])
        pair = {'id': 'unit', 'facts': 'fact', 'answers': ['a', 'b'], 'expected': 'tie', 'defects': ['ab', 'cd']}
        with self.assertRaises(ValueError):
            packets([pair])
        pair['defects'] = [[], []]
        public, key = packets([pair])
        key[public[0]['id']]['defects'] = ['bad', 'bad']
        with self.assertRaises(ValueError):
            assess(key, [])
