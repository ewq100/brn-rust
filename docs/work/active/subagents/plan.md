# Helper sub-agents: plan

**State:** ACCEPTED (14 October 2026). The owner decided items 1–9 as recorded under [Owner decisions — as decided](#owner-decisions--as-decided). Phase 1 implementation and up to 40 live Codex runs on synthetic data are authorized once the offline tests pass. The [design](design.md) holds the details.

## Owner decisions — as decided

| # | Decision |
|---|---|
| 1 | **Accepted** the direction and the architecture amendments: read-only helpers, an owner-chosen helper model, and the main agent as the only writer. The "one provider run first" rule is amended. |
| 2 | **Owner-only setting, off by default, any configured provider allowed** (not limited to the parent's provider). Settings shows a visible note: "Helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results." Choosing a helper on another provider is the owner's consent to send helper context there. Its credentials come from the existing configured routes, so the auth file or credentials folder is explicit. |
| 3 | **Unavailable helper:** the main run continues, with a visible "helper unavailable" notice. No other model is substituted. |
| 4 | **Read-only helpers:** they search and read, then report. They cannot draft changes or spawn helpers. |
| 5 | **Helper context:** the main agent's research question, the owner's current instruction and the Project instructions only. |
| 6 | **Usage is hidden in the UI and kept in traces only.** There is no usage line on finished runs and no cost display. The "helper unavailable" notice from decision 3 still shows. The Run record gains no usage fields. |
| 7 | **Budgets:** 5 steps per helper run. Per main run, at most 4 research calls, 8 helper runs, 3 concurrent and 800k helper input tokens. |
| 8 | **Phase 1 surface:** all chat runs except writing Rewrite: Ask, intake and knowledge review. |
| 9 | **Evaluation:** implement phase 1, then run up to 40 live Codex runs on synthetic data after the offline tests pass. |
| 10 | Unchanged: later roles (digest, triage, verification) each need their own decision. |

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

## Phase 1 implementation plan

**State:** implemented offline on `work/subagent-design`; live evaluation waits for an independent review and the lead's go-ahead. Guide bundle `brn-threads-guidance-11` (10 is taken by other in-flight branches; renumber at merge if needed).

**Slice.** One `research` host tool and read-only helper sub-runs, an owner-only helper setting (core, CLI `configure`, desktop Settings with the tested-models note), packaged guidance, nested traces, the accepted architecture amendments, offline tests and harness helper arms. Where the owner decisions differ from the [design](design.md), the decisions win.

**Compatible reuse check.**

| Candidate | Choice | Why |
|---|---|---|
| Rig 0.44 `Agent::into_tool` | Adapt, do not adopt | It takes one free-text prompt and returns free text. BRN needs typed questions, a typed and verified report, per-call budgets, cancellation, trace and scope binding. |
| BRN `run_model`, `BatchHook`, `collect_stream`, `provider_client`, cache key and session pin | Reuse | A helper sub-run is an ordinary BRN run of the same loop with a role preamble, a four-tool allowlist plus `submit_findings`, its own hook and a child cancellation token. |
| BRN `search_notes`, `note_connections`, `read_note`, `read_records` | Reuse unchanged | Helper calls go through the same handlers, so chat and Project visibility, `live()` fencing and search scope apply as they do to the parent. |
| Store `record_visible_in_chat` and `read_note_range` | Reuse | Citation verification reads exact versions with the parent's chat visibility. |
| `tokio::sync::Semaphore` | Reuse | Caps concurrent helper sub-runs at 3. |

**Shape.**

1. `WorkspaceSettings.helper: Option<HelperSettings>` (provider, model, effort, explicit auth file or credentials folder). It is omitted when unset, so existing bytes and hashes stay unchanged; `configure` validates it and the settings version already fences runs.
2. `begin()` reads the helper from Settings and records a helper plan in the run scope: the owner's current instruction (≤ 4 KB) and Project instructions (≤ 2 KB). Writing Rewrite runs get none.
3. `research` is registered only when a helper is set, search is available and the run is not a writing Rewrite. Its arguments have no chat, project or model fields.
4. Each question runs as one helper sub-run with a child scope: the same store, chat, Project, run and fence (so `live()` fences it), its own read ledger and trace, no sources, no owner request, a role flag that every write tool refuses, and 5 tool rounds. The child prompt holds only the question, focus, known notes, date, owner instruction and Project instructions.
5. The helper ends with `submit_findings`, validated like other tools. The host then verifies every citation against the store in the parent's scope before returning a capped report framed as untrusted evidence.
6. Budgets per parent run: 4 research calls, 8 sub-runs, 3 concurrent, 800k helper input tokens; a 180 s wall-time cap per sub-run.
7. Unavailable (credentials, refusal, rate limit or network before any report): that sub-run returns `unavailable`; later research calls in the run return `unavailable` at once; an Activity line and a host notice say "helper unavailable"; no other model is called.
8. Usage goes to the opt-in trace only (`helpers` under the parent trace); the Run record is unchanged.

**Checks.** Offline unit and scripted-model tests for scope and privacy, citation verification, read-only enforcement, budgets and concurrency, cancellation, unavailable without substitution, the parent's full-read guard, trace content levels, Codex and Copilot routes, and setting-off absence; then the repository gates in [verification](../../../development/verification.md). The live evaluation follows design §8 after review.

**Deviations from the design, by decision.** No `depth` argument (decision 7 fixes 5 steps); no Run `delegations` or usage fields and no usage line (decision 6); helper usage is not sent to the parent model either; no `read_history` for helpers (decision 4's tool list); any configured provider with explicit credentials (decision 2).

## Handoff

Start from the [design](design.md). After decisions 1–9, write a phase 1 implementation plan following the [development workflow](../../../development/workflow.md), including its compatible reuse check. The design proposes adapting Rig's agent-as-tool pattern inside a BRN host tool instead of adopting `Agent::into_tool` directly.
