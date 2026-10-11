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
| Exports and backup | Fresh complete snapshot publication; failed exports stay visibly failed; restored BRN records/assets/history in a clean directory. |
| Runtime guides and thread behavior | Representative success/boundary scenarios, actual skill loading and bundle identity, quiet routine outcomes, exact attention reasons, separate Action completion and fresh-invocation continuation. |
| Component reuse or removal | Relevant component tests plus coherent removal of obsolete routes, commands, helpers, and tests; retained dependency fixes remain verified. |

Use the pinned toolchain and lockfiles. The [dependency qualification plan](../work/active/threads-rebuild/plan.md#dependency-qualification) defines Rust 1.99.0 qualification against unchanged dependencies, consistent compiler selection in retained scripts, and the separate Rig and GPUI upgrade proofs. Source inspection is not a passing build or native result. Default workspace checks do not cover optional native features. Run focused checks as needed; repeat broad checks only for relevant changes or a concrete unresolved risk. No zero-test or self-skipped run counts as proof.

## CI and integration

The [CI workflow](../../.github/workflows/ci.yml) now covers the Threads core/app/runtime/CLI and new native state. It has a documentation/tooling job and core/native lanes. Main currently requires these contexts:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

Adapt meaningful jobs with the replacement code. Do not add no-op passing jobs, suppress failures, or weaken repository protection to make a rebuild green. Product scope is Mac first; an unrelated platform issue should be reported without silently becoming another development program.

## Evidence and limits

Record tested commit/tree, environment, features, exact command, result, and limitation. Use fresh explicit synthetic data and preserve task evidence. Existing [tooling](tooling.md) can record gate output; read commands before use.

Distinguish implemented, verified, native-observed, owner-accepted, and merged. Provider calls and downloads follow current task authority. CI does not receive account credentials. If the Mac is locked or inaccessible, gather one explicit UI qualification task and continue useful headless work without claiming UI success.


## Current local gates

Build the maintained `brn-intake-helper` before the Mac workspace tests. PDF fixture qualification needs Poppler `pdftotext`/`pdfimages`; CI supplies their explicit paths, and local tests accept `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT`, `BRN_PDFIMAGES`. The default Mac test suite includes the source inventories and transient-cleanup witnesses.

`verify-storage.sh` covers default formatting/build/lint/tests. `verify-end-to-end.sh` adds fresh CLI/desktop Save, recovery, search, export/backup, startup and foreign-data refusal. `verify-desktop-shell.sh` compiles/lints/tests native state. The 5,000-note witness in `brn-threads-app/tests/user_scale.rs` proves current-revision search refresh and archive exclusion. Older standalone experiments remain historical evidence, with no production engine dependency.
