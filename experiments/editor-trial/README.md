# BRN native editor trial

A standalone experiment for the roadmap's native editor trial. It evaluates GPUI Kit for long Markdown editing, selected-passage comments and revision review. It does not change the production `brn` CLI or provider harness.

## Scope

The opened document is an immutable baseline; typing changes only the in-memory working copy. Comments and edits disappear on exit. Original files are never saved or overwritten by this trial. The default document is a synthetic Markdown fixture. No model, account or network service is involved in editing.

The trial captures a real editor selection against the current document before focus moves to the comment field. Comments retain their original quote and are either attached to a valid range or visibly unresolved. Overlapping changes and ambiguous repeated quotes are not silently reattached. Exact restoration of the original complete document can restore an original anchor. This conservative snapshot model may orphan a comment after complex multi-location changes; it is not a production history/CRDT design.

The diff compares the opened baseline with the current text. It is refreshed explicitly, rather than recalculated after every keystroke.

## Build

Run from the repository root using Rust 1.98.1:

```sh
cargo +1.98.1 test --manifest-path experiments/editor-trial/Cargo.toml --locked --offline
cargo +1.98.1 build --manifest-path experiments/editor-trial/Cargo.toml --features native-ui --locked --offline
cargo +1.98.1 fmt --manifest-path experiments/editor-trial/Cargo.toml -- --check
cargo +1.98.1 clippy --manifest-path experiments/editor-trial/Cargo.toml --features native-ui --all-targets --locked --offline -- -D warnings
```

On this Mac, Cargo is available after `export PATH=/opt/homebrew/opt/rustup/bin:$PATH`. The first dependency setup requires `cargo +1.98.1 fetch --manifest-path experiments/editor-trial/Cargo.toml`; the checked-in lockfile fixes resolution. The optional `native-ui` feature keeps model-only tests independent of the graphical build.

## Run the trial

After building, run:

```sh
./experiments/editor-trial/target/debug/brn-editor-trial
# Or open an explicitly chosen local document:
./experiments/editor-trial/target/debug/brn-editor-trial /absolute/path/to/sample.md
```

The default sample is synthetic. The trial accepts one regular UTF-8 file up to 2 MiB. Select text, click **Capture selection**, type in the comment field, and click **Add comment**. **Show passage** selects an anchored passage; an unresolved comment retains its original quote. Edit the document and click **Refresh diff** to inspect changes from the opened revision. Native keyboard undo/redo and copy/paste belong to the editor widget.

For the automated build/model/CLI checks, run `bash scripts/verify-editor-trial.sh`. This script never opens a window or contacts a provider.

## Dependencies and target

- GPUI Kit `=0.6.6`, Apache-2.0: candidate native framework with Markdown editor and built-in assets. Its matching GPUI packages resolve through the umbrella crate; see Cargo.lock.
- `similar 2.7.0`, Apache-2.0: unified line diff.
- Verified host prerequisites: macOS 15.3.1 arm64, Command Line Tools, Rust 1.98.1. No full application distribution or minimum-OS deployment qualification is claimed.

Official references checked on 2026-09-27: [installation](https://gpui-kit.com/docs/installation/), [getting started](https://gpui-kit.com/docs/getting-started/), [Editor](https://gpui-kit.com/component/editor/), [published GPUI Kit API](https://docs.rs/gpui-kit/0.6.6/gpui_kit/), [similar API](https://docs.rs/similar/2.7.0/similar/). Implementation uses the downloaded pinned source as the authority where the website's examples are incomplete. This trial's dependency record is not a redistribution audit.

## Acceptance boundary

Build/test success alone does not establish native editing quality. The evidence report must distinguish deterministic tests, agent-observed macOS interaction and the user's own acceptance. Before choosing this UI/document model for production, the user should try selection, commenting, nearby edits, diff review, Unicode, clipboard, undo/redo, deletion, repeated text, scrolling and window resizing. IME, accessibility, large-file performance and signed packaging require their own evidence.

See [EVIDENCE.md](EVIDENCE.md) for commands and observed results, and [the plan](../../docs/editor-trial-plan.md) for the scoped design.
