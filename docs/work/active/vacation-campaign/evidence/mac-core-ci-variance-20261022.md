# Mac Core CI timing comparison — 2026-10-22

## Result

The longer job is substantially explained by a cache-state change, not by a demonstrated source or test-body increase. The two jobs computed the same macOS Core cache key. Main205 restored an exact 1,327,152,455-byte cache hit; main206 reported **“No cache found”** for that identical key. Main206 then rebuilt both locked document helper graphs and the default workspace. Its post-job cache action successfully uploaded 1,327,307,246 bytes. Why the earlier entry was unavailable to main206 is not established by these logs. CPU/runner variance is also not measurable from the available metadata.

## Job identities

| | Earlier completed lane | Later completed lane |
|---|---|---|
| Run / job | [39712960408 / 119608907840](https://github.com/ewq100/brn-rust/actions/runs/39712960408/job/119608907840) | [39715111426 / 119615626870](https://github.com/ewq100/brn-rust/actions/runs/39715111426/job/119615626870) |
| Source SHA | `0016b21057b44fa9a64cef2d77d4ae09e317f540` | `79fa3980c5022e475b07888d9e08f396f45b0238` |
| Job interval UTC | 02:45:37–03:01:34 | 03:08:48–03:39:19 |
| Job elapsed / result | 957 s / success | 1,831 s / success |
| Runner label / ID | `macos-15` / `GitHub Actions 1000003627` | `macos-15` / `GitHub Actions 1000003640` |

The parent workflow run for the earlier job was later cancelled; the selected Core job itself completed successfully. The later parent run concluded failure outside this selected successful Mac job. This comparison does not reinterpret the other required checks or the lead’s current integration status.

GitHub’s compare API reports four commits ahead and only one changed file, `crates/brn-threads-core/src/restore.rs`; its patch is confined to the Unix test module/process witness. No manifest, lockfile, workflow, or production code changed between these sources, so a dependency-graph or production compile-scope change is not evidenced.

## Cache and toolchain evidence

Both job logs report the same computed key:

`v0-rust-macOS-ARM64-default-d9baa4d448b113de44d1e765f31756a4d46680b8efb1da913177d12d2337015b-headless-Darwin-arm64-4b070e83-bf6157b2`

Main205 reports a full exact-key match, successful restore, and ~1,266 MB (1,327,152,455 bytes). The restore action took 27,686 ms. Main206 reports “No cache found” 962 ms after its cache lookup began. Its post-job action cleaned the root and both isolated helper targets, then uploaded 1,327,307,246 bytes; the action finished successfully in 41,132 ms. This confirms the later run repopulated the same scoped archive, but does not establish why it was missing at lookup or how long it will remain available.

The shared setup action scopes the macOS cache to root `target` plus `.brn-ci-document-helpers/{stock,adapted}` and keys it with the lane, compiler/host/toolchain/lock dimensions plus helper inputs. Both logs show rustc 1.98.1 and Cargo 1.98.1. Cargo reports the same profiles: default build/lint `dev [unoptimized]`, default workspace tests `test [optimized]`, and the isolated helper test/lint builds `test/dev [unoptimized]`. Both run with `CARGO_INCREMENTAL=0`. The cache key and profile inputs are unchanged.

## Where elapsed time moved

GitHub job step timestamps show a 874-second increase (14m34s). The largest increases were:

| Step | Main205 | Main206 | Change |
|---|---:|---:|---:|
| Prepare both locked document helper graphs | 124 s | 637 s | +513 s |
| Build default workspace | 18 s | 124 s | +106 s |
| Lint default workspace | 26 s | 63 s | +37 s |
| Test default workspace | 328 s | 445 s | +117 s |
| Both document helpers: API/restrictions test and lint | 206 s | 343 s | +137 s |
| Test provider capability fixtures offline | 138 s | 112 s | −26 s |
| Setup Rust and restore cache | 48 s | 23 s | −25 s |

Cargo’s own finish reports sharpen the attribution. In helper preparation, the two unoptimized dev builds went from 53.83 s + 62 s to 5m22s + 5m03s (625 s total, +509 s), closely matching the step’s +513 s. Default workspace build went from 17.44 s to 2m04s; lint from 26.11 s to 1m02s; optimized workspace test-profile compilation from 3m52s to 5m42s. The helper verification step also spent longer in dev-profile checks (about 44 s total to about 222 s); its unoptimized test-profile builds were not slower in aggregate.

Harness output does not show a corresponding increase in test-body time: both runs report 106 workspace test suites, with summed per-suite `finished in` values of 90.90 s and 98.43 s (+7.53 s). The six helper suites sum to 1.47 s and 1.01 s; the scripted CLI test is 0.60 s and 0.64 s; the provider capability suites are 18.81 s and 16.01 s. These are harness-reported elapsed sums, not CPU measurements; tests inside each binary may run concurrently. They nevertheless show that test-body durations do not account for the 874-second job delta.

## Interpretation and limits

The cache miss is the strongest measured explanation: it coincides with 10m25s of helper graph builds, a 2m04s workspace build, slower lint/test compilation, while the observed test harness times remain close. Some step differences do not map one-for-one to compilation, and the logs do not expose runner CPU allocation/load, so a residual runner contribution remains possible but unproven. The two runs are a small sample, not a benchmark or future timing guarantee.

This aligns with earlier evidence: `ci-cost-triage.md` found no safe lane/test/cache simplification; `ci-main192-cache-observation.json` warns that one full helper-cache hit still compiled both graphs and that cache retention is not a speed guarantee; `ci-helper-cache-qualification.json` did not establish future cache availability; and `mac-core-test-body-triage.md/json` separates Cargo compilation from harness time. The current completed logs resolve this pair’s cache hit/miss directly, without changing those prior limits.

**One next action:** Keep the workflow and tests unchanged. If a naturally occurring subsequent main Mac Core job again misses this exact key, inspect cache availability/eviction for that key before attributing the remaining variance to runner speed or proposing an optimization. No rerun or cache operation was initiated.

## Preserved evidence and commands

Raw completed job logs and GitHub run/job/compare metadata are retained in this directory. The logs are uncompressed because they are below 1 MB. SHA-256 values are also in `metadata-summary.json`.

Read-only acquisition commands:

```text
gh api 'repos/ewq100/brn-rust/actions/runs/39712960408/jobs?per_page=100' > run39712960408-jobs.json
gh api 'repos/ewq100/brn-rust/actions/runs/39715111426/jobs?per_page=100' > run39715111426-jobs.json
gh api repos/ewq100/brn-rust/actions/runs/39712960408 > run39712960408.json
gh api repos/ewq100/brn-rust/actions/runs/39715111426 > run39715111426.json
gh run view 39712960408 --job 119608907840 --log > main205-job119608907840.log
gh run view 39715111426 --job 119615626870 --log > main206-job119615626870.log
gh api repos/ewq100/brn-rust/compare/0016b21057b44fa9a64cef2d77d4ae09e317f540...79fa3980c5022e475b07888d9e08f396f45b0238 > compare-main205-main206.json
```
