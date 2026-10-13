# Standalone chat candidate evidence

**Date:** 13 October 2026. **Scope:** stage one of the [accepted brief](spec.md); [plan](plan.md), [research](research.md). Projects, publication, merge, release and real owner-data access are unassigned. The owner authorized native queue, steering and attachment end-to-end testing with synthetic data and selected-provider calls on 13 October; the follow-up evidence below supersedes the earlier native deferral for those journeys.

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
