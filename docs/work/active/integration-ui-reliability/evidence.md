# UI polish and daily-workflow reliability: integration evidence

**Date:** 16 October 2026. **Branch:** `work/integration-ui-reliability`, local only with no upstream, worktree `.worktrees/integration-ui-reliability`.

**Built from:** `work/ui-polish` `354c1940`, then a `--no-ff` merge of `work/daily-workflow-reliability` `192cd765` (`067470d5`), then a `--no-ff` merge of main `a4ad9677` (`6f51d22c`). The integration commit `649494d6` is on `6f51d22c`, and fix pass 1 (the nine findings of [review.md](review.md)) adds one commit on `649494d6`. `work/ui-polish` `81129f07` (docs and shots after `354c1940`) is not merged here; the record step merges it.

**Sources:** [UI polish evidence](../ui-polish/evidence.md), [reliability evidence](../daily-workflow-reliability/evidence.md), the coordination board's owner decisions of 16 October 2026, and the UI branch's native recheck of `354c1940` (R1–R4, in that branch's `verification.md` at `81129f07`).

Each status is reported on its own: implemented or verified does not mean owner-accepted or merged.

| Item | Implemented | Automated-verified | Native-observed | Owner-accepted | Merged |
|---|---|---|---|---|---|
| UI polish S1–S7 and both fix passes | yes (`work/ui-polish`) | yes, on this tree | on its own builds (`ed8daa82`, recheck `354c1940`), not on this build | text-floor rule yes (16 Oct); the rest no | no |
| Reliability rounds 1–6 | yes (`work/daily-workflow-reliability`) | yes, on this tree | on its own builds (rounds 4 and 6), not on this build | decisions 1–3 yes (16 Oct); the build no | no |
| Test expectations for the reliability service (§1) | yes | yes | decisions 1 and 2 seen on `6db39420` ([native recheck](#native-observed-recheck-of-6db39420)) | n/a | no |
| F-UI1: "In chats" in Notes search and ⌘K (§2) | yes | yes | yes, on `6db39420` ([native recheck](#native-observed-recheck-of-6db39420)) | no | no |
| U4: an edit already open elsewhere (§3) | yes | yes | yes, on `6db39420` ([native recheck](#native-observed-recheck-of-6db39420)) | no | no |
| Data-folder startup message (§4) | yes (test of the existing path) | yes | no | no | no |
| U10: one recorded notice on four surfaces (§5) | yes | yes | yes, on `6db39420` ([native recheck](#native-observed-recheck-of-6db39420)) | no | no |
| U11: an archived chat focuses Unarchive (§6) | yes | yes | yes on open and after ⌘K; ⌘L not tried ([native recheck](#native-observed-recheck-of-6db39420)) | no | no |
| R1–R4 from the UI recheck (§7) | yes | yes | R1 yes; R4's Needs you row seen; R2, R3 no ([native recheck](#native-observed-recheck-of-6db39420)) | no (R1 is a lead decision of 16 Oct that the owner can overrule) | no |
| Contract and docs (§8) | yes | link check | n/a | decisions 1–3 yes; wording no | no |
| Fix pass 1: review findings 1–6, 8 and 9 ([Fix pass 1](#fix-pass-1-review-of-649494d6)) | yes | yes | 1, 2 and 5 yes; 3, 4, 6, 8 and 9 no ([native recheck](#native-observed-recheck-of-6db39420)) | no | no |
| Fix pass 1: finding 7, Windows compile | no (no Windows target here); a pre-merge CI run is required | no | n/a | no | no |

Nothing was pushed, merged, published or released. No Mac window was opened in the integration step or in fix pass 1. The later native recheck of `6db39420` ran under `MAC-IN-USE` ([Native-observed](#native-observed-recheck-of-6db39420)).

## What changed

All product changes are in `crates/brn-desktop`. No shared seam (`brn-ai/src/threads.rs`, `brn-threads-app/src/{chat,intake}.rs`, `brn-threads-core`, the search module) was edited. `useful_intake`'s logic and both `interrupt_chat_run` call sites (`pause_failed_worker`, `resume_chat_queue`) are unchanged. No dependency, `Cargo.lock` or schema change.

### 1. Tests with new expectations

| Expectation | Test | Note |
|---|---|---|
| A settings change records Interrupted, not Cancelled | `intake_tests::a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice` (reliability) | Runs on this tree. No UI test expected Cancelled after a settings change. |
| A seeded Working Run with no lease counts as crashed | Resume: `intake_tests::resume_after_a_kill_…`, `…_killed_queued_intake_shows_one_notice` (reliability). Steer: new `steering_tests::steer_of_a_reply_no_process_holds_records_the_interrupted_notice`. Continue: new `intake_tests::continue_in_a_chat_whose_reply_no_process_holds_interrupts_it`. Dispatch: service `run_recovery` tests. | |
| A seeded Run that stands for a live worker holds a `RunLease` | `steering_tests` fixture (Working state), `dialogs_tests::working_run`, `projects_tests::native_project_delete_…`, and (fix pass 1) `chat_tests::hold_worker`, used by `queue_notices_follow_the_queue_state_and_paused_wins_after_stop`, `while_work_runs_send_reads_queue_…` and `a_short_window_keeps_a_running_queue_…` | `stop_and_send_click_is_durable_…` now also asserts that Steer of live work adds no notice and leaves the lease file. The project delete test now asserts the chat's Interrupted notice, which project commands always record. The queue-notice test's Stop now goes through the live path and asserts that it adds no notice for the Run. The workbench `run_fixture` holds none: its tests never Steer, Resume or Dispatch, and the owner's Stop adds no notice either way. |
| The owner's U1 Stop of a lease-free Run: no notice, Review items, Interrupted | new `intake_tests::the_owners_stop_of_a_killed_reply_adds_its_review_but_no_notice` (clicks Stop… and the dialog's confirm) | `chat_tests::a_saved_working_reply_stops_only_…` keeps its Interrupted assertion. |
| Starting a Run clears the chat's notice (decision 1) | new `intake_tests::continue_clears_the_interrupted_notice_when_the_next_run_starts` | The "Start a fresh invocation" change's chat attention is the Review only. |
| Stop, Archive or Resume with a prepared proposal adds a Review | Stop: the new test above. Archive: `archiving_a_killed_chat_adds_its_review_but_no_notice`. Resume: `resume_after_a_kill_…`. | |
| The old "Run failed; check provider…" text is gone | `brn-ai` notice tests | It survives only in the legacy lists that recognise saved notices (`run_attention.rs` `LEGACY_NOTICES`, a `brn-ai` test fixture); nothing produces it. No desktop test asserts it. |
| Ten agent tools; bundle `brn-threads-guidance-12` | `brn-ai` `threads.rs` (10 tools), `threads_guides.rs` | Main `a4ad9677` uses bundle 11, so 12 is not taken and was kept. |
| `search.sqlite` v4 rebuilds on first open | `brn-threads-app/tests/search_messages.rs` (v2 and v3 rebuild) | |
| Temp paths canonicalize | every desktop test root uses `tempdir_in(temp_dir().canonicalize())`; reliability's intake tests compare canonical locators | No UI test compares a raw temp path with `Source.locator`. |

No assertion was weakened. The one changed assertion (`pages_tests::kept_edits_read_recover_edit_with_an_unsaved_chip`) follows R4's new row text and gains layout checks.

### 2. F-UI1: saved chat replies in search

- **Notes search** (`threads_search.rs`): an "In chats" group under "In note text". Each hit shows the chat title, the role ("You" or "BRN"), the highlighted snippet, and an Archived chip when `archived` is true. Activating a hit opens the chat and scrolls its transcript to that Message (`MessageScrollerState::scroll_to_item`, once).
- **Paging:** notes and Messages are separate lanes. A new query fills both from one answer. Each lane's Previous/Next requests its own cursor (`cursor` or `message_cursor`) and updates only its own page. Each lane waits only for its own request; a new query's request is both lanes' (fix pass 1, finding 4).
- **States:** "Searching…" while a query runs, "Searching chats…" while the chat lane pages, a "N messages · a–b" status line. The group stays hidden while a new query's answer is pending (an explicit flag since fix pass 1, so paging both lanes keeps it) and while the page has no Messages. When notes have no text hits but Messages exist, the notes empty state says "Matching chat messages are listed below."
- **Archived label:** a Message hit and a note hit from an archived chat carry the same neutral Archived chip (`hit-N-archived`, `chat-hit-N-archived`), which joins the row's announced name (fix pass 1, finding 8; the note hit's lowercase "archived chat" fact is gone).
- **⌘K:** the launcher runs its own first-page owner search (up to 5 Messages) as the query changes and lists them under "In chats", after every label match (Chats, Notes, Commands), with the snippet highlighted, the chat and role as muted metadata and the Archived chip. They arrive after the label matches, so they come last: the answer landing never moves a row the selection or ↵ points at, and a theme command whose name is also in saved chat text keeps ↵ (fix pass 1, finding 1). ↵ opens the chat at the Message. While the search runs, "Searching chats…" shows instead of "No matches" when nothing else matches, and as a last line below the label matches otherwise. Group IDs replace spaces with `-` (`launcher-group-in-chats`).
- Tests: `tiled_tests::notes_search_lists_saved_chat_messages_and_opens_their_chat` (the round 5 section F repro, "berth", archived label after Archive, transcript scrolled to the hit), `tiled_tests::note_hits_and_chat_messages_page_independently` (14 notes and 14 Messages, each lane paged alone, chat paging available while a notes page loads, the group kept while both lanes page), `tiled_tests::an_archived_chats_note_hit_reads_archived_like_its_messages`, `omakase_tests::launcher_lists_saved_chat_messages_and_opens_their_chat`, `omakase_tests::launcher_keeps_a_matching_command_ahead_of_chat_messages` ("raba": the theme command stays row 0 after the answer, ↵ switches the theme, ↓ ↵ opens the chat), `lists::tests::launcher_lists_matching_chat_messages_after_the_label_matches`.

### 3. U4: an edit already open elsewhere

- `note_changed` matches `AppError::Core(Error::EditSessionOpen { note, base_version, .. })`, never the text, and uses only `note` and `base_version`.
- The note becomes read-only like any kept edit. The notice reads "An edit of this note is already open from vN. What you typed here isn't saved."
- Its banner (`note-edit-open-banner`) replaces the generic kept-edit banner: "An edit of this note is already open from vN and is kept. What you typed here isn't saved. Compare it with the saved note, recover it, or discard it." The actions are Recover edit (the existing primary), Compare and Discard edit….
- **Compare** opens a read-only dialog with "Open edit · from vN" beside "Saved · vM".
- **Discard edit…** reuses the existing `discard-note` consequence dialog, worded for the other edit since fix pass 1 (finding 5): "Discard the edit open elsewhere?" / "Its changes since vN will be lost. What you typed here is kept. This can't be undone." The owner's own Discard… keeps "Discard your unsaved edit?" / "Your changes since vN will be lost. This can't be undone." It discards the open edit that `recovery_buffers` lists for the note, then keeps what was typed here as this view's new unsaved edit.
- Test: `writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard` (a second `Workspace` opens the edit, as another window or the CLI would).

### 4. Data-folder error

The desktop's startup path (`main.rs` `launch`, headless and native) already prints the core's owner-terms `DataFolder` text through `Display`. AppError `Core` is `#[error(transparent)]`, and no desktop path formats errors with `{:?}`. New test `tests::an_unusable_data_folder_reads_in_owner_terms_at_startup`: a symlinked parent gives "…part of its path is a symbolic link. Choose the folder's real location." (`SymlinkRefused`), a file in the path gives "…is a file rather than a folder.", with no "os error", "DataFolder", "Io(" or "unsafe data directory" text, and nothing created. The gate's `--data-dir /data` run printed "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." (exit 1). In the native app a startup failure is still reported only on stderr, before any window opens, as before this step; see Open items.

### 5. U10: one recorded notice

For a Failed or Interrupted Run, the chat's Blocker attention whose `record` is that Run is the source on every surface:

- the status line (UI branch, `terminal_notice`, unchanged)
- Needs you (attention reasons, unchanged)
- the chat's recovery card reason line (`Recovery::with_notice`)
- the Note's `writing-run-status` (`RunOutcome::Recorded`)

The Run's progress, the agent's own summary, is a separate muted line below the notice, only when it differs from it: `{card}-detail` on the card, and since fix pass 1 (finding 2) `writing-run-detail` in the Note, whose `writing-run-status` is the notice alone. The card's announced name reads every visible line, so it includes that progress line. `threads-behavior.md` § State and attention now says the progress may follow as a separate secondary line. Without a recorded notice (the owner's Stop records none) each surface keeps its previous text. Tests: `state::tests::a_recorded_notice_is_the_recovery_reason_and_progress_is_detail`, `chat_tests::a_failed_reply_reads_as_its_recorded_notice_on_the_card_and_in_needs_you`, `polish_tests::a_failed_rewrite_reads_as_the_chats_recorded_notice`, `workbench::tests::a_recorded_notice_leads_and_progress_is_secondary`, and the existing `steering_tests::a_failed_or_interrupted_finish_shows_the_recorded_terminal_text`.

### 6. U11: archived chat

On this tree the archived chat already had one Unarchive (the composer banner's `reopen-thread`) and an unavailable Continue. The composer still took focus when the chat opened, and that is fixed: the focus landing for an archived chat is its banner, and after the next frame focus moves to Unarchive. Since fix pass 1 (finding 6) `focus_composer`, which ⌘L (`FocusAsk`) and quoting a selection into the chat use, lands there too; a quote still goes into the composer's text. Opening the chat or ⌘L after Unarchive focuses the composer. Test: `chat_tests::archive_states_its_consequence_and_an_archived_chat_has_one_unarchive` (focus assertions on open and on ⌘L, before and after Unarchive).

### 7. R1–R4 from the UI branch's recheck of `354c1940`

| Item | Change | Test |
|---|---|---|
| R1: the writing bar clipped with a stacked comment draft at 900×600 | **Lead decision, 16 Oct 2026, extending the owner-accepted banner rule to the drafting state; the owner can overrule** (option A of three; the others were to let the draft input scroll out of view, reversing fix 3b, or to leave the bar clipped). While a draft is open, the draft keeps its whole input and the text gives way to the 48 px floor, as under a banner. The text still asks for 120 px and gives way last. The whole bar, including a filled Retry saved rewrite, stays in the Note, with or without a notice row. Measured at 900×600: the text gets 48 px without a notice row and 68.5 px in the run with one. plan.md's height-share rule records it and names both exceptions to the 120 px floor (the banner stack and an open draft). | `review_tests::an_open_stacked_draft_keeps_the_whole_writing_bar_at_900x600`: the bar is whole, the draft input is whole (the same `assert_draft_in` that fix 3b's test uses), and `note-body` is at least 48 px. Before the fix the bar ended at 579.5 px against the Note's 508 px. Fix 3b's whole-input assertion in `stacked_comments_keep_every_comment_capability` is unchanged. |
| R2: Edit note said "BRN is working" while the status bar said "Idle" | The gating is unchanged: a saved queue item still blocks Edit note. The reason now reads "a queued message is paused · Resume or clear the queue" (or "a queued message is about to run"), and "BRN is working" appears only with a worker. With nothing running and saved queue work, the status bar shows "Queue paused · N waiting" (or "Queue · N waiting") instead of "Idle". | `context_actions_tests::a_paused_queue_blocks_edit_note_with_a_reason_the_status_bar_shares` |
| R3: raw JSON in Changes' "May need refresh" pills | Names in owner terms: a rewrite request reads "Rewrite request · {note}", another chat Message reads "Chat message", and other records keep their title, whitespace collapsed. The pill is one line that ellipsizes inside the panel, and the tooltip and accessible name carry the full name. Since fix pass 1 (finding 9) the rewrite request's note title is resolved once per `refresh()`, when history reloads, into `refresh_names`; rendering Changes reads no request snapshot. | `polish_tests::changes_name_a_rewrite_request_that_may_need_refresh_in_owner_terms` |
| R4: "Recov… [Unsaved]" at 170 pt | Kept-edit rows lead with the note's title, with "Recover edit" as the reason line. In the Chats panel the Unsaved marker is its icon, announced and in the tooltip as "Unsaved". Home keeps the text chip. | `pages_tests::kept_edits_read_recover_edit_with_an_unsaved_chip` (title 94 px of the 170 pt row, icon chip 34 px) |

### 8. Contract and docs

- `threads-behavior.md`:
  - State and attention: decision 1 (a notice clears when the next Run starts; this replaces "keeps its Interrupted notice until the next Run in its chat records a result"), one notice per chat naming its cause, Interrupted for stops from outside the chat, the four surfaces, and decision 2 for Archive and the owner's Stop.
  - Stop and recovery: leases and orphan recovery.
  - A new Search paragraph: decision 3, with labelled archived hits and agent retrieval unchanged.
  - Source identity: real paths, one instruction and reuse.
  - Ten tools and `withdraw_proposal`, plus the bundle-12 sentence.
- `invariants.md`: orphan recovery and the round 6 lease and notice sentence, the backup and restore stage sweep, and the data-folder refusal.
- `threads-target.md:136` and `chats-and-projects/spec.md:70`: the currency correction for owner search.
- `docs/status.md`: one entry covering both branches, and nine to ten registered tools.
- `ui-polish/plan.md` and `ui-polish/evidence.md`: the text-floor rule under a banner is **owner-accepted 16 Oct 2026** (unchanged rule), and the R1 extension is recorded under "Height share (round-5 N3)".
- Fix pass 1: `threads-behavior.md` § State and attention adds that the Run's own progress, where it says something else, follows the notice on the card and in the Note as a separate secondary line, never as part of it.

## Fix pass 1: review of 649494d6

[review.md](review.md) and `review.json` (verdict NEEDS_CHANGES, one blocking finding) reviewed `649494d6`. Every product change is still in `crates/brn-desktop`; no shared seam, dependency, `Cargo.lock` or schema changed.

| # | Finding | Change | Test |
|---|---|---|---|
| 1 (blocking) | ⌘K's ↵ target moved when chat hits arrived | `lists::launcher` lists the label matches (Chats, Notes, Commands, at most 20) first and the owner search's Messages after them, so the answer only appends rows. While the search runs, "Searching chats…" is the list's last line. | `lists::tests::launcher_lists_matching_chat_messages_after_the_label_matches` (the "raba" command keeps index 0 and every earlier row with and without Messages), `omakase_tests::launcher_keeps_a_matching_command_ahead_of_chat_messages` (a Message "Raba walk on Saturday": after the answer, row 0 is the theme command, ↵ switches the theme and opens no chat, ↓ ↵ opens the chat) |
| 2 | Progress appended to the recorded notice | `writing-run-status` is the notice alone; the progress is `writing-run-detail`, a muted small line below it, only when it differs (`RunOutcome::detail`). The card already showed it as `{card}-detail`; its announced name keeps every visible line. The contract sentence is added (§8). | `workbench::tests::a_recorded_notice_leads_and_progress_is_secondary`, `polish_tests::a_failed_rewrite_reads_as_the_chats_recorded_notice` (status label is exactly the notice; the detail sits below it in the Note) |
| 3 | `hold_worker` stood for a live worker without a lease | It acquires the Run's `RunLease` and returns it with the sender; all three callers keep it for the test. | `queue_notices_follow_the_queue_state_and_paused_wins_after_stop` now also asserts that the owner's Stop of live work adds no attention item for the Run |
| 4 | Chat paging not independent of notes | An explicit `fresh` request (a new query's) replaces the inferred "both lanes pending". Chat paging waits only for the chats lane, and the group hides only for a new query. | `tiled_tests::note_hits_and_chat_messages_page_independently`: while a notes page is in flight, chats' Next reads "Next →" and pages; with both lanes in flight the group stays with "Searching chats…"; both answer to `((14, 0, 12), (14, 12, 2))` |
| 5 | U4 Discard dialog worded as the owner's own edit | The open-edit path has its own title and body (§3); the owner's Discard… is unchanged. | `writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard` (body), `polish_tests::discard_always_confirms_and_enter_never_discards` (the owner's own body, unchanged) |
| 6 | ⌘L focused an archived chat's composer | `focus_composer` sends focus to the archived banner and then Unarchive, as `focus_landing` does (§6). | `chat_tests::archive_states_its_consequence_and_an_archived_chat_has_one_unarchive` (⌘L from the root: Unarchive focused, composer not; after Unarchive ⌘L focuses the composer) |
| 7 | Windows native-ui compiled only after merge | Not resolvable here: no Windows target is installed, and this pass stays local. **Before the combined PR merges, dispatch `ci.yml` (`workflow_dispatch`) on the PR branch** so the native-ui and native-retrieval lanes build and test on `windows-2025` and `ubuntu-24.04`; on a pull request they run on `macos-15` only. Accepting the risk instead is the owner's call. | — |
| 8 | Archived note hits read "archived chat" | One `archived_chip` helper gives note and Message hits the same Archived chip, joined to the row's announced name (§2). | `tiled_tests::an_archived_chats_note_hit_reads_archived_like_its_messages` (`hit-0` reads "Curlew attachment · Archived", chip `hit-0-archived`). The Notes search has no project scope, where the service lists such notes, so the test sets the hit's `archived` directly. |
| 9 | Store reads in the Changes render path | `refresh()` resolves each flagged rewrite request's note title into `refresh_names` when history reloads; `refresh_name` reads that map (§7). The pre-existing per-receipt `refresh_records` read in the same row is unchanged. | `polish_tests::changes_name_a_rewrite_request_that_may_need_refresh_in_owner_terms` (unchanged expectations, passing) |

**Measured while checking finding 2 (not a test, not committed).** In the busiest 900×600 Note state (stale banner, open stacked comments, a failed rewrite with its partial reply, a dirty writing bar) **with a recorded notice** ("Run failed: The provider account needs reconnecting. Continue when the provider is ready.") and a 76-character progress summary, the text sits at its 48 px floor and the writing bar ends 2.5 px below the Note (`note-bar` 436.5 + 132 px against the tile's 566 px). With `649494d6`'s inline "notice · progress" the same state put the bar 8.5 px below. The committed busiest-state test records a failure without a notice, which reliability now always adds, so it doesn't reach this state. In the GPUI test text system, which is wider than macOS fonts. Not native-observed. Which row gives way when every row is at its floor isn't settled by the owner rule, so nothing was changed for it (Open items).

## Verification

Run on the final tree of the fix pass 1 commit, from the worktree with `CARGO_TARGET_DIR=<worktree>/target`, `--locked --offline`, Rust 1.98.1, macOS arm64. `TMPDIR` and all data were fresh synthetic folders under `/private/tmp/brn-integ-gate2/`. The desktop and CLI binaries in the last rows come from the same build. No provider call, credential or owner data was used. The first run, on `649494d6`, gave the same results except the desktop-shell lane (292 passed then; fix pass 1 adds two tests).

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | ok |
| `cargo build --workspace --bins` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **827 passed, 1 failed, 6 ignored.** The failure is `brn-threads-intake` `qualification::prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` ("Pdf preparation failed: Unavailable"). Poppler is not installed here, so this is a known environment gap, **not a pass**. |
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` (helper and desktop build, clippy `-D warnings` on all targets, `native-ui,native-test-support` tests) | **294 passed, 0 failed, 1 ignored**; the ignored test needs a CLI intake's shared data (reliability's cross-surface check) |
| `cargo clippy -p brn-desktop --features native-ui --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui` | **80 passed** |
| `cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui,native-retrieval` | **80 passed** |
| `cargo clippy -p brn-desktop --all-targets -- -D warnings` (default features) | clean |
| `cargo test -p brn-desktop` (default features) | **30 passed** |
| `cargo build -p brn -p brn-desktop --bins` (the binaries below) | ok |
| `TMPDIR=… scripts/verify-end-to-end.sh --fixtures-only` | passed (provider and intake retirement; offline Save/recovery/search/export/backup/startup/refusal journeys) |
| `cli-journey.py target/debug/brn /private/tmp/brn-integ-gate2/data/cli-journey` | exit 0. 43 steps: 40 succeeded, and 3 were refused as designed (two `begin-edit` with `edit_session_open`, one occupied backup destination), as in reliability round 6 |
| `brn --help`, `brn-desktop --help` | ok |
| `brn-desktop --data-dir <fresh> --headless-check startup` | "Threads ready: 2 records" |
| `brn-desktop --data-dir /data --headless-check startup` | refused, exit 1: "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." |
| `python3 scripts/check-markdown-links.py` | 142 files, 576 local links, 0 failures (run again after this record was written; the extra file is the committed [review.md](review.md)) |
| `git diff --check` | clean |
| `git diff --exit-code a4ad9677 -- Cargo.lock` | clean |

**Fail-before checks.** Each fix was disabled in place, its test was rerun, and the source was restored:

- U11: with the archived-chat focus guard off, `archive_states_its_consequence_…` failed (the composer took focus).
- F-UI1: with the transcript scroll off, `notes_search_lists_saved_chat_messages_…` failed (the hit's Message was not in view).
- R1: with the draft-open floor change reverted, `an_open_stacked_draft_keeps_the_whole_writing_bar_at_900x600` failed (the bar ended at 579.5 px, below the Note's 508 px).
- Fix pass 1, finding 1: with `649494d6`'s launcher order restored, `launcher_lists_matching_chat_messages_after_the_label_matches` failed (row 0 was the Message, not `Theme(Raba)`) and `launcher_keeps_a_matching_command_ahead_of_chat_messages` failed (`launcher-item-0` read "Message in Weekend, You: Raba walk on Saturday").
- Finding 4: with chat paging gated on either lane again, the paging test failed ("Next, unavailable: search is preparing" while a notes page loaded); with the group hidden on "both lanes pending" again, it failed with `search-chats` missing.
- Finding 6: with the `focus_composer` guard off, `archive_states_its_consequence_…` failed on ⌘L.
- Finding 2: with the inline "notice · progress" restored, `a_failed_rewrite_reads_as_the_chats_recorded_notice` failed on the status label.

The other new tests assert text or state that did not exist before this step (the "In chats" rows, the U4 banner and dialogs, the launcher's Message items, the U10 card detail, the R2–R4 wording, the note hit's Archived chip). Their pre-change values are named in the test comments.

**Not verified:**

- Windows was not compiled; no Windows target is installed. Neither this step nor fix pass 1 adds a platform-specific call, and the one Unix-only test is `#[cfg(unix)]`. On a pull request, CI runs the native-ui lanes on `macos-15` only, so the manual `ci.yml` run on the PR branch (finding 7) is the first Windows compile of this desktop code.
- No native observation in this verification run. The later [native recheck](#native-observed-recheck-of-6db39420) of `6db39420` covers F-UI1, U4, U10, U11, R1, Continue and the killed-then-archived chat; R2, R3 and the items it lists as not observed remain automated-verified only, in the GPUI test text system, which is wider than macOS fonts.
- No two-process lease check beyond the reliability branch's in-process one.

## Native-observed: recheck of 6db39420

This section reports only what was seen on screen and in the stores with real synthetic input on 16 October 2026. It is not owner acceptance and nothing was merged. Shots are in [native/](native/), downscaled to 900×600, which is 1:1 with window points.

**How:**

- **Go-ahead and lock.** This step asked the parent for a go-ahead before opening any window. The orchestrator replied "GO" (the owner gave it in advance) and allowed a Codex store with no credentials, under conditions that were met (below). `MAC-IN-USE` was absent; this step created it at 12:25 and deleted it at 12:39:43. Windows were open from 12:26:58 to 12:37:17 (main store, quit with ⌘Q) and from 12:37:40 to 12:39:23 (continue store, quit with ⌘Q). Afterwards `pgrep` found no `brn-desktop` process and ScreenCaptureKit listed no BRN window.
- **Build.** One `cargo build -p brn -p brn-desktop --features brn-desktop/native-ui --bins --locked --offline` at `6db39420`, in the worktree's `target/`. `brn-desktop` sha256 `fddf34e3…a6bc8`, `brn` sha256 `a8fafd90…db72`. The CLI calls during the check used that `brn`. The desktop binary ran directly with `--data-dir`, with no app bundle.
- **Data.** Two fresh stores under `/private/tmp/brn-integration-recheck-20261016/data/`. They were seeded through the shared `Workspace` service by a temporary example ([native/recheck-seed.rs.txt](native/recheck-seed.rs.txt)), built in a `git archive` export outside Git. `HOME`, `CODEX_HOME` and `TMPDIR` pointed at empty scratch folders, which were still empty at the end.
  - *main:* provider `offline-fixture`, no credentials. It holds the notes, a protected decision, "Pilot proposal" with two comments and a prepared rewrite, "Crane arrival plan" with a Failed rewrite Run and its recorded notice, a killed reply (a Working Run that no process holds, an unlocked lease file and one prepared proposal), the chats "Harbour berth survey" and archived "Replies" that mention "berth", "Weekend" with "Raba walk on Saturday", and a paused queue.
  - *continue:* Codex `gpt-6.1-sol` with `auth_file` null and `credentials_dir` null. `brn settings` on a copy confirmed both before launch. It holds a Working Run left by `prepare_run`, which the delegation choice then interrupted with the notice.
- **Input and capture.** `AXIsProcessTrusted()` returned `true`. [`input.swift`](../ui-polish/observation/input.swift) posted the clicks, keys and scrolls. Text went in through System Events `keystroke`, because the helper's Unicode typer reached the Notes filter but not the ⌘K field (a harness gap). [`wincap.swift`](../ui-polish/observation/wincap.swift) captured the BRN window only. The observation harness was neither built nor used, and nothing was compiled into `crates/` or `scripts/`.
- **No provider call.** The main store's `offline-fixture` provider starts no Run, and none was started there. In the continue store, `lsof -i` on the BRN process showed no internet socket before Continue, in four samples over 1.2 s after it, or after the failure. With no auth file and no credentials folder, `provider_client` returns `ReconnectNeeded` before it builds any client (code reading). Sampling alone can't rule out a connection shorter than about 300 ms, but that code path opens none.

**Results:**

| Check | Native result on `6db39420` | Status | Shots |
|---|---|---|---|
| F-UI1 Notes search | "berth": "In note text · 1 match · 1–1" (Harbour logistics), then "In chats · 4 messages · 1–4". Two hits from Harbour berth survey (You, BRN) and two from Replies, each with the Archived chip; the tooltip reads "Replies · You · Archived". The archived BRN hit opened Replies, scrolled to "Saved the berth depth reply…". | **pass** | 01, 02 |
| F-UI1 ⌘K | "berth": the Chats match "Harbour berth survey", then "In chats" with four rows, chat · role metadata, and Archived chips on the Replies rows. | **pass** | 03 |
| ⌘K order (fix pass 1, finding 1) | "raba": "Theme: Raba · dark bog peat · red" is row 0, and "Raba walk on Saturday" (Weekend · You) comes after it once the answer lands. ↵ applied Raba ("Raba applied. It is saved for next time once no work is running.") and opened no chat. | **pass** (↓ ↵ onto a Message not tried) | 04, 05 |
| U4 | Vendor comparison was open in Write in the desktop. Then the CLI, on the same data, ran `begin-edit --base 1` and `update-buffer` (generation 1). Typing in the desktop gave the notice "An edit of this note is already open from v1. What you typed here isn't saved." and the banner "…already open from v1 and is kept. … Compare it with the saved note, recover it, or discard it." with Recover edit, Compare and Discard edit…. **Compare:** "Compare the open edit", with "Open edit · from v1" (the CLI text) and "Saved · v1". **Discard edit…:** "Discard the edit open elsewhere?" / "Its changes since v1 will be lost. What you typed here is kept. This can't be undone." After Discard: "Discarded the open edit. What you typed here is now your unsaved edit.", with "Unsaved edit · recoverable" and Save ⌘S. `brn recovery` then listed one buffer (base 1, generation 1) holding the desktop's text. Only the first typed character reached it, because the note turns read-only at the first keystroke. | **pass** | 06–09 |
| U10, four surfaces | **Seeded failed rewrite** (Raba): the Note's run status reads "Run failed: The provider account needs reconnecting. Continue when the provider is ready.", with the muted progress "Read the note and drafted a shorter opening." below it. The Needs you row's tooltip and the chat's Needs you card show the same text. The writing chat has no separate Failed card; the Note offers Retry saved rewrite. **Live failure** (continue store, light and Virmalised): the status line reads "Work in Plan the crane lift · Run failed: The provider account needs reconnecting. Continue when the provider is ready.". The recovery card "Failed" has the same reason line, and the Needs you card and row match. Round 6 showed "Work stopped: Reconnect this account before continuing." here. | **pass** | 10, 11, 21–23 |
| Continue clears the notice (decision 1) | **Before:** Needs you "Run interrupted. Continue starts fresh." and the card "Interrupted" with the same reason. **After Continue:** one Needs you card "Run failed: …" and the Failed card, with no Interrupted card. **Store, read after quit:** chat v2 "Owner configured workspace and stopped prior invocations" [Blocker Interrupted, old Run], v3 "Start a fresh invocation" [], v4 "Record terminal run result" [Blocker "Run failed: …", new Run]. The old Run is Interrupted at fence 2, the new Run is Failed, and `derived/run-leases/` is empty. | **pass** | 20–22 |
| U11 | Replies opened from search shows the header chip "Archived", one Unarchive (the banner's "Archived. Unarchive to reply."), the focus ring on Unarchive and no caret in the composer. After the launcher closed, focus was back on Unarchive. Its card reads "Not answered yet · Unarchive to continue". ⌘L and typing into the composer were not tried. | **pass** (see finding 3) | 02, 05 |
| Killed, then archived (decision 2) | **Before Archive:** "May still be running · Reading the crane source", with Stop… and Continue (U1). **Archive:** "Chat archived. Work stopped and the queue is paused. Tasks keep their progress.", with the Needs you row "Information from cr… / Review changes to Cr…". **The chat:** the Review card "Review changes to Crane arrival — reported date +2 −0" with Show in note, Dismiss and Apply, then the Run's state card "Interrupted · Reading the crane source · Unarchive to continue" with Continue disabled. There is **no** "Run interrupted. Continue starts fresh." notice card. **Store:** chat v2 is archived and its attention is exactly one Review naming the proposal. The Run is Interrupted (fence 0→1, as seeded). The candidate is unretired and unapplied, and `derived/run-leases/` is empty. | **pass** for decision 2: the Review is the only attention item and the only actionable card. The Run's Interrupted state card also shows, because the recovery card reads Run state. | 12–15 |
| R1, open draft at 900×600 | Pilot proposal (light) with comments open, "ready" selected and a ⌘/ draft "Name the team". The draft input and its Cancel / Add comment are whole. The writing bar is whole inside the Note, from top to bottom: Rewrite ready · 2 comments · Dismiss · Review rewrite, the overall-feedback field, Retry saved rewrite and Rewrite with AI ⌘⇧R. The text box keeps about 80 pt. Cancel closed the draft. No notice row was present in this state. | **pass** | 16 |
| Regression: Note review | Review rewrite opened Proposal / Changes / Result. Apply gave v2 with "Applied · v2", Undo, Rewrite again and Mark 2 comments addressed, plus "Saved. 6 related items may need refresh. Open Changes to review them." Undo gave v3 with the original text, "Undone. 6 related items…" and Retry saved rewrite. | **pass** | 17, 18 |
| Regression: lists, chat states, Settings | Chats sidebar: Recent, All · 12, Archived · 1→2, Needs you (question, failure, recoverable edit "Vendor com… ⚠ / Recover edit", Review rows) and Recent chats. Notes list and search. Chat states seen: archived, killed, interrupted and failed. Settings → General lists the five themes. | **pass**. Projects and Tasks were not opened. | 13, 19 |

**Themes:** light Sammal (F-UI1, R1, review, Settings, Continue), dark Raba (U4, the Note's U10 surface, the killed chat), dark Virmalised (U10 after Continue).

**Not observed:** ⌘K ↓ ↵ onto a Message hit. F-UI1 paging (only four Messages) and an archived *note* hit. U11 through ⌘L. R2 and R3. Fix pass 1 findings 3, 4, 6, 8 and 9. The busiest Note with a recorded notice (the open question above). The owner's Stop… of a killed Run (Archive was used instead). The data-folder message. Projects, Tasks and restart recovery.

**New findings (native, not fixed):**

1. **Notes search, In chats at 170 pt.** When the role and the Archived chip share a row, the chat title shrinks to "R…". The full name appears only in the tooltip (01).
2. **⌘K snippet with Markdown.** For a reply containing a table, the In chats row shows raw syntax: "…Item | Owner | | --- | --- | --- | | 5 min | Numbers…" (seen while typing "pilot proposal").
3. **Archived chats show the global queue.** The strip "Paused · 1 waiting" with Resume queue, and the Run card's "1 waiting message stays paused until you resume the queue", appear in archived chats, even though the waiting message belongs to another chat. That is a second live control beside Unarchive. Whether it fits U11 is a lead or owner call (02, 15).
4. **A hit's snippet may not be clickable.** One click on the snippet text of a Notes-search chat hit didn't open the chat; a click on its title line did. Seen once, not isolated.

## Open items

**From the integration decisions** (owner message, item 3):

- *Continue after a crash:* resolved by decision 1 (the notice clears when the next Run starts). Desktop witness: `continue_clears_the_interrupted_notice_when_the_next_run_starts`.
- *An archived killed chat:* resolved by decision 2 (Review only, no new notice) and U11 (Unarchive is the one next step and takes focus). Continue stays unavailable until Unarchive.
- *Resolved project chats in search:* resolved by decision 3 after the caveat check. Chat-only notes always showed in their project's search (review-F misread this), and archived chats' Messages now show too, labelled.

**Carried from `work/daily-workflow-reliability`:**

- `brn project-delete` of a project whose chat has a Run fails with "run control changes must advance the fence" (pre-existing on main).
- The backup-cleanup sweep would remove a data folder the owner opened inside an abandoned stage.
- Slice H review notes: marker reads on an existing folder can still surface `storage: … (os error N)`, and "Choose a folder you own" also covers a macOS privacy denial.
- A desktop instruction whose Run never started is posted again on re-supply (slice E, code reading).
- G review notes: a later ordinary Run can withdraw an earlier Rewrite proposal, and a replay after the owner's Undo removes the restored Review item again.
- The technical failure cause is not persisted; only the owner-terms notice survives a restart.
- Findings 5–10, and U5 (verbatim imports are always Partial, an owner rule).
- No real-model quality judgment has been made.

**Carried from `work/ui-polish`:**

- Fix-pass items not yet re-observed natively: 5c (the busiest Note), INT-1 and INT-4 (a state the seed fixture lacks). U10 and this step's main changes were observed in the [native recheck](#native-observed-recheck-of-6db39420).
- The All chats day grouping stays in UTC while times are local.
- U3: Review rows can name their targets only while the candidate is still prepared.
- `resume_chat_queue` stops a queue-claimed Working Run through `interrupt_chat_run` without checking its lease. It can stop a Run that is live in another process, and this step left it unchanged.

**New in this step:**

- **U4 and `work/cli-agent-contract` round 3.** Discard edit… discards the open edit through `Store::discard` with the session that `recovery_buffers` lists for the note. Caller-bound sessions (round 3) would make another caller's edit undiscardable without its secret. That design should say how the desktop discards or adopts an edit it did not open.
- **The native app's startup failure is invisible outside a terminal.** An unusable data folder is reported in owner terms on stderr, but a Finder launch shows no window. A startup error window would be a product change; it was not made.
- **R1 extends the text floor.** The owner accepted the 48 px floor under a banner (16 Oct). The lead decided on 16 Oct to apply the same floor beside an open stacked draft, so the writing bar stays whole. The owner can overrule that decision. It was native-observed on `6db39420` with system fonts: the draft input and the whole writing bar stay inside the Note. The board's wording of the owner's rule, "Every other state keeps 120 px", now has two exceptions: the banner stack and an open draft.

**New in fix pass 1:**

- **Windows before merge (finding 7).** Dispatch `ci.yml` on the combined PR branch and wait for its Windows and Ubuntu native-ui lanes before merging, or the owner accepts the risk explicitly.
- **The busiest Note with a recorded notice.** In the measured state above, the writing bar ends 2.5 px below the Note at 900×600 (8.5 px on `649494d6`), with the text at its 48 px floor. The owner-accepted rule keeps the text at 48 px or more and never hides banners, failure reasons or the writing bar, so it doesn't say which row gives way when all of them are at their floor. The text-floor assertions were not lowered. It needs a native look (macOS fonts may fit) and, if it reproduces, an owner choice. The native recheck did not build this state.
- **The card's announced name** includes the progress line after the notice, because it reads every visible line. The Note's `writing-run-status` doesn't.
- **Archived note hits** get the chip, but the desktop's Notes search runs without a project scope, so the service currently lists no note hit from an archived chat there.

**Open questions:**

- Which row gives way in the busiest Note with a recorded notice (above). This step took the conservative option and changed nothing; it is on the board under the lead's section.
- Everything else here was settled by the board decisions and contract docs.
