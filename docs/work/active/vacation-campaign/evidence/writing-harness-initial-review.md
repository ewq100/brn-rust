# Independent compiled writing-harness review

Disposition: two consequential findings. Do not treat this revision's `verify` success as sufficient qualification. Fix finding 1 before qualification; finding 2 can be eliminated by requiring exact `data_directory` binding for this disposable consumer. No account dispatch is authorized by this report.

## Findings

1. **[P1] Standalone verification can turn a deadline failure into a pass.** `src/main.rs:505-508` requires only a `Completed` terminal and a presently useful candidate. `execute_deadlined` explicitly permits `DeadlineObserved("Completed")` during the five-second cancellation grace (`:790-798`), and `run` records that state with `deadline_fired=true` / `failure_kind=ClientDeadline` (`:419-420`). `run` correctly refuses qualification because its `useful` predicate excludes deadlines (`:437-440`), but a subsequent ordinary `verify DATA` ignores that exclusion and the saved failed `saved_outcome`, and prints success. Trigger: a complete useful proposal exists and the production future settles Completed during cancellation grace, for example while terminal persistence is finishing. Require the saved non-deadline successful result predicate in verify, with matching candidate/run provenance; preserve the deadline outcome as failed or unknown. This is an evidence-correctness failure, not a direct Apply authority escape.

2. **[P2] The evidence-directory fallback does not make a reserved attempt one-shot.** `src/main.rs:754-774` accepts any DATA descendant of an attempt's `evidence_directory` when no exact `data_directory` is present. The only durable dispatch/result exclusion is inside that DATA root (`:290-294`, `:363-366`); the ledger row stays `reserved` because updates belong to the lead. Trigger: after a crash/uncertain launch from evidence/data-a, restarting against a newly prepared evidence/data-b with the same reserved attempt still passes every reservation check, while session/day counts include only one ledger row. This breaks restart-safe attempt accounting on the supported fallback path. Require the reservation's exact canonical DATA path, or place an exclusive attempt-scoped durable marker at a shared reserved location. For the intended single launch, a lead-created exact `data_directory` reservation already avoids this path; retaining the broad fallback still leaves the advertised guard incomplete.

## Scope and identities

Read all 895 lines of actual `src/main.rs`, Cargo.toml, README, build-binding, feature-tree and compiler evidence; parsed the complete 3,129-line lock and compared every retained third-party package against the product lock. Read relevant Workspace/settings/run APIs, runtime request/prepare/apply/finalization paths, and core writing evidence, marker, candidate-slot and apply gates. Product source comparison from b0eb18d7 to f8e725385e56172e2ac654409d5af3e20352e1c8 found no changes in crates, agent, Cargo manifests/lock or toolchain. Existing dirty progress documentation was preserved.

Harness root: `/Users/evokessler/.codex/worktrees/vacation-hardening/codex-writing-evidence/harness`.

SHA-256:

- `src/main.rs`: `ddf1b698dfd444798bcd0ea09fa8f54b6e6875b2976da1907996a8b1d0540b07`
- `Cargo.toml`: `5340dfde69a293a2031b9773eb87f3528a749ac54e8d4df00cf6e25d0f7f8195`
- `Cargo.lock`: `47cba3e6ebe8c7e30b4596580774df99f0a7657020b837f20787c86f8e658c42`
- Ordered Cargo.toml + Cargo.lock + src/main.rs aggregate used by `source_digest()`: `398b858172a1f53d57a53dfed3afaccffc2b9f0524e13443c4735617e2c99fd2`
- `target/debug/brn-writing-smoke-harness`: `068698555d8dcadd47b0ae8ea49fbd36e7b25678cd874f216443c4b5d191db3d`
- README: `160120fad11538a965a970ee4cecd28ba09b0d19c8e6920059ef45ea975907c0`
- feature-tree.log: `2ac961f58f3c84afed3520e735eeae6f13e2c1b6d90e525166920cb282f4d87c`
- compile.log: `73ca653f22901693e7f2eb53ca24b593876062e5aaa39535569953d879eb3872`
- build-binding.json: `1c25d5ffe606b56cbb914171c13fbe1775b4b6732522074dea26caf605a757fd`

The initial review brief called 398b... the main.rs hash. The lead clarified, and independent calculation confirmed, that it is the ordered source aggregate. This was a label discrepancy, not an observed source change. The executable contains the exact complete current Cargo.toml, lock and main.rs bytes at offsets 24654121, 24654784 and 24731146, respectively. That supports the aggregate-to-binary source binding alongside the final successful build log, without executing or rebuilding the binary.

## Qualified source observations

- `prepare` validates an absolute canonical existing parent and uses exclusive `create_dir` to claim an absent root before Workspace initialization. Existing/symlink leaves fail the claim. `run` and `verify` require canonical existing directories and a regular-file manifest. No old owner dataset was opened in review.
- Preparation uses normal `Workspace::open`; bootstrap derives only the approved auth location and does not read credentials. Synthetic configure preserves that location, selects Codex/Luna/low, and removes alternate credentials-directory configuration. Runtime follows the ordinary Codex AccessToken account route with read-only credential opening and no API-key fallback/persistence. No auth file or private owner file was inspected by the reviewer.
- Scenario source bytes are short and fixed; the initial payload check is below 8 KiB. The saved request binds one exact current Note/comment and v2 setup. Before dispatch the full record hashes, versions, request hash, pending state and empty candidate list are checked. The production run is RewriteNote, sources empty, four tool rounds, Summary trace, and no payload event logging.
- Proposal-only behavior has a checked core gate, not merely prompt wording: WritingSnapshot::current_inputs/change_request preserves Current request/setup/comment inputs; runtime writing_request fixes one whole-note target and metadata; runtime apply uses maintenance authority for RewriteNote (`threads.rs:2129`); core apply recognizes canonical writing markers (`store.rs:3112`) and returns NeedsReview for any non-owner authority carrying them (`store.rs:1852`). This remains true with standing maintenance enabled. Core prepare reserves one active candidate per Note (`store.rs:3034-3060`), preventing a second competing frozen rewrite candidate in this fixture.
- Exact candidate text, frozen request/thread evidence, captured comment input, unchanged original/sentinel hashes/versions, pending comment, and absence of note-change/Undo receipts are checked. The harness has no Apply/Undo mode. Candidate comment-address suggestions may be empty by the production contract; actual feedback stays pending.
- A dispatch marker precedes the production spawn. Same-DATA retries are refused, including uncertain outcomes. At 180 seconds the token is cancelled, five seconds of settlement are observed, and upstream cancellation is always explicitly unconfirmed. If settlement or persistence cannot be read, an Unknown result is retained. A detached runtime future after grace can still have uncertain effects until process shutdown, so Unknown must consume the attempt and never cause an automatic retry.
- Error output uses fixed messages and error-kind names; result JSON contains synthetic IDs, run state/configuration and numeric usage/trace only. Provider answer/detail/event payloads are not printed by this consumer.
- Consumer manifest enables no special features. Feature-tree plus compiled brn-ai fingerprint show empty features; compiled Rig fingerprints omit test-utils, scripted-runs and capability-spike. All 303 retained third-party package name/version/source/checksum tuples exactly match root Cargo.lock. Compiler log preserves the initial failed type-check and later successful checks/builds. The reviewer did not rerun them.

## Limits and lead-owned gates

This is read-only consequential source/guard review of the pinned compiled artifact, not fresh compilation, functional execution, account/transport qualification or native acceptance. No binary or harness mode, fixture initialization, tests, builds, account/provider/native/Page calls, Git mutation, acquisition or nested helper ran. Only this requested report was written.

The actual ledger read contains the three closed 19 October attempts and no 20 October reservation. Its schema is compatible with the harness checks; a new exact reservation must be created before any dispatch. The harness leaves current quota, actual Tallinn calendar date, campaign expiry, single actor enforcement and ledger transition/result bookkeeping to the lead. It checks the reserved row's fixed 20 October date rather than a live clock, so source approval must not be reused on another date. Current model availability, actual effective transport configuration, allowance before/after, credential behavior and runtime outcome remain unobserved. Preserve the five-percent floor and count failures/uncertain launches. Apply/Undo and actual Mac controls remain pending for the consolidated native session.
