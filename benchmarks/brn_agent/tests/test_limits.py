import concurrent.futures
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from limits import Reservations, LimitReached, dry_plan
import proof


class LimitTests(unittest.TestCase):
    def test_concurrent_reservations_never_overadmit(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'limits.jsonl'
            def attempt(index):
                try:
                    Reservations(path, 10).reserve(str(index), 3)
                    return True
                except LimitReached:
                    return False
            with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
                self.assertEqual(sum(pool.map(attempt, range(8))), 3)
            with self.assertRaises(LimitReached):
                Reservations(path, 10).reserve('later', 2)

    def test_duplicate_unknown_and_corrupt_attempts_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'limits.jsonl'
            ledger = Reservations(path, 10)
            ledger.reserve('attempt1/event1', 3)
            with self.assertRaises(LimitReached):
                ledger.reserve('attempt1/event1', 3)
            with self.assertRaises(LimitReached):
                Reservations(path, 20).reserve('new', 3)
            with path.open('a') as stream:
                stream.write('{"identity":')
            with self.assertRaises(LimitReached):
                ledger.reserve('attempt2', 3)

    def test_spend_cap_never_uses_unknown_subscription_price(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'limits.jsonl'
            with self.assertRaises(LimitReached):
                Reservations(path, 10, spend_cap=100)
            self.assertFalse(path.exists())
        with self.assertRaises(LimitReached):
            dry_plan(proof.build()[1], 100, 2, spend_cap=0)

    def test_dry_plan_accounts_for_every_event(self):
        suite = proof.build()[1]
        self.assertFalse(dry_plan(suite, 31, 2)['admitted'])
        self.assertEqual(dry_plan(suite, 32, 2)['reserved_calls'], 32)
        self.assertTrue(dry_plan(suite, 32, 2)['admitted'])

    def test_native_scheduler_refuses_before_cli_dispatch(self):
        from types import SimpleNamespace
        from unittest.mock import Mock
        sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
        import evaluate_wiki_v2 as v2
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            v2.LIMITS.Reservations(root / 'model-call-reservations.jsonl', 1).reserve('prior', 1)
            sample = object.__new__(v2.Sample)
            sample.root, sample.folder = root, root / 'attempt'
            sample.args = SimpleNamespace(model_call_limit=1, max_total_model_calls=1, live=True)
            sample.cli = Mock()
            sample.sample, sample.caps = {}, {'trace': True}
            with self.assertRaises(v2.LIMITS.LimitReached):
                sample.run_step(1, {'kind': 'ask', 'thread': 'case', 'prompt': 'synthetic'}, {'case': 't'}, {})
            sample.cli.call.assert_not_called()
