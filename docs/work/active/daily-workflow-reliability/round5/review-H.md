# Owner-terms error for an unusable data folder (round 5 slice H)

`Store::open` now sends the I/O failures from its symlink-ancestor check, `create_dir_all`, `read_dir` and the marker create/write through one classifier. Each failure becomes `Error::DataFolder { folder, problem, cause }`, whose text is "BRN cannot use the data folder {folder}: {problem}" with one of five fixed owner reasons. The `io::Error` stays in `cause` and is reachable only through `source()`. A symlinked data folder moves from `UnsafeDirectory("symlink directory")` to `DataFolder { SymlinkRefused }`. No host code changed: the CLI, the desktop headless check and the desktop UI already print `to_string()` from `Workspace::open`, so all three pick up the new text.

Watch for: an existing folder's marker reads (`symlink_metadata`, `read_to_string`, `entry.file_type()`) still use `?` into `Error::Io`, so they can still show "storage: … (os error N)". The Limitations list does not name this (confirmed, non-blocking). Also, the macOS privacy-denial EPERM maps to `PermissionDenied`, whose advice "Choose a folder you own" is wrong for a folder the owner does own (likely, non-blocking).

**Verdict**: APPROVED

## High-level view

The classifier only adds labels. It names five `ErrorKind`s and returns the original `Error::Io` for anything else, so a full disk or a race on an existing folder is never relabeled. `cause` is a `#[source]` field and does not appear in the `#[error]` string, so the `DataFolder` message cannot contain OS text, whatever the OS returns.

The fix sits in the shared checked service. Every host reaches it through `Workspace::open` and prints only `to_string()`, and no host adds its own folder check. The agent's Run path drops the text entirely.

Ownership holds. `crates/brn/src/main.rs` has 0 changed lines, the desktop crate has none, `HELP` and exit codes are unchanged, `docs/status.md` is untouched and `Cargo.lock` is unchanged. Two integration items are recorded: the work/cli-agent-contract exhaustive match needs the new variant, and the symlink refusal changes variant. This branch has no other caller of the old symlink variant.

Every reason has a test, and each new or changed test has a recorded fail-before. The recorded failure lines match the committed files, and the workspace count rises by exactly the seven new tests. A spot-check of the desktop headless binary reproduced the recorded owner text and exit 1.

The classifier's coverage has two gaps. The existing-folder marker reads still bypass it. The `PermissionDenied` advice also covers EPERM from macOS privacy controls, where "Choose a folder you own" misleads. Neither is blocking.

<details>
<summary>Issues (2)</summary>

1. **Existing-folder marker reads keep OS text**: `fs::symlink_metadata(&marker)?`, `fs::read_to_string(&marker)?` and `entry.file_type()?` in `Store::open` still produce "storage: … (os error N)". Add this path to the slice H Limitations list, or wrap the marker reads in `.map_err(unusable)`.
2. **"Choose a folder you own" for macOS privacy denials**: `EPERM` from macOS privacy controls maps to `PermissionDenied`, and the advice is wrong for a folder the owner owns. Revisit the wording at integration, for example "Choose a folder BRN is allowed to use".

</details>

<details>
<summary>Details</summary>

### Where the classifier stops inside `Store::open`

The plan named four call sites, and the diff wraps exactly those. The existing-folder branch is unchanged and still converts with `?`:

```rust
if existing {
    if fs::symlink_metadata(&marker)?.file_type().is_symlink()
        || fs::read_to_string(&marker)? != IDENTITY
    { ... }
    for entry in entries {
        let kind = entry.file_type()?;
```

Confirmed: if a BRN folder's directory can be listed but its marker cannot be read, the owner sees "storage: Permission denied (os error 13)". That happens when the marker is mode 0600 and owned by another account, or when the marker path is a directory ("Is a directory (os error 21)"). This is uncommon: the default umask gives the marker mode 0644, and most read-only or permission-denied existing workspaces fail later in SQLite, which is a recorded limitation. Even so, the commit message and verification claim permission denied without qualification, while the Limitations list names only the SQLite path.

### Owner wording against real failure sources

Rust maps both `EACCES` and `EPERM` to `ErrorKind::PermissionDenied`. On macOS, the privacy controls return `EPERM` when the terminal or app lacks access to a protected folder such as Documents, Desktop or a removable volume. The owner does own that folder, so the second sentence of "BRN does not have permission to use it. Choose a folder you own." gives the wrong advice (likely). The plan prescribed this wording, so any change belongs at integration. No OS text appears in either case.

### One service path to three hosts

```
brn --data-dir P notes ─────────┐
brn-desktop --headless-check ───┼─► Workspace::open ─► Store::open ─► Error::DataFolder
brn-desktop (native UI launch) ─┘        (AppError::Core, transparent)
                                              │
                         host: .map_err(|e| e.to_string()) ─► eprintln! ─► exit 1
brn-ai Run: Store::open(..).map_err(storage) ─► AiErrorKind::Storage (text dropped)
```

The host call sites are CLI `main.rs:410`, desktop headless `main.rs:42` and desktop UI `threads_ui.rs:3592`. None of them walk `source()` or use `{:#}`/`{:?}`. No host logs errors today, so `source()` is the only place the technical detail survives, which meets the brief's "if an existing place exists".

### Fail-before evidence and test coverage

The recorded failure lines (`:17` core, `:20` CLI, `:1103` `core.rs`, `:2510` unit) are each file's first message or variant assertion. The workspace count goes from 801 to 808, which is 1 unit, 4 core integration and 2 CLI tests. The `NotADirectory` (code 20) arm was not isolated by its own mutation: `a_file_in_place_of_the_folder_is_named` fails first on its code-17 assertion, and the unit loop stops at code 30.

Not tested: read-only end to end on Linux, where only the raw-code unit test runs; permission denied on privileged runners, where the integration test skips; missing parent through the filesystem, which is unit-only, as the Limitations list states; and the permission and symlink cases at the CLI level, which the plan did not require.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/lib.rs`: new `Error::DataFolder` variant and the `DataFolderProblem` enum with its owner texts.
- `crates/brn-threads-core/src/store.rs`: `Store::open` routes four I/O sites through the new `data_folder_error` classifier; symlink refusal becomes `DataFolder { SymlinkRefused }`; new unit test for the classification.
- `crates/brn-threads-core/tests/core.rs`: symlink-ancestor test now expects `DataFolder { SymlinkRefused }`.
- `crates/brn-threads-core/tests/data_folder.rs`: new; read-only (macOS), permission, file-in-place and symlink cases through `Store::open`.
- `crates/brn/tests/data_folder.rs`: new; CLI stderr and exit-1 checks for a file and a read-only disk (macOS).
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice H record (before/after table, fail-before, commands, limitations, integration items).

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show 28e64cdf`

</details>
