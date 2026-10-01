# Markdown note editing evidence

Date: 1 October 2026

Implementation: pending. Product verification: not run. Native/user acceptance: pending. Integration: documentation only; no feature implementation, migration, merge or release.

## Design and planning

- Inspected a clean local `main` at `4f059881f5a34d22ffdf1bb53e79954c94c8608f`.
- User selected explicit Save/Cmd-S, a local vault, no force overwrite, and coordinated journaled exchange with explicit non-cooperating-writer limitations.
- User approved four design sections, then explicitly reviewed/approved the written [specification](../../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md) committed at `32077d8`.
- [Implementation plan](plan.md) prepared from the existing store/workflow/CLI/worker/native structures. Execution choice remains pending.
- The visual companion was declined. No application process, provider, note vault, credentials or model assets were accessed.

## Documentation checks

The specification commit passed `git diff --cached --check`; a local checker validated 47 file/fragment links across its three changed documents. Commit scope and clean worktree were inspected after `32077d8`. These are documentation observations, not application verification.

Planning documentation passed `git diff --check`; the local checker validated 60 file/fragment links across the six changed documents. A red-flag scan found no unfinished markers or deferred-code instructions in the plan. Inline self-review checked spec coverage, task dependencies, interface consistency, replay ordering, copy reconciliation and atomic provider-outcome/currentness bookkeeping. All nine implementation tasks remain unchecked.

No Rust build, test suite, crash experiment or native editing demonstration is claimed.

## Plan review and revision

Opus 5.5 with high reasoning reviewed the plan at `dca2b98` read-only. Its verdict was ready after fixes, without a redesign. The user requested a plan update, not execution. This documentation revision started on clean local `main` at `de0eb45621f9d724cf0fa60be74f36a6ab5c08ae`; the independent workspace-shell design/plan remains unchanged.

| Finding | Revised contract or check |
| --- | --- |
| 1. Global ownership versus test isolation | Directory-descriptor locks share the production identity namespace while using disposable fixture roots; no private lock-directory override. |
| 2. Restart acquisition and multiple vaults | Registry-based lazy root reacquisition, one vault per workspace, explicit unavailable/busy state, recovery/history accessible without the root. |
| 3. Startup-interrupted saves | Dedicated typed note reconciliation; generic Interrupted transitions remain unchanged. |
| 4. Inaccessible crash hooks | Process-crash children run as lib unit tests; integration tests exercise only public reopen/replay interfaces. |
| 5. Unresolved-original gate | Persisted resolution and partial uniqueness constraint, separate from generic operation status and cleanup. |
| 6. Current document surfaces | Shared source projection, explicit state/reason, guarded CLI/native/headless presentation; no old bytes labeled current. |
| 7. Weak evidence tests | Real keyword index and positive query before invalidation, exact error assertions, explicit approval/no-op/reapproval paths. |
| 8. Buffer/save generations | Equal-generation identical-text saves and higher-generation capture; later edits cannot be lowered by completion. |
| 9. Reserved paths and aliases | Preserve registered-path ownership; case/Unicode alias checks veto ambiguous copies and require volume qualification. |
| 10. Intermediate CLI breakage | Exhaustive mappings and affected constructors/callers updated in the introducing task; workspace/native compile gates. |
| 11. Stale ask replay | Typed stale failure with original provider outcome/history, no resubmission; CLI subprocess assertion. |
| 12. Shutdown and close | Critical note jobs drain/join; GUI close waits asynchronously for durable recovery/save acknowledgement or explicit discard. |
| 13. Presenter event route | Dedicated queue to lightweight worker polling, independent of terminal events, with coalescing/rescan and idle observation. |
| 14. Copy/artifact/receipt precision | Explicit absent precondition, preallocated target ID, source/destination receipts, tagged stored failures and retained non-text artifact metadata. |
| 15. Phases/proof/cleanup | Crash checkpoint assertions, missing-proof uncertainty, separately journaled cleanup and protected uncertain recovery after explicit acknowledgement. |
| 16. Foundation feature name | `NSOperation` supplies `NSOperationQueue`; concrete API compatibility remains a compile-gate requirement. |
| 17. Duplicate qualification | Run the integrated default script once; separate optional/native/resource checks and observed native acceptance. |

Feedback was checked against the existing store startup/transition rules, ask replay, CLI exhaustive error mapping and document display, worker admission/shutdown, native draft-only close guard, and integrated verification script. Expected whole-corpus IndexStale remains valid; not every exclusion should be forced into EvidenceStale. Enrollment does not inherit approval, and unavailable native profiles cannot stand in for evidence-filtering tests.

The directory-descriptor lock choice is a planned implementation of process-global vault ownership, not a verified filesystem capability. macOS `flock(2)`/`fsync(2)` manuals and [Foundation's feature list](https://docs.rs/crate/objc2-foundation/0.3.2/features) informed the correction. Directory-lock support, overlapping-root exclusion, file/directory durability, accessor/protocol compatibility and native exact-byte round trips still require execution-time qualification. No assumption that `F_FULLFSYNC` works on directory descriptors or that process-kill checks prove power-loss durability was adopted.

Only this evidence file and the [plan](plan.md) were revised. The approved specification and implementation scope remain unchanged; all implementation task checkboxes remain unchecked.

Revision checks: `git diff --check` passed. The local checker validated 66 file/fragment links across the plan, evidence, approved specification, documentation index, status and active-work index. Static checks confirmed nine unchecked tasks, the reviewed contracts, no unfinished markers/deferred-code instructions, and one nonduplicated final integrated-suite invocation. Inline self-review checked persisted result/resolution types, staging-path crash recovery, mutable observation callers, copy acknowledgements and specification coverage. These are documentation checks only; no Rust/native/provider/vault checks were run.

## Next action

Select execution when authorized. At that time, establish an isolated worktree, implement task-by-task with red-green verification, and append actual commands/results/limitations here. Preserve the original vault and all unrelated work.
