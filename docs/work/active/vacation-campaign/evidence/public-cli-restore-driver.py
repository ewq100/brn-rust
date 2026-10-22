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
import stat
import subprocess
import sys
import threading
import time
import uuid

FROZEN_BIN = Path('/Users/evokessler/.codex/artifacts/brn-precise-edit-quote-28fd581e/brn')
FROZEN_SHA256 = '242b141ce518f0ff87d58f7e36b3250269c344211d1552a3d96206cfc62e5f7d'
FROZEN_SOURCE = '28fd581ea8451ecd95d899248132b62c8f890d03'
MAIN205 = '0016b21057b44fa9a64cef2d77d4ae09e317f540'
RUNTIME_PATHS = [
    'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
    'crates/brn', 'crates/brn-ai', 'crates/brn-threads-core',
    'crates/brn-threads-app', 'crates/brn-threads-intake',
    'crates/brn-intake', 'crates/brn-retrieval',
]
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


def tree_snapshot(root):
    entries = []
    for path in sorted(root.rglob('*')):
        rel = str(path.relative_to(root))
        meta = path.lstat()
        if stat.S_ISLNK(meta.st_mode):
            raise Stop('unexpected symlink in completed restore tree')
        if stat.S_ISDIR(meta.st_mode):
            entries.append({'path': rel, 'kind': 'directory',
                            'mode': stat.S_IMODE(meta.st_mode)})
        elif stat.S_ISREG(meta.st_mode):
            entries.append({'path': rel, 'kind': 'file',
                            'mode': stat.S_IMODE(meta.st_mode), 'bytes': meta.st_size,
                            'sha256': sha256_file(path)})
        else:
            raise Stop('unexpected nonregular object in completed restore tree')
    return entries


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
    tmp = root / 'tmp'
    tmp.mkdir(mode=0o700, exist_ok=True)
    env.update(TMPDIR=str(tmp))
    return env


def start_drains(proc, prefix):
    stats = {'stdout': {'complete': False}, 'stderr': {'complete': False}}
    paths = {'stdout': prefix.with_name(prefix.name + '.stdout.prefix'),
             'stderr': prefix.with_name(prefix.name + '.stderr.prefix')}
    threads = []
    for name, stream in [('stdout', proc.stdout), ('stderr', proc.stderr)]:
        thread = threading.Thread(target=bounded_drain,
                                  args=(stream, paths[name], stats[name]), daemon=True)
        threads.append(thread)
        try:
            thread.start()
        except BaseException as error:
            stats[name]['error'] = type(error).__name__
            stats[name]['complete'] = False
    return stats, paths, threads


def finish_drains(stats, threads, timeout=10):
    for thread in threads:
        if thread.ident is not None:
            thread.join(timeout=timeout)
    for name, thread in zip(('stdout', 'stderr'), threads):
        stats[name]['thread_alive'] = thread.is_alive()
        stats[name]['complete'] = (bool(stats[name].get('complete'))
                                   and thread.ident is not None and not thread.is_alive())
    return all(s['complete'] and not s.get('error') for s in stats.values())


def run_cli(binary, root, data_dir, args, label, attempts, timeout=CLI_TIMEOUT):
    argv = [str(binary), '--json', '--data-dir', str(data_dir), *args]
    attempt = {'label': label, 'argv': argv[1:], 'timeout_seconds': timeout,
               'status': 'starting'}
    attempts.append(attempt)
    started = time.monotonic()
    proc = None
    state = {'signal_attempted': False, 'reaped': False}
    stats = None
    threads = []
    paths = {}
    limit_at = started + timeout
    try:
        proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, env=safe_env(root))
        stats, paths, threads = start_drains(proc, root / f'{label}.capture')
        try:
            exit_code = proc.wait(timeout=timeout)
            state['reaped'] = True
        except subprocess.TimeoutExpired:
            attempt['status'] = 'timeout'
            attempt['timed_out'] = True
            signal_owned_child_once(proc, state)
            wait_owned_child(proc, state, 10)
            exit_code = proc.returncode
        attempt['exit'] = exit_code
        attempt['seconds'] = round(time.monotonic() - started, 3)
        attempt['timed_out'] = time.monotonic() >= limit_at and attempt.get('status') == 'timeout'
    except BaseException as error:
        attempt['status'] = 'launch_or_wait_error'
        attempt['error_type'] = type(error).__name__
        attempt['error'] = str(error)[:300]
        raise
    finally:
        if proc is not None and not state['reaped']:
            try:
                if proc.poll() is None:
                    signal_owned_child_once(proc, state)
                wait_owned_child(proc, state, 10)
            except BaseException as cleanup_error:
                attempt['cleanup_error'] = type(cleanup_error).__name__
                attempt['cleanup_error_text'] = str(cleanup_error)[:200]
        if threads:
            ok = finish_drains(stats, threads)
            attempt['output'] = stats
            attempt['output_prefix_paths'] = {k: str(v) for k, v in paths.items()}
            attempt['output_complete'] = ok
            if not ok:
                attempt['status'] = 'output_capture_failed'
        attempt.setdefault('exit', proc.returncode if proc is not None else None)
        attempt.setdefault('seconds', round(time.monotonic() - started, 3))
    if attempt.get('status') == 'starting':
        attempt['status'] = 'ok' if attempt.get('exit') == 0 else 'nonzero_exit'
    if attempt.get('status') != 'ok':
        raise Stop(f'{label} did not complete with successful bounded output capture')
    return attempt


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
    try:
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
        result.update(bytes=total, saved_bytes=saved, sha256=digest.hexdigest(), complete=True)
    except BaseException as error:
        result.update(bytes=total, saved_bytes=saved, sha256=digest.hexdigest(),
                      complete=False, error=type(error).__name__)


def signal_owned_child_once(proc, state):
    if proc.poll() is not None or state['signal_attempted']:
        return
    state['signal_attempted'] = True
    proc.kill()


def wait_owned_child(proc, state, timeout=10):
    if state['reaped']:
        return proc.returncode
    result = proc.wait(timeout=timeout)
    state['reaped'] = True
    return result


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
    for ref in (FROZEN_SOURCE, MAIN205):
        r = subprocess.run(['git', '-C', str(repo), 'rev-parse', '--verify', ref],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        if r.returncode != 0:
            raise Stop(f'required source ref is unavailable: {ref}')
    dirty = subprocess.run(['git', '-C', str(repo), 'status', '--porcelain',
                            '--untracked-files=all', '--', *RUNTIME_PATHS],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if dirty.returncode != 0 or dirty.stdout.strip():
        raise Stop('runtime/Cargo/toolchain paths have staged/working/untracked changes')
    head_diff = subprocess.run(['git', '-C', str(repo), 'diff', '--quiet',
                                MAIN205, 'HEAD', '--', *RUNTIME_PATHS], check=False)
    if head_diff.returncode == 1:
        raise Stop('actual worktree HEAD runtime/Cargo/toolchain differs from pinned main205')
    if head_diff.returncode != 0:
        raise Stop('could not compare actual worktree HEAD to main205')
    diff = subprocess.run(
        ['git', '-C', str(repo), 'diff', '--quiet', FROZEN_SOURCE, MAIN205,
         '--', *RUNTIME_PATHS], check=False)
    if diff.returncode == 1:
        raise Stop('runtime/Cargo/toolchain differs between frozen source and pinned main205')
    if diff.returncode != 0:
        raise Stop('could not verify frozen source against main205')
    head = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
    return {'binary': str(binary), 'binary_sha256': actual,
            'frozen_source': FROZEN_SOURCE, 'main205_sha': MAIN205,
            'actual_head': head, 'compared_paths': RUNTIME_PATHS,
            'source_to_main205_diff': 'empty', 'main205_to_actual_head_diff': 'empty',
            'selected_paths_working_or_untracked_diff': 'empty'}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo', required=True, type=Path,
                    help='the reviewed worktree containing pinned source commits')
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
    child_state = {'signal_attempted': False, 'reaped': False}
    restore_attempt = None
    restore_stats = None
    restore_paths = None
    restore_threads = []
    restore_output_finished = False
    body_path = root / 'synthetic-note.md'
    interrupted = False
    evidence_error = None

    try:
        env_root = root
        source = root / 'source'
        destination_parent = root / 'destination-parent'
        source.mkdir(mode=0o700)
        destination_parent.mkdir(mode=0o700)
        complete_dest = destination_parent / 'complete-restored'
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

        run_cli(FROZEN_BIN, env_root, source, ['init', '--purpose', 'test'],
                'init-test', receipt['commands'])
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after init')
        run_cli(FROZEN_BIN, env_root, source,
                ['note-create', '--title', NOTE_TITLE, '--text-file', str(body_path)],
                'note-create', receipt['commands'])
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after Note creation')
        run_cli(FROZEN_BIN, env_root, source, ['backup', '--destination', str(backup)],
                'backup', receipt['commands'])
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
        if source_snapshot['tables']['edit_session_bindings']:
            raise Stop('fixture unexpectedly has edit-session bindings')
        if complete_dest.exists() or dest.exists() or stage_sizes(destination_parent):
            raise Stop('fresh restore destination parent is not empty of restore stages')
        receipt['source_snapshot_sha256'] = snapshot_digest(source_snapshot)
        receipt['source_note_sha256'] = hashlib.sha256(source_note).hexdigest()

        # Preserve a prior complete public restore and prove its complete file tree
        # stays byte-for-byte and path-for-path unchanged through the later retry.
        run_cli(FROZEN_BIN, env_root, complete_dest,
                ['restore', '--source', str(backup)], 'restore-complete', receipt['commands'])
        complete_snapshot = readonly_snapshot(complete_dest / DB_NAME)
        if complete_snapshot != expected:
            raise Stop('initial complete public restore differs from expected data')
        complete_tree = tree_snapshot(complete_dest)
        receipt['initial_complete_restore'] = {
            'path': str(complete_dest), 'tree': complete_tree,
            'tree_sha256': hashlib.sha256(canonical(complete_tree)).hexdigest(),
        }
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after complete restore')
        if dest.exists() or dest.is_symlink() or stage_sizes(destination_parent):
            raise Stop('destination parent changed after complete restore')

        # The only interruption launch. Record before process creation so every
        # launch, wait, signal and capture outcome remains in the receipt.
        argv = [str(FROZEN_BIN), '--json', '--data-dir', str(dest),
                'restore', '--source', str(backup)]
        restore_attempt = {'label': 'restore-interrupt-once', 'argv': argv[1:],
                           'observation_seconds': OBSERVE_SECONDS, 'status': 'starting'}
        receipt['commands'].append(restore_attempt)
        child_started = time.monotonic()
        try:
            child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, env=safe_env(env_root))
            restore_stats, restore_paths, restore_threads = start_drains(
                child, root / 'restore-interrupt.capture')
        except BaseException as launch_error:
            restore_attempt['status'] = 'launch_or_capture_start_error'
            restore_attempt['error_type'] = type(launch_error).__name__
            restore_attempt['error'] = str(launch_error)[:300]
            raise

        observed = None
        deadline = child_started + OBSERVE_SECONDS
        while time.monotonic() < deadline:
            child_status = child.poll()
            if child_status is not None:
                restore_attempt['status'] = 'finished_before_partial_observation'
                restore_attempt['exit'] = child_status
                break
            stages = stage_sizes(destination_parent)
            qualifying = [s for s in stages
                          if 1024 * 1024 < s['workspace_bytes'] < backup.stat().st_size
                          and sum(x['bytes'] for x in s['db_files']) > 1024 * 1024]
            if qualifying:
                observed = qualifying[0]
                break
            time.sleep(0.01)

        if observed is None and child.poll() is None:
            restore_attempt['status'] = 'observation_window_elapsed_cleanup_kill'
            receipt['stage_sizes_at_window_end'] = stage_sizes(destination_parent)
            raise Stop('no qualifying partial stage observed within ten seconds')
        if observed is None:
            receipt['outcome'] = 'inconclusive_child_finished_before_observation'
            raise Stop('restore child finished before a partial stage was observed')

        restore_attempt['observed_stage'] = observed
        signal_owned_child_once(child, child_state)
        restore_attempt['signal_attempted'] = child_state['signal_attempted']
        restore_attempt['signal'] = signal.SIGKILL
        wait_owned_child(child, child_state, 10)
        restore_attempt['exit'] = child.returncode
        restore_attempt['reaped'] = child_state['reaped']
        restore_attempt['seconds'] = round(time.monotonic() - child_started, 3)
        post = stage_sizes(destination_parent)
        receipt['post_kill_stages'] = post
        exact_stage = next((s for s in post if s['path'] == observed['path']), None)
        receipt['post_kill_observed_stage'] = exact_stage
        if child.returncode != -signal.SIGKILL:
            restore_attempt['status'] = 'child_not_signal_terminated'
            raise Stop('owned restore child did not report SIGKILL termination')
        if dest.exists() or dest.is_symlink():
            restore_attempt['status'] = 'final_path_exists_after_kill'
            raise Stop('final destination exists after killed restore')
        if exact_stage is None or not (0 < exact_stage['workspace_bytes'] < backup.stat().st_size):
            restore_attempt['status'] = 'post_kill_partial_not_proven'
            raise Stop('same observed stage is not retained as a proven partial copy')
        receipt['qualified_partial_stage'] = exact_stage

        output_ok = finish_drains(restore_stats, restore_threads)
        restore_output_finished = True
        restore_attempt['output'] = restore_stats
        restore_attempt['output_prefix_paths'] = {k: str(v) for k, v in restore_paths.items()}
        restore_attempt['output_complete'] = output_ok
        if not output_ok:
            restore_attempt['status'] = 'output_capture_failed'
            raise Stop('interrupted restore stdout/stderr capture did not complete')
        restore_attempt['status'] = 'killed_partial_stage_qualified'

        # Exactly one retry, reachable only after retained same-stage partial proof.
        run_cli(FROZEN_BIN, env_root, dest, ['restore', '--source', str(backup)],
                'restore-retry', receipt['commands'])
        if not dest.is_dir():
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
        final_complete_tree = tree_snapshot(complete_dest)
        if final_complete_tree != receipt['initial_complete_restore']['tree']:
            raise Stop('the earlier complete restore output tree changed')
        receipt['earlier_complete_restore_unchanged'] = True
        if tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB')
        receipt['outcome'] = 'qualified_partial_copy_retry_and_exact_data_checks_passed'

    except BaseException as error:
        interrupted = isinstance(error, KeyboardInterrupt)
        if receipt.get('outcome') == 'running':
            receipt['outcome'] = 'interrupted_nonqualified' if interrupted else 'failed_or_inconclusive'
        receipt['reason'] = str(error)[:400] or type(error).__name__
    finally:
        # Signal at most once, but always make a bounded wait/reap attempt even
        # when the signal call or an earlier wait failed.
        if child is not None:
            if child.poll() is None:
                try:
                    signal_owned_child_once(child, child_state)
                    if restore_attempt is not None:
                        restore_attempt['signal_attempted'] = child_state['signal_attempted']
                        restore_attempt['signal'] = signal.SIGKILL
                        if restore_attempt.get('status') == 'starting':
                            restore_attempt['status'] = 'cleanup_killed_owned_child'
                except BaseException as cleanup_error:
                    receipt['cleanup_signal_error'] = type(cleanup_error).__name__
                    receipt['cleanup_signal_error_text'] = str(cleanup_error)[:200]
            try:
                wait_owned_child(child, child_state, 10)
            except BaseException as cleanup_error:
                receipt['cleanup_reap_error'] = type(cleanup_error).__name__
                receipt['cleanup_reap_error_text'] = str(cleanup_error)[:200]
            if restore_attempt is not None:
                restore_attempt['exit'] = child.returncode
                restore_attempt['reaped'] = child_state['reaped']
                if not child_state['reaped']:
                    receipt['outcome'] = 'failed_owned_child_not_reaped'
        if restore_threads and not restore_output_finished:
            try:
                output_ok = finish_drains(restore_stats, restore_threads)
                restore_attempt['output'] = restore_stats
                restore_attempt['output_prefix_paths'] = {k: str(v) for k, v in restore_paths.items()}
                restore_attempt['output_complete'] = output_ok
                if not output_ok:
                    receipt['outcome'] = 'failed_output_capture'
            except BaseException as drain_error:
                restore_attempt['output_capture_error'] = type(drain_error).__name__
                receipt['outcome'] = 'failed_output_capture'
        try:
            if body_path.exists():
                body_path.unlink()
        except BaseException as cleanup_error:
            receipt['fixture_cleanup_error'] = type(cleanup_error).__name__
            receipt['outcome'] = 'failed_fixture_cleanup'
        try:
            receipt['task_root_bytes'] = tree_bytes(root)
            receipt_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + '\n',
                                    encoding='utf-8')
        except BaseException as write_error:
            evidence_error = f'{type(write_error).__name__}: {str(write_error)[:200]}'
            receipt['outcome'] = 'failed_evidence_persistence'
            print(f'receipt_persistence_failed={evidence_error}', file=sys.stderr)

    print(json.dumps({'receipt': str(receipt_path), 'outcome': receipt['outcome'],
                      'source_backup_sha256': receipt.get('source_backup', {}).get('sha256'),
                      'task_root_bytes': receipt.get('task_root_bytes'),
                      'receipt_written': evidence_error is None}, ensure_ascii=False))
    if evidence_error is not None:
        return 2
    if interrupted:
        return 130
    return 0 if receipt['outcome'] == 'qualified_partial_copy_retry_and_exact_data_checks_passed' else 2


if __name__ == '__main__':
    try:
        sys.exit(main())
    except Stop as e:
        print(f'preflight_stop={str(e)[:300]}', file=sys.stderr)
        sys.exit(2)
