# Test-family audit (bounded)

Snapshot: current vacation-campaign pilot after integrating shared Note and Review
lifecycle owners. This is a practical family map, not a per-case retirement
authorization. Historical timing evidence is explicitly labeled. Keep behavior at
one authoritative interface; add a second layer only for a distinct failure class
(native wiring, serialized bytes, persistence, OS behavior, package, or pixels).

| Family / source and interface | Distinct guarantee; consumer or lane | Action and evidence boundary |
|---|---|---|
| Checked durable writes: `crates/brn-threads-core/tests/{core,writing,edit_tokens,converge}.rs`; `crates/brn-threads-app/tests/{writing,projects,thread_lifecycle}.rs` | Version/protection/scope checks, replay, dirty guards, grouped changes, Undo, service callers across desktop/CLI/agent | **KEEP.** Store/app service is authoritative; UI mocks cannot replace persistence/replay witnesses. Check full consumer graph before retiring a case. |
| Durable recovery, backup, restore, upgrades: `crates/brn-threads-core/tests/{backup_directory,data_folder,writing}.rs`, `crates/brn-threads-core/src/restore.rs`; `crates/brn-threads-app/tests/backup_directory.rs` | Restore/reopen, data-root refusal, supported schema migrations, recovery, derived-index rebuild | **KEEP.** Current source/schema and every upgrade fixture require audit before any removal; preserve schemas 3–9 and backup-first upgrade contract. |
| Production writing lifecycle pilot: `brn-writing-ui` `NoteLifecycle`/`ReviewLifecycle`, its owner tests and independent writing host, plus `threads_writing_tests.rs`, `threads_document_tests.rs`, `threads_review_tests.rs` | Shared private owners now handle Note and candidate Review lifecycle policy/effects for desktop and independent host; Store remains authoritative for durable versions, generations, guards, replay and Undo; toolkit owns editor mechanics. Real host tests cover callbacks/wiring separately. | **INTEGRATED PILOTS; old desktop mutation routes retired.** Independent Sol source review passed for both owners. Available owner/host tests: writing 44, independent host 12 passed. Final desktop result: 309 passed, 1 ignored; the ignored shared CLI/desktop case separately passed1 with its synthetic offline fixture. Final production Linux X11 Note input/recovery/Save/OS-close/reopen passed on committed material, exactv3 text and empty recovery. These are pilot evidence, not full workflow or OS qualification; no blanket test retirement follows. |
| Native desktop domain journeys: `threads_chat_tests.rs`, `threads_projects_tests.rs`, `threads_documents_tests.rs`, `threads_intake_tests.rs`, `threads_backup_tests.rs`, `threads_context_actions_tests.rs`, `threads_mcp_tests.rs`, `threads_skills_tests.rs` | Actual callbacks/menus/dialogs plus store effects across distinct domains | **KEEP** unique real-control journeys. Classify each case against owner-level tests before considering **CONSOLIDATE**; similarly named fixtures do not prove equivalent outcomes. |
| Desktop layout/focus/presentation: `threads_focus_tests.rs`, `threads_tiled_tests.rs`, `threads_pages_tests.rs`, `threads_polish_tests.rs`, `threads_omakase_tests.rs` | Shortcut/focus traversal, tile geometry, responsive layout, visual text and action availability | **KEEP** representative rendering/focus witnesses; possible **CONSOLIDATE** only for verified duplicated geometry/presentation setup. Pixel/native behavior is a distinct Linux/macOS layer. |
| Desktop state decision tests: `threads_state.rs` cfg tests and `threads_steering_tests.rs` | Continue/recovery path, queue ownership, recorded outcome/notice decisions | **REWRITE_AT_OWNER** only when a production owner interface takes over the same decision. Preserve pure tests for consequential policy if they remain the authoritative unit seam. |
| Search/retrieval: `crates/brn-threads-app/tests/{search_worker,search_dense,retrieval_eval,user_scale}.rs`, `crates/brn-retrieval/tests/{search_index,model_download}.rs`, `crates/brn-desktop/src/threads_search.rs` | Current-state search freshness, cancellation/identity, retrieval ranking, scale, Linux installer contention | **KEEP.** Retrieval Linux failures were concrete source/CI evidence; installer is not redundant with UI search. Scale test is opt-in strict (`BRN_SCALE_STRICT=1`). |
| Xberg/helper API and restrictions: `tools/xberg-intake-helper` and adapted helper graph; `brn-threads-intake/tests/{intake_safety,document_modes}.rs` | Both intended stock/adapted helper identities, process supervision, restrictions, document modes | **KEEP** both graphs and exact package witnesses. They are separate binaries/contracts, not duplicate tests. Old reader/prepared-page/Poppler routes remain retired; do not revive. |
| Intake formats and fixtures: `crates/brn-intake/tests`, `brn-threads-intake/tests/fixtures`, `brn-threads-app/tests/{import_coverage,document_conversion_settings}.rs` | Format coverage, complete-note vs useful-info intent, asset retention and conversion gaps | **KEEP** semantic coverage and independently authored fixtures where contracts differ. Review fixture duplication by bytes/schema and consumer before consolidation. |
| Provider capability fixture orchestration: `crates/brn-ai/src/provider_fixture_runtime.rs`, `crates/brn-ai/src/provider_formats_tests.rs` | Provider transport/auth/parser and fixture-only capability execution; non-shipping orchestration | **RELOCATE OR REPLACE** only after mapping guarantees to shipped run interface; preserve HTTP bytes/parser/auth guarantees and genuinely probe-specific checks. Roadmap source snapshot reports 81 test attributes / 6,669 lines; this inventory did not recount them, and size is not deletion evidence. **CONSOLIDATE** only identical local helpers noted below. |
| Provider fixture helper definitions in `crates/brn-ai/src/provider_formats_tests.rs` | Repeated fixture setup knowledge | **CONSOLIDATE** identical preamble helpers at lines 4315/5039 and definition helpers at 4993/5682 (roadmap audit). Similar closed-helper cases assert different required fields, so keep those distinctions. Reconfirm line/content identity at edit time. |
| Notice text/tone association in desktop state tests | Whether a direct overwrite inherits an old tone | **RETIRE candidate, conditional.** Roadmap reports `notice_tone_for` comparison and production notice setters with some direct clears remaining. First close split ownership/remove fallback; then retire only sixth direct-overwrite case in `outcome_tones_and_a_direct_overwrite_never_inherit_an_older_tone`; keep five outcome cases, move two direct-write fixtures to retained operation. This repair is optional, not prerequisite. |
| Chat/run/candidate coordination: `brn-threads-app/tests/{shared_run,run_recovery,killed_run_candidate,cw2_review}.rs`; desktop chat/review journeys | Run IDs, queue identity, stop/recovery, exact candidate and service boundary | **KEEP** distinct service and callback layers. Don't label duplicate by title; same saved outcome under async worker is a different guarantee from pure state choice. |
| CLI/end-to-end contracts: `scripts/verify-end-to-end.sh`, fixtures; `crates/brn/tests` | Fresh startup, documented JSON route, export/backup and foreign-data refusal | **KEEP** as cross-process durable behavior; fixture-only scripted provider lane is not a live-provider claim. |

## CI graph and cost evidence

`.github/workflows/ci.yml` carries the default workspace, helper, scripted document,
capability, native desktop and retrieval lanes. The merged `main` workflow includes
Windows native UI. Current branch workflow adds Ubuntu native UI alongside macOS to the PR matrix; the
retrieval PR lane remains macOS-only. Ubuntu widget tests have run locally, but that
is not a hosted pass. The final local desktop result is309 passed and1 ignored; the ignored shared
CLI/desktop fixture was executed separately and passed1. The current available writing-owner/host results are 44 and 12
passed. Record the exact hosted Ubuntu and macOS checks before claiming PR CI
qualification. Neither widget tests nor headless startup qualify a real Linux
window. The final production X11 Note input/recovery/Save/close/reopen journey passed on
committed material with exactv3 text and empty recovery. The 25-domain inventory remains
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
required gate meaning and historical evidence.
