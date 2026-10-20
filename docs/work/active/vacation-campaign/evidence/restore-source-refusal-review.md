# Independent review: restore source refusal

Reviewed repository: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`.
Branch: `codex/restore-source-refusal`.
Candidate: `166d823232b1d4b7785630f90e99cb899999f60f`.
Baseline and verified merge base: `a23f486b109c67e8d07c77800030e527b9a18002`.
The checkout was clean on entry and after inspection. All four changed files were reviewed, including the entire actual diff and relevant surrounding implementation/callers.

## Findings

No actionable source or contract findings.

The production change at `crates/brn-threads-core/src/restore.rs:36-49` catches only `Error::Sqlite(SqliteFailure(...))` with primary `NotADatabase`, and only from source `validate_schema`. The source remains opened read-only. All other source-open/validation errors propagate as before, and copy, schema upgrade, post-copy validation, staging, filesystem and publication paths are unchanged. Validation still precedes `prepare`, so the new refusal occurs before destination-parent creation, abandoned-stage sweeping or stage allocation (`restore.rs:50-73`). No schema, dependency, native UI or shipped runtime-guide change is present.

The new core error remains owned by the existing restore service rather than CLI presentation. The CLI calls that service before workspace opening (`crates/brn/src/main.rs:983-987`). Existing mappings produce `unsafe_directory` (`contract.rs:580`), rejected/exit 6 (`contract.rs:50,236`) and retry `refuse` (`contract.rs:1143-1146`). The new text tells the caller to choose a BRN backup file. The contract meaning and README error table match that limited extension and retain contract version 3.

## Witness effectiveness

The strengthened existing core test (`restore.rs:431-472`) checks typed refusal for literal non-SQLite bytes alongside newer-schema, foreign-SQLite and empty-file sources; it also verifies unchanged source bytes, no nested destination parent and no restore stage. It is a meaningful regression assertion rather than an implementation mirror.

The process-level CLI extension (`crates/brn/tests/agent_contract.rs:1256-1282`) invokes the built executable with literal non-SQLite input and a missing destination parent, checks rejected/unsafe_directory, actual exit status 6 through the shared envelope helper, retry refusal, understandable BRN-backup guidance, unchanged input bytes and no destination/parent/stage. Existing `assert_error` also checks absent result/outcome and an empty changed list. Existing restore witnesses retain successful nested/empty restore, occupied-destination collision, source preservation, copy-failure cleanup, symlink refusal, abandoned/live-stage handling and preservation after post-publication failure. In particular the direct `copy_from` malformed-source witness still expects a SQLite error, supporting the intended validation-only classification boundary.

## Observed checks and limits

No builds, tests, network/native/account actions or nested helpers were performed by this reviewer. Guidance read: repository AGENTS, architecture/invariants, core README, verification/strategic-programming guidance, campaign scope, independent-review skill/resource policy and the installed Rust review checklist. The external best-practices link was not fetched under the read-only/no-network assignment.

Read existing `/tmp/brn-restore-refusal-qualification/red.log`: the strengthened core regression test failed once against the original behavior (0 passed, 1 failed, 43 filtered); this is consistent with the actual changed assertion. The lead supplied the original-source identity; the log itself does not embed a commit hash.

At the review observation, `/tmp/brn-restore-refusal-qualification/green.log` records the core library suite passing (43 passed, 1 ignored), including every restore test. The broader command then failed in `mcp_policy` with four `DataFolder::SymlinkRefused` errors on `/var/folders/...` temporary paths. The serial script has `set -eu`, so its later agent_contract and Clippy commands were not reached in that attempt. That is an actual failed qualification attempt, not a completed green lane. Lead disposition and a completed applicable check run remain required before integration. This review makes no native usability, owner acceptance, merge, release or cross-platform execution claim.

Lead owns test-failure disposition, any subsequent candidate changes and integration. Source review applies to the exact clean candidate above; material changes require affected review evidence to be refreshed.
