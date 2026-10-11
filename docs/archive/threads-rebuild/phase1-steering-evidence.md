# Phase 1 Stop-and-send evidence

> Historical evidence retained from [its original location](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/work/active/threads-rebuild/phase1-steering-evidence.md) at baseline `3a3aa7a`. Commands, task authority, pass counts and pending gates below describe that recorded work; consult [current status](../../status.md) for the present assignment.

## Authority and baseline

The owner explicitly authorized publishing/merging delegation after required CI and continued Phase 1 work without another routine approval. This increment follows the [reviewed steering plan](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/work/active/threads-rebuild/phase1-steering-plan.md). It uses isolated `brn-phase1-steering`, branch `work/phase1-steering`, baseline `36ad2d61e5895c9a564dcff8f6eec1053376245b` (PR #116 merge). Source directory `sources/` and the original checkout remain untouched.

## Delegation integration

[PR #116](https://github.com/ewq100/brn-rust/pull/116) merged after all four branch-required checks and documentation/tooling passed in [PR run 38136175465](https://github.com/ewq100/brn-rust/actions/runs/38136175465). Published tree was verified identical to reviewed local `e2f2e12`. Branch requirements were reread before merge. Non-required Windows Core/CLI failed on unchanged Unix-only credential APIs (`auth.rs`/`codex_auth.rs`); logs were inspected. Protection and checks were not modified.

[Post-merge run 38136490578](https://github.com/ewq100/brn-rust/actions/runs/38136490578) completed: all four required contexts passed again, alongside documentation/tooling and Linux native UI. Non-required Windows lanes and Linux native retrieval failed; overall CI is not all-platform green.

## Plan review

Independent GPT-6.1 Sol review: APPROVED. The plan explicitly addresses startup run identity, atomic message/fence persistence, terminal/newer invocation races, fresh authority, complete old-event draining, durable Continue recovery and offline native fixtures.

## Implementation and verification

Core/app/runtime and desktop changes have separate GPT-6.1 Sol owners. The shared writer now atomically saves the exact instruction and fences the exact run. Runtime preparation exposes only an opaque handle and durable Run ID; pre-cancelled/fenced/archived startup stops before credential access. Existing core apply already enforced nonarchived Run/Thread bindings; the new runtime checks close the earlier credential-startup gap. Desktop serializes the handoff, preserves drafts and provides receipt-ordered Continue recovery.

Core/app complete suites passed. Eight new core and two new app tests cover atomic fencing, checked-apply rollback, exact intent, lost-response replay/reopen, changed identity, terminal races and invalid destinations. AI `threads::` suite passed 44 tests, including five new runtime startup/fresh-scope regressions. Strict core/app/AI all-target Clippy passed. No live provider, real credentials or private data were used.

Commands (from the isolated worktree):

```sh
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo test --locked --offline -p brn-threads-core -p brn-threads-app
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo test --locked --offline -p brn-ai threads::
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo clippy --locked --offline -p brn-threads-core -p brn-threads-app -p brn-ai --all-targets -- -D warnings
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
```

Initial API tests failed because the APIs did not exist. The archived-destination runtime regression also failed behaviorally: the synthetic credential directory was opened before rejecting the archived destination; it now stays absent. Desktop component checks caught and fixed waiting-text overflow that obscured Stop. Native component evidence is recorded separately from actual native observation, which remains deferred.

Desktop final checks passed: 18 default tests and 34 native component/state tests (overlapping), including six focused native steering tests. Strict all-target Clippy passed for default and native-test-support; native UI build passed. Tests used synthetic worker channels, actual production persistence/dispatch, persisted unsupported providers and empty credential paths. They prove offline refusal/recovery and the handoff state transitions, not successful live provider replacement or observed native usability.

```sh
CARGO_TARGET_DIR=../threads-macbook-local/target cargo test --locked --offline -p brn-desktop
CARGO_TARGET_DIR=../threads-macbook-local/target cargo test --locked --offline -p brn-desktop --features native-test-support
CARGO_TARGET_DIR=../threads-macbook-local/target cargo clippy --locked --offline -p brn-desktop --all-targets -- -D warnings
CARGO_TARGET_DIR=../threads-macbook-local/target cargo clippy --locked --offline -p brn-desktop --all-targets --features native-test-support -- -D warnings
CARGO_TARGET_DIR=../threads-macbook-local/target cargo build --locked --offline -p brn-desktop --features native-ui
```

Independent GPT-6.1 Sol candidate review: APPROVED against `36ad2d6`. Reviewed full diff and all new modules/tests, including close/Quit pending cleanup and terminal Converted event handling. No actionable findings. Reviewer performed source review and diff checks; implementation helpers supplied test/build evidence. No production change followed approval.

## Remaining scope

Native observation is deferred. Transactional Threads upgrades/pre-upgrade backups, external backup destination and storage/recovery qualification remain outstanding; no old-BRN migration or release is included.
