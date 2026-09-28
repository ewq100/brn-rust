# brn-workflow

Shared authoritative application flow for desktop and headless use: imports, eligibility, index lifecycle, grounded answers, saved sessions, drafts and comments. Owns application coordination across adapters.

## Interfaces and source

[Workspace API](src/lib.rs), [worker commands/events](src/worker.rs), [draft workflow](src/drafts.rs), [comment workflow](src/comments.rs), [brn-flow CLI](src/main.rs).

## Dependencies and features

Depends on `brn-store`, `brn-provider`, `brn-retrieval`. Default features are empty; `native-retrieval` forwards to retrieval `native`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-workflow --locked
bash scripts/verify-end-to-end.sh
```

Tests cover flow, drafts and comments. Keep durable authority in store and coordinate provider/retrieval through their adapters. Preserve evidence validation, operation identity and late-response safety.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
