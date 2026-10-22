# Desktop refresh scale diagnostic candidate

Candidate commit: `05e8e54406b56c5d2bbfd42a0de7bcd7c83bf748` (`Measure desktop refresh at 10k notes`), based on clean `main208` `11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1`. Only `crates/brn-desktop/src/threads_omakase_tests.rs` changed.

Added `ten_thousand_notes_report_headless_refresh_duration` using the existing offline `fixture` and `update` helpers. Fixture setup writes 10,000 Notes in 20 checked owner batches of 500, with a fixed short synthetic body. After `ThreadsUi::new` and fixture render/park work, the test times only a direct `view.refresh()` call. It asserts only the refreshed Note count; it introduces no latency bound. A single parseable diagnostic line reports elapsed microseconds, Notes/records/receipts, candidates, recovery buffers, asset count, Topics configuration, writing-request/applied-rewrite counts, and the profile label `headless_gpui_refresh_only` with `native_interaction=unobserved`.

This ordinary fixture has no request/Apply pair and prints `request_apply_path=unactivated`; it also reports actual candidates, recovery-buffer, asset, and Topics counts rather than implying they were populated. The existing `ThreadsUi::new` starts background search preparation outside the timed interval; setup and render/park time are excluded, but the later measurement is not a physical-host UI or interactive latency qualification. The test uses no provider or live account path. It does not claim native rendering, OS event-loop, scrolling, IME, accessibility, or owner-perceived performance.

Validation performed: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-desktop/src/threads_omakase_tests.rs` and `git diff --check` passed. No build, test, benchmark, fixture execution, provider, network, account, or native app was run. The lead owns execution and measurement disposition. Local worktree is clean at the candidate commit; nothing was pushed.

The selected task ID renderer lookup was attempted in this checkout and returned `unknown task ID: desktop-refresh-scale-diagnostic`; the lead-provided selection remains the authority. No shared record or overview was edited.

## Independent review correction

The initial candidate `05e8e54406b56c5d2bbfd42a0de7bcd7c83bf748` and its report are preserved as history. Independent review identified that `refresh()` reports read failures by setting the UI notice rather than returning an error; checking the 10,000-note count alone could therefore accept stale constructor-loaded rows as a valid timing sample. The test now requires an empty startup notice and non-error tone immediately before the timer, and an empty notice/non-error tone immediately after the timed method call. These checks are outside the `Instant` interval, so the timed interval remains only `view.refresh()`.

The fixture still uses `ThreadsUi::new`, which starts background `prepare_search`; `cx.run_until_parked()` around the fixture does not prove its detached search worker is idle. The measurement excludes fixture/setup/render/park durations and measures the direct synchronous method, but concurrent background CPU work may affect that sample. It is not an isolated-CPU benchmark, full startup measurement, or native responsiveness qualification. No native app or owner input is claimed.

Final local candidate: `be97dc0aeb4a441e12a4bb8193055eb6ece64024`, with only `crates/brn-desktop/src/threads_omakase_tests.rs` changed relative to the previous candidate. Pinned rustfmt and `git diff --check` passed after this correction. No test, build, or workload was run; lead owns execution after full independent review.
