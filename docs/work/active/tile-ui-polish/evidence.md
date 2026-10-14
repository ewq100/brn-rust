# Reusable workspace areas — evidence

**Date:** 14 October 2026. **Baseline:** `89a1a944edd79069ce2968111daf013607f5ec14`. **Branch:** `codex/tile-ui-polish`. Candidate is the commit containing this record; combined changed-Rust path/content SHA-256: `9158ad3decb1a1f0703e8d05ee91bd0ee5e37a027364e3aecded2f5c6b9edf99` (sorted relative paths, NUL, bytes, NUL).

## Implemented behavior

Normal clicks reuse Navigation, Main and Companion; existing panels are focused and revealed. Only Open beside creates extra splits. Persistent navigation, Close extra splits, a separate Settings view, reversible Solo and reliable focus keep the workspace manageable. Browsing notes preserves the selected chat and unsent drafts. Search/alphabet and Settings section controls remain outside their scrolling content. Note controls have a separate row so they do not crowd the title. No schema, checked service, provider or dependency changes.

## Automated verification

macOS arm64, pinned Rust 1.98.1 and lockfile, GPUI Kit 0.7.3 / GPUI 0.3.9. Reused compatible cache `CARGO_TARGET_DIR=/Users/evokessler/repos/brn-rust/target`; all fixtures use fresh canonical temporary paths and offline provider configuration.

- Initial failing witness: repeated ordinary opens accumulated a fourth panel at Linked. The final replacement behavior passes.
- `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline`: **111 passed, 0 failed**, recorded in `/private/tmp/brn-tile-ui-polish/tests-final.log`.
- `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings`: **passed**, recorded in `/private/tmp/brn-tile-ui-polish/clippy-final.log`.
- `cargo build -p brn-desktop --features native-ui,native-test-support --locked --offline`: **passed**, recorded in `/private/tmp/brn-tile-ui-polish/native-build-final.log`.
- `cargo fmt --all -- --check`, `git diff --check`, `python3 scripts/check-markdown-links.py`: **passed**; Markdown check covered 62 files and 272 local links with zero failures.
- Headless startup and reopen succeeded with explicit `/private/tmp/brn-tile-ui-polish/native-preview-workspace`. The first `/tmp` attempt correctly refused its symlink ancestor; canonical `/private/tmp` was then used.

New regression coverage exercises 30 ordinary navigation transitions, companion replacement after reorder, explicit splits and deduplication, cleanup, Settings/maximize safety, Solo restoration, far-off and already-focused panel reveal, retained chat drafts, exact review text/candidate identity, and same-review/new-note-chat focus in Solo.

Earlier runs exposed and fixed outdated navigation expectations and hidden scroll controls. The existing decision-dialog mouse test intermittently failed in two intermediate full runs (missing dialog/incorrect edited content), then passed in its isolated run and subsequent complete runs. These failures remain in `tests.log` and `tests3.log`; this record does not claim their cause was established or that a green run proves absence of timing sensitivity. The compiler also reports the existing dependency future-incompatibility notice for `block 0.1.6`; strict lint succeeds without suppressions.

## Independent review

Read-only plan review approved. Full diff review found three concrete problems: Settings could become an invalid maximized tile, reopening an already-focused panel did not reveal it after manual scrolling, and direct review/new-chat opens could leave keyboard focus in hidden content during Solo. All three were fixed and covered by native regression tests. Scoped re-review approved with no remaining actionable findings. Header-only polish, including the separate wrapping Note toolbar, was reviewed separately and approved.

## Native observation

Built a task-local `BRN Tile Preview.app` with a unique bundle ID and explicit synthetic data directory. This is a development preview, not an installed release. CUA observed normal Notes/Actions navigation reusing areas, Settings/Escape returning to the same arrangement, and Open beside adding Connections as a fourth panel. A synthetic unsaved note survived Actions navigation and launcher reopening. After restart on the final build, the same text recovered unchanged, and Solo toggled to one panel and restored the three-panel arrangement. The final toolbar build was restarted and inspected: the complete note title is readable above the editing controls and the unsaved text remains intact. No provider request was sent and no real user workspace was opened.

Representative native behavior is observed; broad IME, other platforms, real-data adoption and owner acceptance of the implemented UI are not claimed. The temporary preview remains available at `/private/tmp/brn-tile-ui-polish/BRN Tile Preview.app` for inspection.

## Gate separation

- Defined: owner accepted reuse/main/companion and explicit Open beside.
- Implemented: complete.
- Automated verification: 111 native tests, strict lint, native build, formatting, links and whitespace passed.
- Independent review: approved after supported findings were fixed.
- Native-observed: representative synthetic navigation, draft recovery and Solo checks.
- Owner-accepted implementation: pending the owner's use of the preview.
- Published / merged / released: not performed; current work is isolated from the owner's retrieval changes.
