# Independent D4 technical review, round 1: candidate `a5d148a3` against base `9a88a0a9`

**Verdict: APPROVE WITH QUALIFICATIONS.** I found no blocking or medium-severity defect in the source. Three low-severity items are below. The widget Stop failure is still unexplained and must be carried into acceptance.

This review was read-only. I ran no builds, tests, mutation testing, live MCP, GUI, delegation or edits.

**Scope**
- I re-derived the full diff hash and it matches the parent's: `4cc6409a…93fa92` (75 files, +14394/−88). The scratch copy `base-to-material.diff` has the same hash.
- The worktree's later commit `98854f6d` adds only evidence documents, so `crates/` is identical to the candidate. I read the code there.

**Not read in full:**
- `lifecycle-evidence.md`
- `native-review-snapshot-evidence.md`
- `dependency-graph.json`
- most per-receipt scratch logs

## Findings

**F1 — Low (error handling).** `crates/brn-ai/src/threads.rs:219`, in `attach_external_input`, has `let _ = finish(&self.scope, &answer)?;`.
- **Trigger:** the store returns an error inside `finish` (lock poisoning or a SQLite failure) while an attach refusal is already being handled.
- **Impact:**
  - The storage error replaces the original refusal, either `mcp_unavailable` or a budget preflight error. The CLI then reports a storage failure instead of Unavailable.
  - The Run can stay Working until the existing orphan sweep fences it. No authority leaks, because the grant is dropped with the scope.
- **Fix:** keep the original error. Return `Err(error)` even when `finish` fails, and record the `finish` failure separately.

**F2 — Low (test strength).** `crates/brn-ai/src/threads_mcp_http_tests.rs:429-468`, `sdk_http_streamed_comments_redirect_encoding_and_invalid_json_fail_closed`.
- **Trigger:** each of the four fault modes (streamed comments, redirect, encoding, invalid JSON) asserts only `invoke(...).is_err()`, plus one call and no redirect egress.
- **Impact:** any unrelated error would also pass, such as a deadline expiry or an SDK decoding change. So the test does not prove that each mode is refused by its intended check. By contrast, the 202/204 acknowledgement test (`:579-633`) has a positive control.
- **Fix:** for each mode, assert the error class or message, and that the call finished well inside `DEADLINE`.

**F3 — Low (performance).** `crates/brn-desktop/src/threads_mcp.rs:66`.
- **Trigger:** the render paths at `:410` and `:476` call `mcp_stamp()`. Each call scans the in-memory records and the receipt history through `latest_run`, on every frame while the MCP settings or detail panel is open.
- **Impact:** CPU cost that grows with history size. There is no I/O and no effect on correctness.
- **Fix:** compute the stamp once in `refresh_mcp` and compare against that cached value in render.

**Not defects (checked and accepted):**
- `store.records()` full scans under the store lock in `Invocation::prepare`/`snapshot`. Existing `threads.rs` code uses the same pattern at lines 866, 1218, 1741 and 3509.
- A lazy `read_skill` advances the fence and ends the external call. This is documented and approved: `threads-behavior.md:179`, acceptance-map line 14, and tested.
- Native approval replaces the whole policy with one tool and one access row. This is the bounded UI.
- `Chat(id)` access still applies after the chat moves into a Project. The input is still owner-reviewed, and the native snapshot barrier refuses a move made between review and prepare.

## Standards, specification and acceptance mapping

**Egress and endpoint**
- `canonical_mcp_endpoint` (`external_data.rs:15-78`) accepts only `http://` to 127.0.0.1 or `[::1]`, with a nonzero port and a restricted path. It must round-trip to the canonical form.
- The HTTP client (`threads_mcp_http.rs`):
  - disables proxies, retries and compression;
  - follows no redirects;
  - requires the request URI to equal the endpoint exactly;
  - refuses auth headers.
- `response_policy` (Location, Content-Location, non-identity encoding) runs before the 202/204 Accepted return, and on GET and DELETE.
- The dispatch gate runs synchronously immediately before `execute`.
- JSON bodies and Server-Sent Events (SSE) streams are capped cumulatively before parsing. Error bodies are never read.
- Tests cover redirects, encodings, alternate locations, oversize bodies and SDK NeverRetry.
- ✅ Matches the approved design.

**Tool schemas**
- Only typed `Empty{}` or `Query{query}` inputs are accepted, with `deny_unknown_fields`, an 8 KiB input limit and a 2048-byte query limit.
- The whole schema must match the contract digest. There is no general JSON-schema interpreter.
- Self-reported `readOnlyHint` is required but does not approve a tool by itself (`threads_mcp_tests.rs:48`, `:127`).
- ✅ Matches.

**Grant**
- Fields are private, with no Debug or Serialize. The grant is issued only under Owner `HostAuthority`.
- It binds:
  - the instruction (id, version, content);
  - the chat, `context_epoch` and privacy flag;
  - the Project and Settings versions;
  - the policy, endpoint, tool, schema and canonical input hash;
  - the Working Run and its fence.
- It is checked again at the channel and at dispatch.
- The Store epoch is a fresh `Arc<()>` held as `Weak` and compared with `ptr_eq`. `mcp_policy.rs:410` proves that a still-valid grant is refused by:
  - a second `Store` on the same directory;
  - a restored copy;
  - a reopened Store.
- ✅ Matches.

**One call, no retry, batch rejection**
- A `spent` compare-and-swap sits in the dispatch gate, behind a one-permit semaphore.
- Mixed, unknown, wrong-input or duplicate batches are rejected before any local or network effect (runtime tests at `:153` and `:304`).
- Helpers get `external: None`.
- ✅ Matches.

**Cancellation, Stop and leases**
- The SDK's cancellation request is observed on the real SDK server for Stop, deadline, dropped caller and disconnect (`http_tests:353`).
- Drain is bounded to 7 s before disconnect.
- `InvocationGate` holds a `Weak` reference, and dropping it breaks the ownership cycle (`runtime_tests:63`).
- In-flight remote work may finish, but late acceptance and writes are fenced, and no remote Undo is claimed (`:348`).
- The lease is still released with the `Scope`.
- ✅ Matches.

**Schema 9**
- Old schema-8 binaries cannot ignore the new policy; the new binary refuses an un-upgraded schema-8 store.
- Backup, restore and upgrade paths are preserved, and there is no new SQL table.
- `mcp` is kept on both the Configure and replay paths.
- ✅ Matches. Evidence is qualified (item 3 below).

**Guide identity and summary key**
- `guide_identity` is `{skills}:mcp-{policy}:{model_name}` and contains no transient data.
- The summary key includes `run.guide_identity` (`threads_summary.rs:279`).
- Full trace, canonical records and backup bytes contain no input or result canary (`runtime_tests:63`).
- ✅ Matches.

**Rig reuse**
- Uses `McpTool::from_mcp_server` through the rmcp client and `sse_stream::SseStream`. There is no forked protocol or session parser.
- The `Cargo.lock` delta adds exactly 12 packages, with no version changes or removals: axum, axum-core, darling ×3, httpdate, matchit, rig-rmcp, rmcp, rmcp-macros, sse-stream and tokio-stream.
- The direct dependency is rmcp `=2.2.0` without default features (`default-features = false`). The axum and SDK server pieces are development dependencies only.
- ✅ Matches.

**Shared surfaces**
- App `configure_mcp` and `disable_mcp` use Owner authority.
- The CLI checks `--mcp-tool`/`--mcp-input` pairing, name and size before any workspace I/O, and attaches after the single `prepare_thread_run`. Argument persistence is unconfirmed, but no CLI journal or persist mechanism exists for it.
- The native path takes the review before prepare and attaches with the saved snapshot after prepare. The snapshot barrier refuses a policy switch or Project move (`desktop threads_mcp_tests.rs:145-255`). In that case:
  - the guide is not changed;
  - the fence advances only by the normal terminal +1;
  - the Run ends Failed;
  - the instruction is kept;
  - no worker starts.
- Only the exact tuple Other + `mcp_client` + `mcp_unavailable` qualifies, and spoof negatives are tested.
- ✅ Matches.

**Queue**
- `saved_review_refusal_ends_only_its_preparation_and_preserves_queued_instruction` covers refusal both before and after prepare.
- Native queue dispatch with a pending review fails closed through the existing pause path.
- ✅

## Disposition of the six known items

1. **Full workspace run, 1224/5/6, exit 101: stays failed.** The four D4 omissions in the CLI exhaustive tables and README are corrected in source, and their targeted checks (29/0, 23/0) passed. The pdftoppm qualification is carried over, not a D4 regression. It must not be relabelled as workspace green.

2. **Widget 314/1/1 at `the_owners_stop_of_a_killed_reply_adds_its_review_but_no_notice`: unexplained, not closed.** Source inspection:
   - D4's only change to `threads_intake_tests.rs:922-962` binds the record once and adds Debug output to the unchanged `Interrupted && fence == 2` predicate.
   - For runs without an invocation, `mcp_runtime::connect` and cleanup do nothing.
   - The Stop, lease and `stop_chat_run` paths are unchanged.
   - `refresh_mcp` only reads.
   - The test opens no MCP UI.

   I found no causal D4 path. The later 315/0/1 and isolated 1/0 results do not explain the original failure. **It does not block D4 source approval.** Acceptance must still record it as an unexplained native-suite failure, not as flaky or a baseline issue.

3. **Schema 8/9 compatibility.** Accepted as qualified evidence:
   - the schema-8 fixture was produced from `9a88` production code;
   - the old CLI binary is from `90f50da`;
   - the new CLI binary is the WIP build `54c2f4b7`;
   - all 11 tables matched exactly;
   - adopting the restore owner purpose is the documented exception.

   This is not a fresh run at the final pin.

4. **Earlier failures before fixes.** These were preserved, and the before-failures are genuine behavioural witnesses, not compiler errors. The corrected tests are meaningful:
   - the Store epoch test asserts a still-valid grant on the issuing Store as a positive control;
   - the `Weak` upgrade is `None` after drop;
   - the acknowledgement-header test has an Accepted positive control.

   The queue fixture correction keeps the existing `InterruptedToolRejected` precedence and changes no production code.

5. **Unqualified areas remain unqualified:** native/human, hosted CI, live D4, remote or authenticated MCP, general JSON schemas, and actual model retrieval. SDK cancellation does not prove that the server undid its work.

6. **This is the first review round.** There are no prior findings.

## Requirements still unverified

Not observed by me:
- native/human observation;
- hosted CI;
- live or external MCP servers;
- an actual low-level NACK fixture (proof is from source and config only);
- other platforms;
- a mutation campaign;
- a contemporaneous old-schema-8 build at `9a88`;
- confirmation that the full original workspace suite passes;
- main/INT-W integration.

I spot-checked only the summary line of the full scripted AI log (477/0/1). I did not repeat any test counts.

## Verdict

**APPROVE WITH QUALIFICATIONS.** I recommend fixing F1 and F2 before or at acceptance. F3 is optional. The parent retains acceptance and integration, with the widget failure and items 1, 3 and 5 explicitly carried as open qualifications.
