# Editor trial evidence

Date: 2026-09-27. Base: eb62bfc. Branch: trial/editor-selection-comments.

## Starting state

The provider lifecycle trial was clean and pushed at eb62bfc. The user authorized best-judgment continuation; the original roadmap's native editor trial is the next Phase A step. Changes are isolated at `/private/tmp/brn-editor-trial`. No applicable AGENTS.md was found in the prior repository/ancestor inspection.

`sw_vers` reports macOS 15.3.1 (24D70); `xcode-select -p` reports `/Library/Developer/CommandLineTools`; Rust is 1.98.1. Official GPUI Kit 0.6.6 metadata and pinned source were inspected. Cargo fetched and locked its dependencies; the lock includes cross-platform packages, not all of which build on macOS. No dependency install changed account settings or installed an application into Applications.

`PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh` passed from this worktree: original starter/provider build, format, Clippy, 16 provider tests and nine smoke checks. No live provider call was made for the editor trial.

## Implementation and native checks

`PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-editor-trial.sh` exited 0: 11 core model tests, native build, formatting, native Clippy with warnings denied, and five headless CLI checks (`--help`, unknown argument, surplus arguments, missing file, invalid UTF-8). Model tests cover range shifts/boundaries, overlapping/deleted/duplicate quotes, exact full-buffer undo/redo, UTF-8 boundaries, stale selection capture, diff newline behavior and bounded input loading.

The actual native application compiled successfully. A temporary unsigned `.app` wrapper outside Git was created at `/private/tmp/brn-editor-trial-local/BRN Editor Trial.app`, with a symlink to the built debug executable. `open -n` returned 0. This is a local launch helper, not a signed or redistributable app package.

Computer-use attempted to inspect that app but reported: **the Mac is locked and automatic unlock failed**. The user was asked to unlock manually. Consequently rendering, real selection/focus, native clipboard, undo/redo, resize, scrolling and deleted-anchor interaction have NOT been observed. No human acceptance or production framework selection is claimed. Code review continues while that interaction gate is blocked.

## Native dependency build

`cargo +1.98.1 build --manifest-path experiments/editor-trial/Cargo.toml --features native-ui --locked --offline` compiled the native dependency graph and an initial empty bootstrap in 51.25 seconds. This establishes the build prerequisites, not a functional editor. No full Xcode installation was required.

Cargo reported a transitive future-incompatibility warning in `block 0.1.6`: `static of uninhabited type` for `_NSConcreteStackBlock`. `cargo +1.98.1 report future-incompatibilities --id 1` from the editor trial directory identified that dependency. Current Rust 1.98.1 accepts it; a future toolchain may not. No vendor patch or toolchain change was made to hide the warning.

## Review and remaining acceptance

Astra's source review confirmed UTF-8 selection semantics and GPUI change-event wiring, including native undo/redo event emission, by inspecting the pinned dependency source. This is static evidence, not observed interaction. Review requested full original-quote access and revealing the editor when selecting an anchored passage. These were implemented; a scoped follow-up found that quote scroll areas needed unique IDs, which were added. Astra scoped re-review approved the fixes with no remaining implementation findings. Final whole-branch Astra review accepted the implementation checkpoint with no Critical or Important findings. A minor stale progress-ledger note was corrected. The verdict does not complete native acceptance or select a framework.

The Mac remained locked on a second computer-use attempt. No workaround was used to bypass the lock. The native acceptance checklist below remains unchecked:

- [ ] Window renders correctly with the long synthetic fixture (323 lines).
- [ ] Mouse and keyboard selection survive Capture selection and comment-entry focus changes.
- [ ] Nearby edits preserve a safe anchor; deleting/duplicating a passage visibly makes it unresolved.
- [ ] Full original quotes remain readable after deletion, including multiple independent quote scroll areas.
- [ ] Show passage visibly reveals and selects the correct passage.
- [ ] Clipboard and Unicode editing work, native undo/redo updates comments and diff correctly.
- [ ] Narrow-window resizing, document/review scrolling, and diff rendering remain usable.
- [ ] The user tries the flow and accepts or rejects GPUI/document-model suitability.

Other unverified areas: IME, accessibility quality, screen-reader behavior, performance near the 2 MiB input limit, sustained editing, app packaging/signing, and persistence. Memory-only state is intentional and labeled; comments and edits are discarded on exit. Framework selection stays provisional.

## Final checkpoint verification

After the last UI-only review fixes, native build, formatting and native Clippy with warnings denied were rerun and passed. The unchanged model remains covered by the 11 passing tests; the complete editor script had passed the five CLI cases. The original starter/provider regression suite also passed. A secret-pattern scan covered all 13 changed/authored files, and manual final review found no apparent secrets. Cargo.lock registry sources and direct version pins were checked. `git diff --check` passed. Build outputs and the local `.app` wrapper are excluded from Git. Both earlier trial worktrees remain clean.

Delivery is an implementation checkpoint on `trial/editor-selection-comments`; nothing is merged or released. Native acceptance remains blocked on an unlocked Mac. Once unlocked, quit any earlier trial window and relaunch the final binary before following the unchecked native acceptance checklist.
