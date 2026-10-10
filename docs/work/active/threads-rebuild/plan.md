# BRN Threads rebuild plan

**Branch:** `rebuild/threads`. **Reference baseline:** `af9239c741c7ab0983e62f0253e607b9607727e6`. **Date:** 10 October 2026.

## Assignment and target

The owner asked for a prepared environment that a build agent can take over. This setup supplies the contract, branch, review packet, and prompts. It has not implemented the new core.

When assigned the [build prompt](build-prompt.md), implement the first usable release described by the [canonical target](../../../architecture/threads-target.md). Refactor or rewrite as needed. The owner is not using the app; there is no gradual data migration requirement. Preserve unrelated work, existing user data, and repository history.

The target is the single requirements owner. Product/architecture overview files summarize it. If review reveals a contradiction or unnecessary mechanism, record and apply the smallest justified correction across affected current documents. Do not make competing plans or treat every library choice as an owner approval.

## Recommended review before core interfaces settle

Use [the independent review brief](review-brief.md) for one focused pass, preferably Opus when available. Its report belongs in [review.md](review.md). The build lead checks each finding and records accepted, rejected with reason, or needs evidence, plus the smallest fix/proof.

Review is not a demand for both Opus and Astra or a recurring redesign cycle. An unavailable named model does not block authorized work. If building is assigned before an external report exists, use an available independent reviewer where possible, state any unavailable independence, and resolve concrete core risks before depending on them.

## First decisions owned by the build lead

Resolve these from the actual checkout and bounded trials, not an owner questionnaire:

1. Minimal current-record/revision/operation model and transaction boundary, including note assets and durable operation identity.
2. Host-owned authority scope and protection checks; confirmation belongs to a revision.
3. Editing-buffer coordination across restart and grouped changes that touch a currently edited note.
4. Maintained converter/reader fit for a representative research paper and process description.
5. Minimum native comparison/editor/long-note reading surface with honest comment limits.
6. Reused provider runtime behavior for transient raw inputs, durable progress, retries, and cancellation.

Do not freeze a complete low-level schema before these contracts are clear. Do not build a general event framework, dependency solver, sandbox platform, or universal document converter to answer them.

## Reuse and replacement map

| Area | Starting point | Direction |
|---|---|---|
| Providers | `crates/brn-ai`, patched Rig and current provider/auth integration | Reuse compatible transport and selected-model behavior; prove tool execution/cancellation. |
| Native UI | `crates/brn-desktop`, existing GPUI toolkit | Reuse controls and platform foundation; rebuild around Threads and the new service. |
| Input conversion | `crates/brn-intake`, maintained decoders/helpers | Keep useful decoding and isolation; replace original-retention assumptions with transient intake and explicit full notes. |
| Retrieval | `crates/brn-retrieval` | Adapt useful indexing/search to current database records; keep indexes rebuildable. |
| Persistence | `crates/brn-store` | Fresh schema and revision/receipt service; no wholesale old WorkStore import. |
| Orchestration | `crates/brn-workflow` and desktop worker routes | Replace route-specific approval, mutable-vault coordination, and separate old lifecycles. |
| CLI | `crates/brn` | Thin commands over the same checked service; old command compatibility is not required. |
| Tests/CI | Existing scripts, behavioral cases, required lanes | Retain meaningful checks; replace retired-contract assertions with new behavior when replacing code. |

Produce a concise dependency/deletion map before removing coupled paths. The source remains temporarily during setup for reference. Each replacement slice removes the superseded reachable production path; no permanent `legacy/` runtime or dual writes.

## Milestone 1 Prove the smaller core

Build the final-use headless core with notes, immutable meaningful revisions, ordinary/protected policy, source observations, grouped changes, checked authority, versions, and compensation.

Demonstrate:

- One atomic multi-record change and an idempotent retry after commit but before response.
- Rejection when content, metadata, protection, or lifecycle changed since preparation.
- Ordinary delegated writes, protected-write refusal, and a specific owner-authorized protected change.
- Unsaved human writing defers AI commits, including after buffer recovery; grouped writes stay coherent.
- Immediate multi-record Undo and a later-edit conflict that preserves subsequent writing.
- Crash/restart and a complete backup/restore into a new directory.

Use synthetic data. Establish new schema identity and safe explicit data selection before invoking binaries. Do not create a disposable second engine. Update affected commands, tests, and CI with the new core.

In parallel, clarify the native thread/review layout with existing controls or a lightweight prototype. UI exploration should not block core invariants.

## Milestone 2 Complete an agent and import journey

Reuse one working selected provider and a small tool set over the shared service. Run the email-to-project-note-to-suggested-Action-to-reply-draft journey using synthetic/public input. Routine maintenance applies under delegation; protected conflicts surface in one thread.

Retain provenance and actual operation outcomes. Verify cancellation, a bounded run, resumed work, and raw-payload cleanup without hiding a source archive in logs/checkpoints.

Qualify full-note import using at least one representative research paper and one process description. Preserve complete substantive text, necessary tables/equations/figures, source structure, and references within the supplied scope. Show partial status for a deliberately unsupported substantive element. Full notes start protected, remain readable offline, and permit linked summaries. Do not confuse deferred Office output with required full-content input.

Try provider availability early when the build assignment permits bounded live tests. Surface a needed sign-in promptly and continue independent offline work. Do not silently fall back to another account, paid API, provider, or model.

## Milestone 3 Make daily interaction usable

Implement native Home/Needs you, Ask or delegate, thread detail, notes, Save, current/proposed review, simple comments, current search, Action progress, history/Undo, and export/backup status.

Use one logical note with section navigation for long imports. Use supported representations for math/tables/figures; a rich editor fork or CRDT needs a specific demonstrated benefit.

Show actual native journeys on the target Mac when available. Local reading, writing, search, Actions, existing-candidate review, and recovery work without inference. If the Mac is unavailable or locked, retain one explicit interaction test task and continue independent work.

## Milestone 4 Qualify and remove the old production paths

Run the [canonical acceptance journeys](../../../architecture/threads-target.md#acceptance-journeys). Check semantic outcomes against inputs; deterministic tests alone cannot prove agent judgment or conversion completeness.

Provide a usable Mac candidate, remaining known limitations, and reproducible owner scenarios. Remove superseded storage/orchestration/recovery routes and update documentation and actual CI commands. Check that no default command accidentally opens old BRN data or launches the old engine.

Stop optional testing once the relevant behavior is sufficiently verified. Distinguish implementation, verification, owner acceptance, and integration. Do not merge to main or release under this setup-only instruction.

## Later scope

DOCX/PPTX/HTML generation is required later. Direct external sending, broad connectors, daemon/sync/multiplayer, arbitrary branch/hunk merging, sentence-level trust, and a plugin marketplace are deferred. Jujutsu, CRDTs, raw-original archives, and permanent keystroke replay are not prerequisites.

## Handoff and progress

Use this plan and [status](../../../status.md) across sessions. Record exact current HEAD, completed proofs, relevant environment, unresolved findings, and next action. Commit/push task-owned milestones without waiting for ceremonial approval. Fetch and reconcile concurrent remote changes without force-pushing.

[Setup evidence](evidence.md) records what this preparation actually checked. The [review record](review.md) records the independent review and lead dispositions when performed.
