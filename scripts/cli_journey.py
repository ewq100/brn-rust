#!/usr/bin/env python3
"""Maintained BRN CLI journey through the documented `brn --json` contract.

Usage: cli_journey.py BRN_BINARY [--evidence-dir ABSOLUTE_DIR] [--keep-root]

Every step runs the built `brn` with a leading `--json` on fresh synthetic
data under a private temporary root, and checks only the documented
envelope and description: no SQLite access, no reads under a data
directory, no provider calls (no `ask`, Markdown intake only). Text reaches
`brn` through files or stdin, so the transcript holds paths and SHA-256
digests, never content. `Brn`, `child_env()` and `FIXTURES` are reusable by
other witnesses.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

STATUSES = {'ok': 0, 'internal': 1, 'usage': 2, 'stale': 3, 'deferred': 4,
            'needs_review': 5, 'rejected': 6, 'failed': 7, 'interrupted': 130, 'terminated': 143}
RETRY_CLASSES = {'read', 'replay', 'converge', 'refuse', 'duplicate'}
# Usage refusals found before a known command was read report `command: null`.
NULL_COMMAND_CODES = {'missing_data_dir', 'invalid_data_dir', 'missing_command',
                      'unknown_command', 'malformed_option'}
# BRN's scratch namespace in a temporary parent.
LEAK = re.compile(r'^\.?brn-|^threads-intake-')
TIMEOUT_SECONDS = 60

FIXTURES = {
    'project_title': 'Harbor pilot',
    'document_title': 'Survey route',
    'document': '# Survey route\n\nThe curlew survey starts at the north jetty.\n',
    'note_title': 'Field plan “β”',
    'note': ('# Field plan\n\n$HOME ${PATH} $(whoami) `echo INJECTED`\r\n'
             '“curly” and "straight" quotes\n\tTabbed line\n👩‍💻 ❤️ مرحبا 你好\n'),
    'edit_a': '# Field plan\n\nRevision A.\n',
    'edit_b': '# Field plan\n\nRevision B from another session.\n',
    'edit_late': '# Field plan\n\nRevision A2, based on A.\n',
    'merged': '# Field plan\n\nRevision B from another session.\n\nRevision A2, merged.\n',
    'keyed_title': 'Keyed note',
    'keyed': 'Created once under a key.\n',
    'keyed_other': 'Different text under the same key.\n',
    'intake': '# Harbor\n\n| Step | Owner |\n|---|---|\n| Check | Ana |\n',
    'intake_title': 'Harbor process',
    'intake_other_title': 'Harbor process, renamed',
}


class JourneyFailure(Exception):
    """A failed check, with the step it belongs to."""

    def __init__(self, message, step=None):
        super().__init__(message)
        self.step = step


def child_env(root):
    """The parent environment without BRN_*, with home and temp dirs inside `root`."""
    root = Path(root)
    environment = {k: v for k, v in os.environ.items() if not k.startswith('BRN_')}
    home, tmp = root / 'home', root / 'tmp'
    environment.update(HOME=str(home), CODEX_HOME=str(home),
                       TMPDIR=str(tmp), TMP=str(tmp), TEMP=str(tmp))
    return environment


def option_name(name):
    return '--' + name.replace('_', '-')


class Brn:
    """Runs `brn --json` without a shell and checks every envelope."""

    def __init__(self, binary, root, timeout=TIMEOUT_SECONDS):
        self.binary = str(binary)
        self.root = Path(root)
        self.timeout = timeout
        self.steps = []
        self.checks = 0
        self.notes = []

    def check(self, condition, message, step=None):
        self.checks += 1
        if not condition:
            raise JourneyFailure(message, step)

    def _run(self, argv, stdin):
        started = time.monotonic()
        process = subprocess.Popen(
            argv, stdin=subprocess.PIPE if stdin is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=child_env(self.root),
            start_new_session=True)
        try:
            stdout, stderr = process.communicate(stdin, timeout=self.timeout)
            timed_out = False
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            timed_out = True
        return process.returncode, stdout, stderr, time.monotonic() - started, timed_out

    def _record(self, command, argv, stdin, code, stdout, stderr, seconds):
        step = {
            'index': len(self.steps), 'command': command, 'argv': argv[1:],
            'stdin_sha256': hashlib.sha256(stdin).hexdigest() if stdin is not None else None,
            'exit': code, 'seconds': round(seconds, 3), 'status': None, 'outcome': None,
            'operation': None, 'key': None, 'retry': None, 'error_code': None,
            'stderr_tail': stderr.decode('utf-8', 'replace')[-400:], 'stdout': stdout,
        }
        self.steps.append(step)
        return step

    def _one_line(self, step, stdout, code, timed_out):
        if timed_out:
            raise JourneyFailure(f'timed out after {self.timeout} s', step)
        text = stdout.decode('utf-8', 'replace')
        self.check(text.endswith('\n') and text.count('\n') == 1,
                   f'stdout is not exactly one line ({text.count(chr(10))} newlines)', step)
        try:
            return json.loads(text)
        except json.JSONDecodeError as error:
            raise JourneyFailure(f'stdout is not JSON: {error}', step) from None

    def call(self, command, *, data, stdin=None, expect=0, **options):
        """One envelope. `data=None` omits `--data-dir`; `expect` is an exit code or a predicate."""
        argv = [self.binary, '--json']
        if data is not None:
            argv += ['--data-dir', str(data)]
        argv.append(command)
        for name, value in options.items():
            argv += [option_name(name), str(value)]
        code, stdout, stderr, seconds, timed_out = self._run(argv, stdin)
        step = self._record(command, argv, stdin, code, stdout, stderr, seconds)
        envelope = self._one_line(step, stdout, code, timed_out)
        self.check(isinstance(envelope, dict), 'the envelope is not an object', step)
        step.update({name: envelope.get(name) for name in ('status', 'outcome', 'operation', 'key', 'retry')})
        step['error_code'] = (envelope.get('error') or {}).get('code')
        self.check(envelope.get('contract') == 2, 'contract is not 2', step)
        self.check('page' in envelope and 'stop' in envelope, 'the envelope lacks page or stop', step)
        self.check(envelope.get('exit') == code, f'exit {envelope.get("exit")} differs from {code}', step)
        self.check(envelope.get('ok') == (code == 0), 'ok does not match exit', step)
        self.check(STATUSES.get(envelope.get('status')) == code,
                   f'unknown status {envelope.get("status")!r} for exit {code}', step)
        self.check(envelope.get('retry') in RETRY_CLASSES, f'unknown retry {envelope.get("retry")!r}', step)
        error = envelope.get('error') or {}
        if envelope.get('command') is None:
            self.check(envelope.get('status') == 'usage' and error.get('code') in NULL_COMMAND_CODES,
                       'command is null outside a pre-command usage refusal', step)
        else:
            self.check(envelope.get('command') == command,
                       f'command {envelope.get("command")!r} differs from {command!r}', step)
        if callable(expect):
            self.check(expect(envelope), 'expectation failed', step)
        else:
            self.check(code == expect, f'expected exit {expect}, got {code}', step)
        return envelope

    def describe(self):
        """`brn --json --help`, with the one-line and timeout checks only."""
        argv = [self.binary, '--json', '--help']
        code, stdout, stderr, seconds, timed_out = self._run(argv, None)
        step = self._record('--help', argv, None, code, stdout, stderr, seconds)
        description = self._one_line(step, stdout, code, timed_out)
        self.check(code == 0, f'--json --help exited {code}', step)
        return description

    def ok(self, command, *, data, stdin=None, **options):
        return self.call(command, data=data, stdin=stdin, expect=0, **options)['result']


def created(envelope, kind):
    return next(c['id'] for c in envelope['changed'] if c['kind'] == kind and c['created'])


def journey(brn, root):
    """The representative journey; about 60 calls."""
    fixtures = root / 'fixtures'
    fixtures.mkdir()
    files = {}
    for name, text in FIXTURES.items():
        path = fixtures / f'{name}.md'
        path.write_bytes(text.encode())
        files[name] = str(path)
    check = brn.check
    workspace, usage_dir = root / 'workspace', root / 'never-created'
    restored = root / 'restored'

    # 1. Discovery.
    description = brn.describe()
    check(description.get('contract') == 2, 'description contract is not 2')
    commands = {c['name']: c for c in description['commands']}
    used = ['init', 'settings', 'project-create', 'project-document', 'note-create', 'read', 'begin-edit',
            'update-buffer', 'save', 'discard', 'search', 'history', 'undo', 'candidates', 'notes',
            'intake', 'backup', 'export', 'restore', 'projects']
    check(all(name in commands for name in used), 'a journey command is missing from the description')
    check(len(description['statuses']) == 10, 'description does not list 10 statuses')
    check(any(v['when'] == {'option': 'intent', 'equals': 'useful'} for v in commands['intake']['retry']['variants']),
          'intake lacks the --intent useful variant')
    owner = [name for name in used if commands[name]['owner_decision']]
    check(set(owner) == {'project-create', 'project-document', 'undo'}, f'unexpected owner decisions {owner}')
    brn.notes.append({'owner_decision_commands_run_by_test_harness_on_synthetic_data': owner})

    # 2. A fresh workspace with no credentials.
    brn.call('init', data=workspace)
    settings = brn.ok('settings', data=workspace)
    # A workspace the CLI creates has no credential source until `configure`.
    check(settings.get('auth_file') is None and settings.get('credentials_dir') is None,
          'a CLI-created workspace has a credential source')

    # 3-4. A keyed project and its document, from files.
    project_call = brn.call('project-create', data=workspace, title_file=files['project_title'], key='p1')
    replayed = brn.call('project-create', data=workspace, title_file=files['project_title'], key='p1')
    check(replayed['operation'] == project_call['operation'], 'keyed project-create did not replay')
    check(replayed['retry'] == 'replay', 'keyed project-create is not replay')
    project = created(project_call, 'Project')
    document = brn.call('project-document', data=workspace, project=project, title_file=files['document_title'],
                        text_file=files['document'], key='d1')
    document_id = created(document, 'Note')

    # 5. Multiline Unicode through stdin, read back byte for byte.
    note_call = brn.call('note-create', data=workspace, stdin=FIXTURES['note'].encode(),
                         title_file=files['note_title'], text_file='-')
    note = note_call['result']['id']
    read = brn.ok('read', data=workspace, id=note)['data']['Note']
    check(read['markdown'] == FIXTURES['note'] and read['title'] == FIXTURES['note_title'],
          'stdin text did not round-trip exactly')

    # 6. Edit A with identical retries.
    a = brn.ok('begin-edit', data=workspace, note=note, base=1)['id']
    brn.call('update-buffer', data=workspace, session=a, generation=1, text_file=files['edit_a'])
    retry = brn.call('update-buffer', data=workspace, session=a, generation=1, text_file=files['edit_a'])
    check(retry['outcome'] == 'Ignored', 'identical update-buffer retry was not Ignored')
    save_a = brn.call('save', data=workspace, session=a, generation=1)
    check(save_a['changed'][0]['version'] == 2, 'save A did not produce version 2')
    again = brn.call('save', data=workspace, session=a, generation=1)
    check(again['operation'] == save_a['operation'], 'save retry did not replay')

    # 7-8. Project boundary in search; history has the save.
    scoped = brn.ok('search', data=workspace, query='curlew', project=project)
    check(any(hit['note'] == document_id for hit in scoped['hits']), 'project search misses the document')
    general = brn.ok('search', data=workspace, query='curlew')
    check(all(hit['note'] != document_id for hit in general['hits']), 'general search shows the project document')
    history = brn.ok('history', data=workspace)
    check(any(receipt['operation'] == save_a['operation'] for receipt in history), 'history lacks the save')

    # 9. A stale save, and the open session it leaves.
    b = brn.ok('begin-edit', data=workspace, note=note, base=2)['id']
    brn.call('update-buffer', data=workspace, session=b, generation=1, text_file=files['edit_b'])
    brn.call('save', data=workspace, session=b, generation=1)
    a2 = brn.ok('begin-edit', data=workspace, note=note, base=2)['id']
    brn.call('update-buffer', data=workspace, session=a2, generation=1, text_file=files['edit_late'])
    stale = brn.call('save', data=workspace, session=a2, generation=1, expect=3)
    check(stale['outcome'] == 'Stale', 'save of A2 was not Stale')
    check(stale['conflict'] == [{'id': note, 'kind': 'Note', 'version': 3, 'archived': False, 'open_edit': True}],
          f'unexpected stale conflict {stale["conflict"]}')
    kept = stale['result']['session']['markdown']
    check(kept == FIXTURES['edit_late'], 'the stale result lost the unsaved text')
    busy = brn.call('begin-edit', data=workspace, note=note, base=3, expect=6)
    check(busy['error']['code'] == 'edit_session_open', 'second begin-edit was not edit_session_open')
    # Owner decision B3 part 1: the open session is never named; the runner
    # acts on A2 only because its own successful begin-edit returned it.
    check('session' not in busy['error']['details'] and a2 not in json.dumps(busy),
          'edit_session_open names the open session')
    check(busy['error']['details'] == {'note': note, 'base_version': 2}, 'edit_session_open details changed')

    # 10. Recover: discard A2, merge onto version 3, save version 4.
    brn.call('discard', data=workspace, session=a2, generation=1)
    c = brn.ok('begin-edit', data=workspace, note=note, base=stale['conflict'][0]['version'])['id']
    brn.call('update-buffer', data=workspace, session=c, generation=1, text_file=files['merged'])
    merged = brn.call('save', data=workspace, session=c, generation=1)
    check(merged['changed'][0]['version'] == 4, 'merged save did not produce version 4')
    check(brn.ok('read', data=workspace, id=note)['data']['Note']['markdown'] == FIXTURES['merged'],
          'merged text not saved')
    closed = brn.call('save', data=workspace, session=a2, generation=1, expect=3)
    check(closed['outcome'] == 'Closed', 'save on the discarded session was not Closed')

    # 11. A Deferred keyed undo, retried after the guard clears.
    d = brn.ok('begin-edit', data=workspace, note=note, base=4)['id']
    deferred = brn.call('undo', data=workspace, operation=merged['operation'], key='u1', expect=4)
    check(deferred['retry'] == 'replay' and deferred['operation'], 'deferred keyed undo lacks replay identity')
    check(deferred['conflict'][0]['kind'] == 'Note', 'deferred undo does not name the note')
    brn.call('discard', data=workspace, session=d, generation=0)
    undone = brn.call('undo', data=workspace, operation=merged['operation'], key='u1')
    check(undone['outcome'] == 'Applied' and undone['operation'] == deferred['operation'],
          'keyed undo did not apply the same compensation')
    replay = brn.call('undo', data=workspace, operation=merged['operation'], key='u1')
    check(replay['operation'] == deferred['operation'], 'keyed undo did not replay')
    check(brn.ok('read', data=workspace, id=note)['data']['Note']['markdown'] == FIXTURES['edit_b'],
          'undo did not restore revision B')
    candidates = brn.ok('candidates', data=workspace)
    check(all(c['operation'] != deferred['operation'] for c in candidates), 'the compensation is left unapplied')

    # 12. Keyed creates replay; reuse with other inputs is refused.
    before = len(brn.ok('notes', data=workspace))
    first = brn.call('note-create', data=workspace, title_file=files['keyed_title'], text_file=files['keyed'], key='n1')
    second = brn.call('note-create', data=workspace, title_file=files['keyed_title'], text_file=files['keyed'], key='n1')
    check(first['operation'] == second['operation'], 'keyed note-create did not replay')
    check(len(brn.ok('notes', data=workspace)) == before + 1, 'keyed note-create made more than one note')
    conflict = brn.call('note-create', data=workspace, title_file=files['keyed_title'],
                        text_file=files['keyed_other'], key='n1', expect=6)
    check(conflict['error']['code'] == 'key_conflict' and conflict['retry'] == 'refuse', 'key reuse not refused')
    check(len(brn.ok('notes', data=workspace)) == before + 1, 'refused key reuse changed notes')
    unkeyed = brn.call('note-create', data=workspace, title_file=files['keyed_title'], text_file=files['keyed'])
    check(unkeyed['retry'] == 'duplicate', 'unkeyed note-create is not duplicate')

    # 13. Keyed Markdown intake replays; a new title under the key is refused.
    imported = brn.call('intake', data=workspace, source=files['intake'], intent='full',
                        title_file=files['intake_title'], key='i1')
    check(imported['outcome'] == 'Applied', 'keyed intake did not apply')
    reimported = brn.call('intake', data=workspace, source=files['intake'], intent='full',
                          title_file=files['intake_title'], key='i1')
    check(reimported['operation'] == imported['operation'], 'keyed intake did not replay')
    renamed = brn.call('intake', data=workspace, source=files['intake'], intent='full',
                       title_file=files['intake_other_title'], key='i1', expect=6)
    check(renamed['error']['code'] == 'key_conflict', 'retitled keyed intake was not key_conflict')

    # 14. Usage refusals create nothing.
    missing = brn.call('note-create', data=None, title_file=files['keyed_title'], expect=2)
    check(missing['error']['code'] == 'missing_data_dir' and missing['command'] is None, 'missing data dir')
    mixed = brn.call('note-create', data=usage_dir, title_file=files['keyed_title'], text='x',
                     text_file=files['keyed'], expect=2)
    check(mixed['error']['code'] == 'conflicting_options', '--text with --text-file was not refused')
    bogus = brn.call('notes', data=usage_dir, bogus='x', expect=2)
    check(bogus['error']['code'] == 'unknown_option' and bogus['error']['details']['option'] == 'bogus',
          'unknown option not reported')
    check(not usage_dir.exists(), 'a usage refusal created the data directory')

    # 15. Not found.
    absent = brn.call('read', data=workspace, id='no-such-record', expect=6)
    check(absent['error']['code'] == 'not_found', 'unknown read was not not_found')

    # 16. Backup, export, restore.
    notes = brn.ok('notes', data=workspace)
    projects = brn.ok('projects', data=workspace)
    backup, export = root / 'backup.sqlite3', root / 'export'
    brn.call('backup', data=workspace, destination=backup)
    occupied = brn.call('backup', data=workspace, destination=backup, expect=6)
    check(occupied['error']['code'] == 'destination_exists', 'second backup was not destination_exists')
    brn.call('export', data=workspace, destination=export)
    occupied = brn.call('export', data=workspace, destination=export, expect=6)
    check(occupied['error']['code'] == 'destination_exists', 'second export was not destination_exists')
    brn.call('restore', data=restored, source=backup)
    check(brn.ok('notes', data=restored) == notes, 'restored notes differ')
    check(brn.ok('projects', data=restored) == projects, 'restored projects differ')
    copy = brn.ok('search', data=restored, query='curlew', project=project)
    check(any(hit['note'] == document_id for hit in copy['hits']), 'project search fails on the restored copy')
    again = brn.call('restore', data=restored, source=backup, expect=6)
    check(again['error']['code'] == 'unsafe_directory', 'restore retry was not unsafe_directory')


def failure_report(brn, error, root):
    step = error.step if error.step is not None else (brn.steps[-1] if brn.steps else None)
    summary = {'journey': 'failed', 'check': str(error), 'steps': len(brn.steps), 'checks': brn.checks,
               'root': str(root)}
    lines = [f'cli journey failed: {error}', f'journey root: {root}']
    if step is not None:
        summary['step'] = step['index']
        summary['envelope'] = {k: step.get(k) for k in ('status', 'outcome', 'operation', 'error_code', 'retry')}
        stdout = step.get('stdout', b'')
        lines += [f'step {step["index"]}: {step["command"]}', f'argv: {step["argv"]}', f'exit: {step["exit"]}',
                  f'stdout: {stdout.decode("utf-8", "replace") if isinstance(stdout, bytes) else stdout}',
                  f'stderr: {step["stderr_tail"]}']
    lines.append('last steps:')
    lines += [json.dumps(public(s)) for s in brn.steps[-5:]]
    print('\n'.join(lines), file=sys.stderr)
    return summary


def public(step):
    return {k: v for k, v in step.items() if k != 'stdout'}


def run(binary, *, evidence_dir=None, keep_root=False, parent=None, journey=journey, timeout=TIMEOUT_SECONDS):
    """Runs `journey` in a private root under `parent`; returns the exit code."""
    parent = Path(parent or tempfile.gettempdir()).resolve()
    before = set(os.listdir(parent))
    root = Path(tempfile.mkdtemp(prefix='brn-cli-journey-', dir=parent)).resolve()
    (root / 'tmp').mkdir(mode=0o700)
    (root / 'home').mkdir()
    brn = Brn(binary, root, timeout)
    summary = None
    try:
        journey(brn, root)
    except JourneyFailure as error:
        summary = failure_report(brn, error, root)
    finally:
        if evidence_dir:
            evidence = Path(evidence_dir)
            evidence.mkdir(parents=True, exist_ok=True)
            with open(evidence / 'transcript.jsonl', 'w') as transcript:
                for note in brn.notes:
                    transcript.write(json.dumps(note) + '\n')
                for step in brn.steps:
                    transcript.write(json.dumps(public(step)) + '\n')
            if summary is not None:
                (evidence / 'failure.json').write_text(json.dumps(summary, indent=2) + '\n')
        if not keep_root:
            shutil.rmtree(root, ignore_errors=True)
    added = set(os.listdir(parent)) - before
    leaked = sorted(name for name in added if LEAK.match(name) and not (keep_root and name == root.name))
    if leaked:
        print(f'cli journey left entries in {parent}: {", ".join(leaked)}', file=sys.stderr)
        summary = summary or {'journey': 'failed', 'check': 'temporary parent not left clean'}
        summary['leaked'] = leaked
    if summary is not None:
        print(json.dumps(summary, separators=(',', ':')))
        return 1
    print(json.dumps({'journey': 'passed', 'steps': len(brn.steps), 'checks': brn.checks}, separators=(',', ':')))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('binary', type=Path)
    parser.add_argument('--evidence-dir', type=Path)
    parser.add_argument('--keep-root', action='store_true')
    args = parser.parse_args(argv)
    if not args.binary.is_absolute() or not os.access(args.binary, os.X_OK):
        parser.error('BRN_BINARY must be an absolute path to an executable')
    if args.evidence_dir is not None and not args.evidence_dir.is_absolute():
        parser.error('--evidence-dir must be absolute')
    return run(args.binary, evidence_dir=args.evidence_dir, keep_root=args.keep_root)


if __name__ == '__main__':
    sys.exit(main())
