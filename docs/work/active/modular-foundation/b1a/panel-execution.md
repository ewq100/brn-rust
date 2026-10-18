# B1a panel source execution

**Base:** clean `df5e48e38549ad19a70daeaf47f9b6b185ef6d0e`, `work/b1a-cw2-20261016`. Current principal allocation owns the exact [panel scope](panel-scope.md), including shared support/callers and F15; it supersedes that proposal's historical unallocated wording. Helper source is independently approved and stays unchanged.

**Goal:** move the current writing panel and view state once into brn-writing-ui, retaining the checked host service, synchronous dirty guard and CW2 display/context contracts.

**Architecture:** a plain host-owned WritingPanel; ephemeral view borrows of the real Workspace/canonical caches; concrete requests invoke the current host handlers synchronously. Host retains runtime, selected scope/conversation and the one complete checked display cache. Optional GPUI modules only under native-ui; no host/editor trait, provider copy or second datastore.

- [x] Prepare exact shared renderer/theme/components/layout support and existing tests; keep desktop-only shell/list/form functions.
- [x] Prepare panel local state/editor/document/workbench/render implementation; forward current host calls and retain once-per-render CW2 cache gating.
- [x] Adapt existing test field/API access without changing assertions/events/reduced-motion setups; add the allocated real-editor F15 witness.
- [x] Direct Rust1.98.1 rustfmt and source/path/provenance review; prepare source-ready fingerprint and exact manifest/lock/native-test requests.
- [x] First granted window: library44 tests, strict native lint/build, F15/work-bound, one complete desktop native suite, fmt and default graph/build.
- [x] Second granted window: execute default full lint/tests/F22 and retrieval/combined/headless gates; retain the baseline PDF failure and exact counts.
- [ ] Principal local acceptance/dependent allocation; no worker source or heavy reservation remains after final handoff.

**Current gate:** [second-window qualification and complete handoff](panel-handoff.md) are ready for principal local acceptance. Native/library/combined/headless checks passed; full default tests retain the carried PDF Unavailable failure. No publication, native-human or B1a acceptance claim.

**Review focus:** exact candidate/context cache keys and error clearing; early private disclosure refusal/mandatory footer; immediate first edit generation guard; saved selection after Save; local editor Undo after Apply/refresh without stale resurrection. Existing tests plus F15 and the bounded display-work witness passed the first-window native lanes. Broader qualification and independent review remain pending.

Source preparation and its limits are recorded in [panel-source-prep](panel-source-prep.md). Checked boxes describe source work only; the final compile/behavior/review gate remains pending.

The principal-requested eager-display regression is corrected within five owned source files: view construction now borrows state and defers existing concrete display queries to consumers. The [revised source receipt](panel-source-prep.md#bounded-deferred-display-correction-after-principal-inspection) retains the first fingerprint, performance-witness plan and principal-owned verification-file deltas. Heavy/lock/test gates remain pending.

The [first-window receipt](panel-window1.md) supersedes the source-only gate: it records actual results/failures, exact test relocation/counts, principal CI/script and local lock inclusion, and release to the principal after the local pin. UI source ownership remains with this worker pending review/fix allocation.

Independent [panel Opus round1](panel-opus-review-round1.md) approved exact ca44bd79. The [bounded LOW response](panel-round1-response.md) restores explanatory docs, consolidates section_header and narrows the work-bound claim in source only. **BUILDNEED:** affected native checks and remaining default/combined/headless qualification await an explicit second heavy window; D1A currently owns the slot. Prior ca44 checks remain historical and are not a post-correction pass.

**Final local handoff:** [second-window results and full ownership release](panel-handoff.md) supersede the deferred/BUILDNEED scheduling statements above. Independent Opus approved ca44; the bounded LOW corrections have actual affected checks and remaining offline qualification, with the default PDF failure still open. Principal acceptance, hosted CI and native-human observation are separate.
