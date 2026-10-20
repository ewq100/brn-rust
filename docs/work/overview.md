# Development overview

**Goal:** Make BRN more dependable and easier to improve; make progress and remaining decisions easy to see.
**Owner attention:** No owner decision requested. Required hosted checks are running; prepared real Mac checks use the daily batch.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

## Working on

### Prepare grouped-review test workspace

Owner: development lead · State: **active** · Last observed: 2026-10-20T07:15:13.743308+00:00
Activity: Published PR179 at unchanged ec9190d8; required hosted checks running. No local build/desktop/account actor active.
Next: Resume after hosted results change; inspect all exact-head required checks and live protection, normally merge when eligible, then verify resulting main.
Resource waits: Hosted PR179 checks; remote head frozen while progress records stay local.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** candidate complete; UI659fdb98 and fixturee64c1fac source hashes in qualification
- **checks:** passed: fixture/reopen/refusals,333desktop state tests (1existing ignored),affected strict lint,format,ordinary native build; initial fixture E0107 retained
- **integration:** published PR179 ec9190d835217beca08ef9af77a4458fdb252d54; required head/main gates pending
- **native:** pending MAC-08 actual OS toggle observation; headless tests do not establish native usability
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/review-group-native-preparation.md](active/vacation-campaign/review-group-native-preparation.md), [docs/work/active/vacation-campaign/review-group-accessibility.md](active/vacation-campaign/review-group-accessibility.md), [docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json), [docs/work/active/vacation-campaign/evidence/review-group-source-review.json](active/vacation-campaign/evidence/review-group-source-review.json), [https://github.com/ewq100/brn-rust/pull/179](https://github.com/ewq100/brn-rust/pull/179), [docs/work/active/vacation-campaign/evidence/review-group-receipt-audit.json](active/vacation-campaign/evidence/review-group-receipt-audit.json), [docs/work/active/vacation-campaign/evidence/pr179-integration.json](active/vacation-campaign/evidence/pr179-integration.json)

Attempts:

- First build failed E0107: wildcard-imported core Result has one type parameter.: Qualified std::result::Result in the reopen collection; corrected build and public fixture assertions pass. ([docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json))

</details>

### Expose grouped Include/Omit selection to accessibility tools

Owner: development lead · State: **active** · Last observed: 2026-10-20T07:15:13.743308+00:00
Activity: Published PR179 at unchanged ec9190d8; required hosted checks running. No local build/desktop/account actor active.
Next: Resume after hosted results change; inspect all exact-head required checks and live protection, normally merge when eligible, then verify resulting main.
Resource waits: Hosted PR179 checks; remote head frozen while progress records stay local.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** candidate complete; UI659fdb98 and fixturee64c1fac source hashes in qualification
- **checks:** passed: fixture/reopen/refusals,333desktop state tests (1existing ignored),affected strict lint,format,ordinary native build; initial fixture E0107 retained
- **integration:** published PR179 ec9190d835217beca08ef9af77a4458fdb252d54; required head/main gates pending
- **native:** pending MAC-08 actual OS toggle observation; headless tests do not establish native usability
- **owner acceptance:** not claimed

Evidence: [docs/work/active/vacation-campaign/review-group-native-preparation.md](active/vacation-campaign/review-group-native-preparation.md), [crates/brn-writing-ui/src/writing.rs](../../crates/brn-writing-ui/src/writing.rs), [docs/work/active/vacation-campaign/review-group-accessibility.md](active/vacation-campaign/review-group-accessibility.md), [docs/work/active/vacation-campaign/evidence/review-group-qualification.json](active/vacation-campaign/evidence/review-group-qualification.json), [docs/work/active/vacation-campaign/evidence/review-group-source-review.json](active/vacation-campaign/evidence/review-group-source-review.json), [https://github.com/ewq100/brn-rust/pull/179](https://github.com/ewq100/brn-rust/pull/179), [docs/work/active/vacation-campaign/evidence/review-group-receipt-audit.json](active/vacation-campaign/evidence/review-group-receipt-audit.json), [docs/work/active/vacation-campaign/evidence/pr179-integration.json](active/vacation-campaign/evidence/pr179-integration.json)

</details>

## Next

None recorded.

## Needs attention

### Check saving, reopening and backup in the real Mac app

Owner: development lead; single desktop actor · State: **blocked** · Last reported: 2026-10-20T06:45:18.405208+00:00
Activity: Prepared Save/reopen/backup/recovery scenarios can use source-bound qualified production; ordinary allowance available.
Next: Run ready synthetic journeys in the daily consolidated Mac batch after confirming actual controls/artifact identity.
Resource waits: Daily18:00Tallinn consolidated native session; single desktop/build actor and source identity must be confirmed.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** existing application; no new source fix selected
- **checks:** reconcile source-bound build before native session
- **integration:** use reviewed integrated source
- **native:** pending Save/reopen, backup and unsaved-recovery journeys
- **owner acceptance:** pending; native evidence does not imply owner acceptance

Evidence: [docs/work/active/vacation-campaign/native-queue.md](active/vacation-campaign/native-queue.md)

</details>

Waiting:

- resource: Daily18:00Tallinn consolidated native session; single desktop/build actor and source identity must be confirmed.

### Prepare grouped-review test workspace

Owner: development lead

Waiting:

- resource: Hosted PR179 checks; remote head frozen while progress records stay local.

### Expose grouped Include/Omit selection to accessibility tools

Owner: development lead

Waiting:

- resource: Hosted PR179 checks; remote head frozen while progress records stay local.

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

## Dependencies

```mermaid
graph TD
  t1["Mac feedback choices expose their selected state"]
  t2["Confirm checks after the Mac correction merged"]
  t6["Prepare grouped-review test workspace"]
  t7["Expose grouped Include/Omit selection to accessibility tools"]
  t8["Check grouped review controls on the Mac"]
  t1 -->|Correction is normally merged so resulting-main checks exist. (satisfied)| t2
  t6 -->|Fresh offline two-group fixture passed public reopen/discovery/unchanged-state checks. (satisfied)| t8
  t7 -->|Corrected UI source reviewed and ordinary production binary qualified. (satisfied)| t8
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-20T07:19:49.288497+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
