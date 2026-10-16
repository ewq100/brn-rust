# B1a panel source execution

**Base:** clean `df5e48e38549ad19a70daeaf47f9b6b185ef6d0e`, `work/b1a-cw2-20261016`. Current principal allocation owns the exact [panel scope](panel-scope.md), including shared support/callers and F15; it supersedes that proposal's historical unallocated wording. Helper source is independently approved and stays unchanged.

**Goal:** move the current writing panel and view state once into brn-writing-ui, retaining the checked host service, synchronous dirty guard and CW2 display/context contracts.

**Architecture:** a plain host-owned WritingPanel; ephemeral view borrows of the real Workspace/canonical caches; concrete requests invoke the current host handlers synchronously. Host retains runtime, selected scope/conversation and the one complete checked display cache. Optional GPUI modules only under native-ui; no host/editor trait, provider copy or second datastore.

- [x] Prepare exact shared renderer/theme/components/layout support and existing tests; keep desktop-only shell/list/form functions.
- [x] Prepare panel local state/editor/document/workbench/render implementation; forward current host calls and retain once-per-render CW2 cache gating.
- [x] Adapt existing test field/API access without changing assertions/events/reduced-motion setups; add the allocated real-editor F15 witness.
- [x] Direct Rust1.98.1 rustfmt and source/path/provenance review; prepare source-ready fingerprint and exact manifest/lock/native-test requests.
- [x] First granted window: library44 tests, strict native lint/build, F15/work-bound, one complete desktop native suite, fmt and default graph/build.
- [ ] Next principal allocation: default full lint/tests, combined/headless qualification and fresh independent review/acceptance.

**Current gate:** first local qualification window passed at the [recorded source identity](panel-window1.md); local pin/review and remaining acceptance are distinct. Default full lint/tests and combined/headless lanes are deferred. No publication or B1a completion is claimed.

**Review focus:** exact candidate/context cache keys and error clearing; early private disclosure refusal/mandatory footer; immediate first edit generation guard; saved selection after Save; local editor Undo after Apply/refresh without stale resurrection. Existing tests plus F15 and the bounded display-work witness passed the first-window native lanes. Broader qualification and independent review remain pending.

Source preparation and its limits are recorded in [panel-source-prep](panel-source-prep.md). Checked boxes describe source work only; the final compile/behavior/review gate remains pending.

The principal-requested eager-display regression is corrected within five owned source files: view construction now borrows state and defers existing concrete display queries to consumers. The [revised source receipt](panel-source-prep.md#bounded-deferred-display-correction-after-principal-inspection) retains the first fingerprint, performance-witness plan and principal-owned verification-file deltas. Heavy/lock/test gates remain pending.

The [first-window receipt](panel-window1.md) supersedes the source-only gate: it records actual results/failures, exact test relocation/counts, principal CI/script and local lock inclusion, and release to the principal after the local pin. UI source ownership remains with this worker pending review/fix allocation.
