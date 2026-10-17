# INT-D history-only harness/preflight review, round 1 (Copilot Opus 5.5 high)

**Verdict: REQUEST_CHANGES.** The harness, gating, reservation and preflight are largely sound. But every real candidate the model prepares will make the exporter fail, so the first successful case stops the whole window. Fixing that requires a new pin and a new preflight.

This was a read-only review. I made no edits, builds, tests, lint or script runs. I invoked no observer, preparer, generator, exporter or runner. I made no Git mutations, network, Auth or provider calls, and opened no Store or SQLite database. I did not delegate. I did not re-run any recorded check; where I say "recorded", the evidence is the receipt, not my own execution.

## 1. Identity and scope I checked directly

- **Worktree:** `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407`, branch `work/int-d-20261017`, HEAD `f6d063e8…`, clean.
- **Material pin `db757e32`:** tree `4903768…` and crates `c0f6d86…` both match. The SHA of `git diff b28f4c0c..db757e32` is `9d8bfddb…`, as stated. `db757e32..HEAD` changes docs and evidence only.
- **Full diff read, not just the summary.** It touches 34 files:
  - Two Rust test files, both append-only: `crates/brn/tests/int_d.rs` and `tests/support/int_d.rs`.
  - Three scripts, each with appended history functions and a `__main__` dispatch: `prepare-qualification.py`, `observe-workflow.py`, `check-observer.py`.
  - `int-d/plan.md` (+2 lines) and the new `history-phase/**` docs.
  - Legacy entry and function prefixes are unchanged.
  - If the legacy `main()` is given a history preflight, it fails closed with a KeyError on `plan["mcp"]`.
  - The history prompt is byte-identical to the legacy prompt semantics.
  - I found no hidden activation path in `--help`, imports or module-level code.
- **334 inputs:** every one matches both its pin Git blob and the working bytes. Only the two test files differ from base. Production, guides, CLI/API, auth/core/app/native, deps, manifest, lock, corpus, the parent capture hook and the manual runner are unchanged. The `.d` dependency files are covered by the input set.
- **Preflight** `/private/tmp/brn-int-d-history-preflight-20261017-db757e32/preflight.json`:
  - SHA `67e54848…`; directory mode 0700.
  - `activation.json`, `observations/` and any output or reservation are all absent. No `brn-int-d-candidate-export-20261017-history-*` path exists.
  - Scripts, binary `1f66cf32…`, test binary `2dd13989…`, `.d` files, method, rubric, corpus, prompts, manifests, contexts and saved records all match their recorded hashes.
- **Live DB triples:** each store is a main file only (no WAL, no SHM), and all four equal `final-identities.json` and the preflight.

  | Case | Arm | Main DB SHA | Bytes | Settings | Selected |
  |---|---|---|---|---|---|
  | 1 | r1 control | `b595b187…` | 380928 | v4 | No |
  | 2 | r1 candidate | `61afc72e…` | 389120 | v6 | Yes |
  | 3 | r2 candidate | `be4f6684…` | 389120 | v6 | Yes |
  | 4 | r2 control | `92a52815…` | 380928 | v4 | No |

  All four contexts have 68 history inputs.
- **`preparation/actions.json`:** 39 commands, all exit 0. The sequence is:
  1. Two pinned generators.
  2. For each store: initial `records`, `candidates` and `runs`.
  3. One reference configure per store with the exact argv `--timeout 120 configure --provider Codex --model gpt-6.1-sol --effort medium --auth-file /Users/evokessler/.codex/auth.json`. Each took about 0.02 s with empty stderr.
  4. Final `records`, `candidates`, `runs` and `skills`.
  5. The closed-copy interop test (`int_d-8d0cf15f…`).
  6. The history preparer.

  `Workspace::configure` (`crates/brn-threads-app/src/settings.rs:88-122`) is metadata-only: it checks the path is absolute and does not read credentials.

## 2. Blocker

### B1 (HIGH): the exporter refuses every real AI-prepared candidate

- **Where:** `crates/brn/tests/support/int_d.rs:98-108`, together with `crates/brn-ai/src/threads.rs:1582-1602`, which is called at `:1839`.
- **Cause:** on the AI prepare path, `bind_project_inputs` always adds two inputs to the request:
  - the chat Thread as `ChatContext`, at version `scope.chat_context_revision`;
  - for a Project chat, the Project record as `Current`.

  The exporter only accepts inputs equal to `case["note"]` or listed in `case["history_messages"]`. Those are Message IDs only; the manifest keeps `chat` and `project` as separate keys.
- **Counterexample:** case 1 (r1 control) completes and the model prepares the exact Note v2 edit. The observer's checks pass (Prepared, no receipt) and it runs the exporter. The exporter meets the chat input and returns `Err("input is outside the explicit corpus")`. The test unwraps that error, panics and exits 101. `local("candidate-export")` raises `ValueError`, and `observe-workflow.py:444-448` sets `window_failed`. The whole window stops and cases 2–4 are never started.

  So the rubric's PASS path is turned into a technical STOP. Only "Completed with no candidate" (an objective gap) can finish. The window could therefore never yield a PASS case.
- **Why existing checks missed it:**
  - The exporter's positive tests (`support/int_d.rs:197`, `:329`) prepare candidates through the owner path, with no chat or Project inputs.
  - `check-observer.py:23,177-201` uses a fake exporter.
  - The earlier live window never invoked the exporter.
  - `history-phase/reuse.md:3` says the existing exporter's guards are adequate. That claim is not established for runtime-prepared candidates.
- **Bounded remedy** (requires a principal hunk allocation for the test-support exporter only; no production change):
  1. In the exporter, additionally allow exactly two inputs:
     - `case["chat"]` with `InputExpectation::ChatContext`, at a version that exists for that record;
     - `case["project"]` with `InputExpectation::Current`, equal to the current Project version.

     Still refuse any other record. Optionally also require that every history input is `Historical` at version 1.
  2. Add one deterministic positive test. It should export a candidate prepared by the real AI runtime (scripted SDK) from a Project chat, and assert that the chat and Project inputs are present and accepted.
  3. Re-pin, then regenerate, reconfigure and redo the interop and preparation.
     - Pair names embed `pin[:8]`, and both the artifact hashes and the 334-input set change.
     - Generate new stores and a new preflight; do not reuse the current four stores.
     - The new preflight then needs a short focused re-review.

## 3. Medium

### M1: some failure classes don't trigger the persisted window STOP

- **Where:** `observe-workflow.py:444`.
- **Problem:** after spawn, the code catches only `OSError`, `ValueError`, `KeyError`, `TypeError` and `TimeoutError`. These are not caught:
  - `AttributeError`, for example:
    - at `:409`, `envelope.get("result", {}).get(...)` when `result` is a list or string;
    - at `:419`, when `snapshots["note"]["result"]` is not a dict;
    - at `:425`, when an item in `c["writes"]` is not a dict;
  - `RuntimeError` from `group_reap`;
  - `KeyboardInterrupt`.
- **Effect:** in those cases the `finally` block still reaps and saves the ledger. However, `window_failed` stays false and no `{ordinal}-harness-failure.json` is written.

  No duplicate paid start can follow: the case is missing from `finished_cases`, so `:335` refuses later ordinals. But the STOP is not recorded with a typed cause, as the method requires. If `group_reap` raises inside `finally`, the final ledger save is skipped; the start row saved before spawn remains.
- **Remedy:** catch `Exception` with the same static class name (`BaseException` for interrupts), and protect the `finally` ledger save. This can go into the same re-pin as B1. Until then, the operating rule is that any observer traceback counts as a window STOP.

## 4. Optional residuals (LOW/INFO, not blockers)

1. **Authority re-check is narrow.** Between cases the observer checks only the target Note's ID and that its version is 2, plus that the candidate is Prepared with no receipt. It does not re-check:
   - Settings version or policy hash;
   - other receipts or operations;
   - grants and connectors;
   - unchanged byte content beyond the version number.

   Any Note Apply would still be caught by the version check. The rest is left to the principal's rubric review and should be labelled that way.
2. **More than one target candidate stops the window** (`:422-423`). That is model behaviour being classed as a technical stop, and the method and rubric do not declare it in advance. Add one sentence to the method.
3. **Durability.**
   - The directory created by `out.mkdir` (`observations/`) is not fsynced in its parent preflight directory.
   - macOS `fsync` is not `F_FULLFSYNC`.
   - The risk is power loss only; the in-process ordering is correct.
4. **Git reads.** The script and method checks call `git show` with the inherited `GIT_*` environment and honour replace refs. Only `verify_source` strips `GIT_*`. The working-byte hashes and receipt hashes are the real anchor, so the practical risk is low.
5. **`.pyc` writes.** Loading the preparer via `importlib` may write a gitignored `__pycache__/*.pyc` into the worktree during activation. Consider setting `PYTHONDONTWRITEBYTECODE`.
6. **Timeout race.** The Python wait of 120 s equals the CLI's `--timeout 120`, so the CLI's own deadline can race the cleanup `SIGINT`. The docs correctly say cleanup is never treated as a Stop. A margin (Python wait > CLI timeout) would give a cleaner terminal classification.
7. **PID reuse.** After a normal exit, `group_reap` still calls `killpg` on the reaped group ID, so there is a theoretical PGID-reuse risk.
8. **Inherited environment.** The child inherits the parent environment except `BRN_*` and `AGENT_CONTRACT_*` variables, so `RUST_LOG`, proxy and `SSL_*` settings carry through. `HOME` and `CODEX_HOME` are synthetic. Auth HTTP, the account, homes and caches remain UNKNOWN; the docs say so honestly.
9. **Installed skill bytes are not stamped.** The `skills/` directory in each pair sits outside the DB triple. The runtime does verify artifact digests when it reads a package (`crates/brn-ai/src/skill_packages.rs:303-308`), and the selected digest is bound in policy. A per-case tree hash would close the gap.
10. **Fake-matrix gaps.** These are not covered:
    - expiry over 900 s, `issued_at` in the future, ordinal 0 or 5;
    - an activation in a different directory, a symlinked preflight;
    - malformed or mismatched export JSON, an operation that isn't Prepared, ambiguous candidates;
    - a crash between reservation and ledger save, and non-dict `result` values (M1).

    The `len(starts) >= 4` check can never be reached. The cap is actually enforced by the ordinal range 1..4 plus the create-new reservation, which is adequate.
11. **Metadata helper.**
    - Its 128-entry cap and a symlink inside a copy tree are not tested.
    - The link between each closed copy and its original comes from external preparation evidence, not from the helper itself.
    - The `/usr/bin/shasum` stdin receives only the serialized Settings. That includes the auth reference path but no credential bytes, so nothing private leaks and Auth is never opened.
12. **Preservation bookkeeping.** For the 6 collision rows, the initial supplementary scratch copies were overwritten. The provenance is adequate because:
    - all 26 original qualified copies are unchanged;
    - 26 new unique SHA-named copies were verified;
    - the corrected association is recorded without rewriting history.

    This is no evidence that the initial supplementary copies were correct, and none should be claimed.

## 5. Positive spec findings

- **Gates run before any reservation, ask or Auth.** In order:
  - Activation has the exact key set, format, task constant, `ACTIVATED` state, source pin, preflight SHA and `max_starts` of 4.
  - `issued ≤ now < expires ≤ issued + 900`, and the activation sits in the same 0700 directory.
  - The route and limits are exact.
  - All 334 sources plus the three scripts match their working bytes and pin.
  - The artifacts, `.d` files, method, rubric, corpus, fixed order and exact ask argv match.
  - No stop, continue or configure command is present.
  - Every row's prompt, manifest, context and records hashes match, and `history_context` is recomputed.
  - Only the current case's live main/WAL/SHM triple is checked, so cases already mutated can legitimately differ.
  - For the output namespace and ledger, `finished_cases` must equal `1..ordinal-1`, there must be `ordinal-1` starts, and a set `window_failed` is refused.
- **Reservation:** there is exactly one create-new reservation with file and directory fsync, then the ledger save, then spawn with `start_new_session`. Process-group timeout and reaping are present. Local inspections use a 2 s CLI timeout with a 3 s Python limit; the export limit is 15 s. Failures stop the window and are persisted (subject to M1). Nothing retries or continues automatically.
- **Route:** the configured route is the normal CLI, Codex, `gpt-6.1-sol`, medium effort, with an auth-file reference only. Nothing reads credentials, lists models or discovers accounts, and Auth is not configured at activation.
- **Arms:** the control (installed but not selected, Settings v4) and the candidate (Project-selected, v6) run on the same runtime. The method makes no D1A/B causality claim, uses no old binary and claims no general superiority.
- **Export ordering is correct:** the exporter runs after the ask has been reaped and only on a Prepared operation with no receipt, a single target and the current version. It never Applies anything. Only its input guard (B1) is wrong.
- **Settings digests:** Python's `ordered_settings_digest` (`json.dumps` with compact separators, no key sorting) matches Rust's `to_vec(to_value)` field order.

## 6. Building-block statement

The new source, method and preflight are internally consistent. The trust chain (pin, then the 334 inputs, then artifacts and `.d`, then contexts, then live triples) is verifiable from the hashes. That makes it a sound basis for a separate future activation once B1 is fixed and the preflight is re-prepared and re-reviewed (fixing M1 in the same pin is advisable).

**The current preflight `67e54848…` should not be activated.** Any rebuild that changes artifacts invalidates it, as expected.

This review grants no activation, real-Auth success, semantic or source success, owner, program or main acceptance, and no paid start. All historical limits stay as recorded, including:

- the SDK 474/4/1 and native widget 314/1/1 results remain unexplained;
- PDF is still unavailable (pdftoppm missing);
- the three original causes remain UNKNOWN;
- strict Copilot record sanitization is deferred;
- there is no foreign canary, so absence of private egress is unproven;
- auth HTTP and account details remain UNKNOWN;
- native human observation, INT-W combination, I01 semantics and owner live Apply are unfinished.
