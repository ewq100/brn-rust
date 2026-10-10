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

Source inspection on 10 October 2026 supports qualifying Rust 1.99.0 as the build baseline, Rig 0.44.0 before the new agent integration and GPUI Kit 0.7.1 before the new native UI. These upgrades have not been compiled or exercised in this preparation environment. Qualify the compiler with unchanged dependencies first, then use separate owned dependency changes so failures remain attributable. The small core can proceed independently; coordinate shared Cargo manifests/lockfiles through the lead.

### Rust 1.99.0 as the build baseline

The repository currently pins Rust 1.98.1. Version 1.99.0 is a reasonable maintenance update during the rebuild, not a Threads feature requirement or a demonstrated BRN performance improvement. Its relevant qualification surface is the compiler/LLVM update, warnings and bundled formatting/lint tools. Rig 0.44.0 declares a lower minimum of Rust 1.95.0; the inspected GPUI Kit manifests do not declare a 1.99 requirement. Actual native compatibility still needs a build.

1. Install the exact 1.99.0 toolchain with rustfmt and Clippy in the build environment. Keep 1.98.1 available as the existing fallback. Test the compiler-only change against the current lockfile before changing Rig or GPUI.
2. In that change, update `rust-toolchain.toml` and make actively retained verification scripts respect the root pin. Six scripts currently override it with `cargo +1.98.1`: `verify-storage.sh`, `verify-end-to-end.sh`, `verify-desktop-shell.sh`, `verify-retrieval-trial.sh`, `verify-trial.sh` and `verify-editor-trial.sh` under `scripts/`. Check their working-directory assumptions rather than replacing every historical version string. CI setup and development preflight already read the root pin.
3. Update current setup instructions and decide the supported minimum in retained first-party `rust-version` metadata explicitly. The compiler pin and a minimum-version claim are different promises; this application does not need a second compiler-support matrix. Preserve historical experiment/evidence versions. Do not add an edition migration or broad dependency refresh to this change.
4. Run the existing formatter check, relevant workspace build/tests and Clippy with the existing warning policy, then the required Mac native UI/retrieval/combined lanes and bounded native interaction check. Record exact compiler, features and results. Review any formatter or new-warning fixes; do not suppress the existing gates. Native compilation alone does not prove input/rendering behavior.
5. Adopt the new pin after these checks pass. If it exposes a substantial problem in a path about to be removed, record the concrete blocker and continue independent core work on 1.98.1 instead of turning the optional update into a prerequisite project.

CI already sets `CARGO_INCREMENTAL=0`, so Cargo's new CI default does not improve that configuration. The new built-in `debug` profile currently behaves like `dev`; no profile rewrite or faster-build claim follows from its existence.

Sources: [current toolchain](../../../../rust-toolchain.toml), [CI setup](../../../../.github/actions/setup-rust/action.yml), [Rust announcement](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/), [versioned release notes](https://github.com/rust-lang/rust/blob/1.99.0/RELEASES.md), [Rig minimum](https://github.com/0xPlaygrounds/rig/blob/v0.44.0/Cargo.toml).

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

If a target version has a concrete blocker, record the affected behavior and smallest workaround or temporary retained version. Continue independent core work. A newer library is a reuse opportunity, not permission to expand the product or reopen the entire architecture.

### Other updates follow the retained component

The focused release/registry check found this scope. Recheck relevant releases when implementing the slice; these are observations on 10 October 2026, not floating dependency constraints.

| Component | Observed version and decision |
|---|---|
| SQLite/rusqlite | Keep pinned rusqlite 0.40.2, already the latest verified release, and its bundled SQLite. Prove transactions, restart and backup; updating system SQLite does not update this bundled dependency. |
| BetterOffice and image | Keep the four BetterOffice Rust crates at 0.3.0 and image at 0.25.10, already the latest verified registry versions. BetterOffice's newer npm/React release number is not a Rust crate upgrade. |
| ZIP | Assess 8.6.0 to 9.0.1 during import qualification. The new major includes malformed-archive robustness fixes, but BetterOffice 0.3.0 requires ZIP 8. A direct pin bump alone can leave both readers in the graph. Identify actual executing readers and prove bounded malformed/oversized input handling before choosing the smallest supported change. |
| HTML tokenizer | The existing html5ever 0.27.0 can move to 0.40.1 when email/HTML intake is rebuilt, if its direct tokenizer is still useful. Adapt the changed sink API or remove the dependency if the selected converter replaces its job. |
| Email parser | mail-parser 0.11.9 is an optional patch from 0.11.8 during intake work. Its published addition is an optional decoding feature BRN does not currently enable; no urgent benefit is established. |
| Native retrieval | FastEmbed 7.1.1 lists spelling fixes only; keep 7.1.0 for now. Its ONNX binding remains ort/ort-sys 2.0.0-rc.13, already locked by BRN. |
| GitHub checkout action | Qualify v7.0.1 from the current v6 references as a separate small CI maintenance change. Resolve and pin the verified full commit, preserving permissions, credential handling, event triggers and required check names. This does not block the core. |

Run a bounded advisory check on the retained lockfile around the selected dependency changes, using maintained `cargo-audit` tooling and recording its advisory database date/result. Trace relevant findings through the actual target/features before choosing an update. The lockfile contains optional and cross-platform packages; duplicate versions or informational maintenance notices alone do not justify a rendering fork or dependency-unification project. This preparation performed focused source checks, not a complete audit. Do not run a blanket `cargo update` or upgrade system/native tooling without a relevant requirement or failure.

Sources: [rusqlite release](https://github.com/rusqlite/rusqlite/releases/tag/v0.40.2), [BetterOffice OPC registry and ZIP constraint](https://github.com/rust-lang/crates.io-index/blob/master/be/tt/betteroffice-opc), [image registry](https://github.com/rust-lang/crates.io-index/blob/master/im/ag/image), [ZIP 9.0.1](https://github.com/zip-rs/zip2/releases/tag/v9.0.1), [HTML tokenizer API](https://github.com/servo/html5ever/blob/html5ever-v0.40.1/html5ever/src/tokenizer/interface.rs), [mail-parser changelog](https://github.com/stalwartlabs/mail-parser/blob/main/CHANGELOG.md), [FastEmbed 7.1.1](https://github.com/anush008/fastembed-rs/releases/tag/v7.1.1), [checkout 7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1), [cargo-audit](https://github.com/rustsec/rustsec/blob/main/cargo-audit/README.md).

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


## Candidate execution update, 11 October 2026

M1 is committed at `ca0342bc0437af293b26cf0b891bc14c85227875`; its archived committed core passed38 tests. M2 and M3 implementation now use the new shared service/runtime and native views; the old store/workflow and coupled entry points are removed. Local M4 headless/provider/import/native-build qualification passed; candidate source `1f7e7fd80111cb29d2fb562fb2c548b685721093`; see [evidence](evidence.md) for results. The UI uses Kit0.7.1/GPUI0.3.8. Rig0.44 is blocked by demonstrated malformed-stream continuation, so0.43 plus its existing logging patch stays. Rust1.99 qualification against the original lock is recorded, but the pin remains1.98.1 until the planned interaction gate. No broad dependency update or new engine was added.

The unsigned candidate and one pending [native acceptance task](acceptance.md) are ready. The Mac reported locked, so native interaction and owner acceptance are pending; no unlock attempted. Hosted checks, source publication and remaining limitations are recorded independently. PR #114 remains draft; no main merge or release.


Final handoff: all four required hosted Linux/Mac contexts and documentation passed on `df0cd00ef02389baf06fa010b2b15ceeeb9d3297`, with unchanged product source `1f7e7fd80111cb29d2fb562fb2c548b685721093`. The informational Windows build failed in Unix credential filesystem handling; overall CI is therefore not all-platform green. The following documentation-only handoff restarts normal CI without changing tested product source. The remaining product gate is the single native interaction/owner acceptance task, blocked by the locked Mac session. No merge/release.
