import contextlib
import datetime
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import types

LEAD = Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust')
WORKER = Path('/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust')
OUT = Path('/tmp/brn-active-overview-qualification')
HEAD = '7ccb552efc152443f1ae57a044673500e7fe91c3'
BASE = 'e918b2b303f44cfca6cbd49347ea5b4fd4e335f1'
INPUT_COMMIT = '8d91c86d8978e59b4d22d936f7807555afa64b65'
RECORD = 'docs/work/active/vacation-campaign/campaign.json'
START = time.monotonic()
steps = []
receipt = {'started_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
           'candidate': HEAD, 'base': BASE, 'input_commit': INPUT_COMMIT,
           'qualified': False, 'steps': steps,
           'limits': 'Python/documentation qualification only. Branch snapshot and current lead authority stay separate. No Rust, native, account, provider, hosted CI or integration proof; byte counts are not token/cost/speed measurements.'}

def sha(data):
    return hashlib.sha256(data).hexdigest()

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args])

def run(label, args):
    began = time.monotonic()
    p = subprocess.run(args, cwd=WORKER, env=env, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=180)
    for suffix, data in [('stdout', p.stdout), ('stderr', p.stderr)]:
        assert len(data) <= 2 * 1024 * 1024, (label, suffix, len(data))
        (OUT / (label + '.' + suffix + '.log')).write_bytes(data)
    steps.append({'name': label, 'command': args, 'returncode': p.returncode,
                  'duration_seconds': round(time.monotonic() - began, 3),
                  'stdout_bytes': len(p.stdout), 'stderr_bytes': len(p.stderr),
                  'stdout_sha256': sha(p.stdout), 'stderr_sha256': sha(p.stderr)})
    assert p.returncode == 0, (label, p.returncode, p.stderr[-2000:])
    return p

try:
    assert shutil.disk_usage(OUT).free > 1024 ** 3
    assert git(WORKER, 'rev-parse', 'HEAD').decode().strip() == HEAD
    assert not git(WORKER, 'status', '--porcelain').strip()
    env = dict(os.environ)
    temporary = OUT / 'tmp'
    temporary.mkdir()
    env['TMPDIR'] = str(temporary)
    source_hashes = {p: sha((WORKER / p).read_bytes()) for p in
                     ['scripts/render-work-overview.py', 'scripts/tests/test_work_overview.py', 'docs/work/overview.md']}
    receipt['source_hashes'] = source_hashes
    branch_record = (WORKER / RECORD).read_bytes()
    assert branch_record == git(WORKER, 'show', BASE + ':' + RECORD)
    receipt['branch_record_sha256'] = sha(branch_record)
    raw = git(LEAD, 'show', INPUT_COMMIT + ':' + RECORD)
    frozen_record = OUT / 'source-record.json'
    frozen_record.write_bytes(raw)
    receipt['input_record_sha256'] = sha(raw)
    view = json.loads(raw)['work_view']

    old = types.ModuleType('before_renderer')
    old.__file__ = str(LEAD / 'scripts/render-work-overview.py')
    exec(compile(git(WORKER, 'show', BASE + ':scripts/render-work-overview.py'),
                 old.__file__, 'exec'), old.__dict__)
    spec = importlib.util.spec_from_file_location('after_renderer', WORKER / 'scripts/render-work-overview.py')
    new = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(new)
    old_text = old.render(view, LEAD / 'docs/work/overview.md', LEAD / RECORD)
    new_text = new.render(view, LEAD / 'docs/work/overview.md', LEAD / RECORD)
    (OUT / 'before-overview.md').write_text(old_text)
    (OUT / 'after-overview.md').write_text(new_text)
    receipt['same_input_projection'] = {'before_utf8_bytes': len(old_text.encode()),
                                       'after_utf8_bytes': len(new_text.encode()),
                                       'before_sha256': sha(old_text.encode()),
                                       'after_sha256': sha(new_text.encode()),
                                       'task_count': len(view['tasks'])}
    assert len(new_text.encode()) < len(old_text.encode())
    selected_id = 'sqlite-cold-start-crash'
    selected = next(t for t in view['tasks'] if t['id'] == selected_id)
    target = OUT / 'uncreated-owner-view' / 'overview.md'
    assert not target.parent.exists()
    script = str(WORKER / 'scripts/render-work-overview.py')
    args = [sys.executable, script, '--record', str(frozen_record), '--output', str(target)]
    query = run('actual-selected-task', args + ['--task', selected_id])
    assert json.loads(query.stdout) == selected
    assert not target.parent.exists()
    assert frozen_record.read_bytes() == raw
    receipt['detail_lookup'] = {'task': selected_id, 'exact_object': True,
                                'evidence_items': len(selected['evidence']),
                                'attempts': len(selected['attempts']),
                                'input_unchanged': True, 'output_parent_uncreated': True}
    run('actual-custom-render', args)
    run('actual-custom-check', args + ['--check'])
    assert frozen_record.read_bytes() == raw

    tests = run('development-tooling-tests', [sys.executable, '-m', 'unittest', 'discover', '-s', 'scripts/tests', '-v'])
    count = re.search(rb'Ran (\d+) tests?', tests.stderr)
    assert count and int(count.group(1)) > 0
    receipt['tests_run'] = int(count.group(1))
    run('branch-overview-check', [sys.executable, 'scripts/render-work-overview.py', '--check'])
    run('markdown-links', [sys.executable, 'scripts/check-markdown-links.py'])
    run('whitespace', ['git', 'diff', '--check', BASE, HEAD])
    assert source_hashes == {p: sha((WORKER / p).read_bytes()) for p in source_hashes}
    assert (WORKER / RECORD).read_bytes() == branch_record
    assert git(WORKER, 'rev-parse', 'HEAD').decode().strip() == HEAD
    assert not git(WORKER, 'status', '--porcelain').strip()
    receipt.update(qualified=True, source_unchanged=True, branch_record_unchanged=True,
                   worker_clean=True)
except BaseException as error:
    receipt['error'] = repr(error)
    raise
finally:
    receipt['duration_seconds'] = round(time.monotonic() - START, 3)
    receipt['finished_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (OUT / 'qualification.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))
