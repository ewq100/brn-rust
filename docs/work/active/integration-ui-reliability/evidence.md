# UI polish and daily-workflow reliability: integration evidence

**Date:** 16 October 2026. **Branch:** `work/integration-ui-reliability`, local only with no upstream, worktree `.worktrees/integration-ui-reliability`.

**Built from:** `work/ui-polish` `354c1940`, then a `--no-ff` merge of `work/daily-workflow-reliability` `192cd765` (`067470d5`), then a `--no-ff` merge of main `a4ad9677` (`6f51d22c`). This step adds one commit on `6f51d22c`. `work/ui-polish` `81129f07` (docs and shots after `354c1940`) is not merged here; the record step merges it.

**Sources:** [UI polish evidence](../ui-polish/evidence.md), [reliability evidence](../daily-workflow-reliability/evidence.md), the coordination board's owner decisions of 16 October 2026, and the UI branch's native recheck of `354c1940` (R1–R4, in that branch's `verification.md` at `81129f07`).

Each status is reported on its own: implemented or verified does not mean owner-accepted or merged.

| Item | Implemented | Automated-verified | Native-observed | Owner-accepted | Merged |
|---|---|---|---|---|---|
| UI polish S1–S7 and both fix passes | yes (`work/ui-polish`) | yes, on this tree | on its own builds (`ed8daa82`, recheck `354c1940`), not on this build | text-floor rule yes (16 Oct); the rest no | no |
| Reliability rounds 1–6 | yes (`work/daily-workflow-reliability`) | yes, on this tree | on its own builds (rounds 4 and 6), not on this build | decisions 1–3 yes (16 Oct); the build no | no |
| Test expectations for the reliability service (§1) | yes | yes | n/a | n/a | no |
| F-UI1: "In chats" in Notes search and ⌘K (§2) | yes | yes | no | no | no |
| U4: an edit already open elsewhere (§3) | yes | yes | no | no | no |
| Data-folder startup message (§4) | yes (test of the existing path) | yes | no | no | no |
| U10: one recorded notice on four surfaces (§5) | yes | yes | no | no | no |
| U11: an archived chat focuses Unarchive (§6) | yes | yes | no | no | no |
| R1–R4 from the UI recheck (§7) | yes | yes | no | no (R1 is a lead decision of 16 Oct that the owner can overrule) | no |
| Contract and docs (§8) | yes | link check | n/a | decisions 1–3 yes; wording no | no |

Nothing was pushed, merged, published or released. No Mac window was opened in this step, and `MAC-IN-USE` was not taken.

## What changed

All product changes are in `crates/brn-desktop`. No shared seam (`brn-ai/src/threads.rs`, `brn-threads-app/src/{chat,intake}.rs`, `brn-threads-core`, the search module) was edited. `useful_intake`'s logic and both `interrupt_chat_run` call sites (`pause_failed_worker`, `resume_chat_queue`) are unchanged. No dependency, `Cargo.lock` or schema change.

### 1. Tests with new expectations

| Expectation | Test | Note |
|---|---|---|
| A settings change records Interrupted, not Cancelled | `intake_tests::a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice` (reliability) | Runs on this tree. No UI test expected Cancelled after a settings change. |
| A seeded Working Run with no lease counts as crashed | Resume: `intake_tests::resume_after_a_kill_…`, `…_killed_queued_intake_shows_one_notice` (reliability). Steer: new `steering_tests::steer_of_a_reply_no_process_holds_records_the_interrupted_notice`. Continue: new `intake_tests::continue_in_a_chat_whose_reply_no_process_holds_interrupts_it`. Dispatch: service `run_recovery` tests. | |
| A seeded Run that stands for a live worker holds a `RunLease` | `steering_tests` fixture (Working state), `dialogs_tests::working_run`, `projects_tests::native_project_delete_…` | `stop_and_send_click_is_durable_…` now also asserts that Steer of live work adds no notice and leaves the lease file. The project delete test now asserts the chat's Interrupted notice, which project commands always record. The workbench `run_fixture` holds none: its tests never Steer, Resume or Dispatch, and the owner's Stop adds no notice either way. |
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
- **Paging:** notes and Messages are separate lanes. A new query fills both from one answer. Each lane's Previous/Next requests its own cursor (`cursor` or `message_cursor`) and updates only its own page. Paging waits while a new query's answer is pending.
- **States:** "Searching…" while a query runs, "Searching chats…" while the chat lane pages, a "N messages · a–b" status line. The group stays hidden while the page has no Messages. When notes have no text hits but Messages exist, the notes empty state says "Matching chat messages are listed below."
- **⌘K:** the launcher runs its own first-page owner search (up to 5 Messages) as the query changes and lists them under "In chats", after Notes and before Commands, with the snippet highlighted, the chat and role as muted metadata and the Archived chip. ↵ opens the chat at the Message. While the search runs and nothing else matches, it shows "Searching chats…" instead of "No matches". Group IDs replace spaces with `-` (`launcher-group-in-chats`).
- Tests: `tiled_tests::notes_search_lists_saved_chat_messages_and_opens_their_chat` (the round 5 section F repro, "berth", archived label after Archive, transcript scrolled to the hit), `tiled_tests::note_hits_and_chat_messages_page_independently` (14 notes and 14 Messages, each lane paged alone), `omakase_tests::launcher_lists_saved_chat_messages_and_opens_their_chat`, `lists::tests::launcher_lists_matching_chat_messages_before_commands`.

### 3. U4: an edit already open elsewhere

- `note_changed` matches `AppError::Core(Error::EditSessionOpen { note, base_version, .. })`, never the text, and uses only `note` and `base_version`.
- The note becomes read-only like any kept edit. The notice reads "An edit of this note is already open from vN. What you typed here isn't saved."
- Its banner (`note-edit-open-banner`) replaces the generic kept-edit banner: "An edit of this note is already open from vN and is kept. What you typed here isn't saved. Compare it with the saved note, recover it, or discard it." The actions are Recover edit (the existing primary), Compare and Discard edit….
- **Compare** opens a read-only dialog with "Open edit · from vN" beside "Saved · vM".
- **Discard edit…** reuses the existing `discard-note` consequence dialog ("Your changes since vN will be lost. This can't be undone."). It discards the open edit that `recovery_buffers` lists for the note, then keeps what was typed here as this view's new unsaved edit.
- Test: `writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard` (a second `Workspace` opens the edit, as another window or the CLI would).

### 4. Data-folder error

The desktop's startup path (`main.rs` `launch`, headless and native) already prints the core's owner-terms `DataFolder` text through `Display`. AppError `Core` is `#[error(transparent)]`, and no desktop path formats errors with `{:?}`. New test `tests::an_unusable_data_folder_reads_in_owner_terms_at_startup`: a symlinked parent gives "…part of its path is a symbolic link. Choose the folder's real location." (`SymlinkRefused`), a file in the path gives "…is a file rather than a folder.", with no "os error", "DataFolder", "Io(" or "unsafe data directory" text, and nothing created. The gate's `--data-dir /data` run printed "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." (exit 1). In the native app a startup failure is still reported only on stderr, before any window opens, as before this step; see Open items.

### 5. U10: one recorded notice

For a Failed or Interrupted Run, the chat's Blocker attention whose `record` is that Run is the source on every surface:

- the status line (UI branch, `terminal_notice`, unchanged)
- Needs you (attention reasons, unchanged)
- the chat's recovery card reason line (`Recovery::with_notice`)
- the Note's `writing-run-status` (`RunOutcome::Recorded`)

The Run's progress, the agent's own summary, is a secondary line (`{card}-detail`, muted) or a trailing "· progress" in the Note, only when it differs from the notice. Without a recorded notice (the owner's Stop records none) each surface keeps its previous text. Tests: `state::tests::a_recorded_notice_is_the_recovery_reason_and_progress_is_detail`, `chat_tests::a_failed_reply_reads_as_its_recorded_notice_on_the_card_and_in_needs_you`, `polish_tests::a_failed_rewrite_reads_as_the_chats_recorded_notice`, `workbench::tests::a_recorded_notice_leads_and_progress_is_secondary`, and the existing `steering_tests::a_failed_or_interrupted_finish_shows_the_recorded_terminal_text`.

### 6. U11: archived chat

On this tree the archived chat already had one Unarchive (the composer banner's `reopen-thread`) and an unavailable Continue. The composer still took focus when the chat opened, and that is fixed: the focus landing for an archived chat is its banner, and after the next frame focus moves to Unarchive. Opening the chat again after Unarchive focuses the composer. Test: `chat_tests::archive_states_its_consequence_and_an_archived_chat_has_one_unarchive` (new focus assertions).

### 7. R1–R4 from the UI branch's recheck of `354c1940`

| Item | Change | Test |
|---|---|---|
| R1: the writing bar clipped with a stacked comment draft at 900×600 | **Lead decision, 16 Oct 2026, extending the owner-accepted banner rule to the drafting state; the owner can overrule** (option A of three; the others were to let the draft input scroll out of view, reversing fix 3b, or to leave the bar clipped). While a draft is open, the draft keeps its whole input and the text gives way to the 48 px floor, as under a banner. The text still asks for 120 px and gives way last. The whole bar, including a filled Retry saved rewrite, stays in the Note, with or without a notice row. Measured at 900×600: the text gets 48 px without a notice row and 68.5 px in the run with one. plan.md's height-share rule records it and names both exceptions to the 120 px floor (the banner stack and an open draft). | `review_tests::an_open_stacked_draft_keeps_the_whole_writing_bar_at_900x600`: the bar is whole, the draft input is whole (the same `assert_draft_in` that fix 3b's test uses), and `note-body` is at least 48 px. Before the fix the bar ended at 579.5 px against the Note's 508 px. Fix 3b's whole-input assertion in `stacked_comments_keep_every_comment_capability` is unchanged. |
| R2: Edit note said "BRN is working" while the status bar said "Idle" | The gating is unchanged: a saved queue item still blocks Edit note. The reason now reads "a queued message is paused · Resume or clear the queue" (or "a queued message is about to run"), and "BRN is working" appears only with a worker. With nothing running and saved queue work, the status bar shows "Queue paused · N waiting" (or "Queue · N waiting") instead of "Idle". | `context_actions_tests::a_paused_queue_blocks_edit_note_with_a_reason_the_status_bar_shares` |
| R3: raw JSON in Changes' "May need refresh" pills | Names in owner terms: a rewrite request reads "Rewrite request · {note}", another chat Message reads "Chat message", and other records keep their title, whitespace collapsed. The pill is one line that ellipsizes inside the panel, and the tooltip and accessible name carry the full name. | `polish_tests::changes_name_a_rewrite_request_that_may_need_refresh_in_owner_terms` |
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

## Verification

Run on the final tree of this commit, from the worktree with `CARGO_TARGET_DIR=<worktree>/target`, `--locked --offline`, Rust 1.98.1, macOS arm64. `TMPDIR` and all data were fresh synthetic folders under `/private/tmp/brn-integ-gate/`. The desktop and CLI binaries in the last rows come from the same build. No provider call, credential or owner data was used.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | ok |
| `cargo build --workspace --bins` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast` | **827 passed, 1 failed, 6 ignored.** The failure is `brn-threads-intake` `qualification::prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` ("Pdf preparation failed: Unavailable"). Poppler is not installed here, so this is a known environment gap, **not a pass**. |
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` (helper and desktop build, clippy `-D warnings` on all targets, `native-ui,native-test-support` tests) | **292 passed, 0 failed, 1 ignored**; the ignored test needs a CLI intake's shared data (reliability's cross-surface check) |
| `cargo clippy -p brn-desktop --features native-ui --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui` | **80 passed** |
| `cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui,native-retrieval` | **80 passed** |
| `cargo clippy -p brn-desktop --all-targets -- -D warnings` (default features) | clean |
| `cargo test -p brn-desktop` (default features) | **30 passed** |
| `cargo build -p brn -p brn-desktop --bins` (the binaries below) | ok |
| `TMPDIR=… scripts/verify-end-to-end.sh --fixtures-only` | passed (provider and intake retirement; offline Save/recovery/search/export/backup/startup/refusal journeys) |
| `cli-journey.py target/debug/brn /private/tmp/brn-integ-gate/data/cli-journey` | exit 0. 43 steps: 40 succeeded, and 3 were refused as designed (two `begin-edit` with `edit_session_open`, one occupied backup destination), as in reliability round 6 |
| `brn --help`, `brn-desktop --help` | ok |
| `brn-desktop --data-dir <fresh> --headless-check startup` | "Threads ready: 2 records" |
| `brn-desktop --data-dir /data --headless-check startup` | refused, exit 1: "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." |
| `python3 scripts/check-markdown-links.py` | 141 files, 572 local links, 0 failures (run again after this record was written) |
| `git diff --check` | clean |
| `git diff --exit-code a4ad9677 -- Cargo.lock` | clean |

**Fail-before checks.** Each fix was disabled in place, its test was rerun, and the source was restored:

- U11: with the archived-chat focus guard off, `archive_states_its_consequence_…` failed (the composer took focus).
- F-UI1: with the transcript scroll off, `notes_search_lists_saved_chat_messages_…` failed (the hit's Message was not in view).
- R1: with the draft-open floor change reverted, `an_open_stacked_draft_keeps_the_whole_writing_bar_at_900x600` failed (the bar ended at 579.5 px, below the Note's 508 px).

The other new tests assert text or state that did not exist before this step (the "In chats" rows, the U4 banner and dialogs, the launcher's Message items, the U10 card detail, the R2–R4 wording). Their pre-change values are named in the test comments.

**Not verified:**

- Windows was not compiled; no Windows target is installed. This step adds no platform-specific call, and its one Unix-only test is `#[cfg(unix)]`.
- No native observation of this build. In particular, F-UI1, U4, U10, U11 and R1–R4 are automated-verified only, in the GPUI test text system, which is wider than macOS fonts.
- No two-process lease check beyond the reliability branch's in-process one.

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

- Fix-pass items not yet re-observed natively: 5c (the busiest Note), INT-1, INT-4 and U10 (they need a provider, or a state the seed fixture lacks), plus every change in this step.
- The All chats day grouping stays in UTC while times are local.
- U3: Review rows can name their targets only while the candidate is still prepared.
- `resume_chat_queue` stops a queue-claimed Working Run through `interrupt_chat_run` without checking its lease. It can stop a Run that is live in another process, and this step left it unchanged.

**New in this step:**

- **U4 and `work/cli-agent-contract` round 3.** Discard edit… discards the open edit through `Store::discard` with the session that `recovery_buffers` lists for the note. Caller-bound sessions (round 3) would make another caller's edit undiscardable without its secret. That design should say how the desktop discards or adopts an edit it did not open.
- **The native app's startup failure is invisible outside a terminal.** An unusable data folder is reported in owner terms on stderr, but a Finder launch shows no window. A startup error window would be a product change; it was not made.
- **R1 extends the text floor.** The owner accepted the 48 px floor under a banner (16 Oct). The lead decided on 16 Oct to apply the same floor beside an open stacked draft, so the writing bar stays whole. The owner can overrule that decision, and it is not native-observed. The board's wording of the owner's rule, "Every other state keeps 120 px", now has two exceptions: the banner stack and an open draft.

**Open questions:** none needing an owner choice in this step. The board decisions and contract docs settled every behavior here.
