# B1b shared writing start implementation plan

**Goal:** retire the remaining writing-specific desktop start adapter while consuming CW2's existing shared run/context seam once.

**Base:** `5e98b0157a70076f2fe69346f1496cccc869bc78`, clean accepted B1a foundation, same T3-bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`; new branch `work/b1b-20261016`. B1a and every other ref/worktree are preserved. Local origin/main tracking baseline is `8f768943964a77711b26d5235df146f8efe1dc6c`; no fetch/publication or main update.

**Spec:** [master §7.2/§9/§11/F registry](../proposal/final/01-master-plan.md), [lead prompt](../proposal/final/02-foundation-lead-prompt.md), [Stage2](../../build-sequence/README.md), current user allocation, [B1a disposition](../b1a/principal-disposition.md), [CW2 disposition](../cw2/principal-disposition.md), canonical [behavior](../../../../architecture/threads-behavior.md), [target](../../../../architecture/threads-target.md), [invariants](../../../../architecture/invariants.md) and [dependencies](../../../../architecture/dependencies.md).

**Architecture:** the panel produces the existing `ThreadRunStart` from its displayed Note, canonical scope and exact saved request. It borrows the real Workspace to use the existing contextual reader; it owns no runtime or service. One desktop `start_request` consumes that request or an ordinary-chat request through `Workspace::prepare_thread_run`, then the unchanged `spawn_prepared` worker/channel path. Provider resolution, search, durable preparation, leases and authorization remain CW2/runtime-owned.

## Constraints and review focus

- **SOURCE/TEST/DOC PREPARATION ONLY.** D1A has the sole heavy slot. No Cargo/metadata/build/test/lint/mutant/stress/headless/GUI, process polling, lock write or hidden compile before an explicit principal grant. Direct Rust1.98.1 rustfmt and static diff checks only.
- Existing selection/Copilot catalogue tone, continuation/completion messages, queue claims, Stop/lease/Undo, saved v1/v2 readers and first dirty-buffer ordering must survive.
- A retry must not retarget to current Chat or an old cached writing conversation; unavailable/archived/wrong Note/scope must refuse before new run writes. Current input changes keep the existing stale warning and pending feedback.
- The host CW2 cache keeps its complete token/Note/scope/conversation/exact-candidate key and one render refresh. Mandatory private disclosure, early Compare/review refusal and scoped status/worker/result filtering remain untouched.
- No AI dependency in writing UI, new framework/host trait, runtime, manifest version, root/CI/script or AI/core/CLI production change. The two explicit additional hunks are recorded in [ownership](ownership.md).
- Five review inputs: unrelated/hidden Chat; wrong displayed Note/scope; explicit conversation change/archive; saved-input staleness; ordinary chat queue and error/completion presentation. Two new fixtures plus retained native/shared-start suites cover these proportionally; no full F21/native-human claim.

## One bounded implementation unit

- [x] Inspect accepted source and identify already delivered contracts. CW2 `Workspace::prepare_thread_run` and CLI consumption already exist. `ensure_writing_thread`, selection/opening already use checked context. There is no first-linked start fallback to delete.
- [x] Move request construction/current-input check into `WritingPanel::rewrite_run_start(&self, &Workspace, &str, Option<&str>, Vec<String>) -> Result<ThreadRunStart, RewriteStartError>`. Use `writing_request_in_context`, not cached/raw request authority. Return typed Context/StaleInputs failures; build the existing RewriteNote payload with exact saved identity and no chat queue claim.
- [x] Route `ThreadsUi::retry_rewrite` directly to shared `start_request`. Keep ordinary `start_run` chat queue/EditNote request assembly; remove its RewriteNote detection and writing-specific strings. Extract the existing busy/refreshed admission into `begin_run_start`, invoked once before dispatch; keep one provider/search/preparation/worker implementation.
- [x] Prepare two native fixtures in `threads_writing_tests.rs`: real panel Rewrite/Retry catalogue refusal with Chat different/hidden, followed by the same panel request consumed by the shared preparer and existing scripted runtime with actual model-input inspection; and wrong Note/general-vs-project/explicit-conversation-change/archive refusals with exact record/receipt/feedback preservation. Preserve every existing test byte.
- [x] Apply only the separately granted app type re-export and desktop test-only dependency hunks. No lock update in this window.
- [ ] On explicit heavy grant: reconcile only an expected desktop→existing-rig lock edge if needed; execute a separate-target baseline refusal fixture, affected native library/desktop/shared-start/CLI checks, strict lint/format/default F22 graph/build and any principal-bounded guard recheck. Retain all failures and original source associations.
- [ ] Create/qualify a final compiled source pin and obtain fresh principal-owned Copilot Opus actual-diff review/local acceptance. A source-only preparation pin is not this gate.

## Later commands, not executed

Use pinned Rust1.98.1, `CARGO_INCREMENTAL=0`, locked/offline after the separately granted minimal lock reconciliation, own B1b target and fresh synthetic TMPDIR. Failing-before source must have a **different source root and target identity**, never alias the candidate target. Reuse the maintained native script once as the affected build/lint/library44/full-widget lane; no redundant individual F15/A/B reruns after the full suite covers the same source.

```sh
bash scripts/verify-desktop-shell.sh
cargo test -p brn-threads-app --test shared_run --test writing_context --test writing --test writing_setup --locked --offline
cargo test -p brn --test agent_runs --test agent_retry --test agent_edits --locked --offline
cargo fmt --all -- --check
cargo tree --workspace --edges normal --locked --offline
cargo tree --workspace --edges features --locked --offline
cargo build --workspace --locked --offline
```

Principal allocates the serial window and strict/default/combined breadth. A narrowly affected guard check may be justified by the new contextual-reader integration; no unchanged-guard campaign, benchmark or broad stress. Any targeted mutants require a separate grant, failing-before evidence, normal checks/pin and one original cumulative ≤1800s slice cap.

F04/F06/F07: scoped saved request and delivered model input; F09: retained stale input and synchronous dirty guard; F10/F21: contextual retry refusal plus unchanged CW2 privacy/access witnesses (partial program evidence); F12/F15: retained exact Apply/durable Undo/local editor Undo suites; F16: saved retry/reopen/queue witnesses; F22: default GPUI-free graph/build/lint/tests qualification later. Reference-ranking/intake changes and B3/B4/PC are outside this slice. PDF/native pdftoppm, shared external fixture, hosted CI, native human/owner and full program gates remain open.
