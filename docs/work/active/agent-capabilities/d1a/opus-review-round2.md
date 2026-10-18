**Verdict: CHANGES_REQUIRED at `d3f67954`.** All five round-1 findings are corrected. The blocking issue is a new Medium finding: D1A introduced it at `202bf`, round 1 missed it, and this correction did not cause it. I made no edits or builds and delegated nothing. I skipped the repository code-review skill because it hands work to sub-agents.

The checkout identity matches: HEAD, whole tree and crates tree are as given. The receipts for the corrected suite list 44 source files, and every hash matches `d3f`. The precommit crates-diff hash equals the actual `202bf..d3f` crates diff, and the material-diff hash matches the retained file. The corrected log shows 381/0/1 (SHA `2b89e248…`). The before log shows 10/5 with the five named failures (SHA `b0aa9b0d…`).

## Round-1 findings

| # | Status | What I checked |
|---|---|---|
| 1 HIGH output cap | Fixed | Both `.max_tokens` calls are gone and no other cap is set. Parent and helper Copilot tests cover Chat then Responses, the model, effort, the same-model fallback and absence of all three cap fields. |
| 2 MEDIUM unknown cap | Fixed | The 4MiB constant passes the compile-time check: the envelope sum is about 2.7MB. The input bounds behind it hold (128KiB prompt, 128KiB project instructions, 60KiB intake). The wording distinguishes the local estimate from an unknown model limit. |
| 3 MEDIUM Copilot overflow | Fixed | The code is allow-listed, the error kind stays `Other`, and parent and helper tests show the budget outcome, one request and no canary leak. No app or CLI code reads the old code. |
| 4 LOW fixture | Fixed | The selection policy is recorded and no model scope was granted. |
| 5 LOW scan cost | Fixed | One invocation-local map built with the same visibility check. Order, cut and visibility are unchanged. |

## New findings

**1. MEDIUM: valid tool results are refused, which ends the whole run.**
- **Where:** `threads_context_budget.rs:31` (`RESULT`) and `:292` (the result check); `threads.rs:780-804` (`read_records` for a truncated note) and `:438` (`read_note` advertises a 131072-byte maximum).
- **Why it fails:**
  - Rig 0.44 serializes structured tool JSON once (`tool/output.rs`). Each newline, quote or backslash adds a byte, and each control character adds five.
  - After the 1024 estimate add-on and the read wrapper, a full 131072-byte read has only about 2.8KB of headroom.
  - A maximum `read_note` of Markdown with more than about 2.8K escaped characters is refused.
  - `read_records` by ID on any note over 128KiB returns the 128KiB prefix plus up to 256 heading entries. Headings and newlines will usually exceed the headroom.
  - Reading several medium notes by ID in one call can exceed it too.
  - Each refusal is a terminal budget failure on every route, including ChatGPT. The base admitted all of these.
- **Evidence:** the corrected max-read tests use `õ` or `x`, which need no escaping. The boundary test calibrates exactly to the limit. The existing large-note tests call `read_records` directly, outside the budgeted model loop.
- **Wording:** this refusal reuses the request-cap notice. On Copilot it says "request-size estimate cap was reached", when only the per-result limit was hit.
- **Bounded fix:** either make the read tools return whole content up to the encoded limit, with an exact `next_start` or the omitted IDs, or derive `RESULT` from the tools' real maximum output. Keep the 4MiB check consistent with whichever you choose. Add model-loop tests: a newline-dense maximum `read_note`, a >128KiB note with many headings read by ID, and a multi-ID read.

**2. LOW: a provider can spoof the local estimate notice.**
- **Where:** `error.rs:26`, combined with the `_exceeded` pattern in `provider_code` (`:93`).
- **Why:** a provider error code `brn_request_size_estimate_exceeded` passes the allow-list, so it would be shown as a BRN-local estimate. The `_unknown` code is filtered out.
- **Bounded fix:** only match the `brn_*` codes when the error origin is `context_budget`, and add a test with a provider reply carrying that code.

## Remaining risks and limits
- ChatGPT compares bytes against a 272000-token window. It is disclosed as conservative, and maximum escaped prompts or large project instructions can be refused locally.
- There is no fresh full-workspace result for `d3f`. The 1094/1/6 run belongs to the prior source.
- The mutation supplement has not started, and the new tests are not counted as mutant catches.
- There is no live, native, GUI, owner or integration evidence. C02/C06 stay deferred to D1B.
- The raw whitespace warning is cosmetic and is not a defect.
