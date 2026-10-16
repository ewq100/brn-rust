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


def envelope(command='notes', status='ok', exit_code=0, code=None, details=None, authority=True):
    value = {'contract': 3, 'command': command, 'ok': exit_code == 0, 'status': status, 'exit': exit_code,
             'outcome': None, 'operation': None, 'key': None, 'retry': 'refuse' if code else 'read',
             'changed': [], 'conflict': None, 'result': [],
             'error': {'code': code, 'message': 'synthetic', 'details': details or {}} if code else None,
             'page': None, 'stop': None}
    if authority:
        value['authority'] = {'workspace_purpose': 'test', 'cli_authority': 'full', 'enforced': False,
                              'actor': None}
    return value


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
            'contract is not 3': printing(json.dumps({**envelope(), 'contract': 2}) + '\n'),
            'lacks authority': printing(json.dumps(envelope(authority=False)) + '\n'),
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
        # A freshly written stub can take seconds to start on a busy host. An
        # attempt whose stub was killed before it spawned the grandchild still
        # proves the timeout, but not the group kill, so it is repeated.
        for _ in range(5):
            with tempfile.TemporaryDirectory() as root, tempfile.TemporaryDirectory() as bin_dir:
                root = Path(root)
                (root / 'tmp').mkdir()
                (root / 'home').mkdir()
                pid_file = root / 'grandchild.pid'
                body = (f"child = subprocess.Popen(['sleep', '30'])\n"
                        f"open({str(pid_file)!r}, 'w').write(str(child.pid))\n"
                        f"time.sleep(30)")
                brn = cli_journey.Brn(stub(bin_dir, body), root, timeout=2)
                started = time.monotonic()
                with self.assertRaises(cli_journey.JourneyFailure) as raised:
                    brn.call('notes', data=root / 'data')
                self.assertLess(time.monotonic() - started, 10)
                self.assertIn('timed out', str(raised.exception))
                self.assertEqual(raised.exception.step['index'], 0)
                written = pid_file.read_text() if pid_file.exists() else ''
                if not written:
                    continue
                grandchild = int(written)
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        os.kill(grandchild, 0)
                    except ProcessLookupError:
                        return
                    time.sleep(0.05)
                self.fail('the process group survived the timeout')
        self.fail('the stub never spawned its grandchild before the timeout')


class RoundTwoHelperTests(unittest.TestCase):
    def test_contract_3_statuses_and_option_names(self):
        self.assertEqual(cli_journey.STATUSES['terminated'], 143)
        self.assertEqual(cli_journey.option_name('text_file'), '--text-file')
        self.assertEqual(cli_journey.option_name('command_'), '--command')
        # Round 3: tokens travel by file, never by value.
        self.assertEqual(cli_journey.option_name('token_file'), '--token-file')
        self.assertEqual(cli_journey.option_name('token_out'), '--token-out')
        self.assertEqual(cli_journey.option_name('purpose'), '--purpose')
        self.assertEqual(cli_journey.option_name('actor'), '--actor')

    def test_a_stale_token_refusal_is_an_expected_rejected_envelope(self):
        refused = envelope(command='update-buffer', status='rejected', exit_code=6,
                           code='session_token_invalid', details={'reason': 'mismatch'})
        with tempfile.TemporaryDirectory() as parent, tempfile.TemporaryDirectory() as bin_dir:
            def journey(brn, root):
                result = brn.call('update-buffer', data=root / 'data', session='s', generation=1,
                                  text='x', token_file=root / 'old.token', expect=6)
                brn.check(result['error']['details'] == {'reason': 'mismatch'}, 'mismatch not reported')
                argv = brn.steps[-1]['argv']
                brn.check('--token-file' in argv and not any(a.startswith('brnet1_') for a in argv),
                          'the token reached argv')
            code, out, _ = quiet(cli_journey.run, stub(bin_dir, printing(json.dumps(refused) + '\n', 6)),
                                 parent=parent, journey=journey)
        self.assertEqual(code, 0, out)

    def test_pages_follow_the_cursor_and_require_a_mirrored_page(self):
        class Paged(cli_journey.Brn):
            def __init__(self):
                self.checks = 0
                self.seen = []
            def call(self, command, *, data, stdin=None, expect=0, **options):
                self.seen.append(options.get('cursor'))
                cursor = {None: '2', '2': None}[options.get('cursor')]
                items = [options.get('cursor') or '1']
                return {'result': {'items': items, 'cursor': cursor},
                        'page': {'limit': 1, 'shown': 1, 'cursor': cursor, 'total': None}}
        brn = Paged()
        self.assertEqual(brn.pages('history', 'items', data='/d', limit=1), ['1', '2'])
        self.assertEqual(brn.seen, [None, '2'])


class SamePages(cli_journey.Brn):
    """Two pages of `[1]` then `[2]`; the unpaged call returns `unpaged`."""

    def __init__(self, unpaged):
        self.checks = 0
        self.unpaged = unpaged

    def call(self, command, *, data, stdin=None, expect=0, **options):
        if 'limit' not in options:
            return {'result': self.unpaged}
        cursor = {None: '1', '1': None}[options.get('cursor')]
        items = [2] if options.get('cursor') else [1]
        return {'result': {'notes': items, 'cursor': cursor},
                'page': {'limit': 1, 'shown': 1, 'cursor': cursor, 'total': None}}


class RoundTwoBHelperTests(unittest.TestCase):
    def test_same_pages_accepts_matching_pages(self):
        brn = SamePages([1, 2])
        self.assertEqual(brn.same_pages('notes', 'notes', data='/d', limit=1), [1, 2])

    def test_same_pages_fails_when_pages_differ_from_the_list(self):
        for unpaged in ([1], [2, 1], [1, 2, 3]):
            with self.subTest(unpaged):
                with self.assertRaises(cli_journey.JourneyFailure) as raised:
                    SamePages(unpaged).same_pages('notes', 'notes', data='/d', limit=1)
                self.assertIn('notes pages differ from the unpaged list', str(raised.exception))

if __name__ == '__main__':
    unittest.main()
