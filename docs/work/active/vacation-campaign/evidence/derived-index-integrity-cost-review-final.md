# Corrected cost supervisor: independent complete review

**Exact final disposition: approved supervisor `a6ccf7aa8c0dc1346c934674c226a8de16c0c2600b17cfc3cf13b0b85e76dca8` with unchanged runner `cccea7ea4ebe0d91d649d6606801230ba17bf6f0dbf272eac21d9bc239f34a62`, for the one selected strict cost experiment. No outstanding findings.** The complete v1 review and its small remaining P2 are preserved below; the exact-final follow-up at the end closes that finding and controls the final verdict. No workload has run in this review.

Reviewed complete supervisor **`ce2958c7938455fd72a47369352ae75572cb720009cb32bcf9d1d258a3593cdd`** (185 lines), complete unchanged runner **`cccea7ea4ebe0d91d649d6606801230ba17bf6f0dbf272eac21d9bc239f34a62`**, author `cost-supervisor-correction.md`, and actual temporary source/patch at source HEAD **`a499ae47c81de2e9101676743d164d6e5ca2cb9b`**. Source inventory remains only the expected unstaged `search_index.rs` overlay. The original supervisor `c06b33018b60bcc7a20cd27daff76ffe10a6c5c782270aacd54236ee222dfcac` and original located review `cost-review-initial.md` are preserved unchanged.

**Verdict at `ce2958`: one remaining P2 evidence-outcome correction before launch.** Whole-group shutdown and independently guarded source/receipt cleanup are valid/fixed. The cost source and unchanged strict-witness interpretation remain approved. This reviewer authored none of the scripts/corrections and performed static reads/hashing only: no workload/test/build/fixture/CLI invocation, signals, source edits, account/native actions or nested helpers. This report is the only write for this review.

## Remaining P2

**Mark receipt persistence failure nonqualified before fallback printing.** Locations: `cost-supervise.py:173,175–185`.

The new `receipt['qualified']` field is computed before writing `cost-completion.json`. If an otherwise successful test/restoration is followed by receipt-write failure (for example an unwritable evidence directory or full filesystem), the exception sets only `receipt_error`; stdout then embeds `qualified:true` alongside `receipt_written:false`. The final return correctly becomes nonzero, so this is not an exit-zero bug, but it leaves contradictory qualification evidence for the same failed persistence path. The selected experiment requires a retained completion receipt; the initial review explicitly classified missing receipt as nonqualified/inconclusive.

Set `qualified` false (and preferably an explicit failed-evidence outcome/reason) on receipt-write failure before fallback printing. Preserve nonzero exit. This is a small operational-evidence correction only; no experiment or additional product change is needed to establish it by source inspection. A complete supervisor receipt should not claim qualification when it failed to persist.

## Accepted original P2 disposition

The original group-lifetime defect is valid/fixed. `start_new_session=True` creates the selected owned group; `group_present` probes that group separately from the shell leader and represents permission/other probe errors as unknown. Shutdown attempts TERM with ten-second group grace and KILL with another ten-second group wait whenever absence is not proven, even when the leader has exited. The leader is separately waited/reaped. A final absent-group probe plus reaped leader is required for resolved state. A zombie leader may keep the intermediate group probe present until its separate reap; the final probe is authoritative and the path remains finite. No arbitrary PID/name/unrelated-group targeting was added.

Unknown/alive group state prevents restoration and leaves `source_restored_exactly` and `qualified` false. Unresolved cleanup, signal/probe/reap errors and restore errors cannot produce a successful supervisor exit. OS refusal is recorded, not converted into claimed termination.

The original skipped-restoration/receipt defect is also valid/fixed for the inspected operational paths. Signal/group-wait errors are contained before later cleanup stages; source read/hash, write, post-write verification, final disk observation and receipt persistence have separate guards. Unexpected source bytes are preserved. The saved original is hash-checked before any process launch; failure there safely refuses the experiment without claiming it ran. If the original is valid and the group is resolved, restoration writes only over the known overlay hash and requires the exact original hash afterward. A failure during write still attempts verification and records its result. Receipt construction records group status/errors, source restoration/hash and disk observation instead of relying solely on test exit status. The remaining persistence-outcome contradiction above is the only outstanding finding.

## Whole-runner and experiment reassessment

Reconfirmed unchanged hashes:

- Patch `165ddcde548738d1e05b7b4082a7239d3bab9cfc81ca3acf403c8a6018d5547c` (14 added source lines).
- Actual temporary source and saved cost-only copy `61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9`.
- Preserved original and actual HEAD source `94efb75c085c933130196aa2681aa47624674f6a2a3074522c1280992044cc70`.

Reused the complete source/requirements assessment in `cost-review-initial.md` after rereading the complete actual runner/supervisor and verifying those identities. The runner retains exact HEAD/hash/only-modified-file/no-untracked guards, pinned Rust environment, greater-than3 GiB initial free-space gate, named synthetic TMPDIR, strict release mode, locked/offline dependencies, jobs1 and one exact unchanged `user_scale` test selection. The test itself creates a fresh canonicalized temporary fixture. There is no retry loop or target weakening.

The overlay checks four healthy FTS lanes in one Immediate transaction only on usable admission, commits and returns Existing. It adds no recovery classification/rebuild/canonical write semantics. Worker caching avoids repeated checks on same-worker queries, but a fresh process/worker pays admission. The test measures that total preparation/reopen cost rather than just isolated SQL time.

The supervisor's30-minute stop and1 GiB free-space floor are sampled every five seconds, followed by bounded group/leader cleanup grace. This is not a hard live disk quota or an instantaneous30-minute cutoff; compilation/CPU/RAM acceptance is not otherwise newly qualified. Lead serializes shared-target work. Hash restoration and clean source status require final lead verification; an experiment log is not a committed product candidate.

A failure **at the unchanged reopen assertion** (under50 ms and under quarter cold-build) is valid negative cost evidence: actual reopen/cold times are printed by that assertion. It can justify rejecting/defering naive automatic admission scanning while leaving the confirmed integrity defect unresolved. Compilation/setup/integrity-command/outer-stop failures are inconclusive for that timing question. A green run would qualify only the bounded synthetic Note-heavy corpus; empty/negligible Message lanes do not establish Message-heavy cost or recovery correctness. Do not repeat, weaken the budget, silently relabel failure as success, or treat this cost-only overlay as production repair.

After the remaining tiny evidence correction, preserve its new exact hash and source disposition before the lead's one experiment. Retain actual test result/location/timings, raw log, completion receipt, cleanup status, restored hash and final source inventory. No runtime result is established by this static review.

## Exact final follow-up and approval

The actual final supervisor hashes to **`a6ccf7aa8c0dc1346c934674c226a8de16c0c2600b17cfc3cf13b0b85e76dca8`**. Independently read its persistence exception and verified the entire final delta without executing the supervisor: removing only the two added lines `receipt['qualified']=False` and `receipt['evidence_persistence_failed']=True` in memory reproduces the complete previously reviewed **`ce2958c7938455fd72a47369352ae75572cb720009cb32bcf9d1d258a3593cdd`** hash. No other final supervisor change is accepted implicitly.

**Remaining persistence P2: valid/fixed.** A failed receipt write now makes fallback qualification false and marks evidence persistence failure, while the existing final return remains nonzero. The original v0 group-lifetime and independent-cleanup findings remain valid/fixed by the fully inspected v1. All findings are closed at this exact final identity.

Reconfirmed unchanged runner, patch and saved source identities: runner **`cccea7ea4ebe0d91d649d6606801230ba17bf6f0dbf272eac21d9bc239f34a62`**; overlay patch **`165ddcde548738d1e05b7b4082a7239d3bab9cfc81ca3acf403c8a6018d5547c`**; temporary source **`61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9`**; exact original **`94efb75c085c933130196aa2681aa47624674f6a2a3074522c1280992044cc70`** at source HEAD **`a499ae47c81de2e9101676743d164d6e5ca2cb9b`**. The final supervisor retains whole-owned-group probe/TERM10s/KILL10s handling, separate leader reap/final group proof, no restoration when group state is unresolved, independent hash-guarded restoration/verification and receipt attempts, sampled30-minute/1 GiB stop conditions and unchanged initial3 GiB admission. Operational errors cannot produce a successful qualified result through these reviewed paths.

**Approved for one cost-only strict release Mac experiment; not a recovery candidate.** The unchanged50 ms/quarter-cold assertion remains mandatory. Its intended timing failure is valid negative cost evidence; compilation/setup/scan/outer-stop failure is inconclusive for the latency question. The Note-heavy fixture does not qualify Message-heavy scan cost. Source identity, exact restoration, full actual outcome/log/receipt and group cleanup must be retained. No repeat, budget weakening, product adoption, native/account action or wider framework is implied by this source approval.
