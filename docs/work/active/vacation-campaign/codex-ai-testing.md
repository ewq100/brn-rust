# Owner-authorized Codex account tests

On19October2026 the owner explicitly authorized using their existing Codex account
for AI tests, including native AI testing, with reasonable limits. This updates
the earlier live-provider exclusion only for bounded Codex-account qualification.
The [campaign](plan.md) remains the execution/expiry/merge authority. Two bounded command-line probes have now run; their [qualification receipt](evidence/codex-ai-smoke-qualification.json) preserves one failed natural request and one successful explicit-ID diagnostic. Actual native AI testing remains pending.

## Limits and execution

- Use the existing signed-in Codex account and an already available supported
  model. No new API key, purchase, credit/reset redemption, acquisition, credential
  edit, other provider activation or model fallback. Do not copy/log auth material.
- Use fresh explicit synthetic BRN data only. Keep each initial scenario to one
  short synthetic Note and a small prompt/comment (at most8KiB combined input).
  No owner notes/vault, external sends, connectors/MCP activation or publication.
- At most3 attempted BRN AI runs per consolidated test session and6 per Tallinn
  calendar day, counting failures and uncertain launches. One lead/actor at a time.
  Reserve the attempt in the [ledger](evidence/codex-ai-test-ledger.json) before
  dispatch so restart cannot silently duplicate it.
- Limit each run to180seconds of client execution, then use the supported Stop/
  deadline path; record actual terminal/committed/unknown outcome. CLI uses
  `--timeout 180`; enforce the same deadline around a native run. Stop does not
  itself prove upstream cancellation. Use at most4 tool rounds when the current
  checked host exposes that setting; retain existing runtime/context/output guards
  everywhere. No retry loop or repeated generation merely to obtain a passing run.
- Read actual allowance before a live batch and after it. The campaign's roughly5%
  remaining floor overrides this budget; unknown/exhausted allowance defers calls.
  A failed account/provider check counts as an attempt and receives diagnosis
  before another attempt. These are usage limits, not a dollar estimate.

Start with one small normal-runtime account smoke test. If it works, use a v2
writing proposal with one captured comment and exact unchanged/unrelated checks;
review and Apply/Undo through ordinary controls in the batched Mac session.
The third attempt is available only for a distinct useful scenario or a diagnosed
repair. Record prompt/input hashes, source/binary/model identity, actual outcome,
useful saved result, unintended changes and available usage evidence. Keep secrets
and raw authentication responses out of evidence. Deterministic fixtures, live
model usefulness, native input, owner acceptance and hosted/main gates stay separate.
