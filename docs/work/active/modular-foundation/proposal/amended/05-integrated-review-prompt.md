# Independent review — combined BRN foundation candidate

Read the canonical modular-foundation plan, contracts/ownership/baseline, A/B/C evidence, current repository authority and the lead's exact combined candidate ref. Review the **actual combined tree**, not only three separate branches or their reports.

You are independent and read-only by default. Do not change implementation, merge, publish, release, access owner data or call paid models unless separately assigned. Synthetic offline checks may be run in an isolated worktree. Report exact commands and limitations. Do not accept another agent's passing-test summary without checking material evidence.

## Review objectives

1. **Product fit:** Is reference-assisted writing complete inside BRN? Can references/brief persist across chats and restart, with document-local feedback/review and Chat closed? Does the development host reuse the same implementation and real shared service?
2. **Real modularity:** Trace dependencies. Ensure writing is not secretly a wrapper around the full desktop/workspace: `brn-writing-ui` must not depend on `brn-desktop`, and run start and conversation selection must come from the shared service, not copies in desktop, CLI and host. Check that the default workspace build on Linux compiles no GPUI (F22). Intake and retrieval remain reusable behind narrow interfaces. Reject duplicated storage, permissions, context membership, provider clients, saved-operation Undo or parallel writing implementations.
3. **Safety contracts:** Trace every changed write/read path to shared authority. Test stale document/protection/reference/scope changes, dirty generations, exact candidate binding, replay after lost responses, cancellation-before-save, grouped atomicity, local-vs-durable Undo, private provenance and restart/backup behavior.
4. **Information scope:** Search/exact reads/source panes/assets/snippets/counts/cursors/Connections/caches/traces use the same host-owned scope. Historical citations do not defeat access revocation; specifically, owner-review Apply of a writing candidate rechecks visibility of its `Historical` reference inputs. A reference private to one conversation's scope never reaches a run in another (F21). Source content cannot expand tools/provider/permissions.
5. **Source fidelity:** Full import has independently checked expected content or honest gaps. Table/figure association is verified, not just word presence. Original files are not secretly retained. Sources used reflect actual supplied/read retained revisions, and top-k retrieval is not labeled exhaustive reading.
6. **Dependency decisions:** Inspect the Xberg/Zorite spike reports: exact pins/features, package versus upstream workspace differences, GUI/native runtime graph, licenses/provenance, build/maintenance footprint and mandatory qualification failures. Confirm no spike code reached production paths without a separate approved cutover. Negative adoption decisions are valid. Neither marketing nor successful demos prove qualification.
7. **Baseline behavior:** Dedicated conversion behavior is preserved or explicitly superseded with evidence. Existing model selection, delegated maintenance, projects/private scopes, queue, recovery and supported import formats have not regressed incidentally.
8. **Tests:** Relevant tests assert actual outcomes and are not zero-test/self-skipped/mocked-only substitutes. Required native and Linux core contexts remain meaningful. Mutation survivors and performance/retrieval comparisons have honest interpretation. No relaxed threshold or deleted test hides a regression.
9. **Cleanup and compatibility:** Retired routes are gone after qualified cutover; no hidden fallback or duplicate policy remains. Necessary saved-data readers survive. Schema changes, backups and code/data rollback are coherent.
10. **Evidence:** Distinguish implemented, deterministic/live/native verified, reviewed, owner-accepted, locally integrated, published, merged and released. A locked/unavailable Mac means native gates remain open.

## Output

Lead with blocking/high-severity findings, each with exact file/line, reproducible scenario, violated contract and the smallest coherent fix. Then list medium/low findings and residual unverified risks. Classify each as a confirmed defect, evidence gap or design trade-off; do not inflate preferences into defects.

Give a clear verdict: ready for the next stated gate; ready only with named limitations; or not ready with explicit blockers. State which checks you actually ran. Do not authorize a release or claim untested usability.
