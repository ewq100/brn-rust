# Agent-facing CLI foundation — evidence

Date: 2026-09-29. Tested commit: `7a37069` on branch `feature/agent-cli-foundation` (base `main` @ `18f3891`), plus the final documentation commit. Environment: macOS (Apple Silicon), rust 1.98.1 pinned toolchain, offline Cargo with the repository lockfile. Default feature set (no `native-retrieval`, no `native-ui`).

## Commands and results

Run from the repository root with `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`:

| Command | Result |
| --- | --- |
| `cargo +1.98.1 fmt --all -- --check` | Passed |
| `cargo +1.98.1 build --workspace --locked --offline` | Passed |
| `cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed (no warnings) |
| `cargo +1.98.1 test --workspace --locked --offline` | Passed; `brn` crate: 4 unit + 41 subprocess tests (cli_basic 8, cli_core 10, cli_review 8, cli_ask 10, cli_ownership 1); all other crate suites green |
| `bash scripts/verify-end-to-end.sh` | Passed (brn-flow behavior and JSON contract unchanged) |
| `git diff --check` | Clean |

`cargo test -p brn` was additionally looped 20× after the store lock fix with zero flakes (previously the suite flaked ~8% under parallel execution; see below).

## Observations

- The actual `brn` binary is exercised as a subprocess throughout (`CARGO_BIN_EXE_brn`); assertions cover exit codes, single-object JSON envelopes, exact UTF-8/CRLF content preservation, no implicit approval/index-building, no keyword fallback for semantic/hybrid, operation resubmission and conflicts, ownership contention, SIGINT (exit 130) and deadline (exit 124) with deterministic `submitted.log` synchronization, and provider-child reaping via `pgrep`.
- A real cross-process race was found and fixed during integration: a concurrent `fork+exec` in the test process briefly duplicates the workspace flock descriptor into the child until `exec` closes it, so a lock released moments before could transiently report busy. `Store::open` now retries `try_lock` within a bounded 1 s grace before reporting `WORKSPACE_BUSY`; exclusivity is unchanged (no force, no bypass, same error after the grace). The same window exists in production while `ask` holds the lock and spawns the provider child.
- SIGINT classification is checked against the global cancel flag by construction; a completed provider turn stays a success even if the deadline flag fires in the late race window, and non-completed turn statuses exit 1 with honest messages.
- Independent review (fresh reviewer, not an implementer) verified dependency direction, sentinel-prefix error classification, safety invariants and brn-flow unchanged; its findings (stale help text, `index build` positional acceptance, ask-without-`--codex` classification, swallowed SIGINT on read commands, EPIPE panic, weak assertions) were fixed and re-verified in the final commit.

## Limitations

- Provider behavior is verified exclusively with deterministic fake-provider fixtures; no live Codex access, credentials or accounts were used or verified.
- Native retrieval compilation is feature-forwarded (`native-retrieval`) but not exercised; `status` reports `native_retrieval: false` in default builds. No model downloads were performed.
- The store lock retry adds up to 1 s latency before genuine `WORKSPACE_BUSY` failures.
- SIGINT during index build shares the tested classification path with `ask`, but has no dedicated subprocess test (no deterministic interruption seam for the fast keyword builder).
- No control of an already-running desktop instance; exclusive workspace ownership means CLI and desktop cannot operate the same data directory simultaneously.

## Handoff

Branch `feature/agent-cli-foundation`, commits `399e887` (foundation), `3720b23` (ingestion/retrieval + review surfaces), `7a37069` (ask/conversations), plus the documentation commit. Not pushed, not merged. Implementation: GLM 5.3 Flash subagents (foundation, C1+C2 in parallel, provider/session, review-fix package); integration, cross-cutting lock fix, reviewer fixes on outcome classification, documentation and acceptance: GLM 5.3.
