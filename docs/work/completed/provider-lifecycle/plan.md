# Chunk 02 provider lifecycle trial

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

Date: 2026-09-27. Base: `d3b2410` (already pushed, newer than the supplied `c8f01c8` handoff).

## Design and scope

Extend the disposable Rust App Server harness, not the production CLI. Persist a small versioned JSON record containing the server thread ID, fixture working directory, and turn outcome. Codex retains authoritative conversation history and all credentials. Separate harness commands must create and resume a conversation across executable invocations; invalid state must fail without silently starting a replacement thread. Save pending/uncertain state before sending work; do not retry a possibly accepted turn.

Use supported `account/read` in managed ChatGPT mode. A separate explicit refresh check calls `refreshToken: true`, without reading or printing credentials. Auth failures return actionable instructions to use supported Codex login, never automatic logout or login changes. Synthetic tests cover unavailable, expired/revoked, and unsupported auth while the real sign-in stays intact.

Resolve an explicit `CODEX_BIN` first, then an installed CLI on PATH. Launch stdio App Server, bound startup/request/turn waits, report early exit distinctly, and close stdin for bounded graceful shutdown with kill/wait fallback. Do not redistribute the executable embedded in ChatGPT.app. Research standalone CLI licensing and retain signing/notarization as a separate packaging gate.

The user's requested Chunk 02 is a provider follow-up; the roadmap's existing editor trial 02 remains pending and unchanged in scope.

## Global constraints

- Trial-only changes; preserve unrelated work and the working subscription sign-in.
- No credential reads, copies, raw account/error payload logs, or commits; no API-key fallback.
- No new paid services, UI work, broad refactors, merge, release, or force-push.
- Meaningful credential-free persistence and failure tests, with live tests bounded and explicitly invoked.
- Central integration and final Astra review; Sol implementation; Luna narrow research/checks.

## Task 1: Implement the lifecycle probe

Files: `experiments/codex-app-server/src/main.rs`, focused state/auth/sidecar modules if needed, and credential-free integration tests. Keep serde_json as the only direct dependency unless a concrete need arises.

Interfaces: keep `live` and `resume-live`; add `persist-start STATE_PATH`, `persist-resume STATE_PATH`, `auth-check`, `auth-refresh`, and `sidecar-check`. Reject unknown or surplus arguments before spawning a process. State is versioned, atomically written with restrictive permissions, not committed. Existing paths are not silently overwritten by start. Resume checks returned thread identity and explicit non-ephemeral persistence. Clear errors for missing, corrupt, unsupported-version, invalid-ID, and uncertain/pending state. A synthetic marker in the initial prompt must be recalled on resume without including it in the resumed prompt.

- [x] Write and run failing persistence and error handling tests.
- [x] Implement minimal state, auth recovery classification/redaction, and sidecar supervision; preserve prior live checks.
- [x] Exercise fake subprocesses for missing binary, startup failure, unexpected exit, timeout, graceful/fallback shutdown; no real account needed.
- [x] Run build, fmt, Clippy and tests for the harness; provide report with commands and outcomes.
- [x] Independent review of task compliance and quality, followed by fixes as needed.

## Task 2: Central integration, evidence, and delivery

Files: trial README, `docs/work/completed/early-checkpoints/status-history.md`, roadmap clarification, this record, reproducible verification scripts where useful.

- [x] Check official docs and distribution terms; record exact sources and remaining packaging gates.
- [x] Run starter build/fmt/Clippy/tests and five CLI smoke cases; harness equivalents and failure smoke cases.
- [x] Launch installed Apple Silicon sidecar, check managed auth, request supported refresh once, then run independent start/resume processes against synthetic data.
- [x] Record what was observed versus unverified expiry/revocation, reboot, packaging, signing, and crash recovery.
- [x] Review complete diff for correctness and secrets with Astra; resolve important findings.
- [x] Commit, push the new trial branch, verify remote SHA, leave original checkout untouched.

## Review focus

Partial/corrupt state must not erase a prior conversation. A lost turn response must not lead to automatic duplicate submission. Server error payloads and stderr must not leak credentials. Chatty notifications must not defeat deadlines. Sidecar drop must reap its child even after initialization fails.

## Progress

- Original checkout clean; no applicable AGENTS.md found in repository or ancestor paths inspected.
- Local and remote baseline both `d3b2410d91a7e7a6553ad8d88c30c04875ddd7d4`.
- Native worktree tool unavailable for this thread (fixture cwd is not a Git repository); authorized Git worktree created at `/private/tmp/brn-chunk-02`.
- Baseline Rust 1.98.1 offline tests: starter 0 tests, harness 4 tests, all passed.

- Final verification: offline script passed all gates, 16 harness tests and 9 smoke cases; final transport code passed live regressions and saved-state resume. Astra whole-branch review approved with no findings.

- Delivery verified: implementation checkpoint `7c49770407a84b4bb62eb5047fcc4a2f48964323` was pushed to the private remote and matched `git ls-remote`. A documentation-only follow-up records this result; no merge or release occurred.
