# Shared provider operations (#263)

This is Agent C's separate cloud stage after [Cycle 2](development-speed-255-cycle2.md).
Cycle 2 merged in [PR266](https://github.com/ewq100/brn-rust/pull/266) as
`c388da267a9e41cfe9d65051cd27b95817c9a523`, with all seven PR CI jobs passing,
current-head Codex code/security receipts reporting no findings, and the owner's
explicit instruction to skip unavailable Council. No Council sessions ran and no
score is supplied. Resulting-main checks are separate from PR qualification.

The implementation base contains A's selected `a921e325` runtime. A's explicit
shared-file release was requested on [#253](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6696819563).
The owner subsequently confirmed “A is done continu”; that owner-reported completion/release is recorded on #253. No new A-authored handoff receipt is claimed. This branch now contains the extraction. B's first-alpha build
is fixed at `a921e325`; this work does not alter that candidate or the frozen
#254/#256 experiments. Later integration must respect B's publication window.

## Inspected responsibilities

| Caller | Existing policy and contract |
|---|---|
| App `run.rs` | Provider aliases, catalogue refusal, effort/model validation, caller-specific error order before allocating a Run. |
| Chat `threads.rs` | Copilot account captured at preparation; ChatGPT auth-file precedence; pre/post-authentication account checks; session pin and resend reporting. |
| Summary `threads_summary.rs` | Parent authenticated client/account and account-scoped cache; tool-free compaction; existing same-model fallback before output. |
| Helper `threads_research.rs` | Independent saved selection/effort/auth; own account captured when parent starts; restricted tools and parent cancellation/response guard. |
| Refinement `conversion.rs` | Off opens no credentials; On uses its own provider/account, fixed Luna/high, one tool-free turn, separate deadlines and text cap. |
| App `subscription_search.rs` | Codex supported auth-file route only; Copilot's own captured sign-in; effective web permission and late-result fencing remain host-owned. |
| Native `copilot_search.rs` | Query-only Responses request; captured account checks; exact hosted tool, real cited sources, existing byte/call/time limits. |
| Auth adapters | Existing credential validation and authenticated Rig construction; non-strict Copilot schemas; no new auth/discovery request or fallback. |

Eight mapped callers are byte-identical between measurement baseline `6897fe56`
and selected runtime `a921e325`; chat and helpers changed to share A's response
guard. The guard is one Arc per run, shared with summaries/helpers: per-response
20-minute and 64-MiB streamed-wire limits, duration-only whole-body account
exchanges, refusal of later requests after a trip. Existing Stop and idle limits
remain separate. Conversion and search retain their distinct limits.

## Shared boundary

Use a small module in `brn-ai`, reusing `Selection`, `ReasoningEffort`,
`AccountPin`, `ProviderClient` and Rig's `DynHttpClient`. Do not introduce a
public crate, generic executor or session store. Reuse `Provider::from_setting`
instead of the duplicate alias matches. Keep catalogue checks and their error
precedence at the app boundary.

The shared owner binds the selected credential paths/account and construct
the existing client with the same auth-file precedence and Copilot pre/post-auth
checks. Transport decorators remain explicit at operations: chat/helper resend
and A's run guard cannot silently become conversion/search retry policy. Missing
account and unsafe-path error precedence must survive extraction.

The existing Copilot completion-route/schema policy and same-model
empty-refusal predicate live in that owner; chat and summary retain their execution
loops, while search/refinement retain their explicit Responses-only contracts.
Unknown catalogue capabilities remain unknown, not implied entitlement. Remove
each replaced branch from callers as it migrates.

Tools, writing authority, canonical state, checked writes, version guards,
provenance and Undo stay with existing services. Token estimation, new direct
Responses optimization, defaults, providers and SDK changes are excluded.

## Qualification plan

Run existing offline transports/handler suites before and after. Compare exact
payload/schema assertions, route order/counts, operation-specific errors and
checked effects. Cover missing/changed accounts before and during authentication,
no adoption of later sign-in, independent helper settings, refinement Off/On,
web Off/no dispatch, cancellation/late results, retry bounds, malformed/oversized
streams and checked-write guards. Keep actual skipped/failed results visible.

Freeze three temporary maintenance tasks before implementation: add one saved
provider alias; change one account-binding refusal policy; change one existing
same-model route-refusal condition. Specify accepted behavior independently of
the eventual module shape, run the corresponding offline checks on baseline and
candidate patches, and restore all experimental changes. Record patch spread,
review findings and rework. An independent reviewer must supply a confirmation
variant after the main design is fixed. Without that independent confirmation,
do not claim the full maintainability acceptance or an elapsed-speed improvement.

Hosted Codex code/security review and applicable normal platform CI, including
macOS production-feature lanes, remain required for the eventual implementation.
Prior live/native receipts keep their original source scope. No implemented,
preserved-behavior, maintainability or fresh native acceptance is claimed here.

## Offline baseline receipt

[Baseline and artifact identities](evidence/shared-provider-263/baseline.json)
record source `c388da26`, the existing subreaper, locked offline commands and
helper hashes. Workspace library suites passed 763 tests with two parent-invoked
subprocess witnesses ignored. The explicit AI scripted-runs command passed 635
with one such witness ignored; the same AI binary also ran in the workspace
command, so these counts are not distinct coverage. An initial passing attempt
used different activation-default helper binaries; its log remains recorded, but
the matched baseline uses the exact helpers recorded in Cycle 2. No refactor
after-result or performance comparison exists yet.

## Preparation publication and remaining handoff

This separate branch is [draft PR267](https://github.com/ewq100/brn-rust/pull/267).
Preparation commit `1efe4281` has completed [code](https://github.com/ewq100/brn-rust/pull/267#issuecomment-6697023286)
and [security](https://github.com/ewq100/brn-rust/pull/267#issuecomment-6697056622)
receipts reporting no findings; zero inline threads were open. The owner-only
detailed security report was inaccessible through available cloud tools. No
independent maintenance confirmation variant was supplied. These are preparation
reviews, not a completed refactor review; later heads require their own checks.

[Local tooling receipts](evidence/shared-provider-263/preparation-tooling.json)
retain the failed invocation and the corrected 329-passed/31-skipped run. The
first invocation omitted cloud activation and the existing subreaper, producing
six locked-archive lookup errors and one orphan-process assertion failure.
Correcting that invocation required no source or assertion change.

Stage1 resulting-main run [42300540347](https://github.com/ewq100/brn-rust/actions/runs/42300540347)
has all four required contexts passing. Optional Windows failures remain separate
in the selected record and #255; no overall-main success is claimed.

A current explicit release of shared source ownership is still required by
[#263](https://github.com/ewq100/brn-rust/issues/263). B has built its separate
fixed `a921e325` alpha artifacts; [PR268](https://github.com/ewq100/brn-rust/pull/268)
contains its release-approval work. Its Mac resource-slot release does not supply
A source ownership. No shared runtime file was edited while that prerequisite
was unresolved. Resume this same draft after the handoff; reconcile B task-record
changes and its publication window before integration.

## Implementation and newly exposed runtime gap

`provider.rs` now owns `ProviderOperation` credential/account/client policy,
`Provider` saved aliases and account capture, saved effort parsing, and Copilot
completion/schema/initial-route/fallback decisions. App chat/helper/search
selection uses those existing normalized provider names. Chat/helper transport
resend wrapping stays run-owned. Summary construction still reuses its parent
client; refinement/search retain their own transport/deadline/call contracts.
The former chat client-policy branches and conversion/search auth-policy copies
are removed, as are duplicate provider/effort matches. Existing Auth and Rig
remain the low-level credential/protocol owners.

The original 763 workspace library tests passed after extraction. The first
compile exposed four test-only imports that had relied on the parent runtime's
Auth import; those imports are now explicit. A new during-auth mutation fixture
then exposed an existing token-store overwrite, reproduced independently on
untouched c388da26. The callback changed the account, but the old combined-record
save restored the starting bootstrap. No extraction regression or fresh live
qualification is inferred.

That behavior repair is isolated in [PR272](https://github.com/ewq100/brn-rust/pull/272),
not implemented as a provider-refactor policy change. It protects stale Copilot
token-store reload/save/delete and has its own offline regression, review and
platform qualification. The refactor's final before/after baseline will include
that reviewed runtime, preserving the original A/Cycle2 receipts unchanged.
Temporary maintenance-task specifications were frozen before extraction; their
original source pin and finite allocation remain retained separately. No
maintenance task acceptance or elapsed-speed gain has yet been claimed.

## Completing the settings/fence boundary

A full-consumer audit found three further name-policy copies beyond the first
routing pilot: helper-save validation, the durable core run/settings fence, and
the runtime preflight settings check. These now use the same identity map.
`brn-threads-core::canonical_provider_name` owns only persisted provider-name
identity because the durable fence cannot depend on `brn-ai`. The internal AI
provider module projects that identity into `Provider` and owns operation policy.
No new crate, dependency, account store, setting value, schema or model default is
introduced. Unknown raw names retain their prior core equality behavior; runtime
selection still refuses unsupported providers.

Helper saving reuses shared provider/effort/model validation while retaining its
existing error order, absolute-path and one-credential-route rules. The legacy
trimmed-model syntax check still preserves the saved bytes; runtime helper
selection continues to validate the exact model bytes. A core compatibility
matrix preserves known alias pairs, unknown-name equality and exact model/effort
fencing. Existing runtime/account settings-snapshot integration checks remain.

The original alias pilot proved only direct runtime resolution, not successful
helper configuration and durable Run preparation. It is retained with that limit.
The stronger acceptance test saves both parent/helper settings and prepares a
canonical-provider Run. Its first launch had an invalid fixture: credentials were
placed inside the canonical data folder, which Store correctly refused. That
receipt is preserved as setup failure, not evidence of baseline coding difficulty.
The corrected fixture uses sibling data/credential folders and a bounded run on
both trees. No elapsed-speed or statistical coding-effort claim is made.
