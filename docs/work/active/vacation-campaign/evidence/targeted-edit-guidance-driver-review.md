# Final-session guidance driver review

Reviewed the complete `/tmp/brn-targeted-edit-account/check-guidance.py` and its delta from the previously reviewed diagnostic driver. Exact reviewed SHA-256: `d7857d41619d718cd69ebf7729bb0607f184ab19c224b49c3f532e8168b8814b`.

**Disposition: no actionable findings for the specified third and final possible attempt in this consolidated session.** This is source-only review. No driver, account/provider, qualification process, test, build, native, Git, credential or network action was invoked. This report is the only authored file.

The delta changes the explicit fresh fixture root and frozen CLI/source identity, and repairs nonzero-envelope diagnosis. For a nonzero CLI exit, `error:null` now safely bypasses dictionary-only code lookup and falls back to `result.terminal` when the result is a dictionary, then to `failed`. Every such exit still raises; it cannot enter the success path. Raw stdout/stderr and the command receipt are saved before the envelope is interpreted. A malformed or unexpected envelope still fails closed and reaches the existing independent failure-state inspection. I did not execute the supplied offline replay; the fix is valid by source inspection for its stated envelope shape.

The new frozen identity is pinned to source `eac295b14ddb4a57b1e3b62e6c51f0d40ae699a5` and binary SHA-256 `8c76e00d29a0881a3af0c5c730a96234710b697e8419899682c288305dbcea57`. Product-source qualification and the claimed guidance-only behavior change belong to the lead's separate evidence; they were not independently requalified in this driver review.

The complete guards remain intact:

- Same consolidated session; all session entries count toward three, including the reserved third row and failures. Exactly one reserved row must match this fresh evidence root. The dated ledger count must be at most six. Consuming the reservation before dispatch prevents reuse. With the two prior counted attempts, a subsequent fourth reservation would fail the session-count check.
- Explicit Tallinn execution date and campaign expiry; ordinary usage allowed; remaining allowance strictly above five percent; recent nonfuture allowance observation. Actual fresh allowance and the after-batch reading remain lead actions.
- Frozen binary continuity; reserved-row binding to script and launch-seal digests; rechecked manifest, prompt, settings and canonical records; empty initial candidates; combined initial input at most 8192 UTF-8 bytes. Settings require Codex, Luna, low effort, chosen review-first delegation, disabled helper/MCP and the existing account pointer.
- One ask with four tool rounds, the supported 180-second deadline and separate 190-second emergency watchdog. Timeout output/receipt and cancellation uncertainty are preserved. No retry loop or Apply command is introduced.
- Passing still requires terminal Completed, full target/sentinel equality, one remaining candidate containing one exact target write at the captured version, unchanged archive state and all other Note data, candidate operation membership in the Run, a successful precise-edit trace result returning that exact operation, zero dropped calls, and absence of the listed alternate preparation routes. The envelope fix does not relax any of these proofs.
- Failure still records `passed=false`, independently inspects target/sentinel/candidates/runs with per-inspection errors, retains unknown state where unavailable, merges the current ledger entry, saves the result, and exits nonzero. `upstream_cancellation_confirmed` remains false.

As in the earlier reviews, this assumes ordinary unoptimized Python execution, the sole lead, an exclusively owned fresh fixture and the existing runtime's credential-output protections. The wrapper does not read/copy the auth file or alter global HOME/CODEX_HOME; its pathname replacement is not general secret sanitization.

Fresh actual quota and a durable matching reservation remain prerequisites before any launch; the after-batch quota reading remains required afterwards. This review introduces no owner approval gate and authorizes no fourth session attempt or repeat until green. No live-success, native-usability, owner-acceptance, merge or release claim is made.
