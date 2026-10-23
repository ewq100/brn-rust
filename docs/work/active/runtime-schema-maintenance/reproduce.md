# Reproduce maintenance checks

Start with this candidate stacked on `125f8ef0`, Rust 1.98.1, fetched locked
crates, and the existing Linux cloud setup. Use synthetic fixtures only. The
saved environment keeps toolchains and targets outside the repository; source
`/workspace/brn-environment/activate.sh`, then set `CARGO_BUILD_JOBS=1`.
Use the existing explicit fixture parent outside Git for credential-path tests:
`TMPDIR=/var/tmp/brn-runtime-schema-20261023`. The sandbox injects a protected `.git` at writable roots, making credential
fixtures unsafe even in a new empty directory. The supported `require_escalated`
execution request was approved for the synthetic checks; no app validation or
system security configuration changed. Do not hide/remove Git metadata or weaken
path validation. The old fixture parent remains untouched.

Build both helpers using the supported script and existing source-associated
target caches:

```sh
bash scripts/build-document-helpers.sh --offline --jobs 1 \
  --target-root /workspace/brn-environment/document-helpers
export BRN_XBERG_HELPER=/workspace/brn-environment/document-helpers/artifacts/brn-xberg-helper
export BRN_XBERG_HELPER_BRN=/workspace/brn-environment/document-helpers/artifacts/brn-xberg-helper-brn
cp "$BRN_XBERG_HELPER" "$BRN_XBERG_HELPER_BRN" "$CARGO_TARGET_DIR/debug/"
cargo fmt --all --check
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --no-fail-fast --locked --offline
cargo test -p brn-ai --features capability-spike --all-targets --locked --offline
cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings
# macOS-only module: Linux selects zero tests; record as NOT RUN
cargo test -p brn --features scripted-document-tests --bin brn document_route_tests --locked --offline --jobs 1
cargo clippy -p brn --features scripted-document-tests --all-targets --locked --offline --jobs 1 -- -D warnings
cargo test -p brn-desktop --features native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo build -p brn-desktop --features native-ui --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline --jobs 1 -- -D warnings
python3 -m unittest discover -s scripts/tests
bash scripts/verify-end-to-end.sh --fixtures-only
python3 scripts/check-markdown-links.py
python3 scripts/render-work-overview.py --check
git diff --check
```

For each helper, run `cargo test` and `cargo clippy` with `--all-targets --locked
--offline --jobs 1` (Clippy additionally `-- -D warnings`). Stock manifest:
`tools/xberg-intake-helper/Cargo.toml`; adapted generated manifest:
`target/xberg-adapted/workspace/Cargo.toml`. Set `CARGO_TARGET_DIR` to the matching
`/workspace/brn-environment/document-helpers/stock` or `adapted` cache. The helper
generator validates pinned source and the candidate adapted lock before use.

The storage CLI/headless checks also ran separately: both binaries' `--help`,
expected failures for each binary's `--unknown` and `brn` without arguments,
then `brn-desktop --headless-check startup --data-dir` in a fresh `mktemp`
synthetic directory. See `headless-startup.log`.

This container's PID 1 does not reliably reap adopted children. Commands in the
receipt ran under the existing
`python3 /workspace/brn-environment/run-with-reaping.py COMMAND ...` subreaper,
which preserves exit status and test assertions. The exact serial runner and
raw output are retained under `/workspace/brn-environment`; this is a local
execution aid, not a new app runtime or a permission workaround.

The aggregate steps above are explicit equivalents using existing helper caches;
`verify-storage.sh` itself is not claimed as executed. Feature-specific provider and native tests add coverage beyond the default
workspace. The scripted-document module is macOS-only and selected zero Linux
tests; its successful command is not a passing acceptance result. Linux document
CLI and all-eight-kind/both-engine checks are in the workspace/helper suites. Linux
checks do not establish Mac menu/close/accessibility acceptance, and scripted
provider fixtures do not establish live-model quality.

## Actual native window smoke

After the production native build and Clippy, source
`/workspace/brn-environment/a11y/activate.sh`, set `TMPDIR=/tmp`, then run:

```sh
python3 /workspace/brn-environment/run-with-reaping.py \
  python3 /workspace/brn-environment/maintenance-native-smoke.py
```

The saved harness uses the established task-local Xvfb display `:98`, software
Vulkan, AT-SPI bus, xdotool and clipboard. It creates the fresh synthetic directory
`/tmp/brn-runtime-schema-native-20261023`, pauses AI, edits exact Unicode text,
checks recovery and saved version 2, exercises accessible/keyboard/pointer menu
paths, quits, reopens and compares clipboard bytes and full persisted snapshots.
It closes the app normally. Choose a new unused synthetic root for another run;
do not erase prior evidence. The harness and driver remain in this saved cloud
workspace; raw screenshots/AX/state logs are under
`/workspace/brn-environment/logs/runtime-schema-maintenance/native`.
No native file-picker request or live provider call is part of this smoke.
