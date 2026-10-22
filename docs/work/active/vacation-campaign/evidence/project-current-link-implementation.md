# Project current-document link guard candidate

- Branch: `codex/project-current-document-link`
- Test-only baseline: `8c47af45bb0b0ded56cf58632d8d0232de9419c4`
- Production candidate commit: `046b9da2c8a872ed22656587653eb0ffada87a5b`
- Production file changed: `crates/brn-threads-core/src/projects.rs`

At the checked `ProjectCommand::LinkDocument` boundary, the command now refuses an active Note whose `superseded_by` is set, with `Invalid("Choose the current replacement document")`. The shared Link/Unlink arm applies the guard only to LinkDocument; `UnlinkDocument` still resolves and removes an already-linked superseded Note by its original ID. No command shape, version framework, context policy, migration, privacy, history, or Undo behavior changed.

The lead's test-only baseline run compiled and failed at the intended assertion because the old implementation accepted the new link. The candidate has not been built or tested in this worktree; qualification remains with the lead. Formatting and patch whitespace only: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-threads-core/src/projects.rs`; `git diff --check` passed. Not run: builds, tests (including `cargo test -p brn-threads-app --test projects`), CLI workloads, native/account/network/SQL/dependency work. Worktree clean after commit.

## Contract documentation update

Added the concise behavior contract to `docs/architecture/threads-behavior.md` in documentation commit `18abae94fae754834af14214b9fc8910562d046d`. This states that new Project links require a current unsuperseded Note, supersession does not retarget prior links, and owners can unlink by the original Note identity. Final branch HEAD: `18abae94fae754834af14214b9fc8910562d046d`. `git diff --check` passed; no builds or tests were run.
