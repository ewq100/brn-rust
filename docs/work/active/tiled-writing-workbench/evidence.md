# Tiled writing workbench verification

Baseline: `adc3b273e1985bc56a36d912ad26b34af99e703a`, isolated branch `work/tiled-writing-workbench`. The PR commit/tree identifies the published candidate. No dependency, schema, provider configuration or user data changes.

## Environment

macOS Apple Silicon; repository-pinned Rust 1.98.1, locked offline dependencies. Cargo target cache `/tmp/brn-ui-repairs-target-20261010`. Tests use disposable synthetic workspaces. Default fixture tests use the maintained intake helper and explicit Poppler converters. Logs are retained locally in `/tmp/brn-tiled-writing-tests/`.

## Checks

| Check | Result |
|---|---|
| Core/app writing behavior | 10 focused tests passed (6 app + 4 core), including addressed-state preservation across mapping-only manual Save and actual feedback-edit refusal; strict core/app all-target Clippy passed. |
| Explicit rewrite runtime | 11 focused tests passed: exact complete saved chat, proposal-only authority, Current inputs, stale rejection, target/metadata limits, mutable-read refusal, startup/retry, no synthetic owner message, no-proposal failure, receipt replay. |
| Default workspace | `cargo test --workspace --locked --offline` passed. Three existing ignored tests are not counted as evidence. |
| Full runtime | `cargo test -p brn-ai --features capability-spike --all-targets --locked --offline`: 216 library + 3 capability binary tests passed; one existing ignored test excluded. |
| Runtime lint | `cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings` passed. |
| Native widgets | Full native-test-support suite: 50 tests passed (48 component/state tests plus 2 desktop entry-point tests), including 9 dedicated writing tests: keyboard selection, separate chat/comment drafts, batch snapshot/retry, dirty/stale preservation, review conversation switching, addressed subsets, restart/Undo receipts, resolved-chat recovery, narrow layout access. |
| Optional retrieval integration | Full app tests with `--features native-retrieval` passed. |
| Default strict lint | `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed. |
| Combined desktop build | `cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline` passed. |
| Default build | `cargo build --workspace --locked --offline` passed; final workspace tests rerun passed after review corrections. |
| Native UI build/lint | Build and strict all-target Clippy with `--features native-ui` passed. |
| Native widgets lint/tests | Strict all-target Clippy and all-target tests with `--features native-ui,native-test-support` passed. |
| Combined lint | Strict all-target Clippy with `--features native-ui,native-retrieval` passed. |
| Combined tests | `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline` passed: 20 desktop state/entry-point tests. |
| Formatting and documentation | `cargo fmt --all -- --check`, `git diff --check` and Markdown links passed. |

Initial service/UI test runs failed compilation for missing new APIs; these do not establish a behavioral red test. Runtime test execution overlapped implementation and initially passed. Later focused failures and fixes are retained in local logs.

Existing dependency `block` 0.1.6 emits a future-Rust compatibility notice in native builds; it is not introduced by this change. No warning suppression or dependency change was added.

## Independent review

GPT-6.1 Sol read-only plan review approved before implementation. A separate GPT-6.1 Sol read-only code review approved the complete diff and invariants for PR publication, with no remaining blockers. Three P1 findings were fixed and regression-tested:

- Opening a proposal from another conversation restores the correct composer draft instead of sending the previous chat's draft to the wrong destination.
- A resolved selected conversation refuses new writing actions before posting, and New chat provides an explicit recovery path while retaining drafts.
- Mapping-only manual Save revisions preserve addressed feedback; changed feedback remains pending and confirmation checks current revisions.

Lead review also tightened the Undo/confirmation race, actual Apply receipt reconstruction, per-note Undo after restart, snapshot inspection, shared exact-range validation, narrow-pane access and removal of duplicate embedded passage cards. No architecture or stack rewrite was needed.

The independent review covered 22 changed/new files, including 14 Rust source/test files. Its final reviewed source-manifest SHA-256 is `4326b42d3133e5cca4671ef393bc1299e100f622e824652b403ae67a18035e33`; this includes two independently reviewed mechanical Clippy corrections. Documentation was finalized afterward. Final code is identified by the PR commit/tree, not by this self-referential evidence file.

## Qualification limits

No live provider/account request was made. Provider-shaped mock tool flows verify host restrictions, not actual model quality. Native widget tests are headless; real Mac typing, focus, IME, selection, pixels and responsiveness remain explicitly deferred, as does owner acceptance of the native implementation. No release or merge is part of this delivery. Hosted CI status belongs to the exact PR head and is reported on the PR.
