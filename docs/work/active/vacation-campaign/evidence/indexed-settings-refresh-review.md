# Independent changed-source Settings refresh diagnostic review

## Disposition and identities

**Approve the inert diagnostic source and adapter for the one selected changed-source observation, conditional on the lead confirming the shared build slot is free after the existing-check batch finishes. No actionable findings.** The stated busy slot `24422` was not probed, interrupted, or bypassed by this review. Approval is not an execution result or permission to merge the diagnostic fixture into product source.

Verified `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust` is clean, including untracked files, on `codex/indexed-settings-refresh-diagnostic`, exact HEAD `88c61df18cf6d09e7efc3a2d2349484645b2378a`. Its direct parent is approved product candidate `eae61dde04cda19c764246993966b13b997014eb`, based on actual main209 `5fab84d87f2cff3bc03f40fa5987c1fe73be98ab`. The complete parent-to-diagnostic diff changes only `crates/brn-desktop/src/threads_omakase_tests.rs`, adding 104 lines. Its SHA-256 is `e36c67a6d0b137aeb5ed5de97ce2cde02356cd800afa947a3e1966828b3ce6dd`.

I independently compared that entire diff byte-for-byte with the prior approved baseline diagnostic diff from main208 `11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1` to `be97dc0aeb4a441e12a4bb8193055eb6ece64024`: **identical**. The entire final test file remains SHA-256 `832d9e68810d6f52ea5eda317e301b8b574c04516a7ba6d16659339fb89a40ba`.

Both product files are byte-identical between the approved parent, diagnostic commit, and current working files:

- core `store.rs`: `2f9d8dd582b9e38ec8cbdb45baa24672917bd309dfc5ac96330d9b2513b864f6`
- app `settings.rs`: `0441aef20b72dd0d76bed8890f3bf8242efe109127624561968b230db2026aeb`

Thus the prior complete product review `/tmp/brn-indexed-settings-read-review.md` is reusable for these unchanged files. The original complete fixture review `/tmp/brn-desktop-refresh-scale-final-review.md` and adapter review `/tmp/brn-desktop-refresh-scale-runner-review.md` are reused after these actual equality checks, not merely on the author's description. No product-branch mutation was made by this review.

## Fixture and purpose

Read the complete actual diagnostic addition. It retains the existing real headless GPUI fixture and explicit offline/no-poll setup. It seeds 10,000 fixed short synthetic Notes as 20 checked owner batches of 500; it is a large-corpus witness, not a large-individual-Note/body or message-heavy witness. Constructor/setup/render/park occur before the timer. The only timed call is direct `view.refresh()`; the elapsed value is captured before assertions and reporting.

The accepted empty-notice/non-error-tone precondition and postcondition remain outside the interval. This prevents a refresh query error from being reported as a successful sample solely because constructor-loaded records remain. It asserts all 10,000 Notes loaded and reports record/history/candidate/buffer/asset/Topics/writing-request/applied-rewrite dimensions. Request-linked Apply remains explicitly unactivated. Correctness for this ordinary real UI consumer is useful additional qualification of the shared Settings read; existing broader Settings/core/app/desktop gates remain separate and cannot be replaced by this diagnostic.

No provider dispatch, model acquisition, account interaction, native OS app launch, private owner data, schema/write authority, cache, or query-policy change is introduced by the diagnostic. Its owner fixture writes remain checked synthetic initialization. Existing detached search preparation may consume CPU/I/O concurrently; this measurement does not isolate that background work. The direct timer excludes render-time `refresh_writing_review_cache`, whose actual caller at `threads_shell.rs:1642` was established in the corrected earlier assessment. Setup/update/render/park can still execute that helper outside the timer. The old no-caller statement is not reused.

## Complete adapter review

Read both entire inert files and independently compared their contents with the previously approved baseline adapters:

- `/tmp/brn-indexed-settings-refresh-qualification/run.sh`: SHA-256 `bbcf13cb7a85bbd1c2498fb08667c3175f4e85693d88ac475ad8fbf780b0e4cf`
- `/tmp/brn-indexed-settings-refresh-qualification/supervise.py`: SHA-256 `d07cd8c224568bd4a87af81291821a5001d6a7f02d597c3751db3d27c408299d`

The comparison confirms **only the expected task paths, candidate HEAD, and recorded base strings changed**. Execution, observation, cleanup algorithms, source guards, resource bounds, and log acceptance are identical. The runner retains pinned Rust 1.98.1/offline/jobs1 test-opt1 configuration and unchanged exact one-test filter, fixture-file hash guard, clean HEAD guard, >3 GiB initial free-space check, and task-specific synthetic TMPDIR. The supervisor retains the 30-minute trigger, sampled 1 GiB floor, owned new process group, bounded TERM/KILL and independent reap/final group verification, error recording, post-run unchanged-checkout verification, and no overlay/source restoration writes.

Qualification still requires successful process exit, resolved/reaped ownership, no recorded cleanup errors, exact unchanged candidate/checkout, accepted log <=8 MiB, exactly one diagnostic marker, an actual `1 passed; 0 failed` harness summary, and durable receipt persistence. Persistence failure sets qualified false and returns failure. Sampled free-space/deadline checks and accepted log size are not hard OS disk/runtime quotas. Reusing this source-reviewed adapter does not experimentally prove exceptional cleanup paths. The shared target requires serialized ownership and attention to actual compilation/source identity in the resulting log; no source-byte comparison alone proves a reused binary's identity.

## Interpretation and stopping boundary

The baseline remains exactly one actual sample on `be97dc0a`, based on main208. The proposed observation is one distinct changed-source sample on `88c61df1`, based on main209 plus approved Settings reuse. These are not identical base trees. Independently checked baseline-to-diagnostic differences are the intended Settings pair plus main209's provider-test helper and writing-ui Note guard/README/host-journey changes. The whole desktop fixture, `threads_ui.rs`, `threads_shell.rs`, root `Cargo.toml`, `Cargo.lock`, and toolchain file are identical between the two diagnostics. This supports consistent method/profile, while retaining the nonidentical-base disclosure rather than claiming a controlled causal experiment.

If the selected run passes, report its duration and actual dimensions alongside the original 80.819 ms sample as two source-bound observations. A lower value is not a statistically established speedup, a guaranteed user latency, or proof that all difference came from Settings reads. A higher value is likewise not by itself a product regression under an invented threshold. No native/full-render/startup or latest-main result follows. Do not repeat, scale, or rerun until green merely to improve a duration. Source failure or mismatched dimensions should preserve the failure and prompt bounded diagnosis, not silent adjustment.

The current task authorizes one useful changed-source observation after existing qualification and slot release; this review launches nothing. I performed only read-only source/Git/hash comparisons and wrote this report. No builds, tests, fixtures, workloads, network, account/native actions, source changes, or nested helpers were performed. The full product checks and this distinct diagnostic outcome remain pending and separately attributable.
