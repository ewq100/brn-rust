# Passage-comment draft audit

## Scope and result

Read-only audit of `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust` at clean HEAD `a77b2edece4fb91ec6303f829d9ed8792b9adf95` (`codex/knowledge-check-refresh`). No source changes, builds, tests, native actions, account/network access, credentials, or private data were used. Only this report was written.

No source-supported draft loss, wrong-note attachment, duplicate posting, or guessed re-anchoring defect was found. Passage drafts belong to the open window and are keyed by note ID; their body, exact `(note, version, byte range, quote)` anchor, and retry operation are captured/restored together. A changed saved note makes the old anchor stale and the checked writer refuses it. The user must select the saved passage again; that changed anchor clears the old operation key. A successful post clears the in-memory draft before refreshing comments. Unsent passage drafts have no restart guarantee.

## Source trace

- `crates/brn-writing-ui/src/native.rs:31-36,143-148` defines `CommentDraft { anchor, body, operation }`, with the active draft map in `WritingState`; nothing there persists it across restart.
- `crates/brn-desktop/src/threads_ui.rs:1155-1181` saves and restores all three values under the open note ID. `open_note_in_context` preserves before switching, opens the requested note, restores that note's draft, then refreshes (`:1219-1240,1250`). This is the A→B→A ownership route.
- The body editor's real change event clears the active operation key (`threads_ui.rs:569-577`), while programmatic `EditorState::set_value` suppresses `InputEvent::Change` (pinned `gpui-base-0.7.3/src/input/editor/mod.rs:895-905`). Thus navigation restoration does not accidentally discard the captured retry key, but user-edited body text gets a fresh key.
- `comment_on_selection` refuses an unsaved edit; it captures the currently open saved version and exact selected UTF-8 slice. A different anchor/version clears the prior operation (`threads_ui.rs:1301-1335`).
- `post_comment` refuses if the wrong note is open or an edit session is active. It sends the captured version/range/quote plus the current body to `add_writing_comment`; an error retains the anchor/body/key, then runs the normal context adoption, draft preservation and refresh path (`threads_ui.rs:1677-1730`). Success clears anchor/key/body before that refresh (`:1713-1729`). `refresh` reloads records/history and does not overwrite these draft fields (`:805+`).
- The checked core binds a stable operation key to the serialized request, replays an existing operation, verifies the current note version and open linked thread, then requires an exact nonempty range/quote before writing the comment/link/context (`crates/brn-threads-app/src/writing.rs:500-540`; binding/replay at `:360-395`; version check at `:400-413`). A stale version returns “The saved note changed. Refresh the saved passage before continuing.” No stale comment is committed.

## Reproducible source sequence

1. Open Note A, select a nonempty exact saved passage, enter body `A passage draft`, and navigate to Note B. `preserve_comment_draft` stores A's anchor/body/operation by A ID; opening B restores only B's entry.
2. Return to A. The source path restores A's exact draft; existing UI coverage asserts its text and range survive A→B→A. If A acquired a newer canonical version meanwhile, the restored anchor remains bound to the old version—no code silently maps it to the new text.
3. Posting that stale anchor reaches `add_writing_comment`, whose `writing_note(id, Some(version))` rejects it. `post_comment` leaves the draft intact and displays the error. Reopening/refreshing A alone still cannot make the old anchor current. Selecting the passage from the newly opened saved text produces a new versioned anchor and clears the old operation; posting then validates and creates one comment.
4. After success, active draft state becomes empty before the refresh. Later switching away/back restores the empty A draft; the persisted comment remains canonical and is shown from `writing_comments`.

This is a source-derived scenario, not a claim that it was executed in this audit.

## Existing witnesses and gap

- `threads_writing_tests.rs:607-660`, `note_and_selected_chat_drafts_and_dirty_buffers_survive_navigation`, exercises A→B→A and confirms A's anchor/body and independent chat draft return; it also verifies a rejected post with an unsaved edit preserves the draft. It does not assert the operation key.
- `threads_document_tests.rs:348-410`, `stale_save_and_comment_keeps_typed_source_and_comment_draft`, covers the Save-and-comment stale-edit refusal and preservation, not a stale `add_writing_comment` attempt.
- `threads_writing_tests.rs:720-741`, `stale_comment_and_saved_rewrite_preserve_feedback_with_actionable_recovery`, changes A's canonical version behind the open view and retains comment input/anchor while a rewrite is refused; it does not call `post_comment`.
- `threads_document_tests.rs:118-184`, `read_select_all_comments_the_exact_markdown_source_without_starting_ai`, covers successful exact-range posting and canonical quote/range. `threads_writing_tests.rs:6-110` covers successful comment addition, preservation of unrelated chat drafts, reopen behavior and no AI run.
- `brn-threads-app/tests/writing.rs:121-133,215-245` covers rejection of malformed UTF-8 range and exact-operation replay across reopen. It does not compose stale-version posting with the UI draft lifecycle.
- The existing `composer-drafts-audit.md` is specifically about ordinary chat composer destination/body recovery and attachments. It is useful context for chat separation but does not assess this Note-local passage anchor/body/operation path; no duplicated chat audit was done here.

There is no demonstrated correctness defect. The only bounded witness gap is one integrated stale-post recovery path: capture A draft, externally advance A, attempt the old post, navigate A→B→A, explicitly reselect current A text, post successfully, then refresh and confirm exactly one canonical comment plus cleared draft. Current coverage proves the component rules separately, and the checked version/operation boundaries make this a low-risk coverage gap rather than evidence of a defect. No reducer, persistence change, or duplicate/mirror test is warranted from this audit alone.

## Accepted contract / relevant docs

`docs/architecture/invariants.md:18` requires comment note/revision/range/quote provenance and forbids guessed attachment. `docs/architecture/threads-behavior.md:41` says passage drafts get window-local per-note navigation preservation, adding a comment remains a separate replayable action, and unsent drafts have no restart guarantee. `threads-behavior.md:92-96` preserves original comment context and permits only reliable mappings. The canonical target says comments bind note/revision/range/original quote and unresolved locations stay honest (`docs/architecture/threads-target.md:413`); Save/comment commits remain checked and posting is separate (`:421,425`).
