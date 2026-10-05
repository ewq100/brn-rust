#!/usr/bin/env python3
"""Optional evidence for BRN's existing gates; never accepts arbitrary commands."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent.parent
GATES = {
    'verify-end-to-end.sh': {'': 'default', '--fixtures-only': 'default (fixtures only)', '--retirement-only': 'production retirement only'},
    'verify-storage.sh': {'': 'default'},
    'verify-desktop-shell.sh': {'': 'default', '--native': 'native-ui,native-retrieval,native-test-support'},
}


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args])


def identity(root=ROOT):
    status = git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all')
    digest = hashlib.sha256()
    digest.update(status)
    digest.update(git(root, 'diff', '--binary', 'HEAD', '--'))
    # Include untracked contents, names and modes. Ignored build output is excluded.
    for name in git(root, 'ls-files', '--others', '--exclude-standard', '-z').split(b'\0'):
        if not name:
            continue
        path = root / os.fsdecode(name)
        digest.update(name + b'\0' + str(path.lstat().st_mode).encode() + b'\0')
        content = os.fsencode(os.readlink(path)) if path.is_symlink() else path.read_bytes()
        digest.update(hashlib.sha256(content).digest())
    return {
        'checkout': str(root.resolve()),
        'branch': git(root, 'branch', '--show-current').decode().strip(),
        'commit': git(root, 'rev-parse', 'HEAD').decode().strip(),
        'tree': git(root, 'rev-parse', 'HEAD^{tree}').decode().strip(),
        'dirty': bool(status),
        'dirty_snapshot_sha256': digest.hexdigest() if status else None,
    }


def utc():
    return datetime.now(timezone.utc).isoformat(timespec='seconds')


def publish(path, record):
    # Readers see a whole record, even during interruption or an I/O failure.
    fd, temporary = tempfile.mkstemp(prefix='.record-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(record, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def record_gate(gate, args, parent, root=ROOT):
    if gate not in GATES or len(args) > 1 or (args[0] if args else '') not in GATES[gate]:
        raise ValueError('Only the documented BRN product gate invocations can be recorded')
    raw_parent = Path(parent)
    if not raw_parent.is_absolute() or not raw_parent.is_dir() or raw_parent.resolve() != raw_parent:
        raise ValueError('BRN_VERIFY_OUTPUT_DIR must be an existing absolute physical disposable directory')
    if raw_parent == root or root.resolve() in raw_parent.parents:
        raise ValueError('Evidence output must be outside the checkout')
    output = Path(tempfile.mkdtemp(prefix=f'{gate.removesuffix(".sh")}-', dir=raw_parent))
    command = ['bash', str(root / 'scripts' / gate), *args]
    record = {
        'schema': 1, 'identity': identity(root), 'command': command,
        'features': GATES[gate][args[0] if args else ''],
        'start_utc': utc(), 'end_utc': None, 'terminal': 'running', 'exit_code': None,
        'toolchain_pin': (root / 'rust-toolchain.toml').read_text().strip() if (root / 'rust-toolchain.toml').is_file() else None,
        'environment': {'platform': sys.platform, 'target_dir': os.environ.get('CARGO_TARGET_DIR', str(root / 'target')),
                        'tmpdir': os.environ.get('TMPDIR'), 'native_model_configured': bool(os.environ.get('BRN_NATIVE_MODEL_DIR'))},
    }
    publish(output / 'record.json', record)
    print(f'BRN verification evidence: {output}', flush=True)
    interrupted = []
    child = None

    def stop(signum, _frame):
        interrupted.append(signum)
        if child is not None:
            try:
                os.killpg(child.pid, signum)
            except ProcessLookupError:
                pass

    previous = {sig: signal.signal(sig, stop) for sig in (signal.SIGINT, signal.SIGTERM)}
    code = 1
    try:
        env = dict(os.environ, _BRN_EVIDENCE_CHILD='1')
        child = subprocess.Popen(command, cwd=root, env=env, start_new_session=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        if interrupted:
            os.killpg(child.pid, interrupted[0])
        with (output / 'output.log').open('wb') as log:
            while chunk := child.stdout.read1(65536):
                log.write(chunk)
                log.flush()
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
        result = child.wait()
        code = 128 + interrupted[0] if interrupted else (128 - result if result < 0 else result)
        record['terminal'] = 'interrupted' if interrupted or result < 0 or result in (130, 143) else ('passed' if code == 0 else 'failed')
    except BaseException:
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
            child.wait()
        record['terminal'] = 'failed'
        raise
    finally:
        record.update(end_utc=utc(), exit_code=code, end_identity=identity(root))
        record['checkout_changed'] = record['identity'] != record['end_identity']
        publish(output / 'record.json', record)
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('gate', choices=GATES)
    parser.add_argument('gate_args', nargs='*')
    opts = parser.parse_args()
    try:
        return record_gate(opts.gate, opts.gate_args, os.environ.get('BRN_VERIFY_OUTPUT_DIR', ''))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f'Verification evidence failed: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
