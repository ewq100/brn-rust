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

Use the repository pin (currently Rust 1.98.1) and lockfile. Compiler/dependency changes require separate qualification; this cleanup assigns no upgrade. Source inspection is not a passing build or native result. Default workspace checks do not cover optional native features. Run focused checks as needed; repeat broad checks only for relevant changes or a concrete unresolved risk. No zero-test or self-skipped run counts as proof.

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

Build both current document helpers before Mac workspace tests and declare their absolute `BRN_XBERG_HELPER` / `BRN_XBERG_HELPER_BRN` paths. Current gates exercise normal stock/adapted output, exact identity, PNG/assets, bounded reads, transient cleanup, cancellation/reaping and synthetic fixed-Luna refinement. No old helper, prepared-page protocol or Poppler installation is required. Historical legacy-PDF failures remain recorded, not converted into passing current checks.

`verify-storage.sh` covers default formatting/build/lint/tests. `verify-end-to-end.sh` adds fresh CLI/desktop Save, recovery, search, export/backup, startup and foreign-data refusal. `verify-end-to-end.sh` also runs the CLI journey through the documented `--json` contract. `verify-desktop-shell.sh` compiles/lints/tests native state. Semantic search is optional: a native build installs the pinned model with `brn model-install --destination …`, and hosts use the folder named by `BRN_EMBEDDING_MODEL_DIR`; `retrieval_eval --retriever hybrid` qualifies it. The 10,000-note witness in `brn-threads-app/tests/user_scale.rs` proves reopen without rebuild, one-note sync, query latency, current-revision search refresh and archive exclusion; run it with `BRN_SCALE_STRICT=1 cargo test --release -p brn-threads-app --test user_scale` for the acceptance bounds. Removed standalone experiments remain accessible through the [fixed baseline archive](../archive/README.md); maintained production/test fixtures remain in the crates.


## D3a instruction-package management

Use `cargo test -p brn-ai --lib skill_packages`, `cargo test -p brn-threads-core --test data_folder`, `cargo test -p brn-threads-app --test skills` and `cargo test -p brn --test skill_packages`, with the pinned toolchain and locked/offline flags. These cover strict manifest/path/byte bounds, hostile filesystem/collision/refusal, exact-digest removal, real owner-service/CLI reopen, and canonical positive-control backup/restore/export with installed instruction bytes excluded. This exclusion is an explicit local-configuration limitation, not a failed canonical backup or a complete D3 runtime qualification.

D3a does not prove runtime selection/loading, per-run frozen catalogue/update/revoke fencing or project-scope injection S01/S03/S04; those remain D3b. No installed instruction may self-enable a tool, connector or protected-write grant. Preserve actual before-source failures and all candidate errors; targeted mutation runs only after a compiled immutable pin and passing copied baseline under its single cumulative slice cap.


## Standard-default document conversion

[Reproduction and the eight-kind matrix](../work/active/modular-foundation/x2/standard-default/verification.md) identify both standalone locks, the SHA-verified optional source generation, declared artifacts and platform limits. Build both document helpers before Mac tests and export `BRN_XBERG_HELPER` / `BRN_XBERG_HELPER_BRN`; missing artifacts fail Mac qualification. Restricted execution is Mac-only; portable DTO/settings checks and explicit non-Mac unavailable controls remain separate.

Run the explicit offline synthetic CLI lane with `cargo test -p brn --features scripted-document-tests --bin brn document_route_tests --locked --offline --jobs 1` and its strict all-target Clippy equivalent. This is an in-process test-only transport check, not a live provider or subprocess AI-success claim. Ordinary builds do not expose injection. Standalone helper test/lint commands use their own literal stock/adapted targets, not the root workspace target.

The app distribution contains desktop plus stock and BRN helpers, without a legacy production helper or Poppler dependency. The old typed-reader implementation/tests have been retired coherently; their earlier failures and qualifications remain historical evidence. Representative normal PDF conversion does not establish encrypted/scanned/complex fidelity, custom refusal or resource proof. Encrypted support remains owner-deferred.
