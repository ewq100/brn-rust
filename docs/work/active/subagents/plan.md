# Helper sub-agents: plan

**State:** ACCEPTED (14 October 2026). Phase 1 is **accepted as experimental, off by default** ([final decision](#final-owner-decision-phase-1-14-october-2026)). The owner decided items 1–9 as recorded under [Owner decisions — as decided](#owner-decisions--as-decided). Phase 1 implementation and up to 40 live Codex runs on synthetic data are authorized once the offline tests pass. The [design](design.md) holds the details.

## Owner decisions — as decided

| # | Decision |
|---|---|
| 1 | **Accepted** the direction and the architecture amendments: read-only helpers, an owner-chosen helper model, and the main agent as the only writer. The "one provider run first" rule is amended. |
| 2 | **Owner-only setting, off by default, any configured provider allowed** (not limited to the parent's provider). Settings shows a visible note: "Helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results." Since the final decision, it reads: "Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower." Choosing a helper on another provider is the owner's consent to send helper context there. Its credentials come from the existing configured routes, so the auth file or credentials folder is explicit. |
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
4. Each question runs as one helper sub-run with a child scope: the same store, chat, Project, run and fence (so `live()` fences it), its own read ledger and trace, no sources, no owner request, a role flag that every write tool refuses, and 5 tool rounds. Its `read_records` returns only Note, Link, Source and Comment records, and a `query` matches notes only, so conversation messages, chats and run progress never reach a helper. Citation checks reject superseded notes and compare a historical version only when that revision is visible in the parent's chat. The child prompt holds only the question, focus, known notes, date, owner instruction and Project instructions.
5. The helper ends with `submit_findings`, validated like other tools. The host then verifies every citation against the store in the parent's scope before returning a capped report framed as untrusted evidence.
6. Budgets per parent run: 4 research calls, 8 sub-runs, 3 concurrent, 800k helper input tokens; a 180 s wall-time cap per sub-run.
7. Unavailable (credentials, refusal, rate limit or network before any report): that sub-run returns `unavailable`; later research calls in the run return `unavailable` at once; an Activity line and a host notice say "helper unavailable"; no other model is called. Helper clients go through the same bounded same-request re-send as the parent (each sub-run its own budget); re-sends and failure detail stay in the helper's trace, and the notice adds only a fixed reason and the HTTP status (a provider 5xx counts as unavailable).
8. Usage goes to the opt-in trace only (`helpers` under the parent trace); the Run record is unchanged.

**Checks.** Offline unit and scripted-model tests for scope and privacy, citation verification, read-only enforcement, budgets and concurrency, cancellation, unavailable without substitution, the parent's full-read guard, trace content levels, Codex and Copilot routes, and setting-off absence; then the repository gates in [verification](../../../development/verification.md). The live evaluation follows design §8 after review.

**Deviations from the design, by decision.** No `depth` argument (decision 7 fixes 5 steps); no Run `delegations` or usage fields and no usage line (decision 6); helper usage is not sent to the parent model either; no `read_history` for helpers (decision 4's tool list); any configured provider with explicit credentials (decision 2). Added by implementation: a 180 s wall-time cap per sub-run so a stalled provider cannot hold the parent; on ChatGPT the helpers share one cache key per chat (`brn-<chat>-scout`), separate from the parent's; the harness counts helper reads as evidence for `read` and privacy checks.

**Evaluation.** `scripts/evaluate_wiki_v2.py` takes `helper=MODEL:EFFORT` as an arm option (configured on the arm's provider and credentials; other arms are configured with `--helper off`), reports helper runs, outcomes, citation verification counts, helper tokens and peak parent turn input, and reads nested helper traces for evidence. The v2 fixture adds the design §8 research-heavy cases M1 (multi-note synthesis with a superseded note), M2 (parallel supplier sub-questions), D1 (a long imported document with a planted exception) and I1 (a planted injection in a note, zero tolerance). They seed and pass the offline harness run.

## Handoff

Implemented and verified offline on `work/subagent-design`; not reviewed, accepted or merged. Next: an independent review, then the lead's go-ahead for the live evaluation (≤ 40 Codex runs on synthetic data), recorded in this folder. At merge, check the guide bundle number against main. Phase 2 (more roles, writing helpers) needs a new owner decision.

## Phase 1 live evaluation (14 October 2026)

**Setup:** branch `work/subagent-design` at `a71506d2` (bundle 11), Codex `gpt-6.1-sol` at medium effort.
- Arm A: the parent alone.
- Arm B: the parent with helper `gpt-6-luna` at high effort.
- Smoke test (R4, 2 runs) plus the main comparison of R4, R7, R9, R10, M1, M2, D1 and I1, two repetitions each (32 runs).
- 34 of the 40 authorized runs were used. The 4 medium-effort helper runs were skipped because the helper was almost never called.

| | A (Sol alone) | B (Sol + Luna helper) |
|---|---|---|
| Cases passed | 7/8 (14/16 runs) | 7/8 (14/16 runs) |
| Median cost per run (Codex credits) | 0.821 | 0.724 |
| Cost, sum of per-case medians | 6.45 | 6.42 (0.99×) |
| Median tokens per run (secondary) | 24.2k | 26.2k |
| Median latency | 21.0 s | 19.7 s |
| Runs that called `research` | — | 2/16 (both R7) |
| Helper outcomes | — | 1 `failed` (Luna sent an invalid batch of 16 tool calls), 1 `not_found` |
| Citations (exact/stale/mismatch/rejected) | — | 0/0/1/0 |
| Safety blockers | 0 | 0 |

- D1 (long-document digest) failed on both arms in the same way: the answer summarized the opening of the manual and never read the planted section. This is not helper-specific.

**Against the acceptance criteria:** 1 (safety), 2 (quality) and 5 (latency) pass. 3 (citation integrity: one citation, a mismatch) and 4 (value: 0.99× cost at the same quality) **fail**.

*Corrected 14 October 2026:* this section first compared raw tokens. Criterion 4 is judged on cost (credits per 1M tokens: Sol 50 input, 2.5 cached input, 250 output; Luna 2.5, 0.25, 12.5). The cost figures above come from `evaluate_wiki_v2.py --report-only` over the stored run traces, and the verdict is unchanged.

**Reading:**
- With prompt caching and host intake in place, the parent's own searches are cheap, and the parent chose not to delegate on these cases.
- The one helper failure points to Luna's tool-batch discipline.

Per the design's rule, phase 1 is **not shipped** on this evidence. The code stays on this branch, reviewed and off by default.

**Options for the owner:**
- (a) Keep the branch unmerged and record the result.
- (b) Iterate on what could create value: parent guidance that delegates long-document digests and multi-question research, D1-type cases, and helper batch limits. Then re-measure.
- (c) Merge, off by default, as an experimental setting.

## Iteration 1 (15 October 2026)

The owner chose option (b): teach the parent when to delegate, fix helper batch limits, add D1-type cases, then re-measure with at most 20 runs.

**Diagnosis from the run traces.**

- *Why Sol did not delegate.* On M1, M2, R4, R9, R10 and I1, Sol answered in 2–5 rounds from compact search rows and small notes; nothing in its context suggested a read-heavy task. On D1, `search_notes` gave no size signal, so Sol read the 120 KB manual whole with `read_records` (121 KB result, 34k input tokens): exactly the digest a helper should do. The guidance named "a long document" but the host never said which notes were long.
- *Helper failure.* Luna's 16 calls were spread over 5 rounds (at most 4 per turn), not one batch. Its sixth turn, past the round budget, was a `submit_findings` that listed all 16 searches against a cap of 12, so `BatchHook` stopped the sub-run as `invalid_batch`. The role guide asked for "every query you ran". Even a valid report in that turn would have been refused, because a report-only turn counted as a sixth round.
- *D1.* All four D1 answers did report the exception (phone, duty manager, holiday). The check failed only because `read_range` counted `read_note` ranges, while Sol read the whole note by ID with `read_records`. The question needs the planted section and the answer checks were fair; the read check was not.

**Changes.**

- Host signal: when `research` is offered, `search_notes` results and `read_records` listings add `large_notes` (notes of 32 KiB or more, with sizes) and a one-line hint that `research` can digest them. Notes read by ID get no hint, and runs without a helper see unchanged output.
- Parent guidance (`agent/BRN.md`, bundle 11) and the tool description name three triggers: a `large_notes` note that must be digested for a specific detail, two or more independent sub-questions, and evidence spread over many notes. Delegation stays optional.
- Helper batch discipline (`BatchHook` for helpers only):
  - The host runs at most 6 calls per turn and skips the rest with an instructive result.
  - A turn of only `submit_findings` never spends a round. After the 5 rounds, one grace turn runs only `submit_findings` and skips other calls with "budget spent, submit now"; a later turn stops the sub-run (`budget`).
  - Schema-invalid helper calls, including an over-limit report, are skipped with the limits spelled out, so the helper can resubmit. Unparseable or non-helper tool calls still end the sub-run.
  - `searched` and `gaps` past their caps (up to 4× the cap) are trimmed rather than rejected.
  - The role guide states the per-turn limit and asks for at most 12 searches, most informative first.
- Harness and cases:
  - `read_range` accepts a `read_records` read by ID that covers the range: a whole note, or a truncated prefix ending past the range.
  - D1 adds an answer check for the sign-off deadline ("08:00" or "next working day").
  - New case D2, a long-document digest: two protected imports of 200 KB and 150 KB, with planted sections past 128 KiB. The two-part question needs both sections.
  - New case M3, independent research questions: three separate sub-questions about an office move (provider and install date, agreed rent, key handover), with a stale quote, a draft budget and 20 similar office notes as traps.
  - All cases were validated and seeded offline.

**Deviation.** The design's "5 steps per helper run" now means 5 tool rounds plus one report-only grace turn (at most 9 model turns with retries). This keeps a late report from being lost.

## Iteration 1 live re-measure (15 October 2026)

**Setup:** branch `work/subagent-design` at `acde9910` (bundle 11), Codex `gpt-6.1-sol` at medium effort. Arm A was the parent alone; arm B added helper `gpt-6-luna` at high effort. Cases D1, D2, M3, M1 and R7 at scale 1000, two repetitions each: 20 runs, the full authorized budget. Two runs ended in a provider `server_is_overloaded` stream error (D1 A-r2 and R7 B-r2). They are excluded, not counted as passes, and were not re-run.

| | A (Sol alone) | B (Sol + Luna helper) |
|---|---|---|
| Completed runs passed | 9/9 | 9/9 |
| Runs that called `research` | — | 6/9 (D1 2/2, D2 2/2, M3 2/2, M1 0/2, R7 0/1) |
| Helper outcomes | — | 13 `answered`; no `failed`, `budget` or `stopped` |
| Citations (exact/stale/mismatch/rejected) | — | 10/0/7/1 (56% exact) |
| Median cost per run (Codex credits) | 1.011 | 0.700 (parent 0.659, helper 0.071) |
| Cost, sum of per-case medians | 7.31 | 4.47 (0.61×) |
| Median total tokens, parent + helpers (secondary) | 41.8k | 67.4k |
| Median latency | 17.4 s | 32.3 s |
| Safety blockers | 0 | 0 |

Per case, medians of cost, total tokens and latency (A → B):

| Case | A | B | Cost ratio | Note |
|---|---|---|---|---|
| D1 | 1.68, 60.3k, 16.1 s (1 run) | 0.86, 159.8k, 53.5 s | 0.51× | One helper read the 120 KB manual whole with `read_note` at `max_bytes` 131072, then again with `read_records` (187k helper tokens, cheap at Luna's price). |
| D2 | 3.05, 120.2k, 13.3 s | 0.56, 65.3k, 30.0 s | 0.18× | The parent sent two helpers and read only the two planted ranges itself. |
| M3 | 0.71, 17.9k, 17.0 s | 0.61, 76.5k, 37.3 s | 0.86× | Three helpers for three small notes. Sol alone needs 2 rounds. |
| M1 | 0.93, 33.4k, 30.4 s | 0.89, 32.3k, 30.9 s | 0.95× | No delegation. |
| R7 | 0.94, 48.7k, 35.1 s | 1.55, 52.6k, 25.8 s (1 run) | 1.65× | No delegation; parent variance on one run; no leak. |

*Corrected 14 October 2026:* this section first judged value on raw tokens, which ignores that Luna costs 1/20 of Sol. The cost columns use the prices above and replace the token verdict.

**What the iteration fixed.**

- Sol now delegates when the triggers apply: every D1, D2 and M3 run in arm B called `research`, and none of the small or privacy cases did.
- The helper batch failure is gone: 13 of 13 helpers answered. One late `read_note` was skipped by the grace rule, and the helper still reported.
- D1 now passes on both arms with the fairer read check.

**What the data shows next.**

- *Mismatches (7 of 18 citations).* They came from helpers that had read their target notes and answered correctly. The likely cause is byte offsets the model computed itself. Traces below Full hold no quote text or cited IDs, so this is not confirmed. The quote is dropped, and the claim stays unsupported (safe, but wasted).
- *Rejected (1).* Luna miscopied a UUID (`422dd2ab` for `422db2ab`). Its reads of that ID failed, and the host dropped its citation of it. This was not a visibility rejection.
- *Helper read size.* Helpers may read up to 128 KiB per `read_note`. One D1 helper did, then re-read the note by ID, which accounts for most of D1's helper tokens.
- *Latency.* Helpers add a serial phase of 15–40 s. Even D2, the clear cost win, is 2.3× A's latency.
- *Small multi-question asks.* M3 shows that independent sub-questions alone do not justify delegation when each answer is one short note.

**Against the acceptance criteria:**

| # | Criterion | Result |
|---|---|---|
| 1 | Safety: zero blockers | **Pass** on the cases run (no R7 leak, no writes, no out-of-allowlist helper call). I1 was not in this re-measure. |
| 2 | Quality: B passes every case A passes | **Pass** on completed runs: 9/9 and 9/9. R7 B and D1 A each have one run lost to the provider; R9 and R10 were not re-run. |
| 3 | Citation integrity: ≥95% exact, no visibility rejection reaching the parent | **Fail**: 56% exact. The one rejection was dropped before the parent saw it. |
| 4 | Value: ≤0.7× cost at equal quality, or a quality gain at no more cost | **Pass where the helper is used for digests** (corrected to cost). D1 is 0.51× and D2 0.18× at equal quality. M3 is 0.86×, which is cheaper but above 0.7×. M1 (0.95×) and R7 did not delegate. All five cases together are 0.61×. R4 and M2 were not re-run. |
| 5 | Latency: B p50 ≤ 1.2× A | **Fail**: 1.85×. |

Per the design's rule, phase 1 is still **not shipped**. The code stays on this branch, reviewed and off by default.

**Proposed next iteration** (offline changes, then a smaller re-measure on D1, D2 and M3):

- Have the host locate a quote that the helper's range missed, at the cited version. A quote found exactly once in that version would be verified as `exact` with the corrected range; otherwise it remains a `mismatch`. This keeps the host the verifier and removes model arithmetic from the integrity measure.
- Cap a helper's `read_note` slice (for example 16 KiB), and do not let helpers read notes whole by ID with `read_records`. A helper then digests by section, not by reading the whole document.
- Narrow the multi-question trigger to sub-questions that each need several reads or searches, so asks like M3's stay with the parent.
- Accept that latency will rise for long-document digests, or amend criterion 5 for that class: the owner decides.

## Iteration 2 (14 October 2026)

**Owner decision:** "Iterate once more: host relocates quotes (citation fix), cap helper reads to ~16 KiB, keep delegation for large notes and real multi-question asks; re-measure D1, D2, M3 (~12 runs); accept slower latency for digests if cost/quality hold". The owner also set criterion 4 to be judged on cost, not raw tokens, and relaxed criterion 5 for long-document digests (D1, D2): slower latency is accepted when B's median cost is at most 0.7 × A's and quality is equal. [Design §8](design.md#8-evaluation-plan) records both amendments. The other criteria are unchanged.

**Changes (offline, bundle 11 unchanged):**

- *Cost reporting.* `scripts/evaluate_wiki_v2.py` prices each run's parent usage at the parent model and each helper's usage at the helper model, with cached input at its own price. It reports the cost per arm and the per-case cost ratios (`--report-only` works on stored roots). Token counts remain secondary columns.
- *Quote relocation.* When a cited quote is not inside its range, the host searches the cited version for it. That version must be visible to the chat. If the quote occurs exactly once, the host re-anchors the range and marks the citation `relocated` at a current version, or `stale` with `relocated: true` at an older one. If the quote is absent or ambiguous, or the version is hidden, the citation is a `mismatch`. Traces and the report count `relocated` separately. For criterion 3, `relocated` counts with `exact` as host-verified stored text: the host, not the model, fixed the range.
- *Helper read cap.* A helper's `read_note` slice is clamped to 16 KiB (the result reports `max_bytes`). A helper's `read_records` by ID returns a longer note truncated at 16 KiB with `next_start` and its headings, and that read never counts as complete. The parent's reads are unchanged.
- *Guidance.* The multi-question trigger now names sub-questions that each need several notes or a long note. Several sub-questions that each one short note answers stay with the parent. Large-note delegation is kept. The research-scout guide describes the 16 KiB slices and asks for distinctive quotes.

## Iteration 2 live re-measure (14 October 2026)

**Setup:** branch `work/subagent-design` at `2d7687c8` (bundle 11), Codex `gpt-6.1-sol` at medium effort, authenticated by `--auth-file`. Arm A was the parent alone; arm B added helper `gpt-6-luna` at high effort. Cases D1, D2 and M3 at scale 1000, two repetitions each: 12 runs, the full authorized budget. No transport failures. Costs use the prices in [iteration 1's correction](#iteration-1-live-re-measure-15-october-2026).

| | A (Sol alone) | B (Sol + Luna helper) |
|---|---|---|
| Runs passed | 6/6 | 6/6 |
| Runs that called `research` | — | 4/6 (D1 2/2, D2 2/2, M3 0/2) |
| Helper outcomes | — | 5 `answered`, 1 `partial` (the helper's own status); no `failed`, `budget` or `stopped` |
| Citations (exact/relocated/stale/mismatch/rejected) | — | 5/6/0/0/0: 11 of 11 host-verified, none dropped |
| Median cost per run (credits) | 0.769 | 0.599 (parent 0.530, helper 0.069) |
| Cost, sum of per-case medians | 3.50 | 2.26 (0.65×) |
| Median latency | 19.4 s | 41.5 s |
| Safety blockers | 0 | 0 |

Per case, medians (A → B):

| Case | Cost A → B (ratio) | Latency A → B | Note |
|---|---|---|---|
| D1 | 1.011 → 1.330 (**1.32×**) | 19.4 s → 54.0 s | The question asks for an overall summary, so both helpers paged the whole 120 KB manual in 16 KiB slices (helper cost 0.144). In B-r1 the parent then read the whole note itself as well (121 KB, parent 1.19 credits). Arm A's prompt cache hit 56% and 92% of input, against 47% in iteration 1, so A cost 1.01 here against 1.68 then. |
| D2 | 1.903 → 0.599 (**0.31×**) | 25.8 s → 45.6 s (1.77×) | Two helpers each digested one long note; the parent read only the cited ranges. |
| M3 | 0.584 → 0.334 (0.57×) | 14.8 s → 14.6 s | No delegation in B, as the narrowed trigger intends. Both arms took the same path, so the cost difference is cache variance, not helper value. |

**Against the acceptance criteria** (with the owner's 14 October amendments):

| # | Criterion | Result |
|---|---|---|
| 1 | Safety: zero blockers | **Pass** on the cases run (no writes, no out-of-allowlist helper call). The privacy and injection cases (R7, I1) were not in this re-measure. |
| 2 | Quality: B passes every case A passes | **Pass**: 6/6 and 6/6. |
| 3 | Citation integrity: ≥95% verified, no visibility rejection reaching the parent | **Pass** if `relocated` counts as verified: 11 of 11 host-verified (45% exact as cited, 55% re-anchored by the host), no mismatch, no rejection. The relocation fix removed every mismatch (7 of 18 in iteration 1). |
| 4 | Value on cost: ≤0.7× at equal quality | **Mixed**. D2 passes (0.31×). D1 fails (1.32×). M3 did not delegate. All three together are 0.65×. Across iterations, D1 was 0.51× and then 1.32×: with N = 2, the parent's cache hits and whether the parent re-reads after a report decide the D1 result. |
| 5 | Latency: B p50 ≤ 1.2× A, relaxed for digests at ≤0.7× cost and equal quality | D2 is **accepted** under the relaxation (1.77×, 0.31× cost). D1 **fails** (2.8×, and its cost is above 0.7×). M3 is equal. |

Per the design's rule, phase 1 is still **not shipped** while D1 fails criteria 4 and 5. The code stays on this branch, reviewed and off by default.

**Reading.** The changes worked as intended:
- relocation turned every misplaced quote into a verified citation;
- the read cap held every helper slice to 16 KiB;
- the narrowed trigger kept M3 with the parent;
- D2-type digests (find specific rules in long notes) are a clear and repeatable cost win: 0.18× and 0.31×.

D1 is different: an overall summary needs the whole document. The helper reads all of it anyway (cheaply), and the parent sometimes reads it again.

**Options for the owner** (no change made):
- (a) Accept phase 1 for targeted long-note digests on the D2 evidence, recording D1 as a known non-win.
- (b) Add parent guidance not to reread a note in full after a report unless it will change it, and re-measure D1 with more repetitions (cache variance dominates at N = 2).
- (c) Keep the branch unmerged.

## Final owner decision: phase 1 (14 October 2026)

**Owner decision:** "Add the 'don't re-read after a report' guide line, rerun the safety cases R7+I1 once per arm (~4 runs), then merge off-by-default as experimental".

**Outcome:** phase 1 is **ACCEPTED as experimental, off by default**. It is merged once the safety re-run below shows zero blockers.

- *Citations.* The owner accepted that `relocated` citations count as host-verified for criterion 3: the host found the quote exactly once in the cited, visible version.
- *Latency.* The owner accepted slower runs: about 2× when the parent delegates.
- *Known limit.* Whole-document summaries like D1 are not a cost win, because the helper must read the whole document. Targeted digests of long notes like D2 are the measured win (0.18× and 0.31× cost).
- *Guidance (bundle 11).* After a report, the parent does not reread a whole note unless it will change it; it checks a specific cited passage with a `read_note` slice instead.
- *Settings note.* Desktop Settings and the CLI show: "Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower."
