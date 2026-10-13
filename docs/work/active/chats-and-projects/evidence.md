# Standalone chat candidate evidence

**Date:** 13 October 2026. **Scope:** stage one of the [accepted brief](spec.md); [plan](plan.md), [research](research.md). Projects, publication, merge, release, live providers and real owner-data access are unassigned. Native observation remains explicitly deferred by repository authority.

**Baseline:** `65119a73a6aaea13764b8bc6290f7bb6dc79ff2d` (PR #128), isolated branch `codex/thread-product-discovery`, checkout `/Users/evokessler/.codex/worktrees/thread-product-discovery/brn-rust`. Original checkout `work/copilot-provider-fixes` and its unrelated `experiments/` remain unchanged. **Implementation commit:** `73211572e74ab658e748aa28950365ac2d40776e`. Subsequent qualification-record commits change only task documentation/witness artifacts; implementation remains this pinned source.

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

## Owner qualification checklist — pending, native observation deferred

Use a fresh synthetic data directory with disposable public files when native qualification is resumed:

1. Ask a short and a long question; read/select/copy while the answer streams. Scroll upward, switch chats and return; verify the reading position and Jump to latest. Check keyboard focus, IME, themes and narrow-window controls.
2. Enter two follow-ups while BRN answers; verify ordered separate turns. Edit/remove/reorder waiting items. Use Steer and verify the explicit stopped/fresh transition and preserved earlier partial answer.
3. Stop, provider failure and cold restart with queued messages; verify saved drafts/partial progress/queue and no automatic restart. Resume deliberately. Archive while streaming; inspect saved partial history and independent attention/Actions, then Unarchive without restarting.
4. Attach representative supported documents, inspect figures and coverage gaps, queue a question during preparation, remove/move the original and reopen BRN. Verify continued reading, private exclusion and explicit Keep/Undo on the same note.
5. Repeat existing writing/comment/rewrite/review/Undo workflows beside chat. Confirm current settings/authority and errors remain understandable in quiet mode.

Automated widget/state tests do not establish native typing, focus, dialogs, IME, pointer feel, selection behavior or live provider streaming. Owner **Accepted/Rejected with observation** remains pending. Required hosted CI is unrun because this build assigns no publication/integration; local green checks do not establish merge readiness. Projects and the inherited project BRN-knowledge switch remain stage two.
