"""Contain only proven read-only evaluator failures, never candidate failures."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]/'experiments/d254'
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location('d254_outputs', HERE/'supervise_outputs.py')
outputs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(outputs)


class OutputContainmentTests(unittest.TestCase):
    def reader_setup(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        folder = Path(temporary.name)
        root = folder/'never-launched'
        (folder/'operation-reader.json').write_text(json.dumps({'root':str(root),'role':'phase1'}))
        request = folder/'request.json'
        request.write_text(json.dumps({'kind':'reader'}))
        (folder/'grading-dispositions').mkdir()
        disposition = folder/'grading-dispositions/reader.json'
        disposition.write_text(json.dumps({'kind':'reader-setup-never-dispatched',
            'root':str(root),'request_path':str(request),
            'request_sha256':hashlib.sha256(request.read_bytes()).hexdigest(),'actual_HTTP':0}))
        return folder, root, request, disposition

    def test_reader_setup_accepts_only_absent_root_with_bound_reader_request(self):
        folder, root, _, _ = self.reader_setup()
        outputs.audit_operations(folder)
        self.assertFalse(root.exists())

    def test_reader_setup_refuses_existing_root_changed_hash_or_non_reader(self):
        for mutation in ('root-exists','hash-mismatch','wrong-kind'):
            with self.subTest(mutation=mutation):
                folder, root, request, disposition = self.reader_setup()
                if mutation=='root-exists':
                    root.mkdir()
                else:
                    request.write_text(json.dumps({'kind':'candidate'}))
                    if mutation=='wrong-kind':
                        row=json.loads(disposition.read_text())
                        row['request_sha256']=hashlib.sha256(request.read_bytes()).hexdigest()
                        disposition.write_text(json.dumps(row))
                with self.assertRaises(outputs.native_supervisor.CycleStop):
                    outputs.audit_operations(folder)

    def witness(self, role='grader', cleanup=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        folder = Path(temporary.name)
        root = str(folder/'run')
        (folder/'operation-one.json').write_text(json.dumps({'root':root,'role':role}))
        receipt = folder/'run.deadline.jsonl'
        receipt.write_text(json.dumps({'event':'terminal','exit_code':1,
                                      'cleanup_errors':cleanup or []})+'\n')
        dispositions = folder/'grading-dispositions'
        dispositions.mkdir()
        disposition = dispositions/'one.json'
        disposition.write_text(json.dumps({'kind':'uncertain-read-only-grader-contained',
            'root':root, 'reservation_retained':True,
            'deadline_receipt_sha256':hashlib.sha256(receipt.read_bytes()).hexdigest()}))
        return folder, receipt, disposition

    def test_reaped_read_only_failure_can_close_without_erasing_receipt(self):
        folder, receipt, _ = self.witness()
        original = receipt.read_bytes()
        outputs.audit_operations(folder)
        self.assertEqual(receipt.read_bytes(), original)

    def test_candidate_failure_cannot_use_grader_disposition(self):
        folder, _, _ = self.witness('phase2-development')
        with self.assertRaises(outputs.native_supervisor.CycleStop):
            outputs.audit_operations(folder)

    def test_changed_receipt_or_uncertain_cleanup_stops(self):
        folder, receipt, _ = self.witness()
        receipt.write_text(receipt.read_text()+json.dumps({'event':'amended'})+'\n')
        with self.assertRaises(outputs.native_supervisor.CycleStop):
            outputs.audit_operations(folder)
        folder, _, _ = self.witness(cleanup=['process still alive'])
        with self.assertRaises(outputs.native_supervisor.CycleStop):
            outputs.audit_operations(folder)

    def test_missing_terminal_remains_uncertain_even_with_disposition(self):
        folder, receipt, _ = self.witness()
        receipt.write_text(json.dumps({'event':'started'})+'\n')
        with self.assertRaises(outputs.native_supervisor.CycleStop):
            outputs.audit_operations(folder)


if __name__ == '__main__':
    unittest.main()
