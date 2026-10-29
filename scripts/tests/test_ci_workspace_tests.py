import importlib.util
import io
from pathlib import Path
import sys
import unittest

SPEC = importlib.util.spec_from_file_location("ci_workspace_tests", Path(__file__).resolve().parents[1] / "ci-workspace-tests.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class WorkspaceDiagnostics(unittest.TestCase):
    def run_child(self, source):
        output = io.StringIO()
        status = MODULE.run([sys.executable, "-c", source], output)
        return status, output.getvalue()

    def test_failure_preserves_exit_and_only_annotates_identity(self):
        status, output = self.run_child("print('test module::case ... FAILED'); print('private assertion payload'); raise SystemExit(101)")
        self.assertEqual(status, 101)
        annotations = [line for line in output.splitlines() if line.startswith('::error')]
        self.assertEqual(annotations, ['::error title=Workspace test failed::module::case'])
        self.assertIn('private assertion payload', output)  # Original console output retained.

    def test_success_remains_success_without_annotations(self):
        status, output = self.run_child("print('test module::case ... ok')")
        self.assertEqual(status, 0)
        self.assertNotIn('::error', output)

    def test_unrecognized_failure_is_not_success(self):
        status, output = self.run_child("print('compile failure'); raise SystemExit(2)")
        self.assertEqual(status, 2)
        self.assertIn('No Rust test identity captured', output)

    def test_ansi_duplicate_and_hostile_names(self):
        status, output = self.run_child("print('\\x1b[31mtest module::case ... FAILED\\x1b[0m'); print('test module::case ... FAILED'); print('test bad%0Aname ... FAILED'); raise SystemExit(101)")
        self.assertEqual(status, 101)
        annotations = [line for line in output.splitlines() if line.startswith('::error')]
        self.assertEqual(annotations, ['::error title=Workspace test failed::module::case'])

    def test_annotation_count_bounded(self):
        status, output = self.run_child("[print(f'test case_{i} ... FAILED') for i in range(100)]; raise SystemExit(101)")
        self.assertEqual(status, 101)
        self.assertEqual(output.count('::error title='), 50)

    def test_original_command_is_unchanged(self):
        self.assertEqual(MODULE.COMMAND, ['cargo', 'test', '--workspace', '--no-fail-fast', '--locked', '--offline'])

    def test_interleaved_failure_uses_final_failure_summary(self):
        status, output = self.run_child("print('test one ... test two ... FAILED'); print('failures:'); print(''); print('---- module::case stdout ----'); print('    private_payload'); print('failures:'); print('    module::case'); print(''); print('test result: FAILED. 1 passed; 1 failed; 0 ignored'); raise SystemExit(101)")
        self.assertEqual(status, 101)
        self.assertEqual([line for line in output.splitlines() if line.startswith('::error')], ['::error title=Workspace test failed::module::case'])

    def test_incomplete_failure_section_does_not_annotate_payload(self):
        status, output = self.run_child("print('failures:'); print('    private_payload'); print('panic detail'); raise SystemExit(101)")
        self.assertEqual(status, 101)
        annotations = [line for line in output.splitlines() if line.startswith('::error')]
        self.assertEqual(annotations, ['::error title=Workspace test failed::No Rust test identity captured; inspect build or process failure.'])
