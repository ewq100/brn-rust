# INT-W three changed-guard seeded rechecks

New principal grant: max600s cumulative INCLUDING planning/setup/baselines/compiles/rechecks/restore/reap/source+process/evidence cleanup. Prior CW/B/D budgets remain CLOSED and are not reused. Selection frozen before baselines: M01 export eligibility record_in_chat ONLY; M02 note_rows general-ID-set canonical filter ONLY; M03 displayed-row canonical predicate ONLY. These are **manually seeded omission checks**, not cargo-mutants output; no list/automatic/broad campaign.

Frozen candidate at exact9a0ce46de0a0334af6a7215263a3ced422cbab8a, compiled source0efd73f1, crates9097bafba6c0261ae14a1dee9da129d1f94fa0d1. Candidate source/tests never edited. One genuine synthetic-Git archive from final HEAD at `/private/tmp/brn-int-w-guard-20261017/source`, no remotes/global Git config/credential history, fixed SEPARATE `/private/tmp/brn-int-w-guard-20261017/target`. Same literal copy root+target for both baselines and all variants, never candidate artifacts. External crate caches only; Rust1.98.1, locked/offline, CARGO_INCREMENTAL=0, jobs8, fresh explicit synthetic TMPDIR per invocation, no live/download/model assets/GUI/owner data.

## Actual results

Copied unmutated baselines both passed1/0 (app2 filtered; native288 filtered). All three omissions compiled and failed the intended guard assertions. M01's original controller NO_VERDICT is retained: its title-only message matcher was narrower than the frozen export assertion, which checks generated private IDs too. [Raw source/log audit](guard-recheck/raw-audit.json) confirms the failure at exact cw2_review.rs1213 on a generated private identifier; no claim that later title/body assertions were reached. This is an audited behavioral CAUGHT, not a compiler/fixture-error relabel or another run.

|ID|Single omission|Verdict|Actual counts / reason|
|---|---|---|---|
|M01|crates/brn-threads-app/src/export.rs|**CAUGHT (raw audit; controller NO_VERDICT retained)**|[[0, 1, 0, 2]]; Successful mutated compile and failure at the exact frozen general-export private-marker assertion; generated private ID reported before the title in this run. No additional execution or variant.|
|M02|crates/brn-desktop/src/threads_lists.rs|**CAUGHT**|[[0, 1, 0, 288]]; successful mutated compile followed by expected behavioral assertion failure|
|M03|crates/brn-desktop/src/threads_lists.rs|**CAUGHT**|[[0, 1, 0, 288]]; successful mutated compile followed by expected behavioral assertion failure|

[Full per-command input hashes, literal commands, counts, raw log hashes, binary+dep-info hashes/text, per-ID failure excerpts and restore receipt](guard-recheck/campaign.json). Exact original patches and raw logs are in the same directory; [initial binding](guard-recheck/binding.json), [budget start/selection](guard-recheck/start.json), [controller](guard-recheck/controller.py) and [raw controller output](guard-recheck/controller.log). Original normal checks/before proofs/61-command ledger and Opus candidate remain unchanged.

Only a successful mutated compile followed by the selected behavioral assertion is CAUGHT. Compile/timeout/unrelated fixture errors are NO_VERDICT; a passing variant is MISSED. Later assertions after the actual panic are not claimed reached. Native baseline/M02/M03 reuse a literal binary path within the SAME copied source/target as intended; hashes are captured at each execution return, and earlier files can be superseded. They are not claimed to match the latest mutant file. Final target may retain mutant build artifacts, isolated from candidate; all material source bytes are restored and synthetic Git is clean, with no restore test rerun implied.

## Release and limits

[Final timing/source/process/handoff receipt](guard-recheck/release.json) records controller end and precommit cleanup elapsed time (commit-inclusive timing is retained in the external finalize.log), all332 copy+candidate material hashes and owned PGIDs. Heavy slot and ALL follow-up guard evidence paths returned to principal, no source ownership retained. No candidate Cargo/build/test run or source/test change, no new branch, no merge/publication/acceptance. Principal comprehensive Opus/local acceptance remains pending. PDF tooling, hosted/native-human/owner/program and prior mutation Missed/NO_VERDICT/unfinished limits remain unchanged; three narrow omissions cannot establish a general mutation pass or full F21 acceptance.
