# PR214 Windows Core failure triage

**Conclusion:** The failed optional Windows Core job is the same documented Windows compilation boundary seen on resulting main213, not a new PR214-specific failure mechanism. The evidence is unusually direct: both jobs fail the same `brn-ai` compilation with the same 26 compiler diagnostics and the same set of codes (`E0425`, `E0433`, `E0599`) in `crates/brn-ai/src/auth.rs`.

## Evidence

- PR214 run `39764798850`, job `119777477928`, “Core and CLI (windows-2025)”; head `234956cb171beb025acb1dc17b41be98992bddf0`. The job completed `failure` at 2026-10-22 10:43:43Z. Setup and checkout succeeded; `Build the default workspace` failed, and lint, tests, helpers, CLI, and end-to-end steps were skipped. The compiler ended with `could not compile brn-ai (lib) due to 26 previous errors` and process exit 101.
- The diagnostics are for Unix-only APIs unavailable on Windows: `std::os::unix`, `libc::O_NOFOLLOW`, `O_NONBLOCK`, `geteuid`, and Unix extension methods such as `custom_flags`, `ino`, `dev`, `mode`, `nlink`, and `uid` (plus `Permissions::from_mode`). These occur in `crates/brn-ai/src/auth.rs`.
- Comparison: resulting-main213 evidence [`pr213-resulting-main.json`](pr213-resulting-main.json) identifies main SHA `3827d6a13953533c7322fd7157d975a32dee5cb2`, run `39750356369`, and Windows Core job `119729094562` as failed, while explicitly recording “Optional Windows failures retained.” I fetched that job’s actual log as well: it has the same 26 diagnostics, same three codes, and same Unix API/type-method pattern in the same source file, ending with the same `brn-ai` compilation failure and exit 101.
- Bounded raw logs are retained at `/tmp/brn-pr214-windows-triage/windows-job.log` (110,650 bytes, SHA-256 `a04f9ecd6dadb922863bc74a7f6176be6614ccc82f1f2d3c652a921b0c93de11`) and `/tmp/brn-pr214-windows-triage/main213-windows-core.log` (108,191 bytes, SHA-256 `e6b78430d9837e09feb5d3454b573501da77a2be57c378e12ce7772a3bc43597`). Job metadata came from the GitHub Actions jobs API; no job was rerun.

## Limits

This establishes a repeated Windows compile failure, not whether Windows support is intended or whether the Unix-only implementation should be ported or cfg-gated. It does not establish that PR214’s required checks have completed: the enclosing run was still in progress when inspected, with macOS jobs running. No source, build, or runtime checks were performed locally.
