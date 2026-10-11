# Threads application service

`Workspace` supplies shared product use cases over the checked SQLite core:
current notes/search, threads/Needs you, ordered messages, owner changes, full-note
import, candidate review, history/Undo, Markdown snapshot export, and backup.
Desktop, headless commands, and the built-in runtime use these same operations.

A fresh workspace persists Codex / gpt-6.1-sol / medium and the normal Codex auth
file location. It does not read or copy credentials during bootstrap. Fresh Settings
start with no chosen delegation and maintenance disabled. Existing unrecorded choices
remain readable but do not grant standing maintenance. `choose_delegation` records the
owner’s maintenance or review-first choice against current Settings; provider Configure
and Resume cannot manufacture a choice. Provider Configure preserves all current policy
fields, including a choice or pause made after the provider form was opened. Manual
work and scoped direct instructions remain available while pending. Configuration
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

Full and useful-information intake retain a typed coverage report on Source and
Note provenance. Detected omissions, converter limitations, and coverage not
independently verified remain distinct through backup and restore; an unchecked
conversion is not evidence of missing content. Old records without a report
retain their existing gap text without implying independent coverage checks.
Useful-information intake supplies converted figures to the runtime as transient
host-held assets. `read_source` returns opaque figure links rather than image
bytes; selected links in checked Note writes materialize only those figures as
managed SQLite assets with source provenance. Unselected figures and raw source
bodies are not archived. Selected asset digests and current source identity are
checked before immutable preparation, and the resulting writes use the usual
atomic apply, replay, review, and stale-input checks.
Used transient sources carry unchanged expected-version observations in the
prepared group. The SQLite writer therefore rejects a source changed between
Prepare and Apply, including during later owner review; an already committed
receipt remains replayable. A model can improve a source label but cannot replace
host-recorded coverage, omissions, or the observed source fingerprint.

Owner editing of a candidate assigns fresh creation IDs and rewrites structural
references and managed figure URLs by complete, exact asset identity. Unknown
asset URL suffixes, paths, or queries cannot match a shorter known ID. Core replacement preparation, retirement,
and review-attention transfer share a transaction. Failed replacement leaves the
original candidate active. Accepted review removes only the matching attention
through an idempotent checked follow-up; replay finishes cleanup after a lost
response, while preserving unrelated questions/reviews. Explicit owner dismissal retires an unapplied proposal and clears only its attention, releasing the note slot without changing current notes. Retry repairs interrupted cleanup; applied operations cannot be dismissed.

Export stages and verifies current Markdown and retained assets, then publishes
with an exclusive directory rename. Occupied paths and missing managed figures
leave no partial published snapshot. Each note starts with a YAML front matter
wrapper containing its title and `sources` (an empty list for plain notes). Import
provenance and active note-to-source links supply deduplicated source record IDs,
external locators, and descriptive labels. Strings use JSON double-quoted scalar
escaping, preserving quotes, newlines, Unicode, and YAML delimiters. Source
observation/version fields are omitted because a later observation can advance
the Source while an imported note retains earlier content. The original Markdown
body follows the wrapper unchanged except for portable managed-asset links;
existing body front matter is preserved without parsing. Manifest hashes cover
the complete exported bytes, including the wrapper. SQLite backup restores the
complete state into an explicit fresh directory; search is rebuilt after opening
restored state.

The core now uses schema version 3 for durable host-input and candidate-replacement binding; earlier
synthetic Threads databases are refused safely, without migration. Save maps
supported UTF-8 comment anchors under its writer transaction, marks intersections
unresolved, and records actual Comment writes for Undo. Unsupported arbitrary
text remapping remains visibly unresolved. Closed edit sessions retain ordering
tombstones while clearing buffer text.

Qualification uses public/synthetic data, offline application integration tests,
and locked core tests. This service does not itself prove live provider behavior,
native usability, universal import completeness, or external execution.
