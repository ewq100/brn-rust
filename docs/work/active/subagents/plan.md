# Helper sub-agents: plan

**State:** PROPOSED design, awaiting owner decisions (14 October 2026). No implementation, schema change, guide change or live provider call is authorized. The [design](design.md) holds the details.

## Outcome

The main agent can hand bounded read-only research to an owner-selected helper model, such as Sol delegating to Luna. It gets back a compact report whose quotes the host has verified at exact note versions, without losing scope, approval or no-fallback guarantees. The research scout comes first; later roles reuse the same mechanism.

## Owner decisions

The design's section references point to these numbers.

1. **Direction and architecture amendments.** Accept read-only helper sub-runs inside a parent tool call. Replace "Keep one provider run before considering parallel investigation" in the threads target, clarify "Concurrent provider workers remain deferred" in the behavior doc, and extend invariant 16 to the helper model.
2. **Helper model setting.** Add an owner-only setting, off by default, on the parent's provider and account in phase 1. The parent model never chooses the helper model. Proposed evaluation effort for Luna: high, with medium compared.
3. **Unavailable helper.** Proposed: the sub-run ends `unavailable`, the owner sees a notice, the parent may continue with its own tools, and no other model is called. Alternative: strict mode, in which the parent run fails.
4. **Authority.** Helpers are read-only, cannot draft candidates and cannot spawn helpers (depth 1). There are no writer sub-agents.
5. **Helper context.** Proposed: the owner's current instruction and Project instructions. Alternative: recent conversation turns as well.
6. **Records and cost display.** Add `delegations` and per-model usage to the Run record (additive, no text). Show token usage per model, and decide whether to also show a credits or cost estimate.
7. **Budgets and parallelism.** Starting values: depth maps to 3, 5 or 8 rounds; per parent run, at most 4 calls, 8 sub-runs, 3 concurrent and 800k helper input tokens.
8. **Phase 1 surface.** Proposed: all chat runs except writing Rewrite runs. Alternative: Ask-only first.
9. **Evaluation.** Authorize at most 40 live Codex runs on synthetic data (38 planned: Sol alone versus Sol with a Luna helper) after the offline tests pass.
10. **Later scope.** The digest, triage and verification roles, and any cross-provider helper, each need their own later decision and consent.

## Phases

| Phase | Work | Gate |
|---|---|---|
| 0 | Owner review of this design | Decisions 1–10 |
| 1 | Research scout, Run usage, nested trace, offline tests, then live evaluation | Design §8 acceptance criteria, independent review, owner acceptance |
| 2 | Long-document digest, citation chips | Separate cases and runs |
| 3 | Knowledge-review triage, draft verification | Separate cases and runs |

## Handoff

Start from the [design](design.md). After decisions 1–9, write a phase 1 implementation plan following the [development workflow](../../../development/workflow.md), including its compatible reuse check. The design proposes adapting Rig's agent-as-tool pattern inside a BRN host tool instead of adopting `Agent::into_tool` directly.
