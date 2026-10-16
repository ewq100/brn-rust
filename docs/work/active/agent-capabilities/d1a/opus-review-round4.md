**Verdict: APPROVED** for round 4 at `4db5b357`. I found no MEDIUM or higher defects in the seven changed paths.

I read the source only and ran nothing: no builds, tests, Cargo or mutants. I skipped the code-review skill because it delegates to sub-agents, which this task forbids.

**Identity**
- HEAD, the whole tree and the crates tree match the given SHAs.
- The efc→4db crates diff hash is `6e5d5e55…`, and it touches exactly the seven paths.
- The AI receipt records HEAD as efc plus WIP `650be3ae…`. All 44 owned source hashes in it match 4db.
- The AI log adds up to 393/0/1 and the before log to 26/5/0, with the five named behavioural failures.

**Round-3 corrections confirmed**

1. **Write results after a committed effect (M1): fixed.**
   - `prepare` (`threads.rs:1543`), `apply` (`:1572`, `:1843`) and refresh (`threads_history.rs:169`) still return the full output whenever it fits.
   - When it doesn't fit, they return compact identities. These include asset media type, byte length and SHA-256, using `content_hash` from core.
   - The durable `Prepared`/`Receipt` values are untouched. The output adaptation is infallible and runs after the read-ledger and attention work.
   - The last-resort acknowledgement (`:1493`) terminates and always reports `metadata_truncated`, `metadata_complete:false` and the omitted counts.
   - Refresh `complete` is set honestly, including when the core's `needs_refresh` IDs no longer resolve to records.
   - Withdraw and research produce no receipt-sized output. The CLI receipt parser in `contract.rs` reads CLI output, not tool output.
2. **Unbounded source and historical reads (M2): fixed.**
   - `read_source` checks scope through `source_value` before validating `start`. An invalid or out-of-range offset is refused.
   - Offsets refer to the text after figure-link remapping, and the schema says so.
   - The exact-version `read_history` path keeps its visibility guards, fits its passage and grants no current-note ledger coverage.
   - `fit_prefix` handles empty text correctly.
3. **`next_start` at the end of a non-last section (L3): fixed.** It is now bounded by `section_end.unwrap_or(total_bytes)`, which matches the core. Whole-note completeness is unchanged.

**Non-blocking observations**
- **Test gap (LOW):** the oversized-result witness (`threads_budget_tests.rs:473-544`) now builds the error with `exhausted_failure` instead of observing it. As a result, no test reaches the production mapping from the result-limit cause to the failed terminal (`threads.rs:3586`). I checked that mapping by reading it; it is correct.
- **Nit:** the compact `Prepared` output adds `"refresh":[]` (`:1558`), and its final fallback shapes refresh as an object with `complete:true`. `Prepared` has no refresh field, so this shape is inconsistent but harmless.
- **Existing behaviour, worth noting:** after a compacted apply, `reads.complete` (`:1812`) still treats the after-text as known, even though the model never saw the host's link remapping. That remapping is deterministic, and any later proposal still goes through exact-candidate owner review.

**Limits:** the mutation supplement for 4db is still pending, and I verified none of it. Full-workspace, native, hosted, live, owner, integration and program qualification remain open. C02/C06 belong to D1B. Nothing here is integrated or owner-accepted.
