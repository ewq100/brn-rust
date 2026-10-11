# App-wide tiled UX and preview evidence

**Date:** 11 October 2026. **Baseline:** `7c189c71d3eb664ef6493294d5b73d3e12b90b7d` (PR #122, four required PR and main checks passed). **Candidate:** isolated `work/tiled-app-preview`; the PR records exact published head/tree and integration receipts. [Plan](plan.md) and [scope](spec.md) define the assignment.

## Implemented boundary

GPT-6.1 Sol helpers implemented shared native tiles, focused UI journeys and portable preview packaging. One independent Sol helper reviewed the plan before changes; a separate helper reviewed the full implementation. UI changes are confined to `threads_ui.rs` and the new `threads_tiled_tests.rs`; no checked service, storage schema, runtime authority, dependency or lockfile changes. All original write/Undo callbacks and control IDs remain.

Home, current-note search, grouped Actions, receipt History, labelled Settings, navigation/imports, thread/source, note/chat/comments/review and composer share one palette and square tile treatment. Menlo labels and system-font prose retain toolkit focus/selected/disabled behavior. Narrow layouts stack or scroll.

## Local verification

macOS arm64; repository Rust 1.98.1 and locked dependencies. `PATH=/opt/homebrew/opt/rustup/bin:$PATH`; debug `CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010`. Synthetic fixtures only; no native app launch or live-provider calls. Logs are under `/tmp/brn-tiled-app-preview-tests/` on the implementation host.

| Check | Result |
|---|---|
| `bash scripts/verify-storage.sh`, with explicit maintained helper and Poppler paths | Pass: formatting, default workspace build/strict lint, 458 tests passed, 3 ignored; synthetic startup passed. `storage.log`. |
| `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline` | Pass: 56 tests, including six new tiled journeys. `native-all.log`. |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings` | Pass. `native-clippy.log`. |
| `cargo build`, strict all-target `cargo clippy` and `cargo test`, each `-p brn-desktop --features native-ui,native-retrieval --locked --offline` | Pass: combined build/lint and 20 state tests; no model assets. `combined.log`. |
| Optimized `cargo rustc --release` native desktop and helper, `MACOSX_DEPLOYMENT_TARGET=15.0`, final `-C link-arg=-mmacosx-version-min=15.0` | Built for the preview; binary architecture, minimum OS and linked-library checks are enforced by packaging. `release-desktop-final.log`, `release-helper-final.log`. |

The six new UI witnesses cover dirty-note and per-context chat draft preservation across all menu destinations, actual search input/results/clear, Action changes with checked receipt and versioned Undo, settings preservation/delegation, meaningful empty states, and scroll-reachable controls at 850×600. They complement existing writing proposal, focus, steering and backup witnesses. They do not establish observed native usability.

Additional final local checks: all 31 Python tooling tests pass (`python3 -m unittest discover -s scripts/tests -v`, `tooling-final.log`), including real synthetic arm64 Mach-O signing, archive relocation/signature checks, installed hashes and ten release-policy tests. Markdown links pass (42 files, 141 local links), launcher shell syntax and `git diff --check` pass.

Initial full tooling attempts failed with SIGKILL while directly executing the legacy synthetic launcher in an unnecessarily nested temporary folder. Removing that new wrapper's extra nesting restored the script's normal physical temp parent and retained its own fresh fixture, direct executable launch and all existing assertions. The OS cause was not established; the failed `tooling-parent.log` is retained, and the corrected full suite passed independently for lead and release helper. No product behavior or test assertion was bypassed.

Independent full-diff review approved the UI, authority/Undo preservation, release workflow and packaging. Its first-use provider instructions finding was corrected and rechecked. Lead review also corrected post-sign binary hashes, extracted-archive signature verification, exclusive output publication and explicit final link target to prevent cached binaries retaining an older minimum macOS.

At source freeze, the actual optimized distribution still awaits an exact candidate commit stamp and headless package qualification; the PR will record that result before integration. The exact merged source commit is stamped into each published distribution manifest by the release workflow. Actual main CI and published asset receipts are available on GitHub.

## Release boundary

`v0.1.0-preview.1` is an ad-hoc-signed, unnotarized macOS 15+ arm64 preview. No provider credentials or embedding models are bundled. External Poppler is required for PDFs. The stable recipient workspace uses core ownership/schema checks. The release workflow requires an explicitly changed approved manifest, same-repository main push, exact current-main SHA and all four required CI jobs from that run attempt. It verifies all draft assets and rechecks main/manifest immediately before publication. Existing published assets are never replaced.

Native interaction, live-provider and broader cold-restart/recovery acceptance remain deferred/pending. Non-required Windows and Linux retrieval failures remain visible. Neither this preview nor headless tests certify daily use with valuable data or all-platform support.
