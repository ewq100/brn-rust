# Session B — reusable writing workspace and Zorite editor evaluation

You own writing/editor work in the BRN modular-foundation program. Read the supplied master plan and the current canonical `docs/work/active/modular-foundation/plan.md`, contracts, ownership, baseline and coordination records. Session C owns core/retrieval/shared integration; A owns intake.

## Objective

Deliver the **writing milestone** on current components:

- Extract the existing writing UI into a reusable `brn-writing-ui` crate with the current editor (B1a, then B1b).
- Build scoped brief and references with honest Sources used on C's shared changes (B3).
- Prove the same implementation in a minimal development host (B4).

Separately, run the time-boxed Zorite experiment (Z1) against named missing capabilities. Adoption is a later decision. Do not ship a second production app or fork NoteGen/Zed/Zorite.

The user workflow is: open/create a document, attach references, set a brief, write directly, add several passage comments and overall feedback, ask AI to revise using those materials, inspect sources and changes, Apply/reject, Undo and recover after restart. It must work with ordinary chat closed and across separate conversations about the same document within one writing context (master plan section 7.2).

## Preflight and constraints

Read `AGENTS.md`, status, architecture target/behavior/invariants, workflow/verification, current writing-polish/projects plans, relevant writing/core/app/AI/UI code and tests. Inspect actual local state and active owners. Use C's integration ref and an isolated worktree/data directory, and preserve newer work and unrelated changes. Z1 may instead start before C0 from an explicitly recorded existing snapshot, with no production or shared-file changes, and reconciles to C's ref afterwards.

Own `brn-writing-ui`, the writing-related desktop paths and `experiments/zorite-*`. Do not edit desktop writing files until `work/ui-polish` meets the master plan section 11 release condition. That means landed, or owner-paused with its work preserved and not on the baseline; a pause is not a merge. C owns core/app writing operations (`brn-threads-core::writing`, `brn-threads-app::writing`), the scoped writing setup, request v2, candidate provenance, the shared run start and active-writing-context resolution, schema, root manifests, the lockfile and generic scope logic. Submit explicit requests for shared changes. Disclose any minimal local manifest/module patches needed to compile; C reconciles them.

Work locally with synthetic data, review, documentation and commits. Publication/merge/release, real-data access, provider spending or downloads require current explicit authority. Use existing compiler/GUI/Rig/provider pins unless a separately qualified coordinated change is necessary. No computer use on a locked Mac; collect native gates instead.

## B1 — preserve and extract with the existing editor

Characterize current Read/Write, formatting, saved selections, passage and overall feedback, captured request context, Stop/retry, review, exact Apply, rejection, Undo and draft recovery. Keep those flows working while extracting.

**B1a (UI-only extraction; depends on C0 and the `ui-polish` release condition).**

- Move the writing panel and its view-state out of the monolithic `impl ThreadsUi` (`threads_writing.rs` plus the writing parts of `threads_ui.rs` and `threads_workbench.rs`) into `brn-writing-ui`.
- The panel emits host requests such as "rewrite requested". The desktop answers them with its existing `ThreadsUi::start_run`, unchanged; there is no behavior change and nothing is copied.
- The crate depends on `brn-threads-app` and an optional `gpui-kit` (`native-ui`), never on `brn-desktop`, and compiles no GPUI in default builds (F22).
- The existing core/app writing modules are the headless component; do not add a separate headless crate or host traits. Avoid dependency cycles or large generic-common crates.

**B1b (shared-run wiring; depends on B1a and CW2).** Switch the desktop's writing panel to C's shared run start and active-writing-context resolution. Then remove the writing-specific parts of `ThreadsUi::start_run` and the first-linked-thread fallback. Never copy run orchestration to get around this dependency.

Use a thin editor seam inside the crate, not a public trait, while there is one editor. Do not copy a second workflow into the development host. Separate extraction from editor/dependency replacement in reviewable slices.

## B2 — Zorite qualification, not assumed adoption

Z1 is a time-boxed experiment in `experiments/zorite-*`. It may start before C0 from a recorded existing snapshot. It does not gate the writing milestone; record its result in master plan section 17 as passed, failed, blocked, pending or deferred. Target the named missing capabilities: (a) range geometry for margin comments, (b) live WYSIWYG, (c) IME.

Verified facts:

- `zorite-editor 0.11.2` is on crates.io under MIT and depends on `gpui-pre ^0.3`, the same family as BRN's 0.3.9.
- The Zorite application is GPL-3.0-or-later; never depend on it. Its workspace patch is not inherited by consumers.
- Offsets are UTF-8 bytes. `replace_range` emits no `Changed`. `set_text` clears undo history.
- There is no read-only mode. Geometry is caret-only.
- WYSIWYG uses `zorite-markdown`, a second Markdown interpreter.

Gates, in order:

1. **Stock compatibility:** unmodified `zorite-editor =0.11.2` compiles against `gpui-kit =0.7.3` with no patches and no pin upgrades.
   - Normal pinned crate acquisition is fine.
   - A cache miss or unavailable network is an environment blocker, not a result.
   - If stock compilation fails, record "stock incompatible". Any bounded adaptation is a separate recorded decision.
2. Headless checks of offsets, read-only enforcement and rendered-to-source mapping.
3. Native checks on the target Mac.

Compare the current editor and the pinned Zorite package using the same fixtures and host operations. Inspect actual public APIs, published-package contents versus upstream workspace patches, license/source boundaries and native dependencies.

Resolve one compatible GPUI dependency graph with BRN's current GPUI Kit. Do not upgrade or replatform the interface to make one demo compile. Report stock compatibility separately from any adaptation. An adaptation names its patch, owner, tests and maintenance/upstream path, and is decided separately.

Mandatory evidence: exact rendered-to-source selection; correct occurrence in repeated text; Unicode/graphemes/emoji/CRLF; scroll/wrap/resize/font/first-layout geometry; native keyboard/mouse/clipboard/IME; safe programmatic replacement; real read-only/review state; no implicit save on display refresh; formatting round-trip; long-document performance; current safe Markdown/asset/link policy; exact review identity; and coordination between local editing Undo and durable BRN Undo.

Native tests must exercise real interactions on the target Mac. Headless tests or screenshots alone do not establish IME/selection usability. Report unavailable native gates explicitly.

Choose adopt, keep-current, or a bounded adaptation with exact tests/owner and a maintenance plan. If Zorite fails, continue writing with the current editor. Do not add an indefinite fallback, unbounded fork or duplicate OOXML/Markdown interpreter.

## B3 — persistent writing context and sources

Use existing BRN project/document links, the core `visible()` predicate and the existing scoped search/read tools. Master plan section 7.2 fixes storage, scope, capture and input binding, and C implements them (CW1/CW2). A workspace retains one editable document, references, optional brief, comments/overall feedback and associated conversations. Do not create a competing project membership/permissions system or duplicate canonical notes per project.

Support existing BRN reference notes. A file becomes a reference through the existing full-note import followed by attaching the resulting note; writing never consumes conversion results. References and the brief belong to the writing context (working Note plus scope), not to one chat. Within that context they persist across new/switch/closed chat, note navigation and restart. They are never shared automatically into another context of the same Note. The writing tile shows the active context and conversation, resolved by C's shared service, never an arbitrary linked conversation. A reference no longer visible in the request's scope is excluded and shown as "no longer available here" (F21).

Brief/reference-purpose metadata can express audience/tone/requirements and whether a reference supplies requirements, evidence or style. It must not become a global taxonomy or grant authority. Reference content is read-only to writing; embedded source instructions remain untrusted.

Capture is defined in master plan section 7.2. Preserve safe explicit Save-and-rewrite/Save-and-comment semantics. No silent save to start a run. Unsaved-snapshot requests are out of scope for this program.

Targeted edits use the existing scoped search/read tools. Show three things separately (master plan section 7.2):

- the reference's import coverage;
- the retained text read ("read in full", "partially read (X of Y bytes)" or "not read");
- the requirements reported as checked, with verified quotations.

An exhaustive check is "complete over retained text" only when every requirements reference version was read in full; otherwise it is partial. Never claim full original-file coverage or that all requirements were verified. Do not paste every reference into every request or confuse serialized-byte limits with model token limits. Attached does not mean read in full.

Show actual Sources used with stable retained revisions/locators and validated quotations. Do not invent bibliography entries or treat a quote match as proof of all generated claims. Distinguish available/used/incomplete/changed/removed/inaccessible source state. Access revocation must clear/restrict cached previews as required by the host.

A proposal changes only the working document. Keep exact candidate identity and atomic checked Apply. Apply section 7.2's binding:

- Setup edits (brief, reference list or purposes) make the candidate stale.
- Ordinary reference edits leave it applicable, with "changed since" against the recorded version.
- A reference in a request that requires current material makes the candidate stale, and Refresh is needed.
- Access loss refuses Apply. No earlier approval or automatic retry applies it.
- v1 requests and saved candidates keep working.

Retry preserves identity/snapshot only where allowed. Refresh/regeneration needs a new request and new review. Cancellation fences late results.

## B4 — development host (depends on B1a and CW2; an editor cutover is a later assignment)

Build the development host as a binary or example of `brn-writing-ui` behind `native-ui`. It opens a real `brn-threads-app` `Workspace` in a disposable directory and uses C's shared run start and active-writing-context resolution, with a scripted provider for deterministic journeys. It does not depend on `brn-desktop`. Mocks alone do not prove reuse. No separate production data model, sync system, installer, settings product or release.

Deliver the Z1 report separately. Any editor switch after a positive report is a separate owner-approved slice that reruns writing, native and security gates and removes replaced routes.

## Verification and handoff

Run the F01–F22 cases that the master plan section 13 registry assigns to B or to the writing milestone. They cover:

- persistent context and active-context resolution;
- exact repeated/Unicode selection;
- multi-comment rewrite with Chat closed;
- reading statuses;
- cross-scope setup and references (F21);
- malicious references;
- stale draft, setup and reference changes, and access revocation;
- exact Apply/replay/Undo;
- local undo after refresh;
- cancellation and restart;
- the default Linux build (F22);
- unchanged ordinary chat/projects/tasks behavior.

Hand C exact commits/contract revision, paths and shared-file patches, the Z1 result status and stock-versus-adaptation findings, native observation and limitations, source-context tests, independent review, data/rollback compatibility and remaining gates. Update relevant canonical docs and runtime guidance through coordinated ownership. Separate implemented, tested, live-verified, native-observed, accepted and merged; do not claim a finished product based on a successful build alone.
