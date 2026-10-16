# Agent withdrawal of its own obsolete proposal (round 5 slice G)

Commit `7ed74f26` adds the runtime tool `withdraw_proposal { operation }`. A later Run can use it to dismiss a pending candidate that an earlier Run of the same chat prepared, once the owner's decision has made it obsolete. The owner's dismissal loop moves into core as `Store::clear_review_attention` and `Store::dismiss_candidate`. Owner Dismiss/Accept and the tool both use it, the tool under owner authority bound to the Run's fence, chat, Project context and Settings. The tool never applies the candidate. It refuses applied operations, rejects owner, other-chat and unknown operations, and rejects calls from Rewrite runs and helper scopes. Tools go from 9 to 10 and the bundle to `brn-threads-guidance-12`, with no desktop, `main.rs`, `docs/status.md`, schema or `Cargo.lock` change. Every new test has a recorded fail-before.

Watch for: an ordinary Run can withdraw a proposal from an earlier Rewrite run in its chat, which `refresh_candidate` deliberately refuses to touch (confirmed; listed as a limitation). Retirement happens before any fenced check, so a Run stopped by another connection right after `live()` can still retire its candidate (likely, narrow window). On replay, the tool clears a Review item that the owner restored with Undo (confirmed). No test covers another project's chat, a Rewrite run or a stopped Run (confirmed).

**Verdict**: APPROVED

## High-level view

The tool has two refusals, and the order matters. First the operation must be in `Run.operations` of a Run in this chat. Only `prepare_changes` and `refresh_candidate` push to that list, with keys `{run}:proposal:…` and `{run}:refresh:…`, so owner operations and other chats' or projects' operations can never pass. Second, the receipt check runs before the replay check, because `active_candidates` also excludes applied operations. M2 shows that without the receipt check, an applied proposal takes the replay path and reports `Withdrawn`.

Owner receipts, keys, reasons, authority and error texts are unchanged. The agent's cleanup uses the owner-scoped, Run-bound authority pattern that the runtime already uses for its own attention writes and for refresh bookkeeping, and the core's chat and project visibility checks still apply to it. The weak spot is ordering: `dismiss_candidate` retires before the fenced cleanup, and `retire` takes no authority.

Replay is idempotent only while attention stays cleared. The replay path always re-runs the cleanup. So after an owner Undo restores the Review item, a later withdrawal removes it again and writes a new receipt. Undo otherwise matches Dismiss: the Review item returns and the candidate stays retired, so the owner's Apply is still refused.

Helpers cannot reach the tool through `HELPER_TOOLS`, the helper builder or the `tool!` guard. One test checks all three paths (fail-before M5 and M6).

<details>
<summary>Issues (5)</summary>

1. **Rewrite proposals can be withdrawn** (confirmed; non-blocking): `withdraw` has no writing-request check, while `threads_history::refresh` rejects any candidate with a `WRITING_REQUEST_ROLE` input. A later ordinary Run can therefore retire an owner-requested Rewrite proposal before the owner reviews it. Either refuse such candidates the way refresh does, or have the owner accept this behavior at integration (it is already listed as a limitation).
2. **Retirement runs before the fence check** (likely; non-blocking): `dismiss_candidate` calls `Store::retire`, which checks no authority, before the Run-bound cleanup. The agent path is guarded only by the `live()` read just before it. A stop committed by another connection in that gap still retires the candidate and leaves its Review item. Retire inside a transaction that checks the Run binding, or widen the recorded limitation to cover the gap between `live()` and retirement.
3. **Replay undoes an owner Undo** (confirmed; non-blocking): for an already retired candidate the tool always calls `clear_review_attention`. After the owner Undoes a withdrawal or Dismiss cleanup, a later `withdraw_proposal` removes the restored Review item again and adds a receipt. Say so in the Undo paragraph of the verification doc, or have replay leave attention alone once the cleanup was undone.
4. **Some refusals have no test** (confirmed; non-blocking): `withdraw_proposal.rs` covers another chat, an owner-created candidate and an applied proposal. Nothing covers a project chat's proposal, a Rewrite-run call or a call from a stopped Run. Add a project-member chat case to `withdraw_proposal.rs`, and a brn-ai test for the Rewrite-run and stopped-Run rejections.
5. **Bundle history not in the integration items** (confirmed; doc only): `threads-behavior.md` line 95 lists each bundle through `-11`. The integration items propose new text only for §142 (the tool list). Add a proposed sentence for `brn-threads-guidance-12` (when to withdraw your own obsolete proposal) next to the §142 proposal.

</details>

<details>
<summary>Details</summary>

## Membership and the refusal order in `withdraw`

```
withdraw(op)
  writing_snapshot?            -> rejected (Rewrite run)
  live(scope)                  -> rejected if fenced/settings/project changed
  operation exists             -> else rejected
  op ∈ Run.operations, Run.thread == this chat
                               -> else rejected (owner, other chat/project, unknown)
  receipt(op)?                 -> {"outcome":"Refused", ...}
  prepared(op)                 -> else rejected
  active?  no  -> clear_review_attention          -> Withdrawn, replay:true
           yes -> prepared + retire + clear_review_attention -> Withdrawn
  cleanup Unexpected/Contended -> rejected
```

Membership comes first, so an applied proposal from another chat gets the generic rejection, not `Refused`, and the tool reveals nothing about another chat's receipts. M1 (no membership check) withdrew an owner-created candidate, and the test caught it at `:302`.

## Rewrite proposals and the asymmetry with refresh

`refresh` rejects a candidate with any `WRITING_REQUEST_ROLE` input (`threads_history.rs` lines 113–119). The core also refuses to drop writing review context from a replacement. `withdraw` rejects only calls *from* a Rewrite run. It accepts a Rewrite proposal that an earlier Run in the same chat prepared. Pending feedback survives, as it does after an owner dismissal (`threads-behavior.md` line 11). But a whole-note proposal the owner explicitly asked for can disappear before the owner sees it, and once retired it cannot be applied even after Undo. This fits the slice's "prepared by agent Runs in the same chat" wording, so it does not block, but it is a product choice worth confirming.

## Unfenced retirement

`Store::retire` opens its own immediate transaction and checks only for an existing receipt. The Run binding is checked later, in the cleanup's `apply`. The handler holds the in-process `scope.store` mutex, but a desktop Stop or a CLI process writes through a separate connection. If a stop commits between `live()` and `retire`, a Run that is no longer Working still retires its candidate. The cleanup then returns `Superseded` and the tool reports a rejection. The chat keeps a Review item whose Apply fails with "candidate retired"; the owner's Dismiss clears it. The verification describes this end state only for a stop *after* retirement. Refresh has no such gap: `replace_candidate_inner` retires and re-points attention in one fenced transaction.

## Replay after Undo

`withdrawing_twice_replays_without_another_receipt` covers replay only when no attention is left. After an owner Undo of the cleanup (G-A6 shows the Review item returns), the replay branch finds attention naming the operation again. It writes a fresh cleanup with a new digest and a new receipt. The candidate is retired either way, so nothing the owner could apply is hidden. But the agent reverses a visible owner action, and the Undo paragraph in the verification does not mention it.

## Tests and evidence

Not tested: a proposal from a project-member chat (the membership rule is the same as for another chat, but no case exercises it), a call from a Rewrite run, and a call from a stopped Run.

The coder wrote the implementation before the tests, which reverses the plan's order; the verification says so. Each test's fail-before comes from a mutation of the fixed source. M0 (no catalog entry and no `parse_valid` arm) reproduces the pre-fix "unknown tool" state. The cited failure lines match the committed files, apart from `:3916` against today's `:3918`, which fits M0 removing two lines above it. The counts are consistent: brn-ai +2 to 343, workspace 793 (slice F) + 8 = 801, desktop unchanged at 158, and the only failure is the baseline Poppler test.

The integration items cover the tool count in `threads-behavior.md` §142 and `docs/status.md`, the new activity label and a possible renumber at merge. They omit the bundle history at `threads-behavior.md` line 95, which still ends at `-11`.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/run_attention.rs`: new `CleanupOutcome`, `Store::clear_review_attention`, `Store::dismiss_candidate` (the owner loop, moved).
- `crates/brn-threads-app/src/review.rs`: `review_dismiss`/`review_apply` delegate to core and map outcomes to the existing error texts.
- `crates/brn-ai/src/threads.rs`: `WithdrawArgs`, `parse_valid` and schema arms, `tool!`/catalog entry, the `withdraw` handler, outcome labels, tool count test 9 → 10.
- `crates/brn-ai/src/threads_research.rs`: `#[cfg(test)] test_child_scope`.
- `crates/brn-ai/src/threads_research_tests.rs`: helper exclusion test.
- `crates/brn-ai/src/threads_trace.rs`: the trace records only the operation ID.
- `crates/brn-ai/src/threads_guides.rs`: bundle `-12` and the guide test.
- `agent/BRN.md`, `agent/skills/resolve-conflict/SKILL.md`: one sentence each.
- `crates/brn-ai/README.md`: "Ten registered tools".
- `crates/brn-threads-app/tests/withdraw_proposal.rs`: six scripted app tests (new).
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice G section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show 7ed74f26`.

</details>
