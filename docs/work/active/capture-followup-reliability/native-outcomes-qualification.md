# Combined Linux outcomes observation — 2026-10-26

Source `ef09669e` combines the reviewed Capture/schema13, follow-up views, Search transitions and reliability slices on baseline `bc699c41`. This is local integration evidence, not a merge or a substitute for each draft PR's exact-head hosted CI. [Raw evidence and hashes](evidence/native-outcomes-20261026/manifest.json) identify the source, three immutable copied binaries, scripts, screenshots and AT-SPI trees.

## Deterministic integration

The final combined desktop native-test-support suite passed **377 tests, zero failures, one ignored** in 59.48 seconds. Desktop/CLI native binaries and the synthetic fixture example built successfully. The CLI upgrade suite passed all four tests against the combined schema13 source. The prior source without the schema13 gate ran **371 passed, six failed, one ignored**: document-worker tests lacked the existing qualified helper environment variables. With those variables set, the same test binary passed eight document tests and one inbox test, including all six failures. These nine are a targeted recovery run, not nine additional unique tests. Both original and corrected logs are retained. No helper, assertion or product guard was weakened.

## Actual window observation

A fresh synthetic workspace ran in an isolated Linux Xvfb/Openbox session using software rendering. No user folder, picker or portal was invoked. The fixture uses an offline provider, no credentials, one posted chat, two saved Notes and five checked Actions. The sequence never clicks Propose outcomes, Send, Continue, Confirm done or Archive.

- The [Tasks screenshot](evidence/native-outcomes-20261026/02-followups.png) shows yesterday under Overdue, today under Follow up today, tomorrow under Upcoming, and separate undated Waiting/Open groups. Due dates intentionally disagree with follow-up dates and do not change grouping. Local date/as-of disclosure and Waiting on text are visible.
- The [Capture confirmation](evidence/native-outcomes-20261026/04-capture-confirmation.png) visibly states General scope, one captured posted message, model use, proposal review and separate archiving. AT-SPI invoked the Capture button and Cancel successfully.
- The [post-Cancel screenshot](evidence/native-outcomes-20261026/05-capture-cancelled.png) shows the dialog dismissed. Fresh CLI reads confirm records, history and candidates exactly unchanged, zero Runs, and no persisted Capture request or archive.

The first sequence stopped because the rendered chat row had no actionable exact-name AT-SPI match. It did not reach Capture. After inspecting the screenshot, the second sequence opened that sole synthetic row using a bounded pointer click at the observed fixed-layout location. This is a pointer interaction result, **not accessibility acceptance**. Group headings also do not all appear in AT names; the visual associations were inspected directly. Both scripts and the first failed probe artifact are retained. No product change or hidden retry was used to claim an AT pass.

## Remaining boundaries

This proves a narrow actual Linux grouping/open/Cancel flow. It does not qualify model selection/deduplication, model dispatch, actual Mac visuals, focus, accessibility, native pickers or owner acceptance. No new live Runs occurred; the previous narrow Mac evidence remains thirteen cumulative Runs. Historical cloud credential-boundary results (355 passed / 156 failed / one ignored) are not replaced by this native subset.

The reliability slice's corrected head `0fe6187fefe987483a1e170ae9271fbf4009d3b2` has terminal [CI40337695045](https://github.com/ewq100/brn-rust/actions/runs/40337695045): six supported Mac/Linux/documentation jobs passed, optional Windows core failed. The initial seeder failure and correction remain in PR232. Other current gates and publication state belong to [the task record](record.json).

The full Python tooling run executed 213 tests: 211 passed, one failed and one skipped. The process-group timeout test reported that its grandchild still existed after termination; the assertion uses `kill(pid, 0)`, which also succeeds for zombies. Read-only inspection found only zombie sleep processes in the two correlated groups, adopted by this environment’s non-reaping PID1 (`tail`), with no live members. The failed test’s exact child PID was not retained, so this is correlated environment evidence rather than an exact retrospective PID match. No product leak is established; the test is unchanged and normal hosted CI remains the full-suite gate. This local failure is preserved separately from hosted exact-head CI and is not reported as a tooling pass.
