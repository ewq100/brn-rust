---
name: test-brn-app
description: Run and drive the BRN desktop app or CLI against synthetic data. Use to reproduce a UI bug, check a desktop or CLI change end to end, or capture before/after screenshots for a PR.
---

# Test the BRN app

Work only in this checkout's `.brn-dev/` data folder or a fresh folder under `$TMPDIR`. The owner's real BRN data is never an input.

1. **Prepare.** If `.brn-dev/env.sh` is missing, run `bash scripts/setup-worktree.sh`. Then `source .brn-dev/env.sh`. For a clean start, `rm -rf "$BRN_DEV_DATA_DIR" && mkdir "$BRN_DEV_DATA_DIR"`.
2. **Seed.** Create content through the CLI so it passes the same checked service as the app:
   ```bash
   brn() { cargo run -q -p brn -- "$@"; }
   brn --data-dir "$BRN_DEV_DATA_DIR" init
   brn --data-dir "$BRN_DEV_DATA_DIR" note-create --title Harbor --text 'Synthetic note'
   ```
   `brn COMMAND --help` lists options. For a broad workspace, `python3 scripts/cli_journey.py target/debug/brn --keep-root` runs the maintained journey and keeps its temporary root. Desktop never opens a workspace created with `--purpose test`; seed desktop data with plain `init`.
3. **Run.**
   - Startup only: `cargo run -p brn-desktop --features native-ui -- --data-dir "$BRN_DEV_DATA_DIR" --headless-check startup`.
   - Interactive: start the same command without `--headless-check` in the background, record its PID, and drive the window with computer use. Wait on visible state, such as a title or row, rather than fixed sleeps.
   - CLI: use `brn --json ...` and read the envelope; `brn --json --help` describes every command.
4. **Capture.** Take screenshots of the states that prove the change: before and after for UI changes. Upload them to the PR description or a PR comment; keep them out of the repository.
5. **Clean up.** Stop only the PID you started (`kill <pid>`). Leave other BRN windows and processes alone; they may belong to the owner or another agent.

## Recovery

- `missing_workspace` from a read command: run `init` first.
- `upgrade_required`: the data folder came from an older build; recreate it.
- The window does not appear: run the headless check; its error names the cause.
- Document intake fails with a helper error: rerun `bash scripts/setup-worktree.sh` and `source .brn-dev/env.sh`.
