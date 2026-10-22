# Main 209 Mac Core CI timing and cache assessment

## Result

The completed main209 Core job again missed the exact macOS cache key that main208 had restored. This is a repeated availability pattern, not evidence of why the entry disappeared. Main209 then successfully saved a fresh cache with that key. The cache miss and much longer compilation-oriented steps explain the timing difference far better than test-body duration; runner CPU/load remains unmeasured.

## Job evidence

Repository: `ewq100/brn-rust`.

| | Main208 | Main209 |
|---|---|---|
| Run / selected job | `39722288570` / `119638268861` | `39726448605` / `119651335348` |
| Source SHA | `11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1` | `5fab84d87f2cff3bc03f40fa5987c1fe73be98ab` |
| Selected job | Core and CLI (`macos-15`), success | Core and CLI (`macos-15`), success |
| Job interval UTC | 04:25:27–04:36:55 | 05:10:00–05:45:49 |
| Elapsed | 688 s (11m28s) | 2,149 s (35m49s) |
| Runner | GitHub Actions `1000003680` | GitHub Actions `1000003693` |

Both selected Core jobs completed successfully. Both enclosing workflow runs report failure, so this assessment makes no claim about their other jobs or overall merge gates.

The action logs show the identical cache key:

`v0-rust-macOS-ARM64-default-d9baa4d448b113de44d1e765f31756a4d46680b8efb1da913177d12d2337015b-headless-Darwin-arm64-4b070e83-bf6157b2`

Main208 reports an exact full-match restore of 1,327,307,246 bytes. Main209 reports `No cache found` 287 ms after the restore attempt began. Its post-job action successfully sent all 1,327,265,224 bytes for a new cache; the repository cache listing captured after completion shows the exact key on `refs/heads/main`, created at 05:45:45Z, size 1,327,265,224 bytes. The new entry’s timestamp follows the miss and aligns with the successful post-job save.

This repeats the earlier main206 miss. The existing main207 observation records a successful restore of the same exact key, and main208 also restored it. Thus the observed sequence is hit (main207), hit (main208), miss then save (main209), with an earlier miss (main206). Existing evidence does not establish whether deletion, eviction, retention, concurrency, or another cache-service effect caused the intermittent absence.

Because the repeat-miss condition was met, I read the current repository cache-list and usage metadata. The usage endpoint reported 11 active caches totaling 11,062,576,769 bytes; the list endpoint returned 8 entries and included the newly saved exact key. These are current aggregate snapshots, not historical state at main209 lookup, and the count discrepancy means they do not establish quota pressure or eviction as the cause. I did not inspect other repositories or alter cache state.

## Timing attribution

| Step | Main208 | Main209 | Change |
|---|---:|---:|---:|
| Setup Rust and restore cache | 39 s | 22 s | −17 s |
| Prepare both locked document helper graphs | 102 s | 775 s | +673 s |
| Build default workspace | 12 s | 116 s | +104 s |
| Lint default workspace | 13 s | 57 s | +44 s |
| Test default workspace | 252 s | 546 s | +294 s |
| Both document helpers: API/restrictions test and lint | 134 s | 412 s | +278 s |
| Scripted document CLI | 24 s | 27 s | +3 s |
| Offline provider capability fixtures | 90 s | 124 s | +34 s |

The largest directly relevant compile signal is the helper preparation step. Main209’s log contains two unoptimized dev-profile helper builds of 6m11s and 6m32s; main208’s two corresponding finishes were 48.34s and 47.19s. Main209 also reports 1m56s default-workspace dev build, 56.43s default lint, and 7m29s optimized default workspace test-profile build. The job’s compilation, lint, test-profile compilation, and helper-verification steps dominate the added elapsed time.

Both logs contain 115 Cargo test-suite result summaries. Summed reported suite durations are 94.58s for main208 and 112.09s for main209. Main209 breakdown is 93.26s default workspace, 1.17s helper tests, 0.59s scripted CLI, and 17.07s provider fixture suites. The main208 log labels its test output `UNKNOWN STEP`, so its aggregate is taken over all 115 test-result lines. These sums are harness-reported and may include parallel suites; they are not CPU time or wall-clock job duration. Their roughly 17.5-second increase cannot account for the 1,461-second elapsed difference.

Both logs show Rust/Cargo 1.98.1 for `aarch64-apple-darwin`, `CARGO_INCREMENTAL=0`, and the same key dimensions. The GitHub compare API reports eight commits and four changed paths: provider stderr tests, writing UI README, a writing UI example journey, and `writing-ui/src/note.rs`. No Cargo manifest or lockfile is changed in that compare, so no dependency-graph input difference is evidenced. Source changes may affect compiled units, but the logs do not quantify that contribution separately. The distinct runner IDs and lack of host-load/CPU allocation measurements leave runner variance possible but unproven.

Prior `mac-core-ci-variance-20261022.md/json`, `main207-mac-core-cache-observation.json`, `mac-core-test-body-triage.md/json`, and helper-cache evidence are preserved as context: they separate test-body timing from compile work and previously observed the same key’s hit/miss behavior. The new run strengthens the availability observation but does not resolve its cause.

## One next action

Keep workflow/cache configuration unchanged. On a later naturally occurring main Mac Core job, compare this exact key’s lookup with the post-job cache record again; the present evidence supports monitoring that boundary, not an optimization or a claim that cache pressure caused the miss.

## Preserved evidence

Raw logs and JSON metadata are in `/tmp/brn-main209-mac-core-evidence/`; each raw log is below 1 MiB. SHA-256:

- `main208-job119638268861.log`: `6852cf9ae75ddcbc3b1b1a176c151c0e1f40a16168a383c4f12df3a4e3723a1c`
- `main209-job119651335348.log`: `709ca4fd3ed50c5da5f215a9ffa38ae1286040b473ddc666e71b9323bae5c48f`
- `run39722288570-jobs.json`: `ccadea8a48fdf97006d91103ebde3420a929cdfa6c9a0d837b6038001be433f9`
- `run39726448605-jobs.json`: `ac72cad14cdc51a66f1eeae22ddb23223d792fe84f69f074eb6e0a54b4e3d4dc`
- `repo-cache-list.json`: `733418ddced059481d562502215eec9bf354a2a3b83a43221f570b502c73ade4`
- `repo-cache-usage.json`: `ed4334d8b3d5181de6d690257ed357dc8b9a06844f89b470fc9678c6783e2b32`
- `compare-main208-main209.json`: `fa4f84d72a301f0f0dae15700a70cd00fad218ae61952216df4a90a719082eef`

Read-only commands used: `gh api` for the two run/job records, exact-repository cache list and usage, and source compare; `gh run view <run> --job <job> --log` for each completed log. No cache operations, reruns, builds, or local workloads were performed. The local checkout was not changed; its existing lead-owned campaign/overview updates remained untouched.
