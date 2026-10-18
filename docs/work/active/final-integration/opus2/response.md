# Response to Opus round 2 (CHANGES_REQUIRED, M1 and M2)

18 October 2026. Docs-only. The report is copied verbatim in [opus-review-round2.md](opus-review-round2.md) (SHA-256 `b39d1b1e3998b3fa1bd6d99844f266da611d48aac7bb27ec7c71ba7c6c58a74d`, byte-identical to the root file; the terminal JSON summary equals it except one trailing newline). The round 1 report under `opus1/` is untouched. Prior candidate: commit `ae63fb7d5110327c8aecad5684e65e0573267fe4`, tree `5f877d636499f613af3cdd74c5cf23a2e75016c1`. The reviewer held B1 to B4 and advisories 1 to 8 of round 1; only the points below changed. This is not an approval; the parent inspects and records what closed.

## M1 (from B1): who can keep Poppler or narrow a current kind

**Accepted.** The owner scope says to preserve currently supported behavior, not to drop a format, lower quality or keep an undisclosed fallback, and to take any unmet contract as an explicit consequential decision. Under AGENTS.md that is the owner's.

- **FP-4** ([plan](../plan.md) section 2; [inventory](../inventory.json) `final_precondition`): "If the principal or owner instead decides to keep Poppler" is replaced by: keeping Poppler or any old backend or fallback is an **owner-only** cited decision because it changes the all-document objective; not a plan default, and a principal disposition cannot make it.
- **One cross-cutting rule** (plan section 2, "Authority rule"; inventory `authority_rule`): any FP-2 or FP-3 outcome that narrows what a current kind supports today (refusing inputs the current route accepts, lower fidelity, changed profile acceptance, a dropped kind) needs a cited **owner** decision. The principal may choose behavior-preserving technical solutions and make FP-5 extra-family admit, defer or refuse decisions with reasons; it cannot change the all-document objective.
- **FP-3** now requires the profile disposition to state whether it narrows current PDF acceptance.
- No owner question is raised: no narrowed proposal exists. The text only fixes who decides if one appears.

## M2 (from B2): case C evidence

**Accepted.** Checked against the source the reviewer cites: `start_request` spawns the worker in the background, and a nonexistent `auth_file` can fail the run at any time, so "not Failed" is timing-dependent; `attach_external_input` (read at `73ea9ca7` `brn-ai/src/threads.rs`) on success only sets the in-memory `scope.external` and appends to the prepared input string, so no persisted field exists; on failure it records a terminal Failed run before returning the error. [NF1-test-spec.md](../NF1-test-spec.md) case C now uses only `start_run`'s synchronous control flow. Immediately after it returns: `worker.is_some()`; the notice contains neither the take refusal nor the "MCP input authorization refused" attach refusal; `mcp_view.pending` is `None`; exactly one new Run record whose id equals the worker's run id. No Failed, Working or terminal-state assertion. A take refusal returns before prepare (no Run) and an attach refusal fails before spawn (`worker == None`), so these assertions are conclusive for both steps. The existing direct take/prepare/attach positive stays the evidence for attachment content. No test API or code is invented. The wording is mirrored in the inventory `NF-1` summary and in the round 1 response, which now carries a supersession note.

## Advisories

| # | Advisory | Disposition |
|---|---|---|
| A1 | `retry_rewrite`'s prompt can equal a composer prompt | Accepted. Verified in Git: INT-W `brn-writing-ui/src/native.rs:195` and X2 `threads_ui.rs:2234` use the same fixed string (`Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.`). Spec and `merge-simulation.md` now say a review for a different prompt is refused, an exactly equal review may attach as in X2, and `false` differs from X2 only if a queue item's text equals that string. Case B2 records the limit if a matched item with `dispatch == None` cannot be built through current checked paths; no unsupported fixture. No production decision is made. |
| A2 | Cite the policy file | Plan sections 1 and 6.2 now name `final-integration-kiro-opus1-corrections-20261018.md` (and the family disposition file) with their root path; `inventory.json` `carried_failures_gate_policy.policy_record` carries it. |
| A3 | Lock STOP wording | Plan section 5 and NF-4 now read: a move **not declared by the audited handoff and approved** is a STOP; the six HTML-slice moves do not forbid later exactly approved changes the all-document candidate needs (for example `tokio` 1.53.2 for Excel, `mail-parser` 0.11.9 for email, side-by-side `zip`/`quick-xml`), which are still at the old versions in the WIP. The four-companion disposition is unchanged and the actual final graph is stated as unknown. |
| A4 | PDF carry at FI | Plan section 6.2 item 5 and the G2 note say the local `PDFUnavailable` carry and "cannot finish locally" apply to TCI as written. At FI, FP-4 retires Poppler, so the PDF test, the Homebrew Poppler CI step and `check-intake-retirement.py` change in the Xberg lane and are re-derived from the handoff; no automatic local PDF-test exception. Mirrored in the inventory. |
| A5 | TCI publication | Plan section 0: no TCI is published to `main` ahead of the owner-set Xberg sequence unless a cited owner authority changes it; no TCI grant, build or publication exists now. Mirrored in the inventory `names`. |

## Untouched

Round 1 report; `status-draft.md` (no FP-4 text there); `docs/status.md`; every X2 doc and ledger. The 82 old link failures remain outside these docs and are the X2 lane's conditional fix.

## Limits

Git and text reads, read-only JSON parse of the terminal files, hash and link/whitespace checks ([checks.json](checks.json)). No Cargo, build, test, lint, formatter, Store, CLI, network, fetch, auth, provider, GUI, mutation or live command; no heavy slot; no import of the X2 WIP; the X2 verification task was not touched. Nothing here claims source verification, native observation, owner acceptance, a ready PR or a merge.
