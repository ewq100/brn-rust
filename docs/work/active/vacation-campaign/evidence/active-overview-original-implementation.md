# Active overview history compaction

## Candidate

Implemented on isolated branch `codex/concise-active-overview` from freshly fetched `origin/main` at `e918b2b303f44cfca6cbd49347ea5b4fd4e335f1`. Candidate commit: `a61325dbc0e17c85bc8ed30207483d2f6d71e4cf`.

Changed paths:

- `scripts/render-work-overview.py`: active/blocked task summaries still show the current title, owner, state, observation timestamp/origin, activity, next action, dependencies, resource waits, gate values, and dependency graph. Raw evidence lists and attempt outcomes/corrections no longer render into that summary. The gate fold is now titled “Gate status.” The selected-record instructions retain the exact `--record ... --task ...` lookup route. Removed the now-unused evidence-link formatter; evidence is available in the validated task JSON.
- `scripts/tests/test_work_overview.py`: adjusted the summary expectation and added one growth regression. A synthetic active and blocked task each carry 120 evidence entries and 40 attempts. Their rendered summaries must exactly match equivalent tasks with empty attempt lists and minimal evidence, while retaining owner/time/activity/next/gates/waits/dependency graph. The existing custom-record `--task` route must return each exact selected task object and leave both record bytes and output path unchanged.

No documentation file or generated campaign overview was changed. `docs/development/workflow.md` already documents `--task` with custom `--record`; the generated overview already links the selected record and documents that lookup. I did not regenerate `docs/work/overview.md` from a potentially stale worker record; lead integration should render the eventual authoritative record when appropriate.

## Checks and evidence

- Before the renderer correction, the focused new regression failed on summary equality; unittest reported a 58,696-character diff from raw history expansion.
- After correction: `python3 -m unittest scripts.tests.test_work_overview` — **14 tests passed**.
- `git diff --check` and committed-diff whitespace check passed.
- No Rust builds/tests, provider, account, native, or external-workload checks were run.

Reuse/retirement: this keeps the existing validated canonical record and `--task` CLI as the sole detailed-history route. It adds no tracker or sidecar. The one test that asserted evidence links in the default summary was updated to assert they are absent; the new test verifies exact history remains retrievable through the existing task route. No unrelated tests were removed.

## Limits

The selected task reported an 48,851-byte overview before this change. The synthetic regression proves summary size/content is independent of evidence and attempt-history growth; I did not calculate a new size against the mutable lead campaign or regenerate its overview. Independent review, live campaign projection/check, integration, publication, and owner acceptance remain pending with the lead.

Checkout is clean at `a61325dbc0e17c85bc8ed30207483d2f6d71e4cf`.
