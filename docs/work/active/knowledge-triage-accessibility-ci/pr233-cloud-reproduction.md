# PR233 cloud reproduction

On 2026-10-28, the old PR233 head `7a71e660` was tested offline with `cargo test --workspace --no-fail-fast --locked --offline` (one job, Rust 1.98.1). Compilation took 672 seconds. The suite exited 101: 1,207 passed, 175 failed and 7 were ignored. Of the failures, 132 explicitly reported `UnsafeCredentials`; the remaining failures were not uniformly root-caused. Search code is excluded by the default feature selection, so this run does not establish a search-code cause.

A focused rerun of the unchanged process cancellation test also exited 101 on its PID-disappearance assertion. Descendant 18023 was observed live under 18019 in process group 18019, then as a zombie under PID 1. It was no longer executing, but the test still failed. This observation does not establish the cause of the hosted failure.

The original cause of [hosted run 40422659424](https://github.com/ewq100/brn-rust/actions/runs/40422659424) remains unknown; the denied log download was not retried. No production or test changes were made in this reproduction. PR233 was refreshed by a normal merge of main `cfe3f96c` into `35bd84cb`; exact-head CI is pending, and the refresh is not a claim that a cause was repaired. PR235 is merged at main `cfe3f96c`; the parent reported seven supported jobs passed and three optional Windows jobs failed. Knowledge `f229b5d0` and accessibility `3252f940` remain preserved locally.

Existing expanded lead merge authority covers approved tracks only. It does not extend to live providers, Mac, release, security changes or new scope. Coding usage counters, the lead model and effort, and cost were unavailable. This documentation subtask used `gpt-6.1-terra`; no task token or cost figure is claimed.
