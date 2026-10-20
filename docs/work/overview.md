# Development overview

**Goal:** Make BRN more dependable and easier to improve; make progress and remaining decisions easy to see.
**Owner attention:** No owner decision requested. Remaining external waits are checks or capacity.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

## Working on

None recorded.

## Next

None recorded.

## Needs attention

### Clearer instructions and one progress view

Owner: development lead · State: **blocked** · Last observed: 2026-10-20T05:34:13.914672+00:00
Activity: Implemented, independently reviewed and published; only required hosted checks/normal merge/resulting-main verification remain.
Next: Finish required PR178 exact-head checks; preceding main is verified. Then inspect live protection and normal match-head merge c6666cb9, followed by resulting-main checks.
Resource waits: External required hosted checks on frozen PR178 head c6666cb9. Local progress checkpoint is intentionally ahead and must not be pushed onto that PR while checks run.

<details>
<summary>Checks, evidence and corrections</summary>

Gates:

- **implementation:** complete; docs, read-only renderer and personal planning skill; runtime prompts unchanged
- **checks:** local and hosted documentation/tooling passed; required Mac retrieval passed; remaining exact-head app checks are pending, inspect actual PR results before integration
- **integration:** PR178 published; normal merge and resulting-main verification pending
- **native:** inapplicable: development documentation/view only
- **owner acceptance:** not requested; owner authorized implementation

Evidence: [docs/work/active/vacation-campaign/visibility-adoption.md](active/vacation-campaign/visibility-adoption.md), [docs/work/active/vacation-campaign/evidence/visibility-adoption-preflight.json](active/vacation-campaign/evidence/visibility-adoption-preflight.json), [https://github.com/ewq100/brn-rust/pull/177](https://github.com/ewq100/brn-rust/pull/177), [docs/work/active/vacation-campaign/instruction-audit.md](active/vacation-campaign/instruction-audit.md), [docs/work/active/vacation-campaign/evidence/visibility-automation-adoption.json](active/vacation-campaign/evidence/visibility-automation-adoption.json), [docs/work/active/vacation-campaign/evidence/writing-plans-adoption.json](active/vacation-campaign/evidence/writing-plans-adoption.json), [https://github.com/ewq100/brn-rust/pull/178](https://github.com/ewq100/brn-rust/pull/178), [docs/work/active/vacation-campaign/evidence/visibility-adoption-review.json](active/vacation-campaign/evidence/visibility-adoption-review.json), [docs/work/active/vacation-campaign/evidence/visibility-adoption-qualification.json](active/vacation-campaign/evidence/visibility-adoption-qualification.json), [docs/work/active/vacation-campaign/evidence/visibility-adoption-integration.json](active/vacation-campaign/evidence/visibility-adoption-integration.json)

Attempts:

- Preflight complete; stopped at ordinary reserve floor.: Owner explicitly approved using reserve for this adoption; implementation resumed. ([docs/work/active/vacation-campaign/evidence/visibility-adoption-preflight.json](active/vacation-campaign/evidence/visibility-adoption-preflight.json))
- Independent review found readiness was inferred from overall task completion and overlapping skill review rules.: Explicit prerequisite-condition satisfaction now controls readiness; skill review triggers and inputs aligned with governing workflow. ([scripts/tests/test_work_overview.py](../../scripts/tests/test_work_overview.py), [docs/work/active/vacation-campaign/evidence/writing-plans-adoption.json](active/vacation-campaign/evidence/writing-plans-adoption.json))

</details>

Waiting:

- resource: External required hosted checks on frozen PR178 head c6666cb9. Local progress checkpoint is intentionally ahead and must not be pushed onto that PR while checks run.

### Check saving, reopening and backup in the real Mac app

Owner: development lead; single desktop actor · State: **blocked** · Last reported: 2026-10-20T05:10:37.356608+00:00
Activity: Prepared scenarios remain; previously passing feedback journeys need no replay.
Next: After ordinary capacity returns, qualify ready synthetic scenarios in one consolidated Mac session.
Resource waits: Ordinary campaign admission: reserve exception covers visibility adoption only.; Single desktop/build slot must be free; confirm actual access and artifact identity.

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

- resource: Ordinary campaign admission: reserve exception covers visibility adoption only.
- resource: Single desktop/build slot must be free; confirm actual access and artifact identity.

## Completed

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

## Dependencies

```mermaid
graph TD
  t1["Mac feedback choices expose their selected state"]
  t2["Confirm checks after the Mac correction merged"]
  t1 -->|Correction is normally merged so resulting-main checks exist. (satisfied)| t2
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-20T05:34:13.914672+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
