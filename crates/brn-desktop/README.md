# brn-desktop

Desktop entry point, GPUI views and transient interaction state. Also retains sample headless shell checks. Integrated operations go through the workflow worker.

## Interfaces and source

[Entry point](src/main.rs), [native views](src/native.rs), [draft UI](src/drafts.rs), [comment UI](src/comments.rs), [CLI tests](tests/cli.rs).

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
