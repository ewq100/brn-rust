# Linux-only shared Rust CI cache candidate

- Repository: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Base: `eae61dde04cda19c764246993966b13b997014eb`
- Branch: `codex/mac-first-ci-cache-scope`
- Candidate: `786ebfd2f1a29f93eb2be4f43ee64606023db791`
- Changed path: `.github/actions/setup-rust/action.yml` only
- Final checkout: clean
- Previous frozen branch `codex/backup-process-witness-bounds` remains at `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5`.

The non-macOS shared Rust-cache step now runs only when `runner.os == 'Linux'`. An adjacent comment states that Windows checks still run without shared Rust caching and that storage is reserved for the supported Linux/macOS lanes. The separate macOS cache action, including its inputs, cache key, workspaces, and save condition, is unchanged. No jobs, commands, timeouts, gates, dependencies, or protection settings changed.

This implements the selected CI scope decision in `ci-cache-scope-selection.md`: preserve checks while avoiding refresh/copy of optional Windows build graphs in this Mac-first campaign. The selection records 1,540,479,660 bytes in existing Windows cache entries and a repeated exact-key Mac miss, but neither cause nor guaranteed storage reclamation or speedup is established. Existing Windows entries remain subject to natural retention/eviction; this candidate performs no deletion.

Checks: `git diff --check` passed; Ruby/Psych parsed `.github/actions/setup-rust/action.yml` successfully. An initial parser invocation used an `aliases:` keyword unsupported by this installed Ruby version; the compatible `YAML.load_file` invocation passed. No tests, builds, CI runs, cache operations, or workloads were run; lead owns independent review and any appropriate Python checks.

Full patch: `/tmp/brn-ci-cache-scope.patch` (746 bytes, SHA-256 `4fd1bd75df005ac079f9fc802a8fd79128599d0175846254679f2b35d7f51722`). Candidate commit is local and not published. Parent PR210 integration remains a prerequisite before publication.
