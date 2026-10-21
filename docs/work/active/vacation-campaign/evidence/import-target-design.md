# Ordinary import target ownership

Bounded read-only design assessment for `ordinary-import-target-ownership`.

## Source and instructions

- Source checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Observed HEAD: `f6dd4d6666320437d631759440b68f2b43d33a9d`
- Selected task read from lead campaign: the transient `ThreadsUi.attaching_chat` target is written by two UI callers and consumed by the picker helper; assess a simpler explicit interface, preserving ordinary/general/reference semantics and busy guards. This is a maintenance assessment, not a defect report.
- Strategic-programming rules applied: group state only when it removes a real responsibility burden; interfaces must hide meaningful decisions; retire obsolete route/callers; reject an interface whose cost exceeds the simplification.
- No source/shared-record/Git edits, builds, tests, provider/network/account/native activity, or nested agents.

## Actual writers and consumers

`ThreadsUi.attaching_chat: Option<String>` is initialized to `None`, assigned at exactly two call sites, and consumed only by `choose_import_for_reference` when `reference.is_none()`:

1. `threads_chat.rs::choose_chat_attachment` refuses while any Worker exists, resolves or creates the current/new chat with `composer_chat`, switches chat while preserving the composer draft, then stores `Some(id)` into `attaching_chat` and invokes `choose_import(FullNote, ...)`.
2. The project menu's “Import file…” action in `threads_projects.rs` similarly assigns the captured project chat ID and invokes `choose_import(FullNote, ...)`. The menu is disabled when busy; `choose_import_for_reference` has its own worker guard as a second check.

`choose_import` delegates to `choose_import_for_reference(intent, None, ...)`. That function checks busy **before** consuming the transient target. For ordinary import, it takes the field into local `attachment_chat`, then opens the native path prompt. The picker callback checks for a newly busy Worker again before conversion, then passes the captured local target to `convert_file`. A cancelled picker has already consumed the field and simply returns; it leaves no stale target. The selected path and captured target are held in the async callback, so later navigation does not retarget the pending import.

`convert_file` takes `attachment_chat: Option<String>` explicitly. The worker thread captures it in `WorkerEvent::Converted.chat`; completion sends it to checked `import_full_scoped_with_conversion`. The destination is therefore explicit and invocation-local after the picker returns. `attach_chat_path` bypasses the picker/field and calls `convert_file(..., Some(chat), ...)` directly.

Ordinary general imports from `threads_tiles.rs` (UsefulInformation and FullNote) and `threads_writing.rs` (UsefulInformation and FullNote) call `choose_import` without a target. Reference import has a separate `choose_import_for_reference` call with a validated `ReferenceImportIntent`; its path sets `attachment_chat = None` and uses its captured checked reference target. This distinction should remain clear.

## Cancel, busy, and navigation behavior

- There is no supported chooser retry state: each attachment action computes and supplies the destination on every invocation.
- The proposed rejected claim that cancelling the picker leaves a stale chat destination is false: `.take()` occurs before the prompt opens.
- If a Worker appears while the picker is open, the selected-path callback warns and returns before `convert_file`; the already captured target is dropped with that callback. If conversion starts, the destination has moved into its worker event payload.
- Navigation after opening the picker or while conversion runs does not retarget the import. A missing/unavailable destination is handled by the checked import scope; no fallback to the newly selected chat is present.
- Busy checks should stay at `choose_import_for_reference`, the post-picker callback, and `convert_file`; caller guards/menu disablement remain presentation affordances. No change to current direct-path import behavior.

## Smallest useful interface and recommendation

Pass the destination as an explicit argument to `choose_import` / `choose_import_for_reference`, then retire `ThreadsUi.attaching_chat`, its initializer, the two ambient assignments and the picker helper's `.take()`:

```rust
enum ImportDestination {
    Unattached,
    Chat(String),
}
```

`choose_import(intent, ImportDestination::Unattached, ...)` covers general calls; the chat and project attachment callers pass `Chat(id)`; reference import continues through its separate checked `ReferenceImportIntent` route. In `choose_import_for_reference`, capture the ordinary target directly from the request and pass it into `convert_file`; reference import still uses no ordinary chat target.

An `Option<String>` argument is the smallest textual migration and matches `convert_file`'s existing parameter and `WorkerEvent::Converted.chat`; semantically, however, it leaves callers to remember what `None` means and makes an ordinary chat destination visually similar to a missing value. The two-way enum adds only a small type and makes the request explicit at the six ordinary `choose_import` call sites (two attach callers and four general callers); `convert_file` may keep its existing `Option<String>` as a lower-level payload mapping, so the enum need not spread into worker or persistence contracts. It also avoids an overly broad request struct for one optional argument. If minimizing type surface is the lead's priority, `Option<String>` is still behaviorally sufficient; do not build a generic import request abstraction.

This is a modest but real ownership improvement: remove ambient mutable state that must be set in tandem with a future picker call, while preserving capture at the same moment and all current checks. It does not add destination policy or change checked service authority. If the enum would cause additional adapter layers or scope creep, retaining the field is acceptable because the current path consumes it before the async boundary and no defect is demonstrated.

## Existing evidence and proportionate checks

Existing source-backed evidence already covers destination capture and checked completion. `threads_writing_tests.rs::b3_reference_disconnect_clears_intent_before_the_next_chat_attachment` injects a synthetic ordinary `Converted { chat: Some(chat_b) }` completion after a disconnected reference import, then asserts the imported Note is linked to that captured chat and unrelated writing setup/queue remain unchanged. `evidence/composer-drafts-audit.md` also records the existing synthetic poller witness for ordinary chat attachment and explicitly limits it: it does not automate a native picker while changing views. Reference-specific tests cover stale/changed targets, and `reference-import-qualification` records selected import behavior.

A small interface-only refactor should reuse these existing checks and existing import qualification; no new tests are needed merely to exercise enum plumbing. If changing picker wiring, the consequential assertion is already represented by ordinary conversion completion with `Converted.chat = Some(target)` and `None` remaining unattached. Native picker interaction remains a separate deferred observation and is not needed to decide this ownership boundary.

## Disposition

Recommend explicit destination input and retirement of `attaching_chat`, preferably with a small `ImportDestination::{Unattached, Chat(String)}` enum at the picker-request boundary, while retaining the current Option-valued async/conversion payload. The strongest benefit is removal of transient side-channel state; no stale-cancel defect is claimed. Keep busy checks, reference intent, `attach_chat_path`, captured target behavior, and checked import semantics unchanged. The existing source and synthetic completion evidence are proportionate; do not expand into picker UX or tests without a concrete gap.
