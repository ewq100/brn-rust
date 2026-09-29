# Evidence: PR #2 review fixes

Tested commit: `14696b3` on `feature/agent-cli-foundation` (working tree clean; base `df47114` = PR #2 reviewed head; remote branch still at `df47114` at check time — commits not pushed). Environment: macOS arm64 (target Mac), rust 1.98.1 pinned toolchain, offline Cargo, no native features, fake python providers and synthetic CODEX_HOME only. No credentials, live Codex calls, or model downloads.

Baseline before changes (measured fresh at `df47114`): fmt clean, build clean, clippy `-D warnings` clean, `cargo test --workspace --locked` **157 passed / 0 failed**.

## Finding disposition

| Finding | Status | Fix commit | Regression evidence |
| --- | --- | --- | --- |
| R1 SIGPIPE bypasses provider cleanup | Confirmed; fixed | `cb1c454` | Lead reproduced live at baseline: fake provider closing its stdin read end (`os.close(0)`) after answering `initialize` → `brn ask` died by signal 13 (shell exit 141), no envelope, provider child left alive. After fix (lead, live): exit 1, `WORKFLOW_ERROR` envelope with context, child reaped. Test `broken_provider_pipe_is_recoverable_and_reaps_child` red on baseline (signal death), green after. |
| R2 late SIGINT misreports committed operations | Confirmed; fixed | `cb1c454` | Source-confirmed: `escalate_interrupt` converted any Ok non-ask result + flag → INTERRUPTED, discarding committed import/approval IDs; unit test blessed it (deleted). Red on baseline via new unit test (store created then 130). New: pre-dispatch refusal creates no `brn.sqlite3`; `finish` preserves completed results with the flag set. |
| R3 help checked after required arguments | Confirmed; fixed | `af39233` | Lead reproduced at baseline: `brn ask --help` → exit 2 "missing ask QUESTION"; `brn documents --help` → missing subcommand; `brn revisions diff --help` → missing `--draft`. Red: 2 new tests failed exit 2. Green after pre-scan; 16-invocation matrix + no-files-in-data-dir + unchanged-usage guards. |
| R4 ask failures lack structured context | Confirmed; fixed | `b6552a2` | Source-confirmed: IDs only in prose; new-conversation session id lost on error paths. Red: 9 CLI context assertions failed (missing `error.context`). Green incl. timeout-after-submission shape `{operation_id, session_id: <created>, recorded_status: "interrupted", provider_outcome: "unknown"}` with exit 124 (lead re-ran this live); persisted turn matches context ids; same-operation retry does not resubmit (`submitted.log` stays one line). |
| R5 codes from string matching | Confirmed; fixed | `cf09dc0` | Source-confirmed: `classify_workflow` used `starts_with` on English prefixes. Fixed: typed `ErrorKind` from store/retrieval/workflow variants; unit tests prove wording changes and look-alike `Error::Invalid("data directory is already owned: …")` text do not change the code; unknown stays `WORKFLOW_ERROR`. Existing subprocess code tests (WORKSPACE_BUSY, INDEX_MISSING/STALE/INVALID, PROFILE_UNAVAILABLE, OPERATION_CONFLICT) stayed green throughout. |

Implementation credits: packages A (R3), B (R1+R2), C1 (R5), C2 (R4) and the final doc/dead-code pass were implemented by GLM 5.3 Flash subagents under GLM 5.3 lead direction (integration, shared contracts, safety review of signal/cancellation/outcome semantics, verification). A fresh GLM 5.3 Flash agent independently reviewed the integrated diff: verdict APPROVE, R1–R5 CONFIRMED-FIXED, seven non-blocking minors (A1/A2/A3/A6 fixed in `14696b3`; A4/A5/A7 parked with rulings below).

## Verification results (all at `14696b3`, run by the lead unless noted)

- `cargo fmt --all -- --check` — pass.
- `cargo build --workspace --locked` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --locked` — **178 passed / 0 failed** (baseline 157/0; net +21 tests).
- `bash scripts/verify-end-to-end.sh` (workspace checks offline + provider-free brn-flow fixture) — pass, including brn-flow behavior/output unchanged.
- `git diff --check` — clean; working tree clean.
- Live on target Mac: broken provider pipe → structured error + reaping; `brn ask --help` → help exit 0; timeout envelope carries context matching persisted state; provider reaped after timeout.

Per-crate implementer runs (same tree, before the final docs-only commit): brn 47/0 then 56/0 after C2; workspace 168/0 after C1, 178/0 after C2. Reviewer independently ran `cargo test -p brn --locked` (56/0) and `cargo test -p brn-workflow --locked` (21/0).

## Rulings and parked findings

- A4: no subprocess test for SIGINT-during-`index build` exit 130 — no deterministic mid-build seam exists without new test hooks; covered by unit tests and `flow.rs` cancelled-build coverage. Parked.
- A5: TIMEOUT/INTERRUPTED prose messages name the caller's `--session` (compat-frozen strings asserted by brn-flow-era tests) while `context.session_id` carries the actually-established session. By design; context is the machine-readable truth.
- A7: one avoidable `AskFailure` clone on the CLI error path. Parked (churn > value).
- `provider_outcome: "interrupted"` is documented as reserved but unreachable: an interrupted record is ambiguous and reports `"unknown"`; a same-run server-confirmed interruption is also reported `"unknown"` (stateless honesty rule — avoids a second Ok-path return type).

## Limitations

- Signal/process tests are unix-only (consistent with the existing suite). Linux was not used; results above are native macOS (arm64).
- Late-SIGINT-after-commit preservation is covered by the deterministic unit test on the dispatch path, not a subprocess race test (import commits too fast to signal deterministically from outside).
- Native retrieval, live-provider behavior, publication and user acceptance remain unqualified, as before.
- The five commits are local only. Push when authorized: `git push origin feature/agent-cli-foundation` (fast-forward; no force). PR #2 left open and unmerged; no independent reviewer approval of these fixes is claimed beyond the recorded Flash review.
