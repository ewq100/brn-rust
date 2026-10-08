# Verification guide

Run from the repository root. Use the pinned toolchain, lockfiles, disposable explicit data directories and synthetic fixtures. Read scripts before running them. Record actual environment and results; do not copy historical pass counts as new evidence.
[Development tooling](tooling.md) provides read-only preflight, opt-in gate evidence,
exact-attempt CI summaries and the current Markdown link gate.

## Select checks by change

| Changed area | Relevant checks |
| --- | --- |
| Documentation only | `git diff --check`; `python3 scripts/check-markdown-links.py` for local Markdown file/fragment links and moved-path references; compare documented commands and features with manifests/scripts |
| Integrated workflow or cross-crate behavior | `bash scripts/verify-end-to-end.sh` (retirement check, workspace format/build/Clippy/tests, current-vault Save/recovery/read/search fixtures); `--fixtures-only` after a completed shared gate |
| Storage, migrations or recovery | `bash scripts/verify-storage.sh`; inspect relevant process-crash tests in `crates/brn-store/tests` |
| Drafts or comments | `cargo test -p brn-store -p brn-workflow --locked`; then integrated checks; add native checks if interaction changes |
| Native UI | `bash scripts/verify-desktop-shell.sh --native`; manually exercise changed flows on the unlocked target Mac with fresh disposable data; retain safe captures in the [UI screenshot index](../ui/screenshots/README.md) |
| Rig AI/auth adapter | `cargo test -p brn-ai --lib --locked --offline` (real Rig routes with synthetic transports); standalone `verify-trial.sh` is historical, never a product gate |
| Search index or keyword retrieval | `cargo test -p brn-retrieval --locked`; integrated workflow checks |
| Native retrieval | `cargo test -p brn-retrieval --features native --locked` (set `BRN_NATIVE_MODEL_DIR` to run the local model test; otherwise it is skipped); `cargo check -p brn-desktop --features native-ui,native-retrieval --locked` |
| Editor experiment | `bash scripts/verify-editor-trial.sh` (includes native build) |
| Retrieval experiment | `bash scripts/verify-retrieval-trial.sh`; add `--native` for native checks; `scripts/verify-retrieval-state.sh` requires a completed synthetic state directory |
| Local macOS launcher | `bash scripts/test-make-macos-app.sh`; observe launch/relaunch if behavior changed |
| Shell scripts | `for script in scripts/*.sh; do bash -n "$script"; done` plus the affected behavioral script |

Do not repeat entire workspace suites through several scripts when their shared checks already passed, unless a new change or unresolved failure warrants it. Standalone experiment tests are not included by `cargo test --workspace`.

Give each checkout its own Cargo target and keep Cargo calls sequential within
that target. A baseline probe must not reuse another checkout's package artifacts:
Cargo's relative dependency paths and timestamps can retain stale local crates
across different trees. If a task-owned cache was transferred, clean its BRN
packages before verifying another tree; preserve dependency caches and record the
failed attempt separately. A zero-test filtered run is not behavioral evidence.

Inbox Action worker-test waits use one absolute ten-second allowance per ordinary
operation. A validated captured approval group executes members sequentially and
receives that allowance per member (three members: thirty seconds); unrelated
events never restart its deadline. Invalid groups keep ten seconds. This is a
test-harness budget, not a product approval deadline or permission to skip slow
tests. Test-only group timings report UUIDs/counts/durations without evidence
payloads, distinguishing a slow completed group from a stuck or failed approval.

Credential fixtures require an explicit current-user-owned parent outside Git;
create a private synthetic parent such as `/private/tmp/brn-fixtures`, then set
`TMPDIR` to that existing canonical directory. Do not use original data or
credentials. `verify-end-to-end.sh` requires that existing absolute parent,
creates one exclusive UUID-owned fixture and removes only its own entries.
No account, model asset, inference or graphical interaction is performed.

The retirement check (`--retirement-only`) scans root/crate manifests,
production Rust and product scripts. Integration tests and trailing private
test modules may explicitly reject old flags; historical docs and standalone
trials are not shipped production configuration. Tokens are not disguised.

## Optional native offline qualification

After targeted fixes, run the four baseline commands below **once**, using
`--locked --offline`, followed by:

```sh
cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn --features native-retrieval --locked --offline
bash scripts/verify-end-to-end.sh --fixtures-only
bash scripts/test-make-macos-app.sh
git diff --check
```

Native workflow library tests include the private worker/loader seams as well
as model contracts; integration `models` runs unfiltered. Synthetic downloads
and vectors are not real ONNX or asset qualification. An unset local-model
environment can self-skip tests reported as passed; record those limitations.
Native builds/state tests do not establish GUI usability; upstream
`block v0.1.6`'s future-compiler warning is a known separate limitation.
`native-test-support` enables the pinned toolkit's headless widget context for
exact title/body checks. Normal native application builds omit this test feature.

## Baseline Rust checks

```sh
cargo fmt --all -- --check
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The intake helper must be built beside the CLI/desktop/test binaries before workflow
conversion tests. It activates native restrictions before reading copied input;
missing helper installation is a failed gate, never a skipped conversion check.

Default workspace checks exclude optional native feature paths. Native builds and interactions require separate evidence. The scripts generally use offline Cargo; see [setup](setup.md).

## GitHub-hosted CI

[BRN CI](../../.github/workflows/ci.yml) runs default workspace builds/tests and
CLI help checks on Ubuntu 24.04 (x64), Windows Server 2025 (x64), and macOS 15
(Apple Silicon). Linux also checks formatting and Clippy on every PR; main and
manual runs check Clippy on all three systems. The macOS job runs the current
synthetic read/search, Save/recovery/Copy and marker-refusal end-to-end fixtures
with an exclusive physical parent under `RUNNER_TEMP`. Save requires macOS
filesystem coordination; Ubuntu retains the shared workspace checks and CLI help.
All Unix test jobs use an explicit physical `TMPDIR` under `RUNNER_TEMP`, so
credential-safety checks do not encounter symlinked macOS system temp paths.
After a successful default build, tests, CLI help and fixtures can still run if
an earlier independent check fails; the failed check keeps the job red.

Native UI and native retrieval have separate jobs. PRs run both on macOS;
main and manual runs probe both on all three systems. The native UI job also
lints and tests actual macOS widgets with `native-test-support`; shipping builds
remain separately qualified without that feature. The same job starts
and shuts down the real AppWorker twice against one fresh data directory, checking
that `brn.sqlite` exists and the retired `brn.sqlite3` does not. Each lane installs the
pinned toolchain, fetches locked dependencies before offline Cargo checks, and
uses a cache separated by OS, architecture, compiler, lockfiles and features.
Outdated runs are cancelled. Jobs use read-only repository permissions, do not
receive provider credentials, and never request model assets or a live login.
Native retrieval may download its build-time ONNX Runtime dependency; model
tests use synthetic fixtures, and the real-model test is excluded.

These are platform qualification checks, not a claim that BRN is already
portable. Windows currently has unconditional Unix filesystem/credential APIs;
managed-note coordination still requires macOS. A failing platform remains
visible: jobs do not use `continue-on-error`. Fixing those product boundaries
needs a separate authorized portability slice. Native build/state checks do
not open the GUI or establish usability, packaging or Windows 11 qualification.
Standalone experiments remain outside this workflow.

## Evidence standards

Record date, commit (and dirty changes), platform/toolchain, feature flags, exact commands, result and limitations. Distinguish passed, failed, skipped and blocked checks; fixture-gated tests that did not exercise native resources are not live resource verification. Identify fresh verification separately from previous evidence.

Live provider checks or model downloads require task authorization. Manual native evidence should name the observed scenario, disposable data, restart behavior and unresolved user/IME/accessibility qualification. A build or headless test does not establish those results.
