# Reproducible check commands

Run in the saved cloud repository after environment activation. All data paths
below are disposable synthetic fixtures; choose fresh paths for another run.

```sh
source /workspace/brn-environment/activate.sh
export TMPDIR=/tmp
cargo test -p brn-desktop --features native-test-support --bin brn-desktop \
  --locked --offline --jobs 1
cargo build -p brn-desktop --features native-ui --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-test-support --all-targets \
  --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
python3 scripts/check-markdown-links.py
python3 scripts/render-work-overview.py --check
python3 scripts/check-provider-retirement.py
python3 scripts/check-intake-retirement.py
```

Initial 0.7.3 logs: [desktop tests](evidence/checks/desktop-tests-final.log),
[native build](evidence/checks/native-build.log),
[strict Clippy](evidence/checks/clippy.log). The ignored test remains ignored;
this batch does not change its status. No Windows or macOS execution, full
workspace aggregate, live model qualification or remote CI run was performed
for this local branch.

Separate checked-service import/backup/restore example (not native chooser):

```sh
brn --json --data-dir /tmp/FRESH-synthetic/workspace init --purpose test
brn --json --data-dir /tmp/FRESH-synthetic/workspace pause
brn --json --data-dir /tmp/FRESH-synthetic/workspace document-conversion --converter xberg --ai off
brn --json --data-dir /tmp/FRESH-synthetic/workspace intake \
  --source /tmp/synthetic.md --intent full
brn --json --data-dir /tmp/FRESH-synthetic/workspace backup \
  --destination /tmp/FRESH-synthetic/backup.sqlite3
brn --json --data-dir /tmp/FRESH-restored restore \
  --source /tmp/FRESH-synthetic/backup.sqlite3
```

Use `$CARGO_TARGET_DIR/debug/brn` if the CLI is not on PATH. Inspect complete
records, immutable revisions, receipts/history, recovery and workspace purpose;
restore deliberately invalidates edit-session bindings and creates owner purpose.
Retain backup hashes before/after and verify saving restored data cannot alter
original data. Do not print or publish session tokens.

For native reading, source the existing AT-SPI activation and launch
`$CARGO_TARGET_DIR/debug/brn-desktop --data-dir /tmp/FRESH-restored` on the task
X11 display. Use actual AT-SPI actions/current bounds, settled screenshots and
checked persisted-state observations. [Connections](connections.md) documents
continuous native recording; [accessibility](accessibility.md) lists observed
menu/Project journeys. File chooser actions remain denied as described in
[file workflows](file-workflows.md); do not retry that denial without new authority.

Tracked logs trim terminal trailing whitespace only; [normalization hashes](evidence/log-normalization.json) distinguish raw from tracked bytes. The Connections receipt retains original recording-log hashes.

For the current 0.8.0 results, see [upgrade qualification and logs](gpui08.md).
Also run `cargo test -p brn-writing-ui --features native-test-support --locked --offline --jobs 1`;
the current strict Clippy run selected both `-p brn-desktop -p brn-writing-ui`.
Toolkit tests are separate from the actual native observations in that record.
