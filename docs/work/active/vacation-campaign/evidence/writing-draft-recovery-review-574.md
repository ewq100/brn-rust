# Independent draft-recovery correction review

Verdict: **prior P2 valid/fixed and verified; one new P2 dismissal/provenance finding in the correction**. Reused the complete690 Store/source review; no additional Store or draft-transfer defect was found.

## Exact candidate and complete correction

- Checkout `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/writing-draft-recovery`.
- Frozen clean HEAD `5744d102573134474c38ae1352ea1abfccb53567`, direct parent `690c2c3f31a0f11e3405f2f2d0479be024190eb8`.
- Whole original task base remains `bc8a08e64b0dc1e2b33ff181dfdb54b62f59f5e4`.
- Complete parent→candidate correction is five paths: desktop README, `threads_notice.rs`, `threads_shell.rs`, `threads_ui.rs`, `threads_writing_tests.rs`; 226 additions / 9 deletions.
- Independent binary diff SHA-256 matches `18587c448196b6c3f6d320e15a693e0e2988a46ddeaeec1db805d79110f4c2f7`.
- No Store/schema/dependency/provider/runtime-prompt or CI change. Worker remained clean at final identity inspection.

Applied previously loaded independent-review/resource and repository guidance. Inspected complete correction, complete NoticeState, all notice method callers, background completion, actual rendered Dismiss callback, handoff success/failure and new tests. No build, test execution, source/Git mutation, API/account/network/native action, merge or delegation occurred. Only this report was written.

## Prior P2 disposition

**Valid/fixed, independently verified at574.** Recovery attention is now an explicit per-Note entry keyed by Note ID rather than an ordinary notice string. `current()`/tone select that attention, `is_work_status` correctly returns false while it is displayed, and `can_restore`/restore refuse to replace it. Ordinary `set`, background `clear` and `complete_work` preserve the scoped attention, while completed work is still retained in `last_work`. A committed handoff clears only the same Note's entry. Messages identify the Note. No string matching or blanket sticky-error policy was introduced.

The new actual-callback witness installs a synthetic existing output worker, invokes `post_comment` to produce the real occupied-target refusal, delivers Finished through `poll_worker`, then checks unchanged destination/editor/source/target caches, generations, stored draft rows and canonical Note before the notice assertion. Its saved690 red run compiled and failed at the expected visible-warning assertion: actual background completion versus required recovery warning. The corrected witness passes and checks the rendered banner plus actual Dismiss control and later Last work status. This meaningfully reproduces and fixes the original asynchronous overwrite; it does not depend on a live provider.

The second UI witness resolves two failed Notes independently, including success for a Note other than the still-attended Note. The state witness also checks unrelated Note success cannot acknowledge another entry. Existing ordinary notice/work provenance remains unchanged when no recovery attention exists. The new Dismiss edge below is not covered by these tests.

## New finding

**[P2] Dismissing recovery attention also discards an unseen ordinary error.** Location: `crates/brn-desktop/src/threads_notice.rs:89–93`; relevant priority at `:26–33`, actual button at `threads_shell.rs:1346`, and production error example at `threads_ui.rs:1379–1381`.

Concrete sequence:

1. A refused Note handoff creates recovery attention A, which is the rendered banner.
2. While A is unresolved, another actual operation fails—for example note edit recovery calls `fail("Could not preserve this edit ... Keep this window open or copy your text.")`. `set()` stores it as `Presentation::Ordinary`, but `current()` still displays A.
3. The owner clicks Dismiss on A. `dismiss()` removes A and unconditionally calls `clear()`, erasing the underlying ordinary error too. The owner never saw or acknowledged that failure. It cannot be restored by Last work status, which only retains `last_work`.

This crosses the new recovery-attention boundary: acknowledging the displayed scoped handoff warning acknowledges a distinct hidden failure as well. It can remove a consequential copy/keep-window instruction. It is not a request to make all ordinary errors sticky or to redesign notice handling.

Keep dismissal scoped to the displayed recovery entry and preserve any distinct pending ordinary notice for its normal presentation/dismissal. Existing hide/restore behavior for remembered work can remain as designed. Validate the bounded sequence `composer_recovery_failed(A) -> set(Error, other failure) -> dismiss()`; the ordinary failure should remain available to be shown and explicitly dismissed, with its correct non-work provenance. Current tests clear the ordinary state or replace it with completed work before Dismiss, so they do not exercise this case. Static source/control-flow review supports the finding; reviewer did not execute it. Lead owns validation and disposition.

## Saved evidence and source bindings

- `/tmp/brn-draft-recovery-correction-02-old-source-red.*`: parent690 plus test-only diff, exit101, behavioral failure at expected warning visibility after actual completion. Its patch lists only `threads_writing_tests.rs`.
- `-04-background-green.*`: corrected background witness passes.
- `-05-final-draft-focus.*`: 24 passed; `-06-final-notice-focus.*`: 17 passed; `-07-final-format.*`: exit0. Their recorded parent690 plus patch hash independently equals the exact18587c44 correction.
- Broad checks were incomplete at dispatch, but their completed saved receipts became available during review and were read: `-08-final-desktop-suite.*` reports339 passed / 0 failed / 1 existing ignored; `-09-final-clippy.*` exit0; `-10-ordinary-native-build.*` exit0. These pin clean HEAD574 and an empty dirty-patch hash. Relevant desktop compilation/check output points to this checkout.
- The existing `block v0.1.6` future-compatibility warning remains visible. No reviewer rerun is claimed.

Unchanged core chat recovery and atomicity qualification from690 remain applicable; no redundant core build was needed. Green tests do not cover the new dismissal sequence and therefore do not settle it.

## Limits and next action

One bounded complete correction review is finished. Validate/disposition the new P2, make a narrow correction if confirmed, retain both original red witnesses and refresh affected checks/review at a new pin. No broader notice architecture is requested. Lead current metadata/PR183 integration/resulting-main activity is separate from this frozen source: merged CI63345a8e and any CI results do not qualify new draft code. Publication, hosted gates, normal integration, native usability and owner acceptance remain separate. No live provider, private workspace or existing native artifact was opened or changed.
