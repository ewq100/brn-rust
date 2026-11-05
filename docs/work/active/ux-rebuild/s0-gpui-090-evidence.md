# S0 GPUI Kit 0.9.0 migration evidence

Branch `work/ux-s0-gpui-090` from `origin/work/ux-rebuild-20261105` at `ab455c6d`. Tested commit: `10e23db0` (code and lockfile; this file is the only later change). Environment: macOS (Apple Silicon), Rust 1.98.1 from `rust-toolchain.toml`, `--locked` throughout. Synthetic fixtures only; no provider calls or user data.

## Resolved graph

`gpui-kit =0.9.0` is pinned in `crates/brn-desktop/Cargo.toml` and `crates/brn-writing-ui/Cargo.toml` with features unchanged. `cargo tree -i` for `brn-desktop` (native-ui, native-test-support) and `brn-writing-ui` shows one version each:

| Crate | Before | After |
|---|---|---|
| gpui-kit, gpui-component, gpui-base, gpui-kit-assets | 0.8.0 | 0.9.0 |
| gpui-pre (default backend, all `gpui-pre-*`) | 0.3.9 | 0.3.11 |
| gpui-fast (optional backend, not enabled by BRN) | 0.1.10 | 0.1.13 |

`cargo tree -d` shows no duplicate `gpui*` or `taffy` versions. The BRN backend and features are unchanged.

Other lockfile changes come from the new GPUI releases: taffy 0.15.0 (replacing 0.13/0.14), dropping wgpu/naga 29, criterion and its dependencies through `gpui-pre-bench-metrics`, `objc2-uniform-type-identifiers`, and `icu_segmenter` 2.3.0 with its locale fallback crates. gpui-fast 0.1.13 requires proptest 1.12.0, which pulls in proptest-macro 0.6, rand_xorshift 0.5 and bit-set/bit-vec. No BRN crate uses proptest directly. A plain `cargo update -p gpui-kit --precise 0.9.0` also raised the ICU family to 2.4 and moved tinystr, yoke and zerovec. The final lock keeps those at their earlier versions by resolving `icu_segmenter` at 2.3.0, the minimum gpui-pre accepts. The manifests carry no extra constraint for this.

## API migrations

- Test fixture `threads_dialogs_tests.rs::dialog_surface`: Kit 0.9.0 gives the dialog surface `id("surface")` instead of `id(layer_ix)` (the per-dialog id moved to the `dialog-instance` wrapper). The helper now finds `"surface"` inside `"dialog"`. The geometry assertions are unchanged.
- No other source changes were needed. Here is why each release-note item needed no work:
  - No BRN code builds theme struct literals (the `..Default::default()` hits are all `WindowOptions`).
  - Nothing calls `Window::text_system()`.
  - Nothing uses `catch_unwind` or unwind-safety markers.
  - Nothing calls `Modifiers::secondary*`, `Keystroke::unparse` or `ScrollWheel`.
  - Every `KeyBinding::new` uses a static literal.
- `InputEditsEvent` is not adopted; BRN still uses `InputEvent::Change` alone.
- Animation tests: the dialog and confirmation tests already set reduced motion. The remaining `std::thread::sleep` loops wait on real worker threads, not animations, so no test clock changes were needed.
- Kit 0.9.0 behavior that comes with the upgrade and is not a BRN design change: dialogs now animate out on close, unless reduced motion is on or the entrance is `None`. `has_active_dialog` already excludes closing dialogs in the tests above. Native observation of the exit motion is pending.

## Verification

Results at `10e23db0`:

| Command | Result |
|---|---|
| `cargo build -p brn-writing-ui --features native-ui --locked` | pass |
| `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets --locked -- -D warnings` | pass |
| `cargo test -p brn-writing-ui --features native-ui,native-test-support --locked` | 44 + 12 passed, 0 failed |
| `cargo build -p brn-desktop --features native-ui,native-test-support --locked` | pass |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked -- -D warnings` | pass |
| `TMPDIR=<fresh canonical dir> bash scripts/verify-desktop-shell.sh` (builds document helpers, exports `BRN_XBERG_HELPER*`, runs commands 1–6 `--offline`) | pass; desktop 386 passed, 0 failed, 1 ignored |
| `cargo build/clippy/test -p brn-desktop --features native-ui --locked --offline` (CI lane) | pass; 80 passed |
| `cargo build -p brn-desktop --features native-retrieval --locked` | pass |
| `cargo build/clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked` + `cargo test` (CI combined lane) | pass; 80 passed |
| `brn-desktop --data-dir <fresh> --headless-check startup` ×2 | pass; `threads.sqlite3` created, no `brn.sqlite` |
| `cargo fmt --all -- --check`, `git diff --check` | pass |

The ignored test is `intake_tests::a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left`, which needs `BRN_DWR_SHARED_*` from a CLI run. It was ignored before this change too.

Six tests need a temp directory without symlinks: three `capture::tests` and three `web_search::tests`, which use `tempfile::tempdir()` without canonicalizing. On macOS the default `TMPDIR` under `/var/folders` sits behind a symlink, so they fail with `SymlinkRefused`. They fail the same way on the 0.8.0 baseline `ab455c6d` (380 passed, 6 failed, in a separate worktree and target). CI uses a canonical `RUNNER_TEMP`. The run in the table used a fresh canonical `TMPDIR` like CI, with no test changes. The 0.9.0 run with the default `TMPDIR` had these six plus the dialog fixture failure, which the `fix:` commit resolves.

## Not verified

Hands-on native interaction on the Mac (writing engine, save/recovery, comments, review, Apply/Undo and the new dialog exit motion) was not observed; only headless and test-harness state checks ran. CI for this branch has not been observed here.
