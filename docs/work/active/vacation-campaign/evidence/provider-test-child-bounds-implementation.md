# Provider stderr test child bounds

Candidate commit: `5d4ae6dbbbb3d4d835b7409db673124f356e926b` (`Bound provider stderr test child`), based on the selected clean branch `codex/provider-stderr-child-bounds` at `8c18845ca5b70a43d36b0bc0d2811da573086a79`.

Changed only `crates/brn-ai/src/provider_stderr_tests.rs`. The synthetic parent now owns its direct child with a local RAII guard, polls completion with a 30-second deadline, and kills/reaps on timeout or unwind. It routes stdout and stderr to separate files inside the existing mode-0700 temporary directory, checks each file is at most 64 KiB before reading, and preserves the success, one-child-test, three-routes, and redaction assertions. Failure messages do not print captured contents. The child route, synthetic fixture data, and parent environment behavior are unchanged.

Reuse: followed the existing in-crate owned-process pattern conceptually, narrowed to direct child ownership and file-backed capture. No shared framework, dependency, or production code was added.

Commands run: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-ai/src/provider_stderr_tests.rs`; `git diff --check`; review of the single-file diff; commit. A direct `rustfmt +1.98.1 ...` invocation was unavailable because the standalone shim was not on PATH; the pinned `rustup run` invocation succeeded.

Commands not run: all tests, builds, native checks, and controlled-negative execution. Those remain with the lead. This is an implementation candidate pending independent review and lead-owned execution. After commit the worktree was clean at the candidate hash.

## Review correction

Independent review found lossy UTF-8 decoding would newly accept malformed captures. Corrected both reads to retain strict UTF-8 acceptance while using distinct constant-only panic messages (`synthetic child stdout was not UTF-8` / `synthetic child stderr was not UTF-8`), without exposing capture bytes. Correction commit: `a6b6d9902c8797a696cb96e1d9e848c564425b9e` (`Preserve strict child output decoding`), atop the initial candidate. Re-ran only pinned formatting and `git diff --check`; both passed. No tests/builds/controlled-negative execution run. Worktree is clean at the correction commit.
