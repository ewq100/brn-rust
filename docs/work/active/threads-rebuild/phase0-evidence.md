# Phase 0 and F1 evidence

## Identity and authority

- Baseline and remote main: `d768db7a221ad1245241fa5fdca4461bc3cc2409`, checked 11 October 2026.
- Isolated branch: `work/phase0-stale-input`; source checkout remains unchanged.
- Scope: [approved plan](phase0-plan.md). Owner accepted product decisions, not unseen candidate UI. No main merge/release authorized.
- Plan reviewer and main implementation helpers: GPT-6.1 Sol, high reasoning, selected through native subagent controls.
- Independent plan review: approved; no blocking findings.

## Implementation and focused checks

Candidate source is the task-owned diff against the baseline above, with no dependency or schema-version change. Final commit identity is supplied in the handoff; this file travels in that commit. All data fixtures are synthetic and use fresh explicit directories. Existing BRN data was not opened.

### F1 current inputs

`RevisionInput.expectation` defaults to `Current` and is omitted when serialized at that default. Old request bytes/hashes therefore survive; old unapplied candidates gain current-read validation. Explicit `Historical` references retain revision-existence/provenance checks without claiming current truth. Apply validates current versions and active/unsuperseded state inside the existing writer transaction before any grouped mutation. Successful receipt replay remains before mutable checks. Scope, protection, dirty-editor guards, target preconditions, and Undo remain in the existing checked-write path.

Red witness: after the independent connection changed Monday v1 to Friday v2, the old implementation returned `Applied` with `needs_refresh: []`. The corrected regression returns `Stale` with no derived write or receipt. Nine new core tests also cover stale-before-Prepare, mixed current/historical groups, archive/supersession, legacy bytes and replay after reopen, self-input/Undo, and candidate refresh. Runtime tests cover strict tool validation, both default spellings, historical hints, and the runtime stale-input journey. App review selection/remapping preserves expectation modes.

### Desktop and exports

The composer keeps session-only Home/per-thread drafts and shows its destination. Successful posting clears only the submitted draft; failure preserves text. Run/Import have Stop controls and cancellation shortcuts; Export/Backup display finishing status and offer no cancellation.

Export now wraps each note with safely quoted title and source record/locator/label metadata. Body content (including existing front matter) remains intact apart from managed asset URL rewriting. No latest-source version is attributed to an older note. Existing manifest hashing, validation, occupied-destination refusal, and exclusive publication remain intact.

### Automated gates

Commands ran from the isolated worktree with Rust 1.98.1 and the existing lockfile. `CARGO_TARGET_DIR=../threads-macbook-checkout/target` reused build cache for core/app/AI; `../threads-macbook-local/target` was used for desktop. The final export/app rerun and baseline build used `/private/tmp/brn-phase0-benchmark-target`. Cache reuse did not modify the baseline checkout's sources.

| Command | Result |
|---|---|
| `cargo test -p brn-threads-core -p brn-threads-app --locked --offline` | Core 54 passed; initial app 25 passed. After the extra export provenance regression, full app rerun passed 26 tests. |
| `cargo test -p brn-ai threads --locked --offline` | 38 passed, 161 filtered. |
| `cargo test -p brn-threads-app --test export_metadata --test synthetic_baseline --locked --offline`; `cargo test -p brn-threads-app --locked --offline` | 4 export + 3 fixture tests; full app rerun 26 passed. |
| `cargo test -p brn-desktop --locked --offline` | 16 passed. |
| `cargo test -p brn-desktop --features native-test-support --locked --offline` | 25 passed, none ignored. |
| `cargo clippy -p brn-threads-core -p brn-threads-app -p brn-ai --all-targets --locked --offline -- -D warnings` | Passed after fixing the new fixture's three lint findings. |
| `cargo clippy -p brn-desktop --features native-test-support --all-targets --locked --offline -- -D warnings` | Passed. |
| `cargo build -p brn-desktop --features native-ui --locked --offline` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `python3 -m unittest discover -s scripts/tests -q` | 18 passed. |
| `python3 scripts/check-markdown-links.py`; `git diff --check` | Passed; repeated on final docs. |

Test-fixture caveat: an initial expanded desktop test changed only in-memory settings. Navigation restored saved provider defaults and unexpectedly spawned a runtime worker before the assertion failed. No provider response was observed; whether it reached authentication or a request is unknown. The two new fixtures now persist an unsupported `offline-fixture` provider with empty credential paths before constructing the UI. Final tests passed with those guards. This task does not claim live-provider qualification or that every preliminary test run was network-inert.

## Native observations and baseline

### Reproducible headless baseline

The opt-in `synthetic_baseline` app example uses supported checked interfaces and never selects a default user data directory. Creation atomically reserves a new absolute directory, rejects existing paths and symlink parents, and bounds the corpus to 20–1,000 notes. Three integration tests verify refusal and repeatable semantic content. IDs/timestamps are fresh, so physical DB bytes are not deterministic.

Measured on macOS 26.5, Mac17,9 / Apple M5 Pro, 24 GiB RAM, 15 cores. Release build, default app features, native retrieval disabled. Baseline read paths in `crates/brn-threads-app/src/lib.rs` match `d768db7` exactly (SHA-256 `a6f7d81f24d587de7a0fe8a29d6c46793cc77734a535b1491b303ef5130f9ddc`). The captured binary predates only semantics-neutral fixture Clippy fixes; no production performance optimization is included.

Corpus: 500 notes (375 short, 100 medium, 25 long), 5,947,770 current Markdown bytes, 100 managed PNGs / 291,400 bytes, 100 Sources, 125 notes with three additional revisions each, 50 threads / 600 messages / 50 Actions, 1,551 records and 1,526 receipts. Semantic content SHA-256: `f3a579c7bc63f2c9bcc2f81f4bdf701720d59608e26165b4a0acfd80c1716440`.

| Measurement | Result |
|---|---:|
| Database | 83,763,200 bytes (79.9 MiB) |
| Markdown snapshot | 6,421,682 bytes |
| Workspace open | 161.67 ms |
| First search / cold in-memory index | 127.11 ms |
| Warm search median / max | 7.77 / 8.39 ms |
| Selected long-note point read median | 0.063 ms |
| Thread messages median / max | 24.25 / 26.91 ms |
| Navigation-equivalent reads median / max | 44.08 / 44.74 ms |
| All receipts read | 20.00 ms |
| Measurement process peak RSS | 79,413,248 bytes (75.7 MiB) |
| Measurement process elapsed | 0.80 s |

Cold search means a new in-memory index; OS disk caches were not flushed. Navigation-equivalent timing combines notes, threads, Needs you and messages reads; it excludes rendering/event-loop delay. RSS is the measurement process, not the native desktop. No hardware-specific latency assertion was added to CI.

Captured fixture: `/private/tmp/brn-phase0-corpus-20261011-export` (`data` child). Raw reports: `/private/tmp/brn-phase0-corpus-create.json`, `/private/tmp/brn-phase0-corpus-measure.json`, and matching `.time` files. Captured binary: `/private/tmp/brn-phase0-baseline-captured`, SHA-256 `d1bbc2cf90a3b04e85a1aee0e5f172985f7a3e58dc3213291cfd17483beea849`. These scratch paths may expire; the generator and compact results are retained here.

Reproduce with an unused canonical root (replace `NEW`):

```sh
CARGO_TARGET_DIR=/private/tmp/brn-phase0-benchmark-target cargo build -p brn-threads-app --example synthetic_baseline --release --locked --offline
/private/tmp/brn-phase0-benchmark-target/release/examples/synthetic_baseline create --root /private/tmp/brn-phase0-corpus-NEW --notes 500 --source 'Phase0 candidate; record git HEAD'
/usr/bin/time -l /private/tmp/brn-phase0-benchmark-target/release/examples/synthetic_baseline measure --root /private/tmp/brn-phase0-corpus-NEW
```

On macOS the measurement resource-accounting call required approved escalation after the sandbox denied `kern.clockrate`; the escalated measurement succeeded. Initial creation completed despite that first accounting failure.

### Native candidate and limits

Candidate bundle: `/private/tmp/brn-phase0-native/BRN Phase 0 Candidate.app`, debug/native-ui build, 154,732 KiB allocated (151.1 MiB). Desktop executable is 127,935,456 bytes, SHA-256 `2acd152642bae32f4fdf86c5c86b38d68af8cb60865d68d71f02dec1d12df698`, matching the tested build; intake helper is 30,494,976 bytes. This development bundle size is not a release installer measurement and is not directly comparable to a different feature/profile build.

An initial baseline Home screenshot was observable. Later baseline reads failed with ScreenCaptureKit errors -3811/-3812. Candidate access returned **“Computer Use was not approved to use BRN Phase 0 Candidate”**. The denial was not bypassed. Native candidate startup, typing latency, rendered destination/Stop controls, and native navigation remain **unobserved**. App-selection tool elapsed time is not a startup measurement.

Remaining native procedure after permission is available: use the fresh synthetic candidate workspace; verify A/B/Home draft navigation, successful/failed sends with an offline fixture, and rendered Run/Import versus Export/Backup controls; record a startup-to-ready observation and typing/native-navigation latency against a copy of the representative corpus. The prepared candidate workspace points at a deliberately absent synthetic auth path, not a real account. Do not send a live provider request to qualify these changes.

## Independent review

Independent GPT-6.1 Sol reviewer inspected every implementation diff and untracked source/test against `d768db7`, applying the Rust review checklist. **APPROVED: no actionable implementation defects found.** Reviewer relied on reported test/Clippy results rather than repeating them. Native observation remains an evidence gap, not a passing gate.

## Acceptance and integration

On 11 October 2026, after repeated Computer Use permission failures, the owner explicitly deferred native observation and requested a PR, merge after CI passes, and continued foundation work. Native behavior remains unobserved; this deferral does not turn component tests into native evidence. PR publication and CI-gated merge are authorized; release is not requested. Hosted CI and the eventual merged commit are recorded in the PR and subsequent status.
