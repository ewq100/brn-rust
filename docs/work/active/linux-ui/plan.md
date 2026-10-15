# Linux UI testing

**Assignment:** 15 October 2026. The owner authorized starting the Linux GPUI widget-test milestone, using parallel coding agents, with about one hour available. Baseline: current main `771116f0f5b8607bf3ffc1239da53d0e64e174f2`. The cloud task already has an isolated checkout; use it rather than creating another worktree. Mac development can continue on separate branches.

## Selected outcome

Run the existing desktop widget suite on Linux using the pinned Rust 1.98.1 toolchain and unchanged dependency lockfile. Fix diagnosed platform differences without weakening test assertions or changing Mac behavior. Save repeatable local/cloud instructions and run the same suite in Linux CI. Preserve existing Mac validation.

If time and supported facilities permit, separately exercise the existing GPUI headless pixel renderer with synthetic data. A rendered Linux check does not establish Mac input, dialogs, signing, packaging or converter restrictions. Those remain separate platform checks.

## Work

1. Inspect the latest baseline and install native build prerequisites with normal signature/checksum verification. Keep machine-specific tooling outside Git.
2. Build and run the unmodified widget suite; diagnose failures before editing application/tests.
3. Correct actual portability issues, add a guarded repeatable widget command and Linux CI coverage, then rerun the affected suite and strict lint.
4. Independently review the final changes and record current-run results in [evidence](evidence.md), separating successful checks, limitations and unrun Mac checks.

## Ownership

The lead owns shared integration, builds, environment configuration and development documentation. Helpers have bounded ownership of native dependencies outside Git, keyboard portability tests, the verification runner/CI, and an optional new rendered witness. An independent helper reviews correctness and preservation of Mac behavior. Only the lead runs Cargo against the shared target directory.

No live provider calls, owner data, release, merge or branch-protection changes are assigned. Use fresh synthetic application/credential fixtures under a private parent outside any `.git` ancestor.

## Outcome

The selected milestone and optional rendered witness passed within the session budget: 147 desktop tests passed, the real Linux pixel/input/Save/reopen witness passed separately, strict lint passed, and complete saved setup/startup commands were replayed. See [evidence](evidence.md) for initial failures, corrections, test counts and remaining platform limits. The changes are prepared for review; no merge or release is assigned.
