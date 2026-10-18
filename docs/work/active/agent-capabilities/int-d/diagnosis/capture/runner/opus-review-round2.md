**Verdict: APPROVE_WITH_QUALIFICATIONS.** B1 is genuinely fixed. The new preflight `54c748a5…` is consistent with itself, with the pinned source and test binary, and with the unused fixture, so it can bind a separate future explicit activation. There are no blocking findings.

This approves only the reviewed runner source and the corrected preflight. It does not authorize activation and makes no claim of authenticated success: the ignored runner and normal Auth have never been executed.

This was a read-only review. I made no edits, builds, tests, Auth, provider or network calls, git mutations, delegation or GUI use. I ran no script, and I never opened a Store or SQLite DB; I only hashed files and parsed JSON. I did not load the code-review skill because it starts subagents. Test results below come from the recorded receipts, not from runs of mine. My only writes were two scratch copies of the test file in `/tmp`, now deleted.

## Identity
- **Pins match.**
  - Branch `work/int-d-20261017`, HEAD `8ab807a4`, worktree clean.
  - Material `a65a6a61` has tree `2f381e5e…` and crates `ece76539…`.
  - The b60..a65 diff hashes to `560df05a…`, and 9d..a65 to `1d608202…`.
- **Commits after the pin:** the two commits after `9d3741e6` change only docs (`b60cd8c1` and `8ab807a4`). `a65..HEAD` touches none of the 334 compiled inputs.
- **Only one source file changed since `9d3741e6`:** `threads_int_d_tests.rs`. The 9d file minus its final `}\n` is an exact byte prefix of the a65 file. The original `f0c2ae65` file is also an exact prefix, so the original module, the parent cfg hooks and production code are unchanged.
- **`fixture_facts` is byte-identical** to its 9d version (sha `86cbe6da…`). The gate, reservation, observer and effects code are unchanged.

## Is B1 resolved? Yes.
- **I recomputed the digest from the old post-configuration overlay** (keys in the order provider, model, effort, auth_file, credentials_dir, maintenance, review_first, delegation_chosen):
  - in that order it gives `34e43ec3…`, which is correct;
  - with sorted keys it gives `eeb16838…`, the old wrong value.
- **The Python fix is minimal.** `prepare-preflight.py` now has a `settings_digest` that hashes compact JSON in the order the Settings were recorded, with no sorting. There is no other Python change.
- **The new test is safe.**
  - The default branch checks the real field order and that serde_json keeps insertion order (`{"z":1,"a":2}` round-trips unchanged).
  - The external branch only accepts a copy under the `brn-int-d-runner-interop-20261017-` prefix, so it cannot open the original `brn-int-d-qualification-…` fixture.
  - It refuses symlinks (requires `canonicalize()` to equal the path), writes its output with create-new and no-follow, and never touches the ignored runner, gate, reservation or Auth.
- **The red result is genuine.**
  - The "before" expected context equals the old preflight context exactly.
  - The failure is at line 3075, the final whole-context assert, after the check that everything except the policy digest matched had passed.
  - Recorded result: 0 passed, 1 failed, exit 101.
- **The green result is a whole-value match.**
  - The after run passed 1/0.
  - `rust-actual-context.json` (`1bdd3258…`) equals `python-expected-context.json` (`1e4e2eb2…`) as whole values.
  - Both copies' Rust outputs hash to `1bdd3258…`.
  - Both copies still have main DB `e26d55cd…` and no WAL or SHM files, so the read-only open did not mutate them.
- **The receipts check out.**
  - All check-index file hashes match.
  - The decoded log hash equals each receipt's `log_sha256`.
  - Every receipt reports 334 sources, `source_unchanged` true, and source hashes equal to the a65 blobs.
  - No command uses `--ignored`.
  - The test binary `c4d5d972…` was not modified after any of the three test receipts.

## Current preflight `54c748a5…`
- **The live file matches the committed copy byte for byte**, as do the rebind receipt (`180ac7bb…`), which equals the recorded stdout hash, and the rubric (`b45106e5…`), which equals the repo copy.
- **Only these fields differ from the old preflight:**
  - `material_pin`, `runner_sha256`, `executable_sha256`, `rubric_sha256` and `output`;
  - `context.settings.policy_sha256`;
  - the `threads_int_d_tests.rs` source hash.

  Everything else is unchanged and still correct: the 334 keys equal the compiled input set, and all values match both the a65 blobs and the worktree. The observer, corpus, prompt (`523b69c7…`), manifest (`64d2b250…`) and database stamp are identical.
- **The context is right.**
  - It equals the Rust proof, with policy hash `34e43ec3…` in the correct field order.
  - It has 68 inputs sorted by ID.
  - Settings are v4 with the literal reference.
  - Target version 2 matches the manifest's control `note_version`.
- **Live state matches.**
  - The executable at that path and its `.d` hash to `c4d5d972…` and `9cd8059b…`.
  - The fixture main DB is `e26d55cd…`, with no WAL or SHM, and the fixture tree mtime is still 18:28.
  - There is no `activation.json` in either preflight directory, no output directory, and the new directory is 0700.
- **The old preflight is preserved.** All 12 old files are byte-unchanged, including `befbd639…`, and it is labelled SUPERSEDED FAILED READINESS.
- **The rebind script did only what it claims.** It reads metadata and calls `git show` or `rev-parse`. It never opens a Store, generates, configures or calls Auth, and it re-checks the origin snapshot before writing.

## Optional #2 (rubric for frames after the first pair)
The change is prospective only and the observer is unchanged. It is consistent with the earlier disposition: a tail after the pair is not automatically FAIL, it needs actual Interrupted/no-Apply/free-lease proof, and without that it stays UNKNOWN.

## Blocking findings
None.

## Optional residuals (none block)
1. **LOW: the activation instructions point at the old preflight.** `runner/preflight/future-invocation.json` still names the old 9d preflight and activation paths, and there is no updated equivalent under `opus1/`. If used as-is, the gate refuses before reservation, so no start is lost. The parent should point the environment at `…-b1-a65a6a61/{preflight,activation}.json`.
2. **LOW: the test binary is easy to invalidate.** The preflight binds `/private/tmp/brn-int-d-20261017-target/debug/deps/brn_ai-0a77908ed974f7b9`. Any later Cargo build in that target directory may overwrite it and make the gate refuse. Run that exact binary directly and do not rebuild before activation.
3. **INFO: the `.d` hash is weak binding.** It is unchanged across rebuilds because it lists paths, not bytes. That predates this change, and the receipt chain still provides the binding.
4. **INFO: one rubric case is unclassified.** A filesystem failure after a complete pair is neither FAIL nor explicitly UNKNOWN. In practice it would be scored UNKNOWN through "missing metadata".
5. **INFO: the default run does not compare against the real fixture.** The default 7/0/1 run only executes the field-order branch; the comparison on the copied real fixture is the separate exact 1/0 run. Both are labelled accurately.
6. **Deferred items remain deferred, as listed:** lost result on panic, dropped static error class, re-read after hashing, the expiry upper bound (the parent will issue a short expiry), the inherited Git environment, the cooperative timeout, the race between gate and prepare on purpose and no-Runs, artifact trust resting on the receipt chain, compactor overhead, and the status-Activity backstop.

## Retained qualifications
- CLI 4/0 is from the 9d run only; there is no new CLI run.
- The original 3 errors remain UNKNOWN.
- The old 2-start window is closed with no candidate, and the unused 10 carry no authority.
- PDF, unexplained SDK/widget, platform, hosted, native-human, compatibility, INT-W, main, owner and program, and live I01 qualifications all remain.
- A future run may still give a non-unique `tool_rejected` or unknown cause.
- The full ignored-runner plus Auth path has never been executed, and this review issues no activation.
