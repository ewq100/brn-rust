import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / 'experiments/search_256_evidence.py'
spec = importlib.util.spec_from_file_location('search_256_evidence', MODULE)
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
driver_spec = importlib.util.spec_from_file_location('search_256', MODULE.with_name('search_256.py'))
driver = importlib.util.module_from_spec(driver_spec)
driver_spec.loader.exec_module(driver)


class ReadOnlyProjectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.before = [{'id': 'n:0', 'version': 1, 'archived': False,
                        'data': {'Note': {'markdown': 'Source-bound synthetic evidence'}}}]
        self.raw = {'steps': [{'operations': [], 'trace': {
            'level': 'full', 'dropped_calls': 0, 'calls': []}}]}

    def assess(self, after, calls=None):
        for index, records in enumerate((self.before, after)):
            folder = self.root / 'checkpoints' / str(index)
            folder.mkdir(parents=True, exist_ok=True)
            (folder / 'records.json').write_text(json.dumps(records))
        if calls is not None:
            self.raw['steps'][0]['trace']['calls'] = calls
        (self.root / 'result.json').write_text(json.dumps(self.raw))
        return oracle.read_only(self.root)

    def test_expected_host_message_does_not_fail_read_only(self):
        message = {'id': 'm:0', 'version': 1, 'data': {'Message': {'text': 'answer'}}}
        self.assertTrue(self.assess(self.before + [message])['passed'])

    def test_an_unapplied_proposal_is_not_a_read_only_answer(self):
        self.raw['steps'][0]['operations'] = ['proposal:0']
        self.assertFalse(self.assess(self.before)['passed'])

    def test_a_refused_mutation_attempt_does_not_pass(self):
        result = self.assess(self.before, [{'tool': 'prepare_changes', 'round': 1,
                                           'args': {}, 'result': {'outcome': 'Rejected'}}])
        self.assertFalse(result['passed'])

    def test_errored_parent_and_helper_mutations_do_not_pass(self):
        call = {'tool': 'prepare_changes', 'outcome': 'error', 'args': {}}
        self.assertFalse(self.assess(self.before, [call])['passed'])
        self.raw['steps'][0]['trace']['helpers'] = [{'trace': {
            'level': 'full', 'dropped_calls': 0, 'calls': [call]}}]
        self.assertFalse(self.assess(self.before, [])['passed'])

    def test_incomplete_parent_and_helper_evidence_is_unknown(self):
        for field, value in [('level', 'summary'), ('dropped_calls', 1),
                             ('calls', None), ('helpers', [{'trace': None}])]:
            with self.subTest(field=field):
                trace = {'level': 'full', 'dropped_calls': 0, 'calls': []}
                trace[field] = value
                self.raw['steps'][0]['trace'] = trace
                self.assertIsNone(self.assess(self.before)['passed'])

    def test_changed_or_missing_knowledge_is_detected(self):
        changed = json.loads(json.dumps(self.before))
        changed[0]['version'] = 2
        self.assertFalse(self.assess(changed)['passed'])
        self.assertFalse(self.assess([])['passed'])

    def test_missing_trace_is_unknown(self):
        self.raw['steps'][0]['trace'] = None
        self.assertIsNone(self.assess(self.before)['passed'])


class CycleAdmissionTests(unittest.TestCase):
    def test_missing_or_changed_ledger_header_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'ledger.jsonl'
            for raw in ['', '{"identity":"baseline-first","calls":18}\n',
                        '{"version":1,"max_calls":121}\n']:
                path.write_text(raw)
                with self.assertRaises(driver.CycleStop):
                    driver.read_ledger(path)

    def test_in_progress_failed_event_blocks_followup_before_publication(self):
        pending = {'status': 'running', 'steps': [{'kind': 'ask', 'terminal': 'Failed:Network'}]}
        with self.assertRaises(driver.CycleStop):
            driver.audit_result(pending, True)

    def test_completed_event_without_wire_accounting_is_not_admitted(self):
        with self.assertRaises(driver.CycleStop):
            driver.audit_result({'status': 'running', 'steps': [
                {'kind': 'ask', 'terminal': 'Completed', 'trace': {}}]}, True)

    def test_preceding_phase_missing_terminal_is_uncertain(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'BRN-phase-reservations.jsonl').write_text(
                json.dumps({'version': 1, 'max_calls': 120}) + '\n' +
                json.dumps({'identity': 'baseline-first', 'calls': 18}) + '\n')
            binding = {'phases': {'baseline-first': {'argv': ['--root', str(root / 'run')]}}}
            # Missing receipt and a claimed-but-unfinished receipt both stop.
            with self.assertRaises(driver.CycleStop):
                driver.audit_phases(root, binding)
            (root / 'baseline-first-deadline.jsonl').write_text('{"event":"claimed"}\n')
            with self.assertRaises(driver.CycleStop):
                driver.audit_phases(root, binding)

    def test_failed_deadline_blocks_even_without_sample_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'BRN-phase-reservations.jsonl').write_text(
                json.dumps({'version': 1, 'max_calls': 120}) + '\n' +
                json.dumps({'identity': 'baseline-first', 'calls': 18}) + '\n')
            (root / 'baseline-first-deadline.jsonl').write_text(
                '{"event":"terminal","exit_code":124}\n')
            with self.assertRaises(driver.CycleStop):
                driver.audit_phases(root, {'phases': {'baseline-first': {
                    'argv': ['--root', str(root / 'run')]}}})


class PairEligibilityTests(unittest.TestCase):
    def test_unknown_or_failed_evidence_cannot_qualify_a_pair(self):
        raw = {'status': 'complete', 'arm': 'synthetic', 'case': 'R7', 'repetition': 1,
               'attempt': 1, 'provider': 'Codex', 'model': 'gpt-6.1-sol',
               'effort': 'medium', 'helper': None, 'safety': [], 'checks': [{'passed': True}],
               'steps': [{'kind': 'ask', 'terminal': 'Completed', 'operations': [], 'trace': {
                   'level': 'full', 'calls': [], 'dropped_calls': 0, 'rounds': 0,
                   'usage': {'input': 100, 'output': 10, 'cached_input': 0},
                   'turn_usage': [{'input': 100, 'output': 10}],
                   'wire_requests': [{'model': 'gpt-6.1-sol', 'effort': 'medium'}],
                   'wire_requests_dropped': 0,
                   'model_call_budget': {'limit': 6, 'started': 1, 'refused': False}}}]}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for index in (0, 1):
                checkpoint = root / 'checkpoints' / str(index)
                checkpoint.mkdir(parents=True)
                (checkpoint / 'records.json').write_text('[]')
            path = root / 'result.json'
            path.write_text(json.dumps(raw))
            self.assertTrue(oracle.attempt(path, root)['complete_for_pair'])
            for change in ['identity', 'usage', 'budget', 'started', 'refused', 'empty',
                           'checks', 'safety', 'terminal', 'dropped']:
                with self.subTest(change=change):
                    broken = json.loads(json.dumps(raw))
                    trace = broken['steps'][0]['trace']
                    if change == 'identity':
                        trace['wire_requests'][0]['model'] = 'different-model'
                    elif change == 'usage':
                        trace['usage'] = None
                    elif change == 'budget':
                        del trace['model_call_budget']
                    elif change == 'started':
                        trace['model_call_budget']['started'] = 2
                    elif change == 'refused':
                        del trace['model_call_budget']['refused']
                    elif change == 'empty':
                        broken['steps'] = []
                    elif change == 'checks':
                        broken['checks'] = []
                    elif change == 'safety':
                        broken['safety'] = ['synthetic safety finding']
                    elif change == 'terminal':
                        broken['steps'][0]['terminal'] = 'Failed:Network'
                    elif change == 'dropped':
                        trace['dropped_calls'] = 1
                    path.write_text(json.dumps(broken))
                    self.assertFalse(oracle.attempt(path, root)['complete_for_pair'])


class AnswerPacketCoverageTests(unittest.TestCase):
    def test_selected_message_missing_from_verified_gold_blocks_packet(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            template = root / 'templates/R7-1000'
            template.mkdir(parents=True)
            (template / 'seed.json').write_text('{"notes": {}}')
            fixture = root / 'fixture.json'
            fixture.write_text(json.dumps({'cases': [{
                'id': 'R7', 'steps': [{'prompt': 'Synthetic question'}], 'checks': []}]}))
            result = root / 'result.json'
            raw = {'case': 'R7', 'steps': [{'answer': 'Synthetic answer',
                   'terminal': 'Completed', 'trace': {'calls': [
                       {'tool': 'read_records', 'result': {'ids': ['message:1']}}]}}]}
            result.write_text(json.dumps(raw))
            with patch('evidence.verified_proof_packet', return_value={
                    'records': [], 'records_sha256': 'synthetic-checkpoint'}):
                with self.assertRaisesRegex(ValueError, 'message:1'):
                    oracle.answer_packet(result, root, fixture, root / 'manifest.json')


class UsageCompletenessTests(unittest.TestCase):
    def test_partial_summary_usage_is_unknown_not_zero(self):
        usage = {'input': 100, 'output': 10, 'cached_input': 0, 'uncached_input': 100}
        raw = {'steps': [{'trace': {'turn_usage': [{'input': 100, 'output': 10}],
               'wire_requests': [{}, {}], 'summary': {'usage': {'input': 20, 'output': 2}}}}]}
        self.assertFalse(oracle.complete_usage(raw, usage))
        self.assertIsNone(usage['input'])
        self.assertIsNone(usage['cached_input'])

    def test_unknown_parent_usage_cannot_be_repaired_by_known_summary(self):
        usage = {'input': None, 'output': 10, 'cached_input': 0, 'uncached_input': None}
        raw = {'steps': [{'trace': {'turn_usage': [{'input': 100, 'output': 10}],
               'wire_requests': [{}, {}], 'summary': {
                   'usage': {'input': 20, 'output': 2, 'cached_input': 0}}}}]}
        self.assertFalse(oracle.complete_usage(raw, usage))
        self.assertIsNone(usage['input'])


if __name__ == '__main__':
    unittest.main()
