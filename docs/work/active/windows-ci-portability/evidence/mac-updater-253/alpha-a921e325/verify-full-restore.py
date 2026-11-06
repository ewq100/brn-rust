import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile

REPO = Path('/Users/evokessler/Documents/Codex/brn-mac-updater-253')
OUTPUT = Path(__file__).resolve().parent
PACKAGE = OUTPUT.parents[1] / 'outputs/mac-alpha-a921e325/B/BRN Threads.app'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip() == 'a921e32533dc03134ed8a2001bd82f27194c3302'
with tempfile.TemporaryDirectory(prefix='brn-full-restore-a921-') as scratch:
    root = Path(scratch).resolve()
    home = root / 'home'
    home.mkdir()
    bindir = root / 'test-only-bin'
    bindir.mkdir()
    shutil.copy2(REPO / 'target/release/brn', bindir / 'brn')
    for helper in ('brn-xberg-helper', 'brn-xberg-helper-brn'):
        shutil.copy2(PACKAGE / 'Contents/Resources/bin' / helper, bindir / helper)
    env = {k: v for k, v in os.environ.items() if not k.startswith('BRN_')}
    env['HOME'] = str(home)
    workspace = root / 'source'
    restored = root / 'restored'
    calls = []
    def call(*args, data=workspace):
        result = subprocess.run([str(bindir / 'brn'), '--json', '--data-dir', str(data), *args],
                                env=env, capture_output=True, text=True, timeout=30)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        envelope = json.loads(result.stdout)
        assert envelope['status'] == 'ok', envelope
        calls.append(list(args))
        return envelope['result']
    call('init', '--purpose', 'test')
    note = call('note-create', '--title', 'Full restore synthetic note', '--text', 'Harbor level: 12 cm. UTF-8 õ ü.')
    project = call('project-create', '--title', 'Full restore synthetic project')
    project_id = next(w['after']['id'] for w in project['writes'] if 'Project' in w.get('after', {}).get('data', {}))
    call('project-link', '--project', project_id, '--note', note['id'])
    thread = call('thread-start', '--title', 'Full restore synthetic chat')
    call('queue', '--thread', thread['id'], '--text', 'Synthetic queued text, never dispatched')
    call('configure', '--provider', 'Copilot', '--model', 'synthetic-unavailable-model', '--effort', 'low')
    call('delegation', '--choice', 'review-first')
    token = root / 'synthetic-edit-token'
    edit = call('begin-edit', '--note', note['id'], '--base', '1', '--token-out', str(token))
    call('update-buffer', '--session', edit['id'], '--generation', '1', '--text', 'Unsaved synthetic draft', '--token-file', str(token))
    call('intake', '--source', str(REPO / 'crates/brn-intake/tests/fixtures/harbor.docx'),
         '--intent', 'full', '--title', 'Synthetic figure import', '--key', 'full-restore-figure')
    inspections = ('notes', 'records', 'history', 'settings', 'recovery', 'projects', 'threads')
    before = {name: call(name) for name in inspections}
    asset_count = sum('Asset' in r.get('data', {}) for r in before['records'])
    assert asset_count > 0
    backup = root / 'full-snapshot.sqlite3'
    call('backup', '--destination', str(backup))
    with sqlite3.connect(backup.as_uri() + '?mode=ro', uri=True) as connection:
        assert connection.execute('PRAGMA quick_check').fetchall() == [('ok',)]
    restore_result = call('restore', '--source', str(backup), data=restored)
    after = {name: call(name, data=restored) for name in inspections}
    compared = {name: before[name] == after[name] for name in inspections if name != 'recovery'}
    expected_recovery = json.loads(json.dumps(before['recovery']))
    # Public JSON spellings from EditHolder's serde renames in types.rs.
    assert expected_recovery and all(session['holder'] == 'caller' for session in expected_recovery)
    for session in expected_recovery:
        session['holder'] = 'desktop'
    compared['recovery_draft_bytes_and_metadata_preserved_owner_held'] = after['recovery'] == expected_recovery
    denied = subprocess.run([str(bindir / 'brn'), '--json', '--data-dir', str(restored),
                             'update-buffer', '--session', edit['id'], '--generation', '2',
                             '--text', 'Stale caller must not overwrite restored draft', '--token-file', str(token)],
                            env=env, capture_output=True, text=True, timeout=30)
    compared['original_caller_token_refused_without_draft_mutation'] = denied.returncode != 0 and call('recovery', data=restored) == after['recovery']
    report = {'selected_source': 'a921e32533dc03134ed8a2001bd82f27194c3302',
              'scope': 'Real selected release CLI and relocated signed helpers; full synthetic backup/restore, no native/provider session',
              'checks': compared, 'asset_count': asset_count, 'backup_quick_check': 'ok',
              'restore_result': restore_result, 'recovery_before': before['recovery'], 'recovery_after': after['recovery'],
              'expected_authority_change': 'restore.rs deletes original edit-session token bindings; draft remains owner-held',
              'snapshot_sha256': {name: hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest() for name, value in before.items()},
              'commands': calls}
    (OUTPUT / 'full-restore.json').write_text(json.dumps(report, indent=2) + '\n')
    assert all(compared.values()), compared
    print('Full synthetic records, notes, history, settings, recovery draft, projects, threads and figure assets preserved by backup/restore')
