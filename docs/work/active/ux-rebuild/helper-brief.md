# Helper brief for UX rebuild slices S3–S6

Read this before your slice prompt. The integration lead owns `work/ux-rebuild-20261105`, shared interfaces and merges.

## Read first

- `AGENTS.md`, `docs/architecture/invariants.md` (skim), `docs/architecture/threads-behavior.md` (sections for your area).
- `docs/product/ux-rebuild/BRN_UX_Rebuild_Spec.md` §§1, 5, 7, 8, 10, 11 and the sections named in your prompt; `BRN_Design_Tokens.json`; the PDF `BRN_Design_Reference.pdf` and `BRN_Interactive_Reference.html` (source) for composition.
- This folder: `plan.md`, `preservation-map.md` (lead decisions are binding), `capability-inventory.md` (your areas), `s2-shell-evidence.md` (the shell you build on).

## Shared rules

- Build on the S2 shell: `Layout` in `threads_layout.rs`, `show_tile`, the header action slot (`header_actions` in `threads_shell.rs`) and the inline notice. Do not change the shell model or sidebar; if you need a shell hook, add the smallest one and record it in your evidence.
- Colors only through `theme::pal()` roles; sizes and type through `theme::metrics` (`crates/brn-writing-ui/src/theme.rs`). No raw color literals. Light and Dark must both read well.
- Visual target: simple, clean, ChatGPT-inspired. Flat prose, quiet dividers, cards only for meaningful results, primary actions with text labels, icon-only controls with accessible names, nothing hover-only, visible focus.
- Preserve every capability in your inventory rows. Every service rule stays: scope, protection, delegation, versioned saves, exact-candidate Apply, receipt Undo, queue order, recovery. UI code calls the existing services; no direct SQL, no duplicated approval logic, no new framework, parser, database or editor.
- Truthful states only: never show Saved/Applied/success before the service confirms it. No sample data, fake models or placeholder statistics.
- `threads_ui.rs` is shared by all slices: keep edits there minimal; put new render code in your own files.
- Tests: keep tests that check user-visible behavior, adapt them to the new presentation without weakening assertions; remove only tests that enforce a retired presentation, with a reason in your evidence. Add focused tests for real risks you touch.

## Verification

- Rust 1.98.1, `--locked`, `--jobs 4` (four slices build in parallel). `export TMPDIR=$(mktemp -d /private/tmp/brn-sN.XXXX)` before tests.
- `cargo fmt --all -- --check`; clippy `-D warnings` and tests for `brn-writing-ui` and `brn-desktop` with `--features native-ui,native-test-support`; tests for any other crate you touch. The full desktop suite needs the document helpers: follow `scripts/verify-desktop-shell.sh`.
- Native: `cargo build -p brn-desktop --features native-ui --locked`, then copy `target/debug/brn-desktop` to a unique name such as `/private/tmp/brn-sN-app/brn-desktop-sN` and launch that copy with `--data-dir` set to a fresh absolute directory under `/private/tmp`. Other slices run their own apps at the same time, so always filter by your unique name:
  - window id: `swift /Users/evokessler/repos/brn-ux-rebuild-artifacts/tools/windows.swift brn-desktop-sN`
  - resize: `osascript -e 'tell application "System Events" to tell process "brn-desktop-sN" to set size of window 1 to {1280, 928}'` (and `{900, 628}`)
  - capture only that window: `screencapture -x -o -l <id> <file>`. Never capture the full screen; the owner is using this Mac. Do not change the OS appearance.
  - Switch Light/Dark through Settings → Appearance or the ⌘K launcher.
- Seed synthetic content through the app or the `brn` CLI against your fresh data directory only. The long-article fixture is `docs/product/ux-rebuild/fixtures/`.
- Save captures to `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/sN/`, inspect them yourself, fix visible defects, quit the app and remove temporary directories.

## Deliver

- Reviewable commits on your branch `work/ux-sN-*`, pushed to origin. No PR, no merge into other branches, no edits to `docs/product/ux-rebuild/`.
- `docs/work/active/ux-rebuild/sN-*-evidence.md`: commits, what moved where (CAP ids), tests removed/replaced with reasons, commands and results with counts, screenshots, acceptance cases you exercised (AC ids, with what was observed natively versus only automated), known gaps.
- Final reply: branch, head SHA, summary, verification, gaps.
