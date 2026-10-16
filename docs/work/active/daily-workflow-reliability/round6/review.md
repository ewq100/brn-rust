# Round 6 gate review: owner decisions 1–3 on the reliability branch

Reviewed `b76cc5a7..9f10665a` on `work/daily-workflow-reliability`, four commits: `7ecc43aa` (decision 1, and decision 2 for Archive), `b5212618` (decision 2 applied to the owner's Stop), `747755b3` (decision 3) and `9f10665a` (docs). Starting a chat's next Run now clears that chat's Interrupted or Failed notice in the same checked "Start a fresh invocation" change. Archive and the owner's Stop of a lease-free Run add Review items and remove the free lease file, but add no new notice. BRN's own stops keep the notice. Owner search keeps archived and legacy Resolved chats' Messages under their normal scope, and both hit kinds carry a structured `archived` flag. Agent retrieval is untouched.

Watch for: the evidence and one test doc comment still call the Stop interpretation "pending", but the lead confirmed it on the board (confirmed). The caveat's proposed correction to `threads-target.md:136` and `chats-and-projects/spec.md:70` is outside the files the board's integration plan lists, so it could be missed at integration (confirmed). Desktop search always asks in the general context, so decision 3's chat-only-note half only shows through CLI `--thread`/`--project` searches (confirmed). None of these blocks this gate.

**Verdict**: APPROVED

## High-level view

Decision 1 is implemented where the owner asked, in `begin()`. Inside the change that saves the new Working Run, it removes every Blocker whose text is a host terminal notice (current or legacy) and whose `record` names a Run of this chat. That is how `finish()` already defines a host notice, so Questions, Reviews, Conflicts and independent Blockers stay, even when one carries notice text. A fenced orphan in the same chat no longer gets a notice that the same change would remove. `finish()` still adds exactly one notice when the new Run fails or is interrupted. Re-applying the start operation returns the recorded receipt.

Decision 2 has two halves in two commits. In `7ecc43aa`, Archive and ArchiveProgress no longer compute orphans. The core gates the notice on `ChatCommand::Pause`, so Archive keeps existing notices, adds Review items, writes `Interrupted` as Pause always has (review-D), and the receipt scan still removes the free lease. In `b5212618`, `stop_chat_run` becomes the owner's Stop with no orphan notice, and a new `interrupt_chat_run` keeps round 5 C's behavior for `pause_failed_worker` and the resume-stale stop. These two call-site swaps in `threads_chat.rs` are the only desktop production edits. `b5212618` reverts cleanly on HEAD, and no later commit references `interrupt_chat_run`. Orphan recovery, settings, project commands, imports and Steer are untouched and still add the notice.

Decision 3 makes one rule change. `message_scope` no longer hides an archived chat, while a chat whose project is in Trash stays `Hidden`. A new `chat_is_archived` (archived flag or legacy `Resolved`) drives `ChatMessageHit.archived` and `SearchHit.archived`, both serde-default `false` and read from the hit's own snapshot. The note label is computed only when `owner` is set. The agent path (`notes_request`, `NoteSearchResult`) gets no field and no Messages, and `context.rs` is unchanged. `search.sqlite` moves to v4, so a v3 index with archived chats tagged `hidden` is rebuilt. The caveat check found no archive-based exclusion of chat-only notes: project chat notes are project documents, and general chat notes stay private to their chat. With no contract reason to report, there was nothing to pause for before the code change.

Every new or changed test failed with its fix disabled and passed again after restore. The recorded gate counts line up with the test additions: 817 = 808 + 9 new workspace tests, and desktop 162 = 161 + 1.

<details>
<summary>Issues (6)</summary>

1. **Stale "pending" Stop interpretation** — `evidence.md:40`, `:559`, `:592` and the doc comment at `killed_run_candidate.rs:627` say the Stop interpretation is pending, but the board's lead ANSWER confirms it. Update them to "confirmed by the lead, 16 Oct 2026" when posting round 6.
2. **Canonical-target correction may be missed at integration** — the caveat proposes new text for `threads-target.md:136` and `chats-and-projects/spec.md:70`, but the board's integration plan only lists `threads-behavior.md`, `invariants.md` and `docs/status.md`. Post this correction on the board so the integration lead applies it.
3. **Inaccurate "Before" comment in the archived project chat test** — `search_messages.rs:499` says "Before: gone from project search once the chat was archived" over both the note and Message assertions. Only the Message was gone; the note always showed, and only its label is new. Reword the comment.
4. **Round 5 verification record not pointed forward** — `round5/verification.md:334` still says Stop and Archive of a lease-free Run add the notice, and that Continue shows it until the result. Add a one-line pointer to round 6, as `round5/native-recheck.md` got.
5. **Board and native re-check still pending** — the reliability section on the board is still at `b76cc5a7`, and `round6/native-recheck.md` is a procedure that hasn't been run. Post the round 6 result there, run the re-check under the Mac lock, and only then post READY FOR INTEGRATION.
6. **Decision 3's note half isn't visible in the desktop** — desktop search always uses the general context, so archived chats' chat-only notes and project chats' Messages never show there. Only CLI `--thread`/`--project` searches show them. Post this on the board so the lead (F-UI1) and the owner know what "chat-only notes appear" means in practice. No change is needed on this branch.

</details>

<details>
<summary>Details</summary>

### Notice clearing at Run start (`begin()`)

The clearing edits the Thread write `ended_run_attention` already queued (Review items for a fenced Run's candidate). If there isn't one, it pushes a single Thread write, and only when a notice actually goes. `records` includes archived rows, so a notice naming an older archived Run of the chat is still recognized. For project chats, the same change binds the chat as a `Current` input. `apply` checks self-inputs against the prestate (`store.rs:1671`), so the new Thread write doesn't make the start stale.

The filter is narrower than "any attention where `is_terminal_notice` is true". It also requires `kind == Blocker` and a `record` naming a Run of this chat. That matches `finish()`'s existing supersession rule and the decision's own "Questions and every other attention are kept". The brn-ai unit test pins it: a notice-texted Blocker naming a non-Run record survives the start.

```
Continue in chat C (orphan R1 Working, C.attention = [Review(op), Blocker(R1)])
  begin():  R1 -> Interrupted, fence+1; Review(op) kept; Blocker(R1) removed
            new Run R2 Working; lease file of R1 removed after apply
  finish(R2, Failed): replace_terminal_notice -> [Review(op), Blocker(R2, "Run failed: ...")]
```

### Archive and the owner's Stop of a lease-free Run

With the orphan set now empty for Archive and the owner's Stop, the lease file goes another way: `remove_ended_leases` scans the receipt for Working→ended Runs and calls `remove_free`, which never removes a held lease. Archive's Review items land in the Thread write that already carries `archived: true`.

The app call graph after `b5212618`:

```
desktop cancel() / Escape   -> stop_chat_run       -> notice_orphans=false  (owner's Stop)
pause_failed_worker         -> interrupt_chat_run  -> notice_orphans=true   ("worker-failure:")
resume-stale before Resume  -> interrupt_chat_run  -> notice_orphans=true   ("resume-stale:")
archive_chat(_progress)     -> chat_command        -> notice_orphans=false
recover_orphaned_runs / steering / projects / intake: unchanged, notice=true
```

`notice_orphans` is not part of the operation binding, so a key replays its recorded receipt whichever entry point is called. The desktop keys are distinct per path. `stop_chat_run` has no CLI caller.

### Owner search and archived chats

An archived chat's Messages appear exactly where the open chat's did, because `message_scopes_for` and the note path are unchanged. So "appear in owner search" has a scope limit. An archived private general chat's Messages and chat-only notes show only from that chat (`brn search --thread C`). A project chat's show only in its project (`--project P`). That follows the privacy and project-boundary contracts (`threads-behavior.md:46`, `chats-and-projects/spec.md:43`). The desktop's Notes search always asks in the general context (`threads_search.rs:91`, `request_page(.., None, ..)`). So in the desktop, decision 3 only adds archived general chats' Messages. Chat-only notes and project chats' Messages stay invisible there, archived or not, as on the base (confirmed). That's correct service behavior, but the owner may expect more from "its chat-only notes both appear", and F-UI1 decides which context the desktop asks in. A note whose `chat` is a Project id gets `archived: false`, because `chat_is_archived` matches only Thread records.

The caveat doc's history and table match the code. `document_allowed` admits project documents before `scope_allowed`, and `project_document_ids` ignores the chat's state, so there was no exclusion to explain. The doc classifies `threads-target.md:136` ("default retrieval excludes … archived material unless requested") and `chats-and-projects/spec.md:70` as currency rules. The owner decision explicitly overrides them for owner search, and the proposed correction keeps agent retrieval as written.

### Fail-before results (this review)

Each mutation was applied to the production code at HEAD, the named tests were run with `CARGO_TARGET_DIR=/tmp/brn-dwr-target --locked --offline`, and the file was restored with `git checkout --`.

- Decision 1 (`begin()` back to the orphan notice, and `clear_terminal_notices` made a no-op): `clearing_terminal_notices_keeps_every_other_item` failed at `run_attention.rs:410`. `completed_fresh_run_clears_only_superseded_host_terminal_blockers_after_reopen` failed at `threads.rs:4537`. `continue_in_the_killed_chat_starts_without_a_notice…` failed at `:885`, `continue_after_a_crash_clears_the_notice…` at `:953`, and `continue_after_the_settings_stop_clears_its_notice…` at `:563`.
- Archive (orphans recomputed for Archive/ArchiveProgress, core notice ungated): `archive_keeps_existing_attention_and_adds_no_notice_for_a_killed_run` failed at `:724`. The archive leg of `stop_archive_and_steer_of_a_killed_run…` failed at `:612`. Desktop `archiving_a_killed_chat_adds_its_review_but_no_notice` failed at `:860`.
- Owner's Stop (`stop_chat_run` passing `true`): `the_owners_stop_adds_no_notice_but_brns_own_stops_still_do` failed at `:649`. The stop leg of `stop_archive_and_steer…` failed at `:612`.
- BRN's own stops (`interrupt_chat_run` passing `false`): `the_owners_stop…` failed at `:649` (worker-failure leg), `stop_after_a_failure…` at `:372` and `resume_after_a_killed_queued_run…` at `:442`. With the desktop's resume-stale call site switched back to `stop_chat_run`, desktop `resume_after_a_killed_queued_intake_shows_one_notice` failed at `:806`, so the plumbing is guarded too.
- Decision 3, Message scope (`thread.archived` check restored): core `an_archived_or_resolved_chat_keeps_its_message_scope` failed at `:306`. App `archived_chats_stay_labelled…` failed at `:352`, `an_archived_chats_messages_stay_counted_and_labelled` at `:430`, `an_archived_project_chats_note…` at `:506` and `a_v3_index…` at `:700`. CLI `search_lists_a_saved_chat_message_with_its_chat` failed at `:97`, and desktop `the_notes_search_answer_carries_a_saved_chat_message` at `:952`.
- Legacy Resolved (`chat_is_archived` reading only the archived flag): core failed at `:315`, and app at `:369` and `:501` (legacy legs).
- Note label forced off: `an_archived_project_chats_note…` failed at `:501`.
- `SCHEMA_VERSION` back to 3: `a_v3_index_is_rebuilt_so_an_archived_chat_is_rescoped` failed at `:700`.

After restore, the targeted binaries pass at HEAD: app `killed_run_candidate` 15/1 ignored and `search_messages` 11, core `message_search_scope` 11, the two unit tests, CLI `search_messages` 1, and the three desktop tests. `git status` is clean at `9f10665a`. The caveat witnesses (`an_archived_project_chats_notes_stay_project_documents`, `…general_chats_notes_stay_private_to_it`, `a_trashed_chats_notes_stay_out…`) and the agent legs are preservation tests by design. They passed under every mutation.

### Boundaries and recorded evidence

The diff has no `threads.sqlite3` schema, `Cargo.toml`, `Cargo.lock`, `main.rs` or `docs/status.md` change. `context.rs` is untouched. The desktop production diff is the two `threads_chat.rs` call sites, and the other desktop changes are in `threads_intake_tests.rs`. Nothing on this branch touches `.worktrees/ui-polish` or `cli-agent-contract`. `ui-polish` has uncommitted desktop edits, but they belong to its own fix pass in progress (board, 16 Oct), not to this branch. The commit messages and the evidence's Round 6 table record what ran: fmt, the helper build, strict clippy (workspace and desktop), workspace 817/1 Poppler-only/6, desktop 162/1, e2e fixtures, the CLI journey, headless startup, `git diff --check` and links. The fail-before line numbers in the commit messages match the failures above. Spot checks here: `git diff --check` is clean, and the links check reports 0 failures in 127 files. The corrected contract sentences replace round 5's "keeps its Interrupted notice until the next Run in its chat records a result" in the proposed sentences, which quote the old wording as replaced. The new invariants sentence matches the path table and the code. `round6/native-recheck.md` is present, its expected states match the code, and `round5/native-recheck.md` points to it.

</details>

<details>
<summary>File map</summary>

- `crates/brn-ai/src/threads.rs` — `begin()` clears the chat's host terminal notices, and an orphan gets no notice at start. Unit test expectation updated.
- `crates/brn-threads-core/src/run_attention.rs` — new `clear_terminal_notices`, with a unit test.
- `crates/brn-threads-core/src/chat.rs` — orphan notice gated on `Pause`, so Archive never adds one.
- `crates/brn-threads-app/src/chat.rs` — orphans only for notice-giving Pause. `stop_chat_run` (owner) vs new `interrupt_chat_run` (BRN).
- `crates/brn-desktop/src/threads_chat.rs` — `pause_failed_worker` and the resume-stale stop call `interrupt_chat_run`.
- `crates/brn-threads-core/src/reader.rs`, `lib.rs` — `message_scope` keeps archived chats' scope. New `chat_is_archived`.
- `crates/brn-threads-app/src/search.rs`, `lib.rs` — `archived` on `MessageMatch`/`NoteMatch` and `ChatMessageHit`/`SearchHit`, owner-only for notes. `messages` renamed `owner`.
- `crates/brn-retrieval/src/search_index.rs` — `SCHEMA_VERSION` 4.
- Tests: `killed_run_candidate.rs`, `search_messages.rs` (app and CLI), `message_search_scope.rs`, `threads_intake_tests.rs`.
- Docs: `evidence.md` Round 6, `round6/decision3-caveat.md`, `round6/native-recheck.md`, a pointer in `round5/native-recheck.md`.

Full diff: `git -C .worktrees/daily-workflow-reliability diff b76cc5a7..9f10665a`.

</details>
