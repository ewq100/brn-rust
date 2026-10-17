**Verdict: APPROVED.** I found no blocking defects. This approves the round-2 correction at source level only; it is not D3a acceptance or merge authority.

I reviewed read-only: no builds, tests, edits, delegation or network calls. HEAD `9b8acf66`, its tree `b4fb8d06` and the `crates` tree `2f00dc92` all match the pin, and the worktree is clean. From `a499712d` to `32e571f5` only docs change. From `32e571f5` to `9b8acf66` there are three crates files and three docs files.

## The fix

- **The one production line is correct.** At `crates/brn-ai/src/skill_packages.rs:315`, damaged stored identity or digest now returns `bad("installed package identity/digest mismatch")`. That is a plain validation error, the same class as the malformed-JSON case at `:309`.
  - The app layer (`brn-threads-app/src/skills.rs:5-22`) maps it to `Invalid`. Its message matches no exact or prefix entry in `REFUSAL_REASONS` (`contract.rs:305`), so it carries no `reason`.
  - `conflict()` is still returned only at `:396` (install), `:453` (remove with the wrong digest) and `:479` (moved bytes changed during remove).
  - The guard conditions, `PackageRefusal`, native error mapping, busy and not-found handling, and platform gates are unchanged.
- **The tests cover it in proportion.**
  - The AI test (`skill_packages_tests.rs:385-394`) now checks the returned error is not `DigestConflict`, for both the changed-body and changed-name variants. The bytes-preserved and restore-then-remove checks remain.
  - The new CLI test (`brn/tests/skill_packages.rs:165-246`) runs malformed JSON and a well-formed stale digest through all four commands, including installing an unrelated package. Each must return exit 6, `rejected`, `invalid_request` and no `reason`. The damaged bytes must be preserved and the unrelated package not installed. It then restores the file and checks inspect and digest-bound remove succeed.
  - The positive control still requires `skill_digest_conflict` for a real wrong-digest removal (`:53-61`).
  - None of the earlier error meanings come back.

## Receipts I checked against the actual files

- All 337 `corrected_input_hashes` match the files at the pin.
- The final patch (`564ded6d…`) is byte-identical to `git diff 32e571f5 9b8acf66 -- crates`.
- The tests-first patch (`ee969436…`) is exactly the test-only part of that diff.
- In both before-phases, the overridden `skill_packages.rs` hash `87d3fadb…` equals the file at `a499712d`.
- **Before runs:** the AI test failed at `:386` on the changed-body variant with the `DigestConflict` message. The CLI test failed at `:213` on `skills` with `skill_digest_conflict`, which is the well-formed case, after the malformed loop passed. Both failed 0/1 with exit 101.
- **After the fix:** the scripted run's 57 summaries total 852/0/2, and the new and control tests appear as `ok`. Clippy and fmt exited 0.
- Every recorded log hash matches its text, and every phase records the source as unchanged.
- The release record shows 0 mutation commands and the 776.295/1800 s budget unchanged.

## Optional, not blocking

- **Low (docs wording):** `crates/brn/README.md:177` and `:446` say identity conflicts carry `skill_digest_conflict`. Stored-package damage now deliberately carries no reason. One clause, for example "a damaged stored artifact is a plain `invalid_request` without `reason`", would make that explicit. This is optional and does not justify another review round.

The round-2 Low residuals and the earlier qualifications remain as you listed them. Linux, Windows, native, live, the full workspace run and S01–S04 were not verified.
