# brn

Minimal dependency-free CLI build probe. Accepts exactly one `--help` or `--version` argument; invalid invocations fail. It is not the integrated application CLI.

## Interfaces and source

[CLI entry point](src/main.rs).

## Dependencies and features

No workspace or external dependencies. Root default workspace member.

## Verification

Run from the repository root:

```sh
cargo run -p brn -- --help
cargo run -p brn -- --version
```

CLI subprocess smoke checks are included in `scripts/verify-storage.sh`. Use `brn-flow` from `brn-workflow` for application operations.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
