# Round 3: U6 notice wording, U4 typed edit-session error, U2 desktop intake reuse

**Date:** 15 October 2026. **Base:** `c408b1d4` on `work/daily-workflow-reliability` (rebased onto `771116f0`). Local only: nothing pushed, merged or released. U3 was a read-only check; its result is in [u3-finding.md](u3-finding.md).

**Environment:** worktree `.worktrees/daily-workflow-reliability`, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, rustc 1.98.1, `--locked --offline`. Synthetic data under `/private/tmp/brn-dwr-data/` and test temp directories only. No provider calls: every run had no credentials or a missing auth file and failed as `ReconnectNeeded` before any network. No Mac UI was driven.

## Changes

### U6: failure notice wording (`crates/brn-ai/src/threads.rs`)

`failed_attention` maps each `AiErrorKind` to one cause sentence in owner terms. "Continue when the provider is ready." is added only for provider failures, and another recovery sentence only where it is true. `AiError`'s Display is unchanged. The desktop's live run outcome line (`threads_ui.rs`, `Failed(e) => Err(format!("{e}…"))`) and the CLI's `failure` JSON still carry the technical cause, including `Retry after N seconds`, so no cause is lost from the run's progress. Nothing was written to `Run.progress`, because that field feeds the next model run.

| Kind | Saved notice | Chars |
|---|---|---|
| `ReconnectNeeded` | Run failed: The provider account needs reconnecting. Continue when the provider is ready. | 89 |
| `CodeExpired` | Run failed: The sign-in code expired. Continue when the provider is ready. | 74 |
| `RateLimited` (any retry delay) | Run failed: The account usage limit was reached. Continue when the provider is ready. | 85 |
| `Network` | Run failed: BRN couldn't reach the provider. Continue when the provider is ready. | 81 |
| `Other` with a retryable provider detail (5xx, overload, dropped stream) | Run failed: The provider is unavailable right now. Continue when the provider is ready. | 87 |
| `ModelRefused` | Run failed: The provider doesn't offer this model. Choose another model, then Continue. | 87 |
| `ToolLimitReached` | Run failed: The tool-call limit was reached. Continue starts fresh. | 67 |
| `TimeLimitReached` | Run failed: The work time limit was reached. Continue starts fresh. | 67 |
| `InvalidToolUse` | Run failed: The model asked for a step BRN doesn't offer. Continue starts fresh. | 80 |
| `QuoteNotFound`, `QuoteAmbiguous`, `QuoteOccurrenceInvalid` | Run failed: A quotation didn't match the saved note. | 52 |
| `ToolRejected` | Run failed: BRN refused one of the run's steps. | 47 |
| `UnsafeCredentials` | Run failed: The saved sign-in isn't stored safely. | 50 |
| `IndexStale` | Run failed: The note index needs refreshing. | 44 |
| `Storage` | Run failed: BRN couldn't save its work. | 39 |
| `Other` | Run failed: The run didn't finish. | 34 |
| Interrupted (`INTERRUPTED_ATTENTION`) | Run interrupted. Continue starts fresh. | 39 |

"Continue starts fresh." is used where Continue is the natural next step and does start a new run from the saved conversation and current notes (desktop `continue_run`, CLI `ask`). It is omitted for storage, index, credential, rejection and unclassified failures, where Continuing again is not a known fix.

`is_terminal_notice` still recognises `"Run interrupted; Continue starts fresh"`, `"Run failed; check provider or continue from retained progress"` and every `"Run failed: …"` text, so a saved chat's old notice is replaced, not duplicated.

**Before/after (saved Blocker reason):**

| Case | Before (round 1) | After |
|---|---|---|
| No credentials | Run failed: Reconnect this account before continuing. Continue when the provider is ready. | Run failed: The provider account needs reconnecting. Continue when the provider is ready. |
| Tool limit | Run failed: The tool round limit was reached. Continue when the provider is ready. | Run failed: The tool-call limit was reached. Continue starts fresh. |
| Storage | Run failed: Local storage operation failed. Continue when the provider is ready. | Run failed: BRN couldn't save its work. |
| Rate limit, 1 h delay | Run failed: Account usage limit reached. Retry after 3600 seconds. Continue when the provider is ready. (103 chars; the delay goes stale after restart) | Run failed: The account usage limit was reached. Continue when the provider is ready. |
| Stopped at startup | Run interrupted; Continue starts fresh | Run interrupted. Continue starts fresh. |

### U4: typed error for an open edit session

- `brn_threads_core::Error::EditSessionOpen { note, session, base_version }`, with Display "this note already has an open unsaved edit". It reaches the app as `AppError::Core(Error::EditSessionOpen { .. })`.
- `Store::begin_edit` (`crates/brn-threads-core/src/edit.rs`) looks up the open session for the note **inside the same IMMEDIATE transaction, before the insert**, and returns the typed error. Dropping the transaction writes nothing. The open session is never discarded, overwritten or touched. Save, discard, the dirty guard (`store.rs` open-session check), version checks, replay and Undo are unchanged, and the unique index stays as a backstop.
- CLI `begin-edit` (`crates/brn/src/main.rs`) exits non-zero and prints to stderr:
  `{"error":{"kind":"edit_session_open","message":"This note already has an open unsaved edit. Save or discard that session first; the recovery command lists it.","note":"…","session":"…","base_version":1}}`. Other `begin-edit` errors are unchanged text.
- Desktop: no code change. `note_changed` already formats any error, so the notice becomes "Could not preserve this edit: this note already has an open unsaved edit. Keep this window open or copy your text." instead of the raw SQLite text.

### U2: desktop useful intake reuses its chat (`crates/brn-desktop/src/threads_ui.rs`)

`useful_intake` now calls `Workspace::source_intake_chat(project, &converted.source_reference, "Information from {title}")` instead of `new_thread`. The project is that of the chat open in the chat tile (`project_for_chat(&self.records, chat)`), or `None` for general. Nothing else in `useful_intake` changed: no layout, styling, control or wording. The only other desktop edits are the test module declaration and the new `crates/brn-desktop/src/threads_intake_tests.rs`.

## Fail-before / pass-after

| Test | Before | After |
|---|---|---|
| `brn-desktop … intake_tests::supplying_the_same_source_twice_in_general_reuses_its_chat` | **FAILED**: `left: "1689116e-…:0"`, `right: "164f467e-…:0"` (two chats) | ok |
| `… intake_tests::supplying_from_inside_a_project_keeps_the_source_in_a_project_chat` | **FAILED**: `left: None`, `right: Some("13d32760-…:0")` (the intake chat was general) | ok |
| `… intake_tests::a_chat_whose_run_is_still_live_is_not_reused` | ok (`new_thread` never reused anything) | ok: a chat with a held lease is skipped and its Working Run keeps fence 1 |
| `brn --test threads a_second_begin_edit_reports_the_open_session_as_structured_json` (U4 sources stashed to HEAD) | **FAILED**: `SQLite: UNIQUE constraint failed: edit_sessions.note` | ok |
| `brn-threads-core --test core a_second_edit_session_names_the_open_one_and_leaves_it_untouched` | does not compile before (new variant) | ok: ids and base 1 for both a fresh and a stale-base session; the session and `recovery_buffers` are unchanged |
| `brn-threads-app --test product an_open_edit_session_is_a_typed_app_error_naming_note_and_session` | does not compile before | ok |
| `brn-ai threads::tests::failure_notices_name_the_cause_in_owner_terms_and_only_true_recovery` | does not compile before (new constants) | ok: every kind plus a 1 h rate limit and a retryable 503. Checks the `Run failed: ` prefix, ≤ 100 chars, the provider sentence only on provider kinds, and no `SQLite`, `/`, `\`, `tool round`, `Retry after`, `503` or `overloaded` |
| `… threads::tests::a_notice_saved_by_an_earlier_build_is_replaced_not_duplicated` | n/a (wording) | ok for all four earlier saved texts: one Blocker with the new text on the newest Run |
| `… threads::tests::terminal_notices_are_recognised_including_every_earlier_saved_text` | replaces the round-1 recognition test | ok |

Updated assertions: `repeated_failures_keep_one_notice_naming_the_cause_until_an_interrupt_replaces_it` (brn-ai) and `useful_intake_for_a_project_stays_in_one_project_chat_across_retries` (CLI) now expect the new ReconnectNeeded notice exactly.

## Desktop plus CLI from one build, shared synthetic data

Both binaries came from the same tree and target directory (`/tmp/brn-dwr-target/debug/brn` and the `brn-desktop` test binary). Data: `/private/tmp/brn-dwr-data/u2-shared/`, Codex configured with a missing auth file.

1. CLI `intake --source …/mari-email.txt --intent useful` gave `terminal: Failed:ReconnectNeeded`, `thread: b3fb033a-…:0`, `reused_chat: false`, one chat.
2. Desktop harness, the ignored test `a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left`, with `BRN_DWR_SHARED_DATA`, `_SOURCE` and `_CHAT` set and `-- --ignored`: **ok**. It ran in `b3fb033a-…:0`, with no new chat and one more Run.
3. CLI intake again gave `reused_chat: true`, the same `thread`, and one chat in total.

Persisted state after all three: 1 Thread, 3 Runs in it (all Failed), 1 Source (general, `chat: null`), 3 owner Messages, and **1** Blocker: "Run failed: The provider account needs reconnecting. Continue when the provider is ready.". No Run was live at any step, so reuse was expected.

## Repository checks (all run once on the final tree)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed (after `cargo fmt --all` reflowed two new tests) |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | passed |
| `cargo build --workspace --locked --offline` | passed (fresh `brn` and `brn-desktop` for the checks below) |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | passed |
| `cargo test --workspace --locked --offline --no-fail-fast` | **706 passed, 12 failed, 4 ignored** |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **150 passed, 0 failed, 1 ignored** (the env-gated cross-process test) |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |
| `brn-desktop --headless-check startup --data-dir /private/tmp/brn-dwr-data/round3-headless/data` | `Threads ready: 2 records`, exit 0 |
| `TMPDIR=/private/tmp/brn-dwr-data/round3-e2e scripts/verify-end-to-end.sh --fixtures-only` | exit 0: provider retirement, intake retirement and offline Save/recovery/search/export/backup/startup/refusal journeys passed |
| `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/round3-journey` | exit 0 (observations below) |
| `git diff --check` | clean |
| `python3 scripts/check-markdown-links.py` | 83 files, 366 local links, 0 failures (including this note) |

**Workspace failures:** all 12 are the known `brn-threads-intake` sandbox tests, because this agent shell cannot apply a `sandbox-exec` profile (`Err(Unavailable)`). They are `process::tests` (3) and `tests/qualification.rs` (9, including `cancellation_after_pdf_copy_removes_raw_input_before_return`, whose result varies by run here). There are no other failures. Against the post-rebase run (702 passed, 11 failed), this round adds 5 passing tests (core 1, app 1, CLI 1, brn-ai 2 net), and the variable cancellation test failed this time.

**CLI journey, changed observations:** `second_session_error` and `begin_edit_current_while_old_open` are now the `edit_session_open` JSON with the open session's id and `base_version: 1` (before: `SQLite: UNIQUE constraint failed: edit_sessions.note`). Both Needs you chats show one Blocker each: "Run failed: The provider account needs reconnecting. Continue when the provider is ready.". `useful_intake_new_threads` stays 1 and the email Source stays in the project chat. Restore and refusal results are unchanged.

## Evidence classes

- **Automated:** everything above.
- **Native observation:** none. The desktop checks use the headless gpui state harness, not the Mac UI.
- **Unverified:** how the UI branch renders the new notices and the U4 notice. Whether a real provider run in a reused chat avoids duplicate notes (model quality, needs authorized live runs).

## Integration items for the UI branch

1. **Desktop files edited here:** `crates/brn-desktop/src/threads_ui.rs` (the `useful_intake` call with a short project lookup, and a `mod intake_tests` declaration) and the new `crates/brn-desktop/src/threads_intake_tests.rs`. Expect a small textual conflict if the UI branch touches the module list or `useful_intake`.
2. **Notice text changed.** The tests the UI branch is changing should match the table above, or better, assert on `brn_ai::threads::INTERRUPTED_ATTENTION` and the `Run failed: ` prefix. Saved chats from older builds are still replaced.
3. **U4:** match `AppError::Core(Error::EditSessionOpen { note, session, base_version })` to offer Compare/Discard. `Workspace.store.edit_session(&session)` returns the buffer and its generation for Discard. Until then, the generic notice shows the non-raw Display.
4. **Project choice timing:** `useful_intake` reads the open chat's project when conversion finishes, not when the file was chosen. If the owner switches chats during conversion, the source follows the chat open at the end. Capturing it at choose time needs a `WorkerEvent::Converted` field, which is UI-owned plumbing.
5. **Repeated instruction messages:** each desktop supply still posts its instruction (unchanged logic), and the CLI and desktop instruction texts differ. A reused chat therefore gets one owner Message per supply, and in the CLI-desktop-CLI check there were 3 for 3 supplies. The CLI alone does not repost on an identical retry.
6. **CLI JSON errors** go to stderr. `HELP` does not mention `edit_session_open` yet, to avoid a conflict with `work/cli-agent-contract`.
