# BRN Threads verification

The [canonical acceptance journeys](../architecture/threads-target.md#acceptance-journeys) define what the rebuild must demonstrate. Existing scripts and tests are reusable evidence; tests that enforce retired mechanisms must be replaced coherently with their production paths.

## Select checks for the change

| Change | Evidence |
|---|---|
| Setup and documentation | `git diff --check`; `python3 scripts/check-markdown-links.py`; current path/command consistency and explicit environment inventory. |
| Core persistence and authorization | Atomic grouped commit, prepare and apply replay after lost responses, stale record/protection changes, protected writes, versioned Save, cross-process dirty guards, stale human buffers and delayed recovery writes, crash/restart, immediate compensation, and restore. |
| Agent journey | Real selected provider/tool path against synthetic/public scenarios, checked committed result, relevant source references, Action outcome, and run interruption/budget behavior. |
| Full-note import | Representative paper and process fixtures with substantive prose, steps, tables, figures, and references; no mathematical equation fidelity gate; compare a hand-checked PDF/DOCX fixture inventory and source coverage; verify partial outcomes and offline retained content. |
| Native interaction | Actual queue/thread/editor/review/comment/search/Action/history/export journeys on the target Mac, with restart checks and understandable conflicts. |
| Exports and backup | Fresh complete snapshot publication; failed exports stay visibly failed; restored BRN records/assets/history in a clean directory. Backup/restore staging must not expose incomplete final paths, collisions preserve existing content, and marked stores with missing databases refuse recreation. Saved backup folders survive reopen/restore, unavailable folders fail without fallback, and all backup entry points refuse canonical workspace storage. |
| Runtime guides and thread behavior | Representative success/boundary scenarios, actual skill loading and bundle identity, quiet routine outcomes, exact attention reasons, separate Action completion and fresh-invocation continuation. |
| Component reuse or removal | Relevant component tests plus coherent removal of obsolete routes, commands, helpers, and tests; retained dependency fixes remain verified. |

Use the repository pin (currently Rust 1.98.1) and lockfile. Compiler/dependency changes need applicable task scope and relevant qualification. Source inspection is not a passing build or native result. Default workspace checks do not cover optional native features. Run focused checks as needed; repeat broad checks only for relevant changes or a concrete unresolved risk. No zero-test or self-skipped run counts as proof.

## Behavior and architecture evidence

Follow the [strategic programming guide](strategic-programming.md). A regression test should expose the original bug when practical; document the limitation when the old behavior cannot safely or deterministically be reproduced. Preserve distinct boundary, feature and failure cases when consolidating tests. Test or line counts do not establish equivalence.

For a stateful component, use its specified states/events to check legal transitions, rejected events and invariants. Add sequence/property tests when ordering is the risk, and controlled fault tests for persistence or external effects. The reference expectation must come from the behavior contract rather than calling the production transition function to compute its own expected answer. Reducer tests do not verify event wiring, focus, scrolling, rendering or OS integration; retain the applicable integration and native journeys.

Use targeted mutation testing when it can expose weak assertions around changed high-consequence decisions. Bound the scope/runtime and report meaningful survivors, equivalent cases with reasons, unviable mutations and incomplete work separately. Follow any more specific current task requirements. There is no repository-wide mutation or 100% coverage gate.

Prefer Rust types/visibility, existing retirement checks, manifest/dependency checks and real consumer builds for architecture promises. Add a cheap check alongside a meaningful new boundary when justified, with an actionable failure message. Do not silently suppress a finding, relax a baseline or introduce a no-op check to achieve green. Any justified exception belongs in the review with its reason and impact.

## CI and integration

The [CI workflow](../../.github/workflows/ci.yml) now covers the Threads core/app/runtime/CLI and new native state. It has a documentation/tooling job and core/native lanes. Main currently requires these contexts:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

Keep meaningful jobs with the maintained code. Do not add no-op passing jobs, suppress failures, or weaken repository protection to make a rebuild green. Product scope is Mac first; an unrelated platform issue should be reported without silently becoming another development program.

## Evidence and limits

Record tested commit/tree, environment, features, exact command, result, and limitation. Use fresh explicit synthetic data and preserve task evidence. Existing [tooling](tooling.md) can record gate output; read commands before use.

Distinguish implemented, verified, native-observed, owner-accepted, and merged. Provider calls and downloads follow current task authority. CI does not receive account credentials. If the Mac is locked or inaccessible, gather one explicit UI qualification task and continue useful headless work without claiming UI success.


## Current local gates

Build the maintained `brn-intake-helper` before the Mac workspace tests. PDF fixture qualification needs Poppler `pdftotext`/`pdfimages`/native `pdftoppm`; CI installs Poppler, and local tests accept `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT`, `BRN_PDFIMAGES`, `BRN_PDFTOPPM`. The default Mac test suite includes the source inventories, prepared-reader/formatter checks, renderer restriction and transient-cleanup witnesses. Provider transport tests use mocks; live dedicated Luna qualification uses fresh synthetic sources only when authorized and reports source QA separately from model success.

`verify-storage.sh` covers default formatting/build/lint/tests. `verify-end-to-end.sh` adds fresh CLI/desktop Save, recovery, search, export/backup, startup and foreign-data refusal. `verify-end-to-end.sh` also runs the CLI journey through the documented `--json` contract. `verify-desktop-shell.sh` compiles/lints/tests native state. Semantic search is optional: a native build installs the pinned model with `brn model-install --destination …`, and hosts use the folder named by `BRN_EMBEDDING_MODEL_DIR`; `retrieval_eval --retriever hybrid` qualifies it. The 10,000-note witness in `brn-threads-app/tests/user_scale.rs` proves reopen without rebuild, one-note sync, query latency, current-revision search refresh and archive exclusion; run it with `BRN_SCALE_STRICT=1 cargo test --release -p brn-threads-app --test user_scale` for the acceptance bounds. Removed standalone experiments remain accessible through the [fixed baseline archive](../archive/README.md); maintained production/test fixtures remain in the crates.
