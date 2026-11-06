# Shared provider operations (#263)

This is Agent C's separate cloud stage after [Cycle 2](development-speed-255-cycle2.md).
Cycle 2 merged in [PR266](https://github.com/ewq100/brn-rust/pull/266) as
`c388da267a9e41cfe9d65051cd27b95817c9a523`, with all seven PR CI jobs passing,
current-head Codex code/security receipts reporting no findings, and the owner's
explicit instruction to skip unavailable Council. No Council sessions ran and no
score is supplied. Resulting-main checks are separate from PR qualification.

The implementation base contains A's selected `a921e325` runtime. A's explicit
shared-file release was requested on [#253](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6696819563).
Until that handoff, this branch contains preparation only. B's first-alpha build
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

## Boundary to implement after handoff

Use a small module in `brn-ai`, reusing `Selection`, `ReasoningEffort`,
`AccountPin`, `ProviderClient` and Rig's `DynHttpClient`. Do not introduce a
public crate, generic executor or session store. Reuse `Provider::from_setting`
instead of the duplicate alias matches. Keep catalogue checks and their error
precedence at the app boundary.

The shared owner should bind the selected credential paths/account and construct
the existing client with the same auth-file precedence and Copilot pre/post-auth
checks. Transport decorators remain explicit at operations: chat/helper resend
and A's run guard cannot silently become conversion/search retry policy. Missing
account and unsafe-path error precedence must survive extraction.

Move the existing Copilot completion-route/schema policy and same-model
empty-refusal predicate to that owner; chat and summary retain their execution
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
