# Independent complete-candidate review: Run continuation ownership

Result: **no actionable findings**. The exact candidate is suitable to advance through the lead's remaining qualification and integration gates. Source review does not establish a passing build, passing runtime tests, native usability, owner acceptance, or merge readiness on its own.

## Reviewed identity and inventory

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/run-continuation-owner`
- Candidate: `bd60131dfd8b4652b17c0b142c411bfe87550570`
- Candidate tree: `7f1bf67a27c524ef8aa48f611d1e032713b01d3c`
- Base and verified merge base: `f6dd4d6666320437d631759440b68f2b43d33a9d`
- Worktree: clean at initial and closing identity inspection; no staged, unstaged, or untracked candidate additions.
- Complete actual diff: five files, 149 insertions, 32 deletions.
- SHA256 of `git diff --no-ext-diff --binary BASE CANDIDATE`: `4cd02f0d477a828ce8383cbf1c0bbd40e087c01cda14912162422f1876863642`.

| Changed file | Candidate SHA256 |
|---|---|
| `crates/brn-desktop/src/threads_chat.rs` | `f1738862362d012a80aadef20ea63e1c8b295ba4ae79a0d194fea9d003ff35d2` |
| `crates/brn-desktop/src/threads_projects.rs` | `a15ac7b603624802d6d8ac25fc931ed57347a879ed4874e305ab4386a7cba3c1` |
| `crates/brn-desktop/src/threads_steering_tests.rs` | `e4d9cb1008dd5bd81f00ae2c75187b3ed4d32faf2751841e002e1e466c7a850c` |
| `crates/brn-desktop/src/threads_ui.rs` | `64c5169c2d26fa0fbda6799fb516d53206754836b189dd3052492ec9f434c28a` |
| `crates/brn-desktop/src/threads_worker.rs` | `c4c2c4a758aa9b02c76e0206f1a5525c61ad5c3d6e3fe7a5fead4ca7535c0b30` |

## Scope and authorities

Applied the independent candidate review skill and its resource policy, repository AGENTS guidance, strategic programming guide, architecture/invariants, desktop README, verification guide, and the canonical Steer/queue behavior contract. Read the lead's current `run-continuation-ownership` task record and continuation design in the vacation-hardening checkout. The task packet's explicit correction controls Archive: preserve actual base behavior, so **any successful archive discards continuation; only a matching Run token is canceled**. The older design/task wording about matching archive success is not treated as authority for a behavior change.

Reviewed every actual changed hunk and the relevant complete Worker implementation, UI start/stop/archive/poll/disconnect/close/Quit paths, queue enqueue/dispatch/acknowledgment and failed-worker recovery consumers, project receipt fencing, checked Workspace steering/chat services, and relevant steering, navigation/archive, project interruption, and batch-admission witnesses. No selected-hunk-only review was used.

## Requirements and correctness assessment

- `PendingSend` and the free UI field/initialization are retired. Exactly one `Option<String>` lives on `WorkerOperation::Run` beside the existing single `RunIdentity`. Non-Run variants cannot retain a continuation. The payload contains no copied thread or Run ID.
- Worker remains invocation data/channel admission. Its four small continuation operations do not decide persistence, checked stop, cancellation, queue handling, navigation, authority, or dispatch. A separate reducer, duplicated identity, lifecycle framework, dependency, or configuration route was not introduced. The accessor returning `Option<&str>` supports exact payload inspection while production callers only test presence. No actionable interface-complexity issue was found.
- `can_stop_and_send` still requires an empty continuation and a Run whose thread is displayed. Checked `stop_and_post_with_partial` still precedes arming, composer recovery/UI clearing, and token cancellation. Checked failure leaves text, worker, and cancellation untouched. Arming explicitly rejects duplicate and non-Run use without overwrite or panic; the post-commit host branch supplies the saved-in-chat/manual-Continue warning if arming is unexpectedly unavailable.
- `RunFinished` acknowledgment occurs while both the original Worker identity and continuation remain present. The missing-terminal recovery decision uses the same continuation presence as the base. Extraction occurs only afterward in the Finished handler and before Worker clearing; thread is derived from that original Run identity into a poll-local tuple.
- `Worker::drain` and its terminal classification/cutoff are unchanged. The host handles the full admitted old batch, clears the old worker, and only then starts the extracted saved prompt. Continuation retains precedence over queued dispatch, uses fresh current settings, and carries no inherited protected-edit grant. Trailing old events cannot reach a successor through this poll path.
- Explicit Cancel discards before checked stop/pause and therefore still suppresses automatic launch if that checked operation later fails. Successful Archive discards for both matching and unrelated chats, matching the base; failure retains it. Project commands discard/cancel only after a successful receipt that ends the current Run. The mutable borrow change introduces no conflicting ownership or additional mutation policy.
- Disconnect preserves/interrupts the original Run with visible partial before clearing, then discards without launching. Close and Quit explicitly discard before cancellation. Other production Worker starts retain their existing busy guards, so they cannot silently replace a live Run carrying a continuation.

## Test strength and preserved assertions

No existing behavior assertion was removed or weakened to obtain this candidate. Assertions about a nonexistent free UI slot are coherently replaced with absence of a continuation on the only possible owner; an absent Worker structurally implies no continuation. Existing subsequent worker-presence/absence and behavioral assertions remain.

The new Worker unit witness checks empty state, exact payload retention, duplicate rejection without overwrite, original destination extraction, single consumption, and non-Run rejection. The integrated steering witness still checks the exact Unicode prompt `New exact instruction · õun`, adds direct durable owner-message observation, retains original Run identity, waits before Finished, rejects late old text, preserves viewed chat/composer, keeps queued later drafts, reloads persisted provider settings, and preserves manual Continue. Failure, disconnect, cancellation, partial-answer deduplication, queue pause/reopen, navigation/archive ownership and project interruption assertions remain intact. Existing first-terminal payload, FIFO/backlog, terminal-budget-edge and disconnect-remainder tests are unchanged.

Coverage limitations remain explicit: the new unit witness does not independently prove every discard effect, and the steering test uses offline startup refusal as its dispatch witness rather than a real successor provider invocation. Some preserved edge conditions (e.g. continuation discard followed by a refused Stop and unrelated successful Archive while armed) are established here by direct source comparison rather than a new dedicated runtime witness. These are not new behavior defects or weakened baseline tests; no test was run in this review.

## Allowed actions and readiness

This reviewer changed only `/tmp/brn-run-continuation-review.md`. No source/shared-record/Git changes, tests, builds, native interaction, account/provider/network actions, or nested agents were performed. Identity, inventory, diff/hash, and source inspection are the only executed evidence. The lead owns qualification, findings disposition if subsequent evidence changes the conclusion, prior activity-owner dependency/integration, exact-head required checks, and any normal protected merge. Actual native interaction and owner acceptance remain pending under the existing campaign boundary.
