**Verdict: APPROVED for exact `ca44bd79` (tree `a487a685`, base `df5e48e3`).** I found two low-severity issues and no behavior, security or performance defects. The worktree is still clean and I made no edits.

**Findings (not blocking):**
1. **LOW: doc comments were lost or attached to the wrong code during the move.** About 50 explanatory lines on the writing functions were dropped. Examples:
   - `confirm_discard`: discarding closes the edit with no Undo.
   - `open_edit_elsewhere`: the U4 note about another open edit.
   - The `toggle-margin` single-handler note and the Escape-collapses note.
   - Docs for the comments, banners, note bar and margin.
   - The `RECOVER_FIRST_REASON` doc in `document.rs`.

   Docs that now sit on the wrong item:
   - `threads_ui.rs:262`: "6 px control radius" sits on `apply_tone`.
   - `threads_ui.rs:~279`: a stray piece of the `terminal_notice` doc sits on `finish_tone`.
   - `threads_ui.rs:~389`: seven docs from removed fields stack onto `chat_narrow`.
   - `native.rs:~85`: "Tone of an applied change" sits above `title()`.
   - `threads_components.rs:16–33`: the Lucide-catalog doc sits on `icon_button_with_action`, and "A mode switch" sits on `overflow_menu`.

   Fix: restore the docs on the library functions and delete or move the misplaced ones.
2. **LOW: two implementations of the group-label style.** `native.rs:65` `section()` copies the styling of desktop `section_header(label, None, None)` (`threads_components.rs`), which the old code called. That conflicts with the "single implementation" requirement. Fix: move `section_header` into the library's `components.rs`, re-export it from desktop, and have `section()` call it.
3. **Informational:** the work-bound test's assertion message claims more than the test proves. Its fixture has no linked writing request, and renders that do have one still read the latest Run, as before.

**Resolved areas:**
- **Dependencies and CI:** the lock only adds the brn-writing-ui→brn-threads-app edge. `default = []` and the optional gpui-kit 0.7.3 are unchanged, and the new CI lanes keep the existing jobs.
- **One handler per request:** every `WritingRequest` goes to the existing host handler.
- **Dirty guard:** on the first edit, `begin_edit`/`update_buffer` run synchronously before the Read text, selection and notify. Programmatic reload and the order of Read syncing in the note tile are unchanged.
- **CW2 cache:** the full five-part key, clearing before a fallible load and on error, and the before/after token check are all intact.
- **Disclosure and assets:** the review refuses early as before, and image maps are scoped to the caller with no all-assets fallback.
- **Performance:** building `WritingView` does no scanning, and assets and the latest Run load only where they are rendered.
- **Moves:** components, theme, markdown and layout code moved faithfully.
- **Tests:** library 44 and desktop 277−8+2=271 reconcile file by file, the edits to existing tests are mechanical, and the F15 and work-bound tests do what they claim.

**Residual qualification:** this was a static review only; I ran no builds or tests. Full-workspace Clippy and tests, the combined/headless lanes and the full F22 lane are still pending. The pdftoppm PDF gap was there before this change, and the one ignored test is the existing `BRN_DWR_SHARED` fixture. Nothing has been checked on a native GUI, accepted by the owner or merged.
