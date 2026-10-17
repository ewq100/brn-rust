**Verdict: CHANGES_REQUIRED.** One Medium change is needed, and it is a one-line source fix. This verdict covers source only; it accepts nothing and authorizes no merge.

The pin matches: HEAD `a499712d`, both trees, and both full-diff SHA-256s in `review1-source-pin.json`. The only uncommitted file is that pin record. I did not recompute the two material hashes because the pin file does not say how they were computed. The correction diff touches no core, `threads_guides.rs`, manifest or lock file.

## Required change

**Medium — damaged stored packages are reported as digest conflicts (new in this correction).**
- **Where:** `crates/brn-ai/src/skill_packages.rs:311-316`. `Catalog::read` now returns `conflict()` for a stored package whose recorded digest or name doesn't match its contents. Round 1 returned a plain "identity/digest mismatch" refusal here.
- **What goes wrong:** Every command reads stored packages through `Catalog::read`, so one damaged package affects all of them:
  - `skills` fails with `skill_digest_conflict`.
  - `skill-install` of an **unrelated** name fails with `skill_digest_conflict` (through `installed()` at `:399`), although that name has no conflict.
  - `skill-inspect` and `skill-remove` of the damaged name return the same reason. The message tells the caller to "inspect, remove with its exact digest". Both of those also fail, so an agent following the advice loops, and recovery needs manual file deletion.
  - A *malformed* package (bad JSON or failed validation, `:309-310`) still gets a generic refusal with no reason. Two kinds of damage now get two different classifications.
- **Why it matters:** Round 1 asked to stop conflating corrupted packages with digest conflicts. This change reintroduces that conflation, and `crates/brn/README.md` (reason table) says `skill_digest_conflict` is recoverable by inspecting.
- **Correction:**
  - At `:315`, return a distinct damage error (the previous `bad(...)` text, or `InvalidData`).
  - Keep `conflict()` at `:396`, `:453` and `:479`.
  - In `well_formed_body_digest_and_file_name_binding_refuse_without_deletion`, assert the error is not `PackageRefusal::DigestConflict`.
  - Optionally add a command-line test that `skills` with a damaged package carries no `skill_digest_conflict` reason.

  The existing `||`→`&&` mutation kills are unaffected.

## Low, not blocking

1. **Missing source folder:** `skill-install --source /typo` hits ENOENT in `Dir::absolute` (`skill_packages_fs.rs:56`) and reports `io_error`/`internal`, whose guidance is "Report it". That is a caller mistake. You could map NotFound while reading the source to `Invalid`.
2. **Test assumes non-root:** `actual_lock_permission_denied_is_io_error_not_caller_refusal` relies on EACCES, which root does not get.
3. **Absent-package mapping relies on text:** it goes through `AppError::Invalid` with an exact string that the command-line adapter matches (`contract.rs:614`). It is confined to the skill commands and works; this is a preference.

## Confirmed sound

- **Error handling:**
  - Raw OS error numbers are kept before any `ErrorKind` mapping; only symlink and not-a-directory errors (ELOOP/ENOTDIR) are deliberately turned into validation errors.
  - A held catalogue lock reports `busy`/`failed` with retry `read`.
  - Relative `--source` is refused as `invalid_value`/`usage` before any workspace I/O.
  - An absent inspect reports `not_found` and creates nothing.
  - Busy and not-found remapping applies only to the four skill commands; the shared error mapping is unchanged.
- **Unsupported platforms:** the guard runs before restore, workspace opening or any package file access. Gating is consistent across the AI, app and command-line layers and their tests.
- **Identity tests:** each test makes one condition fail on its own. The test-only hook replaces the package between the checked read and the rename, so the real recheck and rollback run.
- **Cleanup:** stage cleanup no longer replaces the original publication error.

## Evidence and gates I did not verify

- **Final test run verified:** the raw log hash matches, its summaries total 851 passed / 0 failed / 2 ignored over 57 test binaries, and all 8 new tests appear. All 277 recorded source hashes match the pin.
- **Not verified:**
  - Linux and the non-macOS/Linux compile (static review only).
  - Running as root.
  - The mutation supplement, which is pending.
  - The stale full-workspace run (1163/2/6, exit 101).
  - Native and live behaviour.
  - The full runtime acceptance checks, S01–S04, which belong to D3b.
