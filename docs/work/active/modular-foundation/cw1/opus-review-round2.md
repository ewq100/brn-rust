**Verdict: APPROVED** as a source review of the immutable candidate `690fb705` (tree `af0e8c82`). This is not slice acceptance: the final default, native and combined lanes and a fresh mutation run are still pending, and I did not treat them as passed.

I read only `git show`/`git diff` output for `c8b3026f..690fb705` and `56ca..690fb705`, plus the relevant unchanged code that writes, replays, undoes and scopes records. I ran no Cargo, builds or tests and made no edits. I did not delegate, and I skipped the code-review skill because it uses sub-agents.

## Prior findings
1. **Fixed (HIGH).** An ordinary chat now resolves to the general scope.
   - **Privacy:** the `WritingSetup` clause is removed from `chat_private` (`context.rs:232-237`). The resolver (`writing.rs:47-61`) uses only the pre-CW1 privacy rule.
   - **Non-canonical writes:** a raw write with `chat = Some(ordinary chat)` is refused at `writing.rs:387`.
   - **Witnesses:** the app and AI tests fail on `56ca` (logs committed) and cover maintenance, source and search parity and Undo.
2. **Fixed (MED).**
   - **Working Note:** checked against the host-bound conversation or a single Thread input, which must resolve to the same canonical scope (`writing.rs:350-411`).
   - **References:** stay strict to the Project scope (`:415`); the core and app tests prove a conversation-only reference is refused.
   - **No new access:** I found no model-driven grant, automatic sharing or widening.
3. **Fixed (LOW).** Reading and inspection accept inactive conversations; mutations go through `require_active_writing_scope` and `writing_thread`. A test covers three states.
4. **Fixed (LOW).** Each valid v2 envelope is parsed once, with the scope resolved once per read through `OnceCell`. No new framework was added.

## Material corrections I checked
- **Undo and ChatContext** (`store.rs:663-685`): Undo copies the original operation's Thread inputs at their original versions, so the compensation hash stays stable. The deferred-Undo-after-restart witness fails on the intermediate correction and passes on the candidate.
- **Fail-closed after access changes:**
  - A project move bumps `context_epoch` (`projects.rs:478`), so Undo goes stale.
  - A removed link or knowledge-off change is caught by the check against current state.
  - No stale grant is revived and private context is not expanded.
- **`messages` via `message_page`:** apply is the only writer of revisions, so ordering by first revision rowid matches receipt order. Cursors, lookahead and v1 visibility are preserved.

## Non-blocking residuals (all LOW)
- **R1, Undo depends on the conversation even when it isn't needed** (`store.rs:663-685`, `writing.rs:393-398`):
  - Thread context is kept even for archive-only compensations (Undo of the first capture or of a setup creation) and for general or private scopes.
  - Undo is then refused if that conversation is later archived, Resolved, moved, or gains a private Note. This narrows Undo of the existing capture operation.
  - It fails closed with no privacy impact.
  - Fix: add the inputs only when a restored, non-archived setup has a Project scope.
- **R2, no direct core test for the original defect:** nothing refuses a raw `chat: Some(ordinary chat)` write and asserts the chat stays non-private. The AI test derives its scope from the code under test, so it wouldn't catch the `chat_private` clause coming back.
- **R3, no Undo test after a project move or link removal.** I verified the fail-closed behaviour by reading the code only.
- **R4, trashed projects:** in a trashed (archived) project, `message_page` hides v2 requests but shows v1 and ordinary messages (`context.rs:72`). They reappear when the project is restored.
- **R5, performance:** `Workspace::messages` (`lib.rs:285-301`) reloads `records()` on each 100-row page that contains a v2 request, and every rewrite adds one.
- **R6, replay brittleness** (`app writing.rs:116`): `set_writing_setup` binds the Thread at its exact version. A thread change from another process between prepare and apply leaves that key permanently stale. This matches the existing capture convention.
- **R7, schema-6 receipt:**
  - It has no file hashes taken after the refusals; the core tests cover byte-unchanged refusal instead.
  - It was built at `56ca`; the upgrade, backup and restore code is unchanged through `690fb705`.
  - The old binary refuses schema 7 with `unsafe_directory`.

## Gates still pending
- Default, native and combined lanes on `690fb705`.
- A fresh targeted mutation run.
- Strict Clippy and fmt, and the capability and CLI journeys on the final tree.
- Principal reconciliation.
- Known baseline issues remain disclosed separately: the PDF failure (missing `pdftoppm`) and the native widget failures, Stop Working vs Interrupted intermittency, and B1a.

This review makes no claim about native observation, owner acceptance, integration, publication or merge.
