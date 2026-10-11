---
name: maintain-notes
description: Update ordinary knowledge as evidence arrives, organize related work, or refresh a stale maintenance candidate.
---

# Maintain notes

1. Use `read_records` for current notes, source observations, related links, thread and Action. Read the relevant full passage with `read_note` at its observed version when a note is truncated. Compare new evidence with dates, quantities, units, qualifications and who reported or decided each fact.
2. Prepare the smallest useful group of typed writes with `prepare_changes`: edit Notes, add or update Link relationships, preserve source references, update an existing Action when supported, and reuse the relevant Thread. Include the observed versions of direct inputs for derived updates. Similar wording alone does not prove two sources, notes or goals are identical.
3. Apply the prepared operation with `apply_changes` and inspect its real outcome. An applied receipt records the result in What changed; routine success stays quiet. Ordinary unprotected working-note maintenance needs no selected-note grant when maintenance is enabled; protected edits require the current scoped target/version grant. Honor writing guards. A deferred member defers the whole group; continue unrelated investigation.
4. If the candidate is stale, reread current targets and inputs and reassess its meaning. Use `refresh_candidate` for a fresh immutable replacement rather than altering the old candidate or retrying its stale versions. Apply the replacement under current authority and record its actual outcome.

Archive superseded ordinary material only when the relationship is clear and the host permits that operation; this is the record's checked `archived` change, not a separate unregistered tool. Preserve protected decisions. If new evidence conflicts, add one Conflict attention item to the relevant Thread through the same checked request and use the resolve-conflict playbook. Writing a conflicting ordinary note does not supersede a protected decision.

## Example

Two project emails confirm a meeting date and add a qualified delivery estimate. Read the existing note and Action, preserve the two source references as links and direct inputs, and prepare the ordinary note update in one group. Apply and report its receipt without creating an attention item per email. If another accepted edit changed the base, reread it and refresh the candidate first.

## Boundaries

- A content-edit grant does not authorize archiving another note or changing protection.
- A note guarded by unsaved writing defers its dependent group.
- A source contradicting a protected decision requires a concrete owner choice rather than silent replacement.
