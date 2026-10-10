# BRN Threads roadmap

Follow the [rebuild plan](work/active/threads-rebuild/plan.md). This replaces the old V1 stage sequence for this branch.

| Milestone | Owner-visible outcome | Engineering proof |
|---|---|---|
| 1 Smaller core | Notes and grouped local changes have understandable history and dependable immediate Undo. | Atomic persistence, retry, protection, versions, active-writing coordination, backup/restore. |
| 2 Complete agent journey | An email updates project knowledge, suggests work, and produces a reply draft; selected papers/processes remain fully readable. | Real provider/tool path, shared commits, source references, faithful qualified full imports, honest gaps. |
| 3 Daily interaction | Native Home, threads, editor, review, search, Actions, history, comments, and export work together. | Native interaction on the target Mac, restart behavior, understandable conflicts and offline use. |
| 4 Qualification and replacement | A usable candidate implements the agreed daily journeys. | Relevant acceptance evidence; superseded runtime paths removed; integration ready for owner direction. |

The independent plan review is complete and its [lead dispositions](work/active/threads-rebuild/review.md#build-lead-dispositions) are incorporated. The [behavior design](architecture/threads-behavior.md) is ready to implement. Qualify Rig 0.44.0 before the new runtime and GPUI Kit 0.7.1 before the new UI; the small core can progress independently.

After the first usable release: DOCX/PPTX/HTML generation, then other capabilities justified by actual use. External sending, broad connectors, sync/multiplayer, a background daemon, rich branch management, and plugin/theme marketplaces are outside the first-release gate.

Status and acceptance live in [current status](status.md), not in historical milestone pass counts.
