import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import copy
import unittest
from external import transform, digest


class ExternalTests(unittest.TestCase):
    def test_gold_isolation_and_shared_history_split(self):
        case = {'question_id': 'synthetic_abs', 'question_type': 'unit', 'question': 'What changed?',
                'question_date': '2026-01-02', 'answer': 'grader secret', 'answer_session_ids': ['one'],
                'haystack_session_ids': ['one'], 'haystack_dates': ['2026-01-01'],
                'haystack_sessions': [[{'role': 'user', 'content': 'Synthetic fact', 'has_answer': True}]]}
        cases = [case, dict(case, question_id='synthetic-two')]
        admission = {'dataset': 'xiaowu0162/longmemeval-cleaned', 'revision': 'unit-revision',
                     'license_text_sha256': 'unit-license', 'provenance': 'synthetic test only',
                     'reviewed_by': 'unit witness', 'access_date': '2026-10-28', 'allowed_use': 'approved',
                     'variant': 's', 'raw_sha256': digest(cases)}
        public, gold, locks = transform(cases, admission)
        self.assertNotIn('has_answer', str(public))
        self.assertNotIn('grader secret', str(public))
        self.assertNotIn('synthetic_abs', str(public))
        self.assertEqual(locks[0]['split_group'], locks[1]['split_group'])
        self.assertEqual(gold[0]['answer'], 'grader secret')
        for change in ({'variant':'oracle'}, {'allowed_use':'pending'}, {'raw_sha256':'wrong'}):
            with self.assertRaises(ValueError):
                transform(cases, dict(admission, **change))
