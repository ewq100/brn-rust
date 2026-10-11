# Phase 1 remembered backup folder evidence

## Authority and baseline

Continued Phase 1 implementation and required-CI-gated integration are explicitly authorized. Isolated `brn-phase1-backup-folder` / `work/phase1-backup-folder` starts at PR #119 merge `aca1c28c8fe79030a7fce5360143a4beeb1d0cb1`. See the [plan](phase1-backup-folder-plan.md). Only fresh synthetic stores; no provider, real user data, native observation or release.

## Prior integration

Delegation #116, Stop-and-send #117, atomic backups #118 and staged restore/missing-storage safety #119 merged after all four required checks passed. All four passed all required checks again on main. Restore main run [38139383720](https://github.com/ewq100/brn-rust/actions/runs/38139383720) also passed documentation and Linux native UI. Existing non-required Windows and Linux native-retrieval failures remain; no all-platform green claim.

## Plan and implementation

Independent GPT-6.1 Sol plan review approved, with explicit stable raw path intent before filesystem checks and a restored-into-saved-folder refusal witness. Implementation is complete; final candidate checks passed and independent GPT-6.1 Sol review APPROVED, with no remaining actionable findings. No production changes followed approval. The desktop now remembers the selected folder, supports save-only Change folder and save-then-backup first use, reloads the persisted choice at dispatch, and shows errors without fallback. The preference is an omitted-None Settings field; no schema or dependency changed.

Parent review found explicit `Store::backup` paths could bypass the new folder-selection safety rule and write a backup inside canonical storage, breaking the next open. The shared backup entry point now has the same outside-workspace check with core/app/CLI witnesses; relative paths and exclusive publication remain supported.

Independent review found that an untagged folder intent could collide with a prior delegation enum string when the same operation ID was reused. The correction tags new backup-folder intents distinctly; legacy delegation/pause bindings remain unchanged. A cross-method reuse regression verifies refusal.

## Local verification

Darwin arm64, pinned Rust 1.98.1, locked/offline. Core cache `../threads-macbook-checkout/target`, desktop cache `../threads-macbook-local/target`:

```sh
cargo test --locked --offline -p brn-threads-core -p brn-threads-app
cargo test --locked --offline -p brn-ai threads
cargo test --locked --offline -p brn -p brn-desktop
cargo test --locked --offline -p brn-desktop --features native-test-support
cargo clippy --locked --offline -p brn-threads-core -p brn-threads-app -p brn-ai --all-targets -- -D warnings
cargo clippy --locked --offline -p brn -p brn-desktop --all-targets -- -D warnings
cargo clippy --locked --offline -p brn -p brn-desktop --features brn-desktop/native-test-support --all-targets -- -D warnings
cargo build --locked --offline -p brn-desktop --features native-ui
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
```

All passed: core 95, app 36, AI thread tests 45, CLI 7, default desktop 18, native state 39. The native suite includes five new backup tests, with synthetic picker outcomes and the actual dispatch/worker path. Existing dependency `block 0.1.6` reports a future-compatibility warning; no new dependency was added. Markdown: 106 files, 391 local links, no failures.

Independent reviewer reran the new core/app backup-directory tests (5 + 4 passed) and all nine prior backup unit witnesses (including relative paths, WAL snapshot, collisions and flush/cleanup). These overlap the complete suites. Approved 12-file implementation fingerprint: `c264aa7d91a0e890a17632e483080339b0777553a22f5dc19b46a9abf8cd8c5a` (sorted code/test relative paths + NUL + file bytes + NUL).

A test fixture initially tried creating a non-UTF-8 filename that macOS itself refuses; it now tests direct API refusal. Clippy caught a helper placed after the test module; relocation resolved it. These were fixture/layout corrections, not relaxed gates.

## Integration gate

Local candidate and independent review satisfied. Required PR CI, merge and main verification are the remaining gates, already authorized. Exact final integration evidence will be recorded on the PR.

## Limits

Native observation remains explicitly deferred. Remembered destination does not verify external backup coverage. Cold interruption/restart and broader recovery acceptance remain qualification work; injected failures and reopen tests do not prove physical power-loss behavior. Transactional upgrades/pre-upgrade backups accompany a real future adopted-schema change, not an artificial schema bump.
