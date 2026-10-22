import datetime, hashlib, json, os, pathlib, subprocess, sys, time
from zoneinfo import ZoneInfo

ROOT = pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/codex-targeted-edit-evidence/2026-10-22-local-edit-01')
CLI = pathlib.Path('/Users/evokessler/.codex/artifacts/brn-targeted-edit-2ecf634d/brn')
LEDGER = pathlib.Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/docs/work/active/vacation-campaign/evidence/codex-ai-test-ledger.json')
SOURCE = '2ecf634d7de93fbf7381317168109b6d199531e4'
FROZEN_SHA256 = '61366a4eff077c9ab6fba0d37182e6d4f76f1335c986aee9685485dd603fe0c5'
SESSION = '2026-10-22-local-edit-01'
AUTH = pathlib.Path('/Users/evokessler/.codex/auth.json')
ORIGINAL = '# Harbors\n\nÕun stays beside the pier.\nThe cobalt boats cross the bay at dawn.\n\n尾灯 remain unchanged.\n'
EXPECTED = ORIGINAL.replace('cobalt boats', 'teal boats')
SENTINEL = 'Unrelated synthetic record: amber lantern remains unchanged.\n'

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save(name, value):
    (ROOT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')

def call(label, args, timeout=30):
    command = [str(CLI), '--json', '--data-dir', str(ROOT / 'workspace'), *args]
    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = '/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/lib'
    env['TMPDIR'] = str(ROOT / 'tmp')
    started = time.monotonic()
    try:
        result = subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        (ROOT / (label + '.stdout.partial')).write_text((error.stdout or b'').decode(errors='replace').replace(str(AUTH), '[existing Codex account file]'))
        (ROOT / (label + '.stderr.partial')).write_text((error.stderr or b'').decode(errors='replace').replace(str(AUTH), '[existing Codex account file]'))
        save(label + '.command.json', {'args': [s.replace(str(AUTH), '[existing Codex account file]') for s in command], 'exit': None, 'outer_timeout': True, 'outer_watchdog_seconds': timeout, 'supported_client_deadline_seconds': 180 if label == 'ask' else None, 'elapsed_seconds': time.monotonic() - started, 'upstream_cancellation_confirmed': False})
        raise
    stdout = result.stdout.decode().replace(str(AUTH), '[existing Codex account file]')
    stderr = result.stderr.decode().replace(str(AUTH), '[existing Codex account file]')
    (ROOT / (label + '.stdout.json')).write_text(stdout)
    (ROOT / (label + '.stderr.log')).write_text(stderr)
    save(label + '.command.json', {'args': [s.replace(str(AUTH), '[existing Codex account file]') for s in command], 'exit': result.returncode, 'elapsed_seconds': time.monotonic() - started})
    parsed = json.loads(stdout)
    if result.returncode != 0:
        raise RuntimeError(label + ':' + parsed.get('error', {}).get('code', 'failed'))
    return parsed['result']

def prepare():
    ROOT.mkdir(parents=True, mode=0o700, exist_ok=False)
    (ROOT / 'tmp').mkdir(mode=0o700)
    call('init', ['init', '--purpose', 'test'])
    note = call('target', ['note-create', '--title', 'Synthetic local edit', '--text', ORIGINAL])
    sentinel = call('sentinel', ['note-create', '--title', 'Unchanged sentinel', '--text', SENTINEL])
    chat = call('thread', ['thread-start', '--title', 'Synthetic local edit qualification'])
    call('configure', ['configure', '--provider', 'Codex', '--model', 'gpt-6-luna', '--effort', 'low', '--auth-file', str(AUTH), '--helper', 'off'])
    call('delegation', ['delegation', '--choice', 'review-first'])
    assert call('candidates-before', ['candidates']) == []
    assert call('target-before', ['read', '--id', note['id']]) == note
    assert call('sentinel-before', ['read', '--id', sentinel['id']]) == sentinel
    prompt = ('In Note ' + note['id'] + ', propose only changing the exact passage "cobalt boats" to "teal boats". '
              'Use the precise passage-editing capability. Preserve every other byte, title and metadata. '
              'Leave the proposal unapplied for review; do not change other records or use external sources. '
              'Report the prepared operation. This is a short synthetic test.')
    assert len(ORIGINAL.encode()) + len(SENTINEL.encode()) + len(prompt.encode()) <= 8192
    (ROOT / 'prompt.txt').write_text(prompt)
    save('manifest.json', {'source': SOURCE, 'binary_sha256': digest(CLI), 'target': note, 'sentinel': sentinel, 'chat': chat, 'expected': EXPECTED, 'initial_utf8_bytes': len(ORIGINAL.encode()) + len(SENTINEL.encode()) + len(prompt.encode()), 'prepared_at': now(), 'owner_request': None, 'delegation': 'review-first'})

def seal():
    # Local-only checks, also used before dispatch. Never call a provider here.
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    assert digest(CLI) == FROZEN_SHA256 == manifest['binary_sha256']
    assert manifest['source'] == SOURCE and manifest['owner_request'] is None
    assert manifest['target']['data']['Note']['markdown'] == ORIGINAL
    assert manifest['sentinel']['data']['Note']['markdown'] == SENTINEL
    assert manifest['expected'] == EXPECTED
    assert call('sealed-target', ['read', '--id', manifest['target']['id']]) == manifest['target']
    assert call('sealed-sentinel', ['read', '--id', manifest['sentinel']['id']]) == manifest['sentinel']
    assert call('sealed-candidates', ['candidates']) == []
    settings = call('sealed-settings', ['settings'])
    assert (settings['provider'], settings['model'], settings['effort']) == ('Codex', 'gpt-6-luna', 'low')
    assert settings['review_first'] is True and settings['delegation_chosen'] is True
    assert settings.get('helper') is None and settings.get('mcp') is None
    assert settings['auth_file'] == '[existing Codex account file]' and settings['credentials_dir'] is None
    prompt_bytes = (ROOT / 'prompt.txt').read_bytes()
    assert len(ORIGINAL.encode()) + len(SENTINEL.encode()) + len(prompt_bytes) <= 8192
    save('launch-seal.json', {'source': SOURCE, 'binary_sha256': digest(CLI), 'manifest_sha256': digest(ROOT / 'manifest.json'), 'prompt_sha256': digest(ROOT / 'prompt.txt'), 'settings': settings, 'initial_utf8_bytes': len(ORIGINAL.encode()) + len(SENTINEL.encode()) + len(prompt_bytes)})

def run():
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    assert digest(CLI) == FROZEN_SHA256 == manifest['binary_sha256']
    clock = datetime.datetime.now(datetime.timezone.utc)
    assert clock.astimezone(ZoneInfo('Europe/Tallinn')).date().isoformat() == '2026-10-22'
    assert clock < datetime.datetime.fromisoformat('2026-11-01T21:41:51+00:00')
    prior_seal = json.loads((ROOT / 'launch-seal.json').read_text())
    seal()
    assert json.loads((ROOT / 'launch-seal.json').read_text()) == prior_seal
    ledger = json.loads(LEDGER.read_text())
    attempts = [a for a in ledger['attempts'] if a.get('session') == SESSION]
    assert len(attempts) == 1 and attempts[0]['status'] == 'reserved'
    attempt = attempts[0]
    assert attempt['binary_sha256'] == digest(CLI) and attempt['data_directory'] == str(ROOT / 'workspace')
    assert attempt['harness_source_sha256'] == digest(pathlib.Path(__file__))
    assert attempt['launch_seal_sha256'] == digest(ROOT / 'launch-seal.json')
    assert attempt['desktop_allowance_before']['remaining_percent'] > 5
    assert attempt['desktop_allowance_before']['ordinary_usage_allowed'] is True
    observed = datetime.datetime.fromisoformat(attempt['desktop_allowance_before']['observed_at'])
    assert 0 <= (clock - observed).total_seconds() < 300
    assert len([a for a in ledger['attempts'] if a['tallinn_date'] == '2026-10-22']) <= 6
    attempt.update(status='running', started_at_utc=now())
    LEDGER.write_text(json.dumps(ledger, indent=2) + '\n')
    result = {'attempt_id': attempt['id'], 'source': SOURCE, 'model': 'gpt-6-luna', 'native_observed': False, 'upstream_cancellation_confirmed': False}
    try:
        answer = call('ask', ['--timeout', '180', 'ask', '--thread', manifest['chat']['id'], '--prompt-file', str(ROOT / 'prompt.txt'), '--budget', '4', '--trace', 'summary'], 190)
        result.update(answer=answer, account_transport_succeeded=True)
        candidates = call('candidates-after', ['candidates'])
        original = call('target-after', ['read', '--id', manifest['target']['id']])
        sentinel = call('sentinel-after', ['read', '--id', manifest['sentinel']['id']])
        run_record = call('run-after', ['read', '--id', answer['run']])
        assert answer['terminal'] == 'Completed'
        assert original == manifest['target'], 'canonical target changed'
        assert sentinel == manifest['sentinel'], 'sentinel changed'
        assert len(candidates) == 1, 'expected one unapplied candidate'
        prepared = candidates[0]
        writes = prepared['request']['writes']
        assert len(writes) == 1 and writes[0]['id'] == original['id']
        assert writes[0]['expected_version'] == original['version']
        assert writes[0]['archived'] == original['archived']
        expected_data = json.loads(json.dumps(original['data']))
        expected_data['Note']['markdown'] = EXPECTED
        assert writes[0]['data'] == expected_data, 'candidate content or metadata differs'
        assert run_record['data']['Run']['operations'] == [prepared['operation']]
        precise = [c for c in answer['trace']['calls'] if c['tool'] == 'prepare_note_edit' and c['outcome'] == 'success']
        assert len(precise) == 1, 'expected exactly one successful precise-edit preparation'
        assert not any(c['tool'] in ['prepare_changes', 'refresh_candidate', 'withdraw_proposal'] for c in answer['trace']['calls']), 'additional preparation route used'
        # Existing trace deliberately omits precise-edit arguments/result data.
        # A complete trace, one successful preparation route, and one Run operation
        # bind the surviving exact candidate without inferring hidden arguments.
        assert answer['trace']['dropped_calls'] == 0
        result.update(passed=True, original_unchanged=True, sentinel_unchanged=True, exact_candidate=True, candidate_operation=prepared['operation'], candidate_unapplied=True)
        attempt['status'] = 'passed'
    except Exception as error:
        result.update(passed=False, error_type=type(error).__name__, error=str(error))
        attempt['status'] = 'failed'
    finally:
        if not result.get('passed'):
            observations = {}
            for label, arguments in [('target', ['read', '--id', manifest['target']['id']]), ('sentinel', ['read', '--id', manifest['sentinel']['id']]), ('candidates', ['candidates']), ('runs', ['runs', '--thread', manifest['chat']['id']])]:
                try:
                    observations[label] = call('failure-' + label, arguments)
                except Exception as error:
                    observations[label] = {'unknown': True, 'error_type': type(error).__name__, 'error': str(error)}
            result['failure_durable_observations'] = observations
            result['terminal_status'] = result.get('answer', {}).get('terminal', 'unknown')
        attempt.update(finished_at_utc=now(), result_file=str(ROOT / 'result.json'))
        save('result.json', result)
        current_ledger = json.loads(LEDGER.read_text())
        current_attempt = next(a for a in current_ledger['attempts'] if a['id'] == attempt['id'])
        current_attempt.update(attempt)
        LEDGER.write_text(json.dumps(current_ledger, indent=2) + '\n')
    print(json.dumps({k: result.get(k) for k in ['attempt_id', 'passed', 'error_type', 'error']}))
    if not result.get('passed'):
        raise SystemExit(1)

if __name__ == '__main__':
    {'prepare': prepare, 'seal': seal, 'run': run}[sys.argv[1]]()
