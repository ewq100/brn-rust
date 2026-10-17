**Principal D3a accepted locally, 17 October 2026:** [Disposition](d3a/principal-disposition.md) accepts source9b8acf66 after fresh Opus3 approval and final affected852/0/2 plus strict lint/fmt. D3b may start the existing runtime/frozen catalogue/settings scope. S01–S04 and production retention/recovery reconciliation remain D3b acceptance gates. D-S/D-M already approved; D4 waits D3b, external qualification remains separate.

**D3b current checkpoint, 17 October:** [Round 1 correction scope](d3b/round1-correction-scope.md) implements the principal-accepted native inventory and inactive-Project deselection fixes with genuine before witnesses; normal qualification has passed; [fresh corrected brief](d3b/round1-corrected-review-brief.md) records the actual source-associated lanes. [Owner L is approved](owner-live-qualification-20261017.md); source6ff first live window completed4starts with a candidate preparation gap, without whole-program semantic acceptance. Original mutation900s is closed at898.433739s (3caught/1compilation-timeout-no-verdict/3unexecuted). Fresh corrected pin/Opus2 and principal acceptance remain pending; D4 remains blocked.


**Current owner decisions, 17 October 2026:** [D-S and D-M explicitly APPROVED](owner-decisions-20261017.md), with dated canonical target amendments. D3a parse/validate/install/list/remove is now eligible from locally accepted D2 fd4e6a36, followed by D3b then D4. Earlier unresolved-gate status is historical. Strict bounded parser/no dependency remains C0 choice; live qualification/real data and publication/main merge stay unapproved. Source/test work and independent review remain required.

**D2 qualified checkpoint, 17 October:** Opus1 approved d252 source-only; [low follow-up handoff](d2/qualification-handoff.md) pins ca2bd94b with unchanged approved logic, description/test/doc precision, single1/0/D2focused11/0/fullAI430/0/1 and affected lint/fmtPASS. Fixed supplement3caught/1NO VERDICT is source-bound; principal acceptance/integration remain separate. This supersedes older pending-review/ungranted mutation checkpoints below.

**D2 implementation checkpoint, 17 October:** [D2 review brief](d2/implementation.md) and [actual source-bound checks](d2/self-checks.json) record the remaining bounded continuation delta on accepted8ae8a729: AI430/0/1, core31/0/0, app13/0/0, CLI161/0/0 and strict affected lint/fmt pass. Material pin `d252a505` is frozen for principal review; [handoff](d2/final-handoff.md) explicitly releases heavy with an empty process receipt. Independent review/source acceptance remain pending. No D2 mutation is granted or started; prior D1B budget is closed. This supersedes the dated source-only/heavy-owner status below.

# Agent capabilities Slice 0 implementation plan

**Principal disposition, 17 October:** D2 source d252a505 is independently approved; qualification ca2bd94b and [local contract acceptance](d2/principal-disposition.md) preserve the approved logic. Actual guard outcomes are3caught/1NO VERDICT at242.189064/300s, closed. B4 owns heavy; D-S/D-M remain owner gates for D3/D4. INT-D and broader qualification are separate. This supersedes earlier slice checkpoints below.

**Principal disposition, 17 October:** D1B source a94b9c99 has Opus2 approval and [local contract acceptance](d1b/principal-disposition.md), preserving all qualification limits. D2 may prepare the remaining continuation delta source-only from the local acceptance commit. B3 owns the sole heavy slot; D-S/D-M remain unresolved. This supersedes the dated current-slice checkpoint below.

**Current slice:** D1A remains accepted at `964567aa`. D1B corrected source a94b9c99 is frozen for fresh principal Opus2; [handoff](d1b/review1-final-handoff.md) records AI419/0/1, core15/0/0, CLI161/0/0, export5/0/0, affected lint/fmt and six actual targeted supplement catches. Original mutation verdicts remain source-bound; cumulative827.261617/1800s, heavy explicitly released. Independent approval/principal disposition/live qualification remain separate.

**Goal:** prove minimal upstream reuse and choose bounded adapters, without shipping D features.
**Architecture:** apply Rig policies explicitly to BRN-owned history, retain checked progress/operations and add narrowly scoped adapters only in their assigned slices.
**Stack:** Rig 0.44.0 and Rust 1.98.1; experiment owns its manifest/lock/target.
**Spec:** [amended Rig-first plan](proposal/amended/BRN_Rig_First_Implementation_Plan.md), especially §4/§9, subject to [build ordering](../build-sequence/README.md).

The [combined C0 plan](../modular-foundation/plan.md) owns baseline, paths, contract revision, writer/build slots and O1/CW-first order. [Evidence](evidence.md) owns proof status and exact results. No production dependency/schema/canonical behavior change is assigned here. Principal lead handles independent actual-diff review; no worker delegation.

## Capability matrix

| Capability | Already implemented | Reusable upstream | BRN adapter / deferred work |
|---|---|---|---|
| History budget | Saved Messages, fixed recent assembly, cache identity, context-limit error classification | `rig::memory::{MemoryPolicy,TokenWindowMemory,TokenCounter}`; explicit `apply` | D1A budgets **whole request**, preserves mandatory input and freezes history cut once/run at coarse compaction boundaries; estimated counters labelled |
| Older discussion | Scoped owner-message search and exact saved identities from reliability | Existing scoped search service | D1A reuse first; add bounded exact conversation read only if needed, no second search framework |
| Semantic continuity | Canonical conversation/revision source and existing derived directory | `Compactor`; `CompactingMemory` process-local orchestration | D1B model-backed summary plus bounded persisted derived cache; selected model/account/effort; live qualification blocked by L |
| Work recovery | Leases/orphan recovery, five Run states, one terminal notice, prepared candidate review, queue fences, receipts/replay, `withdraw_proposal`, `prepare_run` | `AgentRun` disposable sans-IO state and existing hook-aware `Agent::resume` | D2 only delta: objective/remaining progress and truthful budget-limited continuation via attention/fresh invocation, no new job store/Run variant without witness |
| Skills | Four allowlisted packaged skills, `read_skill`, bundle identity | Existing files/tool/schema | D3 gated D-S: strict metadata subset, frozen catalogue/digest, package containment and owner installation; no executable skills |
| MCP | No outbound connection | `rig::tool::rmcp::{McpTool,McpClientHandler}`, dynamic tools, rmcp 2.2.0 | D4 gated D-M: exactly one HTTP transport, owner-approved destination/tools/schema/scope, payload limits and revoke/Stop gates; `readOnlyHint` insufficient |
| Authority | Shared checked core/app and host scope; exact review and Undo | Existing primitives, not AgentRun serialization | CW2 precedes D; no raw histories in canonical state/backups, no inherited grant on Continue |

## Four decisions

1. **Limit source:** prefer selected-route validated provider metadata (`rig::model::ModelInfo.context_length: Option<u32>`); where absent, a small model/route table with primary-source URL, retrieval date and test fixture. Baseline BRN has no context-window source. D1A must establish actual numeric entries for selected supported routes before enforcing them; C0 invents none and makes no credentialed model-list call. Unknown limits remain explicitly unknown and provider `context_length_exceeded` is the truthful backstop. No universal window; `HeuristicTokenCounter` is an estimate and cannot prove the total request fits. Source/API review dated 16 October 2026 in [evidence](evidence.md).
2. **Summary storage:** bounded rebuildable cache under existing `derived/`, excluded from search/export/backup. Key conversation, exact covered message range/content digest, visibility epoch and summarizer/instruction configuration; check generation again before publication. Rig's summary/watermark is process-local, so persistence needs a BRN adapter. Lost cache permits recomputation, never loss/duplication of canonical Messages. C02/C04 must pass in D1B; no reason found to change schema 6. Summary is untrusted derived context, not default `TextSummary` system authority.
3. **MCP transport:** `rmcp =2.2.0`, direct feature `transport-streamable-http-client-reqwest` for eventual production; one maintained HTTP transport, no child-process spawning. C0 uses in-process duplex (`transport-io` test-only) to prove adapter semantics, not HTTP network qualification. HTTP localhost qualification remains D4 after D-M. The pinned in-process probe observes local abort sending no request cancellation; disconnect retires tools while the handler remains pending. D4/M04 must qualify supervised explicit request cancellation and local result fencing; neither drop nor disconnect establishes that remote work stopped or was undone. Record full graph delta, including duplicate reqwest/schemars, in experiment report. No root dependency change.
4. **Skill parser:** strict documented frontmatter subset: bounded UTF-8 manifest (64 KiB file, 8 KiB header), exactly one single-line `name` (`[a-z][a-z0-9-]{0,63}`) and `description` (1–1024 UTF-8 bytes), use literal `key: value` lines between `---` delimiters, reject unknown/duplicate keys, empty/control-bearing values, leading YAML sigils/quotes, multiline YAML, tags/aliases/anchors, nested structure and unsupported package requirements. This is **not YAML**. Extend existing `threads_guides.rs` validation to that subset in D3, preserving built-in IDs; no parser dependency and **zero lockfile delta**. Full YAML interoperability is deferred. S02 includes malformed/oversized, collision and traversal fixtures. D-S still required.

## Slice acceptance and stop gates

Exact meanings remain in [§9 registry](proposal/amended/BRN_Rig_First_Implementation_Plan.md#9-acceptance-registry); C0 proof of an API is not these production acceptance passes.

| Slice / owner | IDs / required evidence |
|---|---|
| D1A/D1B, C lead serial after CW2 | C01 early requirement >60 Messages; C02 cold restart/cache; C03 whole budget/limit errors; C04 visibility invalidation; C05 injection; C06 compactor concurrency/failure/Stop; C07 prefix/cache metrics |
| D2, C lead after D1B | R01 receipts around crash; R02 stale authority; R03 stopped/queued old callbacks; R04 budget/objective continuation |
| D3, C lead only after D-S | S01 actual install/load/reopen; S02 manifest/containment rejection; S03 frozen update/revoke; S04 project scope/injection |
| D4, C lead only after D-M | M01 real adapter/disable; M02 allowlist/schema refresh; M03 zero private egress without grant; M04 failure/cancel/bounds; M05 injection/canaries |
| INT-D, lead | I01 combined real-provider journey separately authorized by L; I02 existing product/CLI regression and required lanes |

Review focuses: mandatory input exceeding budget; stale summary publication after revoke; raw checkpoint retention of private payloads; tools-list refresh expanding egress; skill catalogue version changes mid-run. Their tests belong in C03/C06/R02/M02/S03 respectively.

- [x] Read current routes, map implemented recovery/search instead of reimplementing it.
- [x] Choose four bounded mechanisms, document their remaining qualification.
- [x] Run and report explicit memory/compactor, scripted-HTTP prefix, rmcp conversion/cancellation/refresh and checkpoint proofs (initial 4 passed; corrected 5-test proof includes actual resume and local Stop semantics; see evidence).
- [x] Copilot Opus approved corrected C0 candidate `adfab879`; principal accepted C0 exit. [Review](../modular-foundation/c0/opus-review-round2.md) and [disposition](../modular-foundation/evidence.md#principal-exit-and-cl-disposition). D slices retain their CW2 and owner-decision dependencies.

No live/provider/native qualification or owner acceptance is claimed. D-S/D-M unresolved; PC/T deferred; X retains Poppler; L prevents live comparison. Raw `AgentRun` serialization is disposable test material: use small BRN progress records plus fresh invocations in production. Rig already provides hook-aware `Agent::resume`; no custom driver is proposed. The CallTools probe observes re-execution of a pre-crash effect, settlement hooks, zero memory loads and one memory append per resume. Replaying a checkpoint can repeat both effect and append; it is not BRN receipt/authority recovery. D2 retains the bounded progress/fresh-invocation design (R01/R03).


D3a implementation checkpoint, 17 October 2026: approved D-S authorizes inactive owner package management; the [D3a receipt](d3a/README.md) maps real service/CLI/reopen and S02 guards to the existing plan. Exact principal core directory-admission allocation and stage-only backup/export exclusion are recorded. D3b runtime/frozen catalogue/enablement and D4 remain unimplemented. New source pin/review/mutation and principal acceptance are separate; normal workspace1163/2/6 and targeted README repair stay qualified.
