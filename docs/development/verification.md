# Verification guide

Run from the repository root. Use the pinned toolchain, lockfiles, disposable explicit data directories and synthetic fixtures. Read scripts before running them. Record actual environment and results; do not copy historical pass counts as new evidence.

## Select checks by change

| Changed area | Relevant checks |
| --- | --- |
| Documentation only | `git diff --check`; check local Markdown file/fragment links and moved-path references; compare documented commands and features with manifests/scripts |
| Integrated workflow or cross-crate behavior | `bash scripts/verify-end-to-end.sh` (workspace format/build/Clippy/tests plus provider-free import/search fixture) |
| Storage, migrations or recovery | `bash scripts/verify-storage.sh`; inspect relevant process-crash tests in `crates/brn-store/tests` |
| Drafts or comments | `cargo test -p brn-store -p brn-workflow --locked`; then integrated checks; add native checks if interaction changes |
| Native UI | `bash scripts/verify-desktop-shell.sh --native`; manually exercise changed flows on the unlocked target Mac with fresh disposable data |
| Provider adapter | `cargo test -p brn-provider --locked` (fake sidecar); `bash scripts/verify-trial.sh` if the standalone provider trial changes |
| Keyword retrieval | `cargo test -p brn-retrieval --locked`; integrated workflow checks |
| Native retrieval | `cargo test -p brn-retrieval --features native --locked`; build/test the affected consumer with `native-retrieval`; read fixture/resource conditions in the native smoke tests |
| Editor experiment | `bash scripts/verify-editor-trial.sh` (includes native build) |
| Retrieval experiment | `bash scripts/verify-retrieval-trial.sh`; add `--native` for native checks; `scripts/verify-retrieval-state.sh` requires a completed synthetic state directory |
| Local macOS launcher | `bash scripts/test-make-macos-app.sh`; observe launch/relaunch if behavior changed |
| Shell scripts | `bash -n scripts/*.sh` plus the affected behavioral script |

Do not repeat entire workspace suites through several scripts when their shared checks already passed, unless a new change or unresolved failure warrants it. Standalone experiment tests are not included by `cargo test --workspace`.

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
