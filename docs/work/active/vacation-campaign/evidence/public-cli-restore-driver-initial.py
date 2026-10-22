#!/usr/bin/env python3
"""Bounded public-CLI restore interruption qualification; review before use."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import sys
import threading
import time
import uuid

FROZEN_BIN = Path('/Users/evokessler/.codex/artifacts/brn-precise-edit-quote-28fd581e/brn')
FROZEN_SHA256 = '242b141ce518f0ff87d58f7e36b3250269c344211d1552a3d96206cfc62e5f7d'
FROZEN_SOURCE = '28fd581ea8451ecd95d899248132b62c8f890d03'
MAX_BODY = 8 * 1024 * 1024
MAX_BACKUP = 128 * 1024 * 1024
MAX_ROOT = 512 * 1024 * 1024
MIN_FREE = 2 * 1024 * 1024 * 1024
CLI_TIMEOUT = 60
OBSERVE_SECONDS = 10
DB_NAME = 'threads.sqlite3'
MARKER_NAME = '.brn-threads-data'
STAGE_PREFIX = '.brn-restore-stage-'
NOTE_TITLE = 'Public restore interruption qualification · Ω'


class Stop(Exception):
    pass


def sha256_file(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(',', ':')).encode('utf-8')


def sql_value(value):
    if isinstance(value, bytes):
        return {'blob_hex': value.hex()}
    return value


def readonly_snapshot(path):
    uri = path.resolve(strict=True).as_uri() + '?mode=ro'
    con = sqlite3.connect(uri, uri=True, timeout=2)
    try:
        con.execute('PRAGMA query_only=ON')
        integrity = con.execute('PRAGMA integrity_check').fetchone()[0]
        if integrity != 'ok':
            raise Stop(f'read-only integrity check failed: {integrity}')
        schema = [list(r) for r in con.execute(
            "SELECT type,name,tbl_name,sql FROM sqlite_master "
            "WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name"
        ).fetchall()]
        tables = [r[1] for r in schema if r[0] == 'table']
        data = {}
        for table in tables:
            quoted = '"' + table.replace('"', '""') + '"'
            cols = [r[1] for r in con.execute(f'PRAGMA table_info({quoted})')]
            rows = con.execute(f'SELECT * FROM {quoted}').fetchall()
            encoded = [dict(zip(cols, (sql_value(v) for v in row))) for row in rows]
            encoded.sort(key=canonical)
            data[table] = encoded
        return {'schema': schema, 'tables': data}
    finally:
        con.close()


def snapshot_digest(snapshot):
    return hashlib.sha256(canonical(snapshot)).hexdigest()


def tree_bytes(root):
    return sum(p.stat().st_size for p in root.rglob('*') if p.is_file())


def normalized_expected(source):
    expected = json.loads(json.dumps(source))
    profile = expected['tables'].get('workspace_profile')
    if profile is None or len(profile) != 1:
        raise Stop('source does not have exactly one workspace_profile row')
    if profile[0].get('purpose') != 'test':
        raise Stop('source workspace was not created by init --purpose test')
    profile[0]['purpose'] = 'owner'
    profile[0]['changed_at'] = None
    bindings = expected['tables'].get('edit_session_bindings')
    if bindings is None or bindings:
        raise Stop('fixture edit_session_bindings is absent or nonempty')
    return expected


def note_bytes(snapshot):
    for row in snapshot['tables'].get('records', []):
        body = row.get('body')
        if not isinstance(body, str):
            continue
        try:
            record = json.loads(body)
            note = record['data']['Note']
        except (ValueError, KeyError, TypeError):
            continue
        if note.get('title') == NOTE_TITLE:
            return note['markdown'].encode('utf-8')
    raise Stop('synthetic Note was not found in records')


def safe_env(root):
    secret_prefixes = ('BRN_', 'CODEX_', 'OPENAI_', 'ANTHROPIC_', 'AZURE_',
                       'AWS_', 'GOOGLE_', 'GITHUB_', 'GH_', 'COHERE_',
                       'MISTRAL_', 'HF_')
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(secret_prefixes) and k not in {'GITHUB_TOKEN', 'GH_TOKEN'}}
    home, tmp = root / 'home', root / 'tmp'
    home.mkdir(mode=0o700, exist_ok=True)
    tmp.mkdir(mode=0o700, exist_ok=True)
    env.update(HOME=str(home), TMPDIR=str(tmp), TMP=str(tmp), TEMP=str(tmp))
    return env


def run_cli(binary, root, data_dir, args, label, timeout=CLI_TIMEOUT):
    argv = [str(binary), '--json', '--data-dir', str(data_dir), *args]
    started = time.monotonic()
    try:
        result = subprocess.run(argv, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                env=safe_env(root), timeout=timeout, check=False)
    except subprocess.TimeoutExpired as e:
        raise Stop(f'{label} exceeded {timeout}s') from e
    elapsed = time.monotonic() - started
    receipt = {
        'label': label, 'argv': argv[1:], 'exit': result.returncode,
        'seconds': round(elapsed, 3), 'stdout_bytes': len(result.stdout),
        'stderr_bytes': len(result.stderr),
        'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
        'stderr_sha256': hashlib.sha256(result.stderr).hexdigest(),
    }
    if result.returncode != 0:
        raise Stop(f'{label} failed with exit {result.returncode}; '
                   f'stdout/stderr hashes are in the receipt')
    return receipt


def stage_sizes(parent):
    stages = []
    for p in parent.iterdir():
        if not p.name.startswith(STAGE_PREFIX) or not p.is_dir() or p.is_symlink():
            continue
        files = []
        workspace_total = 0
        for child in p.rglob('*'):
            if child.is_file() and not child.is_symlink():
                size = child.stat().st_size
                if 'workspace' in child.relative_to(p).parts:
                    workspace_total += size
                if child.name in (DB_NAME, DB_NAME + '-wal'):
                    files.append({'path': str(child), 'bytes': size})
        stages.append({'path': str(p), 'workspace_bytes': workspace_total,
                       'db_files': files})
    return stages


def bounded_drain(stream, destination, result):
    digest = hashlib.sha256()
    total = 0
    saved = 0
    with destination.open('wb') as output:
        while True:
            block = stream.read(64 * 1024)
            if not block:
                break
            digest.update(block)
            total += len(block)
            if saved < 64 * 1024:
                keep = block[:64 * 1024 - saved]
                output.write(keep)
                saved += len(keep)
    result.update(bytes=total, saved_bytes=saved, sha256=digest.hexdigest())


def stop_child_once(proc, state):
    if proc.poll() is not None or state['kill_attempted']:
        return
    state['kill_attempted'] = True
    proc.kill()
    proc.wait(timeout=10)


def source_preflight(repo, binary):
    if not binary.is_file() or binary.is_symlink():
        raise Stop('frozen CLI artifact is missing or is a symlink')
    actual = sha256_file(binary)
    if actual != FROZEN_SHA256:
        raise Stop(f'frozen CLI SHA-256 mismatch: {actual}')
    qualification = repo / 'docs/work/active/vacation-campaign/evidence/precise-edit-quote-qualification.json'
    try:
        prior = json.loads(qualification.read_text(encoding='utf-8'))
        recorded = prior['binary']
        provenance_ok = (prior.get('candidate') == FROZEN_SOURCE
                         and Path(recorded['path']) == binary
                         and recorded.get('sha256') == FROZEN_SHA256)
    except (OSError, ValueError, KeyError, TypeError):
        provenance_ok = False
    if not provenance_ok:
        raise Stop('frozen binary/source qualification receipt does not prove this artifact')
    for ref in (FROZEN_SOURCE, 'main205'):
        r = subprocess.run(['git', '-C', str(repo), 'rev-parse', '--verify', ref],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        if r.returncode != 0:
            raise Stop(f'required source ref is unavailable: {ref}')
    scoped = ['Cargo.toml', 'Cargo.lock', 'crates']
    dirty = subprocess.run(['git', '-C', str(repo), 'status', '--porcelain',
                            '--untracked-files=all', '--', *scoped],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if dirty.returncode != 0 or dirty.stdout.strip():
        raise Stop('runtime crates or Cargo graph have staged/working/untracked changes')
    head_diff = subprocess.run(['git', '-C', str(repo), 'diff', '--quiet',
                                'main205', 'HEAD', '--', *scoped], check=False)
    if head_diff.returncode == 1:
        raise Stop('actual worktree HEAD runtime crates/Cargo graph differs from main205')
    if head_diff.returncode != 0:
        raise Stop('could not compare actual worktree HEAD to main205')
    diff = subprocess.run(
        ['git', '-C', str(repo), 'diff', '--quiet', FROZEN_SOURCE, 'main205',
         '--', *scoped], check=False)
    if diff.returncode == 1:
        raise Stop('runtime crate/Cargo graph differs between frozen source and main205')
    if diff.returncode != 0:
        raise Stop('could not verify frozen source against main205')
    return {'binary': str(binary), 'binary_sha256': actual,
            'frozen_source': FROZEN_SOURCE,
            'main205': subprocess.check_output(
                ['git', '-C', str(repo), 'rev-parse', 'main205'], text=True).strip(),
            'runtime_crates_and_cargo_graph_diff': 'empty'}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo', required=True, type=Path,
                    help='the reviewed worktree containing main205 and frozen source')
    ap.add_argument('--root', required=True, type=Path,
                    help='absolute, nonexistent task-owned data/evidence root')
    ap.add_argument('--run-qualification', action='store_true',
                    help='explicitly start the bounded synthetic CLI workload')
    ns = ap.parse_args()
    if not ns.run_qualification:
        raise Stop('driver is inert without --run-qualification')
    repo = ns.repo.resolve(strict=True)
    root = ns.root
    if not root.is_absolute() or root.exists() or root.is_symlink():
        raise Stop('run root must be absolute, nonexistent, and not a symlink')
    if root.resolve(strict=False) != root:
        raise Stop('run root path contains a symlink or noncanonical component')
    if shutil.disk_usage(root.parent).free < MIN_FREE:
        raise Stop('less than 2 GiB is free before workload creation')
    proof = source_preflight(repo, FROZEN_BIN)
    root.mkdir(mode=0o700)
    root = root.resolve(strict=True)
    receipt = {'qualification': 'public-cli-restore-interruption',
               'run_id': str(uuid.uuid4()), 'root': str(root),
               'source_proof': proof, 'commands': [], 'outcome': 'running'}
    receipt_path = root / 'receipt.json'
    child = None
    child_state = {'kill_attempted': False}
    body_path = root / 'synthetic-note.md'
    try:
        env_root = root
        source = root / 'source'
        destination_parent = root / 'destination-parent'
        source.mkdir(mode=0o700)
        destination_parent.mkdir(mode=0o700)
        dest = destination_parent / 'restored'
        backup = root / 'source-backup.sqlite3'
        sentinel = destination_parent / 'sibling-sentinel.txt'
        sentinel_bytes = b'BRN restore qualification sibling sentinel\n'
        sentinel.write_bytes(sentinel_bytes)
        pattern = '雪と海 · Καλημέρα · مرحبا · note bytes preserved\n'.encode('utf-8')
        body = pattern * (MAX_BODY // len(pattern))
        if not body or len(body) > MAX_BODY:
            raise Stop('synthetic body exceeded the 8 MiB bound')
        body_path.write_bytes(body)
        receipt['fixture'] = {'note_title': NOTE_TITLE, 'note_bytes': len(body),
                              'note_sha256': hashlib.sha256(body).hexdigest(),
                              'sibling_sentinel_sha256': hashlib.sha256(sentinel_bytes).hexdigest()}
        receipt['commands'].append(run_cli(FROZEN_BIN, env_root, source,
                                          ['init', '--purpose', 'test'], 'init-test'))
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after init')
        receipt['commands'].append(run_cli(FROZEN_BIN, env_root, source,
                                          ['note-create', '--title', NOTE_TITLE,
                                           '--text-file', str(body_path)], 'note-create'))
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after Note creation')
        receipt['commands'].append(run_cli(FROZEN_BIN, env_root, source,
                                          ['backup', '--destination', str(backup)], 'backup'))
        if not backup.is_file() or backup.stat().st_size > MAX_BACKUP:
            raise Stop('source backup is absent or exceeds 128 MiB')
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB before restore')
        receipt['source_backup'] = {'path': str(backup), 'bytes': backup.stat().st_size,
                                    'sha256': sha256_file(backup)}
        source_db = source / DB_NAME
        source_snapshot = readonly_snapshot(source_db)
        expected = normalized_expected(source_snapshot)
        source_marker = source / MARKER_NAME
        if not source_marker.is_file():
            raise Stop('public CLI source workspace marker is missing')
        source_marker_hash = sha256_file(source_marker)
        source_note = note_bytes(source_snapshot)
        if source_note != body:
            raise Stop('public CLI source Note bytes differ from supplied UTF-8 bytes')
        source_bindings = source_snapshot['tables']['edit_session_bindings']
        if source_bindings:
            raise Stop('fixture unexpectedly has edit-session bindings')
        if dest.exists() or dest.is_symlink() or stage_sizes(destination_parent):
            raise Stop('fresh restore destination parent is not empty of restore stages')
        receipt['source_snapshot_sha256'] = snapshot_digest(source_snapshot)
        receipt['source_note_sha256'] = hashlib.sha256(source_note).hexdigest()

        # Keep only this process handle. No process-name or process-group kills.
        out_path, err_path = root / 'restore.stdout.prefix', root / 'restore.stderr.prefix'
        argv = [str(FROZEN_BIN), '--json', '--data-dir', str(dest),
                'restore', '--source', str(backup)]
        child_started = time.monotonic()
        child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, env=safe_env(env_root))
        stream_stats = {'stdout': {}, 'stderr': {}}
        drains = [threading.Thread(target=bounded_drain,
                                   args=(child.stdout, out_path, stream_stats['stdout']), daemon=True),
                  threading.Thread(target=bounded_drain,
                                   args=(child.stderr, err_path, stream_stats['stderr']), daemon=True)]
        for thread in drains:
            thread.start()
        observed = None
        deadline = child_started + OBSERVE_SECONDS
        try:
            while time.monotonic() < deadline:
                status = child.poll()
                if status is not None:
                    receipt['restore_natural_exit'] = status
                    break
                stages = stage_sizes(destination_parent)
                qualifying = [s for s in stages if 1024 * 1024 < s['workspace_bytes'] < backup.stat().st_size
                              and sum(x['bytes'] for x in s['db_files']) > 1024 * 1024]
                if qualifying:
                    observed = qualifying[0]
                    break
                time.sleep(0.01)
            if observed is None and child.poll() is None:
                receipt['outcome'] = 'inconclusive_observation_window_elapsed'
                receipt['stage_sizes_at_window_end'] = stage_sizes(destination_parent)
                stop_child_once(child, child_state)
                receipt['restore_child_exit'] = child.returncode
                raise Stop('no qualifying partial stage observed within ten seconds')
            if observed is None:
                receipt['outcome'] = 'inconclusive_child_finished_before_observation'
                raise Stop('restore child finished before a partial stage was observed')
            receipt['observed_stage'] = observed
            stop_child_once(child, child_state)
            receipt['restore_child_exit'] = child.returncode
            post = stage_sizes(destination_parent)
            receipt['post_kill_stages'] = post
            receipt['restore_kill_signal'] = signal.SIGKILL
            if child.returncode != -signal.SIGKILL:
                receipt['outcome'] = 'inconclusive_child_not_signal_terminated'
                raise Stop('owned restore child did not report SIGKILL termination')
            if dest.exists() or dest.is_symlink():
                receipt['outcome'] = 'inconclusive_final_path_exists_after_kill'
                raise Stop('final destination exists after killed restore')
            kept = [s for s in post if Path(s['path']).exists()]
            partial = [s for s in kept if 0 < s['workspace_bytes'] < backup.stat().st_size]
            if not partial:
                receipt['outcome'] = 'inconclusive_post_kill_stage_not_proven_partial'
                raise Stop('post-kill stage bytes do not prove a retained partial copy')
            receipt['qualified_partial_stage'] = partial[0]
        finally:
            if child.poll() is None:
                try:
                    stop_child_once(child, child_state)
                except Exception as cleanup_error:
                    receipt['restore_cleanup_error'] = type(cleanup_error).__name__
            for thread in drains:
                thread.join(timeout=10)
            receipt['restore_output'] = stream_stats

        receipt['commands'].append(run_cli(FROZEN_BIN, env_root, dest,
                                          ['restore', '--source', str(backup)],
                                          'restore-retry'))
        if dest.exists() is False or not dest.is_dir():
            raise Stop('retry did not publish the restored destination')
        remaining = stage_sizes(destination_parent)
        receipt['remaining_restore_stages'] = remaining
        if remaining:
            raise Stop('retry left an abandoned restore stage')
        if sha256_file(backup) != receipt['source_backup']['sha256']:
            raise Stop('source backup bytes changed')
        if sha256_file(sentinel) != receipt['fixture']['sibling_sentinel_sha256']:
            raise Stop('unrelated sibling sentinel changed')
        source_after = readonly_snapshot(source_db)
        if source_after != source_snapshot or sha256_file(source_marker) != source_marker_hash:
            raise Stop('source workspace changed during restore/retry')
        restored = readonly_snapshot(dest / DB_NAME)
        if restored != expected:
            receipt['restored_snapshot_sha256'] = snapshot_digest(restored)
            raise Stop('restored logical schema or table rows differ from expected normalization')
        restored_note = note_bytes(restored)
        if restored_note != body:
            raise Stop('restored Note UTF-8 bytes differ from source')
        receipt['restored_snapshot_sha256'] = snapshot_digest(restored)
        receipt['restored_note_sha256'] = hashlib.sha256(restored_note).hexdigest()
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB')
        receipt['outcome'] = 'qualified_partial_copy_retry_and_exact_data_checks_passed'
    except Exception as e:
        if child is not None and child.poll() is None:
            try:
                stop_child_once(child, child_state)
            except Exception as cleanup_error:
                receipt['cleanup_error'] = type(cleanup_error).__name__
        if receipt.get('outcome') == 'running':
            receipt['outcome'] = 'failed_or_inconclusive'
        receipt['reason'] = str(e)[:400]
    finally:
        try:
            if body_path.exists():
                body_path.unlink()
            receipt['task_root_bytes'] = tree_bytes(root)
            receipt_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + '\n',
                                    encoding='utf-8')
        except Exception as e:
            print(f'receipt_write_error={type(e).__name__}', file=sys.stderr)
    print(json.dumps({'receipt': str(receipt_path), 'outcome': receipt['outcome'],
                      'source_backup_sha256': receipt.get('source_backup', {}).get('sha256'),
                      'task_root_bytes': receipt.get('task_root_bytes')}, ensure_ascii=False))
    return 0 if receipt['outcome'] == 'qualified_partial_copy_retry_and_exact_data_checks_passed' else 2


if __name__ == '__main__':
    try:
        sys.exit(main())
    except Stop as e:
        print(f'preflight_stop={str(e)[:300]}', file=sys.stderr)
        sys.exit(2)
