# D1B implementation and review brief

Base: accepted local D1A `964567aa5ea41a9333bccb6790be464e0018dfa3`, bound `work/d1b-20261017`. No D1A evidence or mutation budget reopened. This implements existing amended Rig-first §5.1B/C01–C07, not D2.

## Actual seam and ownership

- `threads_summary.rs`: Rig0.44 `Compactor` contract with a typed semantic artifact; existing Rig model/stream/budget hook; bounded rebuildable cache and host validation. No automatic/process-local Rig persistence or default TextSummary conversion.
- `threads_context_budget.rs`: factors the existing canonical permitted Message/Run/receipt rows and exposes whole evicted coarse chunks. The one-time D1A memory cut, mandatory request, scope/role predicates, result caps and actual growing CompletionRequest guard remain.
- `threads.rs`: private prepared compaction input, one selected-account authentication, unpinned summary client then pinned chat transport, existing selected route/model/effort, final gap notices. No helper/source/history/read/permission implementation changes.
- `threads_trace.rs`: optional structure-only summary outcome/coverage/reported usage, separate from parent model-turn usage; trace format remains1 and existing absent-field output remains unchanged.
- New focused summary tests; EXACT allocated app test appended to `export_metadata.rs`, with its entire original prefix preserved. No app/core/CLI/UI production edits.
- EXACT manifest promotion of existing `sha2="0.10"` from dev to production. Actual locked/offline builds reuse0.10.9; Cargo.lock remains byte-identical. No version/feature/other manifest change.

## Behavior and bounds

The compactor runs before chat, outside DB transactions, with no tools. It sees untrusted evicted canonical rows plus bounded prior semantic carry. Objective, requirements, questions, proposals and actual applied outcomes are distinct JSON fields. Claims link to exact permitted Message/Run IDs and verified operation IDs. Host-owned ID/kind/version/content SHA-256, operation request hash/applied status, covered-prefix digest and first/last stamps are computed independently of model text. Invented/foreign sources, candidate-only applied claims, unknown fields and oversized artifacts fail closed.

One model call per run, at most four new completed8KiB chunks,8KiB semantic JSON,32 host source/operation references each,32KiB cache,30s timeout and16KiB collected response bound. These are host bounds, not provider token limits or wire output caps. Backlog and every compactor/cache failure use ONE policy: valid prior coverage plus frozen bounded recent/current mandatory context, with explicit gap through input/activity/notices/trace. No claim that all history was considered. Full v2 writing input is unchanged; D1A's mandatory writing path does not compact it.

Per-chat derived files use core-validated `derived/`, Unix NOFOLLOW/private file creation, regular-file checks, a nonblocking OS publication lease and atomic staged replacement. Fresh canonical generation, current visibility/config/source prefix and run liveness/cancellation are checked after staged file sync immediately before rename. No DB transaction spans a model call. Every cache reuse/exposure revalidates current key and exact source bytes. Concurrent busy writers retain valid prior coverage and disclose a gap. Corrupt/lost cache is recomputable; canonical Messages/receipts/assets are never replaced or duplicated.

Keys bind chat security transitions (including revoke/restore, excluding attention-only churn), current project/settings, guide/summarizer instructions, selected provider/model/effort and resolved authenticated account/endpoint. Corrected at a94b9c99: stable ChatGPT account identity and the checked Copilot bootstrap fingerprint preserve reuse across routine exchanged-token rotation; token-digest fallback applies only when ChatGPT has no account ID. Missing or unstable identity disables optional reuse conservatively. Credentials/account names are not stored. The same already resolved client config serves summary and chat. Summary has no chat prompt-cache key and uses Rig's fresh unpinned session header; chat keeps the existing stable key/session and resend policy. No output cap, alternate model, secondary account or hidden runtime was added.

Rendered summary remains explicitly UNTRUSTED DERIVED USER context, never system guidance, fresh owner/protected/settings/skill authority, Note read evidence or canonical Note. It is frozen before the chat stream; admitted transcript prefixes are never rewritten. Existing actual whole-request budgeting includes the final summary. Provider limits/heuristic estimate distinctions are unchanged from accepted D1A.

## Actual evidence and limits

[Before receipt](before-receipt.json) verifies exact accepted production plus test-only registration/witnesses. The two witnesses compile, then fail behaviorally0/2 for missing separate model call and missing derived cache. New-feature absence is not a compiler failure. Before source/target are separate from candidate.

Fourteen focused production/model/HTTP scenarios pass. The final fixture uses68 canonical Messages with an early-only bilingual requirement; the initial72-message cold fixture crossed a real coarse cut3→4 and its failed receipt is retained. Source edits move a Message in receipt order; an initial invalid fixture citation was rejected correctly and corrected to cite actual remaining coverage. Compiler errors, the incorrect cache-key expectation, a no-op config-change fixture and initial lint failure remain in raw receipts.

Latest full AI scripted407/0/1; core note_read/message_search_scope15/0/0; full CLI161/0/0; app export target5/0/0; strict workspace all-target Clippy and workspace fmt pass. Helper/default workspace builds and offline capability fixtures pass. One workspace no-fail-fast invocation exits101:1129passed/1failed/6ignored across94raw summaries, sole failure is existing PDF preparation Unavailable (pdftoppm absent). No skip/rerun. See [self-check receipts](self-checks.json) for exact commands, hashes and source fingerprints.

The app marker test is explicitly a filesystem exclusion fixture, not model-generation/cold-reuse evidence: marker exists in the actual derived cache area, export/backup restore/current+restored search exclude it, a distinct canonical Note remains exported/restored/searchable, and deleting cache leaves canonical records unchanged. Model generation/reuse is demonstrated separately through real scripted HTTP paths.

No live semantic superiority/cache-performance claim. Usage numbers are actual fixture-supplied reports, not measured live caching. L blocks live semantic qualification; native/hosted/owner/program acceptance and integration remain principal gates. This source is not self-approved. Independent principal-owned Opus review and the separate new D1B targeted mutation campaign remain pending at pin preparation.
