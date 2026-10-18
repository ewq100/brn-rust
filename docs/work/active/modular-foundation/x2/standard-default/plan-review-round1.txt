## Review
**Verdict:** CHANGES_REQUIRED

**Blocking findings:**

- **Useful-information conversion provenance is uncovered.** Task2 [line 189](/Users/evokessler/.t3/worktrees/brn-rust/t3-d611a161/docs/work/active/modular-foundation/x2/standard-default/implementation-plan.md:189) persists provenance only in `ImportCoverage.conversion`, while Task3 line 218 promises stored provenance through existing imports. The brief requires actual converter/bundle/AI provenance for conversion results, including the useful-information route. That route discards `ConversionResult.converter` when constructing `TransientSource` in CLI [line 1588](/Users/evokessler/.t3/worktrees/brn-rust/t3-d611a161/crates/brn/src/main.rs:1588) and desktop [line 3423](/Users/evokessler/.t3/worktrees/brn-rust/t3-d611a161/crates/brn-desktop/src/threads_ui.rs:3423). Neither `TransientSource` nor its persisted `Source.report` currently carries conversion identity; this route does not create the full-import `ImportCoverage`.

  **Correction criterion:** Extend Tasks2–3 with a concrete, host-owned provenance path through useful-information handoff and persisted source reports. Specify affected interfaces/files and synthetic verification that captured converter/bundle/AI identity survives reopen and cannot be forged by model-authored Source updates. Keep historical records unlabelled and source identity unchanged.

**Advisory suggestions:**

- Include replay fixtures for Source-only “already imported” receipts and prepared-but-unapplied operations; these differ from receipts containing a newly created Note.

**Coverage summary:**

- **Task1:** Eight kinds, normal stock output, isolated patched graph, light dependency, shared restricted runtime and helper identity.
- **Task2:** Persisted defaults, schema10 compatibility, backup/restore, selected-engine extraction, optional tool-free Luna refinement, variant-aware replay/dedup and protected-note provenance.
- **Task3:** Native Settings, shared CLI service, captured preferences and existing import/report surfaces without a comparison screen.
- **Task4:** Both bundled helpers, relocation, source/license custody, CI and subsequent independent implementation review.
- **Uncovered:** Durable useful-information conversion provenance, as above.

The scope is materially reduced. Source inspection supports the proposed build separation; actual builds and relocation remain explicitly assigned implementation gates. No stock fidelity limitation is treated as a blocker.
