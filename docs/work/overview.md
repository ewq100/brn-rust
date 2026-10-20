# Development overview

**Goal:** Make BRN more dependable and easier to improve; make progress and remaining decisions easy to see.
**Owner attention:** No owner decision requested. PR183 merged; resulting-main required checks running. Draft recovery has a narrowly scoped review correction in progress; no publication/native claim. Mac batch remains18:00 Tallinn.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

## Working on

### Allow the complete Mac Core check to finish within a bounded job

Owner: lead implementation; independent Sol source review completed · State: **active** · Last observed: 2026-10-20T12:16:42.598235+00:00
Activity: PR183 normally merged as63345a8e after fresh strict protection and all four required exact-head/preceding-main checks passed. Resulting tree exactly matches passing head1e6abb60; resulting-main checks pending.
Next: Inspect all required resulting-main633 checks; retain optional failures and actual cache-save observations without claiming a benchmark.
Resource waits: Required resulting-main63345a8e checks.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete isolatedfe65b8ec; only CI timeout/comment
- **checks:** PASS complete independent source/evidence review and targeted9de54608 reconciliation; YAML structural comparison, overview, Markdown links, whitespace
- **integration:** Normal merge63345a8e verified; all four required head checks PASS; resulting-main pending
- **native:** inapplicable CI resource policy only
- **owner acceptance:** not requested; existing in-scope maintenance authority

Evidence: [docs/work/active/vacation-campaign/evidence/ci-main20f-timeout.json](active/vacation-campaign/evidence/ci-main20f-timeout.json), [.github/workflows/ci.yml](../../.github/workflows/ci.yml), [docs/work/active/vacation-campaign/evidence/mac-core-budget-review.md](active/vacation-campaign/evidence/mac-core-budget-review.md), [docs/work/active/vacation-campaign/evidence/mac-core-budget-qualification.json](active/vacation-campaign/evidence/mac-core-budget-qualification.json), [docs/work/active/vacation-campaign/evidence/ci-cache-candidate-review.md](active/vacation-campaign/evidence/ci-cache-candidate-review.md), [docs/work/active/vacation-campaign/evidence/ci-cache-reconciliation-review.md](active/vacation-campaign/evidence/ci-cache-reconciliation-review.md), [https://github.com/ewq100/brn-rust/pull/183](https://github.com/ewq100/brn-rust/pull/183), [docs/work/active/vacation-campaign/evidence/pr183-publication.json](active/vacation-campaign/evidence/pr183-publication.json), [docs/work/active/vacation-campaign/evidence/pr183-premerge.json](active/vacation-campaign/evidence/pr183-premerge.json), [docs/work/active/vacation-campaign/evidence/pr183-integration.json](active/vacation-campaign/evidence/pr183-integration.json)

Attempts:

- Python YAML module unavailable in current interpreter.: Used installed Ruby standard YAML parser without adding dependencies; reviewer independently compared parsed base/candidate. ([docs/work/active/vacation-campaign/evidence/mac-core-budget-qualification.json](active/vacation-campaign/evidence/mac-core-budget-qualification.json))

</details>

### Keep shared Rust caches without duplicate PR uploads

Owner: lead implementation; independent Sol source review completed · State: **active** · Last observed: 2026-10-20T12:16:42.598235+00:00
Activity: PR183 normally merged as63345a8e after fresh strict protection and all four required exact-head/preceding-main checks passed. Resulting tree exactly matches passing head1e6abb60; resulting-main checks pending.
Next: Inspect all required resulting-main633 checks; retain optional failures and actual cache-save observations without claiming a benchmark.
Resource waits: Required resulting-main63345a8e checks.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete isolated4b50fc96: one save-if input/comment; no helper targets/production/tests changed
- **checks:** PASS complete independent source/evidence review and targeted9de54608 reconciliation; YAML structural comparison, overview, Markdown links, whitespace
- **integration:** Normal merge63345a8e verified; all four required head checks PASS; resulting-main pending
- **native:** inapplicable CI policy only
- **owner acceptance:** not requested; authorized campaign follow-up

Evidence: [docs/work/active/vacation-campaign/ci-cache-investigation.md](active/vacation-campaign/ci-cache-investigation.md), [docs/work/active/vacation-campaign/evidence/ci-cache-investigation.json](active/vacation-campaign/evidence/ci-cache-investigation.json), [.github/actions/setup-rust/action.yml](../../.github/actions/setup-rust/action.yml), [docs/work/active/vacation-campaign/evidence/ci-cache-candidate-review.md](active/vacation-campaign/evidence/ci-cache-candidate-review.md), [docs/work/active/vacation-campaign/evidence/ci-cache-reconciliation-review.md](active/vacation-campaign/evidence/ci-cache-reconciliation-review.md), [https://github.com/ewq100/brn-rust/pull/183](https://github.com/ewq100/brn-rust/pull/183), [docs/work/active/vacation-campaign/evidence/pr183-publication.json](active/vacation-campaign/evidence/pr183-publication.json), [docs/work/active/vacation-campaign/evidence/pr183-premerge.json](active/vacation-campaign/evidence/pr183-premerge.json), [docs/work/active/vacation-campaign/evidence/pr183-integration.json](active/vacation-campaign/evidence/pr183-integration.json)

</details>

### Preserve unsent drafts when a note creates its writing conversation

Owner: lead integration; bounded Luna dismissal correction; independent Sol review · State: **active** · Last observed: 2026-10-20T12:26:12.703752+00:00
Activity: Candidate574 fixes background completion overwrite; original red witness and final339/0/1, strict lint/native build retained. Independent review confirms prior P2 fixed, finds distinct unseen ordinary error erased by recovery Dismiss. Lead validated; bounded Luna correction in progress.
Next: Prove574 Dismiss sequence fails, preserve unseen ordinary error with a tiny scoped correction, then final affected checks and independent recheck before publication.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** 574 local atomic draft transfer and scoped recovery attention implemented; final dismissal correction pending
- **checks:** 574 local gates PASS, independent review one new P2 valid/fix; no publication qualification yet
- **integration:** PR183 merged separately; draft candidate remains local until final independent review/checks and hosted gates
- **native:** not observed; synthetic toolkit test is separate from OS interaction
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/composer-draft-adoption-audit.md](active/vacation-campaign/evidence/composer-draft-adoption-audit.md), [crates/brn-desktop/src/threads_ui.rs](../../crates/brn-desktop/src/threads_ui.rs), [crates/brn-desktop/src/threads_context_actions.rs](../../crates/brn-desktop/src/threads_context_actions.rs), [docs/work/active/vacation-campaign/evidence/writing-draft-recovery-qualification.json](active/vacation-campaign/evidence/writing-draft-recovery-qualification.json), [crates/brn-threads-core/src/chat.rs](../../crates/brn-threads-core/src/chat.rs), [docs/work/active/vacation-campaign/evidence/writing-draft-recovery-review-690.md](active/vacation-campaign/evidence/writing-draft-recovery-review-690.md), [docs/work/active/vacation-campaign/evidence/writing-draft-recovery-690-qualification.json](active/vacation-campaign/evidence/writing-draft-recovery-690-qualification.json), [docs/work/active/vacation-campaign/evidence/writing-draft-recovery-review-574.md](active/vacation-campaign/evidence/writing-draft-recovery-review-574.md), [docs/work/active/vacation-campaign/evidence/writing-draft-recovery-574-qualification.json](active/vacation-campaign/evidence/writing-draft-recovery-574-qualification.json)

Attempts:

- Compiled old source failed at fresh-view exact-draft assertion twice; final witness also retains unrelated Note and Thread draft.: Selected safe atomic transfer instead of in-memory-only adoption; candidate execution and independent review remain pending. ([docs/work/active/vacation-campaign/evidence/writing-draft-recovery-qualification.json](active/vacation-campaign/evidence/writing-draft-recovery-qualification.json))
- Independent690 review found recovery warning can be replaced by asynchronous worker completion; local offline case had no worker.: Valid/fix selected. Prove actual completion path, keep unresolved recovery attention distinct from remembered work result; final candidate/review pending. ([docs/work/active/vacation-campaign/evidence/writing-draft-recovery-review-690.md](active/vacation-campaign/evidence/writing-draft-recovery-review-690.md))
- 574 independently fixes prior asynchronous warning overwrite but recovery Dismiss also clears a separate hidden Ordinary error. Original574 checks339/0/1 and native artifact retained.: Lead verified source/caller consequence; tiny scoped dismissal correction, preserve normal remembered work hide/restore. No general sticky-error redesign. ([docs/work/active/vacation-campaign/evidence/writing-draft-recovery-review-574.md](active/vacation-campaign/evidence/writing-draft-recovery-review-574.md))

</details>

## Next

None recorded.

## Needs attention

### Check saving, reopening and backup in the real Mac app

Owner: development lead; single desktop actor · State: **blocked** · Last reported: 2026-10-20T08:26:01.079451+00:00
Activity: Matching ordinary native-ui candidate build7225c208 passed and immutable executable65083ccc saved; original artifacts preserved. Live proposal ready for actual Apply/Undo.
Next: Use consolidated18:00Mac session for source-reconciled synthetic Save/reopen/backup/recovery and writing proposal controls. Confirm native access and helpers at entry.
Resource waits: Daily18:00Tallinn consolidated native actor; actual source/main must be reconciled before launch.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** existing application; no new source fix selected
- **checks:** reconcile source-bound build before native session
- **integration:** use reviewed integrated source
- **native:** pending Save/reopen, backup and unsaved-recovery journeys
- **owner acceptance:** pending; native evidence does not imply owner acceptance

Evidence: [docs/work/active/vacation-campaign/native-queue.md](active/vacation-campaign/native-queue.md), [docs/work/active/vacation-campaign/writing-account-verification.md](active/vacation-campaign/writing-account-verification.md), [docs/work/active/vacation-campaign/evidence/export-native-build-qualification.json](active/vacation-campaign/evidence/export-native-build-qualification.json)

</details>

Waiting:

- resource: Daily18:00Tallinn consolidated native actor; actual source/main must be reconciled before launch.

### Check grouped review controls on the Mac

Owner: single native actor in consolidated batch · State: **blocked** · Last reported: 2026-10-20T07:05:09.863171+00:00
Activity: MAC-08 scenario and immutable source-bound desktop/fixture binaries ready for scheduled actual control observation.
Next: Run MAC-08 during consolidated native session, then retain actual AX states and unchanged canonical data separately from source/build checks.
Depends on: review-group-fixture when Fresh offline two-group fixture passed public reopen/discovery/unchanged-state checks. (reported satisfied: true); review-group-toggle when Corrected UI source reviewed and ordinary production binary qualified. (reported satisfied: true)
Resource waits: Daily18:00Tallinn native batch; confirm actual desktop access and no competing actor.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** scenario prepared; no new product feature
- **checks:** matching source/binary/fixture must be confirmed at session start
- **integration:** prefer then-current reviewed main; candidate observations kept separate
- **native:** pending actual true/false/true pressed-state and unchanged data
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/native-queue.md](active/vacation-campaign/native-queue.md), [docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json)

</details>

Waiting:

- resource: Daily18:00Tallinn native batch; confirm actual desktop access and no competing actor.

### Allow the complete Mac Core check to finish within a bounded job

Owner: lead implementation; independent Sol source review completed

Waiting:

- resource: Required resulting-main63345a8e checks.

### Keep shared Rust caches without duplicate PR uploads

Owner: lead implementation; independent Sol source review completed

Waiting:

- resource: Required resulting-main63345a8e checks.

## Completed

### Clearer instructions and one progress view

Owner: development lead · State: **complete** · Last observed: 2026-10-20T07:12:17.611061+00:00
Activity: Progress view and instruction consolidation normally merged as PR178; all four required resulting-main checks passed.
Next: Use the concise startup route and maintain current tasks; detailed guidance loads only when relevant.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete; docs, read-only renderer and personal planning skill; runtime prompts unchanged
- **checks:** passed: local and independent review, all four required exact-head and resulting-main checks; optional Windows failures retained
- **integration:** verified normal merge1009495db414f63141d17a8f67a8bad3df087892; all four required main checks passed
- **native:** inapplicable: development documentation/view only
- **owner acceptance:** not requested; owner authorized implementation

Evidence: [docs/work/active/vacation-campaign/visibility-adoption.md](active/vacation-campaign/visibility-adoption.md), [docs/work/active/vacation-campaign/evidence/visibility-adoption-preflight.json](active/vacation-campaign/evidence/visibility-adoption-preflight.json), [https://github.com/ewq100/brn-rust/pull/177](https://github.com/ewq100/brn-rust/pull/177), [docs/work/active/vacation-campaign/instruction-audit.md](active/vacation-campaign/instruction-audit.md), [docs/work/active/vacation-campaign/evidence/visibility-automation-adoption.json](active/vacation-campaign/evidence/visibility-automation-adoption.json), [docs/work/active/vacation-campaign/evidence/writing-plans-adoption.json](active/vacation-campaign/evidence/writing-plans-adoption.json), [https://github.com/ewq100/brn-rust/pull/178](https://github.com/ewq100/brn-rust/pull/178), [docs/work/active/vacation-campaign/evidence/visibility-adoption-review.json](active/vacation-campaign/evidence/visibility-adoption-review.json), [docs/work/active/vacation-campaign/evidence/visibility-adoption-qualification.json](active/vacation-campaign/evidence/visibility-adoption-qualification.json), [docs/work/active/vacation-campaign/evidence/visibility-adoption-integration.json](active/vacation-campaign/evidence/visibility-adoption-integration.json)

Attempts:

- Preflight complete; stopped at ordinary reserve floor.: Owner explicitly approved using reserve for this adoption; implementation resumed. ([docs/work/active/vacation-campaign/evidence/visibility-adoption-preflight.json](active/vacation-campaign/evidence/visibility-adoption-preflight.json))
- Independent review found readiness was inferred from overall task completion and overlapping skill review rules.: Explicit prerequisite-condition satisfaction now controls readiness; skill review triggers and inputs aligned with governing workflow. ([scripts/tests/test_work_overview.py](../../scripts/tests/test_work_overview.py), [docs/work/active/vacation-campaign/evidence/writing-plans-adoption.json](active/vacation-campaign/evidence/writing-plans-adoption.json))

</details>

### Mac feedback choices expose their selected state

Owner: development lead · State: **complete** · Last observed: 2026-10-20T05:34:13.914672+00:00
Activity: Reviewed Mac feedback correction normally merged as PR176; all four required resulting-main checks now passed.
Next: Reuse qualified source; broader native/owner usability remains separate.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete; final writing.rs SHA b14e8bc9…932ffe8
- **checks:** all four required exact-head checks passed at 7e41da52; optional Windows Core failed
- **integration:** verified normal merge1ea0ac875f6e7aa128ce21bde781075e87784fff; required main checks passed
- **native:** captured off/on/off and inclusion on/off/on observed on qualified source87; unchanged records
- **owner acceptance:** pending; no VoiceOver/IME/general usability claim

Evidence: [docs/work/active/vacation-campaign/evidence/feedback-inclusion-qualification.json](active/vacation-campaign/evidence/feedback-inclusion-qualification.json), [docs/work/active/vacation-campaign/evidence/pr176-integration.json](active/vacation-campaign/evidence/pr176-integration.json), [https://github.com/ewq100/brn-rust/pull/176](https://github.com/ewq100/brn-rust/pull/176)

Attempts:

- Actual Mac check exposed missing selected-state accessibility metadata.: Added toggle metadata and model-suggestion context; both real control sequences then passed. ([docs/work/active/vacation-campaign/evidence/feedback-accessibility-qualification.json](active/vacation-campaign/evidence/feedback-accessibility-qualification.json))

</details>

### Confirm checks after the Mac correction merged

Owner: hosted checks; development lead inspects results · State: **complete** · Last observed: 2026-10-20T05:34:13.914672+00:00
Activity: All four required checks passed on merged Mac correction1ea0ac87.
Next: Reuse verified main; keep optional Windows failures visible.
Depends on: feedback-accessibility when Correction is normally merged so resulting-main checks exist. (reported satisfied: true)

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: verification only
- **checks:** passed: all four required resulting-main39373391210 jobs; optional Windows failed; optional Ubuntu retrieval still running at observation
- **integration:** verified normal merge1ea0ac87
- **native:** reuse unchanged qualified controls; no new native claim
- **owner acceptance:** inapplicable to hosted verification

Evidence: [https://github.com/ewq100/brn-rust/commit/1ea0ac875f6e7aa128ce21bde781075e87784fff](https://github.com/ewq100/brn-rust/commit/1ea0ac875f6e7aa128ce21bde781075e87784fff), [docs/work/active/vacation-campaign/evidence/pr176-integration.json](active/vacation-campaign/evidence/pr176-integration.json)

</details>

### AI note reads use the exact note identity

Owner: development lead · State: **complete** · Last observed: 2026-10-20T05:10:37.356608+00:00
Activity: PR175 merged and all four required resulting-main jobs passed; no reimplementation needed.
Next: Reuse source-bound evidence; keep broader native AI testing separate.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** merged 9cecd00b: schema guidance and truthful truncation trace
- **checks:** all four required head/main jobs passed; optional Windows failures retained
- **integration:** verified merged
- **native:** native AI not qualified; deterministic/live CLI evidence separate
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/pr175-integration.json](active/vacation-campaign/evidence/pr175-integration.json), [docs/work/active/vacation-campaign/evidence/codex-ai-smoke-qualification.json](active/vacation-campaign/evidence/codex-ai-smoke-qualification.json)

Attempts:

- Natural request omitted an ID suffix and failed safely; explicit-ID diagnostic passed.: Clarified tool schema; a new natural-request probe passed with Note bytes unchanged. ([docs/work/active/vacation-campaign/evidence/codex-ai-smoke-qualification.json](active/vacation-campaign/evidence/codex-ai-smoke-qualification.json))

</details>

### Earlier reliability and agent-workflow improvements

Owner: development lead · State: **complete** · Last observed: 2026-10-20T05:16:23.634823+00:00
Activity: Merged shared worker/Run and recovery coordination, notice/reference/MCP fixes, bounded argument correction, exact local editing and selected-feedback review.
Next: Reuse recorded guarantees; qualify remaining native journeys rather than rebuild completed work.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** merged in earlier campaign PR163–174; per-slice identities retained in receipts
- **checks:** all four required head/main gates through174 passed; optional Windows failures retained
- **integration:** verified per dated integration receipt; no whole-program completion claim
- **native:** partial actual Mac feedback/Apply/Undo/reopen evidence; Save/backup/unsaved journeys pending
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence.md](active/vacation-campaign/evidence.md), [docs/work/active/agent-first-simplification/reconciliation.md](active/agent-first-simplification/reconciliation.md)

</details>

### Prepare grouped-review test workspace

Owner: development lead · State: **complete** · Last observed: 2026-10-20T08:10:49.158833+00:00
Activity: PR179 normally merged b0eb18d7; all four required resulting-main39387204718 checks passed. Optional Windows failures retained; actual grouped Mac controls still pending.
Next: Reuse unchanged qualified production source/artifacts in the consolidated native session.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** candidate complete; UI659fdb98 and fixturee64c1fac source hashes in qualification
- **checks:** PASS independent review/local checks and all four required exact-head/main jobs; optional Windows failures retained
- **integration:** PASS normal merge b0eb18d7db7560269fa18d96c5bbf92eee1f40da and all four required resulting-main checks
- **native:** pending MAC-08 actual OS toggle observation; headless tests do not establish native usability
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/review-group-native-preparation.md](active/vacation-campaign/review-group-native-preparation.md), [docs/work/active/vacation-campaign/review-group-accessibility.md](active/vacation-campaign/review-group-accessibility.md), [docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json), [docs/work/active/vacation-campaign/evidence/review-group-source-review.json](active/vacation-campaign/evidence/review-group-source-review.json), [https://github.com/ewq100/brn-rust/pull/179](https://github.com/ewq100/brn-rust/pull/179), [docs/work/active/vacation-campaign/evidence/review-group-receipt-audit.json](active/vacation-campaign/evidence/review-group-receipt-audit.json), [docs/work/active/vacation-campaign/evidence/pr179-integration.json](active/vacation-campaign/evidence/pr179-integration.json)

Attempts:

- First build failed E0107: wildcard-imported core Result has one type parameter.: Qualified std::result::Result in the reopen collection; corrected build and public fixture assertions pass. ([docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json))

</details>

### Expose grouped Include/Omit selection to accessibility tools

Owner: development lead · State: **complete** · Last observed: 2026-10-20T08:10:49.158833+00:00
Activity: PR179 normally merged b0eb18d7; all four required resulting-main39387204718 checks passed. Optional Windows failures retained; actual grouped Mac controls still pending.
Next: Reuse unchanged qualified production source/artifacts in the consolidated native session.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** candidate complete; UI659fdb98 and fixturee64c1fac source hashes in qualification
- **checks:** PASS independent review/local checks and all four required exact-head/main jobs; optional Windows failures retained
- **integration:** PASS normal merge b0eb18d7db7560269fa18d96c5bbf92eee1f40da and all four required resulting-main checks
- **native:** pending MAC-08 actual OS toggle observation; headless tests do not establish native usability
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/review-group-native-preparation.md](active/vacation-campaign/review-group-native-preparation.md), [crates/brn-writing-ui/src/writing.rs](../../crates/brn-writing-ui/src/writing.rs), [docs/work/active/vacation-campaign/review-group-accessibility.md](active/vacation-campaign/review-group-accessibility.md), [docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json), [docs/work/active/vacation-campaign/evidence/review-group-source-review.json](active/vacation-campaign/evidence/review-group-source-review.json), [https://github.com/ewq100/brn-rust/pull/179](https://github.com/ewq100/brn-rust/pull/179), [docs/work/active/vacation-campaign/evidence/review-group-receipt-audit.json](active/vacation-campaign/evidence/review-group-receipt-audit.json), [docs/work/active/vacation-campaign/evidence/pr179-integration.json](active/vacation-campaign/evidence/pr179-integration.json)

</details>

### Check that command-line and desktop imports reuse the same conversation

Owner: development lead; single local build actor · State: **complete** · Last observed: 2026-10-20T07:13:50.680942+00:00
Activity: Existing public CLI-to-desktop shared-intake witness passed1/0/0 using fresh synthetic offline data; same chat and exact owner Message, one additional Run.
Next: Reuse source-bound qualification; preserve the opt-in default test and keep real Mac/AI usefulness checks separate.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: exercising existing test and public routes; no source change selected
- **checks:** passed: explicit existing ignored witness1passed/0failed/0ignored; public records6→7 with Runs1→2, unchanged owner Message and source bytes
- **integration:** inapplicable: no source changes; qualification record saved locally while PR179 head stays frozen
- **native:** headless toolkit only; actual OS interaction not exercised
- **owner acceptance:** not claimed

Evidence: [crates/brn-desktop/src/threads_intake_tests.rs](../../crates/brn-desktop/src/threads_intake_tests.rs), [docs/work/active/vacation-campaign/shared-intake-verification.md](active/vacation-campaign/shared-intake-verification.md), [docs/work/active/vacation-campaign/evidence/shared-intake-qualification.json](active/vacation-campaign/evidence/shared-intake-qualification.json)

Attempts:

- Default CLI printed a progress line before JSON, so the external orchestration parser failed after a successful offline intake.: Parse the single retained JSON result line and reuse the same fixture; no duplicate intake/Run. ([docs/work/active/vacation-campaign/evidence/shared-intake-qualification.json](active/vacation-campaign/evidence/shared-intake-qualification.json))

</details>

### Read development progress remotely in BRN Space

Owner: existing development lead; single Page writer · State: **complete** · Last observed: 2026-10-20T07:19:49.288497+00:00
Activity: Created BRN Development Progress under the existing BRN Space and linked it from the home page. Saved content and navigation link verified by readback.
Next: Refresh this same remote summary from current repository facts at meaningful campaign transitions; preserve human additions.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete: account-accessible Space summary and home-page navigation; no product source change
- **checks:** passed: connector readback of saved content, corrected refresh timestamp, actual parent/Space and preserved navigation rows
- **integration:** remote Page saved; repository routing saved locally while PR179 remote head remains frozen
- **native:** inapplicable to BRN product; remote browser/mobile rendering not observed
- **owner acceptance:** not requested; remote Page write explicitly authorized

Evidence: [https://chatgpt.com/space/page_2300ce21dbb88191a290f861b9eb6f68](https://chatgpt.com/space/page_2300ce21dbb88191a290f861b9eb6f68), [docs/work/active/vacation-campaign/evidence/remote-progress-page.json](active/vacation-campaign/evidence/remote-progress-page.json)

Attempts:

- Overview refresh rejected the new task record because its required attempts field was absent; the previous valid page was preserved.: Added this bounded correction history, then reran the existing renderer and documentation checks. ([docs/work/active/vacation-campaign/evidence/remote-progress-page.json](active/vacation-campaign/evidence/remote-progress-page.json))

</details>

### Inspect remaining Save and run-preview coordination risks

Owner: lead with two bounded read-only Luna medium helpers · State: **complete** · Last observed: 2026-10-20T07:36:27.202196+00:00
Activity: Two independent-scope source audits found no actionable defect: existing Save/recovery generation guards and run-preview identity gates cover inspected sequences. Retained existing checks without speculative refactors or extra tests.
Next: Use existing witnesses and pending actual Mac scenarios; revisit these boundaries only if new evidence exposes a defect.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: no correction selected; no source edits
- **checks:** source inspection only; helpers ran no tests/builds/account/native calls
- **integration:** audit records saved locally; no product integration needed
- **native:** pending existing Mac scenarios; source audit is not native proof
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/2026-10-20-save-recovery-audit.md](active/vacation-campaign/evidence/2026-10-20-save-recovery-audit.md), [docs/work/active/vacation-campaign/evidence/2026-10-20-run-preview-audit.md](active/vacation-campaign/evidence/2026-10-20-run-preview-audit.md)

</details>

### Qualify a bounded real-account writing proposal

Owner: development lead; independent Sol guard review passed · State: **complete** · Last observed: 2026-10-20T08:08:15.363275+00:00
Activity: Bounded real-account frozen-v2 writing proposal passed: exact requested phrase change, original/sentinel/comment unchanged; pending candidate only. First pre-dispatch failure and corrected sidecar review retained.
Next: Reuse successful synthetic fixture for real Mac review/Apply/Undo in consolidated daily batch; no extra generation needed.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete external developer consumer only; no production command/framework/prompt changes
- **checks:** passed corrected independent review, fresh prepare/reopen/refusal, one production Luna/low run and read-only exact candidate/record verification; deadline-verdict probe refused
- **integration:** inapplicable to external harness; repository evidence pending later reviewed publication
- **native:** pending actual Mac review/Apply/Undo; headless live usefulness is not OS input or owner acceptance
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/codex-ai-testing.md](active/vacation-campaign/codex-ai-testing.md), [docs/work/active/vacation-campaign/evidence/codex-ai-test-ledger.json](active/vacation-campaign/evidence/codex-ai-test-ledger.json), [docs/work/active/vacation-campaign/evidence/2026-10-20-writing-route.md](active/vacation-campaign/evidence/2026-10-20-writing-route.md), [docs/work/active/vacation-campaign/evidence/writing-harness-initial-review.md](active/vacation-campaign/evidence/writing-harness-initial-review.md), [docs/work/active/vacation-campaign/evidence/writing-harness-final-review.md](active/vacation-campaign/evidence/writing-harness-final-review.md), [docs/work/active/vacation-campaign/evidence/writing-harness-sidecar-review.md](active/vacation-campaign/evidence/writing-harness-sidecar-review.md), [docs/work/active/vacation-campaign/evidence/writing-account-qualification.json](active/vacation-campaign/evidence/writing-account-qualification.json), [docs/work/active/vacation-campaign/writing-account-verification.md](active/vacation-campaign/writing-account-verification.md)

Attempts:

- Independent review: verify could promote a deadline failure to success; broad evidence-directory binding could reuse one reserved attempt across sibling data roots.: Require explicit successful non-deadline verdict and exact data_directory binding; one focused correction/re-review pass before any launch. Ledger still has no20October attempt. ([docs/work/active/vacation-campaign/evidence/writing-harness-initial-review.md](active/vacation-campaign/evidence/writing-harness-initial-review.md))
- Reserved attempt e77bcbb1 refused before provider spawn; normal core rejects unexpected manifest.json in canonical DATA.: Preserve fixture/failure and count launch. Move only harness metadata to dedicated exact-data sibling sidecar; production folder guard unchanged. ([docs/work/active/vacation-campaign/evidence/codex-ai-test-ledger.json](active/vacation-campaign/evidence/codex-ai-test-ledger.json))
- Diagnosed repair passed on second fresh workspace: one useful proposal in10.88seconds,3/4rounds; no canonical edits.: Metadata moved to sibling sidecar, normal reopen added; final independent review and read-only verify passed. Session/day2/3 and2/6 consumed including first refusal; no further account call selected. ([docs/work/active/vacation-campaign/evidence/writing-account-qualification.json](active/vacation-campaign/evidence/writing-account-qualification.json))

</details>

### Integrate remote progress routing and verification records

Owner: development lead; independent Sol high review completed · State: **complete** · Last observed: 2026-10-20T09:30:16.780781+00:00
Activity: PR180 main02445b28 now passed all4required resulting-main39392850688 jobs. Optional Windows failures retained.
Next: Reuse reviewed progress route; refresh the same remote summary at meaningful transitions.
Depends on: review-group-toggle when PR179 normally merged so documentation can build on the integrated source. (reported satisfied: true)

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete documentation-only candidate; no runtime prompt/script/dependency/product changes
- **checks:** PASS independent review, required exact-head and resulting-main jobs; optional Windows failures retained
- **integration:** PASS normal merge02445b28 and all4required resulting-main checks
- **native:** inapplicable: documentation projection; mobile/browser rendering remains unobserved
- **owner acceptance:** not requested; owner authorized remote progress access

Evidence: [docs/work/active/vacation-campaign/evidence/remote-cycle-review.md](active/vacation-campaign/evidence/remote-cycle-review.md), [https://chatgpt.com/space/page_2300ce21dbb88191a290f861b9eb6f68](https://chatgpt.com/space/page_2300ce21dbb88191a290f861b9eb6f68), [https://github.com/ewq100/brn-rust/pull/180](https://github.com/ewq100/brn-rust/pull/180), [docs/work/active/vacation-campaign/evidence/pr180-integration.json](active/vacation-campaign/evidence/pr180-integration.json)

</details>

### Inspect backup and restore failure boundaries

Owner: bounded read-only Luna medium helper; lead validates · State: **complete** · Last observed: 2026-10-20T08:05:07.042761+00:00
Activity: Source audit identified missing Unix directory flushes in export publication, contrasted with existing backup/restore. Lead validated source and selected bounded correction.
Next: Implement selected export-publication-durability task in isolated worktree; keep inspection distinct from qualification.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: source inspection only until an evidenced defect selects work
- **checks:** passed source inspection/lead validation only; no executed qualification
- **integration:** inapplicable to inspection
- **native:** existing Mac backup scenario remains separate
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/2026-10-20-backup-failure-audit.md](active/vacation-campaign/evidence/2026-10-20-backup-failure-audit.md)

</details>

### Flush complete export folders before reporting success

Owner: development lead; Luna implementation and independent Sol reviews completed · State: **complete** · Last observed: 2026-10-20T11:37:02.369505+00:00
Activity: All4required main20f jobs nowPASS after one diagnosed MacCore timeout retry. Original45m cancellation retained; no source repair attributed to retry.
Next: Reuse reviewed integrated export source; actual native/power-loss qualification remains separate.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete isolated0f64a034; app export source+README only
- **checks:** PASS independent source/evidence reviews,6focused local tests,Clippy/fmt/nativebuild and all4required exact-head39392977994 checks; optional WindowsCore failed
- **integration:** PASS normal merge20f and all4required resulting-main checks; original cancelled MacCore attempt retained
- **native:** daily synthetic export journey separate; no power-loss qualification claim
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/2026-10-20-backup-failure-audit.md](active/vacation-campaign/evidence/2026-10-20-backup-failure-audit.md), [docs/work/active/vacation-campaign/evidence/export-durability-review.md](active/vacation-campaign/evidence/export-durability-review.md), [docs/work/active/vacation-campaign/evidence/export-durability-qualification.json](active/vacation-campaign/evidence/export-durability-qualification.json), [docs/work/active/vacation-campaign/evidence/followthrough-candidate-review.json](active/vacation-campaign/evidence/followthrough-candidate-review.json), [https://github.com/ewq100/brn-rust/pull/181](https://github.com/ewq100/brn-rust/pull/181), [docs/work/active/vacation-campaign/evidence/pr181-integration.json](active/vacation-campaign/evidence/pr181-integration.json), [docs/work/active/vacation-campaign/evidence/ci-main20f-timeout.json](active/vacation-campaign/evidence/ci-main20f-timeout.json)

</details>

### Inspect saved-message recovery after Stop and restart

Owner: bounded read-only Luna medium helper; lead validates · State: **complete** · Last observed: 2026-10-20T08:18:03.635242+00:00
Activity: Bounded source audit found no consequential saved-message/Stop/Resume/orphan recovery defect or missing witness. Existing durable identity/fence/replay tests cover inspected boundary.
Next: Retain existing witnesses; select further correction only if concrete failure or new risk appears.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: inspection only until an evidenced defect selects repair
- **checks:** complete source inspection only; no tests/app execution
- **integration:** inapplicable to audit
- **native:** actual Stop/restart usability remains separate
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/2026-10-20-queue-recovery-audit.md](active/vacation-campaign/evidence/2026-10-20-queue-recovery-audit.md)

</details>

### Measure current automated-check feedback cost

Owner: bounded read-only Luna medium helper; lead validates · State: **complete** · Last observed: 2026-10-20T09:34:14.850064+00:00
Activity: Measured two completed CI runs; lead independently matched six Mac job durations. Required helper/feature lanes already reuse appropriate build targets; no safe optimization established.
Next: Retain current required checks. Revisit only with comparable cache/feature/toolchain and per-test compile/body evidence.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: bounded investigation until a demonstrated correction selects work
- **checks:** PASS bounded timing/source assessment only; no tests rerun or code changed
- **integration:** inapplicable to investigation
- **native:** not observed; separate native batch
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/ci-cost-triage.md](active/vacation-campaign/evidence/ci-cost-triage.md), [docs/work/active/vacation-campaign/evidence/ci-cost-triage.json](active/vacation-campaign/evidence/ci-cost-triage.json)

</details>

### Inspect Action and history Undo safeguards

Owner: bounded read-only Luna medium helper; lead validates · State: **complete** · Last observed: 2026-10-20T09:34:14.850064+00:00
Activity: No apparent identity/version defect; found missing actual Confirm done UI completion-evidence and receipt Undo witness. Lead validated existing test only exercises Waiting and selected a bounded extension.
Next: Extend the existing Action/Undo journey, preserving Waiting coverage and test count.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable: bounded investigation until a demonstrated correction selects work
- **checks:** source/test inspection only; selected missing consequential UI-to-Store behavior witness
- **integration:** inapplicable to investigation
- **native:** not observed; separate native batch
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/action-history-audit.md](active/vacation-campaign/evidence/action-history-audit.md)

</details>

### Exercise Confirm done and its exact Undo in the desktop journey

Owner: development lead; bounded Luna implementation and independent Sol high review completed · State: **complete** · Last observed: 2026-10-20T12:14:36.811083+00:00
Activity: PR182 normally merged bc8a08e6; all four required resulting-main checks now PASS. Optional Windows failures retained.
Next: Reuse verified integrated source; actual Mac controls and owner acceptance remain separate.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete12679ef4; one test file +71lines, existing test count unchanged
- **checks:** PASS complete independent review/local checks and all4required exact-head39403071180; optional WindowsCore failed
- **integration:** PASS normal merge bc8a08e6 plus all four required resulting-main39415372292 checks
- **native:** toolkit checks are not OS interaction or owner acceptance
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/evidence/action-history-audit.md](active/vacation-campaign/evidence/action-history-audit.md), [docs/work/active/vacation-campaign/evidence/action-completion-qualification.json](active/vacation-campaign/evidence/action-completion-qualification.json), [docs/work/active/vacation-campaign/evidence/action-completion-review.md](active/vacation-campaign/evidence/action-completion-review.md), [https://github.com/ewq100/brn-rust/pull/182](https://github.com/ewq100/brn-rust/pull/182), [docs/work/active/vacation-campaign/evidence/pr182-premerge.json](active/vacation-campaign/evidence/pr182-premerge.json), [docs/work/active/vacation-campaign/evidence/pr182-resulting-main.json](active/vacation-campaign/evidence/pr182-resulting-main.json)

Attempts:

- Test authoring initially looked for Tasks controls while History was focused, then incorrectly expected Undo to reuse the original record version.: Navigate actual Tasks view explicitly and bind receipt.before to actual restored record (version3). Final checks pass; no production defect found. ([docs/work/active/vacation-campaign/evidence/action-completion-qualification.json](active/vacation-campaign/evidence/action-completion-qualification.json))

</details>

### Route older agent-first results to current task facts

Owner: development lead; mechanical routing inspection · State: **complete** · Last observed: 2026-10-20T09:45:22.800409+00:00
Activity: Marked two older comparison/reconciliation pages as19October history and routed current work to overview/campaign instead of checkpoint. Supplied plan and dated results preserved.
Next: Use concise current startup route; consult historical results only when relevant.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete two-page routing/label correction only
- **checks:** PASS lead mechanical diff inspection, overview current,606Markdown files/2844local links,whitespace; independent review inapplicable to routing labels only, source candidate separately reviewed
- **integration:** pending reviewed witness publication; no production behavior or standing authority change
- **native:** inapplicable documentation-only
- **owner acceptance:** not requested

Evidence: [docs/work/active/agent-first-simplification/reconciliation.md](active/agent-first-simplification/reconciliation.md), [docs/work/active/agent-first-simplification/outcome-comparison.md](active/agent-first-simplification/outcome-comparison.md)

</details>

### Inspect Project deletion and Trash recovery safety

Owner: bounded read-only Luna medium helper; lead validates · State: **complete** · Last observed: 2026-10-20T09:48:47.861687+00:00
Activity: Bounded audit found no actionable Project/Trash defect or consequential missing witness. Lead validated captured preview/full snapshot refusal and atomic stale-restore witness; no duplicate UI mirror test or new abstraction selected.
Next: Retain existing checked-service witnesses; revisit only with a reproduced failure or new boundary risk.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inapplicable until evidenced defect selects a bounded correction
- **checks:** PASS bounded source/test inspection and lead validation only; no test/build/native execution
- **integration:** inapplicable to inspection
- **native:** not observed; actual controls remain separate
- **owner acceptance:** not claimed

Evidence: [crates/brn-desktop/src/threads_projects.rs](../../crates/brn-desktop/src/threads_projects.rs), [crates/brn-desktop/src/threads_projects_tests.rs](../../crates/brn-desktop/src/threads_projects_tests.rs), [docs/work/active/vacation-campaign/evidence/project-trash-audit.md](active/vacation-campaign/evidence/project-trash-audit.md)

Attempts:

- Overview refused a task with empty evidence; previous valid overview preserved.: Record actual source/test entry points before audit dispatch. ([scripts/render-work-overview.py](../../scripts/render-work-overview.py))

</details>

### Find why Mac CI recompiles converters and misses reuse

Owner: development lead plus two bounded read-only Luna medium helpers · State: **complete** · Last observed: 2026-10-20T11:37:02.369505+00:00
Activity: Two bounded audits plus lead verified same-source cold45m08/warm14m46, earlier same-key hit then unavailable cache, PR-only duplicate storage and root-only graph pruning. Exact disappearance reason unknown; selected narrow main-only saves.
Next: Review/publish bounded cache-save policy; broader converter caching needs archive-size/source-guard and warm-run evidence.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** Investigation complete; bounded main-only save policy selected separately, broad helper caching deferred
- **checks:** PASS observed logs/cache inventory/pinned implementation and lead validation; one pair is not stable performance benchmark
- **integration:** inapplicable until concrete change selected
- **native:** inapplicable CI investigation
- **owner acceptance:** owner directly selected investigation

Evidence: [.github/actions/setup-rust/action.yml](../../.github/actions/setup-rust/action.yml), [.github/workflows/ci.yml](../../.github/workflows/ci.yml), [docs/work/active/vacation-campaign/evidence/ci-main20f-timeout.json](active/vacation-campaign/evidence/ci-main20f-timeout.json), [docs/work/active/vacation-campaign/ci-cache-investigation.md](active/vacation-campaign/ci-cache-investigation.md), [docs/work/active/vacation-campaign/evidence/ci-cache-investigation.json](active/vacation-campaign/evidence/ci-cache-investigation.json), [docs/work/active/vacation-campaign/evidence/ci-cache-causes.md](active/vacation-campaign/evidence/ci-cache-causes.md), [docs/work/active/vacation-campaign/evidence/ci-compile-reuse.md](active/vacation-campaign/evidence/ci-compile-reuse.md)

</details>

### Retire disposable cache copies for merged PR182

Owner: development lead; exact-ID generated-cache maintenance · State: **complete** · Last observed: 2026-10-20T11:46:11.997400+00:00
Activity: Removed four exact obsolete PR182 cache IDs after merged-state/ref/key/version revalidation:5886330190bytes. Readback shows all targets absent and all five preexisting main cache IDs retained.
Next: Future PR-save suppression belongs to the reviewed CI candidate; no recurring broad cleanup selected.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete exact-ID generated-cache cleanup; no main/other-ref DELETE or storage/billing changes
- **checks:** PASS four successful DELETE outcomes and complete before/after inventory; configured storage-limit read unavailableHTTP402 retained
- **integration:** inapplicable disposablecachemaintenance
- **native:** inapplicable
- **owner acceptance:** not requested; standing repository maintenance authority

Evidence: [docs/work/active/vacation-campaign/evidence/ci-cache-pr182-cleanup.json](active/vacation-campaign/evidence/ci-cache-pr182-cleanup.json), [docs/work/active/vacation-campaign/evidence/ci-cache-investigation.json](active/vacation-campaign/evidence/ci-cache-investigation.json)

</details>

### Inspect stale search results across view and scope changes

Owner: bounded read-only Luna medium helper; development lead validates · State: **complete** · Last observed: 2026-10-20T11:56:42.338154+00:00
Activity: Luna bounded source/test audit and lead validation found no actionable Search/launcher defect. Request identity guards reject stale completion; existing witnesses cover supersession and independent paging.
Next: No new mirror test or refactor selected; revisit on a concrete failure.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inspection only; no source change selected
- **checks:** PASS bounded inspection only; tests not executed
- **integration:** inapplicable until a concrete candidate exists
- **native:** not observed
- **owner acceptance:** not claimed

Evidence: [crates/brn-desktop/src/threads_search.rs](../../crates/brn-desktop/src/threads_search.rs), [crates/brn-desktop/src/threads_pages_tests.rs](../../crates/brn-desktop/src/threads_pages_tests.rs), [docs/work/active/vacation-campaign/evidence/general-search-audit.md](active/vacation-campaign/evidence/general-search-audit.md)

</details>

### Inspect drafts and attachments when switching conversations

Owner: bounded read-only Luna medium helper; development lead validates · State: **complete** · Last observed: 2026-10-20T11:56:42.338154+00:00
Activity: Initial navigation/send/attachment pass found no issue. Lead follow-up isolated a distinct first-writing-chat draft adoption persistence gap; supplemental audit confirmed source-supported restart risk.
Next: Selected writing-draft-recovery task owns reproduction and correction; preserve both original and supplemental audit.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** inspection only; no source change selected
- **checks:** Bounded source inspection; recovery defect requires a disposable view-recreation witness before implementation
- **integration:** inapplicable until a concrete candidate exists
- **native:** not observed
- **owner acceptance:** not claimed

Evidence: [crates/brn-desktop/src/threads_state.rs](../../crates/brn-desktop/src/threads_state.rs), [crates/brn-desktop/src/threads_ui.rs](../../crates/brn-desktop/src/threads_ui.rs), [crates/brn-desktop/src/threads_chat_tests.rs](../../crates/brn-desktop/src/threads_chat_tests.rs), [docs/work/active/vacation-campaign/evidence/composer-drafts-audit.md](active/vacation-campaign/evidence/composer-drafts-audit.md), [docs/work/active/vacation-campaign/evidence/composer-draft-adoption-audit.md](active/vacation-campaign/evidence/composer-draft-adoption-audit.md)

</details>

## Dependencies

```mermaid
graph TD
  t1["Mac feedback choices expose their selected state"]
  t2["Confirm checks after the Mac correction merged"]
  t6["Prepare grouped-review test workspace"]
  t7["Expose grouped Include/Omit selection to accessibility tools"]
  t8["Check grouped review controls on the Mac"]
  t13["Integrate remote progress routing and verification records"]
  t1 -->|Correction is normally merged so resulting-main checks exist. (satisfied)| t2
  t6 -->|Fresh offline two-group fixture passed public reopen/discovery/unchanged-state checks. (satisfied)| t8
  t7 -->|Corrected UI source reviewed and ordinary production binary qualified. (satisfied)| t8
  t7 -->|PR179 normally merged so documentation can build on the integrated source. (satisfied)| t13
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-20T12:26:12.703752+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
