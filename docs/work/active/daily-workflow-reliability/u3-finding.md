# U3: do Needs you and candidate review carry target notes and protection?

**Date:** 15 October 2026. Read-only check at the branch head; no fields were added.

**Answer:** yes, by joining data the desktop already holds. A Needs you item names the candidate's operation. The candidate's writes name each target note and carry its proposed title and `protected` flag. The desktop's current records carry the note's current title and `protected` flag. No single struct carries all of it, and the desktop's Needs you row model drops the link.

## What carries what

| Struct (file:line) | Field | Carries |
|---|---|---|
| `Attention` (`crates/brn-threads-core/src/types.rs:129`) | `kind`, `reason`, `record` | For Review, Conflict and "Changes are waiting for the editor" items, `record` is the candidate's operation ID (`crates/brn-ai/src/threads.rs:1215`, push at `:1253`). `reason` is fixed text such as "Review prepared changes". It has no note IDs, titles or protected flag. A replacement candidate moves `record` to the new operation (`crates/brn-threads-core/src/store.rs:394`). Terminal Blockers use the Run ID. |
| `Prepared` (`types.rs:357`) | `operation`, `request: ChangeRequest` (`:351`) | `operation` matches `Attention.record`. `request.writes` is the exact candidate's write set. |
| `Put` (`types.rs:317`) | `id`, `expected_version`, `archived`, `data` | For a note target, `id` is the note ID and `data` is `RecordData::Note(Note)`, the proposed state with `title` and `protected` (`types.rs:16`, `:22`). `expected_version` is `Some(v)` for an existing note and `None` for a new one. Action targets are `RecordData::Action` and have no protected flag. |
| `Note` in `ThreadsUi.records` (`crates/brn-desktop/src/threads_ui.rs:423`) | `title`, `protected` | The target's current title and protection, looked up by `Put.id`. |
| `ThreadsUi.candidates: Vec<Prepared>` (`threads_ui.rs:425`, refreshed at `:817`) | | Every active candidate, already loaded. |
| `ReviewGroup` (`crates/brn-threads-app/src/review_groups.rs:5`, from `Workspace::review_groups` at `:172`) | `id`, `title`, `kind`, `writes`, `required_groups` | The root Note or Action write ID and its proposed title. No protected flag. |
| `ApplyOutcome::NeedsReview` (`types.rs:383`) | `denied: Vec<String>` | The write IDs that need review, but only in an apply result. It is not saved in attention. |

## The join the UI can do today

1. Take `Attention.record` for a Review or Conflict item.
2. Find the `Prepared` in `ThreadsUi.candidates` whose `operation` equals it.
3. For each `Put` whose `data` is a Note, take `id` and the proposed `title` and `protected`.
4. Look up `id` in `ThreadsUi.records` for the current `title` and `protected`. Mark Protected when either is true; the chat proposal card already does this (`crates/brn-desktop/src/threads_writing.rs:348`, `:374`, `:467`).

## What is missing, all desktop-side

- `ThreadRow.reasons: Vec<String>` (`crates/brn-desktop/src/threads_lists.rs:46`, built at `:101`) keeps only `reason` and drops `kind` and `record`. The Needs you panel (`crates/brn-desktop/src/threads_tiles.rs:622`) therefore cannot reach the candidate. Keeping `record` in the row model is a UI-branch change.
- The Home "Review changes" buttons (`threads_tiles.rs:656-657`) and the sidebar "Review · …" rows (`threads_tiles.rs:254`) show `candidate.request.reason`, not target titles.
- An attention item whose operation is no longer an active candidate (dismissed, applied elsewhere) finds no `Prepared`. The UI should then show the reason alone.

No service field is needed for titles or protection. Structured target IDs on `Attention` would only save the UI this join; the UI branch said it would ask if it wants them.
