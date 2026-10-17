**B4 review, round 2: CHANGES_REQUIRED.** One small but real defect remains. Everything else in the correction checks out.

Pins match: HEAD `4b04bc03`, tree `dc12ea1f`, and the `crates` tree is `bdace41c` at both 4a8 and 4b04. The worktree is clean and no cargo or rustc processes are running. I read the actual `50fa..4a8` diff and used `c5..4a8` as context. I ran no builds or tests and made no edits. I didn't use the code-review skill because it hands work to sub-agents, which the brief forbids.

### Defect

**LOW: the host binds Rewrite to the wrong shortcut, and the new test locks that in.**
- `examples/writing_host/host.rs:559` binds `cmd-shift-enter` to Rewrite.
- `journey.rs:363` and `:370` test that same key.
- Everything else uses ⌘⇧R for Rewrite:
  - The shared panel button the host renders says "Rewrite with AI ⌘⇧R" (`writing.rs:1829` and `:1835`).
  - Desktop binds `cmd-shift-r` to Rewrite and `cmd-shift-enter` to MaximizeTile (`threads_ui.rs:4700` and `:4702`).
  - The product spec says "⌘⇧R rewrite" and "⌘⇧↵ maximize" (`docs/work/active/omakase-ui/spec.md:52`).
- In the host, pressing ⌘⇧R does nothing. That breaks the brief's rule against advertised controls that don't work (`brief.md:7`), the same kind of problem as round 1's main finding, though narrower.
- The binding dates from c95, but round 1 didn't catch it. It only became reachable once this correction wired up the key context.
- No toolkit binding conflicts with `cmd-shift-r`; the nearest toolkit binding is the input's own `shift-enter`.
- **Fix:** change the chord at `host.rs:559` to `"cmd-shift-r"` and update `journey.rs:363` and `:370` to match. Nothing else needs to change. A round 3 should only need to check this one token.

### Verified, not reopened
- **Keyboard and menu wiring:**
  - `render` now sets `key_context("WritingHost")` and adds four handlers, each forwarding to the existing `dispatch`.
  - Ask in chat reaches the existing explicit "Unavailable in this development host" notice.
  - The editor is inside the host element, so actions from the focused editor reach the handlers.
  - Comment opening keeps its "save first" check and doesn't write a comment.
- **New tests check real outcomes:**
  - cmd-/ opens a comment anchor on the saved note version and passage, with zero comments written.
  - cmd-s saves the editor text, and the test fails if no edit happened.
  - The Rewrite shortcut produces a real run and one candidate.
  - The menu-action test confirms Ask in chat starts no run, makes no candidate and writes no comment. The test's starting notice differs from the "Unavailable" text, so that check isn't vacuous.
  - The claim about native menus is honestly limited to action dispatch, not a real menu click.
- **Apply refresh:** `journey.rs:160-163` checks `editor == REVISED` right after Apply and before Undo.
- **Failing-first evidence:** the receipts show the new keyboard test failing on the old host (0 passed / 1 failed / 4 filtered).
- **Example now tested and linted:**
  - With `test = true`, the ordinary native writing test command runs both the library tests (44) and the example tests (5).
  - Strict all-target Clippy now covers the example's test module, including `journey.rs`.
  - The example still requires `native-ui`, so the default build stays free of GPUI. Default features, dependencies and `Cargo.lock` are unchanged versus c5.
- **CI and script:**
  - Net change versus c5 is zero.
  - The macOS "Test reusable writing UI" step and the script line now cover the example tests, so removing the extra command was correct.
  - Job names and other steps are unchanged.
- **Docs:** historical c95 claims are now marked as superseded, and the open limits are stated honestly.

### Not defects (left as is)
- The editor typing test only checks state after updates settle, not before render.
- Cleanup on a real GUI quit is still untested.
- `latest_run` copies desktop logic, and Apply checks the candidate twice.

This covers source review only. Principal acceptance, INT-W and the native, human and GUI checks remain open.
