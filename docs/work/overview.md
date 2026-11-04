# Development overview

**Goal:** Subscription-search changes integrated; preserve the separate native Mac acceptance gate.
**Owner attention:** PR244 merged at098dc556. All four required PR and resulting-main checks passed; repeated35+3 offline checks passed. No owner action needed for this integration. Native Mac acceptance remains open; no release.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/subscription-search-mac-acceptance/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

None recorded.

## Next

None recorded.

## Needs attention

### Observe actual Mac controls

Task ID: `native-mac`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-11-04T10:27:42.844068+00:00
Activity: No new native observation. Original30October inventory reported locked; merged Label/search code has headless/CI evidence only.
Next: A separate owner-selected native qualification can address this gate. No UI/unlocking is part of the completed headless integration.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** reviewed scoped candidate
- **checks:** 24 page +3 Capture +3 search harness tests
- **integration:** MERGED: PR244 at098dc556; exact PR and resulting-main required contexts PASS. No release.
- **native:** UNOBSERVED: actual Mac buttons/focus/rendering/accessibility/IME/pickers and owner acceptance remain separate.
- **owner acceptance:** not claimed

</details>

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

3 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `document-workflow` — Import knowledge and produce a referenced draft
- `publication` — Integrate reviewed fixes through required CI
- `subscription-search` — Qualify subscription-only search

Local page reflects the last saved observations in this record (2026-11-04T10:27:42.844068+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
