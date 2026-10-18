# D1A Opus review, round 3 (read-only) — efc0e380

**Verdict: CHANGES_REQUIRED.** Both round-2 findings are corrected for the paths they named. Two Medium defects and one Low defect remain. 202bf introduced the two Medium ones and rounds 1 and 2 (including my round 2) missed them; the efc correction did not cause them. efc did introduce the Low one.

I made no edits, ran no builds, tests or mutants, and delegated nothing. I skipped the code-review skill because it hands work to sub-agents.

## Identity and receipts
- **Checkout:** HEAD, whole tree and crates tree match the given SHAs. The only untracked file is `review2-pin.json`.
- **Changed code:** `d3f..efc` changes only the five named crate files.
- **Hashes:** each file's SHA in `review2-ai-final.json` matches efc. The AI log hash is `b1f71e80…` and its results sum to **386/0/1**. The before log hash is `e262d24d…`.

## Round-2 corrections

| Item | Status | What I checked |
|---|---|---|
| M1 (`read_note`) | **Fixed** | A `Value` tool output reaches `ToolOutput::json(value)` unchanged in Rig 0.44 (`output.rs:198-223`, `contextual.rs:186`). The hook's `result.output()` is that same `&ToolOutput`. Both sides use the same `estimate`, and `<=` against `>` gives one boundary, pinned on both sides by `exact_tool_result_admission_boundary…`. `fit_prefix` only cuts at character boundaries, grows monotonically and never returns an empty partial. The ledger slice records only the delivered `end`. |
| M1 (`read_records`) | **Fixed** | Visibility, writing, role, helper and query guards are unchanged. Omitted rows come only from eligible rows, in canonical order, with at most one partial Note followed by a stop. Ledger writes happen after the final fit, including the `large_notes` hint. Omitted rows and unshown suffixes grant no read, quote or proposal coverage. |
| M1 (wording) | **Fixed** | The result-limit cause (`AtomicU16` value 2) and the request-limit cause (value 1) now produce different notices. Cancellation still takes precedence, and the invalid-tool override is unchanged in both the main and helper paths. |
| L2 (spoofing) | **Fixed** | All `brn_*` codes now require `origin == context_budget`. The provider's origin is set by the host (`http_status`). The spoof test goes through actual HTTP sanitization. |
| Transcript | **Intact** | The new tests assert the earlier request history is an unchanged prefix of the later request. |

The new fitting code is application-specific and reuses Rig's output type and the existing core readers. It adds no framework, transport or second reader.

## Findings

**1. MEDIUM — write results can end the run after a committed effect.**
- **Where:** `threads.rs:1690` (`apply` returns the full receipt, with whole before/after records), `threads.rs:1557` (`prepare` returns the whole prepared request) and `threads_sources.rs:595` (figure assets are added to that request). The refusal itself is `threads_context_budget.rs:337-343`.
- **Why it fails:** an applied edit of a Note of about 66KB or more makes before + after exceed 135168 bytes. A selected figure of about 40KB or more serializes as a JSON number array of roughly 140KB+. The hook runs after the tool has executed, so the run ends as Failed after a committed apply or a stored proposal. The base (b617) had no result hook.
- **Bounded fix:** when the full output fails `result_fits`, return a compact form: operation, request_hash, write IDs with versions or expected versions and kinds, the refresh list, and asset metadata (media type, byte length, SHA-256), plus `compacted:true`. Keep the full output when it fits.
- **Tests to add:** a model-loop apply of a ~70KB Note that completes with a next dispatch, and a `prepare` with a ~40KB figure.

**2. MEDIUM — round-2 M1 is only partly corrected: two other readers still end the run.**
- **`read_source`** (`threads.rs:994`, `threads_sources.rs:72-79`) returns the whole source text, and sources are accepted up to 1MiB. Any source over about 130KB encoded ends the run, which affects the source-supported revision scenario.
- **`read_history` with a version** (`threads_history.rs:44-53`, Low on its own) still returns up to 131072 bytes with no encoded fit. The retained no-later-write witness depends on this.
- **Bounded fix:** add an optional `start` to `read_source` and reuse `fit_prefix` / `result_fits`, returning start, end, total_bytes, next_start and complete. Apply the same fitting to `read_history` (no ledger involved). Move the no-later-write witness to a test-only oversized tool.

**3. LOW — efc regression: `next_start` at the end of a section.**
- **Where:** `threads.rs:961-976` recomputes `next_start` against `total_bytes`.
- **Why it fails:** the core bounds it by `section_end` (`read.rs:139,158`), and `brn-threads-core/tests/note_read.rs:48` asserts it is `None` at the end of a section. A fully read section that is not the last one now returns `next_start = section_end`.
- **Bounded fix:** compute `limit = section_end.unwrap_or(total_bytes)` and use it in both the fitting closure and the final values. Add tests for a section that is not the last one, read whole and cropped.

## Test gaps (not defects)
- The heading test never proves the 8KiB heading limit or that the exact leading entries are kept. Its `headings_truncated` flag is already set by the existing 256-entry cap.
- The heading test also lacks a positive `covers(0, shown)` check.
- The CW2 undelivered-quote test asserts only a generic `is_err()`. That is acceptable, because the only difference from the successful proposal is the ledger coverage check, but a positive control (quoting a delivered range succeeds) would strengthen it.
- The medium-batch test does not assert exactly one partial Note or that Note's slice coverage.

## Limits
- This verdict comes from reading the source only; I ran nothing.
- The targeted mutation run on efc is still pending, within the remaining 1373.13s; this review does not require restarting all 41 mutants. The four earlier survivors need actual rechecks, and the tests added since are not retroactive kills.
- Full-workspace, native, hosted, live, owner, integration and program acceptance remain open. C02/C06 stay deferred to D1B.
- Nothing here is integrated or owner-accepted.
