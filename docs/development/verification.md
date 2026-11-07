# BRN Threads verification

The [acceptance journeys](../architecture/threads-target.md#acceptance-journeys) define what the product must demonstrate. Run the smallest check that covers the change; CI runs the full set. `bash scripts/check-changed.sh` is the default local check: it formats, lints and tests the changed crates and runs the matching script, link and budget checks (`--plan` prints the commands).

## Select checks for the change

| Change | Evidence |
|---|---|
| Setup and documentation | `git diff --check`; `python3 scripts/check-markdown-links.py`. |
| Core persistence and authorization | Atomic grouped commit, prepare and apply replay after lost responses, stale record/protection changes, protected writes, versioned Save, cross-process dirty guards, stale human buffers and delayed recovery writes, crash/restart, immediate compensation, and restore. |
| Agent journey | Real selected provider/tool path against synthetic/public scenarios, checked committed result, relevant source references, Action outcome, and run interruption/budget behavior. |
| Full-note import | Representative paper and process fixtures with substantive prose, steps, tables, figures, and references; compare a hand-checked PDF/DOCX fixture inventory and source coverage; verify partial outcomes and offline retained content. |
| Native interaction | Actual queue/thread/editor/review/comment/search/Action/history/export journeys on the Mac, with restart checks and understandable conflicts. |
| Exports and backup | Fresh complete snapshot publication; failed exports stay visibly failed; restored BRN records/assets/history in a clean directory. Backup/restore staging must not expose incomplete final paths, collisions preserve existing content, and marked stores with missing databases refuse recreation. |
| Runtime guides and thread behavior | Representative success/boundary scenarios, actual skill loading and bundle identity, quiet routine outcomes, exact attention reasons, separate Action completion and fresh-invocation continuation. |
| Instruction packages | `cargo test -p brn-ai --lib skill_packages`, `cargo test -p brn-threads-core --test data_folder`, `cargo test -p brn-threads-app --test skills`, `cargo test -p brn --test skill_packages`. |
| Document conversion | `cargo test -p brn --features scripted-document-tests --bin brn document_route_tests --locked --offline --jobs 1` (offline scripted transport, not a live provider check). [Conversion matrix](https://github.com/ewq100/brn-rust/blob/3e2f9f71526f974389784554f159d8f0c36cb6c6/docs/work/active/modular-foundation/x2/standard-default/verification.md). |
| Component reuse or removal | Relevant component tests plus removal of obsolete routes, commands, helpers, and tests. |

A zero-test or self-skipped run is not a pass. Default workspace checks do not cover optional native features.

## Scripts

Run `bash scripts/setup-worktree.sh` and `source .brn-dev/env.sh` first (see [setup](setup.md)). It also copies both document helpers beside the built CLI in `$CARGO_TARGET_DIR/debug/`, which direct CLI process tests need; those tests clear `BRN_*` overrides and use sibling discovery.

- `scripts/verify-storage.sh`: formatting, build, lint and default tests.
- `scripts/verify-end-to-end.sh`: fresh CLI/desktop Save, recovery, search, export/backup, startup, foreign-data refusal and the `--json` CLI journey.
- `scripts/verify-desktop-shell.sh`: native state compile, lint and tests.
- Semantic search (optional): install the pinned model with `brn model-install --destination …`, point `BRN_EMBEDDING_MODEL_DIR` at it and qualify with `retrieval_eval --retriever hybrid`.
- Scale: `BRN_SCALE_STRICT=1 cargo test --release -p brn-threads-app --test user_scale` (10,000 notes).
- Offline native retrieval needs the cached ONNX Runtime setup in [tooling](tooling.md); Cargo offline mode alone does not select it.

When sharing a target directory between checkouts, timestamp freshness can keep another checkout's local library after a source switch. If behavior looks stale, force the affected local crates to rebuild.

## Tests and design

Follow [strategic programming](strategic-programming.md). A regression test should expose the original bug when practical. For stateful components, test legal transitions, rejected events and invariants against the behavior contract rather than the production transition function. Add sequence/property or fault tests when ordering, persistence or external effects are the risk. Reducer tests do not cover event wiring, focus, rendering or OS integration.

## CI

The [CI workflow](../../.github/workflows/ci.yml) has a change-scope job, a documentation/tooling job and core/native lanes. PRs and main pushes run only the four required Mac-first contexts, which the preview release gate and `scripts/ci-summary.py` check:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

A PR that changes only Markdown outside `crates/`, `agent/`, `tools/` and `fixtures/` directories skips these Rust lanes; its `Change scope` job records the skip, and Documentation and tooling still runs. Main pushes never skip them, because the [preview release gate](preview-release.md#publication-gate) needs them to pass.

The informational Linux native and Windows lanes run nightly and on manual dispatch (`gh workflow run ci.yml --ref BRANCH`, or `-f windows_only=true` for Windows alone). Dispatch the full matrix before a change that targets those platforms, or before an owner-approved preview release.

The separate [agent context budget](../../.github/workflows/agent-context-budget.yml) check fails when `AGENTS.md` or `agent/` guidance grows; trim it or add the `agent-context-increase` label. After intended changes run `python3 scripts/check-agent-context-budget.py --write`.

CI has no account credentials. Product scope is Mac first.
