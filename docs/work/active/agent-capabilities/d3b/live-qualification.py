#!/usr/bin/env python3
"""Pinned, finite BRN CLI qualification. No credential reader or transport.
Run ONLY after normal checks/pin, using the principal-selected route. Existing
output folders refuse, so failed starts cannot disappear through a rerun.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--candidate-sha256', required=True)
    parser.add_argument('--pin', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert args.output.is_absolute() and not args.output.exists()
    assert args.baseline.is_absolute() and args.candidate.is_absolute()
    assert sha(args.candidate) == args.candidate_sha256
    here = Path(__file__).resolve().parent
    repo = here.parents[4]
    assert (repo / 'Cargo.toml').is_file(), repo
    pin = subprocess.check_output(['git', 'rev-parse', args.pin], cwd=repo, text=True).strip()
    subprocess.run(['git', 'diff', '--exit-code', pin, '--', 'crates', 'Cargo.lock'], cwd=repo, check=True, stdout=subprocess.DEVNULL)
    fixture_path = here / 'live-fixtures.json'
    fixture = json.loads(fixture_path.read_text())
    assert fixture['route']['model'] == 'gpt-6.1-sol'
    assert fixture['route']['effort'] == 'medium'
    args.output.mkdir(mode=0o700)
    package = repo / 'agent/skill-examples/document-review'
    state = {'source_pin': pin, 'candidate_binary': str(args.candidate), 'candidate_sha256': sha(args.candidate),
             'baseline_source': '000ed92026ccb180f2257b28b07e0525b1493f23', 'baseline_binary': str(args.baseline), 'baseline_sha256': sha(args.baseline),
             'fixture_sha256': sha(fixture_path), 'route': fixture['route'], 'window_cap': fixture['initial_window'],
             'workflow_starts': 0, 'attempts': [], 'calls': [], 'status': 'preparing synthetic canonical fixture'}
    def save():
        (args.output / 'qualification.json').write_text(json.dumps(state, indent=2, ensure_ascii=False) + '\n')
    save()
    env = os.environ.copy()
    for key in ('BRN_EMBEDDING_MODEL_DIR', 'BRN_NATIVE_MODEL_DIR', 'RUST_LOG', 'RUST_LOG_STYLE'):
        env.pop(key, None)
    def call(binary, data, command, workflow=False):
        if workflow:
            assert state['workflow_starts'] < 12
            state['workflow_starts'] += 1
            save()  # Charge before dispatch, including failed/interrupted starts.
        argv = [str(binary), '--json', '--data-dir', str(data), '--timeout', '120', *command]
        started = time.monotonic()
        try:
            result = subprocess.run(argv, env=env, capture_output=True, text=True, timeout=135)
        except subprocess.TimeoutExpired:
            state['status'] = 'external watchdog timeout; CLI child killed/reaped, upstream cancellation not proven'
            state['attempts' if workflow else 'calls'].append({'binary': str(binary), 'workspace': str(data), 'command': command, 'status': 'watchdog_timeout', 'elapsed_seconds': time.monotonic() - started})
            save()
            raise SystemExit(state['status'])
        elapsed = time.monotonic() - started
        value = json.loads(result.stdout)
        # Public synthetic CLI results only. Never retain stderr/auth headers,
        # credential contents, account claims or a raw provider transcript.
        row = {'binary': str(binary), 'workspace': str(data), 'command': command, 'exit': result.returncode,
               'elapsed_seconds': elapsed, 'status': value.get('status'), 'outcome': value.get('outcome'),
               'error': {k: value.get('error', {}).get(k) for k in ('code', 'message')} if value.get('error') else None,
               'stderr_bytes': len(result.stderr.encode())}
        state['calls'].append(row)
        if workflow:
            answer = value.get('result') or {}
            trace = answer.get('trace') or {}
            turns = trace.get('turn_usage', [])
            state['attempts'].append({**row, 'run': answer.get('run'), 'answer': answer.get('answer'),
                'terminal': answer.get('terminal'), 'notices': answer.get('notices'), 'guide': trace.get('guide'),
                'model_turns': trace.get('model_turns'), 'usage': trace.get('usage'), 'turn_usage': turns,
                'turn_cached_share': [t['cached_input'] / t['input'] if t.get('input') else None for t in turns],
                'summary': trace.get('summary'), 'helpers': trace.get('helpers', []), 'tool_calls': trace.get('calls', []),
                'provider_retries_count': len(trace.get('provider_retries', [])),
                'account_identity': 'not exposed', 'auth_request_count': 'not exposed',
                'semantic_scores': 'pending source-grounded assessment; no automatic superiority claim'})
        save()
        if result.returncode and not workflow:
            state['status'] = 'stopped on actual command failure; no workflow retry/fallback'
            save()
            raise SystemExit(f"{command[0]} failed ({row['status']}, {row['error']}); retained attempt, no retry")
        return ((value.get('result') or {}, result.returncode != 0) if workflow else value['result'])
    def created(receipt, kind):
        return next(w['after']['id'] for w in receipt['writes'] if kind in w['after']['data'])
    seed = args.output / 'schema7-fixture'
    old = args.baseline
    call(old, seed, ['init', '--purpose', 'test'])
    call(old, seed, ['configure', '--provider', 'Codex', '--model', 'gpt-6.1-sol', '--effort', 'medium', '--auth-file', '/Users/evokessler/.codex/auth.json', '--helper', 'off'])
    call(old, seed, ['delegation', '--choice', 'review-first'])
    project = created(call(old, seed, ['project-create', '--title', 'Synthetic live document review']), 'Project')
    chat = created(call(old, seed, ['project-chat', '--project', project, '--title', 'Synthetic review']), 'Thread')
    ids = {}
    for key in ('working_note', 'requirements_note', 'evidence_note'):
        note = fixture[key]
        ids[key] = created(call(old, seed, ['project-document', '--project', project, '--title', note['title'], '--text', note['markdown']]), 'Note')
    other = created(call(old, seed, ['project-create', '--title', 'Unrelated synthetic scope']), 'Project')
    note = fixture['unrelated_private_note']
    call(old, seed, ['project-document', '--project', other, '--title', note['title'], '--text', note['markdown']])
    snapshot = args.output / 'schema7-fixture.sqlite3'
    call(old, seed, ['backup', '--destination', str(snapshot)])
    prompt = fixture['prompt'] + '\nExact current Project Note IDs: ' + json.dumps(ids)
    state['canonical_ids'] = {'project': project, 'chat': chat, **ids}
    state['fixture_snapshot_sha256'] = sha(snapshot)
    state['status'] = 'paired runs'
    save()
    for pair in (1, 2):
        for label, binary in (('control', old), ('candidate', args.candidate)):
            data = args.output / f'{label}-{pair}'
            call(binary, data, ['restore', '--source', str(snapshot)])
            settings = call(binary, data, ['settings'])
            assert settings['provider'] in ('Codex', 'Chatgpt') and settings['model'] == 'gpt-6.1-sol' and settings['effort'] == 'medium'
            assert settings['auth_file'] == '/Users/evokessler/.codex/auth.json' and settings.get('helper') is None
            if label == 'candidate':
                installed = call(binary, data, ['skill-install', '--source', str(package)])
                call(binary, data, ['skill-enable', '--id', installed['id'], '--digest', installed['digest'], '--project', project])
                call(binary, data, ['skill-select', '--id', installed['id'], '--project', project])
            answer, failed = call(binary, data, ['ask', '--thread', chat, '--prompt', prompt, '--budget', '8', '--trace', 'summary'], workflow=True)
            state['attempts'][-1]['pair'] = pair
            state['attempts'][-1]['label'] = label
            # Preserve public retained outcome/provenance and operation identities.
            retained = {name: call(binary, data, argv) for name, argv in (
                ('runs', ['runs', '--thread', chat]), ('candidates', ['candidates']),
                ('working_note', ['read', '--id', ids['working_note']]), ('history', ['history']))}
            (args.output / f'{label}-{pair}-retained.json').write_text(json.dumps(retained, indent=2, ensure_ascii=False) + '\n')
            save()
            if failed:
                state['status'] = 'stopped on actual workflow failure after retaining canonical outcomes; no retry/fallback'
                save()
                raise SystemExit(state['status'])
    state['status'] = 'four paired starts completed; semantic assessment pending'
    save()


if __name__ == '__main__':
    main()
