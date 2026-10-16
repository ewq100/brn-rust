# D1B source-linked semantic context execution record

**Goal:** implement existing amended Rig-first plan§5.1B/C01–C07 on accepted D1A964567aa, with canonical BRN persistence and a rebuildable untrusted semantic cache. This is its execution record, not a replacement roadmap.

**Architecture:** one frozen history window from D1A, a small Rig Compactor model adapter on the selected authenticated account/model/effort, and bounded derived cache. Calls happen before chat dispatch and outside DB transactions. Raw conversation stays canonical; no automatic Rig persistence, provider transcript or Note summary store.

**Authority:** D1B exclusive heavy slot granted once focused source preparation is ready; B1b released. Rust1.98.1 locked/offline, no shared source-root target. Focused checks precede broad gates. Direct pinned rustfmt/static inspection allowed. No delegation, live accounts/models, credentials inspection, Astra, downloads, GUI, publication or D2. L blocks live semantic qualification; do not ask again. D1A mutation budget is finalized and never reused.

## Paths and reuse

- New threads_summary.rs owns bounded schema/model-backed Compactor, cryptographic source/config coverage, safe derived cache, freshness/cancellation and one visible degradation policy.
- threads_context_budget.rs factors its existing canonical scoped rows/coarse chunk cut once; exposes evicted chunks for D1B and includes frozen summary in existing whole-request checks. Existing admission/caps/model source unchanged.
- threads.rs registers modules, carries frozen compaction plan in private PreparedThreadRun (no helper Scope change), invokes summary before first parent dispatch, reuses one authenticated resolved configuration and transport, and merges bounded notices. Writing request/allowed tools/authority unchanged.
- threads_trace.rs records bounded summary phase/outcome/usage separately from chat turns, without summary content/raw request/credential identities.
- New focused threads_summary_tests.rs and necessary cache/budget witnesses exercise production runner/HTTP, cache reuse/invalidation and failures, not implementation arithmetic mirrors.
- Exact principal manifest allocation: promote existing sha2="0.10" dev declaration into dependencies. Same version/features; expected sha2 0.10.9 existing edge/checksum; lock byte-identical after actual locked/offline verification. No other dependency/schema/core/app/CLI/UI/guide changes. Additional exact test-only allocation: append ONE derived-summary exclusion witness to app/tests/export_metadata.rs, preserving its full existing prefix; no app production/dependency edits.

Reuse: Rig0.44 Compactor::compact(conversation,evicted,carry), DynModel/AgentBuilder/stream collector/hooks; existing authentication/client config cloning and resend transport; Store::derived_directory safe boundary, existing OS-file-lock/NOFOLLOW patterns, checked live/run/settings/project fences and source predicates. R0 process-local CompactingMemory is not persisted; default TextSummary system conversion is not used.

## Bounded decisions

- Coarse D1A completed8KiB chunks drive summary coverage; recent raw window remains append-only. At most four new chunks per run, one model call, with bounded valid prior carry. Backlog is explicit and retried at later run starts; never mutate an admitted transcript.
- Structured semantic content covers objective/requirements/questions/proposals/outcomes and links to exact canonical Messages and real operation receipts. The host derives ID/version/digest/range/config stamps; model cannot mint coverage or accepted receipts. Output/body/citation/cache bounds reject rather than silently crop semantic claims.
- A per-conversation bounded cache file lives directly in core-validated derived/, excluded by existing SQLite-only backup and record-driven export/search interfaces. Corrupt/lost files cause recomputation only. No raw canonical rows/provider transcript are stored there; bounded derived claims may quote source.
- Cache key includes chat identity and actual security-transition stamp (attention alone must not invalidate), current project/settings/instruction/summarizer identity, selected provider/model/effort and account identity. Token-only routes conservatively invalidate on credential rotation without persisting credentials. Exact prefix content digests validate covered chunks, source versions and operation outcomes.
- Recheck live state and exact captured canonical change token/context/source coverage before atomic publication; serialize cache publications with a nonblocking OS file lease, never a DB transaction spanning the model. The lease serializes publication, so no older writer replaces newer coverage. Freshness is checked after staged file sync immediately before atomic rename. Every load/use revalidates current visibility/coverage. No cache grants authority.
- Clone the already authenticated provider config to unpinned summary transport and pinned chat transport. One auth/account resolution; summary has no chat prompt_cache_key or pinned chat session; Rig draws its own fresh session header. Preserve provider route, effort, transient retries and wire output policy (reserve remains estimator-only).
- Render summary only inside explicitly labelled UNTRUSTED DERIVED user context, never as system/owner instruction, settings/skill/protected grant, current reference read or canonical Note.
- One failure policy: retain valid prior coverage plus the frozen bounded raw context/current mandatory request; disclose missing summary coverage through activity/notices/trace and input metadata. No hidden alternate runtime/model. Whole-request guard includes the final block; mandatory overflow stops before provider dispatch.

## Preparation and later acceptance

- [x] Verify bound branch/base and accepted D1A/Opus4; preserve refs and old mutation receipts.
- [x] Inspect plan§5.1B/C01–C07, C0/R0 and pinned Rig/provider/storage seams.
- [x] Prepare model/cache/history/trace integration and meaningful scripted/HTTP fixtures.
- [x] Record granted build handoff, exact source fingerprints, empty heavy processes and synthetic TMPDIR before first Cargo.
- [x] Under explicit grant: focused/fullAI/app/CLI, strict lint/fmt and material pin; one NEW bounded D1B campaign with all setup/baseline/cleanup charged.
- [ ] Principal independent source review/disposition; mutation gaps and PDF/native/hosted/live/owner limits remain explicitly qualified.

| Acceptance | Prepared witness |
|---|---|
| C01/C03 | >60 canonical Messages, early bilingual requirement, model-backed linked structured output; final real CompletionRequest contains mandatory request and bounded summary; tiny budget zero provider dispatch |
| C02 | Cold reopen at same coverage zero repeat summary calls; next coarse chunk increment carries prior artifact; source identity unchanged and no duplicate canonical Messages |
| C04 | Actual project settings/move/knowledge-off/archive/delete/restore and source/version changes invalidate; no foreign ID/content/count leaks |
| C05 | Malicious history and model summary remain user-role derived evidence; no settings/protected/unrelated-scope authority or read-ledger promotion |
| C06 | Controllable barriers for Stop/supersession/config changes and concurrent cache publication; timeout/invalid response/corruption preserve valid prior cache/current request with visible gap; no sleeps/stress |
| C07 | Captured HTTP summary separate/no chat cache key; selected route/model/effort/same account; parent raw prefix append-only within run and stable between cuts; real reported per-call usage only |
| Existing interfaces | Real backup/restore/export/search exclude derived cache; source/helper/writing/CW2/CLI contracts retained |

Behavioral before witnesses compiled and failed0/2 at exact accepted production; candidate focused14/14 passed. Initial compiler/fixture failures are preserved. Final stronger-fixture fullAI407/0/1; core15/0/0, CLI161/0/0, app5/0/0, strict workspace all-target Clippy/fmt and helper/default builds pass. One workspace no-fail-fast invocation:1129passed/1PDF-Unavailable failure/6ignored across94raw summaries. No rerun. At pin preparation no D1B mutation had run; [final targeted campaign](mutation-results.json) subsequently records6caught/2missed with both gaps retained. Semantic quality/live is BLOCKED by L; workspace/native/hosted/owner/program acceptance remain separate gates. Verification is not source approval.
