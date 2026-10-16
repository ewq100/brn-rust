**Verdict for `144848ff` source: CHANGES_REQUIRED**

One new Medium finding blocks approval: the round-1 #4 performance correction regresses on the typing path. Every other prior finding is resolved in source, including the HIGH privacy defect and the archived-evidence bypass.

I reviewed the source read-only. I ran no edits, Cargo, builds, tests, mutation or network calls. The only check I ran was `rustfmt --check` on pinned copies of the files, at each crate's edition.

### New finding

| Severity | Location | Trigger and evidence | Fix |
|---|---|---|---|
| Medium (performance, UI thread) | `threads_ui.rs` `refresh_writing_review_cache` → `review.rs` `writing_context_review`, the `for record in &rows` request loop | **Trigger:** typing in the open Note. `note_changed` calls `update_buffer`, which runs an `UPDATE edit_sessions` on the same connection. That bumps `total_changes`, so the cache key changes on every editor change.<br>**Cost per keystroke:** the next render reloads all records, receipts and candidates. It also validates every writing-request Message in the workspace, for every Note. Each validation runs `writing_request_from_record` (a revision lookup and parse for the Note, each comment and each message, plus `validate_saved_setup`) before the Note mismatch rejects it.<br>**Effect:** cost grows with the whole workspace's writing history on the hottest path. d4 validated only the first matching request in one thread. The "10 notify renders, 0 loads" witness doesn't cover typing. | Filter requests to the displayed Note before canonical verification. Ideally limit them to the context conversation plus the current worker's instruction ID, which are the only consumers. Also stop edit-buffer-only writes from invalidating the cache, using any existing canonical change signal plus `data_version`. Add a native witness for typing renders with many other-Note requests. |

### Prior findings

- **CLI typed errors: resolved.**
  - Provider and effort errors map to `invalid_value`, matching `provider()`/`effort()`.
  - Model and preparation errors go through `run_error`, as in the `3e` baseline.
  - No text matching is used, and desktop `AppError` messages are unchanged.
- **#1 Formatting: resolved.** All changed files pass at their crate editions, including `main.rs` at 2021.
- **#2 Undo: resolved.**
  - Ancestor provenance is now checked only for even-depth compensations (Undo of Undo).
  - Plain Undo restores receipt `before` state and creates nothing.
  - v1 visibility, the hash and the marker are unchanged; replay still comes first.
  - Core and app witnesses cover plain Undo, fenced redo, explicit redo after restoring access, replay and a third Undo.
- **#3 Context choice: resolved.**
  - Repeating the active choice returns `None` before allocation.
  - `open_thread` no longer records a choice; the picker and New still do.
  - The resolver skips archived (undone) selections.
  - The background predicate requires a non-empty receipt whose writes are all writing-context Messages.
- **#4 Repeated scans: partially resolved.** Batching, fail-closed clearing, before/after token equality and Apply's fresh checks are correct. The new finding above remains.
- **#5 HIGH privacy: resolved.**
  - AI reads, search, review and the core gate now give selected-conversation access to `snapshot.note.id` only.
  - Every other Note uses the canonical scope, and `None` maps to the strict general index scope.
  - The `read_records` rewrite is equivalent to `records_for_chat`, and reserved roles stay excluded.
- **#6 Cleanup: resolved.** The constant is used, the dead parameter is gone, and the safe `Invalid` refusal comes before allocation.
- **Archived-evidence replacement: resolved.**
  - Replacement now requires identical data, archived flag and `expected_version`; ID remapping is still allowed.
  - The witness is a direct `Store::replace_candidate` call that leaves every state unchanged.

Library and Rig reuse and the single production route continue. Test changes only add assertions, and the `aea4830f` widget calls are untouched.

**Non-blocking, pre-existing (Low):** `review_replace` accepts any operation ID. Replacing a pending, fenced Undo of Undo creates a fresh operation with no Undo ancestry, so its Apply skips the gate. Only an owner host can do this, with an explicit operation ID, and the desktop doesn't surface pending compensations. A later hardening could refuse replacement of compensation operations.

**Pending verification, not source defects:** exact-pin native rerun, normal/feature/lint runs, the remaining mutation budget, hosted CI, F20/F21, the `pdftoppm` gap and widget-B attribution.

**Re-review needed:** only the cache/request-batch fix and its typing-path witness.
