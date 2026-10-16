I'm requesting changes to the reviewed source (`56ca7c93`, tree `40925b3f`). I read only the immutable `c8b3026f..56ca7c93` diff and the unchanged code it depends on. I ran no Cargo, builds or tests, made no edits, and did not review the other worktree's diagnostic WIP.

## Findings

**1. HIGH, defect: setting up writing in an ordinary conversation makes that conversation private, and Undo does not reverse it.**
- **Where:**
  - `crates/brn-threads-core/src/writing.rs:48-63`: `writing_setup_scope` accepts any open thread.
  - `writing.rs:328+` (`validate_setup_writes`) accepts that scope.
  - `crates/brn-threads-core/src/context.rs:204`: the new `WritingSetup` clause in `chat_private` ignores archived setups.
- **Trigger:** the owner calls `set_writing_setup(note, Some(ordinary_chat))`, where the chat is neither in a project nor already private.
- **Impact:**
  - **Conversation turns private:** the AI source-chat path (`brn-ai/src/threads.rs` about 2257) starts treating the chat as private.
  - **Delegated maintenance breaks:** non-local maintenance writes in that chat are now refused (`store.rs` `apply_on`, about 1597-1620). The requirements say unrelated delegated maintenance must be preserved.
  - **Request scope changes:** `writing_request_scope` moves from general to the thread. Earlier v2 requests in that chat are hidden, and the general setup stops applying.
  - **Search disagrees:** owner search (`reader::message_scope`/`message_scope_rows`) still classifies the chat as General because it doesn't load setup rows.
  - **Undo doesn't restore it:** the chat stays private after Undo because the clause also matches archived setups.
  - **API trap:** `writing_setup(note, Some(general_chat))` returns a thread scope that capture never uses.
- **Why it conflicts with the contract:** master plan §7.2 says scope is the existing scope that `context::visible` already evaluates, not a new scope created by setup.
- **Fix:** resolve thread scope only for chats already private under the pre-CW1 rule (`private_context` or the chat's own Notes); otherwise refuse or use general. Remove the `WritingSetup` clause from `chat_private`. Correct `threads-behavior.md:15` to match. Add failing-before tests on an ordinary chat:
  - maintenance and source-chat outcomes are unchanged;
  - search and context classify the chat the same way;
  - Undo leaves no lasting change.
- **If owner-created private scopes are actually wanted,** that is a product decision needing owner authority, not CW1.

**2. MEDIUM, defect plus missing test: writing in a project chat with BRN knowledge off now fails.**
- **Where:** `crates/brn-threads-app/src/writing.rs:48,65` (`record_in_chat(.., Some(project_id))`) and the working-Note `usable` check in `validate_setup_writes`.
- **Trigger:**
  - The project has `use_brn_knowledge=false`.
  - The working Note is a general Note linked to the project chat (`link_writing_thread`), not a project document.
  - `document_allowed`'s link exception needs `Link.from == chat`, but the check passes the Project ID rather than the thread.
- **Impact:**
  - The setup read says the working Note is unavailable, and both default-setup creation and capture are refused.
  - Before CW1 this capture worked, and AI `begin` still accepts the Note (`record_visible_in_chat(note, thread)`).
  - Privacy is not affected; this is purely a lost capability.
- **Fix:** check the working Note against the request conversation (the thread) while keeping setup identity keyed by the project. Keep reference checks at project scope. Add a test. Alternatively, the principal can explicitly accept and document the narrowing.

**3. LOW, missing test or intent check: v2 requests and setups disappear in Resolved conversations.**
- **Where:** `writing.rs:59-63`. `writing_setup_scope` errors for any non-Open thread, and `context::visible` relies on it.
- **Trigger:** a conversation is Resolved (legacy) or otherwise not Open.
- **Impact:** v2 requests and setups vanish from `messages()`, `message_page` and `records_for_chat`, while v1 requests in the same conversation still show.
- **Fix:** confirm this is intended and add a test, or make visibility tolerate non-Open threads.

**4. LOW, performance.**
- **Where:** the Message branch of `context::visible`.
- **Trigger:** each v2 request in the same thread or project is visited.
- **Impact:** each one is parsed twice (up to 128 KiB each) and scope is re-resolved against all rows; `message_page` repeats this per page. The role filter keeps the cost to same-scope v2 requests.
- **Fix:** resolve scope once per call and parse once.

**5. LOW, notes (no correction required).**
- **Undo of a capture can go stale:** capture bundles creating or un-archiving the default setup. Undo of that capture becomes stale if the setup was edited afterwards.
- **Concurrent first-create:** two simultaneous first-creates hit a unique-index constraint error instead of a clean "changed; reload".
- **Capture authority is narrower:** capture now runs with chat-bound authority. This is consistent with AI `begin`, but it is a behavior change.

## Verified with no finding
- **Narrowing the setup check to Note proposals is correct and not exploitable:**
  - `writing_markers_on` adds the setup Current input only when the request writes `snapshot.note`.
  - `replace_candidate` derives the required markers from the original request, so a replacement cannot drop or downgrade them.
  - AI `writing_request`/`apply` rebuild the writes and inputs through `change_request`.
  - Addressed confirmation (message-only) is correctly exempt.
- **v1 compatibility:** v1 payloads stay byte-identical and are tied to v1 (the `(1,false)`/`(2,true)` pairing). `from_message` still requires version 1, and a fixed-hash v1 test exists.
- **Reference handling:** no reference content is copied. Capture records reference IDs, exact versions and purposes only, and setup edits make v2 proposals stale.
- **Exposure checks:** AI `read_records`, `chat_messages`, `parse_valid`, `check_scope` and `read_history` all exclude or reject setups and v2 requests. Export covers general Notes only. Owner search indexes only owner and assistant messages.
- **Schema 7:** the partial unique index uses COALESCE on chat. The upgrade backs up first and the CLI refuses schemas 3..=6. Tests cover rollback after a failure post-backup, reopen and restore.
- **Desktop:** exactly three one-line arms; `threads_lists.rs` is unchanged.
- **Comments:** semantics are unchanged.

## Missing tests (besides the findings)
- No test shows an older schema-6 binary refusing schema 7; only a schema-5 test exists.
- No test covers the ordinary-chat side effects in Finding 1.

## Pending gates (not counted as defects)
- The full default lane rerun after the addressed-confirmation fix.
- The AI writing tests, which predate the new ordinary-history assertion.
- Strict Clippy, capability and offline CLI journeys on the final tree.
- Native lanes, with the baseline disclosures attributed separately.
- Targeted mutation testing.
- A committed final evidence summary. The logs in `/private/tmp` are mutable and I did not inspect them.
- Principal re-review of the corrections.

I make no claim about publication, main merge, native observation or owner acceptance.
