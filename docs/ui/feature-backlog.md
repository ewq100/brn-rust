# UI feature backlog

Handoff features hidden from the workspace shell until a backend exists for them. Each was intentionally left out of [slice 1](../superpowers/specs/2026-10-01-workspace-shell-design.md) so the shell never presents fixture data as connected behaviour. Source: the BRN UI/UX handoff (`UI-SPEC.md`, `COMPONENT-INVENTORY.md`). The handoff assumes a Pi SDK provider; this repository uses the Codex App Server, so provider-facing items must be mapped to `brn-workflow` capabilities rather than re-implemented in the UI.

| Feature (handoff component) | Planned slice | Prerequisite |
| --- | --- | --- |
| Model and thinking-effort control (ModelEffortControl) | 2 · Chat polish | Workflow exposes provider model and effort capabilities and read-back |
| Context usage, capacity and estimate markers (ContextUsage) | 2 · Chat polish | Provider-reported usage surfaced by workflow |
| Explicit context chips (ContextChip) | 2 · Chat polish | Workflow accepts explicit attachments for a turn |
| Evidence inspection with Return (SourceInspection) | 2 · Chat polish | Existing evidence data; navigation stack design |
| Connection status, reconnect and actionable provider errors | 2 · Chat polish | Provider state surfaced by workflow |
| Notes, folder hierarchy and Archive branch (VaultExplorer) | 3 · Vault rail and note editor | [Safe Markdown note editing](../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md); archive design |
| Editorial Markdown reader and Markdown/reader toggle (DocumentReader, MarkdownView) | 3 · Vault rail and note editor | Safe Markdown rendering decision |
| Selection toolbar: Ask AI and Add comment (SelectionToolbar) | 3 · Vault rail and note editor | Editor selection API |
| Revision list with AI events, Undo AI edit, restore as new revision (RevisionList, UndoPreview) | 4 · History and diff | [Roadmap 13](../roadmap.md#13-revision-from-a-comment-batch) for AI edits |
| Diff with Show previous, change navigation, Expand all, Changes/Clean (RevisionDiff) | 4 · History and diff | Diff algorithm decision |
| Comment batch revision (CommentBatch) | 4 · History and diff | Roadmap 13 |
| Review queue and pending count (ReviewQueue) | 5 · Review and publication | Roadmap 13 and 14 |
| Exact publication preview, Approve & publish, publication states (PublicationReview) | 5 · Review and publication | [Roadmap 14](../roadmap.md#14-approval-and-publication) |
| Manual import review with metadata, provenance and placement (ImportReview) | 5 · Review and publication | Import-to-working-draft workflow |
| Persist unsent composer text across relaunch | Later | Credential-free local persistence decision |
| VoiceOver hierarchy, announcements and high-contrast qualification | Later | GPUI accessibility support review |

Remove a row only when its slice ships and its evidence is recorded.
