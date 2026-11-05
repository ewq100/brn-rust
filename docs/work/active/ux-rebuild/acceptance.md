# Acceptance results

Spec §16 cases for the UX rebuild on `work/ux-rebuild-20261105`. Results: Pass, Partial, Fail, Blocked or Not run. Every result names its build, theme, fixture and evidence. *Automated* means checked by the named offline test on synthetic data; *native* means observed in the real app on macOS with a fresh synthetic data directory.

Build: recorded per row as the integration commit. Platform: macOS (Apple silicon). Fixture: `brn-ux-rebuild-artifacts/tools/seed-fixture.sh` unless stated.

## Acceptance cases

| Case | Title | Result | Theme / size | Evidence |
|---|---|---|---|---|
| AC-01 | Workspace continuity | Not run | | |
| AC-02 | Chat and draft isolation | Partial | Light + Dark 1280 | Native: the composer stays bound to each chat while switching. Automated: per-chat drafts, clear-once-after-accept, a refused send keeps its text and names the retry. Not natively driven across a restart. [S3 evidence](s3-chat-evidence.md) |
| AC-03 | Queued messages | Pass | Light + Dark 1280 and 900 | Native: paused queue of four in the chat, the start screen and the 900 sheet, with aligned Edit/Remove/↑/↓; Enter queues while paused. Automated: edit, reorder both ways, remove and dispatch order. [S3 evidence](s3-chat-evidence.md) |
| AC-04 | Stop, Resume, and Steer | Partial | Light + Dark 1280 | Native: Resume queue control present. Automated: Stop pauses the queue, explicit Resume, Steer persistence and partial answers. A live run needs paid provider calls, so Stop/Steer were not driven natively. [S3 evidence](s3-chat-evidence.md) |
| AC-05 | Provider and model selection | Partial | Light + Dark 1280 | Native: model and effort controls show "ChatGPT · gpt-6.1-sol" and "medium"; no run started. Automated: display names and busy reasons. [S3 evidence](s3-chat-evidence.md) |
| AC-06 | Chat to document canvas | Not run | | |
| AC-07 | Project attachments and Keep in BRN | Not run | | |
| AC-08 | Articles and All files | Not run | | |
| AC-09 | Search and scope | Not run | | |
| AC-10 | Direct editing and saving | Partial | Light + Dark 1280 | Automated: edits survive refresh, theme, pane toggles and navigation; Unsaved → Saved; Conflict on stale Save. Native: Saved/Unsaved header and recoverable footer (`dry2/light-recovery`). Native typing and Save not driven. [S4 evidence](s4-writing-evidence.md) |
| AC-11 | Passage comments and rewrite | Partial | Light + Dark 900 | Automated: anchoring, re-anchoring, "moved" status, unresolved quote kept. Native: commenting on an unsaved edit refused with "Save your edit first". A posted comment (VE-04) not captured natively. [S4 evidence](s4-writing-evidence.md) |
| AC-12 | Review-required and protected edits | Partial | Light + Dark 1280 | Automated: review summary, Discard leaves note unchanged, Approve on protected. Native: pending proposal with Added/Removed in words and Apply whole note. Protected Approve not observed natively. [S4 evidence](s4-writing-evidence.md) |
| AC-13 | Delegated ordinary maintenance | Not run | | |
| AC-14 | Apply, reopen, and Undo | Partial | Light + Dark 1280 | Automated: apply then reopen; receipt Undo; editor undo cannot resurrect stale text. Native: Apply → v2 text, "Working · v2", Saved. Non-Rewrite proposals show Undo only in Recent changes; Undo not driven natively. [S4 evidence](s4-writing-evidence.md) |
| AC-15 | Manual Inbox Scan | Pass | Light 1280×912 (S6); Dark 1280×900 (`90119da9`) | Native: explicit Scan of one `.md` + one invalid `.docx` → 1 imported, 1 failed keeping its name; rescan creates no duplicate and now reports "N unchanged" (`f220de1f`). Automated: `threads_inbox_tests`, `manual_inbox`, `inbox_scale`. Shots `s6/light-1280-scan-*`, `int-pre/dark-imports`. |
| AC-16 | Tasks and completion receipts | Partial | Light 1280×912 | Native: Tasks page and detail captured (`s6/light-1280-task*`); completion receipt natively pending. Automated: `a_task_opens_its_detail_panel_and_completes_explicitly`. |
| AC-17 | History and restoration | Partial | — | Automated: history views labelled; preview leaves current content unchanged; ⋯ Document history. Native not captured. [S4 evidence](s4-writing-evidence.md) |
| AC-18 | Settings and retained capabilities | Partial | Light + Dark | Native: every Settings section reachable, Appearance/Data captured in both themes. Restart persistence and reset automated (settings tests, S1 presentation tests); natively pending. Documents/MCP section bodies unchanged in design. |
| AC-19 | Multi-note proposal atomicity | Not run | | |
| AC-20 | Stale proposal conflict | Not run | | |
| AC-21 | Undo after later work | Not run | | |
| AC-22 | Authority changes during a run | Not run | | |
| AC-23 | Concrete permission flow | Not run | | |
| AC-24 | Save failure and retry | Partial | — | Automated: Conflict keeps the edit and never shows Saved; recovery buffer retained. A controlled I/O failure is not injectable, so "Save failed · Retry" is exercised only by code path. [S4 evidence](s4-writing-evidence.md) |
| AC-25 | Interruption and restart recovery | Not run | | |
| AC-26 | Provenance and missing sources | Not run | | |
| AC-27 | Context switching and archival | Not run | | |
| AC-28 | Provider and run failures | Partial | — | Automated only: failed and interrupted finish text, refused Stop/Undo banners, MCP refusals. Not native: needs a live provider failure. [S3 evidence](s3-chat-evidence.md) |
| AC-29 | System, Light, and Dark | Partial | Light + Dark | Native Light and Dark tours of every page (`s1`–`s6`, `int-pre`). OS appearance flip with System selected is covered by the automated seam test only (owner's OS appearance is not changed). |
| AC-30 | Window sizes and layout | Not run | | |
| AC-31 | Keyboard and focus | Partial | Light + Dark | Native: Escape closes the chip and + menus; Shift-Enter inserts a newline; controls reachable by accessible name (every tour step pressed through AX). Automated: Enter/Shift-Enter/IME/⌘↵, Jump to latest label. Full keyboard-only traversal not recorded. [S3 evidence](s3-chat-evidence.md) |
| AC-32 | Native accessibility semantics | Not run | | |
| AC-33 | Native text and long content | Not run | | |
| AC-34 | Actual app, toolkit migration and declared platform gates | Partial | — | `gpui-kit =0.9.0` resolved once in the lockfile; `--locked` builds and the real app launched on macOS (Apple silicon). Other declared release targets are not available in this environment (see `s0-gpui-090-evidence.md`). |
| AC-35 | Export and backup restore | Partial | — | Export, full backup and isolated restore driven through the shared service by CLI (S6 evidence); UI panels present in Settings → Data but not natively driven. Automated: backup/restore tests. |
| AC-36 | Project movement and deletion | Not run | | |
| AC-37 | Explicit web and MCP boundaries | Not run | | |
| AC-38 | Intake intent and coverage | Partial | Light | Offline xberg conversion of `.docx`/`.md`, provenance shown in Imports. Controlled partial conversion and figures covered by intake tests only. |
| AC-39 | Explicit writing decision and capture actions | Not run | | |
| AC-40 | Knowledge checks and useful navigation | Not run | | |
| AC-41 | Outline navigation and version correctness | Partial | Light + Dark 1280 and 900 | Automated: duplicate, skipped, Unicode, long and fenced headings; pointer and keyboard; generation-bound refusal; compact view. Native: docked outline with current section; compact contents view and Go. Native keyboard traversal not captured. [S4 evidence](s4-writing-evidence.md) |
| AC-42 | Long-article presentation and compact supporting views | Pass | Light + Dark 1280 and 900 | Native: prose, quote, table, code with Copy, wide table scroll, citation links, stored figure (not inverted in Dark); compact contents and empty-outline views. [S4 evidence](s4-writing-evidence.md) |

## Visual evidence

| Set | Subject | Required | Result | Screenshots |
|---|---|---|---|---|
| VE-01 | Conversation with an answer, references, active context, and composer | Light and Dark; 1280×900 | Pass | `shots/s3/ve01-{light,dark}-1280-conversation.png` (answer with references, paused queue, attachment row, composer) |
| VE-02 | Document canvas with pending review, old/new text, assistant, and Apply/Discard | Light and Dark; 1280×900 | Pass | `shots/s4/{light,dark}-1280-review-pending.png`, `dark-1280-review-result.png`, `{light,dark}-1280-review-applied-undo.png` |
| VE-03 | Article Library with a current delegated-update example and access to All files | Light and Dark; 1280×900 | Not run | |
| VE-04 | Direct-edit draft with a passage comment and a useful pane configuration | Light and Dark; 900×600 | Not run | |
| VE-05 | One concrete permission dialog; one representative provider or persistence error | Permission in Light; error in Dark; 1280×900 | Not run | |
| VE-06 | Manual Inbox Scan result; task completion receipt | Scan in Light; receipt in Dark; 1280×900 | Not run | |
| VE-07 | Settings with the appearance choice and visible keyboard focus | Either theme; 900×600 | Not run | |
| VE-08 | Supplied long article with the outline/current section and representative table, image and citation states; compact contents fallback | Light and Dark; 1280×900 and 900×600 | Not run | |
