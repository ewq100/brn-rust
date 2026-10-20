# Independent review: Action completion witness and complete composition

Verdict: **no actionable findings** in either the test-source candidate or the complete actual composed candidate. Requirements and correctness lenses passed bounded independent source/evidence review. Integration remains with the lead.

## Exact reviewed identity and scope

- Complete candidate: `a5cf01304dbe5d8b6da353cd46512f6b343fd6a0`.
- Complete-candidate checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`; HEAD matched and checkout was clean, including untracked files, at final inspection.
- Base and independently verified merge base: `20fcd7383662cd2f67607c3ce04194aed50809b5`.
- Complete inventory: 28 paths: one Rust test file plus 27 documentation/evidence/archive paths. Every changed path was inspected, including decompressed archived evidence. No hidden production, runtime prompt, dependency, manifest, configuration, CI or script change is present.
- Test-source candidate: `12679ef4cd7248e44efb1d89ce37f59352ac2fd8`, inspected initially in clean `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/action-completion-witness`.
- Test diff: only `crates/brn-desktop/src/threads_tiled_tests.rs`, 71 insertions and no deletions. Independently computed binary diff SHA-256 `6918165362dc2c7a419d49f99951a9f04195fbdee493a85f2ecb266e7805eda0` matches qualification. All `crates`, `agent`, root Cargo/lock/toolchain, `.github` and `scripts` bytes match between the source and complete candidates.

Read repository AGENTS, desktop README, current status/index/overview, architecture/invariants, workflow, verification, relevant strategic/Action behavior guidance, reviewing-product-candidates skill and its shared resource policy. Current review authority governs; historical campaign records grant no new action authority. The user-supplied `docs/work/active/README.md` route is absent in this candidate; current repository guidance routes active work through `docs/work/overview.md`.

## Test requirements and effectiveness

Reviewed the entire changed test, fixture/click helpers, retained Waiting/Undo sequence, task-row callback, shared checked change service, History callback and shared Undo service.

- Original Waiting menu journey and first receipt/Undo assertions remain intact at `threads_tiled_tests.rs:348–403`; no extra test attribute, parallel backend test or new framework was introduced.
- Lines 407–422 capture the actual restored record, return to Actions, require the open group/button and click rendered `done-action-{action.id}` through the existing toolkit click helper. The actual `action_row` callback in `threads_tiles.rs:950–989` reaches `ThreadsUi::change`; the test does not substitute a direct mutation for that UI event.
- Lines 425–433 independently specify Done plus exact owner completion evidence. Full Action equality also preserves description, due date, chat, internal and retention fields. Expected state/evidence are not obtained from the production transition helper.
- Lines 435–441 require exactly one additional receipt with exactly one write to the intended Action, complete after-record equal to persisted state, and complete before-record equal to the actual post-Waiting-Undo record. Capturing this real record correctly respects compensating versions rather than incorrectly expecting the initial version.
- Lines 448–457 find the completion operation's exact receipt and its Undo control and click that operation-specific control. The production receipt callback captures and passes that exact operation to checked Undo.
- Lines 460–473 require the original full Action data, version `initial + 4`, exactly one compensation receipt, the completion operation retained at its original history position, and `undo_of` equal to that completion operation. Intermediate Done is `initial + 3`; Waiting and its earlier compensation account for the first two advances.

No ineffective new assertion or actual identity/version mismatch was found. The extension checks the selected completion/evidence/receipt/compensation journey within the existing boundary and introduces no duplicated persistence or authorization implementation.

## Complete evidence and status composition

Action qualification: independently verified all 11 raw archives against recorded compressed hashes, decompressed hashes and original byte counts. Decompressed original failed runs support `lead_log_reconciliation`: `focused.log` is the compiled missing-Actions-control failure while History is focused; `focused-rerun.log` is the version-3 versus initial-version-1 before-record failure. The unchanged helper run-record has inaccurate file mapping, explicitly corrected by this reconciliation. The selector failure is reported only and its raw output is explicitly not claimed archived. Original formatter failure and subsequent final success records remain visible.

Saved focused log compiles desktop from the reviewed checkout and reports 1 passed / 333 filtered; full suite compiles the same checkout, includes the named passing journey and reports 333 passed / 0 failed / 1 ignored. The existing ignored intake test requires explicit CLI shared-intake inputs. Strict all-target Clippy finishes successfully. Final formatter/diff logs are empty, with success reported in the run record. The dependency `block v0.1.6` future-compatibility warning remains visible. No reviewer rerun or independently captured original shell exit integers are claimed.

Native-build qualification: verified the archived build-log hash and successful native-ui build output, plus preserved executable length and SHA-256 `65083ccc34d1e7d6ed9f9c7533f1df7dacaeb57dcd7c5237c3972b19d7f1c8dd`. Independently compared relevant production/source/config paths across reviewed export `0f64a0344eac23edb43403034bf3e0deae87655c`, build/published head `7225c20877edcb5c4dd20d50d530095682d10c40` and integrated base `20fcd738`; they match. Qualification requires reconciling source/helpers before actual native use and explicitly says not launched, no owner acceptance, resulting-main pending.

Integration receipts: PR180's new dated resulting-main observation names `02445b28` / run39392850688, shows all four required contexts successful and retains optional Windows failures/overall failure. PR181 preserves initial pending observations, actual live-protection contexts, exact-head run39392977994 successes, preceding-main satisfaction, normal merge to `20fcd738`, and resulting-main pending. Campaign/overview agree: progress-record task complete; export task remains active with required resulting-main pending. The new witness itself remains pending independent review/integration at its saved observation. No fully verified export/main/native/acceptance claim is made.

CI timing: reviewed corrected report, JSON and decompressed original. Corrected ancestry and Windows PR-lane wording are explicit; independently verified inspected run SHAs are ancestors of `20fcd738`. Job elapsed values and start-offset limitations are consistently described; step timing remains helper-inspected and lead's six-job API verification is a reported observation, not a new reviewer API fetch. Two different-commit/cache runs establish no safe test/cache/lane removal. No CI change occurs.

Other changed records: Action/history audit explicitly names its inspected historical checkout/source and selected coverage gap. Evidence index routes to current qualification. Workflow/test-family headers label earlier inventories as historical and point current execution/gates to campaign state/overview. Remote Page receipts preserve dated sequence6/7 update outputs and readback claims with browser/mobile limitations; they do not imply a fresh reviewer readback, rendering, membership or schedule verification. Current overview is consistent with the campaign changes inspected. Lead-reported documentation-link, renderer and diff checks passed; reviewer did not rerun them.

## Limits and handoff

This review performed read-only source/Git/archive/hash inspection, no builds, tests, mutation experiments, source edits, data changes, live provider/account access, native interaction, PR/Page operations or delegation. The only review output write is this file. Saved check results are inspected evidence; they are not fresh reviewer executions. Toolkit/component and canonical-state witnesses do not establish native OS interaction, focus/rendering/IME usability, owner acceptance, merge or release of this composed candidate. Required hosted head/resulting-main gates remain with the lead and require applicable actual observations.

No correction requested. Bounded complete-candidate review is finished. Material changes beyond review-receipt/current-gate observations need affected review refreshed.
