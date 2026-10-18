**Verdict: CHANGES_REQUIRED.** The required change is one bounded CLI error-mapping fix. I found no data-loss, overwrite, or authority-escalation defect in the package code.

I reviewed the actual code and receipts read-only, without delegation, builds, or tests. The pin matches: HEAD `15c05741`, both trees, the full diff SHA `00ae450d…`, and 32 files. The runtime part of `threads_guides.rs` before the test module is byte-identical to the base (`8c738662…`). `threads.rs`, the manifests, and the lockfile are unchanged. The only working-tree change is the untracked `source-pin.json`.

## Required change

**Medium — install/remove/inspect errors come back with the wrong CLI error codes.** Evidence: `crates/brn-threads-app/src/skills.rs:5-11`.
- **Problem:**
  - A busy lock (`WouldBlock`) becomes `Unavailable`. The contract says `unavailable` means "not available in this build or platform".
  - Every other I/O error becomes `invalid_request` with status `rejected`, a "caller's fault" refusal. That includes digest conflicts, not-found, corrupted artifacts, and real disk errors such as EIO, ENOSPC, or EACCES.
- **Why it matters:** The brief says "other lock errors remain actual IO errors", but the CLI does not do this. The rest of the app returns `AppError::Io` for real I/O errors, which the CLI reports as `io_error`. Agents cannot tell a conflict from a missing package or a disk failure, so they cannot choose a correct retry.
- **Fix:**
  - Real I/O errors: pass them through as `AppError::Io`.
  - Validation failures: keep `Invalid`.
  - Digest conflict: add a `REFUSAL_REASONS` entry such as `skill_digest_conflict`.
  - `skill-inspect` of an absent package: return `not_found`.
  - Busy lock: return `busy`, or `unavailable` with a specific reason, and update that code's meaning text.
  - Add `("skill-install","source",Grammar::AbsolutePath)` at `spec.rs:~438-447`. A relative path should then be refused as `invalid_value`, like the other ABSOLUTE options.
  - In `crates/brn/tests/skill_packages.rs:53`, assert the actual error code and status instead of only `code != 0`.

## Low or non-blocking

1. **Marker creation can fail under a race or crash** (`skill_packages.rs:229-266`). If two first installs race, or a process dies between creating the `skills` folder and writing the marker, commands fail with ENOENT or "foreign skills folder". A crash leaves skill management stuck until the folder is deleted by hand. It fails closed and loses nothing. Fix: write the marker to a stage file and rename it exclusively into place, then accept an empty or BRN-staged `skills` folder. No test covers concurrent first initialization.
2. **Parser accepts a few YAML-divergent values** (`threads_guides.rs:168-185`). For example, `description: a #b` is read as `a #b` but YAML strips the comment; `: ` inside a value, `~`, `null`, and `true` also differ. Bidi format characters (U+202E, U+2066–2069) are accepted. This is interop and display only. Fix: reject ` #`, `: `, and bidi characters, or document that values run literally to the end of the line.
3. **Stage cleanup can hide the original error** (`skill_packages.rs:399-400`). The `?` inside the cleanup condition returns any metadata error in place of the real failure. Use non-propagating checks.
4. **Read-only commands take the exclusive lock**, so `skills` or `skill-inspect` fail as busy during an install. A shared lock for reads would avoid this.
5. **Built-in entries show the installed-package `retention_notice`** ("Installation does not enable…") alongside `enabled:true`. Cosmetic.
6. **Some paths have no direct test:**
   - rollback of a removal when the moved content no longer matches;
   - stage cleanup by inode;
   - detecting a file that changes during a bounded read.

## Confirmed sound

- **Filesystem handling:**
  - All opens go through descriptor-relative NOFOLLOW calls.
  - Files must be single-link and non-executable, with bounded reads that check size and mtime.
  - Publication writes a stage, fsyncs it, then renames without overwrite; removal is digest-bound, rechecks the moved bytes, and rolls back without overwrite.
  - The length-framed SHA-256 is unambiguous, the source folder is never touched, and the limits match the C0 decision.
- **Core change:** only the literal `skills` real-directory entry was added, with the symlink, marker, adoption, and database guards intact. This matches the existing refusal of symlinked data-folder ancestors.
- **Scope:**
  - No model route, enablement, or actor grant.
  - The four built-in IDs are preserved and installed names cannot shadow them.
  - The example package is not a fifth built-in.
- **Retention:** the backup/export exclusion is real, stated in docs and help, and has a positive control. It stays a D3b reconciliation gap against target lines 250/448; this is not complete D3 backup coverage.

## Qualification gaps

- The `final-capability` (431/0/1) and affected-Clippy receipts fingerprint the exact pin.
- The `final-service-cli-core` 10/0, `final-package` 9/0, and README 1/0 receipts fingerprint an earlier `skill_packages.rs`. That is disclosed as before the lock-error fix. The app, CLI, and core tests have therefore not run at the pin; rerun them after the fix.
- The full-workspace run stays 1163/2/6 with exit 101.
- The mutation campaign started after the pin (`mutation-start.json`); I make no claim about its results.
- Linux and native platforms were not exercised.
- S01 runtime loading, S03 per-run fencing, and S04 project scope remain D3b work. Full S01–S04 does not pass at this slice.
