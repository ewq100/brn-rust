"""Synthetic tooling regression checks; no Cargo, account or private data access."""
import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import verification_evidence as evidence


def module(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), SCRIPTS / f'{name}.py')
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


links = module('check-markdown-links')
ci = module('ci-summary')


class MarkdownTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()

    def tearDown(self):
        self.temp.cleanup()

    def test_fences_inline_samples_and_reference_links(self):
        target = self.root / 'target.md'
        target.write_text('# Real\n# Real\n# Real-1\n# Näide 日本語\n')
        page = self.root / 'page.md'
        page.write_text('''# Page
`[sample](absent.md)`
````markdown
[example](absent.md)
```
[still sample](absent.md)
````
~~~
[example](absent.md)
~~~
[ok](target.md#real-1-1)
[unicode](target.md#näide-日本語)
[reference][ref]
[ref]: target.md#real
''')
        errors, count = links.check(self.root, [page])
        self.assertEqual(errors, [])
        self.assertGreaterEqual(count, 3)

    def test_collects_all_missing_files_and_fragments(self):
        target = self.root / 'target.md'
        target.write_text('# Present\n')
        page = self.root / 'page.md'
        page.write_text('[file](missing.md)\n[fragment](target.md#absent)\n[root](/also-missing.md)\n- parent\n    - [nested](nested-missing.md)\n')
        errors, count = links.check(self.root, [page])
        self.assertEqual(count, 4)
        self.assertEqual(len(errors), 4)

    def test_balanced_paths_encoded_spaces_and_html_anchor(self):
        target = self.root / 'some (file).md'
        target.write_text('Heading `code`\n===\n<a id="custom"></a>\n')
        page = self.root / 'page.md'
        page.write_text('[ok](some%20(file).md#heading-code)\n[angle](<some (file).md#custom> "title")\n')
        self.assertEqual(links.check(self.root, [page]), ([], 2))

    def test_nested_labels_check_outer_links_and_inner_images(self):
        page = self.root / 'page.md'
        page.write_text('[![badge](https://example.invalid/badge.svg)](missing.md)\n[a [b]](another-missing.md)\n[![local](missing-image.png)](third-missing.md)\n')
        errors, count = links.check(self.root, [page])
        self.assertEqual(count, 4)
        self.assertEqual(len(errors), 4)
        for destination in ('missing.md', 'another-missing.md', 'missing-image.png', 'third-missing.md'):
            self.assertTrue(any(f': {destination}:' in error for error in errors))

    def test_blockquoted_fences_exclude_samples_without_hiding_real_links(self):
        page = self.root / 'page.md'
        page.write_text('> ~~~markdown\n> [example](missing.md)\n> ~~~\n> > ~~~markdown\n> > [nested example](missing.md)\n> > ~~~\n> [actual](real-missing.md)\n')
        errors, count = links.check(self.root, [page])
        self.assertEqual(count, 1)
        self.assertEqual(len(errors), 1)
        self.assertIn('real-missing.md', errors[0])

    def test_heading_link_destinations_with_parentheses_do_not_pollute_slug(self):
        (self.root / 'file(1).md').write_text('# Destination\n')
        page = self.root / 'page.md'
        page.write_text('# a [b](file(1).md)\n[self](#a-b)\n')
        self.assertEqual(links.check(self.root, [page]), ([], 2))

    def test_explicit_historical_policy_still_validates_incoming(self):
        historical = self.root / 'docs/work/completed/old'
        historical.mkdir(parents=True)
        old = historical / 'plan.md'
        old.write_text('# Kept\n[old bad](missing.md)\n')
        page = self.root / 'README.md'
        page.write_text('[kept](docs/work/completed/old/plan.md#kept)\n[bad](docs/work/completed/old/plan.md#absent)\n')
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        self.assertEqual(links.current_paths(self.root), {page})
        self.assertEqual(len(links.check(self.root, [page])[0]), 1)


class CiTests(unittest.TestCase):
    def run_info(self, **updates):
        result = dict(id=12, run_attempt=2, head_sha='a'*40, event='pull_request', status='completed', conclusion='failure')
        result.update(updates)
        return result

    def test_failed_informational_platform_remains_red(self):
        jobs = [dict(name=name, conclusion='success', status='completed') for name in ci.APPLICABLE]
        jobs.append(dict(name='Core and CLI (windows-2025)', conclusion='failure', status='completed'))
        report, ready = ci.summarize(self.run_info(), jobs, 'a'*40)
        self.assertTrue(ready)
        self.assertIn('overall: failure', report)
        self.assertIn('informational platform probe | failure', report)
        with self.assertRaises(ValueError):
            ci.summarize(self.run_info(), jobs, 'b'*40)

    def test_missing_cancelled_or_failed_applicable_checks_do_not_pass(self):
        for conclusion in ('failure', 'cancelled', None):
            jobs = [dict(name=name, conclusion=conclusion, status='in_progress') for name in ci.APPLICABLE]
            self.assertFalse(ci.summarize(self.run_info(), jobs, 'a'*40)[1])
        self.assertFalse(ci.summarize(self.run_info(), [], 'a'*40)[1])

    def test_main_exit_requires_overall_success_and_all_applicable_ready(self):
        for overall, job_result, expected in (
            ('success', 'success', 0), ('success', 'skipped', 1),
            ('success', None, 1), ('failure', 'success', 1),
        ):
            with self.subTest(overall=overall, job_result=job_result):
                jobs = [dict(name=name, conclusion='success', status='completed') for name in ci.APPLICABLE]
                jobs[0].update(conclusion=job_result, status='in_progress' if job_result is None else 'completed')
                replies = [self.run_info(conclusion=overall), {'jobs': jobs}]
                argv = ['ci-summary.py', 'run', '--run', '12', '--attempt', '2', '--commit', 'a'*40]
                with patch.object(sys, 'argv', argv), patch.object(ci, 'gh_json', side_effect=replies), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(ci.main(), expected)
        with patch.object(sys, 'argv', argv), patch.object(ci, 'gh_json', side_effect=[self.run_info(conclusion='success'), {'jobs': []}]), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(ci.main(), 1)

    def test_comparison_keeps_error_assertion_and_backtrace_differences(self):
        before = '2026-10-05T11:00:00.123Z \x1b[31merror[E0425]: bad name\x1b[0m\nassertion left: 1\n  3: 0x12345678 - caller::first pid=42\n'
        equivalent = '2026-10-06T11:00:00Z error[E0425]: bad name\nassertion left: 1\n  3: 0xabcdef12 - caller::first pid=77\n'
        self.assertEqual(ci.normalize_log(before), ci.normalize_log(equivalent))
        for changed in (equivalent.replace('E0425', 'E0432'), equivalent.replace('left: 1', 'left: 2'), equivalent.replace('caller::first', 'caller::second')):
            self.assertNotEqual(ci.normalize_log(before), ci.normalize_log(changed))
        self.assertNotEqual(ci.normalize_log('a\nb\n'), ci.normalize_log('b\na\n'))
        self.assertNotEqual(ci.normalize_log('assertion left: 0xdeadbeef'), ci.normalize_log('assertion left: 0x12345678'))


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve() / 'checkout'
        self.root.mkdir()
        (self.root / 'scripts').mkdir()
        self.output = Path(self.temp.name).resolve() / 'evidence'
        self.output.mkdir()
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        subprocess.run(['git', '-C', str(self.root), 'config', 'user.email', 'synthetic@example.invalid'], check=True)
        subprocess.run(['git', '-C', str(self.root), 'config', 'user.name', 'Synthetic'], check=True)
        (self.root / 'tracked').write_text('baseline')
        subprocess.run(['git', '-C', str(self.root), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.root), 'commit', '-qm', 'Synthetic baseline'], check=True)

    def tearDown(self):
        self.temp.cleanup()

    def launch(self, body):
        (self.root / 'scripts/verify-end-to-end.sh').write_text(body)
        script = f'import sys; from pathlib import Path; sys.path.insert(0,{str(SCRIPTS)!r}); import verification_evidence as e; sys.exit(e.record_gate("verify-end-to-end.sh", ["--retirement-only"], {str(self.output)!r}, Path({str(self.root)!r})))'
        return subprocess.Popen([sys.executable, '-c', script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def result(self):
        return json.loads(next(self.output.glob('*/record.json')).read_text())

    def test_pass_failure_exact_identity_and_atomic_publication(self):
        for expected in (0, 7):
            child = self.launch(f'echo synthetic\nexit {expected}\n')
            stdout, stderr = child.communicate(timeout=10)
            self.assertEqual(child.returncode, expected, stderr)
            record = self.result()
            self.assertEqual(record['terminal'], 'passed' if expected == 0 else 'failed')
            self.assertEqual(record['exit_code'], expected)
            self.assertTrue(record['identity']['dirty'])
            self.assertEqual(len(record['identity']['dirty_snapshot_sha256']), 64)
            self.assertTrue(record['start_utc'] and record['end_utc'])
            self.assertFalse(record['checkout_changed'])
            self.assertEqual(record['command'][-1], '--retirement-only')
            self.assertFalse(list(self.output.glob('*/.record-*')))
            self.assertIn(b'synthetic', stdout)
            # Keep each iteration's evidence isolated.
            self.output = Path(self.temp.name).resolve() / f'evidence-{expected}'
            self.output.mkdir()

    @unittest.skipIf(os.name == 'nt', 'Unix product gate signal behavior')
    def test_interrupted_never_passes_and_initial_record_is_running(self):
        child = self.launch('echo started\nsleep 30\n')
        deadline = time.monotonic() + 5
        while not list(self.output.glob('*/record.json')) and time.monotonic() < deadline:
            time.sleep(.02)
        self.assertEqual(self.result()['terminal'], 'running')
        self.assertIsNone(self.result()['exit_code'])
        child.send_signal(signal.SIGTERM)
        child.communicate(timeout=10)
        self.assertEqual(child.returncode, 143)
        self.assertEqual(self.result()['terminal'], 'interrupted')
        self.assertEqual(self.result()['exit_code'], 143)

    def test_checkout_changes_are_explicit_and_cannot_reuse_start_identity(self):
        child = self.launch('echo changed > tracked\n')
        _stdout, stderr = child.communicate(timeout=10)
        self.assertEqual(child.returncode, 0, stderr)
        record = self.result()
        self.assertTrue(record['checkout_changed'])
        self.assertNotEqual(record['identity'], record['end_identity'])

    def test_untracked_contents_invalidate_snapshot(self):
        path = self.root / 'untracked'
        path.write_text('one')
        before = evidence.identity(self.root)
        path.write_text('two')
        self.assertNotEqual(before['dirty_snapshot_sha256'], evidence.identity(self.root)['dirty_snapshot_sha256'])

    def test_refuses_arbitrary_gate_and_checkout_output(self):
        with self.assertRaises(ValueError):
            evidence.record_gate('anything.sh', [], str(self.output), self.root)
        with self.assertRaises(ValueError):
            evidence.record_gate('verify-end-to-end.sh', [], str(self.root), self.root)
        with self.assertRaises(ValueError):
            evidence.record_gate('verify-end-to-end.sh', [], 'relative', self.root)


if __name__ == '__main__':
    unittest.main()
