# S6 Inbox, imports, tasks, settings, data, recovery — evidence

Branch `work/ux-s6-inbox` from `origin/work/ux-rebuild-20261105`. Status: implemented and verified (automated); partly native-observed on synthetic data (see [native observation](#native-observation)). Not owner-accepted, not merged.

## Commits

| Commit | Content |
|---|---|
| `532e02a2` | Move `overview_tile`, `actions_tile`, `history_tile`, `settings_tile` out of `threads_tiles.rs` into S6 files; add For you and Imports views |
| `88c90ee9` | For you leads with the originating chat or note; Home tests adapted to For you |
| `6df66bd4` | Tasks detail panel: recorded facts, explicit completion, checked Due and Follow up |
| `bcc2de7e` | Recent changes restyle; Settings section list, Appearance, Data and credential truth |
| `5730bbaf` | Recovery wording points to Inbox → For you (CAP-137) |
| `cfe3d362` | Tasks: centred page frame with lead line |
| `c57592fc` | Imports: a scan with failed files finishes in the warning tone |
| `a706e5f6` | Settings section list left-aligned |
| this file | Evidence |

## What moved where

| File | Now holds |
|---|---|
| `threads_inbox_view.rs` | Inbox page frame and lead, For you (Needs your review / Failed / Waiting on someone else, each row opening its exact chat or note; caught-up state with a capture action; Recent changes pointer), Imports page, Recent changes body |
| `threads_inbox.rs` | Manual folder inbox panel: separately confirmed folder, General-knowledge destination consent, explicit Scan, per-item progress, cancel, failures (filename kept) and retry by scanning again |
| `threads_tasks.rs` | Tasks list (summary fields, state chips) and detail panel (state, reported owner, waiting-on, separate Follow up and Due, checked edits, explicit Confirm done) |
| `threads_changes.rs` | Recent changes receipts with Undo where valid |
| `threads_settings.rs` | Settings section list (Appearance, General, Models, Delegation, Documents, Data, Skills, MCP) and sections |
| `threads_restore.rs` | Restore panel with the real post-restore exceptions (`RESTORE_EXCEPTIONS`) shown before and after publishing |
| `threads_skills.rs` | Truthful Skills empty state |

Shell hooks (smallest possible, in lead-owned files): `TileKind::Imports` (layout label/key, shell icon, Inbox local switch For you / Imports / Tasks), and the sidebar Inbox count now comes from `for_you_count()` so it counts only actionable items. `threads_ui.rs` changes: Imports dispatch and the Inbox scan warning tone.

## Capabilities

| CAP | Outcome |
|---|---|
| CAP-008 / CAP-039 Home | Attention content is Inbox → For you (S2 already routed ⌘0 to New chat); `overview_tile` survives only as For you's body |
| CAP-137 "Recover it from Home" | Wording now says Inbox → For you |
| CAP-207 retry-capture | Verified wired: a failed capture offers Retry, which calls the existing capture service (covered by intake tests) |
| CAP-007 Recent changes | Workspace menu (S2) and the For you pointer |
| Settings rows | "Keys and mouse" is now "Keyboard shortcuts" in General; S1 Appearance controls kept and restyled only |

## Tests removed or replaced

| Before | After | Reason |
|---|---|---|
| `home_shows_the_intro_only_on_first_run` | removed | Home is retired; first-use content is S3's New chat surface (asserted by S2/S3 tests) |
| `a_clicked_settings_section_takes_the_focus_ring_off_general` | `…_off_appearance` | Settings now opens on Appearance; same focus assertion |
| Home/overview clicks in tiled, dialogs, intake, omakase tests | Inbox → For you / Imports equivalents, `settings-section-*` ids | Same behavior through the new entry points |
| — | `a_task_opens_its_detail_panel_and_completes_explicitly` (also asserts the lead and centred column) | New detail panel |
| — | `follow_up_dates_are_checked_in_owner_terms` | Invalid dates keep the form open with input intact |
| — | `settings_lists_its_sections_and_data_says_what_each_copy_is` (1280 and 900×600) | Section list and Data copy |
| — | `models_shows_unavailable_credentials_truthfully` | Credential truth |
| `scan_button_runs_stock_worker_…` | same test, extended | Adds an invalid `.docx`: 2 entries, the failure keeps its filename, the scan finishes in the warning tone |
| restore tests | extended | Asserts the three restore exceptions |

## Commands and results

`--locked`, `--jobs 4`, fresh `TMPDIR` under `/private/tmp`, `BRN_XBERG_HELPER` / `BRN_XBERG_HELPER_BRN` from `target/document-helpers/artifacts`. Final run at `a706e5f6`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p brn-writing-ui -p brn-desktop --all-targets --features brn-desktop/native-ui,brn-desktop/native-test-support -- -D warnings` | pass |
| `cargo test -p brn-desktop --features native-ui,native-test-support` | 393 passed, 1 ignored |
| `cargo test -p brn-writing-ui --features native-test-support` | 48 + 12 passed |
| `cargo build -p brn-desktop --features native-ui` | pass |

## Native observation

`brn-desktop --data-dir /private/tmp/brn-s6-data.*` (fresh, synthetic: two notes, chat "Festival budget questions", project "Harbour festival", harbor.docx via CLI intake, four tasks). Window-only captures (`screencapture -l`) in `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s6/`; window 1280×912 (screen height limit). Driven through the Accessibility API (AXPress by accessible name); file panels were real NSOpenPanels.

Captured: `light-1280-{for-you,imports,tasks,task-detail,imports-confirm-folder,scan-result,scan-again,settings-appearance,settings-general,settings-models,settings-skills,settings-data}`. The Light `settings-models` and `settings-skills` shots predate the section-list alignment fix; `settings-data`, `settings-appearance` and `settings-general` show it.

| AC | Native | Automated |
|---|---|---|
| AC-15 manual folder scan | Folder chosen in NSOpenPanel, separate General-knowledge consent, explicit Scan of `/private/tmp/brn-s6-intake` (one `.md`, one invalid `.docx`): 1 imported, 1 failed keeping its filename ("Document conversion failed: Invalid"); rescanning created no duplicate | `threads_inbox_tests` |
| AC-16 task completion receipt | Not captured (screen locked mid-pass) | `a_task_opens_its_detail_panel_and_completes_explicitly` |
| AC-18 preference persists across restart | Not exercised natively | Settings tests |
| AC-35 export, backup, restore | CLI-driven against the same data dir (shared service): Markdown export (4 notes, 1 asset, manifest), full backup, restore into a fresh isolated data dir (4 notes, matching the source). UI panels not driven | restore/backup tests, restore exceptions |
| AC-38 converter offline | Stock xberg conversion of `.docx` and `.md` offline (CLI intake and Scan); provenance shown in Imports | intake tests |

Observed and fixed during the pass: Tasks page left-hugging without lead (now centred page frame), scan with failures shown in the success tone (now warning), Settings section names centred (kit buttons centre labels; now a left-aligned filling child, accessible name unchanged).

Operational note: overwriting the copied app binary in place makes macOS kill the next launch (code-signature cache); replace the file (`rm` then `cp`) between runs.

## Known gaps

- Dark captures, VE-05 (error in Dark), VE-06 Dark task completion receipt, VE-07 (900×600 keyboard focus) and AC-16/AC-18 native runs are missing: the screen locked during the pass and window capture/AX stopped working.
- Status chip icons (e.g. task state) render blank; the chip is the shared `components::status_chip` in `brn-writing-ui`, outside S6.
- Fixed at integration (lead): a rescan now reports unchanged files separately ("N unchanged") in the scan summary and Folder inbox ledger.
- Fixed at integration after independent review: Retry keeps the original attempt's Project (refused if it is no longer available) instead of re-deriving it from the open chat, and an intake or full-note publication refused after a successful conversion is now a failed, retryable attempt. Covered by `import_retry_keeps_its_project_and_reports_intake_refusal`.
- Documents and MCP settings sections are restyled by the shell only; their bodies are unchanged.
- No live provider was used; Models shows credential status only.
- VoiceOver, large text and IME were not exercised.
- For you shows routine delegated maintenance only as a pointer to Recent changes.
