# Status snapshot draft (NF-3), not applied

18 October 2026. A draft of the replacement top section for `docs/status.md`, for the integration lead to apply during final integration after conflicts are resolved. `docs/status.md` is read only for this task and was not edited. Every row is a Git or record fact at the time of writing; re-check against Git before applying, as the newer AGENTS.md requires. The all-document objective stays open.

```markdown
## Current repository snapshot, <date>

**Inspected main:** `<origin/main SHA>`. This snapshot reconciles repository state; it does not start a proposed program or change owner acceptance. Check Git and CI again when starting another assignment.

| Work | Current repository state |
|---|---|
| Foundation and agent-capability programs | Not proposed-only any more. C0, CL (a no-op), CW1, CW2, B1a, B1b, B3, B4, INT-W (writing line) and D1A, D1B, D2, D3a, D3b, D4, INT-D source (capability line) are implemented and **locally accepted on two unmerged branches**. Nothing from them is on `main` until the final integration lands. Records: `docs/work/active/modular-foundation/`, `docs/work/active/agent-capabilities/`. |
| Writing line | `work/int-w-20261017` `d2d84e66` (source `0efd73f1`). Deterministic local readiness; F01 PDF qualification, F20 native observation and hosted lanes open. |
| Capability line | `work/x2-intake-20261017` (source `39a67e4f`, docs through `781c34dc`). I01 real-provider live goal incomplete: closed windows, four unapplied history candidates, no Stop/Continue or live owner Apply. |
| Xberg all-document intake | Owner scope is all current document kinds plus explicit extra families. Only a first schema-1 HTML helper component exists as an uncommitted candidate; PDF resource STOP, Office, spreadsheet, MIME, host cutover and Poppler retirement are open. Not done. |
| Final integration | Plan only: `docs/work/active/final-integration/plan.md`. Starts after its section 2 precondition. No combined build or review exists. |
| Passage-comment positioning | Draft #149, owner direction **do not merge**. |

**Verification state, none of it combined or native:** the carried PDF qualification (native `pdftoppm` absent locally), the unexplained scripted-AI SDK 474/4/1 and widget 314/1/1 records, and the hosted main native-lane failure are kept as recorded and are not called baseline, flaky or closed. Hosted Windows/Linux optional failures remain visible. No release, real-data or native-human claim.
```

Items to correct when applying: the Xberg row after any pin; the "Inspected main" SHA after the fetch; link the final-integration evidence after the run; replace dated 16 to 17 October paragraphs below the snapshot with links rather than rewriting their results.
