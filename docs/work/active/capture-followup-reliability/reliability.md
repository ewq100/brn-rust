# Reliability coverage and evidence

Base inspected: `bc699c41`. This lane extends the existing daily workflow,
agent-evaluation, v2 wiki and guidance-oracle tests. No additional harness,
provider call, Mac execution or production-service change is introduced.

## Inventory and selection

| Existing guarantee | Existing evidence | Decision |
|---|---|---|
| Protected contradiction, draft is not completion, source resupply/reopen | `daily_workflow_eval.rs` original daily journey and corrected-source case | Retain; no repeated scenario |
| Precise preview/Apply/Undo/reopen, dirty and stale owner buffers, lost Apply replay, dismissal, replacement, competing candidates | `daily_workflow_eval.rs` precise journeys | Retain; add only cross-boundary crash/owner-edit/retry composition |
| Killed proposal recovery, notices, lease/queue replay, stop/archive/continue | `killed_run_candidate.rs`, `run_recovery.rs` | Retain; do not add another isolated crash or queue test |
| Abandoned selected-skill candidate survives backup/restore and owner review/Undo | `int_d.rs` | Retain; no duplicate restore journey |
| Retrieval/version/visibility boundaries over real search | `agent_eval_journeys.rs` | Extend current-source read delivery for two new fixtures |
| Unconfirmed delivery, document exceptions, estimates, identifiers, conflicting measurements, topic correction/protection and duplicate evidence | Existing v2 R/S/M/D/I and DW1–DW12 cases | Retain; add withdrawn permission and partial acceptance, not more renamed examples |
| Meeting cardinality, saved/pending authority, source Current inputs, completion evidence | Guidance evaluator and tests | Close canonical changed-commitment source-check omission |
| Passed duplicate-meeting live observation | Exact `800d26bb` historical Mac receipt | Preserve; do not rerun under offline task authority |

## Added boundaries

- **DW13:** the current revision withdraws previously granted access; a draft must
  retract the old slot and leave replacement timing unknown. Historical validity
  must not be treated as current authority.
- **DW14:** accepting one quote item does not approve its other item or whole
  total. A draft retains item-specific amounts and the unresolved condition.
- A scripted runtime test reads both kinds of source through the actual tool
  adapter, asserts exact expected source text reaches the model, bounds requests,
  checks trace redaction, and leaves canonical records/candidates unchanged.
- A scripted abandoned precise proposal is followed by an owner edit, repeated
  orphan recovery, stale refusal, explicit dismissal and fresh reviewed retry.
  Undo must return to the owner's later text, not the older proposal base; the
  protected decision remains exact. This is an omitted-terminal crash fixture,
  not an actual process-kill or native interaction claim.
- v2 answer-exclusion checks now mark missing answers **unverified**, including
  partial multi-step results. An observed forbidden value still fails even when
  another answer is missing. Missing evidence cannot pass `case_passed`.
- The canonical changed-commitment guidance case now declares source steps
  `[0,1]`. The existing oracle requires both Sources on the active Action's Links
  and exact Current inputs. Regression demonstrates the old omitted field allowed
  a false pass, and that an inter-dispatch `[0]` check requires only evidence that
  has actually arrived. Its preexisting protected Note is preserved, not confused
  with an unauthorized new accepted record. Completion evidence remains null.

The new fixture answer markers are bounded lexical checks, not sufficient proof
of semantic grounding. Each fixture includes a human-readable rubric. Scripted
answers are authored inputs, not live model evidence. Correctness, grounding,
safety and recovery remain separate evaluator dimensions; token/cost evidence is
not synthesized from scripted responses. Real live usage/cost is unavailable
because no live calls occurred.

## Checks and limits

- Python v2 evaluator: **101 passed**.
- Python guidance evaluator: **32 passed**. New canonical-source regression first
  failed before adding `source_steps`; synthetic legacy omission remains explicitly
  tested as a false pass. Original red/green logs are preserved in
  `/workspace/brn-next-evidence/reliability/guidance-{red,green}.log`.
- Planner inspection: M1, DW9, DW10, DW13 and DW14 each select exactly one sample
  with repetition override 1 and no diagnostics; scales/budgets checked from
  current fixtures.
- Evaluator/scenario help commands passed without live execution.
- Rust targeted workflow tests: **19 passed** (`agent_eval_journeys`: 9;
  `daily_workflow_eval`: 10), no ignored or filtered tests. Command:
  `cargo test -p brn-threads-app --test daily_workflow_eval --test agent_eval_journeys --locked --offline --jobs 1 -- --test-threads=1`.
  Log `rust-journeys-1.log` identifies `/workspace/brn-reliability` as the rebuilt
  local application source; compilation took 10.47 seconds, tests 4.87 + 0.28.
- Strict targeted lint passed:
  `cargo clippy -p brn-threads-app --test daily_workflow_eval --test agent_eval_journeys --locked --offline --jobs 1 -- -D warnings`.
  Logs use `/workspace/brn-next-evidence/reliability/`; both existing saved base
  and native environment scripts were sourced, with one shared Cargo writer.
- Formatting/diff and Markdown links passed (673 Markdown files, 3,207 local links).

Historical cloud 355/156/1 credential-boundary evidence remains unchanged; these
selected checks do not resolve that earlier full-suite result. Native rendering,
OS access/focus/pickers and owner acceptance remain separate. See the
[proposed Mac headless handoff](mac-live-handoff.md), which requires new explicit
approval and excludes already-passed duplicate/precise samples.
