# B1b exact ownership and compiled-window handoff

Worker: Codex Sol6.1, branch `work/b1b-20261016`, base `5e98b0157a70076f2fe69346f1496cccc869bc78`, bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`. The initial source-only phase left the heavy slot with D1A. Principal later granted the exclusive normal window recorded in verification.md; this worker returns the heavy slot after the compiled pin. D1A source/manifests and every other worktree remain untouched.

| Path | Exact owned symbols/hunks | Current action |
|---|---|---|
| crates/brn-writing-ui/src/native.rs | New RewriteStartError and WritingPanel::rewrite_run_start | Context-checked immutable request → existing app ThreadRunStart; no runtime/worker/provider implementation. Existing panel state/render untouched. |
| crates/brn-desktop/src/threads_ui.rs | retry_rewrite; start_run; common begin_run_start/start_request extraction; one corrected adopt_writing_thread comment | One worker path; ordinary queue/EditNote construction retained, writing branch removed. Existing worker event/cancel/channel behavior and presentation messages retained. |
| crates/brn-desktop/src/threads_writing_tests.rs | Append only the two new B1b fixtures | Existing file prefix/assertions/events/reduced-motion setup remain byte-identical. |
| crates/brn-threads-app/src/lib.rs | **Additional principal grant:** one `pub use brn_ai::threads::{OwnerRequest, TraceLevel};` | Re-export existing types only; no app operation/context/AI source modification. |
| crates/brn-desktop/Cargo.toml | **Additional principal grant:** [dev-dependencies] brn-ai path + scripted-runs and rig =0.44.0, default-features=false, test-utils | Matches existing app test dependencies; no production dependency/feature/root hunk. |
| docs/work/active/modular-foundation/b1b/ | Plan, ownership, source/fixture/evidence receipts | Preparation only; no acceptance claim. |

Granted reserve, **no current delta**: narrowly necessary library writing/workbench adapters; desktop threads_writing host adapters and threads_workbench_tests; minimal status/plan/evidence links. None is used merely to fill scope. Do not expand into other source/tests without an exact principal hunk.

**Granted lock hunk:** Cargo.lock now adds exactly the existing brn-desktop package dependency edge to rig. Offline metadata preserved all 953 package/source/version/checksum identities. This edge was explicitly allocated in the heavy grant; no other lock ownership, D1A memory integration, new library AI dependency, manifest/AI upgrade or root/CI/script edit.

`writing_threads` remains the canonical-resolution presentation/draft cache. It is populated by checked open/select/ensure calls and consumed by cache keys, composer draft handoff and contextual display. It is not a first-linked fallback. Deleting it indiscriminately would break accepted CW2/B1a behavior. Only the obsolete start_run routing is retired here.

Worker retains these B1b UI/source/test hunks pending compile/review/fix allocation. No heavy slot was acquired; no permission to build is implied by source readiness. Principal owns independent Opus review, acceptance, integration and any next slice.

**Compiled-window handoff:** [qualification](verification.md) records actual checks and limitations. The heavy slot is released after the compiled local pin and empty process receipt. Retained/frozen source ownership is only the five tabled source/manifest hunks plus this exact lock edge and B1b docs; unused narrow library writing/workbench, desktop threads_writing and workbench-test reserves have no delta and no edits are planned. Principal may reallocate unused reserves; any review fix still follows exact hunk ownership.
