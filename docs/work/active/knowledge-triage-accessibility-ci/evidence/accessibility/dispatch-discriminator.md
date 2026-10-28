# GPUI focus/dispatch discriminator

On 2026-10-28, one additional test, `threads_ui::pages_tests::chat_focus_dispatch_with_tasks_panel_and_split_enter`, passed on base `3252f940f100d8b9f245ce9028ca624c06690578` plus the test-only change committed with this receipt. The tested `threads_pages_tests.rs` SHA256 is `b257de30be6d453d75b5df7a4bc7137a1966d664217256d9c337a6c25e7b5f92`. Production source and locked dependencies were unchanged.

## Scope and result

The fixture mounts Chats and Tasks together, activates Recent using the test toolkit's pointer event pair, then traverses Recent → All → Archived → the implicit chat-row focus using Tab. No direct focus injection or production/test-only focus hook was added. It records the actual GPUI handle and contexts, sends Enter down and up separately, and repeats with an extra explicit redraw.

Both variants passed. The first route observed navigation handles 58/59/60, then handle 62; the second observed 158/159/160, then 162. The post-row-Tab context stack was `[Root, BrnWorkspace]`. Enter resolved no action and retained that context and focused handle; key-down left the chat unselected, key-up selected the expected synthetic chat, and neither step opened a menu. Records and history remained equal to their snapshots, with zero Run records and no worker.

This refutes a general deterministic Tasks-sibling/Popover dispatch leak in this fixture. It **does not diagnose or resolve** the preserved Linux AT/keyboard mismatch. No production fix follows from this result.

## Exact checks and preserved failures

Environment: sourced `/workspace/brn-environment/activate.sh`, then `/workspace/brn-environment/native/activate-native.sh`; overrode `CARGO_BUILD_JOBS=1` and `TMPDIR=/tmp/brn-accessibility-dispatch-fixtures` after creating that directory. This differs from the earlier lane environment (`/workspace/brn-batch-evidence/environment.sh` plus native activation) and rebuilt dependencies. No environment switch occurred during the run.

```sh
cargo test --locked --offline -p brn-desktop --features native-test-support chat_focus_dispatch_with_tasks_panel_and_split_enter -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Final focused result: **1 passed, 0 failed**, 380 filtered out, 0.21s runtime; final test compilation took 1m36s. Formatting and whitespace checks passed. This is one unique test containing two variants, not two additional unique tests. No full suite, lint or native actor was run for this test-only addition.

Raw logs remain in `/workspace/brn-next-tracks-evidence/accessibility/`:

- `dispatch-discriminator-attempt1.log`: compiler E0597 because the diagnostic loop passed a borrowed dynamic row ID to `window.find`; corrected by passing an owned string. No runtime test executed.
- `dispatch-discriminator-attempt2.log`: runtime failure querying the row snapshot's `.focused()`. Kit cannot observe pre-existing/implicit focus bindings and deliberately panics. Enter was not reached; this was an observation limitation, not reproduction of the native failure.
- `dispatch-discriminator-attempt3.log`: passing final test. It observes `Window::focused` and `Window::context_stack` without changing implicit focus behavior; expected row selection on key-up identifies the activation target.

## Limits

The test platform reports `a11y_active=false`; its public APIs do not activate the native adapter. The setup pointer event pair is not an AT-SPI action invocation. The earlier native `BRN` focus entry denotes the root Window node, whereas this test records the underlying GPUI handle. Kit cannot independently name the implicit row handle before activation; the successful row-selection callback provides the target evidence afterward.

The test event helpers already render before and after each individual key event. The redraw variant adds an extra frame; this does not compare a zero-redraw native event path. The fixture reproduces the relevant sibling panels and navigation sequence, not the entire native follow-up dataset, adapter, window system or observation timing.

Both original Linux failures and their invariance receipts remain preserved. Native chat selection and Capture Cancel/Escape qualification, static-text interface qualification, actual Mac review and owner acceptance remain unresolved gates.
