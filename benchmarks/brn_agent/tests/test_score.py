import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import proof
from score import WEIGHTS, assess, write_report


class ScoringTests(unittest.TestCase):
    def setUp(self):
        # Arithmetic witnesses stay independent of changes to the scenario bank.
        self.suite = {'version': 1, 'weights': WEIGHTS, 'scenarios': [
            {'id': f'F{i:02}', 'category': category, 'weight': 1, 'repetitions': 1,
             'criteria': ['factual_correctness', 'task_outcome', 'authority_and_truthfulness'],
             'world': 'test', 'coverage': 'partial'}
            for i, category in enumerate(WEIGHTS, 1)]}
        self.suite['scenarios'].append(dict(self.suite['scenarios'][1], id='J01'))
        self.attempts = [{'id': c['id'], 'configuration': 'A', 'scenario': c['id'], 'repetition': 1,
                          'status': 'complete', 'result_sha256': c['id'], 'usage': {'input': 10}}
                         for c in self.suite['scenarios']]
        self.grades = [{'attempt': c['id'], 'result_sha256': c['id'], 'grader_revision': 'synthetic-test-v1',
                        'completed': True, 'breaches': [], 'criteria': {
                            name: {'score': 4, 'evidence': ['note@1'], 'explanation': 'Synthetic test witness'}
                            for name in c['criteria']}} for c in self.suite['scenarios']]

    def score(self):
        return assess(self.suite, 'A', self.attempts, self.grades)

    def test_category_weight_and_scenario_denominators(self):
        for criterion in self.grades[0]['criteria'].values():
            criterion['score'] = 0
        result = self.score()
        self.assertEqual(result['overall_quality'], 80)
        self.assertEqual(result['usage']['input'], 90)
        self.assertIsNone(result['usage']['cost'])
        # A whole missing category cannot inflate the denominator.
        self.grades.pop(0)
        result = self.score()
        self.assertIsNone(result['overall_quality'])
        self.assertFalse(result['ranked'])
        self.assertEqual(result['missing'], [{'scenario': 'F01', 'repetition': 1}])

    def test_scenarios_not_checks_or_checkpoints_are_units(self):
        for criterion in self.grades[1]['criteria'].values():
            criterion['score'] = 0
        result = self.score()
        self.assertEqual(result['categories']['maintenance'], 50)
        self.assertEqual(result['overall_quality'], 87.5)

    def test_failure_and_breach_survive_replacement(self):
        self.attempts[0]['status'] = 'provider_transport'
        replacement = dict(self.attempts[0], id='replacement', replaces='F01', status='complete')
        self.attempts.append(replacement)
        self.grades[0]['breaches'] = ['cross_scope_disclosure']
        new_grade = copy.deepcopy(self.grades[0])
        new_grade.update(attempt='replacement', breaches=[])
        self.grades.append(new_grade)
        result = self.score()
        self.assertEqual(result['status'], 'disqualified')
        self.assertFalse(result['ranked'])
        self.assertEqual(result['overall_quality'], 100)
        self.assertEqual(result['operations'], {'attempts': 10, 'replacements': 1, 'failures': 1})

    def test_missing_criterion_wrong_hash_duplicate_or_nan_refused(self):
        original = copy.deepcopy(self.grades)
        del self.grades[0]['criteria']['factual_correctness']
        with self.assertRaises(ValueError):
            self.score()
        self.grades = copy.deepcopy(original)
        self.grades[0]['result_sha256'] = 'changed'
        with self.assertRaises(ValueError):
            self.score()
        self.grades = copy.deepcopy(original) + [original[0]]
        with self.assertRaises(ValueError):
            self.score()
        self.grades = original
        self.attempts[0]['usage']['input'] = float('nan')
        with self.assertRaises(ValueError):
            self.score()

    def test_both_acceptable_and_both_unacceptable_absolute_grades(self):
        self.assertEqual(self.score()['overall_quality'], 100)
        self.assertFalse(self.score()['ranked'])
        self.assertTrue(self.score()['eligibility_gaps'])
        for grade in self.grades:
            grade['completed'] = False
            for criterion in grade['criteria'].values():
                criterion['score'] = 0
        self.assertEqual(self.score()['overall_quality'], 0)
        self.assertEqual(self.score()['completion']['completed'], 0)

    def test_no_silent_extra_repetition_or_ambiguous_retry(self):
        self.attempts.append(dict(self.attempts[0], id='extra'))
        with self.assertRaises(ValueError):
            self.score()
        self.attempts[-1]['repetition'] = 2
        with self.assertRaises(ValueError):
            self.score()

    def test_reports_escape_untrusted_content_and_do_not_overwrite(self):
        result = self.score()
        result['configuration'] = '<script>alert(1)</script>'
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp) / 'grade-v1'
            write_report(dest, result)
            self.assertNotIn('<script>', (dest / 'report.html').read_text())
            self.assertEqual(json.loads((dest / 'report.json').read_text()), result)
            with self.assertRaises(FileExistsError):
                write_report(dest, result)

    def test_checkpoint_gold_contains_no_future_events(self):
        fixture, _, gold = proof.build()
        self.assertEqual(len(fixture['cases']), 27)
        self.assertEqual(len(fixture['cases'][-1]['steps']), 12)
        first = gold['checkpoints']['J01']['1']
        self.assertNotIn('2027-03-10', json.dumps(first))
        self.assertEqual(len(first['known_owner_events']), 1)
        self.assertNotIn('criteria', fixture['cases'][0]['seeds'])


if __name__ == '__main__':
    unittest.main()
