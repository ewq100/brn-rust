#!/usr/bin/env python3
"""OFFLINE preparation only; ONE fresh pair, one checked auth-REFERENCE configure, no ask/runner/activation.

Reuses the checked CLI corpus generator. No SQLite reader/writer, credential
reader, network client or alternative execution driver. One exact separately allocated LOCAL checked configure stores only the literal
auth path reference after initial-state preservation. It never opens that file.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def digest(b):
    return hashlib.sha256(b).hexdigest()


def sha(path):
    return digest(Path(path).read_bytes())


def settings_digest(settings):
    # CLI JSON retains WorkspaceSettings declaration order. Rust fixture_facts
    # hashes to_value(settings) with the actual serde_json preserve_order feature
    # (Rig's unified graph); alphabetical sorting is a DIFFERENT byte contract.
    return digest(json.dumps(settings, separators=(',', ':'), ensure_ascii=False).encode())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('pin', 'cli', 'generator', 'runner', 'runner-receipt',
                'runner-artifacts', 'cli-receipt', 'cli-artifacts'):
        parser.add_argument('--' + key, required=True)
    a = parser.parse_args()
    # Locate by the explicit existing source receipt, never owner filesystem scan.
    runner_receipt = json.loads(Path(a.runner_receipt).read_text())
    root = Path(runner_receipt['source_root'])
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if head != a.pin:
        parser.error('current HEAD must equal immutable supplied pin')
    cli_receipt = json.loads(Path(a.cli_receipt).read_text())
    if not runner_receipt['source_unchanged'] or not cli_receipt['source_unchanged']:
        parser.error('command source changed')
    if runner_receipt['source_hashes'] != cli_receipt['source_hashes']:
        parser.error('runner and generator inputs differ')
    for path, expected in runner_receipt['source_hashes'].items():
        if sha(root / path) != expected or digest(subprocess.check_output(
                ['git', 'show', a.pin + ':' + path], cwd=root)) != expected:
            parser.error('current/Git compiled input mismatch: ' + path)
    artifacts = []
    for receipt, artifact_path, receipt_path in [
            (runner_receipt, a.runner_artifacts, a.runner_receipt),
            (cli_receipt, a.cli_artifacts, a.cli_receipt)]:
        rows = json.loads(Path(artifact_path).read_text())
        if rows['receipt_sha256'] != sha(receipt_path) or receipt['exit']:
            parser.error('artifact/command receipt mismatch or failed command')
        artifacts.extend(rows['artifacts'])
    chosen = {}
    for kind, path in [('cli', a.cli), ('generator', a.generator), ('runner', a.runner)]:
        path = Path(path).resolve(strict=True)
        rows = [r for r in artifacts if r['path'] == str(path)
                and r['not_modified_after_receipt'] and sha(path) == r['sha256']]
        if not rows or not os.access(path, os.X_OK):
            parser.error(kind + ' executable lacks actual matching build association')
        row = rows[0]
        if sha(row['dep_info']) != row['dep_info_sha256']:
            parser.error(kind + ' dep-info mismatch')
        chosen[kind] = row
    tag = a.pin[:8]
    pair = Path('/private/tmp/brn-int-d-qualification-20261017-runner-' + tag)
    pre = Path('/private/tmp/brn-int-d-capture-preflight-20261017-' + tag)
    output = Path('/private/tmp/brn-int-d-capture-output-20261017-' + tag)
    if any(p.exists() or p.is_symlink() for p in (pair, pre, output)):
        parser.error('fresh paths only; no adoption/replacement')
    pre.mkdir(mode=0o700)
    env = {k: v for k, v in os.environ.items()
           if not k.startswith('BRN_') and not k.startswith('AGENT_CONTRACT_')}
    env['TMPDIR'] = tempfile.mkdtemp(prefix='brn-int-d-runner-prep-', dir='/private/tmp')
    env['BRN_INT_D_FIXTURE_ROOT'] = str(pair)
    env['AGENT_CONTRACT_BASELINE_BIN'] = chosen['cli']['path']
    command = [chosen['generator']['path'], '--exact',
               'checked_source_labelled_pair_reopens_through_cli_with_explicit_scoped_selection',
               '--test-threads=1', '--nocapture']
    result = subprocess.run(command, cwd=root, env=env, capture_output=True, timeout=60)
    (pre / 'generation.stdout').write_bytes(result.stdout)
    (pre / 'generation.stderr').write_bytes(result.stderr)
    if result.returncode:
        parser.error('generator failed; preserve this attempt, no replacement')
    manifest = json.loads((pair / 'manifest.json').read_text())
    control = next(c for c in manifest['cases'] if c['case'] == 'control')
    if control['selected'] or control['data_dir'] != str(pair / 'control'):
        parser.error('control identity changed')
    data = pair / 'control'
    # Actual existing owner read command only. Do not archive record body bytes.
    records_cmd = [chosen['cli']['path'], '--json', '--data-dir', str(data),
                   '--timeout', '2', 'records']
    result = subprocess.run(records_cmd, cwd=root, env=env, capture_output=True, timeout=3)
    if result.returncode:
        parser.error('canonical inspection failed; no retry')
    envelope = json.loads(result.stdout)
    if envelope['status'] != 'ok':
        parser.error('canonical inspection refused')
    records = envelope['result']
    if not isinstance(records, list):
        parser.error('unexpected actual records contract')
    by_id = {r['id']: r for r in records}
    project, chat, note = [by_id[control[k]] for k in ('project', 'chat', 'note')]
    settings_record = next(r for r in records if 'Settings' in r['data'])
    settings = settings_record['data']['Settings']
    if (settings.get('helper') is not None or settings.get('mcp') is not None
            or any(b['enabled'] or b['selected'] for b in settings.get('skills', []))
            or settings['auth_file'] is not None or settings['credentials_dir'] is not None
            or settings['model'] != 'gpt-6.1-sol' or settings['effort'] != 'medium'):
        parser.error('generator control configuration changed')
    if not note['data']['Note']['protected'] or any('Run' in r['data'] for r in records):
        parser.error('protected fresh fixture required')
    if envelope.get('authority', {}).get('workspace_purpose') != 'test':
        parser.error('CLI checked authority does not confirm Test purpose')
    initial = pre / 'initial-state'
    initial.mkdir(mode=0o700)
    (initial / 'manifest.json').write_bytes((pair / 'manifest.json').read_bytes())
    (initial / 'settings.json').write_text(json.dumps(settings_record, indent=2) + '\n')
    initial_db = {}
    for suffix in ('', '-wal', '-shm'):
        path = data / ('threads.sqlite3' + suffix)
        initial_db[suffix or 'main'] = sha(path) if path.exists() else None
        if path.exists():
            (initial / path.name).write_bytes(path.read_bytes())
    configure = [chosen['cli']['path'], '--json', '--data-dir', str(data), '--timeout', '120',
                 'configure', '--provider', 'Codex', '--model', 'gpt-6.1-sol', '--effort', 'medium',
                 '--auth-file', '/Users/evokessler/.codex/auth.json']
    configured = subprocess.run(configure, cwd=root, env=env, capture_output=True, timeout=120)
    (pre / 'reference-configure.stdout').write_bytes(configured.stdout)
    (pre / 'reference-configure.stderr').write_bytes(configured.stderr)
    if configured.returncode or json.loads(configured.stdout)['status'] != 'ok':
        parser.error('one checked reference configure failed; preserve/no retry')
    result = subprocess.run(records_cmd, cwd=root, env=env, capture_output=True, timeout=3)
    if result.returncode:
        parser.error('postconfiguration canonical read failed; no retry')
    envelope = json.loads(result.stdout)
    if envelope['status'] != 'ok' or envelope.get('authority', {}).get('workspace_purpose') != 'test':
        parser.error('postconfiguration purpose/status changed')
    records = envelope['result']
    if any('Run' in r['data'] for r in records):
        parser.error('unexpected Run after reference-only command')
    after = next(r for r in records if 'Settings' in r['data'])
    expected = dict(settings, auth_file='/Users/evokessler/.codex/auth.json')
    if after['data']['Settings'] != expected or after['version'] != settings_record['version'] + 1:
        parser.error('unexpected configure delta')
    (pre / 'postconfiguration-overlay.json').write_text(json.dumps({
        'command': configure, 'initial_database': initial_db,
        'original_manifest_sha256': sha(pair / 'manifest.json'),
        'initial_settings': settings_record, 'post_settings': after,
        'credential_file_read_by_preparer': False, 'provider_starts': 0}, indent=2) + '\n')
    if {r['id']: r for r in records if r['id'] != after['id']} != {k: v for k, v in by_id.items() if k != after['id']}:
        parser.error('reference-only configure changed other canonical records')
    settings_record, settings = after, after['data']['Settings']
    context = {'purpose': 'Test', 'project': {'id': project['id'], 'version': project['version']},
               'chat': {'id': chat['id'], 'version': chat['version'],
                        'privacy': chat['data']['Thread'].get('private_context', False),
                        'epoch': chat['data']['Thread'].get('context_epoch', 0)},
               'settings': {'id': settings_record['id'], 'version': settings_record['version'],
                            'policy_sha256': settings_digest(settings),
                            'auth_reference': settings['auth_file']},
               'target': {'id': note['id'], 'version': note['version'], 'protected': True},
               'inputs': [{'id': mid, 'version': by_id[mid]['version']}
                          for mid in sorted(control['history_messages'])]}
    prompt = (f"Revise the synthetic Aurora delivery Note {control['note']} using the saved discussion. "
              'Find the early accepted requirement and distinguish the later unaccepted proposal. '
              'Read the exact saved Message and cite its ID/version and passage. '
              'If an enabled review skill is available, read its reference before proposing. '
              'Retain your objective and remaining work. Prepare the exact protected Note revision, '
              'leave it for owner review, and do not claim it applied. Do not access external tools.\n')
    (pre / 'prompt.txt').write_text(prompt)
    rubric = root / 'docs/work/active/agent-capabilities/int-d/diagnosis/capture/runner/rubric.json'
    (pre / 'rubric.json').write_bytes(rubric.read_bytes())
    database = {'main': sha(data / 'threads.sqlite3')}
    for key, suffix in [('wal', '-wal'), ('shm', '-shm')]:
        path = data / ('threads.sqlite3' + suffix)
        database[key] = sha(path) if path.exists() else None
    inputs = runner_receipt['source_hashes']
    p = {'format': 'BRN-INT-D-CAPTURE-PREFLIGHT-V1', 'material_pin': a.pin,
         'source_inputs': inputs, 'observer_sha256': inputs['crates/brn-ai/src/threads.rs'],
         'runner_sha256': inputs['crates/brn-ai/src/threads_int_d_tests.rs'],
         'executable': chosen['runner']['path'], 'executable_sha256': chosen['runner']['sha256'],
         'dep_info_sha256': chosen['runner']['dep_info_sha256'],
         'corpus_sha256': sha(root / 'docs/work/active/agent-capabilities/int-d/corpus.json'),
         'prompt_sha256': sha(pre / 'prompt.txt'), 'rubric_sha256': sha(pre / 'rubric.json'),
         'fixture': str(pair), 'manifest_sha256': sha(pair / 'manifest.json'), 'output': str(output),
         'database': database, 'context': context, 'provider': 'Codex/ChatGPT',
         'model': 'gpt-6.1-sol', 'effort': 'medium',
         'auth_reference': '/Users/evokessler/.codex/auth.json',
         'rounds': 8, 'deadline_seconds': 120, 'maximum_starts': 1}
    (pre / 'preflight.json').write_text(json.dumps(p, indent=2) + '\n')
    status = {'activation_ready': False, 'paid_starts': 0, 'activation_created': False,
              'blockers': ['No principal activation/review yet; ignored runner never executed.'],
              'preflight_prepared': True, 'reference_only_configure_count': 1,
              'preflight': str(pre / 'preflight.json'), 'sha256': sha(pre / 'preflight.json'),
              'generation_command': command, 'canonical_read_command': records_cmd,
              'main_wal_shm_after_final_local_inspection': database,
              'artifacts': chosen, 'purpose_evidence': 'Actual checked Test-purpose generator; future gate checks Store purpose again.',
              'earlier_O3': 'This one new preparer is executed prospectively only; old preparer remains unexecuted at its old pin.'}
    (pre / 'readiness.json').write_text(json.dumps(status, indent=2) + '\n')
    print(json.dumps(status, indent=2))


if __name__ == '__main__':
    main()
