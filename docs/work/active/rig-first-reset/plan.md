# Rig-First Reset Implementation Plan

> **Superseded** on 2 October 2026 by the [simple Rig-based notes app specification](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md). Do not execute this plan pack; a replacement plan follows that specification's review.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when delegation is explicitly chosen. Steps use checkbox (`- [ ]`) syntax for tracking. Repository/user delegation preferences take precedence over automatic delegation.

**Goal:** Replace Codex App Server with two qualified direct Rig subscriptions while delivering current Markdown notes, SQLite retrieval, durable conversations and separate AI revision candidates on macOS.

**Architecture:** Desktop and CLI cross the same workflow interface. Rig owns agent/model execution; workflow owns the single SQLite authority, capability decisions, currentness and file mutations. Platform adapters stay private; the Rig runtime exchanges acknowledged dispatch/tool events with the workflow owner rather than borrowing its store from another thread.

**Tech Stack:** Pinned Rust `1.98.1`; Rig `=0.43.0` as the first qualification candidate; existing rusqlite `=0.40.2`, FastEmbed `=7.1.0`, GPUI-kit `=0.6.6`; FTS5, sqlite-vec, Tokio, Serde, SHA-256 and UUID. Companion versions/features are selected only by Q1's recorded resolution, not presumed compatible.

**Spec:** [Approved reset specification](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md). Read both spec and this master plan before a task plan.

Status/date: User approved the reviewed/revised plan at `db18da8` on 2026-10-01. The execution-choice prompt was skipped; implementation remains unstarted pending execution confirmation. New interfaces below are proposed contracts, not existing APIs; Q1 must freeze the compiled Rig-specific seams and any necessary pin/adapter revision before downstream execution.

**Integration prerequisite:** `main@6323e53` now includes PR #13's managed-note store/workflow/CLI/native implementation and schema V6. The approval above applies to the recorded planning baseline, not a verified adaptation to those later changes. Before execution, reconcile the proposed interfaces and N1-N4 steps with existing note modules, check every affected consumer, and allocate unused migration versions for N1/A2/A4 rather than running the proposed new V6 unchanged. Reuse existing mechanisms where appropriate while preserving durable work and provenance. Record and review the adapted plan before implementation; this documentation merge changes no product code or saving guarantees.

## Global constraints

- Historical planning baseline: `19f180c80cd607b47f22473841971ffc5065ac91`; store schema version was `5`. The integration prerequisite above takes precedence over the migration numbers and assumed existing APIs below.
- Plan creation/commit is authorized. Product implementation, live authentication/calls, original-vault access, model acquisition, signing/notarization and release are not authorized by this document.
- "ChatGPT/Codex and GitHub Copilot, both directly through Rig."
- "None: no Codex App Server, Copilot CLI/SDK, alternate account or paid-API substitution."
- "macOS first." Windows interfaces are designed, not implemented or qualified.
- "Explicit, basic editor-grade Markdown saving, with bounded local recovery and documented concurrent-writer limitations."
- "Protected application-private local files using Rig auth. Keychain is deferred."
- "Rig conversation mechanics plus BRN currentness enforcement; no centralized Context Engine and no automatic summarization initially."
- Preserve the existing `1 MiB` note/editor byte limit, exact UTF-8/BOM/line endings, immutable originals, comments, evidence hashes and half-open character-boundary byte ranges.
- Keep buffers/checkpoints/candidates/current files distinct. Model output never authorizes adoption, saving or publication.
- Current eligibility is fail-closed. No archived/historical/unapproved/unresolved-save content enters normal tools, memory or evidence through an old snapshot/index.
- Preserve exclusive data-directory ownership and operation ID/payload binding. Unknown external outcomes never trigger automatic replay.
- Use explicit disposable directories and synthetic fixtures. Preserve existing data; no bulk deletion or Codex-thread migration.
- Every implementation commit includes `Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>`. Never amend without explicit permission.
- Every task includes its consumers/exhaustive matches; keep the branch buildable. Run the narrow selectors below, then `cargo check --workspace --locked` for cross-crate interface changes. Native and standalone graphs need separate checks.
- Use the app-managed isolated worktree at execution time. Do not create another checkout merely to write plans.

---

## Plan pack and reviewable deliverables

This is a staged reset, not one giant rewrite. Independent seams have separate task plans:

| Plan | Task IDs | Deliverable |
| --- | --- | --- |
| [Rig qualification](qualification.md) | Q1-Q3 | Resolved downstream graphs, protected auth, deterministic protocol/agent parity and explicitly authorized live evidence. |
| [Notes and retrieval](notes-retrieval.md) | N1-N5 | Registry, basic saves, FTS5/currentness, desktop/CLI editing and qualified local semantic/hybrid retrieval. |
| [AI workflow](ai-workflow.md) | A1-A4 | Production Rig runtime, durable conversations, complete shared ask path and validated note candidates. |
| [Retirement and distribution](release.md) | D1-D2 | Removed production sidecar/Lance dependencies and clean-machine macOS qualification. |
| [Evidence](evidence.md) | All | Actual observations and pending gates, not historical pass counts. |

Tasks correspond to PR-sized review gates; their steps are smaller test/implementation actions. Do not create/push PRs or merge merely because this plan groups tasks into PRs.

## Dependencies

```text
Q1 -> Q2 -> Q3
Q1 -> N1 -> N2 -> N3 -> N4
Q1 + Q2 + Q3 -> A1
A1 + N1 -> A2
A1 + A2 + N3 -> A3
A3 + N2 -> A4
Q1 + N3 -> N5
N4 + N5 + A4 -> D1 -> D2
```

`+` joins required inputs; it does not denote a new task. In particular, A2 consumes N1, not N3.

- Q1 fixes dependency choices shared by A1/N3/N5. N1 has no model dependency but starts after Q1 so manifest/schema ownership is settled.
- Q2 builds secure auth in the disposable harness; Q3 qualifies execution. A1 cannot claim provider qualification without both.
- N2 consumes N1's records. N3 consumes N1/N2's eligibility and reuses existing retrieval contracts.
- N4 consumes N1-N3. N5 consumes Q1/N3; it does not require live conversational providers.
- A1 consumes Q1-Q3. A2 consumes A1's selected-provider/history contracts and N1's schema extension; serialize migration commits.
- A3 consumes A1/A2/N3. A4 consumes A3/N2 and existing comment/revision primitives.
- D1 consumes N4/N5/A4. D2 consumes D1 and explicit distribution authorization.

G1 is staged to avoid a circular prerequisite: Q1 closes G1a (candidate resolution, interfaces and linked native probe), allowing the qualification harness and foundation work. A1 must close G1b on the actual root-locked desktop integration before completing; A3 cannot replace production ask without both G1 stages and G2. An isolated proxy check never closes G1b.

N4 and A1/A2 can run independently after their actual prerequisites. N5 and A3 can run independently. Avoid concurrent edits to `brn-store/src/lib.rs`, workspace manifests, `worker.rs` and CLI dispatch; land interface/migration changes sequentially.

## File map

| Path | Planned responsibility |
| --- | --- |
| `experiments/rig-qualification/{Cargo.toml,Cargo.lock,src/*,tests/*}` | Disposable candidate graph, mocked HTTP, auth, streaming/tools/extractor qualification. |
| `crates/brn-ai/src/{lib.rs,config.rs,auth.rs,credentials.rs,run.rs,history.rs,tools.rs}` | Thin production Rig module; account/cache policy, canonical Rig history encoding, acknowledged dispatch/tool bridge. |
| `crates/brn-store/src/{notes.rs,ai.rs,note_candidates.rs}` | Registry/buffer/save records; independent AI conversation/event journal; note proposal provenance. |
| `crates/brn-store/src/lib.rs` | Sequential migrations `6` (notes), `7` (AI), `8` (note candidates), schema/integrity validation and startup observations. |
| `crates/brn-workflow/src/notes/{mod.rs,files.rs,macos.rs,save.rs,eligibility.rs}` | Shared current-file operations; private platform implementation and test-only fault injection. |
| `crates/brn-workflow/src/{ai.rs,ask.rs,tools.rs,candidates.rs}` | Auth/settings, single-owner run coordinator, typed tool dispatch and exact candidate validation. |
| `crates/brn-retrieval/src/{lib.rs,sqlite.rs,embeddings.rs,native.rs}` | Derived SQLite generations, FTS5, eligible vector search, Rig local embedding adapter and RRF. |
| `crates/brn/src/cli/{mod.rs,notes.rs,ai.rs,ask.rs,status.rs,review.rs,error.rs}` | Headless parity, selected-provider configuration, current-note commands, candidacy review and stable envelope errors. |
| `crates/brn-desktop/src/{notes.rs,ai.rs,candidates.rs,native/mod.rs,native/shell/*}` | Presentation state/settings/editor/candidate UI; no SQL/provider/file operations. |
| `crates/brn-workflow/src/{lib.rs,main.rs,worker.rs,error.rs}` | Config/exports, headless driver and worker actions/outcomes wired at each feature task. |
| `scripts/{verify-end-to-end.sh,verify-desktop-shell.sh,make-macos-app.sh,test-make-macos-app.sh}` | Deterministic fixtures and sidecar-free launcher regression coverage. |
| `scripts/{verify-rig-reset.sh,package-macos.sh,test-package-macos.sh}` | Integrated non-live gates and separate explicit signed distribution workflow. |
| `docs/{architecture/*,development/*,status.md,roadmap.md,README.md}` and crate READMEs | Implemented-vs-proposed boundaries, commands, save limitations and release qualification. |

No file above is created by planning. Small extra private files are allowed to keep implementations focused; public interface changes require synchronized consumer changes and review.

Each task assumes the executor has also read the relevant crate README, [architecture/invariants](../../../architecture/invariants.md), [verification selectors](../../../development/verification.md) and [handoff workflow](../../../development/workflow.md). Preserve implemented contracts until that task's replacement is verified.

## Shared contract decisions

Task plans contain exact new interfaces. These rules resolve cross-task ownership:

1. Keep current `Document`, `Evidence`, `Profile`, `DraftStamp`, comment mapping and generation correlation. Add explicit eligible-version input to retrieval; do not move BRN permission decisions into Rig/vector SQL.
2. Use new `ai_conversations`/`ai_turns`/`ai_events` tables. Preserve old V1-V5 records for historical inspection without converting their thread state into Rig history. Do not write fake thread IDs into the old schema.
3. New `note_candidates` reference `ai_turns`, the registered note and an exact immutable approved snapshot. They are not the old "copy an arbitrary answer into a draft revision" action.
4. Serialize Rig messages in a validated versioned `HistoryBlob`; do not invent a second generic message/agent framework. Rig types remain inside brn-ai.
5. The workflow worker remains the store owner. A private bounded channel bridges Rig's async callbacks to it; the worker services these requests while a scoped Rig runtime drives the agent. Never have a Rig callback await the same worker command queue that is blocked waiting for Rig.
6. Persist dispatch intent before acknowledging network/tool dispatch. Persist tool results before returning them to Rig. Returned final text/history becomes durable only after a single workflow transaction commits the final outcome.
7. Model cancellation/drop is not confirmation of remote cancellation. Keep remote outcome, durable local status and evidence currentness as independent recorded facts.
8. Current-note dependencies exist independently of passage citations. Listing identity/title results, including empty notes, retain exact note-state/metadata dependencies through restart and history windowing.
9. Bind every ask input, including vault and retrieval profile, before replay or dispatch. Expose retained editor-baseline bytes through workflow; UI/CLI never reconstruct them from historical snapshots.

## Gate policy and execution stop conditions

| Gate | Evidence required | If it fails |
| --- | --- | --- |
| G1a candidate dependency/API | Q1 published manifests, selected lockfile, compiled hook/history/tool/stream interfaces and linked native probe tests. | Stop candidate qualification; record diagnostics and review a thin adapter or pinned fix. |
| G1b production native integration | A1 builds/tests the actual root-locked desktop with native UI/retrieval and Rig wired through workflow; record its resolved tree. Repeat after relevant dependency changes. | Keep A1 incomplete and production ask replacement blocked. Proxy/check-only evidence does not pass. |
| G2 both subscriptions | Q2/Q3 deterministic matrix plus separately authorized live completion, stream, tools, typed output and restart for each selected provider/model/account. | Keep production replacement blocked; do not select another route. |
| G3 current notes | N1-N4 exact-byte/conflict/recovery/exclusion/consumer tests, plus manual disposable native editing observations. | Do not describe Markdown-first currentness or native usability as delivered. |
| G4 retrieval | N5 frozen-fixture measurements and explicit acceptance of the comparison. | Keep LanceDB out of the deletion task; no silent semantic fallback. |
| G5 distribution | Terms/support-risk review, supported target declaration, signed/notarized clean-Mac results and both live provider paths. | Keep distributable state blocked even if deterministic implementation passed. |

Q1 may discover that the inspected release needs changes. An exact reviewed version/patch and its tests must be recorded before downstream tasks; stop and revise this plan's pin/API contracts if the candidate changes. This is an explicit investigation deliverable, not permission to guess APIs.

## Specification coverage/self-review map

| Spec requirement | Tasks |
| --- | --- |
| Direct providers, no fallback, macOS/Windows seam | Q1-Q3, A1, D1-D2 |
| Registry/Markdown authority, exact bytes/permission | N1-N4 |
| Basic save/conflict/recovery/uncertain outcomes | N2/N4 |
| Archive/history/current evidence and stale memory | N3, A2-A3 |
| Rig agent/tools/hooks and durable operation identity | Q3, A1-A3 |
| Typed revision candidates/manual adoption | A4 |
| Protected file auth/reconnect/account/redaction | Q2-Q3, A1, D2 |
| FTS5/local embeddings/sqlite-vec/RRF/benchmarks | N3/N5 |
| Sidecar/Lance retirement and packaging | D1-D2 |
| Documentation, actual evidence, no implicit account actions | Every task, D1-D2 |

Before accepting the plan, check every type/signature used below against the task that produces it, every subprocess selector against its future test file, and every source path against this baseline. Expected future failures/passes are instructions, not observations.

## Execution handoff

Status: Revised plan accepted by the user at `db18da8`; execution choice remains unconfirmed. No implementation or product verification performed.

Default execution is inline with checkpoints, respecting the repository's delegation preference. Fresh per-task subagents are an option only if the user selects delegation. In either mode, review each deliverable and stop at human/account/resource gates. Plan approval is not authorization for live calls, model downloads, signing or release.
