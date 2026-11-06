# Final review and corrections

Two funded Council rounds. Actual routes: Codex/GPT-6.1 Sol, Copilot/Claude Opus
5.5 and Copilot/Kimi K3, high effort. Kiro/Opus failed its monthly quota in design;
the installed policy's named Copilot/Opus fallback was used. Provider request and
token counts remain unknown. Six read-only tool calls per final reviewer were
reported; tool calls are not HTTP requests. Review bounds were requested, not
hard-enforced by T3. All final roles completed before the requested ten-minute
boundary; reconciliation found no active runs. Holdout contents were not reviewed.

| Reviewer | Design | Final |
| --- | ---: | ---: |
| Sol | 6/10 | 8/10 |
| Opus fallback | 5.5/10 | 7/10 |
| Kimi | 8/10 | 8/10 |

The [raw reviews](final-reviews.json) preserve findings and reviewer limitations.
Kimi's code inspection was shallow; Opus did not inspect the actual candidate
patch. Scores describe the reviewed state, before these lead-made corrections.
No further review round or live rerun was funded or requested.

- Sol found that a post-run audit rejection could retain a native zero exit.
  The supervisor now sets failure status in its exception handler. A controlled
  regression failed on the executed version (`0 == 0`) and passes after the fix,
  checking both the process exit and actual failed deadline terminal. No observed
  result is reclassified; the fresh stop was an enforced deadline, not this path.
- Opus found missing tests for the actually used never-dispatched reader setup
  exemption. Tests now accept the narrow valid witness and reject an existing
  root, changed request hash, and non-reader request with a valid recomputed hash.
- Opus's scheduling findings are disclosed in both reports: Phase 1 unseen was
  not admitted with time still remaining; phase clocks overlapped; fresh Phase 2
  preceded development readers/broad regressions; its new ten-minute coordinator
  window could not fit the worst-case twenty-minute native workload. It used up
  the fresh cases on an unqualified candidate. Remaining whole-cycle time was not
  the immediate stop cause. The coordinator's reporting overrun is explicit.
- Kimi's evidence glob found no files. Direct stat and SHA verification confirmed
  the named artifacts, then every exported visible manifest entry. Those witnesses
  are retained. Final Council status, per-repetition efficiency, reader ceiling and
  unrun consistency checks are now explicit. The improvement branch is stacked
  on the final comparison layer, with no active production-guide change.

Exact [executed source](evidence/executed-source/manifest.json) was archived before
the post-run correction. Verification-only freeze specifications redirect the
supervisor file to those same original bytes and preserve both original freeze
identities; original specifications and receipts remain untouched. The current
corrected source was not used in the experiment and has only offline verification.

Saved-state quality, fresh reader quality and broader regressions remain unknown.
Those acceptance objections are unresolved. Both reports retain the original
baseline and make no adoption, default-model, desktop, merge or release claim.
