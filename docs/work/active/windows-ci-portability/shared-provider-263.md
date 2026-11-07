# Shared provider operations (#263)

Agent C's separate cloud implementation is [PR267](https://github.com/ewq100/brn-rust/pull/267).
The preceding [Cycle 2](development-speed-255-cycle2.md) closed with no retained
performance change in [PR266](https://github.com/ewq100/brn-rust/pull/266), merged
`c388da267a9e41cfe9d65051cd27b95817c9a523`. Its finite measurement budget and
negative findings remain unchanged. This refactor makes no elapsed-speed claim.

A's selected response-limit runtime is `a921e325`. The owner subsequently
confirmed “A is done continu”, supplying the reported completion/release after
C's current handoff request on #253. No A-authored handoff receipt is invented.
A newly exposed, independently reproduced stale Copilot auth-write defect was
separated into [PR272](https://github.com/ewq100/brn-rust/pull/272), normally merged
as `75f3722142bfe5598c3871acc8a8079f991aad9a` (reviewed `ce36eb94` product tree).
The final refactor baseline includes that repair; earlier A/Cycle2/live receipts
are not relabeled. See [the repair record](provider-auth-write-263.md).

## Maintained policy and retired copies

| Policy owner | Consumers and contracts |
|---|---|
| Core `canonical_provider_name` | Pure persisted-name identity: case-insensitive Codex/ChatGPT and Copilot; unknown stays unknown. The durable run/settings fence reuses it while preserving raw unknown-name equality and exact model/effort. Core cannot depend on AI. |
| AI `provider.rs`: `Provider` / `ReasoningEffort` | Typed saved-name projection, case-sensitive effort parsing and Copilot-only operation-start account capture. App chat/helper/search settings, helper-save validation and runtime preflight use this owner. No universal account snapshot is introduced. |
| AI `ProviderOperation::client` | Existing ChatGPT auth-file preference; selected Copilot credential folder and captured pin before/after authentication; existing safe Auth/Rig client construction; existing errors and optional summary-cache identity. Replaces the chat/helper context and refinement/search copies. Holds references and an existing pin, never serialized raw credentials. |
| AI Copilot completion / route / retry functions | Existing Rig conversation route choice, non-strict Responses schemas and same-model empty-refusal predicate. Chat/helper and summary loops call the shared rule; refinement/search explicitly force their existing Responses route. |
| Existing host/caller services | Catalogue refusal/error precedence, effective permissions, transport decorators, operation limits, state/cancellation, checked writes, approval scope, versions, provenance and Undo. These are not moved into a generic executor. |

Chat/helper resend and A's response guard stay at the run transport boundary;
constructing a client cannot add resend to refinement/search. Summaries retain
the parent client/account and account-scoped cache. Helpers retain independent
saved provider/model/effort/credentials and restricted tools. Refinement retains
Off/no credential access and On/tool-free Luna/high, its own deadline and text cap.
Native search retains query-only input, exact hosted search tool/cited sources,
its limits and host-owned late-result fence. Unknown catalogue capability is not
entitlement. Missing-account versus unsafe-path error order remains caller-specific.

The old provider/client branches and repeated setting matches were removed from
those callers. Low-level Auth remains the credential/protocol owner. The small
core identity function is the dependency-safe primitive, not a second policy map.
No new crate, SDK, framework, session store, provider, setting value, model default,
token estimator or direct-to-Responses optimization is introduced. The legacy
helper-save check still validates trimmed model syntax but stores original bytes;
runtime selection still validates the exact model. CLI flag parsing retains its own error objects while reusing the shared provider/effort parsers. The desktop canonical key and its credential/form presentation reuse the core identity; labels/icons remain presentation. These final forwarding adapters are outside the measured seven-file runtime alias task. Unknown/default presentation is preserved.

Missing credential directories
still have their existing product behavior: the confirmation's passive-open
change was temporary and is not shipped.

## Offline behavior qualification

The original matched-helper baseline at `c388da26` passed 763 library tests and
an explicit 635-test AI scripted suite. Those overlap; counts are not added.
The repaired baseline at `ce36eb94` passed 765 library tests with two
parent-invoked subprocess fixtures ignored, strict all-target Clippy, full storage
verification and the fixture-only 108-step/1,266-check CLI journey. Its dedicated
helper root and exact source are recorded separately from the earlier baseline.

Final product source `72bb6241` passes the same existing full storage and
fixture-only end-to-end commands, including strict all-target Clippy and the
108-step/1,266-check CLI journey. Its library suites pass 771 tests with two
parent-invoked fixture ignores (AI 641, App 17, Core 50); every prior passing
case name remains present. Dedicated final helper identities and explicit helper
ENV for both legs are recorded; this is behavior qualification, not timing.
The initial complete full-gate log is retained before the adapter-only pass.

Existing synthetic transport/handler checks cover selected payload/model/effort,
route order/counts, non-strict schema behavior, independent helper configuration,
summary account/cache identity, missing/changed accounts, late sign-in refusal,
web Off/no dispatch, Stop/cancellation/late results, response bounds, malformed
streams, tool permissions and checked-write/Undo guards. Two permanent 503
regressions execute both before and after: search and refinement each make one
authentication plus one operation request, then fail without chat resend or route
fallback. A new during-auth switch fixture verifies that the account actually
changed, its bytes survive, only authentication was sent, and dispatch refuses.
The repair's baseline reproduction and initial failed extraction witness remain
recorded; no refactor regression or live acceptance is inferred from them.

## Maintenance tasks and independent confirmation

[Evidence index](evidence/shared-provider-263/qualification.json) records commands,
source/hash identities, all attempts, restored patches and raw compressed receipts.
The three tasks were frozen before extraction. The same lead/configuration/tools
were used across arms; actual coding-model/effort identity and usage are not
independently exposed by this executor. No elapsed times, costs, statistical
coding-effort improvement or composite score are claimed.

| Temporary task | Executed result and interpretation |
|---|---|
| Add a saved Copilot alias across the selected runtime services | Corrected acceptance passes before/after: save actual parent/helper configuration, prepare a canonical-provider Run through the durable fence, and check helper/search/refinement selection plus error precedence. Seven measured baseline production files versus one core map after; executable integration acceptance supports the ownership observation. |
| Change the account-changed diagnostic code | Both arms pass the new diagnostic assertions and existing account checks. Two production files in each; already-shared control, no localization gain claimed. |
| Refuse empty same-model fallback when a retry hint exists | Both arms pass the predicate matrix and existing Copilot checks. One production file in each; already-shared control, no localization gain claimed. |
| Fresh evaluator's passive Copilot startup challenge | Both arms pass the same five new integration cases, Copilot-refinement and ChatGPT positive controls, and 37 auth checks. Before: five production files/seven sites; after: Auth plus provider.rs, three sites. Missing-directory, late-sign-in, helper-unavailable, Off conversion and provider-isolation outcomes are exercised. The unchanged conversion-response source also retains one Message, fixed effort and no conversation history. |

The alias routing pilot omitted helper-save and durable/runtime fence coverage;
its successful result is explicitly limited. The first stronger baseline fixture
put credentials inside the canonical data folder; Store correctly refused it.
The corrected sibling-folder fixture passes on both trees. The original
2-attempt alias allocation was exceeded by that setup invocation; all receipts
are retained, with one bounded corrected run per arm, no coding-difficulty claim
and no reopening of Cycle2's budget.

The general hosted Codex task route could not create a confirmation task
(“create an environment for this repo”). The owner selected exactly one fresh
read-only evaluation subagent, `/root/provider_confirmation`, with no inherited
conversation. It inspected immutable before/candidate sources in eight read-only
calls, specified the challenge independently of the tuning receipts, and changed
no files. This is procedural fresh-context independence, not technical filesystem
isolation. The full specification and provenance are archived. It did not implement,
execute or review the resulting patches. Normal hosted code/security review is
separate. No Council sessions ran, under the owner's explicit skip instruction;
missing Council is not counted as completed and no score is supplied.

The confirmation's original two baseline attempts failed on test-fixture mistakes:
`WorkspaceSettings` has no `Default`, then Text conversion correctly added a
terminal newline. One additional fixture-only correction was explicitly recorded
within the original 15-minute arm window, with unchanged production policy.
The corrected baseline passes on its third invocation; the candidate passes on
its first invocation using the exact same corrected harness and case set. Both
arms stay inside the original 15-minute wall allowance. No production-policy
rework was needed after either fixture correction.
All temporary behaviors and tests are restored to their pinned source bytes;
none of the experimental alias/diagnostic/retry/passive-startup changes ships.
The observations support localized, executable changes for the selected policies;
they do not establish that future changes take less time or fewer attempts.

## Review, integration and later A/B adoption

Product source `72bb6241` is the local qualification pin. The final evidence/docs
commit preserves that product tree. PR267 owns the current-head hosted macOS/
Linux/tooling checks, Codex code/security receipts, thread dispositions and
normal merge; earlier clean reviews of `4ef6ced8` are historical. This source
record is the pre-merge qualification snapshot; the linked PR and #263/#253
closeout receipts record actual integration and resulting-main outcomes.

Repair272 had all seven PR lanes and current explicit code/security receipts
reporting no findings. Its resulting-main run42337726231 has all four required
contexts and all Linux/macOS lanes passing. Three optional Windows failures remain:
core Clippy's existing `skills.rs`187/191 dead code and both native builds' existing
`threads_composer.rs` cfg/import errors. They are outside the repair/refactor
product diff; overall-main success is not claimed. Earlier preparation reviews,
helper-mismatch/activation/reaper attempts and corrected tooling checks remain
historical receipts, not current implementation reviews.

B's first-alpha artifacts and new MacBook rebuild stay pinned to `a921e325`;
D/E's frozen #254/#256 experiments retain their original source. C uses cloud and
normal hosted macOS CI, with no owner Mac, copied credentials or live product
provider call. B's latest coordination records a fixed-source build, not a final
publication reservation. Recheck #253 immediately before integration; do not move
main during B's reserved publication window.

Later A/B adoption requires a newly selected source/package and affected combined
sign-in/reconnect/switch (including during token exchange), chat/helper/summary,
web and refinement journeys. Prior live/native receipts cover only their recorded
source; cloud fixtures and hosted builds are not fresh installed-app or owner
acceptance. No alpha publication or release authority is inferred here.
