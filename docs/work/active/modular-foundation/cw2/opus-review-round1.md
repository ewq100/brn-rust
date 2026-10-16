**Verdict for d4a51409 source: CHANGES_REQUIRED**

The privacy, Apply-gate, evidence and shared-preparation core is sound. Four Medium-or-higher source issues block approval. I made no edits and ran no Cargo, build, test, mutation or network commands. The only check I ran was `rustfmt --check` on archived copies of the files.

### Acknowledged defect, tracked separately
- **CLI structured error loss (`crates/brn-threads-app/src/run.rs`, `prepare_thread_run`):** it wraps `AiError` as `AppError::Invalid`, so the `tool_rejected` detail is lost. This is unresolved at d4. The fix is in the post-pin commit f282f7f6, which I did not review.
- Also check this in the follow-up review: invalid provider and effort settings used to produce `CliError::invalid_value("provider"/"effort")`. They now go through the shared preparation, so the CLI error codes may have changed.

### Findings at d4a51409

| # | Severity | Location | Problem and evidence | Correction |
|---|---|---|---|---|
| 1 | Medium (blocks CI) | `crates/brn/src/main.rs` imports, lines 9–21 | `brn` is an edition-2021 crate, but its imports were rewritten in 2024 style (`{Value, json}`, `sync::{Arc, atomic::…}`). `rustfmt 1.98.1 --check --edition 2021` fails at the pin and at f282f7f6 but passes at baseline. CI runs `cargo fmt --all -- --check` (`ci.yml:64`). | Run `cargo fmt -p brn`. |
| 2 | Medium–High (Undo contract and existing v1 data) | `crates/brn-threads-core/src/store.rs` `apply_on`, the loop walking `undo_of` ancestors (~1430–1535); test `cw2_visibility.rs::access_loss_fences_undo_…` | The gate checks the original candidate's inputs on every compensation. After a reference is revoked, a plain Undo is refused with `NeedsReview`, even though it only restores the pre-AI text and releases nothing. `writing_markers_on` matches any writing-request Message, so existing v1 receipts change behaviour too. This conflicts with F12 "safe compensation" and invariant 11. The handoff states it as design, but the spec only gates re-applying candidate content. | Gate only compensations that re-apply candidate content (Undo-of-Undo, by depth parity). Leave plain Undo ungated and update the test. Otherwise the owner must explicitly accept this behaviour. |
| 3 | Medium (F19 UX and data growth) | `crates/brn-desktop/src/threads_ui.rs` `open_thread` (~1378) → `select_writing_thread` (~1903) | Each time a linked chat is opened while its Note is open, a fresh UUID key commits a durable receipt plus a `writing-context` Message. There are about 19 callers, including search reveal, refresh follow-up and seeding. The Changes view fills with receipts that have Undo buttons and no titles. `active_writing_context` scans grow with each one, and Undo does not revert the selection. | Make it a no-op when the chat is already the active conversation. Record only on explicit picker or New actions, or mark these receipts as background. |
| 4 | Medium (performance) | `threads_workbench.rs` `writing_proposals`, `latest_note_writing_request`, `writing_run_outcome`, `writing_bar_row`, `writing_result_note`; `threads_writing.rs` `note_bar`, `review_body`, `compare_tile`; `context.rs` `revision_visible_in_chat` | Each render runs `writing_candidate_review` and `active_writing_context` per item. Each of those calls `records()` in full and scans all receipts, and the wider-Note check costs about O(projects × records). During streaming the Note tile re-renders often. AI `read_records` also calls the gate for each listed record, which is O(N²). | Compute once per refresh, Note open or scope change, and cache in UI state. Use indexed or single-load checks. Keep the app gate authoritative. |
| 5 | Low | `brn-ai/src/threads.rs` `writing_read_allowed` (~945) and the core gate; `threads_search.rs` `host()` | The selected-chat scope is applied to every non-reference input, though the comment says it is for "the working Note alone". Search falls back to the chat scope when the canonical scope is None. `chat_private()` mostly makes these equivalent, but a Note created during a run in a general-scope context is an edge case. | Limit the exception to the working Note, or document why they are equivalent. |
| 6 | Low | `writing_request` (literal reason string), `ensure_writing_thread` (dead `version` parameter), `attach_writing_evidence` | The reason is a duplicated literal instead of `WRITING_PUBLIC_REASON`. An unavailable must-current or exhaustive reference leads to an opaque prepare rejection with no reason shown to the owner. | Use the constant, remove the parameter, and surface a typed refusal. |

### Verified with no finding
- **Evidence handling:**
  - Evidence role and strict validation are in place, with request, thread and version binding.
  - The 128 KiB limit and the quote, claim and size caps are enforced.
  - Replacement retention cannot drop evidence or inputs.
  - Models cannot forge reserved roles.
  - Parse failures fail closed and are never classed as legacy.
- **Reading proof:** quotes are checked byte-for-byte against the ledger. Top-k search snippets record the version only, with 0 bytes covered.
- **Apply gate:** it runs inside the transaction using the canonical setup scope. Access loss refuses Apply, and restoring access still needs a fresh explicit Apply. Replay comes first, and backup/restore and reopen are tested.
- **Hiding reserved data:** reserved messages are kept out of history, transcripts and search (allowlist). Public reasons stay generic.
- **Desktop:**
  - Safe "unavailable" states appear early in Review and Compare.
  - The disclosure sits outside the Inputs toggle.
  - `open_review` no longer overwrites the chat.
  - Apply is context-aware.
  - Native test changes are additions only.
- **Shared preparation:** CLI and desktop use one preparation path. Selection errors stop before any write. Duplicated routes were removed, no new dependencies were added, and reuse of the AI layer and Rig is appropriate.

### Not covered by this approval
- **Pending verification, not source defects:** final committed-source core/AI/app/CLI, feature and combined runs; the mutation run (1800 s cap); hosted CI; F20 native observation; full F21. Old open items stay open: the `pdftoppm` environment limit and widget-B attribution.
- **Outside CW2's allocation:** generic `open_note` paths, such as Note links inside project chats, don't pass the scope they were opened from.

### Re-review
A fresh round on a new pin should cover:
- the CLI fix;
- items 1–4, including the updated Undo test or the owner's acceptance of the current behaviour;
- the affected core, app, native and CLI checks, plus `cargo fmt --check`, before mutation.
