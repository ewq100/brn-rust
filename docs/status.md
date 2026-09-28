# BRN Rust development status

Status: cloud and macOS Apple Silicon CLI build probes verified; private GitHub repository created. The chunk 01 provider trial is recorded in [the experimental App Server harness](../experiments/codex-app-server/README.md). The requested Chunk 02 provider lifecycle follow-up is recorded in [its evidence report](../experiments/codex-app-server/CHUNK-02.md). The [native editor experiment](../experiments/editor-trial/README.md) now builds on Apple Silicon; native interaction and user acceptance remain pending.

## Design and implementation record

- The Cargo workspace contains the original dependency-free `brn` probe, a UI-independent `brn-core` crate, a `brn-desktop` shell with an optional GPUI native feature, and an independent `brn-store` SQLite backend.
- The original `brn` probe accepts exactly one `--help` or `--version` argument. Missing, extra, and unknown arguments return a failure exit code and print an error.
- Rust 1.98.1 is pinned in `rust-toolchain.toml`; the official stable manifest version was verified before recording it. The profile is minimal, with rustfmt and Clippy components.
- The existing TypeScript BRN remains separate. The desktop and headless driver now share integrated storage, retrieval and provider work; no private-vault or user-data migration has run. Earlier experiment crates remain separate historical probes.

## Verification and next step

- Verified on Linux x86_64 using an isolated installation of Rust 1.98.1: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` all passed.
- `cargo test --workspace --locked` passed with zero unit tests. Five subprocess smoke checks passed: help and version succeed; missing, unknown, and extra arguments fail. The version output is `brn 0.0.0`.
- Private repository `ewq100/brn-rust` was created on GitHub on 2026-09-27. This starter records the verified cloud build probe and the remaining native verification work.
- Verified on macOS Apple Silicon (`Darwin arm64`) using Rust/Cargo 1.98.1: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked` passed. Five CLI smoke checks passed: help and version succeeded; missing, unknown, and extra arguments failed.
- The App Server trial demonstrated managed-ChatGPT streamed text, one read-only fixture tool call, conversation continuation, server-reported interruption, and persisted thread resume across two App Server processes. Four credential-free protocol tests passed. See its README for versions, commands, security limits, and the App Server versus Rig recommendation.
- Desktop interaction and packaging remain unverified. A separate native editor trial has now been built; its acceptance record is below.

## Chunk 02 provider lifecycle follow-up

- Based on the actual clean, pushed `d3b2410` handoff, which already extended `c8f01c8` with App Server-process resume.
- Separate harness invocations now persist and resume a synthetic conversation using a versioned private state file. Missing/invalid/uncertain state fails closed.
- Existing managed ChatGPT sign-in and one supported proactive refresh succeeded without exposing credentials or changing account settings. Natural expiry/revocation remains unverified.
- Installed Apple Silicon App Server launch/initialization/shutdown succeeded outside the command sandbox. No sidecar redistribution or signed app packaging is included.
- The original editor trial numbered 02 in the roadmap remains pending. Full commands, final check counts, review outcomes and remaining limitations live in the evidence report.

## Native editor trial (original roadmap 02)

- Standalone GPUI Kit 0.6.6 experiment added on `trial/editor-selection-comments`, based on the pushed provider lifecycle work at `eb62bfc`. The production CLI and provider source remain unchanged.
- The editor captures real selected passages, holds in-memory comments and an immutable opened revision, conservatively marks ambiguous/deleted anchors unresolved, and displays a revision diff. No source-file save/overwrite flow is included.
- Native build and automated model/CLI checks passed on macOS 15.3.1 arm64 with Rust 1.98.1 and Command Line Tools. Detailed commands and review results are in [EVIDENCE.md](../experiments/editor-trial/EVIDENCE.md).
- Native computer-use inspection was blocked by the locked Mac. The user was asked to unlock; real selection/focus, clipboard, scrolling, resize, undo and visual acceptance remain unverified. GPUI and the document model are still provisional candidates.
- Next acceptance step: unlock and exercise the runnable editor, then gather the user's own feedback. The retrieval adapter trial is now independently verified on synthetic fixtures; see below.

## Retrieval adapter trial (roadmap 03)

Implemented and verified on `trial/retrieval-adapters`, based on editor checkpoint `d422e192`. This standalone experiment uses synthetic fixture documents only; it does not import the user’s vault or choose production architecture. Real local FastEmbed/LanceDB semantic retrieval, keyword ranking, hybrid fusion, typed filters and separate-process reopening passed the recorded checks. Eight contract tests, three native state tests, seven actual-state failure checks, build/format/Clippy and the existing provider/editor regressions passed. Astra source review and scoped re-review found no blocking issues. See [evidence](retrieval-trial-evidence.md) for measured ranking, timings, limitations and reproduction.

Next: complete the deferred native editor and shell interaction/user evaluation gates; the architecture has since been approved as recorded below. Retrieval quality on a representative approved corpus, production indexing and packaging remain unverified.

## Architecture checkpoint (roadmap 04)

A concrete [architecture proposal](architecture-checkpoint.md) and [dependency record](architecture-dependencies.md) are prepared on `trial/architecture-checkpoint`, based on retrieval checkpoint `0e70a90`. This was initially a review artifact. On 2026-09-28 the user approved the design and explicitly deferred native editor acceptance, as recorded below. No account setting changed.

## Desktop shell (chunk 05)

On 2026-09-28 the user approved the concrete architecture and explicitly deferred native editor acceptance. The architecture checkpoint is accepted with GPUI provisional; native acceptance is not implied. Bounded desktop-shell implementation is complete on `trial/desktop-shell`, based on `199d8c5`. See [implementation plan](desktop-shell-plan.md). No provider, retrieval or authoritative storage integration is part of this chunk.

Shell implementation verification: native build, formatting, Clippy, 12 core tests, five CLI tests, three headless scenarios and the existing provider/starter regressions passed. Astra review and scoped re-reviews found no remaining Critical or Important source issues. [Commands and limitations](desktop-shell-evidence.md) preserve the still-open native shell/editor acceptance gates. The production workflow is not yet integrated.

## Authoritative storage and recovery (chunk 06)

The independent `brn-store` backend is implemented on `trial/storage-recovery`, based on shell checkpoint `8540de4`. It provides transactional schema migrations, exact immutable source versions, session/message projections, UUID operation deduplication, single-owner exclusion and interrupted-state recovery. Full [plan](storage-recovery-plan.md) and [verification evidence](storage-recovery-evidence.md) record commands, crash tests, review and limitations.

Workspace build/format/Clippy, one storage unit test plus 13 integration entries, existing core/CLI checks and starter/provider regressions passed. Forced-process tests preserve acknowledged records, roll back spilled uncommitted writes and prevent uncertain operations from replaying. Migration-header recovery includes an explicit fault-injection fixture; power-loss durability is not claimed. The desktop shell still uses its in-memory sample and is not wired to this backend.

Next implementation work: chunk 07 Markdown/text import and version tracking over the storage boundary. Native shell/editor acceptance remains open; no vault import, merge or release has occurred.


## First end-to-end personal trial (chunks 07–10)

On `trial/end-to-end-flow`, based on `a2fa7d5`, the shared workflow now connects selected text/Markdown imports, stable versions and explicit retrieval approval, durable derived generations, all three search profiles, exact source evidence, grounded streamed answers and persisted session reopening. [Evidence](end-to-end-flow-evidence.md) records the live synthetic-source run and remaining qualification. A new process continued the same server-side conversation; repeating a completed operation returned saved data without a provider executable configured.

Native UI controls are implemented over the same worker/workflow. Native interaction acceptance is still blocked by the locked Mac; this does not establish a fully accepted desktop or release. Writing/comments, publication, graph integration, backup/restore and distributable packaging remain later milestones. No original documents, credentials or account settings were modified.

### Native end-to-end acceptance (2026-09-28, after unlock)

The first-flow native gate is now observed: import/approval, real indexing, exact passage inspection, hybrid answer, process restart and semantic conversation continuation, cancellation, and closing during an active provider turn. Two completed and two interrupted synthetic turns survived reopening; the closed app and its owned sidecar were reaped. A clipped search label was corrected and visually rechecked. See [native acceptance evidence](end-to-end-flow-evidence.md#native-acceptance-after-unlock-2026-09-28). Packaging/signing, clean-machine installation, natural authentication expiry, broader layouts and screen-reader acceptance remain open.

## Focused desktop usability follow-up

The next requested milestone continues `trial/end-to-end-flow` from `7380b63`: native file selection, scrollable layouts, clear operation/error states and a repeatable unsigned local launcher. The [plan](desktop-usability-plan.md) tracks implementation and the [evidence](desktop-usability-evidence.md) separates automated checks from observed native behavior. Earlier locked-Mac notes above remain historical.

Usability implementation and native checks passed: single-file selection/cancel/validation, wrapped small-window layout with persistent controls, visible saved answers, progress/disabled actions, same-session restart/follow-up and cancellation from Activity. Workspace/native/launcher regressions passed, and Astra final review found no remaining issues. The durable local launcher is `~/Applications/BRN Usability Trial.app`; see the evidence for its explicit workspace/model paths and qualification limits.
