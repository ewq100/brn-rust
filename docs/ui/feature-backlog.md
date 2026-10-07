# UI feature backlog

**Historical handoff, superseded for planning on 2026-10-03.** Current UX/UI direction lives in the [design handbook](../design/README.md). The slice numbers and prerequisites below preserve earlier UI ideas, not an approved feature queue. New UI work follows the frozen [target](../architecture/overview.md#frozen-target) and [current roadmap](../roadmap.md), using the shared workflow.

These features were hidden from [slice 1](../superpowers/specs/2026-10-01-workspace-shell-design.md) so the shell did not present fixture data as connected behaviour. Source: the BRN UI/UX handoff (`UI-SPEC.md`, `COMPONENT-INVENTORY.md`). Its Pi SDK assumption and BRN's former App Server direction are historical; the current AI adapter uses Rig.

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
| Revision list with AI events, Undo AI edit, restore as new revision (RevisionList, UndoPreview) | 4 · History and diff | Historical milestone 13; now see [proposal outcomes, Stage 4](../roadmap.md#reviewed-outcomes) |
| Diff with Show previous, change navigation, Expand all, Changes/Clean (RevisionDiff) | 4 · History and diff | Diff algorithm decision |
| Comment batch revision (CommentBatch) | 4 · History and diff | Historical milestone 13; superseded by whole-proposal review |
| Review queue and pending count (ReviewQueue) | 5 · Review and publication | Historical milestones 13/14; superseded by current proposal outcomes |
| Exact publication preview, Approve & publish, publication states (PublicationReview) | 5 · Review and publication | Historical milestone 14; now see [proposal outcomes, Stage 4](../roadmap.md#reviewed-outcomes) |
| Manual import review with metadata, provenance and placement (ImportReview) | 5 · Review and publication | Import-to-working-draft workflow |
| Persist unsent composer text across relaunch | Later | Credential-free local persistence decision |
| VoiceOver hierarchy, announcements and high-contrast qualification | Later | GPUI accessibility support review |

Keep this record as history; select new UI scope against current requirements rather than treating every row as a delivery obligation.
