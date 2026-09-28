# brn-provider

Codex App Server adapter. Owns its launched sidecar, request correlation, thread start/resume, turn streaming/cancellation and bounded shutdown.

## Interfaces and source

[Client and protocol API](src/lib.rs), [fake-sidecar tests](tests/fake_sidecar.rs).

## Dependencies and features

No workspace dependencies. Uses serde_json and Unix libc; consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-provider --locked
```

Default tests use a fake sidecar. Live calls need an explicitly configured executable and task authorization. Provider-owned authentication and thread history must not become copied credential state.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
