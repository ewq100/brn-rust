# Writing workbench polish

**Owner assignment, 14 October 2026:** “I want comments beside the text and the chat in its own panel. lets make this.” This follows the owner's explicit choice to keep multiple passage comments and agent rewriting with all captured context, and to improve readability, rich text/formatting and smoothness.

## Accepted outcomes

- The document and passage feedback share the Note tile; the full linked chat stays in its separate resizable Chat tile. Comment cards never move into chat or below the note.
- Readable headings, emphasis, lists, tables and figures; a compact formatting toolbar and quieter comment controls. Preserve the existing multi-comment, full-context, proposal-only rewrite and Apply/Undo lifecycle.
- Preserve typing, composer/comment drafts, exact range/version/quote anchors, protection, candidate identity and current-state checks through navigation, save, review and recovery.

## Concrete implementation within the pinned toolkit

Keep one authoritative Markdown note buffer. Provide a formatted **Read** view of that buffer alongside **Write** with selection-based formatting actions. Comment selection in Read uses the toolkit's exact source mapping; unmapped selections cannot create guessed anchors. True WYSIWYG editing is unavailable in GPUI Kit 0.7.3 and is not claimed by this implementation. Write remains available for editing and existing keyboard journeys.

Feedback is reachable beside both views and has a clear pending count, quote, inclusion and passage reveal. Normal Save remains explicit; a **Save and comment** continuation may preserve/revalidate the exact selected range and quote, and proceeds only when Save committed the exact current buffer without a stale/generation conflict.

Save and comment saves the human buffer and opens the existing comment composer; it does not post a comment. The subsequent Add comment remains a separate replayable operation. A failed Add preserves the draft, anchor and operation identity; a saved human edit stays saved. Comment drafts retain existing window-local navigation preservation; restart durability is not newly promised. Recompute Read from the current editor buffer after edit/recovery/Save/Apply; invalidate previous selections whenever that source changes and verify parser source equality before accepting a rendered selection.

Read/Write and formatting operate only on the human note. During displayed candidate review, disable these controls and use the existing Edit final text path for candidate changes. Keeping a candidate for later permits human editing; Save preserves its immutable identity, and stale Apply refuses until a refreshed candidate has been displayed and reviewed.

Rewrite captures the saved note, included pending comments and all posted selected-chat messages. Read/Write switches do not save, submit drafts, start inference or grant write authority. The user can inspect the snapshot. Comments remain pending until actual Apply and explicit addressed confirmation. Local Undo retains the original contract.

The existing 128 KiB rewrite-context bound remains; too-large input fails explicitly without silent truncation.

## Qualification boundaries

Use Rust 1.98.1, GPUI Kit 0.7.3 and the existing lockfile. No framework, editor fork, canonical store/schema, provider configuration, global permissions or runtime behavior changes. Synthetic data only. Native interaction is in scope for this visible implementation on the available Mac; provider output quality, real data, release and merge remain separate.

## Owner correction after native preview

The owner wants the complete writing flow inside the writing tile: passage comments, overall feedback, rewrite progress/Stop/retry, proposal review, Apply and Undo. A reply/question from the exact terminal writing run is readable there when no proposal is pending; new overall feedback answers it through a new Rewrite. Chat remains an independent optional panel; writing requests and writing proposal/continuation controls no longer occupy its conversation. Existing posted linked-chat context remains captured without requiring new chat interaction.

Overall feedback is a per-note window draft, captured atomically into the immutable rewrite request when Rewrite is pressed, with no separate chat Send. Snapshot persistence makes retries/restart use exactly that feedback; newer drafts affect only a new rewrite. Empty feedback preserves legacy requests. The shared service binds feedback to replay identity and the existing 128 KiB bound. No database schema or authority changes.

Comments become quieter: body first, muted clickable passage quote, one inclusion toggle; only exceptional unlocated/excluded/addressed states need extra status. Exact anchors, explicit addressed confirmation and Undo remain unchanged.
