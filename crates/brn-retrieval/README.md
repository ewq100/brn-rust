# brn-retrieval

Derived retrieval generations and keyword, semantic and hybrid search. Exposes typed documents, profiles and exact evidence; validates persisted generation state and hits.

## Interfaces and source

[Index API](src/lib.rs), [native adapter](src/native.rs), [keyword tests](tests/keyword.rs), [native smoke tests](tests/native_smoke.rs).

## Dependencies and features

No workspace dependencies. Default features are empty. `native` enables FastEmbed/ONNX, LanceDB, Arrow and supporting async libraries. Consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-retrieval --locked
cargo test -p brn-retrieval --features native --locked
```

Native tests require inspection of fixture/resource conditions; a skipped resource path is not model verification. Preserve exact evidence and generation checks; never treat an index as authoritative storage.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
