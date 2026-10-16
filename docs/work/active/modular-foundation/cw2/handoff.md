# CW2 source checkpoint for independent review

This is a bounded implementation checkpoint, not acceptance. Base is exact local integration `3e09b979dbe5204098d871469d0ed280186db20d`; branch `work/cw2-20261016`, isolated `t3-3205580d`. The adjacent `path-ownership.json` hashes every changed crate path, including new files. `69d5df82` is the original core failing-first test commit. Reviewed widget correction `3cdece37` is separately cherry-picked as `aea4830f`; its two original test changes are not CW2 production scope.

## Contracts and seams

The authoritative master §§7.2–7.3, §10 CW2 and §13 remain unchanged. CW1 setup/schema/v1 saved bytes are not migrated. New request options are omitted at default. Existing current request/working Note/comments/setup bindings remain; actual read versions become Historical unless exhaustive requirements or owner must-current marks require Current. Read/search tools use canonical reference scope and the existing core predicate; currently unavailable selected references are removed from the model clone before content delivery. The original saved snapshot retains its identity.

Existing ReadLedger owns actual coverage. Exact retained bytes, whole/partial reads and host-verified quotations are persisted atomically in a reserved scoped evidence Message within the immutable candidate request. Top-k normalized snippets bind the version with zero exact retained bytes; they cannot prove full reading or quotes. Evidence has strict mandatory fields, a 128 KiB bound and exact request/thread/target/version binding. Model input lists supply no authority. Replacement retains evidence and input binding. Model explanation is stored scoped, labelled untrusted in review; generic v2 operation/receipt reason is host-owned public text. Existing v1 reasons remain unchanged.

Core actual Apply rechecks every writing input, including Historical, against existing `context::visible` in the correct canonical scope. It runs inside the checked transaction. Undo follows original request provenance, including Undo-of-Undo; replay returns the original receipt without new writes. Restoration requires another explicit exact owner Apply. No persisted approval or retry bypass is added.

App resolves `(working Note, canonical displayed scope)` independently of selected Chat. Explicit owner conversation choices and requests durably record last use; no first-linked fallback. Closed Chat/restart use the same resolver. Failed opening leaves old Note/context intact. Shared-Note comment visibility remains unchanged; its product privacy question remains OPEN.

App `prepare_thread_run` composes settings, typed Selection/Preparation errors and search wiring around existing AI `prepare_run(ThreadRunRequest) -> PreparedThreadRun`. AI `spawn_prepared` consumes the same single-use identity and calls `run_prepared`; CLI uses this preparation too. UI event/cancellation/lease/run/context fences remain. No new driver, schema, crate, dependency or architecture framework.

## Exact desktop allocations and traced surfaces

- `threads_ui.rs`: start_run; active-context open/explicit selection/new/ensure and storage field; open_review new-style implicit-context-switch fence; existing single apply_candidate handler uses context-aware checked Apply and exact displayed Note. Programmatic set_chat cannot record an owner selection. No B1b extraction.
- `threads_projects.rs`: only open_project_note passes explicit opening Project before choosing context, preserving failed-open and dirty-review returns.
- `threads_writing.rs`: runtime context lookups; review_body safe early return before any private reason/MessageJSON/Inputs/text; mandatory named disclosure outside collapsed Inputs; compare_tile same checked result before versions/headings/text; note_bar candidate-presence expression only.
- `threads_workbench.rs`: writing_proposals filters before feedback/count/newest/summary; latest_note_writing_request uses shared context/request check before Saved inputs, including pre-candidate v2; writing_bar_row/writing_run_outcome/writing_result_note candidate presence filtered before state/suppression; writing_worker_for filters display eligibility only, retaining exact Note/request/run identity and actual worker state.
- ONE added native test in threads_writing_tests covers default disclosure, wrong-scope expanded Inputs/Result/Compare/proposal feedback/Saved inputs/status, no context overwrite after close, access loss refusal, restoration and exact Apply. Existing tests/assertions and both reviewed reduced-motion calls remain intact. No components or unrelated desktop test edits.

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
