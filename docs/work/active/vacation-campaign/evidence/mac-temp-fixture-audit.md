# macOS MCP policy temp fixture audit

## Finding

This is a small test-fixture portability repair, with no need to weaken the data-folder symlink guard or add shared infrastructure. The recorded failure log shows all four `mcp_policy` integration tests failing at `Store::open` with `DataFolderProblem::SymlinkRefused`; their temp roots were under `/var/folders/.../T/.tmp...` (`/tmp/brn-restore-refusal-qualification/first-green.log:270-298`). The fixture path is the cause: `tempfile::tempdir()` selected a default macOS temp path with a symlinked ancestor, and `Store::open` correctly refused it.

At the selected baseline `a23f486b109c67e8d07c77800030e527b9a18002`, `mcp_policy.rs` has four direct roots: lines 141-142, 196-197, 329-330, and 411-413. Each uses `tempfile::tempdir()` and then opens a child as a BRN data folder. Other files in this crate use a canonical temp parent, for example `mcp_format.rs:8-9`, `skill_policy.rs:2-4`, and `data_folder.rs:7-9`; `chat_storage.rs` also canonicalizes each root before passing it to `Store::open` (for example lines 30-32 and 97-101). No existing shared test support module is present under `crates/brn-threads-core/tests`.

## Verification route

The direct crate command is documented as the core qualification route (`crates/brn-threads-core/README.md:123-130`), so it should pass with a normal macOS environment. It currently fails there when the inherited `TMPDIR` resolves through `/var/folders/...`.

The maintained CI setup addresses this environment condition: `.github/actions/setup-rust/action.yml:82-89` creates a temporary fixture parent, resolves it with `pwd -P`, and exports it as `TMPDIR`, explicitly noting macOS system-temp symlinks. The CI end-to-end fixture lane likewise canonicalizes its fixture parent (`.github/workflows/ci.yml:133-141`). However, local `scripts/verify-storage.sh:26-37` runs workspace tests without setting a safe `TMPDIR`; `docs/development/verification.md:59` identifies that script as the local storage/build/lint/test gate. Thus CI setup covers the issue, but the documented direct crate test and local verification script do not guarantee the safe temp root on a default Mac shell.

## Smallest coherent repair

Add a private file-local helper in `mcp_policy.rs` following the existing adjacent convention, such as `tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()`, and use it for the four roots. This changes only test setup, retains fresh isolated data, and leaves the production symlink protection exercised as intended. No global inventory or new shared test framework is warranted.

Affected checks: the documented `cargo test -p brn-threads-core`, the full `scripts/verify-storage.sh` workspace test stage, and macOS CI core tests. The supplied run confirms the failure with inherited default temp; this audit performed no checks or edits. This fixture-only correction does not overlap the restore-source production repair. Per the task context, open PR 190 is source-frozen and has no `mcp_policy.rs` change, so this is not a duplicate of that candidate.
