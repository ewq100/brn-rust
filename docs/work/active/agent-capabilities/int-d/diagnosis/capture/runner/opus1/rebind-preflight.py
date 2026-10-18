#!/usr/bin/env python3
"""Metadata-only B1 rebind. NEVER generate/configure/ask/activate/open Store/Auth.

Reuse only the exact unused origin fixture. Canonical proof is the actual Rust
fixture_facts read on a closed byte copy, tied to its passing command receipt.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess

OLD = Path('/private/tmp/brn-int-d-capture-preflight-20261017-9d3741e6')
FIXTURE = Path('/private/tmp/brn-int-d-qualification-20261017-runner-9d3741e6')
OLD_SHA = 'befbd639c2c699fd27502a9cb70a618ce00154987dbdbcd49574a85ea2d5b85f'
MAIN_SHA = 'e26d55cd59c1f5ecd528ee27cfbfca61db31c084338c0fdc79e868802bd923ee'
TEST = 'threads::int_d_tests::capture_runner::offline_python_context_interoperates_with_exact_rust_fixture_facts'


def read(path, cap=256*1024):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as f:
        st = os.fstat(f.fileno())
        if not stat.S_ISREG(st.st_mode) or st.st_size > cap:
            raise ValueError('bounded regular metadata required')
        b = f.read(cap + 1)
        if len(b) > cap:
            raise ValueError('metadata grew past bound')
        return b


def sha(path, cap=1024*1024*1024):
    return hashlib.sha256(read(path, cap)).hexdigest()


def snapshot():
    data = FIXTURE / 'control'
    if FIXTURE.resolve() != FIXTURE or data.resolve() != data:
        raise ValueError('origin path changed')
    if (OLD / 'activation.json').exists() or (OLD / 'activation.json').is_symlink():
        raise ValueError('old activation exists')
    p = json.loads(read(OLD / 'preflight.json'))
    if sha(OLD / 'preflight.json') != OLD_SHA or p['fixture'] != str(FIXTURE):
        raise ValueError('origin preflight changed')
    if Path(p['output']).exists() or Path(p['output']).is_symlink():
        raise ValueError('origin output/reservation exists')
    db = {'main': sha(data / 'threads.sqlite3')}
    for key in ('wal', 'shm'):
        path = data / ('threads.sqlite3-' + key)
        db[key] = sha(path) if path.exists() or path.is_symlink() else None
    if db != p['database'] or db != {'main': MAIN_SHA, 'wal': None, 'shm': None}:
        raise ValueError('origin DB/sidecars changed')
    if sha(FIXTURE / 'manifest.json') != p['manifest_sha256']:
        raise ValueError('origin manifest changed')
    return p, db


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    for name in ('pin', 'receipt', 'artifacts'):
        ap.add_argument('--' + name, required=True)
    a = ap.parse_args()
    receipt = json.loads(read(a.receipt))
    root = Path(receipt['source_root'])
    if root != Path.cwd() or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() != a.pin:
        ap.error('exact current material pin required')
    old, before = snapshot()
    for path, h in old['source_inputs'].items():
        if hashlib.sha256(subprocess.check_output(['git', 'show', old['material_pin'] + ':' + path], cwd=root)).hexdigest() != h:
            ap.error('old origin source association changed')
    if (receipt['exit'] or not receipt['source_unchanged']
            or receipt['test_summaries'] != [[1, 0, 0]] or TEST not in receipt['command']
            or '--exact' not in receipt['command'] or '--ignored' in receipt['command']):
        ap.error('passing exact OFFLINE interop receipt required')
    source = receipt['source_hashes']
    if source.keys() != old['source_inputs'].keys():
        ap.error('compiled input set changed')
    for path, h in source.items():
        if sha(root / path) != h or hashlib.sha256(subprocess.check_output(['git', 'show', a.pin + ':' + path], cwd=root)).hexdigest() != h:
            ap.error('new compiled source differs from material pin')
    artifacts = json.loads(read(a.artifacts))
    if artifacts['receipt_sha256'] != sha(a.receipt):
        ap.error('artifact receipt mismatch')
    runner = next(r for r in artifacts['artifacts'] if '/deps/brn_ai-' in r['path'])
    if not runner['not_modified_after_receipt'] or sha(runner['path']) != runner['sha256'] or sha(runner['dep_info']) != runner['dep_info_sha256']:
        ap.error('runner binary/dep-info changed')
    env = receipt['explicit_fixture_env']
    copy = Path(env['BRN_INT_D_OFFLINE_CONTEXT_COPY'])
    if copy.parent != Path('/private/tmp') or not copy.name.startswith('brn-int-d-runner-interop-20261017-') or copy.resolve() != copy:
        ap.error('explicit task-owned closed-copy proof required')
    actual_path = copy / 'actual-context.json'
    expected_path = copy / 'expected-context.json'
    if (env['BRN_INT_D_OFFLINE_CONTEXT_OUTPUT'] != str(actual_path)
            or env['BRN_INT_D_OFFLINE_EXPECTED_CONTEXT'] != str(expected_path)
            or sha(copy / 'control/threads.sqlite3') != MAIN_SHA):
        ap.error('copied proof identity changed')
    if any((copy / ('control/threads.sqlite3-' + suffix)).exists() for suffix in ('wal', 'shm')):
        ap.error('copied proof has unexpected sidecars')
    actual = json.loads(read(actual_path, 32*1024))
    expected = json.loads(read(expected_path, 32*1024))
    if actual != expected:
        ap.error('whole Rust/Python context proof did not pass')
    module = importlib.util.spec_from_file_location('fixed_preparer', Path(__file__).parent.parent / 'prepare-preflight.py')
    preparer = importlib.util.module_from_spec(module)
    module.loader.exec_module(preparer)  # no main/generation/configuration invocation
    overlay = json.loads(read(OLD / 'postconfiguration-overlay.json'))
    settings = overlay['post_settings']
    if settings['version'] != 4 or settings['id'] != old['context']['settings']['id'] or settings['data']['Settings']['auth_file'] != '/Users/evokessler/.codex/auth.json':
        ap.error('origin Settings overlay changed')
    context = json.loads(json.dumps(old['context']))
    context['settings']['policy_sha256'] = preparer.settings_digest(settings['data']['Settings'])
    if context != actual:
        ap.error('origin facts/noRuns/candidates proof differs')
    dest = Path('/private/tmp/brn-int-d-capture-preflight-20261017-b1-' + a.pin[:8])
    output = Path('/private/tmp/brn-int-d-capture-output-20261017-b1-' + a.pin[:8])
    if any(path.exists() or path.is_symlink() for path in (dest, output)):
        ap.error('fresh preflight/output namespace required')
    dest.mkdir(mode=0o700)
    for name in ('prompt.txt',):
        (dest / name).write_bytes(read(OLD / name))
    rubric = Path(__file__).parent.parent / 'rubric.json'
    (dest / 'rubric.json').write_bytes(read(rubric, 16*1024))
    new = dict(old, material_pin=a.pin, source_inputs=source,
               runner_sha256=source['crates/brn-ai/src/threads_int_d_tests.rs'],
               executable=runner['path'], executable_sha256=runner['sha256'],
               dep_info_sha256=runner['dep_info_sha256'], rubric_sha256=sha(dest / 'rubric.json'),
               output=str(output), context=context)
    if sha(dest / 'prompt.txt') != old['prompt_sha256'] or snapshot() != (old, before):
        ap.error('origin/prompt changed during rebind; preserve partial destination')
    (dest / 'preflight.json').write_text(json.dumps(new, indent=2) + '\n')
    report = {'old_preflight': str(OLD / 'preflight.json'), 'old_sha256': OLD_SHA,
              'old_status': 'SUPERSEDED FAILED READINESS (B1), unchanged',
              'fixture_origin_material': old['material_pin'], 'new_material': a.pin,
              'same_unused_fixture': str(FIXTURE), 'original_DB_sidecars_before_after': before,
              'Rust_copy_proof': {'receipt': a.receipt, 'receipt_sha256': sha(a.receipt),
                                  'actual_context': str(actual_path), 'sha256': sha(actual_path),
                                  'copied_main_sha256': MAIN_SHA, 'whole_context_equal': True},
              'old_policy_sha256': old['context']['settings']['policy_sha256'],
              'corrected_policy_sha256': context['settings']['policy_sha256'],
              'preflight': str(dest / 'preflight.json'), 'sha256': sha(dest / 'preflight.json'),
              'new_rubric_sha256': new['rubric_sha256'], 'prompt_unchanged': True,
              'generation_count': 0, 'configure_count': 0, 'Auth_or_runner_or_provider_starts': 0,
              'activation_created': False, 'reservation_created': False,
              'activation_ready': False, 'pending': 'NEW focused parent review and explicit activation; full ignored entry unexecuted'}
    (dest / 'rebind-receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
