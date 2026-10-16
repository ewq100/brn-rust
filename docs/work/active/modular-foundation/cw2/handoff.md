# CW2 source checkpoint for independent review

This is a bounded implementation checkpoint, not acceptance. Base is exact local integration `3e09b979dbe5204098d871469d0ed280186db20d`; branch `work/cw2-20261016`, isolated `t3-3205580d`. The adjacent `path-ownership.json` hashes every changed crate path, including new files. `69d5df82` is the original core failing-first test commit. Reviewed widget correction `3cdece37` is separately cherry-picked as `aea4830f`; its two original test changes are not CW2 production scope.

## Contracts and seams

The authoritative master §§7.2–7.3, §10 CW2 and §13 remain unchanged. CW1 setup/schema/v1 saved bytes are not migrated. New request options are omitted at default. Existing current request/working Note/comments/setup bindings remain; actual read versions become Historical unless exhaustive requirements or owner must-current marks require Current. Read/search tools use canonical reference scope and the existing core predicate; currently unavailable selected references are removed from the model clone before content delivery. The original saved snapshot retains its identity.

Existing ReadLedger owns actual coverage. Exact retained bytes, whole/partial reads and host-verified quotations are persisted atomically in a reserved scoped evidence Message within the immutable candidate request. Top-k normalized snippets bind the version with zero exact retained bytes; they cannot prove full reading or quotes. Evidence has strict mandatory fields, a 128 KiB bound and exact request/thread/target/version binding. Model input lists supply no authority. Replacement retains evidence and input binding. Model explanation is stored scoped, labelled untrusted in review; generic v2 operation/receipt reason is host-owned public text. Existing v1 reasons remain unchanged.

Core actual Apply rechecks every writing input, including Historical, against existing `context::visible` in the correct canonical scope. It runs inside the checked transaction. Plain safe Undo restores pre-AI content after evidence revocation. Even-depth compensation that re-applies candidate content follows original request provenance and rechecks visibility; replay returns the original receipt without new writes. Restoration requires another explicit exact owner Apply. No persisted approval or retry bypass is added.

App resolves `(working Note, canonical displayed scope)` independently of selected Chat. Explicit owner conversation choices and requests durably record last use; no first-linked fallback. Closed Chat/restart use the same resolver. Failed opening leaves old Note/context intact. Shared-Note comment visibility remains unchanged; its product privacy question remains OPEN.

App `prepare_thread_run` composes settings, typed Selection/Preparation errors and search wiring around existing AI `prepare_run(ThreadRunRequest) -> PreparedThreadRun`. AI `spawn_prepared` consumes the same single-use identity and calls `run_prepared`; CLI uses this preparation too. UI event/cancellation/lease/run/context fences remain. No new driver, schema, crate, dependency or architecture framework.

## Exact desktop allocations and traced surfaces

- `threads_ui.rs`: start_run; active-context open/explicit selection/new/ensure and storage field; open_review new-style implicit-context-switch fence; existing single apply_candidate handler uses context-aware checked Apply and exact displayed Note. Programmatic set_chat cannot record an owner selection. No B1b extraction.
- `threads_projects.rs`: only open_project_note passes explicit opening Project before choosing context, preserving failed-open and dirty-review returns.
- `threads_writing.rs`: runtime context lookups; review_body safe early return before any private reason/MessageJSON/Inputs/text; mandatory named disclosure outside collapsed Inputs; compare_tile same checked result before versions/headings/text; note_bar candidate-presence expression only.
- `threads_workbench.rs`: writing_proposals filters before feedback/count/newest/summary; latest_note_writing_request uses shared context/request check before Saved inputs, including pre-candidate v2; writing_bar_row/writing_run_outcome/writing_result_note candidate presence filtered before state/suppression; writing_worker_for filters display eligibility only, retaining exact Note/request/run identity and actual worker state.
- ONE added native test in threads_writing_tests covers default disclosure, wrong-scope expanded Inputs/Result/Compare/proposal feedback/Saved inputs/status, no context overwrite after close, access loss refusal, restoration and exact Apply. Existing tests/assertions and both reviewed reduced-motion calls remain intact. No components or unrelated desktop test edits. The subsequently granted mechanical callers below are the only additional existing desktop test hunks.

Read-only trace: note_tile Result and review body delegate to gated review_body. Compare had a separate direct candidate_view and now has the same app gate. The only proposal_card caller is attention_card in thread_detail, whose calling path excludes writing candidates before rendering. Writing bar/request/status expressions above were individually traced and allocated. Generic Home/Changes tiles show receipt.reason and affected_titles: the actual writing shape is one existing working-Note write plus a metadata Message; titles derive from that fixed Note, not the Message. Scoped model explanation is absent from generic reasons. No generic list/history UI edit was needed.

Boundary trace: reserved context/evidence Messages are excluded from generic model records/history and owner message_page; v2 request paging retains canonical scope checks. Search indexes ordinary owner/assistant Messages and Notes; metadata roles are not indexed. Markdown export exports Notes/assets, not these Messages. Privileged raw Store/owner prepared-request inspection can inspect immutable records and is not claimed to hide them. Existing source tool asset/count/cursor/cache/trace regression coverage is retained, not a new exhaustive audit of all future surfaces. Unrelated native observation and complete F21 qualification remain open.

## Acceptance mapping (original F01–F22)

| Cases | CW2 evidence / limits |
|---|---|
| F01–03 | Existing affected core/app import/operation/run regressions; no intake implementation or live provider qualification. |
| F04, F06 | writing_context service: two Project scopes, explicit second conversation, closed Chat/restart, no arbitrary link; existing writing comments/request tests. |
| F05, F15 | Editor behavior unchanged; existing native assertions retained. Human keyboard/IME observation deferred. |
| F07–08 | AI CW2 actual read/range/oversize/top-k/quote tests and app review distinguish import coverage, exact retained coverage and model requirements claims; never all-original-requirements proof. |
| F09–10 | Mandatory failing-before owner Historical visibility; corrected revocation/archive/ordinary-edit/Current/setup regression tests, native access loss refusal. |
| F11 | Existing authority/tool scope protections and scripted forged declared-input/quote tests; no model authority expansion. |
| F12, F16 | Core replay/Undo/Undo-of-Undo and app immutable candidate/replacement/reopen tests; existing recovery/run/cancel fences. |
| F13–14 | Relevant AI read/search/project/history/source regressions plus changed canonical scope/search checks. Ranking/model downloads untouched. |
| F17 | App CW2 synthetic backup/restore preserves exact scoped evidence and quotations; existing backup suites. |
| F18 | Real Workspace shared-run scripted tests with single prepared identity and fail-before-Run; same CLI preparation, no second host. B4 native host qualification not assigned here. |
| F19 | Affected full core/app/AI and native suites; four new task regressions fixed through typed concise startup errors, existing assertions unchanged. |
| F20 | OPEN: headless native behavioral tests are not human native observation. |
| F21 | Two-scope service/model/native surface witnesses and trace above; source review and full integration qualification pending. No global full-F21 claim. |
| F22 | Strict affected-target lint/native compile local; required Linux/default workspace hosted CI belongs to principal integration, unperformed here. |

## Receipts and review focus

`run.py` records exact commands/features, UTC, HEAD, tracked diff hash, every relevant Rust file hash (including then-untracked new files), task-specific target, exit, duration and log hash. It never records credentials/environment payloads. Source hashes distinguish before/corrected receipts. No shared cache was trusted.

- `before-visibility`: original test-only 69d5df82, 1 pass / 2 fail / 0 ignored; mandatory owner Historical access-loss bypass proved.
- `before-native-disclosure`: 0/1/0, missing mandatory default disclosure; actual review panel existed.
- `corrected-all-core-ai-app`: earlier source 785/0/3. `focused-source-corrected`: later source 402/0/2. Neither is relabelled final source.
- `native-expanded-regression`: 306/4 NEW task failures/1 ignored. Separate historical PDF/widget failures are not used to excuse them.
- `native-errors-corrected`: 310/0/1 at exact stored source hashes; only direct formatting followed. `lint-source-final`: strict affected all-target lint passed after formatting. Final committed-source normal checks/mutation follow separately.

Review exact immutable diff, especially candidate evidence replacement/atomicity, malformed-role no-legacy fallback, historical visibility/Undo provenance, canonical displayed scope vs actual conversation, all allocated privacy gates, generic reason vs scoped explanation, single-use shared preparation and error cleanup. Any actionable review findings require bounded fixes and affected rechecks. No self-claimed independent approval.

Heavy slot and shared threads_ui.rs/threads_writing.rs remain exclusively CW2. B1a is parked and may not edit until explicit serial handoff; this checkpoint is not that handoff. Principal owns scheduling Opus and integration. No push, PR, main merge, release, live provider, owner data or GUI was used. PDF tool failure and original widget-B attribution remain recorded limits; local corrected native results are not hosted CI.


### Typed CLI preparation follow-up

The first committed-source combined command at `d4a51409` stopped in CLI `agent_contract`: 22 passed/1 NEW failed, after 12 prior CLI agent_bounds tests passed. Shared preparation had erased `AiErrorKind::ToolRejected` into generic AppError, losing the existing machine details/default text. `RunPreparationError::Preparation` now retains the actual AiError; Settings failures retain AppError separately. The same existing CLI `contract::run_error` handles preparation and run errors. No rendered-string inference, second start path or test weakening. `cli-error-corrected` verifies the unchanged 23 CLI contract tests and 2 shared-run tests (25/0/0). The existing service invalid-budget witness now also checks the typed error and no Run side effects. This three-file follow-up supersedes the first source pin for review.


## Round-1 correction allocations and current checkpoint

The verbatim [Opus round-1 report](opus-review-round1.md) is CHANGES_REQUIRED for d4a51409. f282f7f6 remains the prior immutable pin; these corrections are not yet independently approved. The original master and acceptance cases remain the specification.

Exact new desktop hunks granted for corrections:

- threads_ui.rs private cache fields/constructor plus ONE refresh_writing_review_cache helper; threads_shell.rs Render entry ONE call. Cache key includes local total_changes/external data_version, displayed Note/canonical scope/selected conversation and exact Prepared request content/identity/hash. Any token/load/batch error clears private results; token observed before/after batch must agree. Actual Apply/read bypass this display cache and check fresh.
- Existing allocated review/request/presence gates consume one checked batch loaded from authoritative records, receipts and candidates. App batch reuses the existing predicates and rejects removed/replaced/unbound candidates; no second permission predicate or schema/index framework.
- threads_lists.rs is_background predicate ONLY: nonempty receipt writes must ALL be reserved host context Messages; mixed Note/Action/user-message receipts remain visible. Programmatic open_thread records no durable choice, explicit picker/New choices do; same-active app selection is a no-write no-op. Resolver ignores archived/undone selections.
- Mechanical dead-parameter removal ONLY: threads_chat.rs composer_chat NewForNote call and adjacent now-unused version lookup; threads_context_actions.rs call; threads_context_actions_tests.rs one call; threads_workbench_tests.rs two calls. Existing assertions/guards/data are untouched. All other edits in these four files are excluded.
- The ONE allocated native privacy regression additionally checks ten unchanged notify renders reuse the batch, exact candidate-content mutation invalidates without a store write, wrong scope and local/external revocation/restoration update on notify alone, and fresh checked Apply refuses revoked evidence.

Additional AI/app mechanical paths for existing RunNoteSearch Option<&str> scope: threads_intake.rs (two Some wrappers), threads_intake_tests.rs, threads_research_tests.rs, threads_search_tests.rs, app search.rs trait implementation, tests/host_intake.rs mock, tests/search_messages.rs two Some wrappers. Canonical None is real general scope, never an empty sentinel/selected-chat fallback. Ordinary/research/v1 intended Some contexts remain.

The original low-5 hypothesis is now a HIGH actual privacy defect with separate source-hashed failing-before direct read_records and actual runtime search-route receipts (each 0/1/0). Only host-bound snapshot.note.id gets the actual selected conversation exception; unrelated Notes use frozen canonical None/Project/private scope. A real app SearchHandle/scripted model-delivery witness asserts selected-chat count 2 versus canonical count 1, working-Note positive access, and absence of unrelated private text/title/ID from actual tool delivery and candidate evidence.

Current corrected native receipt r1-native-cache-final is 310/0/1 at its exact dirty source hash; this is headless behavioral evidence, not human observation/full F21. Focused465/0/2 is earlier affected-source evidence. Broader final verification, strict lint, final source pin and fresh independent review remain pending. Mutation original usage556.338371s, remaining1243.661629s within ONE total1800s cap; original app24caught/48missed/64unviable remains pinned f282. No core/AI mutants ran in the first controller because baseline harness setup failed. No reset, full pass, review acceptance, integration or B1a/shared-file handoff is implied.


## Immutable correction source and completed normal verification

Correction source is `144848ffd86f8705f156553aeef418de23d204a5`, crates tree `430d1746d27ec718974b96bac0dfa6845c55e025`. Principal started fresh read-only Copilot Opus round2 on that exact pin. The prior broad d4 review and f282 results are not relabelled. Numbered [round1-response](round1-response.md), all exact owned paths/hashes, source trace and original F01–F22 mapping above accompany the actual diff: f282..144848 correction; 3e09b979..144848 full CW2 foundation diff.

Exact committed-source results are indexed in `r1-pin-normal-summary.json` (commands, features, source hashes and log hashes), with executed/build artifact hashes in `r1-pin-normal-artifacts.json`:

| Gate | Passed / failed / ignored |
|---|---|
| Core + AI scripted + app + CLI libs/bins/tests | 953 / 0 / 3 |
| Native behavior, native-ui + native-test-support | 310 / 0 / 1 |
| Offline capability fixtures, all targets | 357 / 0 / 1 |
| App native-retrieval | 239 / 0 / 1 |
| Retrieval native/no assets, lib + model_download refusal fixtures | 35 / 0 / 0 |
| Combined native-ui + native-retrieval desktop state | 95 / 0 / 0 |
| Fresh/reopen actual desktop headless startup | 2 / 0 / 0 |

Native CLI/desktop combined builds, all-affected feature-union strict Clippy and actual repository `cargo fmt --all -- --check` passed. No native model was installed/downloaded; retrieval tests use synthetic/local sources and cancelled/occupied production installer refusals only. The first small headless check accidentally overlapped lint; retained as diagnostic and superseded by successful serial `r1-pin-headless-serial`. Source remained frozen. Other normal builds/tests were serial; the process check before mutation found no Cargo/rustc/mutants children.

Original mutation f282: core/AI baseline harness failures, zero mutants executed there; app136 IDs completed24caught/48missed/64unviable. All three leaked copies hash204/204 relevant Rust files to f282. `mutation-outcomes.json` retains each ID/verdict/diff/log hash/path and conservative survivor classification; no unproved equivalence or retroactive kill. Correction-pin supplement uses the original remaining1243.661629 seconds after556.338371 seconds setup/initial usage, ONE total1800 second cap. All supplemental setup is inside its controller, selected priority guards core replacement/Apply/Undo then AI scope/evidence then changed app resolver/batch. Final supplemental outcomes are pending. No new budget or complete-pass claim.

Independent round2 verdict, any required corrections, final mutation receipt and integration remain principal-owned/pending. Heavy slot and shared files remain exclusively CW2 until explicit serial release. No B1a integration/handoff is implied by this source pin.


## Cache correction normal checks and source scope

Final corrected-source receipts: `r2-core-app-complete`442/0/2; `r2-native-final-source`310/0/1; strict affected all-target `r2-strict-lint-corrected`PASS; repository `r2-format`PASS. Exact commands, features and all Rust file/diff hashes remain in receipts. Unchanged AI/CLI/capability/native-retrieval/combined portions reuse144848 receipts only with that source qualification, not relabelled new-pin runs.

Production changes since144848 are only Store::change_token's canonical watermark/exact pending keys/external commit token; its simple ViewChangeToken tuple alias/export (strict lint, no new framework/schema); the existing UI cache key's token type; and app request-batch exclusion before canonical verification. App test-only thread-local work counter has no production build/API effect. Core Apply/Undo/read/search guards are unchanged in this correction. Tests add real typing/many-Note/work-bound assertions inside the ONE allocated native regression, one app canonical verification-bound unit and three direct core invalidation/critical-edge journeys. Existing assertions and reviewed widget calls are untouched. No new writer hunk or desktop allocation.

Mutation at correction checkpoint remains total1567.9456461259397s; any final recheck controller is limited to actual232.0543538740603s remaining and records its new pin/actual verdicts separately. Full mutation/F21/hostedCI/native-human/acceptance/integration stay open. Low raw-owner compensation replacement ancestry issue remains deferred. Shared source/heavy ownership retained; B1a still parked.


## Frozen final source and completed capped rechecks

Source pin **`12b0d3296ce116e99d4dee5800f2f02edebec5ac`**, crates tree **`53bcfa293e56c04c629b5eb80fee306d955d3eae`**. `r2-source-pin.json` associates all204 Rust byte hashes from the four final normal receipts with this immutable pin and hashes56 executed/build artifacts. Receipts truthfully retain the precommit HEAD144848 plus their actual dirty diff; their source-byte association is verified rather than relabelled a committed-HEAD run. Fresh principal-owned Opus round3 is read-only on144848..12b0 and pending. Only six crate paths changed in this correction; the complete ownership map and original acceptance mapping above remain authoritative.

Final normal results remain core/app442passed/0failed/2ignored; native310/0/1; strict affected all-target Clippy and repository cargo fmt pass. Unchanged144848 feature/AI/CLI results retain their earlier pin qualification. Real typing eleven characters among100 unrelatedNote requests performs no extra checked batch; app canonical request verification is2 rather than32. Local dirty-buffer writes preserve the canonical token; canonical/candidate/context and local/external access-loss/restoration paths invalidate. The token still performs a shallow pending-operation eligibility/key scan; no global constant-time claim.

`mutation-guard-recheck.json` records the final copied12b0 recheck: core4selected/4caught and app1selected/1caught, both baselines pass, no interrupted/unexecuted cases within these five IDs. Actual core mutants are change_token's default return, workingNote scope comparison and two historical visibility operators; app mutant is the request routing comparison. Added tests are not kills absent these actual reruns. The two corresponding144848 core variants keep their original Missed verdicts; the third preference/stale-outcome survivor was not rechecked or declared equivalent.

All three campaign ledgers are separate in `mutation-outcomes.json`: originalf282 app136completed24caught/48missed/64unviable, with core/AI baseline harness failures and zero mutants there; copied144848 core31completed25caught/3missed/3unviable, AI113selected53completed16caught/8missed/29unviable with60unexecuted, app120selected45completed21caught/19missed/5unviable with75unexecuted; final12b0 five actual catches. Every actual ID retains verdict/log/diff hash and conservative survivor classification. All eight leaked copies verify204/204 relevant Rust files against their respective pin after reaping. No complete mutation pass, blanket equivalence or retroactive kill claim.

Original total budget usage is **1604.808563916944 seconds /1800**, leaving **195.191436083056 seconds unused**. This includes failed setup and every controller/supplement. No further mutation command is planned. `post-final-recheck-process.json` records no active Cargo/rustc/mutant/controller/task-target child. Source remains frozen; evidence-only commits do not change the reviewed crates tree.

Review verdict/source acceptance/integration remain principal-owned. Full F21, hosted CI, native human observation, original widget-B attribution, default PDF environment gap and deferred privileged raw compensation-replacement ancestry limit remain open. Heavy slot and shared UI files are retained until an explicit safe serial release; B1a remains parked. This receipt is not that release.
