#!/usr/bin/env python3
"""Inert, bounded public-CLI backup interruption qualification driver."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import stat
import subprocess
import sys
import time
import uuid

REPO = Path('/Users/evokessler/.codex/worktrees/export-durability/brn-rust')
SOURCE_COMMIT = '7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5'
BINARY = Path('/Users/evokessler/.codex/artifacts/brn-backup-interruption-7dde6a41/brn')
QUALIFICATION = Path('/tmp/brn-public-backup-interruption/cli-qualification.json')
HARNESS = Path('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/docs/work/active/vacation-campaign/evidence/public-cli-restore-driver.py')
HARNESS_SHA256 = '17fa8390811d961db2940dd95582b3e16cb597b57aad97195a069905d64f4cfd'
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
OBSERVE_SECONDS = 10
MIN_COPY_BYTES = 1024 * 1024
DB_NAME = 'threads.sqlite3'
MARKER_NAME = '.brn-threads-data'
STAGE_PREFIX = '.brn-backup-stage-'
NOTE_TITLE = 'Public backup interruption qualification · Ω'
QUALIFIED = 'qualified_partial_backup_retry_and_exact_data_checks_passed'


class Stop(Exception):
    pass


def sha256_file(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def load_harness():
    if not HARNESS.is_file() or HARNESS.is_symlink():
        raise Stop('reviewed restore harness is missing or is a symlink')
    actual = sha256_file(HARNESS)
    if actual != HARNESS_SHA256:
        raise Stop('reviewed restore harness SHA-256 mismatch')
    spec = importlib.util.spec_from_file_location('brn_reviewed_restore_harness', HARNESS)
    if spec is None or spec.loader is None:
        raise Stop('reviewed restore harness cannot be loaded')
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    # The imported extractor uses this module global. Keep it aligned with the
    # adapter's public Note creation and exact-byte checks.
    module.NOTE_TITLE = NOTE_TITLE
    return module, actual


def validate_root_path(root):
    if not root.is_absolute() or root.exists() or root.is_symlink():
        raise Stop('run root must be absolute, nonexistent, and not a symlink')
    private_tmp = Path('/private/tmp').resolve(strict=True)
    parent = root.parent.resolve(strict=True)
    if parent != private_tmp or root.resolve(strict=False) != root:
        raise Stop('run root must be a canonical direct child of /private/tmp')
    if not stat.S_ISDIR(parent.stat().st_mode):
        raise Stop('/private/tmp is not a real directory')
    if shutil.disk_usage(parent).free < MIN_FREE:
        raise Stop('less than 2 GiB is free before workload creation')


def git_output(repo, *args):
    result = subprocess.run(['git', '-C', str(repo), *args],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            check=False, text=True)
    if result.returncode:
        raise Stop('source checkout validation command failed')
    return result.stdout.strip()


def validate_provenance(repo, binary, qualification):
    if repo.resolve(strict=True) != REPO.resolve(strict=True):
        raise Stop('repo must be the selected source checkout')
    if qualification != QUALIFICATION:
        raise Stop('qualification must be the selected current-source CLI receipt')
    if git_output(repo, 'rev-parse', 'HEAD') != SOURCE_COMMIT:
        raise Stop('selected source checkout is not at the pinned source commit')
    git_output(repo, 'cat-file', '-e', f'{SOURCE_COMMIT}^{{commit}}')
    dirty = git_output(repo, 'status', '--porcelain', '--untracked-files=all',
                       '--', *RUNTIME_PATHS)
    if dirty:
        raise Stop('runtime/Cargo/toolchain paths have staged, working, or untracked changes')
    if not binary.is_absolute() or binary != BINARY or not binary.is_file() or binary.is_symlink():
        raise Stop('current-source CLI artifact is missing, unexpected, or a symlink')
    binary_hash = sha256_file(binary)
    if not qualification.is_file() or qualification.is_symlink():
        raise Stop('current CLI qualification receipt is missing or a symlink')
    if qualification.stat().st_size > 1024 * 1024:
        raise Stop('current CLI qualification receipt exceeds 1 MiB')
    try:
        receipt = json.loads(qualification.read_text(encoding='utf-8'))
    except (OSError, ValueError):
        raise Stop('current CLI qualification receipt is unreadable') from None
    if not isinstance(receipt, dict):
        raise Stop('current CLI qualification receipt is not an object')
    recorded_source = receipt.get('source_commit')
    binary_record = receipt.get('binary')
    if not isinstance(binary_record, dict):
        raise Stop('CLI receipt has no structured binary identity')
    recorded_path = binary_record.get('path')
    recorded_hash = binary_record.get('sha256')
    recorded_tree = git_output(repo, 'rev-parse', f'{SOURCE_COMMIT}^{{tree}}')
    if (recorded_source != SOURCE_COMMIT or recorded_path != str(binary)
            or recorded_hash != binary_hash
            or binary_record.get('bytes') != binary.stat().st_size
            or receipt.get('source_worktree') != str(repo)
            or receipt.get('source_tree') != recorded_tree
            or receipt.get('source_clean_after_build') is not True
            or receipt.get('process_exit_code') != 0
            or receipt.get('command') != 'cargo build -p brn --locked --offline --jobs 1'
            or not str(receipt.get('rust_toolchain', '')).startswith('1.98.1-')):
        raise Stop('CLI receipt does not bind the expected source, artifact path, and current hash')
    return {
        'repo': str(repo.resolve(strict=True)), 'source_commit': SOURCE_COMMIT,
        'head': SOURCE_COMMIT, 'runtime_paths': RUNTIME_PATHS,
        'runtime_paths_clean': True, 'binary': str(binary),
        'binary_sha256': binary_hash, 'qualification_receipt': str(qualification),
        'source_tree': recorded_tree, 'receipt_source': recorded_source,
    }


def backup_stages(parent):
    stages = []
    for path in parent.iterdir():
        if not path.name.startswith(STAGE_PREFIX):
            continue
        meta = path.lstat()
        if not stat.S_ISDIR(meta.st_mode):
            raise Stop('unexpected non-directory backup-stage entry in fresh output parent')
        files = {}
        for child in path.iterdir():
            child_meta = child.lstat()
            if not stat.S_ISREG(child_meta.st_mode):
                raise Stop('unexpected non-regular entry in owned backup stage')
            files[child.name] = child_meta.st_size
        copy_bytes = files.get('snapshot.sqlite3', 0) + files.get('snapshot.sqlite3-wal', 0)
        stages.append({'path': str(path), 'files': files, 'copy_bytes': copy_bytes})
    return stages


def sqlite_storage_snapshot(database):
    # Sidecar bytes are diagnostic, not immutable acceptance data: WAL can hold
    # committed pages while SHM is a transient index/coordination file. The
    # exact main-file hash and read-only logical snapshot carry source checks.
    files = {}
    for suffix in ('', '-wal', '-shm', '-journal'):
        path = database.with_name(database.name + suffix)
        try:
            meta = path.lstat()
        except FileNotFoundError:
            files[suffix or 'database'] = None
            continue
        if not stat.S_ISREG(meta.st_mode):
            raise Stop('source SQLite storage contains a non-regular database component')
        files[suffix or 'database'] = {
            'bytes': meta.st_size, 'sha256': sha256_file(path),
        }
    return files


def readonly_standalone_snapshot(harness, database):
    database = database.resolve(strict=True)
    companions = [database.with_name(database.name + suffix)
                  for suffix in ('-wal', '-shm', '-journal')]

    def require_no_companions(when):
        present = [str(path) for path in companions if path.exists() or path.is_symlink()]
        if present:
            raise Stop(f'{when}: standalone backup has companion SQLite files')

    require_no_companions('before read-only validation')
    uri = database.as_uri() + '?mode=ro'
    connection = sqlite3.connect(uri, uri=True, timeout=2)
    try:
        connection.execute('PRAGMA query_only=ON')
        journal_mode = connection.execute('PRAGMA journal_mode').fetchone()[0].lower()
    finally:
        connection.close()
    if journal_mode != 'delete':
        raise Stop('published backup is not in standalone DELETE journal mode')
    snapshot = harness.readonly_snapshot(database)
    require_no_companions('after read-only validation')
    return snapshot, {'journal_mode': journal_mode, 'companions_before': [],
                      'companions_after': []}


def require_absent(path, label):
    if path.exists() or path.is_symlink():
        raise Stop(f'{label} unexpectedly exists')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-qualification', action='store_true',
                        help='explicitly start the synthetic public CLI workload')
    parser.add_argument('--repo', type=Path, default=REPO)
    parser.add_argument('--binary', type=Path, default=BINARY)
    parser.add_argument('--qualification', type=Path, default=QUALIFICATION)
    parser.add_argument('--root', type=Path, required=True,
                        help='fresh canonical direct child of /private/tmp')
    args = parser.parse_args()
    if not args.run_qualification:
        raise Stop('driver is inert without --run-qualification')

    harness, harness_hash = load_harness()
    repo = args.repo.resolve(strict=True)
    binary = args.binary
    qualification = args.qualification
    source_proof = validate_provenance(repo, binary, qualification)
    root = args.root
    validate_root_path(root)
    root.mkdir(mode=0o700)
    root = root.resolve(strict=True)

    receipt = {
        'qualification': 'public-cli-backup-interruption', 'run_id': str(uuid.uuid4()),
        'root': str(root), 'source_proof': source_proof,
        'harness': str(HARNESS), 'harness_sha256': harness_hash,
        'limits': {'note_bytes_max': MAX_BODY, 'backup_bytes_max': MAX_BACKUP,
                   'root_bytes_max': MAX_ROOT, 'initial_free_bytes_min': MIN_FREE,
                   'cli_timeout_seconds': harness.CLI_TIMEOUT,
                   'observation_seconds': OBSERVE_SECONDS,
                   'output_prefix_bytes_max': 64 * 1024,
                   'minimum_observed_copy_bytes': MIN_COPY_BYTES},
        'commands': [], 'outcome': 'running',
    }
    receipt_path = root / 'receipt.json'
    body_path = root / 'synthetic-note.md'
    child = None
    child_state = {'signal_attempted': False, 'reaped': False}
    interrupt_attempt = None
    drain_stats = None
    drain_paths = None
    drain_threads = []
    drains_finished = False
    evidence_error = None

    try:
        source = root / 'source'
        output_parent = root / 'backup-output'
        source.mkdir(mode=0o700)
        output_parent.mkdir(mode=0o700)
        previous_backup = output_parent / 'complete.sqlite3'
        destination = output_parent / 'interrupted.sqlite3'
        sentinel = output_parent / 'sibling-sentinel.txt'
        unrelated = output_parent / 'ordinary-unrelated-directory'
        unrelated.mkdir(mode=0o700)
        sentinel_bytes = b'BRN backup interruption sibling sentinel\n'
        unrelated_bytes = b'preserve unrelated ordinary directory\n'
        sentinel.write_bytes(sentinel_bytes)
        (unrelated / 'keep.txt').write_bytes(unrelated_bytes)

        pattern = '雪と海 · Καλημέρα · مرحبا · note bytes preserved\n'.encode('utf-8')
        body = pattern * (MAX_BODY // len(pattern))
        if not body or len(body) > MAX_BODY:
            raise Stop('synthetic Note exceeds the 8 MiB bound')
        body_path.write_bytes(body)
        receipt['fixture'] = {
            'note_title': NOTE_TITLE, 'note_bytes': len(body),
            'note_sha256': hashlib.sha256(body).hexdigest(),
            'sibling_sentinel_sha256': hashlib.sha256(sentinel_bytes).hexdigest(),
            'unrelated_tree': harness.tree_snapshot(unrelated),
        }

        harness.run_cli(binary, root, source, ['init', '--purpose', 'test'],
                        'init-test', receipt['commands'])
        if harness.tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after init')
        harness.run_cli(binary, root, source,
                        ['note-create', '--title', NOTE_TITLE, '--text-file', str(body_path)],
                        'note-create', receipt['commands'])
        if harness.tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB after Note creation')

        source_db = source / DB_NAME
        source_marker = source / MARKER_NAME
        if not source_marker.is_file() or source_marker.is_symlink():
            raise Stop('public CLI source workspace marker is missing or unsafe')
        source_snapshot = harness.readonly_snapshot(source_db)
        source_note = harness.note_bytes(source_snapshot)
        if source_note != body:
            raise Stop('source Note UTF-8 bytes differ from submitted fixture bytes')
        if harness.tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB before initial backup')
        source_storage_before = sqlite_storage_snapshot(source_db)
        source_before = {
            'database_file': source_storage_before['database'],
            'sidecars_diagnostic': {key: value for key, value in source_storage_before.items()
                                    if key != 'database'},
            'marker_sha256': harness.sha256_file(source_marker),
            'snapshot_sha256': harness.snapshot_digest(source_snapshot),
            'note_sha256': hashlib.sha256(source_note).hexdigest(),
        }
        receipt['source_before'] = source_before

        # The earlier complete public backup shares the output parent with the
        # interrupted target and is retained byte-for-byte through retry.
        harness.run_cli(binary, root, source,
                        ['backup', '--destination', str(previous_backup)],
                        'backup-complete', receipt['commands'])
        if not previous_backup.is_file() or previous_backup.stat().st_size > MAX_BACKUP:
            raise Stop('earlier public backup is absent or exceeds 128 MiB')
        previous_hash = harness.sha256_file(previous_backup)
        previous_snapshot, previous_standalone = readonly_standalone_snapshot(
            harness, previous_backup)
        if previous_snapshot != source_snapshot:
            raise Stop('earlier public backup differs from source logical rows/schema')
        output_baseline = {
            'complete_backup_sha256': previous_hash,
            'complete_backup_bytes': previous_backup.stat().st_size,
            'complete_backup_standalone': previous_standalone,
            'sentinel_sha256': harness.sha256_file(sentinel),
            'unrelated_tree': harness.tree_snapshot(unrelated),
        }
        if output_baseline['sentinel_sha256'] != receipt['fixture']['sibling_sentinel_sha256']:
            raise Stop('sibling sentinel changed before interruption')
        if output_baseline['unrelated_tree'] != receipt['fixture']['unrelated_tree']:
            raise Stop('unrelated directory changed before interruption')
        receipt['output_baseline'] = output_baseline
        require_absent(destination, 'interrupted backup target')
        if backup_stages(output_parent):
            raise Stop('fresh output parent unexpectedly contains backup stages')
        if harness.tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB before interrupted backup')

        argv = [str(binary), '--json', '--data-dir', str(source),
                'backup', '--destination', str(destination)]
        interrupt_attempt = {
            'label': 'backup-interrupt-once', 'argv': argv[1:],
            'observation_seconds': OBSERVE_SECONDS, 'status': 'starting',
        }
        receipt['commands'].append(interrupt_attempt)
        child_started = time.monotonic()
        try:
            child = subprocess.Popen(argv, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     env=harness.safe_env(root))
            drain_stats, drain_paths, drain_threads = harness.start_drains(
                child, root / 'backup-interrupt.capture')
        except BaseException as error:
            interrupt_attempt.update(status='launch_or_capture_start_error',
                                     error_type=type(error).__name__)
            raise

        observed = None
        deadline = child_started + OBSERVE_SECONDS
        completed_early = False
        while time.monotonic() < deadline:
            status = child.poll()
            if status is not None:
                interrupt_attempt.update(status='finished_before_partial_observation',
                                         exit=status)
                completed_early = True
                break
            stages = backup_stages(output_parent)
            qualifying = [stage for stage in stages
                          if MIN_COPY_BYTES < stage['copy_bytes'] < previous_backup.stat().st_size]
            if len(qualifying) == 1:
                observed = qualifying[0]
                break
            if len(qualifying) > 1:
                interrupt_attempt['status'] = 'multiple_eligible_stages'
                raise Stop('multiple eligible backup stages prevent attribution; no retry will run')
            time.sleep(0.01)
        if observed is None:
            interrupt_attempt['stage_sizes_at_observation_end'] = backup_stages(output_parent)
            if not completed_early:
                interrupt_attempt['status'] = 'observation_window_elapsed_no_eligible_stage'
            raise Stop('no eligible partial backup stage was observed; no retry will be attempted')

        interrupt_attempt['observed_stage'] = observed
        harness.signal_owned_child_once(child, child_state)
        interrupt_attempt['signal_attempted'] = child_state['signal_attempted']
        interrupt_attempt['signal'] = signal.SIGKILL
        harness.wait_owned_child(child, child_state, 10)
        interrupt_attempt['exit'] = child.returncode
        interrupt_attempt['reaped'] = child_state['reaped']
        interrupt_attempt['seconds'] = round(time.monotonic() - child_started, 3)
        post_stages = backup_stages(output_parent)
        interrupt_attempt['post_kill_stages'] = post_stages
        exact_stage = next((stage for stage in post_stages
                            if stage['path'] == observed['path']), None)
        interrupt_attempt['post_kill_observed_stage'] = exact_stage
        if child.returncode != -signal.SIGKILL:
            interrupt_attempt['status'] = 'child_not_signal_terminated'
            raise Stop('owned backup child did not report SIGKILL termination')
        require_absent(destination, 'interrupted backup target after SIGKILL')
        if (exact_stage is None or not
                (MIN_COPY_BYTES < exact_stage['copy_bytes'] < previous_backup.stat().st_size)):
            interrupt_attempt['status'] = 'post_kill_partial_not_proven'
            raise Stop('same observed backup stage was not retained as a partial copy after kill')
        interrupt_attempt['status'] = 'killed_partial_stage_qualified'

        output_ok = harness.finish_drains(drain_stats, drain_threads)
        drains_finished = all(not thread.is_alive() for thread in drain_threads)
        interrupt_attempt['output'] = drain_stats
        interrupt_attempt['output_prefix_paths'] = {k: str(v) for k, v in drain_paths.items()}
        interrupt_attempt['output_complete'] = output_ok
        if not output_ok:
            interrupt_attempt['status'] = 'output_capture_failed'
            raise Stop('interrupted backup stdout/stderr capture did not complete')

        # Exactly one same-target public retry, gated on the retained post-kill
        # partial stage, SIGKILL status, absent final path, and complete drains.
        harness.run_cli(binary, root, source,
                        ['backup', '--destination', str(destination)],
                        'backup-retry', receipt['commands'])
        remaining = backup_stages(output_parent)
        receipt['remaining_backup_stages'] = remaining
        if remaining:
            raise Stop('public backup retry left an abandoned backup stage')
        if not destination.is_file() or destination.is_symlink():
            raise Stop('public backup retry did not publish a regular final backup')
        if destination.stat().st_size > MAX_BACKUP:
            raise Stop('retried backup exceeds 128 MiB')
        retried_snapshot, retried_standalone = readonly_standalone_snapshot(
            harness, destination)
        retried_note = harness.note_bytes(retried_snapshot)
        if retried_snapshot != source_snapshot:
            raise Stop('retried backup logical schema or rows differ from source')
        if retried_note != body:
            raise Stop('retried backup Note UTF-8 bytes differ from exact source bytes')

        source_after = harness.readonly_snapshot(source_db)
        source_storage_after = sqlite_storage_snapshot(source_db)
        if (source_after != source_snapshot
                or source_storage_after['database'] != source_before['database_file']
                or harness.sha256_file(source_marker) != source_before['marker_sha256']):
            raise Stop('source database, logical rows, or workspace marker changed')
        if harness.sha256_file(previous_backup) != previous_hash:
            raise Stop('earlier complete public backup bytes changed')
        previous_after, previous_standalone_after = readonly_standalone_snapshot(
            harness, previous_backup)
        if previous_after != source_snapshot:
            raise Stop('earlier complete public backup logical rows/schema changed')
        if harness.sha256_file(sentinel) != output_baseline['sentinel_sha256']:
            raise Stop('sibling sentinel changed')
        if harness.tree_snapshot(unrelated) != output_baseline['unrelated_tree']:
            raise Stop('unrelated ordinary directory changed')
        if harness.tree_bytes(root) > MAX_ROOT:
            raise Stop('task root exceeded 512 MiB')
        receipt['retry_backup'] = {
            'path': str(destination), 'bytes': destination.stat().st_size,
            'sha256': harness.sha256_file(destination),
            'snapshot_sha256': harness.snapshot_digest(retried_snapshot),
            'note_sha256': hashlib.sha256(retried_note).hexdigest(),
            'standalone': retried_standalone,
        }
        receipt['source_after'] = {
            'database_file': source_storage_after['database'],
            'sidecars_diagnostic': {key: value for key, value in source_storage_after.items()
                                    if key != 'database'},
            'marker_sha256': harness.sha256_file(source_marker),
            'snapshot_sha256': harness.snapshot_digest(source_after),
        }
        receipt['earlier_backup_standalone_after_retry'] = previous_standalone_after
        receipt['earlier_backup_unchanged'] = True
        receipt['sibling_sentinel_unchanged'] = True
        receipt['unrelated_directory_unchanged'] = True
        receipt['outcome'] = QUALIFIED

    except BaseException as error:
        receipt['outcome'] = 'failed_or_inconclusive'
        receipt['reason_type'] = type(error).__name__
        receipt['reason'] = str(error)[:400]
    finally:
        if child is not None:
            if child.poll() is None and not child_state['signal_attempted']:
                try:
                    harness.signal_owned_child_once(child, child_state)
                    if interrupt_attempt is not None:
                        interrupt_attempt['cleanup_signal_attempted'] = child_state['signal_attempted']
                except BaseException as error:
                    receipt['cleanup_signal_error_type'] = type(error).__name__
            try:
                if not child_state['reaped']:
                    harness.wait_owned_child(child, child_state, 10)
            except BaseException as error:
                receipt['cleanup_reap_error_type'] = type(error).__name__
            if interrupt_attempt is not None:
                interrupt_attempt['exit'] = child.returncode
                interrupt_attempt['reaped'] = child_state['reaped']
            if not child_state['reaped']:
                receipt['outcome'] = 'failed_owned_child_not_reaped'
        if drain_threads and not drains_finished:
            try:
                output_ok = harness.finish_drains(drain_stats, drain_threads)
                if interrupt_attempt is not None:
                    interrupt_attempt['output'] = drain_stats
                    interrupt_attempt['output_prefix_paths'] = {
                        k: str(v) for k, v in drain_paths.items()
                    }
                    interrupt_attempt['output_complete'] = output_ok
                if not output_ok:
                    receipt['outcome'] = 'failed_output_capture'
            except BaseException as error:
                receipt['cleanup_output_error_type'] = type(error).__name__
                receipt['outcome'] = 'failed_output_capture'
        try:
            if body_path.exists():
                body_path.unlink()
        except BaseException as error:
            receipt['fixture_input_cleanup_error_type'] = type(error).__name__
            receipt['outcome'] = 'failed_fixture_input_cleanup'
        try:
            receipt['task_root_bytes'] = harness.tree_bytes(root)
            temporary_receipt = receipt_path.with_suffix('.json.tmp')
            temporary_receipt.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + '\n',
                                         encoding='utf-8')
            os.replace(temporary_receipt, receipt_path)
        except BaseException as error:
            evidence_error = type(error).__name__
            receipt['outcome'] = 'failed_evidence_persistence'

    print(json.dumps({'receipt': str(receipt_path), 'outcome': receipt['outcome'],
                      'task_root_bytes': receipt.get('task_root_bytes'),
                      'receipt_written': evidence_error is None}, ensure_ascii=False))
    if evidence_error is not None:
        return 2
    return 0 if receipt['outcome'] == QUALIFIED else 2


if __name__ == '__main__':
    try:
        sys.exit(main())
    except Stop as error:
        print(f'preflight_stop={str(error)[:300]}', file=sys.stderr)
        sys.exit(2)
