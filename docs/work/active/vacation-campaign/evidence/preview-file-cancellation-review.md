# Independent review: preview-file cancellation

Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`.
Branch: `codex/preview-file-cancellation`.
Reviewed frozen candidate: `a054232c8fa5ddf019263823f83753b9a67013f9`.
Verified base/merge base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`.
The checkout was clean. Reviewed the complete three-file diff: `input.rs`, the `project-delete` path in `main.rs`, and `agent_bounds.rs` (60 additions, 6 deletions).

## Findings

No actionable findings.

The change places the potentially blocking preview read behind the existing cancellation/channel mechanism without introducing a second reader policy. `read_limited` now delegates its unchanged bounded-text read to generic `read_cancellable`; the original cancellation precheck, 25 ms receive polling, channel-disconnection error and result forwarding remain intact. Text/token byte limits, UTF-8/NUL checks, terminal handling, stdin handling and existing error classifications are unchanged.

`read_preview_file` deliberately retains raw `std::fs::read` and the previous `invalid_input` / `preview-file` / `unreadable` mapping. The caller retains `serde_json::from_slice` and the previous `invalid_json` mapping. It does not apply text normalization, UTF-8/NUL preprocessing, stdin interpretation, a byte-size ceiling, metadata/type restriction, symlink refusal or absolute-path requirement. Relative paths still resolve against unchanged process cwd, symlinks are followed as before, `-` is still a literal filename for this option, and a readable FIFO remains a supported byte source. No new numeric policy is invented.

The reader thread captures only the owned path/read closure and channel sender; it has no Workspace, operation key, authority or write callback. A soft cancellation error exits before the complete ProjectCommand is constructed and before `Workspace::project_command` binds/allocates a key or performs deletion. The CLI's existing finish/restamp path translates the interrupted error to SIGINT, SIGTERM or deadline results and retains the command key/retry classification. No receipt/transaction/preview binding logic is changed. The existing workspace-open boundary is retained, so a previously absent workspace can still have been initialized before a preview-input refusal; the test and baseline use an already-created workspace and do not claim otherwise.

The read thread is detached, as in the original `read_limited` implementation. A cancellation does not forcibly interrupt its blocked FIFO system call or join it. That is acceptable for this one-command CLI: after returning the soft error, the program reports its outcome and exits, which terminates the reader with the process. It cannot subsequently publish a deletion. This is not a reusable long-lived background reader or a guarantee that an OS read itself was cancelled. At most one preview-reader thread is added for this command. Existing channel-result/cancellation race semantics are preserved rather than strengthening the contract into transactional cancellation of every finite read.

## Evidence and witness effectiveness

Read AGENTS, the independent-review skill and previously loaded resource/Rust guidance, the full resulting `input.rs`, complete changed call path and its surrounding project-command dispatch, signal/watchdog/restamp/output handling, shared project service/key binding and checked Delete path, relevant CLI README contracts, the process-test support/Root/FIFO helpers, adjacent timeout/SIGINT/SIGTERM tests and existing preview unreadable/malformed JSON tests.

The source-bound baseline receipt `/tmp/brn-preview-cancellation-qualification/baseline-reproduction.json` names base `9e2a81c0`, its compiled binary hash, the actual keyed FIFO invocation, 4.026 seconds observed runtime, timeout/exit 7 with `hard: true`, and unchanged row counts/hashes for every recorded canonical table. This supports the stated defect: the synchronous pre-effect preview wait ignored the safe cancellation point and reached the watchdog's unknown-state response. The receipt does not imply the baseline performed a deletion.

The new Unix process witness uses the built CLI and a fresh canonical isolated root with HOME/CODEX_HOME/temp paths scoped to that root. It initializes a disposable workspace, snapshots public records/history, creates a fresh 0600 FIFO without a writer and invokes the real keyed project-delete path with a one-second deadline. It checks actual exit/envelope agreement, timeout classification, deadline seconds, `hard: false`, deadline stop reason, and unchanged records/history. The generous elapsed upper bound catches a hang without relying on precise scheduler timing; `hard: false` is the substantive discriminator from the observed four-second hard-stop baseline. Expectations are literal public-contract values, not computed by the new helper.

The test reaches file opening and a pre-effect wait without needing a valid preview or real project, which is appropriate to this defect. It does not assert successful FIFO-delivered JSON deletion, symlink/relative preview success, preview-size policy, direct SIGINT/SIGTERM on this exact option, or full internal-table equality. Those limitations do not invalidate the targeted witness: unchanged raw-read code preserves input acceptance/error semantics, existing generic-reader signal tests cover the reused cancellation mechanism, and source inspection verifies no project operation is allocated on the error return. Required final process/contract checks remain lead-owned.

## Checks and limits

`git diff --check 9e2a81c0 HEAD` passed. Reviewed file SHA-256 values:

- `input.rs`: `e1815839e8ab4a7694d0fe7071bf0698808538709047ea53efc479db18a36e18`.
- `main.rs`: `10b31f0483991faf64d2033b06f1803022629e2abbcb7d377c1108051cb798e9`.
- `agent_bounds.rs`: `b58cddc0aca1e97b2615e3ff992099f5766b9bbcef5d3bca79816f9417922f2a`.

No candidate build, test, lint, signal experiment or replay was launched by this reviewer; qualification was pending and is not inferred from source review. No repository/shared-record edits, provider/account/network/native actions or nested work were performed. Only this requested review artifact was written. No native usability, owner acceptance, integration or release is claimed. Lead owns completed checks and integration; material candidate changes require affected review evidence to be refreshed.
