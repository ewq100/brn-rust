# Independent queued-edit destination review

## Verdict and reviewed identity

**Source-review approval: no actionable consequential findings.** This approves the bounded implementation against its selected outcome; it does not establish passing builds/tests, native observation, owner acceptance, merge or release.

- Reviewed checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`.
- Branch: `codex/queued-edit-destination-owner`.
- Candidate: `f79f6540276c00155ab3bc0d7f019f4ea5d00c9e`.
- Task base and verified merge base: `dee96c9ed4542eb3c8b595280116628580dc896a`.
- The candidate checkout was clean at entry and at the final source inspection.
- Reviewed the complete base-to-candidate diff: five files, 121 insertions and 10 deletions, including the regression introduced by test-only source `97fa29c21480cd995f4aa96f1b7c1c04acf48d84`.

The review applied the repository guide, architecture/invariants/behavior and verification guidance, plus the independent-candidate and Rust review skills. It used the supplied design and regression evidence from the lead checkout and `/tmp/brn-queued-edit-destination-implementation.md`. Those handoffs describe the old-source red observation; this reviewer did not execute or independently reproduce that test run.

## Requirements and correctness assessment

1. **Captured edit identity and one resolver.** `threads_ui.rs:357` adds private `QueuedEdit { item, thread }`, and `threads_ui.rs:1057` resolves it only for the displayed `ChatTarget::Thread` matching the captured original thread. The actual queue Edit callback in `threads_chat.rs:607` captures both values from the selected row. No identity is inferred from the subsequently displayed chat. New/Home and new-note composers do not resolve an unrelated held edit.

2. **Admission, display and mutation agree.** `ask` in `threads_ui.rs:2122`, the composer label/tooltip in `threads_writing.rs:698` and `threads_writing.rs:946`, and `enqueue_composer` in `threads_chat.rs:260` all use that resolver. Keyboard Enter and the root Ask action continue to call the same `ask` method as the actual composer button. Rendering borrows the captured ID without allocation; the enqueue mutation boundary clones the resolved item intentionally to end the immutable borrow before mutable workspace calls.

3. **Independent sends preserve the original edit.** Navigation still passes through `set_chat` (`threads_ui.rs:1072`), which saves the departing destination's exact composer text and restores the arriving destination's draft. An independent queued send clears only its own captured destination and retains the held edit because `editing_queue` is now cleared only on successful edited sends (`threads_chat.rs:281`). The idle direct-send route also leaves the held edit untouched. Returning to the original chat resolves the original edit again, so Save queued updates its captured item rather than adding a duplicate. Explicitly choosing another queue Edit still replaces the single current edit intent, as selected; this does not introduce a multi-edit product.

4. **Refusal retains checked authority and input.** The original-chat resolver deliberately does not check whether the item is still in a cached queue. It sends the captured item ID to unchanged `Workspace::edit_queued` (`crates/brn-threads-app/src/chat.rs:109`), which uses unchanged checked `ChatCommand::Edit` (`crates/brn-threads-core/src/chat.rs:218`). A missing/removed item returns Missing. A pending item whose associated Run is Working is refused. On either error the desktop preserves the composer text and edit intent, reports the refusal, and neither clears the draft nor enqueues a replacement. There is no fallback or retargeting branch.

   Precise existing policy: not every pending/claimed item is refused. A pending item without a Working bound Run remains editable under the existing core behavior; that edit clears its old message/run/dispatch association. This candidate does not alter that policy, and this review makes no stronger all-pending refusal claim.

5. **Guards and ownership remain intact.** No service/core/store/schema/provider/runtime/dependency/configuration changes occur. Existing busy and paused-queue admission predicates, dispatch checks, Run/lease/claim identity, stop/completion handling, destination-keyed recovery generations, and writing dirty/recovery guards remain unchanged. UI-local captured edit intent does not become canonical authority. No new asynchronous mutation, unsafe code, public API or silent error handling is introduced.

6. **Contract matches the bounded behavior.** The changed sentence in `docs/architecture/threads-behavior.md:70` describes original-chat binding, independent sends and draft restoration. It does not promise persistent edit sessions across restart or broaden queue authority.

## Regression assessment

`threads_chat_tests.rs:1378` uses fresh temporary synthetic data, creates the A waiting item and paused queue, records a synthetic Working Run, and holds the local worker's sender and liveness lease throughout the send sequence. The fixture disables its polling task, and the queue remains paused while the local worker is held. No production provider worker is launched by this witness.

The regression clicks the real `queue-edit-0` control and real `ask-delegate` composer action. It navigates through the real `open_thread` method, checks A's exact retained draft, then verifies that B's send leaves A's original text unchanged and appends B's own destination/text. It checks the paused state, held worker and Working Run. Returning to A restores the edit text; the second actual composer click updates A with B unchanged and exactly two items. These assertions expose the selected cross-destination overwrite and a correction that simply drops edit intent or creates a duplicate on return. It is a meaningful event-wiring/state regression, not a direct call to the resolver with expectations computed from production logic.

The existing queue-notices test changes only its direct edit-mode seed to construct the new captured type; its behavior assertions remain intact. The candidate regression does not exercise native typing, focus, IME or OS interaction, and does not itself execute the unchanged missing/Working-item refusal branches; those branches were traced through the actual unchanged service/core implementation in this source review.

## Checks and remaining gates

This reviewer performed read-only Git/source/document inspection and wrote this review report only. **No builds, test runs, network, provider/account actions, native interaction, nested agents or source edits were performed.** The author's reported formatting/diff checks and the lead's reported red observation are attributed evidence, not checks performed by this reviewer.

Lead-owned focused red-to-green and relevant broader desktop checks remain separate qualification evidence. Required integration checks, resulting-main verification, native observation and owner acceptance must retain their actual statuses. No extra source correction or test is requested solely to re-prove unchanged core semantics.
