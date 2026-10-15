#!/usr/bin/env python3
"""Time cold offline builds; log commands, results and build contention."""
import json, os, pathlib, subprocess, threading, time, shlex
ROOT = pathlib.Path(__file__).resolve().parent
CARGO = pathlib.Path('/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo')
env = os.environ.copy()
env['PATH'] = str(CARGO.parent) + ':' + env['PATH']
env['CARGO_TARGET_DIR'] = '/private/tmp/brn-x1-target'
env['CARGO_BUILD_JOBS'] = '4'
records = []
for name, extra in [('baseline', []), ('xberg-probe', ['--features', 'candidate'])]:
    command = ['/usr/bin/sandbox-exec', '-p', '(version 1) (allow default) (deny network*)', str(CARGO),
        'build', '--manifest-path', str(ROOT / 'Cargo.toml'), '--locked', '--offline', '--release', '--bin', name] + extra
    samples = []; stop = threading.Event()
    def monitor():
        while not stop.is_set():
            ps = subprocess.check_output(['ps', '-axo', 'pid,ppid,etime,command'], text=True)
            samples.extend(line for line in ps.splitlines() if any(word in line for word in ['rustc ', '/cargo ', '/clang ']))
            stop.wait(1)
    monitor_thread = threading.Thread(target=monitor); monitor_thread.start()
    started = time.time(); start = time.monotonic()
    with (ROOT / 'evidence' / f'build-{name}.log').open('w') as log:
        result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT)
    elapsed = time.monotonic() - start
    stop.set(); monitor_thread.join()
    (ROOT / 'evidence' / f'processes-{name}.txt').write_text('\n'.join(samples) + '\n')
    binary = pathlib.Path(env['CARGO_TARGET_DIR']) / 'release' / name
    record = dict(command=shlex.join(command), started_unix=started, seconds=elapsed, returncode=result.returncode,
        bytes=binary.stat().st_size if binary.exists() else None, environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS']})
    records.append(record)
    (ROOT / 'evidence' / 'builds.json').write_text(json.dumps(records, indent=2) + '\n')
    print(json.dumps(record), flush=True)
    if result.returncode: raise SystemExit(result.returncode)
    (ROOT / 'evidence' / f'otool-{name}.txt').write_text(subprocess.check_output(['/usr/bin/otool', '-L', str(binary)], text=True))
