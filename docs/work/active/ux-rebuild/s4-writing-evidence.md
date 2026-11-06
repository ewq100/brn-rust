# S4 reading, writing, comments, review, outline: evidence

Branch `work/ux-s4-writing`, from `origin/work/ux-rebuild-20261105` at `7dc1c794` (S2 merged).

Status:
- Implemented and verified by automated tests.
- Partly native-observed in the real app on synthetic data (see [Native observation](#native-observation)).
- Not owner-accepted and not merged.

## Commits

| Commit | Content |
|---|---|
| `2b3adf9c` | Outline built from the renderer's own parse, with a docked or temporary view; shared reading styles; header save state from actual Save results; calm writing canvas; Library articles open in Read |
| `1251d6e0` | Connections and Document history moved into the document ⋯; Outline works in Edit too; on-demand overall feedback; test migration |
| `b07eadfb` | Review summary (document or group, changes, sources, base unchanged); Added and Removed stated in words; Discard |
| `edc83f5e` | A missing image shows a bounded, labelled placeholder; scrollbar gutter for the reading column |
| `6470d837` | A re-anchored comment says that it moved; launcher command "Show or hide outline" |
| `b64a1c39` | A stale Save shows Conflict in the header, never Saved (test) |
| this file | Evidence |

## What changed

### Reading (READ-05, §10.1)

The document opens in **Read** for Library articles and found notes. A recovered edit still opens in Edit. Read and Edit are a small local switch.

`document::readable_document` applies the shared styles:
- Column and spacing: the `metrics::DOCUMENT_WIDTH` column, centred, with a 16 pt right gutter so text never runs under the scrollbar. Body 16/1.7, paragraph gap 12.
- Headings: H1–H6 at 28/22/18/16/16/16, weight medium. Each heading has 24 before and 8 after; a first heading collapses into its 24.
- Blocks: quotes have a quiet divider. Code is monospaced with Copy.
- Tables: header rows are distinguishable and cells padded. The gpui-kit adaptive layout is used (`overflow.x = Scroll`): columns shrink to a floor, then the table scrolls inside its own frame.

Images come only from stored assets (`brn-asset://`, unchanged from CAP-156). They fit the column, keep their aspect ratio and are never inverted in Dark. An image that is not stored now renders a bounded placeholder, "Image unavailable · {alt}", in block and inline forms (`markdown.rs` `MissingImage`). The parser revision hashes the stored asset keys, so a newly stored asset re-renders.

### Editing (WRITE-01..04)

- The heavy bordered editor box is gone; Edit is a calm canvas in the same column.
- The duplicated large title above the document heading was removed. The header carries the title.
- Formatting controls appear only in Edit.
- The overall-feedback field is no longer permanently shown under the editor. **Feedback** in the writing bar opens it (`ToggleFeedback`). It reads "Feedback added" when text exists and stays included in the next Rewrite.
- Header save state comes from the actual Save outcome:
  - **● Unsaved · ⌘S**: the default for an open edit.
  - **Saving…**: while the checked Save runs.
  - **Saved**.
  - **Save failed · Retry**: the service errored and the edit is kept.
  - **Conflict · edit kept**: stale, generation mismatch, closed, or another edit open.
  - The state is recorded per note and edit session (`canvas::SaveStatus`, `SaveAttempt`).
  - The failed and conflict states carry the reason in the accessible name and tooltip.
  - ⌘S and Save semantics are unchanged.
- Document ⋯ (`note_menu_section`): Save, Reload, Save copy, Comment, Rewrite, Connections, Brief & references, Protect / Make working note, Document history, Export, Coverage, New chat, Discard edit…. Connections left the toolbar for this menu.

### Outline (READ-01..04; replaces CAP-151)

`outline.rs` and `canvas.rs`. **Entries come from the renderer's own parsed blocks** for the displayed text, through gpui-kit's `TextView` rendered-text structure and source↔rendered ranges. There is no regex scan and no second parser:
- Fenced code creates no headings.
- Levels 1–6 are kept, including nested and skipped levels.
- Unicode labels are preserved.
- Duplicate titles become distinct targets in distinct blocks.
- Long labels wrap, and each entry's accessible name gives the full text and level.

Targets and freshness:
- Each target carries a `Stamp`: document id, content generation (committed revision, or unsaved buffer generation) and a hash of the displayed source.
- A target whose stamp or range no longer resolves is refused with "That heading is no longer in the displayed text. The outline has refreshed; choose a heading again." It navigates nowhere.
- The outline re-derives on rename or removal, Read/Edit switch, Save, Apply, Undo and reopen. It shows "Updating" until the renderer has parsed the current text, which discards stale results.

Placement:
- **Docked**: a 200 pt column (`metrics::OUTLINE`) beside the document, used only when the document region keeps at least 560 after the sidebar, outline, assistant and comment margin (`outline_docks_at`).
- **Temporary contents view** otherwise: it covers the document region and returns to the document at the chosen section.
  - It never stacks with the assistant sheet.
  - It never overwrites the stored docked preference.
  - Escape returns focus to the toggle.
- **Closed on first open.** Visibility is remembered by stable document id for the session (see [gaps](#known-gaps)).

Navigation and current section:
- Click or Enter reveals the heading under the toolbar. A nested heading reveals its own range.
- In Edit, the caret is placed at the heading, through the editor's selection API.
- The current section has a leading bar, a heavier label and selected semantics, so it is not shown by color alone. It updates on scroll without moving focus.

Keyboard and entry points:
- Tab reaches the toggle and the entries. Up and Down move between entries without wrapping. Enter activates. Escape or ✕ closes.
- The outline hides during review.
- "No headings yet" when the document has none.
- Toolbar **Outline** toggle, plus a new launcher command, **Show or hide outline** (⌘K), see the shell hook below.

### Comments (§6.6)

- Selection strip: **Comment ⌘/** plus the existing actions. Comments toggle with count; existing margin modes (Wide, Narrow, Stacked).
- Comment anchors are honest. A located comment reads normally.
- A comment written on an earlier version and re-anchored to the current one now says **"Moved with edits · written on vB, re-anchored to vN"** (`layout::moved_anchor`). It is joined with "Excluded from rewrite" when that applies.
- An unresolved comment keeps its quote visible and says "Tied to vN…" or "Passage not found in vN…" (existing `unlocated_reason`).
- **Mark addressed** stays explicit.

### Rewrite and review (§6.6–6.7, REVIEW-01..04)

Rewrite is unchanged in behaviour and stays distinct from chat: ⌘⇧R, the "Rewrite with AI" bar, and progress in the document with chat closed.

Review summary (`review_summary`):
- The document or group, the number of changes, the sources, and "base unchanged".
- A legend that says "Added text is underlined · removed text is struck through", so the change does not rely on color.
- The label reads "Proposal · whole note · N added · M removed".
- **Compare side by side** opens the two-column view on demand.
- Apply / Approve the exact candidate, candidate edit, and **Discard** (was "Dismiss"; the note is unchanged).
- Receipt Undo appears only after a confirmed Apply and is separate from editor undo.

History and compare views stay labelled, and previewing them never changes current content. Document history is a separate list, reached from ⋯.

## Shell hooks outside S4 ownership

These are small and listed for the lead:

| File | Change | Why |
|---|---|---|
| `threads_shell.rs` | Header Save pill reads `WritingPanel::save_status()` (five states) | WRITE-02 needs the actual service state in the header slot |
| `threads_lists.rs` | `Command::Outline` "Show or hide outline" in the launcher, plus test assertions | READ-01 launcher entry |
| `threads_shell.rs` | `command_icon` for Outline (TableOfContents) | Same |
| `threads_ui.rs` `run_command` | Dispatches `WritingRequest::ToggleOutline`, or warns "Open a document to show its outline." | Same |
| `threads_tiles.rs` | A Library row calls `read_next()` before `open_note` | Library articles open in Read (§6.5) |
| `threads_state.rs` | Removed the `sections` re-export and its unit test | Replaced by the renderer-parse outline (see tests) |

## Tests removed or replaced

| Before | After | Reason |
|---|---|---|
| `threads_state` `headings_in_fences_do_not_become_sections` (buffer scan) | `the_outline_lists_the_rendered_fixture_headings_with_distinct_targets` | Same guarantee (fenced headings absent) on the renderer parse, using the full fixture: duplicate Review headings, skipped level, Unicode, long heading |
| Outline popup-menu clicks (`threads_focus_tests`, `threads_tiled_tests`) | `outline-entry-{n}` clicks; the caret lands at the heading line | The dropdown became the Outline region; Edit puts the caret at the heading instead of selecting its text |
| `edit_note_and_connections_stay_in_the_toolbar_when_this_notes_row_fits` | `edit_note_stays_in_the_toolbar_and_connections_lives_in_the_document_menu` | Connections and Document history moved to ⋯ |
| Workbench Connections toolbar clicks | `open_connections` through ⋯ | Same journey, new entry point |
| Format ▾ folding at 1250 | Same assertion at 980 | The 1250 row now fits all five buttons beside the assistant |
| Centred column width 728 | `metrics::DOCUMENT_WIDTH` | Column follows §8 (680–760 with page inset) |
| `overall-feedback` always visible | `toggle-feedback` visible; budget tests open feedback first | Feedback became on demand |
| Review label "+a −r", "Dismiss" | "N added · M removed", "Discard" | Change semantics in words; spec naming |
| `read_view_shows_block_html_literally_…` expected text | Adds "Image unavailable · gone" | The missing image is now a labelled placeholder |

Added:
- `outline_targets_are_bound_to_their_document_generation`: stale target refused after an edit, Save and reopen; no navigation.
- `a_compact_window_uses_a_temporary_contents_view_that_keeps_the_preference`.
- `a_document_without_headings_says_no_headings_yet`.
- `the_launcher_outline_command_toggles_the_docked_outline`.
- `outline.rs` unit tests: current section; generation and range binding.
- `layout.rs` `a_re_anchored_comment_says_it_moved_and_an_untouched_one_does_not`.
- Conflict, not Saved, assertions on the stale-Save test.

## Commands and results

Environment:
- Rust 1.98.1, `--locked --offline --jobs 4`.
- `TMPDIR` is a fresh directory under `/private/tmp`.
- `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN` point at the built document helpers.

The steps follow `scripts/verify-desktop-shell.sh` and were run one by one. The final run was at the head named in the PR or handback.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-writing-ui --features native-ui,native-test-support` | 51 + 12 passed, 0 failed |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-desktop --features native-ui,native-test-support` | 392 passed, 0 failed, 1 ignored |
| `git diff --check` | clean |

## Acceptance cases

**A** is automated in-process GPUI tests on the real views and services. **N** is native, in the real app on synthetic data.

| Case | Evidence | Gaps |
|---|---|---|
| AC-10 direct edit and save | **A**: edit survives refresh, theme, pane toggles and navigation (existing writing and omakase tests); header Unsaved → Saved, Conflict on stale Save. **N**: Saved state in header (shots). | Native typing and Save not driven (see below) |
| AC-11 passage comments and rewrite | **A**: comment anchoring and re-anchoring; "moved" status; unresolved quote kept (existing comment tests + new unit test). **N**: at 900, commenting on an unsaved edit is refused with "Save your edit first", in both themes. | A posted passage comment on a draft (VE-04) was not captured natively |
| AC-12 review-required and protected edits | **A**: review summary, Discard leaves the note unchanged, Approve on protected (review tests). **N**: pending proposal at 1280 with assistant open, in Light and Dark: Changes/Result, Added and Removed in words, "saved v1 unchanged until you apply", Apply whole note. | Protected Approve not observed natively |
| AC-14 apply, reopen, Undo | **A**: apply then reopen; receipt Undo; editor undo cannot resurrect stale text (`local_editor_undo_after_apply_…`). **N**: Apply shows the v2 text, "Working · v2" and "Saved." in both themes. | A proposal that is not a Rewrite shows no in-note Undo; its Undo is in Recent changes (see gaps). Undo not driven natively |
| AC-17 history | **A**: history views labelled, preview does not change current content (existing tests); ⋯ Document history. | Native not captured |
| AC-24 save failure and retry | **A**: Conflict keeps the edit, never Saved; recovery buffer retained. | A controlled I/O failure is not injectable in tests; "Save failed · Retry" is exercised only by code path |
| AC-41 outline navigation and version correctness | **A**: fixture headings, duplicates, skipped, Unicode, long, fenced; pointer and keyboard; generation-bound refusal; Read/Edit; compact view; launcher. **N**: docked outline with current section at 1280 in Light and Dark; compact contents view at 900 with current entry, then the document at the chosen section after Go, in both themes; "No headings yet" at 900 in both themes. | Native keyboard traversal not captured |
| AC-42 long-article presentation | **N**: prose, headings, quote, table, code with Copy, wide table, citation links, stored figure in Light and Dark at 1280; the 900 article at Results with table and citation, compact contents view and empty outline at 900. | None beyond the known gaps |

## Native observation

Setup:
- Binary: `target/debug/brn-desktop`, copied as `brn-desktop-s4`.
- Data: a fresh `--data-dir` under `/private/tmp`. It held only synthetic fixture content and was deleted afterwards.
- Seeding used the existing `brn` CLI and services:
  - `intake --intent full` of `fixtures/reading-quality-article.md` ("Cedar Reading Room", Protected).
  - `note-create` of the same article ("Cedar working draft"), a no-heading variant, and a similar-headings variant.
  - `intake` of a synthetic DOCX holding `fixtures/assets/reading-quality-chart.png` ("Figure check").
- Theme was chosen in the app; the OS appearance was never changed.
- A temporary, **uncommitted** in-app step driver (`BRN_S4_SCRIPT`) issued the same requests as the controls: open, read, edit, outline, outline-go, theme, assistant, sidebar rail, append text, select, comment and post, Save, a synthetic proposal through the checked `prepare` service, Result and Apply. It avoided pointer automation on a shared Mac.
- Captures are window-only (`screencapture -l`) in `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s4/`. Window sizes were 1280×928 and 900×628.

| Shot | Observed |
|---|---|
| `light-1280-read-top.png` | Read mode with no title duplicated in a large app title; coverage banner; centred column; quote divider; citation link "Source A" |
| `{light,dark}-1280-outline-results.png` | Docked outline (assistant closed); current section "Results from the invented rounds" with bar, heavier label and tint; nested, skipped and Unicode levels; fenced fake headings absent; table header distinguishable |
| `{light,dark}-1280-outline-code-wide-table.png` | Code block with Copy; wide table cut at the column edge as its own horizontal scroll viewport; current section follows scrolling (earlier build of this branch) |
| `{light,dark}-1280-stored-figure.png` | Stored asset fits the column, keeps aspect ratio, not inverted in Dark; caption below |
| `{light,dark}-900-outline-empty.png` | 900×600: the temporary contents view covers the document region and says "No headings yet"; toggle shows pressed |
| `{light,dark}-900-outline-compact.png` | 900×600 protected article: the temporary contents view lists all levels (duplicate Review, skipped level, Unicode, wrapped long heading) with the current entry bar |
| `{light,dark}-900-after-contents-go.png` | After choosing an entry the view closes and the document sits at "Results from the invented rounds", with table and citation readable |
| `{light,dark}-1280-review-pending.png` | VE-02: pending whole-note proposal beside the open assistant; Changes shows struck "stores short" and underlined "keeps brief"; legend chips "Added" and "Removed"; "saved v1 unchanged until you apply"; Apply whole note. At this note width the bar is compact, so Keep for later, Discard and the counts are in the header ⋯ by design |
| `dark-1280-review-result.png` | Result tab renders the proposed text as the reading view; base still v1 |
| `{light,dark}-1280-review-applied-undo.png` | After Apply: v2 text, "Working · v2", header Saved, notice "Saved."; no in-note Undo for this non-Rewrite proposal |
| `{light,dark}-900-comment-needs-save.png` | Edit at 900 with the sidebar rail: header "Unsaved · ⌘S", footer "Unsaved edit · recoverable", Save ⌘S primary; commenting is refused with "Save your edit first: comments anchor to the saved version of the note."; empty comments margin "No comments yet" |

The 900 outline shots show the sidebar Search tooltip ("Search notes, chats and commands ⌘K") over the Read/Edit switch. That is a hover tooltip from the parked pointer, not layout.

Observed and fixed earlier in the slice:
- Debug prints left by the previous helper were removed.
- The scrollbar overlapped text, so a gutter was added.
- A missing image showed only alt text, so a bounded placeholder was added.

Not captured natively:
- VE-04 with a posted comment: a saved draft with a passage comment in the margin, then a further unsaved edit, at 900×600 in both themes.

Two other helper apps (S3, S5) shared the Mac and held the Stage Manager stage. A freshly launched window, captured at once, reached the stage for most shots. For the final VE-04 sequence, every fresh launch stayed in the strip; those strip thumbnails were not kept. The screen was not locked. Comment posting and the margin are covered by the automated tests above. **Native validation of VE-04 with a posted comment is pending.**

## Known gaps

- **Fixture image.** Markdown intake turns the article's relative image into the text `[Image: alt (assets/…)]`, because the converter receives bytes only. This is intake behaviour outside S4. The stored-image path was therefore observed through the synthetic DOCX "Figure check". A missing stored image renders the labelled placeholder.
- **Outline preference.** Visibility is remembered by document id for the session only; it is not persisted across restarts. This belongs with the lead's layout persistence.
- **Folds.** Optional per-branch folding of the outline is not implemented; the spec makes it optional.
- **Saving… state.** The checked Save is synchronous, so "Saving…" shows only for its duration.
- **Save failure.** Natively unobserved. The lead's review fix below covers it with a held SQLite write lock in tests.
- **Not exercised natively:** large text, IME, VoiceOver, and a 5,000-note responsiveness check.
- **Undo after a plain proposal.** The in-note "Applied · vN" status with Undo appears for an applied Rewrite. Other applied proposals show "Saved." and keep their receipt Undo in Recent changes (CAP-201, S6). An in-note Undo for every Apply would need the shell notice to carry an action, which is outside S4 ownership.
- **Wide tables** scroll inside their frame but show no persistent scroll cue at rest; the cue is gpui-kit's.

## Lead fixes after independent review

Sol 6.1 reviewed `7dc1c794..aeab9a80` and reported two major findings and one minor finding. The lead fixed all three on the integration branch.

- **A failed first-dirty preservation showed "Saved".**
  - *Problem:* if `begin_edit` failed, for example because the database was busy, there was no edit session and the header said Saved.
  - *Fix:* the panel now records the failed preservation for that note, and Saved needs the visible text to equal the committed note. Text that is neither kept nor committed shows "Save failed · Retry" or Unsaved. Retrying Save while still busy keeps showing the failure.
  - *Test:* `a_failed_first_dirty_preservation_never_shows_saved` holds a real `BEGIN IMMEDIATE` lock from a second connection (WRITE-02, AC-24).
- **⋯ Document history opened workspace-wide Recent changes.**
  - *Fix:* it now opens this document's own history in place of the document (`threads_doc_history.rs`):
    - accepted versions, newest first;
    - a read-only preview of an earlier version, labelled "Earlier version N · read-only. The current document (vM) is unchanged.";
    - Back to versions and Back to current;
    - Restore version N, which puts that text in the editor as an unsaved edit of the current version. The ordinary checked Save then makes it a new current version with its own receipt and Undo, and every earlier version stays in history.
  - *Refusals:* Restore is refused while an edit is unsaved, another edit holds the note, or a proposal is in review.
  - *Test:* `document_history_previews_and_restores_through_a_checked_save` (AC-17).
- **An empty compact outline lost Escape.**
  - *Fix:* with no entries, the contents view now focuses Close, so Escape returns to the document and focus returns to the toggle.
  - *Test:* `an_empty_compact_outline_still_closes_with_escape` (READ-03, AC-41).
