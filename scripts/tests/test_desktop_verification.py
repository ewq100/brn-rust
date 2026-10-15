"""Exercise the desktop verification gate with synthetic Cargo output only."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


RUNNER = Path(__file__).resolve().parents[1] / 'verify-desktop-shell.sh'
WIDGET_ARGS = [
    '-p', 'brn-desktop', '--features', 'native-ui,native-test-support',
    '--bin', 'brn-desktop', 'threads_ui::', '--locked', '--offline',
]


@unittest.skipIf(os.name == 'nt', 'Unix Bash verification gate')
class DesktopVerificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='brn-desktop-gate-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.log = self.root / 'cargo.jsonl'
        self.fake_cargo = self.root / 'fake_cargo.py'
        self.fake_cargo.write_text('''import json
import os
from pathlib import Path
import sys

args = sys.argv[1:]
with Path(os.environ['BRN_TEST_CARGO_LOG']).open('a') as stream:
    stream.write(json.dumps(args) + '\\n')
case = os.environ['BRN_TEST_CARGO_CASE']
if '--list' in args:
    if case == 'discovery-failure':
        print('synthetic discovery failure', file=sys.stderr)
        sys.exit(37)
    if case == 'empty':
        print('0 tests, 0 benchmarks')
    elif case == 'unit-discovery':
        print('threads_ui::layout::tests::synthetic_unit: test')
    else:
        print('threads_ui::focus_tests::synthetic_widget: test')
    sys.exit(0)
if args[0] == 'test':
    if case == 'test-failure':
        print('test result: FAILED. 0 passed; 1 failed; 0 ignored;')
        sys.exit(39)
    if case == 'ignored':
        print('test threads_ui::focus_tests::synthetic_widget ... ignored')
        print('test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s')
    else:
        if case == 'unit-execution':
            print('test threads_ui::layout::tests::synthetic_unit ... ok')
        else:
            print('test threads_ui::focus_tests::synthetic_widget ... ok')
        print('test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s')
''')

    def run_gate(self, case='success', args=('--widgets-only',)):
        # A shell function wins over every PATH entry, including Homebrew's
        # prepended Rust path. These tests must never invoke the real Cargo.
        wrapper = '''cargo() { "$BRN_TEST_PYTHON" "$BRN_TEST_FAKE_CARGO" "$@"; }
runner="$1"
shift
source "$runner" "$@"
'''
        self.log.unlink(missing_ok=True)
        environment = dict(
            os.environ, BASH_ENV='', ENV='', BRN_VERIFY_OUTPUT_DIR='',
            _BRN_EVIDENCE_CHILD='1', BRN_TEST_PYTHON=sys.executable,
            BRN_TEST_FAKE_CARGO=str(self.fake_cargo),
            BRN_TEST_CARGO_LOG=str(self.log), BRN_TEST_CARGO_CASE=case,
        )
        result = subprocess.run(
            [shutil.which('bash'), '-c', wrapper, 'desktop-gate-test', str(RUNNER), *args],
            env=environment, capture_output=True, text=True, timeout=10,
        )
        calls = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        return result, calls

    def test_widget_run_discovers_and_executes_the_locked_suite(self):
        result, calls = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, [
            ['test', *WIDGET_ARGS, '--', '--list', '--format', 'terse'],
            ['test', *WIDGET_ARGS, '--', '--format', 'pretty', '--color', 'never'],
        ])
        self.assertIn('Discovered 1 Threads widget tests.', result.stdout)

    def test_empty_widget_discovery_refuses_to_execute_tests(self):
        result, calls = self.run_gate('empty')
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(calls), 1)
        self.assertIn('No Threads widget tests were discovered', result.stderr)

    def test_pure_unit_discovery_does_not_count_as_widgets(self):
        result, calls = self.run_gate('unit-discovery')
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(calls), 1)
        self.assertIn('No Threads widget tests were discovered', result.stderr)

    def test_discovery_failure_exit_is_preserved(self):
        result, calls = self.run_gate('discovery-failure')
        self.assertEqual(result.returncode, 37)
        self.assertEqual(len(calls), 1)
        self.assertIn('synthetic discovery failure', result.stderr)

    def test_test_failure_exit_is_preserved_through_output_capture(self):
        result, calls = self.run_gate('test-failure')
        self.assertEqual(result.returncode, 39)
        self.assertEqual(len(calls), 2)
        self.assertIn('test result: FAILED.', result.stdout)

    def test_entirely_ignored_widget_run_cannot_pass(self):
        result, calls = self.run_gate('ignored')
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(calls), 2)
        self.assertIn('No Threads widget tests passed', result.stderr)

    def test_pure_unit_execution_does_not_count_as_widgets(self):
        result, calls = self.run_gate('unit-execution')
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(calls), 2)
        self.assertIn('No Threads widget tests passed', result.stderr)

    def test_default_keeps_helper_build_strict_lint_and_all_desktop_tests(self):
        result, calls = self.run_gate(args=())
        self.assertEqual(result.returncode, 0, result.stderr)
        desktop_args = ['-p', 'brn-desktop', '--features', 'native-ui,native-test-support']
        self.assertEqual(calls, [
            ['build', '-p', 'brn-intake', '--features', 'helper', '--bin', 'brn-intake-helper', '--locked', '--offline'],
            ['build', *desktop_args, '--locked', '--offline'],
            ['clippy', *desktop_args, '--all-targets', '--locked', '--offline', '--', '-D', 'warnings'],
            ['test', *WIDGET_ARGS, '--', '--list', '--format', 'terse'],
            ['test', *desktop_args, '--locked', '--offline', '--', '--format', 'pretty', '--color', 'never'],
        ])

    def test_default_also_requires_a_passing_widget(self):
        for case in ('ignored', 'unit-execution'):
            with self.subTest(case=case):
                result, calls = self.run_gate(case, args=())
                self.assertEqual(result.returncode, 1)
                self.assertEqual(len(calls), 5)
                self.assertIn('No Threads widget tests passed', result.stderr)

    def test_unknown_arguments_are_rejected_before_cargo(self):
        result, calls = self.run_gate(args=('--unknown',))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(calls, [])
        self.assertIn('Usage:', result.stderr)


if __name__ == '__main__':
    unittest.main()
