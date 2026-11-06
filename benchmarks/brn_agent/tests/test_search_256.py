import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

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
        self.raw = {'steps': [{'operations': [], 'trace': {'calls': []}}]}

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

    def test_changed_or_missing_knowledge_is_detected(self):
        changed = json.loads(json.dumps(self.before))
        changed[0]['version'] = 2
        self.assertFalse(self.assess(changed)['passed'])
        self.assertFalse(self.assess([])['passed'])

    def test_missing_trace_is_unknown(self):
        self.raw['steps'][0]['trace'] = None
        self.assertIsNone(self.assess(self.before)['passed'])


class CycleAdmissionTests(unittest.TestCase):
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


if __name__ == '__main__':
    unittest.main()
