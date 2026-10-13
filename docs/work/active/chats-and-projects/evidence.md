# Standalone chat candidate evidence

**Date:** 13 October 2026. **Scope:** stage one of the [accepted brief](spec.md); [plan](plan.md), [research](research.md). Projects, release and real owner-data access remain unassigned. The owner accepted the chat changes and authorized publication/merge once required checks pass on 13 October; integration evidence is recorded below. The owner authorized native queue, steering and attachment end-to-end testing with synthetic data and selected-provider calls on 13 October; the follow-up evidence below supersedes the earlier native deferral for those journeys.

**Baseline:** `65119a73a6aaea13764b8bc6290f7bb6dc79ff2d` (PR #128), isolated branch `codex/thread-product-discovery`, checkout `/Users/evokessler/.codex/worktrees/thread-product-discovery/brn-rust`. This task made no edits in the original checkout; its unrelated `experiments/` are preserved. The original checkout was on `work/copilot-provider-fixes` at start and is now externally advanced to `main` at the same `65119a7` baseline; task implementation remains isolated. **Implementation commit:** `73211572e74ab658e748aa28950365ac2d40776e`. The original automated qualification below describes this source. **Native follow-up candidate:** `8bd4dafbc77736ade5a3d4b1d92b0871efbd03c9`, with the demonstrated native fixes recorded below.

## Delivered behavior

- Chats remain in history; Archive stops work and preserves attention/Actions, and Unarchive leaves the queue paused. Legacy Thread IDs remain stable; UI vocabulary is Chat.
- Enter queues during work; Steer separately fences/posts a new instruction and starts fresh after the old worker drains. Stop/failure/disconnect/restart save waiting work and require Resume. Exact dispatch claims, Message/Run identities and terminal fences prevent duplicate or late completion from advancing/re-pausing newer work.
- Composer recovery stores the latest generation per destination; delayed clears fail without replacing newer text. Quiet is independent of provider settings. Selectable answers stream with stable response identities, Copy and per-chat MessageScroller state/Jump to latest.
- Chat attachments preserve converted readable content, figures and coverage limitations after the original disappears. General lists/search/export and scoped runtime reads/writes exclude private content, including direct IDs, historical revisions, Source observations, typed references and managed image URLs. Full backups preserve it.
- Keep retains the same note/protection/content with version checks, receipt replay and Undo. All referenced owned figures retain their IDs and receive eligible source observations; borrowed same-chat figures are copied onto the retained note without promoting their source document.
- Schema 3 upgrades require a verified external pre-upgrade snapshot and safe writer/edit preconditions. Known old backups upgrade only an unpublished restore copy. Existing global import operation hashes remain replay compatible.

## Automated evidence

Environment: macOS arm64, repository Rust 1.98.1/lockfile, GPUI Kit 0.7.3, Rig 0.44.0. Reused build cache `CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010`. Synthetic fixture parent `/private/tmp/brn-chat-fixtures`; explicit existing helper/Poppler paths. No account/credential payloads, actual provider calls, real owner data or native interaction were used.

| Check | Result / local log |
|---|---|
| `scripts/verify-storage.sh` | Pass: formatting, helper/workspace build, strict default workspace Clippy, **484 passing tests**, headless shipping-entry startup/refusal. `/tmp/brn-chat-first-logs/storage-gate.log`. |
| `scripts/verify-end-to-end.sh --fixtures-only` | Pass: offline Save/recovery/search/export/backup/startup/foreign-data refusal and provider/intake retirement checks. `e2e-gate.log`. |
| Dedicated CLI chat witness | Pass: synthetic v3 adoption refuses without backup; exact old v3 snapshot; chat attachment survives original unlink; private search exclusion; queue/archive pause; Keep and full backup. [`cli-witness.py`](cli-witness.py), `chat-cli-e2e.log`. |
| `cargo build --locked --offline -p brn-desktop --features native-ui` | Pass, production native source. `native-build.log`. |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::` | **87 passed**, zero failures; nonzero widget/state suite. `native-tests.log`. |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | Pass. `native-clippy.log`. |
| Development tooling | **36 passed** via `python3 -m unittest discover -s scripts/tests -v`; `tooling.log`. |
| Markdown / formatting / diff | Current Markdown links, `cargo fmt --all -- --check`, and `git diff --check` pass. |

Logs without an absolute prefix above reside under `/tmp/brn-chat-first-logs/`. Three ignored default-suite entries are isolated test children launched by their parent tests; they are not counted as independently passing tests. Two native tests outside the requested `threads_ui::` filter are covered by the default workspace suite. The pinned `block 0.1.6` dependency reports a pre-existing future-Rust incompatibility warning; it does not fail the current repository toolchain.

Focused proof includes competing dispatchers, pending-message reuse, stale claims/completions, Stop/Resume races, partial failure/Stop retries, archive attention preservation, attachment/asset/history/source/typed-reference isolation, copied/general-write attempts, global/private same-locator observations, Keep/Undo and multi-source figures, migration/restore, navigation/restart drafts, quiet/Stop, queued questions during conversion, Archive/Steer partial retention, image resolver scope and long-chat reading position.

## Independent review

A read-only reviewer assessed the full tracked/untracked diff against the baseline, brief, contracts and invariants. Final verdict: **APPROVED**, no remaining blocking findings. Lead confirmed and corrected the findings; the reviewer did not edit code or claim test execution.

Resolved findings: foreign typed references; retry after partial response; stale failure after Resume; exact scoped Source selection; old import hash replay; managed figure URL/render scope; dispatch after attachment/non-provider completion; Archive/Steer partial retention; attachment replay before mutable destination checks; owned multi-source figure eligibility after Keep. The queue/nonqueued-start race and execution-control Undo paths were tightened as well. A final vocabulary pass and removal of obsolete auto-resolution-only test helpers were covered by the final native suite/lint.

## Owner qualification checklist — acceptance pending

The scoped native follow-up below covers selected journeys; broader owner qualification remains pending:

1. Ask a short and a long question; read/select/copy while the answer streams. Scroll upward, switch chats and return; verify the reading position and Jump to latest. Check keyboard focus, IME, themes and narrow-window controls.
2. Enter two follow-ups while BRN answers; verify ordered separate turns. Edit/remove/reorder waiting items. Use Steer and verify the explicit stopped/fresh transition and preserved earlier partial answer.
3. Stop, provider failure and cold restart with queued messages; verify saved drafts/partial progress/queue and no automatic restart. Resume deliberately. Archive while streaming; inspect saved partial history and independent attention/Actions, then Unarchive without restarting.
4. Attach representative supported documents, inspect figures and coverage gaps, queue a question during preparation, remove/move the original and reopen BRN. Verify continued reading, private exclusion and explicit Keep/Undo on the same note.
5. Repeat existing writing/comment/rewrite/review/Undo workflows beside chat. Confirm current settings/authority and errors remain understandable in quiet mode.

The original automated widget/state tests alone do not establish native usability. The scoped native observations below cover file dialogs, typing, streaming and interruption for these fixtures; general IME, selection, pointer feel and broader daily-use behavior remain unqualified. Owner **Accepted/Rejected with observation** remains pending. Required hosted CI is unrun because this build assigns no publication/integration; local green checks do not establish merge readiness. Projects and the inherited project BRN-knowledge switch remain stage two.

## Native end-to-end follow-up — 13 October 2026

Authorization: “Can you test the queue, steering and attachments in the app end to end and fix any issues you find?” Original owner trial data was preserved. Native automation used a separate workspace at `/private/tmp/brn-chat-native-e2e-20261013-210453/data`, with repository synthetic PDF/DOCX fixtures and normal Codex `gpt-6.1-sol` / medium configuration. No private source documents or credential payloads were inspected/copied. A nonexistent credential path was used only in this test workspace to exercise failure; normal configuration was then restored.

Candidate `8bd4dafb` fixes two native-observed presentation defects: long queue/attachment labels pushed controls outside the chat tile, and an old interruption/paused notice remained while a Steer replacement ran. Queue labels now truncate within available width, controls retain their width, and attachment titles have a separate line above full wrapping coverage/Inspect/Keep controls. Successful worker start clears the old notice; startup errors remain visible.

Native observations and checked-state corroboration:

| Journey | Observed result |
|---|---|
| PDF attachment through native picker | Saved protected, chat-only note; substantive text and retained figure asset. The model read title, table values and appendix facts from the saved note. Coverage limitations remained visible. After the original moved, its retained figure rendered in a native chat answer. |
| Two Enter follow-ups | Separate sequential turns; exactly one saved assistant answer per queued request. |
| Stop / edit / reorder / normal restart | Streamed partial answer saved as partial; both waiting messages remained paused. Edit and reorder persisted. Unsent per-chat draft recovered after reopening. Resume ran the edited order. |
| Steer with waiting follow-up | Old run interrupted; displayed partial saved. New checklist answered the replacement request, then one waiting turn ran once. Repeated on rebuilt candidate with a short replacement; no stale paused notice appeared during new work. |
| DOCX / ordinary question / Keep | Native picker converted the process fixture. An ordinary three-bullet summary request read it without tool-specific instructions. General notes stayed at zero until explicit Keep; the same protected note advanced from version 1/private to version 2/general, with no duplicate. |
| Abrupt exit / failed Resume / reopen | Two waiting items survived SIGKILL of only the test process and stayed paused. Missing test credentials produced a Failed run, retaining exact pending Message and waiting item. Reopening stayed paused. Restored configuration plus Resume reused the exact owner Message; one successful retry and one next answer drained the queue. |
| Rebuilt layout / queued removal | Long saved-turn controls stayed within the chat. Narrow chat beside attachment inspection kept full coverage, Inspect and Keep visible. A removed waiting item was absent from posted messages/model turns. |

Local evidence under `/private/tmp/brn-chat-native-e2e-20261013-210453`: `candidate.json` pins executable hash/source; `initial-queue-records.json`, `stopped-queue.json`, `steering-and-resume.json`, `before-cold-restart.json`, `failed-resume.json`, `recovered-queue-complete.json` `attachment-scopes.json`, `final-native-records.json` and `final-search-witness.json` corroborate checked records. Native tool observations establish the visible flows separately from these records. Original PDF was moved away from its source locator before reopen/read/retry.

Affected final checks: production native build passed; all **89 native widget/state tests passed**, including the two new geometry/notice regressions; strict native Clippy passed; formatting, Markdown links and diff checks passed. Logs `native-build.log`, `native-tests.log`, `native-clippy.log` reside in that evidence directory. The same pre-existing `block 0.1.6` future-Rust warning remains. Independent read-only review of the complete follow-up diff, including the full-coverage wrapping refinement: **APPROVED**, no actionable findings.

Qualification: queue/Steer/Stop/failure/restart and representative native-picker PDF/DOCX attachments are now native-observed against synthetic fixtures. This does not qualify every format, file drag/paste surface, complex source fidelity, general IME, broader recovery or daily use. Owner acceptance, hosted CI, merge and release remain pending/unassigned.


## Quiet off terminal activity — 13 October 2026

Owner observation: Quiet on/off did not feel different. Owner clarification: Quiet off must show tool calls and agent reasoning with a terminal/CLI experience. Candidate `14ec5c385ba591f74d04e4c103d21560308ea7e1` follows qualification baseline `293a7b1b` in the same isolated checkout; no publication, merge or release was assigned.

Diagnosis reproduced the global status-bar bypass both in the actual app and a failing native widget regression (`/private/tmp/brn-chat-first-logs/quiet-regression-red.log`). The final UI has a selected, explicit Quiet toggle, compact Quiet status throughout the window, and an independent bounded monospace activity pane above the composer when Quiet is off. The latest run's trace remains inspectable in its own chat after completion. Moving it outside MessageScroller fixes the native-discovered nested wheel-mask conflict. Activity keeps only 32 entries in memory, with bounded argument/public-summary previews; starting another run or reopening the app clears it. Saved answers, checked receipts and meaningful progress keep their existing durable paths.

Rig dispatch/outcome hooks expose tool names and bounded argument previews. Record write bodies and tool return bodies are omitted. Apply results distinguish Applied, NeedsReview, Deferred, Stale and Superseded rather than treating every successful tool return as a saved write. Responses request explicit public reasoning summaries with the selected effort preserved. Only completed native `summary_text` is displayed, separately from answers; raw reasoning, redacted text and encrypted payloads are excluded. Provider boundary source: [OpenAI Responses streaming events](https://developers.openai.com/api/reference/resources/responses/streaming-events).

Evidence directory `/private/tmp/brn-quiet-native-20261013-213523` contains candidate/binary identity, fresh synthetic workspace, bundle and check logs. Provider calls used Codex `gpt-6.1-sol` / medium against four synthetic route notes (12, 19, 15 and 24 minutes). Original owner-trial data/app were preserved.

Checks against the candidate source:

- `CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010 cargo test --locked --offline -p brn-ai`: **226 passed**, one existing ignored live-provider test; no self-skipped passing claim. Public-summary boundary and truthful Apply outcome tests pass. The mocked nine-tool production runtime confirms dispatched calls and outcomes.
- `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop`: **92 passed**, including Quiet/global status, unchanged run/answer/provider, bounded retention, failure visibility, per-chat scope, unrelated-worker/disconnect isolation and independent activity wheel scrolling.
- Strict native Clippy with `--all-targets -- -D warnings`, production `cargo build --locked --offline -p brn-desktop --features native-ui`, formatting, Markdown links and diff checks: **passed**. The existing `block 0.1.6` future-Rust warning remains.
- Independent read-only review of the full follow-up and final scrolling refinement: **APPROVED**. Corrected findings: unrelated worker status/Stop in retained activity, unrelated disconnect replacing retained outcome, and misleading Apply success labels.

Actual native observation on the intermediate build confirmed real `read_note` calls/argument previews/outcomes, provider public summaries, continued answer streaming under Quiet, compact global status and unchanged Run ID/provider/model/effort across toggles. Completed trace and truthful Completed terminal state remained visible. Final native retest against `14ec5c38` passed: a second selected-provider run read all four synthetic notes through read_records/read_note and completed with the exact times. Scrolling the separate activity pane revealed earlier calls without moving the answer; Quiet on hid the completed trace and Quiet off restored it. The verified app was left open with Quiet off. Candidate binary SHA-256: `b0ebcdf12663c258f6ab837ee7ecdb8d07eecfc8fea515ac0a53a90cb8b2181a`; `candidate.json` and `native-qualification.json` pin the tested build and run outcomes. Owner acceptance remains pending; this evidence does not establish broader daily-use qualification or merge readiness.


## Reopen work status and remove duplicate model controls — 13 October 2026

Owner clarified that the completion banner should remain optional but be recoverable after ×, and reported duplicated model/effort controls in the composer and status bar. Candidate `4b235b9770ad3ee47bf6857c51b263cd4f53b19f`, on the existing isolated branch, keeps the latest completed worker status in memory and adds **Layout → Last work status**. Dismiss hides the banner; restoring it does not dispatch work or replace a newer visible error/status. Restore is unavailable during active work. It is a same-session control; saved answers/receipts retain their existing durable history. The idle status bar now shows **Idle**, with provider/model/effort selectors in the composer and settings. Active progress and Stop are preserved.

The exact dismiss/menu/restore regression first failed because no restore option existed (`banner-red.log`), then passed. Final native suite: **94 passed**. Strict native Clippy, production native build, formatting, Markdown links and diff checks passed; the pinned block warning remains unchanged. Independent read-only full-diff review against `07f303cc`: **APPROVED**, no actionable findings. No provider/runtime/core dependencies changed.

Actual native observation used fresh synthetic data in `/private/tmp/brn-status-banner-test-20261013-9c6kspja/data` and **BRN Status Test.app**. A single short Codex `gpt-6.1-sol` / medium request produced the exact requested answer and a Completed run. × hid the completion banner; Layout → Last work status restored the same banner; canonical records still contained exactly one Completed run. The actual window showed a plain Idle bottom status and a single composer model/effort section. The updated app was left open; prior trial data/apps were preserved. Candidate/binary identity and qualification receipts are in that evidence directory, alongside check logs. Binary SHA-256: `2cc2ba0cfd75feabd13f4fd16616b0729b68c5b076e53c67c1a8d80cb0ccf1cb`.

Implemented, automated-verified and native-observed; owner acceptance of this follow-up remains pending. Publication, merge and release remain unassigned.


## Contextual Edit note and Save as decision — 13 October 2026

Owner authorized replacing the composer's Ask/Edit note/Record decision menu with contextual actions. Candidate `7f2b4fc7d824869aa33b2f56195c092452ccec55` follows `0ef0f81c` on the same isolated branch. The menu and ComposeMode are removed; ordinary Enter, queued messages and Steer carry no additional note or decision grant. **Edit note** appears with the open saved note and collects a separate instruction bound to the exact current note/version. **Save as decision** appears on completed saved replies, opens editable title/text review and explicitly saves the exact owner-reviewed content in general BRN as protected and confirmed. It does not invoke a model or promote the chat or documents.

The shared decision use case commits the note and chat link in one owner group, with source-message/current-chat provenance, SHA-bound operation identity, replay and Undo. Private/missing figures refuse save without silently promoting their documents; eligible general figures keep their exact references and version inputs. Dialog changes replace candidate identity; unchanged retries reuse it. Note editing refuses dirty/recoverable/stale notes and pending/busy work, uses the existing EditNote invocation grant, and preserves composer drafts. First writing-chat creation moves its unsent draft/recovery into that chat after verified target persistence.

Final automated checks: **58 shared-app tests passed**, including four new broad decision journeys; **97 native tests passed**, including contextual dialog typing/cancel/save, exact edit target, stale/dirty/queue refusal, preserved drafts, first-chat recovery, inline figure errors and preview-edited retries. Retired mode-menu tests were replaced with behavioral contextual-action tests. Strict app/native Clippy with all targets and `-D warnings`, production native build, formatting, Markdown links and diff checks passed. The pinned block warning remains unchanged.

Independent read-only full-diff review: **APPROVED**. One real retry finding was fixed: draft handoff verification now precedes posting the instruction, so two refused generation-conflict retries post zero Messages, then a recovered retry posts exactly one. The native typing tests also exposed an editor-height/focus issue; full-size dialog editors now accept clicks throughout their text areas. No remaining findings.

Fresh native workspace `/private/tmp/brn-chat-actions-test-20261013-9jqe4iqc/data`, **BRN Chat Actions Test.app**, Codex `gpt-6.1-sol` / medium. A synthetic saved reply exposed Save as decision. Cancel changed no notes. Editing title/body then saving produced the exact Unicode/newline text in a protected, confirmed note linked to the original chat. Its reply and unsent chat draft remained visible and unchanged; records still had exactly one Completed run, proving save made no extra model call. Contextual Edit note Cancel made no change. An explicit edit of the protected note committed the requested Beta content as v2, kept protection and correctly invalidated prior confirmation; the unrelated route-plan fixture stayed at v1. Native observation exposed the stale editor snapshot, corrected in follow-up `6610a0e4`. Final rebuild/retest on the same synthetic data then committed Delta as v3 and automatically refreshed the visible note text/version, preserving the chat draft (also recovered after restart). Exactly three Completed runs account for the initial reply and two explicit edits; decision capture itself dispatched none. Candidate/binary receipt and logs are in the evidence directory; binary SHA-256 `f73f4ff5582cfef29c891a1497c04082c5a8f0f85dd6cc3b0c5a89e4d149b9a9`.

Owner acceptance and integration remain separate; publication, merge and release are unassigned.


Final refresh candidate `6610a0e448c308e75288314c42b5c6e7fdaa43f6`: **98 native tests passed**, including unchanged-editor refresh, preserved human typing and no reopening of hidden note tiles. Strict app/native lint and production build passed. Refresh metadata is tied to the exact EditNote Run/base; dirty/recovery/review/hidden editors retain their state. The final binary SHA-256 is `1248d3d4f9016b51e4f9f6b694b1b059adf12ab2504c764a325fce0f33f902d6`; candidate/native qualification receipts and final check logs are in the native evidence directory. **BRN Chat Actions Test** was left open on the updated protected note with its unsent chat draft. Implemented, automated-verified and native-observed; owner acceptance remains pending and integration/release unassigned.


Independent read-only final review of the complete contextual-action changes from `0ef0f81c` through `6610a0e4`: **APPROVED**, no actionable findings remain, including hidden/review/recovery/typing refresh guards and disconnect metadata cleanup.

## Owner acceptance and integration authorization — 13 October 2026

Owner **Accepted**: “Looks good. Let’s merge the chat changes into main once the required checks pass.” This supersedes earlier pending/unassigned owner-acceptance and publication/merge statements, which describe their dated candidates. Projects, release and real owner-data access remain outside scope.

Remote baseline refreshed to `f6babfbf` (#129). Its ordinary wiki runtime guidance and separate UI prototype records are preserved. GitHub main protection was read directly: strict up-to-date required contexts are Core and CLI (ubuntu-24.04), Core and CLI (macos-15), Native UI build and state (macos-15), and Native retrieval and combined build (macos-15); admin enforcement is enabled, force pushes/deletions disabled. Hosted CI and merged-tree/main verification are pending. Final receipts will be recorded on the integration PR.
