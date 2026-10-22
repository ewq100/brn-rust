# Independent final recovery-copy privacy review

## Verdict and identity

**Approve exact source `ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b` and the exact inert final qualification adapters below, subject to lead-owned execution and ordinary integration gates. Both prior P2 findings are valid/fixed. No remaining actionable findings.** Source approval is not a passing test/build/native result.

Verified clean `codex/recovery-copy-privacy` in `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, exact HEAD above. Complete main211 `aded398a0c1960d1feabe48166284ec0824ee4e1` to final diff is five files, 785 additions/4 deletions, SHA-256 `83628497305f1759f678051093d425c3aabc9a0d64289455bbc674a7e8e39bea`. Entire affected `049819a4` to final delta SHA-256 is `b24e218bd33de0863be95f2ab4982c354ca2c66e1e54583120e313457ea56e4c`: only the two corrected test files plus the new concise behavior contract.

Independently verified both entire production files are byte-identical to the complete prior reviewed candidate `049819a4ae77494dc5da2b0f34e07c3fd1525ca7`:

- desktop `threads_ui.rs`: `4442ae0032cee811c50c28302eefafd51beff825aa8a51cb3540bd68ca130d44`
- app `writing.rs`: `816e51279483226e4ef17ddda6af4632e37528719398d87fdc0ed06d454698f7`

Final corrected files:

- desktop `threads_polish_tests.rs`: `3e0d08fafa562d869306c0f75f6a3e17f40809dfeab67f2b14f7b08832439e5d`
- app `tests/writing_context.rs`: `c0557ab0a892760680309c617011bef00672bbc57664458c69b3a844454696ef`
- behavior contract: `067e1d69e8e59d747582e4fcb7816db38ed8fe9b4c683c78a8eba67fa78f6b0f`

Prior complete review `/tmp/brn-recovery-copy-privacy-review.md` remains unchanged and is explicitly reused for unchanged production, fixture support, original boundary cases and baseline interpretation. I read the entire actual correction delta, reread the changed final app/UI cases in their surrounding existing file context, the added contract and corrected author report, and both complete adapters. Reviewer authored neither candidate nor adapters. No builds/tests/workloads, source/Git edits, account/native/network/cache actions, or helpers were performed; only this report was written.

## Finding dispositions and final test assessment

**P2 dirty-guard fixture: valid/fixed.** The recovered-private-basis UI case now completes the checked general retention before `begin_edit`/`update_buffer`. It then opens the session against the exact older private revision using existing stale-base support. No guarded Note is written afterward to manufacture the fixture. The test verifies the latest displayed Record is public and newer, while the recovered session retains the private basis and exact Unicode buffer; it invokes the actual visible SaveCopy control and verifies refusal, unchanged editor/selection/scope, canonical current and historical records, session, receipts and no general derivative. Using the displayed current Record version instead of `session.base_version` would now admit the wrong copy and fail this test. This fixes setup validity without bypassing a guard or changing production.

**P2 emitted-fence/reopen evidence: valid/fixed.** The existing public-stale app case obtains `Store::prepared(&receipt.operation)` after the actual service commit and asserts the exact Historical(old basis) and Current(observed newer source) inputs. It clones that actual request into a fresh operation, changes only its creation ID, and prepares through normal owner preparation. A second Workspace connection then changes the source into private scope through checked owner mutation. Apply of the pending clone must return Stale naming the source, with unchanged rows/history/private source, absent new target and absent pending-operation receipt. This exposes a missing/wrong Current fence without a fabricated approximation of service inputs or a new hook.

The same case then drops both workspaces and reopens before retrying the original immutable intent against the now-private source. It requires the exact original receipt/copy identity, then checks changed-intent refusal without record/history changes. This now distinguishes replay-before-privacy-admission from revalidating privacy before replay and covers real persistence across connection reopening. The corrected author report accurately identifies public prepared-request inspection.

A bounded same-version public positive case also succeeds and inspects both expectations at the same source ID/version. This checks compatibility of Historical+Current when no stale saved version exists, rather than relying solely on schema/source inference. The other original private/current/captured-context and unavailable-source cases remain; the existing general stale-copy UI positive control is untouched. The removed weaker `needs_refresh` assertion was replaced by direct request and consequential Apply evidence; generic derivation behavior remains core-owned and unchanged.

These are meaningful distinct boundaries, not tests that compute their own privacy policy from the new helper. The selected scope remains general-only recovery, with private duplication explicitly deferred. The new two-line behavior contract accurately describes historical/current general eligibility, captured-scope refusal, stale-public eligibility under a current-version fence, immutable retry and recovery preservation. It does not authorize a private copy/publication feature or add source/history migration.

## Unchanged production assessment and limits

Reuse of `writing_operation`/`writing_replay`/`writing_commit`, exact bound source/basis/context/title/text, historical provenance plus Current writer precondition, same-intent receipt-first replay, owned session basis, and refusal without refresh/reload remain correct in the inspected source. Core Current-input and dirty-guard code is unchanged. The previously reviewed notice ordering still allows a later unavailable replay-target warning to replace the generic saved-copy notice; no independent replay-target notice defect was found. No UI privacy authority, schema, runtime prompt, provider, cache policy or broad lifecycle framework was introduced.

The original before-source witness remains actual 0 passed/1 failed with child 101 at the intended general-Notes privacy assertion. Source/session prerequisites passed; search/export assertions after that failure were not reached. This is retained defect evidence, not a passing final result. The corrected tests and new contract are not yet runtime-qualified by this review, and headless evidence cannot establish native usability or owner acceptance.

## Exact final qualification adapter review

Read complete files and independently hashed:

- `/tmp/brn-recovery-copy-final-qualification/run.sh`: `3e088ba23f24a2f75e377eb1e0cb326449530c3222a3fc4f74c17a571ab1b66f`
- `/tmp/brn-recovery-copy-final-qualification/supervise.py`: `11ae1770dc6406e790d0a82246ec06cc4e8b42d27aa359c6e6a212bb9d12c033`

**Approved for the selected final gate under the lead's exclusive build-slot ownership.** The runner pins final clean HEAD and >3 GiB initial disk floor, uses the previously reviewed Rust 1.98.1 locked/offline/jobs1 environment and synthetic TMPDIR, then performs scoped mutable app+desktop package invalidation. No source overlay/restoration, whole-target/helper clean, frozen bundle change, or native app launch is present.

Explicit stage wrappers retain nonzero command failure and emit pass markers only after success. Stages cover unchanged core `current_inputs`; eight named app suites; all desktop `threads_ui::` tests; affected app/desktop all-target strict Clippy; ordinary native-feature production build; fmt and diff checks. The package-only invalidation and log requirement for actual app/desktop compilation address the known shared-target stale-binary issue. Core current-input implementation/test source is unchanged and appropriately reused, without a redundant full-core replay requirement.

The supervisor preserves reviewed whole-owned-process-group cleanup: 30-minute trigger, sampled 1 GiB floor, TERM 10 s/KILL 10 s, independent bounded leader reap, final group absent/resolved checks, operational error recording, and final clean exact HEAD verification. No competing writer may use the shared target while cleaning/building. Accepted log read is <=8 MiB. Machine acceptance requires exact ordered stage-pass names; all five named new app/UI tests showing actual `ok`; actual app/desktop compilation from this checkout; at least ten nonzero successful test summaries; successful child exit; resolved ownership; unchanged checkout; no cleanup errors; and durable receipt persistence. This excludes a zero-test result or unrelated successful marker from satisfying the selected witnesses. Persistence failure explicitly removes qualification and produces failure.

Bounds remain sampled/accepted-size limits rather than hard OS disk quotas. Stage/named-test/compiler-marker checks supplement raw result review; they do not prove native observation or exercise every operational cleanup-error path. No actual adapter syntax/runtime result is inferred here. After execution, retain exact counts/failures/ignored cases and all raw completion evidence, then apply ordinary required hosted/integration gates to the final reconciled source. No further production or test framework broadening is required by this review.
