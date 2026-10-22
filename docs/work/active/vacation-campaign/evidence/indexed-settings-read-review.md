# Independent indexed Settings read review

## Exact candidate and disposition

**Approve source candidate `eae61dde04cda19c764246993966b13b997014eb`, subject to the lead's pending qualification and integration gates. No actionable findings.** Reviewer authored neither product source nor qualification adapter. This approval establishes source correctness for the selected reuse; it does not establish passing builds/tests, measured speedup, native usability, or merge status.

Verified checkout `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/indexed-settings-read`, exact HEAD above, with no staged, unstaged, or untracked changes. Verified merge base is the supplied actual main209 `5fab84d87f2cff3bc03f40fa5987c1fe73be98ab`. Complete base-to-candidate diff is exactly two files, 5 insertions/11 deletions. `/tmp/brn-indexed-settings-read.patch` is byte-identical to that complete diff, SHA-256 `824c02a76e996350fe78faf12c945d130f5f6f297b33bb13de8137f35e34e53f`.

Whole final file hashes:

- core `src/store.rs`: `2f9d8dd582b9e38ec8cbdb45baa24672917bd309dfc5ac96330d9b2513b864f6`
- app `src/settings.rs`: `0441aef20b72dd0d76bed8890f3bf8242efe109127624561968b230db2026aeb`

Applied the independent-candidate review workflow and both requirements/correctness and responsibility/reuse/test-proportionality lenses. Read repository AGENTS, status/index, architecture/invariants, relevant crate contracts, verification/workflow/strategic guidance, the lead's selected task object, author report, actual complete diff, underlying types/query/schema, existing writer uses, Settings consumers, and representative existing tests. The worker's overview is an older campaign projection; the current lead record supplied the selected task authority. A referenced active-work README is absent; the current status/task routes were used instead. No tests, builds, workload, source/Git mutation, account/native/network actions, or helpers were executed. This report is the sole write.

## Requirements and correctness

At `crates/brn-threads-core/src/store.rs:629–631`, new public `Store::workspace_settings()` forwards directly to `workspace_settings_on(&self.conn)`. At app `settings.rs:16–20`, `Workspace::settings()` delegates to that read and retains precisely `AppError::Invalid("Workspace configuration is missing")` on absence. The obsolete app-wide record-loading/Settings-selector implementation is removed completely.

Independently compared extracted base/final helper and index bytes: both unchanged. Existing helper at final `store.rs:3075` has SHA-256 `5521cef1c50153ebb357e60d582ed249b3a3337271cafa6898d1c56e425ed8e2`; existing `one_workspace_settings` index declaration at `store.rs:163` has SHA-256 `160d732afce9caf431a0e3ca2ceb7931b8873938c81e382a705437952a07f1ca`.

The helper selects a row whose JSON contains Settings and whose `archived` value is false, obtains optional absence only through `OptionalExtension`, deserializes the selected full typed Record, and returns the Settings payload. SQL/row retrieval and selected-record JSON/type errors propagate through core Result and the existing transparent `AppError::Core` conversion (`app/lib.rs:60–62`); none is swallowed or converted into default Settings. Missing or archived valid Settings produces None and the unchanged app error. The schema's unique partial index permits at most one Settings record, including archived records, so removing ID-ordered first-match selection introduces no supported-store ambiguity. `Record::archived` is a required bool (`types.rs:466–470`), not an omitted legacy default that this predicate would accidentally lose. Existing defaulted Settings fields retain the same deserializer and serialized form.

This remains a fresh read on the Store's existing connection. No cached settings, caller-supplied snapshot, cross-read atomicity promise, writer transaction change, version, receipt, replay, authority, or privacy rule is introduced. Existing writer uses of the same helper in run configuration/fence and maintenance checks remain unchanged (`store.rs:1669`, `1870`, `2423`, `2975`). Actual Apply still revalidates authority and current settings under its own checked writer boundary; this read method does not grant permission.

The deliberately narrower error surface must remain explicit: `Workspace::settings()` no longer deserializes unrelated record bodies. Valid JSON with an unrelated invalid typed payload can therefore cease to fail this particular accessor. This is consistent with selected narrow-query responsibility and the author report, not an integrity bypass claim. Other reads retain their own validation; malformed JSON may still cause SQL JSON evaluation errors depending on evaluation. No blanket guarantee of ignoring all unrelated corruption, or of validating the whole store, follows.

## Responsibility, callers, and cost claim

Core already owns the query used by canonical policy checks; exposing it through the Store and retaining app-specific absence wording at Workspace is an appropriately small interface. Desktop, CLI, Topics, run preparation, knowledge, skills, MCP, and other Settings consumers continue to use the shared app method. No UI selector, duplicate visibility filter, generic snapshot/cache layer, schema/index, dependency, or configuration is added. Bootstrap's distinct existence check remains unchanged: it intentionally detects any Settings record, including archived state, rather than selecting active configuration. Retaining that route is correct.

The desktop ordinary unconfigured-Topics refresh previously materialized all records directly, then again for Settings, then again for Topics' Settings pointer. This source change removes the latter two full-record Rust materializations. Configured Topics still uses its existing shared current-visible-record logic. SQL planner choice and total latency are not measured by this review; do not promise constant-time SQL or derive a speedup from the earlier single 80.819 ms test-opt1 sample. The existing helper/index reuse is a concrete structural benefit even without a latency acceptance target.

## Tests and qualification proportionality

No new test is required merely to mirror this three-line wrapper or unchanged absence expression. Existing app active/reopen/configuration paths now exercise the public read through the real Workspace consumer. Existing core delegation tests cover legacy/archived denial and missing Settings at the same helper's policy boundary (`tests/delegation.rs:69`, `561`); those do not independently exercise the new wrapper and should not be described as doing so. App lifecycle legacy Settings, theme/run fencing, configuration/pause, killed-run configuration, delegation, Topics, skills/MCP, conversion, backup, purpose, Projects, and desktop suites provide relevant boundary/consumer coverage. Their results remain pending, not inferred from source.

No additional corruption fixture or query-plan/benchmark framework is warranted for the selected forwarding reuse. A new witness would be appropriate only for a later newly promised corruption-detection, plan, snapshot, or latency guarantee. Existing exact serialization/history/replay and checked writer tests remain intact; no behavior test was retired or weakened.

## Inert qualification adapter addendum

Read the complete subsequently supplied adapter, without execution:

- `run.sh` SHA-256 `e153e9d288df5d07f0a83f807c2aa764eb9b96bafbb402ccb6ef2c54c43abad1`
- `supervise.py` SHA-256 `bc3843f5b1a38dae742e5e0a9edb481ad41f1d86da4b67a903550992608cc906`

No concrete blocking issue found in that bounded adapter. It pins candidate/clean state and >3 GiB initial free space, uses the supplied pinned Rust 1.98.1 offline/jobs1 environment, and serializes full core, the 11 named app suites, desktop `threads_ui::`, affected strict all-target Clippy, ordinary native-feature production build, fmt, and diff checks. It launches no native app/provider. The supervisor owns a new process group, samples the 30-minute/1 GiB stop conditions, performs bounded whole-group TERM/KILL plus leader reap and final group verification even if the leader exits first, records unknown/error outcomes, and verifies unchanged candidate/checkout. No overlay or source restoration is present. Qualification also requires zero exit, accepted <=8 MiB log read, a successful libtest marker, production dev-build marker, clean checkout, resolved group, no cleanup errors, and receipt persistence. Receipt failure explicitly removes qualification and makes exit nonzero.

Limits remain the previously reviewed pattern's limits: resource checks are sampled, accepted log size is not a live hard disk quota, post-run Git verification is an operational check, and natural completion will not prove error/timeout branches. Generic successful-log markers do not replace reviewing actual selected suite counts/ignored/skipped output; no zero-test run may be counted. The environment uses a shared target, so the lead should inspect actual compilation/source identity in the qualification evidence as required by repository verification guidance, rather than treating checkout cleanliness as binary identity. No execution or qualification result is claimed here.
