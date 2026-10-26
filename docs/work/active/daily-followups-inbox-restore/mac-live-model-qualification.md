# Mac headless live model qualification

25 October 2026. **Partial qualification, stopped on failure.** Two runtime defects and one evaluator-contract defect were corrected. The final duplicate-meeting recheck produces the correct structured owner and follow-up date, but still fails the completion-evidence expectation. The remaining cases were not launched. No Mac UI or owner acceptance is established.

## Source and authorization

The owner selected the [pinned prompt](https://github.com/ewq100/brn-rust/blob/68542a9762f97c3855470f082c50cb91b4a024da/docs/work/active/daily-followups-inbox-restore/mac-live-model-prompt.md), clarified a ceiling of 12 tool rounds, then instructed “Fix and continue.” Existing normal login and synthetic data only were used. No desktop interaction, unlocking, security changes, credential copying, other provider/model, merge or release occurred.

The initial source was `90e5bcebac1d64fc98ef315675fd2810e1e26f1e`. Repairs start from newer main `b270ad2fb0e67e10a86f68d513a37bf309760f97`, whose four required Mac/Ubuntu checks passed in [run 40189058904](https://github.com/ewq100/brn-rust/actions/runs/40189058904). Three optional Windows checks failed; this is not a full-platform pass. Its inbox-lease correction does not change the tested ask/intake paths.

Branch: `codex/action-prompt-live-fix-20261025`. First repair: `da06ce762b5858c7885f72307e3ccf0c1393272b`. Final tested runtime: `0df79011bfc08d8ca28a935117d14fa4cc6c1845`, ordinary CLI SHA256 `a570235c225960d8b0dada59f9dd27e93507d088d7b9952579cbfa40eb7af0de`. Rust 1.98.1 and locked offline builds were used. Current immutable stock/adapted helpers were reused: their source graphs are unchanged by these repairs.

## Corrections and deterministic checks

1. The canonical system prompt omitted `owner`, `waiting_on` and `follow_up`, although the tool description and intake playbook offered them. The actual runtime renderer now includes them, unknown values stay null, and revisit dates stay distinct from due dates. The instruction bundle advances from 20 to 21, including the existing cache-invalidation and CLI identity assertions.
2. Explicit nulls for those three offered Action fields failed strict tool argument validation because immutable serialization omits absent values. Validation now normalizes only those equivalent Action forms. Unknown fields, host-owned fields and fields on the wrong record variant remain rejected. Immutable request serialization, hashes, authority and storage contracts are unchanged.
3. The follow-up-only intake rubric permits a saved or pending Action, but a generic evaluator gate additionally demanded an applied Note. That gate no longer applies to intake-only cases requiring an Action. Cardinality, exact facts and saved-versus-pending state checks remain. Explicit requests to save a meeting account still require an applied Note. The unchanged original candidate still fails its missing-owner/date check under the corrected evaluator.

The prompt regression first failed with `missing Action field owner:null`. Two nullable-field tests then reproduced rejection through the validator and production Rig hook before the null normalization. The evaluator regression first rejected a valid pending Action without a Note. These failures are retained in local evidence.

Final checks:

- `cargo fmt --all --check`: passed.
- `cargo test -p brn-ai --lib --features capability-spike --locked --offline --jobs 1`: 509 passed, one existing ignored test.
- Strict all-target Clippy for `brn-ai` and `brn`, including `brn-ai/capability-spike`: passed.
- Ordinary `brn` binary build: passed; the copied immutable binary was used for the final live recheck.
- Guidance/v2/helper-path evaluator tests: 126 passed; scripts unchanged by the final nullable-field correction.
- CLI `agent_runs`: 15 passed, two existing ignored tests at the first repair; its configuration/identity paths are unchanged by the nullable-field correction. This is supporting earlier-source evidence, not a repeated final-head CLI test claim.

## Actual live results

All live Runs used offered `gpt-6.1-sol`, medium effort. Requests had a 180-second client/process bound; ask fixtures used eight tool rounds and intake the verified production default of 12. No helper model or model matrix was used. Fresh fixtures were used for each diagnosed repair; passed cases were not replayed.

| Source | Case | Runs | Result |
| --- | --- | ---: | --- |
| Initial baseline | Smoke | 1 | Exact `SYNTHETIC_READY`, no tools/candidate, canonical records and Unicode sentinel unchanged. |
| Initial baseline | Ambiguous meeting discussion | 1 | Passed structural and prose/source review; no invented Action; source protection and eligible grouped Undo preserved. |
| Initial baseline | Unknown-owner clarification | 1 | One Suggested Action, unknown owner/dates retained; exact source inputs and related links; eligible grouped Undo passed. |
| Initial baseline | Duplicate meeting copies | 2 | Failed: owner/follow-up only in prose, missing structured fields; no duplicate, one pending proposal. Original applied-Note failure also retained. |
| First repair | Fresh duplicate copies | 2 | Failed: answers reported rejected preparation formats; no Action/candidate created. Offline tests exposed the offered-null rejection. |
| Final tested runtime | Fresh duplicate copies | 2 | Correct structured owner/date and one pending proposal; failed because `Action.evidence` contains a Source ID instead of null. |

Nine attempted BRN Runs total, including every failed case. The owner-authorized corrections consumed four additional Runs; there was no blind retry. The final recheck stopped further live dispatch. Changed-commitment conflict, M1, DW9, DW10 and current-head precise edit remain unrun. Historical precise-edit evidence is not renewed by these tests.

### Final duplicate case

Both provider Runs returned `Completed`; that does not make the case pass. The final immutable candidate contains:

```json
{
  "state": "Open",
  "owner": "Mari",
  "follow_up": "2028-03-03",
  "due": null,
  "evidence": "dd8113c9-445d-4c71-997a-9b8c30ce5805:2"
}
```

The description keeps delivery due 10 March separate from the approval request's unspecified deadline. A refreshed single proposal retains the duplicate-source qualification, a Source link and exact observed input. Sending the request and receiving approval remain explicitly unconfirmed. Source coverage stays Unchecked.

Correctness therefore fails the completion-evidence expectation. Narrow source grounding, deduplication and the approval boundary pass: nothing was applied, no external action or completion was claimed, and no accepted Open commitment was manufactured. The shared review surface labels this field **Completion evidence**; a minutes Source ID is not proof of completion. Proposed next correction: clarify this field's role in runtime guidance and retain source provenance through Links and exact versioned inputs. That correction is not implemented or qualified here; the fixture was not weakened.

Candidate operation `ffa7fed7-a1c7-483c-9f5a-4c76ddc9e126`, request hash `b7e1d165e78c8b8d1b22e03522387d7eda7651e2b7809ed2ef1a06506c83ad39`. Later CLI processes confirmed the exact proposal remains pending with no saved Note or Action. Undo is unscored for these pending-only cases; no Apply was used to hide a failure.

## Usage and remaining gates

The smoke exposed 6,235 input tokens, 10 output tokens, zero cached input tokens, one model turn and zero tool rounds. The eight guidance Runs did not expose tokens/rounds in their responses. Total tokens, actual HTTP request count and dollar cost are unavailable, not zero. Native allowance remained admitted, ending at 75% remaining; this includes development usage and is not a per-test cost measurement. No credits were purchased or reset allowance redeemed.

Detailed redacted commands, timestamps, hashes, fixtures, snapshots, original failures and fresh-process checks remain in three local evidence directories, respectively prefixed `brn-live-20261025-`, `brn-live-repair-20261025-` and `brn-nullable-qualification-20261025-`. The receipts are read-only evidence for their recorded sources; no closed historical vacation or precise-edit session was reopened.

Independent code review is pending: the selected live-test prompt excludes helper agents, so lead inspection and tests are not described as independent review. Hosted checks for the repair and Mac UI/IME/focus/accessibility/file-picker acceptance are separate pending gates. This draft is not merged or released. No task-owned live worker or build remains running.


## Subsequent cloud follow-up

The owner later approved the narrow [completion-evidence guidance correction](completion-evidence-guidance.md).
This does not change the historical nine-Run result above. The cloud follow-up
adds independent source/evaluator review and deterministic checks; live semantic
requalification remains for the separately executing Mac agent. No Mac receipt
or failed candidate was rewritten.
