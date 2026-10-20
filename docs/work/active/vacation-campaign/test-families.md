# Test-family audit (bounded)

Historical pilot snapshot after integrating shared Note and Review lifecycle owners, with later source-bound annotations below. Current tasks and gates live in [campaign state](campaign.json) and [the overview](../../overview.md). This is a practical family map, not a per-case retirement authorization. Historical timing evidence is explicitly labeled. Keep behavior at
one authoritative interface; add a second layer only for a distinct failure class
(native wiring, serialized bytes, persistence, OS behavior, package, or pixels).

| Family / source and interface | Distinct guarantee; consumer or lane | Action and evidence boundary |
|---|---|---|
| Checked durable writes: `crates/brn-threads-core/tests/{core,writing,edit_tokens,converge}.rs`; `crates/brn-threads-app/tests/{writing,projects,thread_lifecycle}.rs` | Version/protection/scope checks, replay, dirty guards, grouped changes, Undo, service callers across desktop/CLI/agent | **KEEP.** Store/app service is authoritative; UI mocks cannot replace persistence/replay witnesses. Check full consumer graph before retiring a case. |
| Durable recovery, backup, restore, upgrades: `crates/brn-threads-core/tests/{backup_directory,data_folder,writing}.rs`, `crates/brn-threads-core/src/restore.rs`; `crates/brn-threads-app/tests/backup_directory.rs` | Restore/reopen, data-root refusal, supported schema migrations, recovery, derived-index rebuild | **KEEP.** Current source/schema and every upgrade fixture require audit before any removal; preserve schemas 3–9 and backup-first upgrade contract. |
| Production writing lifecycle pilot: `brn-writing-ui` `NoteLifecycle`/`ReviewLifecycle`, its owner tests and independent writing host, plus `threads_writing_tests.rs`, `threads_document_tests.rs`, `threads_review_tests.rs` | Shared private owners now handle Note and candidate Review lifecycle policy/effects for desktop and independent host; Store remains authoritative for durable versions, generations, guards, replay and Undo; toolkit owns editor mechanics. Real host tests cover callbacks/wiring separately. | **INTEGRATED PILOTS; old desktop mutation routes retired.** Independent Sol source review passed for both owners. At the earlier pilot snapshot, owner/host counts were writing 44 and independent host 12 passed, desktop 309 passed/1 ignored, with the ignored shared CLI/desktop case separately passing1 using its synthetic offline fixture. The production Linux X11 Note input/recovery/Save/OS-close/reopen journey passed on committed material, exactv3 text and empty recovery. These remain pilot evidence, not full workflow or OS qualification; later desktop and hosted evidence is recorded below. No blanket test retirement follows. |
| Native desktop domain journeys: `threads_chat_tests.rs`, `threads_projects_tests.rs`, `threads_documents_tests.rs`, `threads_intake_tests.rs`, `threads_backup_tests.rs`, `threads_context_actions_tests.rs`, `threads_mcp_tests.rs`, `threads_skills_tests.rs` | Actual callbacks/menus/dialogs plus store effects across distinct domains | **KEEP** unique real-control journeys. Classify each case against owner-level tests before considering **CONSOLIDATE**; similarly named fixtures do not prove equivalent outcomes. |
| Desktop layout/focus/presentation: `threads_focus_tests.rs`, `threads_tiled_tests.rs`, `threads_pages_tests.rs`, `threads_polish_tests.rs`, `threads_omakase_tests.rs` | Shortcut/focus traversal, tile geometry, responsive layout, visual text and action availability | **KEEP** representative rendering/focus witnesses; possible **CONSOLIDATE** only for verified duplicated geometry/presentation setup. Pixel/native behavior is a distinct Linux/macOS layer. |
| Desktop state decision tests: `threads_state.rs` cfg tests and `threads_steering_tests.rs` | Continue/recovery path, queue ownership, recorded outcome/notice decisions | **REWRITE_AT_OWNER** only when a production owner interface takes over the same decision. Preserve pure tests for consequential policy if they remain the authoritative unit seam. |
| Search/retrieval: `crates/brn-threads-app/tests/{search_worker,search_dense,retrieval_eval,user_scale}.rs`, `crates/brn-retrieval/tests/{search_index,model_download}.rs`, `crates/brn-desktop/src/threads_search.rs` | Current-state search freshness, cancellation/identity, retrieval ranking, scale, Linux installer contention | **KEEP.** Retrieval Linux failures were concrete source/CI evidence; installer is not redundant with UI search. Scale test is opt-in strict (`BRN_SCALE_STRICT=1`). |
| Xberg/helper API and restrictions: `tools/xberg-intake-helper` and adapted helper graph; `brn-threads-intake/tests/{intake_safety,document_modes}.rs` | Both intended stock/adapted helper identities, process supervision, restrictions, document modes | **KEEP** both graphs and exact package witnesses. They are separate binaries/contracts, not duplicate tests. Old reader/prepared-page/Poppler routes remain retired; do not revive. |
| Intake formats and fixtures: `crates/brn-intake/tests`, `brn-threads-intake/tests/fixtures`, `brn-threads-app/tests/{import_coverage,document_conversion_settings}.rs` | Format coverage, complete-note vs useful-info intent, asset retention and conversion gaps | **KEEP** semantic coverage and independently authored fixtures where contracts differ. Review fixture duplication by bytes/schema and consumer before consolidation. |
| Provider capability fixture orchestration: `crates/brn-ai/src/provider_fixture_runtime.rs`, `crates/brn-ai/src/provider_formats_tests.rs` | Provider transport/auth/parser and fixture-only capability execution; non-shipping orchestration | **RELOCATE OR REPLACE** only after mapping guarantees to shipped run interface; preserve HTTP bytes/parser/auth guarantees and genuinely probe-specific checks. Roadmap source snapshot reports 81 test attributes / 6,669 lines; this inventory did not recount them, and size is not deletion evidence. **CONSOLIDATE** only identical local helpers noted below. |
| Provider fixture helper definitions in `crates/brn-ai/src/provider_formats_tests.rs` | Repeated fixture setup knowledge | **CONSOLIDATED CANDIDATE** at22d5/54eb: three equivalent preamble readers and two identical definition readers now share two parent test helpers. Every caller/assertion/case remains; focused4/7/12 plus strict package lint/format pass. [Receipts](provider-fixture-helpers.md). Similar closed-helper cases assert distinct fields and remain separate. Hosted integration pending. |
| Notice ownership: `threads_notice.rs`, retained steering/control tests | Text/severity/provenance, remembered completion, dismiss/restore and newer/busy refusal | **MERGED in PR165** (notice source `36bf2718`; later integrated reference-owner source `ed859aa4`). The earlier [notice qualification and retirement mapping](notice-state.md) records its two owner sequences, retained GPUI cases, native317/0/1 and strict lint/build. Current local source `ed859aa4` passed desktop321/0/1, strict lint/format and production native-ui build; see the [reference-import qualification](reference-import.md). PR165's four required exact-head contexts and Ubuntu Native UI passed, as did its resulting-main required contexts and Ubuntu UI; PR166's four required exact-head contexts and Ubuntu Native UI passed. Their detailed check identities are in [PR165](evidence/pr165-integration.json) and [PR166](evidence/pr166-integration.json). PR166 resulting-main workflow `39220505231` passed all four required jobs plus docs/Ubuntu UI/retrieval; optional Windows failures remain explicit; Mac interaction and other native/OS qualification remain separate. Only the sixth unsupported direct-write fallback arm was retired with its production fallback; five actual outcome tones and real control assertions remain. |
| Chat/run/candidate coordination: `brn-threads-app/tests/{shared_run,run_recovery,killed_run_candidate,cw2_review}.rs`; desktop chat/review journeys | Run IDs, queue identity, stop/recovery, exact candidate and service boundary | **KEEP** distinct service and callback layers. Don't label duplicate by title; same saved outcome under async worker is a different guarantee from pure state choice. |
| CLI/end-to-end contracts: `scripts/verify-end-to-end.sh`, fixtures; `crates/brn/tests` | Fresh startup, documented JSON route, export/backup and foreign-data refusal | **KEEP** as cross-process durable behavior; fixture-only scripted provider lane is not a live-provider claim. |

## CI graph and cost evidence

The [20 October two-run timing assessment](evidence/ci-cost-triage.md) and [timing receipt](evidence/ci-cost-triage.json) provide newer measured observations. They establish no safe CI/test/cache simplification; distinct required helper and feature guarantees remain. The detailed results below are earlier source-bound history.

`.github/workflows/ci.yml` carries the default workspace, helper, scripted document,
capability, native desktop and retrieval lanes. Historical roadmap snapshots remain
above and in their receipts; they are not current totals. PR165's required exact-head
checks (Core/CLI Ubuntu+macOS, Native UI macOS, Native Retrieval macOS) passed, as did
documentation and Ubuntu Native UI; its resulting-main required checks and Ubuntu
Native UI/retrieval also passed. PR166's same four required exact-head checks,
documentation and Ubuntu Native UI passed. PR166 resulting-main run39220505231 passed all four required jobs, documentation
and Ubuntu UI/retrieval; optional Windows Core/UI/retrieval failed, as recorded in
the [integration receipt](evidence/pr166-integration.json). The
PR166 local desktop result was321 passed/0 failed/1 ignored; the ignored shared
CLI/desktop fixture was separately executed and passed1 at that earlier source.
The later [MCP candidate](mcp-discovery.md) passed326/0/1 at exact0f317e5e, with
focused14/14 and matching strict lint/format/production build. Its four new cases
cover render, canonical-policy and completion consequences; none was retired. The
shared CLI fixture remains ignored and was not separately executed for this slice. Current writing-owner/host
results remain 44 and 12 passed. Neither widget tests nor headless startup qualify a
real Linux window. The production X11 Note input/recovery/Save/close/reopen journey
passed on committed material with exactv3 text and empty recovery. The 25-domain inventory remains
provisional: 2 implemented lifecycle pilots and 23 unqualified domains, with 0
domains fully qualified under the campaign's full acceptance standard; the OS-route
denominator is incomplete. Astra independently source-reviewed the Linux helper
filter; the current stock and adapted suites each passed 13 tests. The final filter
correction is committed in8c951d64.
Read `docs/development/verification.md` for explicit storage, end-to-end, scale and
skill-package commands. Avoid running both full local verification scripts after
each small change; use the narrow command matching the risk, then required
integrated CI.

Historical only: roadmap PR160 run `39154786381` reports Mac Core/CLI 25m55s,
workspace tests 6m35s (4m44s compilation, about 107s summed bodies); both helper
test/lint graphs 6m23s (1.66s test bodies); capability fixtures 2m16s (1m58s
compilation, 17.34s bodies). Helper preparation across three concurrent Mac lanes
was 8m04s, 4m44s, 5m20s (18m08s aggregate, not wall-time savings). PR161 receipt
reports Mac Core/CLI 18m14s. Different commit/cache conditions mean no measured
improvement can be claimed. Before new comparison record commit, toolchain, feature,
platform, cache condition, exact invocation, compile/check duration, body duration,
and repeated invocations separately.

No blanket deletion, coverage target, test-count target, or “fewer tests” objective
follows from this inventory. For each eventual deletion, record former guarantee,
obsolete route, replacement witness and exact feature/consumer checks. Preserve
required gate meaning and historical evidence. The bounded worker/notice/reference
retirement audit selected no further reduction: pending/disconnect, FIFO and terminal
budget edges are distinct worker guarantees; notice lifecycle/provenance tests and
real-control assertions cover separate layers; reference tests retain distinct
completion and preservation effects. No duplicated setup with a measurable retirement
benefit was established in that slice.
