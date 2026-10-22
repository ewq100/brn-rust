# Queued edit destination implementation handoff

Branch: `codex/queued-edit-destination-owner`
Test-only red base: `97fa29c21480cd995f4aa96f1b7c1c04acf48d84`
Implementation commit: `f79f6540276c00155ab3bc0d7f019f4ea5d00c9e`

The edit intent now carries the captured queued item ID and originating thread ID. A single current-chat resolver gates Ask admission, composer label/tooltip and enqueue's edit branch. The actual queue Edit control captures both values. An edit in its original chat still uses the existing checked `edit_queued` operation and clears the edit after success; an independent send from another chat leaves the captured intent and its destination-keyed draft intact. Returning to the original chat restores Save queued. A missing/claimed original item still reaches the checked edit operation and preserves the current text on refusal; there is no retarget or fallback.

The actual-control synthetic regression remains unchanged. The existing notice test's direct edit-mode seed now constructs the captured `QueuedEdit` from the selected row without weakening assertions. The desktop behavior contract states the original-chat binding. Changed files are limited to `threads_ui.rs`, `threads_chat.rs`, `threads_writing.rs`, `threads_chat_tests.rs`, and `docs/architecture/threads-behavior.md`.

No build or test was run by this author; lead owns focused red-to-green and broader desktop checks. Pinned rustfmt and `git diff --check` passed. No provider, network, account, native, store, CLI, runtime prompt, dependency or configuration operation was performed.
