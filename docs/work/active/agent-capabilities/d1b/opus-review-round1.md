# D1B round-1 source review: CHANGES_REQUIRED

**Pin:** `a6736855` (tree `0caa0135`). The diff from `964567aa` has the stated 18 files. `Cargo.lock` is byte-identical and the `export_metadata.rs` prefix is unchanged; the test is appended only. This is a read-only review: I made no edits, ran no builds, tests, Cargo or mutation, and delegated nothing. I read the source, tests and pinned Rig 0.44 (`Compactor`, `TokenWindowMemory`, `with_http`, `authenticate`, Copilot `ApiKeyRecord`).

The architecture is sound and most guarantees hold, but three actual gaps need fixing.

## Actual defects

**1. MEDIUM: the optional summary can turn a request that fits into a hard context-limit failure.**
- **Where:** `threads.rs:2442` checks the budget on the input *before* the summary is added. `threads_summary.rs:634-642` then prepends up to about 32 KiB unconditionally. `threads_context_budget.rs:221-226` still returns the evicted chunks when the "0/N optional entries" fallback fires.
- **Effect:** `BudgetHook` stops before dispatch, so the run fails. §5.1B says to prioritise mandatory content, disclose omissions and use one degradation policy. Near-cap inputs (cap 272000 for catalogued models) worked on D1A and fail on D1B.
- **Test:** `frozen_summary_is_in_the_actual_dispatch_budget` (`threads_summary_tests.rs:659-726`) asserts this failure.
- **Fix:**
  - Skip compaction when history took the 0/N fallback.
  - After rendering, check `budget.accepts(initial_estimate(scope, &prefixed))`. If it fails, use the plain input with a gap notice.
  - Change the test to assert degradation, and keep a separate mandatory-overflow failure test.

**2. MEDIUM: the cache key includes rotating credentials, which defeats restart reuse (C02) for Copilot.**
- **Where:** `threads.rs:2517-2524` digests `config.api_key`. For Copilot that is Rig's exchanged API key, re-fetched after its `expires_at` (in my understanding, roughly every 30 minutes). ChatGPT (`threads.rs:2463-2470`) also digests a refreshable access token even though `account_id` is present.
- **Effect:** routine token refresh is treated as an account change. Coverage rebuilds from zero, at most 4 chunks per run. Long chats may never reach full coverage, and prompt-prefix stability (C07) churns.
- **Test:** the Copilot fixture uses `expires_at: i64::MAX` (`threads_summary_tests.rs:737`), so this isn't caught.
- **Fix:**
  - Key ChatGPT on `[base_url, account_id]`, falling back to the token digest only when `account_id` is `None`.
  - Key Copilot on `base_url` plus a digest of the stable GitHub bootstrap credential from the same resolved credentials folder. That adds no second selection.
  - Add a test that rotates the exchanged token on the same account (expect "cached") and one that switches account (expect invalidation).

**3. MEDIUM: the Copilot route fallback isn't preserved for compaction.**
- **Where:** chat retries on Responses after a `ModelRefused` with empty output on the Chat route (`threads.rs:3555-3562`). The compactor uses only `routes_through_responses` (`threads.rs:2526`).
- **Effect:** for Copilot models served only on Responses, every run sends a refused request and reports a permanent gap.
- **Fix:** reuse the same single-retry route policy, for example via a small shared helper.

**4. LOW: the cache key over-invalidates.**
- **Where:** `threads_summary.rs:256` includes `run_preamble(scope)`. That text changes with whether the run has sources (`threads_guides.rs:44`), and the compactor never sees it. The key also serialises the whole project record, including its `chats` and `documents` lists, so adding another chat to the project invalidates every chat's summary.
- **Fix:** key on the compactor `PREAMBLE` and only the fields that matter: project ID, instructions, `use_brn_knowledge` and archived.

**5. LOW: a fully cached summary reports a false gap when the lease is busy.**
- **Where:** the lease is taken (`:552`) before the full-coverage check (`:557`).
- **Fix:** return a fully covering, validated pre-lease entry before trying the lease.

## Test-quality gaps (LOW)
- The publish-time cancel check (`:593`) isn't independently exercised, because Stop already fails the stream first.
- The key check in the freshness callback is redundant with the change-token check, and the `e.chunks <= evicted.len()` filter (`:613`) is unexercised. Expect these to survive mutation.
- There are no symlink, FIFO, oversize-cache or staged-file cleanup witnesses.
- Mutation survivors must be reported as original verdicts, not killed retroactively.

## Verified sound
- **Model call:** a real Rig `Compactor` with no tools, `max_turns(1)` and no memory, called outside DB transactions. One call per run, within the stated bounds.
- **Host stamps:** host-computed SHA, version, prefix digest and first/last sources. Unknown fields, foreign IDs and candidate-only "applied" claims are rejected. Every use revalidates against the current key and source bytes.
- **Rendering:** the summary goes into user-role "UNTRUSTED DERIVED" context. It grants no system, settings or ledger authority.
- **Publication:** after the staged file is synced, the code checks the lease, liveness, cancellation, change token and source prefix before the atomic rename. Same-chat fencing prevents an older writer downgrading newer coverage.
- **Files:** `NOFOLLOW` and `0600`, regular-file checks, size caps.
- **Requests:** the summary request has no chat cache key and gets a fresh session; chat keeps its pinned session and resend behaviour. `with_http` keeps the authentication Rig resolved once.
- **Other:** `TokenWindowMemory` keeps a contiguous suffix, so the evicted-prefix arithmetic holds. Trace format 1 is compatible. The writing path is untouched.

## Qualification limits (not defects)
- **Live-only (blocked by L):** with no structured-output schema, a model that wraps its JSON in a code fence would fail parsing, so every run would report a gap. Separately, the 30-second timeout combined with high effort may routinely expire.
- **Wasted calls:** the global change token (`data_version` plus revisions) means any concurrent commit discards an otherwise valid result.
- **Performance:** each run adds up to three extra full loads of all records and receipts while holding the store lock. This is unmeasured.
- **Untested here:** orphaned `.tmp` files after a crash are never cleaned up.
- **Pending gates:** the new D1B mutation campaign is still pending. This review isn't live, native, hosted, owner or integration acceptance.
