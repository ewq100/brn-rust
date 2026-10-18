**APPROVED: `01fba9a59b51ea13b01e88f176c3dda1ad9b0961`.** The two documentation fixes below are non-blocking.

I checked the commit, whole tree `30f835c9…` and crates tree `21cb6665…`; all match the brief. No builds or tests were run.

**What checks out**
- **Exact move:** the renamed files differ from the originals only in the 6 + 8 `pub(super)` → `pub` changes. The test-only `Tracked::candidate` correctly stays narrower. 30 + 4 tests are kept; `lib.rs` exposes just the two modules.
- **Evidence matches the commit:** the provenance, lock and receipt hashes all match the committed files. The frozen production patch applied to base reproduces the exact crates tree, lock and root manifest.
- **No duplicate route:** old sources are deleted and no shim or `#[path]` remains. The two `use brn_writing_ui::…` imports reach child modules through `use super::*`. All call sites use only the promoted items.
- **No GPUI by default:** both crates default to no features, `gpui-kit =0.7.3` is optional, native features forward correctly, and the lock adds only the new package and desktop edge. The default tree log has 0 GPUI entries.
- **Logs:** 34 tests passed, 0 failed. Clippy shows the crate was actually checked.

**Defects (low, documentation)**
1. `crates/brn-desktop/README.md:17` still links `src/threads_diff.rs`, which no longer exists. Point it at `../brn-writing-ui/src/diff.rs`.
2. `docs/work/active/modular-foundation/b1a/helper-move.md` (title and "Commit proposal and release") says "pending review" and "No commit or staging has occurred", but it is inside commit `01fba9a`. Add a short disposition note with the commit hash rather than rewriting the receipt.

**Preference (not a defect):** the desktop dependency could be optional and enabled only by `native-ui`, since `threads_ui` is native-only. The current form follows the brief.

**Residual qualifications**
- The per-command receipts don't hash `brn-desktop/Cargo.toml`; only the initial snapshot and the production candidate record cover it.
- Not rerun or proven here: full default tests, workspace Clippy, native widget tests, hosted CI, native observation and owner acceptance.
- This is a partial B1a slice: no panel move, and no `brn-threads-app` dependency yet, as the brief allows.
