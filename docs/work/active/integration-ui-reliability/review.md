# Combined desktop for ui-polish and reliability: F-UI1, U4, U10, U11 and R1–R4

The commit makes the desktop consume what the reliability branch now records. Saved chat Messages appear as an "In chats" group in Notes search and in ⌘K. An edit already open elsewhere is matched on the typed `EditSessionOpen` variant and offers Compare and Discard. The chat's recorded terminal notice drives the recovery card and the Note's run status as well as the status line and Needs you. An archived chat lands focus on Unarchive. Tests are updated for leases, the owner's lease-free Stop and decision 1, and the contract docs carry decisions 1–3 once. Every product change is in `crates/brn-desktop`. The gate is recorded in full with exact counts, and the Poppler-only PDF test is reported as an environment gap, not a pass.

Watch for: ⌘K now puts asynchronously arriving full-text chat Message hits ahead of commands, so Enter's target changes when the search answers (confirmed, blocking). The Note's run status and the card's announced label append the agent's progress to the notice (likely, non-blocking). One live-worker test helper still holds no `RunLease` (confirmed, minor). Windows is not compiled; that is justified in the evidence, but the PR's CI runs native-ui on macOS only (confirmed).

**Verdict**: NEEDS_CHANGES

## High-level view

The integration stays on the desktop side of every shared seam: `useful_intake`, both `interrupt_chat_run` call sites and `Cargo.lock` are untouched. Nothing from this step is native-observed, and the evidence says so in its own column.

F-UI1 splits owner search into two lanes. A new query fills note hits and Messages from one answer; afterwards each lane pages with its own cursor and keeps only its own half of the response. Chat paging still waits on a notes page, and the "In chats" group can disappear while both lanes page (confirmed, minor). ⌘K reuses the hits but orders them before commands, and they arrive after the synchronous label matches, which is the blocking issue.

U4 matches `AppError::Core(Error::EditSessionOpen { note, base_version, .. })` and never reads `session`. The other edit is found by note in `recovery_buffers`, and Discard goes through the generation-checked `Store::discard`. The reused confirmation reads "Discard your unsaved edit?" / "Your changes since vN will be lost", as if the owner's typing here were being discarded; only the tooltip says that text is kept (likely, copy). The evidence records that caller-bound sessions (CLI round 3) would break this Discard.

U10 takes the Blocker attention whose `record` is the Run as the reason on all four surfaces. The agent's progress stays as a secondary line on the card and as a trailing "· progress" in the Note, so two surfaces show more than the notice.

U11 puts the archived composer banner in the focus table, so opening an archived chat focuses Unarchive. The test covers one Unarchive, an unavailable Continue and composer focus after Unarchive. `focus_composer` (the `FocusAsk` action and quote-into-chat) has no archived guard, so an explicit shortcut still focuses an archived chat's composer (confirmed path; whether that counts as "taking focus" is an interpretation).

R2–R4 change wording and row layout. R3's `refresh_name` calls `workspace.writing_request`, a SQLite read and a JSON parse, for every "May need refresh" pill on every render of Changes. The read is confirmed; whether it costs anything noticeable is only possible. R1 is a lead decision, listed below as pending.

Startup prints the core's owner-terms `DataFolder` text through `Display`, and a new test pins two problems against debug and OS wording. The native app still reports a startup failure only on stderr, with no window; the evidence lists it as an open product gap. Contract sentences appear once, decision 1 replaces the "keeps its Interrupted notice…" wording, and both currency sentences match the reliability branch's proposed text word for word.

<details>
<summary>Issues (9)</summary>

1. **⌘K Enter target moves when chat hits arrive** (blocking). Full-text Message hits land asynchronously before commands, and the selection is a bare index. Append Messages after commands, or keep the selection by identity when the list changes, and update the launcher tests and the evidence.
2. **Progress appended to the recorded notice**. The Note's `writing-run-status` reads "notice · progress" and the card announces "Failed · notice · progress · next". Render progress as a separate muted line in the Note as the card does, or state in the contract that progress may follow the notice.
3. **`hold_worker` stands for a live worker without a lease**. `threads_chat_tests.rs` `hold_worker` (three tests, one of which Stops) holds no `RunLease`, and the evidence's lease table doesn't mention it. Hold a lease there, or add it to the evidence's justification beside the workbench `run_fixture`.
4. **Chat paging is not fully independent of notes**. Chats' Previous/Next are disabled while a notes page loads, and `typing` (both lanes pending) hides the whole "In chats" group while both lanes page. Track "new query pending" explicitly instead of inferring it from both lanes.
5. **U4 Discard dialog wording**. "Discard your unsaved edit?" / "Your changes since vN will be lost" describes the other edit as if it were the owner's typing here. Give the open-edit path its own title and body, such as "Discard the edit open elsewhere?" … "What you typed here is kept."
6. **Focus-composer shortcut on an archived chat**. `focus_composer` still focuses an archived chat's composer. Route it to the archived banner as `focus_landing` does, or record that an explicit shortcut may focus it.
7. **Native-ui Windows compile happens only after merge**. On a pull request, CI's native-ui lane runs on macOS only, and this branch was never compiled for Windows. Run the manual all-targets workflow on the PR branch before merging, or accept the risk explicitly.
8. **Archived note hits read differently from archived Message hits**. Note hits from archived chats get a lowercase "archived chat" fact, while Message hits get an "Archived" chip. Use the same Archived label for both, as decision 3 says.
9. **Store reads in the Changes render path**. `refresh_name` calls `workspace.writing_request` (SQLite read and JSON parse) for every "May need refresh" pill on every render. Resolve the names when history refreshes.

</details>

<details>
<summary>Details</summary>

### ⌘K: asynchronous chat hits take the Enter slot

`lists::launcher` now chains label-matched chats, then notes, then `messages` (the owner search's Message hits, unfiltered), then label-matched commands, and truncates at 20. `LauncherSearch` clears `messages` on every keystroke and fills them when the worker answers. `launcher_index` resets to 0 on `InputEvent::Change` and isn't adjusted when the answer lands, and `run_launcher` acts on `items[launcher_index]`.

```
keystroke "raba"  -> items: [Command Theme(Raba)]                index 0 = theme
answer arrives    -> items: [Message "…raba…", Command Theme]    index 0 = chat
```

This is confirmed by code reading, and it has two consequences. Enter right after typing is nondeterministic: it runs the command if the answer hasn't arrived and opens a chat if it has. Once the answer has landed, any saved chat text containing the typed word beats the command. Label matches already beat commands, but those are synchronous and match titles only; full-text Message matches are far broader. Theme switching through ⌘K is an owner decision in the ui-polish plan (open question 4). The theme names (Raba, Sammal, Läänemeri, Paber) are everyday Estonian words, the fixtures' language, so collisions are likely in real use. The list also jumps on every keystroke, as Messages vanish and reappear above the commands.

The existing test calls `launcher("raba", …)` for `Command::Theme(Raba)`, but with `&[]` messages, so the collision is never exercised. The new test `launcher_lists_matching_chat_messages_before_commands` pins the problematic order. Appending Messages after commands keeps every earlier index stable when the answer arrives, which removes the race.

### Two-lane owner search in `threads_search.rs`

A single cursor carries one offset that the service applies to both kinds, so independent paging works by issuing a separate request per lane and keeping only that lane's half. `note_hits_and_chat_messages_page_independently` (14 notes and 14 Messages, each lane paged alone, Previous on chats, a new query resetting both) covers that.

The lane state has two asymmetries (confirmed). `search_paging(Lanes::Chats)` disables on `results.pending()`, which is true while either lane loads, so a notes page blocks chat paging but not the reverse. `chat_results` hides the whole group when `notes.pending() && messages.pending()`, which is meant to detect a new query. It is also true when the owner pages notes while a chat page is loading, so the group and its paging row disappear until one lane answers. Both are cosmetic, but the lanes aren't quite independent. An explicit "new query pending" flag would remove both. Separately, note hits from archived chats read "archived chat" in their facts line, while Message hits carry the Archived chip.

### U10: the notice leads, progress follows

`Recovery::with_notice` moves the old reason (the Run's progress) to `detail` when it differs, and `RunOutcome::Recorded` does the same for the Note. Progress comes only from `retain_progress`, the agent's own summary, so it is never a second failure wording, which was U10's complaint. Two surfaces still show more than the recorded notice, though. `writing-run-status` is one string, "Run failed: … · Synthetic agent progress", in the error tone, and the card is announced as "Failed · {notice} · {progress} · {next}". The status line and Needs you show the notice alone. The Note just doesn't set the agent's text apart visually the way the card does (likely; a design call).

When an owner message postdates the failed Run, the card switches to "Not answered yet" and drops the notice while Needs you keeps it. This is the existing `owner_after` rule, not new in this diff.

### Lease fidelity in the updated tests

`threads_chat_tests.rs` `hold_worker` is documented as "A live local worker for the chat's run, as a running reply has", but holds no lease. `queue_notices_follow_the_queue_state_and_paused_wins_after_stop` Stops through it, so the service takes the lease-free path while the test describes a live worker. Under decision 2 the owner's Stop has the same observable outcome either way. That is the evidence's argument for the workbench `run_fixture`, but this helper isn't covered by that justification (confirmed, minor). The steering, dialogs and project-delete seeds do hold a lease.

### Platform coverage

The commit's only platform-specific code is the `#[cfg(unix)]` data-folder test. The merged reliability code uses std `File::try_lock` and a `cfg(unix)` `O_NOFOLLOW`, and `libc` is available on every target. `brn-threads-core/src/stage.rs`'s `cfg(not(unix))` branch is compiled by PR CI's headless Windows lane. The `threads_lists.rs` fallback (ui-polish FIX-3) and every native-ui module are not: `ci.yml` runs native-ui only on `macos-15` for pull requests, and on Windows only for main or manual runs. Windows was never compiled for this branch, so a native-ui Windows break would first show after merge.

### Requirement coverage

1. Test expectations: done.
   - Settings stop → Interrupted (reliability intake test; no desktop test expected Cancelled after a settings change).
   - Orphans on Resume, Steer and Continue (desktop tests) and Dispatch (service `run_recovery`).
   - Leases for live seeds, except `hold_worker` (issue 3).
   - The owner's lease-free Stop, decision 1, and Review on Stop/Archive/Resume.
   - Legacy text only in `LEGACY_NOTICES`; 10 tools and `brn-threads-guidance-12` (main has 11); `SCHEMA_VERSION = 4` with v2/v3 rebuild tests; canonical temp roots.
   - No assertion line was removed; the one changed expectation (R4 row text) gains layout checks.
2. F-UI1: done, with the ⌘K ordering defect (issue 1) and the paging asymmetry (issue 4). The group uses `section_header`, `ListRow` with role metadata and an Archived `status_chip`, `loading` and a one-sentence `empty_state` body.
3. U4: done. Variant match, Compare and Discard, only `note` and `base_version`.
4. Data folder: done for startup. The missing native error window is recorded as an open item.
5. U10: done, with the progress suffix noted (issue 2).
6. U11: done, with the shortcut path noted (issue 6).
7. Contract and docs: done.
   - Decision 1 replaces the proposed sentence, decision 2 covers Archive and the owner's Stop, and a decision 3 Search paragraph is added.
   - The currency sentences at `threads-target.md:136` and `chats-and-projects/spec.md:70` match reliability's text.
   - `status.md` gains one entry and goes from nine to ten tools; no sentence is duplicated.
   - The evidence keeps the five statuses separate.
8. Gate: present with exact counts.
   - workspace: 827 passed / 1 failed / 6 ignored, the failure being the Poppler-only test reported as an environment gap;
   - desktop shell: 292 passed / 1 ignored; native-ui: 80; native-ui with native-retrieval: 80; default features: 30;
   - workspace and per-lane clippy, fmt, e2e fixtures, the CLI journey from the same build, headless startup, links and `Cargo.lock`: all clean.
   - Windows is not compiled; that is justified (issue 7).

### Pending owner acceptance, not defects

- The 48 px text floor under a Note banner in the busiest 900×600 state is listed here as pending owner acceptance. The board records "i accept your offer" (16 Oct, relayed by the lead), and `plan.md` and `ui-polish/evidence.md` now say owner-accepted, but this review did not see that acceptance first-hand.
- R1 extends the same 48 px floor to an open stacked comment draft. It is a lead decision the owner can overrule, and it isn't native-observed.

### Carried, not from this diff

`resume_chat_queue` still stops a queue-claimed Working Run through `interrupt_chat_run` without a lease check, so it can stop a Run that is live in another process. Also carried:

- the `project-delete` fence error;
- the backup-sweep edge;
- the slice H and G review notes;
- native re-observation, still owed for 5c, INT-1, INT-4, U10 and everything in this step.

The evidence lists all of them.

</details>

<details>
<summary>File map</summary>

Reviewed `649494d6` on `work/integration-ui-reliability` (integration commit on `6f51d22c`), from the diff, commit message, `evidence.md`, `ui-polish/plan.md` and the coordination board. No suite was rerun and no spot-check was needed.

- `crates/brn-desktop/src/threads_search.rs`: two-lane search state, "In chats" group, launcher search, shared `highlighted`/`role_label`.
- `crates/brn-desktop/src/threads_lists.rs`: `LaunchItem::Message`, launcher ordering and tests.
- `crates/brn-desktop/src/threads_shell.rs`: launcher Message rows and loading state; R2 status bar; U11 focus landing.
- `crates/brn-desktop/src/threads_ui.rs`: new state fields, U4 error arm and `discard_session`, launcher wiring.
- `crates/brn-desktop/src/threads_chat.rs`: one-shot transcript reveal.
- `crates/brn-desktop/src/threads_writing.rs`: recovery card notice/detail, archived banner focus handle, U4 banner/Compare/Discard, R1 floor.
- `crates/brn-desktop/src/threads_state.rs`: `Recovery::detail` and `with_notice`.
- `crates/brn-desktop/src/threads_workbench.rs`: `RunOutcome::Recorded`.
- `crates/brn-desktop/src/threads_context_actions.rs`: R2 Edit note reason.
- `crates/brn-desktop/src/threads_tiles.rs`: R3 refresh names, R4 kept-edit rows.
- `crates/brn-desktop/src/main.rs`: data-folder startup test.
- `crates/brn-desktop/src/*_tests.rs`: leases, decision 1/2 tests, F-UI1, U4, U10, U11, R1–R4 tests.
- `docs/architecture/{threads-behavior,invariants,threads-target}.md`, `docs/work/active/chats-and-projects/spec.md`, `docs/status.md`: contract text with decisions 1–3.
- `docs/work/active/ui-polish/{plan,evidence}.md`, `docs/work/active/integration-ui-reliability/evidence.md`: text-floor status, R1 rule, integration record.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/integration-ui-reliability show 649494d6`.

</details>
