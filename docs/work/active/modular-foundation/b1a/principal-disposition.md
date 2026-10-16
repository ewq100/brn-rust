# Principal B1a local disposition — 16 October 2026

The principal accepts B1a source/contracts at `60f604f5d24437507d837dfda14778f83f01bd08` as the local foundation for B1b/B4. The checkout was clean at handoff; its crates tree is `e75715cf02699edf4e17ec8dbf1bf9f5d4b32fba`. This documentation receipt changes no production source. Main and origin/main are unchanged; no push, PR, merge or release was performed.

## Source and independent review

[Helper review](helper-opus-review-round1.md) and [panel review](panel-opus-review-round1.md) approved the actual extracted source. The principal inspected the complete subsequent seven-crate-path delta `ca44bd79..60f604f5`: restored/moved documentation, the existing `section_header` moved unchanged into the library and reused by both hosts, and a test assertion message qualified to its actual no-saved-request fixture. No assertion value, pointer path, authority, scope predicate or data operation changed. These bounded LOW corrections do not need another material review round. [Response](panel-round1-response.md) records their dispositions.

One production writing panel and its local state now live in `brn-writing-ui`; replaced desktop document/workbench/helper implementations are removed. The real Workspace, exact CW2 review/cache predicates, synchronous dirty-buffer update before redraw, native editor and host dispatch remain in use. The crate has no dependency on `brn-desktop`; default workspace dependencies remain GPUI-free. Writing-specific startup adaptation remains B1b work.

## Verification reconciled by the principal

The principal independently matched all **295** production/manifest/CI/script file hashes to the final checkout, all **21** command-log hashes, all contemporaneous source-manifest hashes and each command's 295 source associations. Raw Rust harness totals match the stored receipts. The copied panel review matches SHA256 `2410ec3f2df1a54dce47fef0a4692adafff10fe4473492e84a932de562f75e68`. See [window 2](panel-window2.json) and [handoff](panel-handoff.md) for exact commands, features, artifact identities and durations.

- Workspace build, strict all-target lint, format and default feature checks passed. Full workspace tests: **1109 passed / 1 failed / 6 ignored**; the sole failure is the carried PDF qualification requiring missing native `pdftoppm`.
- Maintained offline fixtures: **108 steps / 1266 checks**, passed.
- Maintained native shell script: all seven commands passed; library **44 passed**, desktop **270 passed / 1 existing ignored**, including CW2 privacy/typing and editor-Undo/work-bound assertions.
- Native retrieval **35 passed**; app/native retrieval **240 passed / 1 existing ignored**; CLI build passed; combined desktop build/lint and **55 tests** passed. Fresh/reopened synthetic headless startup passed twice.
- Five earlier native linking failures occurred before tests and remain recorded. Explicitly selecting the already installed matching ORT static library corrected the offline build environment; source bytes stayed unchanged. No model/download, stub, source fix or suppressed test was used.

No mutation campaign was added for moved, unchanged UI guards. Existing CW mutation qualifications remain separate. Counts across configurations overlap and are not a unique-test total.

## Acceptance boundary and ownership

B1a is implemented, locally automated-verified with the carried PDF failure, source-reviewed and accepted for local dependent development. Full F-suite/program qualification, hosted candidate CI, native human observation, shared external fixture, owner acceptance, publication, main merge and release remain open. The original B stop-test cause remains unconfirmed; a passing corrected fixture is not a production repair claim.

All B1a source/test/shared paths and the heavy slot were explicitly returned to the principal. D1A now owns the sole heavy slot; B1b may prepare isolated source while it runs. No D1A source is integrated into this B1a checkout. PC/T, X/Poppler, live-provider and D-S/D-M gates are preserved.
