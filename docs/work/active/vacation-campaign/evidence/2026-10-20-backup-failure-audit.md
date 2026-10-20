# Bounded backup/export/restore failure audit

Repository: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`
Branch: `codex/agent-first-followthrough`
HEAD: `f8e725385e56172e2ac654409d5af3e20352e1c8`
Inspection only; no builds, tests, app execution, native access, or repository writes were performed. Initial worktree status contained pre-existing edits/untracked evidence under `docs/work/active/vacation-campaign/` and `docs/work/overview.md`; those were left untouched.

## Finding: export success is not durably published across a power loss

`crates/brn-threads-app/src/export.rs:109-115` syncs each output file and the manifest, then calls `rename_exclusive(stage.path(), destination)` and immediately returns `Ok(manifest)`. It does not sync the staged export root (which contains the Markdown and manifest directory entries), the `assets/` directory (which contains asset entries), or the export parent after the rename. On macOS/Linux, the exclusive rename is atomic against replacement but, absent directory flushes, is not a durability barrier. A sudden system/power interruption after `rename_exclusive` returns can therefore lose the publication entry or directory entries while the UI has received success; the manifest can claim files that are no longer reachable after recovery. This is a concrete completion-boundary gap by source inspection, not an observed failure.

Contrast: backup flushes the completed snapshot and then its destination parent after hard-link publication (`crates/brn-threads-core/src/backup.rs:102-106`, `110-125`). Restore flushes the staged database/marker and staged directory before rename, then flushes the published destination parent (`crates/brn-threads-core/src/restore.rs:84-95`, `102-110`, `120-126`). The UI worker reports success only when `export_now` returns `Ok` (`crates/brn-desktop/src/threads_ui.rs:3320-3343`, `2743-2774`); it does not add a durability check.

Existing witnesses: `crates/brn-threads-app/tests/product.rs:785-807` covers ordinary export failure without partial final publication and preserves an occupied destination. `crates/brn-desktop/src/threads_backup_tests.rs:78-164` covers picker cancellation/error and successful export status. Neither witnesses directory-flush failure/durable publication. These tests were read, not run.

Actionable scope if selected: make export publication durability match its success claim by syncing relevant staged directories and the parent after exclusive rename (with platform-appropriate behavior), and add a focused synthetic fault/durability witness. No change was made in this audit.

## Boundaries assessed with no defect found by inspection

- Backup refuses an already-present destination before snapshot work and uses exclusive hard-link publication, so a competing destination is not overwritten (`backup.rs:58-73`, `110-125`). Existing witnesses include `backup.rs:197-211`, `214-229`, and `crates/brn-threads-core/tests/core.rs:931-955`.
- Restore stages the full workspace before publishing; it rechecks freshness and symlink ancestry before rename. Existing destinations/late occupants remain preserved and owned stages are dropped (`restore.rs:42-58`, `127-167`, `169-184`; tests `276-300`, `382-395`, `482-509`).
- Backup/restore stage cleanup is deliberately ownership-checked: exact UUID name, direct real directory, regular marker with exact BRN text, and a free lock before removal (`crates/brn-threads-core/src/stage.rs:92-165`). Backup and restore tests cover killed stages, live stages, and unrelated names/entries (`backup.rs:422-564`; `restore.rs:327-379`).
- Picker failure/cancel status and worker busy admission are covered in `threads_backup_tests.rs:78-164`, `228-280`; unavailable saved backup directory failure does not claim creation (`282-318`). A permission-specific UI worker-result witness was not located in this bounded read, but source flow uses `Result` success/error directly and reports the error text; I found no false-success path there.

No qualification was executed, so this is not a test result or native observation.
