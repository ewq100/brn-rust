# Desktop shell implementation plan — chunk 05

Approved scope: user accepted the architecture and explicitly deferred native editor acceptance on 2026-09-28. GPUI remains provisional. Implement the bounded first production shell, not provider/retrieval/storage integration. Existing authorization covers delegation, routine implementation, tests, review and trial push.

## Structure and ownership

Add `crates/brn-core` for UI-independent state, typed commands/events, generation correlation and a deterministic cancellable worker. Add `crates/brn-desktop` with optional `native-ui` GPUI Kit 0.6.6 binary, keeping heavyweight UI dependencies optional for headless tests. Root workspace contains these plus unchanged build-probe crate. Worker agent owns these crates and root Cargo manifests/lock. Parent owns docs and scripts. Reuse previously verified GPUI APIs; no paid services or production user documents.

## Requirements

- One window with Workspace, Activity and Settings navigation; visibly label this as a shell with sample work, not an integrated editor/search app.
- Display chosen data directory. CLI supports explicit absolute `--data-dir`; a platform-default path may be resolved without writes until launch. Validate directory and useful file/permission errors. Use a disposable explicit directory for verification. No authoritative database, migration or document file write.
- Typed UI-independent requests/events carry operation identity and working-copy generation. Edits increment generation, including changes back to earlier text. Stale progress/completions cannot update current-result state. At most one task active; repeated start returns a useful busy response.
- Deterministic sample work on owned input runs off the UI thread. Bounded/coalesced progress, preserved terminal state, cooperative cancellation and bounded shutdown. No unbounded event accumulation, blocking send deadlock, detached worker leak, or GUI-thread busy loop. Own worker lifecycle through close/drop.
- Expose narrow headless checks for sample completion, cancellation and stale-event rejection. Native rendering/interaction is a separate gate, never inferred from compilation.

## Tasks

1. Sol writes meaningful failing model/worker/CLI tests, runs them, then implements core + shell with verified library APIs. Cover stale generations, concurrent start refusal, cancel-vs-completion, completed worker reuse, close during active work, channel backpressure, valid/missing/non-directory data path and bad CLI arguments. Keep failures explicit; prevent automatic retries or discarded terminal state.
2. Parent builds, formats, runs Clippy and tests; native/headless smoke checks use synthetic input and a freshly created temporary data directory. Attempt native computer-use only if Mac accessible; record locked blocker if still present. Preserve all previous trial branches and unrelated work.
3. Independent Astra review of actual diff and lifecycle behavior; fix/re-review findings. Document exact commands, results, limits, implementation/acceptance status and next step. Commit/push trial/desktop-shell and verify remote SHA. Do not merge/release.

## Acceptance and limits

Implementation can be pushed while Mac is locked, but chunk05 native acceptance remains open until observed launch/responsiveness, navigation, cancellation, stale-result behavior and close-during-work pass. No indefinite test waiting: bound timing tests with generous explicit deadlines. Long-lived storage, external sidecars, model calls, ingestion/retrieval and editor integration stay out of this chunk.

## Execution record

Implementation, headless/native compilation checks, original provider regression checks and independent Astra source review are complete. Review corrections cover oversized visible input, deterministic cancellation/shutdown races, explicit quit hooks and legacy root command compatibility. See [evidence](desktop-shell-evidence.md). Native interaction acceptance remains open under the user's explicit deferral. Delivery is a trial-branch checkpoint, not completion of the native acceptance gate.
