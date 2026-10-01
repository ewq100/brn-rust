# CLI writing primitives integration evidence

## Record status

This is a documentation record of a previously executed integration report. The Cargo checks and the full headless verification script were not rerun during the documentation task that added this record.

## Tested baseline

- Tested code SHA: `a82dd25954cbd809b290d88676aec6344fb326ac`
- Branch: `main`
- Working tree: clean at the tested baseline
- Selected reviewed task refs: all were ancestors of the tested HEAD
- Toolchain: Rust/Cargo `1.98.1`
- Features: default features only

The baseline includes the reviewed CLI writing commands: `drafts create`, `drafts checkpoint`, `drafts save`, `comments add`, `comments resolve` and `comments reopen`. These commands remain implemented through the shared workflow; this record does not add or change product behavior.

## Supplied integration report

The existing headless integration script was run once with a disposable `/tmp` target:

```text
bash scripts/verify-end-to-end.sh
```

The script executed the pinned, locked, offline workspace checks:

```text
cargo +1.98.1 fmt --all --check
cargo +1.98.1 build --workspace --locked --offline
cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test --workspace --locked --offline
```

Recorded outcomes:

- Formatting: passed.
- Locked offline workspace build: passed.
- Locked offline all-target Clippy with warnings denied: passed.
- Locked offline workspace tests: 261 passed, 0 failed.
- Synthetic provider-free headless workflow: passed.
- `git diff --check`: passed.
- Changed CLI/size-report examples and local documentation links were inspected.

## Qualification limits

This report is default-feature/headless evidence only. It deliberately excludes native UI and native retrieval, live provider calls, model assets, GUI startup and real-vault access. The native smoke target contained zero tests under the default features; that is not native qualification. Native size-report verification is also not inferred from this report.

The report does not establish user acceptance, accessibility or IME suitability, representative-corpus search relevance, installation/signing readiness, provider authentication lifecycle behavior, or release readiness. Optional native features and standalone experiments require separate verification.

## Integration state

The tested code baseline was already merged into `main` at the SHA above. This documentation record is separate from that tested code and must not be treated as a new verification run.
