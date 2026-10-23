# Reproduce Rig 0.45 qualification

This local branch stacks on `8f7143f34191ad53e4c7e74f7c061e8dce5e6e35`.
Use Rust 1.98.1 and locked dependencies. No real account calls are authorized.
Source `/workspace/brn-environment/activate.sh` in the saved cloud environment;
set `CARGO_BUILD_JOBS=1`. Existing stock/adapted helpers and their unchanged
maintenance lock graphs are retained. All builds share one serial slot.

Compilation uses `TMPDIR=/tmp`. Synthetic credential fixtures use the established
`TMPDIR=/var/tmp/brn-runtime-schema-20261023` through supported approved execution:
the filesystem sandbox injects protected Git metadata at writable roots, and BRN
correctly refuses credential paths under Git. No app validation or system security
settings are changed. Use `/workspace/brn-environment/run-with-reaping.py` around
test processes because container PID 1 does not reap adopted children reliably.

Commands selected for qualification (results are recorded separately):

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --locked --offline --jobs 1
cargo test -p brn-ai --features capability-spike --all-targets --locked --offline --jobs 1
cargo build --workspace --locked --offline --jobs 1
cargo test --workspace --no-fail-fast --locked --offline --jobs 1
cargo clippy --workspace --all-targets --locked --offline --jobs 1 -- -D warnings
cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline --jobs 1 -- -D warnings
cargo clippy -p brn --features scripted-document-tests --all-targets --locked --offline --jobs 1 -- -D warnings
cargo test -p brn-desktop --features native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo build -p brn-desktop --features native-ui --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline --jobs 1 -- -D warnings
bash scripts/verify-end-to-end.sh --fixtures-only
python3 -m unittest discover -s scripts/tests
python3 scripts/check-markdown-links.py
python3 scripts/render-work-overview.py --check
git diff --check
```

The cloud native harness is `/workspace/brn-environment/rig045-native-smoke.py`.
Source the existing `/workspace/brn-environment/a11y/activate.sh` first. It uses
Xvfb :98, software Vulkan, AT-SPI and clipboard on a disposable copy of the stored
0.44 fixture, whose original CLI binary/hash/snapshot remain under
`/workspace/brn-environment/rig045-baseline`. It compares records, history,
recovery and candidates before edits, then saves Unicode bytes as version 3,
quits/reopens, compares persisted snapshots and unchanged sentinel, and exercises
menu focus/keyboard/pointer behavior. The native picker is excluded due to its
prior access denial. Linux observation cannot satisfy macOS-specific acceptance.

Raw logs, failed attempts and native evidence are retained under
`/workspace/brn-environment/logs/rig045-migration`. A command appearing in this
reproduction list is not itself a pass claim. Live-model quality remains untested.
