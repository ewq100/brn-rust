# Independent C0 correction review

Provider: GitHub Copilot (`acpRegistry_copilot`), model `claude-opus-5.5`.
Task: `node:delegated-task:command%3Amcp%3A71807a36-041e-4638-b672-3410e3401592%3Adelegate-task%3Abrn-c0-opus-review-20261016-round2`.
Reviewed candidate: `adfab8793c98ff4e3ce9d143dd8894662f190cb8`; baseline `8f768943964a77711b26d5235df146f8efe1dc6c`.
The following is the independent review result, recorded by the principal lead.

**Verdict: APPROVED.** I found no blocking findings in candidate `adfab879`.

**What I checked:**
- **Scope:** nothing outside `docs/**` and `experiments/r0-rig-reuse/**` changed between baseline `8f768943` and `adfab879`. Root manifest, lockfile, CI, toolchain, `crates/`, `agent/`, `scripts/` and both proposal packs are untouched.
- **Light checks:** `git diff --check` passes, and the link check reports 158 files, 776 links, 0 failures.
- **Experiment:** I ran the experiment tests once, locked and offline. They passed 5/5, nothing recompiled (so the committed source is what the receipts tested), and the checkout stayed clean.

**Corrections 1–4:**
1. **Ownership map: fixed.** `plan.md:18-34` has explicit rows for core backup, restore, upgrade, types and stage, plus the CLI routes in `main.rs`, `spec.rs`, `contract.rs` and `help.rs`. `threads_ui.rs` has the principal as its single writer, and a final row gives every other path to the principal. Every listed path exists. The CW1 start message matches.
2. **Hook-aware resume: fixed.** `prefix.rs:326-356` calls the real `Agent::resume` twice from a started `CallTools` checkpoint. The tool count goes 1→2→3, settlement hooks 1→2, memory loads stay 0 and appends go 1→2. Rig's source confirms the stale comment claim: `completion.rs:770` says the resumed run does not append to memory, while `runner.rs:641` (`resumed_memory`) returns the memory handle. No custom driver is proposed, and production keeps small BRN progress records plus fresh invocations.
3. **Local Stop: fixed, honestly recorded as a negative result.** `mcp.rs:156-228` uses a 60-second deadline and waits for the server to start before aborting. It asserts no remote cancellation and a still-pending handler, then the same after disconnect, with the server task still running. A test-controlled release lets the handler finish. The failed first run is preserved, and the D4/M04 need for explicit cancellation and late-result fencing is recorded. The unused direct `tokio-util` dependency was removed with no version changes.
4. **Gate classification: fixed.** `evidence.md` (exit audit), `plan.md:72` and the CL, CW1 and B1a start messages agree:
   - the baseline failures are disclosures, not C0 exit prerequisites;
   - the principal owns their disposition;
   - nothing claims all four required lanes green;
   - B1a waits before moving `threads_writing_tests.rs`. Its line 846 citation is correct: `assert_eq!(view.edit_open, None)`.

**Corrections 5–10:**
- **5:** I summed the archived hosted log myself and got 1013/0/6 (88 targets), 64 for the helper and 346/0/1 for capability fixtures, matching the inventory.
- **6:** the two retirement scripts and `brn --help` all exit 0 in the receipts.
- **7:** the assertions are now real: the exact retained slice (`memory.rs:45`), the exact 24-key set, a structured timeout error kind, and a raw wire-byte prefix check across runs (`prefix.rs:98-126`) with tools compared too.
- **8:** the test writes nothing by default. The optional capture must be a new absolute path outside the repository.
- **9:** the acquisition log now matches the qualified "seed provenance unproven" wording.
- **10:** the status title comes first, the stale "pending" text is gone, the lock-delta keys are correct, and the empty CL list names the paths it examined.

**Non-blocking limits:**
- The MCP observations cover a 200 ms window on an in-process transport only.
- The byte-prefix check covers the request up to the messages; tools are compared as parsed JSON.
- The verification receipts were recorded before the commit; my rerun covers that gap.
- Running `complete-test-inventory.py` regenerates its two output files in the repository.

**Readiness:**
- **C0 exit:** ready, subject to the principal's exit decision.
- **Baseline green:** no. The hosted native UI widget failure is unresolved, `pdftoppm` is missing locally, and one local diagnostic repeat showed a separate intermittent failure (Working vs Interrupted).
- **Later slices:**
  - CL can be recorded as a no-op after exit, without a four-green claim.
  - CW1 can start after exit, carrying the failures forward.
  - B1a must wait for the widget-failure disposition before moving `threads_writing_tests.rs`.
  - D3 and D4 stay blocked on the D-S and D-M decisions.
  - Live qualification is blocked by L, and Poppler is retained under X.
  - Shared-note comment visibility is still an open product question.

This review does not authorize publishing, merging to main or native acceptance.
