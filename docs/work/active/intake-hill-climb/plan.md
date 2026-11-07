# Offline intake hill-climb

The owner requested an AI-Off intake experiment, implemented by one agent with
only the two council reviewer seats delegated. Release #253/#268 stays deferred.
Scoring and checks use no provider calls or external network access.

Base is `3e2f9f71526f974389784554f159d8f0c36cb6c6`, fetched from origin/main into
the task worktree. The owner's main checkout and real data are untouched.
`docs/development/workflow.md` is absent at this base; current AGENTS, setup and
verification guidance apply. Dependencies and all locks stay unchanged.

1. Freeze hand-authored manifests, the 16/8 split and an exact offline scorer.
2. Council round 0, fix verified findings and commit the final freeze before
   baseline measurement or candidate implementation.
3. Measure release binaries with ABAB baseline/final arms, five cold and five
   warm samples per file, raw timings and spread. Primary metric is mean fidelity.
   Speed-only acceptance uses the predeclared rule in the scorer README.
4. Try at most eight small candidates, stopping after three consecutive rejects.
   Every candidate gets a table row, including rejected changes.
5. Confirm on held-out once with both arms. Claim no gain if confirmation or any
   guardrail fails. Council the final diff, fix verified findings and rerun tuning.
6. Open one PR. Do not merge or release.

Guardrails are no newly lost expected or unmanifested element, no false structure,
unchanged hostile refusal categories limited to Invalid/Budget, encrypted PDF
still deferred, passing intake/CLI/helper checks, OS network denial, stock source
and dependencies unchanged, and native executable-bundle allocated size no more
than 105% of baseline. Native observation and owner acceptance are not performed.

The [scorer](../../../../tools/intake-score/README.md) documents commands and limits.
Council receipts are `council0-sol.json` and `council0-opus.json`. Both seats scored
the initial scorer 6/10. Kiro's Opus route failed with a monthly usage limit before
review; the authorized Copilot backup supplied the Opus review. Verified findings
were fixed before measurement. The owner required ABAB; its possible fixed-order
timing bias is recorded rather than replaced by another run order.
