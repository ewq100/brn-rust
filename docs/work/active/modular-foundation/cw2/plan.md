# CW2 implementation plan

**Goal:** implement only approved CW2 on the accepted CW1 local foundation.
**Architecture:** existing checked core, scoped read/search tools and ReadLedger remain authoritative. App composes settings/search and AI prepare_run/run_prepared without a second runtime. Candidate evidence uses a host-created scoped Message in the immutable candidate, not a new store/schema or model-declared input list.
**Stack:** Rust 1.98.1, repository lock, Rig 0.44.0; synthetic offline fixtures only.
**Spec:** ../proposal/final/01-master-plan.md §§7.2–7.3, 10 CW2, 13 F01–F22; ../../build-sequence/README.md wins ordering. Current task restricts this worker to CW2 and principal-owned review/integration.

## Baseline and resource gate

Bound path `/Users/evokessler/.t3/worktrees/brn-rust/t3-3205580d` was clean on `t3/cw2-writing-visibility` at `9b47b285d23bb5f89db02ba398244827032fab6b`. Created NEW `work/cw2-20261016` at exact `3e09b979dbe5204098d871469d0ed280186db20d`; local origin/main `8f768943964a77711b26d5235df146f8efe1dc6c`. No reset, root checkout change or additional worktree. Principal notified before source edits. No Cargo/build/test/mutant/stress until explicit heavy-slot grant; widget correction owns the slot. No elapsed-time assumption or polling. Cache reuse needs explicit artifact identity; prefer `/private/tmp/brn-cw2-20261016-target`.

## Exact path ownership

This is the lane-C worker's exclusive proposed edit map, within the task allocation. Existing CW1 evidence stays unchanged.

| Exact paths | Purpose |
|---|---|
| `crates/brn-threads-core/src/{writing.rs,store.rs,context.rs,inspect.rs,lib.rs}` | Candidate provenance types/visibility, immutable request options, actual Apply gate and public exports |
| `crates/brn-threads-core/tests/cw2_visibility.rs` | Historical/Current owner Apply, access restoration, replay/Undo/compatibility witnesses |
| `crates/brn-threads-app/src/{writing.rs,review.rs,lib.rs}` | Shared scoped active context, explicit conversation persistence, review evidence representation/exports |
| `crates/brn-threads-app/src/run.rs` | Settings/selection/search preparation and existing AI worker coordination |
| `crates/brn-threads-app/tests/{cw2_review.rs,writing_context.rs,shared_run.rs}` | Service and scripted contracts including F14/F21 |
| `crates/brn-ai/src/{threads.rs,threads_reads.rs,threads_research.rs,threads_search.rs}` | Actual read bindings, retained coverage, quote verification reuse, v2 reference admission |
| `crates/brn-ai/src/threads_cw2_tests.rs` | Meaningful read/search/provenance/scripted privacy and request tests |
| `crates/brn/src/main.rs` | Current CLI run caller uses shared configuration/start |
| `crates/brn-desktop/src/threads_ui.rs::{start_run,open_note,select_writing_thread,new_writing_thread,ensure_writing_thread}` | Only shared start and active-context consumers; retained UI adapter for B1b |
| `crates/brn-desktop/src/threads_writing.rs` | Only narrowly necessary shared runtime context lookup |
| `docs/work/active/modular-foundation/{plan.md,evidence.md,cw2/**}`, `docs/status.md` | Truthful CW2 plan/proof/handoff |

Excluded: desktop `threads_components.rs`, `threads_writing_tests.rs`, `threads_chat_tests.rs`, other desktop tests, root/all crate manifests, Cargo.lock, CI, other worktrees, D/UI extraction/PC/X. Additional allocation needs principal before edits.

## Contract/source map and decisions

- `WritingSnapshot::{current_inputs,change_request}` preserves v1; v2 setup remains Current. New omitted-by-default v2 request options identify exhaustive checks and owner-selected must-current references. Old payloads/hashes unchanged.
- `threads::{read_records,read_note}` currently reject all rewrites. Permit actual v2 scoped reads, retaining v1 frozen-context behavior. Recheck selected reference visibility in canonical setup scope before content exposure, preserve actual working-Note selected-thread exception. Ordinary actual reads bind Historical; exhaustive/explicit-current binds Current. Model inputs cannot supply authority or evidence.
- `ReadLedger::{slice,complete,shown}` owns actual byte coverage. Capture every entry without top-k truncation; whole reads gain exact totals. Persist exact versions/coverage and host-verified quotations in a dedicated host-created candidate Message. Reuse research verification; model requirements claims remain labelled claims. Reference import coverage, retained read coverage and claims are distinct review fields. Private inputs retain named disclosure and private Message provenance.
- `store::apply_on` currently only checks inputs under authority.chat_binding. Derive writing scope from saved request markers at the checked transaction; recheck every current and historical input using core context::visible, active state and exact historical scope. Owner review cannot bypass. Receipt replay returns the existing receipt without writing again; Undo retains checked compensation. No persisted approval is introduced.
- App active resolver takes `(working Note, opened scope)`; fallback is Note scope then general, never arbitrary linked thread. Inspect does not create. First comment/rewrite creates only when needed; explicit selection/request records scope and conversation durably. Inactive inspection remains distinct from active mutation.
- App already depends on AI, so it owns settings-to-selection/effort and search wiring. Existing `prepare_run(ThreadRunRequest)` remains the sole durable start; PreparedThreadRun stays consumed once. Shared worker wrapper consumes it and invokes run_prepared. UI keeps event presentation/callback fencing; CLI uses the same preparation/configuration.

## Steps and exact checks to run only after grant

- [ ] Add failing-first service visibility fixture: prepare exact writing candidate with a Historical reference, revoke it into another private scope, call review_apply, assert refusal and unchanged Note/receipt. Restore under existing access and require a fresh explicit review call. Include v1 marker, Current edit, ordinary Historical edit, setup edit and private/general scope cases.
- [ ] Extend v2 request options with unchanged legacy encoding; add actual read/ledger/quote candidate evidence and separate review representation. Test complete/partial/overlap/oversized reads, forged bibliography/quotes, unavailable selected reference, historical changed-since, exhaustive Current and F14/F21.
- [ ] Implement scoped active-context resolution/persistence using checked app operations; test two Projects sharing a Note, explicit second conversation, restart, closed Chat and inactive inspection.
- [ ] Share run preparation/selection/search/worker around existing AI seam; replace only duplicated caller paths. Test invalid selection before Run write, consumed prepared identity, scripted cancellation/context/replay paths and existing read/search fences.
- [ ] After explicit grant: run failing-before fixtures against the pinned base using temporary source swaps inside this worktree (no new worktree, restore exact source); preserve test-only commit and exact failing evidence. Run corrected focused tests with locked/offline features, source/artifact receipt and no competing commands.
- [ ] Relevant normal commands: `cargo test --locked --offline -p brn-threads-core --test cw2_visibility`; `cargo test --locked --offline -p brn-threads-app --test cw2_review --test writing_context --test shared_run`; `cargo test --locked --offline -p brn-ai --features scripted-runs`; core/app existing writing/setup/current-input/review/project/read/search/recovery tests. Strict affected-package Clippy; native consumer compile/test per repository features after grant. Default full check only as relevant/allocated, retain PDF/widget failures.
- [ ] Direct installed rustfmt and `git diff --check` permitted now. After corrected tests, coordinate changed-guard cargo-mutants locked/offline, synthetic, total 1800 seconds cap; preserve every per-ID verdict and recheck new witnesses before claiming kills.
- [ ] Commit source pin, actual diff, command/feature/source hashes and exact passed/failed/ignored/unverified counts; hand to principal for fresh Copilot Opus actual-diff review. Do not self-claim independent review, acceptance/integration or proceed to another slice.

## Review focus and limits

F08: top-k is never exhaustive; retained complete is not original coverage/all requirements verified. F10: Historical ordinary edit stays applicable, access loss never does; setup/current edit stale. F14/F21: shared working Note does not leak another scope's setup/evidence through reads, requests, review or search. F04/F18: restart/closed Chat uses explicit scoped conversation, never first linked. F09/F12/F16/F19: checked exact candidate, dirty guards, run fences/cancellation/leases/delayed callback/lost-response/replay/Undo remain intact. F17/F22 retained compatibility/default no-GPUI; full/native qualification remains open. No live providers/model download/GUI/owner data/push/PR/merge/release/delegation.

Exact AI consumer test additions needed for the optional `prepare_changes` writing evidence argument: `crates/brn-ai/src/{threads_history.rs,threads_steering_tests.rs,threads_chat_tests.rs,threads_writing_tests.rs,threads_delegation_tests.rs,threads_sources_tests.rs,threads_project_tests.rs,threads_reads_tests.rs}`. These are allocated relevant AI callers/tests; no desktop test files. `crates/brn-threads-core/tests/writing_setup.rs` receives omitted-default options fields only, preserving existing assertions.

Heavy grant received explicitly from principal after widget child reaping. First process inspection found no executing cargo/rustc/mutant test process (only this inspection's command and unrelated application launch text). Before fixture pin `69d5df82`: core visibility 1 passed / 2 failed / 0 ignored, proving owner revoked/archived Historical bypass. Corrected guard + existing writing/setup/current-input tests pass; detailed receipts retained separately. No shared cache reused.

## Subsequent exact allocations and seam findings

- Principal granted `threads_projects.rs::open_project_note` only: resolve explicit opening Project before conversation choice, preserving refused-open/dirty-review context. The local `note_writing_scope` field is distinct from canonical Project-only `note_project` and from actual selected conversation. Programmatic `set_chat` no longer records owner choice; explicit `open_thread`/writing selection do.
- Principal granted only `threads_writing.rs::review_body` mandatory default disclosure, then a bounded unavailable early return before any new-style private content on wrong displayed scope/access loss; `compare_tile` same checked result before version/text. `threads_ui.rs::apply_candidate` alone consumes app `review_apply_in_context` and requires exact displayed target, preserving the one shared handler/card/footer path. No snapshot-thread shortcut supplies display authority.
- Principal granted ONE new native behavior test near existing review tests in `threads_writing_tests.rs`; existing assertions and both cherry-picked reduce-motion calls remain untouched. It covers default collapsed Inputs disclosure, wrong-context Inputs/Result/Compare without private text, actual Apply refusal and revoked source refusal. Native headless is not human/native observation.
- Exact reviewed widget correction `3cdece37b937ecbd63017d05b2649da870f1ced3` was safely non-overlapping cherry-picked as `aea4830f789a6cb9b3b9318c70fc6e576fb6bb97`, preserving WIP. Its original B cause stays UNCONFIRMED. Source receipts before/after remain separate.
- Read-only tracing found writing-proposal feedback/count preview in `threads_workbench.rs::writing_proposals` and potential saved-request preview through `latest_note_writing_request`; subsequently granted and implemented below. Only `proposal_card` caller is `attention_card` inside `thread_detail`, which excludes writing candidates before rendering. `note_tile` delegates all active review (including Result) to `review_body`; Compare has its own direct candidate_view and needs its granted gate.
- First native command uses task-specific cold target and tests-only disclosure fixture before the disclosure/body/Compare fixes, preserving a meaningful missing-disclosure witness where compilation permits.

Core/AI/app serial whole affected-package run (`--lib --tests`, AI scripted-runs) passed at the recorded dirty source fingerprints. It is not a default whole-workspace or native pass. Earlier test-fixture type/borrow errors, metadata-history leakage caught by existing tests, and interim changed-context assertion failures remain in raw logs; corrected outcomes are separate. No failure was skipped or relabelled passing.


## Source checkpoint, 16 October

The original plan above remains the contract record. All five implementation steps now have source and synthetic witnesses; primary owner-Apply and default private-disclosure failing-before receipts are retained. Heavy slot was explicitly granted; all commands serial and offline in the task-specific target. Final qualification and mutation are pending at this source checkpoint; see [handoff](handoff.md) and [exact paths](path-ownership.json).

Subsequent principal allocations implemented only existing expressions: `threads_workbench::{writing_proposals,latest_note_writing_request,writing_bar_row,writing_run_outcome,writing_result_note,writing_worker_for}`, `threads_writing::note_bar`, and `threads_ui::open_review` implicit conversation-switch fencing. They consume the same app checked displayed-context result/request resolver. ONE native regression was extended, no extra desktop tests or layout work. The old Excluded list applies except for these explicitly granted hunks and the reviewed widget cherry-pick.

The first expanded native suite was **306 passed / 4 NEW task failures / 1 ignored**, distinct from historical baselines. Typed shared preparation errors restored old concise Selection notices/Copilot list, and archived writing context restores explicit Unarchive instruction. No geometry/assertion edits. Corrected native is **310/0/1** at `native-errors-corrected.json` exact source hashes; direct subsequent rustfmt changes syntax only. Affected all-target strict Clippy passes at `lint-source-final.json`. Full source-pin normal verification/mutation and independent Opus review remain pending; no acceptance, hosted CI, human native observation, full F21 or program-green claim.
