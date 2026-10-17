**Verdict: REQUEST_CHANGES.** The runner source is acceptable as a reviewed building block, with qualifications. The prepared preflight at pin `9d3741e6` is **not activation-ready**.

I found no gate leak, corrupted identity, Apply path, or credential or reasoning capture. The only blocking defect is a hash mismatch that would make the gate refuse every start before reservation or Auth.

This was a read-only review: no edits, builds, tests, auth, provider or network access, GUI, git mutations or delegation. I never invoked the ignored runner or any fixture or qualification command, and I never opened the active fixture DBs. All test results below come from recorded receipts. I did not use the code-review skill because it runs subagents.

## Identity
- **Pins match.** HEAD is `9d3741e6`, so there are no post-pin commits, and the worktree is clean. Tree `4044b78e…`, crates `d6a6454b…`, and the BASE..PIN diff sha256 `94a097de…cb0ef9` all match.
- **Only one source file changed:** `crates/brn-ai/src/threads_int_d_tests.rs` grew from 1528 to 2982 lines, and the old file is an exact byte prefix of the new one. `threads.rs`, the two cfg hooks, production code, Cargo and CI are unchanged. Everything else is docs under `capture/runner/**`.
- **Preflight hashes match the pin.** All 334 `source_inputs` sha256 values equal the pin's git blobs. That includes the observer, the runner, the corpus and the immutable capture `material-hashes.json` (`9bc06594…`). The `.d` hash matches, and all 72 runner dep-info paths fall inside the 334 keys. The rubric copy is identical to the committed one, and the initial manifest is preserved unchanged.

## Blocking defect
**B1 (HIGH for readiness; fail-closed, not a safety leak): the settings hash is computed two different ways.**
- `runner/prepare-preflight.py:159` hashes the settings as JSON with keys sorted alphabetically (`sort_keys=True`). That gives `eeb16838…`, the value stored in `preflight.json`.
- `threads_int_d_tests.rs:1708` hashes `serde_json::to_vec(&to_value(settings))`. The build enables serde_json's `preserve_order` feature (pulled in by `rig-core` 0.44; `indexmap` and `preserve_order` appear in the target fingerprints). So Rust keeps `WorkspaceSettings` field order (`provider, model, effort, auth_file, credentials_dir, …`). Recomputing it in that order gives `34e43ec3…`.
- **Counterexample:** a valid activation against the current preflight reaches `facts != p.context` at `:1909` and fails with "canonical fixture or auth reference mismatch". This happens before `reserve_then`, so no start is consumed, but the gate can never pass.
- **Why the offline tests missed it:** the gate test builds `p.context` with Rust `fixture_facts` itself, so Python and Rust were never compared on the CLI-generated fixture.
- **Bounded remedy:** prepare a corrected preflight in a new fresh `brn-int-d-capture-preflight-20261017-*` directory for the same unused fixture and absent output, and keep the old preflight as superseded. Either:
  - compute `policy_sha256` in Rust field order, or
  - make the Rust digest canonical (sorted keys), which needs a new pin and re-review.

  Before claiming readiness, also check the whole `context` object from Python against Rust on the real fixture without opening it for mutation, for example from a byte copy. The other fields only have to match at the value level, and both sides sort `inputs` by ID (`:1700`), so I found no other mismatch.

## Verified (standards and spec)
- **Gate before Auth.**
  - The activation and preflight files must sit together in an owned 0700 preflight directory.
  - Activation parsing is strict JSON with unknown fields rejected. It checks format, task, `principal_activated`, the preflight sha, that the time window has not expired, and a maximum of one start.
  - The preflight check covers route and model, `--auth-file` reference, budget, owned prefixes, a fresh `git` check of each source hash against the pin, `current_exe` plus its hash, `.d`, corpus, prompt, rubric and manifest.
  - The main DB, WAL and SHM are stamped (absent or hashed) before `Store::open_existing_without_upgrade`. Fixture facts are read from the checked store: Test purpose, no Runs or candidates, 68 messages including the early and later texts, and the protected target.
  - All of this runs before `reserve_then`.
- **Reservation.**
  - `reserve_then` creates the output directory, then `one-start-reservation.json` with `create_new` and no-follow, and syncs the file and the directory before calling execute.
  - The manual test reserves before `prepare_run`.
  - Auth happens only in `run_prepared_over` → `provider_client_context`.
  - The settings equality check at `threads.rs:3135-3148` is unchanged.
  - `prepared_matches` compares the prepared snapshot with the preflight before `run_prepared`.
- **Effects and Stop.**
  - Only read tools and `retain_progress` are allowed.
  - `apply`, `withdraw` and `refresh` are stopped when they are dispatched, before their handlers run. Each also checks `live()` before any mutation.
  - The first prepare outcome (success or error) is written before the checked Stop.
  - The tool catalogue was checked against the actual runtime registrations, with no fallback for unknown activity strings.
- **Observer.**
  - It keeps only capture frames up to 2048 bytes, plus at most 2 context rows up to 4096 bytes each.
  - All other Activity, Text and reasoning is dropped.
  - Missing correlation, overflow or a filesystem failure triggers Stop and cancel.
  - `result.json` holds only static labels, counts and usage.
- **Offline tests and receipts.**
  - The tests use a real synthetic Git repo, a checked Test store, a fake Auth closure (0 calls before the gate, 1 after reservation, duplicate reservations refused) and a mock Rig model.
  - Final focused run: 6 passed, 0 failed, 1 ignored, 479 filtered. Strict clippy and fmt both exited 0. CLI int_d: 4/0.
  - No recorded command used `--ignored`, and the failed attempts are retained.
  - There is no `activation.json` and no output directory.
- **Reference-only configure.** It ran once. Settings went from v3 to v4 with the literal reference, all other records were unchanged, and the initial state is preserved. Post-configure main DB is `e26d55cd…`, with WAL and SHM absent.

## Optional residuals (not blocking)
1. **Lost evidence on panic (LOW-MEDIUM).** At `:2312-2330`, the lease assert and several `unwrap`/`panic` calls run before `result.json` is written, so a panic there loses the bounded result of a paid start. Fix: write `result.json` first, including lease status and unknowns.
2. **Post-Stop overflow scoring (MEDIUM-LOW).** If a second capture frame arrives after Stop in the same batch, `:2042` reports "capture event overflow". Under the literal rubric that could turn a valid result into FAIL. Fix: before activation, have the rubric ignore or count frames that arrive after Stop.
3. **Re-read after hashing (LOW).** `prompt.txt` is hashed at `:1878` and re-read at `:1915`, and the manifest likewise at `:1880` and `:1885`. Fix: read once, then hash and use the same bytes. The 0700 directory mitigates the risk.
4. **Dropped error class (LOW).** The `Ok(_)`/Failed branches (`:2247`) drop the static `AiErrorKind`, which would be safe and useful to record.
5. **INFO.**
   - The activation has no upper limit on its expiry time.
   - `git` runs with the inherited environment (e.g. `GIT_DIR`).
   - The current-thread tokio runtime makes the 120 s timeout and 5 s drain cooperative.
   - Purpose and the no-Runs check are not re-checked between the gate and `prepare_run`.
   - The binding of the executable to the other 332 sources rests on the receipt chain, not cryptographic proof.
   - The compactor may call the provider inside the 120 s window.
   - The `prepare_changes` status Activity could serve as a backstop Stop signal.

## Retained qualifications
- The original 3 errors remain UNKNOWN.
- The 2-start window is closed with no candidates, and the unused 10 carry no authority.
- PDF, SDK/widget, platform/hosted/native-human, INT-W, main/owner/program and live I01 remain incomplete.
- The capture-review optional items stay deferred.
- A future run could still yield a non-unique tool_rejected or unknown cause.
- The ignored runner has never been executed. This review issues no activation and authorizes no paid start.
