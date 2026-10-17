# D4 round-2 correction review: `cc8338be` against the reviewed `a5d148a3`

**Verdict: APPROVE WITH QUALIFICATIONS** for the corrected source only. I found no defect that needs a source change.

This review was read-only. I ran no builds, tests, Cargo, mutation testing, live MCP or GUI, made no edits, and delegated nothing. The only file I wrote was a temporary copy under `/tmp`, which I deleted.

**Identity check**
- I re-derived both diff hashes and they match the parent's: `a5d148a3..cc8338be` is `f597ad4c…7f30d`, and `9a88a0a9..cc8338be` is `b98cc941…b4c13`.
- The crates tree is `fb307685…` at both `cc8338be` and `6322048f`. The tree at `cc8338be` is `764dc4c7…`.
- Five source/test files changed, all in `brn-ai`. There are no manifest, lockfile, core, native, schema or authority changes.

## Findings

None that need a correction. Three informational notes:

- **N1 — the overflow observer duplicates the guard's check.** At `threads_mcp_http.rs:209-220`, the cfg(test) observer computes the same over-limit condition as the guard at `:225`, rather than recording from inside the guard's rejection branch.
  - What actually catches a regression is the outer assertion: if the guard is removed or the cap raised, mode 1 returns real data and `unwrap_err` fails.
  - So the test is valid. The observer only supplies the timing and byte evidence. No change needed.
- **N2 — modes 2, 3 and 4 can only assert the catch-all `Transport` class.** The SDK collapses adapter errors, as the allocation anticipated.
  - Each fault is still pinned down: mode 0 runs fault-free first, the elapsed time must be under DEADLINE/2, and a missing encoding check would let mode 3 succeed.
  - Removal of the redirect `Location` check is caught separately, by the mode-5 assertion in `sdk_typed_notification_acknowledgement_refuses_alternate_location`.
  - This is accepted as a qualification, not a defect.
- **N3 — the setup diagnostics cover only the client side.** The `TestServer` listener and accept side has none. A client-side connect failure (for example a raw OS error) would still have been reported at the `post_execute` stage. Not a defect.

## F1: corrected and confirmed

- **Production change.** `threads.rs:214-231`:
  - The original error is cloned into the failure answer and returned unchanged.
  - Only an `Err` from `finish` emits the exact approved static sentence to stderr. The write result is discarded, so a failed write cannot replace the primary error.
  - `Ok(true)` and `Ok(false)` are silent.
  - No storage text, path, input or endpoint is formatted. There is no new public error or trace shape.
  - brn-ai has no `tracing` usage, so a static stderr notice is the smallest available diagnostic.
- **The test drives the real `finish`.** `finish` (`threads.rs:3697`) fails at its first `lock().map_err(storage)`, before any write. The test-only hook at `:215` really poisons the actual `scope.store` mutex. There is no stub.
- **Test isolation.**
  - The hook is a thread-local one-shot (`replace(false)`) under `cfg(all(test, feature="scripted-runs"))`, matching the `mod mcp_tests` cfg at `threads.rs:52`.
  - Attachment is synchronous on the `tokio::test` thread, so the thread-local is sound.
- **What the new test asserts** (`threads_mcp_runtime_tests.rs:782-900`):
  - exact serialized equality with `mcp_runtime::unavailable()`;
  - the exact notice, with the input canary absent;
  - the `Scope` is dropped (the `Weak` reference no longer upgrades) and the lease is free and re-acquirable;
  - the Run record is byte-identical and still Working with the queued instruction;
  - the queue control is unchanged (no acknowledgement or unpause), with exactly one Run;
  - no canary in canonical records, and zero model requests, HTTP requests and calls.
- **The existing refusal test covers both silent paths.** `saved_review_refusal…` (`:768-777`) now asserts no notice for the `Ok(true)` pre-prepare case and for the `Ok(false)` already-Interrupted post-prepare case.
- **The before-witness is genuine.** `attachment-finish-before-module-corrected.log:12-14` shows `{"kind":"storage"}` replacing `other/mcp_client/mcp_unavailable` at `:844`. The earlier panic at `:786` is the intended poisoning panic, not a separate failure.
- **Honest semantics.** A Run left Working after a failed persist, with its lease released, is recovered by the existing orphan path. No terminalization or recovery is claimed.

## F2: qualified, as stated, not fully met

- **Fixture.** Modes 1 and 7 now prefix comments to the real SDK-generated SSE body (`http_tests.rs:151-169`) and remove `content-length`. There is no handwritten protocol or session server.
- **Positive controls.** Mode 0 (fault-free) and mode 7 (prefix below the cap) must both return "fixture value 17" with no overflow recorded. This proves the prefix framing is valid, so mode 1 fails because of size alone.
- **Mode 1 (above-cap prefix):**
  - The outer result is `Timeout`, bounded under DEADLINE + 3 s.
  - The first raw-cap event happens before DEADLINE/2, and the compile-time constant check confirms the comment prefix alone exceeds `MAX_WIRE_BYTES`.
  - I confirmed in the pinned source `rmcp-2.2.0/src/transport/streamable_http_client.rs:1015-1021` that a `StreamResult` error is only logged as a warning and never resolves the pending RPC. The outer `Timeout` is therefore a genuine SDK limit.
  - **The original recommendation (outer result before DEADLINE/2) is not achieved.** The principal's choice of option A is a reasonable, bounded disposition.
- **Observer isolation.**
  - The observer is per client: an `Arc` in `PolicyHttpClient` shared with `Connection`, keeping only the first event (`get_or_insert`).
  - It stores only a timestamp and two counts. There is no global registry and no payload.
  - It does not alter polling or the guard result.
  - The `move` closure captures nothing in non-test builds.
- **Unchanged guards.** One tool call and zero redirect-recipient requests are still asserted for every mode.
- **No production change.** Production HTTP policy and error classification are unchanged.

## F3: deferred, confirmed

There are no native changes. Per-frame in-memory `mcp_stamp` scans remain a residual performance limitation with no I/O or correctness impact.

## New SDK setup failure (474/4/1): does not block the source; stays unexplained

**What the preserved log shows** (`full-ai-474-4-1-preserved/`, `ai-full-scripted-opus1.log`):
- The first three HTTP tests passed. All four later ones failed at setup with `Transport`, at lines `:444`, `:513` and `:367`.
- One failed at `:641`, the first plain `post_message` returning `Client(Transport)`: a basic loopback `execute` error, before any SDK protocol.
- These are immediate failures, not `Timeout`. The run took 22.3 s against the 32.1 s passing suite, consistent with the tests skipping their deadline waits.
- **The window was transient.** Later `threads::mcp_tests` in the same process also start a `TestServer` and connect through the SDK, and they passed (log lines 303-314). That argues against a monotonic leak inside the process.

**The changes in this correction cannot have caused it:**
- The observer runs only on SSE overflow, and `:641` failed earlier, at `execute`.
- The F1 test is thread-local, scoped to one Store, and ran after the failures (strict alphabetical order under `--test-threads=1`).

**Limits of the evidence:**
- The diagnostic run printed zero `BRN SDK test setup` lines, so no class was captured.
- The diagnostic writes straight to stderr, bypassing libtest capture. Any recurrence would therefore show its stage, the reqwest error flags and the OS error number.
- The receipts do not record the fd limit or port and socket state. A transient condition on this shared host is plausible but unproven.

**Disposition:** carry it as an open qualification. It is not closed, not a baseline issue and not shown to be flaky. It does not block source approval, because every failure was fail-closed and happened before any production assertion. No retry loop or timing fix is warranted.

## Prior findings and older qualifications

- **Original full workspace run (1224/5/6): stays failed.** The four D4 registration repairs were done with targeted passes; the PDFUnavailable failure is carried over.
- **Widget failure (314/1/1): unexplained, not closed.** The later 315/0/1 run proves nothing. Round 1 found no causal D4 path, and this correction touches no native code. It does not block source approval.
- **Schema 8/9 compatibility** stays qualified: it rests on the earlier WIP run and the `90f` old binary, not a run at this pin.
- **Not qualified:** hosted CI, native/human observation, other platforms, live, remote or authenticated MCP, general JSON schemas, and actual model retrieval. Integration with main and INT-W is not done.
- **Source or config proof only:** SDK cancellation is not remote Undo, and NACK handling has no actual fixture.
- **Mutation testing:** none has run, and I infer no result.

## What I did not read or verify

- I did not recompute the 328 compiled input hashes.
- I did not fully read `opus1-inputs.json`, `opus1-check-index.json`, `opus1-ownership.json`, `opus1-process-release.json` or `opus1-handoff.md`.
- I did not re-read in full the original `review-brief.md`, `acceptance-map.md` and `storage-and-allocation.md`; I relied on the round-1 report and the four principal allocation records.

Spot-checked:
- the 474/4/1 run (log and receipt);
- the 478/0/1 diagnostic run (log and receipt);
- the F1 before-witness log.

The parent retains acceptance and integration.
