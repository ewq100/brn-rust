# Independent complete-candidate review: RunActivity

Reviewed on 2026-10-22, read-only apart from this report. Review outcome: **no actionable correctness or requirements findings** in the complete candidate. This is source review, with runtime qualification still owned by the lead. It does not establish passing checks, native usability, owner acceptance, or integration.

## Exact identity and inventory

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Branch: `codex/run-activity-state-owner`
- Candidate: `1af5a124cb0fb576adeb69231d70cd3322bbf3c4`
- Candidate tree: `d7679173a8431340906817eb7a1d8a5ebbb8d5fb`
- Requested base and verified merge base: `7fd7142c893b7aefcbebb53953336f00dcfc3212`
- Base tree: `4cc0ba01592ea11543fce2dcb53b756ab0d85f51`
- Candidate working tree clean at initial and final source inventory. Local `main` was `18f3891e29e57d4459c663967d26115f0e57787d`; review deliberately used the requested pinned base, not that moving branch.
- Entire actual diff: five files, 240 insertions / 55 deletions, no manifest/lock/schema/runtime-prompt changes.

SHA-256 of all reviewed task-owned files:

| File under `crates/brn-desktop/src/` | SHA-256 |
| --- | --- |
| `threads_run_activity.rs` (new) | `aa83eb5f79e7a027e0a26164f99749a8fdbb9ecd744b84765ef2435344765c16` |
| `threads_chat.rs` | `0abb0b0947a0424cbb7285f1ae9451f3769374aa95d36146b76af1ff89b46c6b` |
| `threads_ui.rs` | `925e3d9e7a53b1756440ee489e482a229421efa5b7db622e63b7c33de07223c3` |
| `threads_writing.rs` | `492051e2fae67c03dfd95762b20b29b423b552ca2f45fa3aa74561fc08d4d150` |
| `threads_steering_tests.rs` | `f16e10a84c712a49a0650ab7ae9e7a428ee20eea8a9152a24908f7b3504d9d33` |

## Authorities and scope

Read the candidate AGENTS, status/index and overview routes, desktop README, architecture overview/invariants/Threads behavior, strategic-programming and verification/workflow guidance, independent-review skill/resource policy, and local Rust-review skill. The selected requirement is the `run-coordination-state-ownership` design in the lead checkout's `docs/work/active/vacation-campaign/campaign.json` (selected task at lines 2536 onward). The design explicitly limits this slice to activity presentation ownership and permits Idle's first known-chat observation for current transport fixtures. No product defect was claimed or inferred merely from the refactor.

Reviewed all five actual changed files and their production consumers, worker drain contract, run start, completion, disconnect, cancellation/continuation paths and native steering fixture. Requirements lens and correctness/ownership lens were both applied. No nested agents, network, account actions, native interaction, test/build invocation, source/shared-record edit, or Git write was performed. The upstream URL in the Rust skill was not fetched because network access was outside the review assignment.

## Findings

No P0/P1/P2/P3 actionable findings. No merge-blocking source correction identified. This conclusion applies only to the identities above; lead qualification and normal required integration gates remain pending independently.

## Transition and wiring assessment

`threads_run_activity.rs:9-118` now has one private state owner and only immutable reads. Its enum removes independent chat/steps/status mutation. The obsolete fields and `record_activity` implementation are completely retired from production and tests.

| Event / precondition | Reviewed expected result |
| --- | --- |
| Started, from any state | Live in that chat with empty steps and empty terminal display |
| Nonempty Observed in Idle | Establish fixture-compatible known-chat Live state and append |
| Nonempty Observed in matching Live | Append unless exact consecutive duplicate; retain last 32 steps, each at most 8192 Unicode scalars |
| Empty, wrong-chat or Ended observation | No append or scroll effect |
| Matching Live Finished / Disconnected | Ended with same chat and bounded steps; preserve supplied status / use exact `Disconnected` text |
| Wrong-chat, Idle or already Ended terminal event | Preserve previous state |

Production start (`threads_ui.rs:2023`) emits Started after successful spawn, before installing the new worker; failed preparation/spawn retains the previous trace, as before. Early channel events remain queued until installation/polling. Real no-progress runs still have a Live owner, so finish/disconnect produces an Ended empty trace; `has_steps_for` keeps that empty trace out of retained rendering. Direct fixtures without Started retain compatibility through their first admitted progress event; no-progress fixture terminals have no activity pane to display.

Progress and Activity wiring (`threads_ui.rs:2688-2715`) supplies the current worker's own thread and retains the Run-only and not-cancelled gates. Shared progress/stream behavior is unchanged. Cancelled progress cannot append activity or replace Stopping status. Worker drain already stops at the first terminal event and the old receiver is retired before replacement (`threads_worker.rs`), so trailing old events cannot reach a successor, including a successor in the same chat. Run ID/fence ownership remains with Worker and the checked service; the activity model does not copy or decide durable identity.

Finished (`threads_ui.rs:2782-2794`) supplies the same exact checked-run display strings as the old code, before clearing the worker. Disconnect (`threads_ui.rs:2975-2983`) retains the old worker long enough to preserve partial/queue effects and names only a Run identity. Unrelated import/output/connect disconnects send no activity terminal event, preserving a completed chat's outcome. Existing queue acknowledgement, pending continuation, Stop, partial persistence and lease handling are untouched.

The append boolean controls only the existing GPUI scroll effect (`threads_chat.rs:14-20`), after an accepted append. Rendering keeps quiet, per-chat ownership, terminal inspectability and Stop ownership (`threads_chat.rs:33-107`, `threads_writing.rs:166-190`). No widget identifiers or status text changed.

## Test effectiveness and limits

The three new tests use literal independent expectations rather than the production transition function to calculate expected results. They cover completion retention/start reset, fixture Idle admission, wrong-chat observation, disconnect retention/late observation rejection, whitespace, exact duplicate suppression, ordered 32-step eviction and multibyte Unicode-scalar truncation. The `step 2` expectation is correct after the stated sequence. Retained steering assertions were mechanically migrated to immutable reads without weakening their original count, terminal inspectability, quiet behavior, independent scrolling, unrelated worker and wrong-chat visibility checks.

Nonblocking coverage opportunity: the new tests do not directly enumerate wrong-chat Finished/Disconnected, terminal idempotence, positive `has_steps_for` ownership or Started-to-terminal with zero observations. Source reasoning for these cases is recorded above; this review does not claim an exhaustive transition-table execution or mutation test. There is also no new direct integration assertion that cancelled Progress/Activity leaves the activity state unchanged, although the existing admission condition is visibly retained.

The new module is below `threads_ui`, so default-feature desktop tests do not compile or execute these tests. `native-ui` compiles the new production module/unit tests; `native-test-support` is needed for the steering integration suite. Lead should preserve these feature distinctions in reported focused/full tests, ordinary native build and strict lint results. I ran none and assumed none had passed. Headless toolkit tests cannot establish actual rendering, focus, scrolling feel or OS/native usability.

## Responsibility and interface cost

The boundary is proportionate: one local presentation model, four actual lifecycle events, three borrowed/read-only queries and one effect decision. It centralizes reset/retention/admission/bounds without a new crate, framework, event bus, canonical state engine or generic reducer. Host transport/persistence/queue/cancel responsibilities stay in place. Per-event temporary thread cloning is a small additional allocation in the host, not evidence of a material performance regression; no performance claim is made. The first-observation fixture accommodation is explicit in the selected design and bounded by host-supplied Worker context. Full Run coordination remains a separately deferred responsibility.

Next gate: lead examines actual source-bound qualification results, resolves any failures honestly, reconciles current main, and integrates normally only with required exact-head and resulting-main evidence. This source review alone grants no native acceptance or merge proof.

## Final-candidate addendum: lint correction

Reviewed the exact complete delta `1af5a124cb0fb576adeb69231d70cd3322bbf3c4..f6dd4d6666320437d631759440b68f2b43d33a9d` on 2026-10-22. Final checkout remains clean on `codex/run-activity-state-owner`.

- Final candidate: `f6dd4d6666320437d631759440b68f2b43d33a9d`
- Final tree: `bf4bd1a61e6d74f36f2eae58a9e68897988c6cfa`
- Base remains `7fd7142c893b7aefcbebb53953336f00dcfc3212`.
- Sole delta: `crates/brn-desktop/src/threads_ui.rs:2784-2791`, two insertions / two deletions. `if identity.is_some()` plus the guarded `expect` becomes `if let Some(identity) = &identity` plus direct `&identity.thread`.
- Final `threads_ui.rs` SHA-256: `afbff3810394cec4edfc2631a95a7e6f2308be36e3bea2da6484573351b15953`. All four other task-owned file hashes exactly match the inventory above.

Disposition: **accepted; no new actionable findings**. The borrowed pattern preserves the same Some/None admission, worker-thread source and lifetime, status calculation and Finished event payload. The outer identity remains available for the following thread capture and non-Run branch. This removes the unnecessary unwrap without a behavioral or responsibility change. The complete-candidate review above therefore carries forward to final `f6dd4d6666320437d631759440b68f2b43d33a9d`, including its stated coverage limits.

Lead reports original-candidate focused tests 3 passing, quiet integration 1 passing, full native test suite 345 passing / 0 failed / 1 previously ignored, followed by a strict-lint `unnecessary_unwrap` finding that triggered this correction. These are reported lead-owned results, not independently executed or log-audited by this reviewer; final-source qualification and integration gates remain lead-owned. No builds, tests, source/shared-record edits or Git writes were performed for this addendum.
