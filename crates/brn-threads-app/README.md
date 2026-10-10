# Threads application service

`Workspace` supplies shared product use cases over the checked SQLite core:
current notes/search, threads/Needs you, ordered messages, owner changes, full-note
import, candidate review, history/Undo, Markdown snapshot export, and backup.
Desktop, headless commands, and the built-in runtime use these same operations.

A fresh workspace persists Codex / gpt-6.1-sol / medium and the normal Codex auth
file location. It does not read or copy credentials during bootstrap. `settings`,
`configure`, and `pause(true)` support explicit selection/delegation. Configuration
reads canonical Settings and all Working runs at the SQLite writer boundary;
changes and cancellation/fence advancement commit together. Opening a workspace
does not cancel another process's live invocation.

Message chronology follows receipt insertion order and preserves original
creation order when a message is edited; clock precision and UUID order cannot
reorder a conversation. Current search rebuilds from committed, active notes;
unapplied candidates and recovery buffers are excluded.

Full-note import binds the host request key to a digest of title plus typed
conversion result before preparation. Changed retries fail even across the
allocation-to-prepare interruption boundary. Source `external_version` contains
the actual original SHA-256, rather than a hash of that hash. Exact source locator
and retained source-version links prevent duplicate full notes/assets for a
repeat observation under a different host key. Such an observation creates a
Source receipt without incrementing the unchanged note version. A genuinely new
source version can create another protected full note under explicit import.
Conversion completeness remains the converter/inventory's responsibility.

Owner editing of a candidate assigns fresh creation IDs and rewrites structural
references and managed figure URLs by complete, exact asset identity. Unknown
asset URL suffixes, paths, or queries cannot match a shorter known ID. Core replacement preparation, retirement,
and review-attention transfer share a transaction. Failed replacement leaves the
original candidate active. Accepted review removes only the matching attention
through an idempotent checked follow-up; replay finishes cleanup after a lost
response, while preserving unrelated questions/reviews. Explicit owner dismissal retires an unapplied proposal and clears only its attention, releasing the note slot without changing current notes. Retry repairs interrupted cleanup; applied operations cannot be dismissed.

Export stages and verifies current Markdown and retained assets, then publishes
with an exclusive directory rename. Occupied paths and missing managed figures
leave no partial published snapshot. SQLite backup restores the complete state
into an explicit fresh directory; search is rebuilt after opening restored state.

The core now uses schema version 3 for durable host-input and candidate-replacement binding; earlier
synthetic Threads databases are refused safely, without migration. Save maps
supported UTF-8 comment anchors under its writer transaction, marks intersections
unresolved, and records actual Comment writes for Undo. Unsupported arbitrary
text remapping remains visibly unresolved. Closed edit sessions retain ordering
tombstones while clearing buffer text.

Qualification uses public/synthetic data, offline application integration tests,
and locked core tests. This service does not itself prove live provider behavior,
native usability, universal import completeness, or external execution.
