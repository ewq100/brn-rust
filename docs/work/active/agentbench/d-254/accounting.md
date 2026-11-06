# Final cycle accounting and limits

The shared escrow is 1,760: D 1,592 and E 168. These are fixed allocations,
not measured account usage. E's independent grading, Council and failed attempts
belong inside its 168; its 48 confirmation reserve is inside 120 BRN, not additional.
The canonical shared ledger is
`/Users/evokessler/Documents/Codex/brn-shared-de-20261106/reservations.jsonl`,
using the maintained `limits.Reservations(path,1760)` API. Role ledgers are
non-additive projections. Each dispatch debits the protected role before its
native per-run reservation. No failed or uncertain attempt was refunded.

| D role | Allocation | Dispatch debits | Observed direct HTTP |
| --- | ---: | ---: | ---: |
| Phase 1 candidates/readers | 600 | 336 | 202 |
| Phase 2 development | 264 | 144 | 112 |
| Phase 2 broad regressions | 432 | 0 | 0 |
| Phase 2 fresh confirmation | 144 | 42 | 36; total unknown, upper bound 42 |
| Grading/evaluation coordination | 120 | 31 | 10 grader HTTP; one uncertain up to 1; coordinator unknown |
| Codex Council, design/final | 24 | 24 requested | unknown |
| Discovery/uncertain contingency | 8 | 0 | 0 generation; metadata only |
| D total | 1,592 | 577 | 360 observed direct HTTP; total unknown |

The direct BRN/no-tool-grader transport total is conservatively at most 367,
with one uncertain fresh candidate bounded by 6 and one empty interrupted grader
bounded by 1. This bound excludes T3 orchestration. Grader 31 comprises 12 requested
setup turns, 8 requested confirmation-coordinator turns and 11 single-HTTP grader
reservations (8 valid, 2 timeouts, 1 interrupted/unknown). Coordination actual
provider requests/usage are not exposed. Codex Council 24 is requested/debited,
not a verified actual-HTTP count. All-route Council requested caps total 84 across
seven seats, including the failed Kiro seat, inside the 96 ceiling. T3 cannot
hard-enforce those provider-turn limits. Parent orchestration, outside actors and
E's actual usage are not measured by D's runner. A complete account-wide actual
request total remains unknown; separate runner ledgers do not establish it.

The immutable [role caps](evidence/active-role-caps.json),
[operational accounting](evidence/accounting-final-pre-review.json) and native
per-run traces preserve each debit and explicit reported input/output/cache usage.
Cached input is a subset of input. Timed-out/uncertain calls have unknown usage.
No monetary charge or subscription-credit estimate is inferred. Legacy estimated
credit output is preserved as historical runner output only.

## Time, sequencing and admission

The original whole-cycle window is 21:30:06.983387Z–00:30:06.983387Z.
Phase 1 started at 22:11:10.355811Z with 60 minutes; Phase 2 started at
23:09:19.039570Z with 90 minutes, truncated by the whole-cycle deadline.
Waiting/build/review time counts; no clocks were reset. The phase clocks overlapped
by 1 minute 51 seconds. Phase 1 unseen confirmation was not admitted with that
time remaining. The 26 minutes 28 seconds between the last visible native block
and Phase 2 included readers, 179 seconds of direct grading, fault containment,
controller work and preparation. The absence of unseen evidence reflects that
allocation choice, not an already-expired phase clock at the Phase 2 transition.

The declared 20-minute active grading/evaluation budget differs from that wall
interval. Direct grading receipts total 179.080 seconds; T3 setup elapsed
250.509 seconds and confirmation coordination elapsed 618.373 seconds. Their
conservative sum is 1,047.962 seconds (17 minutes 28 seconds), including native
work inside the confirmation role. The direct grader timer excludes T3 roles;
T3's own limits were requested, not hard-enforced by the runner.

The planned order included development readers, then broad regressions, then
fresh confirmation. The lead instead prioritized fresh confirmation immediately
after development candidate runs, while development acceptance was unknown.
This deviation consumed the fresh pack on an unqualified candidate. The added
10-minute coordination window was fixed before fresh results, during development;
it was absent from the initial plan. Eight candidate commands plus eight readers
need 20 minutes in the worst case, before grading, so all could not fit that
window. Its preparation/checks left only 275 seconds for native execution.
The nested deadline, not the cycle deadline, stopped the in-flight seventh
attempt; approximately 61 whole-cycle minutes still remained.

Coordination completed after 10 minutes 18 seconds, overrunning its requested
soft limit by 18 seconds while recording/reporting the already-stopped run.
Cleanup recorded no errors or owned live sessions. Six terminals and one unknown
were preserved. The remote outcome remains uncertain; no replay or further BRN
dispatch occurred. Development readers, broad regressions and final reader/grade
confirmation remain unrun. Protected unused capacity was not refunded or reassigned.

Subscription metadata reported ordinary usage allowed at 29% weekly usage
initially and at 34% before Phase 2 and fresh confirmation. No credits were
purchased, resets applied, or provider/account/model fallback made in BRN.
Metadata does not promise enough quota for the proposed maxima.

## Qualification and delivery

MacBook D/E build and latency slots were serialized. B uses Macmini and was not
a dependency after the owner's correction; the agreement's earlier B-slot wording
is historical. Source-bound builds, exact combined feature closure and immutable
artifacts are identified in the reports. E search and C provider changes are
excluded. The later main synchronization contains documentation only.

Native AgentBench initially passed 96 of 97 tests, with one optional Inspect skip.
After the [final review corrections](review-followups.md), 99 of 100 tests passed
with the same optional skip. Grader Clippy and formatting passed; two wrong lint
command invocations remain recorded. The executed supervisor bytes were archived
before the post-run exit-status correction. Verification-only specifications
bind those same bytes to the original freeze identities. No measured result or
attempt receipt was replaced, and no live rerun followed the correction.

The comparison is inconclusive and the implemented retention candidate is not
adopted. Preserve the original accepted baseline. Saved-state quality, broader
regressions and final confirmation remain unresolved acceptance requirements.
No desktop observation, hosted CI pass, owner acceptance, default change, merge
or release is claimed. A further cycle needs fresh cases and feasible evaluation
windows; the existing stopped allowance is not silently extended.
