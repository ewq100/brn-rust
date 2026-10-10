# BRN Threads rebuild plan

**Branch:** `rebuild/threads`. **Reference baseline:** `af9239c741c7ab0983e62f0253e607b9607727e6`. **Date:** 10 October 2026.

## Assignment and target

The owner asked for a prepared environment that a build agent can take over. This setup supplies the contract, branch, review packet, and prompts. It has not implemented the new core.

When assigned the [build prompt](build-prompt.md), implement the first usable release described by the [canonical target](../../../architecture/threads-target.md). Refactor or rewrite as needed. The owner is not using the app; there is no gradual data migration requirement. Preserve unrelated work, existing user data, and repository history.

The target is the single requirements owner. Product/architecture overview files summarize it. If review reveals a contradiction or unnecessary mechanism, record and apply the smallest justified correction across affected current documents. Do not make competing plans or treat every library choice as an owner approval.

## Review resolved and design ready

The Opus review at `a2af886e5442af0cfa4b8884181446c7d98d1eea` assessed the setup at `3519bfb85e3c68cb17dd6df31061e9f5dc155e73` as ready with specified corrections. [Lead dispositions](review.md#build-lead-dispositions) accept the useful findings with narrower, safer fixes. Mathematical equation support is removed from the first-release requirements following the owner's clarification. No new whole-plan review is a prerequisite.

Read the [thread and agent behavior design](../../../architecture/threads-behavior.md) with the canonical target. It fixes the minimum interaction/state model, instruction packaging, tool outcomes, and concrete journeys. This is the bounded design work before implementation. The first build task translates it into small interfaces and proofs, not another exhaustive specification phase.

### Contracts settled before dependent code

1. Host-owned operation identity exists durably before preparation; both prepare and apply replay idempotently, including creations and lost responses.
2. The SQLite writer boundary checks expected versions, protection, exact host-bound target/action scope and persisted editing guards. Thread links are context, never protected-write authority.
3. Human Save carries its base version and edit-session/generation; delayed autosave cannot resurrect a closed guard. A guarded member defers its dependent group whole.
4. A new final-use core crate owns a distinct schema/application identity and explicit safe data selection. Independently changing links/comments do not bump untouched note versions.
5. Runs persist small BRN-owned progress, guide identities and receipts, not opaque Rig checkpoints or raw tool transcripts. Continue starts a fresh invocation from deliberate durable state.
6. Full imports preserve supported substantive prose/structure, tables and figures. Qualify a normal text-layer PDF and DOCX process example; no math renderer or universal completeness detector.

The lead owns concrete type/table names, dependency choices within this design, and bounded integration trials. Do not create a general event framework, permission-rule editor, dependency solver, sandbox platform, or universal converter to answer these questions.

## Dependency qualification

Source inspection on 10 October 2026 supports qualifying Rig 0.44.0 before the new agent integration and GPUI Kit 0.7.1 before the new native UI. Neither upgrade has been compiled or exercised in this preparation environment. Use separate owned changes so failures remain attributable. The small core can proceed independently; coordinate shared Cargo manifests/lockfiles through the lead.

### Rig 0.44.0 before the new agent runtime

The current AI crate pins Rig, rig-core and rig-reqwest to 0.43.0; the root manifest overrides rig-agent with a narrow local logging patch. The published 0.44 source removes that specific raw stderr print and changes relevant error/stream/tool/history APIs. Its release description is cumulative and its migration guide stops at 0.43, so inspect the actual tag comparison.

1. Align the used Rig-family versions and lockfile, adapting the retained subscription, error, stream and tool boundaries. Preserve explicit provider/account/model/effort selection and cancellation behavior; do not silently replace the subscription adapters or add providers.
2. Set both `max_invalid_tool_call_retries(0)` and `max_consecutive_malformed_tool_calls(0)` initially; keep unknown finish reasons rejected unless a supported route proves a specific need. Qualify a malformed read plus a valid mutation in the same model batch: zero backend tool dispatch and no extra model request. Use synthetic fixtures for the retained ChatGPT Responses and Copilot Chat/Responses adapters.
3. Pass the existing process-stderr regression in `crates/brn-ai/src/provider_stderr_tests.rs` against the published dependency. Remove the vendor override/copy only when its actual fix and relevant behavior pass. Source-level absence of one print alone does not qualify every logging path.
4. Pass relevant authentication-format, streaming, partial-answer, safe-error, budget and cancellation checks. Run a bounded public/synthetic stream-to-read-tool-to-answer journey through the already selected configured route when live testing is assigned.
5. In M2, prove host ToolContext and BRN prepare/apply receipt handling. Rig tool/stream completion events do not establish a SQLite commit; the application receipt does. Do not serialize ToolContext or AgentRun wholesale or add ECS/cassette infrastructure to gain continuation.

Rig's preamble and normal tool registration support the fixed packaged guide catalog in the behavior design. The reviewed release does not provide automatic SKILL.md discovery. A small allowlisted guide tool is sufficient.

Sources: [BRN manifest](../../../../crates/brn-ai/Cargo.toml), [vendor rationale](../../../../vendor/README.md), [Rig release](https://github.com/0xPlaygrounds/rig/releases/tag/v0.44.0), [tag comparison](https://github.com/0xPlaygrounds/rig/compare/v0.43.0...v0.44.0), [streaming contract](https://github.com/0xPlaygrounds/rig/blob/v0.44.0/crates/rig-agent/src/agent/streaming.rs), [RunSpec](https://github.com/0xPlaygrounds/rig/blob/v0.44.0/crates/rig-agent/src/run/spec.rs), [run persistence caveats](https://github.com/0xPlaygrounds/rig/blob/v0.44.0/crates/rig-agent/README.md#the-run-protocol).

### GPUI Kit 0.7.1 before the new native UI

The current desktop pins Kit 0.6.6 and the lockfile uses GPUI snapshot 0.3.6. Moving to 0.7.1 crosses the 0.7.0 breaking changes and its 0.3.8 snapshot family. Useful upstream primitives include rendered-selection-to-source mapping, source-range highlights, editor decorations and long-text/table/IME improvements. The current `native/mod.rs` calls removed manual dialog-layer rendering, so this requires actual shell adaptation.

1. Upgrade the exact Kit pin and matching snapshot family together. Adapt retained startup/Root/overlay handling and affected tests to the supported APIs; preserve headless feature isolation. Do not enable unrelated speech, chart or webview features merely because the release adds them.
2. On the Mac, open a long Markdown note with headings, repeated phrases, Estonian text, a wide table and a managed figure. Exercise selection/copy, typing, Undo, scrolling, heading/search reveal, dialogs and keyboard focus.
3. Select one repeated occurrence, comment, edit before/inside/delete it, then Save/reopen. Persist BRN note/revision/range/quote and known mappings. Snapshot-local selection/highlight APIs and edit decorations do not supply durable identity; unsupported mapping remains visibly unresolved. Deleted decorations are not automatically restored by editor Undo.
4. Present current/proposed protected-note text and apply through the shared service. A useful first review surface is enough; no CRDT, editor fork or hunk-merge engine is required.

Sources: [desktop manifest](../../../../crates/brn-desktop/Cargo.toml), [Kit 0.7.0 migration](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.0), [Kit 0.7.1 release](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.1), [TextView](https://github.com/longbridge/gpui-kit/blob/v0.7.1/crates/base/src/text/state.rs), [source mapping](https://github.com/longbridge/gpui-kit/blob/v0.7.1/crates/base/src/text/range_highlight.rs), [editor decorations](https://github.com/longbridge/gpui-kit/blob/v0.7.1/crates/base/src/input/editor/decorations.rs).

If either target version has a concrete blocker, record the affected behavior and smallest workaround or temporary retained version. Continue independent core work. A newer library is a reuse opportunity, not permission to expand the product or reopen the entire architecture.

## Reuse and replacement map

| Area | Starting point | Direction |
|---|---|---|
| Providers | `crates/brn-ai`, patched Rig and current provider/auth integration | Reuse compatible transport and selected-model behavior; prove tool execution/cancellation. |
| Native UI | `crates/brn-desktop`, existing GPUI toolkit | Reuse controls and platform foundation; rebuild around Threads and the new service. |
| Input conversion | `crates/brn-intake`, maintained decoders/helpers | Keep useful decoding and isolation; replace original-retention assumptions with transient intake and explicit full notes. |
| Retrieval | `crates/brn-retrieval` | Adapt useful indexing/search to current database records; keep indexes rebuildable. |
| Persistence | Useful patterns in `crates/brn-store` | New final-use core crate with a fresh schema and revision/receipt service; no wholesale old WorkStore import or dependency on intake. |
| Orchestration | `crates/brn-workflow` and desktop worker routes | Replace route-specific approval, mutable-vault coordination, and separate old lifecycles. |
| CLI | `crates/brn` | Thin commands over the same checked service; old command compatibility is not required. |
| Tests/CI | Existing scripts, behavioral cases, required lanes | Retain meaningful checks; replace retired-contract assertions with new behavior when replacing code. |

Produce a concise dependency/deletion map before removing coupled paths. The source remains temporarily during setup for reference. Each replacement slice removes the superseded reachable production path; no permanent `legacy/` runtime or dual writes.

## Milestone 1 Prove the smaller core

Build the final-use headless core in a new small crate with notes, meaningful revisions, ordinary/protected policy, source observations, grouped changes, host authority, versioned recovery buffers and compensation. Keep dependencies narrow without an arbitrary library whitelist. The crate must build/test without the desktop, intake, FastEmbed or protoc. Reuse SQLite backup and suitable FTS passage-index patterns when needed; optional semantic/native retrieval can wait for its journey.

Demonstrate:

- One atomic multi-record creation/update and idempotent retries after preparation and after application commit but before either response; exactly one effect and receipt.
- Rejection when content, metadata, protection, or lifecycle changed since preparation.
- Ordinary delegated writes, protected-write refusal, and a specific owner-authorized target/action. A protected B linked to the same thread as authorized A remains protected; content-edit permission cannot unprotect or archive A.
- Stale-open/type/Save preserves typed work and conflicts; persisted guards defer grouped AI changes across desktop/CLI and restart. A delayed recovery write cannot resurrect a saved/discarded buffer or erase newer typing.
- Immediate multi-record Undo, an unrelated new thread link that does not obstruct Undo, and a later real edit that produces a conflict preserving subsequent writing; known direct dependents are reported for refresh.
- Crash/restart and a complete backup/restore into a new directory.

Use synthetic data. Establish a distinct database filename, SQLite application/schema identity and safe explicit data selection before invoking binaries. Reject incompatible existing markers before creating/opening state. While old commands remain reachable, prevent current repo entry points from mixing old/new data directories; do not assume a different filename alone makes sharing safe. Do not create a disposable second engine. Update affected commands, tests, and CI with the new core.

In parallel, qualify the dependency upgrades and realize the behavior design with existing native controls or a lightweight layout trial. Shared manifest edits need one owner. UI exploration does not block core invariants.

## Milestone 2 Complete an agent and import journey

Reuse one working selected provider and a small tool set over the shared service. Run the email-to-project-note-to-suggested-Action-to-reply-draft journey using synthetic/public input. Routine maintenance applies under delegation; protected conflicts surface in one thread.

Implement the packaged base guide, four SKILL.md playbooks and the small allowlisted guide tool from the behavior design. Retain provenance, instruction-bundle identity and actual operation outcomes. Evaluate a few representative success/boundary outcomes instead of making an exact full-prompt fingerprint the semantic acceptance gate. Verify cancellation, bounded work, fresh-invocation continuation, and raw-payload cleanup. Place a unique canary in synthetic source material deliberately excluded from saved results; after restart cleanup it must not remain in logical BRN records, logs, checkpoints or abandoned temporary inputs. Intentional excerpts/full notes and storage-page forensic erasure are different concerns.

Qualify full-note import using one ordinary text-layer PDF and one DOCX process description with a small hand-checked source inventory. Preserve complete substantive text, necessary tables/figures, source structure and references within the supplied scope. The existing helper supports DOCX/PPTX/EML; qualify a maintained PDF conversion path before the dependent journey. Mathematical conversion/typesetting and math-heavy fixtures are outside this release. Show partial status for a deliberately unsupported substantive element. Full notes start protected, remain readable offline, and permit linked summaries. Do not confuse deferred Office output with required full-content input.

Try provider availability early when the build assignment permits bounded live tests. Surface a needed sign-in promptly and continue independent offline work. Do not silently fall back to another account, paid API, provider, or model.

## Milestone 3 Make daily interaction usable

Implement native Home/Needs you, Ask or delegate, thread detail, notes, Save, current/proposed review, simple comments, current search, Action progress, history/Undo, and export/backup status.

Use one logical note with section navigation for long imports. Use supported representations for tables and figures; a rich editor fork or CRDT needs a specific demonstrated benefit.

Show actual native journeys on the target Mac when available. Local reading, writing, search, Actions, existing-candidate review, and recovery work without inference. If the Mac is unavailable or locked, retain one explicit interaction test task and continue independent work.

## Milestone 4 Qualify and remove the old production paths

Run the [canonical acceptance journeys](../../../architecture/threads-target.md#acceptance-journeys). Check semantic outcomes against inputs; deterministic tests alone cannot prove agent judgment or conversion completeness.

Provide a usable Mac candidate, remaining known limitations, and reproducible owner scenarios. Remove superseded storage/orchestration/recovery routes and update documentation and actual CI commands. Check that no default command accidentally opens old BRN data or launches the old engine.

Stop optional testing once the relevant behavior is sufficiently verified. Distinguish implementation, verification, owner acceptance, and integration. Do not merge to main or release under this setup-only instruction.

## Later scope

DOCX/PPTX/HTML generation is required later. Direct external sending, broad connectors, daemon/sync/multiplayer, arbitrary branch/hunk merging, sentence-level trust, and a plugin marketplace are deferred. Jujutsu, CRDTs, raw-original archives, and permanent keystroke replay are not prerequisites.

## Handoff and progress

Use this plan and [status](../../../status.md) across sessions. Record exact current HEAD, completed proofs, relevant environment, unresolved findings, and next action. Commit/push task-owned milestones without waiting for ceremonial approval. Fetch and reconcile concurrent remote changes without force-pushing.

[Setup evidence](evidence.md) records what this preparation actually checked. The [review record](review.md) preserves the independent report and completed lead dispositions; implementation proofs remain to be run.
