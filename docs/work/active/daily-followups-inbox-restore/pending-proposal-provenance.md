# Pending proposal provenance correction

## Scope and starting evidence

The owner approved a narrow cloud implementation and regression tests after the
[Mac continuation](mac-completion-evidence-qualification.md). Publication requires
a separate decision. No new live-model Runs, Mac interaction, merge, release,
credentials/security changes or unrelated product work are authorized here.

At task entry, PR229 was draft/unmerged at
`a829ef2898ce59ce62c6cd8fb9ba37422a452b14`; main was
`b270ad2fb0e67e10a86f68d513a37bf309760f97`. The Mac evidence branch was
`b4decd86a3344dd21cc8f300e6aa6745813f47ab`, based on the guidance code commit
`79422f8fc78918b243e64bc3835552c59e05e068`. The isolated local branch retains
both the Mac receipt and the later cloud guidance documentation.

The receipt records two additional live Runs, eleven cumulative. Both Source
records survived. One pending Action had the correct owner/follow-up and null
completion evidence. The second Run read saved records, including an operation ID
that returned no record, then retried Apply on the unchanged original candidate.
No refresh was attempted. The unchanged immutable request retained only the first
Source's links and exact input version. This establishes missing candidate
provenance, not storage loss, false completion or an unauthorized application.

## Responsibility and chosen remedy

Saved-record reads deliberately exclude proposals, and continuation context
reports bounded operation states/hashes rather than candidate contents. Guidance
alone cannot supply the missing candidate body. Existing checked
`Store::prepared`/operation APIs and `refresh_candidate` already own immutable
replacement, version checks, replay and atomic predecessor retirement. There is
no need for a new mutation service or automatic source-attachment policy.

Add exact-operation candidate inspection to the ordinary parent runtime. Before
returning any body, require a live invocation, its same-chat Run family, an active
unapplied candidate and current record/revision visibility. Owner proposals,
other chats, unavailable private references and frozen writing/helper contexts
remain outside this surface. Return a complete bounded reconstructable request
or refuse; do not silently truncate it. Reverse only typed creation/reference
identities into `new:N`, retaining prose unchanged. Refresh remaps them into the
replacement's fresh identity through the existing checked path.

The agent decides whether another source supports the same obligation. If so,
it can inspect and refresh the pending request with the accurate links and exact
versions, preserving the Action's semantics and null completion evidence. This
grants neither approval nor completion. The owner continues to review the exact
replacement; a retired predecessor cannot be applied as if it were current.

The existing duplicate-meeting evaluator now requires source links and Current
exact-version inputs together on the active candidate. Historical requests or
unrelated receipts cannot fill a missing provenance assertion. Saved Actions
remain eligible only with their own accepting version's lineage. The published
failed snapshot is a negative control; a corrected synthetic candidate is a
positive control. These are deterministic tests, not new live quality evidence.

## Qualification

The implementation is local in `c3e1d4ef8cfd5207ebc54727d36eb818e818a99a`;
strict evaluator corrections are in `e33e9bdef6d50128beb05833620920c2689b3c57`
and `e4bd53ab881a6e442053c804d8051f5529e591a3`. Rust tests ran at `e33e9bde`;
Rust sources are identical at `e4bd53ab`, where strict lint and the final Python
oracle were checked. Independent source/security/evaluator review found no
remaining source blocker. Independent evidence review confirmed the counts below.

| Gate | Actual result |
| --- | --- |
| Baseline regression | Expected failure: pending candidate inspection was absent. |
| Full affected Rust library suite | **FAILED: 355 passed, 156 failed, 1 ignored.** All four new tests, existing history/refresh tests, frozen-writing refusal and bundle 23/prior 22 cache witness passed. |
| Failure comparison | Exact same 156 failing test names as the previous cloud suite; no added/removed failures. Prior audit found 125 explicit unsafe-credential diagnostics, 1 direct admission assertion and 30 downstream failures not individually root-caused. No bypass or unchanged retry. |
| Fresh-process witness | Child process read the exact persisted replacement and acknowledged it. Its separate one-test pass is not added to 355. |
| Strict lint | `cargo clippy -p brn-ai -p brn --all-targets --features brn-ai/capability-spike --locked --offline -- -D warnings` passed. |
| Existing Python evaluators | 31 guidance tests and 99 unchanged v2 tests passed. Published Mac snapshot fails the stronger oracle; corrected synthetic candidate passes. |
| Formatting/navigation | Pinned Rust formatting, generated overview and Markdown links checked. |
| Hosted CI | **Unrun:** no publication authority. Prior PR229 CI does not qualify this local change. |
| Live/native/owner acceptance | **Unrun:** zero additional BRN Runs; eleven historical Runs remain partial/failed evidence. Actual Mac UI acceptance remains separate. |

Rust command: `cargo test -p brn-ai --lib --features capability-spike --locked --offline --jobs 1`. The direct-runtime tests simulate two durable Source
observations and fresh runtime scopes; they do not exercise a converter or a
live model. They prove that the bounded inspection/refresh path can retain both
sources, not that a future model will choose it correctly.

The evaluator uses effective saved Links overridden by the candidate's writes,
with exact Current inputs bound to that candidate. For an accepted Action, only
its uniquely identified accepting receipt can justify using a Source's pre-write
version when that same operation updated Source metadata; the receipt must match
the current Source result. Null/empty expectations, unrelated receipts, older
Action versions and later Source mutations cannot fill the missing provenance.

[Selected check receipt](evidence/pending-provenance-checks.txt) preserves red,
green and failed results. Full logs remain under
`/workspace/brn-next-evidence/pending-provenance-*`. An initial compile invocation
was canceled before tests to restore native environment activation; no result was
claimed from it. No credentials/security controls were changed.

## Publication decision

Nothing from this correction has been pushed. The proposed next step is to
recheck both remote branch heads and, if unchanged, publish the reviewed local
history as a normal fast-forward update to draft PR229, including the preserved
Mac evidence. Then follow exact-head hosted CI. Any concurrent Mac branch change
must be inspected and reconciled first; never force-push it away. Publication,
additional live qualification, merge and release are separate decisions. This
record requests only publication and hosted CI as the next step.
