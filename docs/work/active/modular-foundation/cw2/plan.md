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
