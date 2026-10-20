# Independent review: Mac MCP test roots

Repository: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`.
Branch: `codex/mac-mcp-test-roots`.
Candidate: `4f665daa9fbc430fcf98b68dc029c892bb941fd2`.
Baseline and verified merge base: `a23f486b109c67e8d07c77800030e527b9a18002`.
The checkout was clean at entry and after inspection. Reviewed the complete actual one-file diff and the entire `crates/brn-threads-core/tests/mcp_policy.rs`, together with relevant store guards, adjacent fixture conventions, README, local verification driver and CI caller.

## Findings

No actionable findings.

The helper at `mcp_policy.rs:3-5` canonicalizes the existing OS temporary parent and creates a fresh RAII-managed `TempDir` beneath it. All four independent test fixtures use it (`:144`, `:199`, `:332`, `:414`). This resolves macOS `/var` aliases without accepting a symlinked workspace path or disabling the production guard. It retains per-test random isolation, cleanup ownership and the existing lifetime of every store, backup and restored copy. It neither reuses prior fixtures nor mutates environment variables. A missing/inaccessible temp parent still fails visibly through the existing test-style unwrap rather than silently selecting another location.

All MCP policy/authority, replay, run-fence, private/project scope, grant isolation, canary exclusion and backup/restore assertions are unchanged. The tests can now reach these checks under the observed default Mac environment. No production code, schema, dependency, environment configuration, test count or cross-suite framework changes are present. The local helper duplicates the exact established `skill_policy.rs:2-4` convention; `data_folder.rs:7-9` uses the equivalent canonical-parent approach. This is an appropriately bounded fixture correction and does not require a new meta-test of its own construction code.

The core README already requires canonicalizing macOS OS aliases before choosing a synthetic directory and documents direct `cargo test -p brn-threads-core`. The candidate aligns these fixtures with that contract. The local `verify-storage.sh` invokes the workspace tests before its later canonicalized CLI scratch directory; the fix is useful there as well as for direct tests. No documentation expansion is needed for a test-only correction that adopts the existing rule.

## Original failure proof and qualification

Read `/tmp/brn-restore-refusal-qualification/first-green.log`: all four MCP policy tests fail at their first `Store::open`, with `DataFolder::SymlinkRefused` for fresh `/var/folders/.../T/.tmp.../data` paths. The original fixture source uses `tempfile::tempdir()`. The guard in `store.rs:74-83` checks every ancestor using `symlink_metadata` (`:2884-2894`) before creating/opening workspace contents. The evidence therefore supports a path-fixture failure before any MCP-policy or restore behavior is exercised.

Independently checked that `mcp_policy.rs` and `store.rs` have no diff between failing restore candidate `166d823232b1d4b7785630f90e99cb899999f60f` and this task's baseline `a23f486b109c67e8d07c77800030e527b9a18002`. The earlier source-only restore validation correction does not explain these failures. The preserved log does not itself embed a commit hash; its exact launch/source association is supplied by the lead.

The completed `/tmp/brn-mac-test-roots-qualification/check.log` identifies the inherited `/var/folders/.../T/` TMPDIR and compilation from this worktree, and records all four MCP policy tests passing. Its script has no TMPDIR/TMP/TEMP override. The core test binaries and doc tests record passing results, and the log now ends with focused strict Clippy completion (`Finished dev profile`), with no lint error. The lead owns the actual command-session exit result; this reviewer inspected the log and did not launch tests/builds. `git diff --check` passed.

Corrected reviewer evidence observation: my original report and message incorrectly said CI's canonical TMPDIR override was confined to the later offline end-to-end step. That conclusion inspected only the workflow and missed its composite setup action. The original report is preserved as `/tmp/brn-mac-test-roots-review-original.md`. In the actual baseline/candidate source, **Core and CLI** invokes `./.github/actions/setup-rust` before the workspace test (`.github/workflows/ci.yml:68-70,99-101`). That action's **Create an explicit safe Unix fixture parent** step creates a fresh directory, resolves it with `pwd -P`, and appends `TMPDIR` to `GITHUB_ENV` (`.github/actions/setup-rust/action.yml:82-89`). GitHub Actions makes values written to `GITHUB_ENV` available to subsequent steps in the same job, so the later Cargo test inherits the canonical parent on Mac and Linux. The later inline end-to-end override is additional, not the sole CI protection. The original audit's CI/local-environment explanation is supported. This correction changes evidence accuracy only; the source candidate and no-findings conclusion are unchanged.

## Limits and ownership

This is an independent read-only source/test review under the installed independent-review and Rust checklist guidance already read for this review session. No builds, network, native/account actions, production edits or nested helpers were performed. Only this requested review artifact was written. No native usability, owner acceptance, other-platform execution, merge or release claim is made. The lead owns completed check evidence, dispositions and integration; review applies to the exact clean candidate above.
