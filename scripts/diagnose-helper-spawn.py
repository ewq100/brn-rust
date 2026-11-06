"""Task-only Ubuntu investigation; never part of the updater PR's workflow."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

REPO = Path.cwd()
OUT = REPO / 'diagnosis-results'
OUT.mkdir()
TARGET = 'successful_process_with_wrong_response_identity_is_rejected_and_limits_are_enforced'
BASE = '3b72813df7cfcb857cdef55fff2c3f17f3fe86f6'
PR = '2c074469ae4f910245f182823d6971201d651158'
SUMMARY = []
START = time.monotonic()
# A real fork race retains the exact descriptor inherited from the test process.
# The child uses only libc sleep/_exit; it executes no program or shared Rust code.
STRESS = '''
    let fork_stop = std::sync::Arc::new(AtomicBool::new(false));
    let fork_workers: Vec<_> = (0..4).map(|_| {
        let stop = fork_stop.clone();
        std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Acquire) {
                let pid = unsafe { libc::fork() };
                if pid == 0 { unsafe { libc::usleep(20_000); libc::_exit(0) } }
                if pid > 0 { unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0); } }
            }
        })
    }).collect();
'''
# A scope guard joins fork workers even when the unmodified assertion panics.
GUARD = '''
    struct ForkGuard(std::sync::Arc<AtomicBool>, Vec<std::thread::JoinHandle<()>>);
    impl Drop for ForkGuard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
            for worker in self.1.drain(..) { worker.join().unwrap(); }
        }
    }
    let _fork_guard = ForkGuard(fork_stop, fork_workers);
'''
FIXTURE = '''#!/bin/sh
# Stable executable; per-case response is ordinary data, never executed.
/bin/cat >/dev/null
/bin/cat ../../synthetic-response.json
'''
for label, sha, repair in [('baseline', BASE, False), ('submitted-pr', PR, False), ('static-fixture', PR, True)]:
    root = Path(os.environ['RUNNER_TEMP']) / ('intake-' + label)
    subprocess.run(['git', 'worktree', 'add', '--detach', str(root), sha], check=True)
    test = root / 'crates/brn-threads-intake/tests/document_modes.rs'
    proc = root / 'crates/brn-threads-intake/src/process.rs'
    original = test.read_text()
    source_hash = hashlib.sha256(test.read_bytes()).hexdigest()
    process_hash = hashlib.sha256(proc.read_bytes()).hexdigest()
    # Report only the bounded numeric spawn error; do not print paths/input/stderr.
    text = proc.read_text().replace('command.spawn().map_err(|_| ConversionError::Unavailable)?', 'command.spawn().map_err(|error| { eprintln!("DIAGNOSTIC_SPAWN_ERRNO={:?} KIND={:?}", error.raw_os_error(), error.kind()); ConversionError::Unavailable })?')
    assert 'DIAGNOSTIC_SPAWN_ERRNO' in text
    proc.write_text(text)
    if repair:
        original = original.replace('    use std::os::unix::fs::PermissionsExt;\n', '')
        original = original.replace('    let path = root.path().join("synthetic-helper");', '    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/document-response-helper.sh");')
        start = original.index('        let quoted = json.replace')
        end = original.index('        let actual =', start)
        original = original[:start] + '        std::fs::write(root.path().join("synthetic-response.json"), json).unwrap();\n' + original[end:]
        fixture = test.parent / 'fixtures/document-response-helper.sh'
        fixture.write_text(FIXTURE)
        fixture.chmod(0o755)
    # First compile/run original logic with errno instrumentation only.
    test.write_text(original)
    build_log = OUT / (label + '-build.log')
    with build_log.open('w') as stream:
        built = subprocess.run(['cargo', 'test', '--locked', '-p', 'brn-threads-intake', '--test', 'document_modes', '--no-run', '--message-format=json'], cwd=root, stdout=subprocess.PIPE, stderr=stream, text=True, timeout=300)
    assert built.returncode == 0, build_log
    exe = next(json.loads(line)['executable'] for line in built.stdout.splitlines() if json.loads(line).get('reason') == 'compiler-artifact' and json.loads(line).get('executable'))
    passes = 0
    plain_log = OUT / (label + '-plain.log')
    with plain_log.open('w') as stream:
        for attempt in range(40):
            result = subprocess.run([exe, '--exact', TARGET, '--nocapture'], cwd=root, stdout=stream, stderr=subprocess.STDOUT, timeout=10)
            if result.returncode: break
            passes += 1
    # Same exact assertion/identity cases plus a controlled concurrent-fork load.
    marker = '    for kind in 0..5 {'
    assert marker in original
    stressed = original.replace(marker, '    if std::env::var_os("BRN_DIAGNOSTIC_FORK").is_some() {\n' + STRESS + GUARD + '\n    for _repetition in 0..400 {\n' + marker, 1)
    close = '\n    converter.limits.max_input_bytes = 1;'
    assert close in stressed
    stressed = stressed.replace(close, '\n    }\n    } else {\n' + original[original.index(marker):original.index(close)] + '\n    }\n' + close, 1)
    test.write_text(stressed)
    with (OUT / (label + '-stress.log')).open('w') as stream:
        stress = subprocess.run(['cargo', 'test', '--locked', '--offline', '-p', 'brn-threads-intake', '--test', 'document_modes', TARGET, '--', '--exact', '--nocapture'], cwd=root, env=dict(os.environ, BRN_DIAGNOSTIC_FORK='1'), stdout=stream, stderr=subprocess.STDOUT, timeout=240)
    entry = dict(label=label, commit=sha, original_test_sha256=source_hash, original_process_sha256=process_hash, unmodified_cases_plain_passes=passes, plain_attempt_limit=40, plain_exit=result.returncode, fork_stress_exit=stress.returncode, elapsed_seconds=time.monotonic()-START)
    SUMMARY.append(entry)
    (OUT / 'summary.json').write_text(json.dumps(SUMMARY, indent=2)+'\n')
    print(json.dumps(entry), flush=True)
