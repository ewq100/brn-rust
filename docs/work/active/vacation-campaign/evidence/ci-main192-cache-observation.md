# PR192 → main Mac UI cache observation

**Finding:** the latest main Mac Native UI cache result is unresolved. The completed job metadata has timing but no cache evidence, and its log request failed while the overall workflow was still running. I did not retry or inspect other in-progress jobs. There is no measured basis to call this a cache miss or full hit.

The compared source is PR192 head `0cc1a4007e8e1bedd97f31e650e2446ac7373841`, merged as main commit `2658f8883b47f224b02ed428055efaaba1bb47a4`. The integration receipt records whole-tree equality (`evidence/pr192-integration.json:3-17`). The PR Mac UI job is run `39499647752`, job `118893584792`; the main Mac UI job is run `39502704935`, job `118904878361`.

| Observation | PR192 head | Resulting main |
|---|---:|---:|
| Queue | Not retained in the premerge receipt | 4 seconds (job created 21:46:01Z, started 21:46:05Z) |
| Job duration | 8m15s (21:21:15–21:29:30Z) | 7m14s (21:46:05–21:53:19Z) |
| `setup-rust` step | 1m42s | 1m29s |
| Prepare both locked helper graphs | 1m35s | 1m37s |
| Cache result | Exact full-key hit | Unknown; main log unavailable |

PR192’s raw job log records `save-if: false`, a full match for key `v0-rust-macOS-ARM64-native-ui-d9baa4d448b113de44d1e765f31756a4d46680b8efb1da913177d12d2337015b-native-ui-Darwin-arm64-4b070e83-bf6157b2`, and a compressed archive of `1,902,323,274` bytes (`~1814 MB`). The restore action took `76,057 ms`. Despite that full hit, the helper-preparation step ran for 95 seconds and compiled both stock and adapted helper graphs; the adapted Xberg build reported 74 seconds. A full cache hit therefore does not eliminate this preparation work in the observed PR job.

Main job metadata reports success and the timings above. Its log request returned: “run 39502704935 is still in progress; logs will be available when it is complete.” The log file is empty; the first failure is retained at `/tmp/brn-ci-main192-cache-observation/main-mac-ui-download-error.txt`. As required, I made no retry. Therefore no main cache key, hit/miss, archive size, restore time, or finalization result is claimed.

The earlier same-source warm main pair in `ci-cache-investigation.md:79-88` took 6m23s for the warm job and 1m23s preparing helpers. The current main job is 51 seconds longer than that reported job, but the earlier pair used different actual runners, and this observation does not have the main cache log. The PR192 full-hit job was 8m15s, with 1m35s helper preparation. These timings do not isolate cache behavior or support a speed guarantee. The existing investigation already identifies preparation costs and requires preserving both helper builds and regenerated checkout-specific adapted source; this observation adds no evidence for a safe work-elimination change. **No source or cache configuration change is proposed.**

Evidence and raw artifacts:

- Existing PR receipt: `docs/work/active/vacation-campaign/evidence/pr192-premerge.json` (SHA-256 `0d9c600792a02ea073e895612576b81dc967009f2472936c0e0dbaffc1f57db8`); Mac job step timestamps and conclusion are at lines 159–175 and later step entries.
- Existing merge receipt: `docs/work/active/vacation-campaign/evidence/pr192-integration.json` (SHA-256 `d99dd3f3d6af257837ff2a04591c8311cf2af84df315e5c515a66638007f1bfd`; whole-tree equality at lines 3–17).
- Existing cache analysis and comparator limits: `docs/work/active/vacation-campaign/ci-cache-investigation.md:79-88` and `:90-96`.
- Downloaded PR log: `/tmp/brn-ci-main192-cache-observation/pr192-mac-ui.log`, SHA-256 `f4477aed51a90c70ea8a3b38729b7d3c22b9496b6c540f4d0791c916e41acc6c`.
- Main job metadata response: `/tmp/brn-ci-main192-cache-observation/main-jobs.json`, SHA-256 `e2b3a7f61d7940887aa61dd8c9a789f1696eab9912ffb838356db51076ac43d8`.
- Compact underlying record: `/tmp/brn-ci-main192-cache-observation.json`.

Only one metadata request was made for the main run and two job-log requests were attempted (the PR log downloaded; the main log request failed once). No job reruns, cache operations, source/config edits, or billing/account actions occurred.
