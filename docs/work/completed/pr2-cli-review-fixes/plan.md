# PR #2 review fixes: signals, durable outcomes, help, ask context, typed codes

Status: implemented, verified and merged as PR #2 (see closure note in [evidence](evidence.md)).
Date: 2026-09-29. Branch `feature/agent-cli-foundation`, base `df47114` (PR #2 reviewed head), resulting head `14696b3`. Remote `origin/feature/agent-cli-foundation` was still at `df47114` when checked; the fix commits are local, ready to push.

Follow-up pass (same day): the three remaining findings F1 (cancellation after workspace wait), F2 (output delivery vs BrokenPipe) and F3 (lock-error classification) were fixed at `9e83142..9b33058` on the same branch; see the "Second pass" section in [evidence](evidence.md). At the time of that pass PR #2 was still unmerged.

## Outcome

Fix the five review findings on the agent-facing CLI foundation without changing the architecture (`brn` CLI → `brn-workflow` → store/retrieval/provider), brn-flow output, or the desktop worker contract.

Scope included:

- R1 process-wide SIGPIPE handling bypassing provider cleanup.
- R2 post-hoc escalation of completed non-ask results to INTERRUPTED.
- R3 command help rejected by required-argument validation.
- R4 ask failures lacking structured recovery context.
- R5 CLI error codes classified by English message prefixes.

Excluded: new commands, IPC/daemon/HTTP/MCP/TUI, database migrations, provider replacement, unrelated cleanup, live Codex or model downloads.

## Decisions

- R1: remove the SIGPIPE `SIG_DFL` override entirely; Rust's default ignored disposition makes pipe writes return `EPIPE`, which the provider `writer_loop` already maps to `ProviderError::UnexpectedExit`. All CLI output writes become fallible; a closed consumer pipe on a completed operation exits 0 quietly (delivery to a closed pipe is not promised and is not a domain failure). Progress/delta writes to stderr are best-effort and never affect lifecycle.
- R2: cancellation is checked at one safe boundary — before dispatch of a non-ask command (before the workspace opens). Completed results are never rewritten into interruptions. Error-path cancellation reporting derives from the typed `Cancelled` kind produced by workflow boundary checks, not from the flag state coinciding with an unrelated failure.
- R3: an exact-token `--help` pre-scan in `cli::parse` (same pattern as the existing `--json` pre-scan) wins before all validation at every command level. All help remains the single global text; per-command help text was deliberately not built (presentation polish, not a parser rewrite).
- R4: `brn-workflow` produces a typed `AskFailure` (kind, message, operation id, established session id, recorded status, provider outcome) at the point each fact becomes known; the CLI renders it as an additive-optional `error.context` field (schema_version stays 1). Honesty rules: `recorded_status` is claimed only after the durable write succeeds; every `Err` path reports `provider_outcome: "unknown"` (transport loss is not proof of cancellation); an interrupted record is ambiguous and reports `"unknown"`; `"completed"`/`"failed"` imply provider-confirmed outcomes.
- R5: `brn-store` gained `Error::WorkspaceBusy`/`Error::OperationConflict`; `brn-workflow` gained `ErrorKind`/`WorkflowError` (compat: `From<String>`/`From<WorkflowError> for String` keep brn-flow and the worker string-error surfaces unchanged); the CLI maps kinds to the existing machine codes, with `WORKFLOW_ERROR` as the honest fallback. No message string changed.

## Invariants kept

- SQLite stays authoritative; no migrations; message strings byte-identical for compatibility.
- Operation IDs bind payload identity; uncertain external execution is never replayed or described as exactly once.
- Provider credentials/raw payloads never enter messages or context.
- Owned-sidecar supervision and reaping unchanged (verified by test).

## Verification plan

Red-green per finding (see [evidence](evidence.md)), then: `cargo fmt --all -- --check`, `cargo build --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `bash scripts/verify-end-to-end.sh`, `git diff --check`, plus live manual reproductions of R1/R3/R4 on the target Mac. Fake providers and synthetic homes only; no credentials, live provider calls, or model downloads.
