# Session B — reusable writing workspace and Zorite editor evaluation

You own writing/editor work in the BRN modular-foundation program. Read the supplied master plan and the current canonical `docs/work/active/modular-foundation/plan.md`, contracts, ownership, baseline and coordination records. Session C owns core/retrieval/shared integration; A owns intake.

## Objective

Extract the existing writing UI into a reusable `brn-writing-ui` crate using the current editor, and build persistent brief/references with honest Sources used on C's shared changes. Prove the same implementation in a minimal development host. Separately, run a time-boxed Zorite spike against named missing capabilities; adoption is a later decision. Do not ship a second production app or fork NoteGen/Zed/Zorite.

The user workflow is: open/create a document, attach references, set a brief, write directly, add several passage comments and overall feedback, ask AI to revise using those materials, inspect sources and changes, Apply/reject, Undo and recover after restart. It must work with ordinary chat closed and across separate conversations about the same document.

## Preflight and constraints

Read `AGENTS.md`, status, architecture target/behavior/invariants, workflow/verification, current writing-polish/projects plans, relevant writing/core/app/AI/UI code and tests. Inspect actual local state and active owners. Use C's common ref and an isolated worktree/data directory; preserve newer work and unrelated changes.

Own `brn-writing-ui`, the writing-related desktop paths and `experiments/zorite-*`. Do not edit desktop writing files until `work/ui-polish` has landed or been owner-paused and is on C's integration ref. C owns core/app writing operations (`brn-threads-core::writing`, `brn-threads-app::writing`), the writing-setup record, request v2, candidate provenance, the shared run start, schema, root manifests, the lockfile and generic scope logic. Submit explicit requests for shared changes. Disclose any minimal local manifest/module patches needed to compile; C reconciles them.

Work locally with synthetic data, review, documentation and commits. Publication/merge/release, real-data access, provider spending or downloads require current explicit authority. Use existing compiler/GUI/Rig/provider pins unless a separately qualified coordinated change is necessary. No computer use on a locked Mac; collect native gates instead.

## B1 — preserve and extract with the existing editor

Characterize current Read/Write, formatting, saved selections, passage and overall feedback, captured request context, Stop/retry, review, exact Apply, rejection, Undo and draft recovery. Keep those flows working while extracting.

Move the writing panel and its view-state out of the monolithic `impl ThreadsUi` (`threads_writing.rs` plus writing parts of `threads_ui.rs` and `threads_workbench.rs`) into `brn-writing-ui`. That crate depends on `brn-threads-app` and an optional `gpui-kit` (`native-ui`), never on `brn-desktop`, and compiles no GPUI in default builds (F22). The existing core/app writing modules are the headless component; do not add a separate headless crate or host traits. Call C's shared run start rather than `ThreadsUi::start_run`. Avoid dependency cycles or large generic-common crates.

Use a thin editor seam inside the crate, not a public trait, while there is one editor. Do not copy a second workflow into the development host. Separate extraction from editor/dependency replacement in reviewable slices.

## B2 — Zorite qualification, not assumed adoption

This is a time-boxed spike in `experiments/zorite-*`. It may start now and does not gate B's exit. Target the named missing capabilities: (a) range geometry for margin comments, (b) live WYSIWYG, (c) IME.

Verified facts:

- `zorite-editor 0.11.2` is on crates.io under MIT and depends on `gpui-pre ^0.3`, the same family as BRN's 0.3.9.
- The Zorite application is GPL-3.0-or-later; never depend on it. Its workspace patch is not inherited by consumers.
- Offsets are UTF-8 bytes. `replace_range` emits no `Changed`. `set_text` clears undo history.
- There is no read-only mode. Geometry is caret-only.
- WYSIWYG uses `zorite-markdown`, a second Markdown interpreter.

Gates, in order:

1. Compile against `gpui-kit =0.7.3` without patches.
2. Headless checks of offsets, read-only enforcement and rendered-to-source mapping.
3. Native checks on the target Mac.

Compare the current editor and the pinned Zorite package using the same fixtures and host operations. Inspect actual public APIs, published-package contents versus upstream workspace patches, license/source boundaries and native dependencies.

Resolve one compatible GPUI dependency graph with BRN's current GPUI Kit. Do not upgrade/replatform the entire interface to make one demo compile. Record any required patch and its maintenance/upstream path.

Mandatory evidence: exact rendered-to-source selection; correct occurrence in repeated text; Unicode/graphemes/emoji/CRLF; scroll/wrap/resize/font/first-layout geometry; native keyboard/mouse/clipboard/IME; safe programmatic replacement; real read-only/review state; no implicit save on display refresh; formatting round-trip; long-document performance; current safe Markdown/asset/link policy; exact review identity; and coordination between local editing Undo and durable BRN Undo.

Native tests must exercise real interactions on the target Mac. Headless tests or screenshots alone do not establish IME/selection usability. Report unavailable native gates explicitly.

Choose adopt, keep-current, or a bounded adaptation with exact tests/owner and a maintenance plan. If Zorite fails, continue writing with the current editor. Do not add an indefinite fallback, unbounded fork or duplicate OOXML/Markdown interpreter.

## B3 — persistent writing context and sources

Use existing BRN project/document links, the core `visible()` predicate and the existing scoped search/read tools. Master plan section 7.2 fixes storage, scope, capture and input binding, and C implements them (CW1/CW2). A workspace retains one editable document, references, optional brief, comments/overall feedback and associated conversations. Do not create a competing project membership/permissions system or duplicate canonical notes per project.

Support existing BRN reference notes. A file becomes a reference through the existing full-note import followed by attaching the resulting note; writing never consumes conversion results. References belong to the working document's setup, not one chat. They persist across new/switch/closed chat, note navigation and restart. A reference not visible in the request's conversation is excluded and shown to the owner as "not available in this conversation" (F21).

Brief/reference-purpose metadata can express audience/tone/requirements and whether a reference supplies requirements, evidence or style. It must not become a global taxonomy or grant authority. Reference content is read-only to writing; embedded source instructions remain untrusted.

Capture is defined in master plan section 7.2. Preserve safe explicit Save-and-rewrite/Save-and-comment semantics. No silent save to start a run. Unsaved-snapshot requests are out of scope for this program.

Targeted edits use the existing scoped search/read tools. An exhaustive check counts as complete only when the persisted read ledger shows each requirements reference version completely read; otherwise label it partial. Do not paste every reference into every request or confuse serialized-byte limits with model token limits. Attached does not mean read in full.

Show actual Sources used with stable retained revisions/locators and validated quotations. Do not invent bibliography entries or treat a quote match as proof of all generated claims. Distinguish available/used/incomplete/changed/removed/inaccessible source state. Access revocation must clear/restrict cached previews as required by the host.

A proposal changes only the working document. Keep exact candidate identity and atomic checked Apply. Apply section 7.2's binding:

- Brief or reference-list edits make the candidate stale.
- Reference content edits leave it applicable, with "changed since" shown.
- Access loss refuses Apply.

Retry preserves identity/snapshot only where allowed; refresh/regeneration needs a new request and new review. Cancellation fences late results.

## B4 — development host (an editor cutover is a later assignment)

Build the development host as a binary or example of `brn-writing-ui` behind `native-ui`. It opens a real `brn-threads-app` `Workspace` in a disposable directory, uses C's shared run start with a scripted provider for deterministic journeys, and does not depend on `brn-desktop`. Mocks alone do not prove reuse. No separate production data model, sync system, installer, settings product or release.

Deliver the Zorite spike report separately. Any editor switch after a positive report is a separate owner-approved slice that reruns writing, native and security gates and removes replaced routes.

## Verification and handoff

Run relevant shared F01–F22 cases: persistent context, exact repeated/Unicode selection, multi-comment rewrite with Chat closed, targeted vs full-reference coverage, cross-scope references (F21), malicious references, stale draft/reference/access revocation, exact Apply/replay/Undo, local undo after refresh, cancellation, restart, default Linux build (F22) and unchanged ordinary chat/projects/tasks behavior.

Hand C exact commits/contract revision, paths and shared-file patches, editor decision/pins/API proof, native observation and limitations, source-context tests, independent review, data/rollback compatibility and remaining gates. Update relevant canonical docs and runtime guidance through coordinated ownership. Separate implemented, tested, live-verified, native-observed, accepted and merged; do not claim a finished product based on a successful build alone.
