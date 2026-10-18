# INT-D / Slice 5 milestone review, round 1 (read-only)

**Verdict: REQUEST_CHANGES.** The two required fixes are small edits to test files the worker already owns, and no production defect was found. One new test will fail on Linux every time, including the required hosted Ubuntu lane. One central I01 promise is claimed but never actually tested.

## What I checked
- **Identity:** material `f1a5624f` is a single commit on `c2eca563`. Tree `1a25f36…` and crates `14039d3…` match. HEAD `069bf814` adds docs-only release evidence and the worktree is clean.
- **Hashes:** I re-derived all 333 `material-hashes.json` entries from git objects and they all match. Of the 328 baseline inputs, only `crates/brn-ai/src/threads.rs:51-56` changed, and that change is just the cfg registration. The 5 new inputs are the 4 Rust test files and `corpus.json`. No manifest, lock, CI, script, production, auth, core, desktop or API change.
- **Command receipts:** `cli-corpus-and-export-final`, `strict-workspace-match-style`, `capability-tests`, `scripted-lint` and `format-final` record all 333 material hashes.
  - `ai-full-scripted-int-d` and `workspace-tests` ran with an earlier CLI `int_d.rs` and without `support/int_d.rs`.
  - `int-d-combined-current-input-type` differs only in the AI test file.
  - Every receipt records `source_unchanged=true`. This matches the disclosure: final CLI-only changes were qualified separately and the earlier workspace run was not relabelled.
- **Preflight:** SHA `0148ad45…` matches the claim, and its 8 workflows are in the reversed control/candidate order.
- **Observer vs CLI:** checked the CLI behaviour the observer depends on: exit codes (`main.rs:24-27,43-44`, `contract.rs:1020-1029`), the `candidates` output shape (`inspect.rs:85-154`), `configure --auth-file`, lease-based orphan recovery (`chat.rs:203`) and the `RunTrace` usage fields.

---

## Axis 1: Standards

### S1 · HIGH (blocker) · the exporter test cannot run on Linux
- **Where:** `crates/brn/tests/int_d.rs:1`, plus `crates/brn/tests/support/int_d.rs:9`, `:154` and `:202`.
- **What happens:**
  - The test binary is compiled for both macOS and Linux (`#![cfg(any(target_os="macos", target_os="linux"))]`), and Cargo picks it up automatically.
  - Its default branch (`exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content`, line 129) creates directories with `tempfile::Builder…tempdir_in("/private/tmp").unwrap()` (lines 154 and 202). `owned_root` also requires the parent to be `/private/tmp`.
  - `/private/tmp` does not exist on Ubuntu, so the test panics.
- **Why it blocks:** the required Ubuntu "Core and CLI" lane runs `cargo test --workspace --locked --offline` (`.github/workflows/ci.yml:54,91`). That lane would fail, which contradicts I02 ("required build lanes do not regress").
- **Why local runs missed it:** every run was on macOS. The docs honestly say hosted runs are pending, but this failure is predictable from the source.
- **Repo precedent:** `crates/brn/tests/agent_retry.rs:233-237` explicitly skips when `/private/tmp` is absent.
- **Remedy (test-only, inside the allocation):** restrict the exporter module and its test to `target_os = "macos"`, or skip with an explicit `eprintln!` when `/private/tmp` is missing. Keep the strict `owned_root` containment for real qualification use. Then re-run the CLI `int_d` test, lint, format and strict workspace lint, and re-pin.
- **Effect on live qualification:** the preflight pins the exporter test executable. If the fix only adds a cfg gate or skip, exporter behaviour on macOS is unchanged. Live results gathered under the `f1a5624f` pin stay valid at that pin, and the fix is recorded as a later test-only change. A live rerun just for this is not needed.

### S2 · LOW · exporter refusal paths claimed but not tested
- **Where:** `support/int_d.rs:31-126` against `live-method.md:23`.
- The docs promise refusal for wrong-version and applied/retired operations. The tests cover only missing operation, missing arm, wrong target and existing output.
- The exporter records `target_current` but does not refuse when the target's current version ≠ the write's `expected_version`.
- **Remedy:** add the two negative cases, or narrow the doc claim.

### S3 · LOW · observer robustness and receipt accuracy
- **Recorded vs executed command:** `live-preflight.json` records `poll_command … --timeout 120`, but `observe-workflow.py` polls with `--timeout 2` and a Python timeout of 3.
- **Unhandled timeout:** a Python `TimeoutExpired` on a poll (`observe-workflow.py:136-152`) is not caught. The `finally` block SIGINTs the live `ask` and the observer crashes with no recorded reason.
- **Remedy:** record the command actually executed, and log the poll timeout as a recorded outcome.

### S4 · INFO · diff hash covers tracked files only
- The check-index `diff_sha256` covers only the tracked diff, so passing and failing pairs share one hash.
- The index discloses this, and the per-command `source_hashes` (verified above) are what actually identify the source. Acceptable.

### Standards positives
- Ownership is respected. Private sibling helpers are reused without visibility changes. `unwrap`/`panic` appear only in tests. A single long integration journey suits I01.
- Exporter containment is solid: absolute path, `/private/tmp` parent, required prefix, canonical real directory, `create_dir` fail-safe and a 1 MiB cap. It checks the Test purpose and Prepared state with no receipt, and requires exactly one Note write. It never calls Apply.
- Corpus generation uses the real checked `Store`/CLI and creates fresh roots only. The evidence documents keep earlier failed attempts and qualifications.

---

## Axis 2: Spec (I01 / I02)

### P1 · MEDIUM (blocker) · selected-skill delivery is never asserted
- **Where:**
  - `crates/brn-ai/src/threads_int_d_tests.rs:372-377` checks only that the selected body is absent from `instructions`.
  - `:570-572` checks `loaded_skill_details` only for `references/check.md`.
  - `crates/brn-threads-app/tests/int_d.rs:302-313` checks only that the body is absent from exported files.
- **Why the reference read doesn't prove it:** `read_skill` (`threads.rs:1114-1150`) requires the package to be enabled, not selected. The combined journey would therefore still pass if preloading of the selected skill broke.
- **Effect:** `plan.md` / `review-brief.md` claim "frozen selected provenance" and exact delivered package identity. For the combined witness, that claim is currently untested. The standalone assertions exist only in `threads_skills_tests.rs:95-120`.
- **Remedy (test-only):**
  - Assert that `INT_D_SELECTED_BODY` appears in the first cold chat input, outside `instructions`.
  - Assert `loaded_skill_details` contains `SKILL.md` with `info.id` and `info.digest`.
  - Assert it is present again on Continue and after reinstall in the app test.
  - Re-run only the affected AI and app tests.

### Promises that are genuinely tested (not just fixture checks)
- **Real durable-prepare Stop:** an `AfterResponse` barrier fires only after a reopened `Store` shows exactly one durable candidate and the protected Note unchanged. Only then does it call `stop_and_post` and cancel. Results: Interrupted, no terminal run, Prepared with no receipt, and the candidate write equals the revised Note.
- **Append-only history:**
  - Within a run, the raw byte prefix of the request input array is preserved across turns.
  - Across runs (off arm), body bytes are byte-identical up to the "History coverage:" marker. That comparison covers instructions, derived context and the kept raw history (`threads_context_budget.rs:212-214`).
  - Oldest-first eviction with non-empty `summary_chunks` proves message 0 was evicted, and exact `read_message` then recovers it at version 1.
- **Summary cache:** the summary is warm "updated" then cold "cached". The cold run makes zero summary requests and leaves the cache bytes unchanged. Summary requests carry no `prompt_cache_key`, and the summary is labelled "UNTRUSTED DERIVED CONTEXT".
- **Fresh Continue authority:** no inherited external input or `owner_request`, a fresh read ledger, and no `mcp_` tools. `apply_changes` on the protected Note returns NeedsReview. The prompt carries the UNTRUSTED CONTINUATION marker, the operation and the Prepared state.
- **Owner review:** exact owner Apply with matching `request_hash` and operation; replay returns the identical receipt; Undo and replayed Undo are idempotent. Messages stay at version 1 and the late text never enters records.
- **Recovery (app test):**
  - Backup, then restore to a new directory with `open_existing_no_upgrade`.
  - Orphan recovery acts once and is empty on the second call.
  - A missing package stays selected but unavailable, and `prepare` is refused fail-closed without touching the candidate or Note.
  - Exact reinstall gives the same id and digest. Continue then gets NeedsReview, followed by `review_apply`/replay and `undo_with_key`/replay. Search and export work.
  - The changed-bytes reinstall negative is not in the combined test; it is covered by existing `skills.rs:75` and `threads_skills_tests.rs:224` (Low).
- **CLI corpus/export containment:** 68 exact Messages read back through the CLI with their versions. Control is "installed only"; candidate is CLI enable + select. No auth file and no Runs.
- **I02 mapping:** reasonable and source-associated. Earlier native, ORT and PDF results are labelled historical, not current.

### Live method (no live evidence exists yet, and I claim none)
- **L1 · MEDIUM (gate before activation): one bad exit ends the whole window.**
  - The observer halts on any exit other than 0/130 (`observe-workflow.py:194`). That includes deadline 7, provider failure 7 and Stale 3.
  - `live-method.md` says only that an auth refusal stops the window, and that partial/timeout outcomes are "preserved".
  - One slow first case would therefore wipe out paired comparability.
  - The principal must choose and document one of:
    - accept "any non-0/130 ends the window", or
    - change the observer to record and continue within the cap, then re-pin.
- **L2 · MEDIUM/LOW: fixture hashes recorded but never re-verified.** The preflight records the initial database, manifest and corpus SHAs, but the observer never re-checks them before `configure`/`ask`. The Continue prompt has no recorded hash. **Remedy:** the principal re-verifies these in the activation receipt (the Continue prompt can be derived as `corpus.continue_prompt + "\n"`), or fix the observer and re-pin.
- **L3 · LOW/MEDIUM: assessment criteria are prose only.** `corpus.semantic_assessment` and `live-method.md` give no pass/fail/unknown rubric (C01–C07, R01–R04, early fact, privacy, injection, receipt identity, Stop/fresh authority). **Remedy:** pin a rubric before results arrive, so assessment can't be fitted afterwards.
- **L4 · LOW: Continue runs even when Stop was missed.** If no Stop was observed (`ask` exits 0), the observer still spends a start on Continue. That run should be labelled a non-Stop measurement.
- **L5 · OBSERVATION: extra compactor call on the MCP-on arm.** In the deterministic test, the MCP-on run makes Continue report "updated" instead of "cached". Report this overhead; don't treat it as free. (Live MCP is off, so it is informational there.)
- **Honest as stated:**
  - Same current binary in both arms, so it compares configuration only: no D1A/B causality, no old-binary support, no superiority claim.
  - The fake-observer checks are not proof of live interception.
  - Usage, account and auth/summary overhead are UNKNOWN unless exposed.
  - The binary was built at `c2eca563`+WIP and is associated with the pin by exact input bytes (qualified).

### Out-of-allocation residual
`plan.md` §10 asks for a record of removed paths and confirmation of a single production route (no retired fixed-history policy). The INT-D docs record reuse decisions but no removed-path audit. I did not audit production for leftovers. This is a residual for the principal/final report, not a blocker here.

---

## Required before approval (bounded)
1. **S1:** gate the exporter test to macOS (or skip explicitly when `/private/tmp` is missing); re-run the focused CLI, lint and format checks and re-pin.
2. **P1:** add positive assertions for the selected `SKILL.md` body and its identity in the AI and app tests; re-run those tests.

Then either fix L1/L2 or have the principal document them explicitly before activation. S2, S3, L3 and L4 are recommended.

## Remaining gates and residuals (all still open)
- **Live:** zero starts so far. Activation, results and principal semantic assessment are pending; any live owner Apply needs separate authorization.
- **Platforms:** hosted Ubuntu/Windows/macOS lanes (S1 would currently fail Ubuntu); native-human/GUI observation.
- **Carried failures:** `PDFUnavailable` (missing `pdftoppm`) in workspace 1234/1/6; D4 SDK 474/4/1 and widget 314/1/1 still unexplained; the original workspace 1224/5/6 run; earlier-WIP 8→9 compatibility.
- **Other D4 qualifications:** SDK prompt inner cap and bounded outer timeout; NACK proven by config only; F3 render-stamp scans deferred.
- **Prior campaigns:** mutation campaign closed, no new mutants; the D3b live run (generic prepare failure, no checklist reference, no long-history claim).
- **Integration:** INT-W not combined; principal source acceptance, owner/program acceptance and main integration are all pending.
