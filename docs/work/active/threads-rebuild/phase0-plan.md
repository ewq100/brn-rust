# Phase 0 and stale-input hardening implementation plan

**Goal:** Complete the five accepted Phase 0 items and fix F1 without replacing the checked-write design.

**Architecture:** Keep one authoritative SQLite database and one transactional checked commit/Undo mechanism. Add explicit current-read preconditions distinct from historical provenance; make small desktop/export corrections and collect a representative synthetic baseline.

**Tech stack:** Existing pinned Rust, SQLite/rusqlite, GPUI Kit, Rig; no dependency or compiler upgrade.

**Spec and authority:** Owner's 11 October 2026 review follow-up authorizes Phase 0 plus F1, isolated worktree, GPT-6.1 Sol implementation subagents, and focused tests. The owner accepts the review's recommended product decisions. Canonical architecture remains [Threads target](../../../architecture/threads-target.md). Existing [verification](../../../development/verification.md) applies. Native observation, owner acceptance, and merge remain separate evidence.

## Scope and accepted product decisions

- Primary workflow: source/email to reliable working note to supported next action/reply.
- Personal macOS use first; no external distribution program in this task.
- Ordinary-note autonomy after one explicit delegation choice (implementation is later Phase 1).
- One provider run initially; Stop-and-send steering is later Phase 1, not this task.
- Adopt non-disposable data after Phase 1 acceptance; support Threads upgrades and backups then. No old-BRN migration.
- Backups should use a user-selected destination covered by separate backup; configuring that destination is later work.
- DOCX is the first later output format. No output converter added now.

## Constraints and ownership

Baseline d768db7; branch work/phase0-stale-input; isolated sibling worktree. Preserve source checkout and read-only sources. Use fresh synthetic data, offline fixtures, no live provider calls or credential reads. No main merge/release is authorized. Lead owns integration/docs/plan/evidence; Sol helpers own bounded implementation files. Shared-file changes require coordination. All canonical mutations continue through the existing checked writer transaction, with replay before mutable precondition checks.

## Review focus

1. Input changes before Apply across independent connections must cause an atomic Stale outcome; intentionally historical evidence remains valid.
2. Existing immutable requests/receipts, Undo derivation restoration, self-inputs, review replacement, and repeated Apply must preserve semantics and compatibility.
3. Home/thread draft changes, successful sends, failed sends, and navigation during workers must not lose or misdirect text.
4. Export metadata must round-trip arbitrary titles/provenance (quotes, newlines, Unicode, YAML delimiters), preserve Markdown body bytes, and keep snapshot hashes correct.
5. Performance reports must separate headless timings from observed UI latency and never treat missing native access as successful measurement.

## Task 1: F1 checked current-read dependencies

Files: core types/store/tests, runtime schema/prepare/read adapters and tests, related application request construction, runtime guide contract as required. Coordinate desktop request literal updates with UI owner; export owner uses a new test file.

Interface: explicit distinction between current-read expectations and historical RevisionInput provenance. Use `RevisionInput.expectation: InputExpectation::{Current, Historical}` with serde default Current and omission when Current. Existing serialized requests retain their bytes/hashes; old pending inputs become current expectations, while new explicit historical citations carry Historical. Runtime argument validation accepts omitted or explicit Current without allowing other unknown fields. The model cannot grant authority; existing target checks and scopes remain unchanged.

- [x] Add failing production-core regression: create input v1 Monday; prepare derived note from current v1; second connection updates input to v2 Friday; Apply must return Stale with no derived writes/receipt.
- [x] Implement atomic current-read validation in apply_on, after successful receipt replay and before any grouped mutation. Preserve explicit historical citations; runtime current reads/prepare must not silently opt out.
- [x] Cover input already changed before Prepare, archive/supersession, mixed groups, exact replay after input changes, intentional historical use, self-input/Undo, and refreshed candidates.
- [x] Run cargo test -p brn-threads-core -p brn-threads-app --locked --offline and focused brn-ai Threads tests; record red/green evidence and exact representation rationale.

## Task 2: Phase 0 desktop controls/drafts

Files: crates/brn-desktop/src/threads_ui.rs, threads_state.rs, threads_focus_tests.rs.

- [x] Reproduce shared-composer draft behavior with a focused state/component test: type in A, switch B, type in B, return A, send; text and destination remain per-context.
- [x] Store/restore Home and per-thread drafts on navigation; show the current destination; clear only successfully submitted draft. Preserve failed-submit text.
- [x] Make worker cancellation presentation depend on actual WorkerKind capability: Run/Import cancellable; Backup/Export visibly finishing without Stop.
- [x] Test each worker kind plus draft Home/thread/navigation/send cases through production presentation logic. Run default state tests and native-test-support component tests where available.

## Task 3: Phase 0 portable export metadata

Files: crates/brn-threads-app/src/export.rs and new tests/export_metadata.rs. Reuse serde_json's YAML-compatible quoted scalar escaping; add no parser dependency to production.

- [x] Add a failing export test for a title absent from body, Unicode/quotes/newlines, and linked source provenance.
- [x] Emit a small YAML front matter wrapper carrying title and source-reference metadata before the original body; preserve source Markdown bytes after the wrapper. Existing front matter remains body content rather than being parsed/reformatted. Keep manifest IDs/versions/hashes and atomic publication.
- [x] Verify metadata decoding/escaping, exact body suffix, asset link rewriting, manifest hashes, occupied destinations, and plain notes without provenance. Run focused export tests and application suite.

## Task 4: Phase 0 representative baseline

Files: a reusable explicit synthetic benchmark under crates/brn-threads-app/examples/ or scripts/qualification, plus compact evidence in phase0-evidence.md. No production optimization in this task.

- [x] Create a deterministic corpus with long notes, managed images, repeated revisions, conversations, and receipts using supported interfaces in a fresh explicit directory.
- [x] Record corpus counts/bytes, build profile/features, source commit/tree, machine, database and bundle size, opening/search/navigation-equivalent read timings and peak RSS using available platform facilities.
- [x] Measure startup/typing/native navigation only through a fresh synthetic native app if UI access and build are available. Otherwise record each unavailable metric and concrete remaining native procedure, never infer from headless timings.
- [x] Keep the fixture bounded and opt-in; verify it refuses existing/nonempty data destinations and does not inspect user data. No strict hardware-dependent CI latency assertion.

## Task 5: Phase 0 current documentation and integration

Files: AGENTS.md, README.md, docs/status.md, architecture overview/target/behavior, development setup/workflow as necessary, this plan and phase0-evidence.md.

- [x] Reconcile implemented/merged baseline with current docs; preserve historical rebuild evidence. Record accepted owner decisions and the current Phase 0 assignment with one clear navigation route.
- [x] Describe current serialized execution honestly; retain later steering as a target, not an implemented claim. Update F1 contract and export format descriptions with actual implementation.
- [x] Run Markdown link checks, relevant tooling tests, format and affected Clippy, focused implementation gates; inspect complete diff for unrelated changes.
- [x] Obtain independent read-only review against baseline and this scope; fix supported findings and rerun affected tests.
- [x] Commit task-owned candidate, preserving separate implemented/tested/native-observed/owner-accepted/merged states. Prepare reviewable diff and concise owner verification steps. Main merge remains unperformed.

## Gate status

- Definition/authorization: satisfied by owner request.
- Plan review: satisfied; independent GPT-6.1 Sol review approved with no blockers.
- Implementation/focused verification: passed; see [evidence](phase0-evidence.md).
- Independent final review: approved; no actionable implementation defects found.
- Native observation: blocked by Computer Use permission denial for the built candidate. Headless baseline recorded; native startup/typing/navigation remain unobserved. Task 4 records this explicitly rather than treating it as a native pass.
- Owner acceptance: pending; prior agreement accepts product choices, not unseen UI.
- Merge/release: outside current authorization.
