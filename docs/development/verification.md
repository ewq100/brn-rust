# Verification guide

Run from the repository root. Use the pinned toolchain, lockfiles, disposable explicit data directories and synthetic fixtures. Read scripts before running them. Record actual environment and results; do not copy historical pass counts as new evidence.

## Select checks by change

| Changed area | Relevant checks |
| --- | --- |
| Documentation only | `git diff --check`; check local Markdown file/fragment links and moved-path references; compare documented commands and features with manifests/scripts |
| Integrated workflow or cross-crate behavior | `bash scripts/verify-end-to-end.sh` (retirement check, workspace format/build/Clippy/tests, current-vault Save/recovery/read/search fixtures); `--fixtures-only` after a completed shared gate |
| Storage, migrations or recovery | `bash scripts/verify-storage.sh`; inspect relevant process-crash tests in `crates/brn-store/tests` |
| Drafts or comments | `cargo test -p brn-store -p brn-workflow --locked`; then integrated checks; add native checks if interaction changes |
| Native UI | `bash scripts/verify-desktop-shell.sh --native`; manually exercise changed flows on the unlocked target Mac with fresh disposable data |
| Rig AI/auth adapter | `cargo test -p brn-ai --lib --locked --offline` (real Rig routes with synthetic transports); standalone `verify-trial.sh` is historical, never a product gate |
| Search index or keyword retrieval | `cargo test -p brn-retrieval --locked`; integrated workflow checks |
| Native retrieval | `cargo test -p brn-retrieval --features native --locked` (set `BRN_NATIVE_MODEL_DIR` to run the local model test; otherwise it is skipped); `cargo check -p brn-desktop --features native-ui,native-retrieval --locked` |
| Editor experiment | `bash scripts/verify-editor-trial.sh` (includes native build) |
| Retrieval experiment | `bash scripts/verify-retrieval-trial.sh`; add `--native` for native checks; `scripts/verify-retrieval-state.sh` requires a completed synthetic state directory |
| Local macOS launcher | `bash scripts/test-make-macos-app.sh`; observe launch/relaunch if behavior changed |
| Shell scripts | `bash -n scripts/*.sh` plus the affected behavioral script |

Do not repeat entire workspace suites through several scripts when their shared checks already passed, unless a new change or unresolved failure warrants it. Standalone experiment tests are not included by `cargo test --workspace`.

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
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
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

## Baseline Rust checks

```sh
cargo fmt --all -- --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Default workspace checks exclude optional native feature paths. Native builds and interactions require separate evidence. The scripts generally use offline Cargo; see [setup](setup.md).

## Evidence standards

Record date, commit (and dirty changes), platform/toolchain, feature flags, exact commands, result and limitations. Distinguish passed, failed, skipped and blocked checks; fixture-gated tests that did not exercise native resources are not live resource verification. Identify fresh verification separately from previous evidence.

Live provider checks or model downloads require task authorization. Manual native evidence should name the observed scenario, disposable data, restart behavior and unresolved user/IME/accessibility qualification. A build or headless test does not establish those results.
