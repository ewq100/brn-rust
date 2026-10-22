# Full advisory tool feasibility

Read-only feasibility check; no tool download, build or audit was performed.

## Cheapest trustworthy route

Use the RustSec-published `cargo-audit 0.22.2` Apple Silicon macOS archive from the official `rustsec/rustsec` GitHub release, staged in a new task-local temporary directory after the active build gate. The official expanded release-assets page lists `cargo-audit-aarch64-apple-darwin-v0.22.2.tgz` (5.68 MB) with SHA-256 `ec7ca4263769593df4d909be85b94a6b79efa2897be5d2bb8ebd516e823175af`; the release page identifies tag `cargo-audit/v0.22.2` as signed/verified. Verify the downloaded archive against that published digest before extraction and record the executable hash. This avoids consuming the Rust build slot or installing globally. The archive is a genuine official platform asset; its existence and digest are confirmed from the release metadata, not by a local download or execution.

The cached source-build fallback is less economical. Local read-only inventory found the exact `cargo-audit-0.22.2.crate` (SHA-256 `700c2b240f7fd330c24b675fe429f73a5b676531fcc6300400b2b67f155ba12a`) and `rustsec-0.33.0.crate`; the packaged audit manifest requires Rust 1.88, compatible with the repo pin. Its packaged lock has 398 registry packages, of which 365 exact `.crate` archives are cached and 33 are absent. Some missing entries may be target-specific, so a full offline install is not proven impossible, but completeness cannot be claimed without attempting a build. This is unnecessary given the official arm64 artifact.

## Advisory database freshness

The existing `~/.cargo/advisory-db` checkout is at `7eebec69c352c7191b1f13eb95dd510eeca5d1de`, dated 2026-10-09, so it is not an adequate current snapshot for a later full assessment. Use a fresh task-local DB path and the audit command's explicit `--db` argument, allowing the tool to fetch from its official default `https://github.com/RustSec/advisory-db`; then record that checkout's exact `git rev-parse HEAD` and audit timestamp. This preserves the shared default cache. The source archive confirms the CLI has `--db` and that database fetch is enabled by default; `--file` selects the lockfile. The official cargo-audit README recommends passing `--file` and running from outside the project when auditing an untrusted project, preventing Cargo from generating a lockfile or reading project Cargo configuration.

Suggested future shape, after the lead selects the scan and the build gate is clear:

```sh
cargo-audit --version
cargo-audit audit --file /absolute/path/to/brn-rust/Cargo.lock --db /private/tmp/brn-advisory-audit/advisory-db --format json
```

Confirm the installed archive exposes the shown subcommand shape before invoking; the official Cargo subcommand form is `cargo audit --file ... --db ... --format json`. Run from `/private/tmp/brn-advisory-audit`, not the repository. If auditing the document-helper lockfile too, perform a separate invocation against that exact lockfile and preserve separate output/DB commit evidence. Do not silently reuse the Oct-09 DB or ignore advisories.

## Local evidence and limits

- Checkout inspected: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, source HEAD `250348fcd3919c4982784e30ec49a94c1bcf36ce`; no source edits made.
- `cargo-audit` and `cargo-deny` are absent from normal `PATH`.
- The official release assets page provides the arm64 archive digest; this report did not download or independently reproduce it. Verify the SHA before trusting the staged archive.
- The local advisory DB date/hash above is directly observed; latest DB commit at a future scan is intentionally unknown until fetched and recorded.
- No advisory findings, exposure, or reachability conclusion is made here. A lockfile audit is a name/version match against the selected database snapshot, not proof that a vulnerable code path is reachable.

Primary sources: [RustSec cargo-audit 0.22.2 release and signed tag](https://github.com/rustsec/rustsec/releases/tag/cargo-audit/v0.22.2), [official release asset list and SHA-256 values](https://github.com/rustsec/rustsec/releases/expanded_assets/cargo-audit/v0.22.2), and [cargo-audit README usage](https://github.com/rustsec/rustsec/blob/cargo-audit/v0.22.2/cargo-audit/README.md).
