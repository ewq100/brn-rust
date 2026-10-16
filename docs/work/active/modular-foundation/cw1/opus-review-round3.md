**APPROVED — focused source `521454f2` (tree `cb8c754f`)**

I reviewed the diff from `690fb705` to `521454f2`. The only production change is `Store::undo` in `crates/brn-threads-core/src/store.rs:663-729`. No other production files changed: no API, schema, pagination or desktop changes. I didn't run any Cargo, build or test commands.

**What the new code does**
- **Same rules as apply:** the Undo check rebuilds the post-write records the same way `apply_on` does (`store.rs:1416-1428`). It then asks the same question as `validate_setup_writes` (`writing.rs:399-409`): is the working Note usable with no selected conversation? So skipping the conversation input never lets through something validation would refuse.
- **Undo of a newly created setup, or of a general or private setup:** no conversation input is added. For a non-Project setup, the conversation always had the same scope as the setup itself, so the input added nothing but the extra restriction.
- **Project setup whose working Note is visible to the Project anyway:** no conversation input is added. The check is the same one a fresh owner setup gets, so no new access is granted.
- **Project setup that needs the conversation's link:** the original frozen conversation input is kept. Validation still fails closed if the conversation was moved, archived or epoch-bumped, or if its link was removed.
- **An Undo that was already prepared:** its saved input choice is reused unchanged (`store.rs:673-676`). Receipt replay is checked before this (`store.rs:627`), so the earlier hash check is unaffected.
  - It can't pick up inputs from an unrelated operation: the Undo binding check (`store.rs:624`) and the hash check (`store.rs:785`) reject that.
  - Changes in context don't revive an old grant. A later current check can reject the saved Undo, but it can't switch its grant.

**Tests:** the new tests check real behaviour. They confirm the exact archived records and versions, receipt replay after restart, an unchanged prepared hash with a `Stale` result when the Project's knowledge setting changes, records left unchanged on refusal, and refusal after a move or unlink. The R3 check (`!matches!(Ok(Applied))`) is loose, but the unchanged-state assertions back it up. The 104 focused results add up to 10+22+1+45+9+5+12. Both failing-before logs reproduce the expected failure. Adding tests doesn't by itself prove which mutants are now caught.

**Prior findings:** R1 is fixed. R2 is now covered by the direct ordinary-chat and invalid-scope refusal tests. R3 is covered: Undo after a Project move or link removal fails closed. R4–R7 stay accepted, non-blocking limits.

**Non-blocking (LOW):**
- I found one theoretical fail-closed edge, not confirmed reachable. A general setup whose working Note is visible only through an ordinary chat's Source would now refuse Undo. Under `690fb705` it was allowed while that chat was open. There's no privacy impact.
- The 104-test log doesn't record the commit it ran on, so the principal should match it to `521454f2`.

**Coverage:** the existing `690fb705` receipts plus the focused `521454f2` runs logically cover this change, because the new branch only runs for Undos that touch a writing setup. I don't think any specific further check is needed.

**Still pending (not defects):** the final mutation supplement and its classification, the final evidence-only commit, the open PDF/Stop baseline failures, native observation, owner acceptance and merge. This approval confirms the source only; it doesn't accept the full CW1 slice.
