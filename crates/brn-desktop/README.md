# brn-desktop

Desktop entry point, GPUI views and transient interaction state. Also retains sample headless shell checks. Integrated operations go through the workflow worker.

## Interfaces and source

[Entry point](src/main.rs), [layout model](src/layout.rs) and [tokens](src/tokens.rs) (GPUI-free, default-feature tests), [native shell](src/native/mod.rs) with [theme](src/native/theme.rs) and [regions](src/native/shell/mod.rs), [draft UI](src/drafts.rs), [comment UI](src/comments.rs), [CLI tests](tests/cli.rs). Layout and appearance persist to `layout.json` in the data directory. See the [workspace shell decision](../../docs/architecture/decisions/2026-10-01-workspace-shell.md) and the [UI feature backlog](../../docs/ui/feature-backlog.md).

The native workspace has History and Vault rails around a document/chat centre.
Narrow windows collapse rails and use Document/Chat tabs; Focus hides the rails.
Closing a document retains draft edits, and late draft-open results cannot replace
newer document navigation. Settings uses the toolkit modal host for appearance,
rail widths and layout reset. Layout preferences are stored separately from the
authoritative workflow data.

The three dividers support pointer dragging and keyboard resizing: Tab to focus,
then ←/→ for 8 pt or ⇧←/→ for 32 pt. Widths persist when a drag ends, including
when the window deactivates. The BRN, View and Navigate menus expose Settings
(⌘,), Quit (⌘Q), History (⌘0), Vault (⌥⌘0), Focus (⇧⌘↩), New Chat (⌘N),
Focus Composer (⌘L) and Cancel Running Action (⌘.). New Chat is idle-only;
Quit still respects dirty drafts. Escape remains local to editors and the
toolkit Settings dialog.

## Dependencies and features

Always depends on `brn-core`. Default features are empty; `native-ui` enables GPUI and `brn-workflow`; `native-retrieval` includes native UI and workflow native retrieval.

## Verification

Run from the repository root:

```sh
cargo test -p brn-desktop --locked
bash scripts/verify-desktop-shell.sh --native
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
