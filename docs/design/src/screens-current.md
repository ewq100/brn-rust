# Current app screens

All screens on this page are **Implemented**. Images are real renders of the
production view tree over the synthetic demo workspace
(`scripts/design-capture.sh`), drawn by the toolkit's headless Metal renderer at
1280 × 820 pt. That is *visual* evidence of the implemented appearance; it is not
native window, keyboard or owner verification (see the
[verification record](verification.md)). *Before* images are the same views on
the baseline `3d59cdd`.

## Home: chat with no conversation

| Before | After |
| --- | --- |
| ![Before: home](images/before/01-home-empty-chat-dark.jpg) | ![After: home](images/current/01-home-empty-chat-dark.jpg) |

Changes: proportional type; structured navigator (Workspace, Chats, Review with
counts); setup-aware welcome with a vault/model checklist, *Open Settings* and
example questions; composer without code-editor line numbers; vault rail with
scope meaning and titled notes; status moved to a bottom strip.

## Chat conversation

| Before | After |
| --- | --- |
| ![Before: chat](images/before/02-chat-conversation-dark.jpg) | ![After: chat](images/current/02-chat-conversation-dark.jpg) |

Question and answer are distinct; each answer shows **BRN**, the recorded
provider · model · effort in monospace and a status badge. *Review as new note…*
is a quiet row action instead of a full-width button. Provenance chips are not
implemented yet ([future](screens-future.md)).

## Dashboard (light and dark)

| Before | After |
| --- | --- |
| ![Before: dashboard](images/before/03-dashboard-dark.jpg) | ![After: dashboard](images/current/03-dashboard-dark.jpg) |

![After: dashboard, light](images/current/11-dashboard-light.jpg)

Count tiles carry tone only when non-zero (Overdue red, Follow-up amber); Action
rows show title, state, due/follow-up and owner with separate state and signal
badges; filters are a segmented control; *Complete…* is the primary command.

## Inbox

| Before | After |
| --- | --- |
| ![Before: inbox](images/before/04-inbox-dark.jpg) | ![After: inbox](images/current/04-inbox-dark.jpg) |

The waiting originals now come **first**, as cards with kind badge, title,
received time (UTC) and availability; processing, cleanup and Source preparation
follow; *Add a copy* and the approved-Source analysis come after the queue.

![After: narrow window, rails auto-collapsed](images/current/12-narrow-inbox-dark.jpg)

## Proposal review

| Before | After |
| --- | --- |
| ![Before: proposal review](images/before/07-proposal-review-dark.jpg) | ![After: proposal review](images/current/07-proposal-review-dark.jpg) |

Header with *Proposal · Draft* badge, title and exact identity; trust callout;
labelled sections (Title, Changes with kind badges, before/proposed text, source
versions, Comments and Rewrite, Review state); and a **pinned decision bar**
naming the exact version, with *Reject* and the primary *Review exact approval…*.

## Needs Review, Activity, note and evidence

| Needs Review | Activity |
| --- | --- |
| ![Needs Review](images/current/05-needs-review-dark.jpg) | ![Activity](images/current/06-activity-dark.jpg) |

| Note (current, editable) | Source evidence (read only) |
| --- | --- |
| ![Note editor](images/current/08-note-editor-dark.jpg) | ![Source evidence](images/current/09-source-evidence-dark.jpg) |

Notes show *Current · editable* (green); evidence shows *Source · read only*
(cyan) or *History · read only*. *Save to Markdown* is primary; recovery tools
are quiet; *Save a copy* is its own labelled group.

## Settings

![Settings](images/current/10-settings-dark.jpg)

## Known limitations of the current implementation

- Recorded times show in UTC; local time display is an open preference.
- Approved reply/Action shortcuts, claim chips, activity trail with budget,
  grouped Inbox review, Today, projects, people, graph and chat lifecycle are
  future designs.
- Proposal and Action dialogs (approval, completion, Undo, repair) keep their
  previous layout apart from theme and type changes.
- Several low-level recovery commands remain visible as quiet buttons; hiding
  them until relevant needs per-control test review.
- Toolkit accessibility (VoiceOver) is unqualified.
