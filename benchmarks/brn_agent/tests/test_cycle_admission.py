"""Offline witnesses for issue254's protected role and fail-closed admission."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

PATH = Path(__file__).resolve().parents[1]/'experiments/d254/supervise.py'
spec = importlib.util.spec_from_file_location('d254_supervise', PATH)
cycle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cycle)


class CycleAdmissionTests(unittest.TestCase):
    def test_operation_kind_cannot_use_an_incompatible_role(self):
        for kind, role in [('candidate','grader'),('grader','phase1')]:
            with self.subTest(kind=kind,role=role), tempfile.TemporaryDirectory() as folder:
                root=Path(folder)
                operation={'id':'wrong-role','kind':kind,'role':role,'root':str(root/'run')}
                (root/'operation.json').write_text(json.dumps(operation))
                with self.assertRaises(cycle.CycleStop):
                    cycle.main(['--operation',str(root/'operation.json'),
                        '--account-file',str(root/'unused-account'),'--cycle',str(root/'absent-cycle'),
                        '--roles',str(root/'roles')])
                self.assertFalse((root/'roles').exists())

    def test_post_run_audit_rejection_has_failed_terminal_and_nonzero_exit(self):
        import datetime
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            operation = {'id':'audit-rejection','kind':'candidate','role':'phase1',
                'root':str(root/'run'),'minimum_seconds':1,'seconds':90,
                'model_call_limit':6,'max_calls':6,'fixture':'unused','truth':'unused',
                'seeder':'unused','cases':'unused','repetitions':1,'arms':[],
                'stock_helper':'unused','brn_helper':'unused'}
            (root/'operation.json').write_text(json.dumps(operation))
            deadline = datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(hours=1)
            (root/'cycle.json').write_text(json.dumps({'whole_cycle_deadline':deadline.isoformat()}))
            def install_without_processes(guard):
                guard.timer = SimpleNamespace(cancel=lambda:None)
                guard.hard_timer = SimpleNamespace(cancel=lambda:None)
            with mock.patch.object(cycle.native,'main',return_value=0), \
                    mock.patch.object(cycle,'audit',side_effect=cycle.CycleStop('rejected final sample')), \
                    mock.patch.object(cycle,'audit_operations'), \
                    mock.patch.object(cycle,'install_role'), \
                    mock.patch.object(cycle,'active_audit',return_value=lambda:None), \
                    mock.patch.object(cycle.Guard,'install',install_without_processes):
                result = cycle.main(['--operation',str(root/'operation.json'),
                    '--account-file',str(root/'unused-account'),'--cycle',str(root/'cycle.json'),
                    '--roles',str(root/'roles')])
            rows = [json.loads(x) for x in (root/'run.deadline.jsonl').read_text().splitlines()]
            terminal = next(x for x in rows if x['event']=='terminal')
            self.assertNotEqual(result,0)
            self.assertNotEqual(terminal['exit_code'],0)
            self.assertEqual(terminal['status'],'failed')

    def test_in_memory_failure_is_saved_before_followup_read_can_launch(self):
        from types import SimpleNamespace
        original = cycle.native.Sample.run_step
        sample = SimpleNamespace(result={'status':'running','steps':[]}, persist=lambda: saved.append(True))
        saved = []
        check = None
        def step(sample):
            sample.result['steps'].append({'kind':'ask','terminal':'Failed:RateLimited'})
            check()  # Models Guard.before_launch at run_step's follow-up read.
            self.fail('Follow-up process must never launch')
        try:
            cycle.native.Sample.run_step = step
            check = cycle.active_audit()
            with self.assertRaises(cycle.CycleStop):
                cycle.native.Sample.run_step(sample)
            self.assertEqual(saved, [True])
        finally:
            cycle.native.Sample.run_step = original

    def test_in_progress_failed_event_stops_before_another_launch(self):
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder)/'runs/W/A/result.json'
            p.parent.mkdir(parents=True)
            p.write_text(json.dumps({'status':'running', 'steps':[{
                'kind':'ask','terminal':'Failed:RateLimited'}]}))
            with self.assertRaises(cycle.CycleStop):
                cycle.audit(folder)

    def test_wrong_identity_stops_even_when_native_status_completed(self):
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder)/'runs/W/A/result.json'
            p.parent.mkdir(parents=True)
            p.write_text(json.dumps({'status':'complete','model':'expected','effort':'medium',
                'steps':[{'kind':'ask','terminal':'Completed','trace':{
                    'wire_requests':[{'model':'wrong','effort':'medium'}]}}]}))
            with self.assertRaises(cycle.CycleStop):
                cycle.audit(folder)

    def test_failed_earlier_deadline_prevents_later_operation(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root/'operation-first.json').write_text(json.dumps({'root':str(root/'run')}))
            (root/'run.deadline.jsonl').write_text(json.dumps({
                'event':'terminal','status':'deadline','exit_code':124})+'\n')
            with self.assertRaises(cycle.CycleStop):
                cycle.audit_operations(root)

    def test_role_debit_does_not_spend_another_role_or_refund_uncertainty(self):
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder)
            original = cycle.native.LIMITS.Reservations
            try:
                cycle.install_role(p/'phase1.jsonl','phase1','first',6)
                local = cycle.native.LIMITS.Reservations(p/'native.jsonl',6)
                local.reserve('event1',6)
                with self.assertRaises(ValueError):
                    local.reserve('event2',6)
                # Role debit happens before local refusal; unknown slots remain used.
                rows = [json.loads(x) for x in (p/'phase1.jsonl').read_text().splitlines()]
                self.assertEqual(sum(x.get('calls',0) for x in rows),12)
                self.assertFalse((p/'phase2.jsonl').exists())
                with self.assertRaises(cycle.CycleStop):
                    local.reserve('event3',8)
            finally:
                cycle.native.LIMITS.Reservations = original


if __name__ == '__main__':
    unittest.main()
