# Desktop shell evidence — chunk 05

Date: 2026-09-28. Branch: `trial/desktop-shell`. Base: `199d8c55ed42b67b3a1e5d24f7539ed60b5e1a78`.

## Authorization and scope

The user replied “Ok” to the explicit architecture-approval/native-editor-deferral question. The architecture is accepted on that basis; GPUI remains provisional. This chunk implements only the shell/core sample-work boundary described in [the plan](desktop-shell-plan.md). No provider calls, embeddings, document import, authoritative database, account changes, merge or release are included.

Work remains in the existing isolated `/private/tmp/brn-editor-trial` checkout, on a new branch. Earlier pushed trial branches are preserved. The Mac was checked again through native computer use on 2026-09-28 and is still locked. No workaround was used; observed UI acceptance remains outstanding even if the executable builds.

Sol owns the core/desktop implementation and workspace manifests. Luna owns the verification script. The lead owns integration and documentation. A separate Astra review checked lifecycle and state behavior, followed by scoped re-reviews of fixes.

## Verification record

The final commands all exited 0:

```sh
bash scripts/verify-desktop-shell.sh
CARGO_TARGET_DIR=/private/tmp/brn-editor-trial/experiments/editor-trial/target CARGO_BUILD_JOBS=4 bash scripts/verify-desktop-shell.sh --native
PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh
```

The explicit target directory reuses cached GPUI compilation artifacts; it does not import editor source into the shell. Without it, the script uses the normal root `target` directory. The script passed workspace build, formatting, Clippy with warnings denied, **12 core tests**, **five CLI tests**, native build/Clippy/tests, and three headless scenarios: completion, cancellation and stale-result rejection. CLI smoke cases cover help, malformed/repeated options, surplus help arguments and missing/non-directory/relative data paths. All malformed path checks invoke the headless path so they exercise directory validation rather than merely missing native support.

The existing regression script passed the expanded lightweight workspace checks plus the original starter/provider builds, formatting, Clippy, **16 provider tests and nine CLI smoke cases**. No live or credentialed command ran. Adding a second binary initially broke bare root `cargo run -- --help` with ambiguous binary selection; workspace `default-members = ["crates/brn"]` restores that command. `--workspace` still covers all three crates; desktop launch explicitly selects `-p brn-desktop`.

The observed build uses Rust 1.98.1 on macOS 15.3.1 arm64 and GPUI Kit 0.6.6. Native shell linking succeeded (the initial cached native build took 10.20 seconds). The existing `block 0.1.6` future-Rust incompatibility advisory remains. Native build success is not a rendering or interaction result.

Core tests cover generation changes, stale completion/progress, unrelated operation identity, busy/reuse behavior, cancellation before and after a produced terminal result, bounded/coalesced mailbox behavior, close/drop, explicit long-wait wakeup, closed-shell rejection and oversized-edit recovery. The long-wait test observes a mutex-protected worker-wait signal before closing a worker configured to wait 30 seconds, then requires shutdown in under one second. Deadline-based tests avoid indefinite waits. The corresponding missing accessor was observed as a compile failure before the implementation; not every test had an independently observed behavioral red-green cycle.

CLI tests use newly created temporary directories, verify that write probes are removed, and exercise an unwritable directory on this Unix host. Explicit `--data-dir` must be absolute and already exist. Native launch alone may create the default `~/Library/Application Support/BRN`; `--help`, malformed input and a feature-disabled launch do not initialize that directory. Headless verification always supplies a disposable path. No authoritative data is written.

## Implementation and review

`brn-core` has no external dependencies or GPUI types. It owns one standard Rust worker thread and a mailbox holding at most one coalesced progress event and one preserved terminal event. Cancellation and terminal production share a mutex, so a completed terminal result is not retroactively reported cancelled. Cancellation wakes the worker's condition variable; close joins the owned thread. This bound applies to the deliberately small cooperative sample task, not arbitrary future providers or indexing work.

`brn-desktop` enables GPUI only through `native-ui`. Its window contains Workspace, Activity and Settings. A default sample task runs about three seconds so cancellation and editing can be exercised. Input is limited to 64 KiB for work; every attempted edit advances the working-copy generation and invalidates prior results. Oversized visible input blocks Start until shortened; Start also synchronizes visible text before submitting to guard against a delayed change notification. Transient operation IDs and state reset on process exit; durable UUIDs, idempotence and recovery arrive with storage work later.

The view owns a yielding 40 ms polling task using a weak entity, so it does not keep a closed window alive. A retained app-quit subscription explicitly closes the worker. A last-window-close subscription requests app quit. Astra verified the pinned GPUI APIs and ownership statically; actual close/quit behavior remains a native acceptance check.

Astra's important initial finding was that rejecting an oversized edit kept the old generation alive while displaying new text. That mismatch was fixed and covered by a regression test. Re-review accepted the fix, the race tests, explicit quit callbacks and default-package compatibility repair with no remaining Critical or Important source findings. Root manifests/lock and the verification script were included. Native acceptance is still deferred, not passed.

## Native acceptance checklist

After unlocking the Mac, launch the final `brn-desktop` binary with a disposable absolute data directory and verify:

- One window renders; Workspace, Activity and Settings navigation is usable.
- Sample input remains editable and navigation responds during the background task.
- Progress is visible; cancellation ends the active task and permits another start.
- Editing sample input during work prevents an old result from replacing the current result.
- Closing during work shuts down promptly; reopening starts cleanly.
- Selected directory and useful startup errors are visible; no original documents are modified.

These checks and the earlier editor acceptance remain unverified until performed. The shell does not integrate the editor, provider or retrieval experiments. Native packaging/signing and crash recovery for authoritative data belong to later work.

## Delivery boundary

This is an implementation checkpoint for chunk 05. Native shell acceptance remains open; architecture approval and the user's deferral are recorded separately. Existing experiment source files are unchanged. No merge, release, migration or provider/model account operation occurred. The final task response records the pushed branch and remote-verified commit.

Final preparation: the lightweight script was also run independently and passed, including the feature-disabled no-argument error path. Astra final documentation/source-evidence review approved the implementation checkpoint with no blockers. Minor stale architecture/CLI wording was corrected. Secret-pattern and local-link checks covered all 16 changed/authored files with no findings; all 852 root lockfile dependency sources are crates.io. `git diff --check` passed.
