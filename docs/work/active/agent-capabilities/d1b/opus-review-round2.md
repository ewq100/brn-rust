# D1B correction review, round 2: APPROVED

**Scope:** a read-only review of pin `a94b9c99` (tree `0e7f6710`, crates `100732a4`). I read the actual a673→a94 and 964→a94 diffs, the required evidence and Rig 0.44's Copilot authenticator source. I ran no builds, tests, Cargo, mutants or live calls, made no edits and delegated nothing. Every result cited below comes from the lane's recorded receipts. This verdict covers source review only; it is not principal source acceptance.

**Integrity checks:**
- **Pin state:** HEAD is a94. The only untracked file is `source-pin-round2.json`.
- **Fingerprints:** all seven file SHA-256s recorded in `review1-checks.json` match the a94 blobs.
- **Opus1 report:** the verbatim copy hashes to `aa44196…`, as claimed.
- **Unchanged since a673:** `Cargo.lock`, `brn-ai/Cargo.toml`, and the app, core and CLI crates.
- **Changed since a673:** only the five stated AI files.

## Original findings

**M1 (optional summary pushed a fitting request over the cap): resolved.**
- **Zero-history fallback:** this case now returns `Vec::new()` (`threads_context_budget.rs:229`), so no compaction is scheduled.
- **Admission check:** `prepare_for` builds the final prefix and runs `preflight` on it, which is the same initial estimate (preamble, tools and input). This happens after `host_intake::with_context`. If it fails, the input passed to chat is byte-identical to the plain input, with a notice, Activity and the `omitted_budget` / 0-covered trace.
- **Mandatory overflow:** still fails before any request (`mandatory_overflow_still_refuses…`).
- **Test:** the replacement test derives a near-cap input, proves the control request fits, then proves degradation and dispatch.

**M2 (credential rotation broke restart reuse): resolved.**
- **Key identity:** ChatGPT is keyed on `[base_url, account_id]`, falling back to a token digest only when the account ID is blank. Copilot is keyed on `[base_url, SHA-256(trimmed github-token)]`.
- **Why the Copilot identity is sound:** Rig 0.44 (`copilot/auth/native.rs:51-66, 251-278`) reuses or refreshes `copilot.json` only when it is bound to the current `github-token` fingerprint. So the exchanged key always belongs to the bootstrap BRN hashes.
- **Race handling:** BRN reads the fingerprint before and after authentication on the same `Auth`. A missing, invalid or changed fingerprint gives `unavailable_account` (no reuse, chat proceeds). Only an A→B→A swap during authentication escapes, which is negligible and leaks no data.
- **Auth semantics:** errors are swallowed only in the optional sidecar. `provider_client`'s signature and its research caller are unchanged. No raw secret is stored anywhere; it is hashed three times before reaching the key.

**M3 (no Copilot Chat→Responses fallback for compaction): resolved.**
- **One predicate:** parent and helper chat now share one predicate, refactored without changing its logic.
- **One client:** the compactor builds both routes from the same client config via `copilot_completion`.
- **Retry bounds:**
  - A retry happens at most once, inside the existing 30-second timeout.
  - The budget is unchanged, usage accumulates across both calls, and there is no cache key or output cap.
  - Context overflow and budget exhaustion are not `ModelRefused`, so they never retry.
- **Partial output:** the `StreamExt::inspect` flag records only that non-empty text arrived. The before-correction 0/1 diagnosis shows the partial-refusal test discriminates.

**LOW4 (over-invalidation): resolved.**
- **New key fields:** `PREAMBLE`, `BUNDLE_VERSION` and Project `{id, instructions, use_brn_knowledge, archived}`.
- **Visibility still covered:** project `trash` is dropped from the key, but the chat-archive transition carries that security change. Moving a chat changes the project ID.

**LOW5 (false gap when the lease is busy): resolved.** A fully covering, validated entry now returns before the lease. Final fresh revalidation still runs.

## Remaining items (none blocking)

1. **LOW, stale doc:** `d1b/implementation.md:22` still says "Credential rotation conservatively invalidates through a digest." That contradicts the M2 behaviour. Recommendation: add a one-line supersession note in the next evidence-only commit.
2. **LOW, test gap:** no test asserts the `omitted_budget` trace outcome or `covered_chunks: 0`. The only coverage is the notice and the input-equality assertions.
3. **Nonblocking, cost:** in the omitted path the compactor call is still made, and its result is published but not delivered. This is disclosed and bounded.
4. **Nonblocking, ChatGPT key granularity:** `account_id` is account-level, so two users switching within the same Team workspace would share the local cache. The cache is derived only from local canonical data, so nothing leaks.
5. **Unresolved but disclosed:**
   - The publish-time cancel check and freshness-key check still overlap other guards.
   - The `chunks <= evicted.len()` filter is redundant.
   - The coarse global change token, unmeasured full-record loads and no crash-orphan `.tmp` cleanup remain.
6. **Cosmetic:** the single-line project closure and the `matches!` call are left unformatted by rustfmt.

## Other checks

- **Raw history:** the transcript prefix is appended to, never rewritten.
- **Summary authority:** it stays untrusted user-role context.
- **Writing path:** v2 writing is untouched.
- **Rig reuse:** no duplicate production route or reader. The Rig `Compactor` and `copilot_completion` are reused.
- **Filesystem test:** it covers symlink and FIFO refusal (non-blocking open), oversize refusal, stage cleanup and 0600 publication. It is a private helper proof, as the lane stated.

## Still open

- **Mutation:** the original 6 caught / 2 missed stands. M02 and M07 now have discriminating fixtures, but they are not rechecked.
- **Other gates:** live, native, hosted, owner, `pdftoppm` and principal-acceptance gates remain open.
