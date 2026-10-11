# Phase 1 atomic backup publication evidence

## Authority and baseline

The owner authorized continued Phase 1 implementation and CI-gated integration without routine reauthorization. This follows the [reviewed backup plan](phase1-backup-plan.md). Isolated `brn-phase1-backup` / `work/phase1-backup` begins at reviewed steering candidate `30d1185a75c65e90e8019b0d8505b340af94a616`; it was fast-forwarded onto the identical-tree merge `ed9a1af3ed54f6f59a980ceab3d7f2fd4976be76` before backup publication.

## Steering publication

[PR #117](https://github.com/ewq100/brn-rust/pull/117) publishes exact Git tree `93faa8e4db427d7008e0280bf05bbee1d5093cd2`, identical to reviewed local `5b03107`. All local tests and independent review passed as recorded in [steering evidence](phase1-steering-evidence.md). [PR run 38137281250](https://github.com/ewq100/brn-rust/actions/runs/38137281250) passed all four required contexts and documentation/tooling. PR #117 merged at `ed9a1af3ed54f6f59a980ceab3d7f2fd4976be76`, with the same tested Git tree. Required contexts and exact PR head were checked immediately before merge; none were bypassed. Non-required Windows Core/CLI failed. Post-merge checks are pending.

## Backup plan review

Independent GPT-6.1 Sol review: APPROVED after clarifying relative core path compatibility. The backend preserves existing relative paths through internal absolute resolution; app/CLI documentation describes absolute destinations but the actual backup wrappers forward paths unchanged; implementation preserves that behavior too. Same-parent staging, exclusive final publication and standalone snapshot checks are explicit.

## Implementation and checks

A GPT-6.1 Sol helper moved backup publication into the core `backup.rs` module. Private staging uses exclusive directory creation (0700 on Unix), SQLite Online Backup, standalone DELETE journal mode, close/validation/file flush, exclusive hard-link publication and parent directory flush on Unix. Drop cleans only its owned stage. The existing checked writer, schema, dependencies and backup API remain unchanged. Relative caller paths are resolved without process cwd mutation. Early occupied-destination refusal preserves the previous inexpensive failure, and exclusive publication still handles a later collision.

Observed red: after extracting the previous implementation behind the stage/publish seam, the final-path-absent test failed because the final filename already existed. It passes with private staging. The WAL witness confirmed that Online Backup copies journal mode, so the stage explicitly switches to DELETE and is independently restorable without WAL/SHM files or staging dependencies.

Final complete core/app suites passed (78 core, 31 app), along with the focused CLI full-import/export/backup/restore witness and strict affected all-target Clippy. Nine new core unit witnesses exercise the production stage/publish seam. A second behavioral red caught occupied destinations reaching SQLite work before rejection; the restored early check now passes. Only synthetic stores and temporary destinations were used; no user data, external backup location or live provider was accessed.

The parent-flush failure witness ran as an unprivileged Mac user: a mode-0300 parent allowed publication but denied reopening for flush. The operation reported failure, retained a validated complete final snapshot and removed its owned stage. Privileged test runners explicitly exercise the success branch instead; they do not claim the permission failure occurred.

Commands on Darwin arm64 / Rust 1.98.1, all Cargo commands with `CARGO_TARGET_DIR=../threads-macbook-checkout/target`:

```sh
cargo test -p brn-threads-core --lib backup::tests --locked --offline
cargo test -p brn-threads-app --test product full_import_retains_assets_source_identity_and_rejects_changed_retry_input --locked --offline
cargo test -p brn-threads-core -p brn-threads-app --locked --offline
cargo test -p brn --test threads full_markdown_import_export_and_backup_are_fresh_and_readable --locked --offline
cargo clippy -p brn-threads-core -p brn-threads-app -p brn --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
```

Focused core backup: 9 passed; app retained-assets backup witness: 1 passed; CLI backup/restore witness: 1 passed. These overlap the complete suites. Logs: `/tmp/brn-phase1-backup-{focused-core,focused-app,core-app-tests,cli-tests,clippy}.log`.

Independent review identified one small compatibility issue: a read-only file handle cannot flush on Windows. The snapshot flush now opens read/write without create/truncate, following the [Windows flush contract](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers). The nine focused backup tests and strict affected Clippy passed after this change; formatting and diff checks passed. Local Mac checks do not establish Windows qualification.

Independent GPT-6.1 Sol final review: APPROVED against `ed9a1af`, no remaining actionable findings. The reviewer independently reran all nine core backup witnesses (no skips) and verified the frozen five-file implementation fingerprint `d3650d97aff392e7fe563b6a2da7a8ba4ab8a161b836bf6816fa9a25ef9c9bb8`. No production changes followed approval. Final Markdown check: 102 files, 386 local links, zero failures.

## Limits

This establishes complete backup publication. Saved destination UX, transactional Threads upgrades/pre-upgrade orchestration, and broader crash/recovery qualification remain outstanding. No native UI behavior changes or release are included.
