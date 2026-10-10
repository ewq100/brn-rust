# BRN Threads verification

The [canonical acceptance journeys](../architecture/threads-target.md#acceptance-journeys) define what the rebuild must demonstrate. Existing scripts and tests are reusable evidence; tests that enforce retired mechanisms must be replaced coherently with their production paths.

## Select checks for the change

| Change | Evidence |
|---|---|
| Setup and documentation | `git diff --check`; `python3 scripts/check-markdown-links.py`; current path/command consistency and explicit environment inventory. |
| Core persistence and authorization | Atomic grouped commit, operation replay after lost response, stale record/protection changes, protected writes, active editing buffers, crash/restart, immediate compensation, and restore. |
| Agent journey | Real selected provider/tool path against synthetic/public scenarios, checked committed result, relevant source references, Action outcome, and run interruption/budget behavior. |
| Full-note import | Representative paper and process fixtures with substantive prose, steps, tables, equations, figures, and references; compare source coverage; verify partial outcomes and offline retained content. |
| Native interaction | Actual queue/thread/editor/review/comment/search/Action/history/export journeys on the target Mac, with restart checks and understandable conflicts. |
| Exports and backup | Fresh complete snapshot publication; failed exports stay visibly failed; restored BRN records/assets/history in a clean directory. |
| Component reuse or removal | Relevant component tests plus coherent removal of obsolete routes, commands, helpers, and tests; retained dependency fixes remain verified. |

Use the pinned toolchain and lockfiles. Default workspace checks do not cover optional native features. Run focused checks as needed; repeat broad checks only for relevant changes or a concrete unresolved risk. No zero-test or self-skipped run counts as proof.

## CI and integration

The baseline [CI workflow](../../.github/workflows/ci.yml) remains in place during setup. It has a documentation/tooling job and core/native lanes. Main currently requires these contexts:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

Adapt meaningful jobs with the replacement code. Do not add no-op passing jobs, suppress failures, or weaken repository protection to make a rebuild green. Product scope is Mac first; an unrelated platform issue should be reported without silently becoming another development program.

## Evidence and limits

Record tested commit/tree, environment, features, exact command, result, and limitation. Use fresh explicit synthetic data and preserve task evidence. Existing [tooling](tooling.md) can record gate output; read commands before use.

Distinguish implemented, verified, native-observed, owner-accepted, and merged. Provider calls and downloads follow current task authority. CI does not receive account credentials. If the Mac is locked or inaccessible, gather one explicit UI qualification task and continue useful headless work without claiming UI success.
