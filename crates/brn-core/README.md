# brn-core

UI-independent in-memory sample shell and owned worker lifecycle. It models progress, cancellation, generation correlation and shutdown for the original desktop sample. Integrated durable application operations live in `brn-workflow`.

## Interfaces and source

[Shell model](src/lib.rs), [shell contract tests](tests/shell.rs).

## Dependencies and features

No workspace or external dependencies. Consumed by `brn-desktop`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-core --locked
```

Preserve bounded worker lifetime, stale-result handling and terminal events. Headless desktop scenarios are exercised by `scripts/verify-desktop-shell.sh`.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
