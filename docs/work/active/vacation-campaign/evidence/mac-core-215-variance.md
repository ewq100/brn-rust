# Mac Core timing pair: PR215 vs resulting main214

## Result

The PR215 Mac Core job took **20m17s** versus **13m00s** for the immediately preceding main214 job, a **+7m17s** difference. This pair does not show a cache hit-versus-miss explanation: both jobs had the same computed key, restored the same 1,327,265,224-byte archive from the same fallback key, and both reported `full match: false`. PR215 was slower across most Cargo work, but the logs expose no runner CPU/load metrics. The defensible conclusion is observed build/test timing variance on different runner instances, with its cause unproven; there is no evidence here for a source or dependency regression.

## Identity and source equivalence

- Main214: run `39766894309`, job `119784472572`, head `e7a218fd33489f7a0f55c96d9ef6f5cd0626f284`, `Core and CLI (macos-15)`, runner `GitHub Actions 1000003792`; job 10:56:58–11:09:58Z, success.
- PR215: run `39767073233`, job `119785071964`, head `80ea524e8fd214649b162f66d7eb12a7e33ec206`, same job/label, runner `GitHub Actions 1000003796`; job 10:58:28–11:18:45Z, success.
- The verified publication receipt `docs/work/active/vacation-campaign/evidence/pr215-publication.json` pins base/head and says the full diff changes only `docs/development/tooling.md` and `docs/development/verification.md`; it records both reviewed document blobs unchanged from the qualified docs source. I also checked GitHub’s exact base-to-head compare API response: `ahead`, 2 commits, and exactly those two documentation paths. No production source, manifest, lockfile, or workflow changed between the compared revisions.

## Measured differences

| Step | main214 | PR215 | Difference |
|---|---:|---:|---:|
| Rust/cache setup | 44s | 67s | +23s |
| Prepare both document helper graphs | 94s | 179s | +85s |
| Build default workspace | 24s | 48s | +24s |
| Lint default workspace | 22s | skipped | −22s |
| Test default workspace | 291s | 455s | +164s |
| Test/lint both document helpers | 133s | 225s | +92s |
| Scripted document CLI route | 23s | 77s | +54s |
| Provider capability fixtures | 85s | 133s | +48s |
| Provider fixture lint | 9s | 17s | +8s |

Cargo’s reported `Finished ... profile ... in` durations sum to **579.65s** across 13 commands on main214 and **986.26s** across 12 on PR215. These are elapsed command durations, not CPU-time measurements; they locate much of the delta in compilation/check work. Harness summaries show the same 115 suites and 1,859 passing / 0 failing / 7 ignored tests in each run; summed per-suite `finished in` values are 92.44s and 125.94s, respectively (+33.50s). Those suite sums can include parallel test execution and are not a substitute for job wall time.

## Cache evidence

Both logs report the same primary key:

`v0-rust-macOS-ARM64-default-d9baa4d448b113de44d1e765f31756a4d46680b8efb1da913177d12d2337015b-headless-Darwin-arm64-4b070e83-6c13ef0f`

Both say `Cache hit for restore-key` and restore key ending `...-4b070e83-bf6157b2`, then report `full match: false`. Both restored exactly 1,327,265,224 bytes (~1,266 MiB). The PR transfer/restore took about 48s versus 32s on main, a measured ~16s difference but far smaller than the 437s job delta. Main’s post-job cache action saved a new cache entry; PR’s post action completed in under a second without a save. Neither post-action result explains the elapsed work already done within its job. Cache state was materially alike at restore time; this is not the prior exact-key-hit versus miss pattern.

## Assessment and limit

The PR job ran with the same production tree and same restored fallback archive, yet its compiler/check durations and test-suite time were higher across many stages; its default-workspace lint was skipped, which partially offsets its longer total. Different runner IDs and the restore transfer rate are consistent with machine/host performance variance, but do not prove it: runner resource telemetry, cache contents’ effective reuse per target, and compiler CPU measurements are unavailable. This pair does not justify code changes, workflow edits, or cache operations. **No optimization selected.** Continue to distinguish job timing from test-body timing and reassess only if repeated same-tree jobs show a stable, attributable hotspot.

## Preserved bounded evidence

Raw logs and API metadata are retained under `/tmp/brn-mac-core-215-variance/` (all fetched in about 5.0s; each log is below 8 MiB):

- `pr215-log.log`: 334,333 bytes; SHA-256 `0f0a310e9c2d88cad8d9469d5c18bc4e1f1e9ca5ece38a026e9943d63c476d52`
- `main214-log.log`: 341,069 bytes; SHA-256 `2fbaeec887261e546a837920a894216d1586dbd9e63a594db64d74f9994f936e`
- Run/job metadata JSON for both plus `main214-pr215-compare.json` are saved alongside the logs.

No job was rerun; no source, build, cache, or account state was changed. The run-level conclusions are outside this two-job timing comparison.
