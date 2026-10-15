# Session B — reusable writing workspace and Zorite editor evaluation

You own writing/editor work in the BRN modular-foundation program. Read the supplied master plan and the current canonical `docs/work/active/modular-foundation/plan.md`, contracts, ownership, baseline and coordination records. Session C owns core/retrieval/shared integration; A owns intake.

## Objective

Extract and complete a reference-assisted writing module inside BRN. Independently qualify the Zorite editor and adopt it only after its gates pass. Prove the same writing implementation in a minimal development host. Do not ship a second production app or fork NoteGen/Zed/Zorite as the product.

The user workflow is: open/create a document, attach references, set a brief, write directly, add several passage comments and overall feedback, ask AI to revise using those materials, inspect sources and changes, Apply/reject, Undo and recover after restart. It must work with ordinary chat closed and across separate conversations about the same document.

## Preflight and constraints

Read `AGENTS.md`, status, architecture target/behavior/invariants, workflow/verification, current writing-polish/projects plans, relevant writing/core/app/AI/UI code and tests. Inspect actual local state and active owners. Use C's common ref and an isolated worktree/data directory; preserve newer work and unrelated changes.

Own new writing components, allocated writing-specific app/tests, editor adapter and writing-related desktop paths. C owns shared core types/schema/root manifests/lockfile/general runtime and generic scope logic. Submit explicit requests for shared changes. Disclose any minimal local manifest/module patches needed to compile; C reconciles them.

Work locally with synthetic data, review, documentation and commits. Publication/merge/release, real-data access, provider spending or downloads require current explicit authority. Use existing compiler/GUI/Rig/provider pins unless a separately qualified coordinated change is necessary. No computer use on a locked Mac; collect native gates instead.

## B1 — preserve and extract with the existing editor

Characterize current Read/Write, formatting, saved selections, passage and overall feedback, captured request context, Stop/retry, review, exact Apply, rejection, Undo and draft recovery. Keep those flows working while extracting.

Separate headless writing state/workflow from `ThreadsUi`, global navigation and whichever unrelated chat is selected. The headless component cannot import `brn-desktop`, full `brn-threads-app`, a concrete store handle or provider credentials. Use C's narrow host interfaces; keep writes through existing checked services. Avoid dependency cycles or large generic-common crates.

Add a reusable writing panel and thin editor adapter only where justified. Do not copy a second workflow into the development host. Separate extraction from editor/dependency replacement in reviewable slices.

## B2 — Zorite qualification, not assumed adoption

Compare current editor and exact pinned Zorite package/commit using the same fixtures and host operations. Inspect actual public APIs, published-package contents versus upstream workspace patches, license/source boundaries and native dependencies. A package's main-branch version is not proof of a compatible published build.

Resolve one compatible GPUI dependency graph with BRN's current GPUI Kit. Do not upgrade/replatform the entire interface to make one demo compile. Record any required patch and its maintenance/upstream path.

Mandatory evidence: exact rendered-to-source selection; correct occurrence in repeated text; Unicode/graphemes/emoji/CRLF; scroll/wrap/resize/font/first-layout geometry; native keyboard/mouse/clipboard/IME; safe programmatic replacement; real read-only/review state; no implicit save on display refresh; formatting round-trip; long-document performance; current safe Markdown/asset/link policy; exact review identity; and coordination between local editing Undo and durable BRN Undo.

Native tests must exercise real interactions on the target Mac. Headless tests or screenshots alone do not establish IME/selection usability. Report unavailable native gates explicitly.

Choose adopt, keep-current, or a bounded adaptation with exact tests/owner and a maintenance plan. If Zorite fails, continue writing with the current editor. Do not add an indefinite fallback, unbounded fork or duplicate OOXML/Markdown interpreter.

## B3 — persistent writing context and sources

Use existing BRN project/document links and C's scoped context/evidence interfaces. A workspace retains one editable document, references, optional brief, comments/overall feedback and associated conversations. Do not create a competing project membership/permissions system or duplicate canonical notes per project.

Support existing BRN reference documents plus file imports through A's agreed result contract/current adapter. Develop against fixtures before A's extractor is selected. References belong to the writing task, not one chat; they persist across new/switch/closed chat, note navigation and restart.

Brief/reference-purpose metadata can express audience/tone/requirements and whether a reference supplies requirements, evidence or style. It must not become a global taxonomy or grant authority. Reference content is read-only to writing; embedded source instructions remain untrusted.

Capture exact request identity, document/base version, editor generation where relevant, brief/comments, selected reference revisions, current scope/context fence, model and budgets. Preserve safe explicit Save-and-rewrite/Save-and-comment semantics. No silent save to start a run. Unsaved-snapshot support needs proven generation/dirty-guard behavior and cannot erase typing.

Use C's relevant-passage retrieval for targeted edits and explicit coverage-aware reading for exhaustive checks. Do not paste every reference into every request or confuse serialized-byte limits with model token limits. Report omitted/budget-limited/partial material. Attached does not mean read in full.

Show actual Sources used with stable retained revisions/locators and validated quotations. Do not invent bibliography entries or treat a quote match as proof of all generated claims. Distinguish available/used/incomplete/changed/removed/inaccessible source state. Access revocation must clear/restrict cached previews as required by the host.

A proposal changes only the working document. Keep exact candidate identity and atomic checked Apply. Reference/document/scope changes during the run obey C's current-input policy. Retry preserves identity/snapshot only where allowed; refresh/regeneration needs a new request and new review. Cancellation fences late results.

## B4 — development host and final cutover

Build a minimal native development host that renders the same writing panel without full BRN navigation/tasks. Use synthetic documents/references and at least one real shared-service journey. Mocks alone do not prove reuse. No separate production data model, sync system, installer, settings product or release.

After successful qualification, land any editor switch separately, rerun writing/native/security gates and remove replaced production routes. Preserve data readers and rollback compatibility. If the current editor wins, deliver the module and documented negative Zorite result.

## Verification and handoff

Run relevant shared F01–F20 cases: persistent context, exact repeated/Unicode selection, multi-comment rewrite with Chat closed, targeted vs full-reference coverage, malicious references, stale draft/reference/access revocation, exact Apply/replay/Undo, local undo after refresh, cancellation, restart and unchanged ordinary chat/projects/tasks behavior.

Hand C exact commits/contract revision, paths and shared-file patches, editor decision/pins/API proof, native observation and limitations, source-context tests, independent review, data/rollback compatibility and remaining gates. Update relevant canonical docs and runtime guidance through coordinated ownership. Separate implemented, tested, live-verified, native-observed, accepted and merged; do not claim a finished product based on a successful build alone.
