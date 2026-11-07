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

## Owner-requested additional reviews

The owner requested three further rounds after delivery and selected a two-seat
Council. This supersedes the earlier statement about the original funded rounds.
Round 3 returned Sol 8/10 and Opus 8/10. Sol identified an exported semantic-packet
binding defect. The lead now checks exported packets against regenerated private
preparation, derives metadata from the bound packet and uses its existing event
evidence. Each grading request gets a digest returned by the judge, and the
controller verifies that digest, runtime identity and the one-request accounting
before accepting a reply. Reader-setup dispositions now bind to the original
operation id, root, role, kind and request digest. The automatic empty-receipt
continuation was removed in favor of explicit fresh operation identities.

These changes have offline verification and apply to future execution. Historical
receipts and the unmeasured acceptance gaps retain their recorded status.

Round 4 returned Sol 8/10 and Opus 8/10. The lead then bound the judge's packet
bytes and instructions bytes separately and verified instructions against the
frozen artifact before each dispatch. Both digests and fixed runtime identity
are checked before accepting a response. Reader exports now use regenerated
preparation equality as well as their content digest. Offline regressions cover
instruction drift and reader content altered together with its hash.

The stronger marker checks intentionally refuse old, incompletely bound markers
for fresh dispatch. Historical auditing uses the retained executed controller
and original protocol. The current controller does not resume the stopped cycle.
The initial targeted Rust filter matched zero tests and is retained as an excluded
check attempt. The corrected module filter executed the two wire tests.
