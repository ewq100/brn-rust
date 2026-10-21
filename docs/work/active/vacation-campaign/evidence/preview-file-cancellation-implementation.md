# Preview-file cancellation candidate

Branch: `codex/preview-file-cancellation`  
Base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`  
Commit: `a054232c8fa5ddf019263823f83753b9a67013f9` (`Make preview file reads cancellable`)

Changed only:

- `crates/brn/src/input.rs`: factored the existing detached-reader/polling loop into private `read_cancellable`; text-file reads retain their existing bounded parsing. Added `read_preview_file`, which applies the same cancellation wait to `std::fs::read` and preserves the existing `preview-file/unreadable` mapping and raw bytes.
- `crates/brn/src/main.rs`: routes `project-delete --preview-file` through that reader. JSON parsing and `invalid_json` handling remain at the same call site.
- `crates/brn/tests/agent_bounds.rs`: added a Unix process test with a writerless preview FIFO and one-second deadline. It requires exit 7, timeout with `hard: false`, `stop: deadline`, completion under 20 seconds, and identical records/history before and after.

This keeps FIFO-with-writer, symlink, relative-path, regular-file, raw JSON and I/O classifications intact. A blocked read now returns to the existing stop handler before execution can mutate state. The detached reader may remain blocked until the CLI process exits, as with the existing cancellable file-input mechanism.

Checks run:

- `PATH=/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin:/opt/homebrew/bin:/usr/bin:/bin cargo-fmt --check -p brn` — passed.
- `git diff --check` — passed before commit.
- No build or tests were run, per assignment. In particular, the new regression still needs the lead's candidate run and comparison against the frozen `AGENT_CONTRACT_BASELINE_BIN`.

The initial `cargo +1.98.1 fmt` command was unavailable because `cargo` was not on PATH; invoking the installed pinned `cargo-fmt` with its toolchain directory on PATH completed successfully. No product checks beyond formatting and whitespace were claimed.
