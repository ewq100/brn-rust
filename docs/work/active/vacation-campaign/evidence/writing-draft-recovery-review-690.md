# Independent writing draft recovery review

Verdict: **one P2 UI failure-lifecycle finding**. No additional Store transaction, generation, target occupancy or scope finding was identified. Candidate is implemented and has passing saved checks, but the finding needs validation/disposition before qualification is treated as settled.

## Exact identity and scope

- Worker checkout `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/writing-draft-recovery`.
- Frozen, clean candidate/observed HEAD `690c2c3f31a0f11e3405f2f2d0479be024190eb8`.
- Actual base and independently verified merge base `bc8a08e64b0dc1e2b33ff181dfdb54b62f59f5e4`.
- Complete source diff: 11 paths, 546 additions / 38 deletions. Binary diff SHA-256 independently computed as `5b9f6e2415776e9f78c2c6de180a38f6e57d1c3db4ffdac409eccb75523eec56`.
- Independently verified all 11 file hashes and all 13 saved run-log hashes against `/tmp/brn-draft-recovery-impl-qualification.json`. Final worker checkout remained clean.
- Supporting lead records inspected from PR183 publication `1e6abb60` through lead HEAD `cda207215b7f83cac54e98efdff58558dccc69ce`, plus reported lead-owned dirty campaign/overview updates and new `evidence/pr182-resulting-main.json` at observation12:14. These are not represented as one frozen composed source commit.

Applied previously loaded reviewing-product-candidates/resource guidance; read current AGENTS and relevant invariants, behavior, workflow/verification and strategic test/responsibility guidance. Reviewed all changed source/test/README paths, complete affected functions, initialization/autosave/navigation, both adoption callers, every changed Rewrite caller, shared Store draft persistence and existing contextual-edit/send boundaries. No implementation was authored by this reviewer. No builds, test execution, source/Git mutation, account/API/network/native/Page/PR/merge action or delegation occurred. Only this report was written.

## Finding

**[P2] Keep a refused-handoff warning visible when Rewrite's worker finishes.** Primary changed location: `crates/brn-desktop/src/threads_ui.rs:1738–1747`; related failure at `:1101–1103`, completion at `:2807`, `threads_notice.rs:55–60`, and coverage at `threads_writing_tests.rs:211–221`.

Concrete trigger: retain a nonempty pending Note draft, use the explicit New writing conversation route, type into the newly displayed target, then invoke Rewrite with settings that permit its worker to start. The saved request reaches `retry_rewrite`/`start_request` before `adopt_writing_thread` runs. Adoption correctly refuses the occupied target and shows `Composer recovery could not move ... keep this window open or copy your text.` The worker remains active. On its later `WorkerEvent::Finished`, the poller unconditionally calls `notice.complete_work`, whose implementation replaces the current ordinary error with the work banner. The same overwrite occurs for a stale/failed transfer after a started Rewrite.

Thus the recovery handoff is still unresolved, but its failure/copy instruction disappears without resolution or dismissal. The source draft can remain under the old Note destination while normal reopening selects the resolved Thread; in unsaved/failing recovery cases the copy/keep-window instruction is consequential. This does not claim the Store transaction corrupts/deletes the source: it is loss of the required visible failure state, allowing an unrelated completion to imply the workflow is settled.

Moving adoption after synchronous retry avoids immediate startup messages replacing this warning, but not asynchronous completion. The added occupied-target Rewrite test explicitly uses an offline fixture and asserts `view.worker.is_none()`, so it cannot exercise this sequence.

Validate with the existing synthetic worker-completion/poller facility: arrange the refused adoption while a Rewrite worker is active, deliver its Finished event, and assert that exact source/editor/target/generations remain intact and the unresolved recovery warning remains available. Correct only this handoff/Rewrite ordering or warning lifecycle; no broad notice redesign, provider call or native run is needed. Static call/control flow supports this finding; reviewer did not execute a reproduction. Lead owns validation and disposition.

## Requirements and correctness otherwise

The Store API is appropriately placed in existing host chat recovery, with no schema/dependency/canonical Note change. It validates distinct bounded destinations/text, checked integer increments and exact optional source/target generations; absence differs from a generation-zero row. Under an Immediate SQLite writer transaction it rechecks both rows, refuses nonempty target recovery, writes destination text then source empty generation tombstone, and commits together. Errors roll back both writes. Returned committed generations drive the host caches. Tests cover a second Store handle, stale source/target, occupied destination, absent versus zero, delayed source save fencing, unrelated recovery and a trigger-induced source-clear failure after target insertion proving atomic rollback/reopen.

The UI derives source text from the live NewForNote editor or stored Note draft as appropriate; conservatively checks both live target typing and cached target text before Store admission. It preserves source session text on failure. Only successful transfer clears the source cache, advances both cached generations, supplies target cache/editor text and changes the visible Note target to Thread. Displaying another unrelated chat leaves that editor untouched. Displaying the actual empty target restores its editor as well as cache/durable text. These are meaningful distinct paths, not duplicated Store mirrors.

The old memory-only movement is replaced in the two existing first-comment/Rewrite adoption routes. All Rewrite keyboard/launcher/writing-panel/configured and test callers pass Window coherently. Ordinary send, contextual Edit note, provider/runtime prompts and canonical Note mutation paths are unchanged as scoped. Existing contextual-edit handoff remains a separate existing route; this review does not silently qualify or redesign it.

The extended original witness runs the real first-comment handler, requires no AI, immediately constructs a fresh Workspace/view without an intervening composer edit/navigation, reopens the Note and checks exact Unicode/multiline target text plus unchanged Note/unrelated Note/other Thread draft. Separate new witnesses cover stored Note draft with another chat displayed, explicit New conversation with empty/occupied displayed target, and refusal data/generation stability. These establish recovery and correct immediate behavior; the asynchronous warning boundary is the sole gap found.

## Source-bound evidence inspected

The archived old-source patch/reproduction and both old-source logs match all recorded compressed/decompressed hashes and byte counts. Both logs compiled and failed at the expected recovery assertion: actual empty versus exact Unicode/multiline draft. They are meaningful original behavioral failures, not harness failures. Original audit reports remain lossless, with normalized exact-source links documented separately.

Focused final run metadata records basebc8 plus dirty diff hash `5b9f6e24`, independently equal to base→candidate690 bytes. Saved final focused outcomes are 22 desktop draft tests and the same original recovery witness1, both passing; formatter/diff checks pass. While broad checks were incomplete at dispatch, their final saved receipts became available during this read-only review and were inspected:

- Final `chat_storage`: 11 passed / 0 failed, including four new transfer tests.
- Final native-ui/native-test-support desktop suite: 336 passed / 0 failed / 1 existing ignored external shared-intake fixture.
- Strict all-target Clippy: exit0, relevant core/app/desktop checks from this checkout.
- Ordinary `native-ui` production build: exit0, relevant local crates explicitly compiled from this checkout.

These final broad receipts pin clean HEAD690 with an empty dirty-patch hash. The existing `block v0.1.6` future-compatibility warning remains visible. These are inspected author-run evidence, not reviewer executions; none reproduces the newly identified asynchronous notice issue.

Lead-only records reviewed: publication/pending hosted CI183 chronology, audit import correction and lossless originals, bounded general-search/composer audits, supplemental adoption finding, original red qualification, generated/current records and dated remote readback snapshots. The latest supplied PR182 resulting-main receipt lists all four required contexts successful at sourcebc8 while retaining optional Windows failures. PR183 remains hosted-gated and is not described as merged or as qualifying this new source. Current draft-repair records remain implementation/review/qualification work, with native/owner acceptance separate. No additional documentation/evidence finding was identified in the inspected material; final composed metadata will need to reflect this finding and the actual final check observations.

## Limits and next action

The source is still frozen690; any material correction requires affected checks and review refresh. Lead should validate/disposition the P2 using a deterministic injected completion before correction, retain original failure evidence, then pin the corrected candidate. Do not infer OS usability, owner acceptance, provider usefulness, hosted qualification, merge or release from the component/build results. Existing native artifacts/owner data were not opened or changed. This bounded complete review is finished; no further investigation or nested helper is requested.
