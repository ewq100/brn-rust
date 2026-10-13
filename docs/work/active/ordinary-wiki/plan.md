# Ordinary wiki guidance implementation plan

> **For agentic workers:** the lead implements in this session using the repository's native workflow; one persistent independent reviewer checks this plan and the final candidate. The owner's explicit implementation request supplies execution authority; skill ceremonies do not add routine confirmation gates.

**Goal:** package a source-grounded wiki policy for ordinary knowledge, compare it with existing guidance, and provide notes UI alternatives for review.

**Architecture:** retain four packaged skills and nine runtime tools. Put ordinary page admission, evidence, linking, query persistence and bounded maintenance rules in `maintain-notes`, with small intake/base pointers. Use two compiled CLI binaries for the comparison; the runtime/write path is identical.

**Stack:** Rust 1.98.1, Rig 0.44.0, unchanged Cargo.lock; Python standard library harness; standalone HTML/CSS/JS prototype. **Spec:** [Owner assignment and scope](spec.md). **Baseline:** `65119a73`; isolated checkout `normal-notes-llm-wiki-research`, branch `codex/ordinary-wiki-guidance`. Research is committed at `f7e150a9`; runtime unchanged there. Main's unrelated `experiments/` is preserved.

## Global constraints

- SQLite and the shared checked service own all persistent state; protection, exact candidates, versions, dirty guards, replay and Undo remain intact.
- No schema/dependency/retrieval/native UI changes; roles are guidance conventions; preserve drafts and full-import intent.
- Codex `gpt-6.1-sol`, medium, eight tool rounds, two repetitions, identical fixture ordering and prompts per arm.
- Fresh explicitly owned synthetic directories; credentials/private payloads stay out of evidence. Original source retention follows the existing contract.
- UI variants are throwaway, synthetic and separate from the runtime implementation branch. No release or main push.

## Review focus

- A brief or passing mention stays in thread rather than spawning a page; explicit owner writing remains possible.
- Multiple sources for one subject revise that subject without creating redundant summaries or erasing earlier qualifications.
- A newer reported estimate can conflict with a protected commitment; protection and reported truth remain distinct.
- A synthesis follows support to actual source revisions; generated agreement is not corroboration.
- Missing originals, partial coverage and current-input changes preserve limits; preparation/review/Undo remain exact.

## Task 1 — preserve baseline and define the comparison

**Files:** create `scripts/evaluate-wiki-guidance.py`, `scripts/tests/test_wiki_guidance_eval.py`, `scripts/fixtures/wiki-guidance-cases.json`; document usage/evidence in this directory. **Interfaces:** the harness runs existing `brn --data-dir ABS COMMAND` with argument arrays; exports no new production API. `Cli.call(command, **options)` returns decoded JSON or a recorded bounded failure. `run_arm(binary, scratch, case, selection)` returns a snapshot of notes, candidates, relevant receipts and finalized answer. `structural_checks(before, after)` distinguishes safety/persistence from semantic rubric results. `read_operation_inputs(data_dir, operation_ids)` opens only the exclusively owned synthetic SQLite database in read-only mode and returns declared input record/version/expectation fields for the listed Run operations. It performs no canonical writes and emits no complete request bodies.

- [x] Capture `/private/tmp/brn-wiki-evaluation/bin/baseline-brn` and its SHA-256 from a locked build before editing guidance; preserve the shared helper and guide identity `brn-threads-guidance-2`.
- [x] Write failing harness tests: existing root is refused; failed CLI/provider result is retained as failure; snapshots omit Settings/auth paths; protected note mutation fails the comparison; unrelated notes are not reported as undone.
- [x] Implement exclusively created scratch roots, argument-array subprocesses, per-command timeout with controlled cancellation/failure records, and JSON snapshots excluding Settings. Commands operate only under the created scratch root. Profile paths are passed only to configure and omitted from logs.
- [x] Inspect applied derivation inputs explicitly: `brn candidates` contains only pending candidates and receipt history omits inputs. Match whitelisted Run.operations to operation request rows in the owned synthetic database, decode and retain only input fields. Recursively omit Settings records/auth configuration in candidates, receipts and other nested snapshots. Test applied-input visibility and nested redaction with a synthetic fixture.
- [x] Implement three frozen scenarios. Sequential update: seed ordinary Harbor project note and full-imported protected commitment reference (not a confirmed owner decision); intake two supplied Markdown updates retaining estimated dates, metric units, capacity constraint, Estonian attribution and conflict. Synthesis: import two protected access-process references, ask for a reusable comparison, inspect direct source/Note inputs and actual links. Ephemeral: ask a self-contained arithmetic question with an explicit conversation-only request; assert no new Note.
- [x] Read/write raw comparison artifacts as task-local evidence, record binary/scenario hashes, eight-round settings, elapsed time and terminal outcomes. After snapshots, Undo an eligible recorded model note change and verify the affected note's previous content/restoration or created-note archival without touching later work. Explain if no eligible applied change exists.
- [x] Freeze exact prompts, source facts and rubric in `scripts/fixtures/wiki-guidance-cases.json` before guidance edits. Run `python3 -m unittest discover -s scripts/tests -p test_wiki_guidance_eval.py -v` and baseline fixture seed/check paths before live calls. Live mode is opt-in with explicit provider/model/effort/auth selection; offline mode performs seeding/checks and reports semantic evaluation as not run.

## Task 2 — package the wiki policy and verify boundaries

**Files:** modify `agent/BRN.md`, `agent/skills/intake/SKILL.md`, `agent/skills/maintain-notes/SKILL.md`, `crates/brn-ai/src/threads_guides.rs`; update `docs/architecture/threads-target.md`, `threads-behavior.md`, `docs/status.md`. **Interfaces:** `preamble()` and `read_skill(id)` retain their signatures/catalog IDs; bundle advances to `brn-threads-guidance-3`. Typed Note/Link changes and `RevisionInput` retain existing fields.

- [x] Record baseline runtime/core test outcomes; inspect packaged guide tests and existing checked-write scenarios instead of inventing an LLM result with mocks.
- [x] Define update/create/thread admission, optional entity/concept/synthesis outlines, evidence/inference distinctions, supported note/source links, useful query persistence and bounded health review in the maintenance playbook. Keep general authority/write rules in the base guide. Intake routes semantic organization to that playbook without affecting full imports.
- [x] Update the base/catalog pointers so ingestion, knowledge maintenance and durable synthesis load the right guidance. Bump guide identity for diagnosing runs. Retain clear positive steps with completion criteria and representative examples.
- [x] Update canonical summaries narrowly to the owner's accepted wiki direction and explicitly preserve role/protection independence and runtime-only scope.
- [x] Build candidate CLI and preserve `/private/tmp/brn-wiki-evaluation/bin/wiki-brn` and hash; run `cargo fmt --all --check`, focused `brn-ai`, `brn-threads-core` and `brn-threads-app` tests, strict relevant Clippy, then documentation gates. Record failures and limitations.
- [x] Run the before/after pilot with the exact selected provider/model/effort. Independently inspect both arms against the frozen rubric; report equal results or regressions honestly. Adjust only defects justified by observed evidence and rerun affected cases with the prior failed results retained.

## Task 3 — explore the notes UI and deliver a reviewable candidate

**Files:** write `docs/work/active/ordinary-wiki/ui-design.md` on the implementation branch. Create `crates/brn-desktop/prototypes/ordinary-wiki-notes.prototype.html` on `codex/ordinary-wiki-ui-prototype`, in a separate isolated checkout; production runtime code remains absent from that branch.

- [x] Map existing notes/editor/linked work/history APIs and data fields to the proposed designs; distinguish existing observations from proposed conveniences.
- [x] Build three structural variants with identical synthetic knowledge: reading + context rail, subject overview + evidence, connected-note navigation. Keep Omakase colors/type/chrome and maintain visible protection, source availability, conflict and exact history/Undo boundaries. Use `?variant=A|B|C` and a floating previous/next switcher; arrows ignore text inputs.
- [x] Serve the single file with `python3 -m http.server` from its prototype directory. Inspect all variants in a browser at desktop and narrow widths and exercise sources/links/history/conflict switches. Record the design recommendation and what remains owner-selected.
- [x] Independent read-only final review: complete diff against `65119a73`, spec, relevant invariants, harness outputs and UI mapping. Fix verified findings and rerun affected checks.
- [x] Commit task-owned implementation/docs in reviewable units. Where hosted publication is available, create/attach a draft PR with controlled-pilot findings and separate owner checks. Do not merge without current authority. Report implemented, verified, prototype-previewed, owner-accepted and merged separately.

## Gate record

Defined: owner requested implementation/evaluation and selected UI prototype plus live model. Build ready: plan approved in independent round 2; local runtime and final harness checks pass. Controlled pilot and browser prototype are complete; draft publication supplies the reviewable candidate; hosted CI is tracked on the PR, with owner UI selection and integration separate. Candidate/integration evidence is recorded in [evidence](evidence.md). Native observation and UI variant acceptance are outside the automated gate.
