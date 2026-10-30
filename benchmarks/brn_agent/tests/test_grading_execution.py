"""Offline execution receipts are protocol evidence, never semantic qualification."""
import copy
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import semantic


class GraderExecutionTests(unittest.TestCase):
    def setUp(self):
        from test_semantic import SemanticProtocolTests
        self.base = SemanticProtocolTests()
        self.base.setUp()
        self.addCleanup(self.base.doCleanups)
        self.prepared = semantic.prepare(self.base.root, self.base.suite, self.base.manifest)
        packet = self.prepared['packets'][0]
        self.fixture = {'revision': 'synthetic-execution-v1', 'max_calls': 1,
                        'max_request_bytes': 1000000, 'max_response_bytes': 100000,
                        'calls': [{'id': packet['id'], 'packet_sha256': packet['packet_sha256'],
                                   'response': {'completed': True, 'breaches': [],
                                                'criteria': {name: {'score': 3, 'explanation': 'Controlled witness',
                                                                   'evidence': [{'checkpoint': 1, 'record': 'note', 'version': 1}]}
                                                             for name in packet['packet']['criteria']}}}]}

    def execute(self, fixture, name):
        return semantic.execute_controlled(self.base.root, self.base.suite, self.base.manifest,
                                           fixture, self.base.root/name)

    def test_execution_retains_bound_requests_and_regrades_without_candidate(self):
        before = self.base.manifest.read_bytes()
        result = self.execute(self.fixture, 'grade1')
        self.assertEqual(result['status'], 'complete')
        self.assertFalse(result['qualified'])
        self.assertEqual(result['calls'], 1)
        self.assertEqual(result['candidate_calls'], 0)
        request = json.loads((self.base.root/'grade1/requests.jsonl').read_text())
        self.assertEqual(request['packet'], self.prepared['packets'][0]['packet'])
        revised = copy.deepcopy(self.fixture)
        revised['revision'] = 'synthetic-v2'
        for score in revised['calls'][0]['response']['criteria'].values():
            score['score'] = 4
        second = self.execute(revised, 'grade2')
        self.assertEqual(second['status'], 'complete')
        first_grades = (self.base.root/'grade1/grades.json').read_bytes()
        with self.assertRaises(FileExistsError):
            self.execute(revised, 'grade1')
        self.assertEqual(first_grades, (self.base.root/'grade1/grades.json').read_bytes())
        self.assertEqual(before, self.base.manifest.read_bytes())

    def test_budget_errors_missing_and_invalid_responses_remain_visible(self):
        for name, fixture in [('request-cap', dict(self.fixture, max_request_bytes=1)),
                              ('response-cap', dict(self.fixture, max_response_bytes=1)),
                              ('missing', dict(self.fixture, calls=[]))]:
            result = self.execute(fixture, name)
            self.assertEqual(result['status'], 'incomplete')
            self.assertFalse(result['qualified'])
        outage = copy.deepcopy(self.fixture)
        del outage['calls'][0]['response']
        outage['calls'][0]['error'] = 'synthetic provider outage'
        result = self.execute(outage, 'outage')
        self.assertEqual(result['calls'], 1)
        self.assertEqual(result['grader_failures'][0]['error'], 'synthetic provider outage')
        invalid = copy.deepcopy(self.fixture)
        invalid['calls'][0]['response']['completed'] = 'yes'
        with self.assertRaises(ValueError):
            self.execute(invalid, 'invalid')
        saved = json.loads((self.base.root/'invalid/execution.json').read_text())
        self.assertEqual(saved['status'], 'failed')
        self.assertEqual(saved['calls'], 1)
        self.assertTrue((self.base.root/'invalid/responses.jsonl').is_file())
        wrong = copy.deepcopy(self.fixture)
        wrong['calls'][0]['packet_sha256'] = 'wrong'
        with self.assertRaises(ValueError):
            self.execute(wrong, 'wrong')
        self.assertFalse((self.base.root/'wrong').exists())
