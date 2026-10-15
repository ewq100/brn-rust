"""Checks of the CLI journey runner itself, with stub executables instead of brn."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('cli_journey', ROOT / 'scripts/cli_journey.py')
cli_journey = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli_journey)


def envelope(command='notes', status='ok', exit_code=0, code=None):
    return {'contract': 1, 'command': command, 'ok': exit_code == 0, 'status': status, 'exit': exit_code,
            'outcome': None, 'operation': None, 'key': None, 'retry': 'refuse' if code else 'read',
            'changed': [], 'conflict': None, 'result': [],
            'error': {'code': code, 'message': 'synthetic', 'details': {}} if code else None}


def stub(directory, body):
    """An executable Python stub standing in for brn."""
    path = Path(directory) / 'brn-stub'
    path.write_text(f'#!{sys.executable}\nimport json, os, subprocess, sys, time\n{body}\n')
    path.chmod(0o755)
    return path


def printing(text, exit_code=0):
    return f'sys.stdout.write({text!r})\nsys.exit({exit_code})'


def one_call(command='notes', expect=0, data=True):
    def journey(brn, root):
        brn.call(command, data=root / 'data' if data else None, expect=expect)
    return journey


def quiet(function, *args, **kwargs):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        result = function(*args, **kwargs)
    return result, out.getvalue(), err.getvalue()


class ChildEnvironment(unittest.TestCase):
    def test_child_env_keeps_home_and_temp_inside_the_root_without_brn_variables(self):
        with tempfile.TemporaryDirectory() as root, \
                mock.patch.dict(os.environ, {'BRN_EMBEDDING_MODEL_DIR': '/x', 'BRN_INTAKE_HELPER': '/y'}):
            environment = cli_journey.child_env(root)
        self.assertFalse([name for name in environment if name.startswith('BRN_')])
        for name in ('TMPDIR', 'TMP', 'TEMP'):
            self.assertEqual(environment[name], str(Path(root) / 'tmp'))
        for name in ('HOME', 'CODEX_HOME'):
            self.assertEqual(environment[name], str(Path(root) / 'home'))


class ParentCheck(unittest.TestCase):
    def test_a_leak_into_the_parent_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as parent, tempfile.TemporaryDirectory() as bin_dir:
            parent = Path(parent).resolve()
            leak = parent / 'brn-threads-intake'
            body = f'os.makedirs({str(leak)!r}, exist_ok=True)\n' + printing(json.dumps(envelope()) + '\n')
            code, out, err = quiet(cli_journey.run, stub(bin_dir, body), parent=parent, journey=one_call())
            self.assertEqual(code, 1)
            self.assertIn('brn-threads-intake', err)
            self.assertEqual(json.loads(out)['leaked'], ['brn-threads-intake'])

    def test_scratch_under_the_child_tmpdir_stays_inside_the_root(self):
        with tempfile.TemporaryDirectory() as parent, tempfile.TemporaryDirectory() as bin_dir:
            parent = Path(parent).resolve()
            before = sorted(os.listdir(parent))
            body = ("os.makedirs(os.path.join(os.environ['TMPDIR'], 'brn-threads-intake'), exist_ok=True)\n"
                    + printing(json.dumps(envelope()) + '\n'))
            code, out, _ = quiet(cli_journey.run, stub(bin_dir, body), parent=parent, journey=one_call())
            self.assertEqual(code, 0)
            self.assertEqual(json.loads(out)['journey'], 'passed')
            self.assertEqual(sorted(os.listdir(parent)), before)


class EnvelopeChecks(unittest.TestCase):
    def failing(self, body, journey=None):
        with tempfile.TemporaryDirectory() as parent, tempfile.TemporaryDirectory() as bin_dir, \
                tempfile.TemporaryDirectory() as evidence:
            code, out, err = quiet(cli_journey.run, stub(bin_dir, body), parent=parent,
                                   evidence_dir=evidence, journey=journey or one_call())
            failure = json.loads((Path(evidence) / 'failure.json').read_text())
            transcript = (Path(evidence) / 'transcript.jsonl').read_text().splitlines()
        self.assertEqual(code, 1)
        self.assertEqual(json.loads(out)['journey'], 'failed')
        return failure, transcript, err

    def test_malformed_stdout_is_rejected_with_the_step_index(self):
        good = json.dumps(envelope()) + '\n'
        cases = {
            'not JSON': printing('not json\n'),
            'one line': printing(good + good),
            'differs': printing(json.dumps(envelope(exit_code=3, status='stale')) + '\n', 0),
            'unknown status': printing(json.dumps(envelope(status='weird', exit_code=9)) + '\n', 9),
        }
        for expected, body in cases.items():
            with self.subTest(expected):
                # One good call first, so the failing step is index 1.
                marker = 'marker'
                two_step = (f"m = os.path.join(os.environ['TMPDIR'], {marker!r})\n"
                            f"if not os.path.exists(m):\n    open(m, 'w').close()\n"
                            f"    sys.stdout.write({good!r}); sys.exit(0)\n" + body)

                def journey(brn, root):
                    brn.call('notes', data=root / 'data')
                    brn.call('notes', data=root / 'data', expect=lambda _: True)
                failure, transcript, err = self.failing(two_step, journey)
                self.assertEqual(failure['step'], 1)
                self.assertIn(expected, failure['check'])
                self.assertIn('step 1: notes', err)
                self.assertEqual(len(transcript), 2)
                self.assertNotIn('stdout', json.loads(transcript[0]))

    def test_null_command_is_accepted_only_for_pre_command_usage(self):
        usage = envelope(command=None, status='usage', exit_code=2, code='missing_data_dir')
        with tempfile.TemporaryDirectory() as parent, tempfile.TemporaryDirectory() as bin_dir:
            body = printing(json.dumps(usage) + '\n', 2)
            code, _, _ = quiet(cli_journey.run, stub(bin_dir, body), parent=parent,
                               journey=one_call(expect=2, data=False))
            self.assertEqual(code, 0)
        failure, _, _ = self.failing(printing(json.dumps(envelope(command=None)) + '\n'))
        self.assertIn('command is null', failure['check'])
        failure, _, _ = self.failing(printing(json.dumps(envelope(command='read')) + '\n'))
        self.assertIn("command 'read' differs", failure['check'])

    def test_a_hung_child_is_killed_with_its_process_group(self):
        with tempfile.TemporaryDirectory() as root, tempfile.TemporaryDirectory() as bin_dir:
            root = Path(root)
            (root / 'tmp').mkdir()
            (root / 'home').mkdir()
            pid_file = root / 'grandchild.pid'
            body = (f"child = subprocess.Popen(['sleep', '30'])\n"
                    f"open({str(pid_file)!r}, 'w').write(str(child.pid))\n"
                    f"time.sleep(30)")
            brn = cli_journey.Brn(stub(bin_dir, body), root, timeout=1)
            started = time.monotonic()
            with self.assertRaises(cli_journey.JourneyFailure) as raised:
                brn.call('notes', data=root / 'data')
            self.assertLess(time.monotonic() - started, 10)
            self.assertIn('timed out', str(raised.exception))
            self.assertEqual(raised.exception.step['index'], 0)
            grandchild = int(pid_file.read_text())
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                try:
                    os.kill(grandchild, 0)
                except ProcessLookupError:
                    break
                time.sleep(0.05)
            else:
                self.fail('the process group survived the timeout')


if __name__ == '__main__':
    unittest.main()
